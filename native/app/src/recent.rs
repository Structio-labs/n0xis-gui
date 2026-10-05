// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The targets opened lately, newest first, so that File ▸ Open Recent and the
//! start screen can open one again. Kept in `recent.json` in the GUI's config
//! folder; the path is what is stored, so choosing an entry really opens it.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use serde::{Deserialize, Serialize};

use crate::config;

/// How many targets are remembered.
pub const MAX_RECENT: usize = 8;

const FILE_NAME: &str = "recent.json";

/// The shape of `recent.json`. A file of another version is read as empty.
const VERSION: u32 = 1;

/// Open a target opened before, by its path.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct OpenRecent(pub SharedString);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentTarget {
    pub path: PathBuf,
    /// When it was last opened, in seconds since the Unix epoch.
    pub opened: u64,
}

impl RecentTarget {
    /// `name — folder`, what a menu shows for it.
    pub fn label(&self) -> String {
        let name = self.path.file_name().map_or_else(|| self.path.display().to_string(), |n| n.to_string_lossy().into_owned());
        match self.path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => format!("{name} — {}", dir.display()),
            _ => name,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RecentFile {
    version: u32,
    targets: Vec<RecentTarget>,
}

#[derive(Debug, Default)]
pub struct Recent {
    targets: Vec<RecentTarget>,
}

impl Global for Recent {}

impl Recent {
    pub fn targets(&self) -> &[RecentTarget] {
        &self.targets
    }

    /// `path` was just opened: it goes first, and only once.
    pub fn opened(&mut self, path: &Path, now: u64) {
        self.targets.retain(|t| t.path != path);
        self.targets.insert(0, RecentTarget { path: path.to_path_buf(), opened: now });
        self.targets.truncate(MAX_RECENT);
    }

    pub fn forget(&mut self, path: &Path) {
        self.targets.retain(|t| t.path != path);
    }

    /// Read the list from `json`; a damaged file or one of another version
    /// gives an empty list and the reason.
    pub fn parse(json: &str) -> Result<Self, String> {
        let file: RecentFile = config::parse_versioned(json, VERSION, "a list of targets")?;
        let mut recent = Self { targets: file.targets };
        recent.targets.truncate(MAX_RECENT);
        Ok(recent)
    }

    /// Read the saved list. A missing file is a first run, not a problem; a
    /// damaged one is reported and replaced on the next save.
    pub fn read() -> (Self, Option<String>) {
        let Some(path) = config::dir().map(|d| d.join(FILE_NAME)) else { return (Self::default(), None) };
        match std::fs::read_to_string(&path) {
            Ok(json) => match Self::parse(&json) {
                Ok(recent) => (recent, None),
                Err(why) => (Self::default(), Some(format!("The recent targets in {} were not read: {why}.", path.display()))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("The recent targets in {} were not read: {e}.", path.display()))),
        }
    }

    pub fn write(&self) -> std::io::Result<()> {
        let path = config::dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config folder"))?
            .join(FILE_NAME);
        let file = RecentFile { version: VERSION, targets: self.targets.clone() };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        config::write_atomic(&path, &json)
    }
}

/// Seconds since the Unix epoch, for stamping an open.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{MAX_RECENT, Recent, RecentTarget};
    use std::path::{Path, PathBuf};

    #[test]
    fn the_latest_open_goes_first_and_only_once() {
        let mut r = Recent::default();
        r.opened(Path::new("/a"), 1);
        r.opened(Path::new("/b"), 2);
        r.opened(Path::new("/a"), 3);
        let paths: Vec<_> = r.targets().iter().map(|t| t.path.clone()).collect();
        assert_eq!(paths, [PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(r.targets()[0].opened, 3);
        for i in 0..20 {
            r.opened(Path::new(&format!("/t{i}")), 10 + i);
        }
        assert_eq!(r.targets().len(), MAX_RECENT);
        r.forget(Path::new("/t19"));
        assert_eq!(r.targets()[0].path, PathBuf::from("/t18"));
    }

    #[test]
    fn the_saved_list_reads_back_and_a_damaged_one_says_why() {
        let mut r = Recent::default();
        r.opened(Path::new("/x/target"), 7);
        let json = serde_json::to_string(&super::RecentFile { version: super::VERSION, targets: r.targets().to_vec() }).unwrap();
        assert_eq!(Recent::parse(&json).unwrap().targets(), r.targets());
        assert!(Recent::parse("{").is_err());
        assert!(Recent::parse(r#"{"version":99,"paths":{}}"#).unwrap_err().contains("version 99"), "another version says so whatever its shape");
    }

    #[test]
    fn a_label_names_the_file_and_its_folder() {
        let t = RecentTarget { path: PathBuf::from("/home/u/bin/app.exe"), opened: 0 };
        assert_eq!(t.label(), "app.exe — /home/u/bin");
    }
}
