// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The command palette (Ctrl+Shift+P, Ctrl+P, F1): every command of the menus
//! and keys, the panels, the workspaces, the themes, and the yes/no settings
//! with their values, filtered as you type. The commands are the key table's
//! (`keymap::COMMANDS`), so the palette, the menus and Settings ▸ Keys list
//! the same ones. It is drawn inside the workbench, so the command it runs
//! goes where a menu item's or a key's would.

use gpui_kit::component::Icon;
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::*;

use crate::ClearCaches;
use crate::appearance::{self, SelectTheme, Themes};
use crate::assets::AppIcon;
use crate::keymap::{self, COMMANDS};
use crate::layout::{PanelKind, ShowPanel, SwitchWorkspace, Workspace};
use crate::prefs::{Prefs, SWITCHES, TogglePref};

/// How tall the list may grow before it scrolls.
const LIST_HEIGHT: Pixels = px(420.);

pub struct Palette {
    state: Entity<CommandState>,
}

impl Palette {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        Self { state: cx.new(|cx| CommandState::new(window, cx)) }
    }

    /// Put the caret in the query field.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        let field = self.state.read(cx).focus_handle(cx);
        field.focus(window, cx);
    }

    /// The palette as things are now: the current workspace, theme and each
    /// setting are marked. `close` runs once a command was chosen or the
    /// palette was dismissed.
    pub fn render(&self, workspace: Workspace, has_target: bool, close: impl Fn(&mut Window, &mut App) + Clone + 'static, cx: &mut App) -> Command {
        let mut palette = Command::new(&self.state).placeholder("Type a command, a panel, a theme or a setting").bordered(false).max_h(LIST_HEIGHT);
        for group in keymap::groups() {
            let items = COMMANDS.iter().filter(|c| c.group == group).map(|c| CommandItem::new().label(c.label).action((c.action)()));
            palette = palette.group(CommandGroup::new().label(group).items(items));
        }
        let panels = PanelKind::ALL.map(|kind| {
            CommandItem::new().label(format!("Show {}", kind.title())).icon(Icon::new(kind.icon())).keywords(["panel"]).action(Box::new(ShowPanel(kind.name().into())))
        });
        palette = palette.group(CommandGroup::new().label("Panels").items(panels));
        let workspaces = Workspace::ALL.map(|w| {
            CommandItem::new().label(format!("Workspace: {}", w.title())).checked(w == workspace).action(Box::new(SwitchWorkspace(w.name().into())))
        });
        palette = palette.group(CommandGroup::new().label("Workspaces").items(workspaces));
        let current = appearance::current_theme(cx);
        let themes: Vec<CommandItem> = cx
            .global::<Themes>()
            .list()
            .iter()
            .map(|t| CommandItem::new().label(format!("Theme: {}", t.name)).checked(t.name == current).action(Box::new(SelectTheme(t.name.clone()))))
            .collect();
        palette = palette.group(CommandGroup::new().label("Themes").items(themes));
        let mut prefs = cx.global::<Prefs>().clone();
        let mut settings: Vec<CommandItem> = SWITCHES
            .iter()
            .map(|(name, label)| {
                let on = prefs.switch(name).is_some_and(|value| *value);
                CommandItem::new().label(*label).checked(on).keywords(["setting"]).action(Box::new(TogglePref((*name).into())))
            })
            .collect();
        if has_target {
            settings.push(CommandItem::new().label("Clear this target's caches").icon(Icon::new(AppIcon::Trash)).keywords(["setting", "cache"]).action(Box::new(ClearCaches)));
        }
        palette = palette.group(CommandGroup::new().label("Settings").items(settings));
        let dismiss = close.clone();
        palette.on_confirm(move |_, window, cx| close(window, cx)).on_cancel(move |window, cx| dismiss(window, cx))
    }
}

impl Palette {
    /// Whether the caret is still in the palette: a command that moved it
    /// elsewhere (a prompt, a field) keeps it there when the palette closes.
    pub fn has_focus(&self, window: &Window, cx: &App) -> bool {
        self.state.read(cx).focus_handle(cx).contains_focused(window, cx)
    }
}
