// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The dock: which panels exist, how they are arranged by default, and where
//! the user's own arrangement is kept between runs.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::base::dock::{PanelId, PanelView};
use gpui_kit::component::dock::{DockAreaState, DockLayout, panel_handle, register_panel};
use gpui_kit::*;

use crate::assets::AppIcon;
use crate::bookmarks::BookmarksView;
use crate::console::ConsoleView;
use crate::decompiler::DecompilerView;
use crate::disassembly::DisassemblyView;
use crate::functions::FunctionList;
use crate::graph::GraphView;
use crate::linear::LinearView;
use crate::scanner::ScannerView;
use crate::search::SearchView;
use crate::triage::TriageView;
use crate::types::TypesView;
use crate::xrefs::XrefsView;

/// Every panel the dock can hold. The one place a panel's saved name and title
/// are written down; menus, the registry and the default layout all read them
/// from here. None is special: any of them can be closed and opened again,
/// and its view, with what it holds, outlives its tab (see [`Views`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    Functions,
    Decompiler,
    Disassembly,
    Graph,
    Linear,
    Xrefs,
    Triage,
    Bookmarks,
    Types,
    Find,
    Console,
    Scanner,
}

impl PanelKind {
    pub const ALL: [PanelKind; 12] = [
        Self::Functions,
        Self::Decompiler,
        Self::Disassembly,
        Self::Graph,
        Self::Linear,
        Self::Xrefs,
        Self::Triage,
        Self::Bookmarks,
        Self::Types,
        Self::Find,
        Self::Console,
        Self::Scanner,
    ];

    /// The name a saved layout knows the panel by. Once chosen, never change
    /// it: a renamed panel is one the saved layout no longer finds.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Functions => "functions",
            Self::Decompiler => "decompiler",
            Self::Disassembly => "disassembly",
            Self::Graph => "graph",
            Self::Linear => "linear",
            Self::Xrefs => "xrefs",
            Self::Triage => "triage",
            Self::Bookmarks => "bookmarks",
            Self::Types => "types",
            Self::Find => "find",
            Self::Console => "console",
            Self::Scanner => "scanner",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Functions => "Functions",
            Self::Decompiler => "Decompiler",
            Self::Disassembly => "Disassembly",
            Self::Graph => "Graph",
            Self::Linear => "Linear",
            Self::Xrefs => "Cross-references",
            Self::Triage => "Triage",
            Self::Bookmarks => "Bookmarks",
            Self::Types => "Types",
            Self::Find => "Find",
            Self::Console => "Console",
            Self::Scanner => "Memory scanner",
        }
    }

    /// The icon the Tauri build gave this widget, from the same set.
    pub const fn icon(self) -> AppIcon {
        match self {
            Self::Functions => AppIcon::Strings,
            Self::Decompiler => AppIcon::Decomp,
            Self::Disassembly => AppIcon::Disasm,
            Self::Graph => AppIcon::Graph,
            Self::Linear => AppIcon::Disasm,
            Self::Xrefs => AppIcon::Xref,
            Self::Triage => AppIcon::Chip,
            Self::Bookmarks => AppIcon::Bookmark,
            Self::Types => AppIcon::Type,
            Self::Find => AppIcon::Search,
            Self::Console => AppIcon::Term,
            Self::Scanner => AppIcon::Scan,
        }
    }

    /// The panel whose tab group this one joins when it is opened again; `None`
    /// opens it as a column of its own on the left.
    pub const fn companion(self) -> Option<PanelKind> {
        match self {
            Self::Functions => None,
            Self::Decompiler | Self::Graph | Self::Linear => Some(Self::Decompiler),
            Self::Xrefs | Self::Bookmarks | Self::Find => Some(Self::Xrefs),
            Self::Disassembly | Self::Console | Self::Triage | Self::Types | Self::Scanner => Some(Self::Disassembly),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// Show a panel: select its tab where the dock holds it, add it where it does not.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct ShowPanel(pub SharedString);

/// The dock area's id, and the version of the default arrangement. Bump the
/// version when the default changes in a way an old saved layout should not
/// survive; a saved layout of another version is set aside, and the user told.
pub const AREA_ID: &str = "n0xis-main";
pub const LAYOUT_VERSION: usize = 2;

/// The one instance of each view. A restored layout names panels; the builders
/// registered below return these instances rather than new ones, so what a
/// view holds (the function list, the open decompilation) survives a reload.
#[derive(Clone)]
pub struct Views {
    pub functions: Entity<FunctionList>,
    pub decompiler: Entity<DecompilerView>,
    pub disassembly: Entity<DisassemblyView>,
    pub graph: Entity<GraphView>,
    pub linear: Entity<LinearView>,
    pub xrefs: Entity<XrefsView>,
    pub triage: Entity<TriageView>,
    pub bookmarks: Entity<BookmarksView>,
    pub types: Entity<TypesView>,
    pub find: Entity<SearchView>,
    pub console: Entity<ConsoleView>,
    pub scanner: Entity<ScannerView>,
}

impl Global for Views {}

impl Views {
    pub fn handle(&self, kind: PanelKind) -> Arc<dyn PanelView> {
        match kind {
            PanelKind::Functions => panel_handle(self.functions.clone()),
            PanelKind::Decompiler => panel_handle(self.decompiler.clone()),
            PanelKind::Disassembly => panel_handle(self.disassembly.clone()),
            PanelKind::Graph => panel_handle(self.graph.clone()),
            PanelKind::Linear => panel_handle(self.linear.clone()),
            PanelKind::Xrefs => panel_handle(self.xrefs.clone()),
            PanelKind::Triage => panel_handle(self.triage.clone()),
            PanelKind::Bookmarks => panel_handle(self.bookmarks.clone()),
            PanelKind::Types => panel_handle(self.types.clone()),
            PanelKind::Find => panel_handle(self.find.clone()),
            PanelKind::Console => panel_handle(self.console.clone()),
            PanelKind::Scanner => panel_handle(self.scanner.clone()),
        }
    }

    /// The id the dock knows a panel by.
    pub fn panel_id(&self, kind: PanelKind) -> PanelId {
        let entity = match kind {
            PanelKind::Functions => self.functions.entity_id(),
            PanelKind::Decompiler => self.decompiler.entity_id(),
            PanelKind::Disassembly => self.disassembly.entity_id(),
            PanelKind::Graph => self.graph.entity_id(),
            PanelKind::Linear => self.linear.entity_id(),
            PanelKind::Xrefs => self.xrefs.entity_id(),
            PanelKind::Triage => self.triage.entity_id(),
            PanelKind::Bookmarks => self.bookmarks.entity_id(),
            PanelKind::Types => self.types.entity_id(),
            PanelKind::Find => self.find.entity_id(),
            PanelKind::Console => self.console.entity_id(),
            PanelKind::Scanner => self.scanner.entity_id(),
        };
        PanelId::from(entity)
    }
}

/// Tell the dock how to rebuild each panel a saved layout names. The builders
/// read [`Views`], which the workbench sets before any layout is loaded.
pub fn register_panels(cx: &mut App) {
    for kind in PanelKind::ALL {
        register_panel(cx, kind.name(), move |_, _, cx| cx.global::<Views>().handle(kind));
    }
}

fn tabs(views: &Views, kinds: &[PanelKind], cx: &App) -> DockLayout {
    kinds.iter().fold(DockLayout::tabs(), |tabs, &kind| tabs.panel_view(views.handle(kind), cx))
}

/// Functions on the left; the code views in the middle over the disassembly
/// and the console; references, bookmarks and search on the right.
pub fn default_layout(views: &Views, cx: &App) -> DockLayout {
    use PanelKind::*;
    DockLayout::h_split()
        .child(tabs(views, &[Functions], cx), Some(px(300.)))
        .child(
            DockLayout::v_split()
                .child(
                    DockLayout::h_split()
                        .child(tabs(views, &[Decompiler, Graph, Linear], cx), None)
                        .child(tabs(views, &[Xrefs, Bookmarks, Find], cx), Some(px(430.))),
                    None,
                )
                .child(tabs(views, &[Disassembly, Console, Triage, Types], cx), Some(px(280.))),
            None,
        )
}

/// `ui-layout.json` in the GUI's config directory.
pub fn layout_file() -> Option<PathBuf> {
    crate::config::dir().map(|dir| dir.join("ui-layout.json"))
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

pub fn write(state: &DockAreaState) -> std::io::Result<()> {
    let path = layout_file().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config directory"))?;
    let json = serde_json::to_string_pretty(state).map_err(std::io::Error::other)?;
    crate::config::write_atomic(&path, &json)
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
    use super::{History, LAYOUT_VERSION, PanelKind, SavedLayout, read_saved_from};
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
    fn every_panel_has_its_own_saved_name_and_the_first_three_keep_theirs() {
        let names: Vec<&str> = PanelKind::ALL.iter().map(|k| k.name()).collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{names:?}");
        // Layouts saved before the other panels existed name these three.
        assert_eq!(&names[..3], ["functions", "decompiler", "disassembly"]);
        for kind in PanelKind::ALL {
            assert_eq!(PanelKind::from_name(kind.name()), Some(kind));
        }
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
