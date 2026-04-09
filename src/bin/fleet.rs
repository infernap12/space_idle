use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

#[derive(Debug, Clone)]
struct ShipPlacement {
	ship_type: String,
	mods: Vec<String>,
	positions: Vec<(u8, u8)>, // (col, row)
	count: u32,
}

#[derive(Debug)]
struct FleetEntry {
	resources_used: f64,
	resources_committed: f64,
	fleet_stats: Option<f64>,
	ships: Vec<ShipPlacement>,
}

fn parse_fleet_string(s: &str) -> Option<FleetEntry> {
	let mut s = s.trim();

	// Strip V2 prefix if present
	if s.starts_with("V2") {
		s = &s[2..];
	}

	let parts: Vec<&str> = s.split('|').collect();
	if parts.len() < 2 {
		return None;
	}

	// Parse header: resources_used,resources_committed,fleet_stats
	let header: Vec<&str> = parts[0].split(',').collect();
	let resources_used: f64 = header.get(0)?.parse().ok()?;
	let resources_committed: f64 = header.get(1)?.parse().ok()?;
	let fleet_stats: Option<f64> = header.get(2).and_then(|s| s.parse().ok());

	// parts[1] is events, skip it
	// Remaining parts are ship entries (or boss_damage at the end)

	let mut ships = Vec::new();

	for part in parts.iter().skip(2) {
		if part.is_empty() {
			continue;
		}

		// Check if it's just a number (boss_damage)
		if !part.contains(';') {
			break;
		}

		let ship_parts: Vec<&str> = part.split(';').collect();
		if ship_parts.is_empty() || ship_parts[0] == "Null" {
			continue;
		}

		let ship_type = ship_parts[0].to_string();

		// Parse mods
		let mods: Vec<String> = if ship_parts.len() > 1 && !ship_parts[1].is_empty() {
			ship_parts[1]
				.split(',')
				.filter(|m| !m.is_empty())
				.map(|m| m.to_string())
				.collect()
		} else {
			Vec::new()
		};

		// Parse positions (col,row separated by *)
		let positions: Vec<(u8, u8)> = if ship_parts.len() > 2 {
			ship_parts[2]
				.split('*')
				.filter_map(|pos| {
					let coords: Vec<&str> = pos.split(',').collect();
					if coords.len() >= 2 {
						let col: f64 = coords[0].parse().ok()?;
						let row: f64 = coords[1].parse().ok()?;
						Some((col as u8, row as u8))
					} else {
						None
					}
				})
				.collect()
		} else {
			Vec::new()
		};

		// Parse count
		let count: u32 = if ship_parts.len() > 3 {
			ship_parts[3].parse().unwrap_or(positions.len() as u32)
		} else {
			positions.len() as u32
		};

		ships.push(ShipPlacement {
			ship_type,
			mods,
			positions,
			count,
		});
	}

	Some(FleetEntry {
		resources_used,
		resources_committed,
		fleet_stats,
		ships,
	})
}

fn ship_abbrev(ship_type: &str) -> &str {
	match ship_type {
		"Fighter" => ">",
		"Corvette" => "C",
		"Frigate" => "F",
		"Cruiser" => "R",
		"HeavyCruiser" => "H",
		"Destroyer" => "D",
		"PlayerCommandShip" => "@",
		_ => "?",
	}
}

fn render_grid(ships: &[ShipPlacement]) -> String {
	// Grid: 2 cols (A=0, B=1), 5 rows (0-4)
	let mut grid: HashMap<(u8, u8), char> = HashMap::new();

	for ship in ships {
		let abbrev = ship_abbrev(&ship.ship_type).chars().next().unwrap_or('?');
		for &(col, row) in &ship.positions {
			grid.insert((col, row), abbrev);
		}
	}

	let mut output = String::new();
	output.push_str("   B   A\n");

	for row in 0..5 {
		let b_cell = grid.get(&(1, row)).map_or(' ', |&c| c);
		let a_cell = grid.get(&(0, row)).map_or(' ', |&c| c);
		output.push_str(&format!("{} [{}] [{}]\n", row + 1, b_cell, a_cell));
	}

	output
}

fn format_mods(mods: &[String]) -> String {
	if mods.is_empty() {
		"none".to_string()
	} else {
		mods.join(", ")
	}
}

#[derive(Deserialize)]
struct Config {
	url: String,
	galaxy: u32,
	version: u64,
}

#[derive(serde::Serialize)]
struct FleetRequest {
	fleet_event_id: String,
	version: u64,
	combat_stat_level: f64,
	hazard_node_list: String,
	has_boss: bool,
}

fn prompt(msg: &str) -> String {
	print!("{}", msg);
	io::stdout().flush().unwrap();
	let mut input = String::new();
	io::stdin().read_line(&mut input).unwrap();
	input.trim().to_string()
}

fn find_fleet_event_id(csv_path: &str, galaxy: u32, node_name: &str) -> Option<String> {
	let contents = fs::read_to_string(csv_path).ok()?;
	let prefix = format!("g{}_", galaxy);
	let node_upper = node_name.to_uppercase();

	for line in contents.lines().skip(1) {
		// skip header
		let fields: Vec<&str> = line.split(',').collect();
		if fields.len() < 2 {
			continue;
		}

		let id = fields[0];
		let name = fields[1];

		if id.starts_with(&prefix) && name == node_upper {
			return Some(id.to_string());
		}
	}

	None
}

fn main() {
	// Read config
	let config_str = match fs::read_to_string("config.toml") {
		Ok(c) => c,
		Err(e) => {
			eprintln!("Error reading config.toml: {}", e);
			return;
		}
	};

	let config: Config = match toml::from_str(&config_str) {
		Ok(c) => c,
		Err(e) => {
			eprintln!("Error parsing config.toml: {}", e);
			return;
		}
	};

	// Prompt for node
	let node_name = prompt("Enter node: ");
	if node_name.is_empty() {
		eprintln!("Node name required");
		return;
	}

	// Prompt for stat level
	let stat_level_str = prompt("Enter stat level: ");
	let stat_level: f64 = match stat_level_str.parse() {
		Ok(v) => v,
		Err(_) => {
			eprintln!("Invalid stat level");
			return;
		}
	};

	// Check if boss battle (starts with "bb")
	let has_boss = if node_name.to_lowercase().starts_with("bb") {
		let boss_input = prompt("Boss (y)/n: ");
		boss_input.is_empty() || boss_input.to_lowercase().starts_with('y')
	} else {
		false
	};

	// Find fleet event id from CSV
	let fleet_event_id = match find_fleet_event_id("fleetdata.csv", config.galaxy, &node_name) {
		Some(id) => id,
		None => {
			eprintln!(
				"Could not find node '{}' in galaxy {}",
				node_name, config.galaxy
			);
			return;
		}
	};

	println!("\nFound: {}", fleet_event_id);

	// Build request
	let request = FleetRequest {
		fleet_event_id,
		version: config.version,
		combat_stat_level: stat_level,
		hazard_node_list: String::new(),
		has_boss,
	};

	let request_body = serde_json::to_string(&request).unwrap();
	println!("Sending: {}", request_body);

	// Send POST request
	let client = reqwest::blocking::Client::new();
	let response = match client
		.post(&config.url)
		.header("Content-Type", "application/json")
		.body(request_body)
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			eprintln!("Request failed: {}", e);
			return;
		}
	};

	if !response.status().is_success() {
		eprintln!("Server returned error: {}", response.status());
		return;
	}

	let response_text = match response.text() {
		Ok(t) => t,
		Err(e) => {
			eprintln!("Failed to read response: {}", e);
			return;
		}
	};

	// Parse response as JSON array of strings
	let lines: Vec<String> = match serde_json::from_str(&response_text) {
		Ok(v) => v,
		Err(e) => {
			eprintln!("Error parsing response JSON: {}", e);
			eprintln!("Response was: {}", response_text);
			return;
		}
	};

	// Parse all entries first
	let mut entries: Vec<(usize, FleetEntry)> = lines
		.iter()
		.enumerate()
		.filter_map(|(i, line)| {
			let line = line.trim();
			if line.is_empty() {
				return None;
			}
			parse_fleet_string(line).map(|entry| (i + 1, entry))
		})
		.collect();

	// Sort by resources_used descending
	entries.sort_by(|a, b| b.1.resources_used.partial_cmp(&a.1.resources_used).unwrap());

	for (orig_idx, entry) in entries {
		let saved = entry.resources_committed - entry.resources_used;
		println!("═══ Entry {} ═══", orig_idx);
		println!(
			"Resources: {:.2} / {:.2} (saved: {:.2})",
			entry.resources_used, entry.resources_committed, saved
		);

		if let Some(stats) = entry.fleet_stats {
			println!("Fleet stats: {:.2}", stats);
		}

		println!("\nShips:");
		for ship in &entry.ships {
			println!(
				"  {} x{} [mods: {}]",
				ship.ship_type,
				ship.count,
				format_mods(&ship.mods)
			);
			let pos_str: Vec<String> = ship
				.positions
				.iter()
				.map(|(c, r)| format!("({},{})", c, r))
				.collect();
			println!("    positions: {}", pos_str.join(", "));
		}

		println!("\n{}", render_grid(&entry.ships));
	}
}
