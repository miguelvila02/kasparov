use super::{Position, ZOBRIST};
use crate::moves::{flag, Move};
use crate::types::*;

const fn castle_masks() -> [u8; 64] {
    let mut m = [15u8; 64];
    m[0] = 15 & !WQ;          // a1
    m[7] = 15 & !WK;          // h1
    m[4] = 15 & !(WK | WQ);   // e1
    m[56] = 15 & !BQ;         // a8
    m[63] = 15 & !BK;         // h8
    m[60] = 15 & !(BK | BQ);  // e8
    m
}
static CASTLE_MASK: [u8; 64] = castle_masks();

impl Position {
    pub fn make_move(&self, m: Move) -> Position {
        let mut p = *self;
        let us = self.side;
        let them = us.flip();
        let (from, to, flags) = (m.from(), m.to(), m.flags());
        let (_, piece) = self.board[from as usize].expect("casa de origem vazia");

        // Retirar do hash o en passant e o roque antigos
        if let Some(e) = p.ep {
            p.hash ^= ZOBRIST.ep_file[file_of(e) as usize];
        }
        p.ep = None;
        p.hash ^= ZOBRIST.castling[p.castling as usize];

        // Regra dos 50 lances
        p.halfmove += 1;
        if piece == PieceKind::Pawn || m.is_capture() {
            p.halfmove = 0;
        }

        // Capturas
        if flags == flag::EP_CAPTURE {
            let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
            p.remove(them, PieceKind::Pawn, cap_sq);
        } else if m.is_capture() {
            let (_, captured) = self.board[to as usize].expect("captura sem peça");
            p.remove(them, captured, to);
        }

        // Mover a peça (ou trocá-la pela peça promovida)
        p.remove(us, piece, from);
        p.put(us, m.promotion_piece().unwrap_or(piece), to);

        // Torre no roque
        if flags == flag::KING_CASTLE {
            let (rf, rt) = if us == Color::White { (7, 5) } else { (63, 61) };
            p.remove(us, PieceKind::Rook, rf);
            p.put(us, PieceKind::Rook, rt);
        } else if flags == flag::QUEEN_CASTLE {
            let (rf, rt) = if us == Color::White { (0, 3) } else { (56, 59) };
            p.remove(us, PieceKind::Rook, rf);
            p.put(us, PieceKind::Rook, rt);
        }

        // Nova casa de en passant
        if flags == flag::DOUBLE_PUSH {
            let e = if us == Color::White { from + 8 } else { from - 8 };
            p.ep = Some(e);
            p.hash ^= ZOBRIST.ep_file[file_of(e) as usize];
        }

        // Atualizar os direitos de roque
        p.castling &= CASTLE_MASK[from as usize] & CASTLE_MASK[to as usize];
        p.hash ^= ZOBRIST.castling[p.castling as usize];

        // Mudar o lado a jogar
        if us == Color::Black {
            p.fullmove += 1;
        }
        p.side = them;
        p.hash ^= ZOBRIST.side;
        p
    }
}