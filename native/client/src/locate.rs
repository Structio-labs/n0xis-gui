// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ClientError;

/// The environment variable that names the engine binary explicitly.
pub const ENGINE_ENV: &str = "N0XIS_BIN";

const ENGINE_FILE: &str = if cfg!(windows) { "n0xis.exe" } else { "n0xis" };

/// How to run the engine: a program, plus any arguments that come before the
/// subcommand (a wrapper, or an interpreter for a test double).
#[derive(Debug, Clone)]
pub struct EngineCommand {
    program: PathBuf,
    prefix: Vec<OsString>,
}

impl EngineCommand {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self { program: program.into(), prefix: Vec::new() }
    }

    pub fn with_prefix<I, A>(program: impl Into<PathBuf>, prefix: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<OsString>,
    {
        Self { program: program.into(), prefix: prefix.into_iter().map(Into::into).collect() }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    /// `$N0XIS_BIN`, then `~/.local/bin/n0xis`, then the first `n0xis` on
    /// `PATH`. A `$N0XIS_BIN` that names no file is an error, not a reason to
    /// fall through: whoever set it meant that engine, and silently running a
    /// different one would make every answer come from somewhere unexpected.
    pub fn locate() -> Result<Self, ClientError> {
        if let Some(p) = std::env::var_os(ENGINE_ENV).filter(|p| !p.is_empty()) {
            let p = PathBuf::from(p);
            return if p.is_file() {
                Ok(Self::new(p))
            } else {
                Err(ClientError::EngineNotFound(format!("${ENGINE_ENV} is {}, which is not a file", p.display())))
            };
        }
        let mut looked = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            let p = Path::new(&home).join(".local/bin").join(ENGINE_FILE);
            if p.is_file() {
                return Ok(Self::new(p));
            }
            looked.push(p.display().to_string());
        }
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                let p = dir.join(ENGINE_FILE);
                if p.is_file() {
                    return Ok(Self::new(p));
                }
            }
            looked.push("every directory on PATH".into());
        }
        Err(ClientError::EngineNotFound(format!("set ${ENGINE_ENV}; looked in {}", looked.join(", "))))
    }

    pub(crate) fn command(&self) -> Command {
        let mut c = Command::new(&self.program);
        c.args(&self.prefix);
        c
    }
}
