use engine::board::{Game, Position};
use engine::explain::{bot_turn, explain_move, Verdict};
use engine::moves::Move;

/// Plays `user_uci` from `before_fen`, then a full bot turn with a review.
fn turn(before_fen: &str, user_uci: &str, ms: f64) -> engine::explain::TurnResult {
    let before = Position::from_fen(before_fen).unwrap();
    let m = before.parse_uci(user_uci).unwrap();
    let mut g = Game::new(before);
    g.play(m);
    bot_turn(&mut g, &[m], true, ms * 0.3, ms).unwrap()
}

fn bot_only(fen: &str, ms: f64) -> engine::explain::TurnResult {
    let mut g = Game::new(Position::from_fen(fen).unwrap());
    bot_turn(&mut g, &[], false, 0.0, ms).unwrap()
}

fn explain(fen: &str, uci: &str) -> String {
    let p = Position::from_fen(fen).unwrap();
    let m: Move = p.parse_uci(uci).unwrap();
    explain_move(&p, m, &[], 0, None)
}

#[test]
fn hanging_the_queen_is_a_blunder() {
    // 1.e4 e5 2.Nf3 Qg5?? (Black hangs the queen to Nxg5)
    let t = turn("rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2", "d8g5", 400.0);
    let r = t.review.unwrap();
    assert_eq!(r.verdict, Verdict::Blunder, "{}", r.text);
    assert!(r.text.contains("wins the queen"), "{}", r.text);
    assert_eq!(t.bot_move.to_uci(), "f3g5");
    assert!(t.explanation.contains("Wins the queen on g5"), "{}", t.explanation);
}

#[test]
fn a_sound_developing_move_is_not_flagged() {
    // 1.e4 Nc6 is a normal move
    let t = turn("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1", "b8c6", 400.0);
    let v = t.review.unwrap().verdict;
    assert!(matches!(v, Verdict::Best | Verdict::Good), "{v:?}");
}

#[test]
fn finds_and_names_a_fork() {
    let t = bot_only("r3k3/8/8/3N4/8/8/8/4K3 w - - 0 1", 300.0);
    assert_eq!(t.bot_move.to_uci(), "d5c7");
    assert!(t.explanation.contains("Forks the king and the rook"), "{}", t.explanation);
}

#[test]
fn names_checkmate() {
    let t = bot_only("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 300.0);
    assert!(t.explanation.contains("Delivers checkmate"), "{}", t.explanation);
}

#[test]
fn simple_move_types() {
    assert!(explain("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "g1f3").contains("Develops the knight to f3"));
    assert!(explain("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "e2e4").contains("center"));
    assert!(explain("r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4", "e1g1").contains("Castles kingside"));
    // Blocking a check with the bishop: 1.e4 d5 2.Bb5+ ... Bd7
    assert!(explain("rnbqkbnr/ppp1pppp/8/1B1p4/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2", "c8d7").contains("Blocks the check"));
}

#[test]
fn arrows_start_with_the_bot_move() {
    let t = bot_only("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 300.0);
    assert_eq!(t.arrows[0], (t.bot_move.from(), t.bot_move.to()));
    assert!(t.arrows.len() <= 3);
}

#[test]
fn json_is_valid_and_complete() {
    let t = turn("rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2", "d8g5", 300.0);
    let j = t.to_json();
    for key in ["\"move\":\"f3g5\"", "\"explanation\":", "\"arrows\":[[\"f3\",\"g5\"]", "\"verdict\":\"blunder\""] {
        assert!(j.contains(key), "missing {key} in {j}");
    }
}