use crate::parser::bucket_for;
use crate::types::{DbData, FleetEvent, HazardEntry, HazardSource};
use sqlite_wasm_rs as ffi;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

const SQLITE_OK: c_int = ffi::SQLITE_OK as c_int;
const SQLITE_ROW: c_int = ffi::SQLITE_ROW as c_int;
const SQLITE_DONE: c_int = ffi::SQLITE_DONE as c_int;

pub async fn fetch_db_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| format!("fetch failed: {e}"))?;
    if !response.ok() {
        return Err(format!("Failed to load {url}: HTTP {}", response.status()));
    }
    let bytes = response
        .binary()
        .await
        .map_err(|e| format!("read body failed: {e}"))?;
    Ok(bytes)
}

struct Db {
    handle: *mut ffi::sqlite3,
    _bytes: *mut u8,
}

impl Db {
    fn open_from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        unsafe {
            let mut db: *mut ffi::sqlite3 = ptr::null_mut();
            let name = CString::new("mem.db").unwrap();
            let rc = ffi::sqlite3_open_v2(
                name.as_ptr(),
                &mut db,
                (ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE) as c_int,
                ptr::null(),
            );
            if rc != SQLITE_OK {
                return Err(format!("sqlite3_open_v2 failed: {rc}"));
            }
            let len = bytes.len();
            let boxed = bytes.into_boxed_slice();
            let buf_ptr = Box::into_raw(boxed) as *mut u8;
            let zname = CString::new("main").unwrap();
            let rc = ffi::sqlite3_deserialize(
                db,
                zname.as_ptr(),
                buf_ptr,
                len as i64,
                len as i64,
                (ffi::SQLITE_DESERIALIZE_FREEONCLOSE | ffi::SQLITE_DESERIALIZE_RESIZEABLE) as u32,
            );
            if rc != SQLITE_OK {
                let msg = error_message(db);
                ffi::sqlite3_close(db);
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(buf_ptr, len));
                return Err(format!("sqlite3_deserialize failed: {rc} ({msg})"));
            }
            Ok(Self { handle: db, _bytes: buf_ptr })
        }
    }

    fn query(&self, sql: &str) -> Result<Vec<Vec<Value>>, String> {
        unsafe {
            let csql = CString::new(sql).unwrap();
            let mut stmt: *mut ffi::sqlite3_stmt = ptr::null_mut();
            let rc = ffi::sqlite3_prepare_v2(
                self.handle,
                csql.as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            );
            if rc != SQLITE_OK {
                return Err(format!("prepare failed: {} ({})", rc, error_message(self.handle)));
            }
            let col_count = ffi::sqlite3_column_count(stmt) as usize;
            let mut rows = Vec::new();
            loop {
                let rc = ffi::sqlite3_step(stmt);
                if rc == SQLITE_ROW {
                    let mut row = Vec::with_capacity(col_count);
                    for i in 0..col_count as c_int {
                        row.push(read_column(stmt, i));
                    }
                    rows.push(row);
                } else if rc == SQLITE_DONE {
                    break;
                } else {
                    ffi::sqlite3_finalize(stmt);
                    return Err(format!("step failed: {} ({})", rc, error_message(self.handle)));
                }
            }
            ffi::sqlite3_finalize(stmt);
            Ok(rows)
        }
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.handle);
            // sqlite3 frees the buffer because of SQLITE_DESERIALIZE_FREEONCLOSE.
        }
    }
}

unsafe fn error_message(db: *mut ffi::sqlite3) -> String {
    let p = ffi::sqlite3_errmsg(db);
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

unsafe fn read_column(stmt: *mut ffi::sqlite3_stmt, i: c_int) -> Value {
    let ty = ffi::sqlite3_column_type(stmt, i);
    if ty == ffi::SQLITE_INTEGER as c_int {
        Value::Int(ffi::sqlite3_column_int64(stmt, i))
    } else if ty == ffi::SQLITE_FLOAT as c_int {
        Value::Real(ffi::sqlite3_column_double(stmt, i))
    } else if ty == ffi::SQLITE_TEXT as c_int {
        let ptr = ffi::sqlite3_column_text(stmt, i) as *const c_char;
        if ptr.is_null() {
            Value::Null
        } else {
            Value::Text(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    } else {
        Value::Null
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Real(f64),
    Text(String),
    Null,
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        if let Value::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Real(f) => Some(*f as i64),
            _ => None,
        }
    }
}

pub async fn load_db(url: &str) -> Result<DbData, String> {
    let bytes = fetch_db_bytes(url).await?;
    parse_db(bytes)
}

fn parse_db(bytes: Vec<u8>) -> Result<DbData, String> {
    let db = Db::open_from_bytes(bytes)?;

    let events = db.query("SELECT id, name FROM fleet_events ORDER BY rowid")?;
    let hazard_rows = db.query(
        "SELECT _parent, type, repeat, nodes_effected FROM fleet_events__hazard ORDER BY _row_id",
    )?;
    let ship_rows = db.query(
        "SELECT fs.id, s.select_sprite FROM fleet_ships fs JOIN ships s ON s.id = fs.ship WHERE fs.is_player_ship = 1",
    )?;
    let mod_rows = db.query("SELECT id, icon FROM fleet_mods")?;

    let mut hazards_by_parent: HashMap<String, Vec<HazardEntry>> = HashMap::new();
    for row in &hazard_rows {
        let Some(parent) = row.first().and_then(|v| v.as_text()).map(String::from) else { continue };
        let hazard_type = row.get(1).and_then(|v| v.as_text()).unwrap_or("?").to_string();
        let repeat = row.get(2).and_then(|v| v.as_int()).unwrap_or(0) != 0;
        let nodes_json = row.get(3).and_then(|v| v.as_text()).unwrap_or("[]");
        let nodes: Vec<String> = serde_json::from_str::<serde_json::Value>(nodes_json)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(|item| {
                        item.as_object()
                            .and_then(|o| o.get("fleet_event"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default();
        hazards_by_parent
            .entry(parent)
            .or_default()
            .push(HazardEntry { hazard_type, repeat, nodes_effected: nodes });
    }

    let mut rows: Vec<FleetEvent> = Vec::with_capacity(events.len());
    for row in &events {
        let id = row.first().and_then(|v| v.as_text()).unwrap_or("").to_string();
        let name = row.get(1).and_then(|v| v.as_text()).unwrap_or("").to_string();
        let hazards = hazards_by_parent.remove(&id).unwrap_or_default();
        rows.push(FleetEvent { id, name, hazards });
    }

    let mut ship_sprites = HashMap::new();
    for row in &ship_rows {
        let Some(id) = row.first().and_then(|v| v.as_text()) else { continue };
        let Some(sprite) = row.get(1).and_then(|v| v.as_text()) else { continue };
        if !sprite.is_empty() {
            ship_sprites.insert(id.to_string(), sprite.to_string());
        }
    }

    let mut mod_icons = HashMap::new();
    for row in &mod_rows {
        let Some(id) = row.first().and_then(|v| v.as_text()) else { continue };
        let Some(icon) = row.get(1).and_then(|v| v.as_text()) else { continue };
        if !icon.is_empty() {
            mod_icons.insert(id.to_string(), icon.to_string());
        }
    }

    let mut rows_by_galaxy: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        if let Some(g) = bucket_for(&r.id) {
            rows_by_galaxy.entry(g).or_default().push(i);
        }
    }

    let mut hazards_by_target: HashMap<String, Vec<HazardSource>> = HashMap::new();
    for r in &rows {
        for entry in &r.hazards {
            for tgt in &entry.nodes_effected {
                hazards_by_target
                    .entry(tgt.clone())
                    .or_default()
                    .push(HazardSource {
                        source_id: r.id.clone(),
                        source_name: r.name.clone(),
                        hazard_type: entry.hazard_type.clone(),
                        repeat: entry.repeat,
                        is_self: r.id == *tgt,
                    });
            }
        }
    }

    Ok(DbData {
        rows,
        ship_sprites,
        mod_icons,
        rows_by_galaxy,
        hazards_by_target,
    })
}
