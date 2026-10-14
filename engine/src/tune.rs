//! Texel tuning: adjusts every evaluation weight so the evaluation predicts game
//! results as well as possible.
//!
//! For each training position we know the result (1, 0.5 or 0). The evaluation is
//! turned into an expected result with a sigmoid: +0 => 50%, +300 => ~85%, etc.
//! The tuner minimizes the squared difference between expected and real results,
//! using gradient descent (Adam). Because the evaluation is linear in its weights,
//! each position only needs to be "traced" once.

use crate::board::Position;
use crate::eval::{trace, BISHOP_PAIR, MATERIAL, N_PARAMS, WEIGHTS};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};

const PHASE_MAX: f64 = 24.0;

struct Entry {
    start: usize, // index into the shared coefficient list
    len: usize,
    phase: f64,
    result: f64,
}

struct Data {
    entries: Vec<Entry>,
    coefs: Vec<(u16, i16)>,
}

fn load(path: &str) -> Data {
    let file = File::open(path).unwrap_or_else(|e| panic!("cannot open {path}: {e}"));
    let mut entries = Vec::new();
    let mut coefs = Vec::new();
    let mut skipped = 0;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        let Some((fen, result)) = line.rsplit_once(';') else {
            skipped += 1;
            continue;
        };
        let (Ok(pos), Ok(result)) = (Position::from_fen(fen.trim()), result.trim().parse::<f64>()) else {
            skipped += 1;
            continue;
        };
        let (c, phase) = trace(&pos);
        entries.push(Entry { start: coefs.len(), len: c.len(), phase: phase as f64, result });
        coefs.extend(c);
    }
    eprintln!("Loaded {} positions ({skipped} lines skipped)", entries.len());
    Data { entries, coefs }
}

fn eval(e: &Entry, coefs: &[(u16, i16)], w: &[[f64; 2]]) -> f64 {
    let (mut mg, mut eg) = (0.0, 0.0);
    for &(i, c) in &coefs[e.start..e.start + e.len] {
        mg += w[i as usize][0] * c as f64;
        eg += w[i as usize][1] * c as f64;
    }
    (mg * e.phase + eg * (PHASE_MAX - e.phase)) / PHASE_MAX
}

fn sigmoid(k: f64, score: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-k * score / 400.0))
}

/// Mean squared error over all positions (split across threads).
fn error(data: &Data, w: &[[f64; 2]], k: f64, threads: usize) -> f64 {
    let chunk = data.entries.len().div_ceil(threads).max(1);
    let total: f64 = std::thread::scope(|s| {
        data.entries
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .map(|e| {
                            let d = e.result - sigmoid(k, eval(e, &data.coefs, w));
                            d * d
                        })
                        .sum::<f64>()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum()
    });
    total / data.entries.len() as f64
}

/// Gradient of the error with respect to every (mg, eg) weight.
fn gradient(data: &Data, w: &[[f64; 2]], k: f64, threads: usize) -> Vec<[f64; 2]> {
    let chunk = data.entries.len().div_ceil(threads).max(1);
    let parts: Vec<Vec<[f64; 2]>> = std::thread::scope(|s| {
        data.entries
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut g = vec![[0.0f64; 2]; N_PARAMS];
                    for e in part {
                        let sg = sigmoid(k, eval(e, &data.coefs, w));
                        // d(error)/d(eval), constant factors folded into the learning rate
                        let common = (sg - e.result) * sg * (1.0 - sg);
                        let mg_part = common * e.phase / PHASE_MAX;
                        let eg_part = common * (PHASE_MAX - e.phase) / PHASE_MAX;
                        for &(i, c) in &data.coefs[e.start..e.start + e.len] {
                            g[i as usize][0] += mg_part * c as f64;
                            g[i as usize][1] += eg_part * c as f64;
                        }
                    }
                    g
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    });
    let mut g = vec![[0.0f64; 2]; N_PARAMS];
    for part in parts {
        for (a, b) in g.iter_mut().zip(part) {
            a[0] += b[0];
            a[1] += b[1];
        }
    }
    g
}

/// The sigmoid's scaling constant K that best fits the CURRENT evaluation.
fn find_k(data: &Data, w: &[[f64; 2]], threads: usize) -> f64 {
    let (mut best_k, mut best_e) = (1.0, f64::MAX);
    let mut k = 0.2;
    while k <= 3.0 {
        let e = error(data, w, k, threads);
        if e < best_e {
            best_e = e;
            best_k = k;
        }
        k += 0.05;
    }
    best_k
}

/// engine-uci tune <data file> [epochs] [threads] [output file] [--all]
/// Without --all, only the new terms are tuned; PeSTO material and tables stay fixed.
pub fn run(data_path: &str, epochs: usize, threads: usize, out_path: &str, tune_all: bool) {
    let threads = threads.max(1);
    let data = load(data_path);
    if data.entries.is_empty() {
        eprintln!("No positions to tune on.");
        return;
    }

    let mut w: Vec<[f64; 2]> = WEIGHTS.iter().map(|s| [s.mg as f64, s.eg as f64]).collect();
    let k = find_k(&data, &w, threads);
    eprintln!("K = {k:.2}, starting error = {:.6}", error(&data, &w, k, threads));

    // Adam optimizer
    let (lr, b1, b2, eps) = (0.1, 0.9, 0.999, 1e-8);
    let mut m = vec![[0.0f64; 2]; N_PARAMS];
    let mut v = vec![[0.0f64; 2]; N_PARAMS];
    let scale = 1.0 / data.entries.len() as f64;

    for epoch in 1..=epochs {
        let g = gradient(&data, &w, k, threads);
        for i in 0..N_PARAMS {
            if !tune_all && i < BISHOP_PAIR {
                continue; // keep PeSTO's material and tables as they are
            }
            for p in 0..2 {
                let gi = g[i][p] * scale;
                m[i][p] = b1 * m[i][p] + (1.0 - b1) * gi;
                v[i][p] = b2 * v[i][p] + (1.0 - b2) * gi * gi;
                let mh = m[i][p] / (1.0 - b1.powi(epoch as i32));
                let vh = v[i][p] / (1.0 - b2.powi(epoch as i32));
                w[i][p] -= lr * mh / (vh.sqrt() + eps);
            }
        }
        if epoch % 25 == 0 || epoch == epochs {
            eprintln!("epoch {epoch:>4}/{epochs}  error = {:.6}", error(&data, &w, k, threads));
        }
    }

    // The king has no material value; keep it at zero
    w[MATERIAL + 5] = [0.0, 0.0];

    fs::write(out_path, render(&w)).unwrap_or_else(|e| panic!("cannot write {out_path}: {e}"));
    eprintln!("Wrote tuned weights to {out_path}. Rebuild the engine to use them.");
}

/// Writes the weights in the same layout as params.rs.
fn render(w: &[[f64; 2]]) -> String {
    use crate::eval::*;
    let r = |i: usize| format!("s({}, {}),", w[i][0].round() as i32, w[i][1].round() as i32);
    let row = |from: usize, n: usize| (from..from + n).map(r).collect::<Vec<_>>().join(" ");
    let names = ["pawn", "knight", "bishop", "rook", "queen", "king"];

    let mut out = String::new();
    out.push_str("//! Evaluation weights: (middlegame, endgame) pairs, in centipawns.\n");
    out.push_str("//!\n//! Generated by `engine-uci tune` (Texel tuning). Initial values came from the\n");
    out.push_str("//! PeSTO tables (Ronald Friederich, Chess Programming Wiki, CC BY-SA 3.0).\n\n");
    out.push_str("use super::{s, S, N_PARAMS};\n\n#[rustfmt::skip]\npub const WEIGHTS: [S; N_PARAMS] = [\n");
    out.push_str("    // Material: pawn, knight, bishop, rook, queen, king\n");
    out.push_str(&format!("    {}\n", row(MATERIAL, 6)));
    for (p, name) in names.iter().enumerate() {
        out.push_str(&format!("    // Piece-square table: {name} (row 1 = rank 8, from White's side)\n"));
        for rank in 0..8 {
            out.push_str(&format!("    {}\n", row(PST + p * 64 + rank * 8, 8)));
        }
    }
    let sections: [(&str, usize, usize); 9] = [
        ("Bishop pair", BISHOP_PAIR, 1),
        ("Passed pawn, by rank (from the pawn's own side): 1..8", PASSED, 8),
        ("Isolated pawn", ISOLATED, 1),
        ("Doubled pawn (per extra pawn on a file)", DOUBLED, 1),
        ("Mobility per square: knight, bishop, rook, queen", MOBILITY, 4),
        ("Rook on open file", ROOK_OPEN, 1),
        ("Rook on semi-open file", ROOK_SEMI_OPEN, 1),
        ("King shield (per own pawn in front of the king)", KING_SHIELD, 1),
        ("Attacks on the enemy king zone (per square): knight, bishop, rook, queen", KING_ATTACK, 4),
    ];
    for (title, from, n) in sections {
        out.push_str(&format!("    // {title}\n    {}\n", row(from, n)));
    }
    out.push_str("];\n");
    out
}