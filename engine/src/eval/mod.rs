//! Phase 7 evaluation.
//!
//! The evaluation is a sum of "terms" (material, piece placement, pawn structure,
//! mobility, rooks, king safety). Every term is a count multiplied by a weight from
//! `params.rs`, with a middlegame and an endgame value blended by the game phase.
//!
//! Because the evaluation is written once, generically over a `Sink`, the same code
//! is used for two jobs:
//! - normal play: the sink multiplies each count by its weight (`breakdown`, `evaluate`)
//! - tuning: the sink just records the counts (`trace`), so the tuner can find
//!   better weights without re-running the evaluation for every guess.

mod params;

use crate::attacks::{bishop_attacks, rook_attacks, KING, KNIGHT};
use crate::board::Position;
use crate::types::*;
pub use params::WEIGHTS;

/// Score for checkmate. Far above any material count.
pub const MATE: i32 = 30_000;

/// A middlegame / endgame pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct S {
    pub mg: i32,
    pub eg: i32,
}

pub const fn s(mg: i32, eg: i32) -> S {
    S { mg, eg }
}

// ---------- Parameter layout (indices into WEIGHTS) ----------
pub const MATERIAL: usize = 0; // 6: pawn..king
pub const PST: usize = MATERIAL + 6; // 6 x 64
pub const BISHOP_PAIR: usize = PST + 6 * 64;
pub const PASSED: usize = BISHOP_PAIR + 1; // 8: by relative rank
pub const ISOLATED: usize = PASSED + 8;
pub const DOUBLED: usize = ISOLATED + 1;
pub const MOBILITY: usize = DOUBLED + 1; // 4: knight, bishop, rook, queen
pub const ROOK_OPEN: usize = MOBILITY + 4;
pub const ROOK_SEMI_OPEN: usize = ROOK_OPEN + 1;
pub const KING_SHIELD: usize = ROOK_SEMI_OPEN + 1;
pub const KING_ATTACK: usize = KING_SHIELD + 1; // 4: knight, bishop, rook, queen
pub const N_PARAMS: usize = KING_ATTACK + 4;

pub const PHASE_WEIGHT: [i32; 6] = [0, 1, 1, 2, 4, 0];
pub const MAX_PHASE: i32 = 24;

const KINDS: [PieceKind; 6] = [
    PieceKind::Pawn,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Rook,
    PieceKind::Queen,
    PieceKind::King,
];

// ---------- Term groups (for the breakdown shown to people) ----------
#[derive(Clone, Copy)]
enum Term {
    Material,
    PieceSquares,
    BishopPair,
    PawnStructure,
    Mobility,
    Rooks,
    KingSafety,
}
const N_TERMS: usize = 7;

fn term_of(idx: usize) -> Term {
    if idx < PST {
        Term::Material
    } else if idx < BISHOP_PAIR {
        Term::PieceSquares
    } else if idx == BISHOP_PAIR {
        Term::BishopPair
    } else if idx < MOBILITY {
        Term::PawnStructure
    } else if idx < ROOK_OPEN {
        Term::Mobility
    } else if idx < KING_SHIELD {
        Term::Rooks
    } else {
        Term::KingSafety
    }
}

// ---------- Bitboard helpers ----------
const FILE_A: u64 = 0x0101_0101_0101_0101;
const NOT_A: u64 = 0xfefe_fefe_fefe_fefe;
const NOT_H: u64 = 0x7f7f_7f7f_7f7f_7f7f;

const fn file_mask(f: i32) -> u64 {
    if f < 0 || f > 7 {
        0
    } else {
        FILE_A << f
    }
}

/// Squares in front of a pawn (same and adjacent files) that enemy pawns must not occupy
/// for it to be a passed pawn.
const fn passed_mask(color: usize, sq: usize) -> u64 {
    let f = (sq % 8) as i32;
    let r = (sq / 8) as i32;
    let files = file_mask(f - 1) | file_mask(f) | file_mask(f + 1);
    let mut ranks = 0u64;
    let mut rr = 0;
    while rr < 8 {
        if (color == 0 && rr > r) || (color == 1 && rr < r) {
            ranks |= 0xFFu64 << (rr * 8);
        }
        rr += 1;
    }
    files & ranks
}

static PASSED_MASK: [[u64; 64]; 2] = {
    let mut t = [[0u64; 64]; 2];
    let mut i = 0;
    while i < 64 {
        t[0][i] = passed_mask(0, i);
        t[1][i] = passed_mask(1, i);
        i += 1;
    }
    t
};

fn pawn_attacks(color: Color, pawns: u64) -> u64 {
    match color {
        Color::White => ((pawns << 7) & NOT_H) | ((pawns << 9) & NOT_A),
        Color::Black => ((pawns >> 7) & NOT_A) | ((pawns >> 9) & NOT_H),
    }
}

/// The squares one and two ranks in front of the king, on its file and the two next to it.
fn shield_zone(color: Color, king: Square) -> u64 {
    let f = file_of(king) as i32;
    let r = rank_of(king) as i32;
    let files = file_mask(f - 1) | file_mask(f) | file_mask(f + 1);
    let mut ranks = 0u64;
    for d in 1..=2 {
        let rr = if color == Color::White { r + d } else { r - d };
        if (0..8).contains(&rr) {
            ranks |= 0xFFu64 << (rr * 8);
        }
    }
    files & ranks
}

// ---------- The generic evaluation ----------

/// Receives "this color has `count` of feature `idx`".
pub trait Sink {
    fn add(&mut self, color: Color, idx: usize, count: i32);
}

/// Walks the position and reports every feature to the sink. Returns the game phase.
fn collect<K: Sink>(pos: &Position, sink: &mut K) -> i32 {
    let occ = pos.all();
    let pawns = [pos.pieces[0][PieceKind::Pawn.idx()], pos.pieces[1][PieceKind::Pawn.idx()]];
    let all_pawns = pawns[0] | pawns[1];
    let pawn_att = [pawn_attacks(Color::White, pawns[0]), pawn_attacks(Color::Black, pawns[1])];
    let mut phase = 0;

    for color in [Color::White, Color::Black] {
        let c = color.idx();
        let them = color.flip();
        let own = pos.occupancy[c];
        let enemy_king = pos.king_square(them);
        let king_zone = KING[enemy_king as usize] | bit(enemy_king);

        for kind in KINDS {
            let k = kind.idx();
            let mut bb = pos.pieces[c][k];
            while bb != 0 {
                let sq = pop_lsb(&mut bb);
                let table_idx = (if color == Color::White { sq ^ 56 } else { sq }) as usize;

                sink.add(color, MATERIAL + k, 1);
                sink.add(color, PST + k * 64 + table_idx, 1);
                phase += PHASE_WEIGHT[k];

                let attacks = match kind {
                    PieceKind::Knight => KNIGHT[sq as usize],
                    PieceKind::Bishop => bishop_attacks(sq, occ),
                    PieceKind::Rook => rook_attacks(sq, occ),
                    PieceKind::Queen => bishop_attacks(sq, occ) | rook_attacks(sq, occ),
                    _ => 0,
                };

                match kind {
                    PieceKind::Pawn => {
                        let f = file_of(sq) as i32;
                        if PASSED_MASK[c][sq as usize] & pawns[them.idx()] == 0 {
                            let rel_rank = if color == Color::White { rank_of(sq) } else { 7 - rank_of(sq) };
                            sink.add(color, PASSED + rel_rank as usize, 1);
                        }
                        if (file_mask(f - 1) | file_mask(f + 1)) & pawns[c] == 0 {
                            sink.add(color, ISOLATED, 1);
                        }
                    }
                    PieceKind::Knight | PieceKind::Bishop | PieceKind::Rook | PieceKind::Queen => {
                        let slot = k - 1; // knight=0 .. queen=3
                        // Safe squares: not our own pieces, not attacked by enemy pawns
                        let mobility = (attacks & !own & !pawn_att[them.idx()]).count_ones() as i32;
                        sink.add(color, MOBILITY + slot, mobility);

                        let king_hits = (attacks & king_zone).count_ones() as i32;
                        if king_hits > 0 {
                            sink.add(color, KING_ATTACK + slot, king_hits);
                        }

                        if kind == PieceKind::Rook {
                            let file = file_mask(file_of(sq) as i32);
                            if file & all_pawns == 0 {
                                sink.add(color, ROOK_OPEN, 1);
                            } else if file & pawns[c] == 0 {
                                sink.add(color, ROOK_SEMI_OPEN, 1);
                            }
                        }
                    }
                    PieceKind::King => {
                        let shield = (shield_zone(color, sq) & pawns[c]).count_ones() as i32;
                        if shield > 0 {
                            sink.add(color, KING_SHIELD, shield);
                        }
                    }
                }
            }
        }

        if pos.pieces[c][PieceKind::Bishop.idx()].count_ones() >= 2 {
            sink.add(color, BISHOP_PAIR, 1);
        }
        for f in 0..8 {
            let n = (pawns[c] & file_mask(f)).count_ones() as i32;
            if n > 1 {
                sink.add(color, DOUBLED, n - 1);
            }
        }
    }

    phase.min(MAX_PHASE) // extra queens from promotion can push it above 24
}

// ---------- Normal evaluation ----------

struct WeightSink {
    terms: [[S; 2]; N_TERMS],
}

impl Sink for WeightSink {
    fn add(&mut self, color: Color, idx: usize, count: i32) {
        let w = WEIGHTS[idx];
        let t = &mut self.terms[term_of(idx) as usize][color.idx()];
        t.mg += w.mg * count;
        t.eg += w.eg * count;
    }
}

fn taper(white: S, black: S, phase: i32) -> i32 {
    let mg = white.mg - black.mg;
    let eg = white.eg - black.eg;
    (mg * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE
}

/// Every term of the evaluation, from WHITE's point of view, in centipawns.
#[derive(Clone, Copy, Debug, Default)]
pub struct Breakdown {
    pub material: i32,
    pub piece_squares: i32,
    pub bishop_pair: i32,
    pub pawn_structure: i32,
    pub mobility: i32,
    pub rooks: i32,
    pub king_safety: i32,
    /// 24 = full middlegame, 0 = bare endgame.
    pub phase: i32,
    pub total: i32,
}

pub fn breakdown(pos: &Position) -> Breakdown {
    let mut sink = WeightSink { terms: [[S::default(); 2]; N_TERMS] };
    let phase = collect(pos, &mut sink);
    let t = |term: Term| {
        let [w, b] = sink.terms[term as usize];
        taper(w, b, phase)
    };
    let mut b = Breakdown {
        material: t(Term::Material),
        piece_squares: t(Term::PieceSquares),
        bishop_pair: t(Term::BishopPair),
        pawn_structure: t(Term::PawnStructure),
        mobility: t(Term::Mobility),
        rooks: t(Term::Rooks),
        king_safety: t(Term::KingSafety),
        phase,
        total: 0,
    };
    b.total = b.material + b.piece_squares + b.bishop_pair + b.pawn_structure + b.mobility + b.rooks + b.king_safety;
    b
}

/// Only the total, without sorting each weight into its term (faster; used by the search).
struct TotalSink {
    score: [S; 2],
}

impl Sink for TotalSink {
    #[inline]
    fn add(&mut self, color: Color, idx: usize, count: i32) {
        let w = WEIGHTS[idx];
        let t = &mut self.score[color.idx()];
        t.mg += w.mg * count;
        t.eg += w.eg * count;
    }
}

/// Score from the point of view of the side to move (what the search uses).
pub fn evaluate(pos: &Position) -> i32 {
    let mut sink = TotalSink { score: [S::default(); 2] };
    let phase = collect(pos, &mut sink);
    let total = taper(sink.score[0], sink.score[1], phase);
    if pos.side == Color::White {
        total
    } else {
        -total
    }
}

// ---------- Tuning support ----------

struct TraceSink {
    counts: [i32; N_PARAMS],
}

impl Sink for TraceSink {
    fn add(&mut self, color: Color, idx: usize, count: i32) {
        self.counts[idx] += if color == Color::White { count } else { -count };
    }
}

/// The position as (parameter index, White count minus Black count) pairs, plus its phase.
/// The evaluation equals the sum of count x weight, tapered by phase.
pub fn trace(pos: &Position) -> (Vec<(u16, i16)>, i32) {
    let mut sink = TraceSink { counts: [0; N_PARAMS] };
    let phase = collect(pos, &mut sink);
    let coefs = sink
        .counts
        .iter()
        .enumerate()
        .filter(|(_, &c)| c != 0)
        .map(|(i, &c)| (i as u16, c as i16))
        .collect();
    (coefs, phase)
}