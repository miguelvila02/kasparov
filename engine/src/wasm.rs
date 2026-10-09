use crate::board::{Game, GameStatus, Position, START_FEN};
use crate::types::Color;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(fen: Option<String>) -> Result<WasmGame, String> {
        let fen = fen.unwrap_or_else(|| START_FEN.to_string());
        let pos = Position::from_fen(&fen).map_err(|e| format!("Invalid FEN: {e:?}"))?;
        Ok(WasmGame { game: Game::new(pos) })
    }

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
}