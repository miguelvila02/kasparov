use engine::board::{Game, Position, START_FEN};

fn play(fen: &str, moves: &[&str]) -> Position {
    let mut pos = Position::from_fen(fen).unwrap();
    for uci in moves {
        let m = pos.parse_uci(uci).unwrap_or_else(|| panic!("lance inválido {uci}"));
        pos = pos.make_move(m);
        assert_eq!(pos.hash, pos.compute_hash(), "hash dessincronizado após {uci}");
    }
    pos
}

#[test]
fn fen_ida_e_volta() {
    let fens = [
        START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbqkbnr/1pp1pppp/p7/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
    ];
    for fen in fens {
        assert_eq!(Position::from_fen(fen).unwrap().to_fen(), fen);
    }
}

#[test]
fn fen_invalidos() {
    let maus = [
        "",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP w KQkq - 0 1",      // só 7 filas
        "rnbqkbnr/pppppppp/9/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", // dígito 9
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1", // lado inválido
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkx - 0 1", // roque inválido
    ];
    for fen in maus {
        assert!(Position::from_fen(fen).is_err(), "devia falhar: {fen}");
    }
}

#[test]
fn italiana_com_roque() {
    let pos = play(START_FEN, &["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6", "e1g1"]);
    assert_eq!(
        pos.to_fen(),
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4"
    );
}

#[test]
fn en_passant() {
    let pos = play(
        "rnbqkbnr/1pp1pppp/p7/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
        &["e5d6"],
    );
    assert_eq!(pos.to_fen(), "rnbqkbnr/1pp1pppp/p2P4/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3");
}

#[test]
fn promocao() {
    let pos = play("8/P7/8/8/8/8/8/k6K w - - 0 1", &["a7a8q"]);
    assert_eq!(pos.to_fen(), "Q7/8/8/8/8/8/8/k6K b - - 0 1");
}

#[test]
fn torre_perde_direito_de_roque() {
    let pos = play(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &["a1b1"],
    );
    assert_eq!(
        pos.to_fen(),
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/1R2K2R b Kkq - 1 1"
    );
}

#[test]
fn transposicao_tem_o_mesmo_hash() {
    let a = play(START_FEN, &["g1f3", "g8f6", "b1c3", "b8c6"]);
    let b = play(START_FEN, &["b1c3", "b8c6", "g1f3", "g8f6"]);
    assert_eq!(a.hash, b.hash);
}

#[test]
fn tripla_repeticao() {
    let mut g = Game::new(Position::start());
    let ciclo = ["g1f3", "g8f6", "f3g1", "f6g8"];
    for uci in ciclo {
        let m = g.current().parse_uci(uci).unwrap();
        g.play(m);
    }
    assert!(!g.is_threefold()); // a posição inicial apareceu 2 vezes
    for uci in ciclo {
        let m = g.current().parse_uci(uci).unwrap();
        g.play(m);
    }
    assert!(g.is_threefold()); // 3.ª vez
}