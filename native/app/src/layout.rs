// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The dock: which panels exist, how they are arranged by default, and where
//! the user's own arrangement is kept between runs.

use std::path::PathBuf;

use gpui_kit::component::dock::{DockAreaState, DockLayout, panel_handle, register_panel};
use gpui_kit::*;

use crate::decompiler::DecompilerView;
use crate::disassembly::DisassemblyView;
use crate::functions::FunctionList;

/// Names a saved layout knows the panels by. Once chosen, never change them:
/// a renamed panel is a panel the saved layout no longer finds.
pub const FUNCTIONS_PANEL: &str = "functions";
pub const DECOMPILER_PANEL: &str = "decompiler";
pub const DISASSEMBLY_PANEL: &str = "disassembly";

/// The dock area's id, and the version of the default arrangement. Bump the
/// version when the default changes in a way an old saved layout should not
/// survive; a saved layout of another version is set aside, and the user told.
pub const AREA_ID: &str = "n0xis-main";
pub const LAYOUT_VERSION: usize = 1;

/// The one instance of each view. A restored layout names panels; the builders
/// registered below return these instances rather than new ones, so what a
/// view holds (the function list, the open decompilation) survives a reload.
#[derive(Clone)]
pub struct Views {
    pub functions: Entity<FunctionList>,
    pub decompiler: Entity<DecompilerView>,
    pub disassembly: Entity<DisassemblyView>,
}

impl Global for Views {}

/// Tell the dock how to rebuild each panel a saved layout names. The builders
/// read [`Views`], which the workbench sets before any layout is loaded.
pub fn register_panels(cx: &mut App) {
    register_panel(cx, FUNCTIONS_PANEL, |_, _, cx| panel_handle(cx.global::<Views>().functions.clone()));
    register_panel(cx, DECOMPILER_PANEL, |_, _, cx| panel_handle(cx.global::<Views>().decompiler.clone()));
    register_panel(cx, DISASSEMBLY_PANEL, |_, _, cx| panel_handle(cx.global::<Views>().disassembly.clone()));
}

/// Functions on the left; the decompiler above the disassembly on the right.
pub fn default_layout(views: &Views, cx: &App) -> DockLayout {
    DockLayout::h_split()
        .child(DockLayout::tabs().panel_view(panel_handle(views.functions.clone()), cx), Some(px(340.)))
        .child(
            DockLayout::v_split()
                .child(DockLayout::tabs().panel_view(panel_handle(views.decompiler.clone()), cx), None)
                .child(DockLayout::tabs().panel_view(panel_handle(views.disassembly.clone()), cx), Some(px(280.))),
            None,
        )
}

/// `$XDG_CONFIG_HOME/n0xis/ui-layout.json`, falling back to `~/.config`, or to
/// `%APPDATA%` on Windows.
pub fn layout_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("n0xis").join("ui-layout.json"))
}

/// Why a saved layout was not used. Saying so matters: a layout that silently
/// snaps back to the default looks like the app forgot what the user did.
#[derive(Debug, PartialEq)]
pub enum SavedLayout {
    Missing,
    Unreadable(String),
    OtherVersion(Option<usize>),
}

impl SavedLayout {
    /// A sentence for the user, or nothing when there was simply no saved layout.
    pub fn explain(&self) -> Option<String> {
        match self {
            Self::Missing => None,
            Self::Unreadable(e) => Some(format!("The saved panel layout could not be read ({e}); using the default.")),
            Self::OtherVersion(v) => Some(format!(
                "The saved panel layout is from another version ({}); using the default.",
                v.map_or_else(|| "none".to_string(), |v| v.to_string())
            )),
        }
    }
}

pub fn read_saved_from(json: Option<&str>) -> Result<DockAreaState, SavedLayout> {
    let json = json.ok_or(SavedLayout::Missing)?;
    let state: DockAreaState = serde_json::from_str(json).map_err(|e| SavedLayout::Unreadable(e.to_string()))?;
    if state.version != Some(LAYOUT_VERSION) {
        return Err(SavedLayout::OtherVersion(state.version));
    }
    Ok(state)
}

pub fn read_saved() -> Result<DockAreaState, SavedLayout> {
    let path = layout_file().ok_or(SavedLayout::Missing)?;
    match std::fs::read_to_string(&path) {
        Ok(json) => read_saved_from(Some(&json)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(SavedLayout::Missing),
        Err(e) => Err(SavedLayout::Unreadable(e.to_string())),
    }
}

/// Write through a temporary file and a rename, so a crash mid-write leaves
/// the previous layout, never half of one.
pub fn write(state: &DockAreaState) -> std::io::Result<()> {
    let path = layout_file().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config directory"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(state).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)
}

/// Layout changes, for undo and redo. Entries are whole layout snapshots;
/// identical neighbours are never stored, so loading a snapshot (which reports
/// a change) does not add a step.
#[derive(Default)]
pub struct History {
    past: Vec<DockAreaState>,
    future: Vec<DockAreaState>,
    current: Option<DockAreaState>,
}

/// How many layout steps undo can go back.
pub const HISTORY_DEPTH: usize = 80;

impl History {
    /// Record the layout as it is now; a new edit forks history.
    pub fn record(&mut self, now: DockAreaState) {
        if self.current.as_ref() == Some(&now) {
            return;
        }
        if let Some(previous) = self.current.replace(now) {
            self.past.push(previous);
            if self.past.len() > HISTORY_DEPTH {
                self.past.remove(0);
            }
        }
        self.future.clear();
    }

    /// After loading a snapshot, the dock's own account of what it now holds
    /// replaces the snapshot, so a load that normalizes the layout does not
    /// read as one more edit.
    pub fn replace_current(&mut self, now: DockAreaState) {
        self.current = Some(now);
    }

    /// The layout to go back to, if any.
    pub fn undo(&mut self) -> Option<DockAreaState> {
        let previous = self.past.pop()?;
        if let Some(current) = self.current.replace(previous.clone()) {
            self.future.push(current);
        }
        Some(previous)
    }

    /// The layout to go forward to, if any.
    pub fn redo(&mut self) -> Option<DockAreaState> {
        let next = self.future.pop()?;
        if let Some(current) = self.current.replace(next.clone()) {
            self.past.push(current);
        }
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{History, LAYOUT_VERSION, SavedLayout, read_saved_from};
    use gpui_kit::base::dock::{DockAreaState, PanelInfo, PanelState};

    /// A layout built from the dock's own state types; `tab` tells them apart.
    fn state(version: usize, tab: usize) -> DockAreaState {
        DockAreaState {
            version: Some(version),
            center: PanelState {
                panel_name: "tabs".into(),
                children: Vec::new(),
                info: PanelInfo::Tabs { active_index: tab },
            },
            ..Default::default()
        }
    }

    #[test]
    fn a_saved_layout_of_another_version_is_set_aside_with_a_reason() {
        let json = serde_json::to_string(&state(LAYOUT_VERSION + 1, 1)).unwrap();
        let err = read_saved_from(Some(&json)).unwrap_err();
        assert_eq!(err, SavedLayout::OtherVersion(Some(LAYOUT_VERSION + 1)));
        assert!(err.explain().unwrap().contains("another version"));
    }

    #[test]
    fn a_damaged_layout_file_is_reported_not_loaded() {
        let err = read_saved_from(Some("{ not json")).unwrap_err();
        assert!(matches!(err, SavedLayout::Unreadable(_)), "{err:?}");
        assert_eq!(read_saved_from(None).unwrap_err().explain(), None, "no file is not worth a message");
    }

    #[test]
    fn undo_and_redo_walk_the_recorded_layouts() {
        let (a, b, c) = (state(LAYOUT_VERSION, 1), state(LAYOUT_VERSION, 2), state(LAYOUT_VERSION, 3));
        let mut h = History::default();
        h.record(a.clone());
        h.record(b.clone());
        h.record(b.clone()); // loading a snapshot reports a change; it is not a step
        h.record(c.clone());
        assert_eq!(h.undo(), Some(b.clone()));
        assert_eq!(h.undo(), Some(a.clone()));
        assert_eq!(h.undo(), None);
        assert_eq!(h.redo(), Some(b.clone()));
        h.record(a.clone()); // a new edit forks history
        assert_eq!(h.redo(), None);
        assert_eq!(h.undo(), Some(b));
    }
}
