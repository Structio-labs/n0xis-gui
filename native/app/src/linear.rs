// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The linear listing: the whole program as one listing, function after
//! function in address order. Each function is cut at the end its list entry
//! states, or, where it states none, at the next function's start, and its
//! header says which. Functions are fetched as they come into view and dropped
//! again far from it; the minimap spans the whole address range.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::Selectable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, input, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{ClientError, DecompStyle, Decompile, Disassemble, Engine, FunctionEntry};

use crate::layout::{PanelKind, Views};
use crate::nav::{Location, Navigate, hex, parse_va};
use crate::panel::{dock_panel, header, message};

/// Requests in the engine's queue at once; more wait their turn.
pub const MAX_IN_FLIGHT: usize = 6;
/// Functions kept with their text; the ones farthest from view go first.
pub const MAX_LOADED: usize = 1500;
/// Functions around the top of the view that are worth fetching.
const AHEAD: usize = 48;
const BEHIND: usize = 4;
const ROW: Pixels = px(20.);
const MINIMAP_WIDTH: Pixels = px(14.);
/// Instructions asked for when neither an end nor a next function is known.
const UNBOUNDED_COUNT: u32 = 200;
const MIN_COUNT: u32 = 16;
const MAX_COUNT: u32 = 4000;

/// The key context the listing sits in, for its copy binding.
pub const KEY_CONTEXT: &str = "Linear";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Asm,
    Pseudo,
}

#[derive(Clone)]
struct Line {
    va: Option<u64>,
    bytes: SharedString,
    mnemonic: SharedString,
    text: SharedString,
}

enum Body {
    Lines(Vec<Line>),
    Failed(String),
}

/// A line of the listing: the function it is in (by start) and its place.
/// Ordered by address, then line, which is the listing's own order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Cursor {
    start: u64,
    line: usize,
}

/// Where a function's text stops, and how that was known.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Extent {
    /// The end the function list states.
    Stated(u64),
    /// No end stated: up to where the next listed function starts.
    NextFunction(u64),
    Unknown,
}

impl Extent {
    fn end(self) -> Option<u64> {
        match self {
            Self::Stated(e) | Self::NextFunction(e) => Some(e),
            Self::Unknown => None,
        }
    }
}

fn extent(function: &FunctionEntry, start: u64, next_start: Option<u64>) -> Extent {
    match function.end.as_deref().and_then(parse_va) {
        Some(end) if end > start => Extent::Stated(end),
        _ => match next_start {
            Some(next) if next > start => Extent::NextFunction(next),
            _ => Extent::Unknown,
        },
    }
}

/// How many instructions to ask for: half the byte length over-asks safely,
/// and the answer is cut at the end anyway.
fn count_for(start: u64, extent: Extent) -> u32 {
    match extent.end() {
        Some(end) => u32::try_from((end - start).div_ceil(2)).unwrap_or(MAX_COUNT).clamp(MIN_COUNT, MAX_COUNT),
        None => UNBOUNDED_COUNT,
    }
}

/// Minimap rows, built once per state of the listing rather than per frame.
#[derive(Default)]
struct MinimapCache {
    key: (usize, usize, u64, u32),
    /// Per pixel row: 0 nothing, 1 code, 2 fetched, 3 named.
    rows: Vec<u8>,
    span: (u64, u64),
}

pub struct LinearView {
    engine: Option<Arc<Engine>>,
    mode: Mode,
    list: ListState,
    /// Items the list holds: functions with an address, in address order.
    count: usize,
    order_generation: u64,
    bodies: HashMap<u64, Body>,
    in_flight: HashSet<u64>,
    queue: VecDeque<u64>,
    /// Bumped when what was asked for no longer applies (new mode, new target).
    generation: u64,
    selection: Option<(Cursor, Cursor)>,
    location: Option<Location>,
    active: bool,
    stale: bool,
    minimap: Rc<RefCell<MinimapCache>>,
    minimap_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    minimap_drag: bool,
    /// A send of queued requests is scheduled for after this frame.
    pump_scheduled: bool,
    focus_handle: FocusHandle,
}

impl EventEmitter<Navigate> for LinearView {}

impl LinearView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            engine: None,
            mode: Mode::Asm,
            list: ListState::new(0, ListAlignment::Top, px(600.)),
            count: 0,
            order_generation: 0,
            bodies: HashMap::new(),
            in_flight: HashSet::new(),
            queue: VecDeque::new(),
            generation: 0,
            selection: None,
            location: None,
            active: false,
            stale: false,
            minimap: Rc::new(RefCell::new(MinimapCache::default())),
            minimap_bounds: Rc::new(Cell::new(None)),
            minimap_drag: false,
            pump_scheduled: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.forget(cx);
        self.count = 0;
        self.list.reset(0);
        self.selection = None;
        self.location = None;
        cx.notify();
    }

    /// Fetch every shown function again: a name or a comment in them changed.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.forget(cx);
    }

    /// Drop every fetched function and every pending request.
    fn forget(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        self.bodies.clear();
        self.in_flight.clear();
        self.queue.clear();
        self.list.remeasure();
        cx.notify();
    }

    /// Follow the function list as its pages arrive.
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        let (n, order) = {
            let functions = cx.global::<Views>().functions.read(cx);
            (functions.index().addressed_len(), functions.index().order_generation())
        };
        if order != self.order_generation || n < self.count {
            self.order_generation = order;
            self.list.reset_with_uniform_height(n, ROW * 8.);
        } else if n > self.count {
            if self.count == 0 {
                self.list.reset_with_uniform_height(n, ROW * 8.);
            } else {
                self.list.splice(self.count..self.count, n - self.count);
            }
        }
        self.count = n;
        cx.notify();
    }

    pub fn show(&mut self, location: &Location, cx: &mut Context<Self>) {
        self.location = Some(location.clone());
        if self.active {
            self.reveal(cx);
        } else {
            self.stale = true;
        }
        cx.notify();
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.stale {
            self.reveal(cx);
        }
    }

    /// Scroll the selected function into view, unless it is in view already.
    fn reveal(&mut self, cx: &mut Context<Self>) {
        self.stale = false;
        let Some(location) = &self.location else { return };
        let rank = cx.global::<Views>().functions.read(cx).index().address_rank(location.va);
        if let Some(rank) = rank {
            let off_screen = self.list.item_is_above_viewport(rank).unwrap_or(true) || self.list.item_is_below_viewport(rank).unwrap_or(true);
            if off_screen {
                self.list.scroll_to(ListOffset { item_ix: rank, offset_in_item: px(0.) });
            }
        }
    }

    fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            self.selection = None;
            self.forget(cx);
        }
    }

    /// Ask for a function's text when it comes into view.
    fn want(&mut self, start: u64) {
        if !self.bodies.contains_key(&start) && !self.in_flight.contains(&start) && !self.queue.contains(&start) {
            self.queue.push_back(start);
        }
    }

    /// Send what waits, a few at a time, skipping what has scrolled away.
    fn pump(&mut self, cx: &mut Context<Self>) {
        let top = self.list.logical_scroll_top().item_ix;
        let wanted = top.saturating_sub(BEHIND)..top + AHEAD;
        while self.in_flight.len() < MAX_IN_FLIGHT {
            let Some(start) = self.queue.pop_front() else { break };
            let lookup = {
                let index = cx.global::<Views>().functions.read(cx).index();
                index.address_rank(start).and_then(|rank| {
                    let (s, f) = index.by_address(rank)?;
                    (s == start).then(|| (rank, f.clone(), index.by_address(rank + 1).map(|(next, _)| next)))
                })
            };
            let Some((rank, function, next_start)) = lookup else { continue };
            if !wanted.contains(&rank) {
                continue;
            }
            self.in_flight.insert(start);
            self.request(start, &function, next_start, cx);
        }
    }

    fn request(&mut self, start: u64, function: &FunctionEntry, next_start: Option<u64>, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        let generation = self.generation;
        match self.mode {
            Mode::Asm => {
                let ext = extent(function, start, next_start);
                let pending = engine.send(&Disassemble { addr: hex(start), count: count_for(start, ext) });
                cx.spawn(async move |this, cx| {
                    let result = pending.await.map(|d| {
                        d.insns
                            .into_iter()
                            .take_while(|i| ext.end().is_none_or(|e| parse_va(&i.va).is_some_and(|va| va < e)))
                            .map(|i| {
                                let operands = i.text.strip_prefix(i.mnemonic.as_str()).unwrap_or(&i.text).trim().to_string();
                                Line { va: parse_va(&i.va), bytes: i.bytes.into(), mnemonic: i.mnemonic.into(), text: operands.into() }
                            })
                            .collect()
                    });
                    this.update(cx, |view, cx| view.take(start, generation, result, cx)).ok();
                })
                .detach();
            }
            Mode::Pseudo => {
                let pending = engine.send(&Decompile { addr: hex(start), style: DecompStyle::Structured });
                cx.spawn(async move |this, cx| {
                    let result = pending.await.map(|d| {
                        d.pseudo
                            .into_iter()
                            .map(|text| Line { va: None, bytes: SharedString::default(), mnemonic: SharedString::default(), text: text.into() })
                            .collect()
                    });
                    this.update(cx, |view, cx| view.take(start, generation, result, cx)).ok();
                })
                .detach();
            }
        }
    }

    fn take(&mut self, start: u64, generation: u64, result: Result<Vec<Line>, ClientError>, cx: &mut Context<Self>) {
        if generation != self.generation {
            return;
        }
        self.in_flight.remove(&start);
        self.bodies.insert(start, match result {
            Ok(lines) => Body::Lines(lines),
            Err(e) => Body::Failed(e.to_string()),
        });
        self.remeasure(start, cx);
        self.evict(cx);
        self.pump(cx);
        cx.notify();
    }

    fn remeasure(&self, start: u64, cx: &App) {
        if let Some(rank) = cx.global::<Views>().functions.read(cx).index().address_rank(start) {
            self.list.remeasure_items(rank..rank + 1);
        }
    }

    /// Keep at most [`MAX_LOADED`] functions, dropping the farthest from view.
    fn evict(&mut self, cx: &mut Context<Self>) {
        if self.bodies.len() <= MAX_LOADED {
            return;
        }
        let top = self.list.logical_scroll_top().item_ix;
        let mut ranked: Vec<(usize, u64)> = {
            let index = cx.global::<Views>().functions.read(cx).index();
            self.bodies.keys().map(|&s| (index.address_rank(s).map_or(usize::MAX, |r| r.abs_diff(top)), s)).collect()
        };
        ranked.sort_unstable_by(|a, b| b.cmp(a));
        let excess = self.bodies.len() - MAX_LOADED;
        for &(_, start) in ranked.iter().take(excess) {
            self.bodies.remove(&start);
            self.remeasure(start, cx);
        }
    }

    fn click(&mut self, at: Cursor, va: Option<u64>, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
        match (shift, self.selection) {
            (true, Some((anchor, _))) => self.selection = Some((anchor, at)),
            _ => {
                self.selection = Some((at, at));
                cx.emit(Navigate(va.unwrap_or(at.start)));
            }
        }
        cx.notify();
    }

    fn selected(&self, at: Cursor) -> bool {
        self.selection.is_some_and(|(a, b)| {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            lo <= at && at <= hi
        })
    }

    /// The selected lines as text, as far as they are fetched.
    fn on_copy(&mut self, _: &input::Copy, _: &mut Window, cx: &mut Context<Self>) {
        let Some((a, b)) = self.selection else { return };
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let mut starts: Vec<&u64> = self.bodies.keys().filter(|&&s| s >= lo.start && s <= hi.start).collect();
        starts.sort_unstable();
        let mut out = String::new();
        for &start in starts {
            let Some(Body::Lines(lines)) = self.bodies.get(&start) else { continue };
            for (i, line) in lines.iter().enumerate() {
                let at = Cursor { start, line: i + 1 };
                if at < lo || at > hi {
                    continue;
                }
                match line.va {
                    Some(va) => out.push_str(&format!("{}  {} {}\n", hex(va), line.mnemonic, line.text)),
                    None => out.push_str(&format!("{}\n", line.text)),
                }
            }
        }
        if !out.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(out));
        }
    }

    fn render_item(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let entry = {
            let index = cx.global::<Views>().functions.read(cx).index();
            index.by_address(ix).map(|(start, f)| (start, f.clone(), index.by_address(ix + 1).map(|(next, _)| next)))
        };
        let Some((start, function, next_start)) = entry else { return div().h(ROW).into_any_element() };
        if !self.bodies.contains_key(&start) {
            self.want(start);
            // Items are built while the list lays itself out, when its state
            // cannot be read; the requests go out once the frame is done.
            if !self.pump_scheduled {
                self.pump_scheduled = true;
                let this = cx.entity().downgrade();
                cx.defer(move |cx| {
                    this.update(cx, |view, cx| {
                        view.pump_scheduled = false;
                        view.pump(cx);
                    })
                    .ok();
                });
            }
        }
        let theme = cx.theme();
        let ext = extent(&function, start, next_start);
        let span = match ext {
            Extent::Stated(end) => format!("{}–{}", hex(start), hex(end)),
            Extent::NextFunction(next) => format!("{}–{} · no stated end, up to the next function", hex(start), hex(next)),
            Extent::Unknown => format!("{} · no stated end", hex(start)),
        };
        let here = self.location.as_ref().map(|l| l.va);
        let header_at = Cursor { start, line: 0 };
        let mut item = v_flex().w_full().child(
            h_flex()
                .id(("linear-header", ix))
                .h(ROW)
                .px_2()
                .gap_3()
                .text_xs()
                .bg(theme.secondary)
                .when(self.selected(header_at), |r| r.bg(theme.list_active))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(function.name.clone()))
                .child(div().text_color(theme.muted_foreground).font_family(theme.mono_font_family.clone()).child(span))
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, e: &MouseDownEvent, window, cx| {
                    view.click(header_at, Some(start), e.modifiers.shift, window, cx)
                })),
        );
        match self.bodies.get(&start) {
            None => {
                item = item.child(div().h(ROW).px_2().text_xs().text_color(theme.muted_foreground).child("…"));
            }
            Some(Body::Failed(e)) => {
                item = item.child(div().h(ROW).px_2().text_xs().text_color(theme.danger).child(format!("Not listed: {e}")));
            }
            Some(Body::Lines(lines)) => {
                for (i, line) in lines.iter().enumerate() {
                    let at = Cursor { start, line: i + 1 };
                    let is_here = line.va.is_some() && line.va == here;
                    let va = line.va;
                    let row = h_flex()
                        .id(("linear-line", i))
                        .h(ROW)
                        .px_2()
                        .gap_3()
                        .font_family(theme.mono_font_family.clone())
                        .text_xs()
                        .when(self.selected(at), |r| r.bg(theme.list_active))
                        .when(is_here && !self.selected(at), |r| r.bg(theme.list_hover));
                    let row = match self.mode {
                        Mode::Asm => row
                            .child(div().w(px(130.)).flex_none().text_color(theme.muted_foreground).child(va.map(hex).unwrap_or_default()))
                            .child(div().w(px(150.)).flex_none().truncate().text_color(theme.muted_foreground).child(line.bytes.clone()))
                            .child(div().w(px(60.)).flex_none().text_color(theme.primary).child(line.mnemonic.clone()))
                            .child(div().flex_1().min_w_0().truncate().child(line.text.clone())),
                        Mode::Pseudo => row.child(div().whitespace_nowrap().child(line.text.clone())),
                    };
                    item = item.child(row.on_mouse_down(MouseButton::Left, cx.listener(move |view, e: &MouseDownEvent, window, cx| {
                        view.click(at, va, e.modifiers.shift, window, cx)
                    })));
                }
            }
        }
        item.into_any_element()
    }

    /// Fill the minimap cache for a strip `height` pixels tall.
    fn minimap_rows(&self, height: u32, cx: &App) {
        let index = cx.global::<Views>().functions.read(cx).index();
        let key = (index.addressed_len(), self.bodies.len(), index.order_generation(), height);
        let mut cache = self.minimap.borrow_mut();
        if cache.key == key || height == 0 {
            return;
        }
        let n = index.addressed_len();
        let (Some((lo, _)), Some((last, last_fn))) = (index.by_address(0), index.by_address(n.saturating_sub(1))) else {
            *cache = MinimapCache { key, ..Default::default() };
            return;
        };
        let hi = last_fn.end.as_deref().and_then(parse_va).filter(|&e| e > last).unwrap_or(last + 1);
        let span = (hi - lo).max(1) as f64;
        let mut rows = vec![0u8; height as usize];
        for rank in 0..n {
            let Some((start, f)) = index.by_address(rank) else { continue };
            let y = (((start - lo) as f64 / span) * f64::from(height)) as usize;
            let value = if !f.name.starts_with("sub_") {
                3
            } else if self.bodies.contains_key(&start) {
                2
            } else {
                1
            };
            if let Some(slot) = rows.get_mut(y.min(height as usize - 1)) {
                *slot = (*slot).max(value);
            }
        }
        *cache = MinimapCache { key, rows, span: (lo, hi) };
    }

    fn minimap_jump(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(bounds) = self.minimap_bounds.get() else { return };
        let (lo, hi) = self.minimap.borrow().span;
        if hi <= lo {
            return;
        }
        let t = ((position.y - bounds.origin.y) / bounds.size.height).clamp(0., 1.);
        let va = lo + ((hi - lo) as f64 * f64::from(t)) as u64;
        if let Some(rank) = cx.global::<Views>().functions.read(cx).index().address_rank(va) {
            self.list.scroll_to(ListOffset { item_ix: rank, offset_in_item: px(0.) });
            cx.notify();
        }
    }

    fn render_minimap(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let colors = [theme.border, theme.muted_foreground, theme.primary];
        let view_color = theme.foreground.opacity(0.12);
        let cache = Rc::clone(&self.minimap);
        let bounds_cell = Rc::clone(&self.minimap_bounds);
        let entity = cx.entity_id();
        // The part of the address range in view, for the marker.
        let top = self.list.logical_scroll_top().item_ix;
        let in_view = {
            let index = cx.global::<Views>().functions.read(cx).index();
            (index.by_address(top).map(|(s, _)| s), index.by_address(top + 30).or_else(|| index.by_address(index.addressed_len().saturating_sub(1))).map(|(s, _)| s))
        };
        canvas(
            move |bounds, _, cx| {
                if bounds_cell.get() != Some(bounds) {
                    bounds_cell.set(Some(bounds));
                    cx.notify(entity);
                }
                bounds
            },
            move |bounds, _, window, _| {
                let cache = cache.borrow();
                for (y, &value) in cache.rows.iter().enumerate() {
                    if value == 0 {
                        continue;
                    }
                    let color = colors[usize::from(value - 1).min(2)];
                    window.paint_quad(fill(
                        Bounds::new(point(bounds.origin.x + px(2.), bounds.origin.y + px(y as f32)), size(bounds.size.width - px(4.), px(1.))),
                        color,
                    ));
                }
                let (lo, hi) = cache.span;
                if let (Some(a), Some(b)) = in_view
                    && hi > lo
                {
                    let h = bounds.size.height.as_f32();
                    let y0 = ((a.saturating_sub(lo)) as f64 / (hi - lo) as f64 * f64::from(h)) as f32;
                    let y1 = ((b.saturating_sub(lo)) as f64 / (hi - lo) as f64 * f64::from(h)) as f32;
                    window.paint_quad(fill(
                        Bounds::new(point(bounds.origin.x, bounds.origin.y + px(y0)), size(bounds.size.width, px((y1 - y0).max(3.)))),
                        view_color,
                    ));
                }
            },
        )
        .size_full()
    }
}

impl Render for LinearView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let modes = [(Mode::Asm, "Asm"), (Mode::Pseudo, "Pseudo")];
        let theme_border = cx.theme().border;
        let mut bar = header("Linear listing", cx);
        bar = bar.children(modes.into_iter().map(|(mode, label)| {
            Button::new(SharedString::from(format!("linear-{label}")))
                .label(label)
                .xsmall()
                .ghost()
                .selected(self.mode == mode)
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.set_mode(mode, cx)))
        }));
        let body = if self.engine.is_none() {
            message("Open a target to list it.", false, cx).into_any_element()
        } else if self.count == 0 {
            message("Waiting for the function list…", false, cx).into_any_element()
        } else {
            if let Some(height) = self.minimap_bounds.get().map(|b| b.size.height.as_f32() as u32) {
                self.minimap_rows(height, cx);
            }
            h_flex()
                .size_full()
                .child(
                    list(self.list.clone(), cx.processor(|view, ix, window, cx| view.render_item(ix, window, cx)))
                        .flex_1()
                        .h_full(),
                )
                .child(
                    div()
                        .id("linear-minimap")
                        .w(MINIMAP_WIDTH)
                        .h_full()
                        .flex_none()
                        .border_l_1()
                        .border_color(theme_border)
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(|view, e: &MouseDownEvent, _, cx| {
                            view.minimap_drag = true;
                            view.minimap_jump(e.position, cx);
                        }))
                        .on_mouse_move(cx.listener(|view, e: &MouseMoveEvent, _, cx| {
                            if view.minimap_drag && e.pressed_button == Some(MouseButton::Left) {
                                view.minimap_jump(e.position, cx);
                            } else {
                                view.minimap_drag = false;
                            }
                        }))
                        .on_mouse_up(MouseButton::Left, cx.listener(|view, _: &MouseUpEvent, _, _| view.minimap_drag = false))
                        .child(self.render_minimap(cx)),
                )
                .into_any_element()
        };
        v_flex()
            .size_full()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .child(bar)
            .child(div().flex_1().min_h_0().child(body))
    }
}

dock_panel!(LinearView, PanelKind::Linear, on_active);

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{Cursor, Extent, MAX_COUNT, MIN_COUNT, UNBOUNDED_COUNT, count_for, extent};
    use n0xis_client::FunctionEntry;

    fn f(end: Option<&str>) -> FunctionEntry {
        FunctionEntry { name: "f".into(), va: "0x1000".into(), end: end.map(Into::into) }
    }

    #[test]
    fn a_function_stops_at_its_stated_end_or_says_it_reached_the_next() {
        assert_eq!(extent(&f(Some("0x1040")), 0x1000, Some(0x2000)), Extent::Stated(0x1040));
        assert_eq!(extent(&f(None), 0x1000, Some(0x1080)), Extent::NextFunction(0x1080));
        assert_eq!(extent(&f(None), 0x1000, None), Extent::Unknown);
        assert_eq!(extent(&f(Some("0x0f00")), 0x1000, None), Extent::Unknown, "an end before the start is no end");
    }

    #[test]
    fn the_request_is_bounded_by_the_extent() {
        assert_eq!(count_for(0x1000, Extent::Stated(0x1040)), 32);
        assert_eq!(count_for(0x1000, Extent::NextFunction(0x1001)), MIN_COUNT);
        assert_eq!(count_for(0x1000, Extent::Stated(0x10_0000)), MAX_COUNT);
        assert_eq!(count_for(0x1000, Extent::Unknown), UNBOUNDED_COUNT);
    }

    #[test]
    fn lines_are_ordered_by_address_then_by_place() {
        let a = Cursor { start: 0x1000, line: 9 };
        let b = Cursor { start: 0x2000, line: 0 };
        assert!(a < b);
        assert!(Cursor { start: 0x1000, line: 1 } < a);
    }
}
