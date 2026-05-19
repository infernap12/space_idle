use dioxus::prelude::*;

use crate::components::grid::{mod_visual_html, ship_visual_html};
use crate::state::AppState;
use crate::types::FleetShip;

#[component]
pub fn ShipList(ships: Vec<FleetShip>) -> Element {
    let app = use_context::<AppState>();
    let skin = app.skin.read().clone();
    let sprites = app.db_sprites.read();
    let mod_icons = app.db_mod_icons.read();

    if ships.is_empty() {
        return rsx! {
            div { class: "ship-row", span { class: "mods", "No ships" } }
        };
    }

    rsx! {
        for s in ships.iter() {
            {
                let mod_icons_html: String = s
                    .mods
                    .iter()
                    .map(|m| mod_visual_html(m, &mod_icons))
                    .filter(|h| !h.is_empty())
                    .collect();
                let mod_text = if s.mods.is_empty() {
                    "no mods".to_string()
                } else {
                    s.mods.join(", ")
                };
                let positions = if s.positions.is_empty() {
                    "no positions".to_string()
                } else {
                    s.positions
                        .iter()
                        .map(|(c, r)| format!("({c},{})", r + 1))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let visual = ship_visual_html(&s.ship_type, &skin, &sprites);
                rsx! {
                    div {
                        class: "ship-row",
                        div {
                            class: "icon",
                            dangerous_inner_html: "{visual}",
                        }
                        div {
                            style: "flex:1; min-width: 0;",
                            div {
                                span { class: "name", "{s.ship_type}" }
                                " × {s.count}"
                            }
                            div {
                                class: "mods",
                                if !mod_icons_html.is_empty() {
                                    span {
                                        class: "mod-icons",
                                        dangerous_inner_html: "{mod_icons_html}",
                                    }
                                }
                                span { class: "mod-text", "{mod_text} · {positions}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
