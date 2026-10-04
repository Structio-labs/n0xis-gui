// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Where the GUI keeps its own files (layout, appearance, user themes), and how
//! it writes them.

use std::path::{Path, PathBuf};

/// `$XDG_CONFIG_HOME/n0xis`, falling back to `%APPDATA%\n0xis` on Windows and
/// `~/.config/n0xis` elsewhere.
pub fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("n0xis"))
}

/// Write through a temporary file and a rename, so a crash mid-write leaves
/// the previous file, never half of one.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}
