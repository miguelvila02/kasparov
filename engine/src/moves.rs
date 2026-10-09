use crate::board::Position;
use crate::types::*;

pub mod flag {
    pub const QUIET: u16 = 0;
    pub const DOUBLE_PUSH: u16 = 1;
    pub const KING_CASTLE: u16 = 2;
    pub const QUEEN_CASTLE: u16 = 3;
    pub const CAPTURE: u16 = 4;
    pub const EP_CAPTURE: u16 = 5;
    pub const PROMO: u16 = 8;          // + 0..3 = C, B, T, D
    pub const PROMO_CAPTURE: u16 = 12; // + 0..3
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move(pub u16);

impl Move {
    pub const NULL: Move = Move(0);

    pub fn new(from: Square, to: Square, flags: u16) -> Move {
        Move(from as u16 | (to as u16) << 6 | flags << 12)
    }
    pub fn from(self) -> Square {
        (self.0 & 0x3F) as Square
    }
    pub fn to(self) -> Square {
        ((self.0 >> 6) & 0x3F) as Square
    }
    pub fn flags(self) -> u16 {
        self.0 >> 12
    }
    pub fn is_capture(self) -> bool {
        self.flags() & 4 != 0
    }
    pub fn is_promotion(self) -> bool {
        self.flags() & 8 != 0
    }
    pub fn promotion_piece(self) -> Option<PieceKind> {
        if !self.is_promotion() {
            return None;
        }
        Some(match self.flags() & 3 {
            0 => PieceKind::Knight,
            1 => PieceKind::Bishop,
            2 => PieceKind::Rook,
            _ => PieceKind::Queen,
        })
    }

    pub fn to_uci(self) -> String {
        let mut s = format!("{}{}", square_name(self.from()), square_name(self.to()));
        if let Some(p) = self.promotion_piece() {
            s.push(piece_to_char(Color::Black, p)); // minúscula: e7e8q
        }
        s
    }
}

impl Position {
    /// Converte "e2e4" / "e7e8q" num Move, deduzindo as flags a partir do tabuleiro.
    /// ATENÇÃO: ainda NÃO verifica a legalidade (isso chega na Fase 2).
    pub fn parse_uci(&self, s: &str) -> Option<Move> {
        self.legal_moves().iter().copied().find(|m| m.to_uci() == s)
    }
}