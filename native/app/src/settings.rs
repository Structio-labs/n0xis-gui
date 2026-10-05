// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The settings window (File ▸ Settings…, Ctrl+,): analysis, where projects
//! live and what their caches take, and appearance. A change is kept at once.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{CacheReport, CacheUsage, Engine};

use crate::appearance::{self, SelectTheme, Themes};
use crate::prefs::{self, Prefs, ProjectLocation};

/// The open target's cache usage as the engine last reported it.
#[derive(Default)]
pub struct CacheUsageNow {
    /// `None` until asked; then the report or why there is none.
    report: Option<Result<CacheReport, String>>,
    /// A request is on its way.
    asking: bool,
}

impl Global for CacheUsageNow {}

/// Open the settings window. `engine` is the open target's session, for its
/// cache usage; without one the cache section says there is no target.
pub fn open(engine: Option<Arc<Engine>>, window: &mut Window, cx: &mut App) {
    cx.set_global(CacheUsageNow::default());
    if let Some(engine) = engine.clone() {
        ask_usage(engine, false, cx);
    }
    window.open_dialog(cx, move |dialog, _, _| {
        let engine = engine.clone();
        dialog
            .title("Settings")
            .w(px(900.))
            .child(div().h(px(560.)).child(Settings::new("n0xis-settings").pages(pages(engine))))
    });
}

/// Ask the engine what the caches take, clearing them first when `clear`.
fn ask_usage(engine: Arc<Engine>, clear: bool, cx: &mut App) {
    cx.update_global::<CacheUsageNow, _>(|usage, _| usage.asking = true);
    let pending = engine.send(&CacheUsage { clear });
    cx.spawn(async move |cx| {
        let report = pending.await.map_err(|e| e.to_string());
        cx.update(|cx| {
            cx.set_global(CacheUsageNow { report: Some(report), asking: false });
            cx.refresh_windows();
        });
    })
    .detach();
}

/// Keep a change to the settings, saying so if it cannot be saved.
fn change(cx: &mut App, edit: impl FnOnce(&mut Prefs)) {
    if let Err(e) = prefs::update(cx, edit) {
        let text = SharedString::from(format!("The settings could not be saved: {e}"));
        if let Some(window) = cx.active_window() {
            window.update(cx, |_, window, cx| window.push_notification(text, cx)).ok();
        }
    }
}

fn pages(engine: Option<Arc<Engine>>) -> Vec<SettingPage> {
    vec![analysis_page(), projects_page(engine), appearance_page()]
}

fn analysis_page() -> SettingPage {
    SettingPage::new("Analysis").default_open(true).group(
        SettingGroup::new().title("When a target opens").items([
            SettingItem::new(
                "Run the analysis",
                SettingField::switch(|cx| cx.global::<Prefs>().analyze_on_open, |on, cx| change(cx, |p| p.analyze_on_open = on)),
            )
            .description(
                "The engine's whole-program pass, beside the session: it finds the functions and the class names, and writes \
                 the reference index Cross-references reads. Off, references are looked up by the session itself, which can \
                 take long on a large target. Analyze ▸ Run Analysis starts it by hand.",
            ),
            SettingItem::new(
                "Warm the decompiler cache",
                SettingField::switch(|cx| cx.global::<Prefs>().warm_up, |on, cx| change(cx, |p| p.warm_up = on)),
            )
            .description(
                "Let that pass also decode and cache every function, so the first decompile of any of them is instant. It costs \
                 time and disk when the target opens.",
            ),
        ]),
    )
}

const CENTRAL: &str = "central";
const BESIDE: &str = "beside";
const FOLDER: &str = "folder";

fn projects_page(engine: Option<Arc<Engine>>) -> SettingPage {
    let location = SettingField::dropdown(
        vec![
            (CENTRAL.into(), "Central".into()),
            (BESIDE.into(), "Beside the target".into()),
            (FOLDER.into(), "A folder you choose…".into()),
        ],
        |cx| {
            match cx.global::<Prefs>().project_location {
                ProjectLocation::Central => CENTRAL,
                ProjectLocation::BesideTheTarget => BESIDE,
                ProjectLocation::Folder { .. } => FOLDER,
            }
            .into()
        },
        |choice, cx| match choice.as_ref() {
            CENTRAL => change(cx, |p| p.project_location = ProjectLocation::Central),
            BESIDE => change(cx, |p| p.project_location = ProjectLocation::BesideTheTarget),
            _ => choose_folder(cx),
        },
    );
    let usage = SettingItem::render(move |_, _, cx| render_usage(engine.clone(), cx));
    SettingPage::new("Projects and cache").groups([
        SettingGroup::new().title("Where projects live").items([
            SettingItem::new("Project folder", location).description(
                "A project is a target's names, comments, types and caches. Central keeps one folder per target under your \
                 data folder; beside the target, a <file>.n0xis folder next to it; or one per target in a folder you choose. \
                 A change applies to the next target opened, and starts a separate project for it: what you wrote stays in the \
                 project it was written in.",
            ),
            SettingItem::render(|_, _, cx| {
                let text = match &cx.global::<Prefs>().project_location {
                    ProjectLocation::Folder { path } => format!("Projects go under {}", path.display()),
                    _ => String::new(),
                };
                div().text_xs().text_color(cx.theme().muted_foreground).child(text)
            }),
        ]),
        SettingGroup::new().title("Cache").items([
            SettingItem::new(
                "Keep the cache when a target closes",
                SettingField::switch(|cx| cx.global::<Prefs>().keep_cache_on_close, |on, cx| change(cx, |p| p.keep_cache_on_close = on)),
            )
            .description(
                "Off clears a target's caches when another target is opened or the app quits. Only what the engine can \
                 rebuild is a cache; your names, comments and types are never cleared.",
            ),
            usage,
        ]),
    ])
}

/// Ask for a folder; the choice stands only if one was picked.
fn choose_folder(cx: &mut App) {
    let answer = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Keep projects here".into()),
    });
    cx.spawn(async move |cx| {
        if let Ok(Ok(Some(paths))) = answer.await
            && let Some(path) = paths.into_iter().next()
        {
            cx.update(|cx| {
                change(cx, |p| p.project_location = ProjectLocation::Folder { path });
                cx.refresh_windows();
            });
        }
    })
    .detach();
}

/// What the open target's caches take, by kind, with a button to clear them.
fn render_usage(engine: Option<Arc<Engine>>, cx: &mut App) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let usage = cx.global::<CacheUsageNow>();
    let Some(engine) = engine else {
        return div().text_sm().text_color(muted).child("Open a target to see what its caches take.").into_any_element();
    };
    let summary = match (&usage.report, usage.asking) {
        (_, true) => "Asking the engine…".to_string(),
        (None, false) => String::new(),
        (Some(Err(e)), false) => format!("The engine could not say: {e}"),
        (Some(Ok(r)), false) if r.cleared => {
            let mut text = format!("Cleared {} from this target's caches, in {}.", size(r.freed), r.dir);
            if !r.failures.is_empty() {
                text.push_str(&format!(" {} file(s) could not be removed: {}", r.failures.len(), r.failures.join("; ")));
            }
            text
        }
        (Some(Ok(r)), false) => {
            let kinds: Vec<String> = r.caches.iter().filter(|c| c.files > 0).map(|c| format!("{} {}", c.name, size(c.bytes))).collect();
            let mut text = format!("This target's caches take {}", size(r.bytes));
            if !kinds.is_empty() {
                text.push_str(&format!(" ({})", kinds.join(", ")));
            }
            text.push_str(&format!(", in {}.", r.dir));
            if !r.failures.is_empty() {
                text.push_str(&format!(" {} file(s) could not be removed: {}", r.failures.len(), r.failures.join("; ")));
            }
            if !r.is_local {
                text.push_str(" This is the engine's global project, not the target's: the session is not where it should be.");
            }
            text
        }
    };
    h_flex()
        .w_full()
        .gap_3()
        .child(div().flex_1().min_w_0().text_sm().text_color(muted).child(summary))
        .child(
            Button::new("clear-cache")
                .small()
                .outline()
                .label("Clear now")
                .on_click(move |_, _, cx| ask_usage(Arc::clone(&engine), true, cx)),
        )
        .into_any_element()
}

/// Bytes for a person: `3.2 MB`.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000. && unit < UNITS.len() - 1 {
        value /= 1000.;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}

fn appearance_page() -> SettingPage {
    SettingPage::new("Appearance").group(SettingGroup::new().items([
        SettingItem::render(|_, _, cx| {
            // The theme list can change while the window is open (a theme file
            // added); it is read at render time.
            let names: Vec<SharedString> = cx.global::<Themes>().list().iter().map(|t| t.name.clone()).collect();
            v_flex().gap_1().child(div().text_sm().child("Theme")).child(
                h_flex().flex_wrap().gap_1().children(names.into_iter().map(|name| {
                    let current = appearance::current_theme(cx) == name;
                    Button::new(SharedString::from(format!("theme-{name}")))
                        .small()
                        .when(current, |b| b.primary())
                        .when(!current, |b| b.ghost())
                        .label(name.clone())
                        .on_click(move |_, _, cx| cx.dispatch_action(&SelectTheme(name.clone())))
                })),
            )
        }),
        SettingItem::new(
            "Font size",
            SettingField::number_input(
                NumberFieldOptions { min: f64::from(appearance::FONT_SIZE_MIN), max: f64::from(appearance::FONT_SIZE_MAX), step: f64::from(appearance::FONT_SIZE_STEP) },
                |cx| f64::from(cx.theme().font_size.as_f32()),
                |size, cx| {
                    appearance::set_font_size(size as f32, cx);
                    if let Err(e) = appearance::save(cx) {
                        let text = SharedString::from(format!("The appearance could not be saved: {e}"));
                        if let Some(window) = cx.active_window() {
                            window.update(cx, |_, window, cx| window.push_notification(text, cx)).ok();
                        }
                    }
                },
            ),
        )
        .description("Ctrl+= and Ctrl+- change it too; Ctrl+0 resets it."),
    ]))
}

#[cfg(test)]
mod tests {
    use super::size;

    #[test]
    fn sizes_read_as_a_person_would_say_them() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(3_203_354), "3.2 MB");
        assert_eq!(size(1_000), "1.0 KB");
    }
}
