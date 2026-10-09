// main.rs
use engine::board::{Position, START_FEN};
use engine::perft::divide;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("perft") {
        println!("usage: engine-uci perft <depth> [fen]");
        return;
    }
    let depth: u32 = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(5).max(1);
    let fen = args.get(3).map(String::as_str).unwrap_or(START_FEN);
    let pos = Position::from_fen(fen).expect("invalid FEN");

    let t = Instant::now();
    let mut total = 0;
    for (m, n) in divide(&pos, depth) {
        println!("{m}: {n}");
        total += n;
    }
    let secs = t.elapsed().as_secs_f64();
    println!("\nNodes: {total}\nTime:  {secs:.2}s\nSpeed: {:.0} nodes/s", total as f64 / secs);
}