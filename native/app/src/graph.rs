// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The control-flow graph of the selected function (`ir build`): blocks as the
//! engine cut them, each edge coloured by the kind the engine gave it. The
//! drawing is laid out by `graph_layout`; nothing here names an edge a loop.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{BuildCfg, Cfg, ClientError, Engine, FunctionEntry};

use crate::assets::AppIcon;
use crate::graph_layout::{self, EdgeIn, Layout, Rect};
use crate::layout::PanelKind;
use crate::nav::{Location, Navigate, hex, parse_va};
use crate::panel::{dock_panel, header, message};

/// Graphs with more blocks are not drawn; the view says how many there are.
pub const MAX_BLOCKS: usize = 400;
/// Instructions shown in a block; the rest are counted.
const MAX_LINES: usize = 8;
/// Block text at zoom 1.
const FONT: f32 = 12.;
const LINE: f32 = 16.;
const PAD: f32 = 8.;
const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 2.5;
const ZOOM_STEP: f32 = 1.2;
/// Space kept around the graph when it is fitted to the view.
const FIT_MARGIN: f32 = 16.;
/// How far the pointer moves before a press on a block is a drag, not a click.
const DRAG_SLOP: f32 = 4.;

/// How an edge is drawn, from the engine's own name for its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeKind {
    True,
    False,
    Jump,
    Fall,
    Exception,
    Other,
}

impl EdgeKind {
    fn from_engine(kind: &str) -> Self {
        match kind {
            "cjmp-true" => Self::True,
            "cjmp-false" => Self::False,
            "jmp" => Self::Jump,
            "fall" => Self::Fall,
            "eh" => Self::Exception,
            _ => Self::Other,
        }
    }

    fn color(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Self::True => theme.success,
            Self::False => theme.danger,
            Self::Jump | Self::Fall => theme.info,
            Self::Exception => theme.warning,
            Self::Other => theme.muted_foreground,
        }
    }
}

/// One edge as the canvas draws it, in layout units.
struct DrawnEdge {
    points: Vec<(f32, f32)>,
    color: Hsla,
    dashed: bool,
}

struct BlockBox {
    start: u64,
    end: u64,
    lines: Vec<SharedString>,
}

struct EdgeDraw {
    from: usize,
    to: usize,
    kind: EdgeKind,
    /// The engine's name, shown in the legend beside the drawn colour.
    engine_kind: String,
}

struct Model {
    function: FunctionEntry,
    blocks: Vec<BlockBox>,
    edges: Vec<EdgeDraw>,
    layout: Layout,
}

enum ViewState {
    Empty,
    NoFunction(u64),
    Loading(String),
    TooBig { name: String, blocks: usize },
    Ready(Box<Model>),
    Failed(String),
}

enum Drag {
    Pan { from: Point<Pixels>, start: Point<Pixels> },
    Block { index: usize, from: Point<Pixels>, start: (f32, f32), moved: bool },
}

pub struct GraphView {
    engine: Option<Arc<Engine>>,
    location: Option<Location>,
    active: bool,
    stale: bool,
    state: ViewState,
    zoom: f32,
    offset: Point<Pixels>,
    /// Fit the graph to the view, until the user pans or zooms.
    auto_fit: bool,
    drag: Option<Drag>,
    /// Blocks the user moved: their top-left in layout units.
    moved: HashMap<usize, (f32, f32)>,
    /// Where the drawing area was last painted, in window coordinates.
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
}

impl EventEmitter<Navigate> for GraphView {}

impl GraphView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            engine: None,
            location: None,
            active: false,
            stale: false,
            state: ViewState::Empty,
            zoom: 1.,
            offset: point(px(0.), px(0.)),
            auto_fit: true,
            drag: None,
            moved: HashMap::new(),
            bounds: Rc::new(Cell::new(None)),
            focus_handle: cx.focus_handle(),
            _request: None,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.location = None;
        self.state = ViewState::Empty;
        self._request = None;
        cx.notify();
    }

    pub fn show(&mut self, location: &Location, window: &mut Window, cx: &mut Context<Self>) {
        let same_function = match (&self.state, &location.function) {
            (ViewState::Ready(m), Some(f)) => &m.function == f,
            _ => false,
        };
        self.location = Some(location.clone());
        if same_function && !self.stale {
            cx.notify();
            return;
        }
        if self.active {
            self.load(window, cx);
        } else {
            self.stale = true;
        }
    }

    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.stale {
            self.load(window, cx);
        }
    }

    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(engine), Some(location)) = (self.engine.clone(), self.location.clone()) else { return };
        self.stale = false;
        let Some(function) = location.function.clone() else {
            self.state = ViewState::NoFunction(location.va);
            self._request = None;
            cx.notify();
            return;
        };
        self.state = ViewState::Loading(function.name.clone());
        cx.notify();
        let pending = engine.send_latest("graph", &BuildCfg { addr: function.va.clone() });
        self._request = Some(cx.spawn_in(window, async move |this, cx| {
            let result = pending.await;
            this.update_in(cx, |view, window, cx| {
                view.state = match result {
                    Ok(cfg) if cfg.blocks.len() > MAX_BLOCKS => ViewState::TooBig { name: function.name.clone(), blocks: cfg.blocks.len() },
                    Ok(cfg) => ViewState::Ready(Box::new(build_model(function, cfg, char_width(window, cx)))),
                    Err(ClientError::Superseded) => return,
                    Err(e) => ViewState::Failed(e.to_string()),
                };
                view.moved.clear();
                view.auto_fit = true;
                cx.notify();
            })
            .ok();
        }));
    }

    fn model(&self) -> Option<&Model> {
        match &self.state {
            ViewState::Ready(m) => Some(m),
            _ => None,
        }
    }

    /// Zoom and offset in force: fitted while auto-fit holds, else the user's.
    fn view_transform(&self) -> (f32, Point<Pixels>) {
        match (self.auto_fit, self.model(), self.bounds.get()) {
            (true, Some(m), Some(b)) => fit(&m.layout, b),
            _ => (self.zoom, self.offset),
        }
    }

    /// The user takes over: keep what is on screen as the starting point.
    fn take_over(&mut self) {
        if self.auto_fit {
            (self.zoom, self.offset) = self.view_transform();
            self.auto_fit = false;
        }
    }

    fn zoom_by(&mut self, factor: f32, around: Option<Point<Pixels>>, cx: &mut Context<Self>) {
        self.take_over();
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // Keep the point under `around` (view coordinates) where it is.
        let pivot = around.unwrap_or_else(|| {
            self.bounds.get().map_or(point(px(0.), px(0.)), |b| point(b.size.width / 2., b.size.height / 2.))
        });
        let ratio = new_zoom / self.zoom;
        self.offset = point(pivot.x - (pivot.x - self.offset.x) * ratio, pivot.y - (pivot.y - self.offset.y) * ratio);
        self.zoom = new_zoom;
        cx.notify();
    }

    fn local(&self, window_pos: Point<Pixels>) -> Point<Pixels> {
        self.bounds.get().map_or(window_pos, |b| window_pos - b.origin)
    }

    fn rect(&self, model: &Model, index: usize) -> Rect {
        let base = model.layout.nodes[index];
        match self.moved.get(&index) {
            Some(&(x, y)) => Rect { x, y, ..base },
            None => base,
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Working in the graph makes it the focused panel (Shift+Esc zooms it).
        self.focus_handle.focus(window, cx);
        self.take_over();
        self.drag = Some(Drag::Pan { from: event.position, start: self.offset });
        cx.notify();
    }

    fn on_block_down(&mut self, index: usize, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
        let Some(model) = self.model() else { return };
        let r = self.rect(model, index);
        self.take_over();
        self.drag = Some(Drag::Block { index, from: event.position, start: (r.x, r.y), moved: false });
        cx.stop_propagation();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Left) {
            self.drag = None;
            return;
        }
        let zoom = self.zoom;
        match &mut self.drag {
            Some(Drag::Pan { from, start }) => {
                self.offset = *start + (event.position - *from);
                cx.notify();
            }
            Some(Drag::Block { index, from, start, moved }) => {
                let delta = event.position - *from;
                if !*moved && delta.x.abs() < px(DRAG_SLOP) && delta.y.abs() < px(DRAG_SLOP) {
                    return;
                }
                *moved = true;
                let to = (start.0 + delta.x.as_f32() / zoom, start.1 + delta.y.as_f32() / zoom);
                let index = *index;
                self.moved.insert(index, to);
                cx.notify();
            }
            None => {}
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        // A press on a block that never moved is a click: go to the block.
        if let Some(Drag::Block { index, moved: false, .. }) = self.drag
            && let Some(start) = self.model().and_then(|m| m.blocks.get(index)).map(|b| b.start)
        {
            cx.emit(Navigate(start));
        }
        self.drag = None;
        cx.notify();
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(LINE)).y.as_f32();
        if delta.abs() < f32::EPSILON {
            return;
        }
        let around = self.local(event.position);
        self.zoom_by(1.0015f32.powf(delta), Some(around), cx);
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let title = match &self.state {
            ViewState::Ready(m) => m.function.name.clone(),
            ViewState::Loading(name) | ViewState::TooBig { name, .. } => name.clone(),
            _ => String::new(),
        };
        let mut bar = header(title, cx);
        if let Some(m) = self.model() {
            let theme = cx.theme();
            // The legend names each kind the way the engine does.
            let mut kinds: Vec<(EdgeKind, &str)> = Vec::new();
            for e in &m.edges {
                if !kinds.iter().any(|(_, k)| *k == e.engine_kind) {
                    kinds.push((e.kind, e.engine_kind.as_str()));
                }
            }
            bar = bar
                .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} blocks · {} edges", m.blocks.len(), m.edges.len())))
                .children(kinds.into_iter().map(|(kind, engine)| {
                    h_flex()
                        .gap_1()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(div().w(px(10.)).h(px(3.)).bg(kind.color(cx)))
                        .child(engine.to_string())
                }))
                .child(Button::new("cfg-zoom-out").label("−").xsmall().ghost().on_click(cx.listener(|v, _: &ClickEvent, _, cx| v.zoom_by(1. / ZOOM_STEP, None, cx))))
                .child(Button::new("cfg-zoom-in").label("+").xsmall().ghost().on_click(cx.listener(|v, _: &ClickEvent, _, cx| v.zoom_by(ZOOM_STEP, None, cx))))
                .child(Button::new("cfg-fit").icon(AppIcon::Fit).xsmall().ghost().tooltip("Fit to view").on_click(cx.listener(|v, _: &ClickEvent, _, cx| {
                    v.auto_fit = true;
                    cx.notify();
                })))
                .when(!self.moved.is_empty(), |bar| {
                    bar.child(Button::new("cfg-relayout").icon(AppIcon::Reset).xsmall().ghost().tooltip("Put moved blocks back").on_click(cx.listener(|v, _: &ClickEvent, _, cx| {
                        v.moved.clear();
                        cx.notify();
                    })))
                });
        }
        bar
    }

    fn render_graph(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(model) = self.model() else { return div().into_any_element() };
        let (zoom, offset) = self.view_transform();
        let theme = cx.theme();
        let selected = self.location.as_ref().map(|l| l.va);
        let view_size = self.bounds.get().map(|b| b.size);

        // Edges, in layout units, then drawn by the canvas.
        let mut lines: Vec<DrawnEdge> = Vec::with_capacity(model.edges.len());
        for (i, e) in model.edges.iter().enumerate() {
            let color = e.kind.color(cx);
            let dashed = e.kind == EdgeKind::Exception;
            let pts = if self.moved.contains_key(&e.from) || self.moved.contains_key(&e.to) {
                let (src, dst) = (self.rect(model, e.from), self.rect(model, e.to));
                let original = &model.layout.edges[i];
                let (sx, tx) = match (original.first(), original.last()) {
                    (Some(a), Some(b)) => (src.x + (a.0 - model.layout.nodes[e.from].x), dst.x + (b.0 - model.layout.nodes[e.to].x)),
                    _ => (src.center_x(), dst.center_x()),
                };
                graph_layout::reroute(&src, sx, &dst, tx)
            } else {
                model.layout.edges[i].clone()
            };
            lines.push(DrawnEdge { points: pts, color, dashed });
        }
        let bounds_cell = Rc::clone(&self.bounds);
        let view = cx.entity_id();
        let edges = canvas(
            move |bounds, _, cx| {
                // The size is known only once laid out. When it changes, draw
                // again: a fitted graph refits, and blocks skipped as off-screen
                // for the old size are built for the new one.
                if bounds_cell.get() != Some(bounds) {
                    bounds_cell.set(Some(bounds));
                    cx.notify(view);
                }
                bounds
            },
            move |bounds, _, window, _| {
                let to_screen = |(x, y): (f32, f32)| point(bounds.origin.x + offset.x + px(x * zoom), bounds.origin.y + offset.y + px(y * zoom));
                for DrawnEdge { points: pts, color, dashed } in &lines {
                    if pts.len() < 2 {
                        continue;
                    }
                    let mut path = PathBuilder::stroke(px((1.5 * zoom).clamp(1., 2.5)));
                    if *dashed {
                        path = path.dash_array(&[px(5. * zoom), px(4. * zoom)]);
                    }
                    path.move_to(to_screen(pts[0]));
                    for &p in &pts[1..] {
                        path.line_to(to_screen(p));
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, *color);
                    }
                    // Every route enters its target from above: the head points down.
                    let tip = to_screen(pts[pts.len() - 1]);
                    let (w, h) = (px(4.5 * zoom.max(0.5)), px(7. * zoom.max(0.5)));
                    let mut head = PathBuilder::fill();
                    head.move_to(tip);
                    head.line_to(point(tip.x - w, tip.y - h));
                    head.line_to(point(tip.x + w, tip.y - h));
                    head.close();
                    if let Ok(head) = head.build() {
                        window.paint_path(head, *color);
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let mut blocks: Vec<AnyElement> = Vec::new();
        for (index, block) in model.blocks.iter().enumerate() {
            let r = self.rect(model, index);
            let (x, y, w, h) = (offset.x + px(r.x * zoom), offset.y + px(r.y * zoom), px(r.w * zoom), px(r.h * zoom));
            // Off-screen blocks are not built at all.
            if let Some(size) = view_size
                && (x > size.width || y > size.height || x + w < px(0.) || y + h < px(0.))
            {
                continue;
            }
            let here = selected.is_some_and(|va| va >= block.start && va < block.end);
            let mut body = v_flex()
                .id(("cfg-block", index))
                .absolute()
                .left(x)
                .top(y)
                .w(w)
                .h(h)
                .p(px(PAD * zoom))
                .overflow_hidden()
                .rounded(px(4. * zoom))
                .border_1()
                .border_color(if here { theme.primary } else { theme.border })
                .bg(theme.popover)
                .font_family(theme.mono_font_family.clone())
                .text_size(px(FONT * zoom))
                .line_height(px(LINE * zoom))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, e: &MouseDownEvent, window, cx| view.on_block_down(index, e, window, cx)));
            for (i, line) in block.lines.iter().enumerate() {
                let muted = i == 0 || line.starts_with('+') || line.starts_with('→');
                body = body.child(
                    div()
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_color(if muted { theme.muted_foreground } else { theme.foreground })
                        .child(line.clone()),
                );
            }
            blocks.push(body.into_any_element());
        }

        div()
            .id("cfg-area")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(theme.background)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(edges)
            .children(blocks)
            .into_any_element()
    }
}

/// The width of one character of the code font at the block text size.
fn char_width(window: &mut Window, cx: &App) -> f32 {
    let font = font(cx.theme().mono_font_family.clone());
    let id = window.text_system().resolve_font(&font);
    window.text_system().advance(id, px(FONT), 'm').map_or(FONT * 0.6, |size| size.width.as_f32())
}

/// Turn the engine's answer into what is drawn: each block's text, the edges
/// between blocks of this function, and the layout.
fn build_model(function: FunctionEntry, cfg: Cfg, char_w: f32) -> Model {
    let starts: HashMap<u64, usize> =
        cfg.blocks.iter().enumerate().filter_map(|(i, b)| parse_va(&b.start).map(|va| (va, i))).collect();
    let mut blocks = Vec::with_capacity(cfg.blocks.len());
    let mut edges = Vec::new();
    for (i, b) in cfg.blocks.iter().enumerate() {
        let mut lines: Vec<SharedString> = vec![b.start.clone().into()];
        for insn in b.insns.iter().take(MAX_LINES) {
            lines.push(insn.text.clone().into());
        }
        if b.insns.len() > MAX_LINES {
            lines.push(format!("+{} more", b.insns.len() - MAX_LINES).into());
        }
        for s in &b.successors {
            match parse_va(&s.to).and_then(|to| starts.get(&to)) {
                Some(&to) => edges.push(EdgeDraw { from: i, to, kind: EdgeKind::from_engine(&s.kind), engine_kind: s.kind.clone() }),
                // A successor that starts no block of this function is said in
                // the block, not drawn as an edge to nowhere.
                None => lines.push(format!("→ {} ({}, outside this function)", s.to, s.kind).into()),
            }
        }
        blocks.push(BlockBox {
            start: parse_va(&b.start).unwrap_or(0),
            end: parse_va(&b.end).unwrap_or(0),
            lines,
        });
    }
    let sizes: Vec<(f32, f32)> = blocks
        .iter()
        .map(|b| {
            let chars = b.lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
            (chars * char_w + 2. * PAD + 2., b.lines.len() as f32 * LINE + 2. * PAD + 2.)
        })
        .collect();
    let entry = parse_va(&function.va).and_then(|va| starts.get(&va).copied()).unwrap_or(0);
    let layout = graph_layout::layout(&sizes, &edges.iter().map(|e| EdgeIn { from: e.from, to: e.to }).collect::<Vec<_>>(), entry);
    Model { function, blocks, edges, layout }
}

/// The zoom and offset that show the whole graph in `bounds`.
fn fit(layout: &Layout, bounds: Bounds<Pixels>) -> (f32, Point<Pixels>) {
    let (w, h) = (bounds.size.width.as_f32(), bounds.size.height.as_f32());
    if layout.width <= 0. || layout.height <= 0. {
        return (1., point(px(0.), px(0.)));
    }
    let zoom = ((w - 2. * FIT_MARGIN) / layout.width).min((h - 2. * FIT_MARGIN) / layout.height).clamp(MIN_ZOOM, 1.);
    let x = (w - layout.width * zoom) / 2.;
    // A graph taller than the view even at the least zoom starts at its entry, at the top.
    let y = if layout.height * zoom > h - 2. * FIT_MARGIN { FIT_MARGIN } else { (h - layout.height * zoom) / 2. };
    (zoom, point(px(x), px(y)))
}

impl Render for GraphView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.state {
            ViewState::Empty => message("Select a function to draw its control flow.", false, cx).into_any_element(),
            ViewState::NoFunction(va) => {
                message(format!("No listed function covers {}, so there is no graph to draw.", hex(*va)), false, cx).into_any_element()
            }
            ViewState::Loading(name) => message(format!("Building the graph of {name}…"), false, cx).into_any_element(),
            ViewState::TooBig { blocks, .. } => message(
                format!("This function has {blocks} blocks; graphs over {MAX_BLOCKS} are not drawn. The disassembly shows all of it."),
                false,
                cx,
            )
            .into_any_element(),
            ViewState::Failed(e) => message(format!("No graph: {e}"), true, cx).into_any_element(),
            ViewState::Ready(_) => self.render_graph(cx),
        };
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(self.render_header(cx))
            .child(div().flex_1().min_h_0().child(body))
    }
}

dock_panel!(GraphView, PanelKind::Graph, on_active);

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{EdgeKind, build_model};
    use n0xis_client::{Block, Cfg, CfgInstruction, Edge, FunctionEntry};

    fn block(id: u32, start: &str, end: &str, succ: &[(&str, &str)]) -> Block {
        Block {
            id,
            start: start.into(),
            end: end.into(),
            terminator: "cjmp".into(),
            successors: succ.iter().map(|(to, kind)| Edge { to: (*to).into(), kind: (*kind).into(), confidence: 1. }).collect(),
            insns: vec![CfgInstruction { va: start.into(), mnemonic: "nop".into(), text: "nop".into() }],
        }
    }

    #[test]
    fn edges_join_blocks_of_the_function_and_the_rest_are_said_in_the_block() {
        let cfg = Cfg {
            start: "0x10".into(),
            end: "0x40".into(),
            blocks: vec![
                block(0, "0x10", "0x20", &[("0x20", "cjmp-true"), ("0x30", "cjmp-false")]),
                block(1, "0x20", "0x30", &[("0x9000", "jmp")]),
                block(2, "0x30", "0x40", &[]),
            ],
            callsites: Vec::new(),
        };
        let f = FunctionEntry { name: "f".into(), va: "0x10".into(), end: Some("0x40".into()) };
        let m = build_model(f, cfg, 7.);
        assert_eq!(m.edges.len(), 2, "the jump out of the function is not an edge");
        assert_eq!(m.edges[0].kind, EdgeKind::True);
        assert_eq!(m.edges[1].kind, EdgeKind::False);
        assert!(m.blocks[1].lines.iter().any(|l| l.contains("0x9000") && l.contains("outside")), "{:?}", m.blocks[1].lines);
        assert_eq!(m.layout.nodes.len(), 3);
    }

    #[test]
    fn an_edge_kind_the_view_does_not_know_keeps_the_engines_name() {
        assert_eq!(EdgeKind::from_engine("indirect-maybe"), EdgeKind::Other);
        assert_eq!(EdgeKind::from_engine("eh"), EdgeKind::Exception);
    }
}
