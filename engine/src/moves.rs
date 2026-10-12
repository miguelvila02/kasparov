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
    
    /// Standard algebraic notation for a legal move in this position: "Nf3", "exd5", "O-O", "Qh4#".
    pub fn san(&self, m: Move) -> String {
        let flags = m.flags();
        let mut s = if flags == flag::KING_CASTLE {
            "O-O".to_string()
        } else if flags == flag::QUEEN_CASTLE {
            "O-O-O".to_string()
        } else {
            let (_, piece) = self.piece_at(m.from()).expect("no piece on the origin square");
            let mut s = String::new();

            if piece == PieceKind::Pawn {
                if m.is_capture() {
                    s.push((b'a' + file_of(m.from())) as char);
                    s.push('x');
                }
                s.push_str(&square_name(m.to()));
                if let Some(p) = m.promotion_piece() {
                    s.push('=');
                    s.push(piece_to_char(Color::White, p));
                }
            } else {
                s.push(piece_to_char(Color::White, piece)); // uppercase letter

                // Disambiguation: another identical piece can reach the same square
                let others: Vec<Move> = self
                    .legal_moves()
                    .iter()
                    .copied()
                    .filter(|o| {
                        *o != m
                            && o.to() == m.to()
                            && self.piece_at(o.from()).map(|x| x.1) == Some(piece)
                    })
                    .collect();
                if !others.is_empty() {
                    let same_file = others.iter().any(|o| file_of(o.from()) == file_of(m.from()));
                    let same_rank = others.iter().any(|o| rank_of(o.from()) == rank_of(m.from()));
                    if !same_file {
                        s.push((b'a' + file_of(m.from())) as char);
                    } else if !same_rank {
                        s.push((b'1' + rank_of(m.from())) as char);
                    } else {
                        s.push_str(&square_name(m.from()));
                    }
                }

                if m.is_capture() {
                    s.push('x');
                }
                s.push_str(&square_name(m.to()));
            }
            s
        };

        // Check or checkmate
        let next = self.make_move(m);
        if next.in_check() {
            s.push(if next.legal_moves().is_empty() { '#' } else { '+' });
        }
        s
    }
}