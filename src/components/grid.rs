use dioxus::prelude::*;
use std::collections::HashMap;

use crate::sprites::{apply_skin_to_path, ship_svg, ship_svg_data_url};
use crate::state::AppState;
use crate::types::{is_wide_grid, FleetShip};

#[component]
pub fn ShipGrid(ships: Vec<FleetShip>, event_id: String) -> Element {
    let app = use_context::<AppState>();
    let skin = app.skin.read().clone();
    let sprites = app.db_sprites.read();

    let cols: usize = if is_wide_grid(&event_id) { 3 } else { 2 };
    let col_labels = ["A", "B", "C"];

    let mut occ: HashMap<(i32, i32), String> = HashMap::new();
    let mut count_at: HashMap<(i32, i32), i32> = HashMap::new();
    for s in &ships {
        for (c, r) in &s.positions {
            occ.insert((*c, *r), s.ship_type.clone());
            *count_at.entry((*c, *r)).or_insert(0) += 1;
        }
    }

    let template = format!("1.5rem repeat({cols}, 60px)");

    rsx! {
        div {
            class: "grid",
            style: "grid-template-columns: {template};",
            div { class: "rhdr" }
            for col in (0..cols).rev() {
                div { class: "hdr", "{col_labels[col]}" }
            }
            for row in 0..5usize {
                div { class: "rhdr", "{row + 1}" }
                for col in (0..cols).rev() {
                    {
                        let key = (col as i32, row as i32);
                        if let Some(ty) = occ.get(&key) {
                            let badge_count = count_at.get(&key).copied().unwrap_or(0);
                            let title = format!("{ty} @ ({col},{row})");
                            let visual = ship_visual_html(ty, &skin, &sprites);
                            rsx! {
                                div {
                                    class: "cell",
                                    title: "{title}",
                                    div {
                                        style: "width:100%;height:100%;display:flex;align-items:center;justify-content:center;",
                                        dangerous_inner_html: "{visual}",
                                    }
                                    if badge_count > 1 {
                                        span { class: "badge", "×{badge_count}" }
                                    }
                                }
                            }
                        } else {
                            rsx! { div { class: "cell" } }
                        }
                    }
                }
            }
        }
    }
}

pub fn ship_visual_html(ty: &str, skin: &str, sprites: &HashMap<String, String>) -> String {
    let base = sprites.get(ty).cloned();
    match base {
        None => ship_svg(ty),
        Some(b) => {
            let path = apply_skin_to_path(&b, skin);
            let fb = ship_svg_data_url(ty);
            // Escape quotes in the fallback URL for safe inline use.
            let fb_safe = fb.replace('\'', "%27");
            format!(
                r#"<img class="ship-img" alt="{ty}" src="{path}" onerror="this.onerror=null;this.src='{fb_safe}'">"#
            )
        }
    }
}

pub fn mod_visual_html(mod_id: &str, mod_icons: &HashMap<String, String>) -> String {
    let Some(path) = mod_icons.get(mod_id) else { return String::new() };
    let safe_id = html_escape(mod_id);
    let safe_path = html_escape(path);
    format!(
        r#"<img class="mod-img" alt="{safe_id}" title="{safe_id}" src="{safe_path}" onerror="this.style.display='none'">"#
    )
}

pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
