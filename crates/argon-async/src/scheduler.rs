//! Work-stealing task scheduler.
//!
//! Manages a pool of worker threads. Each worker has a local task deque.
//! Idle workers steal tasks from busy workers' deques. Tasks are boxed
//! futures that are polled to completion.

use crossbeam_deque::{Injector, Stealer, Worker};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake};
use std::thread;

/// Opaque task identifier.
pub type TaskId = u64;

/// A boxed, pinned future that the scheduler can poll.
type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Shared task storage accessible by all worker threads.
type TaskMap = Arc<Mutex<HashMap<TaskId, BoxFuture>>>;

/// Waker that re-enqueues a task when woken.
struct TaskWaker {
    task_id: TaskId,
    injector: Arc<Injector<TaskId>>,
    worker_threads: Arc<Vec<thread::Thread>>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.injector.push(self.task_id);
        // Unpark one worker to pick up the re-enqueued task
        if let Some(t) = self.worker_threads.first() {
            t.unpark();
        }
    }
}

/// The work-stealing scheduler.
pub struct Scheduler {
    injector: Arc<Injector<TaskId>>,
    tasks: TaskMap,
    _workers: Vec<WorkerHandle>,
    #[allow(dead_code)] // stored for future use by external spawn_future callers
    worker_threads: Arc<Vec<thread::Thread>>,
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
        let tasks: TaskMap = Arc::new(Mutex::new(HashMap::new()));
        let mut workers = Vec::with_capacity(nthreads);
        let mut thread_handles = Vec::with_capacity(nthreads);

        for _ in 0..nthreads {
            let worker = Worker::new_fifo();
            let stealer = worker.stealer();
            let inj = injector.clone();
            let run = running.clone();
            let task_map = tasks.clone();

            let handle = thread::spawn(move || {
                Self::worker_loop(worker, inj, run, task_map);
            });

            thread_handles.push(handle.thread().clone());
            workers.push(WorkerHandle {
                thread: Some(handle),
                _stealer: stealer,
            });
        }

        Self {
            injector,
            tasks,
            _workers: workers,
            worker_threads: Arc::new(thread_handles),
            running,
            next_task_id: AtomicU64::new(1),
        }
    }

    fn worker_loop(
        worker: Worker<TaskId>,
        injector: Arc<Injector<TaskId>>,
        running: Arc<AtomicBool>,
        tasks: TaskMap,
    ) {
        let current_thread = vec![thread::current()];
        let worker_threads = Arc::new(current_thread);

        while running.load(Ordering::Relaxed) {
            let task_id = worker
                .pop()
                .or_else(|| match injector.steal() {
                    crossbeam_deque::Steal::Success(id) => Some(id),
                    _ => None,
                });

            if let Some(task_id) = task_id {
                Self::poll_task(task_id, &injector, &worker_threads, &tasks);
            } else {
                thread::park();
            }
        }
    }

    fn poll_task(
        task_id: TaskId,
        injector: &Arc<Injector<TaskId>>,
        worker_threads: &Arc<Vec<thread::Thread>>,
        tasks: &TaskMap,
    ) {
        // Take the future out of the map while polling to avoid holding
        // the lock during execution.
        let mut future = {
            let mut map = tasks.lock().unwrap();
            match map.remove(&task_id) {
                Some(f) => f,
                None => return, // Task already completed or removed
            }
        };

        let waker = Arc::new(TaskWaker {
            task_id,
            injector: injector.clone(),
            worker_threads: worker_threads.clone(),
        })
        .into();
        let mut cx = Context::from_waker(&waker);

        match future.as_mut().poll(&mut cx) {
            Poll::Ready(()) => {
                // Task completed — don't put it back
            }
            Poll::Pending => {
                // Task not done yet — put it back for later re-polling
                let mut map = tasks.lock().unwrap();
                map.insert(task_id, future);
            }
        }
    }

    /// Submit a future as a new task. Returns the task ID.
    pub fn spawn_future<F>(&self, future: F) -> TaskId
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let id = self.next_task_id();
        {
            let mut map = self.tasks.lock().unwrap();
            map.insert(id, Box::pin(future));
        }
        self.injector.push(id);
        self.wake_one_worker();
        id
    }

    /// Submit a task ID to the global queue (for pre-registered tasks).
    pub fn spawn(&self, task_id: TaskId) {
        self.injector.push(task_id);
        self.wake_one_worker();
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

    fn wake_one_worker(&self) {
        for w in &self._workers {
            if let Some(ref t) = w.thread {
                t.thread().unpark();
                break;
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
    use std::sync::atomic::AtomicBool;

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

    #[test]
    fn scheduler_executes_spawned_future() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();

        let sched = Scheduler::new(2);
        sched.spawn_future(async move {
            completed_clone.store(true, Ordering::SeqCst);
        });

        // Give the worker thread time to pick up and execute the task
        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(completed.load(Ordering::SeqCst), "Future should have been polled to completion");
        sched.shutdown();
    }

    #[test]
    fn scheduler_executes_multiple_futures() {
        let counter = Arc::new(AtomicU64::new(0));

        let sched = Scheduler::new(2);
        for _ in 0..10 {
            let counter_clone = counter.clone();
            sched.spawn_future(async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
        }

        std::thread::sleep(std::time::Duration::from_millis(200));

        assert_eq!(counter.load(Ordering::SeqCst), 10, "All 10 futures should have completed");
        sched.shutdown();
    }
}
