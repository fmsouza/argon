//! Argon async runtime.
//!
//! Provides a work-stealing task scheduler with mio-based I/O reactor
//! for the native compilation target. Also exposes C-ABI FFI functions
//! for Cranelift-generated code to call.

pub mod ffi;
pub mod reactor;
pub mod scheduler;
