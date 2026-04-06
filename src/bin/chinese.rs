// Taken from some Chinese guy on steam guides for USI

// Warning: Some `i as usize` is hidden by steam, I cannot recover it.
#![feature(let_chains)]
use std::array;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const W: usize = 6;
const H: usize = 5;
fn main() {
	// init map
	let map = blank_map();

	// map[0][2]=0;
	// map[2][2]=0;
	// map[2][3]=0;
	// map[4][3]=0;

	// init relation
	let mut rel: [[Vec<(i32, i32)>; W]; H] = array::from_fn(|_| array::from_fn(|_| Vec::new()));
	for i in 0..H as i32 {
		for j in 0..W as i32 {
			for k in -2..=2 {
				for l in -2..=2 {
					if ((k != 0) ^ (l != 0))
						&& let Some(1) = map
							.get((i + k) as usize)
							.map(|x| x.get((j + l) as usize))
							.flatten()
					{
						rel[i as usize][j as usize].push((i + k, j + l));
					}
				}
			}
		}
	}
	println!("{:?}", rel);
	let me_state = Arc::new(Mutex::new((0f64, vec![])));
	let mul = 2.46;
	let mut best = 0f64;
	let start = Instant::now();
	for s in 1..36 {
		for i in 0..H as i32 {
			for j in 0..W as i32 {
				if map[i as usize][j as usize] == 1 {
					best = best.max(dfs(
						&map,
						&rel,
						&mut vec![(i, j)],
						s,
						best,
						mul,
						me_state.clone(),
					))
				}
			}
		}
	}
	println!("Elapsed time: {:?}", start.elapsed());
	println!("Best score: {}", best);
	println!("Board:");
	print_board(me_state.lock().unwrap().1.clone())
}

fn blank_map() -> [[i32; 6]; 5] {
	let mut map = [[1; W]; H];
	map[0][4] = 0;
	map[1][0] = 0;
	map[1][5] = 0;
	map[2][1] = 0;
	map[2][4] = 0;
	map[3][0] = 0;
	map[3][5] = 0;
	map[4][1] = 0;
	map
}

fn dfs(
	map: &[[i32; W]; H],
	rel: &[[Vec<(i32, i32)>; W]; H],
	state: &mut Vec<(i32, i32)>,
	len: i32,
	mut score_pre: f64,
	scoremul: f64,
	me_state: Arc<Mutex<(f64, Vec<(i32, i32)>)>>,
) -> f64 {
	if state.len() as i32 == len {
		let mut score = map.clone().map(|x| x.map(|y| y as f64));
		state
			.iter()
			.for_each(|&(i, j)| score[i as usize][j as usize] = 0.);
		for &(i, j) in state.iter() {
			for &(k, l) in rel[i as usize][j as usize].iter() {
				score[k as usize][l as usize] *= scoremul;
			}
		}
		let res = score.iter().flatten().sum::<f64>();
		if res > score_pre {
			// println!("len {} has better score {} ", state.len(), res);
			let mut guard = me_state.lock().unwrap();
			guard.0 = res;
			guard.1 = state.clone();

			// let mut m = map.clone();
			// state
			// 	.iter()
			// 	.for_each(|&(i, j)| m[i as usize][j as usize] = 2);
			// for i in m {
			// 	for j in i {
			// 		if j == 0 {
			// 			print!("   ")
			// 		} else if j == 1 {
			// 			print!(" . ")
			// 		} else {
			// 			print!(" ! ")
			// 		}
			// 	}
			// 	println!("");
			// }
		}
		res
	} else {
		let (mut i, mut j) = *state.last().unwrap();
		loop {
			j += 1;
			if j == W as i32 {
				i += 1;
				j = 0;
			}
			if i < H as i32 {
				if map[i as usize][j as usize] == 1 {
					state.push((i, j));
					let res = dfs(map, rel, state, len, score_pre, scoremul, me_state.clone());
					state.pop();
					if res > score_pre {
						score_pre = res
					}
				} else {
					continue;
				}
			} else {
				break score_pre;
			}
		}
	}
}

fn print_board(state: Vec<(i32, i32)>) {
	let mut m = blank_map();
	state
		.iter()
		.for_each(|&(i, j)| m[i as usize][j as usize] = 2);
	for i in m {
		for j in i {
			if j == 0 {
				print!("   ")
			} else if j == 1 {
				print!(" . ")
			} else {
				print!(" ! ")
			}
		}
		println!();
	}
}

enum SearchType {
	Materials,
	PartSearch(PartSearchParams),
	ComponentSearch(ComponentSearchParams),
}

struct PartSearchParams {
	mat_tiles: usize,
	parts_tiles: usize,
}

impl From<(usize, usize)> for PartSearchParams {
	fn from((i, j): (usize, usize)) -> Self {
		PartSearchParams {
			mat_tiles: i,
			parts_tiles: j,
		}
	}
}

struct ComponentSearchParams {
	mat_tiles: usize,
	parts_tiles: usize,
	component_tiles: usize,
}
impl From<(usize, usize, usize)> for ComponentSearchParams {
	fn from((i, j, k): (usize, usize, usize)) -> Self {
		ComponentSearchParams {
			mat_tiles: i,
			parts_tiles: j,
			component_tiles: k,
		}
	}
}
