// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The function list: every function the engine reports, filterable, rendered
//! a screen at a time however many there are.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{ClientError, DiscoverFunctions, Engine, FunctionEntry};

use crate::assets::AppIcon;
use crate::layout::PanelKind;
use crate::nav::parse_va;
use crate::panel::dock_panel;

/// Functions asked for per request. Large enough to fill the list quickly,
/// small enough that one answer stays a few megabytes.
pub const PAGE_SIZE: u32 = 20_000;

/// The most functions held at once, a guard against a runaway answer. When an
/// image has more, the footer says how many are not shown.
pub const MAX_FUNCTIONS: usize = 400_000;

const ROW_HEIGHT: Pixels = px(24.);

/// The list as data, kept apart from rendering: the entries the engine
/// reported, and which of them match the filter.
#[derive(Default)]
pub struct FunctionIndex {
    entries: Vec<FunctionEntry>,
    /// Lower-cased `name va` per entry, so filtering does not allocate.
    haystack: Vec<String>,
    query: String,
    visible: Vec<usize>,
    /// (start, entry index), sorted by start, for finding the function an
    /// address lies in.
    by_start: Vec<(u64, usize)>,
    /// Bumped whenever `by_start` had to be re-sorted, so a view that numbers
    /// functions by address knows its numbering moved.
    order_generation: u64,
}

impl FunctionIndex {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn visible(&self, row: usize) -> Option<&FunctionEntry> {
        self.visible.get(row).and_then(|&i| self.entries.get(i))
    }

    pub fn first(&self) -> Option<&FunctionEntry> {
        self.entries.first()
    }

    /// Functions that carry a name of their own rather than `sub_<address>`.
    pub fn named(&self) -> usize {
        self.entries.iter().filter(|f| !f.name.starts_with("sub_")).count()
    }

    pub fn extend(&mut self, page: Vec<FunctionEntry>) {
        let was_sorted_to = self.by_start.len();
        for f in page {
            let hay = format!("{} {}", f.name, f.va).to_lowercase();
            if hay.contains(&self.query) {
                self.visible.push(self.entries.len());
            }
            if let Some(start) = parse_va(&f.va) {
                self.by_start.push((start, self.entries.len()));
            }
            self.haystack.push(hay);
            self.entries.push(f);
        }
        // The engine lists in address order; sort only if a page did not.
        if !self.by_start[was_sorted_to.saturating_sub(1)..].is_sorted() {
            self.by_start.sort_unstable();
            self.order_generation += 1;
        }
    }

    /// How many functions have an address, i.e. can be shown in address order.
    pub fn addressed_len(&self) -> usize {
        self.by_start.len()
    }

    /// The `n`th function by address, with its start.
    pub fn by_address(&self, n: usize) -> Option<(u64, &FunctionEntry)> {
        let &(start, ix) = self.by_start.get(n)?;
        Some((start, self.entries.get(ix)?))
    }

    /// The position in address order of the function starting at `va`, or of
    /// the last one starting below it.
    pub fn address_rank(&self, va: u64) -> Option<usize> {
        self.by_start.partition_point(|&(start, _)| start <= va).checked_sub(1)
    }

    pub fn order_generation(&self) -> u64 {
        self.order_generation
    }

    /// The function `va` lies in: the one starting there, or the nearest one
    /// below whose stated end is past `va`. A function with no stated end
    /// covers only its own start; how far it reaches is not guessed here.
    pub fn function_at(&self, va: u64) -> Option<&FunctionEntry> {
        let below = self.by_start.partition_point(|&(start, _)| start <= va);
        let &(start, ix) = self.by_start.get(below.checked_sub(1)?)?;
        let f = self.entries.get(ix)?;
        if start == va {
            return Some(f);
        }
        let end = f.end.as_deref().and_then(parse_va)?;
        (va < end).then_some(f)
    }

    /// The row a function is shown on under the current filter.
    pub fn visible_row(&self, va: u64) -> Option<usize> {
        self.visible.iter().position(|&i| self.entries.get(i).and_then(|f| parse_va(&f.va)) == Some(va))
    }

    /// Where the function starting at `va` sits in the engine's own listing,
    /// which is the order pages arrive in.
    pub fn listing_position(&self, va: u64) -> Option<usize> {
        let at = self.by_start.partition_point(|&(start, _)| start < va);
        self.by_start.get(at).filter(|&&(start, _)| start == va).map(|&(_, ix)| ix)
    }

    /// Put `entry` in place of the listed function at the same address, as the
    /// engine now reports it. Answers whether there was one to replace.
    pub fn replace(&mut self, entry: FunctionEntry) -> bool {
        let Some(ix) = parse_va(&entry.va).and_then(|va| self.listing_position(va)) else { return false };
        let hay = format!("{} {}", entry.name, entry.va).to_lowercase();
        let matches = hay.contains(&self.query);
        let shown = self.visible.binary_search(&ix);
        match (matches, shown) {
            (true, Err(at)) => self.visible.insert(at, ix),
            (false, Ok(at)) => {
                self.visible.remove(at);
            }
            _ => {}
        }
        self.haystack[ix] = hay;
        self.entries[ix] = entry;
        true
    }

    /// Take the engine's current names for the entries from `offset` on, where
    /// the addresses still match. Answers how many changed.
    pub fn rename_from(&mut self, offset: usize, page: Vec<FunctionEntry>) -> usize {
        let mut changed = 0;
        for (i, entry) in page.into_iter().enumerate() {
            let ix = offset + i;
            let Some(old) = self.entries.get(ix) else { break };
            if old.va != entry.va || *old == entry {
                continue;
            }
            self.haystack[ix] = format!("{} {}", entry.name, entry.va).to_lowercase();
            self.entries[ix] = entry;
            changed += 1;
        }
        if changed > 0 && !self.query.is_empty() {
            let query = self.query.clone();
            self.set_query(&query);
        }
        changed
    }

    /// Every listed function called exactly `name`.
    pub fn named_exactly(&self, name: &str) -> Vec<&FunctionEntry> {
        self.entries.iter().filter(|f| f.name == name).collect()
    }

    pub fn set_query(&mut self, query: &str) {
        self.query = query.trim().to_lowercase();
        self.visible = (0..self.entries.len()).filter(|&i| self.haystack[i].contains(&self.query)).collect();
    }
}

/// Where the list came from, shown in the footer: the image's own unwind table
/// states function bounds, the prologue scan infers them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ListSource {
    UnwindTable,
    PrologueScan,
}

pub enum LoadState {
    Idle,
    Loading,
    Done,
    Failed(String),
}

/// Emitted when the user picks a function, and once for the first one loaded.
pub struct FunctionSelected(pub FunctionEntry);

/// Emitted when a page of functions has been added.
pub struct FunctionsLoaded;

/// Emitted when one function's entry was read again and changed.
pub struct FunctionChanged(pub FunctionEntry);

pub struct FunctionList {
    index: FunctionIndex,
    total: Option<u64>,
    source: Option<ListSource>,
    state: LoadState,
    /// The selected function's address: a domain id, not a row number, so a
    /// new filter cannot move the selection to another function.
    selected: Option<String>,
    /// A pass reading the engine's current names into the loaded entries.
    _names: Option<Task<()>>,
    filter: Entity<InputState>,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _load: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<FunctionSelected> for FunctionList {}
impl EventEmitter<FunctionsLoaded> for FunctionList {}
impl EventEmitter<FunctionChanged> for FunctionList {}

impl FunctionList {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name or address"));
        let subscription = cx.subscribe_in(&filter, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                let query = state.read(cx).value().to_string();
                this.index.set_query(&query);
                cx.notify();
            }
        });
        Self {
            index: FunctionIndex::default(),
            total: None,
            source: None,
            state: LoadState::Idle,
            selected: None,
            _names: None,
            filter,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _load: None,
            _subscriptions: vec![subscription],
        }
    }

    /// Page the whole list in from `engine`. The unwind table is tried first; an
    /// image without one (any non-PE, or a PE that has none) is scanned instead.
    pub fn load(&mut self, engine: Arc<Engine>, cx: &mut Context<Self>) {
        self.selected = None;
        self._names = None;
        self.index = FunctionIndex::default();
        self.total = None;
        self.source = None;
        self.state = LoadState::Loading;
        cx.notify();
        self._load = Some(cx.spawn(async move |this, cx| {
            let mut from_table = true;
            let mut offset = 0u64;
            loop {
                let request = DiscoverFunctions { from_unwind_table: from_table, limit: PAGE_SIZE, offset };
                let result = engine.send(&request).await;
                let table_absent = from_table
                    && offset == 0
                    && match &result {
                        Ok(page) => page.functions.is_empty(),
                        Err(ClientError::Engine(_)) => true,
                        Err(_) => false,
                    };
                if table_absent {
                    from_table = false;
                    continue;
                }
                let more = this
                    .update(cx, |list, cx| list.take_page(result, from_table, cx))
                    .unwrap_or(false);
                if !more {
                    break;
                }
                offset += u64::from(PAGE_SIZE);
            }
        }));
    }

    /// Fold one answer into the list; whether to ask for the next page.
    fn take_page(
        &mut self,
        result: Result<n0xis_client::FunctionsPage, ClientError>,
        from_table: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let page = match result {
            Ok(page) => page,
            Err(e) => {
                self.state = LoadState::Failed(e.to_string());
                cx.notify();
                return false;
            }
        };
        let returned = page.functions.len();
        self.source = Some(if from_table { ListSource::UnwindTable } else { ListSource::PrologueScan });
        self.total = page.total.or(self.total);
        let mut functions = page.functions;
        functions.truncate(MAX_FUNCTIONS.saturating_sub(self.index.len()));
        let was_empty = self.index.len() == 0;
        self.index.extend(functions);
        if was_empty && let Some(first) = self.index.first().cloned() {
            self.selected = Some(first.va.clone());
            cx.emit(FunctionSelected(first));
        }
        let more = returned == PAGE_SIZE as usize && self.index.len() < MAX_FUNCTIONS;
        self.state = if more { LoadState::Loading } else { LoadState::Done };
        cx.emit(FunctionsLoaded);
        cx.notify();
        more
    }

    pub fn index(&self) -> &FunctionIndex {
        &self.index
    }

    /// Read the names of the entries loaded so far again, page by page, as the
    /// engine now gives them (after an analysis found class or library names).
    /// The list stays on screen and the selection where it is; pages still to
    /// come arrive with the new names anyway.
    pub fn refresh_names(&mut self, engine: Arc<Engine>, cx: &mut Context<Self>) {
        let (Some(source), loaded) = (self.source, self.index.len()) else { return };
        let from_unwind_table = source == ListSource::UnwindTable;
        self._names = Some(cx.spawn(async move |this, cx| {
            let mut offset = 0usize;
            while offset < loaded {
                let request = DiscoverFunctions { from_unwind_table, limit: PAGE_SIZE, offset: offset as u64 };
                let Ok(page) = engine.send(&request).await else { return };
                let returned = page.functions.len();
                let alive = this.update(cx, |list, cx| {
                    if list.index.rename_from(offset, page.functions) > 0 {
                        cx.emit(FunctionsLoaded);
                        cx.notify();
                    }
                });
                if alive.is_err() || returned < PAGE_SIZE as usize {
                    break;
                }
                offset += PAGE_SIZE as usize;
            }
        }));
    }

    /// Read the function starting at `va` again, from the same listing the
    /// list came from, and show what the engine now calls it.
    pub fn reread(&mut self, engine: Arc<Engine>, va: u64, cx: &mut Context<Self>) {
        let (Some(position), Some(source)) = (self.index.listing_position(va), self.source) else { return };
        let request = DiscoverFunctions {
            from_unwind_table: source == ListSource::UnwindTable,
            limit: 1,
            offset: position as u64,
        };
        let pending = engine.send(&request);
        cx.spawn(async move |this, cx| {
            let Ok(page) = pending.await else { return };
            this.update(cx, |list, cx| {
                // Only an answer for the same address replaces the entry: a
                // listing that moved under it is not guessed at.
                let Some(entry) = page.functions.into_iter().next().filter(|f| parse_va(&f.va) == Some(va)) else { return };
                let changed = list.index.listing_position(va).and_then(|ix| list.index.entries.get(ix)) != Some(&entry);
                if changed && list.index.replace(entry.clone()) {
                    cx.emit(FunctionChanged(entry));
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Mark the function starting at `va` as selected and scroll it into view,
    /// without announcing it: the selection came from elsewhere.
    pub fn reveal(&mut self, va: u64, cx: &mut Context<Self>) {
        self.selected = self.index.function_at(va).map(|f| f.va.clone());
        if let Some(row) = self.selected.as_deref().and_then(parse_va).and_then(|start| self.index.visible_row(start)) {
            self.scroll.scroll_to_item(row, ScrollStrategy::Center);
        }
        cx.notify();
    }

    fn select(&mut self, entry: FunctionEntry, cx: &mut Context<Self>) {
        self.selected = Some(entry.va.clone());
        cx.emit(FunctionSelected(entry));
        cx.notify();
    }

    fn render_row(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(entry) = self.index.visible(row).cloned() else {
            return div().h(ROW_HEIGHT).into_any_element();
        };
        let theme = cx.theme();
        let selected = self.selected.as_deref() == Some(entry.va.as_str());
        let unnamed = entry.name.starts_with("sub_");
        h_flex()
            .id(ElementId::Name(entry.va.clone().into()))
            .w_full()
            .h(ROW_HEIGHT)
            .px_2()
            .gap_2()
            .text_sm()
            .when(selected, |row| row.bg(theme.list_active))
            .when(!selected, |row| row.hover(|s| s.bg(theme.list_hover)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when(unnamed, |name| name.text_color(theme.muted_foreground))
                    .child(entry.name.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(entry.va.clone()),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select(entry.clone(), cx)))
            .into_any_element()
    }

    fn footer_text(&self) -> String {
        let loaded = self.index.len();
        let shown = if self.index.visible_len() == loaded {
            String::new()
        } else {
            format!(" · {} shown", self.index.visible_len())
        };
        let source = match self.source {
            Some(ListSource::UnwindTable) => " · from the unwind table",
            Some(ListSource::PrologueScan) => " · from a prologue scan",
            None => "",
        };
        match &self.state {
            LoadState::Idle => "No target".into(),
            LoadState::Failed(e) => format!("Could not list functions: {e}"),
            LoadState::Loading => match self.total {
                Some(t) => format!("Loading {loaded} of {t}…{source}"),
                None => format!("Loading {loaded}…{source}"),
            },
            LoadState::Done => {
                let cut = self.total.filter(|&t| t > loaded as u64).map(|t| format!(" (of {t}; the rest not loaded)")).unwrap_or_default();
                format!("{loaded} functions{cut}, {} named{shown}{source}", self.index.named())
            }
        }
    }
}

impl Render for FunctionList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let failed = matches!(self.state, LoadState::Failed(_));
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .px_2()
                    .py_2()
                    .child(Input::new(&self.filter).small().prefix(Icon::new(AppIcon::Scan).small())),
            )
            .child(
                uniform_list(
                    "functions",
                    self.index.visible_len(),
                    cx.processor(|this, range: Range<usize>, _window, cx| {
                        range.map(|row| this.render_row(row, cx)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.scroll)
                .flex_1(),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(if failed { theme.danger } else { theme.muted_foreground })
                    .child(self.footer_text()),
            )
    }
}

dock_panel!(FunctionList, PanelKind::Functions);

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::FunctionIndex;
    use n0xis_client::FunctionEntry;

    fn f(name: &str, va: &str) -> FunctionEntry {
        FunctionEntry { name: name.into(), va: va.into(), end: None }
    }

    #[test]
    fn the_filter_matches_names_and_addresses_without_case() {
        let mut ix = FunctionIndex::default();
        ix.extend(vec![f("main", "0x401000"), f("sub_401100", "0x401100"), f("Parse_Header", "0x402000")]);
        ix.set_query("parse");
        assert_eq!(ix.visible_len(), 1);
        assert_eq!(ix.visible(0).unwrap().name, "Parse_Header");
        ix.set_query("0x4011");
        assert_eq!(ix.visible(0).unwrap().name, "sub_401100");
        ix.set_query("");
        assert_eq!(ix.visible_len(), 3);
    }

    #[test]
    fn pages_arriving_under_a_filter_are_filtered_too() {
        let mut ix = FunctionIndex::default();
        ix.set_query("init");
        ix.extend(vec![f("init_a", "0x1"), f("other", "0x2")]);
        ix.extend(vec![f("late_init", "0x3")]);
        assert_eq!(ix.visible_len(), 2);
        assert_eq!(ix.len(), 3);
    }

    #[test]
    fn an_address_belongs_to_a_function_only_inside_its_stated_extent() {
        let mut ix = FunctionIndex::default();
        ix.extend(vec![
            FunctionEntry { name: "a".into(), va: "0x1000".into(), end: Some("0x1040".into()) },
            FunctionEntry { name: "b".into(), va: "0x2000".into(), end: None },
        ]);
        assert_eq!(ix.function_at(0x1000).map(|f| f.name.as_str()), Some("a"));
        assert_eq!(ix.function_at(0x103f).map(|f| f.name.as_str()), Some("a"));
        assert_eq!(ix.function_at(0x1040), None, "the end is exclusive");
        assert_eq!(ix.function_at(0x2000).map(|f| f.name.as_str()), Some("b"));
        assert_eq!(ix.function_at(0x2001), None, "no stated end: no guess at the extent");
        assert_eq!(ix.function_at(0x0fff), None);
    }

    #[test]
    fn pages_out_of_address_order_are_still_found() {
        let mut ix = FunctionIndex::default();
        ix.extend(vec![f("late", "0x3000")]);
        ix.extend(vec![f("early", "0x1000")]);
        assert_eq!(ix.function_at(0x1000).map(|f| f.name.as_str()), Some("early"));
        assert_eq!(ix.function_at(0x3000).map(|f| f.name.as_str()), Some("late"));
    }

    #[test]
    fn a_renamed_entry_is_found_and_filtered_by_its_new_name() {
        let mut ix = FunctionIndex::default();
        ix.extend(vec![f("sub_1000", "0x1000"), f("main", "0x2000")]);
        ix.set_query("parse");
        assert_eq!(ix.visible_len(), 0);
        assert!(ix.replace(f("parse_header", "0x1000")));
        assert_eq!(ix.visible_len(), 1, "the new name matches the filter");
        assert_eq!(ix.named_exactly("parse_header").len(), 1);
        assert!(ix.named_exactly("sub_1000").is_empty());
        assert!(ix.replace(f("sub_1000", "0x1000")));
        assert_eq!(ix.visible_len(), 0, "the old name no longer matches");
        assert!(!ix.replace(f("nowhere", "0x3000")), "no listed function there");
        assert_eq!(ix.listing_position(0x2000), Some(1));
        assert_eq!(ix.listing_position(0x2001), None);
    }

    #[test]
    fn names_read_again_land_on_their_own_addresses_only() {
        let mut ix = FunctionIndex::default();
        ix.extend(vec![f("sub_1", "0x1"), f("sub_2", "0x2"), f("sub_3", "0x3")]);
        ix.set_query("vtable");
        // The second page lines up; an entry whose address moved is left alone.
        let changed = ix.rename_from(1, vec![f("Widget::vtable_user", "0x2"), f("moved", "0x9"), f("past_the_end", "0x4")]);
        assert_eq!(changed, 1);
        assert_eq!(ix.named_exactly("Widget::vtable_user").len(), 1);
        assert!(ix.named_exactly("moved").is_empty() && ix.named_exactly("past_the_end").is_empty());
        assert_eq!(ix.visible_len(), 1, "the filter sees the new name");
        assert_eq!(ix.len(), 3, "renaming never adds entries");
    }

    #[test]
    fn only_functions_with_a_name_of_their_own_count_as_named() {
        let mut ix = FunctionIndex::default();
        ix.extend(vec![f("main", "0x1"), f("sub_2", "0x2"), f("sub_3", "0x3")]);
        assert_eq!(ix.named(), 1);
    }
}
