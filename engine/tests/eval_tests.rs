use engine::board::{Position, START_FEN};
use engine::eval::{breakdown, evaluate};

const FENS: [&str; 5] = [
    START_FEN,
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
];

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap()
}

#[test]
fn start_position_is_equal() {
    assert_eq!(evaluate(&Position::start()), 0);
}

#[test]
fn mirror_symmetry() {
    for fen in FENS {
        let p = pos(fen);
        let m = p.mirrored();
        assert_eq!(evaluate(&p), evaluate(&m), "eval not symmetric: {fen}");
        // Bonus check: move generation must be symmetric too
        assert_eq!(p.legal_moves().len(), m.legal_moves().len(), "movegen not symmetric: {fen}");
    }
}

#[test]
fn missing_queen_is_bad() {
    let white_to_move = pos("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let black_to_move = pos("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
    assert!(breakdown(&white_to_move).total > 800);
    assert!(evaluate(&white_to_move) > 800); // good for White, White to move
    assert!(evaluate(&black_to_move) < -800); // same position, Black to move
}

#[test]
fn centralized_knight_is_better() {
    let center = pos("4k3/8/8/8/4N3/8/8/4K3 w - - 0 1");
    let corner = pos("4k3/8/8/8/8/8/8/N3K3 w - - 0 1");
    assert!(breakdown(&center).total > breakdown(&corner).total);
}

#[test]
fn game_phase() {
    assert_eq!(breakdown(&Position::start()).phase, 24);
    assert_eq!(breakdown(&pos("4k3/8/8/8/8/8/8/4K3 w - - 0 1")).phase, 0);
}
