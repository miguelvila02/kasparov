//! Phase 9: explains moves in plain English, using only facts the engine computed.
//!
//! Nothing here guesses: every sentence comes from a concrete check on the board
//! (what was captured, which pieces are attacked or defended, what the search expects
//! next, which evaluation terms improved). The text is built from templates.

use crate::attacks::{bishop_attacks, rook_attacks, KING, KNIGHT, PAWN_ATTACKS};
use crate::board::Position;
use crate::eval::{
    trace, BISHOP_PAIR, DOUBLED, ISOLATED, KING_ATTACK, KING_SHIELD, MATE, MAX_PHASE, MOBILITY, PASSED, PST,
    ROOK_OPEN, ROOK_SEMI_OPEN, WEIGHTS,
};
use crate::moves::{flag, Move};
use crate::types::*;

/// Simple piece values used for explanations (not the tuned evaluation values).
const VALUE: [i32; 6] = [100, 300, 300, 500, 900, 20_000];
const MATE_BOUND: i32 = MATE - 100;

fn name(k: PieceKind) -> &'static str {
    match k {
        PieceKind::Pawn => "pawn",
        PieceKind::Knight => "knight",
        PieceKind::Bishop => "bishop",
        PieceKind::Rook => "rook",
        PieceKind::Queen => "queen",
        PieceKind::King => "king",
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// ---------- Board facts ----------

/// All pieces of `by` that attack `sq`.
fn attackers(pos: &Position, sq: Square, by: Color) -> Bitboard {
    let occ = pos.all();
    let p = &pos.pieces[by.idx()];
    let i = sq as usize;
    (PAWN_ATTACKS[by.flip().idx()][i] & p[PieceKind::Pawn.idx()])
        | (KNIGHT[i] & p[PieceKind::Knight.idx()])
        | (KING[i] & p[PieceKind::King.idx()])
        | (bishop_attacks(sq, occ) & (p[PieceKind::Bishop.idx()] | p[PieceKind::Queen.idx()]))
        | (rook_attacks(sq, occ) & (p[PieceKind::Rook.idx()] | p[PieceKind::Queen.idx()]))
}

fn lowest_value(pos: &Position, mut bb: Bitboard) -> i32 {
    let mut best = i32::MAX;
    while bb != 0 {
        let s = pop_lsb(&mut bb);
        if let Some((_, k)) = pos.piece_at(s) {
            best = best.min(VALUE[k.idx()]);
        }
    }
    best
}

/// Squares attacked by the piece standing on `sq`.
fn piece_attacks(pos: &Position, sq: Square) -> Bitboard {
    let occ = pos.all();
    match pos.piece_at(sq) {
        Some((c, PieceKind::Pawn)) => PAWN_ATTACKS[c.idx()][sq as usize],
        Some((_, PieceKind::Knight)) => KNIGHT[sq as usize],
        Some((_, PieceKind::Bishop)) => bishop_attacks(sq, occ),
        Some((_, PieceKind::Rook)) => rook_attacks(sq, occ),
        Some((_, PieceKind::Queen)) => bishop_attacks(sq, occ) | rook_attacks(sq, occ),
        Some((_, PieceKind::King)) => KING[sq as usize],
        None => 0,
    }
}

/// Pieces of the other side that `attacker` could win right now: attacked and either
/// undefended or attacked by something cheaper. (A simple test that ignores x-rays.)
fn targets(pos: &Position, attacker: Color) -> Vec<(Square, PieceKind)> {
    let victim = attacker.flip();
    let mut out = Vec::new();
    for kind in [PieceKind::Pawn, PieceKind::Knight, PieceKind::Bishop, PieceKind::Rook, PieceKind::Queen] {
        let mut bb = pos.pieces[victim.idx()][kind.idx()];
        while bb != 0 {
            let sq = pop_lsb(&mut bb);
            let a = attackers(pos, sq, attacker);
            if a == 0 {
                continue;
            }
            if attackers(pos, sq, victim) == 0 || lowest_value(pos, a) < VALUE[kind.idx()] {
                out.push((sq, kind));
            }
        }
    }
    out.sort_by_key(|&(_, k)| std::cmp::Reverse(VALUE[k.idx()]));
    out
}

/// Material from `side`'s point of view (simple values, kings excluded).
fn material(pos: &Position, side: Color) -> i32 {
    let mut total = 0;
    for k in 0..5 {
        total += VALUE[k] * pos.pieces[side.idx()][k].count_ones() as i32;
        total -= VALUE[k] * pos.pieces[side.flip().idx()][k].count_ones() as i32;
    }
    total
}

fn is_passed(pos: &Position, sq: Square, c: Color) -> bool {
    let f = file_of(sq) as i32;
    let r = rank_of(sq) as i32;
    let mut enemy = pos.pieces[c.flip().idx()][PieceKind::Pawn.idx()];
    while enemy != 0 {
        let e = pop_lsb(&mut enemy);
        let (ef, er) = (file_of(e) as i32, rank_of(e) as i32);
        let ahead = if c == Color::White { er > r } else { er < r };
        if (ef - f).abs() <= 1 && ahead {
            return false;
        }
    }
    true
}

fn file_has_pawns(pos: &Position, file: u8) -> bool {
    let mask = 0x0101_0101_0101_0101u64 << file;
    (pos.pieces[0][PieceKind::Pawn.idx()] | pos.pieces[1][PieceKind::Pawn.idx()]) & mask != 0
}

/// SAN for a sequence of moves starting at `pos`, with move numbers: "12...Nf6 13.Bd3".
pub fn line_san(pos: &Position, line: &[Move]) -> String {
    let mut p = *pos;
    let mut out = Vec::new();
    for (i, &m) in line.iter().enumerate() {
        if p.parse_uci(&m.to_uci()).is_none() {
            break; // safety: never describe an illegal move
        }
        let num = p.fullmove;
        let label = if p.side == Color::White {
            format!("{num}.{}", p.san(m))
        } else if i == 0 {
            format!("{num}...{}", p.san(m))
        } else {
            p.san(m)
        };
        out.push(label);
        p = p.make_move(m);
    }
    out.join(" ")
}

// ---------- Positional changes (from the Phase 7 evaluation terms) ----------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Group {
    Placement,
    BishopPair,
    Passed,
    Structure,
    Mobility,
    Rooks,
    Shield,
    KingAttack,
}

fn group_of(idx: usize) -> Option<Group> {
    Some(match idx {
        i if i >= PST && i < BISHOP_PAIR => Group::Placement,
        i if i == BISHOP_PAIR => Group::BishopPair,
        i if i >= PASSED && i < ISOLATED => Group::Passed,
        i if i == ISOLATED || i == DOUBLED => Group::Structure,
        i if i >= MOBILITY && i < ROOK_OPEN => Group::Mobility,
        i if i == ROOK_OPEN || i == ROOK_SEMI_OPEN => Group::Rooks,
        i if i == KING_SHIELD => Group::Shield,
        i if i >= KING_ATTACK && i < KING_ATTACK + 4 => Group::KingAttack,
        _ => return None, // material is described separately
    })
}

const GROUPS: [Group; 8] = [
    Group::Placement,
    Group::BishopPair,
    Group::Passed,
    Group::Structure,
    Group::Mobility,
    Group::Rooks,
    Group::Shield,
    Group::KingAttack,
];

/// Value of each group for White, in centipawns.
fn group_scores(pos: &Position) -> [i32; 8] {
    let (coefs, phase) = trace(pos);
    let mut mg = [0i64; 8];
    let mut eg = [0i64; 8];
    for (i, c) in coefs {
        if let Some(g) = group_of(i as usize) {
            let gi = GROUPS.iter().position(|&x| x == g).unwrap();
            mg[gi] += WEIGHTS[i as usize].mg as i64 * c as i64;
            eg[gi] += WEIGHTS[i as usize].eg as i64 * c as i64;
        }
    }
    let mut out = [0; 8];
    for g in 0..8 {
        out[g] = ((mg[g] * phase as i64 + eg[g] * (MAX_PHASE - phase) as i64) / MAX_PHASE as i64) as i32;
    }
    out
}

/// The evaluation group that improved the most for the mover, if the gain is noticeable.
fn best_positional_gain(before: &Position, after: &Position, mover: Color) -> Option<Group> {
    let (b, a) = (group_scores(before), group_scores(after));
    let sign = if mover == Color::White { 1 } else { -1 };
    let (mut best, mut best_gain) = (None, 12); // ignore tiny changes
    for g in 0..8 {
        let gain = (a[g] - b[g]) * sign;
        if gain > best_gain {
            best_gain = gain;
            best = Some(GROUPS[g]);
        }
    }
    best
}

// ---------- Describing one move ----------

/// What a move does, as a verb phrase ("wins the knight on f6"), plus extra notes.
struct Facts {
    lead: String,
    extras: Vec<String>,
    /// True when the move wins, trades or threatens real material (or mates).
    material: bool,
}

impl Facts {
    fn new(lead: impl Into<String>, extras: Vec<String>, material: bool) -> Facts {
        Facts { lead: lead.into(), extras, material }
    }
}

const CENTER: [Square; 4] = [27, 28, 35, 36]; // d4 e4 d5 e5

/// `prev` = the opponent's move that led to `pos` (to recognize recaptures).
fn describe(pos: &Position, m: Move, line_after: &[Move], prev: Option<Move>) -> Facts {
    let us = pos.side;
    let them = us.flip();
    let (from, to) = (m.from(), m.to());
    let (_, piece) = pos.piece_at(from).expect("moving piece");
    let after = pos.make_move(m);
    let gives_check = after.in_check();
    let mut extras = Vec::new();

    // Material along the expected line, measured only where the line has "settled"
    // (the next move is not a capture), so a line cut off mid-exchange doesn't count.
    let base = material(pos, us);
    let mut settled_gain = None;
    let mut end = after;
    let shown: Vec<Move> = line_after.iter().copied().take(8).collect();
    for (i, &mv) in shown.iter().enumerate() {
        if i == 0 && !mv.is_capture() {
            settled_gain = Some(material(&after, us) - base);
        }
        if end.parse_uci(&mv.to_uci()).is_none() {
            break;
        }
        end = end.make_move(mv);
        if shown.get(i + 1).map_or(false, |n| !n.is_capture()) {
            settled_gain = Some(material(&end, us) - base);
        }
    }
    if shown.is_empty() && !m.is_capture() {
        settled_gain = Some(material(&after, us) - base);
    }
    let line_gain = settled_gain.unwrap_or(0);
    let mating_line = end.in_check() && end.legal_moves().is_empty();

    let threats_before = targets(pos, us);
    let threats_after: Vec<_> = targets(&after, us);
    let new_threats: Vec<_> = threats_after
        .iter()
        .copied()
        .filter(|t| !threats_before.contains(t))
        .collect();

    // 1. Checkmate
    if gives_check && after.legal_moves().is_empty() {
        return Facts::new("delivers checkmate", extras, true);
    }

    // 2. Promotion
    if let Some(p) = m.promotion_piece() {
        let lead = format!(
            "promotes the pawn to a {}{}",
            name(p),
            if gives_check { " with check" } else { "" }
        );
        return Facts::new(lead, extras, true);
    }

    // 3. Captures
    if m.is_capture() {
        let victim = if m.flags() == flag::EP_CAPTURE {
            PieceKind::Pawn
        } else {
            pos.piece_at(to).expect("captured piece").1
        };
        let recaptured = line_after.first().map_or(false, |r| r.to() == to && r.is_capture());
        let is_recapture = prev.map_or(false, |p| p.to() == to && p.is_capture());
        let (vv, pv) = (VALUE[victim.idx()], VALUE[piece.idx()]);
        let sq = square_name(to);
        // Judge the capture by the whole expected line, not just the first exchange
        let ahead = |gain: i32| {
            let p = (gain as f64 / 100.0).round() as i32;
            format!("about {p} pawn{} ahead", if p == 1 { "" } else { "s" })
        };
        let lead = if m.flags() == flag::EP_CAPTURE {
            "captures the pawn en passant".to_string()
        } else if mating_line {
            format!("takes the {} on {sq} and starts a forced mating attack", name(victim))
        } else if is_recapture && !recaptured {
            format!("recaptures the {} on {sq}", name(victim))
        } else if !recaptured && line_gain > -80 {
            format!("wins the {} on {sq}", name(victim))
        } else if line_gain >= 80 && vv <= pv {
            format!("takes the {} on {sq}: after the exchanges it comes out {}", name(victim), ahead(line_gain))
        } else if line_gain >= 80 {
            format!("wins the {} on {sq} for a {}", name(victim), name(piece))
        } else if line_gain > -80 {
            if victim == piece {
                format!("trades {}s on {sq}", name(piece))
            } else {
                format!("trades the {} for the {} on {sq}", name(piece), name(victim))
            }
        } else {
            format!("gives up the {} for the {} on {sq} to gain activity", name(piece), name(victim))
        };
        if gives_check {
            extras.push("It comes with check.".into());
        }
        // Plain trades and recaptures don't win anything
        let wins = mating_line || (line_gain.abs() >= 80) || (!recaptured && !is_recapture);
        return Facts::new(lead, extras, wins && m.flags() != flag::EP_CAPTURE);
    }

    // 4. Castling
    if m.flags() == flag::KING_CASTLE || m.flags() == flag::QUEEN_CASTLE {
        let side = if m.flags() == flag::KING_CASTLE { "kingside" } else { "queenside" };
        return Facts::new(format!("castles {side}: the king gets to safety and the rook joins the game"), extras, false);
    }

    // 4b. Answering a check
    if pos.in_check() {
        let lead = if piece == PieceKind::King {
            "moves the king out of check".to_string()
        } else {
            format!("blocks the check with the {}", name(piece))
        };
        return Facts::new(lead, extras, false);
    }

    // 5. Forks: the moved piece attacks two valuable targets (or the king and a target)
    let hits = piece_attacks(&after, to);
    let mut forked: Vec<PieceKind> = new_threats
        .iter()
        .filter(|&&(sq, k)| hits & bit(sq) != 0 && VALUE[k.idx()] >= 300)
        .map(|&(_, k)| k)
        .collect();
    if gives_check && hits & bit(after.king_square(them)) != 0 {
        forked.insert(0, PieceKind::King);
    }
    if forked.len() >= 2 {
        let lead = format!(
            "forks the {} and the {} with the {}",
            name(forked[0]),
            name(forked[1]),
            name(piece)
        );
        return Facts::new(lead, extras, true);
    }

    // 6. A forced mate, or material won by a combination that doesn't start with a capture
    if mating_line {
        let lead = if gives_check { "gives check and starts a forced mating attack" } else { "starts a forced mating attack" };
        return Facts::new(lead, extras, true);
    }
    let already_hanging = threats_before.first().map_or(0, |&(_, k)| VALUE[k.idx()]);
    if line_gain >= 150 && line_gain >= already_hanging + 150 {
        let pawns = (line_gain as f64 / 100.0).round() as i32;
        let lead = format!(
            "starts a combination that wins material (about {pawns} pawn{} of value)",
            if pawns == 1 { "" } else { "s" }
        );
        return Facts::new(lead, extras, true);
    }

    // 7. Sacrifice
    if line_gain <= -200 {
        let pawns = (-line_gain as f64 / 100.0).round() as i32;
        let lead = format!(
            "gives up material (about {pawns} pawn{} of value) in return for activity",
            if pawns == 1 { "" } else { "s" }
        );
        return Facts::new(lead, extras, true);
    }

    // 9. Checks and threats
    if gives_check {
        let mut lead = "gives check".to_string();
        let mut big = false;
        if let Some(&(sq, k)) = new_threats.first() {
            lead += &format!(" and attacks the {} on {}", name(k), square_name(sq));
            big = VALUE[k.idx()] >= 300;
        }
        return Facts::new(lead, extras, big);
    }
    if let Some(&(sq, k)) = new_threats.first() {
        return Facts::new(format!("attacks the {} on {}", name(k), square_name(sq)), extras, VALUE[k.idx()] >= 300);
    }

    // 10. Saving or defending our own pieces
    let ours_before = targets(pos, them);
    let ours_after = targets(&after, them);
    if ours_before.iter().any(|&(sq, _)| sq == from) && !ours_after.iter().any(|&(sq, _)| sq == to) {
        return Facts::new(format!("moves the attacked {} to safety", name(piece)), extras, false);
    }
    if let Some(&(sq, k)) = ours_before.iter().find(|t| t.0 != from && !ours_after.contains(t)) {
        return Facts::new(format!("defends the {} on {}", name(k), square_name(sq)), extras, false);
    }

    // 11. Development, center, passed pawns, open files
    let back_rank = if us == Color::White { 0 } else { 7 };
    if matches!(piece, PieceKind::Knight | PieceKind::Bishop) && rank_of(from) == back_rank {
        return Facts::new(format!("develops the {} to {}", name(piece), square_name(to)), extras, false);
    }
    if piece == PieceKind::Pawn {
        if is_passed(&after, to, us) {
            let rel = if us == Color::White { rank_of(to) } else { 7 - rank_of(to) };
            if rel >= 3 {
                return Facts::new(format!("pushes the passed pawn to {}, closer to promotion", square_name(to)), extras, false);
            }
        }
        // What the pawn now protects
        let mut guarded = piece_attacks(&after, to) & after.occupancy[us.idx()];
        let mut best_guard: Option<(Square, PieceKind)> = None;
        while guarded != 0 {
            let sq = pop_lsb(&mut guarded);
            if let Some((_, k)) = after.piece_at(sq) {
                if k != PieceKind::King && best_guard.map_or(true, |(_, b)| VALUE[k.idx()] > VALUE[b.idx()]) {
                    best_guard = Some((sq, k));
                }
            }
        }
        let mut hit_pawns = piece_attacks(&after, to) & after.pieces[them.idx()][PieceKind::Pawn.idx()];
        if hit_pawns != 0 {
            let sq = pop_lsb(&mut hit_pawns);
            return Facts::new(format!("challenges the pawn on {}", square_name(sq)), extras, false);
        }
        if CENTER.contains(&to) {
            return Facts::new(format!("takes space in the center with the pawn on {}", square_name(to)), extras, false);
        }
        if let Some((sq, k)) = best_guard {
            return Facts::new(format!("supports the {} on {}", name(k), square_name(sq)), extras, false);
        }
    }
    if piece == PieceKind::Rook && !file_has_pawns(&after, file_of(to)) && file_has_pawns(pos, file_of(from)) {
        let file = (b'a' + file_of(to)) as char;
        return Facts::new(format!("puts the rook on the open {file}-file"), extras, false);
    }

    // 12. Whatever improved most in the evaluation
    let lead = match best_positional_gain(pos, &after, us) {
        Some(Group::Placement) => {
            let central = (2..=5).contains(&file_of(to)) && (2..=5).contains(&rank_of(to));
            if central && matches!(piece, PieceKind::Knight | PieceKind::Bishop) {
                format!("centralizes the {} on {}", name(piece), square_name(to))
            } else {
                format!("moves the {} to a better square", name(piece))
            }
        }
        Some(Group::BishopPair) => "keeps the pair of bishops".into(),
        Some(Group::Passed) => "improves the passed pawns".into(),
        Some(Group::Structure) => "improves the pawn structure".into(),
        Some(Group::Mobility) => "gives the pieces more room to move".into(),
        Some(Group::Rooks) => "activates the rook on an open file".into(),
        Some(Group::Shield) => "keeps the king well covered by its pawns".into(),
        Some(Group::KingAttack) => "adds pressure around the enemy king".into(),
        None => "makes a quiet, useful move that keeps everything protected".into(),
    };
    Facts::new(lead, extras, false)
}

/// Explanation of a move the bot played.
/// `line_after` = what the search expects next (opponent's reply first).
/// `score` = the search score after the move, from the mover's point of view.
/// `prev` = the opponent's move that led to `pos`, if known.
pub fn explain_move(pos: &Position, m: Move, line_after: &[Move], score: i32, prev: Option<Move>) -> String {
    let f = describe(pos, m, line_after, prev);
    let label = line_san(pos, &[m]);
    let mut text = format!("{label}: {}.", capitalize(&f.lead));
    for e in &f.extras {
        text.push(' ');
        text.push_str(e);
    }
    if score >= MATE_BOUND && f.lead != "delivers checkmate" {
        let n = (MATE - score + 1) / 2;
        text += &format!(" The bot sees a forced mate in {n}.");
    }
    if !line_after.is_empty() {
        let after = pos.make_move(m);
        let shown = &line_after[..line_after.len().min(3)];
        text += &format!(" Expected continuation: {}.", line_san(&after, shown));
    }
    text
}

// ---------- Reviewing the user's move ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Best,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Best => "best",
            Verdict::Good => "good",
            Verdict::Inaccuracy => "inaccuracy",
            Verdict::Mistake => "mistake",
            Verdict::Blunder => "blunder",
        }
    }
}

/// Chance of winning in [-1, 1] for a score in centipawns (the same curve Lichess uses).
fn winning_chances(cp: i32) -> f64 {
    let cp = cp.clamp(-1500, 1500) as f64;
    2.0 / (1.0 + (-0.003_682_08 * cp).exp()) - 1.0
}

pub struct Review {
    pub verdict: Verdict,
    pub text: String,
}

/// Reviews `played`, comparing it with the engine's `best` move.
/// Scores are from the mover's point of view, both searched with the same time.
/// `best_line` = the engine's line after `best`; `refutation` = its line after `played`
/// (both start with the opponent's reply).
pub fn review_move(
    pos: &Position,
    played: Move,
    best: Move,
    best_score: i32,
    best_line: &[Move],
    played_score: i32,
    refutation: &[Move],
    prev: Option<Move>,
) -> Review {
    let label = line_san(pos, &[played]);
    if played == best {
        return Review { verdict: Verdict::Best, text: format!("{label}: the engine's top choice too. Well played!") };
    }
    let loss = winning_chances(best_score) - winning_chances(played_score);
    let verdict = if loss >= 0.3 {
        Verdict::Blunder
    } else if loss >= 0.2 {
        Verdict::Mistake
    } else if loss >= 0.1 {
        Verdict::Inaccuracy
    } else {
        Verdict::Good
    };
    if verdict == Verdict::Good {
        return Review { verdict, text: format!("{label}: a good move.") };
    }

    let (title, mark) = match verdict {
        Verdict::Blunder => ("Blunder", "??"),
        Verdict::Mistake => ("Mistake", "?"),
        _ => ("Inaccuracy", "?!"),
    };
    let mut text = format!("{title}: {label}{mark}");

    // Why it's worse: what the opponent can do now
    let after = pos.make_move(played);
    if played_score <= -MATE_BOUND {
        text += &format!(" allows a forced mate: {}.", line_san(&after, &refutation[..refutation.len().min(5)]));
    } else {
        // Mention the opponent's reply only when it really wins or threatens something
        match refutation.first() {
            Some(&reply) if after.parse_uci(&reply.to_uci()).is_some() => {
                let f = describe(&after, reply, &refutation[1..], Some(played));
                if f.material {
                    text += &format!(" allows {}, which {}.", line_san(&after, &[reply]), f.lead);
                }
            }
            _ => {}
        }
        let cost = (best_score - played_score).clamp(0, 2000) as f64 / 100.0;
        if best_score.abs() < MATE_BOUND && cost >= 0.5 {
            text += &format!(" It costs about {cost:.1} pawns of value.");
        }
    }

    // What was better
    let bf = describe(pos, best, best_line, prev);
    text += &format!(" Better was {}, which {}.", line_san(pos, &[best]), bf.lead);
    Review { verdict, text }
}

// ---------- A whole bot turn: review the user's move, think, explain ----------

use crate::board::Game;
use crate::search::{SearchLimits, Searcher};

pub struct TurnResult {
    pub bot_move: Move,
    pub depth: u32,
    pub nodes: u64,
    /// Search score after the bot's move, from White's point of view.
    pub score_white: i32,
    pub pv: Vec<Move>,
    pub explanation: String,
    /// (from, to) squares: the bot's move, the expected reply, the bot's follow-up.
    pub arrows: Vec<(Square, Square)>,
    pub review: Option<Review>,
}

fn limits(ms: f64) -> SearchLimits {
    SearchLimits { time_ms: ms.max(20.0), max_depth: 60 }
}

/// The part of a search's principal variation that comes after its first move.
fn rest_of_pv(pv: &[Move], first: Move) -> &[Move] {
    if pv.first() == Some(&first) {
        &pv[1..]
    } else {
        &[]
    }
}

/// Plays the bot's move in `game`.
/// `moves` = every move played so far (its last one led to the current position).
/// If `review` is true, that last move (the user's) is reviewed first.
pub fn bot_turn(game: &mut Game, moves: &[Move], review: bool, review_ms: f64, think_ms: f64) -> Option<TurnResult> {
    let mut searcher = Searcher::new(vec![]);
    let bot_pos = *game.current();
    let user_move = if review { moves.last().copied() } else { None };
    let before_user = if moves.len() >= 2 { Some(moves[moves.len() - 2]) } else { None };

    // 1. Review the user's move: best move before it vs. the move actually played
    let mut review = None;
    if let Some(um) = user_move {
        game.undo();
        let before = *game.current();
        let ok = before.parse_uci(&um.to_uci()) == Some(um) && before.make_move(um).hash == bot_pos.hash;
        if ok {
            searcher.set_history(game.history_hashes());
            let r1 = searcher.search(&before, limits(review_ms));
            game.play(um);
            if let Some(best) = r1.best_move {
                let best_line = rest_of_pv(&r1.pv, best).to_vec();
                let (played_score, refutation) = if best == um {
                    (r1.score, best_line.clone())
                } else {
                    // Same horizon as the first search: one ply less, because one move is already played
                    searcher.set_history(game.history_hashes());
                    let depth = r1.depth.saturating_sub(1).max(1);
                    let r2 = searcher.search(&bot_pos, SearchLimits { time_ms: review_ms * 2.0, max_depth: depth });
                    (-r2.score, r2.pv)
                };
                review = Some(review_move(&before, um, best, r1.score, &best_line, played_score, &refutation, before_user));
            }
        } else {
            game.play(um); // not the move that led here: skip the review
        }
    }

    // 2. The bot's own move
    searcher.set_history(game.history_hashes());
    let r = searcher.search(&bot_pos, limits(think_ms));
    let m = r.best_move?;
    let line_after = rest_of_pv(&r.pv, m).to_vec();
    let explanation = explain_move(&bot_pos, m, &line_after, r.score, moves.last().copied());

    let mut arrows = vec![(m.from(), m.to())];
    for &x in line_after.iter().take(2) {
        arrows.push((x.from(), x.to()));
    }
    let score_white = if bot_pos.side == Color::White { r.score } else { -r.score };
    game.play(m);

    Some(TurnResult { bot_move: m, depth: r.depth, nodes: r.nodes, score_white, pv: r.pv, explanation, arrows, review })
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

impl TurnResult {
    /// The result as JSON, for the browser:
    /// {"move","depth","nodes","score","pv","explanation","arrows":[[from,to],...],"review":{"verdict","text"}|null}
    pub fn to_json(&self) -> String {
        let pv: Vec<String> = self.pv.iter().map(|m| m.to_uci()).collect();
        let arrows: Vec<String> = self
            .arrows
            .iter()
            .map(|&(f, t)| format!("[{},{}]", json_str(&square_name(f)), json_str(&square_name(t))))
            .collect();
        let review = match &self.review {
            Some(r) => format!("{{\"verdict\":{},\"text\":{}}}", json_str(r.verdict.as_str()), json_str(&r.text)),
            None => "null".into(),
        };
        format!(
            "{{\"move\":{},\"depth\":{},\"nodes\":{},\"score\":{},\"pv\":{},\"explanation\":{},\"arrows\":[{}],\"review\":{}}}",
            json_str(&self.bot_move.to_uci()),
            self.depth,
            self.nodes,
            self.score_white,
            json_str(&pv.join(" ")),
            json_str(&self.explanation),
            arrows.join(","),
            review
        )
    }
}