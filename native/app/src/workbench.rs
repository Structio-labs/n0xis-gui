// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The window's root view: the open target, its engine, and the views over it.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::dock::{DockArea, DockAreaState, DockEvent, DockSkin};
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{ActiveTheme as _, IconName, TitleBar, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{Engine, EngineCommand, EngineStatus, FunctionEntry};

use crate::decompiler::DecompilerView;
use crate::disassembly::DisassemblyView;
use crate::functions::{FunctionList, FunctionSelected};
use crate::appearance::{self, ResetZoom, SelectTheme, ZoomIn, ZoomOut};
use crate::layout::{self, History, Views};
use crate::{About, Open, RedoLayout, ResetLayout, UndoLayout, menus, project};

/// How often the status bar re-reads the engine's state.
const STATUS_POLL: Duration = Duration::from_millis(300);

/// A drag reports a layout change on every step. Wait this long for it to
/// settle, then record one undo step and save once.
const LAYOUT_SETTLE: Duration = Duration::from_millis(400);

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
    dock_area: Entity<DockArea>,
    _dock_skin: Rc<DockSkin>,
    history: History,
    /// Set once a save of the layout has failed, so the user is told once.
    layout_save_failed: bool,
    /// The next settled layout comes from loading a snapshot (start-up, undo,
    /// redo), not from the user: it replaces the current history entry instead
    /// of adding one. A loaded layout is laid out again on the next frame and
    /// its sizes shift, so comparing snapshots cannot tell the two apart.
    next_change_is_load: bool,
    _layout_settle: Option<Task<()>>,
    _theme_poll: Task<()>,
    _status_poll: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl Workbench {
    pub fn new(target: Option<PathBuf>, theme_problems: Vec<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let functions = cx.new(|cx| FunctionList::new(window, cx));
        let decompiler = cx.new(|cx| DecompilerView::new(window, cx));
        let disassembly = cx.new(DisassemblyView::new);
        let views = Views { functions: functions.clone(), decompiler: decompiler.clone(), disassembly: disassembly.clone() };
        cx.set_global(views.clone());
        let (dock_area, dock_skin) = DockSkin::dock_area(layout::AREA_ID, Some(layout::LAYOUT_VERSION), window, cx);
        let restore_note = match layout::read_saved() {
            Ok(state) => match dock_area.update(cx, |area, cx| area.load(state, window, cx)) {
                Ok(()) => None,
                Err(e) => Some(format!("The saved panel layout could not be restored ({e}); using the default.")),
            },
            Err(reason) => reason.explain().or(Some(String::new())),
        };
        // Anything but a clean restore falls back to the default; only a real
        // problem (not a first run) is worth a message.
        if let Some(note) = restore_note {
            dock_area.update(cx, |area, cx| area.set_center(layout::default_layout(&views, cx), window, cx));
            if !note.is_empty() {
                window.defer(cx, move |window, cx| window.push_notification(SharedString::from(note), cx));
            }
        }
        let history = History::default();
        let layout_changes = cx.subscribe_in(&dock_area, window, |this, _, event: &DockEvent, window, cx| {
            if matches!(event, DockEvent::LayoutChanged) {
                this.layout_changed(window, cx);
            }
        });
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
        // Watch the user's theme folder: an edited theme shows at once, a broken
        // file is reported instead of silently ignored.
        let theme_poll = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(appearance::THEME_POLL).await;
                let alive = this.update_in(cx, |wb, window, cx| {
                    if let Some(problems) = appearance::poll_user_themes(cx) {
                        wb.refresh_menus(cx);
                        for problem in problems {
                            window.push_notification(SharedString::from(problem), cx);
                        }
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        if !theme_problems.is_empty() {
            window.defer(cx, move |window, cx| {
                for problem in theme_problems {
                    window.push_notification(SharedString::from(problem), cx);
                }
            });
        }
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
            dock_area,
            _dock_skin: dock_skin,
            history,
            layout_save_failed: false,
            next_change_is_load: true,
            _layout_settle: None,
            _theme_poll: theme_poll,
            _status_poll: status_poll,
            _subscriptions: vec![selection, layout_changes],
        };
        // The first entry of the layout history is the layout as first laid
        // out, taken once it has settled.
        workbench.layout_changed(window, cx);
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

    fn layout_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._layout_settle = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(LAYOUT_SETTLE).await;
            this.update_in(cx, |wb, window, cx| {
                let state = wb.dock_area.read(cx).dump(cx);
                if std::mem::take(&mut wb.next_change_is_load) {
                    wb.history.replace_current(state.clone());
                } else {
                    wb.history.record(state.clone());
                }
                if let Err(e) = layout::write(&state)
                    && !wb.layout_save_failed
                {
                    wb.layout_save_failed = true;
                    window.push_notification(SharedString::from(format!("The panel layout could not be saved: {e}")), cx);
                }
            })
            .ok();
        }));
    }

    fn apply_layout(&mut self, state: DockAreaState, window: &mut Window, cx: &mut Context<Self>) {
        match self.dock_area.update(cx, |area, cx| area.load(state, window, cx)) {
            Ok(()) => self.next_change_is_load = true,
            Err(e) => window.push_notification(SharedString::from(format!("That layout could not be restored: {e}")), cx),
        }
    }

    fn on_undo_layout(&mut self, _: &UndoLayout, window: &mut Window, cx: &mut Context<Self>) {
        match self.history.undo() {
            Some(state) => self.apply_layout(state, window, cx),
            None => window.push_notification("Nothing to undo in the layout.", cx),
        }
    }

    fn on_redo_layout(&mut self, _: &RedoLayout, window: &mut Window, cx: &mut Context<Self>) {
        match self.history.redo() {
            Some(state) => self.apply_layout(state, window, cx),
            None => window.push_notification("Nothing to redo in the layout.", cx),
        }
    }

    fn on_reset_layout(&mut self, _: &ResetLayout, window: &mut Window, cx: &mut Context<Self>) {
        let views = cx.global::<Views>().clone();
        self.dock_area.update(cx, |area, cx| area.set_center(layout::default_layout(&views, cx), window, cx));
    }

    fn refresh_menus(&self, cx: &mut Context<Self>) {
        let bar = self.menu_bar.clone();
        menus::refresh(&bar, cx);
    }

    /// Keep an appearance change for the next run, and say so if that fails.
    fn keep_appearance(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = appearance::save(cx) {
            window.push_notification(SharedString::from(format!("The appearance could not be saved: {e}")), cx);
        }
    }

    fn on_select_theme(&mut self, action: &SelectTheme, window: &mut Window, cx: &mut Context<Self>) {
        if appearance::apply_theme(&action.0, cx) {
            self.keep_appearance(window, cx);
            self.refresh_menus(cx);
        } else {
            window.push_notification(SharedString::from(format!("There is no theme called {}.", action.0)), cx);
        }
    }

    fn on_zoom_in(&mut self, _: &ZoomIn, window: &mut Window, cx: &mut Context<Self>) {
        appearance::zoom(1., cx);
        self.keep_appearance(window, cx);
    }

    fn on_zoom_out(&mut self, _: &ZoomOut, window: &mut Window, cx: &mut Context<Self>) {
        appearance::zoom(-1., cx);
        self.keep_appearance(window, cx);
    }

    fn on_reset_zoom(&mut self, _: &ResetZoom, window: &mut Window, cx: &mut Context<Self>) {
        appearance::reset_zoom(cx);
        self.keep_appearance(window, cx);
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
            self.dock_area.clone().into_any_element()
        } else {
            self.render_empty(cx).into_any_element()
        };
        v_flex()
            .size_full()
            .key_context("Workbench")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_about))
            .on_action(cx.listener(Self::on_undo_layout))
            .on_action(cx.listener(Self::on_redo_layout))
            .on_action(cx.listener(Self::on_reset_layout))
            .on_action(cx.listener(Self::on_select_theme))
            .on_action(cx.listener(Self::on_zoom_in))
            .on_action(cx.listener(Self::on_zoom_out))
            .on_action(cx.listener(Self::on_reset_zoom))
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
