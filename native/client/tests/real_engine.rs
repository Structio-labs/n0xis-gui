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

use n0xis_client::{
    Annotate, DecompStyle, Decompile, Disassemble, DiscoverFunctions, Engine, EngineCommand, EngineStatus, Note,
    RenameVariable, SetVariableType, ShowAnnotations, TypeSubject, VariableKind,
};

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
    build_target_in("real")
}

/// Each test gets its own directory, so one test's edits never reach another's
/// project. The `.n0x/` in it is what makes the engine keep the project there:
/// without one it falls back to the user's global project, and a test would
/// write its names into that.
fn build_target_in(tag: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("n0xis-client-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".n0x")).ok()?;
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
    // What the engine says about itself on its own command line, to hold the
    // banner's version against.
    let own_version = Command::new(engine.program()).arg("--version").output().ok().and_then(|o| {
        String::from_utf8(o.stdout).ok().and_then(|s| s.split_whitespace().nth(1).map(str::to_string))
    });
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

    match engine.status() {
        EngineStatus::Ready { version, .. } => {
            assert!(own_version.is_some(), "`--version` printed no version");
            assert_eq!(version, own_version, "the banner's version is the one the engine prints");
        }
        other => panic!("{other:?}"),
    }
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

/// A process that prints the address of the value it holds, and changes the
/// value on SIGUSR1: the scan's answer is known before the scan runs.
const HOLDER_SOURCE: &str = r#"
#include <signal.h>
#include <stdio.h>
#include <unistd.h>
volatile int planted = 1592594996;
static void bump(int s) { (void)s; planted = 777777; }
int main(void) {
    signal(SIGUSR1, bump);
    printf("%p\n", (void *)&planted);
    fflush(stdout);
    for (;;) pause();
}
"#;

/// Kills the holder however the test ends: a holder left running keeps the
/// test's output pipe open, and whatever reads it waits forever.
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Whether this process may read a process that is not its descendant.
fn may_read_other_processes() -> Result<(), String> {
    let scope = std::fs::read_to_string("/proc/sys/kernel/yama/ptrace_scope").map(|s| s.trim().to_string());
    let root = Command::new("id").arg("-u").output().is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "0");
    match scope.as_deref() {
        _ if root => Ok(()),
        Ok("0") | Err(_) => Ok(()),
        Ok(other) => Err(format!("yama ptrace_scope is {other} and this test is not root, so the engine may not read the holder")),
    }
}

#[test]
fn the_scanner_finds_the_address_a_process_printed_and_narrows_on_change() {
    use n0xis_client::{Narrow, ScanFilter, ScanValue};
    use std::io::BufRead as _;
    let engine = match EngineCommand::locate() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("SKIPPED: {e}");
            return;
        }
    };
    if let Err(why) = may_read_other_processes() {
        eprintln!("SKIPPED: {why}");
        return;
    }
    let dir = std::env::temp_dir().join(format!("n0xis-client-scan-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".n0x")).expect("temp project");
    let (src, exe) = (dir.join("holder.c"), dir.join("holder"));
    std::fs::write(&src, HOLDER_SOURCE).expect("source");
    if !Command::new("cc").arg("-O0").arg("-o").arg(&exe).arg(&src).status().is_ok_and(|s| s.success()) {
        eprintln!("SKIPPED: no C compiler (cc)");
        return;
    }
    let mut holder = KillOnDrop(
        Command::new(&exe)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("start the holder"),
    );
    let mut printed = String::new();
    std::io::BufReader::new(holder.0.stdout.take().expect("stdout")).read_line(&mut printed).expect("the holder prints its address");
    let planted = va(printed.trim());
    let pid = holder.0.id();

    let first = engine
        .run_request(&ScanValue { pid, value_type: "i32".into(), value: "1592594996".into(), save_as: "first".into() }, Some(&dir))
        .expect("scan value");
    assert!(first.matches.iter().any(|m| va(&m.addr) == planted), "the printed address {planted:#x} is among {:?}", first.matches);

    let _ = Command::new("kill").arg("-USR1").arg(pid.to_string()).status();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let narrowed = engine
        .run_request(&ScanFilter { pid, from: "first".into(), narrow: Narrow::Changed, save_as: "second".into() }, Some(&dir))
        .expect("scan filter");
    let kept = narrowed.matches.iter().find(|m| va(&m.addr) == planted).expect("the changed value is kept");
    assert_eq!(kept.value.to_string(), "777777");
    drop(holder);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every edit the GUI makes, sent through the client and read back from the
/// views that show it. The names and the comment are planted here, so the right
/// answer is known before the engine is asked.
#[test]
fn edits_reach_what_the_views_show_and_come_off_again() {
    let engine = match EngineCommand::locate() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("SKIPPED: {e}");
            return;
        }
    };
    let Some(target) = build_target_in("edits") else {
        eprintln!("SKIPPED: no C compiler (cc) to build the known target");
        return;
    };
    let project = target.parent().map(|p| p.to_path_buf());
    let engine = Engine::start(engine, target, project);
    let list = || engine.send(&DiscoverFunctions { from_unwind_table: false, limit: 1000, offset: 0 }).wait().expect("function list");
    let mix = list().functions.into_iter().find(|f| f.name == "mix").expect("the symbol table names `mix`");
    let decompile = || engine.send(&Decompile { addr: mix.va.clone(), style: DecompStyle::Ssa }).wait().expect("decompile");

    // The parameter, as the engine lists it.
    let first = decompile();
    let variables = first.variables.clone().expect("this engine lists variables");
    let param = variables.iter().find(|v| v.kind == VariableKind::Param).unwrap_or_else(|| panic!("no parameter in {variables:?}"));

    // A variable rename reaches the text and the list, under the same key.
    let record = engine
        .send(&RenameVariable { function: mix.va.clone(), key: param.key.clone(), value: Some("seed".into()) })
        .wait()
        .expect("rename a variable");
    assert_eq!(record.var_names.get(&param.key).map(String::as_str), Some("seed"));
    let renamed = decompile();
    let text = renamed.pseudo.join("\n");
    assert!(renamed.variables.as_ref().unwrap().iter().any(|v| v.name == "seed" && v.key == param.key), "{:?}\n{text}", renamed.variables);
    assert!(renamed.pseudo[0].contains("seed"), "the signature shows the new name: {text}");

    // A type on that parameter reaches the signature.
    engine
        .send(&SetVariableType { function: mix.va.clone(), subject: TypeSubject::Variable(param.key.clone()), value: Some("int".into()) })
        .wait()
        .expect("type a parameter");
    let typed = decompile();
    assert!(typed.pseudo[0].contains("int seed"), "{}", typed.pseudo[0]);

    // A function rename reaches the list and the signature.
    engine.send(&Annotate { note: Note::Name, addr: mix.va.clone(), value: Some("mix_planted".into()) }).wait().expect("rename");
    assert!(list().functions.iter().any(|f| f.va == mix.va && f.name == "mix_planted"), "the list shows the new name");
    assert!(decompile().pseudo[0].contains("mix_planted"));

    // A comment reaches the disassembly at its own instruction.
    let insns = engine.send(&Disassemble { addr: mix.va.clone(), count: 4 }).wait().expect("disassemble").insns;
    let at = insns[1].va.clone();
    engine.send(&Annotate { note: Note::Comment, addr: at.clone(), value: Some("planted comment".into()) }).wait().expect("comment");
    let insns = engine.send(&Disassemble { addr: mix.va.clone(), count: 4 }).wait().expect("disassemble").insns;
    assert_eq!(insns[1].comment.as_deref(), Some("planted comment"));
    assert_eq!(insns[0].comment, None, "only the commented instruction carries it");

    // Clearing puts back what the engine found by itself.
    engine.send(&Annotate { note: Note::Name, addr: mix.va.clone(), value: None }).wait().expect("clear the name");
    assert!(list().functions.iter().any(|f| f.va == mix.va && f.name == "mix"), "the symbol's own name is back");
    engine
        .send(&RenameVariable { function: mix.va.clone(), key: param.key.clone(), value: None })
        .wait()
        .expect("clear the rename");
    let back = decompile();
    assert!(back.variables.unwrap().iter().any(|v| v.name == param.name && v.key == param.key), "the synthesized name is back");

    // An address nothing was written at has no record, which is not a failure.
    assert_eq!(engine.send(&ShowAnnotations { addr: "0x10".into() }).wait(), Ok(None));
}
