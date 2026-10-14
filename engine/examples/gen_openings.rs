//! Builds web/src/openings.ts and web/src/opening-names.json.
//! Every move is replayed with the engine, so an illegal or mistyped move stops the build.
use engine::board::Position;
use std::collections::HashMap;
use std::fmt::Write;

fn key(p: &Position) -> String {
    p.to_fen().split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}
fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c { '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), _ => o.push(c) }
    }
    o.push('"');
    o
}

struct Line { name: String, moves: Vec<(String, String, String)> } // uci, san, idea
struct Op { id: String, name: String, eco: String, kind: String, first: String, desc: String, lines: Vec<Line> }

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (src, tsv_dir, out_dir) = (&a[1], &a[2], &a[3]);

    // ---- curated openings ----
    let mut ops: Vec<Op> = Vec::new();
    let mut ideas: HashMap<(String, String), String> = HashMap::new();
    let mut pos = Position::start();
    for (n, raw) in std::fs::read_to_string(src).unwrap().lines().enumerate() {
        let l = raw.trim();
        if l.is_empty() || l.starts_with('#') { continue; }
        if let Some(h) = l.strip_prefix('@') {
            let f: Vec<&str> = h.split('|').map(str::trim).collect();
            ops.push(Op { id: f[0].into(), name: f[1].into(), eco: f[2].into(), kind: f[3].into(), first: f[4].into(), desc: String::new(), lines: vec![] });
        } else if let Some(d) = l.strip_prefix('?') {
            ops.last_mut().unwrap().desc = d.trim().into();
        } else if let Some(name) = l.strip_prefix("- ") {
            ops.last_mut().unwrap().lines.push(Line { name: name.trim().into(), moves: vec![] });
            pos = Position::start();
        } else {
            let (san, idea) = match l.split_once("::") { Some((s, i)) => (s.trim(), i.trim()), None => (l, "") };
            let m = pos.parse_san(san).unwrap_or_else(|| panic!("line {}: illegal move {san} in {}", n + 1, pos.to_fen()));
            let k = (key(&pos), m.to_uci());
            let idea = if idea.is_empty() {
                ideas.get(&k).cloned().unwrap_or_else(|| panic!("line {}: no idea for {san}", n + 1))
            } else {
                ideas.entry(k).or_insert_with(|| idea.to_string()).clone()
            };
            let real_san = pos.san(m);
            ops.last_mut().unwrap().lines.last_mut().unwrap().moves.push((m.to_uci(), real_san, idea));
            pos = pos.make_move(m);
        }
    }

    let mut ts = String::from("// Generated from openings.txt by the engine's gen_openings tool. Do not edit by hand.\n\n");
    ts.push_str("export type BookMove = { uci: string; san: string; idea: string };\n");
    ts.push_str("export type BookLine = { name: string; moves: BookMove[] };\n");
    ts.push_str("export type Opening = {\n  id: string;\n  name: string;\n  eco: string;\n  firstMoves: string;\n  description: string;\n  lines: BookLine[];\n};\n\n");
    for kind in ["white", "defense"] {
        let var = if kind == "white" { "WHITE_OPENINGS" } else { "DEFENSES" };
        writeln!(ts, "export const {var}: Opening[] = [").unwrap();
        for o in ops.iter().filter(|o| o.kind == kind) {
            writeln!(ts, "  {{\n    id: {},\n    name: {},\n    eco: {},\n    firstMoves: {},\n    description: {},\n    lines: [", js(&o.id), js(&o.name), js(&o.eco), js(&o.first), js(&o.desc)).unwrap();
            for line in &o.lines {
                writeln!(ts, "      {{\n        name: {},\n        moves: [", js(&line.name)).unwrap();
                for (u, s, i) in &line.moves {
                    writeln!(ts, "          {{ uci: {}, san: {}, idea: {} }},", js(u), js(s), js(i)).unwrap();
                }
                ts.push_str("        ],\n      },\n");
            }
            ts.push_str("    ],\n  },\n");
        }
        ts.push_str("];\n\n");
    }
    std::fs::write(format!("{out_dir}/openings.ts"), ts).unwrap();
    let nlines: usize = ops.iter().map(|o| o.lines.len()).sum();
    eprintln!("{} openings, {} lines, {} distinct move ideas", ops.len(), nlines, ideas.len());

    // ---- opening names (Lichess chess-openings, CC0) ----
    let mut names: Vec<(String, String)> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for f in ["a", "b", "c", "d", "e"] {
        for row in std::fs::read_to_string(format!("{tsv_dir}/{f}.tsv")).unwrap().lines().skip(1) {
            let c: Vec<&str> = row.split('\t').collect();
            if c.len() < 3 { continue; }
            let mut p = Position::start();
            let mut ok = true;
            for tok in c[2].split_whitespace() {
                if tok.ends_with('.') || tok.contains("...") { continue; }
                match p.parse_san(tok) { Some(m) => p = p.make_move(m), None => { ok = false; break; } }
            }
            if !ok { eprintln!("skipped: {}", c[1]); continue; }
            let k = key(&p);
            let label = format!("{} ({})", c[1], c[0]);
            match seen.get(&k) { Some(&i) => names[i].1 = label, None => { seen.insert(k.clone(), names.len()); names.push((k, label)); } }
        }
    }
    let mut json = String::from("{\n");
    for (i, (k, v)) in names.iter().enumerate() {
        write!(json, "  {}: {}{}\n", js(k), js(v), if i + 1 < names.len() { "," } else { "" }).unwrap();
    }
    json.push_str("}\n");
    std::fs::write(format!("{out_dir}/opening-names.json"), json).unwrap();
    eprintln!("{} named positions", names.len());
}