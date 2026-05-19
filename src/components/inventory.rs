use dioxus::prelude::*;

use crate::components::grid::ship_visual_html;
use crate::state::AppState;
use crate::types::SHIP_TYPES;

#[component]
pub fn Inventory() -> Element {
    let app = use_context::<AppState>();
    let inv_on = *app.inv_filter.read();
    let skin = app.skin.read().clone();
    let sprites = app.db_sprites.read();

    rsx! {
        div {
            class: "inventory",
            label {
                input {
                    r#type: "checkbox",
                    checked: inv_on,
                    oninput: {
                        let app = app.clone();
                        move |evt: FormEvent| {
                            let v = evt.value() == "true" || evt.checked();
                            *app.inv_filter.clone().write() = v;
                        }
                    },
                }
                " Only show fleets I can build"
            }
            div {
                class: if inv_on { "inv-grid" } else { "inv-grid hidden" },
                for kind in SHIP_TYPES.iter() {
                    {
                        let visual = ship_visual_html(kind.name, &skin, &sprites);
                        let name = kind.name;
                        let current = *app
                            .inventory
                            .read()
                            .get(name)
                            .unwrap_or(&0i32);
                        let app2 = app.clone();
                        let name_owned = name.to_string();
                        rsx! {
                            label {
                                class: "inv-item",
                                title: "{name}",
                                span {
                                    class: "icon",
                                    dangerous_inner_html: "{visual}",
                                }
                                span { "{name}" }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    step: "1",
                                    value: "{current}",
                                    oninput: move |evt: FormEvent| {
                                        let n: i32 = evt.value().parse().unwrap_or(0);
                                        app2.inventory.clone().write().insert(name_owned.clone(), n);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
