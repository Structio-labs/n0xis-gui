// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Memory scanner: find the addresses in a running process that hold a value
//! (`scan value`), then narrow them as the value changes (`scan filter`). It
//! needs no open file: each request runs the engine once, against `--pid`, in
//! a folder of its own where the engine keeps the results a later scan narrows.
//! Whether the process can be read at all is the engine's answer, shown as it
//! gives it.

use std::ops::Range;
use std::path::PathBuf;

use gpui_kit::base::{Disableable as _, Selectable as _};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::*;
use n0xis_client::{
    ClientError, EngineCommand, ListProcesses, MemoryMap, Narrow, ProcessInfo, SCAN_TYPES, ScanFilter, ScanResult, ScanValue,
};

use crate::assets::AppIcon;
use crate::layout::PanelKind;
use crate::panel::{dock_panel, header, message};

const ROW_HEIGHT: Pixels = px(22.);
const PROCESS_COLUMN_WIDTH: Pixels = px(260.);

enum Load<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

pub struct ScannerView {
    processes: Load<Vec<ProcessInfo>>,
    process_filter: Entity<InputState>,
    visible: Vec<usize>,
    process: Option<ProcessInfo>,
    /// The engine's answer to reading the process's map: how many regions, or why not.
    access: Load<usize>,
    value_type: &'static str,
    value: Entity<InputState>,
    result: Load<ScanResult>,
    /// The saved result a narrowing scan starts from.
    last_saved: Option<String>,
    scans: u32,
    active: bool,
    process_scroll: UniformListScrollHandle,
    result_scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl ScannerView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let process_filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter processes by name or id"));
        let value = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        let subscriptions = vec![
            cx.subscribe_in(&process_filter, window, |this, _, e, _, cx| {
                if matches!(e, InputEvent::Change) {
                    this.refilter(cx);
                }
            }),
            cx.subscribe_in(&value, window, |this, _, e, _, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.first_scan(cx);
                }
            }),
        ];
        Self {
            processes: Load::Idle,
            process_filter,
            visible: Vec::new(),
            process: None,
            access: Load::Idle,
            value_type: SCAN_TYPES[0],
            value,
            result: Load::Idle,
            last_saved: None,
            scans: 0,
            active: false,
            process_scroll: UniformListScrollHandle::new(),
            result_scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _task: None,
            _subscriptions: subscriptions,
        }
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && matches!(self.processes, Load::Idle) {
            self.refresh_processes(cx);
        }
    }

    /// Run one engine command on a background thread, in the scanner's folder.
    fn run<R, F>(&mut self, request: R, cx: &mut Context<Self>, done: F)
    where
        R: n0xis_client::Request + Send + 'static,
        R::Output: Send + 'static,
        F: FnOnce(&mut Self, Result<R::Output, ClientError>, &mut Context<Self>) + 'static,
    {
        let dir = crate::project::live_dir();
        let task = cx.background_executor().spawn(async move {
            let dir: PathBuf = dir.map_err(|e| ClientError::Spawn(format!("no folder for scan results: {e}")))?;
            EngineCommand::locate()?.run_request(&request, Some(&dir))
        });
        self._task = Some(cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |view, cx| {
                done(view, result, cx);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn refresh_processes(&mut self, cx: &mut Context<Self>) {
        self.processes = Load::Loading;
        self.run(ListProcesses, cx, |view, result, cx| {
            view.processes = match result {
                Ok(mut p) => {
                    p.processes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.pid.cmp(&b.pid)));
                    Load::Ready(p.processes)
                }
                Err(e) => Load::Failed(e.to_string()),
            };
            view.refilter(cx);
        });
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let q = self.process_filter.read(cx).value().trim().to_lowercase();
        self.visible = match &self.processes {
            Load::Ready(list) => list
                .iter()
                .enumerate()
                .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q) || p.pid.to_string().contains(&q))
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        };
        cx.notify();
    }

    fn pick(&mut self, process: ProcessInfo, cx: &mut Context<Self>) {
        self.process = Some(process.clone());
        self.result = Load::Idle;
        self.last_saved = None;
        self.access = Load::Loading;
        self.run(MemoryMap { pid: process.pid }, cx, |view, result, _| {
            view.access = match result {
                Ok(map) => Load::Ready(map.regions.len()),
                Err(e) => Load::Failed(e.to_string()),
            };
        });
    }

    fn next_name(&mut self) -> String {
        self.scans += 1;
        format!("gui-{}-{}", self.process.as_ref().map_or(0, |p| p.pid), self.scans)
    }

    fn first_scan(&mut self, cx: &mut Context<Self>) {
        let Some(pid) = self.process.as_ref().map(|p| p.pid) else { return };
        let value = self.value.read(cx).value().trim().to_string();
        if value.is_empty() {
            self.result = Load::Failed("Type the value to look for.".into());
            cx.notify();
            return;
        }
        let save_as = self.next_name();
        self.result = Load::Loading;
        let request = ScanValue { pid, value_type: self.value_type.to_string(), value, save_as: save_as.clone() };
        self.run(request, cx, move |view, result, _| view.take(result, save_as));
    }

    fn narrow(&mut self, narrow: Narrow, cx: &mut Context<Self>) {
        let (Some(pid), Some(from)) = (self.process.as_ref().map(|p| p.pid), self.last_saved.clone()) else { return };
        let save_as = self.next_name();
        self.result = Load::Loading;
        let request = ScanFilter { pid, from, narrow, save_as: save_as.clone() };
        self.run(request, cx, move |view, result, _| view.take(result, save_as));
    }

    fn take(&mut self, result: Result<ScanResult, ClientError>, save_as: String) {
        match result {
            Ok(r) => {
                self.result = Load::Ready(r);
                self.last_saved = Some(save_as);
            }
            // A failed narrowing keeps the previous result to narrow again from.
            Err(e) => self.result = Load::Failed(e.to_string()),
        }
        self.result_scroll.scroll_to_item(0, ScrollStrategy::Top);
    }

    fn render_process(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Load::Ready(list) = &self.processes else { return div().into_any_element() };
        let Some(p) = self.visible.get(row).and_then(|&i| list.get(i)).cloned() else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let selected = self.process.as_ref().is_some_and(|s| s.pid == p.pid);
        h_flex()
            .id(("process", row))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .text_xs()
            .cursor_pointer()
            .when(selected, |r| r.bg(theme.list_active))
            .when(!selected, |r| r.hover(|s| s.bg(theme.list_hover)))
            .child(div().flex_1().min_w_0().truncate().child(p.name.clone()))
            .child(div().font_family(theme.mono_font_family.clone()).text_color(theme.muted_foreground).child(p.pid.to_string()))
            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.pick(p.clone(), cx)))
            .into_any_element()
    }

    fn render_match(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Load::Ready(r) = &self.result else { return div().into_any_element() };
        let Some(m) = r.matches.get(row) else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let (address, value) = (m.addr.clone(), m.value.to_string());
        let line = h_flex()
            .id(("scan-match", row))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .font_family(theme.mono_font_family.clone())
            .text_xs()
            .child(div().w(px(160.)).child(m.addr.clone()))
            .child(div().text_color(theme.primary).child(m.value.to_string()));
        crate::context::row_with_menu(("scan-match-menu", row), line)
            .context_menu(move |menu, _, _| {
                let menu = crate::context::copy(menu.label(address.clone()), "Copy Address", address.clone());
                crate::context::copy(menu, "Copy Value", value.clone())
            })
            .into_any_element()
    }

    fn render_scan_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let can_narrow = self.last_saved.is_some() && !matches!(self.result, Load::Loading);
        let types = h_flex().gap_1().flex_wrap().children(SCAN_TYPES.into_iter().map(|t| {
            Button::new(SharedString::from(format!("scan-type-{t}")))
                .label(t)
                .xsmall()
                .ghost()
                .selected(self.value_type == t)
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                    view.value_type = t;
                    cx.notify();
                }))
        }));
        let narrow_button = |id: &'static str, label: &'static str, narrow: Narrow, cx: &mut Context<Self>| {
            Button::new(id)
                .label(label)
                .xsmall()
                .disabled(!can_narrow)
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.narrow(narrow.clone(), cx)))
        };
        let exact_value = self.value.read(cx).value().trim().to_string();
        v_flex()
            .px_2()
            .py_1()
            .gap_1()
            .border_b_1()
            .border_color(theme.border)
            .child(types)
            .child(
                h_flex()
                    .gap_1()
                    .child(div().w(px(200.)).child(Input::new(&self.value).xsmall()))
                    .child(
                        Button::new("scan-first")
                            .label("First scan")
                            .xsmall()
                            .primary()
                            .disabled(self.process.is_none())
                            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.first_scan(cx))),
                    )
                    .child(narrow_button("scan-exact", "Next: equals value", Narrow::Exact(exact_value), cx))
                    .child(narrow_button("scan-changed", "changed", Narrow::Changed, cx))
                    .child(narrow_button("scan-unchanged", "unchanged", Narrow::Unchanged, cx))
                    .child(narrow_button("scan-increased", "increased", Narrow::Increased, cx))
                    .child(narrow_button("scan-decreased", "decreased", Narrow::Decreased, cx)),
            )
    }
}

impl Render for ScannerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let processes = match &self.processes {
            Load::Idle | Load::Loading => message("Listing processes…", false, cx).into_any_element(),
            Load::Failed(e) => message(format!("No process list: {e}"), true, cx).into_any_element(),
            Load::Ready(_) => uniform_list(
                "processes",
                self.visible.len(),
                cx.processor(|view, range: Range<usize>, _window, cx| range.map(|row| view.render_process(row, cx)).collect::<Vec<_>>()),
            )
            .track_scroll(&self.process_scroll)
            .size_full()
            .into_any_element(),
        };
        let access = match (&self.process, &self.access) {
            (None, _) => message("Pick a process to scan.", false, cx).into_any_element(),
            (Some(p), Load::Loading) => message(format!("Reading the memory map of {} ({})…", p.name, p.pid), false, cx).into_any_element(),
            (Some(p), Load::Ready(n)) => message(format!("{} ({}): {n} regions readable.", p.name, p.pid), false, cx).into_any_element(),
            (Some(p), Load::Failed(e)) => message(format!("{} ({}) cannot be read: {e}", p.name, p.pid), true, cx).into_any_element(),
            (Some(_), Load::Idle) => div().into_any_element(),
        };
        let results = match &self.result {
            Load::Idle => div().into_any_element(),
            Load::Loading => message("Scanning…", false, cx).into_any_element(),
            Load::Failed(e) => message(format!("No result: {e}"), true, cx).into_any_element(),
            Load::Ready(r) => {
                let theme = cx.theme();
                let summary = format!(
                    "{} matches{} · {} regions{}",
                    r.total_matches,
                    if r.shown < r.total_matches { format!(", the first {} shown", r.shown) } else { String::new() },
                    r.regions,
                    if r.exhaustive { "" } else { " · not every region could be read" },
                );
                v_flex()
                    .size_full()
                    .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child(summary))
                    .child(
                        uniform_list(
                            "scan-matches",
                            r.matches.len(),
                            cx.processor(|view, range: Range<usize>, _window, cx| range.map(|row| view.render_match(row, cx)).collect::<Vec<_>>()),
                        )
                        .track_scroll(&self.result_scroll)
                        .flex_1(),
                    )
                    .into_any_element()
            }
        };
        let theme = cx.theme();
        let refresh = Button::new("scan-refresh")
            .icon(AppIcon::Reset)
            .xsmall()
            .ghost()
            .tooltip("List processes again")
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.refresh_processes(cx)));
        // Processes on the left, the scan on the right: both fit a short panel.
        let left = v_flex()
            .w(PROCESS_COLUMN_WIDTH)
            .flex_none()
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .child(div().px_2().py_1().child(Input::new(&self.process_filter).xsmall().prefix(Icon::new(AppIcon::Search).xsmall())))
            .child(div().flex_1().min_h_0().child(processes));
        let right = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(access)
            .child(self.render_scan_controls(cx))
            .child(div().flex_1().min_h_0().child(results));
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header("Memory scanner", cx).child(refresh))
            .child(h_flex().flex_1().min_h_0().child(left).child(right))
    }
}

dock_panel!(ScannerView, PanelKind::Scanner, on_active);
