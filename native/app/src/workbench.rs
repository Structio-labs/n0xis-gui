// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The window's root view: the open target, its engine, and the views over it.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::base::Placement;
use gpui_kit::base::dock::{DockPlacement, InsertTarget};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::dock::{DockArea, DockAreaState, DockEvent, DockSkin};
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::{ActiveTheme as _, TitleBar, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{Analysis, AnalysisState, Engine, EngineCommand, EngineStatus, ShowAnnotations, WarmUp};

use crate::appearance::{self, ResetZoom, SelectTheme, ZoomIn, ZoomOut};
use crate::bookmarks::BookmarksView;
use crate::console::ConsoleView;
use crate::decompiler::{DecompilerView, EditVariable, VariableIntent};
use crate::disassembly::DisassemblyView;
use crate::edits::{self, Change, Edit, Fact, Journal};
use crate::functions::{FunctionChanged, FunctionList, FunctionSelected, FunctionsLoaded};
use crate::graph::GraphView;
use crate::linear::LinearView;
use crate::assets::AppIcon;
use crate::layout::{self, History, PanelKind, ShowPanel, Views};
use crate::nav::{Location, NavHistory, Navigate, Point, hex, parse_va};
use crate::prompt::{self, Prompt};
use crate::recent::{self, OpenRecent, Recent};
use crate::scanner::ScannerView;
use crate::search::SearchView;
use crate::triage::TriageView;
use crate::types::TypesView;
use crate::xrefs::XrefsView;
use crate::prefs::{Prefs, ProjectLocation};
use crate::{
    About, ClearAnnotations, Comment, GoBack, GoForward, GoTo, Open, OpenSettings, RedoEdit, RedoLayout, Rename,
    ResetLayout, RunAnalysis, SetReturnType, SetType, ToggleBookmark, UndoEdit, UndoLayout, menus, project, settings,
};

/// How often the status bar re-reads the engine's state.
const STATUS_POLL: Duration = Duration::from_millis(300);

/// A drag reports a layout change on every step. Wait this long for it to
/// settle, then record one undo step and save once.
const LAYOUT_SETTLE: Duration = Duration::from_millis(400);

struct Target {
    path: PathBuf,
    engine: Arc<Engine>,
    /// How the engine is run, for the analysis and for clearing caches.
    command: EngineCommand,
    /// Where the target's project is; `None` when none could be made.
    project: Option<PathBuf>,
    /// The whole-program pass over the target, beside the session, when one runs.
    analysis: Option<Analysis>,
    /// The engine binary this target is being analysed with, for About.
    engine_program: PathBuf,
}

pub struct Workbench {
    target: Option<Target>,
    /// Why the last attempt to open a target failed, shown until the next one.
    open_error: Option<String>,
    engine_status: Option<EngineStatus>,
    /// The analysis as last read, to notice when it moves on.
    analysis_state: Option<AnalysisState>,
    /// The one selection every view follows.
    location: Option<Location>,
    /// Where the user has been, for Back and Forward.
    nav: NavHistory,
    /// The user's edits to the target, for Undo and Redo.
    journal: Journal,
    /// An edit, undo or redo is being written; the next waits for it.
    edit_busy: bool,
    /// The dock is on screen without a target: a panel was asked for by name.
    dock_shown: bool,
    views: Views,
    menu_bar: Entity<AppMenuBar>,
    focus_handle: FocusHandle,
    dock_area: Entity<DockArea>,
    _dock_skin: Rc<DockSkin>,
    history: History,
    /// Set once a save of the layout has failed, so the user is told once.
    layout_save_failed: bool,
    /// The same for the list of recent targets.
    recent_save_failed: bool,
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
    pub fn new(target: Option<PathBuf>, startup_notes: Vec<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let views = Views {
            functions: cx.new(|cx| FunctionList::new(window, cx)),
            decompiler: cx.new(|cx| DecompilerView::new(window, cx)),
            disassembly: cx.new(DisassemblyView::new),
            graph: cx.new(GraphView::new),
            linear: cx.new(LinearView::new),
            xrefs: cx.new(XrefsView::new),
            triage: cx.new(|cx| TriageView::new(window, cx)),
            bookmarks: cx.new(BookmarksView::new),
            types: cx.new(|cx| TypesView::new(window, cx)),
            find: cx.new(|cx| SearchView::new(window, cx)),
            console: cx.new(|cx| ConsoleView::new(window, cx)),
            scanner: cx.new(|cx| ScannerView::new(window, cx)),
        };
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
        let selection = cx.subscribe_in(&views.functions, window, |this, _, FunctionSelected(function), window, cx| {
            let Some(va) = parse_va(&function.va) else { return };
            this.select(Location { va, function: Some(function.clone()) }, window, cx);
        });
        // A function the engine now names differently: the selection follows it.
        let renamed = cx.subscribe_in(&views.functions, window, |this, _, FunctionChanged(entry), window, cx| {
            let Some(location) = this.location.clone() else { return };
            if location.function.as_ref().is_some_and(|f| f.va == entry.va) {
                this.show_location(Location { va: location.va, function: Some(entry.clone()) }, window, cx);
            }
        });
        let variable_edits = cx.subscribe_in(&views.decompiler, window, |this, _, event: &EditVariable, window, cx| {
            this.edit_variable(event, window, cx);
        });
        // Quitting closes the target like opening another does.
        let quitting = cx.on_app_quit(|wb, cx| {
            wb.close_target(cx);
            async {}
        });
        // Every view that can send the user somewhere says so the same way.
        let mut navigation = vec![
            cx.subscribe_in(&views.disassembly, window, Self::on_navigate),
            cx.subscribe_in(&views.disassembly, window, |this, _, Point(va), window, cx| {
                let location = this.locate(*va, cx);
                this.show_location(location, window, cx);
            }),
            cx.subscribe_in(&views.graph, window, Self::on_navigate),
            cx.subscribe_in(&views.linear, window, Self::on_navigate),
            // The listing numbers functions by address; it follows the list's pages.
            cx.subscribe_in(&views.functions, window, |this, _, _: &FunctionsLoaded, window, cx| {
                this.views.linear.update(cx, |view, cx| view.sync(cx));
                // A list read again for new names: the selection takes its new entry.
                if let Some(location) = this.location.clone() {
                    let now = this.locate(location.va, cx);
                    if now.function.is_some() && now != location {
                        this.show_location(now, window, cx);
                    }
                }
            }),
            cx.subscribe_in(&views.xrefs, window, Self::on_navigate),
            cx.subscribe_in(&views.triage, window, Self::on_navigate),
            cx.subscribe_in(&views.bookmarks, window, Self::on_navigate),
            cx.subscribe_in(&views.find, window, Self::on_navigate),
        ];
        let status_poll = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(STATUS_POLL).await;
                let alive = this.update_in(cx, |wb, window, cx| {
                    let status = wb.target.as_ref().map(|t| t.engine.status());
                    if status != wb.engine_status {
                        wb.engine_status = status;
                        cx.notify();
                    }
                    let analysis = wb.target.as_ref().and_then(|t| t.analysis.as_ref()).map(Analysis::state);
                    if analysis != wb.analysis_state {
                        let before = std::mem::replace(&mut wb.analysis_state, analysis);
                        wb.analysis_moved(before, window, cx);
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
        if !startup_notes.is_empty() {
            window.defer(cx, move |window, cx| {
                for problem in startup_notes {
                    window.push_notification(SharedString::from(problem), cx);
                }
            });
        }
        let mut workbench = Self {
            target: None,
            open_error: None,
            engine_status: None,
            analysis_state: None,
            location: None,
            nav: NavHistory::default(),
            journal: Journal::default(),
            edit_busy: false,
            dock_shown: false,
            views,
            menu_bar: menus::init(cx),
            focus_handle: cx.focus_handle(),
            dock_area,
            _dock_skin: dock_skin,
            history,
            layout_save_failed: false,
            recent_save_failed: false,
            next_change_is_load: true,
            _layout_settle: None,
            _theme_poll: theme_poll,
            _status_poll: status_poll,
            _subscriptions: {
                navigation.extend([selection, renamed, variable_edits, quitting, layout_changes]);
                navigation
            },
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
        // is said out loud. A chosen place that cannot be used falls back to the
        // central one, and says so.
        let location = cx.global::<Prefs>().project_location.clone();
        let project = match project::project_dir_at(&path, &location) {
            Ok(dir) => Some(dir),
            Err(e) if location != ProjectLocation::Central => {
                let text = format!("This target's project could not be kept where the settings say ({e}), so it is in the central folder.");
                window.push_notification(SharedString::from(text), cx);
                project::project_dir(&path).ok()
            }
            Err(e) => {
                self.open_error = Some(format!("no project directory ({e}); names will not be kept"));
                None
            }
        };
        if project.is_some() {
            self.open_error = None;
        }
        self.close_target(cx);
        let engine_program = command.program().to_path_buf();
        // The analysis writes into the same project the session reads.
        let analysis = cx.global::<Prefs>().analyze_on_open.then(|| start_analysis(&command, &path, project.as_deref(), cx));
        let analyzing = analysis.is_some();
        let engine = Arc::new(Engine::start(command.clone(), path.clone(), project.clone()));
        self.target = Some(Target { path: path.clone(), engine: Arc::clone(&engine), command, project, analysis, engine_program });
        self.analysis_state = None;
        self.engine_status = Some(engine.status());
        self.location = None;
        self.nav.clear();
        self.journal.clear();
        self.edit_busy = false;
        let v = &self.views;
        let e = || Some(Arc::clone(&engine));
        v.decompiler.update(cx, |view, cx| view.set_engine(e(), cx));
        v.disassembly.update(cx, |view, cx| view.set_engine(e(), cx));
        v.graph.update(cx, |view, cx| view.set_engine(e(), cx));
        v.linear.update(cx, |view, cx| view.set_engine(e(), cx));
        v.xrefs.update(cx, |view, cx| {
            view.set_engine(e(), cx);
            // Without an analysis the session builds the index on its first query.
            view.set_index_pending(analyzing.then(|| n0xis_client::Phase::Starting.label().into()), cx);
        });
        v.triage.update(cx, |view, cx| view.set_engine(e(), cx));
        v.bookmarks.update(cx, |view, cx| view.set_engine(e(), cx));
        v.types.update(cx, |view, cx| view.set_engine(e(), cx));
        v.find.update(cx, |view, cx| view.set_engine(e(), cx));
        v.console.update(cx, |view, cx| view.set_engine(e(), cx));
        v.functions.update(cx, |list, cx| list.load(engine, cx));
        let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        window.set_window_title(&format!("{name} — N0xis"));
        cx.update_global::<Recent, _>(|recent, _| recent.opened(&path, recent::now()));
        self.keep_recent(window, cx);
        cx.notify();
    }

    /// Leave the open target: its analysis stops, and its caches are cleared
    /// if the settings say not to keep them. The user's work is never touched.
    fn close_target(&mut self, cx: &App) {
        let Some(target) = self.target.take() else { return };
        drop(target.analysis);
        if !cx.global::<Prefs>().keep_cache_on_close
            && let Some(project) = &target.project
        {
            let _ = target.command.run(&["project".into(), "cache".into(), "--clear".into()], Some(project));
        }
    }

    fn on_run_analysis(&mut self, _: &RunAnalysis, _: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.target.as_mut() else { return };
        let analysis = start_analysis(&target.command, &target.path, target.project.as_deref(), cx);
        target.analysis = Some(analysis);
        self.analysis_state = None;
        let pending = Some(n0xis_client::Phase::Starting.label().into());
        self.views.xrefs.update(cx, |view, cx| view.set_index_pending(pending, cx));
        cx.notify();
    }

    fn on_open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
        settings::open(self.engine(), window, cx);
    }

    /// Save the recent targets and show them in the menu; a failed save is said once.
    fn keep_recent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = cx.global::<Recent>().write()
            && !self.recent_save_failed
        {
            self.recent_save_failed = true;
            window.push_notification(SharedString::from(format!("The recent targets could not be saved: {e}")), cx);
        }
        self.refresh_menus(cx);
    }

    fn on_open_recent(&mut self, action: &OpenRecent, window: &mut Window, cx: &mut Context<Self>) {
        self.open_recent(PathBuf::from(action.0.as_ref()), window, cx);
    }

    fn open_recent(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if path.is_file() {
            self.open(path, window, cx);
            return;
        }
        cx.update_global::<Recent, _>(|recent, _| recent.forget(&path));
        self.keep_recent(window, cx);
        let text = format!("{} is no longer there, so it was taken off the recent targets.", path.display());
        window.push_notification(SharedString::from(text), cx);
    }

    /// Go to `location` and show it everywhere; the place left can be gone back to.
    fn select(&mut self, location: Location, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(from) = self.location.clone()
            && from.va != location.va
        {
            self.nav.left(from);
        }
        self.show_location(location, window, cx);
    }

    /// Make `location` the selection, and show it in every view.
    fn show_location(&mut self, location: Location, window: &mut Window, cx: &mut Context<Self>) {
        let v = self.views.clone();
        v.functions.update(cx, |list, cx| list.reveal(location.va, cx));
        v.decompiler.update(cx, |view, cx| view.show(&location, window, cx));
        v.disassembly.update(cx, |view, cx| view.show(&location, cx));
        v.graph.update(cx, |view, cx| view.show(&location, window, cx));
        v.linear.update(cx, |view, cx| view.show(&location, cx));
        v.xrefs.update(cx, |view, cx| view.show(&location, cx));
        self.location = Some(location);
        cx.notify();
    }

    /// Go to an address some view asked for: the function it lies in, when the
    /// engine's list says which one.
    fn go_to(&mut self, va: u64, window: &mut Window, cx: &mut Context<Self>) {
        let location = self.locate(va, cx);
        self.select(location, window, cx);
    }

    /// `va` with the function the list now puts it in.
    fn locate(&self, va: u64, cx: &App) -> Location {
        let function = self.views.functions.read(cx).index().function_at(va).cloned();
        Location { va, function }
    }

    fn engine(&self) -> Option<Arc<Engine>> {
        self.target.as_ref().map(|t| Arc::clone(&t.engine))
    }

    fn on_go_back(&mut self, _: &GoBack, window: &mut Window, cx: &mut Context<Self>) {
        self.go_back(window, cx);
    }

    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(to) = self.nav.back(self.location.clone()) {
            let location = self.locate(to.va, cx);
            self.show_location(location, window, cx);
        }
    }

    fn on_go_forward(&mut self, _: &GoForward, window: &mut Window, cx: &mut Context<Self>) {
        self.go_forward(window, cx);
    }

    fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(to) = self.nav.forward(self.location.clone()) {
            let location = self.locate(to.va, cx);
            self.show_location(location, window, cx);
        }
    }

    fn on_go_to(&mut self, _: &GoTo, window: &mut Window, cx: &mut Context<Self>) {
        if self.target.is_none() {
            window.push_notification("Open a target first.", cx);
            return;
        }
        let this = cx.entity().downgrade();
        let prompt = Prompt {
            title: "Go to".into(),
            detail: Some("A function's name, or an address.".into()),
            initial: String::new(),
            placeholder: "main or 0x401000",
            ok: "Go",
        };
        prompt::ask(prompt, window, cx, move |answer, window, cx| {
            this.update(cx, |wb, cx| match wb.resolve(&answer, cx) {
                Ok(va) => wb.go_to(va, window, cx),
                Err(why) => window.push_notification(SharedString::from(why), cx),
            })
            .ok();
        });
    }

    /// A typed place: a listed function's exact name first, then an address.
    fn resolve(&self, answer: &str, cx: &App) -> Result<u64, String> {
        let answer = answer.trim();
        if answer.is_empty() {
            return Err("Nothing to go to.".into());
        }
        let list = self.views.functions.read(cx);
        match list.index().named_exactly(answer).as_slice() {
            [one] => return parse_va(&one.va).ok_or_else(|| format!("{answer} has no address in the list.")),
            [] => {}
            many => return Err(format!("{} functions are called {answer}; go to one by its address.", many.len())),
        }
        parse_va(answer).ok_or_else(|| format!("No listed function is called {answer}, and it is not an address."))
    }

    /// Ask for a new value of `fact`, then write it. What the field starts with
    /// is the recorded value, or failing that `shown`, what the user sees now.
    fn edit_fact(&mut self, fact: Fact, label: String, prompt: Prompt, shown: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(engine) = self.engine() else {
            window.push_notification("Open a target first.", cx);
            return;
        };
        if self.edit_busy {
            window.push_notification("The last change is still being written.", cx);
            return;
        }
        let current = engine.send(&ShowAnnotations { addr: hex(fact.addr()) });
        cx.spawn_in(window, async move |this, cx| {
            let record = current.await;
            this.update_in(cx, |_, window, cx| {
                let record = match record {
                    Ok(record) => record,
                    Err(e) => {
                        let text = format!("Could not read what is recorded at {}: {e}", hex(fact.addr()));
                        window.push_notification(SharedString::from(text), cx);
                        return;
                    }
                };
                let before = fact.read(record.as_ref());
                let initial = before.clone().or(shown).unwrap_or_default();
                let this = cx.entity().downgrade();
                prompt::ask(Prompt { initial, ..prompt }, window, cx, move |answer, window, cx| {
                    let after = prompt::as_value(&answer);
                    if after == before {
                        return;
                    }
                    let edit = Edit { label: label.clone(), changes: vec![Change { fact: fact.clone(), before: before.clone(), after }] };
                    this.update(cx, |wb, cx| wb.write_edit(edit, window, cx)).ok();
                });
            })
            .ok();
        })
        .detach();
    }

    /// Write an edit and keep it for Undo, with the values the engine reported.
    fn write_edit(&mut self, edit: Edit, window: &mut Window, cx: &mut Context<Self>) {
        let Some(engine) = self.engine() else { return };
        self.edit_busy = true;
        let writes = edit.redo_writes();
        cx.spawn_in(window, async move |this, cx| {
            let applied = edits::apply(engine, writes).await;
            this.update_in(cx, |wb, window, cx| {
                wb.edit_busy = false;
                let made = applied.now.len();
                let changes: Vec<Change> = edit
                    .changes
                    .iter()
                    .zip(applied.now)
                    .map(|(c, now)| Change { fact: c.fact.clone(), before: c.before.clone(), after: now })
                    .collect();
                let kept = Edit { label: edit.label.clone(), changes };
                let announce = edit.changes.len() > 1 || edit.changes.iter().any(|c| matches!(c.fact, Fact::Bookmark { .. }));
                if !kept.changes.is_empty() {
                    wb.annotations_changed(&kept, window, cx);
                    wb.journal.record(kept);
                }
                let text = match applied.refused {
                    None if announce => format!("{}.", edit.label),
                    None => return,
                    Some(e) if made == 0 => format!("{}: the engine refused it: {e}", edit.label),
                    Some(e) => format!("{}: {made} of {} changes made, then the engine refused: {e}", edit.label, edit.changes.len()),
                };
                window.push_notification(SharedString::from(text), cx);
            })
            .ok();
        })
        .detach();
    }

    fn on_undo_edit(&mut self, _: &UndoEdit, window: &mut Window, cx: &mut Context<Self>) {
        self.replay(true, window, cx);
    }

    fn on_redo_edit(&mut self, _: &RedoEdit, window: &mut Window, cx: &mut Context<Self>) {
        self.replay(false, window, cx);
    }

    /// Undo or redo the last edit, through the engine.
    fn replay(&mut self, undo: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(engine) = self.engine() else { return };
        if self.edit_busy {
            window.push_notification("The last change is still being written.", cx);
            return;
        }
        let taken = if undo { self.journal.take_undo() } else { self.journal.take_redo() };
        let Some(edit) = taken else {
            window.push_notification(if undo { "Nothing to undo." } else { "Nothing to redo." }, cx);
            return;
        };
        self.edit_busy = true;
        let writes = if undo { edit.undo_writes() } else { edit.redo_writes() };
        cx.spawn_in(window, async move |this, cx| {
            let applied = edits::apply(engine, writes).await;
            this.update_in(cx, |wb, window, cx| {
                wb.edit_busy = false;
                let verb = if undo { "Undid" } else { "Redid" };
                let made = applied.now.len();
                let text = match applied.refused {
                    None => {
                        wb.annotations_changed(&edit, window, cx);
                        let text = format!("{verb}: {}.", edit.label);
                        if undo { wb.journal.undid(edit) } else { wb.journal.redid(edit) }
                        text
                    }
                    Some(e) if made == 0 => {
                        let text = format!("Could not {}: {}: {e}", if undo { "undo" } else { "redo" }, edit.label);
                        if undo { wb.journal.put_back_undo(edit) } else { wb.journal.put_back_redo(edit) }
                        text
                    }
                    // Part of it went through: the target is now between the two
                    // states, so the edit cannot be replayed as it was.
                    Some(e) => {
                        wb.annotations_changed(&edit, window, cx);
                        format!("{verb} only {made} of {} changes of {}; the engine refused: {e}", edit.changes.len(), edit.label)
                    }
                };
                window.push_notification(SharedString::from(text), cx);
            })
            .ok();
        })
        .detach();
    }

    /// Show the target again where an edit can have changed what is shown.
    fn annotations_changed(&mut self, edit: &Edit, window: &mut Window, cx: &mut Context<Self>) {
        let v = self.views.clone();
        v.decompiler.update(cx, |view, cx| view.refresh(window, cx));
        if let Some(location) = self.location.clone() {
            v.disassembly.update(cx, |view, cx| view.refresh(&location, cx));
        }
        v.linear.update(cx, |view, cx| view.refresh(cx));
        v.xrefs.update(cx, |view, cx| view.refresh(cx));
        v.bookmarks.update(cx, |view, cx| view.refresh(cx));
        if edit.renames() {
            v.graph.update(cx, |view, cx| view.refresh(window, cx));
            if let Some(engine) = self.engine() {
                for change in &edit.changes {
                    if let Fact::Name { addr } = change.fact {
                        let engine = Arc::clone(&engine);
                        v.functions.update(cx, |list, cx| list.reread(engine, addr, cx));
                    }
                }
            }
        }
    }

    /// The analysis moved on: tell the references view where it is, and when it
    /// finishes, show the names it found.
    fn analysis_moved(&mut self, before: Option<AnalysisState>, window: &mut Window, cx: &mut Context<Self>) {
        let pending = self.analysis_state.as_ref().filter(|s| !s.index_ready()).map(analysis_text);
        self.views.xrefs.update(cx, |view, cx| view.set_index_pending(pending, cx));
        let finished = matches!(before, Some(AnalysisState::Running(_)) | None)
            && matches!(&self.analysis_state, Some(AnalysisState::Finished(report)) if report.names_changed());
        if finished && let Some(engine) = self.engine() {
            let v = self.views.clone();
            v.functions.update(cx, |list, cx| list.refresh_names(engine, cx));
            v.decompiler.update(cx, |view, cx| view.refresh(window, cx));
            v.linear.update(cx, |view, cx| view.refresh(cx));
            if let Some(location) = self.location.clone() {
                v.disassembly.update(cx, |view, cx| view.refresh(&location, cx));
            }
        }
    }

    /// The selection, or a message saying one is needed.
    fn selection(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<Location> {
        if self.target.is_none() {
            window.push_notification("Open a target first.", cx);
            return None;
        }
        let location = self.location.clone();
        if location.is_none() {
            window.push_notification("Select a function or an address first.", cx);
        }
        location
    }

    fn on_rename(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
        let Some(location) = self.selection(window, cx) else { return };
        let (fact, label, prompt, shown) = match &location.function {
            Some(f) => {
                let Some(start) = parse_va(&f.va) else { return };
                (
                    Fact::Name { addr: start },
                    format!("Rename {}", f.name),
                    Prompt {
                        title: format!("Rename function {}", f.name),
                        detail: Some("Leave it empty to go back to the name the engine finds.".into()),
                        initial: String::new(),
                        placeholder: "name",
                        ok: "Rename",
                    },
                    Some(f.name.clone()),
                )
            }
            None => (
                Fact::Name { addr: location.va },
                format!("Name {}", hex(location.va)),
                Prompt {
                    title: format!("Name {}", hex(location.va)),
                    detail: Some("No listed function starts or lies here; the name is kept for the address.".into()),
                    initial: String::new(),
                    placeholder: "name",
                    ok: "Name",
                },
                None,
            ),
        };
        self.edit_fact(fact, label, prompt, shown, window, cx);
    }

    fn on_comment(&mut self, _: &Comment, window: &mut Window, cx: &mut Context<Self>) {
        let Some(location) = self.selection(window, cx) else { return };
        let prompt = Prompt {
            title: format!("Comment at {}", location.label()),
            detail: Some("Shown in the disassembly. Leave it empty to remove it.".into()),
            initial: String::new(),
            placeholder: "comment",
            ok: "Comment",
        };
        self.edit_fact(Fact::Comment { addr: location.va }, format!("Comment at {}", location.label()), prompt, None, window, cx);
    }

    /// Reached when the decompiler had no parameter or local under the caret.
    fn on_set_type(&mut self, _: &SetType, window: &mut Window, cx: &mut Context<Self>) {
        let text = "Put the caret on a parameter or a local in the decompiler to set its type. For the value a function returns, use Edit ▸ Set Return Type.";
        window.push_notification(text, cx);
    }

    fn on_set_return_type(&mut self, _: &SetReturnType, window: &mut Window, cx: &mut Context<Self>) {
        let Some(location) = self.selection(window, cx) else { return };
        let Some(f) = location.function.clone() else {
            window.push_notification("No listed function covers the selection, so it has no return type to set.", cx);
            return;
        };
        let Some(start) = parse_va(&f.va) else { return };
        let prompt = Prompt {
            title: format!("Return type of {}", f.name),
            detail: Some("A C type, such as int, void or char *. Leave it empty to let the decompiler infer it.".into()),
            initial: String::new(),
            placeholder: "int",
            ok: "Set",
        };
        self.edit_fact(Fact::ReturnType { function: start }, format!("Return type of {}", f.name), prompt, None, window, cx);
    }

    fn edit_variable(&mut self, event: &EditVariable, window: &mut Window, cx: &mut Context<Self>) {
        let Some(start) = parse_va(&event.function.va) else { return };
        let (variable, function) = (&event.variable, &event.function.name);
        match event.intent {
            VariableIntent::Rename => {
                let prompt = Prompt {
                    title: format!("Rename variable {}", variable.name),
                    detail: Some(format!("In {function}. Leave it empty to go back to {}.", variable.key)),
                    initial: String::new(),
                    placeholder: "name",
                    ok: "Rename",
                };
                let fact = Fact::VariableName { function: start, key: variable.key.clone() };
                self.edit_fact(fact, format!("Rename {} in {function}", variable.name), prompt, Some(variable.name.clone()), window, cx);
            }
            VariableIntent::SetType => {
                let prompt = Prompt {
                    title: format!("Type of {}", variable.name),
                    detail: Some(format!("In {function}. A C type, such as int, char * or struct Foo *. Leave it empty to let the decompiler infer it.")),
                    initial: String::new(),
                    placeholder: "int",
                    ok: "Set",
                };
                let fact = Fact::VariableType { function: start, key: variable.key.clone() };
                self.edit_fact(fact, format!("Type of {} in {function}", variable.name), prompt, None, window, cx);
            }
        }
    }

    fn on_clear_annotations(&mut self, _: &ClearAnnotations, window: &mut Window, cx: &mut Context<Self>) {
        let Some(location) = self.selection(window, cx) else { return };
        let Some(engine) = self.engine() else { return };
        if self.edit_busy {
            window.push_notification("The last change is still being written.", cx);
            return;
        }
        let current = engine.send(&ShowAnnotations { addr: hex(location.va) });
        cx.spawn_in(window, async move |this, cx| {
            let record = current.await;
            this.update_in(cx, |wb, window, cx| {
                let record = match record {
                    Ok(Some(record)) => record,
                    Ok(None) => {
                        window.push_notification(SharedString::from(format!("Nothing is recorded at {}.", location.label())), cx);
                        return;
                    }
                    Err(e) => {
                        window.push_notification(SharedString::from(format!("Could not read what is recorded at {}: {e}", location.label())), cx);
                        return;
                    }
                };
                let changes: Vec<Change> = Fact::all_in(&record, location.va)
                    .into_iter()
                    .map(|fact| Change { before: fact.read(Some(&record)), fact, after: None })
                    .collect();
                if changes.is_empty() {
                    window.push_notification(SharedString::from(format!("Nothing is recorded at {}.", location.label())), cx);
                    return;
                }
                let label = format!("Cleared {} annotations at {}", changes.len(), location.label());
                wb.write_edit(Edit { label, changes }, window, cx);
            })
            .ok();
        })
        .detach();
    }

    fn on_navigate<V>(&mut self, _: &Entity<V>, Navigate(va): &Navigate, window: &mut Window, cx: &mut Context<Self>) {
        self.go_to(*va, window, cx);
    }

    fn on_show_panel(&mut self, action: &ShowPanel, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = PanelKind::from_name(&action.0) else { return };
        // A panel opened by name is wanted now, target or not (the scanner needs none).
        self.dock_shown = true;
        let (id, handle) = (self.views.panel_id(kind), self.views.handle(kind));
        let companion = kind.companion().map(|c| self.views.panel_id(c)).filter(|&c| c != id);
        self.dock_area.update(cx, |area, cx| {
            if area.panel(id).is_some() {
                area.select_panel(id, window, cx);
                return;
            }
            // Added first (that registers it), then moved where it belongs: into
            // its companion's group, or for the function list, a column on the left.
            area.add_panel_view(handle, DockPlacement::Center, None, window, cx);
            let tree = area.layout(DockPlacement::Center);
            let target = match companion {
                Some(c) => tree.and_then(|t| t.find_panel_node(c)).map(|node| InsertTarget::Tabs { node, ix: None, activate: true }),
                None => tree.map(|t| InsertTarget::Split { node: t.root().id(), placement: Placement::Left, size: Some(px(300.)) }),
            };
            if let Some(target) = target {
                area.move_panel(id, target, window, cx);
            }
        });
        cx.notify();
    }

    fn on_toggle_bookmark(&mut self, _: &ToggleBookmark, window: &mut Window, cx: &mut Context<Self>) {
        let Some(location) = self.selection(window, cx) else { return };
        let Some(engine) = self.engine() else { return };
        if self.edit_busy {
            window.push_notification("The last change is still being written.", cx);
            return;
        }
        let fact = Fact::Bookmark { addr: location.va };
        let current = engine.send(&ShowAnnotations { addr: hex(location.va) });
        cx.spawn_in(window, async move |this, cx| {
            let record = current.await;
            this.update_in(cx, |wb, window, cx| {
                let before = match record {
                    Ok(record) => fact.read(record.as_ref()),
                    Err(e) => {
                        window.push_notification(SharedString::from(format!("The bookmark was not changed: {e}")), cx);
                        return;
                    }
                };
                // The engine's own word for "on" comes back with the answer.
                let (after, label) = match before {
                    Some(_) => (None, format!("Removed the bookmark at {}", location.label())),
                    None => (Some(String::new()), format!("Bookmarked {}", location.label())),
                };
                wb.write_edit(Edit { label, changes: vec![Change { fact, before, after }] }, window, cx);
            })
            .ok();
        })
        .detach();
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
        let views = self.views.clone();
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
            Some(EngineStatus::Ready { label, json_argv, version }) => {
                // An engine that reads only text requests cannot be sent every
                // value; say so rather than let a refused edit look like a bug.
                let note = if *json_argv { "" } else { " · older engine: some values cannot be sent" };
                let version = version.as_deref().map(|v| format!(" {v}")).unwrap_or_default();
                (format!("engine{version} ready · {label}{note}"), theme.success)
            }
            Some(EngineStatus::Crashed { crashes, reason }) => (
                format!("engine crashed ({crashes}×), restarting on the next request: {}", first_line(reason)),
                theme.warning,
            ),
            Some(EngineStatus::GaveUp { reason }) => (format!("engine stopped: {}", first_line(reason)), theme.danger),
            Some(EngineStatus::Stopped) => ("engine stopped".to_string(), theme.muted_foreground),
        };
        let selection = self.location.as_ref().map(|l| format!("{} · {}", l.label(), hex(l.va))).unwrap_or_default();
        let analysis = self.analysis_state.as_ref().map(|state| {
            let color = if matches!(state, AnalysisState::Failed(_)) { theme.warning } else { theme.muted_foreground };
            div().text_xs().text_color(color).truncate().child(analysis_text(state))
        });
        StatusBar::new()
            .left(h_flex().gap_3().min_w_0().child(div().text_xs().text_color(color).truncate().child(text)).children(analysis))
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
                    .icon(AppIcon::Open)
                    .label("Open…")
                    .on_click(cx.listener(|wb, _: &ClickEvent, window, cx| wb.prompt_open(window, cx))),
            )
            .child(
                Button::new("scan-empty")
                    .ghost()
                    .icon(AppIcon::Scan)
                    .label("Scan a running process…")
                    .on_click(cx.listener(|wb, _: &ClickEvent, window, cx| {
                        wb.on_show_panel(&ShowPanel(PanelKind::Scanner.name().into()), window, cx)
                    })),
            )
            .children(self.open_error.clone().map(|e| div().text_sm().text_color(theme.danger).child(e)))
            .children(self.render_recent(cx))
    }

    /// The recent targets on the start screen, newest first.
    fn render_recent(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let targets = cx.global::<Recent>().targets().to_vec();
        if targets.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        let list = v_flex()
            .pt_4()
            .gap_1()
            .items_center()
            .child(div().text_xs().text_color(muted).child("Recent"))
            .children(targets.into_iter().enumerate().map(|(i, target)| {
                let path = target.path.clone();
                Button::new(("recent", i))
                    .ghost()
                    .label(target.label())
                    .on_click(cx.listener(move |wb, _: &ClickEvent, window, cx| wb.open_recent(path.clone(), window, cx)))
            }));
        Some(list.into_any_element())
    }
}

/// Where the analysis is, in a few words.
fn analysis_text(state: &AnalysisState) -> String {
    match state {
        AnalysisState::Running(p) if p.total > 0 => format!("analysis: {} {}/{}", p.phase.label(), p.done, p.total),
        AnalysisState::Running(p) => format!("analysis: {}", p.phase.label()),
        AnalysisState::Finished(r) => {
            format!("analysed · {} functions · {} classes · {} reference targets", r.functions, r.rtti_classes, r.xref_targets)
        }
        AnalysisState::Failed(why) => format!("analysis failed: {}", first_line(why)),
    }
}

/// Start the engine's whole-program pass as the settings ask.
fn start_analysis(command: &EngineCommand, path: &std::path::Path, project: Option<&std::path::Path>, cx: &App) -> Analysis {
    let warm_up = if cx.global::<Prefs>().warm_up { WarmUp::All } else { WarmUp::Skip };
    Analysis::start(command, path, project, warm_up)
}

fn first_line(s: &str) -> &str {
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or(s)
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (background, foreground) = (cx.theme().background, cx.theme().foreground);
        let main = if self.target.is_some() || self.dock_shown {
            self.dock_area.clone().into_any_element()
        } else {
            self.render_empty(cx).into_any_element()
        };
        v_flex()
            .size_full()
            .key_context("Workbench")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_open_recent))
            .on_action(cx.listener(Self::on_open_settings))
            .on_action(cx.listener(Self::on_run_analysis))
            .on_action(cx.listener(Self::on_about))
            .on_action(cx.listener(Self::on_undo_layout))
            .on_action(cx.listener(Self::on_redo_layout))
            .on_action(cx.listener(Self::on_reset_layout))
            .on_action(cx.listener(Self::on_select_theme))
            .on_action(cx.listener(Self::on_zoom_in))
            .on_action(cx.listener(Self::on_zoom_out))
            .on_action(cx.listener(Self::on_reset_zoom))
            .on_action(cx.listener(Self::on_show_panel))
            .on_action(cx.listener(Self::on_toggle_bookmark))
            .on_action(cx.listener(Self::on_rename))
            .on_action(cx.listener(Self::on_comment))
            .on_action(cx.listener(Self::on_set_type))
            .on_action(cx.listener(Self::on_set_return_type))
            .on_action(cx.listener(Self::on_clear_annotations))
            .on_action(cx.listener(Self::on_undo_edit))
            .on_action(cx.listener(Self::on_redo_edit))
            .on_action(cx.listener(Self::on_go_to))
            .on_action(cx.listener(Self::on_go_back))
            .on_action(cx.listener(Self::on_go_forward))
            // The mouse's back and forward buttons walk the same history.
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|wb, _: &MouseDownEvent, window, cx| wb.go_back(window, cx)),
            )
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|wb, _: &MouseDownEvent, window, cx| wb.go_forward(window, cx)),
            )
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
