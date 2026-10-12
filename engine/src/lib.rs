pub mod attacks;
pub mod board;
pub mod movegen;
pub mod moves;
pub mod perft;
pub mod types;
pub mod eval;
pub mod search;



#[cfg(target_arch = "wasm32")]
mod wasm;