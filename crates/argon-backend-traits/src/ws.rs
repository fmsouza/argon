//! WebSocket backend traits.
//!
//! Each compilation target implements these traits to provide
//! platform-specific WebSocket client and server operations.

use crate::fs::IoResult;

/// A received WebSocket message.
#[derive(Debug, Clone)]
pub struct WsMessage {
    /// Text content (empty for binary frames).
    pub data: String,
    /// Binary content (empty for text frames).
    pub bytes: Vec<u8>,
    /// Whether this is a text frame.
    pub is_text: bool,
    /// Whether this is a binary frame.
    pub is_binary: bool,
}

/// WebSocket client operations.
pub trait WsClientOps {
    /// Opaque connection handle type.
    type Connection;

    /// Connect to a WebSocket server at the given URL (ws:// or wss://).
    fn connect(&self, url: &str) -> IoResult<Self::Connection>;
    /// Send a text message.
    fn send(&self, conn: &mut Self::Connection, data: &str) -> IoResult<()>;
    /// Send a binary message.
    fn send_bytes(&self, conn: &mut Self::Connection, data: &[u8]) -> IoResult<()>;
    /// Receive a message (blocks until one arrives).
    fn recv(&self, conn: &mut Self::Connection) -> IoResult<WsMessage>;
    /// Send a ping frame.
    fn ping(&self, conn: &mut Self::Connection) -> IoResult<()>;
    /// Close the connection with an optional close code and reason.
    fn close(&self, conn: Self::Connection, code: u16, reason: &str) -> IoResult<()>;
    /// Check if the connection is still open.
    fn is_open(&self, conn: &Self::Connection) -> bool;
}

/// WebSocket server operations.
pub trait WsServerOps {
    /// Opaque server handle type.
    type Server;
    /// Opaque connection handle type (same as client).
    type Connection;

    /// Start a WebSocket server listening on the given address and port.
    fn listen(&self, addr: &str, port: u16) -> IoResult<Self::Server>;
    /// Accept a new incoming WebSocket connection. Blocks until one arrives.
    fn accept(&self, server: &mut Self::Server) -> IoResult<Self::Connection>;
    /// Get the address the server is bound to.
    fn addr(&self, server: &Self::Server) -> IoResult<String>;
    /// Shut down the server.
    fn close(&self, server: Self::Server) -> IoResult<()>;
}
