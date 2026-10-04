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
