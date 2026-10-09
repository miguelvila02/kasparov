pub mod attacks;
pub mod board;
pub mod movegen;
pub mod moves;
pub mod perft;
pub mod types;



#[cfg(target_arch = "wasm32")]
mod wasm;