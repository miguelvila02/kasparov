use crate::board::Position;
use crate::eval::{evaluate, MATE};
use crate::movegen::MoveList;
use crate::moves::{flag, Move};
use crate::time::now_ms;
use crate::types::*;

pub const MAX_PLY: usize = 64;
const INF: i32 = 32_000;

/// Values used only to order captures (most valuable victim, least valuable attacker).
const ORDER_VALUE: [i32; 6] = [100, 320, 330, 500, 900, 1000];

#[derive(Clone, Copy, Debug)]
pub struct SearchLimits {
    pub time_ms: f64,
    pub max_depth: u32,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    /// From the point of view of the side to move.
    pub score: i32,
    pub depth: u32,
    pub nodes: u64,
    pub pv: Vec<Move>,
    pub time_ms: f64,
}

pub struct Searcher {
    nodes: u64,
    start: f64,
    time_limit: f64,
    stopped: bool,
    /// Hashes of the positions before the current one: game history + current search path.
    history: Vec<u64>,
    pv_table: [[Move; MAX_PLY]; MAX_PLY],
    pv_len: [usize; MAX_PLY],
    prev_pv: Vec<Move>,
}

impl Searcher {
    pub fn new(game_history: Vec<u64>) -> Self {
        Searcher {
            nodes: 0,
            start: 0.0,
            time_limit: f64::INFINITY,
            stopped: false,
            history: game_history,
            pv_table: [[Move::NULL; MAX_PLY]; MAX_PLY],
            pv_len: [0; MAX_PLY],
            prev_pv: Vec::new(),
        }
    }

    /// Iterative deepening: depth 1, 2, 3… until time or max_depth runs out.
    pub fn search(&mut self, pos: &Position, limits: SearchLimits) -> SearchResult {
        self.start = now_ms();
        self.time_limit = limits.time_ms;
        self.nodes = 0;
        self.stopped = false;
        self.prev_pv.clear();

        let legal = pos.legal_moves();
        let mut result = SearchResult {
            best_move: legal.iter().next().copied(), // fallback if time is extremely short
            score: 0,
            depth: 0,
            nodes: 0,
            pv: Vec::new(),
            time_ms: 0.0,
        };

        // Zero or one legal move: nothing to think about
        if legal.len() > 1 {
            for depth in 1..=limits.max_depth.min(MAX_PLY as u32 - 4) {
                let score = self.negamax(pos, depth as i32, -INF, INF, 0);
                if self.stopped {
                    break; // unfinished depth: keep the previous result
                }

                let pv = self.pv_table[0][..self.pv_len[0]].to_vec();
                if let Some(&m) = pv.first() {
                    result.best_move = Some(m);
                }
                result.score = score;
                result.depth = depth;
                result.pv = pv.clone();
                self.prev_pv = pv;

                // A forced mate was found: searching deeper won't change the move
                if score.abs() >= MATE - MAX_PLY as i32 {
                    break;
                }
                // More than half the time used: the next depth wouldn't finish anyway
                if now_ms() - self.start > self.time_limit * 0.5 {
                    break;
                }
            }
        }

        result.nodes = self.nodes;
        result.time_ms = now_ms() - self.start;
        result
    }

    fn should_stop(&mut self) -> bool {
        if !self.stopped && self.nodes & 2047 == 0 && now_ms() - self.start >= self.time_limit {
            self.stopped = true;
        }
        self.stopped
    }

    fn is_repetition(&self, pos: &Position) -> bool {
        // A repetition can only go back as far as the last capture or pawn move
        let n = self.history.len();
        let lim = (pos.halfmove as usize).min(n);
        self.history[n - lim..].iter().any(|&h| h == pos.hash)
    }

    fn negamax(&mut self, pos: &Position, mut depth: i32, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.pv_len[ply] = ply;
        if self.should_stop() {
            return 0;
        }
        self.nodes += 1;

        if ply > 0 && (self.is_repetition(pos) || pos.insufficient_material()) {
            return 0; // draw
        }
        if ply >= MAX_PLY - 1 {
            return evaluate(pos);
        }

        let in_check = pos.in_check();
        if in_check {
            depth += 1; // check extension
        }
        if depth <= 0 {
            return self.quiescence(pos, alpha, beta, ply);
        }

        let mut moves = pos.legal_moves();
        if moves.is_empty() {
            return if in_check { -MATE + ply as i32 } else { 0 }; // checkmate or stalemate
        }
        if ply > 0 && pos.halfmove >= 100 {
            return 0; // 50-move rule (checked after mate, which takes priority)
        }
        self.order_moves(pos, &mut moves, ply);

        self.history.push(pos.hash);
        for &m in moves.iter() {
            let next = pos.make_move(m);
            let score = -self.negamax(&next, depth - 1, -beta, -alpha, ply + 1);
            if self.stopped {
                self.history.pop();
                return 0;
            }
            if score > alpha {
                alpha = score;
                // New best move: this move + the child's line becomes our line
                self.pv_table[ply][ply] = m;
                for i in (ply + 1)..self.pv_len[ply + 1] {
                    self.pv_table[ply][i] = self.pv_table[ply + 1][i];
                }
                self.pv_len[ply] = self.pv_len[ply + 1];

                if alpha >= beta {
                    break; // cutoff: the opponent won't allow this position
                }
            }
        }
        self.history.pop();
        alpha
    }

    /// Keep searching captures (and queen promotions) until the position is quiet.
    fn quiescence(&mut self, pos: &Position, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.pv_len[ply] = ply;
        if self.should_stop() {
            return 0;
        }
        self.nodes += 1;

        // "Stand pat": we can usually just stop capturing, so the static eval is a lower bound
        let stand_pat = evaluate(pos);
        if ply >= MAX_PLY - 1 || stand_pat >= beta {
            return stand_pat;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let mut moves = MoveList::new();
        for &m in pos.legal_moves().iter() {
            if m.is_capture() || m.promotion_piece() == Some(PieceKind::Queen) {
                moves.push(m);
            }
        }
        self.order_moves(pos, &mut moves, ply);

        for &m in moves.iter() {
            let score = -self.quiescence(&pos.make_move(m), -beta, -alpha, ply + 1);
            if self.stopped {
                return 0;
            }
            if score > alpha {
                alpha = score;
                if alpha >= beta {
                    break;
                }
            }
        }
        alpha
    }

    fn order_moves(&self, pos: &Position, moves: &mut MoveList, ply: usize) {
        let hint = self.prev_pv.get(ply).copied();
        moves
            .as_mut_slice()
            .sort_unstable_by_key(|&m| std::cmp::Reverse(score_move(pos, m, hint)));
    }
}

/// Higher = searched earlier.
fn score_move(pos: &Position, m: Move, hint: Option<Move>) -> i32 {
    if Some(m) == hint {
        return 1_000_000; // the previous iteration's line comes first
    }
    let mut s = 0;
    if m.is_capture() {
        let victim = if m.flags() == flag::EP_CAPTURE {
            PieceKind::Pawn
        } else {
            pos.piece_at(m.to()).expect("capture target").1
        };
        let attacker = pos.piece_at(m.from()).expect("moving piece").1;
        s += 100_000 + ORDER_VALUE[victim.idx()] * 10 - ORDER_VALUE[attacker.idx()];
    }
    if let Some(p) = m.promotion_piece() {
        s += 90_000 + ORDER_VALUE[p.idx()];
    }
    s
}