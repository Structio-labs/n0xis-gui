// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! What the user changes about the target (names, comments, types, bookmarks)
//! and how to take it back. The engine holds every fact. A change keeps the
//! value before it and the value after it as the engine reported them, not as
//! the GUI asked for them.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use n0xis_client::{
    Annotate, AnnotationRecord, ClientError, Engine, Note, RETURN_KEY, RenameVariable, SetBookmark, SetVariableType,
    TypeSubject,
};

use crate::nav::hex;

/// How many changes can be undone.
pub const MAX_EDITS: usize = 200;

/// What a bookmark reads as when it is set. A bookmark is on or off; the text
/// only gives it the same shape as the other facts.
const BOOKMARKED: &str = "bookmarked";

/// One fact about the target that the user can set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    /// The name at an address: a function's, when one starts there.
    Name { addr: u64 },
    Comment { addr: u64 },
    /// A type kept as text at an address. Only cleared and restored here.
    TypeNote { addr: u64 },
    Bookmark { addr: u64 },
    /// A variable's name in the function starting at `function`. `key` is
    /// what the engine stores it under (the decompiler's `variables[].key`).
    VariableName { function: u64, key: String },
    VariableType { function: u64, key: String },
    ReturnType { function: u64 },
}

/// The engine's answer to a write: the address's record as it now stands.
pub type Written = Pin<Box<dyn Future<Output = Result<AnnotationRecord, ClientError>>>>;

impl Fact {
    /// The address whose record holds this fact.
    pub fn addr(&self) -> u64 {
        match self {
            Self::Name { addr } | Self::Comment { addr } | Self::TypeNote { addr } | Self::Bookmark { addr } => *addr,
            Self::VariableName { function, .. } | Self::VariableType { function, .. } | Self::ReturnType { function } => {
                *function
            }
        }
    }

    /// The fact's value in `record`; `None` when it is not set.
    pub fn read(&self, record: Option<&AnnotationRecord>) -> Option<String> {
        let r = record?;
        match self {
            Self::Name { .. } => r.name.clone(),
            Self::Comment { .. } => r.comment.clone(),
            Self::TypeNote { .. } => r.type_note.clone(),
            Self::Bookmark { .. } => r.bookmark.then(|| BOOKMARKED.to_string()),
            Self::VariableName { key, .. } => r.var_names.get(key).cloned(),
            Self::VariableType { key, .. } => r.var_types.get(key).cloned(),
            Self::ReturnType { .. } => r.var_types.get(RETURN_KEY).cloned(),
        }
    }

    /// Every fact `record` holds, so that all of them can be cleared and put back.
    pub fn all_in(record: &AnnotationRecord, addr: u64) -> Vec<Fact> {
        let mut facts = Vec::new();
        if record.name.is_some() {
            facts.push(Self::Name { addr });
        }
        if record.comment.is_some() {
            facts.push(Self::Comment { addr });
        }
        if record.type_note.is_some() {
            facts.push(Self::TypeNote { addr });
        }
        if record.bookmark {
            facts.push(Self::Bookmark { addr });
        }
        for key in record.var_names.keys() {
            facts.push(Self::VariableName { function: addr, key: key.clone() });
        }
        for key in record.var_types.keys() {
            facts.push(if key == RETURN_KEY {
                Self::ReturnType { function: addr }
            } else {
                Self::VariableType { function: addr, key: key.clone() }
            });
        }
        facts
    }

    /// Whether changing this fact can change what a name in a list says.
    pub fn renames(&self) -> bool {
        matches!(self, Self::Name { .. })
    }

    /// Set the fact to `value`, or clear it with `None`.
    pub fn write(&self, engine: &Engine, value: Option<String>) -> Written {
        match self {
            Self::Name { addr } => Box::pin(engine.send(&Annotate { note: Note::Name, addr: hex(*addr), value })),
            Self::Comment { addr } => Box::pin(engine.send(&Annotate { note: Note::Comment, addr: hex(*addr), value })),
            Self::TypeNote { addr } => Box::pin(engine.send(&Annotate { note: Note::TypeNote, addr: hex(*addr), value })),
            Self::Bookmark { addr } => Box::pin(engine.send(&SetBookmark { addr: hex(*addr), on: value.is_some() })),
            Self::VariableName { function, key } => {
                Box::pin(engine.send(&RenameVariable { function: hex(*function), key: key.clone(), value }))
            }
            Self::VariableType { function, key } => Box::pin(engine.send(&SetVariableType {
                function: hex(*function),
                subject: TypeSubject::Variable(key.clone()),
                value,
            })),
            Self::ReturnType { function } => {
                Box::pin(engine.send(&SetVariableType { function: hex(*function), subject: TypeSubject::Return, value }))
            }
        }
    }
}

/// One fact's value before and after a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub fact: Fact,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// What one user action changed: one fact, or several when an address is cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// What the action was, for "Undid …" messages.
    pub label: String,
    pub changes: Vec<Change>,
}

impl Edit {
    /// The writes that take the edit back, last change first.
    pub fn undo_writes(&self) -> Vec<(Fact, Option<String>)> {
        self.changes.iter().rev().map(|c| (c.fact.clone(), c.before.clone())).collect()
    }

    /// The writes that make the edit again, in its own order.
    pub fn redo_writes(&self) -> Vec<(Fact, Option<String>)> {
        self.changes.iter().map(|c| (c.fact.clone(), c.after.clone())).collect()
    }

    pub fn renames(&self) -> bool {
        self.changes.iter().any(|c| c.fact.renames())
    }
}

/// The edits that can be undone and redone.
#[derive(Default)]
pub struct Journal {
    done: Vec<Edit>,
    undone: Vec<Edit>,
}

impl Journal {
    /// A new edit: it can be undone, and what was undone before it can no
    /// longer be redone.
    pub fn record(&mut self, edit: Edit) {
        self.done.push(edit);
        if self.done.len() > MAX_EDITS {
            self.done.remove(0);
        }
        self.undone.clear();
    }

    pub fn take_undo(&mut self) -> Option<Edit> {
        self.done.pop()
    }

    pub fn take_redo(&mut self) -> Option<Edit> {
        self.undone.pop()
    }

    /// An undo went through: the edit can be redone.
    pub fn undid(&mut self, edit: Edit) {
        self.undone.push(edit);
    }

    /// A redo went through: the edit can be undone again.
    pub fn redid(&mut self, edit: Edit) {
        self.done.push(edit);
    }

    /// An undo or redo failed before it changed anything: the edit goes back
    /// where it came from.
    pub fn put_back_undo(&mut self, edit: Edit) {
        self.done.push(edit);
    }

    pub fn put_back_redo(&mut self, edit: Edit) {
        self.undone.push(edit);
    }

    /// Forget everything, for a new target.
    pub fn clear(&mut self) {
        self.done.clear();
        self.undone.clear();
    }
}

/// What a run of writes did.
pub struct Applied {
    /// The value each write left, as the engine reported it, in order. Shorter
    /// than the writes when one was refused.
    pub now: Vec<Option<String>>,
    pub refused: Option<ClientError>,
}

/// Send `writes` one after another, stopping at the first the engine refuses.
pub async fn apply(engine: Arc<Engine>, writes: Vec<(Fact, Option<String>)>) -> Applied {
    let mut now = Vec::with_capacity(writes.len());
    for (fact, value) in writes {
        match fact.write(&engine, value).await {
            Ok(record) => now.push(fact.read(Some(&record))),
            Err(e) => return Applied { now, refused: Some(e) },
        }
    }
    Applied { now, refused: None }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{Change, Edit, Fact, Journal, MAX_EDITS};
    use n0xis_client::AnnotationRecord;

    fn record(json: &str) -> AnnotationRecord {
        serde_json::from_str(json).expect("record")
    }

    fn edit(label: &str) -> Edit {
        Edit { label: label.into(), changes: vec![] }
    }

    #[test]
    fn a_fact_reads_its_own_field_of_a_record() {
        let r = record(
            r#"{"va":"0x1100","name":"parse","comment":"c","bookmark":true,
            "var_names":{"v1":"count"},"var_types":{"rcx":"int","@return":"char *"}}"#,
        );
        assert_eq!(Fact::Name { addr: 0x1100 }.read(Some(&r)).as_deref(), Some("parse"));
        assert_eq!(Fact::Bookmark { addr: 0x1100 }.read(Some(&r)).as_deref(), Some("bookmarked"));
        assert_eq!(Fact::VariableName { function: 0x1100, key: "v1".into() }.read(Some(&r)).as_deref(), Some("count"));
        assert_eq!(Fact::VariableType { function: 0x1100, key: "rcx".into() }.read(Some(&r)).as_deref(), Some("int"));
        assert_eq!(Fact::ReturnType { function: 0x1100 }.read(Some(&r)).as_deref(), Some("char *"));
        assert_eq!(Fact::TypeNote { addr: 0x1100 }.read(Some(&r)), None);
        assert_eq!(Fact::Name { addr: 0x1100 }.read(None), None, "no record: nothing set");
    }

    #[test]
    fn clearing_an_address_covers_every_fact_it_holds() {
        let r = record(
            r#"{"va":"0x10","name":"n","type_note":"int(void)","bookmark":true,
            "var_names":{"v1":"a"},"var_types":{"rcx":"int","@return":"void"}}"#,
        );
        let facts = Fact::all_in(&r, 0x10);
        assert_eq!(facts.len(), 6, "{facts:?}");
        assert!(facts.contains(&Fact::ReturnType { function: 0x10 }));
        assert!(!facts.contains(&Fact::VariableType { function: 0x10, key: "@return".into() }), "the return key is a return type");
        assert!(!facts.contains(&Fact::Comment { addr: 0x10 }), "no comment was recorded");
    }

    #[test]
    fn undo_restores_in_reverse_and_redo_replays_in_order() {
        let e = Edit {
            label: "clear".into(),
            changes: vec![
                Change { fact: Fact::Name { addr: 1 }, before: Some("a".into()), after: None },
                Change { fact: Fact::Comment { addr: 1 }, before: Some("b".into()), after: None },
            ],
        };
        let undo = e.undo_writes();
        assert_eq!(undo[0], (Fact::Comment { addr: 1 }, Some("b".into())));
        assert_eq!(undo[1], (Fact::Name { addr: 1 }, Some("a".into())));
        assert_eq!(e.redo_writes()[0], (Fact::Name { addr: 1 }, None));
        assert!(e.renames());
    }

    #[test]
    fn a_new_edit_ends_what_could_be_redone() {
        let mut j = Journal::default();
        j.record(edit("one"));
        j.record(edit("two"));
        let two = j.take_undo().unwrap();
        assert_eq!(two.label, "two");
        j.undid(two);
        j.record(edit("three"));
        assert!(j.take_redo().is_none(), "redo after a new edit would replay a change made on another state");
        assert_eq!(j.take_undo().unwrap().label, "three");
        assert_eq!(j.take_undo().unwrap().label, "one");
        assert!(j.take_undo().is_none());
    }

    #[test]
    fn the_journal_keeps_only_the_latest_edits() {
        let mut j = Journal::default();
        for i in 0..MAX_EDITS + 5 {
            j.record(edit(&i.to_string()));
        }
        let mut n = 0;
        while j.take_undo().is_some() {
            n += 1;
        }
        assert_eq!(n, MAX_EDITS);
    }
}
