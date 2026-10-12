//! Phase 6 search: alpha-beta + transposition table, PVS, null move pruning,
//! late move reductions, reverse futility pruning, killers/history, aspiration windows.
//!
//! Each technique has an on/off switch below, so you can measure exactly what it
//! is worth with fastchess (build once with it off, once with it on, and compare).

use crate::board::Position;
use crate::eval::{evaluate, MATE};
use crate::movegen::MoveList;
use crate::moves::{flag, Move};
use crate::time::now_ms;
use crate::tt::{Bound, TranspositionTable};
use crate::types::*;

// ---------- Feature switches ----------
const USE_TT: bool = true;
const USE_NULL_MOVE: bool = true;
const USE_LMR: bool = true;
const USE_RFP: bool = true; // reverse futility pruning
const USE_KILLERS_HISTORY: bool = true;
const USE_ASPIRATION: bool = true;

pub const MAX_PLY: usize = 64;
const INF: i32 = 32_000;
/// Scores beyond this are mate scores.
const MATE_BOUND: i32 = MATE - MAX_PLY as i32;
const TT_MB: usize = 16;

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
    rep_history: Vec<u64>,
    tt: TranspositionTable,
    pv_table: [[Move; MAX_PLY]; MAX_PLY],
    pv_len: [usize; MAX_PLY],
    /// Two quiet moves per ply that recently caused a cutoff.
    killers: [[Move; 2]; MAX_PLY],
    /// [color][from][to]: how often a quiet move caused a cutoff, weighted by depth.
    hist: [[[i32; 64]; 64]; 2],
    /// Late move reduction amounts, indexed by [depth][move number].
    lmr: [[i32; 64]; 64],
}

impl Searcher {
    pub fn new(game_history: Vec<u64>) -> Self {
        let mut lmr = [[0i32; 64]; 64];
        for (d, row) in lmr.iter_mut().enumerate().skip(1) {
            for (i, r) in row.iter_mut().enumerate().skip(1) {
                *r = (0.75 + (d as f64).ln() * (i as f64).ln() / 2.25) as i32;
            }
        }
        Searcher {
            nodes: 0,
            start: 0.0,
            time_limit: f64::INFINITY,
            stopped: false,
            rep_history: game_history,
            tt: TranspositionTable::new(TT_MB),
            pv_table: [[Move::NULL; MAX_PLY]; MAX_PLY],
            pv_len: [0; MAX_PLY],
            killers: [[Move::NULL; 2]; MAX_PLY],
            hist: [[[0; 64]; 64]; 2],
            lmr,
        }
    }

    /// Replaces the game history (positions before the one being searched).
    pub fn set_history(&mut self, game_history: Vec<u64>) {
        self.rep_history = game_history;
    }

    /// Forgets everything learned (new game).
    pub fn clear(&mut self) {
        self.tt.clear();
    }

    /// Iterative deepening with aspiration windows.
    pub fn search(&mut self, pos: &Position, limits: SearchLimits) -> SearchResult {
        self.start = now_ms();
        self.time_limit = limits.time_ms;
        self.nodes = 0;
        self.stopped = false;
        self.killers = [[Move::NULL; 2]; MAX_PLY];
        self.hist = [[[0; 64]; 64]; 2];

        let legal = pos.legal_moves();
        let mut result = SearchResult {
            best_move: legal.iter().next().copied(), // fallback if time is extremely short
            score: 0,
            depth: 0,
            nodes: 0,
            pv: Vec::new(),
            time_ms: 0.0,
        };

        if legal.len() > 1 {
            let mut prev_score = 0;
            for depth in 1..=limits.max_depth.min(MAX_PLY as u32 - 4) {
                let d = depth as i32;
                let mut delta = 25;
                let (mut alpha, mut beta) = if USE_ASPIRATION && depth >= 4 {
                    (prev_score - delta, prev_score + delta)
                } else {
                    (-INF, INF)
                };

                // Search with a narrow window around the last score; widen if it falls outside
                let score = loop {
                    let s = self.negamax(pos, d, alpha, beta, 0, true);
                    if self.stopped {
                        break s;
                    }
                    if s <= alpha {
                        beta = (alpha + beta) / 2;
                        alpha = (s - delta).max(-INF);
                    } else if s >= beta {
                        beta = (s + delta).min(INF);
                    } else {
                        break s;
                    }
                    delta += delta / 2;
                    if delta > 1000 {
                        alpha = -INF;
                        beta = INF;
                    }
                };
                if self.stopped {
                    break; // unfinished depth: keep the previous result
                }
                prev_score = score;

                let pv = self.pv_table[0][..self.pv_len[0]].to_vec();
                if let Some(&m) = pv.first() {
                    result.best_move = Some(m);
                }
                result.score = score;
                result.depth = depth;
                result.pv = pv;

                if score.abs() >= MATE_BOUND {
                    break; // forced mate found
                }
                if now_ms() - self.start > self.time_limit * 0.5 {
                    break; // the next depth wouldn't finish in time
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
        let n = self.rep_history.len();
        let lim = (pos.halfmove as usize).min(n);
        self.rep_history[n - lim..].iter().any(|&h| h == pos.hash)
    }

    fn negamax(
        &mut self,
        pos: &Position,
        mut depth: i32,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        allow_null: bool,
    ) -> i32 {
        self.pv_len[ply] = ply;
        if self.should_stop() {
            return 0;
        }
        self.nodes += 1;

        let is_root = ply == 0;
        let pv_node = beta - alpha > 1;

        if !is_root && (self.is_repetition(pos) || pos.insufficient_material()) {
            return 0;
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

        // ---- Transposition table ----
        let mut tt_move = Move::NULL;
        if USE_TT {
            if let Some(e) = self.tt.probe(pos.hash) {
                tt_move = e.mv;
                if !pv_node && !is_root && e.depth as i32 >= depth {
                    let s = score_from_tt(e.score, ply);
                    match e.bound {
                        Bound::Exact => return s,
                        Bound::Lower if s >= beta => return s,
                        Bound::Upper if s <= alpha => return s,
                        _ => {}
                    }
                }
            }
        }

        let static_eval = if in_check { -INF } else { evaluate(pos) };

        // ---- Reverse futility pruning ----
        // Shallow depth and we're already far above beta: assume it stays that way.
        if USE_RFP
            && !pv_node
            && !in_check
            && depth <= 6
            && beta.abs() < MATE_BOUND
            && static_eval - 80 * depth >= beta
        {
            return static_eval;
        }

        // ---- Null move pruning ----
        // Give the opponent a free move. If we're still above beta, this position
        // is so good the opponent would never allow it: cut.
        if USE_NULL_MOVE
            && allow_null
            && !pv_node
            && !in_check
            && depth >= 3
            && static_eval >= beta
            && beta.abs() < MATE_BOUND
            && has_non_pawn_material(pos) // avoid zugzwang endgames
        {
            let r = 3 + depth / 6;
            let null_pos = pos.make_null_move();
            self.rep_history.push(pos.hash);
            let score = -self.negamax(&null_pos, depth - 1 - r, -beta, -beta + 1, ply + 1, false);
            self.rep_history.pop();
            if self.stopped {
                return 0;
            }
            if score >= beta {
                return if score >= MATE_BOUND { beta } else { score };
            }
        }

        // ---- Move loop ----
        let mut moves = pos.legal_moves();
        if moves.is_empty() {
            return if in_check { -MATE + ply as i32 } else { 0 };
        }
        if !is_root && pos.halfmove >= 100 {
            return 0;
        }
        self.order_moves(pos, &mut moves, ply, tt_move);

        let orig_alpha = alpha;
        let mut best_score = -INF;
        let mut best_move = Move::NULL;

        self.rep_history.push(pos.hash);
        for (i, &m) in moves.iter().enumerate() {
            let quiet = !m.is_capture() && !m.is_promotion();
            let next = pos.make_move(m);
            let gives_check = next.in_check();

            let mut score;
            if i == 0 {
                // First move (usually the best): full window
                score = -self.negamax(&next, depth - 1, -beta, -alpha, ply + 1, true);
            } else {
                // Late move reductions: quiet moves late in the list are probably bad
                let mut reduction = 0;
                if USE_LMR && depth >= 3 && i >= 3 && quiet && !in_check && !gives_check {
                    reduction = self.lmr[(depth as usize).min(63)][i.min(63)];
                    if pv_node {
                        reduction -= 1;
                    }
                    reduction = reduction.clamp(0, depth - 2);
                }

                // PVS: prove this move is worse than the best so far with a zero-width window
                score = -self.negamax(&next, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, true);
                if score > alpha && reduction > 0 {
                    // The reduced search was surprised: retry at full depth
                    score = -self.negamax(&next, depth - 1, -alpha - 1, -alpha, ply + 1, true);
                }
                if score > alpha && score < beta {
                    // It really is better: get its exact score
                    score = -self.negamax(&next, depth - 1, -beta, -alpha, ply + 1, true);
                }
            }

            if self.stopped {
                self.rep_history.pop();
                return 0;
            }

            if score > best_score {
                best_score = score;
                if score > alpha {
                    alpha = score;
                    best_move = m;

                    self.pv_table[ply][ply] = m;
                    for j in (ply + 1)..self.pv_len[ply + 1] {
                        self.pv_table[ply][j] = self.pv_table[ply + 1][j];
                    }
                    self.pv_len[ply] = self.pv_len[ply + 1];

                    if alpha >= beta {
                        if quiet && USE_KILLERS_HISTORY {
                            self.record_quiet_cutoff(pos, m, ply, depth);
                        }
                        break;
                    }
                }
            }
        }
        self.rep_history.pop();

        if USE_TT {
            let bound = if best_score >= beta {
                Bound::Lower
            } else if best_score > orig_alpha {
                Bound::Exact
            } else {
                Bound::Upper
            };
            self.tt.store(pos.hash, best_move, score_to_tt(best_score, ply), depth, bound);
        }
        best_score
    }

    /// Keep searching captures (and queen promotions) until the position is quiet.
    fn quiescence(&mut self, pos: &Position, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.pv_len[ply] = ply;
        if self.should_stop() {
            return 0;
        }
        self.nodes += 1;

        let stand_pat = evaluate(pos);
        if ply >= MAX_PLY - 1 || stand_pat >= beta {
            return stand_pat;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Only generate pseudo-legal captures; check legality one by one (much cheaper
        // than generating every legal move just to throw the quiet ones away)
        let mut pseudo = MoveList::new();
        pos.generate_pseudo(&mut pseudo);
        let mut moves = MoveList::new();
        for &m in pseudo.iter() {
            if m.is_capture() || m.promotion_piece() == Some(PieceKind::Queen) {
                moves.push(m);
            }
        }
        self.order_moves(pos, &mut moves, ply, Move::NULL);

        let mut best = stand_pat;
        for &m in moves.iter() {
            let next = pos.make_move(m);
            if next.is_square_attacked(next.king_square(pos.side), next.side) {
                continue; // illegal: leaves our king in check
            }
            let score = -self.quiescence(&next, -beta, -alpha, ply + 1);
            if self.stopped {
                return 0;
            }
            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }
        best
    }

    fn record_quiet_cutoff(&mut self, pos: &Position, m: Move, ply: usize, depth: i32) {
        if self.killers[ply][0] != m {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = m;
        }
        let h = &mut self.hist[pos.side.idx()][m.from() as usize][m.to() as usize];
        *h += depth * depth;
        if *h > 400_000 {
            // Keep history scores well below the killer scores
            for side in self.hist.iter_mut() {
                for from in side.iter_mut() {
                    for v in from.iter_mut() {
                        *v /= 2;
                    }
                }
            }
        }
    }

    fn order_moves(&self, pos: &Position, moves: &mut MoveList, ply: usize, tt_move: Move) {
        moves
            .as_mut_slice()
            .sort_unstable_by_key(|&m| std::cmp::Reverse(self.score_move(pos, m, ply, tt_move)));
    }

    /// Higher = searched earlier.
    fn score_move(&self, pos: &Position, m: Move, ply: usize, tt_move: Move) -> i32 {
        if m == tt_move {
            return 2_000_000;
        }
        if m.is_capture() {
            let victim = if m.flags() == flag::EP_CAPTURE {
                PieceKind::Pawn
            } else {
                pos.piece_at(m.to()).expect("capture target").1
            };
            let attacker = pos.piece_at(m.from()).expect("moving piece").1;
            let promo = m.promotion_piece().map_or(0, |p| ORDER_VALUE[p.idx()]);
            return 1_000_000 + ORDER_VALUE[victim.idx()] * 10 - ORDER_VALUE[attacker.idx()] + promo;
        }
        if let Some(p) = m.promotion_piece() {
            return 900_000 + ORDER_VALUE[p.idx()];
        }
        if USE_KILLERS_HISTORY {
            if self.killers[ply][0] == m {
                return 800_000;
            }
            if self.killers[ply][1] == m {
                return 790_000;
            }
            return self.hist[pos.side.idx()][m.from() as usize][m.to() as usize];
        }
        0
    }
}

fn has_non_pawn_material(pos: &Position) -> bool {
    let p = &pos.pieces[pos.side.idx()];
    (p[PieceKind::Knight.idx()] | p[PieceKind::Bishop.idx()] | p[PieceKind::Rook.idx()] | p[PieceKind::Queen.idx()])
        != 0
}

/// Mate scores are stored relative to the stored position, not to the root,
/// so they stay correct when the same position is found at a different depth.
fn score_to_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_BOUND {
        score + ply as i32
    } else if score <= -MATE_BOUND {
        score - ply as i32
    } else {
        score
    }
}

fn score_from_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_BOUND {
        score - ply as i32
    } else if score <= -MATE_BOUND {
        score + ply as i32
    } else {
        score
    }
}