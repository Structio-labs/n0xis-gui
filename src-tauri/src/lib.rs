// N0xis GUI backend — the thin server half of the thin-client GUI.
//
// The engine is not linked in-process; the GUI drives it across the *process seam*
// (the same `n0xis` CLI a human or an agent would call), so the frontend stays a
// pluggable client and the engine can be swapped or upgraded independently.

use serde_json::{json, Value};
use std::process::Command;

/// Resolve the engine binary: an explicit override, then the local user install,
/// then whatever `n0x`/`n0xis` is on PATH. No path is hard-coded into logic.
fn engine_bin() -> String {
    if let Ok(p) = std::env::var("N0XIS_BIN") {
        if !p.is_empty() {
            return p;
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = std::path::Path::new(&home).join(".local/bin/n0xis");
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    "n0xis".to_string()
}

/// Run an arbitrary engine subcommand and return its JSON envelope verbatim.
/// A non-JSON or failed run is wrapped in the same `{ok,error|data}` shape so the
/// frontend never has to special-case transport failures.
fn run_engine(args: &[String]) -> Value {
    let bin = engine_bin();
    match Command::new(&bin).args(args).output() {
        Ok(o) => {
            let txt = String::from_utf8_lossy(&o.stdout);
            match serde_json::from_str::<Value>(&txt) {
                Ok(v) => v,
                Err(_) => json!({
                    "ok": o.status.success(),
                    "data": { "raw": txt },
                    "meta": { "schema": "raw" }
                }),
            }
        }
        Err(e) => json!({
            "ok": false,
            "error": { "message": format!("cannot launch {bin}: {e}") }
        }),
    }
}

/// Generic passthrough: `n0x_run(['guide','--brief'])`.
#[tauri::command]
fn n0x_run(args: Vec<String>) -> Value {
    run_engine(&args)
}

/// Probe: is the engine reachable, and what version / how many commands?
#[tauri::command]
fn engine_info() -> Value {
    let bin = engine_bin();
    let ver = Command::new(&bin).arg("--version").output();
    let version = match ver {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_string(),
        _ => return json!({ "ok": false, "error": { "message": format!("engine '{bin}' not found") } }),
    };
    let guide = run_engine(&["guide".into(), "--brief".into()]);
    let count = guide
        .get("data")
        .and_then(|d| d.get("command_count"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({ "ok": true, "version": version, "commandCount": count, "bin": bin })
}

/// Native file picker for choosing a target binary.
#[tauri::command]
async fn pick_file(app: tauri::AppHandle, title: String) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title(&title)
        .add_filter("Binaries", &["exe", "dll", "so", "elf", "bin", "o", "a"])
        .add_filter("All files", &["*"])
        .pick_file(move |f| {
            let _ = tx.send(f);
        });
    rx.recv()
        .ok()
        .flatten()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![n0x_run, engine_info, pick_file])
        .run(tauri::generate_context!())
        .expect("error while running N0xis GUI");
}
