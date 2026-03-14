//! System linker invocation for producing executables.

use crate::CodegenError;
use argon_target::TargetTriple;
use std::path::PathBuf;
use std::process::Command;

/// Configuration for the linker.
pub struct LinkerConfig {
    pub triple: TargetTriple,
    pub output: PathBuf,
    pub objects: Vec<PathBuf>,
}

/// Link object files into an executable.
pub fn link(config: &LinkerConfig) -> Result<(), CodegenError> {
    let linker = config.triple.default_linker();

    if !config.triple.is_host() {
        return Err(CodegenError::LinkerError(format!(
            "cross-linking not yet supported; generated object file for '{}' — \
             use a cross-linker manually to produce the executable",
            config.triple
        )));
    }

    let mut cmd = Command::new(linker);

    match config.triple.object_format() {
        argon_target::ObjectFormat::Coff => {
            // Windows MSVC linker
            cmd.arg(format!("/OUT:{}", config.output.display()));
            for obj in &config.objects {
                cmd.arg(obj);
            }
            cmd.args(["kernel32.lib", "msvcrt.lib", "legacy_stdio_definitions.lib"]);
        }
        argon_target::ObjectFormat::MachO => {
            // macOS cc
            cmd.arg("-o").arg(&config.output);
            for obj in &config.objects {
                cmd.arg(obj);
            }
            cmd.arg("-lSystem");
        }
        argon_target::ObjectFormat::Elf => {
            // Linux cc
            cmd.arg("-o").arg(&config.output);
            for obj in &config.objects {
                cmd.arg(obj);
            }
            cmd.args(["-lc", "-lm"]);
        }
    }

    let output = cmd.output().map_err(|e| {
        CodegenError::LinkerError(format!(
            "failed to run linker '{}': {}. Is the linker installed?",
            linker, e
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodegenError::LinkerError(format!(
            "linker '{}' failed:\n{}",
            linker, stderr
        )));
    }

    Ok(())
}
