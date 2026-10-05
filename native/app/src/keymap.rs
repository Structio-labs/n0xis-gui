// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The application's key bindings. One table of commands drives both what is
//! bound and the editor in Settings ▸ Keys, so the two cannot disagree. The
//! user's changes are kept in `keybindings.json`; a command not named there
//! keeps its default.
//!
//! Letters and digits match the physical key: on Linux the platform reads a
//! shortcut typed in a non-Latin layout (Ctrl and the key that prints `п`) as
//! the Latin key in that place (`ctrl-g`).

use std::collections::BTreeMap;

use gpui_kit::component::dock::ToggleZoom;
use gpui_kit::*;
use serde::{Deserialize, Serialize};

use crate::appearance::{ResetZoom, ZoomIn, ZoomOut};
use crate::config;
use crate::{
    Comment, GoBack, GoForward, GoTo, Open, OpenSettings, Quit, RedoEdit, RedoLayout, Rename, ToggleBookmark, UndoEdit,
    UndoLayout,
};

const FILE_NAME: &str = "keybindings.json";
const VERSION: u32 = 1;

/// The read-only decompiler is a text field, and a text field takes Ctrl+Z
/// for itself; the edit history is bound there too.
const DECOMPILER_INPUT: &str = "Decompiler > Input";

/// A command the user can bind.
pub struct Command {
    /// What `keybindings.json` calls it.
    pub id: &'static str,
    pub label: &'static str,
    /// The menu it lives in.
    pub group: &'static str,
    pub defaults: &'static [&'static str],
    /// Every context the command is bound in. `None` is everywhere.
    contexts: &'static [Option<&'static str>],
    /// The command's bindings for a keystroke, in the given contexts.
    bind: fn(&str, Option<&'static str>) -> KeyBinding,
}

macro_rules! command {
    ($id:literal, $label:literal, $group:literal, [$($key:literal),*], $action:expr) => {
        command!($id, $label, $group, [$($key),*], $action, [None])
    };
    ($id:literal, $label:literal, $group:literal, [$($key:literal),*], $action:expr, [$($ctx:expr),*]) => {
        Command {
            id: $id,
            label: $label,
            group: $group,
            defaults: &[$($key),*],
            contexts: &[$($ctx),*],
            bind: |key, context| KeyBinding::new(key, $action, context),
        }
    };
}

pub static COMMANDS: &[Command] = &[
    command!("open", "Open…", "File", ["ctrl-o"], Open),
    command!("settings", "Settings…", "File", ["ctrl-,"], OpenSettings),
    command!("quit", "Quit", "File", ["ctrl-q"], Quit),
    command!("undo_edit", "Undo edit", "Edit", ["ctrl-z"], UndoEdit, [None, Some(DECOMPILER_INPUT)]),
    command!("redo_edit", "Redo edit", "Edit", ["ctrl-y"], RedoEdit, [None, Some(DECOMPILER_INPUT)]),
    command!("rename", "Rename…", "Edit", ["f2"], Rename),
    command!("comment", "Comment…", "Edit", ["ctrl-/"], Comment),
    command!("toggle_bookmark", "Toggle bookmark", "Edit", ["ctrl-d"], ToggleBookmark),
    command!("go_to", "Go to address or name…", "Go", ["ctrl-g"], GoTo),
    command!("go_back", "Back", "Go", ["alt-left"], GoBack),
    command!("go_forward", "Forward", "Go", ["alt-right"], GoForward),
    command!("zoom_in", "Zoom in", "View", ["ctrl-=", "ctrl-+"], ZoomIn),
    command!("zoom_out", "Zoom out", "View", ["ctrl--"], ZoomOut),
    command!("reset_zoom", "Reset zoom", "View", ["ctrl-0"], ResetZoom),
    command!("undo_layout", "Undo layout change", "Window", ["ctrl-shift-z"], UndoLayout),
    command!("redo_layout", "Redo layout change", "Window", ["ctrl-shift-y"], RedoLayout),
    command!("zoom_panel", "Zoom the panel", "Window", ["shift-escape"], ToggleZoom),
];

/// The user's changes to the defaults, by command id. An empty list unbinds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Keymap {
    overrides: BTreeMap<String, Vec<String>>,
}

impl Global for Keymap {}

#[derive(Serialize, Deserialize)]
struct KeymapFile {
    version: u32,
    #[serde(flatten)]
    keymap: Keymap,
}

impl Keymap {
    /// The keys `command` is bound to now.
    pub fn keys(&self, command: &Command) -> Vec<String> {
        match self.overrides.get(command.id) {
            Some(keys) => keys.clone(),
            None => command.defaults.iter().map(|k| k.to_string()).collect(),
        }
    }

    pub fn is_default(&self, command: &Command) -> bool {
        !self.overrides.contains_key(command.id)
    }

    /// Bind `command` to `keys`, taking each from any other command that had
    /// it. Answers what was taken, from which command.
    pub fn assign(&mut self, command: &Command, keys: Vec<String>) -> Vec<(String, &'static Command)> {
        let mut taken = Vec::new();
        for other in COMMANDS.iter().filter(|c| c.id != command.id) {
            let had = self.keys(other);
            if had.iter().any(|k| keys.contains(k)) {
                for k in had.iter().filter(|k| keys.contains(k)) {
                    taken.push((k.clone(), other));
                }
                let left: Vec<String> = had.into_iter().filter(|k| !keys.contains(k)).collect();
                self.set(other, left);
            }
        }
        self.set(command, keys);
        taken
    }

    fn set(&mut self, command: &Command, keys: Vec<String>) {
        if keys.iter().map(String::as_str).eq(command.defaults.iter().copied()) {
            self.overrides.remove(command.id);
        } else {
            self.overrides.insert(command.id.to_string(), keys);
        }
    }

    pub fn parse(json: &str) -> Result<Self, String> {
        config::parse_versioned::<KeymapFile>(json, VERSION, "a key binding file").map(|f| f.keymap)
    }

    /// The saved key map, or the defaults and why the file was not used.
    pub fn read() -> (Self, Option<String>) {
        let Some(path) = config::dir().map(|d| d.join(FILE_NAME)) else { return (Self::default(), None) };
        match std::fs::read_to_string(&path) {
            Ok(json) => match Self::parse(&json) {
                Ok(keymap) => (keymap, None),
                Err(why) => (Self::default(), Some(format!("The key bindings in {} were not read, so the defaults apply: {why}.", path.display()))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("The key bindings in {} were not read: {e}.", path.display()))),
        }
    }

    pub fn write(&self) -> std::io::Result<()> {
        let path = config::dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config folder"))?
            .join(FILE_NAME);
        let json = serde_json::to_string_pretty(&KeymapFile { version: VERSION, keymap: self.clone() }).map_err(std::io::Error::other)?;
        config::write_atomic(&path, &json)
    }

    /// Every binding the table and the user's changes make. A key this map does
    /// not parse is left out and named in the second value.
    pub fn bindings(&self) -> (Vec<KeyBinding>, Vec<String>) {
        let mut bindings = Vec::new();
        let mut refused = Vec::new();
        for command in COMMANDS {
            for key in self.keys(command) {
                if Keystroke::parse(&key).is_err() {
                    refused.push(format!("{key} (for {})", command.label));
                    continue;
                }
                bindings.extend(command.contexts.iter().map(|&context| (command.bind)(&key, context)));
            }
        }
        (bindings, refused)
    }
}

/// Bind everything the key map says. Called once at start.
pub fn install(cx: &mut App) -> Option<String> {
    let (bindings, refused) = cx.global::<Keymap>().bindings();
    cx.bind_keys(bindings);
    (!refused.is_empty()).then(|| format!("These key bindings could not be read and were left out: {}.", refused.join(", ")))
}

/// Change what `command` is bound to, now and for the next run. Keys another
/// command had are taken from it; the answer says which, for the user.
pub fn rebind(cx: &mut App, command: &'static Command, keys: Vec<String>) -> Result<Vec<String>, String> {
    if let Some(bad) = keys.iter().find(|k| Keystroke::parse(k).is_err()) {
        return Err(format!("{bad} is not a key the app can bind"));
    }
    let before: Vec<(&'static Command, Vec<String>)> = COMMANDS.iter().map(|c| (c, cx.global::<Keymap>().keys(c))).collect();
    let taken = cx.update_global::<Keymap, _>(|map, _| map.assign(command, keys));
    apply_changes(cx, &before);
    cx.global::<Keymap>().write().map_err(|e| format!("the key bindings could not be saved: {e}"))?;
    Ok(taken.into_iter().map(|(key, from)| format!("{} was taken from {}.", pretty(&key), from.label)).collect())
}

/// Put `command` back to its default keys.
pub fn revert(cx: &mut App, command: &'static Command) -> Result<Vec<String>, String> {
    rebind(cx, command, command.defaults.iter().map(|k| k.to_string()).collect())
}

/// Every command back to its defaults.
pub fn reset_all(cx: &mut App) -> Result<(), String> {
    let before: Vec<(&'static Command, Vec<String>)> = COMMANDS.iter().map(|c| (c, cx.global::<Keymap>().keys(c))).collect();
    cx.set_global(Keymap::default());
    apply_changes(cx, &before);
    cx.global::<Keymap>().write().map_err(|e| format!("the key bindings could not be saved: {e}"))
}

/// Make the live bindings match the key map after a change: every key a
/// command lost is unbound in its contexts, then every key it has is bound,
/// so a key that moved goes to its new command.
fn apply_changes(cx: &mut App, before: &[(&'static Command, Vec<String>)]) {
    let mut bindings = Vec::new();
    for (command, old) in before {
        let now = cx.global::<Keymap>().keys(command);
        for key in old.iter().filter(|k| !now.contains(k)) {
            bindings.extend(command.contexts.iter().map(|&context| KeyBinding::new(key, NoAction, context)));
        }
    }
    let (now, _) = cx.global::<Keymap>().bindings();
    bindings.extend(now);
    cx.bind_keys(bindings);
    cx.refresh_windows();
}

/// A key as a person writes it: `ctrl-shift-z` → `Ctrl+Shift+Z`.
pub fn pretty(key: &str) -> String {
    let Ok(k) = Keystroke::parse(key) else { return key.to_string() };
    let mut parts = Vec::new();
    if k.modifiers.control {
        parts.push("Ctrl".to_string());
    }
    if k.modifiers.alt {
        parts.push("Alt".to_string());
    }
    if k.modifiers.shift {
        parts.push("Shift".to_string());
    }
    if k.modifiers.platform {
        parts.push("Super".to_string());
    }
    let mut chars = k.key.chars();
    let name = match (chars.next(), chars.next()) {
        (Some(c), None) => c.to_uppercase().to_string(),
        _ => {
            let mut c = k.key.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        }
    };
    parts.push(name);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{COMMANDS, Command, Keymap, pretty};

    fn command(id: &str) -> &'static Command {
        COMMANDS.iter().find(|c| c.id == id).expect("a command of the table")
    }

    #[test]
    fn every_command_has_its_own_id_and_its_defaults_parse() {
        let mut ids: Vec<&str> = COMMANDS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), COMMANDS.len());
        let (_, refused) = Keymap::default().bindings();
        assert!(refused.is_empty(), "{refused:?}");
        // No key is a default of two commands.
        let mut keys: Vec<&str> = COMMANDS.iter().flat_map(|c| c.defaults.iter().copied()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before, "a default key is shared");
    }

    #[test]
    fn a_key_moved_to_another_command_is_taken_from_the_first() {
        let mut map = Keymap::default();
        let rename = command("rename");
        let taken = map.assign(rename, vec!["ctrl-g".into()]);
        assert_eq!(map.keys(rename), ["ctrl-g"]);
        assert_eq!(map.keys(command("go_to")), Vec::<String>::new(), "Go to lost its only key");
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].1.id, "go_to");
        // Back to its default keys: the override goes away.
        map.assign(rename, rename.defaults.iter().map(|k| k.to_string()).collect());
        assert!(map.is_default(rename));
    }

    #[test]
    fn the_saved_map_reads_back_and_another_version_says_so() {
        let mut map = Keymap::default();
        map.assign(command("comment"), vec!["ctrl-;".into()]);
        let json = serde_json::to_string(&super::KeymapFile { version: super::VERSION, keymap: map.clone() }).unwrap();
        assert_eq!(Keymap::parse(&json), Ok(map));
        assert!(Keymap::parse(r#"{"version":5}"#).unwrap_err().contains("version 5"));
    }

    #[test]
    fn keys_read_as_a_person_writes_them() {
        assert_eq!(pretty("ctrl-shift-z"), "Ctrl+Shift+Z");
        assert_eq!(pretty("f2"), "F2");
        assert_eq!(pretty("alt-left"), "Alt+Left");
        assert_eq!(pretty("ctrl-/"), "Ctrl+/");
        assert_eq!(pretty("shift-escape"), "Shift+Escape");
    }
}
