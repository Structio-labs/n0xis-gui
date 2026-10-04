// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The target's struct and enum definitions (`type list|struct|enum|rm`). The
//! engine replaces a whole definition on every write, so each edit sends the
//! type complete, then the list is read back: what is shown is what the engine
//! stored, never this view's own copy.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use n0xis_client::{
    ClientError, DefineEnum, DefineStruct, Engine, EnumDef, ListTypes, RemoveType, StructDef, TypeLibrary, Written,
};

use crate::assets::AppIcon;
use crate::layout::PanelKind;
use crate::panel::{dock_panel, header, message};

#[derive(Clone, PartialEq, Eq, Debug)]
enum Selected {
    Struct(String),
    Enum(String),
}

enum ViewState {
    Empty,
    Loading,
    Ready(TypeLibrary),
    Failed(String),
}

pub struct TypesView {
    engine: Option<Arc<Engine>>,
    state: ViewState,
    selected: Option<Selected>,
    /// The last write's outcome, when it is worth a line: a refusal or an engine error.
    note: Option<(String, bool)>,
    new_name: Entity<InputState>,
    field_offset: Entity<InputState>,
    field_name: Entity<InputState>,
    field_type: Entity<InputState>,
    member_name: Entity<InputState>,
    member_value: Entity<InputState>,
    wanted: bool,
    active: bool,
    focus_handle: FocusHandle,
    _request: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl TypesView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let new_name = input("Name of a new type", window, cx);
        let field_offset = input("Offset", window, cx);
        let field_name = input("Field", window, cx);
        let field_type = input("C type", window, cx);
        let member_name = input("Member", window, cx);
        let member_value = input("Value", window, cx);
        // Enter in the last box of a row does what the row's button does.
        let subscriptions = vec![
            cx.subscribe_in(&field_type, window, |this, _, e, window, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.add_field(window, cx);
                }
            }),
            cx.subscribe_in(&member_value, window, |this, _, e, window, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.add_member(window, cx);
                }
            }),
        ];
        Self {
            engine: None,
            state: ViewState::Empty,
            selected: None,
            note: None,
            new_name,
            field_offset,
            field_name,
            field_type,
            member_name,
            member_value,
            wanted: false,
            active: false,
            focus_handle: cx.focus_handle(),
            _request: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn set_engine(&mut self, engine: Option<Arc<Engine>>, cx: &mut Context<Self>) {
        self.engine = engine;
        self.state = ViewState::Empty;
        self.selected = None;
        self.note = None;
        self._request = None;
        self.wanted = self.engine.is_some();
        if self.active {
            self.reload(cx);
        }
        cx.notify();
    }

    fn on_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.active = active;
        if active && self.wanted {
            self.reload(cx);
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        self.wanted = false;
        if !matches!(self.state, ViewState::Ready(_)) {
            self.state = ViewState::Loading;
        }
        let pending = engine.send(&ListTypes);
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |view, cx| {
                view.state = match result {
                    Ok(lib) => ViewState::Ready(lib),
                    Err(e) => ViewState::Failed(e.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn library(&self) -> Option<&TypeLibrary> {
        match &self.state {
            ViewState::Ready(lib) => Some(lib),
            _ => None,
        }
    }

    fn selected_struct(&self) -> Option<&StructDef> {
        let Some(Selected::Struct(name)) = &self.selected else { return None };
        self.library()?.structs.iter().find(|s| &s.name == name)
    }

    fn selected_enum(&self) -> Option<&EnumDef> {
        let Some(Selected::Enum(name)) = &self.selected else { return None };
        self.library()?.enums.iter().find(|e| &e.name == name)
    }

    fn after_write(&mut self, result: Result<String, ClientError>, then_select: Option<Selected>, cx: &mut Context<Self>) {
        match result {
            Ok(done) => {
                self.note = (!done.is_empty()).then_some((done, false));
                self.selected = then_select;
            }
            Err(e) => self.note = Some((e.to_string(), true)),
        }
        self.reload(cx);
    }

    fn send_struct(&mut self, def: Result<DefineStruct, ClientError>, then_select: Selected, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        let def = match def {
            Ok(d) => d,
            Err(e) => {
                self.note = Some((e.to_string(), true));
                cx.notify();
                return;
            }
        };
        let pending = engine.send(&def);
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await.map(|Written| String::new());
            this.update(cx, |view, cx| view.after_write(result, Some(then_select), cx)).ok();
        }));
    }

    fn send_enum(&mut self, def: Result<DefineEnum, ClientError>, then_select: Selected, cx: &mut Context<Self>) {
        let Some(engine) = self.engine.clone() else { return };
        let def = match def {
            Ok(d) => d,
            Err(e) => {
                self.note = Some((e.to_string(), true));
                cx.notify();
                return;
            }
        };
        let pending = engine.send(&def);
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await.map(|Written| String::new());
            this.update(cx, |view, cx| view.after_write(result, Some(then_select), cx)).ok();
        }));
    }

    fn create(&mut self, as_enum: bool, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.new_name.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.note = Some(("Type a name for the new type first.".into(), true));
            cx.notify();
            return;
        }
        let exists = self.library().is_some_and(|lib| {
            lib.structs.iter().any(|s| s.name == name) || lib.enums.iter().any(|e| e.name == name)
        });
        if exists {
            // Defining it again would replace what is there with an empty type.
            self.note = Some((format!("A type called {name} exists already."), true));
            cx.notify();
            return;
        }
        self.new_name.update(cx, |i, cx| i.set_value("", window, cx));
        if as_enum {
            self.send_enum(DefineEnum::new(&name, &[]), Selected::Enum(name), cx);
        } else {
            self.send_struct(DefineStruct::new(&name, None, &[]), Selected::Struct(name), cx);
        }
    }

    fn struct_fields(def: &StructDef) -> Vec<(String, String, String)> {
        def.fields.iter().map(|f| (f.offset.to_string(), f.name.clone(), f.ctype.clone())).collect()
    }

    fn add_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(def) = self.selected_struct().cloned() else { return };
        let field = (
            self.field_offset.read(cx).value().trim().to_string(),
            self.field_name.read(cx).value().trim().to_string(),
            self.field_type.read(cx).value().trim().to_string(),
        );
        if field.0.is_empty() || field.1.is_empty() {
            self.note = Some(("A field needs an offset and a name.".into(), true));
            cx.notify();
            return;
        }
        let mut fields = Self::struct_fields(&def);
        fields.push(field);
        let request = DefineStruct::new(&def.name, def.size, &fields);
        // What was typed stays in the boxes until it is on its way, so a
        // refused field can be corrected rather than typed again.
        if request.is_ok() {
            for input in [&self.field_offset, &self.field_name, &self.field_type] {
                input.update(cx, |i, cx| i.set_value("", window, cx));
            }
        }
        self.send_struct(request, Selected::Struct(def.name.clone()), cx);
    }

    fn remove_field(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(def) = self.selected_struct().cloned() else { return };
        let mut fields = Self::struct_fields(&def);
        if index < fields.len() {
            fields.remove(index);
        }
        self.send_struct(DefineStruct::new(&def.name, def.size, &fields), Selected::Struct(def.name.clone()), cx);
    }

    fn enum_members(def: &EnumDef) -> Vec<(String, String)> {
        def.members.iter().map(|m| (m.name.clone(), m.value.to_string())).collect()
    }

    fn add_member(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(def) = self.selected_enum().cloned() else { return };
        let member = (self.member_name.read(cx).value().trim().to_string(), self.member_value.read(cx).value().trim().to_string());
        if member.0.is_empty() || member.1.is_empty() {
            self.note = Some(("A member needs a name and a value.".into(), true));
            cx.notify();
            return;
        }
        let mut members = Self::enum_members(&def);
        members.push(member);
        let request = DefineEnum::new(&def.name, &members);
        if request.is_ok() {
            for input in [&self.member_name, &self.member_value] {
                input.update(cx, |i, cx| i.set_value("", window, cx));
            }
        }
        self.send_enum(request, Selected::Enum(def.name.clone()), cx);
    }

    fn remove_member(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(def) = self.selected_enum().cloned() else { return };
        let mut members = Self::enum_members(&def);
        if index < members.len() {
            members.remove(index);
        }
        self.send_enum(DefineEnum::new(&def.name, &members), Selected::Enum(def.name.clone()), cx);
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let (Some(engine), Some(selected)) = (self.engine.clone(), self.selected.clone()) else { return };
        let name = match &selected {
            Selected::Struct(n) | Selected::Enum(n) => n.clone(),
        };
        let pending = engine.send(&RemoveType { name: name.clone() });
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = pending.await.map(|removed| {
                if removed { format!("{name} removed.") } else { format!("The engine held no type called {name}.") }
            });
            this.update(cx, |view, cx| view.after_write(result, None, cx)).ok();
        }));
    }

    fn render_list(&self, lib: &TypeLibrary, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (active, hover, muted, border) = (theme.list_active, theme.list_hover, theme.muted_foreground, theme.border);
        let item = |label: String, kind: &'static str, sel: Selected, ix: usize, cx: &mut Context<Self>| {
            let selected = self.selected.as_ref() == Some(&sel);
            h_flex()
                .id((kind, ix))
                .px_2()
                .h(px(22.))
                .gap_2()
                .text_xs()
                .cursor_pointer()
                .when(selected, |r| r.bg(active))
                .when(!selected, |r| r.hover(|s| s.bg(hover)))
                .child(div().w(px(42.)).flex_none().text_color(muted).child(kind))
                .child(div().flex_1().min_w_0().truncate().child(label))
                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                    view.selected = Some(sel.clone());
                    view.note = None;
                    cx.notify();
                }))
        };
        let mut list = v_flex().id("type-list").w(px(200.)).flex_none().h_full().overflow_y_scroll().border_r_1().border_color(border);
        for (ix, s) in lib.structs.iter().enumerate() {
            list = list.child(item(s.name.clone(), "struct", Selected::Struct(s.name.clone()), ix, cx));
        }
        for (ix, e) in lib.enums.iter().enumerate() {
            list = list.child(item(e.name.clone(), "enum", Selected::Enum(e.name.clone()), ix, cx));
        }
        if lib.structs.is_empty() && lib.enums.is_empty() {
            list = list.child(div().p_2().text_xs().text_color(muted).child("No types defined."));
        }
        list.into_any_element()
    }

    fn render_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let row = || h_flex().gap_2().text_xs().h(px(24.));
        if let Some(def) = self.selected_struct().cloned() {
            let size = def.size.map_or("size not set".to_string(), |s| format!("{s:#x} bytes"));
            let mut rows = v_flex().gap_1();
            for (ix, f) in def.fields.iter().enumerate() {
                rows = rows.child(
                    row()
                        .child(div().w(px(70.)).font_family(theme.mono_font_family.clone()).child(format!("{:#x}", f.offset)))
                        .child(div().w(px(140.)).truncate().child(f.name.clone()))
                        .child(div().flex_1().truncate().font_family(theme.mono_font_family.clone()).text_color(theme.muted_foreground).child(f.ctype.clone()))
                        .child(
                            Button::new(("field-rm", ix))
                                .icon(AppIcon::X)
                                .xsmall()
                                .ghost()
                                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.remove_field(ix, cx))),
                        ),
                );
            }
            return v_flex()
                .flex_1()
                .p_2()
                .gap_2()
                .child(h_flex().gap_2().child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("struct {}", def.name))).child(div().text_xs().text_color(theme.muted_foreground).child(size)))
                .child(rows)
                .child(
                    h_flex()
                        .gap_1()
                        .child(div().w(px(80.)).child(Input::new(&self.field_offset).xsmall()))
                        .child(div().w(px(130.)).child(Input::new(&self.field_name).xsmall()))
                        .child(div().flex_1().child(Input::new(&self.field_type).xsmall()))
                        .child(Button::new("field-add").label("Add field").xsmall().on_click(cx.listener(|view, _: &ClickEvent, window, cx| view.add_field(window, cx)))),
                )
                .child(h_flex().child(Button::new("type-delete").label("Delete struct").xsmall().ghost().on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.delete_selected(cx)))))
                .into_any_element();
        }
        if let Some(def) = self.selected_enum().cloned() {
            let mut rows = v_flex().gap_1();
            for (ix, m) in def.members.iter().enumerate() {
                rows = rows.child(
                    row()
                        .child(div().w(px(160.)).truncate().child(m.name.clone()))
                        .child(div().flex_1().font_family(theme.mono_font_family.clone()).child(m.value.to_string()))
                        .child(
                            Button::new(("member-rm", ix))
                                .icon(AppIcon::X)
                                .xsmall()
                                .ghost()
                                .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| view.remove_member(ix, cx))),
                        ),
                );
            }
            return v_flex()
                .flex_1()
                .p_2()
                .gap_2()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("enum {}", def.name)))
                .child(rows)
                .child(
                    h_flex()
                        .gap_1()
                        .child(div().w(px(160.)).child(Input::new(&self.member_name).xsmall()))
                        .child(div().w(px(100.)).child(Input::new(&self.member_value).xsmall()))
                        .child(Button::new("member-add").label("Add member").xsmall().on_click(cx.listener(|view, _: &ClickEvent, window, cx| view.add_member(window, cx)))),
                )
                .child(h_flex().child(Button::new("type-delete").label("Delete enum").xsmall().ghost().on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.delete_selected(cx)))))
                .into_any_element();
        }
        message("Select a type, or create one.", false, cx).into_any_element()
    }
}

impl Render for TypesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.state {
            ViewState::Empty => message("Open a target to see its types.", false, cx).into_any_element(),
            ViewState::Loading => message("Reading types…", false, cx).into_any_element(),
            ViewState::Failed(e) => message(format!("No types: {e}"), true, cx).into_any_element(),
            ViewState::Ready(lib) => {
                let lib = lib.clone();
                let (list, editor) = (self.render_list(&lib, cx), self.render_editor(cx));
                h_flex().size_full().child(list).child(editor).into_any_element()
            }
        };
        let theme = cx.theme();
        let create = h_flex()
            .px_2()
            .py_1()
            .gap_1()
            .border_b_1()
            .border_color(theme.border)
            .child(div().flex_1().child(Input::new(&self.new_name).xsmall()))
            .child(Button::new("new-struct").label("New struct").xsmall().on_click(cx.listener(|view, _: &ClickEvent, window, cx| view.create(false, window, cx))))
            .child(Button::new("new-enum").label("New enum").xsmall().on_click(cx.listener(|view, _: &ClickEvent, window, cx| view.create(true, window, cx))));
        let note = self.note.clone().map(|(text, danger)| {
            div().px_2().py_1().text_xs().text_color(if danger { theme.danger } else { theme.muted_foreground }).child(text)
        });
        v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(header("Types", cx))
            .when(self.engine.is_some(), |v| v.child(create))
            .children(note)
            .child(div().flex_1().min_h_0().child(body))
    }
}

dock_panel!(TypesView, PanelKind::Types, on_active);
