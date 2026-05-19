use serde::Serialize;

pub const API_URL: &str = "https://api.spaceidle.xyz/suggest_fleet_battle/";
const HARD_CAP: usize = 6;

#[derive(Serialize)]
struct RequestBody<'a> {
    fleet_event_id: &'a str,
    version: i64,
    combat_stat_level: f64,
    has_boss: bool,
    hazard_node_list: String,
}

pub struct ApiResult {
    pub lines: Vec<String>,
    pub attempt: usize,
    pub total: usize,
}

pub async fn fetch_suggestion<F>(
    fleet_event_id: &str,
    version: i64,
    combat_stat_level: f64,
    has_boss: bool,
    source_ids: Vec<String>,
    mut on_progress: F,
) -> Result<ApiResult, String>
where
    F: FnMut(usize, usize),
{
    let orderings: Vec<Vec<String>> = if source_ids.len() > HARD_CAP {
        vec![source_ids.clone()]
    } else {
        hazard_orderings(&source_ids)
    };
    let total = orderings.len();
    let mut last_status = 0u16;

    for (i, order) in orderings.iter().enumerate() {
        if total > 1 {
            on_progress(i + 1, total);
        }
        let body = RequestBody {
            fleet_event_id,
            version,
            combat_stat_level,
            has_boss,
            hazard_node_list: order.join(""),
        };
        let response = gloo_net::http::Request::post(API_URL)
            .header("Content-Type", "application/json")
            .json(&body)
            .map_err(|e| format!("encode body: {e}"))?
            .send()
            .await
            .map_err(|e| format!("send: {e}"))?;

        if response.ok() {
            let lines: Vec<String> = response
                .json()
                .await
                .map_err(|e| format!("decode body: {e}"))?;
            return Ok(ApiResult { lines, attempt: i + 1, total });
        }
        last_status = response.status();
        if response.status() != 404 {
            return Err(format!("Server returned {}", response.status()));
        }
    }

    let _ = last_status;
    if source_ids.len() > HARD_CAP {
        Err(format!(
            "Server returned 404 (and {} hazards > {HARD_CAP}, so permutations weren't tried)",
            source_ids.len()
        ))
    } else {
        Err(format!(
            "Server returned 404 for all {total} hazard order{}",
            if total == 1 { "" } else { "ings" }
        ))
    }
}

fn hazard_orderings(ids: &[String]) -> Vec<Vec<String>> {
    if ids.len() <= 1 {
        return vec![ids.to_vec()];
    }
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let seed: Vec<String> = ids.to_vec();
    seen.insert(seed.join(""));
    out.push(seed);

    let mut prefix: Vec<String> = Vec::new();
    let rest: Vec<String> = ids.to_vec();
    permute(&mut prefix, rest, &mut seen, &mut out);
    out
}

fn permute(
    prefix: &mut Vec<String>,
    rest: Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<Vec<String>>,
) {
    if rest.is_empty() {
        let key = prefix.join("");
        if !seen.contains(&key) {
            seen.insert(key);
            out.push(prefix.clone());
        }
        return;
    }
    for i in 0..rest.len() {
        prefix.push(rest[i].clone());
        let mut next = rest.clone();
        next.remove(i);
        permute(prefix, next, seen, out);
        prefix.pop();
    }
}
