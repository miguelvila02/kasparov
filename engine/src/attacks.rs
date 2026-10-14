use crate::board::Position;
use crate::types::*;

const NOT_A: u64 = 0xfefe_fefe_fefe_fefe;
const NOT_AB: u64 = 0xfcfc_fcfc_fcfc_fcfc;
const NOT_H: u64 = 0x7f7f_7f7f_7f7f_7f7f;
const NOT_GH: u64 = 0x3f3f_3f3f_3f3f_3f3f;

const fn knight_from(s: usize) -> u64 {
    let b = 1u64 << s;
    ((b << 17) & NOT_A) |
    ((b << 15) & NOT_H) |
    ((b << 10) & NOT_AB) |
    ((b << 6) & NOT_GH) |
    ((b >> 17) & NOT_H) |
    ((b >> 15) & NOT_A) |
    ((b >> 10) & NOT_GH) |
    ((b >> 6) & NOT_AB)
}

const fn king_from(s: usize) -> u64 {
    let b = 1u64 << s;
    let sides = ((b << 1) & NOT_A) | ((b >> 1) & NOT_H);
    let row = sides | b;
    (row | (row << 8) | (row >> 8)) & !b
}

const fn pawn_from(color: usize, s: usize) -> u64 {
    let b = 1u64 << s;
    if color == 0 {
        ((b << 7) & NOT_H) | ((b << 9) & NOT_A) // white captures upward
    } else {
        ((b >> 7) & NOT_A) | ((b >> 9) & NOT_H) // black captures downward
    }
}

pub static KNIGHT: [u64; 64] = {
    let mut t = [0; 64];
    let mut i = 0;
    while i < 64 { t[i] = knight_from(i); i += 1; }
    t
};

pub static KING: [u64; 64] = {
    let mut t = [0; 64];
    let mut i = 0;
    while i < 64 { t[i] = king_from(i); i += 1; }
    t
};

pub static PAWN_ATTACKS: [[u64; 64]; 2] = {
    let mut t = [[0; 64]; 2];
    let mut i = 0;
    while i < 64 {
        t[0][i] = pawn_from(0, i);
        t[1][i] = pawn_from(1, i);
        i += 1;
    }
    t
};

// ---------- Sliding pieces: precomputed rays ----------
// For each square and each of the 8 directions, all squares from there to the
// board edge. To get the attacks in one direction: take the ray, find the first
// blocker on it, and cut the ray off behind that blocker (by XOR-ing away the
// blocker's own ray in the same direction).

const DIRS: [(i8, i8); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];

const fn ray(sq: usize, df: i8, dr: i8) -> u64 {
    let mut r = 0u64;
    let mut f = (sq % 8) as i8 + df;
    let mut k = (sq / 8) as i8 + dr;
    while f >= 0 && f < 8 && k >= 0 && k < 8 {
        r |= 1u64 << (k * 8 + f);
        f += df;
        k += dr;
    }
    r
}

static RAYS: [[u64; 64]; 8] = {
    let mut t = [[0u64; 64]; 8];
    let mut d = 0;
    while d < 8 {
        let mut s = 0;
        while s < 64 {
            t[d][s] = ray(s, DIRS[d].0, DIRS[d].1);
            s += 1;
        }
        d += 1;
    }
    t
};

/// Does direction `d` increase the square number? Then the first blocker on the
/// ray is its lowest set bit; otherwise its highest.
const fn is_up(d: usize) -> bool {
    let (df, dr) = DIRS[d];
    dr > 0 || (dr == 0 && df > 0)
}

#[inline]
fn ray_attacks(d: usize, s: Square, occ: Bitboard) -> Bitboard {
    let r = RAYS[d][s as usize];
    let blockers = r & occ;
    if blockers == 0 {
        return r;
    }
    let first = if is_up(d) {
        blockers.trailing_zeros() as usize
    } else {
        63 - blockers.leading_zeros() as usize
    };
    r ^ RAYS[d][first]
}

pub fn bishop_attacks(s: Square, occ: Bitboard) -> Bitboard {
    ray_attacks(1, s, occ) | ray_attacks(3, s, occ) | ray_attacks(5, s, occ) | ray_attacks(7, s, occ)
}

pub fn rook_attacks(s: Square, occ: Bitboard) -> Bitboard {
    ray_attacks(0, s, occ) | ray_attacks(2, s, occ) | ray_attacks(4, s, occ) | ray_attacks(6, s, occ)
}

impl Position {
    pub fn is_square_attacked(&self, s: Square, by: Color) -> bool {
        let p = &self.pieces[by.idx()];
        let i = s as usize;

        // Pawns: a pawn of `by` attacks s if a pawn of the other color on s would attack it
        if PAWN_ATTACKS[by.flip().idx()][i] & p[PieceKind::Pawn.idx()] != 0 { return true; }
        if KNIGHT[i] & p[PieceKind::Knight.idx()] != 0 { return true; }
        if KING[i] & p[PieceKind::King.idx()] != 0 { return true; }

        let occ = self.all();
        let queens = p[PieceKind::Queen.idx()];
        if bishop_attacks(s, occ) & (p[PieceKind::Bishop.idx()] | queens) != 0 { return true; }
        if rook_attacks(s, occ) & (p[PieceKind::Rook.idx()] | queens) != 0 { return true; }
        false
    }

    pub fn king_square(&self, c: Color) -> Square {
        self.pieces[c.idx()][PieceKind::King.idx()].trailing_zeros() as Square
    }

    pub fn in_check(&self) -> bool {
        self.is_square_attacked(self.king_square(self.side), self.side.flip())
    }
}