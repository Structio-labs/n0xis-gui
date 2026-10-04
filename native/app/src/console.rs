// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! A console on the open target's engine session: any command, typed as on
//! the command line but without `--file`, answered with the engine's own JSON.
//! How a typed line splits into arguments is the engine's rule: the line is
//! sent as typed.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{ClientError, Engine};

use crate::layout::PanelKind;
use crate::panel::{clip, dock_panel, header, message};

gpui_kit::actions!(console, [HistoryPrevious, HistoryNext]);

/// How much of one answer is drawn; Copy takes all of it.
pub const OUTPUT_CLIP: usize = 16_000;
/// Entries kept; older ones are dropped from the top.
pub const MAX_ENTRIES: usize = 100;
/// Lines kept for ↑ and ↓.
pub const MAX_HISTORY: usize = 300;

/// The key context the console's command line sits in, for its ↑ / ↓ bindings.
pub const KEY_CONTEXT: &str = "Console";

struct Entry {
    line: String,
    /// `None` while the engine has not answered.
    answer: Option<Answer>,
}

struct Answer {
    text: String,
    ok: bool,
}

pub struct ConsoleView {
    engine: Option<Arc<Engine>>,
    input: Entity<InputState>,
    entries: Vec<Entry>,
    history: Vec<String>,
    /// Position while walking history with ↑ / ↓; `None` when not walking.
    history_pos: Option<usize>,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
    _requests: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl ConsoleView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("A command, e.g. function summary --addr 0x1400015c8"));
        let subscription = cx.subscribe_in(&input, window, |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.run(window, cx);
            }
        });
        Self {
            engine: None,
            input,
            entries: Vec::new(),
            history: Vec::new(),
            history_pos: None,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _requests: Vec::new(),
            _subscriptions: vec![subscription],
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.entries.clear();
        self._requests.clear();
        cx.notify();
    }

    fn run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let line = self.input.read(cx).value().trim().to_string();
        let Some(engine) = self.engine.clone() else { return };
        // A blank line ends a session; the client refuses it too, but it is not worth an entry.
        if line.is_empty() {
            return;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
        self.history_pos = None;
        self.input.update(cx, |input, cx| input.set_value("", window, cx));
        self.entries.push(Entry { line: line.clone(), answer: None });
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
        let pending = engine.call_line(line.clone());
        self._requests.push(cx.spawn(async move |this, cx| {
            let answer = match pending.await {
                Ok(envelope) => {
                    let ok = envelope.ok;
                    let text = envelope
                        .raw
                        .as_ref()
                        .and_then(|raw| serde_json::to_string_pretty(raw).ok())
                        .unwrap_or_else(|| "(the engine's answer was not kept)".into());
                    Answer { text, ok }
                }
                Err(e @ ClientError::Unsendable(_)) => Answer { text: format!("Not sent: {e}"), ok: false },
                Err(e) => Answer { text: e.to_string(), ok: false },
            };
            this.update(cx, |view, cx| {
                // The entry may have scrolled off the top meanwhile; the newest
                // unanswered entry with this line is the one this answers.
                if let Some(entry) = view.entries.iter_mut().find(|e| e.answer.is_none() && e.line == line) {
                    entry.answer = Some(answer);
                }
                view.scroll.scroll_to_bottom();
                cx.notify();
            })
            .ok();
        }));
        self.scroll.scroll_to_bottom();
        cx.notify();
    }

    fn walk_history(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.history.is_empty() {
            return;
        }
        let last = self.history.len() - 1;
        let pos = match (self.history_pos, back) {
            (None, true) => Some(last),
            (None, false) => None,
            (Some(p), true) => Some(p.saturating_sub(1)),
            (Some(p), false) if p < last => Some(p + 1),
            (Some(_), false) => None,
        };
        self.history_pos = pos;
        let text = pos.and_then(|p| self.history.get(p)).cloned().unwrap_or_default();
        self.input.update(cx, |input, cx| input.set_value(text, window, cx));
    }

    fn on_history_previous(&mut self, _: &HistoryPrevious, window: &mut Window, cx: &mut Context<Self>) {
        self.walk_history(true, window, cx);
    }

    fn on_history_next(&mut self, _: &HistoryNext, window: &mut Window, cx: &mut Context<Self>) {
        self.walk_history(false, window, cx);
    }

    fn render_entry(&self, ix: usize, entry: &Entry, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (text, color) = match &entry.answer {
            None => ("running…".to_string(), theme.muted_foreground),
            Some(a) => (clip(&a.text, OUTPUT_CLIP), if a.ok { theme.foreground } else { theme.danger }),
        };
        let full = entry.answer.as_ref().map(|a| a.text.clone());
        v_flex()
            .px_2()
            .py_1()
            .gap_1()
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(theme.mono_font_family.clone())
                            .text_xs()
                            .text_color(theme.primary)
                            .child(format!("› {}", entry.line)),
                    )
                    .when_some(full, |row, full| {
                        row.child(
                            Button::new(("console-copy", ix))
                                .label("Copy")
                                .xsmall()
                                .ghost()
                                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(full.clone()));
                                })),
                        )
                    }),
            )
            .child(div().font_family(theme.mono_font_family.clone()).text_xs().text_color(color).child(text))
    }
}

impl Render for ConsoleView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let log = if self.engine.is_none() {
            message("Open a target to talk to its engine.", false, cx).into_any_element()
        } else if self.entries.is_empty() {
            message("Commands run in this target's session, so `--file` is not needed. Try: guide --brief", false, cx)
                .into_any_element()
        } else {
            let entries: Vec<AnyElement> =
                self.entries.iter().enumerate().map(|(ix, e)| self.render_entry(ix, e, cx).into_any_element()).collect();
            div()
                .id("console-log")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .children(entries)
                .into_any_element()
        };
        let theme = cx.theme();
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_history_previous))
            .on_action(cx.listener(Self::on_history_next))
            .child(header("Console", cx))
            .child(div().flex_1().min_h_0().child(log))
            .child(
                div()
                    .key_context(KEY_CONTEXT)
                    .px_2()
                    .py_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(Input::new(&self.input).small()),
            )
    }
}

dock_panel!(ConsoleView, PanelKind::Console);
