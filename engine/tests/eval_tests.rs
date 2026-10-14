use engine::board::{Position, START_FEN};
use engine::eval::{breakdown, evaluate, trace, MAX_PHASE, WEIGHTS};

const FENS: [&str; 6] = [
    START_FEN,
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
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
        assert_eq!(p.legal_moves().len(), m.legal_moves().len(), "movegen not symmetric: {fen}");
    }
}

#[test]
fn missing_queen_is_bad() {
    let w = pos("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let b = pos("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
    assert!(breakdown(&w).total > 800);
    assert!(evaluate(&w) > 800);
    assert!(evaluate(&b) < -800);
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

#[test]
fn passed_pawn_is_recognized() {
    // White's e-pawn is passed in the first position, blocked by a black pawn on e7 in the second
    let passed = pos("4k3/8/8/8/4P3/8/8/4K3 w - - 0 1");
    let blocked = pos("4k3/4p3/8/8/4P3/8/8/4K3 w - - 0 1");
    assert!(breakdown(&passed).pawn_structure > 0);
    assert!(breakdown(&blocked).pawn_structure <= 0);
}

#[test]
fn rook_likes_open_files() {
    let open = pos("4k3/pp4pp/8/8/8/8/PP4PP/3RK3 w - - 0 1"); // d-file has no pawns
    let closed = pos("4k3/pp1p2pp/8/8/8/8/PP1P2PP/3RK3 w - - 0 1");
    assert!(breakdown(&open).rooks > breakdown(&closed).rooks);
}

#[test]
fn trace_matches_evaluation() {
    // The tuner relies on: evaluation == sum of (count x weight), tapered.
    for fen in FENS {
        let p = pos(fen);
        let (coefs, phase) = trace(&p);
        let (mut mg, mut eg) = (0i64, 0i64);
        for (i, c) in coefs {
            mg += WEIGHTS[i as usize].mg as i64 * c as i64;
            eg += WEIGHTS[i as usize].eg as i64 * c as i64;
        }
        let traced = (mg * phase as i64 + eg * (MAX_PHASE - phase) as i64) / MAX_PHASE as i64;
        let real = breakdown(&p).total as i64;
        // Small difference allowed: the breakdown rounds each term separately
        assert!((traced - real).abs() <= 8, "{fen}: traced {traced} vs real {real}");
    }
}