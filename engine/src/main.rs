use engine::board::{Game, Position, START_FEN};
use engine::eval::breakdown;
use engine::perft::divide;
use engine::search::{SearchLimits, Searcher};
use engine::types::Color;
use std::io::{self, BufRead, Write};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("perft") => run_perft(&args),
        Some("eval") => run_eval(&args),
        Some("search") => run_search(&args),
        _ => run_uci(),
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

/// engine-uci search <ms> [fen]
fn run_search(args: &[String]) {
    let ms: f64 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(3000.0);
    let fen = args.get(3).map(String::as_str).unwrap_or(START_FEN);
    let pos = Position::from_fen(fen).expect("invalid FEN");
    let r = Searcher::new(vec![]).search(&pos, SearchLimits { time_ms: ms, max_depth: 60 });
    let pv: Vec<String> = r.pv.iter().map(|m| m.to_uci()).collect();
    println!("{pos}");
    println!(
        "bestmove {}  score {:+}  depth {}  nodes {}  time {:.0}ms  ({:.0} nodes/s)",
        r.best_move.map(|m| m.to_uci()).unwrap_or("none".into()),
        r.score, r.depth, r.nodes, r.time_ms,
        r.nodes as f64 / (r.time_ms / 1000.0).max(0.001)
    );
    println!("pv {}", pv.join(" "));
}

fn run_uci() {
    let mut game = Game::new(Position::start());
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        match line.split_whitespace().next() {
            Some("uci") => {
                println!("id name Kasparov");
                println!("id author Kasparov project");
                println!("uciok");
            }
            Some("isready") => println!("readyok"),
            Some("ucinewgame") => game = Game::new(Position::start()),
            Some("position") => {
                if let Some(g) = parse_position(&line) {
                    game = g;
                }
            }
            Some("go") => {
                let pos = *game.current();
                let limits = parse_go(&line, pos.side == Color::White);
                let r = Searcher::new(game.history_hashes()).search(&pos, limits);
                let pv: Vec<String> = r.pv.iter().map(|m| m.to_uci()).collect();
                println!(
                    "info depth {} score cp {} nodes {} time {:.0} pv {}",
                    r.depth, r.score, r.nodes, r.time_ms, pv.join(" ")
                );
                match r.best_move {
                    Some(m) => println!("bestmove {}", m.to_uci()),
                    None => println!("bestmove 0000"),
                }
            }
            Some("quit") => break,
            _ => {}
        }
        io::stdout().flush().ok();
    }
}

/// "position startpos [moves ...]" or "position fen <FEN> [moves ...]"
fn parse_position(line: &str) -> Option<Game> {
    let t: Vec<&str> = line.split_whitespace().collect();
    let moves_at = t.iter().position(|&x| x == "moves").unwrap_or(t.len());
    let pos = match t.get(1).copied()? {
        "startpos" => Position::start(),
        "fen" => Position::from_fen(&t[2..moves_at].join(" ")).ok()?,
        _ => return None,
    };
    let mut game = Game::new(pos);
    for uci in t.iter().skip(moves_at + 1) {
        let m = game.current().parse_uci(uci)?;
        game.play(m);
    }
    Some(game)
}

/// Supports: movetime, wtime/btime/winc/binc, depth.
fn parse_go(line: &str, white: bool) -> SearchLimits {
    let t: Vec<&str> = line.split_whitespace().collect();
    let get = |key: &str| {
        t.iter()
            .position(|&x| x == key)
            .and_then(|i| t.get(i + 1))
            .and_then(|v| v.parse::<f64>().ok())
    };

    let mut limits = SearchLimits { time_ms: f64::INFINITY, max_depth: 60 };
    if let Some(d) = get("depth") {
        limits.max_depth = d as u32;
    }
    if let Some(mt) = get("movetime") {
        limits.time_ms = mt;
    } else if let Some(left) = get(if white { "wtime" } else { "btime" }) {
        let inc = get(if white { "winc" } else { "binc" }).unwrap_or(0.0);
        // Simple time management: ~1/30 of the remaining time plus most of the increment
        limits.time_ms = (left / 30.0 + inc * 0.75).min(left - 50.0).max(10.0);
    } else if get("depth").is_none() {
        limits.max_depth = 8; // "go infinite" without stop support: cap it
    }
    limits
}