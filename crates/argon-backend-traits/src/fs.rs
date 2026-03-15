//! File system backend traits.
//!
//! Each compilation target (JS, WASM, Native) and the runtime interpreter
//! implement these traits to provide platform-specific file system operations.

/// Result type for all I/O operations.
pub type IoResult<T> = Result<T, IoError>;

/// Unified error type for all I/O operations across fs, net, http, ws.
#[derive(Debug, Clone)]
pub struct IoError {
    /// Error code (e.g., "ENOENT", "EACCES", "EISDIR", "ECONNREFUSED").
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for IoError {}

impl IoError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// File metadata returned by `stat`.
#[derive(Debug, Clone)]
pub struct FileStat {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Last modified time as Unix timestamp in milliseconds.
    pub modified: u64,
    /// Creation time as Unix timestamp in milliseconds.
    pub created: u64,
}

/// Directory entry returned by `readDir`.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

/// Seek position for `File.seek`.
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    Start,
    Current,
    End,
}

/// File open mode.
#[derive(Debug, Clone, Copy)]
pub enum FileMode {
    Read,
    Write,
    Append,
    ReadWrite,
    WriteAppend,
}

/// Read files (whole-file operations).
pub trait FileReadOps {
    /// Read an entire file as a UTF-8 string.
    fn read_file(&self, path: &str) -> IoResult<String>;
    /// Read an entire file as raw bytes.
    fn read_bytes(&self, path: &str) -> IoResult<Vec<u8>>;
}

/// Write files (whole-file operations).
pub trait FileWriteOps {
    /// Write a UTF-8 string to a file, replacing existing content.
    fn write_file(&self, path: &str, content: &str) -> IoResult<()>;
    /// Write raw bytes to a file, replacing existing content.
    fn write_bytes(&self, path: &str, data: &[u8]) -> IoResult<()>;
    /// Append a UTF-8 string to a file.
    fn append_file(&self, path: &str, content: &str) -> IoResult<()>;
}

/// Directory operations.
pub trait DirOps {
    /// List entries in a directory.
    fn read_dir(&self, path: &str) -> IoResult<Vec<DirEntry>>;
    /// Create a directory.
    fn mkdir(&self, path: &str) -> IoResult<()>;
    /// Create a directory and all parent directories.
    fn mkdir_recursive(&self, path: &str) -> IoResult<()>;
    /// Remove an empty directory.
    fn rmdir(&self, path: &str) -> IoResult<()>;
    /// Remove a directory and all its contents recursively.
    fn remove_recursive(&self, path: &str) -> IoResult<()>;
}

/// Path inspection and manipulation.
pub trait PathOps {
    /// Check if a path exists.
    fn exists(&self, path: &str) -> IoResult<bool>;
    /// Get file/directory metadata.
    fn stat(&self, path: &str) -> IoResult<FileStat>;
    /// Rename a file or directory.
    fn rename(&self, from: &str, to: &str) -> IoResult<()>;
    /// Remove a file.
    fn remove(&self, path: &str) -> IoResult<()>;
    /// Copy a file.
    fn copy(&self, from: &str, to: &str) -> IoResult<()>;
    /// Create a symbolic link.
    fn symlink(&self, target: &str, path: &str) -> IoResult<()>;
    /// Read the target of a symbolic link.
    fn readlink(&self, path: &str) -> IoResult<String>;
    /// Get the system temporary directory path.
    fn temp_dir(&self) -> IoResult<String>;
}

/// Streaming file handle operations.
pub trait FileHandleOps {
    /// Opaque file handle type.
    type Handle;

    /// Open a file with the given mode.
    fn open(&self, path: &str, mode: FileMode) -> IoResult<Self::Handle>;
    /// Read up to `max_bytes` from the file as a UTF-8 string.
    fn read(&self, handle: &mut Self::Handle, max_bytes: usize) -> IoResult<String>;
    /// Read up to `max_bytes` from the file as raw bytes.
    fn read_bytes(&self, handle: &mut Self::Handle, max_bytes: usize) -> IoResult<Vec<u8>>;
    /// Write a UTF-8 string to the file. Returns bytes written.
    fn write(&self, handle: &mut Self::Handle, data: &str) -> IoResult<usize>;
    /// Write raw bytes to the file. Returns bytes written.
    fn write_bytes(&self, handle: &mut Self::Handle, data: &[u8]) -> IoResult<usize>;
    /// Seek to a position in the file.
    fn seek(&self, handle: &mut Self::Handle, offset: i64, whence: SeekFrom) -> IoResult<()>;
    /// Close the file handle.
    fn close(&self, handle: Self::Handle) -> IoResult<()>;
}
