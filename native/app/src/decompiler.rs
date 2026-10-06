// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The decompiler view: the engine's pseudo-C for the selected function, shown
//! as the engine wrote it. Colouring is the only thing done here; numbers,
//! names and types are never rewritten in the GUI.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{self, Editor, EditorState};
use gpui_kit::component::native_menu::NativeMenu;
use gpui_kit::component::Icon;
use gpui_kit::base::Selectable as _;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{ClientError, DecompStyle, Decompile, Engine, FunctionEntry, Variable};

use crate::assets::AppIcon;
use crate::context::{EditAt, EditKind, ShowAt};
use crate::layout::PanelKind;
use crate::nav::{Location, parse_va};
use crate::panel::dock_panel;
use crate::{Rename, SetType};

/// The key context of the view, for bindings that must win over the editor's.
pub const KEY_CONTEXT: &str = "Decompiler";

/// The editor the code is shown in. It has no search box: a field inside the
/// code would take the commands' keys, which the code itself does (`keymap`).
pub fn code_editor(window: &mut Window, cx: &mut Context<EditorState>) -> EditorState {
    EditorState::new(window, cx).language("c").line_number(true).soft_wrap(false).searchable(false)
}

/// What the user wants done to a variable they pointed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableIntent {
    Rename,
    SetType,
}

/// Emitted when the user asks to rename or type a variable of the shown function.
#[derive(Clone)]
pub struct EditVariable {
    pub function: FunctionEntry,
    pub variable: Variable,
    pub intent: VariableIntent,
}

/// What is under the caret, judged by the engine's own variable list.
enum Pointed {
    Variable(Variable),
    /// A word the engine does not list as a variable, or no word at all.
    NotAVariable,
    /// More than one variable is shown under this name.
    Ambiguous(String),
    /// The engine does not list variables.
    Unlisted,
}

enum ViewState {
    Empty,
    /// The selection is an address no listed function covers.
    NoFunction(u64),
    Loading,
    Ready { quality: Option<f64> },
    Failed(String),
}

pub struct DecompilerView {
    engine: Option<Arc<Engine>>,
    editor: Entity<EditorState>,
    function: Option<FunctionEntry>,
    /// The engine's list of the shown function's variables; `None` when the
    /// engine does not send one.
    variables: Option<Vec<Variable>>,
    /// The version the engine gave for itself in its last answer.
    engine_version: Option<String>,
    style: DecompStyle,
    state: ViewState,
    /// The variable under the caret, kept as the caret moves. The right-click
    /// menu is built while the editor is busy and cannot be read, so it reads this.
    at_caret: Option<Variable>,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
    _caret: Subscription,
}

impl DecompilerView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| code_editor(window, cx));
        let caret = cx.observe(&editor, |view, _, cx| {
            view.at_caret = match view.pointed(cx) {
                Pointed::Variable(variable) => Some(variable),
                _ => None,
            };
        });
        Self {
            engine: None,
            editor,
            function: None,
            variables: None,
            engine_version: None,
            style: DecompStyle::default(),
            state: ViewState::Empty,
            at_caret: None,
            focus_handle: cx.focus_handle(),
            _request: None,
            _caret: caret,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.function = None;
        self.variables = None;
        self.engine_version = None;
        self.state = ViewState::Empty;
        self._request = None;
        cx.notify();
    }

    /// Decompile the function the location lies in. An address no listed
    /// function covers is said to be one, not decompiled from a guessed start.
    pub fn show(&mut self, location: &Location, window: &mut Window, cx: &mut Context<Self>) {
        match &location.function {
            Some(f) if self.function.as_ref() == Some(f) && !matches!(self.state, ViewState::Failed(_)) => {}
            Some(f) => {
                self.function = Some(f.clone());
                self.request(window, cx);
            }
            None => {
                self.function = None;
                self._request = None;
                self.state = ViewState::NoFunction(location.va);
                cx.notify();
            }
        }
    }

    /// The function shown, if any.
    pub fn function(&self) -> Option<&FunctionEntry> {
        self.function.as_ref()
    }

    /// The shown function's variables as the engine listed them: `None` while
    /// nothing is shown, or when the engine does not list them.
    pub fn variables(&self) -> Option<&[Variable]> {
        match self.state {
            ViewState::Ready { .. } => self.variables.as_deref(),
            _ => None,
        }
    }

    /// The version the engine gave for itself when it last answered.
    pub fn engine_version(&self) -> Option<&str> {
        self.engine_version.as_deref()
    }

    /// Whether a decompilation is shown (not loading, not failed, not empty).
    pub fn is_ready(&self) -> bool {
        matches!(self.state, ViewState::Ready { .. })
    }

    pub fn style(&self) -> DecompStyle {
        self.style
    }

    /// Decompile the shown function again: something it shows has changed.
    pub fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.function.is_some() {
            self.request(window, cx);
        }
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
                        view.variables = decompiled.variables;
                        view.engine_version = decompiled.engine_version;
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

    /// The variable under the caret, or the one selected.
    fn pointed(&self, cx: &App) -> Pointed {
        if !matches!(self.state, ViewState::Ready { .. }) {
            return Pointed::NotAVariable;
        }
        let editor = self.editor.read(cx);
        let text = editor.value();
        let range = editor.selected_range();
        let word = if range.is_empty() { word_at(&text, editor.cursor()) } else { text.get(range).map(str::trim) };
        let Some(word) = word.filter(|w| !w.is_empty()) else { return Pointed::NotAVariable };
        let Some(variables) = &self.variables else { return Pointed::Unlisted };
        let hits: Vec<&Variable> = variables.iter().filter(|v| v.name == word).collect();
        match hits.as_slice() {
            [] => Pointed::NotAVariable,
            [first, rest @ ..] if rest.iter().all(|v| v.key == first.key) => Pointed::Variable((*first).clone()),
            _ => Pointed::Ambiguous(word.to_string()),
        }
    }

    fn on_rename(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_pointed(VariableIntent::Rename, window, cx);
    }

    fn on_set_type(&mut self, _: &SetType, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_pointed(VariableIntent::SetType, window, cx);
    }

    /// Hand a variable under the caret to the workbench. Anything else goes on
    /// to the workbench's own handling (the function is renamed instead).
    fn edit_pointed(&mut self, intent: VariableIntent, window: &mut Window, cx: &mut Context<Self>) {
        match self.pointed(cx) {
            Pointed::Variable(variable) => {
                if intent == VariableIntent::SetType && !variable.kind.takes_a_type() {
                    let text = format!(
                        "{} is neither a parameter nor a local. The engine applies a type only to those, so a type set on it would change nothing.",
                        variable.name
                    );
                    window.push_notification(SharedString::from(text), cx);
                    return;
                }
                if let Some(function) = self.function.clone() {
                    cx.emit(EditVariable { function, variable, intent });
                }
            }
            Pointed::Ambiguous(word) => {
                let text = format!("More than one variable is shown as {word}; point at it in another style, or rename one of them first.");
                window.push_notification(SharedString::from(text), cx);
            }
            Pointed::NotAVariable | Pointed::Unlisted => cx.propagate(),
        }
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
            .children(self.function.as_ref().and_then(|f| parse_va(&f.va)).map(|va| {
                // The Tauri build's "CFG ↗": the same function as a graph.
                Button::new("show-graph")
                    .icon(Icon::new(AppIcon::Graph))
                    .label("Graph")
                    .xsmall()
                    .ghost()
                    .tooltip("Show this function's control-flow graph")
                    .on_click(move |_, window, cx| {
                        window.dispatch_action(Box::new(ShowAt { va, panel: Some(PanelKind::Graph.name().into()) }), cx);
                    })
            }))
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
            ViewState::NoFunction(va) => message(
                format!("No listed function covers {}, so there is nothing to decompile here.", crate::nav::hex(*va)),
                false,
            )
            .into_any_element(),
            ViewState::Loading => {
                let name = self.function.as_ref().map_or(String::new(), |f| format!(" {}", f.name));
                message(format!("Decompiling{name}…"), false).into_any_element()
            }
            ViewState::Failed(e) => message(format!("No decompilation: {e}"), true).into_any_element(),
            ViewState::Ready { .. } => Editor::new(&self.editor)
                .readonly(true)
                .bordered(false)
                .size_full()
                .context_menu({
                    let view = cx.entity().downgrade();
                    move |menu, _, cx| code_menu(menu, &view, cx)
                })
                .into_any_element(),
        };
        v_flex()
            .size_full()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_rename))
            .on_action(cx.listener(Self::on_set_type))
            .child(self.render_header(cx))
            .child(div().flex_1().min_h_0().child(body))
    }
}

impl EventEmitter<EditVariable> for DecompilerView {}

/// The code's menu. A right click moves the caret, so a variable under it is
/// offered by name; the rest is about the function shown.
fn code_menu(menu: NativeMenu, view: &WeakEntity<DecompilerView>, cx: &App) -> NativeMenu {
    let copying = |menu: NativeMenu| menu.menu_with_icon("Copy", Icon::new(AppIcon::Copy), Box::new(input::Copy)).menu("Select All", Box::new(input::SelectAll));
    let Some(view) = view.upgrade() else { return copying(menu) };
    let view = view.read(cx);
    let mut menu = menu;
    if let Some(variable) = view.at_caret.clone() {
        menu = menu.menu_with_icon(format!("Rename Variable {}…", variable.name), Icon::new(AppIcon::Rename), Box::new(Rename));
        if variable.kind.takes_a_type() {
            menu = menu.menu_with_icon(format!("Set Type of {}…", variable.name), Icon::new(AppIcon::Type), Box::new(SetType));
        }
        menu = menu.separator();
    }
    let start = view.function.as_ref().and_then(|f| parse_va(&f.va).map(|va| (f.name.clone(), va)));
    let Some((name, va)) = start else { return copying(menu) };
    let edit = |label: String, icon: AppIcon, edit: EditKind| (label, icon, Box::new(EditAt { va, edit }) as Box<dyn Action>);
    for (label, icon, action) in [
        edit(format!("Rename Function {name}…"), AppIcon::Rename, EditKind::Rename),
        edit("Set Return Type…".into(), AppIcon::Type, EditKind::ReturnType),
        edit("Comment…".into(), AppIcon::Comment, EditKind::Comment),
        edit("Toggle Bookmark".into(), AppIcon::Bookmark, EditKind::Bookmark),
    ] {
        menu = menu.menu_with_icon(label, Icon::new(icon), action);
    }
    menu = copying(menu.separator()).separator();
    for (label, panel) in [
        ("Show in Graph", PanelKind::Graph),
        ("Show in Disassembly", PanelKind::Disassembly),
        ("Show in Linear Listing", PanelKind::Linear),
        ("Show Bytes in Hex", PanelKind::Hex),
        ("Show References", PanelKind::Xrefs),
    ] {
        menu = menu.menu_with_icon(label, Icon::new(panel.icon()), Box::new(ShowAt { va, panel: Some(panel.name().into()) }));
    }
    menu
}

dock_panel!(DecompilerView, PanelKind::Decompiler);

/// The word around byte offset `at` in `text`: letters, digits, `_`, and `.`
/// inside a word (an SSA name carries a version, `rsi.2`). A caret just after
/// a word counts as on it. This only picks the word; whether it is a variable
/// is the engine's answer.
pub fn word_at(text: &str, at: usize) -> Option<&str> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '.';
    let at = at.min(text.len());
    if !text.is_char_boundary(at) {
        return None;
    }
    let start = text[..at].char_indices().rev().take_while(|&(_, c)| is_word(c)).last().map_or(at, |(i, _)| i);
    let end = text[at..].char_indices().find(|&(_, c)| !is_word(c)).map_or(text.len(), |(i, _)| at + i);
    let word = text[start..end].trim_matches('.');
    (!word.is_empty()).then_some(word)
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::word_at;

    #[test]
    fn the_word_under_the_caret_includes_an_ssa_version() {
        let line = "    rsi.5 = ((rsi.3 + count) >> 0x1);";
        assert_eq!(word_at(line, 6), Some("rsi.5"));
        assert_eq!(word_at(line, 9), Some("rsi.5"), "a caret just after the word is on it");
        assert_eq!(word_at(line, line.find("count").unwrap() + 2), Some("count"));
        assert_eq!(word_at(line, 0), None, "blank space is no word");
        assert_eq!(word_at("x = лічильник;", 6), Some("лічильник"));
        assert_eq!(word_at("end.", 4), Some("end"), "a sentence's full stop is not part of the word");
        assert_eq!(word_at("x", 99), Some("x"), "past the end clamps");
    }
}
