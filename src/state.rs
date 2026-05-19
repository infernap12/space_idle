use dioxus::prelude::*;
use std::collections::HashMap;

use crate::types::{DbData, ParsedEntry};

#[derive(Clone, Copy)]
pub struct AppState {
    pub node: Signal<String>,
    pub stat: Signal<String>,
    pub galaxy: Signal<u32>,
    pub version: Signal<i64>,
    pub has_boss: Signal<bool>,
    pub skin: Signal<String>,

    pub selected_hazards: Signal<Vec<String>>,
    pub inv_filter: Signal<bool>,
    pub inventory: Signal<HashMap<String, i32>>,

    pub db_loaded: Signal<Option<DbData>>,
    pub db_sprites: Signal<HashMap<String, String>>,
    pub db_mod_icons: Signal<HashMap<String, String>>,

    pub all_entries: Signal<Vec<ParsedEntry>>,
    pub entries: Signal<Vec<ParsedEntry>>,
    pub index: Signal<usize>,
    pub event_id: Signal<String>,

    pub status: Signal<String>,
    pub is_error: Signal<bool>,
    pub working: Signal<bool>,
}

pub fn new_state() -> AppState {
    AppState {
        node: Signal::new(String::new()),
        stat: Signal::new(String::new()),
        galaxy: Signal::new(4),
        version: Signal::new(80104),
        has_boss: Signal::new(false),
        skin: Signal::new("default".to_string()),
        selected_hazards: Signal::new(Vec::new()),
        inv_filter: Signal::new(false),
        inventory: Signal::new(HashMap::new()),
        db_loaded: Signal::new(None),
        db_sprites: Signal::new(HashMap::new()),
        db_mod_icons: Signal::new(HashMap::new()),
        all_entries: Signal::new(Vec::new()),
        entries: Signal::new(Vec::new()),
        index: Signal::new(0),
        event_id: Signal::new(String::new()),
        status: Signal::new("Enter a node and stat level, then Suggest.".to_string()),
        is_error: Signal::new(false),
        working: Signal::new(false),
    }
}
