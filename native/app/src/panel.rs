// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! What every dockable view shares: the dock's traits, written once, and the
//! small pieces of rendering each view repeats.

use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::*;

/// Make a view dockable. `$kind` is its [`crate::layout::PanelKind`], which
/// holds the name saved layouts know it by and its tab title. Every panel is a
/// widget like any other: it can be closed, and opened again from View ▸ Panels. With
/// `on_active`, the dock's "now shown" / "now hidden" calls reach the view's own
/// `on_active(&mut self, bool, &mut Window, &mut Context<Self>)`, so a hidden
/// tab can leave the engine alone until it is looked at.
macro_rules! dock_panel {
    ($view:ty, $kind:expr) => {
        dock_panel!(@common $view, $kind);
        impl gpui_kit::base::dock::Panel for $view {
            fn panel_name(&self) -> &'static str {
                $kind.name()
            }
        }
    };
    ($view:ty, $kind:expr, on_active) => {
        dock_panel!(@common $view, $kind);
        impl gpui_kit::base::dock::Panel for $view {
            fn panel_name(&self) -> &'static str {
                $kind.name()
            }
            fn set_active(&mut self, active: bool, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Self>) {
                self.on_active(active, window, cx);
            }
        }
    };
    (@common $view:ty, $kind:expr) => {
        impl gpui_kit::Focusable for $view {
            fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
                self.focus_handle.clone()
            }
        }
        impl gpui_kit::EventEmitter<gpui_kit::base::dock::PanelEvent> for $view {}
        impl gpui_kit::component::dock::Panel for $view {
            fn title(&mut self, _: &mut gpui_kit::Window, _: &mut gpui_kit::Context<Self>) -> impl gpui_kit::IntoElement {
                $kind.title()
            }
        }
    };
}
pub(crate) use dock_panel;

/// A view's text where its content would be: what it is waiting for, or why
/// there is nothing. `danger` marks a failure.
pub fn message(text: impl Into<SharedString>, danger: bool, cx: &App) -> Div {
    let theme = cx.theme();
    div().p_4().text_sm().text_color(if danger { theme.danger } else { theme.muted_foreground }).child(text.into())
}

/// The strip at the top of a view: a title, then whatever the view adds on the right.
pub fn header(title: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    h_flex()
        .px_2()
        .py_1()
        .gap_2()
        .border_b_1()
        .border_color(theme.border)
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).truncate().child(title.into()))
        .child(div().flex_1())
}

/// Text in the code font.
pub fn mono(text: impl Into<SharedString>, cx: &App) -> Div {
    div().font_family(cx.theme().mono_font_family.clone()).text_xs().child(text.into())
}

/// Cut `text` to at most `max` bytes on a character boundary, and say how much
/// was left out. Large engine answers are shown in part, never silently.
pub fn clip(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut cut = max;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}\n… {} more bytes not shown", &text[..cut], text.len() - cut)
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::clip;

    #[test]
    fn a_clipped_text_says_how_much_was_left_out() {
        assert_eq!(clip("short", 10), "short");
        assert_eq!(clip("abcdef", 4), "abcd\n… 2 more bytes not shown");
        // Never cut inside a character.
        assert!(clip("ééé", 3).starts_with("é\n"));
    }
}
