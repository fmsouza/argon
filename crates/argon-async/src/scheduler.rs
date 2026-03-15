//! Work-stealing task scheduler.
//!
//! Manages a pool of worker threads. Each worker has a local task deque.
//! Idle workers steal tasks from busy workers' deques.

use crossbeam_deque::{Injector, Stealer, Worker};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// Opaque task identifier.
pub type TaskId = u64;

/// The work-stealing scheduler.
pub struct Scheduler {
    injector: Arc<Injector<TaskId>>,
    _workers: Vec<WorkerHandle>,
    running: Arc<AtomicBool>,
    next_task_id: AtomicU64,
}

struct WorkerHandle {
    thread: Option<thread::JoinHandle<()>>,
    _stealer: Stealer<TaskId>,
}

impl Scheduler {
    /// Create a new scheduler with `nthreads` worker threads.
    pub fn new(nthreads: usize) -> Self {
        let injector = Arc::new(Injector::new());
        let running = Arc::new(AtomicBool::new(true));
        let mut workers = Vec::with_capacity(nthreads);

        for _ in 0..nthreads {
            let worker = Worker::new_fifo();
            let stealer = worker.stealer();
            let inj = injector.clone();
            let run = running.clone();

            let handle = thread::spawn(move || {
                while run.load(Ordering::Relaxed) {
                    // Try local queue first, then global injector
                    if let Some(_task_id) = worker.pop() {
                        // TODO: look up task by ID and call poll(waker)
                    } else if let crossbeam_deque::Steal::Success(_task_id) = inj.steal() {
                        // TODO: poll stolen task
                    } else {
                        // No work — park until woken
                        thread::park();
                    }
                }
            });

            workers.push(WorkerHandle {
                thread: Some(handle),
                _stealer: stealer,
            });
        }

        Self {
            injector,
            _workers: workers,
            running,
            next_task_id: AtomicU64::new(1),
        }
    }

    /// Submit a task to the global queue.
    pub fn spawn(&self, task_id: TaskId) {
        self.injector.push(task_id);
        // Wake a worker to process it
        for w in &self._workers {
            if let Some(ref t) = w.thread {
                t.thread().unpark();
                break;
            }
        }
    }

    /// Allocate a new task ID.
    pub fn next_task_id(&self) -> TaskId {
        self.next_task_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Signal all workers to stop.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
        for w in &self._workers {
            if let Some(ref t) = w.thread {
                t.thread().unpark();
            }
        }
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shutdown();
        for w in &mut self._workers {
            if let Some(t) = w.thread.take() {
                let _ = t.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_creates_and_shuts_down() {
        let sched = Scheduler::new(2);
        let id = sched.next_task_id();
        assert_eq!(id, 1);
        sched.spawn(id);
        sched.shutdown();
    }

    #[test]
    fn scheduler_allocates_unique_ids() {
        let sched = Scheduler::new(1);
        let id1 = sched.next_task_id();
        let id2 = sched.next_task_id();
        assert_ne!(id1, id2);
    }
}
