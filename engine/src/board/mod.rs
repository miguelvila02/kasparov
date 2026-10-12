mod fen;
mod game;
mod makemove;
mod zobrist;

pub use fen::FenError;
pub use zobrist::ZOBRIST;
pub use game::{Game, GameStatus};

use crate::types::*;
use std::fmt;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub pieces: [[Bitboard; 6]; 2],
    pub occupancy: [Bitboard; 2],
    pub board: [Option<(Color, PieceKind)>; 64],
    pub side: Color,
    pub castling: u8,
    pub ep: Option<Square>,
    pub halfmove: u16,
    pub fullmove: u16,
    pub hash: u64,
}

impl Position {
    pub fn empty() -> Self {
        Position {
            pieces: [[0; 6]; 2],
            occupancy: [0; 2],
            board: [None; 64],
            side: Color::White,
            castling: 0,
            ep: None,
            halfmove: 0,
            fullmove: 1,
            hash: 0,
        }
    }

    pub fn start() -> Self {
        Position::from_fen(START_FEN).expect("FEN inicial válido")
    }

    pub fn all(&self) -> Bitboard {
        self.occupancy[0] | self.occupancy[1]
    }

    pub fn piece_at(&self, s: Square) -> Option<(Color, PieceKind)> {
        self.board[s as usize]
    }

    pub(crate) fn put(&mut self, c: Color, p: PieceKind, s: Square) {
        let b = bit(s);
        self.pieces[c.idx()][p.idx()] |= b;
        self.occupancy[c.idx()] |= b;
        self.board[s as usize] = Some((c, p));
        self.hash ^= ZOBRIST.pieces[c.idx()][p.idx()][s as usize];
    }

    pub(crate) fn remove(&mut self, c: Color, p: PieceKind, s: Square) {
        let b = bit(s);
        self.pieces[c.idx()][p.idx()] &= !b;
        self.occupancy[c.idx()] &= !b;
        self.board[s as usize] = None;
        self.hash ^= ZOBRIST.pieces[c.idx()][p.idx()][s as usize];
    }

    /// Calcula o hash de raiz. Usado no FEN e nos testes, para validar o hash incremental.
    pub fn compute_hash(&self) -> u64 {
        let mut h = 0;
        for s in 0..64usize {
            if let Some((c, p)) = self.board[s] {
                h ^= ZOBRIST.pieces[c.idx()][p.idx()][s];
            }
        }
        h ^= ZOBRIST.castling[self.castling as usize];
        if let Some(e) = self.ep {
            h ^= ZOBRIST.ep_file[file_of(e) as usize];
        }
        if self.side == Color::Black {
            h ^= ZOBRIST.side;
        }
        h
    }
    /// The same position with the board flipped vertically and colors swapped.
    pub fn mirrored(&self) -> Position {
        let mut m = Position::empty();
        for s in 0..64u8 {
            if let Some((c, p)) = self.board[s as usize] {
                m.put(c.flip(), p, s ^ 56);
            }
        }
        m.side = self.side.flip();
        // White's rights (bits 0-1) become Black's (bits 2-3) and vice versa
        m.castling = ((self.castling & (WK | WQ)) << 2) | ((self.castling & (BK | BQ)) >> 2);
        m.ep = self.ep.map(|e| e ^ 56);
        m.halfmove = self.halfmove;
        m.fullmove = self.fullmove;
        m.hash = m.compute_hash();
        m
    }
}

/// Impressão ASCII, essencial para depurar.
impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rank in (0..8u8).rev() {
            write!(f, "{} ", rank + 1)?;
            for file in 0..8u8 {
                let c = match self.board[sq(file, rank) as usize] {
                    Some((c, p)) => piece_to_char(c, p),
                    None => '.',
                };
                write!(f, "{} ", c)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")
    }
}