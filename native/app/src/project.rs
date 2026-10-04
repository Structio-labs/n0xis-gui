// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Where a target's engine state lives: the directory whose `.n0x/` holds its
//! names, comments, types and caches. It is the same place the Tauri build
//! uses, so while the two front ends coexist they see one project per binary.

use std::path::{Path, PathBuf};

/// One folder per target, under the user's home directory.
const PROJECTS_DIR: &str = ".local/share/pro.n0xis.gui/projects";

/// FNV-1a, 64-bit. Not for security: it only names a folder after a path.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The folder `name` under the projects directory, created with its `.n0x/`.
fn projects_subdir(name: &str) -> std::io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no home directory"))?;
    let dir = Path::new(&home).join(PROJECTS_DIR).join(name);
    std::fs::create_dir_all(dir.join(".n0x"))?;
    Ok(dir)
}

/// The project directory for `target`, created with its `.n0x/` if missing.
pub fn project_dir(target: &Path) -> std::io::Result<PathBuf> {
    projects_subdir(&format!("{:x}", fnv1a64(target.to_string_lossy().as_bytes())))
}

/// Where scans of running processes keep the results a later scan narrows.
/// One folder for all of them: a result is named after its process.
pub fn live_dir() -> std::io::Result<PathBuf> {
    projects_subdir("live")
}

#[cfg(test)]
mod tests {
    use super::fnv1a64;

    /// The published FNV-1a 64-bit test vectors, so the folder name a target gets
    /// here is the one the Tauri build computes for it.
    #[test]
    fn fnv1a64_matches_the_published_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
