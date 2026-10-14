//! Training data for the tuner: the engine plays fast games against itself and
//! records quiet positions together with how the game ended.
//!
//! Output: one line per position, "FEN;result", where result is from White's
//! point of view: 1.0 = White won, 0.5 = draw, 0.0 = Black won.

use crate::board::{Game, GameStatus, Position};
use crate::search::{SearchLimits, Searcher};
use crate::types::Color;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const RANDOM_OPENING_PLIES: usize = 8;
const MAX_PLIES: usize = 400;
/// Win adjudication: once the search says one side is ahead by this much
/// for this many plies in a row, that side is declared the winner.
/// Without it, won endgames the engine can't convert get labeled as draws,
/// which teaches the tuner that extra material doesn't matter.
const ADJUDICATE_SCORE: i32 = 1000;
const ADJUDICATE_PLIES: usize = 8;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Plays one game and returns its quiet positions labeled with the result.
fn play_game(rng: &mut Rng, searcher: &mut Searcher, depth: u32, book: &[String]) -> Vec<String> {
    // Start from a random book position (if a book was given), then a few random
    // moves, so every game is different
    let start = if book.is_empty() {
        Position::start()
    } else {
        match Position::from_fen(&book[(rng.next() % book.len() as u64) as usize]) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        }
    };
    let random_plies = if book.is_empty() { RANDOM_OPENING_PLIES } else { 2 };
    let mut game = Game::new(start);
    for _ in 0..random_plies {
        let moves: Vec<_> = game.current().legal_moves().iter().copied().collect();
        if moves.is_empty() {
            return Vec::new();
        }
        game.play(moves[(rng.next() % moves.len() as u64) as usize]);
    }
    if game.status() != GameStatus::Ongoing {
        return Vec::new();
    }

    searcher.clear();
    let mut fens = Vec::new();
    let limits = SearchLimits { time_ms: f64::INFINITY, max_depth: depth };
    let mut adjudicated: Option<&str> = None;
    let mut winning_streak = 0usize;
    let mut streak_side = 0i32;

    for _ in 0..MAX_PLIES {
        if game.status() != GameStatus::Ongoing {
            break;
        }
        let pos = *game.current();
        searcher.set_history(game.history_hashes());
        let r = searcher.search(&pos, limits);
        let Some(m) = r.best_move else { break };

        // Keep only "quiet" positions: not in check, the best move isn't a capture
        // or promotion, and nobody is getting mated. Tactical positions confuse the tuner.
        if !pos.in_check() && !m.is_capture() && !m.is_promotion() && r.score.abs() < 2000 {
            fens.push(pos.to_fen());
        }

        // Score from White's point of view
        let white_score = if pos.side == Color::White { r.score } else { -r.score };
        let side = if white_score >= ADJUDICATE_SCORE {
            1
        } else if white_score <= -ADJUDICATE_SCORE {
            -1
        } else {
            0
        };
        if side != 0 && side == streak_side {
            winning_streak += 1;
        } else {
            streak_side = side;
            winning_streak = usize::from(side != 0);
        }
        if winning_streak >= ADJUDICATE_PLIES {
            adjudicated = Some(if streak_side > 0 { "1.0" } else { "0.0" });
            break;
        }

        game.play(m);
    }

    let result = if let Some(r) = adjudicated { r } else { match game.status() {
        GameStatus::Checkmate(Color::White) => "1.0",
        GameStatus::Checkmate(Color::Black) => "0.0",
        _ => "0.5", // draws, and games stopped at the ply limit
    } };
    fens.into_iter().map(|f| format!("{f};{result}")).collect()
}

/// Reads up to `max` positions from an EPD/FEN opening book (one position per line).
fn load_book(path: &str, max: usize) -> Vec<String> {
    use std::io::{BufRead, BufReader};
    let Ok(file) = File::open(path) else {
        eprintln!("cannot open book {path}; using random openings");
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .map(|l| l.split_whitespace().take(4).collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .take(max)
        .collect()
}

/// engine-uci datagen <games> <threads> <depth> <output file> [opening book]
pub fn run(games: usize, threads: usize, depth: u32, out_path: &str, book_path: Option<&str>) {
    let book = book_path.map(|p| load_book(p, 500_000)).unwrap_or_default();
    if !book.is_empty() {
        eprintln!("Using {} opening positions", book.len());
    }
    let book = &book;
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    let threads = threads.max(1);
    let done = AtomicUsize::new(0);

    let results: Vec<Vec<String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let done = &done;
                let my_games = games / threads + usize::from(t < games % threads);
                scope.spawn(move || {
                    let mut rng = Rng((seed ^ (0x9E37_79B9_7F4A_7C15u64.wrapping_mul(t as u64 + 1))) | 1);
                    let mut searcher = Searcher::new(vec![]);
                    let mut lines = Vec::new();
                    for _ in 0..my_games {
                        lines.extend(play_game(&mut rng, &mut searcher, depth, book));
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if n % 50 == 0 || n == games {
                            eprintln!("{n}/{games} games");
                        }
                    }
                    lines
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("datagen thread panicked")).collect()
    });

    let file = File::create(out_path).expect("cannot create the output file");
    let mut w = BufWriter::new(file);
    let mut total = 0;
    for line in results.iter().flatten() {
        writeln!(w, "{line}").expect("write failed");
        total += 1;
    }
    eprintln!("Wrote {total} positions to {out_path}");
}