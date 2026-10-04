// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! N0xis desktop GUI on GPUI. What it has to reach is listed in
//! `docs/CURRENT-UI.md`; the engine always runs as a separate process.

mod decompiler;
mod disassembly;
mod functions;
mod project;
mod workbench;

use std::path::PathBuf;

use gpui_kit::component::{Theme, ThemeMode, TitleBar};
use gpui_kit::*;

gpui_kit::actions!(n0xis, [Quit]);

fn main() {
    // `n0xis-ui <binary>` opens it straight away, from a terminal or a file manager.
    let target = std::env::args_os().nth(1).map(PathBuf::from);
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
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
            ..TitleBar::window_options()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("N0xis");
            cx.new(|cx| workbench::Workbench::new(target, window, cx))
        })
        .expect("open the main window");
        cx.activate(true);
    });
}
