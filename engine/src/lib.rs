pub mod attacks;
pub mod board;
pub mod eval;
pub mod movegen;
pub mod moves;
pub mod perft;
pub mod search;
pub mod time;
pub mod types;
pub mod tt;



#[cfg(target_arch = "wasm32")]
mod wasm;