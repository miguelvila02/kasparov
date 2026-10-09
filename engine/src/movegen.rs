use crate::attacks::*;
use crate::board::Position;
use crate::moves::{flag, Move};
use crate::types::*;

pub struct MoveList {
    moves: [Move; 256],
    len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        MoveList { moves: [Move::NULL; 256], len: 0 }
    }
    pub fn push(&mut self, m: Move) {
        self.moves[self.len] = m;
        self.len += 1;
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        self.moves[..self.len].iter()
    }
}

fn push_promotions(list: &mut MoveList, from: Square, to: Square, capture: bool) {
    let base = if capture { flag::PROMO_CAPTURE } else { flag::PROMO };
    for piece in 0..4 {
        list.push(Move::new(from, to, base | piece)); // knight, bishop, rook, queen
    }
}

impl Position {
    pub fn generate_pseudo(&self, list: &mut MoveList) {
        self.gen_pawns(list);
        self.gen_pieces(list);
        self.gen_castling(list);
    }

    fn gen_pawns(&self, list: &mut MoveList) {
        let us = self.side;
        let enemy = self.occupancy[us.flip().idx()];
        let occ = self.all();
        let (up, start_rank, promo_rank): (i8, u8, u8) =
            if us == Color::White { (8, 1, 7) } else { (-8, 6, 0) };

        let mut pawns = self.pieces[us.idx()][PieceKind::Pawn.idx()];
        while pawns != 0 {
            let from = pop_lsb(&mut pawns);

            // Pushes (a pawn is never on the last rank, so `one` is always on the board)
            let one = (from as i8 + up) as Square;
            if occ & bit(one) == 0 {
                if rank_of(one) == promo_rank {
                    push_promotions(list, from, one, false);
                } else {
                    list.push(Move::new(from, one, flag::QUIET));
                    if rank_of(from) == start_rank {
                        let two = (one as i8 + up) as Square;
                        if occ & bit(two) == 0 {
                            list.push(Move::new(from, two, flag::DOUBLE_PUSH));
                        }
                    }
                }
            }

            // Captures
            let attacks = PAWN_ATTACKS[us.idx()][from as usize];
            let mut caps = attacks & enemy;
            while caps != 0 {
                let to = pop_lsb(&mut caps);
                if rank_of(to) == promo_rank {
                    push_promotions(list, from, to, true);
                } else {
                    list.push(Move::new(from, to, flag::CAPTURE));
                }
            }

            // En passant
            if let Some(ep) = self.ep {
                if attacks & bit(ep) != 0 {
                    list.push(Move::new(from, ep, flag::EP_CAPTURE));
                }
            }
        }
    }

    fn gen_pieces(&self, list: &mut MoveList) {
        let us = self.side;
        let own = self.occupancy[us.idx()];
        let enemy = self.occupancy[us.flip().idx()];
        let occ = self.all();

        for kind in [PieceKind::Knight, PieceKind::Bishop, PieceKind::Rook, PieceKind::Queen, PieceKind::King] {
            let mut bb = self.pieces[us.idx()][kind.idx()];
            while bb != 0 {
                let from = pop_lsb(&mut bb);
                let attacks = match kind {
                    PieceKind::Knight => KNIGHT[from as usize],
                    PieceKind::Bishop => bishop_attacks(from, occ),
                    PieceKind::Rook => rook_attacks(from, occ),
                    PieceKind::Queen => bishop_attacks(from, occ) | rook_attacks(from, occ),
                    PieceKind::King => KING[from as usize],
                    PieceKind::Pawn => unreachable!(),
                };
                let mut targets = attacks & !own;
                while targets != 0 {
                    let to = pop_lsb(&mut targets);
                    let f = if enemy & bit(to) != 0 { flag::CAPTURE } else { flag::QUIET };
                    list.push(Move::new(from, to, f));
                }
            }
        }
    }

    fn gen_castling(&self, list: &mut MoveList) {
        if self.in_check() {
            return; // you can't castle out of check
        }
        let them = self.side.flip();
        let occ = self.all();
        let free = |squares: &[Square]| squares.iter().all(|&s| occ & bit(s) == 0);
        let safe = |squares: &[Square]| squares.iter().all(|&s| !self.is_square_attacked(s, them));

        if self.side == Color::White {
            // e1=4 f1=5 g1=6 | d1=3 c1=2 b1=1
            if self.castling & WK != 0 && free(&[5, 6]) && safe(&[5, 6]) {
                list.push(Move::new(4, 6, flag::KING_CASTLE));
            }
            if self.castling & WQ != 0 && free(&[1, 2, 3]) && safe(&[3, 2]) {
                list.push(Move::new(4, 2, flag::QUEEN_CASTLE));
            }
        } else {
            // e8=60 f8=61 g8=62 | d8=59 c8=58 b8=57
            if self.castling & BK != 0 && free(&[61, 62]) && safe(&[61, 62]) {
                list.push(Move::new(60, 62, flag::KING_CASTLE));
            }
            if self.castling & BQ != 0 && free(&[57, 58, 59]) && safe(&[59, 58]) {
                list.push(Move::new(60, 58, flag::QUEEN_CASTLE));
            }
        }
    }
}

impl Position {
    pub fn legal_moves(&self) -> MoveList {
        let mut pseudo = MoveList::new();
        self.generate_pseudo(&mut pseudo);

        let mut legal = MoveList::new();
        for &m in pseudo.iter() {
            let next = self.make_move(m);
            // after the move it's the opponent's turn: is OUR king attacked by them?
            if !next.is_square_attacked(next.king_square(self.side), next.side) {
                legal.push(m);
            }
        }
        legal
    }
}