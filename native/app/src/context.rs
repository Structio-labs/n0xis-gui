// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! What a right click offers. A menu names the address it was opened on, so
//! its items act there whatever is selected; the workbench does the work, the
//! same way the Edit menu does it for the selection. Only items that do
//! something are listed.

use gpui_kit::component::Icon;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::*;

use crate::assets::AppIcon;
use crate::layout::PanelKind;
use crate::nav::hex;

/// A change to what is recorded at an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum EditKind {
    /// Rename the function the address lies in, or name the address.
    Rename,
    Comment,
    /// The return type of the function the address lies in.
    ReturnType,
    /// Bookmark the address, or take its bookmark off.
    Bookmark,
    /// Take off everything recorded at the address.
    Clear,
}

/// Go to `va`, and show it in `panel` when one is named.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct ShowAt {
    pub va: u64,
    pub panel: Option<SharedString>,
}

/// Change what is recorded at `va`, as the Edit menu does at the selection.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct EditAt {
    pub va: u64,
    pub edit: EditKind,
}

/// Name the known constants in the function at `va`.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct RecognizeAt {
    pub va: u64,
}

/// Put `text` on the clipboard.
pub fn copy(menu: PopupMenu, label: impl Into<SharedString>, text: impl Into<String>) -> PopupMenu {
    let text = text.into();
    menu.item(
        PopupMenuItem::new(label)
            .icon(Icon::new(AppIcon::Copy))
            .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(text.clone()))),
    )
}

/// Go to `va`.
pub fn go(menu: PopupMenu, label: impl Into<SharedString>, va: u64) -> PopupMenu {
    menu.menu_with_icon(label, Icon::new(AppIcon::Follow), Box::new(ShowAt { va, panel: None }))
}

/// Go to `va` and show it in `panel`.
pub fn show_in(menu: PopupMenu, label: impl Into<SharedString>, va: u64, panel: PanelKind) -> PopupMenu {
    menu.menu_with_icon(label, Icon::new(panel.icon()), Box::new(ShowAt { va, panel: Some(panel.name().into()) }))
}

/// One change to what is recorded at `va`.
pub fn edit(menu: PopupMenu, label: impl Into<SharedString>, va: u64, kind: EditKind) -> PopupMenu {
    let icon = match kind {
        EditKind::Rename => AppIcon::Rename,
        EditKind::Comment => AppIcon::Comment,
        EditKind::ReturnType => AppIcon::Type,
        EditKind::Bookmark => AppIcon::Bookmark,
        EditKind::Clear => AppIcon::Trash,
    };
    menu.menu_with_icon(label, Icon::new(icon), Box::new(EditAt { va, edit: kind }))
}

/// The views an address can be looked at in besides the one the menu is in.
pub fn views(menu: PopupMenu, va: u64, except: Option<PanelKind>) -> PopupMenu {
    [
        ("Show in Decompiler", PanelKind::Decompiler),
        ("Show in Disassembly", PanelKind::Disassembly),
        ("Show in Graph", PanelKind::Graph),
        ("Show in Linear Listing", PanelKind::Linear),
        ("Show Bytes in Hex", PanelKind::Hex),
        ("Show References", PanelKind::Xrefs),
    ]
    .into_iter()
    .filter(|(_, panel)| Some(*panel) != except)
    .fold(menu, |menu, (label, panel)| show_in(menu, label, va, panel))
}

/// What can be recorded at `va`: a name, a comment, a bookmark, or nothing.
pub fn annotations(menu: PopupMenu, va: u64) -> PopupMenu {
    let menu = edit(menu, "Rename…", va, EditKind::Rename);
    let menu = edit(menu, "Comment…", va, EditKind::Comment);
    let menu = edit(menu, "Toggle Bookmark", va, EditKind::Bookmark);
    edit(menu, "Clear Annotations Here", va, EditKind::Clear)
}

/// The usual menu of an address shown in a list: go there, look at it
/// elsewhere, record something there, copy it.
pub fn address(menu: PopupMenu, title: impl Into<SharedString>, va: u64, here: Option<PanelKind>) -> PopupMenu {
    let menu = go(menu.label(title), "Go To", va);
    let menu = views(menu, va, here).separator();
    let menu = annotations(menu, va).separator();
    copy(menu, "Copy Address", hex(va))
}

/// A function's menu: look at it in each view, record something on it, copy it.
pub fn function(menu: PopupMenu, name: &str, va: u64, here: Option<PanelKind>) -> PopupMenu {
    let menu = views(menu.label(name.to_string()), va, here).separator();
    let menu = edit(menu, "Rename…", va, EditKind::Rename);
    let menu = edit(menu, "Set Return Type…", va, EditKind::ReturnType);
    let menu = edit(menu, "Comment…", va, EditKind::Comment);
    let menu = edit(menu, "Toggle Bookmark", va, EditKind::Bookmark);
    let menu = edit(menu, "Clear Annotations Here", va, EditKind::Clear).separator();
    let menu = menu.menu_with_icon("Identify Constants", Icon::new(AppIcon::Identify), Box::new(RecognizeAt { va })).separator();
    let menu = copy(menu, "Copy Name", name.to_string());
    copy(menu, "Copy Address", hex(va))
}

/// A row that opens a menu on a right click. The menu is drawn inside the
/// element that opens it and takes on its text style, so a row set in the
/// code font is wrapped rather than given the menu itself.
pub fn row_with_menu(id: impl Into<ElementId>, row: impl IntoElement) -> Stateful<Div> {
    div().id(id).w_full().child(row)
}
