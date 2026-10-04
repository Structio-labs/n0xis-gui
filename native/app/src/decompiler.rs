// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The decompiler view: the engine's pseudo-C for the selected function, shown
//! as the engine wrote it. Colouring is the only thing done here; numbers,
//! names and types are never rewritten in the GUI.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::base::Selectable as _;
use gpui_kit::base::dock::{Panel as DockBehavior, PanelEvent};
use gpui_kit::component::dock::Panel as DockPresentation;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{ClientError, DecompStyle, Decompile, Engine, FunctionEntry};

enum ViewState {
    Empty,
    Loading,
    Ready { quality: Option<f64> },
    Failed(String),
}

pub struct DecompilerView {
    engine: Option<Arc<Engine>>,
    editor: Entity<EditorState>,
    function: Option<FunctionEntry>,
    style: DecompStyle,
    state: ViewState,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
}

impl DecompilerView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| EditorState::new(window, cx).language("c").line_number(true).soft_wrap(false));
        Self {
            engine: None,
            editor,
            function: None,
            style: DecompStyle::default(),
            state: ViewState::Empty,
            focus_handle: cx.focus_handle(),
            _request: None,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.function = None;
        self.state = ViewState::Empty;
        self._request = None;
        cx.notify();
    }

    pub fn show(&mut self, function: FunctionEntry, window: &mut Window, cx: &mut Context<Self>) {
        self.function = Some(function);
        self.request(window, cx);
    }

    fn set_style(&mut self, style: DecompStyle, window: &mut Window, cx: &mut Context<Self>) {
        if self.style != style {
            self.style = style;
            self.request(window, cx);
        }
    }

    fn request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(engine), Some(function)) = (self.engine.clone(), self.function.clone()) else { return };
        self.state = ViewState::Loading;
        cx.notify();
        // One key for this view: when the selection moves faster than the
        // engine answers, only the latest function is decompiled.
        let pending = engine.send_latest("decompile", &Decompile { addr: function.va, style: self.style });
        self._request = Some(cx.spawn_in(window, async move |this, cx| {
            let result = pending.await;
            this.update_in(cx, |view, window, cx| {
                match result {
                    Ok(decompiled) => {
                        let text = decompiled.pseudo.join("\n");
                        view.editor.update(cx, |editor, cx| editor.set_value(text, window, cx));
                        view.state = ViewState::Ready { quality: decompiled.quality };
                    }
                    // A newer request replaced this one; its answer is on the way.
                    Err(ClientError::Superseded) => return,
                    Err(e) => view.state = ViewState::Failed(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let title = self.function.as_ref().map_or("Decompiler".to_string(), |f| f.name.clone());
        let note = match &self.state {
            ViewState::Loading => "decompiling…".to_string(),
            ViewState::Ready { quality: Some(q) } => format!("quality {q:.2}"),
            _ => String::new(),
        };
        h_flex()
            .px_2()
            .py_1()
            .gap_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).truncate().child(title))
            .child(div().flex_1())
            .child(div().text_xs().text_color(theme.muted_foreground).child(note))
            .children(DecompStyle::ALL.into_iter().map(|style| {
                Button::new(SharedString::from(format!("style-{}", style.as_str())))
                    .label(style.as_str())
                    .xsmall()
                    .ghost()
                    .selected(self.style == style)
                    .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| view.set_style(style, window, cx)))
            }))
    }
}

impl Render for DecompilerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let message = |text: String, danger: bool| {
            div()
                .flex_1()
                .p_4()
                .text_sm()
                .text_color(if danger { theme.danger } else { theme.muted_foreground })
                .child(text)
        };
        let body = match &self.state {
            ViewState::Empty => message("Select a function to decompile it.".into(), false).into_any_element(),
            ViewState::Loading => {
                let name = self.function.as_ref().map_or(String::new(), |f| format!(" {}", f.name));
                message(format!("Decompiling{name}…"), false).into_any_element()
            }
            ViewState::Failed(e) => message(format!("No decompilation: {e}"), true).into_any_element(),
            ViewState::Ready { .. } => Editor::new(&self.editor)
                .readonly(true)
                .bordered(false)
                .size_full()
                .into_any_element(),
        };
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(self.render_header(cx))
            .child(div().flex_1().min_h_0().child(body))
    }
}

impl Focusable for DecompilerView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for DecompilerView {}

impl DockBehavior for DecompilerView {
    fn panel_name(&self) -> &'static str {
        crate::layout::DECOMPILER_PANEL
    }

    fn closable(&self, _: &App) -> bool {
        false
    }
}

impl DockPresentation for DecompilerView {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Decompiler"
    }
}
