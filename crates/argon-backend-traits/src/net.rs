//! Networking backend traits.
//!
//! Each compilation target implements these traits to provide
//! platform-specific TCP, UDP, and DNS operations.

use crate::fs::IoResult;

/// Received UDP message with source address.
#[derive(Debug, Clone)]
pub struct UdpMessage {
    pub data: String,
    pub addr: String,
    pub port: u16,
}

/// TCP listener (server socket) operations.
pub trait TcpListenerOps {
    /// Opaque listener handle type.
    type Listener;
    /// Opaque stream handle type (returned by accept).
    type Stream;

    /// Bind to an address and port, returning a listener.
    fn bind(&self, addr: &str, port: u16) -> IoResult<Self::Listener>;
    /// Accept a new incoming connection. Blocks until one arrives.
    fn accept(&self, listener: &mut Self::Listener) -> IoResult<Self::Stream>;
    /// Close the listener.
    fn close(&self, listener: Self::Listener) -> IoResult<()>;
    /// Get the local address the listener is bound to.
    fn local_addr(&self, listener: &Self::Listener) -> IoResult<String>;
}

/// TCP stream (client connection) operations.
pub trait TcpStreamOps {
    /// Opaque stream handle type.
    type Stream;

    /// Connect to a remote address and port.
    fn connect(&self, addr: &str, port: u16) -> IoResult<Self::Stream>;
    /// Read up to `max_bytes` from the stream as a UTF-8 string.
    fn read(&self, stream: &mut Self::Stream, max_bytes: usize) -> IoResult<String>;
    /// Write data to the stream. Returns bytes written.
    fn write(&self, stream: &mut Self::Stream, data: &str) -> IoResult<usize>;
    /// Shut down the write half of the connection (TCP half-close).
    fn shutdown(&self, stream: &mut Self::Stream) -> IoResult<()>;
    /// Close the stream.
    fn close(&self, stream: Self::Stream) -> IoResult<()>;
    /// Get the remote peer address.
    fn peer_addr(&self, stream: &Self::Stream) -> IoResult<String>;
}

/// UDP socket operations.
pub trait UdpSocketOps {
    /// Opaque socket handle type.
    type Socket;

    /// Bind to an address and port.
    fn bind(&self, addr: &str, port: u16) -> IoResult<Self::Socket>;
    /// Send data to a specific address and port. Returns bytes sent.
    fn send_to(
        &self,
        socket: &mut Self::Socket,
        data: &str,
        addr: &str,
        port: u16,
    ) -> IoResult<usize>;
    /// Receive data from the socket. Returns data and source address.
    fn recv_from(&self, socket: &mut Self::Socket, max_bytes: usize) -> IoResult<UdpMessage>;
    /// Close the socket.
    fn close(&self, socket: Self::Socket) -> IoResult<()>;
}

/// DNS resolution operations.
pub trait DnsOps {
    /// Resolve a hostname to a list of IP address strings.
    fn resolve(&self, hostname: &str) -> IoResult<Vec<String>>;
}
