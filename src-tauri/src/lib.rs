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
async fn n0x_run(args: Vec<String>) -> Value {
    run_engine(&args)
}

/// Probe: is the engine reachable, and what version / how many commands?
#[tauri::command]
async fn engine_info() -> Value {
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

/// A target path passed on the command line (`n0xis-gui /path/to/bin`), if any.
/// Lets the app open straight from a terminal or file manager.
#[tauri::command]
fn initial_target() -> Option<String> {
    std::env::args().nth(1).filter(|a| {
        !a.starts_with('-') && std::path::Path::new(a).is_file()
    })
}

/// List running processes with USEFUL names. The engine's `process ps` reads a
/// name that, for Proton/Wine games, is just "wine-preloader"; the kernel's
/// /proc/<pid>/comm carries the real exe name ("Sam2.exe"). On Linux we read
/// that directly; elsewhere we return empty so the caller falls back to the engine.
#[tauri::command]
async fn list_processes() -> Value {
    #[allow(unused_mut)]
    let mut procs: Vec<Value> = Vec::new();
    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for e in entries.flatten() {
            let s = e.file_name();
            let s = s.to_string_lossy();
            if let Ok(pid) = s.parse::<u32>() {
                let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
                    .map(|c| c.trim().to_string())
                    .unwrap_or_default();
                if !comm.is_empty() {
                    procs.push(json!({ "pid": pid, "name": comm }));
                }
            }
        }
    }
    json!({ "ok": true, "data": { "count": procs.len(), "processes": procs } })
}

/// Resolve real per-process icons (Linux). Two routes:
///   • Steam/Proton games: /proc/<pid>/environ carries SteamAppId=<n> → the
///     freedesktop icon 'steam_icon_<n>' (so a Wine game like Sam2.exe gets its
///     Steam library icon even though its process name doesn't match a launcher).
///   • Everything else: match /proc/<pid>/comm to a .desktop's Exec/WMClass/Name.
/// Returns { pid: "data:image/png;base64,…" } only for processes we could resolve.
#[tauri::command]
async fn process_icons() -> Value {
    #[allow(unused_mut)]
    let mut icons = serde_json::Map::new();
    #[cfg(target_os = "linux")]
    {
        use base64::Engine as _;
        use std::collections::HashMap;
        use std::path::Path;

        let home = std::env::var("HOME").unwrap_or_default();

        // ---- build a .desktop index: key (exec-basename / wmclass / name) -> icon name
        let mut index: HashMap<String, String> = HashMap::new();
        let app_dirs = [
            format!("{home}/.local/share/applications"),
            "/usr/share/applications".to_string(),
            "/usr/local/share/applications".to_string(),
        ];
        for dir in &app_dirs {
            let rd = match std::fs::read_dir(dir) { Ok(r) => r, Err(_) => continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("desktop") { continue; }
                let txt = match std::fs::read_to_string(&p) { Ok(t) => t, Err(_) => continue };
                let (mut icon, mut exec, mut wmclass, mut name) = (None, None, None, None);
                for line in txt.lines() {
                    if let Some(v) = line.strip_prefix("Icon=") { if icon.is_none() { icon = Some(v.trim().to_string()); } }
                    else if let Some(v) = line.strip_prefix("Exec=") { if exec.is_none() { exec = Some(v.trim().to_string()); } }
                    else if let Some(v) = line.strip_prefix("StartupWMClass=") { if wmclass.is_none() { wmclass = Some(v.trim().to_string()); } }
                    else if let Some(v) = line.strip_prefix("Name=") { if name.is_none() { name = Some(v.trim().to_string()); } }
                }
                let icon = match icon { Some(i) if !i.is_empty() => i, _ => continue };
                let mut add = |k: String| { let k = k.to_lowercase(); if !k.is_empty() { index.entry(k).or_insert_with(|| icon.clone()); } };
                if let Some(ex) = exec {
                    // basename of the first token, stripped of a .exe suffix
                    if let Some(tok) = ex.split_whitespace().find(|t| !t.starts_with('%')) {
                        let base = tok.rsplit(['/', '\\']).next().unwrap_or(tok);
                        add(base.trim_end_matches(".exe").to_string());
                    }
                }
                if let Some(w) = wmclass { add(w); }
                if let Some(n) = name { add(n); }
            }
        }

        // ---- resolve an icon NAME to a data-uri (cached), searching the icon theme
        let mut cache: HashMap<String, Option<String>> = HashMap::new();
        let icon_dirs = [
            format!("{home}/.local/share/icons/hicolor"),
            "/usr/share/icons/hicolor".to_string(),
            format!("{home}/.icons"),
        ];
        let sizes = ["48x48", "64x64", "96x96", "128x128", "256x256", "32x32", "24x24"];
        let read_uri = |path: &Path| -> Option<String> {
            let bytes = std::fs::read(path).ok()?;
            if bytes.is_empty() || bytes.len() > 262_144 { return None; } // skip huge files
            let mime = match path.extension().and_then(|x| x.to_str()) {
                Some("svg") => "image/svg+xml",
                _ => "image/png",
            };
            Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)))
        };
        let mut resolve = |name: &str| -> Option<String> {
            if let Some(hit) = cache.get(name) { return hit.clone(); }
            let mut found = None;
            if name.starts_with('/') {
                if Path::new(name).is_file() { found = read_uri(Path::new(name)); }
            } else {
                'outer: for base in &icon_dirs {
                    for sz in &sizes {
                        let p = format!("{base}/{sz}/apps/{name}.png");
                        if Path::new(&p).is_file() { found = read_uri(Path::new(&p)); break 'outer; }
                    }
                }
                if found.is_none() {
                    for base in &icon_dirs {
                        let p = format!("{base}/scalable/apps/{name}.svg");
                        if Path::new(&p).is_file() { found = read_uri(Path::new(&p)); break; }
                    }
                }
                if found.is_none() {
                    for ext in ["png", "svg"] {
                        let p = format!("/usr/share/pixmaps/{name}.{ext}");
                        if Path::new(&p).is_file() { found = read_uri(Path::new(&p)); break; }
                    }
                }
            }
            cache.insert(name.to_string(), found.clone());
            found
        };

        // ---- walk processes, pick an icon name, resolve it
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for e in entries.flatten() {
                let s = e.file_name();
                let s = s.to_string_lossy();
                let pid: u32 = match s.parse() { Ok(p) => p, Err(_) => continue };
                let mut icon_name: Option<String> = None;
                // 1) Steam appid from the environment
                if let Ok(environ) = std::fs::read(format!("/proc/{pid}/environ")) {
                    for kv in environ.split(|&b| b == 0) {
                        if let Ok(kv) = std::str::from_utf8(kv) {
                            if let Some(id) = kv.strip_prefix("SteamAppId=") {
                                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                                    icon_name = Some(format!("steam_icon_{id}"));
                                    break;
                                }
                            }
                        }
                    }
                }
                // 2) match the process name to a .desktop
                if icon_name.is_none() {
                    if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                        let key = comm.trim().trim_end_matches(".exe").to_lowercase();
                        icon_name = index.get(&key).cloned();
                    }
                }
                if let Some(name) = icon_name {
                    if let Some(uri) = resolve(&name) {
                        icons.insert(pid.to_string(), Value::String(uri));
                    }
                }
            }
        }
    }
    json!({ "ok": true, "data": { "icons": icons } })
}

/// Persistent function-list cache, keyed by target path + validated by mtime, so
/// re-opening a huge binary (371k functions) is instant instead of re-streaming.
/// Stored under the app cache dir as one JSON blob per target.
fn fncache_path(app: &tauri::AppHandle, path: &str) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    let dir = app.path().app_cache_dir().ok()?.join("fncache");
    let _ = std::fs::create_dir_all(&dir);
    let safe: String = path.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let key = format!("{:x}", md5ish(path));   // short stable key + a readable tail
    Some(dir.join(format!("{key}_{}.json", &safe[safe.len().saturating_sub(24)..])))
}
// tiny non-crypto hash (FNV-1a) — just to key cache files, not for security
fn md5ish(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}
fn file_mtime(path: &str) -> u64 {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs()).unwrap_or(0)
}
/// Return the cached function-list JSON for `path` iff the file hasn't changed.
#[tauri::command]
async fn fncache_get(app: tauri::AppHandle, path: String) -> Option<String> {
    let p = fncache_path(&app, &path)?;
    let raw = std::fs::read_to_string(p).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    if v.get("mtime").and_then(|m| m.as_u64()) == Some(file_mtime(&path)) {
        v.get("data").map(|d| d.to_string())
    } else { None }
}
/// Store the function-list JSON for `path`, stamped with the current mtime.
#[tauri::command]
async fn fncache_put(app: tauri::AppHandle, path: String, data: String) -> bool {
    if let Some(p) = fncache_path(&app, &path) {
        let payload = json!({ "mtime": file_mtime(&path), "data": serde_json::from_str::<Value>(&data).unwrap_or(Value::Null) });
        return std::fs::write(p, payload.to_string()).is_ok();
    }
    false
}

/// A persistent engine session: one long-running `n0xis serve --file <target>`
/// that keeps the parsed image resident, so decomp/disasm/xref/ir calls reuse it
/// instead of re-loading the (possibly 233 MB) binary per click.
struct EngineSess {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    out: std::io::BufReader<std::process::ChildStdout>,
}
static SESSION: std::sync::Mutex<Option<EngineSess>> = std::sync::Mutex::new(None);

fn kill_session(slot: &mut Option<EngineSess>) {
    if let Some(mut s) = slot.take() {
        let _ = s.child.kill();
        let _ = s.child.wait();
    }
}

/// Open (or replace) the persistent session for `path`. Returns the engine's
/// `ready` envelope. On any spawn/IO failure the GUI silently falls back to
/// one-shot calls.
#[tauri::command]
async fn session_open(path: String) -> Value {
    use std::io::BufRead;
    let bin = engine_bin();
    let mut child = match std::process::Command::new(&bin)
        .args(["serve", "--file", &path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return json!({ "ok": false, "error": { "message": format!("spawn {bin}: {e}") } }),
    };
    let stdin = match child.stdin.take() { Some(s) => s, None => return json!({ "ok": false, "error": { "message": "no stdin" } }) };
    let mut out = std::io::BufReader::new(match child.stdout.take() { Some(s) => s, None => return json!({ "ok": false, "error": { "message": "no stdout" } }) });
    let mut ready = String::new();
    let _ = out.read_line(&mut ready);
    let mut slot = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    kill_session(&mut slot);
    *slot = Some(EngineSess { child, stdin, out });
    serde_json::from_str::<Value>(ready.trim()).unwrap_or_else(|_| json!({ "ok": true, "data": { "ready": true } }))
}

/// Run one command through the open session (args as a vector, like `n0x_run`).
/// Returns the engine's JSON envelope, or `{ok:false}` if there is no session.
#[tauri::command]
async fn session_query(args: Vec<String>) -> Value {
    use std::io::{BufRead, Write};
    let mut slot = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    let sess = match slot.as_mut() {
        Some(s) => s,
        None => return json!({ "ok": false, "error": { "message": "no session" } }),
    };
    // Join args into one line; wrap any arg with whitespace in double quotes
    // (split_command_line strips quotes, no backslash escaping).
    let line = args
        .iter()
        .map(|a| if a.is_empty() || a.chars().any(|c| c.is_whitespace()) { format!("\"{a}\"") } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ");
    if writeln!(sess.stdin, "{line}").is_err() || sess.stdin.flush().is_err() {
        kill_session(&mut slot);
        return json!({ "ok": false, "error": { "message": "session write failed" } });
    }
    let mut resp = String::new();
    match sess.out.read_line(&mut resp) {
        Ok(0) => { kill_session(&mut slot); json!({ "ok": false, "error": { "message": "session closed" } }) }
        Ok(_) => serde_json::from_str::<Value>(resp.trim()).unwrap_or_else(|_| json!({ "ok": false, "error": { "message": "bad session response" } })),
        Err(e) => { kill_session(&mut slot); json!({ "ok": false, "error": { "message": format!("session read: {e}") } }) }
    }
}

#[tauri::command]
async fn session_close() {
    let mut slot = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    kill_session(&mut slot);
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
        .invoke_handler(tauri::generate_handler![n0x_run, engine_info, pick_file, initial_target, list_processes, process_icons, fncache_get, fncache_put, session_open, session_query, session_close])
        .run(tauri::generate_context!())
        .expect("error while running N0xis GUI");
}
