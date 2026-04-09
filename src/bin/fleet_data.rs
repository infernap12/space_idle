use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::io::Write;

#[derive(Deserialize)]
struct Column {
	name: String,
}

#[derive(Deserialize)]
struct Data {
	columns: Vec<Column>,
	lines: Vec<serde_json::Map<String, Value>>,
}

fn escape_csv(s: &str) -> String {
	if s.contains(',') || s.contains('"') || s.contains('\n') {
		format!("\"{}\"", s.replace('"', "\"\""))
	} else {
		s.to_string()
	}
}

fn value_to_string(v: &Value) -> String {
	match v {
		Value::Null => String::new(),
		Value::String(s) => s.clone(),
		Value::Bool(b) => b.to_string(),
		Value::Number(n) => n.to_string(),
		_ => v.to_string(),
	}
}

fn main() {
	let input = fs::read_to_string("fleetdata.json").expect("failed to read fleetdata.json");
	let data: Data = serde_json::from_str(&input).expect("invalid json");

	let mut out = fs::File::create("fleetdata.csv").expect("failed to create fleetdata.csv");

	let header: Vec<_> = data.columns.iter().map(|c| escape_csv(&c.name)).collect();
	writeln!(out, "{}", header.join(",")).unwrap();

	for line in &data.lines {
		let row: Vec<_> = data
			.columns
			.iter()
			.map(|c| {
				let val = line.get(&c.name).unwrap_or(&Value::Null);
				escape_csv(&value_to_string(val))
			})
			.collect();
		writeln!(out, "{}", row.join(",")).unwrap();
	}
}
