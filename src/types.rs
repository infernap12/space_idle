use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct FleetEntry {
    pub resources_used: f64,
    pub resources_committed: f64,
    pub fleet_stats: Option<f64>,
    pub boss_damage: f64,
    pub ships: Vec<FleetShip>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FleetShip {
    pub ship_type: String,
    pub mods: Vec<String>,
    pub positions: Vec<(i32, i32)>,
    pub count: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedEntry {
    pub orig_idx: usize,
    pub entry: FleetEntry,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FleetEvent {
    pub id: String,
    pub name: String,
    pub hazards: Vec<HazardEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HazardEntry {
    pub hazard_type: String,
    pub repeat: bool,
    pub nodes_effected: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HazardSource {
    pub source_id: String,
    pub source_name: String,
    pub hazard_type: String,
    pub repeat: bool,
    pub is_self: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DbData {
    pub rows: Vec<FleetEvent>,
    pub ship_sprites: HashMap<String, String>,
    pub mod_icons: HashMap<String, String>,
    pub rows_by_galaxy: HashMap<String, Vec<usize>>,
    pub hazards_by_target: HashMap<String, Vec<HazardSource>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipKind {
    pub name: &'static str,
    pub color: &'static str,
    pub abbr: &'static str,
}

pub const SHIP_TYPES: &[ShipKind] = &[
    ShipKind { name: "Fighter",           color: "#ffd23f", abbr: "Ft" },
    ShipKind { name: "Corvette",          color: "#4ecdc4", abbr: "Co" },
    ShipKind { name: "Frigate",           color: "#6bcB77", abbr: "Fr" },
    ShipKind { name: "Cruiser",           color: "#ff9f45", abbr: "Cr" },
    ShipKind { name: "HeavyCruiser",      color: "#ef476f", abbr: "HC" },
    ShipKind { name: "Destroyer",         color: "#b388ff", abbr: "Ds" },
    ShipKind { name: "PlayerCommandShip", color: "#ffffff", abbr: "CS" },
];

pub fn ship_kind(name: &str) -> ShipKind {
    SHIP_TYPES
        .iter()
        .copied()
        .find(|k| k.name == name)
        .unwrap_or(ShipKind { name: "?", color: "#888", abbr: "?" })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skin {
    pub key: &'static str,
    pub display_name: &'static str,
    pub folder: &'static str,
    pub show: bool,
}

pub const SKINS: &[Skin] = &[
    Skin { key: "default",      display_name: "Default",      folder: "",            show: true  },
    Skin { key: "blue",         display_name: "Blue",         folder: "blue",        show: true  },
    Skin { key: "green",        display_name: "Green",        folder: "green",       show: true  },
    Skin { key: "steampunk",    display_name: "Steampunk",    folder: "steampunk",   show: true  },
    Skin { key: "speed_racer",  display_name: "Speedster",    folder: "racer",       show: true  },
    Skin { key: "spacemas",     display_name: "Spacemas",     folder: "spacemas",    show: true  },
    Skin { key: "spaceversary", display_name: "Spaceversary", folder: "anniversary", show: true  },
    Skin { key: "retrofuture",  display_name: "Retro Future", folder: "retrofuture", show: true  },
    Skin { key: "advanced",     display_name: "Advanced",     folder: "advanced",    show: true  },
    Skin { key: "golden",       display_name: "Golden",       folder: "golden",      show: true  },
    Skin { key: "alien",        display_name: "Alien",        folder: "alien",       show: false },
    Skin { key: "haunted",      display_name: "Haunted",      folder: "haunted",     show: true  },
];

pub fn skin_folder(key: &str) -> &'static str {
    SKINS.iter().find(|s| s.key == key).map(|s| s.folder).unwrap_or("")
}

pub const WIDE_GRID_EVENTS: &[&str] = &[
    "g6_b46", "g7_b7", "g8_b17", "g9_b18", "g9_b19", "g10_b21", "g10_b22",
];

pub fn is_wide_grid(event_id: &str) -> bool {
    WIDE_GRID_EVENTS.contains(&event_id)
}
