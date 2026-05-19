use dioxus::prelude::*;

use crate::components::grid::ShipGrid;
use crate::components::ship_list::ShipList;
use crate::state::AppState;

#[component]
pub fn ResultCard() -> Element {
    let app = use_context::<AppState>();
    let entries = app.entries.read().clone();
    let n = entries.len();
    if n == 0 {
        return rsx! {};
    }

    let mut idx = *app.index.read();
    if idx >= n {
        idx = n - 1;
    }
    let cur = &entries[idx];
    let entry = &cur.entry;
    let orig_idx = cur.orig_idx;
    let event_id = app.event_id.read().clone();
    let has_boss = *app.has_boss.read();

    let saved = entry.resources_committed - entry.resources_used;
    let stats_str = match entry.fleet_stats {
        Some(v) => fmt(v, 2),
        None => "–".to_string(),
    };

    let prev_disabled = idx == 0;
    let next_disabled = idx == n - 1;
    let app_prev = app.clone();
    let app_next = app.clone();

    rsx! {
        div {
            id: "result",
            div {
                class: "card",
                div {
                    class: "nav",
                    button {
                        class: "ghost",
                        disabled: prev_disabled,
                        title: "Previous (←)",
                        onclick: move |_| {
                            let mut sig = app_prev.index.clone();
                            let v = *sig.read();
                            if v > 0 { *sig.write() = v - 1; }
                        },
                        "← Prev"
                    }
                    div {
                        class: "counter",
                        "Entry "
                        b { "{idx + 1}" }
                        " / "
                        b { "{n}" }
                        if idx == 0 {
                            span { class: "best-badge", "BEST" }
                        }
                    }
                    button {
                        class: "ghost",
                        disabled: next_disabled,
                        title: "Next (→)",
                        onclick: move |_| {
                            let mut sig = app_next.index.clone();
                            let v = *sig.read();
                            *sig.write() = v + 1;
                        },
                        "Next →"
                    }
                }

                div {
                    class: "stats",
                    div { div { class: "k", "Resources used" } div { class: "v", "{fmt(entry.resources_used, 2)}" } }
                    div { div { class: "k", "Committed" } div { class: "v", "{fmt(entry.resources_committed, 2)}" } }
                    div { div { class: "k", "Saved" } div { class: "v", "{fmt(saved, 2)}" } }
                    div { div { class: "k", "Fleet stats" } div { class: "v", "{stats_str}" } }
                    if has_boss {
                        {
                            let ratio = if entry.resources_used > 0.0 {
                                entry.boss_damage / entry.resources_used
                            } else { 0.0 };
                            let text = format!("{} ({:.3})", fmt(entry.boss_damage, 2), ratio);
                            rsx! {
                                div { div { class: "k", "Boss dmg / FR" } div { class: "v", "{text}" } }
                            }
                        }
                    }
                    div { div { class: "k", "Entry #" } div { class: "v", "#{orig_idx}" } }
                    div {
                        div { class: "k", "Event" }
                        div { class: "v", style: "font-size:0.8rem", "{event_id}" }
                    }
                }

                div {
                    class: "content",
                    div {
                        h2 { "Grid" }
                        ShipGrid { ships: entry.ships.clone(), event_id: event_id.clone() }
                    }
                    div {
                        h2 { "Ships" }
                        div {
                            class: "ships",
                            ShipList { ships: entry.ships.clone() }
                        }
                    }
                }
            }
        }
    }
}

fn fmt(n: f64, d: usize) -> String {
    if n.is_nan() {
        return "–".to_string();
    }
    // Approximate Number.toLocaleString({minimumFractionDigits: d}) with thousands separators.
    let fixed = format!("{:.*}", d, n.abs());
    let (int_part, frac_part) = match fixed.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f.to_string())),
        None => (fixed.clone(), None),
    };
    let mut grouped = String::new();
    for (i, ch) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let int_grouped: String = grouped.chars().rev().collect();
    let sign = if n < 0.0 { "-" } else { "" };
    match frac_part {
        Some(f) => format!("{sign}{int_grouped}.{f}"),
        None => format!("{sign}{int_grouped}"),
    }
}
