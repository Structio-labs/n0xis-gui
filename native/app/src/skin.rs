// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! How the dock looks. GPUI Kit's skin draws the area, the splits and the
//! content of a group; this one draws each group's tabs, which run along any
//! side of the group, as icons or with names, and answer a right click.
//!
//! Where a group's tabs run is the group's own setting ([`Strip`]). While the
//! app runs it is kept by the group's node ([`Strips`]); in a snapshot and on
//! disk by the names of the group's panels ([`SavedStrip`]), because a load
//! gives every group a new node while each panel is in one group only.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::{Placement, ResizeHandleContext};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dock::{
    AnyDrag, BasePanelView, DockArea, DockAreaRenderer, DockContext, DockPlacement, DockSkin, DragPanel, DropIndicator,
    NodeId, PaneRef, PanelId, PanelState, TabGroupContext, TabGroupRenderer,
};
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::assets::AppIcon;
use crate::layout::{PanelKind, ReplacePanel};

/// How tall a strip of tabs across a group is, and the header above a group
/// whose tabs run along another side.
const BAR_HEIGHT: Pixels = px(30.);
/// How wide a strip down a side is: icons alone, and icons with names.
const ICON_STRIP: Pixels = px(32.);
const NAME_STRIP: Pixels = px(150.);
/// What follows the pointer while a tab is dragged.
const DRAG_PREVIEW: Size<Pixels> = size(px(140.), px(28.));
/// The most a tab's name takes on a strip across a group; a longer one is cut.
const TAB_NAME_WIDTH: Pixels = px(130.);

/// Which side of its group a tab strip runs along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub const ALL: [Side; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    pub fn title(self) -> &'static str {
        match self {
            Self::Top => "Top",
            Self::Bottom => "Bottom",
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }

    /// A strip down the left or right side, rather than across the group.
    pub fn runs_down(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// The side of the dock's drop zone.
    pub fn of(placement: Placement) -> Self {
        match placement {
            Placement::Top => Self::Top,
            Placement::Bottom => Self::Bottom,
            Placement::Left => Self::Left,
            Placement::Right => Self::Right,
        }
    }

    fn lower(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// What a panel dropped on a group's content does: the gestures of the Tauri
/// build's dock. Near an edge it splits the group; over the centre it takes
/// the group's place; with Ctrl held it joins the group as a tab, and the
/// side it was dropped at is where the group's tabs go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropMode {
    /// A group of its own beside this one, on that side.
    Split(Side),
    /// The only panel of this group: the ones it held are closed (they stay
    /// in View ▸ Panels, and layout undo brings them back).
    Replace,
    /// One more tab of this group, with the group's tabs moved to that side.
    Tab(Side),
    /// Dropped on its own group where that would change nothing.
    Cancel,
}

impl DropMode {
    /// `placement` is the zone the dock resolved (`None`: the centre); `own`
    /// whether the panel comes from this group, and `alone` whether it is the
    /// only one there. A panel can split out of its own group only when the
    /// group holds more than it, as in the Tauri build.
    pub fn of(placement: Option<Placement>, ctrl: bool, own: bool, alone: bool) -> Self {
        match (ctrl, placement.map(Side::of)) {
            (true, _) if own => Self::Cancel,
            (true, side) => Self::Tab(side.unwrap_or(Side::Top)),
            (false, None) if own => Self::Cancel,
            (false, None) => Self::Replace,
            (false, Some(_)) if own && alone => Self::Cancel,
            (false, Some(side)) => Self::Split(side),
        }
    }

    /// What the drop target says while the panel is over it.
    pub fn label(self) -> String {
        match self {
            Self::Split(side) => format!("Split · {}", side.lower()),
            Self::Replace => "Replace · Ctrl adds a tab".into(),
            Self::Tab(side) => format!("Tab · strip on the {}", side.lower()),
            Self::Cancel => "Cancel".into(),
        }
    }
}

/// What a tab shows besides its icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Labels {
    /// Names on a strip across the group, icons alone down a side.
    #[default]
    Auto,
    Icons,
    Names,
}

impl Labels {
    pub const ALL: [Labels; 3] = [Self::Auto, Self::Icons, Self::Names];

    pub fn title(self) -> &'static str {
        match self {
            Self::Auto => "Names across, icons down a side",
            Self::Icons => "Icons only",
            Self::Names => "Icons and names",
        }
    }
}

/// Where a group's tabs run, and what they show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Strip {
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub labels: Labels,
}

impl Strip {
    pub fn is_default(self) -> bool {
        self == Self::default()
    }

    /// Whether the tabs show their names.
    pub fn names(self) -> bool {
        match self.labels {
            Labels::Auto => !self.side.runs_down(),
            Labels::Icons => false,
            Labels::Names => true,
        }
    }

    /// How wide the strip is when it runs down a side.
    fn width(self) -> Pixels {
        if self.names() { NAME_STRIP } else { ICON_STRIP }
    }
}

/// A strip as a snapshot or the layout file keeps it: the group is named by
/// the saved names of its panels, sorted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedStrip {
    pub panels: Vec<String>,
    #[serde(flatten)]
    pub strip: Strip,
}

/// The strips worth keeping: every group whose strip is not the default,
/// named by its panels. `groups` holds each group's key and its panels' names.
pub fn keep<K: Copy>(groups: &[(K, Vec<String>)], strip_of: impl Fn(K) -> Strip) -> Vec<SavedStrip> {
    let mut kept: Vec<SavedStrip> = groups
        .iter()
        .filter_map(|(key, panels)| {
            let strip = strip_of(*key);
            let mut panels = panels.clone();
            panels.sort();
            (!strip.is_default() && !panels.is_empty()).then_some(SavedStrip { panels, strip })
        })
        .collect();
    kept.sort_by(|a, b| a.panels.cmp(&b.panels));
    kept
}

/// Which strip each group takes back: the saved one that names exactly its
/// panels. A group no saved strip names keeps the default.
pub fn match_saved<K: Copy>(groups: &[(K, Vec<String>)], saved: &[SavedStrip]) -> Vec<(K, Strip)> {
    groups
        .iter()
        .filter_map(|(key, panels)| {
            let mut panels = panels.clone();
            panels.sort();
            saved.iter().find(|s| s.panels == panels).map(|s| (*key, s.strip))
        })
        .collect()
}

/// Every tab group the area holds, with the saved names of its panels.
fn groups(area: &DockArea, cx: &App) -> Vec<(NodeId, Vec<String>)> {
    let mut found = Vec::new();
    for placement in [DockPlacement::Center, DockPlacement::Left, DockPlacement::Bottom, DockPlacement::Right] {
        let Some(tree) = area.layout(placement) else { continue };
        tree.root().walk(&mut |node| {
            if let PaneRef::Tabs { panels, .. } = node.kind() {
                let names = panels.iter().filter_map(|id| area.panel(*id)).map(|p| p.panel_name(cx).to_string()).collect();
                found.push((node.id(), names));
            }
        });
    }
    found
}

/// Each group's strip, by node, while the app runs. The skin draws from it;
/// the workbench changes it, keeps it with the layout, and puts it back.
#[derive(Clone, Default)]
pub struct Strips(Rc<RefCell<HashMap<NodeId, Strip>>>);

impl Strips {
    pub fn get(&self, node: NodeId) -> Strip {
        self.0.borrow().get(&node).copied().unwrap_or_default()
    }

    pub fn set(&self, node: NodeId, strip: Strip) {
        let mut map = self.0.borrow_mut();
        if strip.is_default() {
            map.remove(&node);
        } else {
            map.insert(node, strip);
        }
    }

    /// The group `raw` names, if `area` still holds it: an action carries a
    /// node as a number, and a node is only made by the dock.
    pub fn node(area: &DockArea, raw: u64, cx: &App) -> Option<NodeId> {
        groups(area, cx).into_iter().map(|(node, _)| node).find(|node| node.as_u64() == raw)
    }

    /// The strips of the groups `area` holds now, to keep with its layout.
    pub fn save(&self, area: &DockArea, cx: &App) -> Vec<SavedStrip> {
        keep(&groups(area, cx), |node| self.get(node))
    }

    /// After `area` was loaded: each group takes back the strip `saved` names
    /// for it, and every other strip is forgotten.
    pub fn restore(&self, saved: &[SavedStrip], area: &DockArea, cx: &App) {
        let matched = match_saved(&groups(area, cx), saved);
        let mut map = self.0.borrow_mut();
        map.clear();
        map.extend(matched.into_iter().filter(|(_, strip)| !strip.is_default()));
    }
}

/// Put a group's tabs along another side of it, or show them another way.
/// `group` is the group's node, as [`NodeId::as_u64`] gives it.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct SetTabStrip {
    pub group: u64,
    pub side: Option<Side>,
    pub labels: Option<Labels>,
}

/// The dock's appearance: GPUI Kit's, with this module's tab groups.
pub struct Skin {
    kit: Rc<DockSkin>,
    strips: Strips,
}

impl Skin {
    /// A dock area wearing it, drawing each group's tabs as `strips` says.
    pub fn dock_area(id: &str, version: Option<usize>, strips: Strips, window: &mut Window, cx: &mut App) -> Entity<DockArea> {
        let id = SharedString::from(id.to_string());
        cx.new(|cx| {
            let kit = DockSkin::new(cx);
            DockArea::new(id, version, window, cx).with_renderer(Rc::new(Skin { kit, strips }))
        })
    }
}

impl DockAreaRenderer for Skin {
    fn frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        self.kit.frame(window, cx)
    }

    fn split_frame(&self, node: NodeId, axis: Axis, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        self.kit.split_frame(node, axis, window, cx)
    }

    fn center_frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        self.kit.center_frame(window, cx)
    }

    fn render_split_handle(&self, handle: &ResizeHandleContext, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.kit.render_split_handle(handle, window, cx)
    }

    fn render_dock(&self, dock: &DockContext, content: AnyElement, window: &mut Window, cx: &mut App) -> AnyElement {
        self.kit.render_dock(dock, content, window, cx)
    }

    fn build_placeholder(&self, state: &PanelState, window: &mut Window, cx: &mut App) -> Option<Arc<dyn BasePanelView>> {
        self.kit.build_placeholder(state, window, cx)
    }

    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(GroupSkin {
            kit: self.kit.tab_group_renderer(),
            strips: self.strips.clone(),
            scroll: ScrollHandle::new(),
            shown: Cell::new(None),
            pressed: Rc::default(),
            group: RefCell::new(None),
            dragged_from: Rc::default(),
        })
    }
}

/// One group's appearance. The dock asks for one per group, so the scroll
/// position of a strip belongs to the group whose tabs it scrolls.
struct GroupSkin {
    /// GPUI Kit's own, for the parts drawn as it draws them.
    kit: Rc<dyn TabGroupRenderer>,
    strips: Strips,
    scroll: ScrollHandle,
    /// The tab shown last frame, so a newly shown one is scrolled into view.
    shown: Cell<Option<usize>>,
    /// The tab a right click landed on, taken by the menu it opens; `None`
    /// when it landed beside the tabs.
    pressed: Rc<Cell<Option<usize>>>,
    /// The group as last drawn, for the drop target, which the dock draws
    /// without it.
    group: RefCell<Option<TabGroupContext>>,
    /// The group the panel being dragged comes from, as the last drag move said.
    dragged_from: Rc<Cell<Option<NodeId>>>,
}

/// The tabs a group draws, by their index in the group.
fn visible(group: &TabGroupContext, cx: &App) -> Vec<usize> {
    group.panels().iter().enumerate().filter(|(_, panel)| panel.visible(cx)).map(|(ix, _)| ix).collect()
}

/// The index of the panel on screen, which is the active tab unless that one
/// went invisible.
fn displayed(group: &TabGroupContext, cx: &App) -> Option<usize> {
    let id = group.active_panel()?.panel_id(cx);
    group.panels().iter().position(|panel| panel.panel_id(cx) == id)
}

/// What a panel is called and drawn with; a panel this build does not know
/// keeps the name its layout gave it.
fn identity(panel: &Arc<dyn BasePanelView>, cx: &App) -> (SharedString, AppIcon, Option<PanelKind>) {
    let name = panel.panel_name(cx);
    match PanelKind::from_name(name) {
        Some(kind) => (kind.title().into(), kind.icon(), Some(kind)),
        None => (name.into(), AppIcon::Decomp, None),
    }
}

/// The drag that moves the tab at `ix` out of its group, unless this group
/// may not be rearranged.
fn tab_drag(group: &TabGroupContext, ix: usize, cx: &App) -> Option<DragPanel> {
    group.is_draggable().then(|| group.drag_panel(ix, cx)).flatten()
}

impl GroupSkin {
    fn strip(&self, group: &TabGroupContext) -> Strip {
        self.strips.get(group.node())
    }

    /// Bring a newly shown tab into view; the group owns which tab is shown,
    /// so the skin notices the change rather than being told of it.
    fn follow_shown(&self, group: &TabGroupContext, visible: &[usize]) {
        let active = group.active_ix();
        if self.shown.replace(Some(active)) != Some(active)
            && let Some(at) = visible.iter().position(|&ix| ix == active)
        {
            self.scroll.scroll_to_item(at);
        }
    }

    /// The row above a group's content: the panel shown, with a marker that
    /// puts another panel in its place, and the group's controls. The only
    /// row of a group of one, and above the content when its tabs run along
    /// another side.
    fn header(&self, group: &TabGroupContext, ix: usize, cx: &mut App) -> AnyElement {
        let theme = cx.theme();
        let panel = &group.panels()[ix];
        let (title, icon, kind) = identity(panel, cx);
        let strip = self.strip(group);
        let drag = tab_drag(group, ix, cx);
        let menu_group = group.clone();
        let marker_kind = kind;
        h_flex()
            .id(("group-header", group.node().as_u64()))
            .flex_none()
            .h(BAR_HEIGHT)
            .pl_1()
            .pr_1()
            .gap_1()
            .items_center()
            .bg(theme.tokens.tab_bar)
            .border_b_1()
            .border_color(theme.border)
            .child(
                Button::new("group-marker")
                    .icon(Icon::new(icon))
                    .xsmall()
                    .ghost()
                    .tab_stop(false)
                    .tooltip("Show another panel here")
                    .dropdown_menu(move |menu, _, _| match marker_kind {
                        Some(kind) => show_instead(menu.label("Show instead"), kind),
                        None => menu,
                    }),
            )
            .child(
                div()
                    .id("group-title")
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .text_color(theme.tab_active_foreground)
                    .child(title)
                    .when_some(drag, |this, drag| {
                        this.on_drag(drag, move |drag, offset, _, cx| {
                            cx.stop_propagation();
                            drag.set_drag_offset(offset);
                            drag.set_preview_size(DRAG_PREVIEW);
                            cx.new(|_| PanelDrag(kind))
                        })
                    }),
            )
            .child(self.toolbar(group, ix, strip, true, cx))
            .context_menu(move |menu, window, cx| group_menu(menu, &menu_group, ix, false, strip, window, cx))
            .into_any_element()
    }

    /// The group's controls: its menu, and, unless the tabs carry their own,
    /// a button that closes the panel shown.
    fn toolbar(&self, group: &TabGroupContext, ix: usize, strip: Strip, close_button: bool, cx: &App) -> impl IntoElement {
        let id = group.panels()[ix].panel_id(cx);
        let closable = close_button && group.is_panel_closable(id, cx);
        let (menu_group, close_group) = (group.clone(), group.clone());
        h_flex()
            .flex_none()
            .gap_0p5()
            .occlude()
            .child(
                Button::new("group-menu")
                    .icon(IconName::Ellipsis)
                    .xsmall()
                    .ghost()
                    .tab_stop(false)
                    .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, window, cx| {
                        group_menu(menu, &menu_group, ix, false, strip, window, cx)
                    }),
            )
            .when(closable, |this| {
                this.child(
                    Button::new("group-close")
                        .icon(Icon::new(AppIcon::X))
                        .xsmall()
                        .ghost()
                        .tab_stop(false)
                        .tooltip("Close")
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            close_group.close(id, window, cx);
                        }),
                )
            })
    }

    /// Tabs side by side, for a strip across the top or the bottom.
    fn across(&self, group: &TabGroupContext, visible: &[usize], strip: Strip, toolbar: bool, cx: &mut App) -> AnyElement {
        let shown = displayed(group, cx);
        let droppable = group.is_droppable();
        let count = group.panels().len();
        let tabs: Vec<Tab> = visible
            .iter()
            .map(|&ix| {
                let panel = &group.panels()[ix];
                let (title, icon, kind) = identity(panel, cx);
                let label = if strip.names() {
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .child(Icon::new(icon).small())
                        .child(div().max_w(TAB_NAME_WIDTH).truncate().child(title.clone()))
                        .into_any_element()
                } else {
                    Icon::new(icon).small().into_any_element()
                };
                let id = panel.panel_id(cx);
                let closable = strip.names() && group.is_panel_closable(id, cx);
                let (select, close, pressed) = (group.clone(), group.clone(), self.pressed.clone());
                Tab::new()
                    .child(label)
                    .selected(Some(ix) == shown)
                    .on_click(move |_, window, cx| select.select_tab(ix, window, cx))
                    .on_mouse_down(MouseButton::Right, move |_, _, _| pressed.set(Some(ix)))
                    .when(!strip.names(), |this| {
                        let title = title.clone();
                        this.tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                    })
                    .when(closable, |this| {
                        this.suffix(
                            Button::new(("close-tab", ix))
                                .icon(Icon::new(AppIcon::X))
                                .xsmall()
                                .ghost()
                                .tab_stop(false)
                                .ml(-px(8.))
                                .mr_1()
                                .on_click(move |_, window, cx| {
                                    cx.stop_propagation();
                                    close.close(id, window, cx);
                                }),
                        )
                    })
                    .map(|this| self.drag_and_drop(this, group, ix, kind, cx))
            })
            .collect();
        let pressed = self.pressed.clone();
        let menu_group = group.clone();
        let empty_group = group.clone();
        let node = group.node();
        let bar = TabBar::new("tab-bar")
            .track_scroll(&self.scroll)
            .children(tabs)
            .last_empty_space(
                div()
                    .id("tab-bar-empty-space")
                    .h_full()
                    .flex_1()
                    .min_w_16()
                    .when(droppable, |this| {
                        let item_group = empty_group.clone();
                        this.drag_over::<DragPanel>(|this, _, _, cx| this.bg(cx.theme().tokens.drop_target))
                            .on_drop(move |drag: &DragPanel, window, cx| {
                                // Past its own last tab a panel takes the last slot;
                                // one from elsewhere joins at the end.
                                let ix = (drag.source() == node).then(|| count.saturating_sub(1));
                                empty_group.drop_panel(drag.clone(), ix, false, window, cx);
                            })
                            .drag_over::<AnyDrag>(|this, _, _, cx| this.bg(cx.theme().tokens.drop_target))
                            .on_drop(move |item: &AnyDrag, window, cx| item_group.drop_item(item.clone(), None, window, cx))
                    }),
            )
            .when(toolbar, |this| {
                let ix = shown.unwrap_or(visible[0]);
                this.suffix(
                    h_flex()
                        .h_full()
                        .px_1()
                        .items_center()
                        .border_l_1()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().tokens.tab_bar)
                        // Tabs with names carry their own close buttons.
                        .child(self.toolbar(group, ix, strip, !strip.names(), cx)),
                )
            });
        let scroll = self.scroll.clone();
        div()
            .id(("group-tabs", node.as_u64()))
            .w_full()
            .child(bar)
            // The kit's strip scrolls only along its own axis, so an ordinary
            // wheel did nothing over it and tabs past the edge could not be
            // reached. Turned vertically, the wheel moves the strip sideways;
            // a sideways turn (or Shift) is the kit's to handle, and is left to it.
            .on_scroll_wheel(move |event, window, _| {
                let delta = event.delta.pixel_delta(window.line_height());
                if !delta.x.is_zero() || delta.y.is_zero() {
                    return;
                }
                let mut offset = scroll.offset();
                let furthest = scroll.max_offset().x;
                offset.x = (offset.x + delta.y).clamp(-furthest, px(0.));
                scroll.set_offset(offset);
                window.refresh();
            })
            .context_menu(move |menu, window, cx| {
                // Set by the tab the press landed on, if any; taken, so a press
                // beside the tabs next time finds none.
                let on_tab = pressed.take();
                let ix = on_tab.or_else(|| menu_group.active_panel().map(|_| menu_group.active_ix())).unwrap_or(0);
                group_menu(menu, &menu_group, ix, on_tab.is_some(), strip, window, cx)
            })
            .into_any_element()
    }

    /// Tabs one above another, for a strip down the left or the right side.
    fn down(&self, group: &TabGroupContext, visible: &[usize], strip: Strip, cx: &mut App) -> AnyElement {
        let theme = cx.theme();
        let shown = displayed(group, cx);
        let droppable = group.is_droppable();
        let count = group.panels().len();
        let node = group.node();
        let left = strip.side == Side::Left;
        let (tab_active, tab_bar, border, primary) = (theme.tokens.tab_active, theme.tokens.tab_bar, theme.border, theme.primary);
        let (active_text, text, hover) = (theme.tab_active_foreground, theme.tab_foreground, theme.list_hover);
        let tabs: Vec<AnyElement> = visible
            .iter()
            .map(|&ix| {
                let panel = &group.panels()[ix];
                let (title, icon, kind) = identity(panel, cx);
                let selected = Some(ix) == shown;
                let (select, pressed) = (group.clone(), self.pressed.clone());
                let tab = h_flex()
                    .id(("side-tab", ix))
                    .flex_none()
                    .h(px(30.))
                    .w_full()
                    .px_2()
                    .gap_1p5()
                    .items_center()
                    .cursor_pointer()
                    .text_sm()
                    .map(|this| {
                        // The active tab is marked on the edge that faces the content.
                        let this = if left { this.border_r_2() } else { this.border_l_2() };
                        if selected {
                            this.bg(tab_active).text_color(active_text).border_color(primary)
                        } else {
                            this.text_color(text).border_color(gpui_kit::transparent_black()).hover(move |s| s.bg(hover))
                        }
                    })
                    .child(Icon::new(icon).small())
                    .when(strip.names(), |this| this.child(div().min_w_0().truncate().child(title.clone())))
                    .when(!strip.names(), |this| {
                        let title = title.clone();
                        this.tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                    })
                    .on_click(move |_, window, cx| select.select_tab(ix, window, cx))
                    .on_mouse_down(MouseButton::Right, move |_, _, _| pressed.set(Some(ix)));
                self.drag_and_drop(tab, group, ix, kind, cx).into_any_element()
            })
            .collect();
        let pressed = self.pressed.clone();
        let menu_group = group.clone();
        let empty_group = group.clone();
        v_flex()
            .id(("group-tabs", node.as_u64()))
            .flex_none()
            .h_full()
            .map(|this| if left { this.border_r_1() } else { this.border_l_1() })
            .w(strip.width())
            .bg(tab_bar)
            .border_color(border)
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(tabs)
            .child(
                div()
                    .id("side-tabs-empty-space")
                    .flex_1()
                    .min_h_8()
                    .w_full()
                    .when(droppable, |this| {
                        let item_group = empty_group.clone();
                        this.drag_over::<DragPanel>(|this, _, _, cx| this.bg(cx.theme().tokens.drop_target))
                            .on_drop(move |drag: &DragPanel, window, cx| {
                                let ix = (drag.source() == node).then(|| count.saturating_sub(1));
                                empty_group.drop_panel(drag.clone(), ix, false, window, cx);
                            })
                            .drag_over::<AnyDrag>(|this, _, _, cx| this.bg(cx.theme().tokens.drop_target))
                            .on_drop(move |item: &AnyDrag, window, cx| item_group.drop_item(item.clone(), None, window, cx))
                    }),
            )
            .context_menu(move |menu, window, cx| {
                let on_tab = pressed.take();
                let ix = on_tab.unwrap_or_else(|| menu_group.active_ix());
                group_menu(menu, &menu_group, ix, on_tab.is_some(), strip, window, cx)
            })
            .into_any_element()
    }

    /// A tab is dragged out like the kit's tabs are, and takes drops like them:
    /// a panel dropped on it goes into that slot.
    fn drag_and_drop<E: StatefulInteractiveElement + Styled + gpui_kit::prelude::FluentBuilder + 'static>(
        &self,
        tab: E,
        group: &TabGroupContext,
        ix: usize,
        kind: Option<PanelKind>,
        cx: &App,
    ) -> E {
        let drag = tab_drag(group, ix, cx);
        let droppable = group.is_droppable();
        let (panel_group, item_group) = (group.clone(), group.clone());
        tab.when_some(drag, |this, drag| {
            this.on_drag(drag, move |drag, offset, _, cx| {
                cx.stop_propagation();
                drag.set_drag_offset(offset);
                drag.set_preview_size(DRAG_PREVIEW);
                cx.new(|_| PanelDrag(kind))
            })
        })
        .when(droppable, |this| {
            this.drag_over::<DragPanel>(|this, _, _, cx| this.bg(cx.theme().tokens.drop_target))
                .on_drop(move |drag: &DragPanel, window, cx| panel_group.drop_panel(drag.clone(), Some(ix), true, window, cx))
                .drag_over::<AnyDrag>(|this, _, _, cx| this.bg(cx.theme().tokens.drop_target))
                .on_drop(move |item: &AnyDrag, window, cx| item_group.drop_item(item.clone(), None, window, cx))
        })
    }
}

impl TabGroupRenderer for GroupSkin {
    fn frame(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        self.group.replace(Some(group.clone()));
        // Positioned, so a strip down a side or along the bottom is laid over it.
        self.kit
            .frame(group, window, cx)
            .relative()
            // Ctrl changes what a drop does; the target says so at once, not at
            // the next move of the pointer.
            .on_modifiers_changed(|_, window, cx| {
                if cx.has_active_drag() {
                    window.refresh();
                }
            })
    }

    fn content_frame(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        let from = self.dragged_from.clone();
        let frame = self
            .kit
            .content_frame(group, window, cx)
            .on_drag_move::<DragPanel>(move |event, _, cx| from.set(Some(event.drag(cx).source())));
        if group.is_collapsed() || visible(group, cx).len() < 2 {
            return frame;
        }
        // The dock lays out a group as its tab bar above its content. Tabs
        // anywhere else are drawn over the whole group (see `render_tab_bar`),
        // and the content leaves room for them and for the header.
        let strip = self.strip(group);
        match strip.side {
            Side::Top => frame,
            Side::Bottom => frame.mt(BAR_HEIGHT).mb(BAR_HEIGHT),
            Side::Left => frame.mt(BAR_HEIGHT).ml(strip.width()),
            Side::Right => frame.mt(BAR_HEIGHT).mr(strip.width()),
        }
    }

    fn render_tab_bar(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> AnyElement {
        if group.is_collapsed() {
            return self.kit.render_tab_bar(group, window, cx);
        }
        let shown = visible(group, cx);
        self.follow_shown(group, &shown);
        let strip = self.strip(group);
        match shown.as_slice() {
            [] => Empty.into_any_element(),
            [only] => self.header(group, *only, cx),
            _ if strip.side == Side::Top => self.across(group, &shown, strip, true, cx),
            _ => {
                // Laid over the whole group: the layout engine places an
                // absolute element against its parent, and the group's frame
                // is the only parent both the strip and the content share. The
                // middle is empty and takes no events, so the content, drawn
                // after, keeps them.
                let active = displayed(group, cx).unwrap_or(shown[0]);
                let header = self.header(group, active, cx);
                let overlay = div().absolute().top_0().left_0().right_0().bottom_0();
                match strip.side {
                    Side::Bottom => overlay
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(div().flex_1())
                        .child(
                            div()
                                .flex_none()
                                .h(BAR_HEIGHT)
                                .border_t_1()
                                .border_color(cx.theme().border)
                                .child(self.across(group, &shown, strip, false, cx)),
                        )
                        .into_any_element(),
                    side => {
                        let column = v_flex().flex_1().min_w_0().child(header).child(div().flex_1());
                        let tabs = self.down(group, &shown, strip, cx);
                        let row = overlay.flex().flex_row();
                        if side == Side::Left { row.child(tabs).child(column) } else { row.child(column).child(tabs) }.into_any_element()
                    }
                }
            }
        }
    }

    fn render_active_panel(&self, panel: AnyView, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> AnyElement {
        self.kit.render_active_panel(panel, group, window, cx)
    }

    /// The target under a dragged panel. It is drawn over the content while
    /// a panel is over it, so it is what the drop lands on: it carries out
    /// every drop there (a split goes back to the dock as it would have), and
    /// says beforehand which of the four it will be. A host drag (a panel from
    /// the widget palette) passes through to the dock, and the workbench
    /// gives it the same four.
    fn render_drop_indicator(&self, indicator: DropIndicator, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let group = self.group.borrow().clone()?;
        let ctrl = window.modifiers().control;
        let own = self.dragged_from.get() == Some(group.node());
        let mode = DropMode::of(indicator.placement(), ctrl, own, group.panels().len() <= 1);
        let theme = cx.theme();
        let (accent, popover, border) = (theme.drag_border, theme.popover, theme.border);
        let picture = match mode {
            DropMode::Tab(side) => {
                // The whole group, with the strip where it is going.
                let bar = div().absolute().bg(accent);
                let bar = match side {
                    Side::Top => bar.top_0().left_0().right_0().h(px(4.)),
                    Side::Bottom => bar.bottom_0().left_0().right_0().h(px(4.)),
                    Side::Left => bar.top_0().bottom_0().left_0().w(px(4.)),
                    Side::Right => bar.top_0().bottom_0().right_0().w(px(4.)),
                };
                Some(div().absolute().top_0().left_0().size_full().bg(theme.tokens.drop_target).child(bar).into_any_element())
            }
            DropMode::Cancel => None,
            DropMode::Split(_) | DropMode::Replace => self.kit.render_drop_indicator(indicator, window, cx),
        };
        let (strips, strip_group) = (self.strips.clone(), group.clone());
        Some(
            div()
                .id("drop-target")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .children(picture)
                .child(
                    div().absolute().top_0().left_0().size_full().flex().items_center().justify_center().child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .bg(popover)
                            .text_sm()
                            .child(mode.label()),
                    ),
                )
                .on_drop(move |drag: &DragPanel, window, cx| {
                    let group = &strip_group;
                    let own = drag.source() == group.node();
                    let panels: Vec<PanelId> = group.panels().iter().map(|p| p.panel_id(cx)).collect();
                    let placement = group.drop_indicator().and_then(|i| i.placement());
                    match DropMode::of(placement, window.modifiers().control, own, panels.len() <= 1) {
                        // The dock splits by the zone it resolved.
                        DropMode::Split(_) => group.drop_panel(drag.clone(), None, true, window, cx),
                        DropMode::Tab(side) => {
                            let node = group.node();
                            strips.set(node, Strip { side, ..strips.get(node) });
                            group.drop_panel(drag.clone(), Some(panels.len()), true, window, cx);
                        }
                        DropMode::Replace => {
                            group.drop_panel(drag.clone(), Some(panels.len()), true, window, cx);
                            // In the order given: the panel joins, then the others leave.
                            for id in panels {
                                group.close(id, window, cx);
                            }
                        }
                        // Back into its own slot, which changes nothing and ends the drag.
                        DropMode::Cancel => {
                            let at = panels.iter().position(|&id| id == drag.panel()).unwrap_or(0);
                            group.drop_panel(drag.clone(), Some(at), true, window, cx);
                        }
                    }
                })
                .into_any_element(),
        )
    }

    fn render_empty(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.kit.render_empty(group, window, cx)
    }
}

/// "Show instead": every other panel, to put in `kind`'s place in its group.
pub fn show_instead(menu: PopupMenu, kind: PanelKind) -> PopupMenu {
    PanelKind::ALL.into_iter().filter(|&other| other != kind).fold(menu, |menu, other| {
        let replace = ReplacePanel { with: other.name().into(), instead_of: kind.name().into() };
        menu.menu_with_icon(other.title(), Icon::new(other.icon()), Box::new(replace))
    })
}

/// What a group offers about its tab `ix`, from a right click on a tab
/// (`on_tab`), on its header or strip, or from its ⋯ button: show or close
/// the tab, put another panel in its place, and where the group's tabs run.
fn group_menu(
    menu: PopupMenu,
    group: &TabGroupContext,
    ix: usize,
    on_tab: bool,
    strip: Strip,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let Some(panel) = group.panels().get(ix).cloned() else { return menu };
    let (title, _, kind) = identity(&panel, cx);
    let id = panel.panel_id(cx);
    let node = group.node().as_u64();
    let shown = displayed(group, cx) == Some(ix);
    let closable = group.is_panel_closable(id, cx);
    let (select, close) = (group.clone(), group.clone());
    let mut menu = menu.label(if on_tab { format!("Tab · {title}") } else { title.to_string() });
    if !shown {
        menu = menu.item(
            PopupMenuItem::new("Show")
                .icon(Icon::new(AppIcon::Follow))
                .on_click(move |_, window, cx| select.select_tab(ix, window, cx)),
        );
    }
    if closable {
        menu = menu.item(
            PopupMenuItem::new(format!("Close {title}"))
                .icon(Icon::new(AppIcon::X))
                .on_click(move |_, window, cx| close.close(id, window, cx)),
        );
    }
    if let Some(kind) = kind {
        menu = menu.separator().submenu_with_icon(Some(Icon::new(AppIcon::AddWidget)), "Show Instead", window, cx, move |menu, _, _| {
            show_instead(menu, kind)
        });
    }
    // Every tab of the group, the one shown marked: a strip too narrow for
    // its tabs hides some of them, and this reaches them all.
    let tabs = visible(group, cx);
    if tabs.len() > 1 {
        let on_screen = displayed(group, cx);
        menu = menu.separator().label("Tabs in This Group");
        for tab in tabs {
            let (tab_title, tab_icon, _) = identity(&group.panels()[tab], cx);
            let select = group.clone();
            // The one shown gets the check mark where the others have their icon.
            let item = PopupMenuItem::new(tab_title).on_click(move |_, window, cx| select.select_tab(tab, window, cx));
            menu = menu.item(if on_screen == Some(tab) { item.checked(true) } else { item.icon(Icon::new(tab_icon)) });
        }
    }
    let menu = menu.separator().label("Tabs Along");
    let menu = Side::ALL.into_iter().fold(menu, |menu, side| {
        let set = SetTabStrip { group: node, side: Some(side), labels: None };
        menu.item(PopupMenuItem::new(side.title()).checked(strip.side == side).action(Box::new(set)))
    });
    let menu = menu.separator().label("Tab Labels");
    Labels::ALL.into_iter().fold(menu, |menu, labels| {
        let set = SetTabStrip { group: node, side: None, labels: Some(labels) };
        menu.item(PopupMenuItem::new(labels.title()).checked(strip.labels == labels).action(Box::new(set)))
    })
}

/// What follows the pointer while a panel is dragged: from a tab, a header,
/// or the widget palette.
pub struct PanelDrag(pub Option<PanelKind>);

impl Render for PanelDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .opacity(0.85)
            .child(match self.0 {
                Some(kind) => crate::panel::tab_title(kind).into_any_element(),
                None => "Panel".into_any_element(),
            })
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{DropMode, Labels, SavedStrip, Side, Strip, keep, match_saved};
    use gpui_kit::base::Placement;

    #[test]
    fn a_drop_does_what_the_tauri_dock_did() {
        // Plain: an edge splits, the centre replaces.
        assert_eq!(DropMode::of(Some(Placement::Left), false, false, false), DropMode::Split(Side::Left));
        assert_eq!(DropMode::of(None, false, false, false), DropMode::Replace);
        // Ctrl: a tab, the strip going to the side dropped at; the centre puts it on top.
        assert_eq!(DropMode::of(Some(Placement::Bottom), true, false, false), DropMode::Tab(Side::Bottom));
        assert_eq!(DropMode::of(None, true, false, true), DropMode::Tab(Side::Top));
        // Its own group: only splitting out of a group holding more than it does anything.
        assert_eq!(DropMode::of(Some(Placement::Right), false, true, false), DropMode::Split(Side::Right));
        assert_eq!(DropMode::of(Some(Placement::Right), false, true, true), DropMode::Cancel);
        assert_eq!(DropMode::of(None, false, true, false), DropMode::Cancel);
        assert_eq!(DropMode::of(Some(Placement::Top), true, true, false), DropMode::Cancel);
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_strip_comes_back_to_the_group_with_the_same_panels_whatever_their_order() {
        let left = Strip { side: Side::Left, labels: Labels::Auto };
        let bottom = Strip { side: Side::Bottom, labels: Labels::Icons };
        // Before: groups 1, 2 and 3; 1 and 3 have strips of their own.
        let before = [(1u32, names(&["xrefs", "bookmarks"])), (2, names(&["functions"])), (3, names(&["hex", "disassembly"]))];
        let kept = keep(&before, |g| match g {
            1 => left,
            3 => bottom,
            _ => Strip::default(),
        });
        assert_eq!(kept.len(), 2, "a default strip is not kept: {kept:?}");
        assert!(kept.iter().all(|s| s.panels.windows(2).all(|w| w[0] <= w[1])), "names are sorted");
        // After a load: new keys, tabs in another order.
        let after = [(7u32, names(&["disassembly", "hex"])), (8, names(&["functions"])), (9, names(&["bookmarks", "xrefs"]))];
        let mut back = match_saved(&after, &kept);
        back.sort_by_key(|(k, _)| *k);
        assert_eq!(back, vec![(7, bottom), (9, left)]);
    }

    #[test]
    fn a_group_that_gained_or_lost_a_panel_is_not_taken_for_the_one_saved() {
        let saved = vec![SavedStrip { panels: names(&["bookmarks", "xrefs"]), strip: Strip { side: Side::Right, labels: Labels::Names } }];
        let grown = [(1u32, names(&["bookmarks", "xrefs", "find"]))];
        let shrunk = [(1u32, names(&["xrefs"]))];
        assert!(match_saved(&grown, &saved).is_empty());
        assert!(match_saved(&shrunk, &saved).is_empty());
    }

    #[test]
    fn labels_follow_the_side_unless_chosen() {
        let across = Strip { side: Side::Bottom, labels: Labels::Auto };
        let down = Strip { side: Side::Left, labels: Labels::Auto };
        assert!(across.names() && !down.names());
        assert!(Strip { side: Side::Right, labels: Labels::Names }.names());
        assert!(!Strip { side: Side::Top, labels: Labels::Icons }.names());
    }

    #[test]
    fn a_saved_strip_reads_back_as_written_and_an_old_file_needs_none() {
        let saved = SavedStrip { panels: names(&["console", "hex"]), strip: Strip { side: Side::Left, labels: Labels::Icons } };
        let json = serde_json::to_string(&saved).unwrap();
        assert_eq!(json, r#"{"panels":["console","hex"],"side":"left","labels":"icons"}"#);
        assert_eq!(serde_json::from_str::<SavedStrip>(&json).unwrap(), saved);
        // A strip written without labels takes the default.
        let short: SavedStrip = serde_json::from_str(r#"{"panels":["hex"],"side":"bottom"}"#).unwrap();
        assert_eq!(short.strip, Strip { side: Side::Bottom, labels: Labels::Auto });
    }
}
