// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! What the user has written about the target: names, comments, bookmarks
//! (`annotate`) and type definitions (`type`). The engine keeps them in the
//! session's project directory.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// `annotate list`: every annotated address.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListAnnotations;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AnnotationRecord {
    pub va: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub type_note: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    /// Renamed variables of the function at `va`: displayed name to new name.
    #[serde(default)]
    pub var_names: BTreeMap<String, String>,
    #[serde(default)]
    pub var_types: BTreeMap<String, String>,
    #[serde(default)]
    pub bookmark: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Annotations {
    pub count: u64,
    pub records: Vec<AnnotationRecord>,
}

impl Request for ListAnnotations {
    type Output = Annotations;

    fn args(&self) -> Vec<String> {
        vec!["annotate".into(), "list".into()]
    }

    fn parse(envelope: Envelope) -> Result<Annotations, ClientError> {
        Ok(envelope.into_parts::<Annotations>(schema::ANNOTATION)?.0)
    }
}

/// `annotate bookmark`: set or clear the bookmark at `addr`. The answer is the
/// address's record as it now stands.
#[derive(Debug, Clone)]
pub struct SetBookmark {
    pub addr: String,
    pub on: bool,
}

impl Request for SetBookmark {
    type Output = AnnotationRecord;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["annotate".into(), "bookmark".into(), "--addr".into(), self.addr.clone()];
        if !self.on {
            a.push("--off".into());
        }
        a
    }

    fn parse(envelope: Envelope) -> Result<AnnotationRecord, ClientError> {
        Ok(envelope.into_parts::<AnnotationRecord>(schema::ANNOTATION)?.0)
    }
}

/// `type list`: the target's struct and enum definitions.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListTypes;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TypeLibrary {
    #[serde(default)]
    pub structs: Vec<StructDef>,
    #[serde(default)]
    pub enums: Vec<EnumDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StructDef {
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StructField {
    pub offset: i64,
    pub name: String,
    #[serde(default)]
    pub ctype: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub members: Vec<EnumMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EnumMember {
    pub name: String,
    pub value: i64,
}

impl Request for ListTypes {
    type Output = TypeLibrary;

    fn args(&self) -> Vec<String> {
        vec!["type".into(), "list".into()]
    }

    fn parse(envelope: Envelope) -> Result<TypeLibrary, ClientError> {
        Ok(envelope.into_parts::<TypeLibrary>(schema::TYPES)?.0)
    }
}

/// A write that answers with nothing worth reading: the caller lists again to
/// see what is now stored, rather than trusting its own copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written;

/// `type struct`: define or replace a struct. Built through [`DefineStruct::new`],
/// which refuses a field the engine's `OFFSET:NAME[:CTYPE]` form would misread.
#[derive(Debug, Clone)]
pub struct DefineStruct {
    name: String,
    size: Option<u64>,
    fields: Vec<String>,
}

impl DefineStruct {
    /// `fields` are (offset as typed, name, C type); the engine parses the offset.
    /// The engine replaces the whole definition, so a size it already holds has
    /// to be sent again or it is dropped.
    pub fn new(name: &str, size: Option<u64>, fields: &[(String, String, String)]) -> Result<Self, ClientError> {
        let name = checked_type_name(name)?;
        let mut specs = Vec::new();
        for (offset, field, ctype) in fields {
            let field = field.trim();
            if field.is_empty() {
                return Err(ClientError::Unsendable(format!("a field at offset {offset} has no name")));
            }
            // The engine splits the spec at the first two colons, so a colon in
            // the name would move part of it into the type without a word.
            if field.contains(':') {
                return Err(ClientError::Unsendable(format!("field name {field:?} holds a ':', which separates the parts of a field")));
            }
            let ctype = ctype.trim();
            specs.push(if ctype.is_empty() { format!("{}:{field}", offset.trim()) } else { format!("{}:{field}:{ctype}", offset.trim()) });
        }
        Ok(Self { name, size, fields: specs })
    }
}

impl Request for DefineStruct {
    type Output = Written;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["type".into(), "struct".into(), "--name".into(), self.name.clone()];
        if let Some(size) = self.size {
            a.extend(["--size".to_string(), size.to_string()]);
        }
        for f in &self.fields {
            a.extend(["--field".to_string(), f.clone()]);
        }
        a
    }

    fn parse(envelope: Envelope) -> Result<Written, ClientError> {
        envelope.into_parts::<serde_json::Value>(schema::TYPES).map(|_| Written)
    }
}

/// `type enum`: define or replace an enum, refusing a member the engine's
/// `NAME=VALUE` form would misread.
#[derive(Debug, Clone)]
pub struct DefineEnum {
    name: String,
    members: Vec<String>,
}

impl DefineEnum {
    /// `members` are (name, value as typed); the engine parses the value.
    pub fn new(name: &str, members: &[(String, String)]) -> Result<Self, ClientError> {
        let name = checked_type_name(name)?;
        let mut specs = Vec::new();
        for (member, value) in members {
            let member = member.trim();
            if member.is_empty() || member.contains('=') {
                return Err(ClientError::Unsendable(format!("enum member name {member:?} is empty or holds a '='")));
            }
            specs.push(format!("{member}={}", value.trim()));
        }
        Ok(Self { name, members: specs })
    }
}

impl Request for DefineEnum {
    type Output = Written;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["type".into(), "enum".into(), "--name".into(), self.name.clone()];
        for m in &self.members {
            a.extend(["--member".to_string(), m.clone()]);
        }
        a
    }

    fn parse(envelope: Envelope) -> Result<Written, ClientError> {
        envelope.into_parts::<serde_json::Value>(schema::TYPES).map(|_| Written)
    }
}

/// `type rm`: remove a struct or enum by name. The answer says whether there
/// was one to remove.
#[derive(Debug, Clone)]
pub struct RemoveType {
    pub name: String,
}

#[derive(Deserialize)]
struct Removed {
    removed: bool,
}

impl Request for RemoveType {
    type Output = bool;

    fn args(&self) -> Vec<String> {
        vec!["type".into(), "rm".into(), "--name".into(), self.name.clone()]
    }

    fn parse(envelope: Envelope) -> Result<bool, ClientError> {
        Ok(envelope.into_parts::<Removed>(schema::TYPES)?.0.removed)
    }
}

fn checked_type_name(name: &str) -> Result<String, ClientError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ClientError::Unsendable("a type needs a name".into()));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(json: &str) -> Envelope {
        serde_json::from_str(json).expect("envelope")
    }

    /// Shapes copied from real answers (n0xis 0.3.2).
    #[test]
    fn annotations_read_the_engines_own_shape() {
        let list = ListAnnotations::parse(env(
            r#"{"ok":true,"data":{"count":2,"records":[
            {"history":[{"field":"name","new":"crc32_z","unix":1788282019}],"name":"crc32_z","va":"0x1510"},
            {"va":"0x1400015c8","var_names":{"v1":"result"},"var_types":{"rcx":"Kind"}}]},
            "meta":{"schema":"n0xis.annotation.v1"}}"#,
        ))
        .unwrap();
        assert_eq!(list.records[0].name.as_deref(), Some("crc32_z"));
        assert!(!list.records[0].bookmark);
        assert_eq!(list.records[1].var_names["v1"], "result");
        let mark = SetBookmark::parse(env(
            r#"{"ok":true,"data":{"bookmark":true,"history":[],"va":"0x18001ae18"},"meta":{"schema":"n0xis.annotation.v1"}}"#,
        ))
        .unwrap();
        assert!(mark.bookmark);
        assert_eq!(SetBookmark { addr: "0x1".into(), on: false }.args(), ["annotate", "bookmark", "--addr", "0x1", "--off"]);
    }

    #[test]
    fn types_read_the_engines_own_shape() {
        let lib = ListTypes::parse(env(
            r#"{"ok":true,"data":{"enums":[{"members":[{"name":"Off","value":0},{"name":"On","value":1}],"name":"Mode"}],
            "structs":[{"fields":[{"ctype":"int","name":"a","offset":0},{"ctype":"char*","name":"b","offset":8}],"name":"Pair"}]},
            "meta":{"schema":"n0xis.types.v1"}}"#,
        ))
        .unwrap();
        assert_eq!(lib.structs[0].fields[1].ctype, "char*");
        assert_eq!(lib.enums[0].members[1].value, 1);
    }

    #[test]
    fn a_removal_says_whether_there_was_anything_to_remove() {
        let gone = env(r#"{"ok":true,"data":{"removed":false},"meta":{"schema":"n0xis.types.v1"}}"#);
        assert_eq!(RemoveType::parse(gone), Ok(false));
    }

    #[test]
    fn a_field_or_member_the_engine_would_misread_is_refused() {
        let ok = DefineStruct::new("Pair", Some(32), &[("0x8".into(), "b".into(), "std::size_t".into())]).unwrap();
        assert_eq!(ok.args(), ["type", "struct", "--name", "Pair", "--size", "32", "--field", "0x8:b:std::size_t"]);
        assert!(DefineStruct::new("Pair", None, &[("0".into(), "a:b".into(), "int".into())]).is_err());
        assert!(DefineStruct::new(" ", None, &[]).is_err());
        assert!(DefineEnum::new("Mode", &[("A=B".into(), "1".into())]).is_err());
        assert_eq!(DefineEnum::new("Mode", &[("On".into(), "0x1".into())]).unwrap().args()[5], "On=0x1");
    }
}
