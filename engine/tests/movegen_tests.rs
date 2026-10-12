use engine::board::{Game, GameStatus, Position, START_FEN};
use engine::perft::perft;

fn check(fen: &str, expected: &[u64]) {
    let pos = Position::from_fen(fen).unwrap();
    for (i, &nodes) in expected.iter().enumerate() {
        let depth = i as u32 + 1;
        assert_eq!(perft(&pos, depth), nodes, "depth {depth} for {fen}");
    }
}

fn san(fen: &str, uci: &str) -> String {
    let p = Position::from_fen(fen).unwrap();
    p.san(p.parse_uci(uci).unwrap())
}

const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
const POS3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
const POS4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
const POS5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
const POS6: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

// Fast: run on every `cargo test`
#[test] fn perft_start()     { check(START_FEN, &[20, 400, 8_902]); }
#[test] fn perft_kiwipete()  { check(KIWIPETE, &[48, 2_039, 97_862]); }
#[test] fn perft_pos3()      { check(POS3, &[14, 191, 2_812, 43_238]); }
#[test] fn perft_pos4()      { check(POS4, &[6, 264, 9_467]); }
#[test] fn perft_pos5()      { check(POS5, &[44, 1_486, 62_379]); }
#[test] fn perft_pos6()      { check(POS6, &[46, 2_079, 89_890]); }

// Deep: run with `cargo test --release -- --ignored`
#[test] #[ignore] fn perft_start_deep()    { check(START_FEN, &[20, 400, 8_902, 197_281, 4_865_609]); }
#[test] #[ignore] fn perft_kiwipete_deep() { check(KIWIPETE, &[48, 2_039, 97_862, 4_085_603]); }
#[test] #[ignore] fn perft_pos3_deep()     { check(POS3, &[14, 191, 2_812, 43_238, 674_624]); }
#[test] #[ignore] fn perft_pos4_deep()     { check(POS4, &[6, 264, 9_467, 422_333]); }
#[test] #[ignore] fn perft_pos5_deep()     { check(POS5, &[44, 1_486, 62_379, 2_103_487]); }
#[test] #[ignore] fn perft_pos6_deep()     { check(POS6, &[46, 2_079, 89_890, 3_894_594]); }

fn play_all(moves: &[&str]) -> Game {
    let mut g = Game::new(Position::start());
    for uci in moves {
        let m = g.current().parse_uci(uci).unwrap_or_else(|| panic!("illegal: {uci}"));
        g.play(m);
    }
    g
}

#[test]
fn fools_mate() {
    let g = play_all(&["f2f3", "e7e5", "g2g4", "d8h4"]);
    assert_eq!(g.status(), GameStatus::Checkmate(engine::types::Color::Black));
}

#[test]
fn stalemate() {
    let g = Game::new(Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap());
    assert_eq!(g.status(), GameStatus::Stalemate);
}

#[test]
fn insufficient_material() {
    let g = Game::new(Position::from_fen("8/8/8/4k3/8/8/8/4K2N w - - 0 1").unwrap());
    assert_eq!(g.status(), GameStatus::InsufficientMaterial);
}

#[test]
fn rejects_illegal_moves() {
    let pos = Position::start();
    assert!(pos.parse_uci("e2e5").is_none()); // pawn can't move 3 squares
    assert!(pos.parse_uci("e1g1").is_none()); // can't castle through pieces
    assert!(pos.parse_uci("f1c4").is_none()); // bishop is blocked
}

#[test]
fn san_notation() {
    assert_eq!(san(START_FEN, "e2e4"), "e4");
    assert_eq!(san(START_FEN, "g1f3"), "Nf3");
    // Two knights can reach d2: name the one moving
    assert_eq!(san("4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1", "b1d2"), "Nbd2");
    // Promotion that gives check down the a-file
    assert_eq!(san("8/P7/8/8/8/8/8/k6K w - - 0 1", "a7a8q"), "a8=Q+");
    // Fool's mate
    assert_eq!(
        san("rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2", "d8h4"),
        "Qh4#"
    );
    assert_eq!(
        san("r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4", "e1g1"),
        "O-O"
    );
}