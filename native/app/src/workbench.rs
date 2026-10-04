// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The window's root view: the open target, its engine, and the views over it.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{
    ActiveTheme as _, IconName, TitleBar, WindowExt as _, h_flex, h_resizable, resizable_panel, v_flex, v_resizable,
};
use gpui_kit::*;
use n0xis_client::{Engine, EngineCommand, EngineStatus, FunctionEntry};

use crate::decompiler::DecompilerView;
use crate::disassembly::DisassemblyView;
use crate::functions::{FunctionList, FunctionSelected};
use crate::{About, Open, menus, project};

/// How often the status bar re-reads the engine's state.
const STATUS_POLL: Duration = Duration::from_millis(300);

struct Target {
    path: PathBuf,
    engine: Arc<Engine>,
    /// The engine binary this target is being analysed with, for About.
    engine_program: PathBuf,
}

pub struct Workbench {
    target: Option<Target>,
    /// Why the last attempt to open a target failed, shown until the next one.
    open_error: Option<String>,
    engine_status: Option<EngineStatus>,
    selected: Option<FunctionEntry>,
    functions: Entity<FunctionList>,
    decompiler: Entity<DecompilerView>,
    disassembly: Entity<DisassemblyView>,
    menu_bar: Entity<AppMenuBar>,
    focus_handle: FocusHandle,
    _status_poll: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl Workbench {
    pub fn new(target: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let functions = cx.new(|cx| FunctionList::new(window, cx));
        let decompiler = cx.new(|cx| DecompilerView::new(window, cx));
        let disassembly = cx.new(DisassemblyView::new);
        let selection = cx.subscribe_in(&functions, window, |this, _, FunctionSelected(function), window, cx| {
            this.selected = Some(function.clone());
            this.decompiler.update(cx, |view, cx| view.show(function.clone(), window, cx));
            this.disassembly.update(cx, |view, cx| view.show(function.clone(), cx));
            cx.notify();
        });
        let status_poll = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(STATUS_POLL).await;
                let alive = this.update(cx, |wb, cx| {
                    let status = wb.target.as_ref().map(|t| t.engine.status());
                    if status != wb.engine_status {
                        wb.engine_status = status;
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        let mut workbench = Self {
            target: None,
            open_error: None,
            engine_status: None,
            selected: None,
            functions,
            decompiler,
            disassembly,
            menu_bar: menus::init(cx),
            focus_handle: cx.focus_handle(),
            _status_poll: status_poll,
            _subscriptions: vec![selection],
        };
        if let Some(path) = target {
            workbench.open(path, window, cx);
        }
        workbench
    }

    /// Open `path`: start an engine session on it and point every view at it.
    fn open(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if !path.is_file() {
            self.open_error = Some(format!("{} is not a file", path.display()));
            cx.notify();
            return;
        }
        let command = match EngineCommand::locate() {
            Ok(c) => c,
            Err(e) => {
                self.open_error = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        // Without a project directory the engine would write the target's names
        // wherever this process was started, so a failure here is not fatal but
        // is said out loud.
        let project = match project::project_dir(&path) {
            Ok(dir) => Some(dir),
            Err(e) => {
                self.open_error = Some(format!("no project directory ({e}); names will not be kept"));
                None
            }
        };
        if project.is_some() {
            self.open_error = None;
        }
        let engine_program = command.program().to_path_buf();
        let engine = Arc::new(Engine::start(command, path.clone(), project));
        self.target = Some(Target { path: path.clone(), engine: Arc::clone(&engine), engine_program });
        self.engine_status = Some(engine.status());
        self.selected = None;
        self.decompiler.update(cx, |view, cx| view.set_engine(Some(Arc::clone(&engine)), cx));
        self.disassembly.update(cx, |view, cx| view.set_engine(Some(Arc::clone(&engine)), cx));
        self.functions.update(cx, |list, cx| list.load(engine, cx));
        let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        window.set_window_title(&format!("{name} — N0xis"));
        cx.notify();
    }

    fn prompt_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = answer.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update_in(cx, |wb, window, cx| wb.open(path, window, cx)).ok();
            }
        })
        .detach();
    }

    fn on_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_open(window, cx);
    }

    fn on_about(&mut self, _: &About, window: &mut Window, cx: &mut Context<Self>) {
        let engine = self
            .target
            .as_ref()
            .map_or_else(|| "none started yet".to_string(), |t| t.engine_program.display().to_string());
        window.open_dialog(cx, move |dialog, _, _| {
            dialog.title("About N0xis").child(
                v_flex()
                    .gap_2()
                    .text_sm()
                    .child(format!("N0xis GUI {}", env!("CARGO_PKG_VERSION")))
                    .child(format!("Engine: {engine}"))
                    .child("Built on GPUI Kit (Apache-2.0).")
                    .child("Source-available under the PolyForm Noncommercial License 1.0.0."),
            )
        });
    }

    /// One strip: the menus, then the open target. On Linux it also carries
    /// the window controls, since the window draws its own frame.
    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let target = self.target.as_ref().map(|t| t.path.display().to_string()).unwrap_or_default();
        TitleBar::new().child(
            h_flex()
                .gap_3()
                .child(self.menu_bar.clone())
                .child(div().text_xs().text_color(theme.muted_foreground).truncate().child(target)),
        )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (text, color) = match &self.engine_status {
            None => ("no target".to_string(), theme.muted_foreground),
            Some(EngineStatus::Starting) => ("engine starting…".to_string(), theme.muted_foreground),
            Some(EngineStatus::Ready { label, json_argv }) => {
                // An engine that reads only text requests cannot be sent every
                // value; say so rather than let a refused edit look like a bug.
                let note = if *json_argv { "" } else { " · older engine: some values cannot be sent" };
                (format!("engine ready · {label}{note}"), theme.success)
            }
            Some(EngineStatus::Crashed { crashes, reason }) => (
                format!("engine crashed ({crashes}×), restarting on the next request: {}", first_line(reason)),
                theme.warning,
            ),
            Some(EngineStatus::GaveUp { reason }) => (format!("engine stopped: {}", first_line(reason)), theme.danger),
            Some(EngineStatus::Stopped) => ("engine stopped".to_string(), theme.muted_foreground),
        };
        let selection = self.selected.as_ref().map(|f| format!("{} · {}", f.name, f.va)).unwrap_or_default();
        StatusBar::new()
            .left(div().text_xs().text_color(color).truncate().child(text))
            .right(div().text_xs().text_color(theme.muted_foreground).child(selection))
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_3()
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Open a binary to analyze"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("PE or ELF. Or start with: n0xis-ui <path>"),
            )
            .child(
                Button::new("open-empty")
                    .primary()
                    .icon(IconName::FolderOpen)
                    .label("Open…")
                    .on_click(cx.listener(|wb, _: &ClickEvent, window, cx| wb.prompt_open(window, cx))),
            )
            .children(self.open_error.clone().map(|e| div().text_sm().text_color(theme.danger).child(e)))
    }
}

fn first_line(s: &str) -> &str {
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or(s)
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (background, foreground) = (cx.theme().background, cx.theme().foreground);
        let main = if self.target.is_some() {
            h_resizable("workbench")
                .child(
                    resizable_panel()
                        .size(px(340.))
                        .size_range(px(220.)..px(640.))
                        .child(self.functions.clone()),
                )
                .child(
                    v_resizable("code")
                        .child(resizable_panel().child(self.decompiler.clone()))
                        .child(resizable_panel().size(px(280.)).child(self.disassembly.clone())),
                )
                .into_any_element()
        } else {
            self.render_empty(cx).into_any_element()
        };
        v_flex()
            .size_full()
            .key_context("Workbench")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_about))
            .bg(background)
            .text_color(foreground)
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().flex().child(main))
            .child(self.render_status_bar(cx))
    }
}

impl Focusable for Workbench {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
