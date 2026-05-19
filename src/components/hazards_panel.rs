use dioxus::prelude::*;

use crate::parser::find_fleet_event_id;
use crate::state::AppState;

#[component]
pub fn HazardsPanel() -> Element {
    let app = use_context::<AppState>();
    let node = app.node.read().clone();
    let galaxy = *app.galaxy.read();
    let db_loaded = app.db_loaded.read().clone();

    let hazards = {
        let Some(data) = db_loaded.as_ref() else { return rsx! {} };
        let trimmed = node.trim();
        if trimmed.is_empty() {
            return rsx! {};
        }
        let Some(id) = find_fleet_event_id(&data.rows, galaxy, trimmed) else {
            return rsx! {};
        };
        data.hazards_by_target.get(&id).cloned().unwrap_or_default()
    };

    if hazards.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "hazards",
            h2 { "Active hazards" }
            div {
                for h in hazards.iter() {
                    {
                        let repeat_tag = if h.repeat { " · repeat" } else { "" };
                        let source_id = h.source_id.clone();
                        let checked = app
                            .selected_hazards
                            .read()
                            .contains(&source_id);
                        let app2 = app.clone();
                        let sid_for_handler = source_id.clone();
                        rsx! {
                            label {
                                class: "hazard-item",
                                input {
                                    r#type: "checkbox",
                                    checked,
                                    oninput: move |evt: FormEvent| {
                                        let on = evt.checked();
                                        let mut set = app2.selected_hazards.clone();
                                        let mut w = set.write();
                                        if on {
                                            if !w.contains(&sid_for_handler) {
                                                w.push(sid_for_handler.clone());
                                            }
                                        } else {
                                            w.retain(|s| s != &sid_for_handler);
                                        }
                                    },
                                }
                                span {
                                    b { "{h.source_name}" }
                                    " "
                                    span {
                                        class: "meta",
                                        "{h.source_id} · {h.hazard_type}{repeat_tag}"
                                    }
                                }
                                if h.is_self {
                                    span { class: "self-tag", "self" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
