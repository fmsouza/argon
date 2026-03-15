//! HTTP backend traits.
//!
//! Each compilation target implements these traits to provide
//! platform-specific HTTP client and server operations.

use crate::fs::IoResult;

/// HTTP header entry.
#[derive(Debug, Clone)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

/// HTTP request options for the client.
#[derive(Debug, Clone)]
pub struct RequestOptions {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    pub body: String,
    /// Request timeout in milliseconds. 0 means no timeout.
    pub timeout_ms: u32,
    /// Whether to follow HTTP redirects automatically.
    pub follow_redirects: bool,
}

/// HTTP response from the client.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<HeaderEntry>,
    pub body: String,
}

/// Incoming HTTP request (server side).
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    pub body: String,
}

/// HTTP response builder (server side).
#[derive(Debug, Clone)]
pub struct HttpResponseBuilder {
    pub status: u16,
    pub headers: Vec<HeaderEntry>,
    pub body: Option<String>,
}

impl Default for HttpResponseBuilder {
    fn default() -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: None,
        }
    }
}

/// Headers collection operations.
pub trait HeadersOps {
    /// Opaque headers handle type.
    type Headers;

    /// Create a new empty headers collection.
    fn create(&self) -> Self::Headers;
    /// Get a header value by name. Returns error if not found.
    fn get(&self, headers: &Self::Headers, name: &str) -> IoResult<String>;
    /// Set a header value.
    fn set(&self, headers: &mut Self::Headers, name: &str, value: &str);
    /// Check if a header exists.
    fn has(&self, headers: &Self::Headers, name: &str) -> bool;
    /// Delete a header.
    fn delete(&self, headers: &mut Self::Headers, name: &str);
    /// Get all header entries.
    fn entries(&self, headers: &Self::Headers) -> Vec<HeaderEntry>;
}

/// HTTP client operations.
pub trait HttpClientOps {
    /// Send an HTTP request and return the response.
    fn request(&self, options: &RequestOptions) -> IoResult<Response>;
}

/// HTTP server operations.
pub trait HttpServerOps {
    /// Opaque server handle type.
    type Server;

    /// Start an HTTP server on the given port.
    /// The handler is called for each incoming request.
    /// Returns a server handle for shutdown.
    ///
    /// In synchronous mode, this blocks the calling thread.
    fn serve(
        &self,
        port: u16,
        handler: Box<dyn Fn(HttpRequest) -> HttpResponseBuilder>,
    ) -> IoResult<Self::Server>;

    /// Get the address the server is bound to.
    fn addr(&self, server: &Self::Server) -> IoResult<String>;

    /// Shut down the server.
    fn close(&self, server: Self::Server) -> IoResult<()>;
}
