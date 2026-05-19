#![allow(non_snake_case)]

use dioxus::prelude::*;

mod api;
mod components;
mod db;
mod parser;
mod sprites;
mod state;
mod types;

use components::hazards_panel::HazardsPanel;
use components::inventory::Inventory;
use components::result_card::ResultCard;
use parser::find_fleet_event_id;
use state::{new_state, AppState};
use types::{ParsedEntry, SKINS};

const GAME_DB_URL: &str = "assets/game_data.sqlite";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let app = use_context_provider(new_state);

    use_future({
        let app = app;
        move || async move {
            match db::load_db(GAME_DB_URL).await {
                Ok(data) => {
                    *app.db_sprites.clone().write() = data.ship_sprites.clone();
                    *app.db_mod_icons.clone().write() = data.mod_icons.clone();
                    *app.db_loaded.clone().write() = Some(data);
                }
                Err(e) => {
                    *app.is_error.clone().write() = true;
                    *app.status.clone().write() = format!("Error loading database: {e}");
                }
            }
        }
    });

    let db_for_list = app.db_loaded.read();
    let galaxy_val = *app.galaxy.read();
    let node_options: Vec<String> = if let Some(data) = db_for_list.as_ref() {
        let g = galaxy_val.to_string();
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        if let Some(idxs) = data.rows_by_galaxy.get(&g) {
            for i in idxs {
                let r = &data.rows[*i];
                if r.name.is_empty() || seen.contains(&r.name) {
                    continue;
                }
                seen.insert(r.name.clone());
                out.push(r.name.clone());
            }
        }
        out
    } else {
        Vec::new()
    };
    drop(db_for_list);

    let status_text = app.status.read().clone();
    let status_class = if *app.is_error.read() { "status error" } else { "status" };
    let working = *app.working.read();

    rsx! {
        document::Title { "Fleet Suggest" }
        TopForm {}
        HazardsPanel {}
        Inventory {}
        button {
            id: "go",
            disabled: working,
            onclick: move |_| {
                let app = app;
                spawn(async move {
                    run(app).await;
                });
            },
            "Suggest"
        }
        div {
            id: "status",
            class: "{status_class}",
            "{status_text}"
        }
        // Browser-built-in autocomplete via <datalist>.
        datalist {
            id: "nodeList",
            for name in node_options.iter() {
                option { value: "{name}" }
            }
        }
        ResultCard {}
        KeyboardShortcuts {}
    }
}

#[component]
fn TopForm() -> Element {
    let app = use_context::<AppState>();

    let node = app.node.read().clone();
    let stat = app.stat.read().clone();
    let galaxy = *app.galaxy.read();
    let version = *app.version.read();
    let boss = *app.has_boss.read();
    let skin = app.skin.read().clone();

    rsx! {
        h1 { "Fleet Suggest" }
        div {
            class: "row",
            div {
                label {
                    "Node "
                    input {
                        r#type: "text",
                        list: "nodeList",
                        placeholder: "e.g. B1 or BB3",
                        autocomplete: "off",
                        value: "{node}",
                        oninput: {
                            let app = app;
                            move |evt: FormEvent| {
                                *app.node.clone().write() = evt.value();
                                // Clear selected hazards so we don't carry over IDs from a different node.
                                app.selected_hazards.clone().write().clear();
                            }
                        },
                    }
                }
            }
            div {
                label {
                    "Stat level "
                    input {
                        r#type: "number",
                        step: "0.05",
                        placeholder: "e.g. 12.5",
                        value: "{stat}",
                        oninput: {
                            let app = app;
                            move |evt: FormEvent| {
                                *app.stat.clone().write() = evt.value();
                            }
                        },
                    }
                }
            }
        }
        div {
            class: "row",
            div {
                label {
                    "Galaxy "
                    input {
                        r#type: "number",
                        value: "{galaxy}",
                        oninput: {
                            let app = app;
                            move |evt: FormEvent| {
                                if let Ok(v) = evt.value().parse::<u32>() {
                                    *app.galaxy.clone().write() = v;
                                    app.selected_hazards.clone().write().clear();
                                }
                            }
                        },
                    }
                }
            }
            div {
                label {
                    "Version "
                    input {
                        r#type: "number",
                        value: "{version}",
                        oninput: {
                            let app = app;
                            move |evt: FormEvent| {
                                if let Ok(v) = evt.value().parse::<i64>() {
                                    *app.version.clone().write() = v;
                                }
                            }
                        },
                    }
                }
            }
        }
        div {
            class: "row",
            div {
                label {
                    input {
                        r#type: "checkbox",
                        checked: boss,
                        oninput: {
                            let app = app;
                            move |evt: FormEvent| {
                                *app.has_boss.clone().write() = evt.checked();
                            }
                        },
                    }
                    " Boss (for BB nodes)"
                }
            }
            div {
                label {
                    "Ship skin "
                    select {
                        value: "{skin}",
                        oninput: {
                            let app = app;
                            move |evt: FormEvent| {
                                *app.skin.clone().write() = evt.value();
                            }
                        },
                        for s in SKINS.iter().filter(|s| s.show) {
                            option { value: s.key, "{s.display_name}" }
                        }
                    }
                }
            }
        }
    }
}

async fn run(app: AppState) {
    *app.is_error.clone().write() = false;
    *app.status.clone().write() = "Working…".to_string();
    *app.entries.clone().write() = Vec::new();
    *app.all_entries.clone().write() = Vec::new();
    *app.working.clone().write() = true;

    let outcome = run_inner(app).await;
    if let Err(e) = outcome {
        *app.is_error.clone().write() = true;
        *app.status.clone().write() = format!("Error: {e}");
    }
    *app.working.clone().write() = false;
}

async fn run_inner(app: AppState) -> Result<(), String> {
    let node = app.node.read().trim().to_string();
    if node.is_empty() {
        return Err("Node name required".into());
    }
    let stat: f64 = app
        .stat
        .read()
        .trim()
        .parse()
        .map_err(|_| "Invalid stat level".to_string())?;
    let galaxy = *app.galaxy.read();
    let version = *app.version.read();
    let has_boss = *app.has_boss.read();

    let db = {
        let guard = app.db_loaded.read();
        match guard.as_ref() {
            Some(d) => d.clone(),
            None => return Err("Database not loaded yet".into()),
        }
    };

    let fleet_event_id = find_fleet_event_id(&db.rows, galaxy, &node)
        .ok_or_else(|| format!("Could not find node '{node}' in galaxy {galaxy}"))?;

    let source_ids = app.selected_hazards.read().clone();
    let progress_app = app;
    let result = api::fetch_suggestion(
        &fleet_event_id,
        version,
        stat,
        has_boss,
        source_ids,
        move |i, total| {
            *progress_app.status.clone().write() = format!("Working… (hazard order {i}/{total})");
        },
    )
    .await?;

    if result.total > 1 {
        web_sys::console::log_1(
            &format!("Hazard order resolved on attempt {}/{}", result.attempt, result.total).into(),
        );
    }

    let mut parsed: Vec<ParsedEntry> = Vec::new();
    for (i, line) in result.lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(entry) = parser::parse_fleet_string(trimmed) {
            parsed.push(ParsedEntry { orig_idx: i + 1, entry });
        }
    }

    if has_boss {
        parsed.sort_by(|a, b| {
            let ra = if a.entry.resources_used > 0.0 {
                a.entry.boss_damage / a.entry.resources_used
            } else {
                f64::NEG_INFINITY
            };
            let rb = if b.entry.resources_used > 0.0 {
                b.entry.boss_damage / b.entry.resources_used
            } else {
                f64::NEG_INFINITY
            };
            match rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal) {
                std::cmp::Ordering::Equal => a
                    .entry
                    .resources_used
                    .partial_cmp(&b.entry.resources_used)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ord => ord,
            }
        });
    } else {
        parsed.sort_by(|a, b| {
            a.entry
                .resources_used
                .partial_cmp(&b.entry.resources_used)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    if parsed.is_empty() {
        return Err("Server returned no fleet entries".into());
    }

    *app.all_entries.clone().write() = parsed;
    *app.event_id.clone().write() = fleet_event_id;
    *app.has_boss.clone().write() = has_boss;
    apply_filter(app);
    Ok(())
}

pub fn apply_filter(app: AppState) {
    let all = app.all_entries.read().clone();
    if all.is_empty() {
        return;
    }
    let on = *app.inv_filter.read();
    let filtered: Vec<ParsedEntry> = if on {
        let inv = app.inventory.read().clone();
        all.iter()
            .filter(|pe| fleet_fits(&pe.entry, &inv))
            .cloned()
            .collect()
    } else {
        all.clone()
    };

    let n = filtered.len();
    let total = all.len();
    let event_id = app.event_id.read().clone();
    *app.entries.clone().write() = filtered;
    *app.index.clone().write() = 0;
    *app.is_error.clone().write() = false;

    if n == 0 {
        *app.status.clone().write() = format!(
            "0 of {total} suggestion{} match your ship inventory.",
            if total == 1 { "" } else { "s" }
        );
    } else {
        let suffix = if on && n != total { format!(" (of {total})") } else { String::new() };
        *app.status.clone().write() = format!(
            "{n} suggestion{}{} for {event_id}.",
            if n == 1 { "" } else { "s" },
            suffix
        );
    }
}

fn fleet_fits(entry: &types::FleetEntry, inv: &std::collections::HashMap<String, i32>) -> bool {
    entry.ships.iter().all(|s| {
        let required = s.count.min(s.positions.len() as i32);
        inv.get(&s.ship_type).copied().unwrap_or(0) >= required
    })
}

#[component]
fn KeyboardShortcuts() -> Element {
    let app = use_context::<AppState>();

    // React to inv_filter / inventory changes by re-filtering.
    let _inv_on = *app.inv_filter.read();
    let _inv_snapshot = app.inventory.read().clone();
    use_effect(move || {
        // Touch the signals so this effect re-runs on change.
        let _ = app.inv_filter.read();
        let _ = app.inventory.read();
        if !app.all_entries.read().is_empty() {
            apply_filter(app);
        }
    });

    use_effect(move || {
        use wasm_bindgen::prelude::*;
        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let key = e.key();
            let typing = document
                .active_element()
                .map(|el| {
                    let tag = el.tag_name();
                    tag == "INPUT" || tag == "TEXTAREA"
                })
                .unwrap_or(false);
            if key == "Enter" && typing && !*app.working.read() {
                let app2 = app;
                wasm_bindgen_futures::spawn_local(async move {
                    run(app2).await;
                });
                return;
            }
            let n = app.entries.read().len();
            if n == 0 {
                return;
            }
            let idx = *app.index.read();
            if key == "ArrowLeft" && idx > 0 {
                *app.index.clone().write() = idx - 1;
            } else if key == "ArrowRight" && idx + 1 < n {
                *app.index.clone().write() = idx + 1;
            }
        });
        let window = web_sys::window().expect("window");
        let _ = window
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
        closure.forget();
    });

    rsx! {}
}
