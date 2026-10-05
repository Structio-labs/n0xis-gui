// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The application menu. Only actions that do something are listed: a menu
//! item that answers with nothing would be the old UI's toast-only entries again.

use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{GlobalState, input};
use gpui_kit::{App, Entity, Menu, MenuItem};

use crate::appearance::{self, ResetZoom, SelectTheme, ToggleRoundedEdges, ZoomIn, ZoomOut};
use crate::layout::{PanelKind, ShowPanel, SwitchWorkspace, Workspace};
use crate::recent::{OpenRecent, Recent};
use crate::{
    About, ClearAnnotations, CloseTarget, CommandPalette, Comment, FindInImage, GoBack, GoForward, GoTo, KeyboardShortcuts,
    Open, OpenSettings, Quit, RecognizeConstants, RedoEdit, RedoLayout, Rename, ResetLayout, RunAnalysis, ScanProcess,
    SetReturnType, SetType, ToggleBookmark, UndoEdit, UndoLayout,
};

/// Create the in-window menu bar (Linux and Windows show it in the title bar)
/// and register the menus with the platform (the macOS menu bar).
pub fn init(cx: &mut App) -> Entity<AppMenuBar> {
    let bar = AppMenuBar::new(cx);
    refresh(&bar, cx);
    bar
}

/// Rebuild the menus: the theme list and its check mark follow what is loaded
/// and in use.
pub fn refresh(bar: &Entity<AppMenuBar>, cx: &mut App) {
    cx.set_menus(build(cx));
    let owned = build(cx).into_iter().map(|menu| menu.owned()).collect();
    GlobalState::global_mut(cx).set_app_menus(owned);
    bar.update(cx, |bar, cx| bar.reload(cx));
}

fn build(cx: &App) -> Vec<Menu> {
    let current = appearance::current_theme(cx);
    let rounded = cx.try_global::<appearance::GraphLook>().is_some_and(|look| look.rounded_edges);
    let themes = cx
        .global::<appearance::Themes>()
        .list()
        .iter()
        .map(|t| MenuItem::action(t.name.clone(), SelectTheme(t.name.clone())).checked(t.name == current))
        .collect();
    let recent: Vec<MenuItem> = cx
        .global::<Recent>()
        .targets()
        .iter()
        .map(|t| MenuItem::action(t.label(), OpenRecent(t.path.to_string_lossy().into_owned().into())))
        .collect();
    let workspaces = Workspace::ALL.into_iter().map(|w| MenuItem::action(w.title(), SwitchWorkspace(w.name().into()))).collect();
    let panels = PanelKind::ALL
        .into_iter()
        .map(|kind| MenuItem::action(kind.title(), ShowPanel(kind.name().into())))
        .collect();
    vec![
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open…", Open),
                MenuItem::Submenu(Menu { name: "Open Recent".into(), disabled: recent.is_empty(), items: recent }),
                MenuItem::action("Close Target", CloseTarget),
                MenuItem::separator(),
                MenuItem::action("Scan a Running Process…", ScanProcess),
                MenuItem::separator(),
                MenuItem::action("Settings…", OpenSettings),
                MenuItem::action("Keyboard Shortcuts…", KeyboardShortcuts),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo Edit", UndoEdit),
                MenuItem::action("Redo Edit", RedoEdit),
                MenuItem::separator(),
                MenuItem::action("Rename…", Rename),
                MenuItem::action("Comment…", Comment),
                MenuItem::action("Set Type…", SetType),
                MenuItem::action("Set Return Type…", SetReturnType),
                MenuItem::action("Toggle Bookmark", ToggleBookmark),
                MenuItem::action("Clear Annotations Here", ClearAnnotations),
                MenuItem::separator(),
                MenuItem::action("Copy", input::Copy),
                MenuItem::action("Select All", input::SelectAll),
            ],
            disabled: false,
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Command Palette…", CommandPalette),
                MenuItem::separator(),
                MenuItem::Submenu(Menu { name: "Panels".into(), items: panels, disabled: false }),
                MenuItem::Submenu(Menu { name: "Theme".into(), items: themes, disabled: false }),
                MenuItem::action("Round Graph Edge Corners", ToggleRoundedEdges).checked(rounded),
                MenuItem::separator(),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Reset Zoom", ResetZoom),
            ],
            disabled: false,
        },
        Menu {
            name: "Analyze".into(),
            items: vec![MenuItem::action("Run Analysis", RunAnalysis), MenuItem::action("Identify Constants in This Function", RecognizeConstants)],
            disabled: false,
        },
        Menu {
            name: "Go".into(),
            items: vec![
                MenuItem::action("Go to Address or Name…", GoTo),
                MenuItem::action("Find in Image…", FindInImage),
                MenuItem::separator(),
                MenuItem::action("Back", GoBack),
                MenuItem::action("Forward", GoForward),
            ],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::Submenu(Menu { name: "Workspace".into(), items: workspaces, disabled: false }),
                MenuItem::separator(),
                MenuItem::action("Undo Layout Change", UndoLayout),
                MenuItem::action("Redo Layout Change", RedoLayout),
                MenuItem::separator(),
                MenuItem::action("Reset Layout", ResetLayout),
            ],
            disabled: false,
        },
        Menu {
            name: "Help".into(),
            items: vec![MenuItem::action("Keyboard Shortcuts…", KeyboardShortcuts), MenuItem::separator(), MenuItem::action("About N0xis", About)],
            disabled: false,
        },
    ]
}
