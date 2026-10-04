// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! N0xis desktop GUI on GPUI. What it has to reach is listed in
//! `docs/CURRENT-UI.md`; the engine always runs as a separate process.

mod appearance;
mod assets;
mod bookmarks;
mod config;
mod console;
mod decompiler;
mod disassembly;
mod functions;
mod graph;
mod graph_layout;
mod layout;
mod linear;
mod menus;
mod nav;
mod panel;
mod project;
mod scanner;
mod search;
mod triage;
mod types;
mod workbench;
mod xrefs;

use std::path::PathBuf;

use gpui_kit::component::dock::ToggleZoom;
use gpui_kit::component::{Theme, ThemeMode, TitleBar};
use gpui_kit::*;

gpui_kit::actions!(n0xis, [Quit, Open, About, ResetLayout, UndoLayout, RedoLayout, ToggleBookmark]);

fn main() {
    // `n0xis-ui <binary>` opens it straight away, from a terminal or a file manager.
    let target = std::env::args_os().nth(1).map(PathBuf::from);
    gpui_kit::application().with_assets(assets::Assets).run(move |cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        layout::register_panels(cx);
        cx.bind_keys([
            KeyBinding::new("ctrl-q", Quit, None),
            KeyBinding::new("ctrl-o", Open, None),
            // Text fields bind these to text undo and redo; inside one, theirs win.
            KeyBinding::new("ctrl-shift-z", UndoLayout, None),
            KeyBinding::new("ctrl-shift-y", RedoLayout, None),
            KeyBinding::new("shift-escape", ToggleZoom, None),
            KeyBinding::new("ctrl-=", appearance::ZoomIn, None),
            KeyBinding::new("ctrl-+", appearance::ZoomIn, None),
            KeyBinding::new("ctrl--", appearance::ZoomOut, None),
            KeyBinding::new("ctrl-0", appearance::ResetZoom, None),
            KeyBinding::new("ctrl-d", ToggleBookmark, None),
            // In the console's command line, ↑ and ↓ walk its history instead
            // of moving the caret.
            KeyBinding::new("up", console::HistoryPrevious, Some("Console > Input")),
            KeyBinding::new("down", console::HistoryNext, Some("Console > Input")),
            // The listing copies its selected lines.
            KeyBinding::new("ctrl-c", gpui_kit::component::input::Copy, Some(linear::KEY_CONTEXT)),
        ]);
        let theme_problems = appearance::init(cx);
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
        // One window: closing it ends the app instead of leaving a headless process.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(900.), px(600.))),
            // On Linux the window draws its own frame, so the title bar below is
            // the only one: menus, target and window controls in one strip. On
            // Windows and macOS the title bar options already hide the system one.
            #[cfg(target_os = "linux")]
            window_decorations: Some(WindowDecorations::Client),
            #[cfg(target_os = "linux")]
            window_background: WindowBackgroundAppearance::Transparent,
            ..TitleBar::window_options()
        };
        let (window, workbench) = gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("N0xis");
            cx.new(|cx| workbench::Workbench::new(target, theme_problems, window, cx))
        })
        .expect("open the main window");
        // Keyboard shortcuts and menu actions reach the workbench through focus.
        let focus = workbench.read(cx).focus_handle(cx);
        window
            .update(cx, |_, window, cx| {
                window.activate_window();
                focus.focus(window, cx);
            })
            .ok();
        cx.activate(true);
    });
}
