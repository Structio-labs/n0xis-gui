// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The shown function's variables, as the engine listed them with its
//! decompilation: the name on the page, the key a rename or a type is stored
//! under, and whether it is a parameter, a local or another value. Nothing is
//! asked of the engine here; the list is the decompiler's own answer, so the
//! two cannot disagree. Renaming and typing go through the same edits as F2.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use n0xis_client::{Variable, VariableKind};

use crate::decompiler::{DecompilerView, EditVariable, VariableIntent};
use crate::layout::PanelKind;
use crate::panel::{dock_panel, header, message, mono};

const ROW_HEIGHT: Pixels = px(26.);

pub struct VariablesView {
    decompiler: Entity<DecompilerView>,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _shown: Subscription,
}

impl EventEmitter<EditVariable> for VariablesView {}

impl VariablesView {
    pub fn new(decompiler: Entity<DecompilerView>, cx: &mut Context<Self>) -> Self {
        // Whatever the decompiler shows next, this lists.
        let shown = cx.observe(&decompiler, |_, _, cx| cx.notify());
        Self { decompiler, scroll: UniformListScrollHandle::new(), focus_handle: cx.focus_handle(), _shown: shown }
    }

    fn edit(&self, variable: Variable, intent: VariableIntent, cx: &mut Context<Self>) {
        let Some(function) = self.decompiler.read(cx).function().cloned() else { return };
        cx.emit(EditVariable { function, variable, intent });
    }

    fn render_row(&self, variable: Variable, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let kind = match variable.kind {
            VariableKind::Param => "param",
            VariableKind::Local => "local",
            VariableKind::Value => "value",
            VariableKind::Other => "other",
        };
        let renamed = variable.name != variable.key;
        let typed = variable.kind.takes_a_type();
        let (rename, set_type) = (variable.clone(), variable.clone());
        h_flex()
            .id(("variable", ix))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_2()
            .text_sm()
            .hover(|s| s.bg(theme.list_hover))
            .child(div().w(px(48.)).flex_none().text_xs().text_color(theme.muted_foreground).child(kind))
            .child(mono(variable.name.clone(), cx).flex_1().min_w_0().truncate())
            .children(renamed.then(|| div().text_xs().text_color(theme.muted_foreground).child(format!("was {}", variable.key))))
            .child(
                Button::new(SharedString::from(format!("variable-rename-{ix}")))
                    .xsmall()
                    .ghost()
                    .label("Rename…")
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.edit(rename.clone(), VariableIntent::Rename, cx))),
            )
            .children(typed.then(|| {
                Button::new(SharedString::from(format!("variable-type-{ix}")))
                    .xsmall()
                    .ghost()
                    .label("Type…")
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.edit(set_type.clone(), VariableIntent::SetType, cx)))
            }))
            .into_any_element()
    }
}

impl Render for VariablesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let decompiler = self.decompiler.read(cx);
        let title = decompiler.function().map_or("Variables".to_string(), |f| format!("Variables · {}", f.name));
        let style = decompiler.style().as_str();
        let state = (decompiler.function().is_some(), decompiler.is_ready(), decompiler.variables().map(<[Variable]>::to_vec));
        let body = match state {
            (false, _, _) => message("Select a function to see its variables.", false, cx).into_any_element(),
            (true, false, _) => message("Waiting for the decompiler…", false, cx).into_any_element(),
            (true, true, None) => message("This engine does not list a function's variables (0.3.3 and earlier do not).", false, cx).into_any_element(),
            (true, true, Some(vars)) if vars.is_empty() => message("The decompiler shows no variables in this function.", false, cx).into_any_element(),
            (true, true, Some(vars)) => uniform_list(
                "variables",
                vars.len(),
                cx.processor(move |view, range: std::ops::Range<usize>, _, cx| {
                    range.filter_map(|ix| vars.get(ix).cloned().map(|v| view.render_row(v, ix, cx))).collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.scroll)
            .flex_1()
            .into_any_element(),
        };
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header(title, cx).child(div().text_xs().text_color(cx.theme().muted_foreground).child(format!("as the {style} view names them"))))
            .child(body)
    }
}

dock_panel!(VariablesView, PanelKind::Variables);
