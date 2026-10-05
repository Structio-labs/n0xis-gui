// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Cross-references of the selection: who references it (`xref to`), and, for
//! a function, the calls it makes as the engine resolved them (`ir build`).
//! `xref from` is not used: it reports one instruction, not a function.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{BuildCfg, CallSite, ClientError, Engine, Xref, XrefsTo};

use crate::layout::{PanelKind, Views};
use crate::nav::{Location, Navigate, hex, parse_va};
use crate::panel::{dock_panel, header, message};

const ROW_HEIGHT: Pixels = px(22.);

enum Load<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Load<T> {
    fn from(result: Result<T, ClientError>) -> Option<Self> {
        match result {
            Ok(v) => Some(Self::Ready(v)),
            Err(ClientError::Superseded) => None,
            Err(e) => Some(Self::Failed(e.to_string())),
        }
    }
}

/// One line of the list: section headers and their entries share one list, so
/// thousands of references scroll like any other list.
#[derive(Clone)]
enum Row {
    Heading(SharedString),
    Note(SharedString, bool),
    Ref(Xref),
    Call(CallSite),
}

pub struct XrefsView {
    engine: Option<Arc<Engine>>,
    location: Option<Location>,
    active: bool,
    /// The selection changed while the tab was hidden.
    stale: bool,
    refs: Load<Vec<Xref>>,
    calls: Load<Vec<CallSite>>,
    rows: Vec<Row>,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _refs: Option<Task<()>>,
    _calls: Option<Task<()>>,
}

impl EventEmitter<Navigate> for XrefsView {}

impl XrefsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            engine: None,
            location: None,
            active: false,
            stale: false,
            refs: Load::Idle,
            calls: Load::Idle,
            rows: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _refs: None,
            _calls: None,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.location = None;
        self.refs = Load::Idle;
        self.calls = Load::Idle;
        self._refs = None;
        self._calls = None;
        self.rebuild(cx);
    }

    pub fn show(&mut self, location: &Location, cx: &mut Context<Self>) {
        let same_subject = self.location.as_ref().map(Location::subject) == Some(location.subject());
        self.location = Some(location.clone());
        if same_subject && !self.stale && !matches!(self.refs, Load::Idle) {
            return;
        }
        if self.active {
            self.load(cx);
        } else {
            self.stale = true;
        }
    }

    /// Ask again for the shown place: a name in the answer may have changed.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.location.is_none() {
            return;
        }
        if self.active {
            self.load(cx);
        } else {
            self.stale = true;
        }
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.stale {
            self.load(cx);
        }
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let (Some(engine), Some(location)) = (self.engine.clone(), self.location.clone()) else { return };
        self.stale = false;
        self.refs = Load::Loading;
        let pending = engine.send_latest("xrefs-to", &XrefsTo { addr: hex(location.subject()) });
        self._refs = Some(cx.spawn(async move |this, cx| {
            let result = pending.await.map(|x| x.refs);
            this.update(cx, |view, cx| {
                if let Some(load) = Load::from(result) {
                    view.refs = load;
                    view.rebuild(cx);
                }
            })
            .ok();
        }));
        match &location.function {
            Some(f) => {
                self.calls = Load::Loading;
                let pending = engine.send_latest("xrefs-calls", &BuildCfg { addr: f.va.clone() });
                self._calls = Some(cx.spawn(async move |this, cx| {
                    let result = pending.await.map(|g| g.callsites);
                    this.update(cx, |view, cx| {
                        if let Some(load) = Load::from(result) {
                            view.calls = load;
                            view.rebuild(cx);
                        }
                    })
                    .ok();
                }));
            }
            None => {
                self.calls = Load::Idle;
                self._calls = None;
            }
        }
        self.rebuild(cx);
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let mut rows = Vec::new();
        match &self.refs {
            Load::Idle => {}
            Load::Loading => {
                rows.push(Row::Heading("Referenced by".into()));
                rows.push(Row::Note("Looking up references… the first query on a target builds the engine's index.".into(), false));
            }
            Load::Failed(e) => {
                rows.push(Row::Heading("Referenced by".into()));
                rows.push(Row::Note(format!("No answer: {e}").into(), true));
            }
            Load::Ready(refs) => {
                rows.push(Row::Heading(format!("Referenced by · {}", refs.len()).into()));
                if refs.is_empty() {
                    rows.push(Row::Note("The engine found no instruction that references this address.".into(), false));
                }
                rows.extend(refs.iter().cloned().map(Row::Ref));
            }
        }
        match &self.calls {
            Load::Idle => {
                if self.location.as_ref().is_some_and(|l| l.function.is_none()) && !matches!(self.refs, Load::Idle) {
                    rows.push(Row::Heading("Calls".into()));
                    rows.push(Row::Note("No listed function covers this address, so it has no calls of its own.".into(), false));
                }
            }
            Load::Loading => {
                rows.push(Row::Heading("Calls".into()));
                rows.push(Row::Note("Reading the function's calls…".into(), false));
            }
            Load::Failed(e) => {
                rows.push(Row::Heading("Calls".into()));
                rows.push(Row::Note(format!("No answer: {e}").into(), true));
            }
            Load::Ready(calls) => {
                rows.push(Row::Heading(format!("Calls · {}", calls.len()).into()));
                if calls.is_empty() {
                    rows.push(Row::Note("The engine found no call in this function.".into(), false));
                }
                rows.extend(calls.iter().cloned().map(Row::Call));
            }
        }
        self.rows = rows;
        cx.notify();
    }

    /// `name+0x12` for an address inside a listed function, nothing otherwise.
    fn place(va: u64, cx: &App) -> String {
        let functions = cx.global::<Views>().functions.read(cx);
        functions.index().function_at(va).map_or(String::new(), |f| {
            Location { va, function: Some(f.clone()) }.label()
        })
    }

    fn render_row(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(ix).cloned() else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let base = h_flex().id(("xref-row", ix)).w_full().h(ROW_HEIGHT).px_2().gap_3().text_xs();
        let mono = |text: String| div().font_family(theme.mono_font_family.clone()).child(text);
        match row {
            Row::Heading(text) => base
                .pt_1()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.muted_foreground)
                .child(text)
                .into_any_element(),
            Row::Note(text, danger) => base
                .text_color(if danger { theme.danger } else { theme.muted_foreground })
                .child(div().truncate().child(text))
                .into_any_element(),
            Row::Ref(r) => {
                let from = parse_va(&r.from);
                let place = from.map(|va| Self::place(va, cx)).unwrap_or_default();
                base.child(mono(r.from.clone()).w(px(130.)).flex_none())
                    .child(div().w(px(160.)).flex_none().truncate().child(place))
                    .child(div().w(px(40.)).flex_none().text_color(theme.muted_foreground).child(r.kind.clone()))
                    .child(mono(r.text.clone()).flex_1().min_w_0().truncate().text_color(theme.muted_foreground))
                    .when_some(from, |row, va| {
                        row.cursor_pointer()
                            .hover(|s| s.bg(theme.list_hover))
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Navigate(va))))
                    })
                    .into_any_element()
            }
            Row::Call(c) => {
                let target = c.target.as_deref().and_then(parse_va);
                let name = match (target, &c.target_name) {
                    (Some(va), _) => {
                        let place = Self::place(va, cx);
                        if place.is_empty() { hex(va) } else { place }
                    }
                    (None, Some(name)) => name.clone(),
                    (None, None) => "not resolved".into(),
                };
                let go = target.or_else(|| parse_va(&c.from));
                base.child(mono(c.from.clone()).w(px(130.)).flex_none())
                    .child(div().w(px(60.)).flex_none().text_color(theme.muted_foreground).child(c.kind.clone()))
                    .child(div().flex_1().min_w_0().truncate().child(name))
                    .when_some(go, |row, va| {
                        row.cursor_pointer()
                            .hover(|s| s.bg(theme.list_hover))
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Navigate(va))))
                    })
                    .into_any_element()
            }
        }
    }
}

impl Render for XrefsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The lists are about the subject (the function, when there is one), not
        // the instruction inside it the user may have landed on.
        let title = self.location.as_ref().map_or("Cross-references".to_string(), |l| {
            format!("References · {}", Location { va: l.subject(), function: l.function.clone() }.label())
        });
        let body = if self.location.is_none() {
            message("Select a function or an address to see what references it.", false, cx).into_any_element()
        } else if self.stale && self.rows.is_empty() {
            message("Waiting to be shown.", false, cx).into_any_element()
        } else {
            uniform_list(
                "xrefs",
                self.rows.len(),
                cx.processor(|this, range: Range<usize>, _window, cx| range.map(|ix| this.render_row(ix, cx)).collect::<Vec<_>>()),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element()
        };
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header(title, cx))
            .child(div().flex_1().min_h_0().child(body))
    }
}

dock_panel!(XrefsView, PanelKind::Xrefs, on_active);
