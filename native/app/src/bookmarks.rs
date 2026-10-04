// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Bookmarks, and every other address the user has annotated, as the engine
//! keeps them (`annotate list`). Bookmarks come first.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{AnnotationRecord, Engine, ListAnnotations};

use crate::layout::{PanelKind, Views};
use crate::nav::{Location, Navigate, parse_va};
use crate::panel::{dock_panel, header, message};

const ROW_HEIGHT: Pixels = px(24.);

enum ViewState {
    Empty,
    Loading,
    Ready,
    Failed(String),
}

#[derive(Clone)]
enum Row {
    Heading(SharedString),
    Record(AnnotationRecord),
}

pub struct BookmarksView {
    engine: Option<Arc<Engine>>,
    state: ViewState,
    records: Vec<AnnotationRecord>,
    rows: Vec<Row>,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
}

impl EventEmitter<Navigate> for BookmarksView {}

impl BookmarksView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            engine: None,
            state: ViewState::Empty,
            records: Vec::new(),
            rows: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _request: None,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.records.clear();
        self.rows.clear();
        self.state = ViewState::Empty;
        // Read at once, shown or not: the list is a small file, and the bookmark
        // toggle needs to know what is marked.
        self.refresh(cx);
    }

    /// Read the list again, after anything that may have changed it.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        if self.records.is_empty() {
            self.state = ViewState::Loading;
        }
        let pending = engine.send_latest("annotations", &ListAnnotations);
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                match result {
                    Ok(list) => {
                        view.records = list.records;
                        view.state = ViewState::Ready;
                    }
                    Err(n0xis_client::ClientError::Superseded) => return,
                    Err(e) => view.state = ViewState::Failed(e.to_string()),
                }
                view.rebuild(cx);
            })
            .ok();
        }));
        cx.notify();
    }

    /// Whether `va` is bookmarked, as of the engine's last answer; `None` while
    /// that answer has not arrived.
    pub fn is_bookmarked(&self, va: u64) -> Option<bool> {
        matches!(self.state, ViewState::Ready)
            .then(|| self.records.iter().any(|r| r.bookmark && parse_va(&r.va) == Some(va)))
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let (marked, other): (Vec<_>, Vec<_>) = self.records.iter().cloned().partition(|r| r.bookmark);
        let mut rows = vec![Row::Heading(format!("Bookmarks · {}", marked.len()).into())];
        rows.extend(marked.into_iter().map(Row::Record));
        rows.push(Row::Heading(format!("Other annotations · {}", other.len()).into()));
        rows.extend(other.into_iter().map(Row::Record));
        self.rows = rows;
        cx.notify();
    }

    /// What the user wrote at an address, in a few words.
    fn summary(r: &AnnotationRecord) -> String {
        let mut parts = Vec::new();
        if let Some(c) = &r.comment {
            parts.push(format!("“{c}”"));
        }
        if let Some(t) = &r.type_note {
            parts.push(format!("type {t}"));
        }
        if !r.var_names.is_empty() {
            parts.push(format!("{} variable name(s)", r.var_names.len()));
        }
        if !r.var_types.is_empty() {
            parts.push(format!("{} variable type(s)", r.var_types.len()));
        }
        parts.join(" · ")
    }

    fn render_row(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(ix).cloned() else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let base = h_flex().id(("bookmark-row", ix)).w_full().h(ROW_HEIGHT).px_2().gap_3().text_xs();
        match row {
            Row::Heading(text) => base
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.muted_foreground)
                .child(text)
                .into_any_element(),
            Row::Record(r) => {
                let va = parse_va(&r.va);
                // The user's own name first; else where the address lies.
                let title = r.name.clone().unwrap_or_else(|| {
                    va.and_then(|va| {
                        let functions = cx.global::<Views>().functions.read(cx);
                        functions.index().function_at(va).map(|f| Location { va, function: Some(f.clone()) }.label())
                    })
                    .unwrap_or_default()
                });
                let mut row = base
                    .child(div().w(px(130.)).flex_none().font_family(theme.mono_font_family.clone()).child(r.va.clone()))
                    .child(div().w(px(180.)).flex_none().truncate().child(title))
                    .child(div().flex_1().min_w_0().truncate().text_color(theme.muted_foreground).child(Self::summary(&r)));
                if let Some(va) = va {
                    row = row
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.list_hover))
                        .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Navigate(va))));
                }
                row.into_any_element()
            }
        }
    }
}

impl Render for BookmarksView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.state {
            ViewState::Empty => message("Open a target to see its bookmarks.", false, cx).into_any_element(),
            ViewState::Loading => message("Reading annotations…", false, cx).into_any_element(),
            ViewState::Failed(e) => message(format!("No annotations: {e}"), true, cx).into_any_element(),
            ViewState::Ready if self.records.is_empty() => {
                message("Nothing annotated yet. Edit ▸ Toggle Bookmark marks the selection.", false, cx).into_any_element()
            }
            ViewState::Ready => uniform_list(
                "bookmarks",
                self.rows.len(),
                cx.processor(|this, range: Range<usize>, _window, cx| range.map(|ix| this.render_row(ix, cx)).collect::<Vec<_>>()),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element(),
        };
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header("Bookmarks", cx))
            .child(div().flex_1().min_h_0().child(body))
    }
}

dock_panel!(BookmarksView, PanelKind::Bookmarks);
