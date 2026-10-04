// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Who references an address (`xref to`), and one function's control flow
//! with its call sites (`ir build`).

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// `xref to`: every instruction that references `addr`. On a static image the
/// engine answers from a whole-program index it builds once per target.
#[derive(Debug, Clone)]
pub struct XrefsTo {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Xref {
    pub from: String,
    pub to: String,
    /// `call`, `jmp`, `cjmp`, `data`, … as the engine names it.
    pub kind: String,
    /// The referencing instruction.
    pub text: String,
    /// The referenced address's name, when the engine has one.
    #[serde(default)]
    pub sym: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Xrefs {
    pub addr: String,
    pub count: u64,
    pub refs: Vec<Xref>,
}

impl Request for XrefsTo {
    type Output = Xrefs;

    fn args(&self) -> Vec<String> {
        vec!["xref".into(), "to".into(), "--addr".into(), self.addr.clone()]
    }

    fn parse(envelope: Envelope) -> Result<Xrefs, ClientError> {
        Ok(envelope.into_parts::<Xrefs>(schema::XREF)?.0)
    }
}

/// `ir build`: the basic blocks of the function at `addr`.
#[derive(Debug, Clone)]
pub struct BuildCfg {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Cfg {
    pub start: String,
    pub end: String,
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub callsites: Vec<CallSite>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Block {
    pub id: u32,
    pub start: String,
    pub end: String,
    /// How the block ends: `cjmp`, `jmp`, `fall`, `ret`, `int`, `tail-call`, …
    pub terminator: String,
    pub successors: Vec<Edge>,
    pub insns: Vec<CfgInstruction>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Edge {
    pub to: String,
    /// `cjmp-true`, `cjmp-false`, `jmp`, `fall`, `eh`, … as the engine names it.
    pub kind: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CfgInstruction {
    pub va: String,
    pub mnemonic: String,
    pub text: String,
}

/// A call the function makes, as the engine resolved it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CallSite {
    pub from: String,
    /// `direct` (a target address), `named` (an import, by name), …
    pub kind: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub target_name: Option<String>,
    /// The import slot a `named` call goes through.
    #[serde(default)]
    pub via_slot: Option<String>,
}

impl Request for BuildCfg {
    type Output = Cfg;

    fn args(&self) -> Vec<String> {
        vec!["ir".into(), "build".into(), "--addr".into(), self.addr.clone()]
    }

    fn parse(envelope: Envelope) -> Result<Cfg, ClientError> {
        Ok(envelope.into_parts::<Cfg>(schema::IR_CFG)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shapes copied from real answers (n0xis 0.3.2 on an x64 PE DLL).
    #[test]
    fn xrefs_read_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"addr":"0x18001ae18","count":1,"dir":"to",
            "refs":[{"from":"0x18001a5ae","kind":"call","text":"call 000000018001AE18h","to":"0x18001ae18"}]},
            "meta":{"schema":"n0xis.xref.v1"}}"#,
        )
        .unwrap();
        let x = XrefsTo::parse(e).unwrap();
        assert_eq!(x.refs[0].from, "0x18001a5ae");
        assert_eq!(x.refs[0].sym, None);
    }

    #[test]
    fn a_cfg_reads_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"block_count":2,"start":"0x18001ae18","end":"0x18001af8c","insn_count":2,
            "blocks":[{"end":"0x18001ae40","id":0,"start":"0x18001ae18","terminator":"cjmp",
              "successors":[{"confidence":1.0,"kind":"cjmp-true","to":"0x18001aee4"},{"confidence":1.0,"kind":"cjmp-false","to":"0x18001ae40"}],
              "insns":[{"flow":"seq","len":5,"mnemonic":"mov","reads":["rsp","rbx"],"text":"mov [rsp+10h],rbx","va":"0x18001ae18"}]},
              {"end":"0x18001ae4c","id":1,"start":"0x18001ae40","terminator":"ret","successors":[],
              "insns":[{"flow":"ret","len":1,"mnemonic":"ret","text":"ret","va":"0x18001ae4b"}]}],
            "callsites":[{"from":"0x18001aeb2","kind":"named","target_name":"other.dll!Allocate","via_slot":"0x18008d028"},
                         {"from":"0x18001aecf","kind":"direct","target":"0x18001a9b0"}],
            "frame":{"frame_size":48},"stats":{"calls":5}},"meta":{"schema":"n0xis.ir.cfg.v1"}}"#,
        )
        .unwrap();
        let g = BuildCfg::parse(e).unwrap();
        assert_eq!(g.blocks.len(), 2);
        assert_eq!(g.blocks[0].successors[1].kind, "cjmp-false");
        assert_eq!(g.callsites[0].target, None);
        assert_eq!(g.callsites[0].target_name.as_deref(), Some("other.dll!Allocate"));
    }
}
