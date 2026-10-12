use super::Position;
use crate::moves::Move;
use crate::types::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Checkmate(Color), // the winner
    Stalemate,
    FiftyMoves,
    Repetition,
    InsufficientMaterial,
}

pub struct Game {
    stack: Vec<Position>,
}

impl Game {
    pub fn new(pos: Position) -> Self {
        Game { stack: vec![pos] }
    }
    pub fn current(&self) -> &Position {
        self.stack.last().unwrap()
    }
    pub fn play(&mut self, m: Move) {
        let next = self.current().make_move(m);
        self.stack.push(next);
    }
    pub fn undo(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }
    /// Quantas vezes a posição atual já apareceu antes. Só é preciso olhar
    /// para trás até ao último lance de peão ou captura.
    pub fn repetitions(&self) -> usize {
        let cur = self.current();
        let n = self.stack.len();
        let lim = (cur.halfmove as usize).min(n - 1);
        self.stack[n - 1 - lim..n - 1]
            .iter()
            .filter(|p| p.hash == cur.hash)
            .count()
    }
    pub fn is_threefold(&self) -> bool {
        self.repetitions() >= 2
    }
    pub fn status(&self) -> GameStatus {
        let pos = self.current();
        if pos.legal_moves().is_empty() {
            return if pos.in_check() {
                GameStatus::Checkmate(pos.side.flip())
            } else {
                GameStatus::Stalemate
            };
        }
        if pos.halfmove >= 100 {
            GameStatus::FiftyMoves
        } else if self.is_threefold() {
            GameStatus::Repetition
        } else if pos.insufficient_material() {
            GameStatus::InsufficientMaterial
        } else {
            GameStatus::Ongoing
        }
    }
    /// Hashes of every position before the current one, oldest first.
    pub fn history_hashes(&self) -> Vec<u64> {
        self.stack[..self.stack.len() - 1].iter().map(|p| p.hash).collect()
    }
}


impl Position {
    pub fn insufficient_material(&self) -> bool {
        let (w, b) = (&self.pieces[0], &self.pieces[1]);
        let p = |k: PieceKind| k.idx();
        let heavy = w[p(PieceKind::Pawn)] | w[p(PieceKind::Rook)] | w[p(PieceKind::Queen)]
            | b[p(PieceKind::Pawn)] | b[p(PieceKind::Rook)] | b[p(PieceKind::Queen)];
        if heavy != 0 {
            return false;
        }
        let minors = (w[p(PieceKind::Knight)] | w[p(PieceKind::Bishop)]
            | b[p(PieceKind::Knight)] | b[p(PieceKind::Bishop)]).count_ones();
        if minors <= 1 {
            return true; // K vs K, or K + one minor piece vs K
        }
        // K+B vs K+B with both bishops on the same square color
        let (wb, bb) = (w[p(PieceKind::Bishop)], b[p(PieceKind::Bishop)]);
        if minors == 2 && wb.count_ones() == 1 && bb.count_ones() == 1 {
            const LIGHT: u64 = 0x55AA_55AA_55AA_55AA;
            return (wb & LIGHT != 0) == (bb & LIGHT != 0);
        }
        false
    }
}
