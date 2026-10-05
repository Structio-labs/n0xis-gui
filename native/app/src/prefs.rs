// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! What the user chose about analysis and the cache, kept in `settings.json` in
//! the GUI's config folder. Appearance (theme, font size) is kept apart, in
//! `ui-settings.json`, by `appearance`.

use std::path::PathBuf;

use gpui_kit::*;
use serde::{Deserialize, Serialize};

use crate::config;

const FILE_NAME: &str = "settings.json";

/// The shape of `settings.json`. A file of another version is read as defaults.
const VERSION: u32 = 1;

/// Where a target's project (its `.n0x/`: names, comments, types and caches) lives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectLocation {
    /// One folder per target under the user's data folder, shared with the
    /// Tauri build. Nothing is written next to the user's files.
    Central,
    /// A `.n0x/` next to the target. Not possible in a folder the user cannot
    /// write to; the target then goes to the central folder, and the user is told.
    BesideTheTarget,
    /// One folder per target under a folder the user picked.
    Folder { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    /// Start the engine's whole-program pass when a target opens.
    pub analyze_on_open: bool,
    /// Let that pass decode and cache every function, so the first decompile of
    /// any of them is instant; costs time and disk at open.
    pub warm_up: bool,
    /// Keep the caches when a target is closed. Off clears them; the user's
    /// own work is never cleared.
    pub keep_cache_on_close: bool,
    pub project_location: ProjectLocation,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { analyze_on_open: true, warm_up: false, keep_cache_on_close: true, project_location: ProjectLocation::Central }
    }
}

impl Global for Prefs {}

#[derive(Serialize, Deserialize)]
struct PrefsFile {
    version: u32,
    #[serde(flatten)]
    prefs: Prefs,
}

impl Prefs {
    /// Read `json`; anything unreadable or of another version gives the reason.
    pub fn parse(json: &str) -> Result<Self, String> {
        config::parse_versioned::<PrefsFile>(json, VERSION, "a settings file").map(|f| f.prefs)
    }

    /// The saved settings, or the defaults and why the file was not used. A
    /// missing file is a first run, not a problem.
    pub fn read() -> (Self, Option<String>) {
        let Some(path) = config::dir().map(|d| d.join(FILE_NAME)) else { return (Self::default(), None) };
        match std::fs::read_to_string(&path) {
            Ok(json) => match Self::parse(&json) {
                Ok(prefs) => (prefs, None),
                Err(why) => (Self::default(), Some(format!("The settings in {} were not read, so the defaults apply: {why}.", path.display()))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("The settings in {} were not read: {e}.", path.display()))),
        }
    }

    pub fn write(&self) -> std::io::Result<()> {
        let path = config::dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config folder"))?
            .join(FILE_NAME);
        let json = serde_json::to_string_pretty(&PrefsFile { version: VERSION, prefs: self.clone() }).map_err(std::io::Error::other)?;
        config::write_atomic(&path, &json)
    }
}

/// Change the settings and keep them; a failed save is reported.
pub fn update(cx: &mut App, change: impl FnOnce(&mut Prefs)) -> std::io::Result<()> {
    cx.update_global::<Prefs, _>(|prefs, _| change(prefs));
    cx.global::<Prefs>().write()
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{Prefs, ProjectLocation};

    #[test]
    fn the_saved_shape_reads_back_and_a_foreign_one_says_why() {
        let prefs = Prefs { warm_up: true, project_location: ProjectLocation::Folder { path: "/data/re".into() }, ..Prefs::default() };
        let json = serde_json::to_string(&super::PrefsFile { version: super::VERSION, prefs: prefs.clone() }).unwrap();
        assert_eq!(Prefs::parse(&json), Ok(prefs));
        assert!(json.contains(r#""kind":"folder""#), "the location says which kind it is: {json}");
        assert!(Prefs::parse("{}").is_err());
        assert!(Prefs::parse(r#"{"version":7}"#).unwrap_err().contains("version 7"));
    }

    #[test]
    fn the_defaults_analyze_and_keep_the_cache() {
        let d = Prefs::default();
        assert!(d.analyze_on_open && !d.warm_up && d.keep_cache_on_close);
        assert_eq!(d.project_location, ProjectLocation::Central);
    }
}
