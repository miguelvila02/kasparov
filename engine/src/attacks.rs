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

fn slide(s: Square, occ: Bitboard, dirs: &[(i8, i8)]) -> Bitboard {
    let mut attacks = 0;
    let (f0, r0) = (file_of(s) as i8, rank_of(s) as i8);
    for &(df, dr) in dirs {
        let (mut f, mut r) = (f0 + df, r0 + dr);
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let t = sq(f as u8, r as u8);
            attacks |= bit(t);
            if occ & bit(t) != 0 {
                break; // blocked
            }
            f += df;
            r += dr;
        }
    }
    attacks
}

pub fn bishop_attacks(s: Square, occ: Bitboard) -> Bitboard {
    slide(s, occ, &[(1, 1), (1, -1), (-1, 1), (-1, -1)])
}

pub fn rook_attacks(s: Square, occ: Bitboard) -> Bitboard {
    slide(s, occ, &[(1, 0), (-1, 0), (0, 1), (0, -1)])
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