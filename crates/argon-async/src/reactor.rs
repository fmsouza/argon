//! I/O reactor wrapping mio::Poll.
//!
//! Manages I/O event registration and polling for async socket operations.

use mio::{Events, Poll};

/// The I/O reactor — wraps mio's event polling.
pub struct Reactor {
    poll: Poll,
    events: Events,
}

impl Reactor {
    /// Create a new reactor.
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            events: Events::with_capacity(1024),
        })
    }

    /// Poll for I/O events with an optional timeout.
    /// Returns the number of events ready.
    pub fn poll_events(&mut self, timeout: Option<std::time::Duration>) -> std::io::Result<usize> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(self.events.iter().count())
    }

    /// Get a reference to the inner mio::Poll for registering interest.
    pub fn registry(&self) -> &mio::Registry {
        self.poll.registry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reactor_creates_successfully() {
        let reactor = Reactor::new();
        assert!(reactor.is_ok());
    }
}
