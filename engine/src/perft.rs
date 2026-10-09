// perft.rs
use crate::board::Position;

pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = pos.legal_moves();
    if depth == 1 {
        return moves.len() as u64; // shortcut: no need to play the last move
    }
    moves.iter().map(|&m| perft(&pos.make_move(m), depth - 1)).sum()
}

/// Count per first move. This is how you find bugs.
pub fn divide(pos: &Position, depth: u32) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = pos
        .legal_moves()
        .iter()
        .map(|&m| (m.to_uci(), perft(&pos.make_move(m), depth - 1)))
        .collect();
    out.sort();
    out
}