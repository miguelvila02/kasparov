use engine::board::{Game, Position, START_FEN};
use engine::eval::breakdown;
use engine::perft::divide;
use engine::search::{SearchLimits, Searcher};
use engine::types::Color;
use std::io::{self, BufRead, Write};
use std::time::Instant;

/// Fixed positions for the benchmark (the perft test positions).
const BENCH_FENS: [&str; 6] = [
    START_FEN,
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("perft") => run_perft(&args),
        Some("eval") => run_eval(&args),
        Some("search") => run_search(&args),
        Some("bench") => run_bench(&args),
        Some("coach") => run_coach(&args),
        Some("datagen") => run_datagen(&args),
        Some("tune") => run_tune(&args),
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
    println!("Pawn structure: {:+}", b.pawn_structure);
    println!("Mobility:       {:+}", b.mobility);
    println!("Rooks:          {:+}", b.rooks);
    println!("King safety:    {:+}", b.king_safety);
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
        r.score,
        r.depth,
        r.nodes,
        r.time_ms,
        r.nodes as f64 / (r.time_ms / 1000.0).max(0.001)
    );
    println!("pv {}", pv.join(" "));
}

/// engine-uci bench [depth]
/// Searches fixed positions to a fixed depth. The total node count is a "fingerprint":
/// it only changes when the search behaves differently, so a change that should only
/// make the engine faster must keep the same number.
fn run_bench(args: &[String]) {
    let depth: u32 = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(8);
    let mut total_nodes = 0u64;
    let mut total_ms = 0.0;
    for fen in BENCH_FENS {
        let pos = Position::from_fen(fen).expect("invalid FEN");
        let r = Searcher::new(vec![]).search(&pos, SearchLimits { time_ms: f64::INFINITY, max_depth: depth });
        println!(
            "{:<8} depth {:>2}  score {:+6}  nodes {:>10}",
            r.best_move.map(|m| m.to_uci()).unwrap_or("none".into()),
            r.depth,
            r.score,
            r.nodes
        );
        total_nodes += r.nodes;
        total_ms += r.time_ms;
    }
    println!(
        "\nBench: {total_nodes} nodes  {:.2}s  {:.0} nodes/s",
        total_ms / 1000.0,
        total_nodes as f64 / (total_ms / 1000.0).max(0.001)
    );
}

/// engine-uci datagen <games> <threads> <depth> [output file] [opening book]
fn run_datagen(args: &[String]) {
    let games: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(2000);
    let threads: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(3);
    let depth: u32 = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(6);
    let out = args.get(5).map(String::as_str).unwrap_or("data/positions.txt");
    let book = args.get(6).map(String::as_str);
    engine::datagen::run(games, threads, depth, out, book);
}

/// engine-uci tune <data file> [epochs] [threads] [output file] [--all]
/// Without --all, only the new terms are tuned; PeSTO material and tables stay fixed.
fn run_tune(args: &[String]) {
    let data = args.get(2).map(String::as_str).unwrap_or("data/positions.txt");
    let epochs: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(500);
    let threads: usize = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(3);
    let out = args.get(5).map(String::as_str).unwrap_or("src/eval/params.rs");
    let tune_all = args.iter().any(|a| a == "--all");
    engine::tune::run(data, epochs, threads, out, tune_all);
}

/// engine-uci coach <ms> <fen> [user move that led to this position, e.g. e2e4] [fen before it]
/// Plays one bot turn from <fen> and prints the explanation (and the review of the user's move).
fn run_coach(args: &[String]) {
    let ms: f64 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(1000.0);
    let fen = args.get(3).map(String::as_str).unwrap_or(START_FEN);
    let (mut game, user_move) = match (args.get(4), args.get(5)) {
        (Some(uci), Some(before_fen)) => {
            let before = Position::from_fen(before_fen).expect("invalid FEN");
            let m = before.parse_uci(uci).expect("illegal user move");
            let mut g = Game::new(before);
            g.play(m);
            (g, Some(m))
        }
        _ => (Game::new(Position::from_fen(fen).expect("invalid FEN")), None),
    };
    let moves: Vec<_> = user_move.into_iter().collect();
    match engine::explain::bot_turn(&mut game, &moves, user_move.is_some(), ms * 0.25, ms) {
        Some(t) => {
            if let Some(r) = &t.review {
                println!("[{}] {}", r.verdict.as_str(), r.text);
            }
            println!("[bot] {}", t.explanation);
            println!("depth {} nodes {} score(White) {:+}", t.depth, t.nodes, t.score_white);
        }
        None => println!("no legal moves"),
    }
}

fn run_uci() {
    let mut game = Game::new(Position::start());
    // One searcher for the whole session, so the transposition table carries over between moves
    let mut searcher = Searcher::new(vec![]);
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
            Some("ucinewgame") => {
                game = Game::new(Position::start());
                searcher.clear();
            }
            Some("position") => {
                if let Some(g) = parse_position(&line) {
                    game = g;
                }
            }
            Some("go") => {
                let pos = *game.current();
                let limits = parse_go(&line, pos.side == Color::White);
                searcher.set_history(game.history_hashes());
                let r = searcher.search(&pos, limits);
                let pv: Vec<String> = r.pv.iter().map(|m| m.to_uci()).collect();
                println!(
                    "info depth {} score cp {} nodes {} time {:.0} pv {}",
                    r.depth,
                    r.score,
                    r.nodes,
                    r.time_ms,
                    pv.join(" ")
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