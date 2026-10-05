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
that lists only working actions; open a target from the command line, a file dialog, or
the last eight opened (File ▸ Open Recent and the start screen, kept in
`~/.config/n0xis/recent.json`; one no longer there is taken off the list, saying so);
the engine's own version (from its banner), its status and the selection in the status bar. Themes: the Tauri build's six,
generated from its stylesheet by `app/themes/build_themes.py`, plus your own GPUI Kit
theme files in `~/.config/n0xis/themes/`, re-read when they change and reported when one
cannot be read; View ▸ Theme; zoom with Ctrl+= / Ctrl+- / Ctrl+0; both kept in
`~/.config/n0xis/ui-settings.json`. Dock: panels in tab groups, drag a tab onto another
group or an edge, Shift+Esc zooms one; the arrangement is saved to
`~/.config/n0xis/ui-layout.json`, with layout undo/redo on Ctrl+Shift+Z / Ctrl+Shift+Y and
Window ▸ Reset Layout.

Panels. Every one is an ordinary widget: View ▸ Panels opens it (back in the group it
belongs with), and it can be closed, moved and opened again without losing what it holds.

- Functions: from the unwind table or a prologue scan, virtualized, filtered.
- Decompiler: three styles, C colouring, a read-only editor with selection and copy.
- Disassembly: bounded by the function's extent; a click selects an instruction (a
  comment goes there), and a double click on a branch or call whose target the engine
  resolved follows it.
- Graph: the control flow of the selected function (`ir build`), laid out in rows with
  edges routed at right angles and coloured by the kind the engine gives them (its own
  names are the legend). Wheel zooms around the pointer, drag pans, a block can be dragged
  and put back, a click on a block goes to it, and the view fits itself until you move
  it. Functions over 400 blocks are not drawn, and the view says so. Known limit: an edge
  that skips several rows runs down a side lane, so very wide functions get long lanes.
- Linear: the whole program as one listing, function after function in address order,
  each cut at the end the function list states (or, where it states none, at the next
  function, and the header says so). Asm or Pseudo. Functions are fetched as they come
  into view, a few at a time, and dropped again far from it; a minimap spans the whole
  address range (named functions in the accent colour, fetched ones brighter) and a click
  or drag on it jumps. Click and Shift+click select lines; Ctrl+C copies them.
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
- Memory scanner: pick a running process (`process ps`), see whether it can be read
  (`mem map`), find the addresses that hold a value (`scan value`, ten types) and narrow
  them as it changes (`scan filter`: equals, changed, unchanged, increased, decreased). It
  needs no open file: each request runs the engine once against the process, and the start
  screen offers it. Where the system will not let a process be read (on Linux, Yama's
  `ptrace_scope`), the engine's own explanation is what the panel shows.

Icons: the Tauri build's whole set, 60 of them (its Lucide icons, ISC licence in
`app/icons/LICENSE-LUCIDE`, and the ones drawn inline on its page), written out as SVG files
by `app/icons/extract_icons.py` and embedded at build time; `extract_icons.py --verify`
renders each next to the page's own markup and compares them pixel by pixel. Each panel's
tab carries the icon the Tauri build gave that widget.

A click in any panel goes to that address. The function it lies in comes from the
engine's function list, and only where the list states the function's extent; otherwise
the address is shown as an address.

Editing. Edit ▸ Rename (F2) names the selected function, or the variable under the caret
in the decompiler; Comment (Ctrl+/) annotates the selected instruction; Set Type gives a
parameter or a local a C type, Set Return Type the function's result; Toggle Bookmark
(Ctrl+D); Clear Annotations Here removes everything recorded at the address. Which words
are variables, and the key a rename is stored under, come from the engine's own variable
list (`decomp pseudo` → `variables`); a value that is neither a parameter nor a local is
refused a type, because the engine would store it and change nothing. Each edit reads the
current value from the engine first, so Ctrl+Z / Ctrl+Y (200 steps) put back what the
engine held, and every view that shows the edited fact asks again. A renamed function's
entry is read again from the engine's own listing.

Navigation. Go ▸ Go to (Ctrl+G) takes a listed function's exact name, or an address.
Back and Forward (Alt+← / Alt+→, or the mouse's side buttons) walk the places gone to,
300 of them; a click on a row moves the selection without becoming a step.

Analysis. Opening a target also starts the engine's whole-program pass (`analyze
--no-cfg`) as a process of its own, in the target's project folder: it discovers
functions, recovers class names and writes the reverse-reference index, which the session
then reads. Its phase is in the status bar, and its counts once it is done. Until the
index is written, Cross-references holds its `xref to` back and says so, rather than make
the session build the same index a second time and wait on it. When the pass found class
or library names, the function list and the views that show names are read again.

Settings (File ▸ Settings…, Ctrl+,), kept in `~/.config/n0xis/settings.json`: whether the
analysis runs when a target opens, and whether it warms the decompiler cache (Analyze ▸ Run
Analysis starts it by hand); where projects live (central, a `<file>.n0xis` folder beside
the target, or a folder you choose; a place that cannot be used falls back to central and
says so); whether caches are kept when a target closes; and what the open target's caches
take, with a button that clears them. Which folders are caches is the engine's answer
(`project cache`), so names, comments and types are never cleared. Theme and font size are
there too.

Workspaces. Decompile, Static, Graph and Dynamic, in a bar under the title (and Window ▸
Workspace), each with its own arrangement and its own layout undo; the panels are shared,
so a view keeps what it holds in every workspace. The bar's Widget palette lists every
panel: a click opens it where it belongs, a drag puts it where it is dropped. A group's ⋯
menu ▸ Show instead puts another panel in the place of the one shown.

Every view shows only what the engine returned; an empty or failed view says so. Still
open from `docs/CURRENT-UI.md` §12: editable key bindings.
