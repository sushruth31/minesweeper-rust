//! Minesweeper: pure rules in [`game`], browser rendering in `app`.
//!
//! `app` is compiled only for `wasm32`, which lets `cargo test` exercise the
//! rules on the host toolchain without pulling in a DOM.

pub mod config;
pub mod game;

#[cfg(target_arch = "wasm32")]
pub mod app;
