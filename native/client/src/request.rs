// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Typed requests. Each one knows its arguments, the schema of its answer and
//! the shape of that answer, so a view never reads a field by guessing its name.
//! Requests name no `--file`: the session supplies the file it was opened with.

use serde::Deserialize;

use crate::{ClientError, Envelope};

/// Payload schemas, as the engine names them in `meta.schema`.
pub mod schema {
    pub const FUNCTION_DISCOVER: &str = "n0xis.function.discover.v1";
    /// An archived `n0x.*` name: the decompiler kept its ported shape.
    pub const DECOMP_PSEUDO: &str = "n0x.decomp.pseudo.v1";
    pub const DECODE: &str = "n0xis.decode.v1";
    pub const PROFILE: &str = "n0xis.profile.v1";
    pub const XREF: &str = "n0xis.xref.v1";
    pub const IR_CFG: &str = "n0xis.ir.cfg.v1";
    pub const ANNOTATION: &str = "n0xis.annotation.v1";
    pub const TYPES: &str = "n0xis.types.v1";
    pub const FIND: &str = "n0xis.find.v1";
    pub const PROCESS_PS: &str = "n0xis.process.ps.v1";
    pub const MEM_MAP: &str = "n0xis.mem.map.v1";
    pub const SCAN: &str = "n0xis.scan.v1";
    pub const ANALYZE: &str = "n0xis.analyze.v1";
    pub const PROJECT_CACHE: &str = "n0xis.project.cache.v1";
    pub const MEM_SPAN: &str = "n0xis.mem.span.v1";
    pub const STRINGS: &str = "n0xis.strings.v1";
}

/// A request the engine answers with one payload shape.
pub trait Request {
    type Output;
    fn args(&self) -> Vec<String>;
    fn parse(envelope: Envelope) -> Result<Self::Output, ClientError>;
}

/// One page of `function discover`.
#[derive(Debug, Clone)]
pub struct DiscoverFunctions {
    /// Read the PE exception table (exact starts and ends) instead of scanning
    /// for prologues. Only x64 PE images have one.
    pub from_unwind_table: bool,
    pub limit: u32,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FunctionEntry {
    pub va: String,
    pub name: String,
    #[serde(default)]
    pub end: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionsPage {
    pub functions: Vec<FunctionEntry>,
    /// How many functions the whole image has, when the engine knows.
    pub total: Option<u64>,
}

#[derive(Deserialize)]
struct FunctionsData {
    functions: Vec<FunctionEntry>,
}

impl Request for DiscoverFunctions {
    type Output = FunctionsPage;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["function".to_string(), "discover".to_string()];
        if self.from_unwind_table {
            a.push("--pdata".into());
        }
        a.extend(["--limit".into(), self.limit.to_string(), "--offset".into(), self.offset.to_string()]);
        a
    }

    fn parse(envelope: Envelope) -> Result<FunctionsPage, ClientError> {
        let (data, meta) = envelope.into_parts::<FunctionsData>(schema::FUNCTION_DISCOVER)?;
        Ok(FunctionsPage { functions: data.functions, total: meta.total })
    }
}

/// `decomp pseudo` rendering styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecompStyle {
    #[default]
    Structured,
    Ssa,
    Goto,
}

impl DecompStyle {
    pub const ALL: [DecompStyle; 3] = [Self::Structured, Self::Ssa, Self::Goto];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Structured => "structured",
            Self::Ssa => "ssa",
            Self::Goto => "goto",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Decompile {
    pub addr: String,
    pub style: DecompStyle,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Decompiled {
    pub pseudo: Vec<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub quality: Option<f64>,
    /// Every variable `pseudo` shows. `None` from an engine that does not list
    /// them (0.3.3 and earlier), which is not the same as a function with none.
    #[serde(default)]
    pub variables: Option<Vec<Variable>>,
}

/// A variable of a decompiled function, as the engine lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Variable {
    /// The name as printed, after any rename.
    pub name: String,
    /// What a rename or a type is stored under.
    pub key: String,
    pub kind: VariableKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VariableKind {
    Param,
    Local,
    /// Neither a parameter nor a local: it takes a name but not a type.
    Value,
    /// A kind this client does not know yet. Treated as taking a name only.
    #[serde(other)]
    Other,
}

impl VariableKind {
    /// Whether the engine applies a C type set on this variable.
    pub fn takes_a_type(self) -> bool {
        matches!(self, Self::Param | Self::Local)
    }
}

impl Request for Decompile {
    type Output = Decompiled;

    fn args(&self) -> Vec<String> {
        vec![
            "decomp".into(),
            "pseudo".into(),
            "--addr".into(),
            self.addr.clone(),
            "--style".into(),
            self.style.as_str().into(),
        ]
    }

    fn parse(envelope: Envelope) -> Result<Decompiled, ClientError> {
        Ok(envelope.into_parts::<Decompiled>(schema::DECOMP_PSEUDO)?.0)
    }
}

#[derive(Debug, Clone)]
pub struct Disassemble {
    pub addr: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Instruction {
    pub va: String,
    pub bytes: String,
    pub mnemonic: String,
    pub text: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Disassembly {
    pub insns: Vec<Instruction>,
}

impl Request for Disassemble {
    type Output = Disassembly;

    fn args(&self) -> Vec<String> {
        vec!["disasm".into(), "--addr".into(), self.addr.clone(), "--count".into(), self.count.to_string()]
    }

    fn parse(envelope: Envelope) -> Result<Disassembly, ClientError> {
        Ok(envelope.into_parts::<Disassembly>(schema::DECODE)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(json: &str) -> Envelope {
        serde_json::from_str(json).expect("envelope")
    }

    /// Shapes copied from a real engine answer (n0xis 0.3.2), not written from memory.
    #[test]
    fn a_functions_page_reads_the_engines_own_shape() {
        let e = env(
            r#"{"ok":true,"data":{"start":"0x140001000","scanned_bytes":0,"count":2,"truncated":true,
            "functions":[{"name":"sub_140001000","va":"0x140001000","end":"0x140001001"},
                         {"name":"sub_140001010","va":"0x140001010","end":"0x140001017"}]},
            "meta":{"schema":"n0xis.function.discover.v1","returned":2,"total":48,"truncated":true}}"#,
        );
        let page = DiscoverFunctions::parse(e).unwrap();
        assert_eq!(page.total, Some(48));
        assert_eq!(page.functions[1].name, "sub_140001010");
        assert_eq!(page.functions[1].end.as_deref(), Some("0x140001017"));
    }

    #[test]
    fn a_decompilation_reads_the_engines_own_shape() {
        let e = env(
            r#"{"ok":true,"data":{"address":"0x140001580","pseudo":["uint64_t f() {","}"],
            "signature":"uint64_t f()","quality":0.9,"style":"structured"},
            "meta":{"schema":"n0x.decomp.pseudo.v1"}}"#,
        );
        let d = Decompile::parse(e).unwrap();
        assert_eq!(d.pseudo.len(), 2);
        assert_eq!(d.signature.as_deref(), Some("uint64_t f()"));
    }

    /// From n0xis after `c7fe32b` (the first engine that lists variables).
    #[test]
    fn a_decompilation_lists_its_variables_and_an_older_engine_says_nothing() {
        let e = env(
            r#"{"ok":true,"data":{"address":"0x1000","pseudo":["uint64_t f(uint64_t input) {","}"],
            "variables":[{"key":"rcx","kind":"param","name":"input"},{"key":"local_8","kind":"local","name":"local_8"},
            {"key":"v1","kind":"value","name":"v1"},{"key":"x","kind":"something-new","name":"x"}]},
            "meta":{"schema":"n0x.decomp.pseudo.v1"}}"#,
        );
        let vars = Decompile::parse(e).unwrap().variables.unwrap();
        assert_eq!(vars[0], Variable { name: "input".into(), key: "rcx".into(), kind: VariableKind::Param });
        assert!(vars[1].kind.takes_a_type() && !vars[2].kind.takes_a_type());
        assert_eq!(vars[3].kind, VariableKind::Other);
        let old = env(r#"{"ok":true,"data":{"pseudo":["f() {","}"]},"meta":{"schema":"n0x.decomp.pseudo.v1"}}"#);
        assert_eq!(Decompile::parse(old).unwrap().variables, None);
    }

    #[test]
    fn a_disassembly_reads_the_engines_own_shape() {
        let e = env(
            r#"{"ok":true,"data":{"start":"0x140001580","count":1,"bytes_consumed":2,
            "insns":[{"bytes":"8b 01","kind":"seq","len":2,"mnemonic":"mov","text":"mov eax,[rcx]","va":"0x140001580"}]},
            "meta":{"schema":"n0xis.decode.v1"}}"#,
        );
        let d = Disassemble::parse(e).unwrap();
        assert_eq!(d.insns[0].text, "mov eax,[rcx]");
        assert_eq!(d.insns[0].target, None);
    }

    #[test]
    fn requests_never_name_a_file() {
        use crate::{Annotate, Note, RenameVariable, SetVariableType, ShowAnnotations, TypeSubject};
        let all = [
            DiscoverFunctions { from_unwind_table: true, limit: 10, offset: 0 }.args(),
            Decompile { addr: "0x1".into(), style: DecompStyle::Ssa }.args(),
            Disassemble { addr: "0x1".into(), count: 4 }.args(),
            Annotate { note: Note::Name, addr: "0x1".into(), value: Some("a".into()) }.args(),
            RenameVariable { function: "0x1".into(), key: "v1".into(), value: None }.args(),
            SetVariableType { function: "0x1".into(), subject: TypeSubject::Return, value: Some("int".into()) }.args(),
            ShowAnnotations { addr: "0x1".into() }.args(),
        ];
        for args in all {
            assert!(!args.iter().any(|a| a == "--file"), "{args:?}");
        }
    }
}
