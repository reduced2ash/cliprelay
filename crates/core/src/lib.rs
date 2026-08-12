//! ClipRelay core services: database, settings, media pipeline, delivery.
//!
//! This crate is UI-free so the whole service layer can be tested with
//! plain `cargo test`.

pub mod cleanup;
pub mod db;
pub mod media;
pub mod paths;
pub mod secrets;
pub mod settings;
pub mod telegram;
pub mod utils;
pub mod x;

pub use db::Database;
pub use settings::Settings;

/// A shared result type for the whole crate.
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;
