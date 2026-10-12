//! Transposition table: remembers positions the search has already analyzed.
//!
//! The same position is reached through many different move orders
//! (1.Nf3 Nf6 2.Nc3 = 1.Nc3 Nf6 2.Nf3). Without a table, each one is searched
//! from scratch. With it, the second time we either reuse the result directly
//! or at least try the best move found last time first.

use crate::moves::Move;

/// What kind of score is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    /// Empty slot.
    None,
    /// The exact score (some move landed between alpha and beta).
    Exact,
    /// The real score is at least this (a move caused a beta cutoff).
    Lower,
    /// The real score is at most this (no move beat alpha).
    Upper,
}

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub score: i32,
    pub mv: Move,
    pub depth: i16,
    pub bound: Bound,
}

impl TTEntry {
    const EMPTY: TTEntry = TTEntry { key: 0, score: 0, mv: Move::NULL, depth: -1, bound: Bound::None };
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
}

impl TranspositionTable {
    /// Creates a table using at most `mb` megabytes (rounded down to a power of two entries).
    pub fn new(mb: usize) -> Self {
        let bytes = mb.max(1) * 1024 * 1024;
        let entry = std::mem::size_of::<TTEntry>();
        let mut n = (bytes / entry).next_power_of_two();
        if n * entry > bytes {
            n /= 2;
        }
        TranspositionTable { entries: vec![TTEntry::EMPTY; n], mask: n - 1 }
    }

    pub fn clear(&mut self) {
        self.entries.fill(TTEntry::EMPTY);
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let e = self.entries[(key as usize) & self.mask];
        if e.bound != Bound::None && e.key == key {
            Some(e)
        } else {
            None
        }
    }

    pub fn store(&mut self, key: u64, mv: Move, score: i32, depth: i32, bound: Bound) {
        let slot = &mut self.entries[(key as usize) & self.mask];
        // Replace a different position always; the same position only with
        // similar-or-deeper information, or an exact score.
        let replace = slot.key != key || bound == Bound::Exact || depth + 2 >= slot.depth as i32;
        if replace {
            // Don't lose a known best move when the new result has none
            let mv = if mv == Move::NULL && slot.key == key { slot.mv } else { mv };
            *slot = TTEntry { key, score, mv, depth: depth as i16, bound };
        }
    }
}