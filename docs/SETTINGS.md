# N0xis GUI — settings reference

Every setting is a **searchable command**. Press **F1** (or the palette button) to
open the command palette and type part of a setting's name — e.g. `cache`,
`analyze` — then press Enter to toggle or cycle it. This is the VS Code model:
there is no buried preferences tree; anything you can configure is findable by
typing. Settings persist in the browser store (`localStorage`, key
`n0x.set.<id>`), per machine.

This file is the catalog an AI (or you) can read to know what exists and how to
change it.

## Cache & storage

The engine keeps two very different kinds of data under a target's `.n0x/`:

- **Derived cache** — the IR cache (`ir-cache/`) and the reverse-xref index
  (`xref-index/`). Recomputable, content-addressed, safe to delete. This is what
  the settings below control.
- **Your work** — names, comments, patches, tables (`annotations.json`,
  `patches/`, `tables/`). Never deleted by any cache action; this is the real
  "project".

| Setting (id) | Values | Default | What it does |
| --- | --- | --- | --- |
| **Cache location** (`cache.mode`) | Central · Beside the binary · Custom folder… | Central | Where the derived cache is stored. *Central* = `~/.local/share/pro.n0xis.gui/projects/<hash>/.n0x/` (nothing scattered next to your files). *Beside the binary* = a `.n0x/` next to the target. *Custom* = a folder you pick. |
| **Keep cache on close** (`cache.keepOnClose`) | On · Off | On | Off deletes the derived cache when the window closes (names/patches are always kept). Use it if you don't want caches to persist. |
| **Auto-analyze on open** (`analyze.autorun`) | On · Off | On | Run discover + RTTI + xref-index in the background when a binary opens. |
| **Warm decompilation cache** (`analyze.warmCfg`) | On · Off | Off | Also pre-decode every function during analysis — more disk and time, but the first decompile view of any function is then instant. Off decompiles lazily (cached on first view). |

### Cache actions (palette)

- **Clear analysis cache** — delete the IR + xref-index cache for the open
  binary (keeps names).
- **Show cache size** — how much disk the derived cache uses for this binary.
- **Re-run analysis** — run the analysis pass again (resumes from cache).

## How analysis maps to the engine

The GUI's analysis is the engine's `n0xis analyze` command run as a background
process in the target's project directory. Its phases stream to the status bar:

`discovering → scanning RTTI → indexing xrefs → (disassembling N/total) → ready`

Because the caches are content-addressed and on disk, closing and reopening a
binary resumes instantly (the xref index reloads instead of rebuilding), unless
**Keep cache on close** is Off.
