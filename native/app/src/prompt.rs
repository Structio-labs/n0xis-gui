// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! One line of text asked for in a dialog: a new name, a comment, a type, an
//! address to go to. Enter confirms, Escape cancels.

use std::rc::Rc;

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{WindowExt as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub struct Prompt {
    pub title: String,
    /// A line under the title saying what the answer changes.
    pub detail: Option<String>,
    /// What the field holds when it opens, selected so typing replaces it.
    pub initial: String,
    pub placeholder: &'static str,
    /// The confirm button's label.
    pub ok: &'static str,
}

/// Ask for one line. `on_answer` gets it as typed, once the user confirms.
pub fn ask(prompt: Prompt, window: &mut Window, cx: &mut App, on_answer: impl Fn(String, &mut Window, &mut App) + 'static) {
    let field = cx.new(|cx| InputState::new(window, cx).placeholder(prompt.placeholder).default_value(prompt.initial.clone()));
    let on_answer = Rc::new(on_answer);
    let dialog_field = field.clone();
    window.open_alert_dialog(cx, move |alert, _, _| {
        let field = dialog_field.clone();
        let on_answer = Rc::clone(&on_answer);
        alert
            .title(prompt.title.clone())
            .when_some(prompt.detail.clone(), |alert, detail| alert.description(detail))
            .confirm()
            .ok_text(prompt.ok)
            .child(v_flex().pt_2().child(Input::new(&dialog_field)))
            .on_ok(move |_, window, cx| {
                let text = field.read(cx).value().to_string();
                on_answer(text, window, cx);
                true
            })
    });
    field.update(cx, |state, cx| {
        state.focus(window, cx);
        state.select_all(window, cx);
    });
}

/// A typed answer as the value to store: surrounding blanks dropped, and an
/// empty answer meaning "clear it".
pub fn as_value(answer: &str) -> Option<String> {
    let answer = answer.trim();
    (!answer.is_empty()).then(|| answer.to_string())
}

#[cfg(test)]
mod tests {
    use super::as_value;

    #[test]
    fn an_empty_answer_clears_and_blanks_are_dropped() {
        assert_eq!(as_value("  parse_header "), Some("parse_header".into()));
        assert_eq!(as_value("   "), None);
        assert_eq!(as_value(""), None);
    }
}
