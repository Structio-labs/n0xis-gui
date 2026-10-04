// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Embed the icons in `icons/n0xis/` and give each one a name in `AppIcon`.
//! The folder is the only list: an icon file without a name, or a name
//! without a file, cannot be built.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let dir = Path::new(&manifest).join("icons").join("n0xis");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|x| x == "svg"))
        .collect();
    files.sort();

    let (mut variants, mut paths, mut bytes, mut all) = (String::new(), String::new(), String::new(), String::new());
    for file in &files {
        let stem = file.file_stem().and_then(|s| s.to_str()).expect("an icon file name is UTF-8");
        let variant: String = stem
            .split(['-', '_'])
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map(|first| first.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
            })
            .collect();
        assert!(
            variant.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && variant.chars().all(|c| c.is_ascii_alphanumeric()),
            "icon file {stem}.svg does not make a Rust name"
        );
        let _ = writeln!(variants, "    {variant},");
        let _ = writeln!(paths, "            Self::{variant} => \"icons/n0xis/{stem}.svg\",");
        let _ = writeln!(bytes, "            Self::{variant} => include_bytes!({:?}),", file.display().to_string());
        let _ = write!(all, "Self::{variant}, ");
    }
    let count = files.len();
    let code = format!(
        "/// One of the Tauri build's icons, embedded from `icons/n0xis/`.\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n\
         pub enum AppIcon {{\n{variants}}}\n\n\
         impl AppIcon {{\n\
         \x20   pub const ALL: [AppIcon; {count}] = [{all}];\n\n\
         \x20   /// Where the asset source serves it.\n\
         \x20   pub const fn asset_path(self) -> &'static str {{\n        match self {{\n{paths}        }}\n    }}\n\n\
         \x20   pub const fn svg(self) -> &'static [u8] {{\n        match self {{\n{bytes}        }}\n    }}\n\
         }}\n"
    );
    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("icons.rs");
    std::fs::write(&out, code).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
}
