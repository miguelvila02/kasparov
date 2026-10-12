use crate::board::{Game, GameStatus, Position, START_FEN};
use crate::search::{SearchLimits, Searcher};
use crate::types::Color;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
    last_depth: u32,
    last_score: i32, // White's point of view
    last_nodes: u32,
    last_pv: String,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(fen: Option<String>) -> Result<WasmGame, String> {
        console_error_panic_hook::set_once();
        let fen = fen.unwrap_or_else(|| START_FEN.to_string());
        let pos = Position::from_fen(&fen).map_err(|e| format!("Invalid FEN: {e:?}"))?;
        Ok(WasmGame {
            game: Game::new(pos),
            last_depth: 0,
            last_score: 0,
            last_nodes: 0,
            last_pv: String::new(),
        })
    }

    /// Searches for `time_ms` milliseconds and plays the best move found.
    pub fn bot_move(&mut self, time_ms: f64) -> Option<String> {
        let pos = *self.game.current();
        let mut searcher = Searcher::new(self.game.history_hashes());
        let r = searcher.search(&pos, SearchLimits { time_ms, max_depth: 60 });
        let m = r.best_move?;

        self.last_depth = r.depth;
        self.last_score = if pos.side == Color::White { r.score } else { -r.score };
        self.last_nodes = r.nodes.min(u32::MAX as u64) as u32;
        self.last_pv = r.pv.iter().map(|m| m.to_uci()).collect::<Vec<_>>().join(" ");

        self.game.play(m);
        Some(m.to_uci())
    }

    pub fn last_depth(&self) -> u32 { self.last_depth }
    pub fn last_score(&self) -> i32 { self.last_score }
    pub fn last_nodes(&self) -> u32 { self.last_nodes }
    pub fn last_pv(&self) -> String { self.last_pv.clone() }

    pub fn fen(&self) -> String {
        self.game.current().to_fen()
    }

    pub fn legal_moves(&self) -> Vec<String> {
        self.game.current().legal_moves().iter().map(|m| m.to_uci()).collect()
    }

    /// Plays a UCI move. Returns false if it's illegal.
    pub fn play(&mut self, uci: &str) -> bool {
        match self.game.current().parse_uci(uci) {
            Some(m) => {
                self.game.play(m);
                true
            }
            None => false,
        }
    }

    pub fn undo(&mut self) {
        self.game.undo();
    }

    pub fn side_to_move(&self) -> String {
        if self.game.current().side == Color::White { "w" } else { "b" }.to_string()
    }

    pub fn status(&self) -> String {
        match self.game.status() {
            GameStatus::Ongoing => "ongoing",
            GameStatus::Checkmate(Color::White) => "checkmate_white",
            GameStatus::Checkmate(Color::Black) => "checkmate_black",
            GameStatus::Stalemate => "stalemate",
            GameStatus::FiftyMoves => "fifty_moves",
            GameStatus::Repetition => "repetition",
            GameStatus::InsufficientMaterial => "insufficient",
        }
        .to_string()
    }

    /// Evaluation from White's point of view, in centipawns.
    pub fn evaluation(&self) -> i32 {
        crate::eval::breakdown(self.game.current()).total
    }

    /// SAN for a UCI move in the current position (call it BEFORE playing the move).
    pub fn san(&self, uci: &str) -> Option<String> {
        let pos = self.game.current();
        let m = pos.parse_uci(uci)?;
        Some(pos.san(m))
    }
}