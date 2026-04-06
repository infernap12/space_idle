use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::sync::Mutex;
use std::time::Instant;
const BASE3_LAYOUT: BaseLayout = BaseLayout {
	height: 5,
	width: 5,
	blocked: &[(0, 1), (0, 3), (2, 2), (4, 0), (4, 4)],
	boost_range: 1,
	swapped_shapes: false,
};

const BASE4_LAYOUT: BaseLayout = BaseLayout {
	height: 5,
	width: 6,
	blocked: &[
		(0, 4),
		(1, 0),
		(1, 5),
		(2, 1),
		(2, 4),
		(3, 0),
		(3, 5),
		(4, 1),
	],
	boost_range: 2,
	swapped_shapes: false,
};

const BASE5_LAYOUT: BaseLayout = BaseLayout {
	height: 5,
	width: 6,
	blocked: &[
		(0, 3),
		(1, 3),
		(1, 4),
		(2, 2),
		(2, 3),
		(3, 1),
		(3, 2),
		(4, 2),
	],
	boost_range: 1,
	swapped_shapes: true,
};

// === CONFIGURATION ===

// What to optimise. Materials is a pure binary search (mats vs booster).
// Parts/Components fix producer tile counts, then brute-force booster vs drain placement.
const SEARCH: SearchType = SearchType::Materials;
// const SEARCH: SearchType = SearchType::Parts(PartSearchParams {
// 	mat_tiles: 3,
// 	parts_tiles: 3,
// });
const BASE: Base = Base::Base5;
// const SEARCH: SearchType = SearchType::Components(ComponentSearchParams {
// 	mat_tiles: 3,
// 	parts_tiles: 1,
// 	comp_tiles: 1,
// });

// Base rates per tile before boost/drain. Only ratios matter for layout ranking,
// but using real values lets you see actual net rates in the output.
//BASE 4
const MAT_PRODUCTION: f64 = 4.34e17;
const PARTS_PRODUCTION: f64 = 9.24e20;
const PARTS_MAT_DRAIN: f64 = 2.77e21;
const COMP_PRODUCTION: f64 = 2.36e22;
const COMP_MAT_DRAIN: f64 = 1.4e23;
const COMP_PARTS_DRAIN: f64 = 1.4e23;

const BOOSTER_MULT: f64 = 3.59;
const DRAIN_REDUCE_MULT: f64 = 257.0;
// // non-components buildings mats, parts mult
const BASE_BOOST: f64 = 5.0;

// BASE 3
// const MAT_PRODUCTION: f64 = 7.79e37;
// const PARTS_PRODUCTION: f64 = 2.74e44;
// const PARTS_MAT_DRAIN: f64 = 2.74e45;
// const COMP_PRODUCTION: f64 = 1.95e50;
// const COMP_MAT_DRAIN: f64 = 7.80e50;
// const COMP_PARTS_DRAIN: f64 = 0.0;
// Multipliers (from in-game upgrade levels)
// const BOOSTER_MULT: f64 = 847.45;
// const DRAIN_REDUCE_MULT: f64 = 6.55e4;
// const BASE_BOOST: f64 = 70.0;

// BASE 5
// const MAT_PRODUCTION: f64 = 3.81;
// const PARTS_PRODUCTION: f64 = 3.71;
// const PARTS_MAT_DRAIN: f64 = 11.14;
// const COMP_PRODUCTION: f64 = 0.85;
// const COMP_MAT_DRAIN: f64 = 5.07;
// const COMP_PARTS_DRAIN: f64 = 5.07;

// const BOOSTER_MULT: f64 = 1.8125;
// const DRAIN_REDUCE_MULT: f64 = 1.5;
// // // Base specific Production multiplier
// const BASE_BOOST: f64 = 1.0;

const FINAL_MATS_RATE: f64 = MAT_PRODUCTION * PRODUCTION_MULT * NONCOMP_MULT;
const FINAL_PARTS_RATE: f64 = PARTS_PRODUCTION * PRODUCTION_MULT * NONCOMP_MULT;
const FINAL_PARTS_MAT_DRAIN: f64 = PARTS_MAT_DRAIN * PRODUCTION_MULT * NONCOMP_MULT;
const FINAL_COMP_RATE: f64 = COMP_PRODUCTION * PRODUCTION_MULT * COMPONENT_MULT;
const FINAL_COMP_MAT_DRAIN: f64 = COMP_MAT_DRAIN * PRODUCTION_MULT * COMPONENT_MULT;
const FINAL_COMP_PARTS_DRAIN: f64 = COMP_PARTS_DRAIN * PRODUCTION_MULT * COMPONENT_MULT;

// all types mult
const PRODUCTION_MULT: f64 = 1.88e10 * BASE_BOOST;
// non-components buildings mats, parts mult
const NONCOMP_MULT: f64 = 113.11 * 8.0485;
const COMPONENT_MULT: f64 = 229.46;
// === TYPES ===

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tile {
	Empty,
	Mats,
	Parts,
	Components,
	Booster,
	DrainReduce,
}

impl Tile {
	fn symbol(self) -> &'static str {
		match self {
			Tile::Empty => "   ",
			Tile::Mats => " B ",
			Tile::Parts => " P ",
			Tile::Components => " C ",
			Tile::Booster => " + ",
			Tile::DrainReduce => " X ",
		}
	}
}

enum SearchType {
	Materials,
	Parts(PartSearchParams),
	Components(ComponentSearchParams),
}

struct PartSearchParams {
	mat_tiles: usize,
	parts_tiles: usize,
}

struct ComponentSearchParams {
	mat_tiles: usize,
	parts_tiles: usize,
	comp_tiles: usize,
}

// === BEST TRACKING ===
// Stores (objective_score, net_mats). Picks higher objective first, then higher net_mats as tiebreaker.

type Best = Mutex<(f64, f64)>;

fn try_update_best(best: &Best, obj: f64, mats: f64) -> bool {
	let mut guard = best.lock().unwrap();
	if obj > guard.0 || (obj == guard.0 && mats > guard.1) {
		*guard = (obj, mats);
		true
	} else {
		false
	}
}

// === BOARD ===

struct Board {
	base: Base,
	usable: Vec<(usize, usize)>,
	/// boost_adj[i] = usable indices within plus-shape range 2 of usable[i]
	boost_adj: Vec<Vec<usize>>,
	/// drain_adj[i] = usable indices within X-shape (diagonal dist 1) of usable[i]
	drain_adj: Vec<Vec<usize>>,
}

impl Board {
	fn new(base: Base) -> Self {
		let layout = base.layout();
		let h = layout.height;
		let w = layout.width;
		let blocked = layout.blocked;

		let mut usable = Vec::new();
		for r in 0..h {
			for c in 0..w {
				if !blocked.contains(&(r, c)) {
					usable.push((r, c));
				}
			}
		}

		let n = usable.len();
		let mut boost_adj = vec![Vec::new(); n];
		let mut drain_adj = vec![Vec::new(); n];

		for i in 0..n {
			let (row_i, col_i) = usable[i];
			for j in 0..n {
				if i == j {
					continue;
				}
				let (row_j, col_j) = usable[j];
				let row_delta = row_j as i32 - row_i as i32;
				let col_delta = col_j as i32 - col_i as i32;

				let range = layout.boost_range;
				let is_plus = (row_delta == 0) ^ (col_delta == 0)
					&& row_delta.abs() <= range
					&& col_delta.abs() <= range;
				let is_x = row_delta.abs() == 1 && col_delta.abs() == 1;

				if layout.swapped_shapes {
					// Base5: boosters use X-shape, drains use +-shape
					if is_x {
						boost_adj[i].push(j);
					}
					if is_plus {
						drain_adj[i].push(j);
					}
				} else {
					// Default: boosters use +-shape, drains use X-shape
					if is_plus {
						boost_adj[i].push(j);
					}
					if is_x {
						drain_adj[i].push(j);
					}
				}
			}
		}

		Board {
			base,
			usable,
			boost_adj,
			drain_adj,
		}
	}

	fn n(&self) -> usize {
		self.usable.len()
	}

	fn print_layout(&self, tiles: &[Tile], rates: (f64, f64, f64), label: &str) {
		let layout = self.base.layout();
		let (mats, parts, comps) = rates;
		let objective = match label {
			"Parts/s" => parts,
			"Components/s" => comps,
			_ => mats,
		};
		let mut buf = ryu::Buffer::new();
		println!("Score: {} {}", buf.format(objective), label);
		println!(
			"  net mats={:.3e}  net parts={:.3e}  net comps={:.3e}",
			mats, parts, comps
		);
		let mut grid = vec![vec![Tile::Empty; layout.width]; layout.height];
		for (idx, &(r, c)) in self.usable.iter().enumerate() {
			grid[r][c] = tiles[idx];
		}
		for r in 0..layout.height {
			for c in 0..layout.width {
				print!("{}", grid[r][c].symbol());
			}
			println!();
		}
		println!();
	}
}

// === SCORING ===

/// Returns (net_mats/s, net_parts/s, net_components/s)
fn score(tiles: &[Tile], board: &Board) -> (f64, f64, f64) {
	let mut net_mats = 0.0;
	let mut net_parts = 0.0;
	let mut net_comps = 0.0;

	for i in 0..tiles.len() {
		match tiles[i] {
			Tile::Mats | Tile::Parts | Tile::Components => {
				let bf = board.boost_adj[i]
					.iter()
					.filter(|&&j| tiles[j] == Tile::Booster)
					.fold(1.0, |acc, _| acc * BOOSTER_MULT);

				let df = board.drain_adj[i]
					.iter()
					.filter(|&&j| tiles[j] == Tile::DrainReduce)
					.fold(1.0, |acc, _| acc * DRAIN_REDUCE_MULT);

				match tiles[i] {
					Tile::Mats => {
						net_mats += FINAL_MATS_RATE * bf;
					}
					Tile::Parts => {
						net_parts += FINAL_PARTS_RATE * bf;
						net_mats -= FINAL_PARTS_MAT_DRAIN * bf / df;
					}
					Tile::Components => {
						net_comps += FINAL_COMP_RATE * bf;
						net_mats -= FINAL_COMP_MAT_DRAIN * bf / df;
						net_parts -= FINAL_COMP_PARTS_DRAIN * bf / df;
					}
					_ => unreachable!(),
				}
			}
			_ => {}
		}
	}

	(net_mats, net_parts, net_comps)
}

// === SEARCH: MATERIALS ===
// Binary: each usable tile is either Mats or Booster. No drain (nothing consumes).
// Iterates over all booster counts, finds best layout for each.

fn search_materials(board: &Board, pb: &ProgressBar) {
	let n = board.n();
	let best = Mutex::new((f64::NEG_INFINITY, f64::NEG_INFINITY));

	(0..=n).into_par_iter().for_each(|booster_count| {
		let mut tiles = vec![Tile::Mats; n];
		mat_dfs(&mut tiles, board, booster_count, 0, 0, &best, pb);
	});
}

fn mat_dfs(
	tiles: &mut Vec<Tile>,
	board: &Board,
	target: usize,
	placed: usize,
	start: usize,
	best: &Best,
	pb: &ProgressBar,
) {
	if placed == target {
		let rates = score(tiles, board);
		if try_update_best(best, rates.0, rates.0) {
			pb.suspend(|| board.print_layout(tiles, rates, "Mats/s"));
		}
		pb.inc(1);
		return;
	}

	let n = tiles.len();
	for i in start..=(n - (target - placed)) {
		tiles[i] = Tile::Booster;
		mat_dfs(tiles, board, target, placed + 1, i + 1, best, pb);
		tiles[i] = Tile::Mats;
	}
}

// === SEARCH: PARTS / COMPONENTS ===
// Fix producer tile counts, enumerate their positions combinatorially,
// then bitmask all 2^R assignments of Booster vs DrainReduce for remaining R slots.
// The first producer phase is parallelized across threads.

fn search_parts(board: &Board, params: &PartSearchParams, pb: &ProgressBar) {
	let n = board.n();
	let best = Mutex::new((f64::NEG_INFINITY, f64::NEG_INFINITY));
	let producers = [
		(Tile::Mats, params.mat_tiles),
		(Tile::Parts, params.parts_tiles),
	];

	let combos = combinations(n, producers[0].1);
	combos.into_par_iter().for_each(|combo| {
		let mut tiles = vec![Tile::Booster; n];
		for &idx in &combo {
			tiles[idx] = producers[0].0;
		}
		place_producers(
			&mut tiles,
			board,
			&producers,
			1,
			0,
			Objective::Parts,
			&best,
			pb,
		);
	});
}

fn search_components(board: &Board, params: &ComponentSearchParams, pb: &ProgressBar) {
	let n = board.n();
	let best = Mutex::new((f64::NEG_INFINITY, f64::NEG_INFINITY));
	let producers = [
		(Tile::Mats, params.mat_tiles),
		(Tile::Parts, params.parts_tiles),
		(Tile::Components, params.comp_tiles),
	];

	let combos = combinations(n, producers[0].1);
	combos.into_par_iter().for_each(|combo| {
		let mut tiles = vec![Tile::Booster; n];
		for &idx in &combo {
			tiles[idx] = producers[0].0;
		}
		place_producers(
			&mut tiles,
			board,
			&producers,
			1,
			0,
			Objective::Components,
			&best,
			pb,
		);
	});
}

#[derive(Clone, Copy)]
enum Objective {
	Parts,
	Components,
}

/// Recursively place each producer type (combination enumeration),
/// then hand off to bitmask enumeration for the remaining Booster/Drain slots.
fn place_producers(
	tiles: &mut Vec<Tile>,
	board: &Board,
	producers: &[(Tile, usize)],
	phase: usize,
	start: usize,
	obj: Objective,
	best: &Best,
	pb: &ProgressBar,
) {
	if phase == producers.len() {
		enumerate_booster_drain(tiles, board, obj, best, pb);
		return;
	}

	let (tile_type, count) = producers[phase];
	place_combo(
		tiles, board, producers, phase, tile_type, count, 0, start, obj, best, pb,
	);
}

fn place_combo(
	tiles: &mut Vec<Tile>,
	board: &Board,
	producers: &[(Tile, usize)],
	phase: usize,
	tile_type: Tile,
	target: usize,
	placed: usize,
	start: usize,
	obj: Objective,
	best: &Best,
	pb: &ProgressBar,
) {
	if placed == target {
		place_producers(tiles, board, producers, phase + 1, 0, obj, best, pb);
		return;
	}

	let n = tiles.len();
	for i in start..n {
		if tiles[i] == Tile::Booster {
			tiles[i] = tile_type;
			place_combo(
				tiles,
				board,
				producers,
				phase,
				tile_type,
				target,
				placed + 1,
				i + 1,
				obj,
				best,
				pb,
			);
			tiles[i] = Tile::Booster;
		}
	}
}

fn enumerate_booster_drain(
	tiles: &mut Vec<Tile>,
	board: &Board,
	obj: Objective,
	best: &Best,
	pb: &ProgressBar,
) {
	let remaining: Vec<usize> = (0..tiles.len())
		.filter(|&i| tiles[i] == Tile::Booster)
		.collect();
	let r = remaining.len();

	let label = match obj {
		Objective::Parts => "Parts/s",
		Objective::Components => "Components/s",
	};

	for mask in 0u64..(1u64 << r) {
		for (bit, &idx) in remaining.iter().enumerate() {
			tiles[idx] = if mask & (1 << bit) != 0 {
				Tile::DrainReduce
			} else {
				Tile::Booster
			};
		}

		let rates = score(tiles, board);
		let (mats, parts, comps) = rates;
		let s = match obj {
			Objective::Parts => {
				if mats < 0.0 {
					pb.inc(1);
					continue;
				}
				parts
			}
			Objective::Components => {
				if mats < 0.0 || parts < 0.0 {
					pb.inc(1);
					continue;
				}
				comps
			}
		};

		if try_update_best(best, s, mats) {
			pb.suspend(|| board.print_layout(tiles, rates, label));
		}
		pb.inc(1);
	}

	// Restore
	for &idx in &remaining {
		tiles[idx] = Tile::Booster;
	}
}

// === COMBINATORICS / PROGRESS ===

/// Collect all C(n, k) combinations of indices 0..n
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
	let mut result = Vec::new();
	let mut current = Vec::with_capacity(k);
	combinations_helper(n, k, 0, &mut current, &mut result);
	result
}

fn combinations_helper(
	n: usize,
	k: usize,
	start: usize,
	current: &mut Vec<usize>,
	result: &mut Vec<Vec<usize>>,
) {
	if current.len() == k {
		result.push(current.clone());
		return;
	}
	for i in start..n {
		current.push(i);
		combinations_helper(n, k, i + 1, current, result);
		current.pop();
	}
}

fn comb(n: u64, k: u64) -> u64 {
	if k > n {
		return 0;
	}
	let k = k.min(n - k);
	(0..k).fold(1u64, |acc, i| acc * (n - i) / (i + 1))
}

fn total_evaluations(n: usize) -> u64 {
	let n = n as u64;
	match SEARCH {
		SearchType::Materials => 1u64 << n,
		SearchType::Parts(ref p) => {
			let m = p.mat_tiles as u64;
			let pt = p.parts_tiles as u64;
			comb(n, m) * comb(n - m, pt) * (1u64 << (n - m - pt))
		}
		SearchType::Components(ref p) => {
			let m = p.mat_tiles as u64;
			let pt = p.parts_tiles as u64;
			let ct = p.comp_tiles as u64;
			comb(n, m) * comb(n - m, pt) * comb(n - m - pt, ct) * (1u64 << (n - m - pt - ct))
		}
	}
}

fn make_progress_bar(total: u64) -> ProgressBar {
	let pb = ProgressBar::new(total);
	pb.set_style(
		ProgressStyle::with_template(
			"{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) ETA: {eta}",
		)
		.unwrap()
		.progress_chars("=>-"),
	);
	pb.enable_steady_tick(std::time::Duration::from_millis(250));
	pb
}

// === MAIN ===

fn main() {
	let board = Board::new(BASE);
	println!("Usable tiles: {}", board.n());

	let total = total_evaluations(board.n());
	println!("Search space: {} evaluations", total);
	let pb = make_progress_bar(total);

	let start = Instant::now();

	match SEARCH {
		SearchType::Materials => search_materials(&board, &pb),
		SearchType::Parts(ref p) => search_parts(&board, p, &pb),
		SearchType::Components(ref p) => search_components(&board, p, &pb),
	}

	pb.finish_with_message("done");
	eprintln!("Elapsed: {:?}", start.elapsed());
}

#[derive(Clone, Copy)]
pub enum Base {
	Base3,
	Base4,
	Base5,
}

impl Base {
	pub const fn layout(self) -> &'static BaseLayout {
		match self {
			Base::Base3 => &BASE3_LAYOUT,
			Base::Base4 => &BASE4_LAYOUT,
			Base::Base5 => &BASE5_LAYOUT,
		}
	}
}

#[derive(Copy, Clone)]
pub struct BaseLayout {
	pub height: usize,
	pub width: usize,
	pub blocked: &'static [(usize, usize)],
	pub boost_range: i32,
	/// When true, boosters use X-shape (diagonal) and drains use +-shape (cardinal).
	/// Default (false): boosters use +-shape and drains use X-shape.
	pub swapped_shapes: bool,
}
