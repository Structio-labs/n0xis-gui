// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Find in image: search the target's bytes for text or a byte pattern
//! (`find`). The engine reads the query; this view only says which form it is.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::base::Selectable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::*;
use n0xis_client::{Engine, Find, FindMatch, FindQuery, FindResult};

use crate::assets::AppIcon;
use crate::context;
use crate::layout::{PanelKind, Views};
use crate::nav::{Location, Navigate, parse_va};
use crate::panel::{dock_panel, header, message};

/// The most matches asked for at once; the engine says when there are more.
pub const MATCH_LIMIT: u32 = 300;

const ROW_HEIGHT: Pixels = px(22.);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Text,
    Utf16,
    Bytes,
    Escaped,
}

impl Mode {
    const ALL: [Mode; 4] = [Self::Text, Self::Utf16, Self::Bytes, Self::Escaped];

    fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Utf16 => "UTF-16",
            Self::Bytes => "Bytes",
            Self::Escaped => "Escaped",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            Self::Text | Self::Utf16 => "Text to find",
            Self::Bytes => "Hex bytes, ?? for any: 48 8B ?? C3",
            Self::Escaped => r"Text with escapes: \x00, \n, \\",
        }
    }

    pub fn query(self, text: &str) -> FindQuery {
        match self {
            Self::Text => FindQuery::Text { text: text.to_string(), utf16: false },
            Self::Utf16 => FindQuery::Text { text: text.to_string(), utf16: true },
            Self::Bytes => FindQuery::Bytes(text.to_string()),
            Self::Escaped => FindQuery::Escaped(text.to_string()),
        }
    }
}

enum ViewState {
    Idle,
    Searching,
    Ready(FindResult),
    Failed(String),
}

pub struct SearchView {
    engine: Option<Arc<Engine>>,
    mode: Mode,
    query: Entity<InputState>,
    state: ViewState,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for SearchView {}

impl SearchView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(Mode::Text.placeholder()));
        let subscription = cx.subscribe_in(&query, window, |this, _, event, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.search(cx);
            }
        });
        Self {
            engine: None,
            mode: Mode::Text,
            query,
            state: ViewState::Idle,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _request: None,
            _subscriptions: vec![subscription],
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.state = ViewState::Idle;
        self._request = None;
        cx.notify();
    }

    fn set_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.query.update(cx, |q, cx| q.set_placeholder(mode.placeholder(), window, cx));
        cx.notify();
    }

    /// The query field, for Ctrl+F to put the caret in.
    pub fn query_focus(&self, cx: &App) -> FocusHandle {
        self.query.read(cx).focus_handle(cx)
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        let text = self.query.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        self.state = ViewState::Searching;
        cx.notify();
        let pending = engine.send_latest("find", &Find { query: self.mode.query(&text), limit: MATCH_LIMIT });
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                view.state = match result {
                    Ok(found) => ViewState::Ready(found),
                    Err(n0xis_client::ClientError::Superseded) => return,
                    Err(e) => ViewState::Failed(e.to_string()),
                };
                view.scroll.scroll_to_item(0, ScrollStrategy::Top);
                cx.notify();
            })
            .ok();
        }));
    }

    fn matches(&self) -> &[FindMatch] {
        match &self.state {
            ViewState::Ready(found) => &found.matches,
            _ => &[],
        }
    }

    fn render_row(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(m) = self.matches().get(ix).cloned() else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let va = parse_va(&m.va);
        let place = va
            .and_then(|va| {
                let functions = cx.global::<Views>().functions.read(cx);
                functions.index().function_at(va).map(|f| Location { va, function: Some(f.clone()) }.label())
            })
            .unwrap_or_default();
        let title = format!("{} {place}", m.va);
        let mut row = h_flex()
            .id(("match", ix))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .text_xs()
            .child(div().w(px(130.)).flex_none().font_family(theme.mono_font_family.clone()).child(m.va.clone()))
            .child(div().w(px(80.)).flex_none().text_color(theme.muted_foreground).child(m.section.clone().unwrap_or_default()))
            .child(div().flex_1().min_w_0().truncate().child(place));
        let Some(va) = va else { return row.into_any_element() };
        row = row
            .cursor_pointer()
            .hover(|s| s.bg(theme.list_hover))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Navigate(va))));
        row.context_menu(move |menu, _, _| context::address(menu, title.clone(), va, Some(PanelKind::Find))).into_any_element()
    }

    fn summary(&self) -> String {
        match &self.state {
            // With a limit, the engine stops at it: then its count is how many
            // came back, and only "there are more" is known about the rest.
            ViewState::Ready(found) if found.truncated => {
                format!("The first {} matches; there are more · {} bytes searched", found.matches.len(), found.bytes_scanned)
            }
            ViewState::Ready(found) => format!("{} match(es) · {} bytes searched", found.count, found.bytes_scanned),
            _ => String::new(),
        }
    }
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let modes = h_flex().gap_1().children(Mode::ALL.into_iter().map(|mode| {
            Button::new(SharedString::from(format!("find-mode-{}", mode.label())))
                .label(mode.label())
                .xsmall()
                .ghost()
                .selected(self.mode == mode)
                .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| view.set_mode(mode, window, cx)))
        }));
        let controls = v_flex()
            .px_2()
            .py_2()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.query).small().prefix(Icon::new(AppIcon::Search).small())))
                    .child(
                        Button::new("find-go")
                            .label("Find")
                            .small()
                            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.search(cx))),
                    ),
            )
            .child(modes);
        let body = match &self.state {
            ViewState::Idle => {
                let text = if self.engine.is_some() { "Matches will be listed here." } else { "Open a target to search it." };
                message(text, false, cx).into_any_element()
            }
            ViewState::Searching => message("Searching…", false, cx).into_any_element(),
            ViewState::Failed(e) => message(format!("No result: {e}"), true, cx).into_any_element(),
            ViewState::Ready(found) if found.matches.is_empty() => {
                message(format!("Not found in {} bytes.", found.bytes_scanned), false, cx).into_any_element()
            }
            ViewState::Ready(_) => uniform_list(
                "find-matches",
                self.matches().len(),
                cx.processor(|this, range: Range<usize>, _window, cx| range.map(|ix| this.render_row(ix, cx)).collect::<Vec<_>>()),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element(),
        };
        let theme = cx.theme();
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header("Find in image", cx))
            .child(controls)
            .child(div().flex_1().min_h_0().child(body))
            .child(div().px_2().py_1().border_t_1().border_color(theme.border).text_xs().text_color(theme.muted_foreground).child(self.summary()))
    }
}

dock_panel!(SearchView, PanelKind::Find);

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::Mode;
    use n0xis_client::FindQuery;

    #[test]
    fn each_mode_asks_the_engine_for_its_own_form() {
        assert_eq!(Mode::Utf16.query("ab"), FindQuery::Text { text: "ab".into(), utf16: true });
        assert_eq!(Mode::Bytes.query("48 ??"), FindQuery::Bytes("48 ??".into()));
        assert_eq!(Mode::Escaped.query(r"\x00"), FindQuery::Escaped(r"\x00".into()));
    }
}
