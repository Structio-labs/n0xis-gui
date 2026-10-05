// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Themes and the base font size: the built-in themes, the user's own theme
//! files, and the choice of both kept between runs.
//!
//! User themes live in `<config>/themes/*.json` (the GPUI Kit theme format).
//! The folder is re-read when a file changes and the theme in use is applied
//! again, so a palette can be tuned with the window open. The theme list is
//! kept here rather than in the library's registry, whose folder reload drops
//! themes added from code and does not re-apply the one in use.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeConfig, ThemeSet};
use gpui_kit::*;
use serde::{Deserialize, Serialize};

/// The Tauri build's six themes, generated from its stylesheet by
/// `themes/build_themes.py`, so their colours are the ones users know.
const BUILTIN_THEMES: &str = include_str!("../themes/n0xis.json");

pub const DEFAULT_THEME: &str = "Midnight";

/// Bounds and step of the base font size, in pixels. Everything sized in rems
/// (most of the interface) scales with it; code text moves by the same amount.
pub const FONT_SIZE_MIN: f32 = 11.;
pub const FONT_SIZE_MAX: f32 = 22.;
pub const FONT_SIZE_STEP: f32 = 1.;

/// How often the user's theme folder is checked for changes.
pub const THEME_POLL: Duration = Duration::from_secs(1);

#[derive(Clone, PartialEq, Deserialize, Action)]
#[action(namespace = n0xis, no_json)]
pub struct SelectTheme(pub SharedString);

gpui_kit::actions!(n0xis, [ZoomIn, ZoomOut, ResetZoom, ToggleRoundedEdges]);

/// Interface scales offered as presets, in percent of the theme's own size.
pub const SCALES: [u32; 4] = [90, 100, 110, 125];

/// How the control-flow graph draws its edges: square corners, or rounded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphLook {
    pub rounded_edges: bool,
}

impl Global for GraphLook {}

/// A file in the themes folder, as last seen: its path, modification time and length.
type FileStamp = (PathBuf, Option<SystemTime>, u64);

/// Every theme on offer, built-in first, then the user's; a user theme with a
/// built-in's name replaces it.
pub struct Themes {
    list: Vec<Rc<ThemeConfig>>,
    seen: Vec<FileStamp>,
    default_font_size: f32,
    default_mono_font_size: f32,
}

impl Global for Themes {}

impl Themes {
    pub fn list(&self) -> &[Rc<ThemeConfig>] {
        &self.list
    }

    fn find(&self, name: &str) -> Option<Rc<ThemeConfig>> {
        self.list.iter().find(|t| t.name.as_ref() == name).cloned()
    }
}

/// What the user chose, kept in `<config>/ui-settings.json`.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub font_size: Option<f32>,
    /// Round the corners of the graph's edges.
    #[serde(default)]
    pub rounded_edges: bool,
}

pub fn user_themes_dir() -> Option<PathBuf> {
    crate::config::dir().map(|dir| dir.join("themes"))
}

fn settings_file() -> Option<PathBuf> {
    crate::config::dir().map(|dir| dir.join("ui-settings.json"))
}

pub fn builtin_themes() -> Vec<ThemeConfig> {
    // The file is generated and checked by a test; a parse failure here is a
    // build defect, not a user error.
    serde_json::from_str::<ThemeSet>(BUILTIN_THEMES).map(|set| set.themes).unwrap_or_default()
}

/// The user's theme files, and a sentence for each one that could not be read.
pub fn read_user_themes(dir: &Path) -> (Vec<ThemeConfig>, Vec<String>) {
    let (mut themes, mut problems) = (Vec::new(), Vec::new());
    let Ok(entries) = std::fs::read_dir(dir) else { return (themes, problems) };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|json| {
            serde_json::from_str::<ThemeSet>(&json).map_err(|e| e.to_string())
        }) {
            Ok(set) if set.themes.is_empty() => problems.push(format!("Theme file {name} holds no themes.")),
            Ok(set) => themes.extend(set.themes),
            Err(e) => problems.push(format!("Theme file {name} was not loaded: {e}")),
        }
    }
    (themes, problems)
}

/// Built-in themes, then user themes; a user theme replaces a built-in of the same name.
pub fn merge(builtin: Vec<ThemeConfig>, user: Vec<ThemeConfig>) -> Vec<Rc<ThemeConfig>> {
    let mut list: Vec<Rc<ThemeConfig>> = builtin.into_iter().map(Rc::new).collect();
    for theme in user {
        match list.iter().position(|t| t.name == theme.name) {
            Some(i) => list[i] = Rc::new(theme),
            None => list.push(Rc::new(theme)),
        }
    }
    list
}

fn stamps(dir: &Path) -> Vec<FileStamp> {
    let mut seen: Vec<FileStamp> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    Some((e.path(), meta.modified().ok(), meta.len()))
                })
                .collect()
        })
        .unwrap_or_default();
    seen.sort();
    seen
}

pub fn clamp_font_size(size: f32) -> f32 {
    size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX)
}

/// Load the themes and apply the saved choice. Returns the problems found with
/// user theme files, for the window to show.
pub fn init(cx: &mut App) -> Vec<String> {
    let dir = user_themes_dir();
    if let Some(dir) = &dir {
        let _ = std::fs::create_dir_all(dir);
    }
    let (user, problems) = dir.as_deref().map(read_user_themes).unwrap_or_default();
    let themes = Themes {
        list: merge(builtin_themes(), user),
        seen: dir.as_deref().map(stamps).unwrap_or_default(),
        default_font_size: cx.theme().font_size.as_f32(),
        default_mono_font_size: cx.theme().mono_font_size.as_f32(),
    };
    cx.set_global(themes);
    let settings = settings_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
        .unwrap_or_default();
    let wanted = settings.theme.as_deref().unwrap_or(DEFAULT_THEME);
    if !apply_theme(wanted, cx) {
        apply_theme(DEFAULT_THEME, cx);
    }
    if let Some(size) = settings.font_size {
        set_font_size(size, cx);
    }
    cx.set_global(GraphLook { rounded_edges: settings.rounded_edges });
    problems
}

/// Apply the theme called `name`, keeping the chosen font size. False if there
/// is no such theme.
pub fn apply_theme(name: &str, cx: &mut App) -> bool {
    let Some(config) = cx.global::<Themes>().find(name) else { return false };
    let size = cx.theme().font_size.as_f32();
    Theme::update(cx, |theme| {
        // Applying a config fills the slot of its own mode; the mode has to
        // follow, or a light theme would sit unused behind a dark one.
        theme.mode = config.mode;
        theme.apply_config(&config);
    });
    set_font_size(size, cx);
    true
}

pub fn current_theme(cx: &App) -> SharedString {
    cx.theme().theme_name().clone()
}

pub fn set_font_size(size: f32, cx: &mut App) {
    let size = clamp_font_size(size);
    let (base, mono) = {
        let t = cx.global::<Themes>();
        (t.default_font_size, t.default_mono_font_size)
    };
    Theme::update(cx, |theme| {
        theme.font_size = px(size);
        theme.mono_font_size = px((mono + size - base).max(FONT_SIZE_MIN));
    });
}

pub fn zoom(steps: f32, cx: &mut App) {
    let now = cx.theme().font_size.as_f32();
    set_font_size(now + steps * FONT_SIZE_STEP, cx);
}

pub fn reset_zoom(cx: &mut App) {
    let base = cx.global::<Themes>().default_font_size;
    set_font_size(base, cx);
}

/// The interface's size as a percentage of the theme's own, rounded.
pub fn scale(cx: &App) -> u32 {
    let base = cx.global::<Themes>().default_font_size;
    (cx.theme().font_size.as_f32() / base * 100.).round() as u32
}

/// Size the interface at `percent` of the theme's own size.
pub fn set_scale(percent: u32, cx: &mut App) {
    let base = cx.global::<Themes>().default_font_size;
    set_font_size(base * percent as f32 / 100., cx);
}

/// Keep the current choice for the next run.
pub fn save(cx: &App) -> std::io::Result<()> {
    let settings = Settings {
        theme: Some(current_theme(cx).to_string()),
        font_size: Some(cx.theme().font_size.as_f32()),
        rounded_edges: cx.try_global::<GraphLook>().is_some_and(|look| look.rounded_edges),
    };
    let path = settings_file().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config directory"))?;
    let json = serde_json::to_string_pretty(&settings).map_err(std::io::Error::other)?;
    crate::config::write_atomic(&path, &json)
}

/// Re-read the user's theme folder if anything in it changed, and apply the
/// theme in use again so an edited palette shows at once. `None` when nothing
/// changed; otherwise the problems found, possibly none.
pub fn poll_user_themes(cx: &mut App) -> Option<Vec<String>> {
    let dir = user_themes_dir()?;
    let now = stamps(&dir);
    if now == cx.global::<Themes>().seen {
        return None;
    }
    let (user, problems) = read_user_themes(&dir);
    {
        let themes = cx.global_mut::<Themes>();
        themes.list = merge(builtin_themes(), user);
        themes.seen = now;
    }
    let current = current_theme(cx).to_string();
    if !apply_theme(&current, cx) {
        apply_theme(DEFAULT_THEME, cx);
    }
    Some(problems)
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{builtin_themes, clamp_font_size, merge, read_user_themes};
    use gpui_kit::component::ThemeMode;

    #[test]
    fn the_six_builtin_themes_load_with_the_old_stylesheets_colours() {
        let themes = builtin_themes();
        let names: Vec<&str> = themes.iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(names, ["Midnight", "Deep", "Light", "Nebula", "Warm", "Forest"]);
        assert_eq!(themes.iter().filter(|t| t.mode == ThemeMode::Light).count(), 1);
        // Planted in ui/styles.css as Midnight's --bg1 and --acc.
        let midnight = serde_json::to_value(&themes[0]).unwrap();
        assert_eq!(midnight["colors"]["background"], "#0e121a");
        assert_eq!(midnight["colors"]["primary.background"], "#3fdcc4");
    }

    #[test]
    fn a_user_theme_replaces_a_builtin_of_the_same_name_and_adds_the_rest() {
        let dir = std::env::temp_dir().join(format!("n0xis-themes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mine.json"),
            r##"{"name":"Mine","themes":[
                {"name":"Midnight","mode":"dark","colors":{"background":"#101010"}},
                {"name":"Calm","mode":"dark","colors":{"background":"#202020"}}]}"##,
        )
        .unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
        let (user, problems) = read_user_themes(&dir);
        let list = merge(builtin_themes(), user);
        assert_eq!(list.len(), 7, "six built-in, one replaced, one added");
        let midnight = serde_json::to_value(&*list[0]).unwrap();
        assert_eq!(midnight["colors"]["background"], "#101010", "the user's Midnight wins");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("broken.json"), "{problems:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_font_size_stays_inside_its_bounds() {
        assert_eq!(clamp_font_size(4.), super::FONT_SIZE_MIN);
        assert_eq!(clamp_font_size(40.), super::FONT_SIZE_MAX);
        assert_eq!(clamp_font_size(15.), 15.);
    }
}
