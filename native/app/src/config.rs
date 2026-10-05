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

/// Read one of the GUI's own versioned files (`{"version": N, …}`). The version
/// is read first, so a file of another version says so instead of reading as a
/// broken file of this one. `what` names the file's content in the reason.
pub fn parse_versioned<T: serde::de::DeserializeOwned>(json: &str, version: u32, what: &str) -> Result<T, String> {
    #[derive(serde::Deserialize)]
    struct Version {
        version: u32,
    }
    let found = serde_json::from_str::<Version>(json).map_err(|e| format!("it is not {what} ({e})"))?.version;
    if found != version {
        return Err(format!("it was written in version {found} of its shape, not {version}"));
    }
    serde_json::from_str(json).map_err(|e| format!("it is not {what} ({e})"))
}

#[cfg(test)]
mod tests {
    use super::parse_versioned;

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct File {
        version: u32,
        items: Vec<u8>,
    }

    #[test]
    fn another_version_says_so_whatever_its_shape() {
        assert_eq!(parse_versioned::<File>(r#"{"version":1,"items":[2]}"#, 1, "a list"), Ok(File { version: 1, items: vec![2] }));
        let other = parse_versioned::<File>(r#"{"version":2,"entries":{}}"#, 1, "a list").unwrap_err();
        assert!(other.contains("version 2"), "{other}");
        assert!(parse_versioned::<File>(r#"{"version":1}"#, 1, "a list").unwrap_err().contains("not a list"));
        assert!(parse_versioned::<File>("[", 1, "a list").is_err());
    }
}
