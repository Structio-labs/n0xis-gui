# CFG Graph — capabilities, context menu, and roadmap

Notes for building the control-flow graph into a self-sufficient RE surface.
Reference points from **Binary Ninja** (validated below), and where we are / where we go.

## What actually belongs to the graph vs. the whole analysis

A common confusion (and a correct correction): **most right-click actions are NOT
graph-exclusive.** Rename, Change type, Add comment, Go to xrefs, Set value, Data-format —
these act on the *analysis database* and appear the same in the linear/disassembly and
decompiler views. The graph is just one visualization of that data.

**Genuinely graph-view-specific** (the visual/topology layer):

| Action | Note |
| --- | --- |
| Zoom to fit (`w`) / zoom to cursor | frame the whole function, or 100% at the cursor |
| Navigate edge (double-click an edge) | jump to the linked block |
| Toggle graph ⇄ linear (`Space`) | same data, other layout |
| Invert branch logic | swaps the true/false edges *and patches* the branch |
| Toggle layout / collapse branches | fold parts of the tree |
| Edge colouring | the topology cue itself (below) |

Everything else in a block's context menu is shared analysis, surfaced here for convenience.

## Edge colouring (Binary Ninja convention — adopted)

- **green** = conditional true
- **red** = conditional false
- **blue** = unconditional (single successor)
- **amber, dashed** = loop **back-edge** (a body block returning to its header)
- (todo) a **colour-blind mode** that swaps hues for shape/label cues

We render these from the edge's `type` (`t/f/u/loop`) with per-colour arrowheads, and a legend.

## Layout — the honest part

The graph must **build itself from the engine's blocks+edges**, not from hand-placed
coordinates. Our current layout is a **Sugiyama-lite**: rank by longest path from entry →
place each node at the barycentre of its neighbours → push apart on overlap → route edges
straight, bowing around only when a block sits in the corridor. This guarantees no overlaps
and puts a child under its parent (e.g. loop body under loop head).

**For real processes** this needs hardening — back-edge detection by DFS (not by a `type`
flag), virtual/dummy nodes on long edges for clean routing, iterative crossing reduction,
and performance for hundreds of blocks. The production move is to **bundle a proven layout
engine — ELK (`elkjs`) or dagre** — the same class of engine real graph viewers use, with our
Sugiyama-lite kept as a lightweight fallback. Nothing above the layout layer changes: the
widget, edge styling, drag-to-reposition and context menus stay.

## Context menu we expose on a block (BN-inspired)

Decompile block · Rename label (`N`) · Invert branch logic · Set breakpoint (`F9`) ·
Find xrefs to block (`X`) · Open in new tab · Comment (`;`) · Copy block address.
These are demo actions today; each maps to a future engine/MCP call.

## Backlog

- IL layers (Disasm / LLIL / MLIL / HLIL / Pseudo-C) toggled per graph, `i` to cycle.
- Double-click an edge to navigate; `w` zoom-to-fit; `Space` graph⇄linear.
- Real layout engine (ELK/dagre) fed by `n0x` CFG data.
- Highlight no-return blocks; entry/exit already coloured.
- Patching actions (Invert branch, NOP) wired to the journaled patch engine.
