//! C-ABI FFI functions for native target integration.
//!
//! These functions are called by Cranelift-generated code in native binaries.

use crate::scheduler::Scheduler;
use std::sync::OnceLock;

static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();

/// Initialize the async scheduler with the given number of worker threads.
/// If `nthreads <= 0`, uses the number of available CPU cores.
#[no_mangle]
pub extern "C" fn __argon_scheduler_init(nthreads: i32) {
    let n = if nthreads <= 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        nthreads as usize
    };
    let _ = SCHEDULER.set(Scheduler::new(n));
}

/// Shut down the scheduler and wait for all workers to stop.
#[no_mangle]
pub extern "C" fn __argon_scheduler_shutdown() {
    if let Some(s) = SCHEDULER.get() {
        s.shutdown();
    }
}

/// Spawn a task on the scheduler. Returns the task ID.
#[no_mangle]
pub extern "C" fn __argon_scheduler_spawn() -> u64 {
    if let Some(s) = SCHEDULER.get() {
        let id = s.next_task_id();
        s.spawn(id);
        id
    } else {
        0
    }
}
