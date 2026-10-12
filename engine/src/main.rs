use engine::board::{Position, START_FEN};
use engine::eval::breakdown;
use engine::perft::divide;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("perft") => run_perft(&args),
        Some("eval") => run_eval(&args),
        _ => {
            println!("usage:");
            println!("  engine-uci perft <depth> [fen]");
            println!("  engine-uci eval [fen]");
        }
    }
}

fn run_perft(args: &[String]) {
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

fn run_eval(args: &[String]) {
    let fen = args.get(2).map(String::as_str).unwrap_or(START_FEN);
    let pos = Position::from_fen(fen).expect("invalid FEN");
    let b = breakdown(&pos);

    println!("{pos}");
    println!("Phase:          {:>3} / 24", b.phase);
    println!("Material:       {:+}", b.material);
    println!("Piece-squares:  {:+}", b.piece_squares);
    println!("Bishop pair:    {:+}", b.bishop_pair);
    println!("-----------------------");
    println!("Total (White):  {:+}  ({:+.2} pawns)", b.total, b.total as f64 / 100.0);
}