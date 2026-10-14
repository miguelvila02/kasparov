// Plays a game: "user" = random move 1/3 of the time, otherwise a short search; bot = full coach turn.
use engine::board::{Game, GameStatus, Position};
use engine::explain::bot_turn;
use engine::search::{SearchLimits, Searcher};
fn main() {
    let seed: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(7);
    let mut rng = seed | 1;
    let mut next = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let mut g = Game::new(Position::start());
    let mut moves = Vec::new();
    for _ in 0..40 {
        if g.status() != GameStatus::Ongoing { break; }
        let pos = *g.current();
        let um = if next() % 3 == 0 {
            let v: Vec<_> = pos.legal_moves().iter().copied().collect();
            v[(next() % v.len() as u64) as usize]
        } else {
            Searcher::new(g.history_hashes()).search(&pos, SearchLimits { time_ms: 30.0, max_depth: 60 }).best_move.unwrap()
        };
        g.play(um);
        moves.push(um);
        if g.status() != GameStatus::Ongoing { break; }
        let t = bot_turn(&mut g, &moves, true, 120.0, 400.0).unwrap();
        moves.push(t.bot_move);
        if let Some(r) = t.review { println!("  [{}] {}", r.verdict.as_str(), r.text); }
        println!("  [bot] {}", t.explanation);
    }
    println!("final: {:?}", g.status());
}