// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Triage: what the image is, from its own headers, and what the engine warns
//! about it. Read once per target, when the panel is first shown.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::*;
use n0xis_client::{Engine, Export, Profile, ProfileReport};

use crate::assets::AppIcon;
use crate::context;
use crate::layout::PanelKind;
use crate::nav::{Navigate, parse_va};
use crate::panel::{dock_panel, header, message, mono};

const ROW_HEIGHT: Pixels = px(22.);

enum ViewState {
    Empty,
    Loading,
    Ready(Box<ProfileReport>),
    Failed(String),
}

pub struct TriageView {
    engine: Option<Arc<Engine>>,
    state: ViewState,
    /// Shown at least once since the target was opened; until then nothing is asked.
    wanted: bool,
    active: bool,
    filter: Entity<InputState>,
    /// Rows of the export table that match the filter.
    visible: Vec<usize>,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for TriageView {}

impl TriageView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter exports"));
        let subscription = cx.subscribe_in(&filter, window, |this, _, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.refilter(cx);
            }
        });
        Self {
            engine: None,
            state: ViewState::Empty,
            wanted: false,
            active: false,
            filter,
            visible: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _request: None,
            _subscriptions: vec![subscription],
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.state = ViewState::Empty;
        self.visible.clear();
        self._request = None;
        self.wanted = self.engine.is_some();
        if self.active {
            self.load(cx);
        }
        cx.notify();
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.wanted {
            self.load(cx);
        }
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        self.wanted = false;
        self.state = ViewState::Loading;
        cx.notify();
        let pending = engine.send(&Profile { exports: true });
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                view.state = match result {
                    Ok(report) => ViewState::Ready(Box::new(report)),
                    Err(e) => ViewState::Failed(e.to_string()),
                };
                view.refilter(cx);
            })
            .ok();
        }));
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.filter.read(cx).value().trim().to_lowercase();
        self.visible = match &self.state {
            ViewState::Ready(report) => report
                .image
                .exports
                .iter()
                .enumerate()
                .filter(|(_, e)| query.is_empty() || e.name.to_lowercase().contains(&query) || e.va.contains(&query))
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        };
        cx.notify();
    }

    fn export(&self, row: usize) -> Option<&Export> {
        match &self.state {
            ViewState::Ready(report) => self.visible.get(row).and_then(|&i| report.image.exports.get(i)),
            _ => None,
        }
    }

    fn render_export(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(export) = self.export(row).cloned() else { return div().h(ROW_HEIGHT).into_any_element() };
        let theme = cx.theme();
        // A forwarder names code in another module: its address is a string, not code to go to.
        let destination = match (&export.forwarder, export.thunk_target.as_deref()) {
            (Some(to), _) => format!("→ {to}"),
            (None, Some(t)) => format!("stub → {t}"),
            (None, None) => String::new(),
        };
        let go = if export.forwarder.is_none() { parse_va(&export.va) } else { None };
        let (name, address, forwarded) = (export.name.clone(), export.va.clone(), destination.clone());
        let row = h_flex()
            .id(("export", row))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_3()
            .text_xs()
            .child(div().flex_1().min_w_0().truncate().child(export.name.clone()))
            .child(div().flex_none().text_color(theme.muted_foreground).child(destination))
            .child(
                mono(export.va.clone(), cx)
                    .flex_none()
                    .w(px(130.))
                    .text_color(if go.is_some() { theme.foreground } else { theme.muted_foreground }),
            )
            .when_some(go, |r, va| {
                r.cursor_pointer()
                    .hover(|s| s.bg(theme.list_hover))
                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Navigate(va))))
            });
        row.context_menu(move |menu, _, _| {
            let menu = menu.label(name.clone());
            let menu = match go {
                Some(va) => {
                    let menu = context::go(menu, "Go To", va);
                    let menu = context::views(menu, va, None).separator();
                    context::annotations(menu, va).separator()
                }
                None => menu,
            };
            let menu = context::copy(menu, "Copy Name", name.clone());
            let menu = if forwarded.is_empty() { menu } else { context::copy(menu, "Copy Destination", forwarded.trim_start_matches("→ ").to_string()) };
            context::copy(menu, "Copy Address", address.clone())
        })
        .into_any_element()
    }

    fn render_report(&self, report: &ProfileReport, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let img = &report.image;
        let fact = |label: &str, value: String| {
            h_flex()
                .gap_2()
                .text_xs()
                .child(div().w(px(150.)).flex_none().text_color(theme.muted_foreground).child(label.to_string()))
                .child(div().font_family(theme.mono_font_family.clone()).child(value))
        };
        let forwarded = match img.forwarded_count {
            Some(n) => n.to_string(),
            None => "not reported by this engine".into(),
        };
        let mut facts = v_flex()
            .gap_1()
            .child(fact("Machine", img.machine.clone()))
            .child(fact("Image", format!("{} – {}", img.module_base, img.image_end)))
            .child(fact(
                "Unwind table",
                if img.pdata_present { format!("{} functions", img.pdata_functions) } else { "none".into() },
            ))
            .child(fact("Exports", format!("{} at {} distinct addresses", img.export_count, img.export_distinct_addresses)))
            .child(fact("Forwarded exports", forwarded))
            .child(fact("Branch stubs", img.thunk_count.to_string()))
            .child(fact("Folded names", format!("{} addresses carry more than one name", img.folded.len())));
        if !img.detoured_exports.is_empty() {
            facts = facts.child(fact("Relayed outside the image", img.detoured_exports.join(", ")));
        }
        for hint in &img.engine_hints {
            facts = facts.child(fact("Runtime hint", format!("{}: {}", hint.engine, hint.evidence)));
        }
        let advisories = v_flex().gap_1().children(report.advisories.iter().map(|a| {
            let color = if a.verdict == "ineffective" { theme.danger } else { theme.warning };
            v_flex()
                .p_2()
                .gap_1()
                .rounded(theme.radius)
                .border_1()
                .border_color(theme.border)
                .child(
                    h_flex()
                        .gap_2()
                        .text_xs()
                        .child(div().text_color(color).font_weight(FontWeight::SEMIBOLD).child(a.verdict.clone()))
                        .child(div().font_family(theme.mono_font_family.clone()).child(a.command.clone())),
                )
                .child(div().text_xs().text_color(theme.muted_foreground).child(a.reason.clone()))
        }));
        let sections = v_flex().children(img.sections.iter().map(|s| {
            h_flex()
                .gap_3()
                .text_xs()
                .font_family(theme.mono_font_family.clone())
                .child(div().w(px(90.)).flex_none().truncate().child(s.name.clone()))
                .child(div().w(px(130.)).flex_none().child(s.va.clone()))
                .child(div().w(px(110.)).flex_none().text_color(theme.muted_foreground).child(format!("{:#x} bytes", s.virtual_size)))
                .child(div().text_color(theme.muted_foreground).child(if s.executable { "code" } else { "" }))
        }));
        let section_title = |text: String| div().pt_3().pb_1().text_xs().font_weight(FontWeight::SEMIBOLD).child(text);
        v_flex()
            .px_3()
            .py_2()
            .child(facts)
            .when(img.forwarded_count.is_none() && img.export_count > 0, |c| {
                c.child(div().pt_2().text_xs().text_color(theme.warning).child(
                    "This engine does not mark forwarded exports: an export's address may be its forwarder string, not code.",
                ))
            })
            .when(!report.advisories.is_empty(), |c| {
                c.child(section_title(format!("Advisories · {}", report.advisories.len()))).child(advisories)
            })
            .child(section_title(format!("Sections · {}", img.sections.len())))
            .child(sections)
            .into_any_element()
    }
}

impl Render for TriageView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.state {
            ViewState::Empty => message("Open a target to profile it.", false, cx).into_any_element(),
            ViewState::Loading => message("Reading the image headers…", false, cx).into_any_element(),
            ViewState::Failed(e) => message(format!("No profile: {e}"), true, cx).into_any_element(),
            ViewState::Ready(report) => {
                let report = report.clone();
                let exports_title = format!("Exports · {} shown of {}", self.visible.len(), report.image.exports.len());
                let facts = self.render_report(&report, cx);
                let border = cx.theme().border;
                v_flex()
                    .size_full()
                    .child(div().id("triage-facts").max_h(relative(0.55)).overflow_y_scroll().child(facts))
                    .child(
                        h_flex()
                            .px_2()
                            .py_1()
                            .gap_2()
                            .border_t_1()
                            .border_color(border)
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(exports_title))
                            .child(div().flex_1())
                            .child(div().w(px(220.)).child(Input::new(&self.filter).xsmall().prefix(Icon::new(AppIcon::Scan).xsmall()))),
                    )
                    .child(
                        uniform_list(
                            "exports",
                            self.visible.len(),
                            cx.processor(|this, range: Range<usize>, _window, cx| {
                                range.map(|row| this.render_export(row, cx)).collect::<Vec<_>>()
                            }),
                        )
                        .track_scroll(&self.scroll)
                        .flex_1(),
                    )
                    .into_any_element()
            }
        };
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header("Triage", cx))
            .child(div().flex_1().min_h_0().child(body))
    }
}

dock_panel!(TriageView, PanelKind::Triage, on_active);
