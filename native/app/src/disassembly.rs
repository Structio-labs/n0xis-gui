// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The disassembly of the selected function, from its start to its end.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{ClientError, Disassemble, Engine, FunctionEntry, Instruction};

const ROW_HEIGHT: Pixels = px(20.);

/// Instructions asked for when a function's end is not known.
const UNKNOWN_EXTENT_COUNT: u32 = 200;
/// Bounds on the request derived from a known extent.
const MIN_COUNT: u32 = 16;
const MAX_COUNT: u32 = 4000;

enum ViewState {
    Empty,
    Loading,
    Ready,
    Failed(String),
}

pub struct DisassemblyView {
    engine: Option<Arc<Engine>>,
    function: Option<FunctionEntry>,
    insns: Vec<Instruction>,
    state: ViewState,
    scroll: UniformListScrollHandle,
    _request: Option<Task<()>>,
}

fn parse_va(s: &str) -> Option<u64> {
    u64::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
}

/// How many instructions to ask for, and where the function stops. Instructions
/// average a few bytes, so half the byte length is a safe over-ask; the answer
/// is then cut at the end, so padding after `ret` is never shown as code.
fn plan(function: &FunctionEntry) -> (u32, Option<u64>) {
    let start = parse_va(&function.va);
    let end = function.end.as_deref().and_then(parse_va);
    match (start, end) {
        (Some(s), Some(e)) if e > s => {
            let count = u32::try_from((e - s).div_ceil(2)).unwrap_or(MAX_COUNT).clamp(MIN_COUNT, MAX_COUNT);
            (count, Some(e))
        }
        _ => (UNKNOWN_EXTENT_COUNT, None),
    }
}

impl DisassemblyView {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            engine: None,
            function: None,
            insns: Vec::new(),
            state: ViewState::Empty,
            scroll: UniformListScrollHandle::new(),
            _request: None,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.function = None;
        self.insns.clear();
        self.state = ViewState::Empty;
        self._request = None;
        cx.notify();
    }

    pub fn show(&mut self, function: FunctionEntry, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        let (count, end) = plan(&function);
        self.function = Some(function.clone());
        self.state = ViewState::Loading;
        cx.notify();
        let pending = engine.send_latest("disassembly", &Disassemble { addr: function.va, count });
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                match result {
                    Ok(d) => {
                        view.insns = d
                            .insns
                            .into_iter()
                            .take_while(|i| end.is_none_or(|e| parse_va(&i.va).is_some_and(|va| va < e)))
                            .collect();
                        view.state = ViewState::Ready;
                        view.scroll.scroll_to_item(0, ScrollStrategy::Top);
                    }
                    Err(ClientError::Superseded) => return,
                    Err(e) => view.state = ViewState::Failed(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn render_row(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(insn) = self.insns.get(row) else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        let operands = insn.text.strip_prefix(insn.mnemonic.as_str()).unwrap_or(&insn.text).trim().to_string();
        h_flex()
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .font_family(theme.mono_font_family.clone())
            .text_xs()
            .child(div().w(px(150.)).flex_none().text_color(theme.muted_foreground).child(insn.va.clone()))
            .child(div().w(px(180.)).flex_none().truncate().text_color(theme.muted_foreground).child(insn.bytes.clone()))
            .child(div().w(px(70.)).flex_none().text_color(theme.primary).child(insn.mnemonic.clone()))
            .child(div().flex_1().min_w_0().truncate().child(operands))
            .into_any_element()
    }
}

impl Render for DisassemblyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let header = h_flex()
            .px_2()
            .py_1()
            .gap_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("Disassembly"))
            .child(div().flex_1())
            .child(div().text_xs().text_color(theme.muted_foreground).child(match &self.state {
                ViewState::Ready => format!("{} instructions", self.insns.len()),
                ViewState::Loading => "disassembling…".into(),
                _ => String::new(),
            }));
        let message = |text: String, danger: bool| {
            div()
                .p_4()
                .text_sm()
                .text_color(if danger { theme.danger } else { theme.muted_foreground })
                .child(text)
                .into_any_element()
        };
        let body = match &self.state {
            ViewState::Empty => message("Select a function to disassemble it.".into(), false),
            ViewState::Loading => message("Disassembling…".into(), false),
            ViewState::Failed(e) => message(format!("No disassembly: {e}"), true),
            ViewState::Ready => uniform_list(
                "disassembly",
                self.insns.len(),
                cx.processor(|this, range: Range<usize>, _window, cx| {
                    range.map(|row| this.render_row(row, cx)).collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element(),
        };
        v_flex().size_full().child(header).child(div().flex_1().min_h_0().child(body))
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{MAX_COUNT, MIN_COUNT, UNKNOWN_EXTENT_COUNT, plan};
    use n0xis_client::FunctionEntry;

    fn f(va: &str, end: Option<&str>) -> FunctionEntry {
        FunctionEntry { name: "f".into(), va: va.into(), end: end.map(Into::into) }
    }

    #[test]
    fn a_known_extent_bounds_the_request_and_the_answer() {
        assert_eq!(plan(&f("0x1000", Some("0x1040"))), (32, Some(0x1040)));
        assert_eq!(plan(&f("0x1000", Some("0x1001"))), (MIN_COUNT, Some(0x1001)));
        assert_eq!(plan(&f("0x1000", Some("0x101000"))), (MAX_COUNT, Some(0x101000)));
    }

    #[test]
    fn an_unknown_or_impossible_extent_falls_back_to_a_fixed_count() {
        assert_eq!(plan(&f("0x1000", None)), (UNKNOWN_EXTENT_COUNT, None));
        assert_eq!(plan(&f("0x1000", Some("0x0ff0"))), (UNKNOWN_EXTENT_COUNT, None));
        assert_eq!(plan(&f("garbage", Some("0x10"))), (UNKNOWN_EXTENT_COUNT, None));
    }
}
