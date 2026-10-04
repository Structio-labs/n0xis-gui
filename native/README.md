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

## Status (2026-10-05)

Shell: one title bar (on Linux the window draws its own frame and controls) with a menu
that lists only working actions; open a target from the command line or a file dialog;
engine status and the selection in the status bar. Themes: the Tauri build's six,
generated from its stylesheet by `app/themes/build_themes.py`, plus your own GPUI Kit
theme files in `~/.config/n0xis/themes/`, re-read when they change and reported when one
cannot be read; View ▸ Theme; zoom with Ctrl+= / Ctrl+- / Ctrl+0; both kept in
`~/.config/n0xis/ui-settings.json`. Dock: panels in tab groups, drag a tab onto another
group or an edge, Shift+Esc zooms one; the arrangement is saved to
`~/.config/n0xis/ui-layout.json`, with layout undo/redo on Ctrl+Shift+Z / Ctrl+Shift+Y and
Window ▸ Reset Layout.

Panels. Every one is an ordinary widget: View ▸ Panels opens it, and it can be closed,
moved and opened again without losing what it holds.

- Functions: from the unwind table or a prologue scan, virtualized, filtered.
- Decompiler: three styles, C colouring, a read-only editor with selection and copy.
- Disassembly: bounded by the function's extent; the selected instruction is highlighted,
  and a branch or call whose target the engine resolved is followed with a click.
- Graph: the control flow of the selected function (`ir build`), laid out in rows with
  edges routed at right angles and coloured by the kind the engine gives them (its own
  names are the legend). Wheel zooms around the pointer, drag pans, a block can be dragged
  and put back, a click on a block goes to it, and the view fits itself until you move
  it. Functions over 400 blocks are not drawn, and the view says so. Known limit: an edge
  that skips several rows runs down a side lane, so very wide functions get long lanes.
- Cross-references: who references the selection (`xref to`), and for a function the calls
  it makes as the engine resolved them (`ir build`).
- Triage: the image's own headers (`profile --exports`): sections, exports with forwarders
  marked, and the engine's advisories.
- Bookmarks: bookmarks first, then every other annotated address (`annotate list`).
  Edit ▸ Toggle Bookmark (Ctrl+D).
- Types: struct and enum definitions. Each edit sends the whole type, then reads the list
  back, so what is shown is what the engine stored.
- Find: text, UTF-16, byte patterns with wildcards, or escaped text (`find`); at most 300
  matches, and it says when there are more.
- Console: any engine command in the target's session, without `--file`; ↑ / ↓ history;
  the engine's own JSON, with Copy.

Icons: the Tauri build's whole set, 60 of them (its Lucide icons, ISC licence in
`app/icons/LICENSE-LUCIDE`, and the ones drawn inline on its page), written out as SVG files
by `app/icons/extract_icons.py` and embedded at build time; `extract_icons.py --verify`
renders each next to the page's own markup and compares them pixel by pixel. Each panel's
tab carries the icon the Tauri build gave that widget.

A click in any panel goes to that address. The function it lies in comes from the
engine's function list, and only where the list states the function's extent; otherwise
the address is shown as an address.

Every view shows only what the engine returned; an empty or failed view says so. Still
open from `docs/CURRENT-UI.md` §12: the linear listing, the memory scanner, editing (rename, comment, types of variables) with undo, go to and history, and
the settings.
