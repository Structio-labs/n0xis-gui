# N0xis GUI

The desktop front-end for **N0xis** — the AI-first reverse-engineering CLI. A beginner-friendly
but pro-capable UI for static *and* dynamic analysis (Cheat-Engine-style), built as a **thin client**
over the existing `n0xis` engine.

> Private prototype. The engine lives in its own repo; this repo is only the GUI.

## Architecture — thin client over the process seam

```
┌───────────────────────────┐        invoke()        ┌────────────────────────┐        argv / JSON        ┌──────────────┐
│  Web UI  (ui/)            │ ────────────────────▶  │  Tauri backend         │ ───────────────────────▶  │  n0xis CLI   │
│  index.html · app.js      │ ◀────────────────────  │  src-tauri/ (Rust)     │ ◀───────────────────────  │  (engine)    │
│  framework-free, themeable│    {ok,data,meta}      │  n0x_run / engine_info │      {ok,data,meta} env.  │              │
└───────────────────────────┘                        └────────────────────────┘                           └──────────────┘
```

* **The engine is never linked in-process.** The backend drives it across the same *process seam*
  a human or an agent would use (`n0xis <cmd> --json`), so the engine stays swappable and the GUI
  stays a pluggable client. See [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs).
* **The UI is self-sufficient.** Opened in a plain browser (no backend), [`ui/bridge.js`](ui/bridge.js)
  detects the absence of Tauri and every engine call resolves to `null`, so the UI falls back to its
  built-in demo data. That is how the design preview and screenshots run.
* **Engine binary resolution** (no hard-coded path): `$N0XIS_BIN` → `~/.local/bin/n0xis` → `n0xis` on `PATH`.

## Features

- Frameless custom window (own title bar, drag regions, min/max/close wired to the OS window).
- **Everything is a widget — no privileged core.** Functions, Decompiler, Disassembly, CFG graph,
  Registers, Watchpoints, Memory scanner, Live memory, Watchlist, Copilot, Details, Output … are
  all dock widgets from one catalog. The **Decompile / Static / Graph / Dynamic** workspaces are
  just *default dock layouts* over that catalog ([`app.js`](ui/app.js) `DEFAULT_LAYOUTS`), so every
  built-in panel is itself resizable, movable, tabbable and closable — same as any widget you add.
- **Blender/AreaKit tiling dock** ([`ui/dock.js`](ui/dock.js)) — a BSP tree of areas:
  - drag an area header (or a palette widget) onto another area — near an **edge** it splits,
    over the **centre** it takes over; hold **Ctrl** to drop it as a **tab** instead;
  - tab-strips scroll and can sit on any side (a vertical strip shows icons); right-click a
    tab for tab / tab-strip options;
  - drag the line between areas to **resize** (proportions preserved); **undo/redo**
    (`Ctrl+Shift+Z` / `Ctrl+Shift+X`); every change **auto-saved** to `localStorage`.
  - the *Add widget* palette (10 pane types); the type marker (top-left of an area) re-points it.
- **Control-flow graph** with aligned edges, true/false/loop colouring, pan + zoom.
- **Right-click context menus** everywhere (functions, code, disassembly, watchpoints, scan hits, graph blocks).
- Command palette (`Ctrl+P`), 6 live-switchable colour themes, out-of-the-box zoom (`Ctrl +/−/0`).
- Dynamic analysis surface: registers, watchpoints, memory scanner, live memory, watchlist, following-AI Copilot.
- Pluggable, model-agnostic AI Copilot (cloud API · local Ollama · external MCP agent · off).

## Running

### Web preview (no engine, demo data)
```bash
cd ui && python3 -m http.server 5177
# open http://127.0.0.1:5177/index.html
# deep links: ?ws=graph · ?ws=dynamic · ?widgets=1 · ?settings=1
```

### Native app (Tauri, connected to the real engine)
```bash
# needs: Rust, the Tauri v2 CLI (cargo tauri), webkit2gtk-4.1, and `n0xis` reachable
cargo tauri dev        # from repo root, or: cd src-tauri && cargo run
```

## Layout

| Path | What |
| --- | --- |
| `ui/` | the web front-end (this is what ships into the webview) |
| `ui/bridge.js` | the thin-client seam: Tauri detection + engine calls + graceful fallback |
| `src-tauri/` | the Rust backend: window + engine bridge commands |
| `src-tauri/src/lib.rs` | `n0x_run`, `engine_info`, `pick_file` |
