use engine::board::{Position, START_FEN};
use engine::eval::MATE;
use engine::search::{SearchLimits, Searcher};
use engine::time::now_ms;

fn search(fen: &str, depth: u32) -> (String, i32) {
    let pos = Position::from_fen(fen).unwrap();
    let r = Searcher::new(vec![]).search(&pos, SearchLimits { time_ms: f64::INFINITY, max_depth: depth });
    (r.best_move.unwrap().to_uci(), r.score)
}

#[test]
fn takes_a_free_queen() {
    let (m, _) = search("4k3/8/8/3q4/4P3/8/8/4K3 w - - 0 1", 3);
    assert_eq!(m, "e4d5");
}

#[test]
fn mate_in_one() {
    let (m, score) = search("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3);
    assert_eq!(m, "a1a8");
    assert_eq!(score, MATE - 1);
}

#[test]
fn mate_in_two() {
    let (_, score) = search("6k1/8/8/8/8/8/R7/1R4K1 w - - 0 1", 4);
    assert_eq!(score, MATE - 3);
}

#[test]
fn avoids_poisoned_pawn() {
    let (m, _) = search("4k3/8/4p3/3p4/8/8/8/3QK3 w - - 0 1", 3);
    assert_ne!(m, "d1d5");
}

#[test]
fn sees_being_mated() {
    let (_, score) = search("6k1/8/8/8/8/8/8/R5K1 b - - 0 1", 4);
    assert!(score > -MATE + 10, "Black should find a way out: {score}");
}

#[test]
fn respects_the_time_limit() {
    let pos = Position::from_fen(START_FEN).unwrap();
    let t = now_ms();
    let r = Searcher::new(vec![]).search(&pos, SearchLimits { time_ms: 300.0, max_depth: 60 });
    let elapsed = now_ms() - t;
    assert!(elapsed < 600.0, "took {elapsed:.0}ms");
    assert!(pos.parse_uci(&r.best_move.unwrap().to_uci()).is_some());
}

#[test]
fn mate_score_survives_the_transposition_table() {
    let pos = Position::from_fen("6k1/8/8/8/8/8/R7/1R4K1 w - - 0 1").unwrap();
    let mut s = Searcher::new(vec![]);
    let limits = SearchLimits { time_ms: f64::INFINITY, max_depth: 6 };
    let first = s.search(&pos, limits).score;
    let second = s.search(&pos, limits).score; // table is now full of this position
    assert_eq!(first, MATE - 3);
    assert_eq!(second, MATE - 3);
}

#[test]
fn pawn_endgame_still_works() {
    // Only kings and pawns: null move pruning switches itself off here (zugzwang risk).
    // A deep search must still run and return a sensible move.
    let pos = Position::from_fen("8/8/8/4k3/8/8/4PK2/8 w - - 0 1").unwrap();
    let r = Searcher::new(vec![]).search(&pos, SearchLimits { time_ms: f64::INFINITY, max_depth: 12 });
    assert!(r.best_move.is_some());
    assert!(r.score > 0, "an extra pawn should be good for White: {}", r.score);
}

#[test]
fn finds_scholars_mate() {
    let (m, score) = search("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4", 6);
    assert_eq!(m, "h5f7");
    assert_eq!(score, MATE - 1);
}