// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! What went wrong while the app quit, said when it next starts: a quitting
//! app has no window left to say it in.

use std::io::Write as _;
use std::path::Path;

use crate::config;

const FILE_NAME: &str = "said-at-next-start.txt";

/// Keep `text` to be said when the app next starts. It is printed too, for a
/// terminal the app was started from, which is all that is left when it
/// cannot be kept.
pub fn keep(text: &str) {
    eprintln!("n0xis-ui: {text}");
    let Some(dir) = config::dir() else { return };
    if let Err(e) = keep_in(&dir, text) {
        eprintln!("n0xis-ui: this could not be kept to be said at the next start ({}): {e}", dir.join(FILE_NAME).display());
    }
}

/// What was kept since the last start, said once: the file goes.
pub fn take() -> Vec<String> {
    config::dir().map_or_else(Vec::new, |dir| take_from(&dir))
}

fn keep_in(dir: &Path, text: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(FILE_NAME))?;
    // One note a line.
    writeln!(file, "{}", text.replace(['\r', '\n'], " "))
}

fn take_from(dir: &Path) -> Vec<String> {
    let path = dir.join(FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => return vec![format!("What went wrong when N0xis last quit could not be read from {}: {e}.", path.display())],
    };
    let mut notes: Vec<String> = text.lines().filter(|line| !line.trim().is_empty()).map(str::to_string).collect();
    if let Err(e) = std::fs::remove_file(&path) {
        notes.push(format!("{} could not be removed, so this may be said again: {e}.", path.display()));
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::{keep_in, take_from};

    #[test]
    fn a_note_kept_on_quitting_is_said_once_at_the_next_start() {
        let dir = std::env::temp_dir().join(format!("n0xis-next-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(take_from(&dir), Vec::<String>::new(), "nothing kept, nothing said");
        keep_in(&dir, "first").unwrap();
        keep_in(&dir, "second,\nover two lines").unwrap();
        assert_eq!(take_from(&dir), ["first", "second, over two lines"]);
        assert_eq!(take_from(&dir), Vec::<String>::new(), "said once");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
