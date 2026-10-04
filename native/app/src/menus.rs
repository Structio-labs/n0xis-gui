// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The application menu. Only actions that do something are listed: a menu
//! item that answers with nothing would be the old UI's toast-only entries again.

use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{GlobalState, input};
use gpui_kit::{App, Entity, Menu, MenuItem};

use crate::{About, Open, Quit};

/// Register the menus with the platform (the macOS menu bar) and with the
/// in-window menu bar that Linux and Windows show in the title bar.
pub fn init(cx: &mut App) -> Entity<AppMenuBar> {
    let bar = AppMenuBar::new(cx);
    cx.set_menus(build());
    GlobalState::global_mut(cx).set_app_menus(build().into_iter().map(|menu| menu.owned()).collect());
    bar.update(cx, |bar, cx| bar.reload(cx));
    bar
}

fn build() -> Vec<Menu> {
    vec![
        Menu {
            name: "File".into(),
            items: vec![MenuItem::action("Open…", Open), MenuItem::separator(), MenuItem::action("Quit", Quit)],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![MenuItem::action("Copy", input::Copy), MenuItem::action("Select All", input::SelectAll)],
            disabled: false,
        },
        Menu { name: "Help".into(), items: vec![MenuItem::action("About N0xis", About)], disabled: false },
    ]
}
