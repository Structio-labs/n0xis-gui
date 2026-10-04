# N0xis native GUI (GPUI)

The next desktop front end, built on [GPUI Kit](https://gpui-kit.com) (`gpui-kit`,
Apache-2.0, pinned to an exact version). It grows here, beside the Tauri build in
`../src-tauri` and `../ui`, until it covers everything listed in
[`../docs/CURRENT-UI.md`](../docs/CURRENT-UI.md).

## Layout

| Path | What |
| --- | --- |
| `client/` | `n0xis-client`: the engine seam. Finds the engine, drives one `n0xis serve` session per target on a worker thread, and turns answers into typed values. No UI dependency. |
| `app/` | `n0xis-ui`: the GPUI application. |

The engine always runs as a separate process. A parser that dies on a hostile file
takes its own process down, not the window; the client reports the crash with the
engine's last stderr lines, starts a new session on the next request, and stops
after three crashes in a row instead of reopening a file that kills it every time.

## Run

```sh
cargo run -p n0xis-ui -- /path/to/binary
```

The engine is found as `$N0XIS_BIN`, then `~/.local/bin/n0xis`, then `n0xis` on
`PATH`. A `$N0XIS_BIN` that names no file is an error, not a reason to try another
engine.

## Test

```sh
cargo test -p n0xis-client   # protocol against a stand-in engine + the real engine on a compiled target
cargo test -p n0xis-ui       # the app's data logic
```

To see and drive the window without a GPU (the conditions of a VM), use
`scripts/drive.sh`: it runs the app on a private Xvfb display, replays clicks and
keys with xdotool, and saves screenshots. Xvfb has no compositor, so there the
window falls back to the system frame; `scripts/kwin-shot.sh` runs it inside a
nested, virtual KWin instead, which shows the window's own frame and controls.

The real-engine test compiles a small C function with planted constants and checks
they appear in its decompilation. It skips, saying why, without a C compiler or an
engine. The protocol tests need Python for the stand-in engine.

## Status (2026-10-04)

Done: a dock (panels in tab groups; drag a tab onto another group or an edge; zoom with
Shift+Esc; the arrangement is saved to `~/.config/n0xis/ui-layout.json` and restored, with
undo/redo of layout changes on Ctrl+Shift+Z / Ctrl+Shift+Y and Window ▸ Reset Layout); one title bar (on Linux the window draws its own frame and controls) with a menu that
lists only working actions (Open, Quit, Copy, Select All, About); open a target (argument or file dialog), function list from the unwind table or
a prologue scan (virtualized, filtered), decompiler (three styles, C colouring,
read-only editor with selection and copy), disassembly bounded by the function's
extent, engine status in the status bar.

Every view shows only what the engine returned; an empty or failed view says so.
The rest of the parity checklist in `docs/CURRENT-UI.md` §12 is still open.
