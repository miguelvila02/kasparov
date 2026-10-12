use crate::board::Position;
use crate::eval::{evaluate, MATE};
use crate::moves::Move;

/// Phase 3 placeholder: look one move ahead and pick the best-scoring move.
/// `seed` picks among equally good moves, so the bot doesn't always play the same game.
pub fn greedy_move(pos: &Position, seed: u32) -> Option<Move> {
    let mut best: Vec<Move> = Vec::new();
    let mut best_score = i32::MIN;

    for &m in pos.legal_moves().iter() {
        let next = pos.make_move(m);
        let score = if next.legal_moves().is_empty() {
            if next.in_check() { MATE } else { 0 } // checkmate or stalemate
        } else {
            -evaluate(&next) // next is from the opponent's view, so negate
        };

        if score > best_score {
            best_score = score;
            best.clear();
            best.push(m);
        } else if score == best_score {
            best.push(m);
        }
    }

    if best.is_empty() {
        None
    } else {
        Some(best[seed as usize % best.len()])
    }
}