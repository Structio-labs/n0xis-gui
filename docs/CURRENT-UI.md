# Current UI inventory (Tauri build)

What the current desktop UI does, written so a rewrite can reach parity without
dropping behaviour. Snapshot of `main` at `818c464` (2026-09-04), read from the
source. Every item says where its data comes from:

- **engine**: the item calls the `n0xis` engine and shows its answer;
- **demo**: the item shows built-in sample content or only a toast, with no engine call.

Section 11 lists defects found while writing this (by reading the code, not by
running it). Section 12 is the parity checklist.

---

## 1. Architecture

| Part | What it is |
| --- | --- |
| Shell | Tauri v2. Frameless window 1440×900, minimum 900×600, identifier `pro.n0xis.gui`. `withGlobalTauri: true`, `csp: null`. Permissions: window minimize/maximize/close/drag, file dialog. |
| Web UI | `ui/index.html`, `ui/styles.css`, `ui/app.js` (about 2 500 lines), `ui/dock.js` (tiling dock), `ui/bridge.js` (the seam to the backend). No framework. |
| Backend | `src-tauri/src/lib.rs`, commands below. |
| Engine binary | `$N0XIS_BIN`, then `~/.local/bin/n0xis`, then `n0xis` on `PATH`. |
| Project directory per target | Holds the engine's `.n0x/`. Central by default: `~/.local/share/pro.n0xis.gui/projects/<FNV-1a of the path>/`. Alternatives: beside the binary, or a chosen folder. |

Backend commands:

| Command | Does |
| --- | --- |
| `n0x_run(args)` | One-shot `n0xis <args>`. Non-JSON stdout is wrapped as `{ok, data:{raw}}`. |
| `engine_info` | `--version` plus `guide --brief`: version, command count, binary path. |
| `initial_target` | `argv[1]` when it is an existing file (`n0xis-gui /path/to/binary`). |
| `pick_file`, `pick_folder` | Native dialogs. File filters: exe, dll, so, elf, bin, o, a, all files. |
| `list_processes` | Linux: pid and name from `/proc`. Other systems: empty, the UI falls back to `process ps`. |
| `process_icons` | Linux: an icon per process, resolved from desktop entries and the icon theme. |
| `fncache_get`, `fncache_put` | Function list cached on disk per target path, valid while the file's mtime is unchanged. |
| `session_open`, `session_query`, `session_close` | One resident `n0xis serve --file <target>` process, working directory = the project directory. One command line in, one JSON line out, behind one mutex. |
| `analyze_start`, `analyze_status` | Background `n0xis analyze --file <target>`: `--no-cfg` by default, `--limit N` or every function when warm-up is on. Progress parsed from stderr lines `[n0x] {phase,done,total}`, polled every 300 ms. |
| `cache_info`, `clear_cache` | Size of, or delete, `.n0x/ir-cache` and `.n0x/xref-index`. Never touches annotations, patches or tables. |
| `set_discard_on_close` | Arms deleting that derived cache when the window closes. |

How the UI calls the engine:

- `eng(args)` goes through the resident session when one is open, otherwise one-shot.
  If the session itself fails, it switches to one-shot calls for good.
- Results are kept in an LRU cache (800 entries) keyed by command, file, address and
  style. The cache is cleared on every annotation change and on opening a target.
- A response that arrives after the selection has moved on is dropped (a selection
  sequence number).
- The "Loading…" indicator appears only after a call has been in flight 180 ms and
  stays at least 320 ms, so cached calls never flash it.

Web-preview mode: without Tauri every bridge call resolves to `null` and the UI keeps
its demo content (used for screenshots). Deep links: `?ws=decompile|static|graph|dynamic`,
`#<workspace>`, `?settings=1`, `?palette=1`, `?widgets=1`.

## 2. Window chrome

- **Title bar**: logo; menus File, Edit, View, Analyze, Debug, Window, Help; target chip
  (`name · static|live`, click opens the File menu); buttons for the command palette,
  attach, settings; window controls wired to the OS window.
- **Workspace bar**: Decompile, Static, Graph, Dynamic; `+` (toast only); `Widget`
  (add-widget palette); layout undo/redo buttons.
- **Launcher** (shown with no target): three cards, "Analyze a file" (file picker),
  "Launch & attach" and "Attach to a process" (both open the process picker); three help
  links (toast only).
- **Status bar**: mode dot and text (`static · PE <machine>` or `live · pid · name`),
  selected address, selected name, analysis status with a spinner, a fixed `FLIRT · WARP`
  label, zoom percentage, and an engine banner (`engine n0xis <version> · <n> cmds`) when
  the engine is found.

## 3. Menus

| Menu | Items |
| --- | --- |
| File | New project · Open file… (Ctrl+O) · Attach to process… · Launch & attach… (same picker) · Open recent (last 8) · Settings · Keyboard shortcuts · Close target · Exit (Ctrl+Q) |
| Edit | Go to address… (Ctrl+G) · Rename… (F2) · Comment… (Ctrl+/) · Set return type… · Invert branch logic, Patch → NOP… (toast only, need a live target) · Command palette |
| View | Command palette · Zoom in / out / reset · Themes & appearance… |
| Analyze | Decompile (F5, switches workspace) · Apply FLIRT signatures (**demo**, see §11) · Find xrefs (toast) · Identify algorithms (**engine**, `const identify`) · Re-run analysis (toast only here; the palette action of the same name is real) |
| Debug | Attach to process… · Memory scan… · Set watchpoint · Find what writes… (these two only switch workspace) · Continue · Step over · Step into · Toggle breakpoint (toast only) |
| Window | Add widget… · Undo / Redo layout · Reset layout · Minimize · Maximize · Close window |
| Help | Getting started · Glossary · Documentation · About (toast only) · Keyboard shortcuts (opens settings) |

## 4. Command palette

Opened by Ctrl+Shift+P, Ctrl+P or F1. Substring filter, ↑/↓, Enter, Esc.

- 15 fixed commands, most switch workspace or show a toast.
- Every setting from §10 with its current value; Enter toggles or cycles it (**engine**
  where the setting touches the engine).
- Actions: Clear analysis cache, Show cache size, Re-run analysis (**engine**).
- The footer's "77 capabilities" is fixed text.

## 5. Dock (`ui/dock.js`)

- **Model**: one binary split tree per workspace. A leaf holds tabs; each tab is a
  widget instance with a stable id. Instance DOM is created once and moved between
  areas, so a widget keeps its state through splits, tabbing and resizing.
- **Drag an area header, a tab or a palette entry onto an area**: near an edge it splits
  that area; over the centre it replaces the content; with Ctrl held it is added as a
  tab, and the drop side sets where the tab strip sits. Dropping onto itself cancels,
  except splitting one tab out of a multi-tab area.
- **Resize** by dragging the line between areas (8 % to 92 %).
- **Tab strips** on any side; left and right strips show icons only. Per area: icons or
  names. Tab context menu: activate, close, strip side, label style.
- **Area marker** (top-left icon): change which widget the area shows, close the area.
- **Undo / redo**: 80 steps per workspace, Ctrl+Shift+Z / Ctrl+Shift+Y.
- **Persistence**: `localStorage` key `n0xis.dock.v2`; on load, duplicated ids from older
  saves are repaired.
- **Empty workspace**: placeholder with "Add widget" and "Reset to default".
- **Add-widget palette**: a click adds the widget by splitting the largest area; a drag
  places it.

Default layouts:

| Workspace | Layout |
| --- | --- |
| Decompile | Functions (left) · tabs Decompiler + Linear over Disassembly · Copilot over tabs Details + Variables + Xrefs + Bookmarks · bottom strip: tabs Output + Console |
| Static | Functions · tabs Decompiler + Linear over Disassembly · tabs Variables + Triage + Types over tabs Strings + Xrefs + Bookmarks |
| Graph | Control-flow graph alone |
| Dynamic | tabs Registers + Stack over Watchpoints · Memory scanner over Live memory · Watchlist over Copilot |

## 6. Widget catalog (24 widgets)

| Key | Title | Source | Notes |
| --- | --- | --- | --- |
| `functions` | Functions | engine | `function discover --pdata`, pages of 20 000 up to 400 000; falls back to the prologue scan when `--pdata` returns nothing. Virtualized above 300 rows (27 px each). Filter works on the data, not on rendered rows. Footer: total, % named, loading progress. Sub-tabs Imports / Strings / Types are cosmetic. |
| `decompiler` | Decompiler | engine | `decomp pseudo --style structured\|ssa\|goto` (style pills). Line numbers; syntax colouring done in the GUI; `CFG ↗` switches to the Graph workspace. |
| `disassembly` | Disassembly | engine | `disasm --count 40`: address, bytes, mnemonic, operands, `→ name` for a branch or call into a named function, the user's comment as `; text`. |
| `codeview` | Code | engine / demo | One pane switching Pseudo-C (engine), Disassembly (sample rows until the next selection repaints them) and Graph. |
| `linear` | Linear · listing | engine | Whole program as one listing. Sliding window of whole functions (at most 5 000 rows) that loads up and down while scrolling. Each function stops at its exact end (`.pdata`) or at the next function. Asm and Pseudo modes. Minimap over the whole function table (brighter = loaded, accent = named); click or drag scrolls, and fast drags collapse into one jump. Clicking a function header selects it; click / shift-click selects a line range, which becomes the AI scope. |
| `hex` | Hex | demo | |
| `graph` | Control-flow graph | engine | See §7. |
| `copilot` | Copilot | demo | Sample conversation. The scope chip is real: the selected function, or the line range selected in Linear. |
| `details` | Details · Provenance | demo | |
| `output` | Output | demo | Sample lines, plus echoes of some actions (see §11). |
| `registers`, `watchpoints`, `livemem`, `watchlist`, `stack`, `strings` | live-target and strings panels | demo | |
| `scanner` | Memory scanner | engine | The scan itself is real: `scan value` / `scan filter`, exact match, types i8 to f64, at most 300 results shown. Needs an attached process. |
| `notes` | Notes | demo | Free text, not saved. |
| `triage` | Triage · overview | engine | `profile --exports`: machine, module base, image end, `.pdata` function count, exports, thunks, distinct export addresses, toolchain hints, sections table, first 40 exports, advisories. |
| `console` | Console · n0xis | engine | Any engine command. Adds `--file <target>` when the command names no source (except guide, doctor, version). History with ↑/↓. Pretty-printed JSON, clipped at 16 000 characters. |
| `xrefs` | Cross-references | engine | `xref to` and `xref from`. Waits until the background analysis has built the reverse index, so a first query cannot block the session. |
| `variables` | Variables | GUI-derived | Parsed in the GUI from the decompiler's signature and text (see §11). |
| `bookmarks` | Bookmarks | engine | `annotate list`: the user's names, comments, types, variable renames and bookmarks; bookmarks first; click jumps. |
| `types` | Types | engine | `type list`, `type struct --name [--field off:name[:ctype]]…`, `type enum --name`, `type rm --name`. Adding a field re-sends the whole struct. |

## 7. Control-flow graph

- **Data**: `ir build --addr`: blocks with id, start, instructions and successors. A block
  shows up to 6 instructions and "+N more". Entry = first block; exit = no successors.
- **Edge kinds**: true / false from the engine's conditional kinds, everything else
  unconditional. An edge to an address at or below its source is drawn as a loop
  back-edge (a GUI guess, see §11).
- **Layout**: layered. Rank by longest path from the entry, order each row by the
  barycentre of its neighbours, push apart on overlap; extra headroom above a row that
  many edges converge on. Graphs over 160 blocks are not drawn; a message says so.
- **Edge routing**: orthogonal. Horizontal runs only in the bands between rows, vertical
  runs in gutters or straight down the target's column when it is clear; parallel runs
  get separate tracks so they never merge. Optional rounded corners (beta, saved as
  `n0x.graph.edges`).
- **Colours**: true green, false red, unconditional blue, back-edge amber and dashed;
  legend popover.
- **Interaction**: pan by dragging; wheel zoom around the cursor (0.3 to 2.4); zoom
  buttons; fit, which also re-enables auto-fit; auto-fit on resize until the user pans or
  zooms; drag a block to move it, edges follow; block context menu.
- Several graph widgets can be open at once; all rebuild when the selection changes.

## 8. Annotations and navigation

- **Edits**, all through `annotate …` in the target's project directory: rename function
  (F2), comment (Ctrl+/), rename variable, set variable type, set return type (`@return`),
  toggle bookmark, remove every annotation at an address.
- **Undo / redo of edits**: Ctrl+Z / Ctrl+Y, 200 steps, each storing before and after.
  Variables are keyed by their synthesized name, so a second rename or an undo reaches
  the same slot.
- **After an edit**: the result cache is cleared, the name in the function list is
  updated, every open view and the bookmarks refresh.
- **Selection**: one selected address and name drive the decompiler, disassembly,
  xrefs, graph, linear listing, AI scope and status bar. Only widgets that are mounted
  are refreshed.
- **Go to** (Ctrl+G): an exact function name or address from the loaded list; otherwise
  any hex address.
- **Find in image** (Ctrl+F): `find --string|--bytes|--escaped <q> --limit 300 [--utf16]`;
  each result shows address and section; a click jumps.
- **History**: Alt+← / Alt+→ and the mouse back / forward buttons, 300 entries.
- **Text input** uses an in-app dialog, because `window.prompt` is unreliable in the webview.

## 9. Opening a target

- **A file**: native picker, launcher card, File ▸ Open, or `argv[1]`. Opening resets all
  state, opens the resident session, runs `profile`, streams the function list (or loads
  it from the disk cache), selects the first function, then starts the background
  analysis if that setting is on.
- **Background analysis** in the status bar: starting → discovering → scanning RTTI →
  indexing xrefs → (disassembling N / total) → `ready · N fns · M classes`.
- **A live process**: filterable process list (at most 400 rows shown). Attaching clears
  the static context, switches to the Dynamic workspace and checks access with
  `mem map --pid`.
- **Recent targets**: the last 8 (name, kind, time).

## 10. Settings, keys, appearance

- **Settings overlay** panes: Appearance (6 themes: Midnight, Deep, Light, Nebula, Warm,
  Forest; UI scale 90 / 100 / 110 / 125 %), Layout, Editor & code, Analysis, Dynamic &
  debug, AI Copilot, Keybindings, Advanced. Only Appearance, Keybindings and Advanced
  (engine binary; reset all preferences) are wired; the other panes are static controls.
- **Palette settings**, saved as `n0x.set.<id>` (see `docs/SETTINGS.md`): `cache.mode`,
  `cache.keepOnClose`, `analyze.autorun`, `analyze.warmCfg`.
- **Keybindings**: one table drives both the settings editor and the dispatcher. Rebind
  by recording a chord (Esc cancels), unbind, revert one, reset all. Letters and digits
  match the physical key, so non-Latin keyboard layouts work. Scopes: always, a target
  open, a live target. Ignored while typing in a field.

  | Group | Bindings |
  | --- | --- |
  | General | Ctrl+Shift+P / Ctrl+P / F1 palette · Ctrl+O open · Ctrl+G go to · Ctrl+F find · Alt+← / Alt+→ history · Ctrl+, settings · Ctrl+= / Ctrl+- / Ctrl+0 zoom · Ctrl+Q close |
  | Analysis | F5 decompile (continue when live) · F2 rename · Ctrl+/ comment · Ctrl+Z / Ctrl+Y undo / redo edit · Shift+F12 references (toast only) |
  | Debug | F10 step over · F11 step into · F9 breakpoint (all toast only) |
  | Layout | Ctrl+Shift+Z / Ctrl+Shift+Y undo / redo layout |

- **Zoom** 70 % to 160 %, saved with the theme in `n0xis.prefs.v1`.
- **Theme tokens** (`styles.css` `:root`): backgrounds `--bg0..3`, `--hov`; borders `--bd`,
  `--bd2`; text `--tx0..2`; accent `--acc`, `--accd`; states `--live`, `--dgr`, `--ok`,
  `--vio`; syntax `--k`, `--ty`, `--fn`, `--st`, `--nu`, `--cm`, `--pu`. Themes override
  subsets of these.

## 11. Defects and risks found while writing this

Found by reading the code, not by running it. Most severe first.

1. **Sample content looks real in the native app.** With a real binary open, Copilot,
   Details · Provenance, Output, Registers, Watchpoints, Live memory, Hex, Watchlist,
   Strings and Stack still show built-in sample content. The status bar starts with a
   sample address and name and a fixed `FLIRT · WARP` label; the target chip starts with
   a sample file name. A user cannot tell a sample from a result.
2. **Messages with no engine call behind them.** Analyze ▸ Apply FLIRT signatures (and its
   palette entry) prints a fixed "named 118 functions"; the function menu's "Apply
   signature" prints "matched 1". Clicking the scanner's New scan / Next scan also emits
   a fixed "ok · 3 candidates" (a delegated click handler for every `.b` button) next to
   the real result.
3. **Untrusted text reaches `innerHTML` unescaped** in some toasts (the profiled source
   name, engine error messages), while `csp` is `null` and the Tauri global is exposed.
   A crafted file name could inject markup into the webview. Escape everything, set a CSP.
4. **The GUI rewrites numbers.** The decompiler colouring turns hex constants that start
   with `ffff` into negative numbers (`0xfffffff0` becomes `-0x10`). In a 64-bit expression
   that changes the meaning: a zero-extended mask turns into a sign-extended one.
   Number formatting belongs to the engine.
5. **Facts re-derived in the GUI.** Variables are parsed with regular expressions from
   the pseudocode text; loop back-edges are guessed from address order. Both should come
   from the engine's structured output.
6. **Open recent does not reopen anything**: only the name is stored, so choosing an
   entry just switches UI state.
7. **One request at a time.** The session mutex serializes every call, so one slow query
   stalls every view. Only xrefs are protected, by waiting for the index.
8. **Field names are guessed.** The UI accepts several names for the same field
   (`addr | address | va`, `name | symbol | label`, `matches | hits | identified |
   constants`), so the engine contract it relies on is not pinned.
9. **Smaller:** notes are not saved; `?widgets=1` names two widgets that do not exist
   (`decomp`, `regs`); the redo shortcut is documented as Ctrl+Shift+X but bound to
   Ctrl+Shift+Y; the `+` workspace tab, help links and several menu items are toast-only;
   the palette footer's capability count is fixed text.

## 12. Parity checklist for a rewrite

Engine and shell

- [x] Engine binary resolution (`$N0XIS_BIN`, user install, `PATH`) and the version banner (native, 2026-10-05)
- [x] Resident engine session per target, project directory per target (native, 2026-10-05). The
  one-shot fallback is not ported on purpose: an engine whose `serve` has no banner predates the
  other requests the GUI makes too, so the GUI names the engine it found and why it cannot use it.
- [x] Background analysis with live phases; xref queries wait for the index (native, 2026-10-05)
- [x] Function list streaming, disk cache (native + engine, 2026-10-05). The cache is the engine's,
  keyed by the code bytes rather than path and mtime, and holds no names, so a rename is never
  read stale: on a 159 MB library the list takes 10.9 s the first time and 0.5 s after.
- [x] Result cache with invalidation on edits; stale responses dropped (native, 2026-10-05). The
  engine caches decompiles, IR and the reference index on disk; an edit makes every view that
  shows the fact ask again; a request a newer one replaced is answered `Superseded` and dropped.
- [x] Open from picker, launcher, menu and command line; full state reset on open (native, 2026-10-05; recent targets reopen the file, §11.6)

Layout

Layout. Done in the native front end (2026-10-05) on GPUI Kit's dock:

- [x] Tiling dock: split, tab, cancel-on-self, resize, on GPUI Kit's dock.
- [x] Tab strips on any side of a group (top, bottom, left, right), icons or names per group,
  saved with the layout and undone with it; a tab's right-click menu (show, close, show
  instead, the strip's side, its labels); a close button on each tab and on the group's
  header (native, 2026-10-06). The dock draws through a renderer that an application may
  replace (`app/src/skin.rs`), so this needed no change to the library. An earlier version of
  this line said the dock "offers no other side" and left the strips out; that was wrong.
- [x] The Tauri dock's drops (native, 2026-10-06, `skin::DropMode`): near an edge a panel
  splits the group; over the centre it takes the group's place (the panels there leave the
  dock, layout undo brings them back); with Ctrl held it joins the group as a tab and the
  side it was dropped at is where the group's tabs go; dropped on its own group where nothing
  would change, it cancels. The target says which while the panel is over it, and changes
  when Ctrl goes down or up. Panels dragged from the widget palette get the same four.
- [x] Change-widget menu (a group's ⋯ menu ▸ Show instead), close, undo / redo per workspace,
  saved layouts (`ui-layout.json`, one layout per workspace; a file from before workspaces
  becomes the Decompile workspace's layout)
- [x] Four default workspaces; add-widget palette (click and drag)

Views (engine-backed first). Done in the native front end (`native/`, 2026-10-05); what each
one shows and how it was checked is in `native/README.md` and the commit messages.

- [x] Functions (virtualized, filter, footer) · Decompiler (3 styles) · Disassembly
- [x] Linear listing (window, exact function ends, Asm / Pseudo, minimap, range selection)
- [x] Control-flow graph (layout, orthogonal routing, colours, pan / zoom / fit, block drag)
- [x] Triage · Console · Xrefs · Bookmarks · Types · Memory scanner · Find in image
- [x] Variables (the engine's `variables`, §11.5) · Hex (`mem span`, gaps shown as gaps) ·
  Strings (`strings`, filtered and paged by the engine)

Not ported, on purpose: the catalog widgets that had no engine behind them in this build
(Code, Copilot, Details, Output, Registers, Watchpoints, Live memory, Watchlist, Stack,
Notes). Each comes back only with an engine result to show. Variables, Hex and Strings came
back that way (2026-10-05): Variables from the decompiler's own list, Hex from `mem span` and
Strings from `strings`, both of which the engine gained for them.

Editing and navigation. Done in the native front end (2026-10-05). Variables come from the
engine's own list (`decomp pseudo` → `variables`), not from the text (§11.5).

- [x] Rename, comment, variable rename and type, return type, bookmark, clear, all with undo / redo
- [x] Go to, history (keys and mouse buttons), one selection driving every view

Interactions. Added 2026-10-06, after the user found the right-click menus and the side tab
strips missing. The checklist above lists views and engine features, not gestures, so
"every item done" said nothing about either. This list was built by enumerating every
gesture the Tauri front end handles, then looking for each in the native code:
`grep -c addEventListener ui/*.js` (89: `dock.js` 15, `app.js` 74), the `menuFor` table
(64 items in eleven right-click menus, ten kinds of row and a default, `app.js:805-926`), the menu bar (45), the key table (24),
the palette (15 commands, 4 settings, 3 actions) and the dock's two menus (9). 25 of the 109
menu items only print a message; those are not ported.

- [x] Right-click menus with real actions only (native, 2026-10-06, `app/src/context.rs`):
  function rows, the decompiler (a variable under the caret by name), variables, instructions,
  linear lines and function headers, graph blocks and the graph around them, cross-references,
  bookmarks, strings, find results, hex rows, triage exports, types, scanner matches, console
  entries, and the workspace bar (the Tauri build's menu for empty space). A menu acts on the
  address it was opened on, whatever is selected. Not ported, being messages only there:
  breakpoints, watchpoints, patching, freezing, "Apply signature", the find-xrefs entries
  (the native Cross-references panel is real and every menu offers it).
- [x] Command palette, Ctrl+Shift+P, Ctrl+P or F1: every command of the key table, the panels,
  the workspaces, the themes, and the yes/no settings with their values; clearing the
  target's caches.
- [x] Analyze ▸ Identify Constants (`const identify`), from the menu, the palette or a
  function's right-click menu: the answer is shown in full (algorithm, role, note, formula),
  where the Tauri build showed a count.
- [x] File ▸ Close Target; File ▸ Scan a Running Process… opens the scanner in the Dynamic
  workspace. Attaching to a process as the session's target is not ported: nothing native
  reads a process except the scanner, which picks its own.
- [x] Ctrl+F opens Find with the caret in its query; the decompiler's Graph button (the Tauri
  build's "CFG ↗"); Help ▸ Keyboard Shortcuts opens Settings at Keys.
- [x] The target chip: the open target's path in the title bar opens a menu (open another,
  close, triage, copy the path).
- [x] Graph edges with square or rounded corners, kept with the appearance; interface scale
  presets (90, 100, 110 and 125 %) beside the font size.
- [x] A workspace with no panel says so and offers one, or the default layout. The dock keeps
  the last panel of a workspace, so this shows only for a layout saved empty.
- [x] Found while checking these, and fixed: closing the panel that held the caret left every
  key going to no element, so the shortcuts stopped until something was clicked.
- Left out on purpose: the graph legend's toggle (the legend is always in the header).

Open gaps. A second audit on 2026-10-06 (reading behaviour, not only handlers) refuted
"everything real is ported". Not done yet, most visible first:

- [ ] `→ name` after a branch or call into a named function, in Disassembly and Linear.
- [ ] The user's comments (`; text`) in the Linear listing.
- [ ] A Types edit redraws the decompiler (new field names show at once).
- [ ] Refresh in Cross-references, Bookmarks and Types, for edits made outside the GUI.
- [ ] Several instances of one panel (two graphs, Linear in Asm beside Linear in Pseudo).
- [ ] Widget palette click splits the largest group, as the Tauri build did.
- [ ] Console without a target, run beside the session so it never blocks the views.
- [ ] Process names of Wine programs (the engine lists them as `wine-preloader`), process icons.
- [ ] Settings: the engine in use (path, version, command count); reset all preferences.
- [ ] Status bar: image format and machine, zoom; zoom up to 160 %.
- [ ] Graph: entry and exit blocks marked; zoom shown.
- [ ] Linear Pseudo follows the decompiler's style; F5 shows the decompiler; Window menu
  minimize / maximize / close; Go to matches names in any case and takes an address only
  with `0x`; Back / Forward say when there is nowhere to go; recent targets show when.
- [ ] A loading indicator that shows only after 180 ms and stays 320 ms, instead of each
  view flashing its own message.
- [ ] Discard-on-close says when the engine cannot clear the cache (it fails silently now).
- [ ] Shortcuts do not fire while typing in a field.
- [ ] Resizing a group or the window freezes the window on the user's machine (NVIDIA,
  Wayland). Not reproduced here; the app's own code costs the same optimised or not.

Settings

- [x] Cache location and keep-on-close, auto-analyze, warm-up; cache size / clear (native,
  2026-10-05: File ▸ Settings…; which folders are caches is the engine's `project cache`)
- [x] Themes and tokens; zoom (native)
- [x] Editable keybindings with physical-key matching (native, 2026-10-05: Settings ▸ Keys; one
  table of commands drives the bindings and the editor; with a Ukrainian layout, Ctrl and the key
  that prints `п` runs Go to, Ctrl+G, checked headless)

Honesty rules the rewrite must hold (from §11). Held by the native front end (checked in its
code 2026-10-05):

- [x] No sample content in the native app; an empty view says it is empty. The demo-only widgets
  were not ported, and each view has an empty and a failed state of its own.
- [x] No message without an engine result behind it. The menus list only working actions; a
  refusal says the engine's reason (a type on a value is refused because the engine's variable
  list says it is neither a parameter nor a local).
- [x] No fact re-derived in the GUI that the engine already knows. Variables come from the
  engine's `variables`; the graph draws an edge that points up around the side by geometry and
  labels edges only with the engine's kinds, never "loop"; numbers are shown as the engine
  wrote them.
- [x] One typed client over the engine's schemas (`meta.schema`), not field-name guessing
  (`native/client`: each request checks the schema of its answer).
- [x] All untrusted text rendered as text. GPUI draws strings, with no markup and no web view, so
  there is no content policy to set.

## Appendix: engine contract used by the current UI

| Command | Fields read |
| --- | --- |
| `--version` | last token of stdout |
| `guide --brief` | `data.command_count` |
| `serve --file <t>` | first line: ready envelope; then one command per line, one envelope per line (arguments joined by spaces, an argument with whitespace wrapped in `"`, no escaping) |
| `analyze --file <t> [--no-cfg \| --limit N]` | stderr `[n0x] {phase,done,total}`; stdout `data.functions`, `data.rtti_classes` |
| `profile --file <t> --exports` | `data.image.{machine, module_base, image_end, pdata_present, pdata_functions, thunk_count, export_count, export_distinct_addresses, sections[], exports[], engine_hints[]}`, `data.advisories[]`, `meta.source` |
| `function discover --file <t> [--pdata] --limit N --offset M` | `data.functions[]{addr, name, end}`, `meta.total` |
| `decomp pseudo --file <t> --addr <a> --style <s>` | `data.pseudo[]`, `data.signature` |
| `disasm --file <t> --addr <a> --count N` | `data.insns[]{va, bytes, mnemonic, text, target, comment}` |
| `ir build --file <t> --addr <a>` | `data.blocks[]{id, start, insns[], successors[]{to, kind}}` |
| `xref to\|from --file <t> --addr <a>` | `data.refs[]{from, to, kind, text, sym}` |
| `annotate show\|list\|name\|comment\|var\|vartype\|bookmark\|rm` | `show`: `name, comment, type_note, var_names, var_types, bookmark`; `list`: `data.records[]` |
| `type list\|struct\|enum\|rm` | `data.structs[]{name, fields[]{offset, name, ctype}}`, `data.enums[]{name, members[]{name, value}}` |
| `find --file <t> --string\|--bytes\|--escaped <q> --limit 300 [--utf16]` | `data.{count, truncated, bytes_scanned, matches[]{va, section}}` |
| `const identify --file <t> --addr <a>` | one of four list names (see §11.8) |
| `process ps` · `mem map --pid <p>` | `data.processes[]{pid, name}` · `ok` as an access check |
| `scan value\|filter --pid <p> … --save-as gui` | `data.{matches[]{addr, value}, total_matches, shown}` |
