//! Target triple abstraction for the Argon compiler.
//!
//! Provides target detection, parsing, and platform-specific configuration
//! for native binary compilation.

use std::fmt;
use std::str::FromStr;
pub use target_lexicon::Triple;

#[derive(Debug, Clone)]
pub struct TargetTriple {
    pub triple: Triple,
}

#[derive(thiserror::Error, Debug)]
pub enum TargetError {
    #[error("unsupported target triple: {0}")]
    UnsupportedTriple(String),
}

/// Object file format for the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFormat {
    Elf,
    MachO,
    Coff,
}

impl TargetTriple {
    /// Returns the host machine's target triple.
    pub fn host() -> Self {
        Self {
            triple: Triple::host(),
        }
    }

    /// Parse a target triple from a string like "x86_64-unknown-linux-gnu".
    pub fn parse(s: &str) -> Result<Self, TargetError> {
        let triple =
            Triple::from_str(s).map_err(|_| TargetError::UnsupportedTriple(s.to_string()))?;
        Ok(Self { triple })
    }

    /// Returns the object file format (ELF, Mach-O, COFF) for this target.
    pub fn object_format(&self) -> ObjectFormat {
        use target_lexicon::BinaryFormat;
        match self.triple.binary_format {
            BinaryFormat::Elf => ObjectFormat::Elf,
            BinaryFormat::Macho => ObjectFormat::MachO,
            BinaryFormat::Coff => ObjectFormat::Coff,
            _ => ObjectFormat::Elf, // fallback
        }
    }

    /// Returns the default linker command for this target.
    pub fn default_linker(&self) -> &'static str {
        use target_lexicon::OperatingSystem;
        match self.triple.operating_system {
            OperatingSystem::Windows => "link.exe",
            _ => "cc",
        }
    }

    /// Returns the executable file suffix for this target.
    pub fn exe_suffix(&self) -> &'static str {
        use target_lexicon::OperatingSystem;
        match self.triple.operating_system {
            OperatingSystem::Windows => ".exe",
            _ => "",
        }
    }

    /// Returns the object file suffix for this target.
    pub fn obj_suffix(&self) -> &'static str {
        use target_lexicon::OperatingSystem;
        match self.triple.operating_system {
            OperatingSystem::Windows => ".obj",
            _ => ".o",
        }
    }

    /// Returns true if this triple targets the same platform as the host.
    pub fn is_host(&self) -> bool {
        let host = Triple::host();
        self.triple.architecture == host.architecture
            && self.triple.operating_system == host.operating_system
    }

    /// Returns the triple as a string.
    pub fn triple_str(&self) -> String {
        self.triple.to_string()
    }

    /// Returns the pointer width in bytes for this target.
    pub fn pointer_bytes(&self) -> u8 {
        use target_lexicon::Architecture;
        match self.triple.architecture {
            Architecture::X86_64 | Architecture::Aarch64(_) => 8,
            Architecture::X86_32(_) | Architecture::Arm(_) => 4,
            _ => 8, // default to 64-bit
        }
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.triple)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_triple_is_valid() {
        let host = TargetTriple::host();
        assert!(host.is_host());
        assert!(!host.triple_str().is_empty());
    }

    #[test]
    fn parse_linux_x86_64() {
        let t = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(t.object_format(), ObjectFormat::Elf);
        assert_eq!(t.exe_suffix(), "");
        assert_eq!(t.obj_suffix(), ".o");
        assert_eq!(t.default_linker(), "cc");
        assert_eq!(t.pointer_bytes(), 8);
    }

    #[test]
    fn parse_macos_aarch64() {
        let t = TargetTriple::parse("aarch64-apple-darwin").unwrap();
        assert_eq!(t.object_format(), ObjectFormat::MachO);
        assert_eq!(t.exe_suffix(), "");
        assert_eq!(t.pointer_bytes(), 8);
    }

    #[test]
    fn parse_windows_x86_64() {
        let t = TargetTriple::parse("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(t.object_format(), ObjectFormat::Coff);
        assert_eq!(t.exe_suffix(), ".exe");
        assert_eq!(t.obj_suffix(), ".obj");
        assert_eq!(t.default_linker(), "link.exe");
    }

    #[test]
    fn invalid_triple_errors() {
        assert!(TargetTriple::parse("not-a-real-triple").is_err());
    }
}
