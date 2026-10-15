# Kasparov ♞

A chess app with a chess engine written from scratch in Rust, running entirely in your browser through WebAssembly. It doesn't just play: it **teaches**. Every move is explained, and your own moves are reviewed.

## Features

### User vs CPU
- Play as White or Black against a strong engine (alpha-beta search, transposition table, hand-built evaluation).
- The bot "thinks" for 1–5 seconds per move, in a background thread, so the page never freezes.
- When you play Black, **pick the bot's opening** (Italian, Ruy Lopez, Queen's Gambit, London, and more) or let it choose.
- **Coach panel:** every bot move is explained in plain English, and arrows show its plan.
- **Move review:** your moves are marked ✓ (best move), ?! (inaccuracy), ? (mistake) or ?? (blunder), with an explanation and the better move.
- **Take back** up to 4 moves in a row.

### Learn Openings
- 15 openings and defenses, each with the idea behind every move, when to play it, its key plans and a common pitfall.
- **Watch** mode: step through the opening or let it play.
- **Try it yourself** mode: play the moves, with hint arrows if you want them. After 3 wrong tries, the coach shows the answer.

### Endgame Academy
- 7 lessons, from easy (checkmating with two rooks, the queen or one rook) through medium (king and pawn, queen against a pawn) to hard (the pawn breakthrough, the Lucena position).
- Each lesson is played twice: first with hints, then on your own. Every move is explained.
- Lessons unlock one by one, and your progress is saved in the browser.

## Requirements

- **Rust** (stable) with the `wasm32-unknown-unknown` target
- **wasm-pack**
- **Node.js** 22 or newer, with npm
- Git

### Option A: conda (one command)

The repository includes an environment file and a setup script:

```bash
bash setup_env.sh
conda activate <environment name>   # the name in environment.yml
```

### Option B: manual install

```bash
# Rust (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# wasm-pack
cargo install wasm-pack        # or: npm install -g wasm-pack

# Node.js 22+: install from https://nodejs.org
```

## Getting started

```bash
git clone https://github.com/<your-username>/kasparov.git
cd kasparov

# 1. Build the engine as a WebAssembly package (creates engine/pkg)
cd engine
wasm-pack build --target web --release

# 2. Install the web app's dependencies and start it
cd ../web
npm install
npm run dev
```

Open the address Vite prints (usually http://localhost:5173).

The engine must be built **before** `npm install`, because the web app depends on the `engine/pkg` folder that step 1 creates.

### After changing the Rust code

Rebuild the engine package and reinstall it in one go, from the `web` folder:

```bash
npm run engine
```

Then reload the page. If the browser still runs old code, do a hard refresh (`Ctrl+Shift+R`).

## Project structure

```
kasparov/
├─ engine/                 Rust chess engine
│  ├─ src/
│  │  ├─ board/            position, FEN, Zobrist hashing, making moves, game status
│  │  ├─ attacks.rs        attack tables for every piece
│  │  ├─ movegen.rs        legal move generation
│  │  ├─ eval/             evaluation (material, piece-square tables, pawns, mobility, king safety…)
│  │  ├─ search.rs         alpha-beta search
│  │  ├─ tt.rs             transposition table
│  │  ├─ explain.rs        move explanations and reviews (the coach)
│  │  ├─ wasm.rs           the API the browser uses
│  │  ├─ datagen.rs, tune.rs   tools for tuning the evaluation weights
│  │  └─ main.rs           command-line tool / UCI engine
│  └─ tests/               perft, evaluation, search and coach tests
└─ web/                    React + TypeScript + Vite app
   └─ src/
      ├─ App.tsx           home menu
      ├─ CpuGame.tsx       User vs CPU
      ├─ LearnOpenings.tsx Learn Openings
      ├─ EndgameAcademy.tsx  Endgame Academy
      ├─ engine.worker.ts  runs the engine in a background thread
      └─ openings.ts, book.ts, openingLessons.ts, endgames.ts   lesson and opening data
```

## Engine tests and tools

Run these from the `engine` folder. Always use `--release`: debug builds are far slower.

```bash
cargo test --release                                   # all tests
cargo test --release -- --ignored                      # deep move-generation tests (perft)

cargo run --release -- perft 5                         # count positions (should print 4865609 nodes)
cargo run --release -- eval "<FEN>"                    # evaluation breakdown of a position
cargo run --release -- search 3000 "<FEN>"             # search a position for 3 seconds
cargo run --release -- bench                           # fixed benchmark (node count + speed)
cargo run --release -- coach 1000 "<FEN>"              # the bot's move and its explanation
```

Without arguments, `engine-uci` speaks the **UCI** protocol, so it works with chess GUIs and testing tools:

```bash
cargo build --release
./target/release/engine-uci
```

### Measuring strength (optional)

Changes to the engine can be checked with [fastchess](https://github.com/Disservin/fastchess), which plays two versions of the engine against each other until it's statistically clear which one is stronger (SPRT). A good opening book for this is `UHO_Lichess_4852_v1.epd` from [official-stockfish/books](https://github.com/official-stockfish/books).

## Troubleshooting

| Problem | Fix |
|---|---|
| `npm install` fails on the `engine` package | Build the engine first (step 1 of Getting started). |
| `Failed to resolve import "./Something"` | A file is missing from `web/src`. File names are case-sensitive. |
| The app runs old engine code | Run `npm run engine`, then hard-refresh the page (`Ctrl+Shift+R`). |
| Blank page | Open the browser console (F12) for the error message. |
| `wasm-pack` errors about `wasm-opt` | Add this to `engine/Cargo.toml`: `[package.metadata.wasm-pack.profile.release]` with `wasm-opt = false` on the next line. |

## Credits

- Piece-square tables and material values: **PeSTO** by Ronald Friederich, from the [Chess Programming Wiki](https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function) (CC BY-SA 3.0).
- Opening names: the [Lichess chess-openings](https://github.com/lichess-org/chess-openings) dataset (CC0).
- Board component: [react-chessboard](https://github.com/Clariity/react-chessboard).
- The [Chess Programming Wiki](https://www.chessprogramming.org) was the main reference for the engine techniques.