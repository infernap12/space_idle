use crate::types::{FleetEntry, FleetShip};

pub fn parse_fleet_string(raw: &str) -> Option<FleetEntry> {
    let mut s = raw.trim();
    let pos_delim = if let Some(rest) = s.strip_prefix("V2") {
        s = rest;
        '*'
    } else {
        '.'
    };

    let parts: Vec<&str> = s.split('|').collect();
    if parts.len() < 2 {
        return None;
    }

    let header: Vec<&str> = parts[0].split(',').collect();
    let resources_used: f64 = header.first()?.parse().ok()?;
    let resources_committed: f64 = header.get(1)?.parse().ok()?;
    let fleet_stats: Option<f64> = header.get(2).and_then(|s| s.parse().ok());

    let mut ships: Vec<FleetShip> = Vec::new();
    let mut consumed_tail = false;

    for part in parts.iter().skip(2) {
        if part.is_empty() {
            continue;
        }
        if !part.contains(';') {
            consumed_tail = true;
            break;
        }
        let sp: Vec<&str> = part.split(';').collect();
        if sp.is_empty() || sp[0] == "Null" {
            continue;
        }
        let ship_type = sp[0].to_string();
        let mods: Vec<String> = sp
            .get(1)
            .map(|m| m.split(',').filter(|x| !x.is_empty()).map(String::from).collect())
            .unwrap_or_default();
        let positions: Vec<(i32, i32)> = sp
            .get(2)
            .map(|p| {
                p.split(pos_delim)
                    .filter_map(|pair| {
                        let (c, r) = pair.split_once(',')?;
                        let c: f64 = c.parse().ok()?;
                        let r: f64 = r.parse().ok()?;
                        Some((c as i32, r as i32))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let count: i32 = sp
            .get(3)
            .and_then(|c| c.parse().ok())
            .unwrap_or(positions.len() as i32);
        ships.push(FleetShip { ship_type, mods, positions, count });
    }

    let mut boss_damage = 0.0;
    if let Some(tail) = parts.last() {
        if !tail.contains(';') {
            if let Ok(n) = tail.parse::<f64>() {
                boss_damage = n;
            }
        }
    }
    let _ = consumed_tail;

    Some(FleetEntry {
        resources_used,
        resources_committed,
        fleet_stats,
        boss_damage,
        ships,
    })
}

pub fn bucket_for(id: &str) -> Option<String> {
    let rest = id.strip_prefix('g')?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

pub fn find_fleet_event_id<'a>(
    rows: impl IntoIterator<Item = &'a crate::types::FleetEvent>,
    galaxy: u32,
    node_name: &str,
) -> Option<String> {
    let prefix = format!("g{galaxy}");
    let up = node_name.to_uppercase();
    for r in rows {
        if !r.id.starts_with(&prefix) {
            continue;
        }
        let next = r.id.as_bytes().get(prefix.len()).copied();
        if let Some(b) = next {
            if b.is_ascii_digit() {
                continue;
            }
        }
        if r.name == up {
            return Some(r.id.clone());
        }
    }
    None
}
