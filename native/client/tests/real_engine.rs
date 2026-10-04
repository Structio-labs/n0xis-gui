// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The client against the real engine, on a target whose answer is known before
//! the question is asked: a C function compiled here, holding constants planted
//! in its source. If the decompilation shown to the user did not contain them,
//! the chain from engine to view would be broken somewhere.
//!
//! Needs a C compiler and an engine (`$N0XIS_BIN`, `~/.local/bin/n0xis` or
//! `PATH`); without either it says so and skips.

use std::path::PathBuf;
use std::process::Command;

use n0xis_client::{DecompStyle, Decompile, Disassemble, DiscoverFunctions, Engine, EngineCommand, EngineStatus};

const SOURCE: &str = r#"
__attribute__((noinline)) unsigned int mix(unsigned int x) {
    return (x ^ 0x5EED1234u) * 2654435761u;
}
int main(int argc, char **argv) {
    (void)argv;
    return (int)mix((unsigned int)argc);
}
"#;

fn build_target() -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("n0xis-client-real-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let src = dir.join("target.c");
    let exe = dir.join("target");
    std::fs::write(&src, SOURCE).ok()?;
    let ok = Command::new("cc").arg("-O1").arg("-o").arg(&exe).arg(&src).status().is_ok_and(|s| s.success());
    ok.then_some(exe)
}

#[test]
fn the_decompilation_of_a_known_function_holds_its_planted_constants() {
    let engine = match EngineCommand::locate() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("SKIPPED: {e}");
            return;
        }
    };
    let Some(target) = build_target() else {
        eprintln!("SKIPPED: no C compiler (cc) to build the known target");
        return;
    };
    let project = target.parent().map(|p| p.to_path_buf());
    let engine = Engine::start(engine, target.clone(), project);

    // A Linux executable has no PE unwind table: the prologue scan is the source.
    let page = engine
        .send(&DiscoverFunctions { from_unwind_table: false, limit: 1000, offset: 0 })
        .wait()
        .expect("function list");
    let mix = page.functions.iter().find(|f| f.name == "mix").unwrap_or_else(|| {
        panic!("the symbol table names `mix`; discovered: {:?}", page.functions.iter().map(|f| &f.name).collect::<Vec<_>>())
    });

    let decompiled = engine
        .send(&Decompile { addr: mix.va.clone(), style: DecompStyle::Structured })
        .wait()
        .expect("decompilation");
    let text = decompiled.pseudo.join("\n").to_lowercase();
    assert!(text.contains("0x5eed1234"), "the xor constant from the source:\n{text}");
    assert!(text.contains("0x9e3779b1") || text.contains("2654435761"), "the multiplier from the source:\n{text}");

    let disassembly = engine.send(&Disassemble { addr: mix.va.clone(), count: 8 }).wait().expect("disassembly");
    assert_eq!(disassembly.insns.first().map(|i| i.va.as_str()), Some(mix.va.as_str()), "it starts where asked");

    assert!(matches!(engine.status(), EngineStatus::Ready { .. }), "{:?}", engine.status());
    let _ = std::fs::remove_dir_all(target.parent().expect("temp dir"));
}

/// A second known target: `twice` calls `leaf` exactly twice, and a string is
/// planted in read-only data. Every answer below is fixed by the source before
/// the engine is asked.
const WIDGET_SOURCE: &str = r#"
#include <stdio.h>
static const char PLANTED[] = "n0xis-planted-7f3a";
__attribute__((noinline)) unsigned int leaf(unsigned int x) { return x * 3u + 1u; }
__attribute__((noinline)) unsigned int twice(unsigned int x) { return leaf(x) + leaf(x ^ 7u); }
int main(int argc, char **argv) {
    (void)argv;
    puts(PLANTED);
    return (int)twice((unsigned int)argc);
}
"#;

/// The address `nm` gives a symbol: the producer's own symbol table, read by a
/// tool that is not the engine.
fn nm_address(exe: &std::path::Path, symbol: &str) -> Option<u64> {
    let out = Command::new("nm").arg(exe).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().find_map(|l| {
        let mut parts = l.split_whitespace();
        let (addr, _, name) = (parts.next()?, parts.next()?, parts.next()?);
        (name == symbol).then(|| u64::from_str_radix(addr, 16).ok()).flatten()
    })
}

fn va(s: &str) -> u64 {
    u64::from_str_radix(s.trim_start_matches("0x"), 16).expect("hex address")
}

#[test]
fn the_widget_requests_answer_questions_whose_answers_are_known() {
    use n0xis_client::{
        BuildCfg, ClientError, DefineEnum, DefineStruct, Find, FindQuery, ListAnnotations, ListTypes, Profile,
        RemoveType, SetBookmark, XrefsTo,
    };
    let engine = match EngineCommand::locate() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("SKIPPED: {e}");
            return;
        }
    };
    let dir = std::env::temp_dir().join(format!("n0xis-client-widgets-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".n0x")).expect("temp project");
    let (src, exe) = (dir.join("widgets.c"), dir.join("widgets"));
    std::fs::write(&src, WIDGET_SOURCE).expect("source");
    if !Command::new("cc").arg("-O1").arg("-o").arg(&exe).arg(&src).status().is_ok_and(|s| s.success()) {
        eprintln!("SKIPPED: no C compiler (cc) to build the known target");
        return;
    }
    let engine = Engine::start(engine, exe.clone(), Some(dir.clone()));
    let functions = engine
        .send(&DiscoverFunctions { from_unwind_table: false, limit: 1000, offset: 0 })
        .wait()
        .expect("function list")
        .functions;
    let find = |name: &str| functions.iter().find(|f| f.name == name).cloned().unwrap_or_else(|| panic!("`{name}` listed"));
    let (leaf, twice) = (find("leaf"), find("twice"));

    // Two calls in the source, so two references, both calls.
    let refs = engine.send(&XrefsTo { addr: leaf.va.clone() }).wait().expect("xref to");
    assert_eq!(refs.refs.len(), 2, "{refs:?}");
    assert!(refs.refs.iter().all(|r| r.kind == "call" && va(&r.to) == va(&leaf.va)), "{refs:?}");

    // The control-flow view of `twice` starts at `twice` and names the same two calls.
    let cfg = engine.send(&BuildCfg { addr: twice.va.clone() }).wait().expect("ir build");
    assert_eq!(cfg.blocks.first().map(|b| va(&b.start)), Some(va(&twice.va)));
    let calls: Vec<u64> = cfg.callsites.iter().filter_map(|c| c.target.as_deref()).map(va).collect();
    assert_eq!(calls, [va(&leaf.va), va(&leaf.va)], "{:?}", cfg.callsites);
    let mut call_sites: Vec<u64> = cfg.callsites.iter().map(|c| va(&c.from)).collect();
    let mut ref_sites: Vec<u64> = refs.refs.iter().map(|r| va(&r.from)).collect();
    call_sites.sort_unstable();
    ref_sites.sort_unstable();
    assert_eq!(call_sites, ref_sites, "the two views agree on where the calls are");

    // The planted string is where the symbol table puts it.
    let found = engine
        .send(&Find { query: FindQuery::Text { text: "n0xis-planted-7f3a".into(), utf16: false }, limit: 10 })
        .wait()
        .expect("find");
    if let Some(planted) = nm_address(&exe, "PLANTED") {
        assert!(found.matches.iter().any(|m| va(&m.va) == planted), "nm says {planted:#x}; found {found:?}");
    } else {
        eprintln!("nm unavailable: checked only that the planted string was found");
        assert!(found.count >= 1, "{found:?}");
    }

    // An ELF image has no PE unwind table, and says so rather than reporting zero.
    let profile = engine.send(&Profile { exports: false }).wait().expect("profile");
    assert!(!profile.image.pdata_present);
    assert!(profile.image.sections.iter().any(|s| s.name == ".text" && s.executable), "{:?}", profile.image.sections);

    // Types and bookmarks are written, then read back from the engine.
    let fields = [("0".to_string(), "a".to_string(), "int".to_string()), ("8".into(), "b".into(), "char*".into())];
    engine.send(&DefineStruct::new("Pair", None, &fields).expect("valid struct")).wait().expect("type struct");
    engine.send(&DefineEnum::new("Mode", &[("Off".into(), "0".into()), ("On".into(), "1".into())]).expect("valid enum"))
        .wait()
        .expect("type enum");
    let lib = engine.send(&ListTypes).wait().expect("type list");
    let pair = lib.structs.iter().find(|s| s.name == "Pair").expect("Pair stored");
    assert_eq!(pair.fields.iter().map(|f| (f.offset, f.name.as_str(), f.ctype.as_str())).collect::<Vec<_>>(), [
        (0, "a", "int"),
        (8, "b", "char*")
    ]);
    assert_eq!(lib.enums.iter().find(|e| e.name == "Mode").map(|e| e.members.len()), Some(2));
    assert!(engine.send(&RemoveType { name: "Pair".into() }).wait().expect("type rm"), "Pair existed");
    assert!(engine.send(&ListTypes).wait().expect("type list").structs.iter().all(|s| s.name != "Pair"));

    engine.send(&SetBookmark { addr: leaf.va.clone(), on: true }).wait().expect("bookmark on");
    let marked = |engine: &Engine| {
        engine.send(&ListAnnotations).wait().expect("annotate list").records.iter().any(|r| va(&r.va) == va(&leaf.va) && r.bookmark)
    };
    assert!(marked(&engine));
    engine.send(&SetBookmark { addr: leaf.va.clone(), on: false }).wait().expect("bookmark off");
    assert!(!marked(&engine));

    // A typed console line is split by the engine; a blank one is never sent,
    // since a blank line ends the session.
    let line = engine.call_line("function discover --limit 2".into()).wait().expect("console line");
    assert_eq!(line.meta.schema.as_deref(), Some("n0xis.function.discover.v1"));
    assert!(matches!(engine.call_line("   ".into()).wait(), Err(ClientError::Unsendable(_))));
    assert!(matches!(engine.status(), EngineStatus::Ready { .. }), "the session survived: {:?}", engine.status());
    let _ = std::fs::remove_dir_all(&dir);
}
