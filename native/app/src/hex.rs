// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The bytes around the selection (`mem span`): address, sixteen bytes a row,
//! and the same bytes as text. The selected address is marked. Bytes are shown
//! exactly as the engine read them; an address it cannot read (unmapped, or a
//! zero-fill tail) shows as `··`, and the window goes on past it.

use std::sync::Arc;

use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::*;
use n0xis_client::{ClientError, Engine, MemSpan, Span};

use crate::context;
use crate::layout::PanelKind;
use crate::nav::{Location, hex};
use crate::panel::{dock_panel, header, message};

/// How many bytes one window shows.
const WINDOW: u64 = 1024;
const ROW_BYTES: u64 = 16;
/// Rows of context above the selection when a window opens on it.
const LEAD_ROWS: u64 = 4;
const ROW_HEIGHT: Pixels = px(20.);
/// What a cell shows where the engine has no byte.
const NO_BYTE: &str = "··";

enum Load {
    Empty,
    Loading,
    Ready(Arc<Span>),
    Failed(String),
}

pub struct HexView {
    engine: Option<Arc<Engine>>,
    /// The address to mark.
    focus: Option<u64>,
    /// Where the window starts: always a whole row.
    start: u64,
    /// The row to bring into view once the window arrives.
    scroll_row: usize,
    load: Load,
    active: bool,
    stale: bool,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
}

/// The window that shows `va` with a few rows above it.
fn window_for(va: u64) -> u64 {
    (va & !(ROW_BYTES - 1)).saturating_sub(LEAD_ROWS * ROW_BYTES)
}

impl HexView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            engine: None,
            focus: None,
            start: 0,
            scroll_row: 0,
            load: Load::Empty,
            active: false,
            stale: false,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _request: None,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.focus = None;
        self.load = Load::Empty;
        self.stale = false;
        self._request = None;
        cx.notify();
    }

    /// Show the bytes around `location`. A selection already in the window
    /// only moves the mark.
    pub fn show(&mut self, location: &Location, cx: &mut Context<Self>) {
        self.focus = Some(location.va);
        if let Load::Ready(span) = &self.load
            && span.contains(location.va)
        {
            let row = ((location.va - span.start) / ROW_BYTES) as usize;
            self.scroll.scroll_to_item(row, ScrollStrategy::Center);
            cx.notify();
            return;
        }
        self.start = window_for(location.va);
        self.scroll_row = ((location.va - self.start) / ROW_BYTES) as usize;
        self.request(cx);
    }

    fn page(&mut self, forward: bool, cx: &mut Context<Self>) {
        self.start = if forward { self.start.saturating_add(WINDOW) } else { self.start.saturating_sub(WINDOW) };
        self.scroll_row = 0;
        self.request(cx);
    }

    fn request(&mut self, cx: &mut Context<Self>) {
        if !self.active {
            self.stale = true;
            return;
        }
        let Some(engine) = self.engine.clone() else { return };
        self.stale = false;
        self.load = Load::Loading;
        cx.notify();
        let pending = engine.send_latest("hex", &MemSpan { addr: hex(self.start), size: WINDOW as u32 });
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                view.load = match result {
                    Ok(span) => Load::Ready(Arc::new(span)),
                    Err(ClientError::Superseded) => return,
                    Err(e) => Load::Failed(e.to_string()),
                };
                view.scroll.scroll_to_item(view.scroll_row, ScrollStrategy::Top);
                cx.notify();
            })
            .ok();
        }));
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.stale {
            self.request(cx);
        }
    }

    fn render_row(&self, span: &Span, row: usize, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let base = span.start + row as u64 * ROW_BYTES;
        let addresses = base..base + ROW_BYTES;
        let marked = self.focus.filter(|f| addresses.contains(f));
        let cells = addresses.clone().map(|va| {
            let byte = span.byte_at(va);
            div()
                .w(px(22.))
                .flex_none()
                .when(byte.is_none(), |d| d.text_color(theme.muted_foreground))
                .when(marked == Some(va), |d| d.bg(theme.list_active).text_color(theme.foreground))
                .child(byte.map_or_else(|| NO_BYTE.to_string(), |b| format!("{b:02x}")))
        });
        let text: String = addresses
            .clone()
            .map(|va| match span.byte_at(va) {
                Some(b) if (0x20..0x7f).contains(&b) => b as char,
                Some(_) => '.',
                None => ' ',
            })
            .collect();
        // What a copy of the row gives: the bytes that are there, a gap as the view shows it.
        let bytes: Vec<String> = addresses.map(|va| span.byte_at(va).map_or_else(|| NO_BYTE.to_string(), |b| format!("{b:02x}"))).collect();
        let (bytes, copied_text) = (bytes.join(" "), text.clone());
        let line = h_flex()
            .id(("hex-row", row))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .font_family(theme.mono_font_family.clone())
            .text_xs()
            .when(marked.is_some(), |r| r.bg(theme.list_hover))
            .child(div().w(px(130.)).flex_none().text_color(theme.muted_foreground).child(hex(base)))
            .child(h_flex().gap_0p5().children(cells))
            .child(div().flex_none().text_color(theme.muted_foreground).child(text));
        context::row_with_menu(("hex-row-menu", row), line)
            .context_menu(move |menu, _, _| {
                let menu = context::go(menu.label(hex(base)), "Go To", base);
                let menu = context::views(menu, base, Some(PanelKind::Hex)).separator();
                let menu = context::annotations(menu, base).separator();
                let menu = context::copy(menu, "Copy Bytes", bytes.clone());
                let menu = context::copy(menu, "Copy Text", copied_text.trim_end().to_string());
                context::copy(menu, "Copy Address", hex(base))
            })
            .into_any_element()
    }
}

impl Render for HexView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = match &self.load {
            Load::Ready(span) if span.read() == span.bytes.len() => format!("{} bytes from {}", span.bytes.len(), hex(span.start)),
            Load::Ready(span) if span.read() == 0 => format!("none of {} bytes from {} can be read", span.bytes.len(), hex(span.start)),
            Load::Ready(span) => format!("{} of {} bytes from {} can be read; {NO_BYTE} where not", span.read(), span.bytes.len(), hex(span.start)),
            _ => String::new(),
        };
        let body = match &self.load {
            Load::Empty => message("Select an address to see its bytes.", false, cx).into_any_element(),
            Load::Loading => message("Reading…", false, cx).into_any_element(),
            Load::Failed(e) => message(format!("The bytes could not be read: {e}"), true, cx).into_any_element(),
            Load::Ready(span) => {
                let span = Arc::clone(span);
                let rows = span.bytes.len().div_ceil(ROW_BYTES as usize);
                uniform_list(
                    "hex",
                    rows,
                    cx.processor(move |view, range: std::ops::Range<usize>, _, cx| {
                        range.map(|row| view.render_row(&span, row, cx)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.scroll)
                .flex_1()
                .into_any_element()
            }
        };
        let paging = self.engine.is_some() && matches!(self.load, Load::Ready(_) | Load::Failed(_));
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(
                header("Hex", cx)
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child(note))
                    .when(paging, |h| {
                        h.child(
                            Button::new("hex-earlier")
                                .xsmall()
                                .ghost()
                                .label("Earlier")
                                .disabled(self.start == 0)
                                .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.page(false, cx))),
                        )
                        .child(
                            Button::new("hex-later")
                                .xsmall()
                                .ghost()
                                .label("Later")
                                .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.page(true, cx))),
                        )
                    }),
            )
            .child(body)
    }
}

dock_panel!(HexView, PanelKind::Hex, on_active);

#[cfg(test)]
mod tests {
    use super::{LEAD_ROWS, ROW_BYTES, window_for};

    #[test]
    fn a_window_starts_on_a_row_with_context_above_the_selection() {
        assert_eq!(window_for(0x1047), 0x1000);
        assert_eq!(window_for(0x1047) % ROW_BYTES, 0);
        assert_eq!(window_for(0x10), 0, "no window starts below address zero");
        assert_eq!((0x1047 - window_for(0x1047)) / ROW_BYTES, LEAD_ROWS);
    }
}
