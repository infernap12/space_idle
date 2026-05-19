use crate::types::{ship_kind, skin_folder};

pub fn apply_skin_to_path(base_path: &str, skin: &str) -> String {
    let folder = skin_folder(skin);
    if base_path.is_empty() || folder.is_empty() {
        return base_path.to_string();
    }
    match base_path.rfind('/') {
        None => format!("{folder}/{base_path}"),
        Some(idx) => {
            let (dir, file) = base_path.split_at(idx + 1);
            format!("{dir}{folder}/{file}")
        }
    }
}

pub fn ship_svg(ty: &str) -> String {
    let k = ship_kind(ty);
    let c = k.color;
    let body = match ty {
        "Fighter" => format!(
            "<polygon points=\"50,15 65,70 50,60 35,70\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>"
        ),
        "Corvette" => format!(
            "<polygon points=\"50,10 70,55 60,75 40,75 30,55\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>"
        ),
        "Frigate" => format!(
            "<polygon points=\"50,8 72,50 62,80 38,80 28,50\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>\
             <rect x=\"42\" y=\"55\" width=\"16\" height=\"25\" fill=\"#000\" opacity=\"0.3\"/>"
        ),
        "Cruiser" => format!(
            "<polygon points=\"50,8 78,35 78,70 50,88 22,70 22,35\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>\
             <circle cx=\"50\" cy=\"50\" r=\"10\" fill=\"#000\" opacity=\"0.35\"/>"
        ),
        "HeavyCruiser" => format!(
            "<polygon points=\"50,6 82,30 85,72 50,92 15,72 18,30\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>\
             <rect x=\"35\" y=\"35\" width=\"30\" height=\"30\" fill=\"#000\" opacity=\"0.35\"/>\
             <rect x=\"42\" y=\"20\" width=\"16\" height=\"10\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"1.5\"/>"
        ),
        "Destroyer" => format!(
            "<polygon points=\"50,8 85,45 75,85 50,75 25,85 15,45\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>\
             <polygon points=\"50,30 62,55 50,60 38,55\" fill=\"#000\" opacity=\"0.4\"/>"
        ),
        "PlayerCommandShip" => format!(
            "<polygon points=\"50,5 60,38 92,38 66,58 76,92 50,72 24,92 34,58 8,38 40,38\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>"
        ),
        _ => format!(
            "<rect x=\"20\" y=\"20\" width=\"60\" height=\"60\" fill=\"{c}\" stroke=\"#000\" stroke-width=\"2\"/>"
        ),
    };
    format!(
        "<svg viewBox=\"0 0 100 100\" xmlns=\"http://www.w3.org/2000/svg\" aria-label=\"{ty}\">{body}</svg>"
    )
}

pub fn ship_svg_data_url(ty: &str) -> String {
    let svg = ship_svg(ty);
    let mut out = String::from("data:image/svg+xml;utf8,");
    for ch in svg.chars() {
        match ch {
            '#' => out.push_str("%23"),
            '"' => out.push_str("%22"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '\n' | '\r' | '\t' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}
