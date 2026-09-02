# N0xis GUI — names & annotations

Functions, variables and addresses carry **names** from three sources, resolved
as one layer so they render everywhere the same way (decompiler, function list,
linear listing, cross-references):

1. **Your renames** (highest priority) — right-click ▸ *Rename…* or **F2** on a
   function/variable; right-click ▸ *Comment…* or **Ctrl+/** for a note. These are
   your truth: they win over everything and are kept, versioned, in the target's
   `.n0x/annotations.json` (every change appended, never overwritten). They survive
   closing the window.
2. **Recovered class names** — `analyze` reads the binary's MSVC RTTI and names
   each class's vtable (`Class::vftable`) and its virtual methods (`Class::vfN`).
   On a large app this is tens of thousands of names, appearing automatically the
   first time you open it. Persisted in `.n0x/rtti-symbols.json` (derived cache —
   safe to delete, rebuilt by re-analyzing).
3. **The binary's own symbols** — PE exports/imports, an imported IL2CPP index,
   FLIRT signatures. Anything still anonymous stays `sub_<address>`.

## How it stays fast and correct

- A **rename shows instantly** in every open view. The engine's IR cache is keyed
  on the current name set, so the decompiler recomputes the renamed function (and
  its callers) rather than serving the old name — no manual refresh, no stale cache.
- Renaming does **not** re-parse the (large) recovered-name file: the two name
  sources are cached independently, so editing stays snappy even on a target with
  50k+ classes.
- Names always resolve against **this** binary's `.n0x/` (the session runs in its
  project dir), so two different targets never share names.

## For an AI driving the GUI/engine

- Rename: `annotate name --addr <va> --value <name>` (omit `--value` to clear).
- Comment: `annotate comment --addr <va> --value <text>`. Type note: `annotate type …`.
- Read back: `annotate show --addr <va>` / `annotate list`.
- Recover class names for the whole image: `analyze --file <pe>` (writes
  `rtti-symbols.json`; `--no-cfg` skips the decompile warm-up).
- The decompiler is the authoritative named surface; `xref` rows carry a `sym`
  field naming the referenced target when it is known.

Run these in the target's project directory (the GUI's session already does), so
they read and write that target's `.n0x/`.
