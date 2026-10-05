// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The text the image holds (`strings`), in address order, filtered as the user
//! types. The engine finds the strings and does the filtering; a page comes in
//! as the list reaches its end. A click goes to the string's address.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{ClientError, Engine, ImageString, ListStrings, StringEncoding};

use crate::assets::AppIcon;
use crate::layout::PanelKind;
use crate::nav::{Navigate, parse_va};
use crate::panel::{dock_panel, header, message};

/// Strings asked for at once.
const PAGE: u32 = 500;
/// The next page is asked for when the list shows a row this close to the end.
const AHEAD: usize = 100;
const ROW_HEIGHT: Pixels = px(22.);

enum State {
    Idle,
    Loading,
    Ready,
    Failed(String),
}

pub struct StringsView {
    engine: Option<Arc<Engine>>,
    filter: Entity<InputState>,
    /// The filter the list holds answers for.
    asked: String,
    strings: Vec<ImageString>,
    /// How many strings match the filter in all.
    total: u64,
    state: State,
    more_pending: bool,
    active: bool,
    stale: bool,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for StringsView {}

/// A string as one line: a tab or a line break would otherwise vanish or
/// break the row, so they are shown as their escapes.
fn one_line(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r")
}

impl StringsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter, in any case"));
        let typed = cx.subscribe_in(&filter, window, |this, _, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.reload(cx);
            }
        });
        Self {
            engine: None,
            filter,
            asked: String::new(),
            strings: Vec::new(),
            total: 0,
            state: State::Idle,
            more_pending: false,
            active: false,
            stale: false,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _request: None,
            _subscriptions: vec![typed],
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.strings.clear();
        self.total = 0;
        self.state = State::Idle;
        self._request = None;
        self.reload(cx);
    }

    /// Ask again from the first page, for the filter as it is now.
    fn reload(&mut self, cx: &mut Context<Self>) {
        if !self.active {
            self.stale = self.engine.is_some();
            return;
        }
        let Some(engine) = self.engine.clone() else { return };
        self.stale = false;
        self.asked = self.filter.read(cx).value().to_string();
        self.state = State::Loading;
        self.more_pending = false;
        cx.notify();
        self.ask(engine, 0, cx);
    }

    /// The next page, once the list nears the end of what it holds.
    fn more(&mut self, cx: &mut Context<Self>) {
        if self.more_pending || !matches!(self.state, State::Ready) || self.strings.len() as u64 >= self.total {
            return;
        }
        let Some(engine) = self.engine.clone() else { return };
        self.more_pending = true;
        self.ask(engine, self.strings.len() as u32, cx);
    }

    fn ask(&mut self, engine: Arc<Engine>, offset: u32, cx: &mut Context<Self>) {
        let contains = Some(self.asked.clone()).filter(|t| !t.is_empty());
        // One key for every page and filter: a new filter supersedes a page
        // still on its way for the old one.
        let pending = engine.send_latest("strings", &ListStrings { contains, limit: PAGE, offset });
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                view.more_pending = false;
                match result {
                    Ok(page) => {
                        if offset == 0 {
                            view.strings = page.strings;
                            view.scroll.scroll_to_item(0, ScrollStrategy::Top);
                        } else {
                            view.strings.extend(page.strings);
                        }
                        view.total = page.total;
                        view.state = State::Ready;
                    }
                    Err(ClientError::Superseded) => return,
                    Err(e) => view.state = State::Failed(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.stale {
            self.reload(cx);
        }
    }

    fn render_row(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        if ix + AHEAD >= self.strings.len() {
            self.more(cx);
        }
        let Some(s) = self.strings.get(ix) else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let va = parse_va(&s.address);
        let mut row = h_flex()
            .id(("string", ix))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .text_xs()
            .child(div().w(px(110.)).flex_none().font_family(theme.mono_font_family.clone()).child(s.address.clone()))
            .child(div().w(px(90.)).flex_none().truncate().text_color(theme.muted_foreground).child(s.section.clone()))
            .child(
                div()
                    .w(px(48.))
                    .flex_none()
                    .text_color(theme.muted_foreground)
                    .child(if s.encoding == StringEncoding::Utf8 { "" } else { s.encoding.label() }),
            )
            .child(div().flex_1().min_w_0().truncate().child(one_line(&s.text)));
        if let Some(va) = va {
            row = row
                .cursor_pointer()
                .hover(|style| style.bg(theme.list_hover))
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Navigate(va))));
        }
        row.into_any_element()
    }

    fn summary(&self) -> String {
        match self.state {
            State::Ready if self.asked.is_empty() => format!("{} of {} strings", self.strings.len(), self.total),
            State::Ready => format!("{} of {} strings with “{}”", self.strings.len(), self.total, self.asked),
            _ => String::new(),
        }
    }
}

impl Render for StringsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.state {
            State::Idle => {
                let text = if self.engine.is_some() { "The image's strings will be listed here." } else { "Open a target to list its strings." };
                message(text, false, cx).into_any_element()
            }
            State::Loading => message("Reading the image…", false, cx).into_any_element(),
            State::Failed(e) => message(format!("No strings: {e}"), true, cx).into_any_element(),
            State::Ready if self.strings.is_empty() && self.asked.is_empty() => {
                message("The image holds no strings in the sections read.", false, cx).into_any_element()
            }
            State::Ready if self.strings.is_empty() => message(format!("No string holds “{}”.", self.asked), false, cx).into_any_element(),
            State::Ready => uniform_list(
                "strings",
                self.strings.len(),
                cx.processor(|this, range: Range<usize>, _, cx| range.map(|ix| this.render_row(ix, cx)).collect::<Vec<_>>()),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element(),
        };
        let theme = cx.theme();
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header("Strings", cx))
            .child(div().px_2().py_2().child(Input::new(&self.filter).small().prefix(Icon::new(AppIcon::Search).small())))
            .child(div().flex_1().min_h_0().child(body))
            .child(div().px_2().py_1().border_t_1().border_color(theme.border).text_xs().text_color(theme.muted_foreground).child(self.summary()))
    }
}

dock_panel!(StringsView, PanelKind::Strings, on_active);

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::one_line;

    #[test]
    fn a_string_shows_on_one_line_with_its_breaks_escaped() {
        assert_eq!(one_line("a\tb\nc\r"), "a\\tb\\nc\\r");
        assert_eq!(one_line(r"C:\dir"), r"C:\\dir", "a backslash is escaped too, so the escapes read back");
        assert_eq!(one_line("Привіт"), "Привіт");
    }
}
