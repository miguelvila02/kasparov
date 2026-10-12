mod pst;

use crate::board::Position;
use crate::types::*;
use pst::{EG_TABLES, MATERIAL_EG, MATERIAL_MG, MAX_PHASE, MG_TABLES, PHASE_WEIGHT};

/// Score for checkmate. Far above any material count.
pub const MATE: i32 = 30_000;

const KINDS: [PieceKind; 6] = [
    PieceKind::Pawn,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Rook,
    PieceKind::Queen,
    PieceKind::King,
];

// Piece values (centipawns), indexed by PieceKind. The king has no material value.
const BISHOP_PAIR_MG: i32 = 30;
const BISHOP_PAIR_EG: i32 = 50;

/// A score with a middlegame and an endgame part.
#[derive(Clone, Copy, Debug, Default)]
struct S {
    mg: i32,
    eg: i32,
}

/// Every term of the evaluation, from WHITE's point of view, in centipawns.
#[derive(Clone, Copy, Debug, Default)]
pub struct Breakdown {
    pub material: i32,
    pub piece_squares: i32,
    pub bishop_pair: i32,
    /// 24 = full middlegame, 0 = bare endgame.
    pub phase: i32,
    pub total: i32,
}

fn taper(white: S, black: S, phase: i32) -> i32 {
    let mg = white.mg - black.mg;
    let eg = white.eg - black.eg;
    (mg * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE
}

pub fn breakdown(pos: &Position) -> Breakdown {
    let mut material = [S::default(); 2];
    let mut psqt = [S::default(); 2];
    let mut bishop_pair = [S::default(); 2];
    let mut phase = 0;

    for color in [Color::White, Color::Black] {
        let c = color.idx();
        for kind in KINDS {
            let k = kind.idx();
            let mut bb = pos.pieces[c][k];
            while bb != 0 {
                let s = pop_lsb(&mut bb) as usize;
                let idx = if color == Color::White { s ^ 56 } else { s };

                material[c].mg += MATERIAL_MG[k];
                material[c].eg += MATERIAL_EG[k];
                psqt[c].mg += MG_TABLES[k][idx];
                psqt[c].eg += EG_TABLES[k][idx];
                phase += PHASE_WEIGHT[k];
            }
        }
        if pos.pieces[c][PieceKind::Bishop.idx()].count_ones() >= 2 {
            bishop_pair[c] = S { mg: BISHOP_PAIR_MG, eg: BISHOP_PAIR_EG };
        }
    }

    let phase = phase.min(MAX_PHASE); // extra queens from promotion can push it above 24
    let material = taper(material[0], material[1], phase);
    let piece_squares = taper(psqt[0], psqt[1], phase);
    let bishop_pair = taper(bishop_pair[0], bishop_pair[1], phase);

    Breakdown {
        material,
        piece_squares,
        bishop_pair,
        phase,
        total: material + piece_squares + bishop_pair,
    }
}

/// Score from the point of view of the side to move (what the search uses).
pub fn evaluate(pos: &Position) -> i32 {
    let total = breakdown(pos).total;
    if pos.side == Color::White { total } else { -total }
}