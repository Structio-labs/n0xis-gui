# Widgets, their data, and how the AI touches it

A design note (not built yet) capturing the intended relationship between widgets,
their underlying data, and the AI. **"Follow AI" is disabled for now** — the concept
(the whole view chasing what the model looks at) isn't earning its keep yet.

## Widgets are views over data — not the data

A widget (Watchlist, Watchpoints, Memory scanner results, Strings…) is a **view over a
data set that lives in the session**, not a private bag of DOM. The watchlist is a list of
watched addresses; the widget just renders it.

That data set has **two writers, one store**:

- the **human**, through the UI;
- the **AI**, through the engine's CLI/MCP surface.

When the AI adds an address to the watchlist, it appends to *the same list* the user edits —
there is one source of truth, and both edit it. This is the Data seam: the widget survives
its own rewrite as long as the data contract holds.

## The AI reads through MCP — paginated, never "ingest everything"

To let the AI *see* a widget's data, we add **CLI/MCP commands that query it**, e.g.:

```
n0x watchlist list --limit 50 --offset 0      # a page of entries
n0x watchlist count                            # how many there are
n0x watchlist get <id>                         # one entry
```

Crucial constraint the user flagged: **the AI must never try to pull the whole set into its
context.** If the user has added thousands of addresses, dumping them all would blow the
model's context and break it. So every list-returning command is **paginated and bounded**
(`--limit`, `--offset`, a hard server cap, and a `count`), and the AI asks for the slice it
needs. The store can be arbitrarily large; the AI only ever holds a window.

This is the same envelope discipline the CLI already has (`{ok,data,meta}` with budgeting) —
the frontend's widget data is just another queryable, paginated surface.

## Two MCP surfaces (unchanged from the frontend plan)

- **engine tools** — analyze, scan, watch, decompile … (produce/change data).
- **view tools** — read widget state, add/remove entries, focus a pane … (paginated).

The human and the AI go through the *same* view/engine tools, so anything the AI can do to a
widget, a person can too, and vice-versa (Process seam, no privileged core).

## Deferred

- **Follow AI** (spotlight the panes the model is examining) — disabled until it has a clear
  job; the plumbing (shared selection bus) can come back when it does.
- Persisting the AI's edits is just persisting the data set (it is shared state), so it
  falls out of the store, not out of a special "AI history".
