//! Backend trait abstractions for Argon stdlib I/O operations.
//!
//! This crate defines Rust traits that abstract platform-specific I/O
//! operations for file system, networking, HTTP, and WebSocket.
//! Each compilation target (JS, WASM, Native) and the runtime interpreter
//! implement these traits to provide their platform-specific behavior.
//!
//! The `IoError` type is shared across all modules as the unified error type.

pub mod fs;
pub mod http;
pub mod net;
pub mod ws;

// Re-export the shared error type at the crate root.
pub use fs::{IoError, IoResult};
