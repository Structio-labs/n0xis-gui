// dock.js — a Blender/AreaKit-style tiling dock.
//
// Model: a binary tree of nodes.
//   split = { t:'split', dir:'row'|'col', ratio:[a,b], kids:[node,node] }
//   leaf  = { t:'leaf', id, tabs:[key,...], active, tabSide:'top'|'bottom'|'left'|'right', tabStyle:'auto'|'icon'|'name' }
// One tree per workspace, persisted to localStorage, every mutation is undoable.
//
// Gestures (from docs/panes.md, adapted):
//   • drag an area header (or a palette item) onto another area:
//       – near an EDGE   → the target splits, the dragged pane lands on that side
//       – over the CENTRE → the target is taken over (its content is replaced)
//   • hold Ctrl while dragging → the pane is inserted as a TAB of the target
//       (drop near a side to place the tab-strip there; a vertical strip shows icons)
//   • drag the line between two areas → resize (proportions are preserved)
//   • click the type marker (top-left) → change what the area shows
//   • Ctrl+Shift+Z / Ctrl+Shift+X → undo / redo   (all changes auto-saved)

let uid = 1;
const nid = () => 'a' + (uid++);
const clone = o => JSON.parse(JSON.stringify(o));

export function initDock(deps) {
  const { container, widgets, ICON, svg, toast, showMenu } = deps;
  const SKEY = 'n0xis.dock.v1';

  // ---- per-workspace state ----
  let ws = 'decompile';
  const trees = {};                 // ws -> root node | null
  const hist = {};                  // ws -> [snapshots]
  const fut = {};                   // ws -> [snapshots]
  const ensure = w => { if (!(w in trees)) { trees[w] = null; hist[w] = []; fut[w] = []; } };

  // ---- persistence ----
  function save() {
    try { localStorage.setItem(SKEY, JSON.stringify(trees)); } catch {}
    onChange();
  }
  function load() {
    try {
      const raw = localStorage.getItem(SKEY);
      if (raw) { const o = JSON.parse(raw); for (const k in o) { trees[k] = o[k]; hist[k] = []; fut[k] = []; } }
    } catch {}
  }
  let onChange = () => {};

  // ---- undo/redo ----
  function snapshot() { ensure(ws); hist[ws].push(clone(trees[ws])); if (hist[ws].length > 60) hist[ws].shift(); fut[ws] = []; }
  function undo() { ensure(ws); if (!hist[ws].length) return toast?.('Nothing to undo'); fut[ws].push(clone(trees[ws])); trees[ws] = hist[ws].pop(); render(); save(); }
  function redo() { ensure(ws); if (!fut[ws].length) return toast?.('Nothing to redo'); hist[ws].push(clone(trees[ws])); trees[ws] = fut[ws].pop(); render(); save(); }

  // ---- node helpers ----
  const leaf = key => ({ t: 'leaf', id: nid(), tabs: [key], active: 0, tabSide: 'top', tabStyle: 'auto' });
  function findLeaf(node, id, parent = null, side = -1) {
    if (!node) return null;
    if (node.t === 'leaf') return node.id === id ? { node, parent, side } : null;
    return findLeaf(node.kids[0], id, node, 0) || findLeaf(node.kids[1], id, node, 1);
  }
  function replaceNode(oldLeafId, newNode) {
    const f = findLeaf(trees[ws], oldLeafId);
    if (!f) return;
    if (!f.parent) trees[ws] = newNode;
    else f.parent.kids[f.side] = newNode;
  }
  function removeLeaf(id) {
    const f = findLeaf(trees[ws], id);
    if (!f) return;
    if (!f.parent) { trees[ws] = null; return; }
    const sibling = f.parent.kids[1 - f.side];
    const gp = findParentOf(trees[ws], f.parent);
    if (!gp) trees[ws] = sibling;
    else gp.parent.kids[gp.side] = sibling;
  }
  function findParentOf(node, target, parent = null, side = -1) {
    if (!node || node.t === 'leaf') return null;
    if (node === target) return { parent, side };
    return findParentOf(node.kids[0], target, node, 0) || findParentOf(node.kids[1], target, node, 1);
  }

  // ---- operations (all snapshot first) ----
  function addToEmpty(key) { snapshot(); trees[ws] = leaf(key); render(); save(); }
  function splitAt(targetId, side, key, movingFrom) {
    snapshot();
    const nl = leaf(key);
    const t = findLeaf(trees[ws], targetId).node;
    const dir = (side === 'left' || side === 'right') ? 'row' : 'col';
    const kids = (side === 'left' || side === 'top') ? [nl, t] : [t, nl];
    replaceNode(targetId, { t: 'split', dir, ratio: [0.5, 0.5], kids });
    if (movingFrom) detach(movingFrom);
    render(); save();
  }
  function takeover(targetId, key, movingFrom) {
    snapshot();
    const t = findLeaf(trees[ws], targetId).node;
    t.tabs = [key]; t.active = 0;
    if (movingFrom) detach(movingFrom);
    render(); save();
  }
  function addTab(targetId, side, key, movingFrom) {
    snapshot();
    const t = findLeaf(trees[ws], targetId).node;
    t.tabs.push(key); t.active = t.tabs.length - 1;
    if (side === 'left' || side === 'right' || side === 'top' || side === 'bottom') t.tabSide = side;
    if (movingFrom) detach(movingFrom);
    render(); save();
  }
  // detach a pane we are moving from its old leaf (remove that tab, collapse leaf if empty)
  function detach(mv) {
    const f = findLeaf(trees[ws], mv.leafId);
    if (!f) return;
    const n = f.node;
    if (n.tabs.length <= 1) removeLeaf(n.id);
    else { n.tabs.splice(mv.tabIndex, 1); n.active = Math.max(0, Math.min(n.active, n.tabs.length - 1)); }
  }
  function closePane(leafId, tabIndex) {
    snapshot();
    const f = findLeaf(trees[ws], leafId);
    if (!f) return;
    const n = f.node;
    if (n.tabs.length <= 1) removeLeaf(leafId);
    else { n.tabs.splice(tabIndex, 1); n.active = Math.max(0, Math.min(n.active, n.tabs.length - 1)); }
    render(); save();
  }

  // =========================================================================
  // rendering
  // =========================================================================
  function render() {
    ensure(ws);
    container.innerHTML = '';
    const root = trees[ws];
    container.style.pointerEvents = root ? 'auto' : 'none';
    container.classList.toggle('dk-has', !!root);
    if (!root) return;
    const wrap = document.createElement('div');
    wrap.className = 'dk-root';
    wrap.appendChild(renderNode(root));
    container.appendChild(wrap);
  }
  function renderNode(node) {
    if (node.t === 'split') {
      const d = document.createElement('div');
      d.className = 'dk-split ' + node.dir;
      const a = renderNode(node.kids[0]); a.style.flex = node.ratio[0];
      const g = document.createElement('div'); g.className = 'dk-gutter ' + node.dir;
      const b = renderNode(node.kids[1]); b.style.flex = node.ratio[1];
      gutterDrag(g, node, d);
      d.append(a, g, b);
      return d;
    }
    return renderLeaf(node);
  }
  function renderLeaf(node) {
    const area = document.createElement('div');
    area.className = 'dk-area'; area.dataset.leaf = node.id;
    const vertical = node.tabSide === 'left' || node.tabSide === 'right';
    const iconOnly = node.tabStyle === 'icon' || (node.tabStyle === 'auto' && vertical);
    const multi = node.tabs.length > 1;

    // tab strip (shown when >1 tab, or when a non-top side is set)
    let strip = null;
    if (multi || node.tabSide !== 'top') {
      strip = document.createElement('div');
      strip.className = 'dk-tabs ' + (vertical ? 'v' : 'h');
      node.tabs.forEach((key, i) => {
        const w = widgets[key] || { title: key, icon: 'decomp' };
        const tab = document.createElement('div');
        tab.className = 'dk-tab' + (i === node.active ? ' on' : '');
        tab.title = w.title;
        tab.innerHTML = `<span class="dk-ti">${svg(ICON[w.icon] || ICON.decomp, '')}</span>` + (iconOnly ? '' : `<span class="dk-tn">${w.title}</span>`) +
          `<span class="dk-tx" title="Close">${svg('M4 4l6 6M10 4l-6 6', '')}</span>`;
        tab.addEventListener('click', e => { if (e.target.closest('.dk-tx')) { closePane(node.id, i); return; } node.active = i; render(); save(); });
        tab.addEventListener('pointerdown', e => { if (e.target.closest('.dk-tx')) return; startDrag(e, { key, leafId: node.id, tabIndex: i }); });
        tab.addEventListener('contextmenu', e => { e.preventDefault(); e.stopPropagation(); tabMenu(e, node, i); });
        strip.appendChild(tab);
      });
    }

    // header: type marker + title + area close (only when single tab; multi uses strip)
    const activeKey = node.tabs[node.active];
    const w = widgets[activeKey] || { title: activeKey, icon: 'decomp', body: () => '' };
    const head = document.createElement('div');
    head.className = 'dk-head';
    head.innerHTML =
      `<span class="dk-marker" title="Change what this shows">${svg(ICON[w.icon] || ICON.decomp, '')}</span>` +
      `<span class="dk-title">${w.title}</span><span class="dk-grow"></span>` +
      `<span class="dk-close" title="Close area">${svg('M4 4l7 7M11 4l-7 7', '')}</span>`;
    head.querySelector('.dk-marker').addEventListener('click', e => { e.stopPropagation(); paneMenu(e, node); });
    head.querySelector('.dk-close').addEventListener('click', e => { e.stopPropagation(); closePane(node.id, node.active); });
    head.addEventListener('pointerdown', e => { if (e.target.closest('.dk-close,.dk-marker')) return; startDrag(e, { key: activeKey, leafId: node.id, tabIndex: node.active }); });
    head.addEventListener('contextmenu', e => { e.preventDefault(); e.stopPropagation(); paneMenu(e, node); });

    const body = document.createElement('div');
    body.className = 'dk-body';
    try { body.innerHTML = w.body ? w.body() : ''; } catch { body.innerHTML = ''; }

    // assemble with the strip on the requested side
    area.classList.add('side-' + node.tabSide);
    if (strip && node.tabSide === 'top') area.append(strip, head, body);
    else if (strip && node.tabSide === 'bottom') area.append(head, body, strip);
    else if (strip && node.tabSide === 'left') { const row = document.createElement('div'); row.className = 'dk-vrow'; row.append(strip, wrapCol(head, body)); area.append(row); }
    else if (strip && node.tabSide === 'right') { const row = document.createElement('div'); row.className = 'dk-vrow'; row.append(wrapCol(head, body), strip); area.append(row); }
    else area.append(head, body);
    return area;
  }
  function wrapCol(a, b) { const c = document.createElement('div'); c.className = 'dk-col'; c.append(a, b); return c; }

  // =========================================================================
  // gutter resize (proportions preserved)
  // =========================================================================
  function gutterDrag(g, node, splitEl) {
    g.addEventListener('pointerdown', e => {
      e.preventDefault(); g.setPointerCapture(e.pointerId); g.classList.add('drag');
      const horiz = node.dir === 'row';
      const rect = splitEl.getBoundingClientRect();
      const total = horiz ? rect.width : rect.height;
      const move = ev => {
        const pos = horiz ? (ev.clientX - rect.left) : (ev.clientY - rect.top);
        let r = Math.max(0.08, Math.min(0.92, pos / total));
        node.ratio = [r, 1 - r];
        splitEl.children[0].style.flex = node.ratio[0];
        splitEl.children[2].style.flex = node.ratio[1];
      };
      const up = () => { g.releasePointerCapture(e.pointerId); g.classList.remove('drag'); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); save(); };
      window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
    });
  }

  // =========================================================================
  // drag & drop (split / takeover / tab)  — Ctrl toggles tab mode
  // =========================================================================
  let drag = null, ghost = null, ov = null, ctrl = false;
  function startDrag(e, payload) {
    e.preventDefault();
    drag = { payload, x: e.clientX, y: e.clientY, moved: false, hit: null };
    ctrl = e.ctrlKey || e.metaKey;
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('keyup', onKey, true);
  }
  // public: start a drag from a palette item
  function startPaletteDrag(e, key) { startDrag(e, { key }); }
  function onKey(e) { if (e.key === 'Control' || e.key === 'Meta') { ctrl = e.type === 'keydown'; if (drag && drag.hit) paint(drag.hit); } }
  function onMove(e) {
    if (!drag) return;
    if (!drag.moved && Math.hypot(e.clientX - drag.x, e.clientY - drag.y) < 5) return;
    if (!drag.moved) { drag.moved = true; mkGhost(); mkOverlay(); }
    ctrl = e.ctrlKey || e.metaKey;                 // authoritative modifier state
    ghost.style.left = e.clientX + 12 + 'px'; ghost.style.top = e.clientY + 10 + 'px';
    const area = document.elementFromPoint(e.clientX, e.clientY)?.closest('.dk-area');
    if (!area) { ov.classList.remove('on'); drag.hit = null; return; }
    const r = area.getBoundingClientRect();
    const fx = (e.clientX - r.left) / r.width, fy = (e.clientY - r.top) / r.height;
    let zone;
    if (fx > 0.32 && fx < 0.68 && fy > 0.32 && fy < 0.68) zone = 'center';
    else { const d = { left: fx, right: 1 - fx, top: fy, bottom: 1 - fy }; zone = Object.keys(d).sort((a, b) => d[a] - d[b])[0]; }
    drag.hit = { leafId: area.dataset.leaf, zone, r };
    paint(drag.hit);
  }
  function paint(hit) {
    if (!ov) return;
    if (!hit) { ov.classList.remove('on'); return; }
    const { r, zone } = hit;
    const tab = ctrl;
    ov.className = 'dk-ov on' + (tab ? ' tab' : '');
    let x = r.left, y = r.top, w = r.width, h = r.height;
    if (!tab) {
      if (zone === 'left') w = r.width / 2;
      else if (zone === 'right') { x = r.left + r.width / 2; w = r.width / 2; }
      else if (zone === 'top') h = r.height / 2;
      else if (zone === 'bottom') { y = r.top + r.height / 2; h = r.height / 2; }
    }
    Object.assign(ov.style, { left: x + 'px', top: y + 'px', width: w + 'px', height: h + 'px' });
    ov.dataset.mode = tab ? 'tab · ' + (zone === 'center' ? 'top' : zone) : (zone === 'center' ? 'replace' : 'split ' + zone);
  }
  function onUp() {
    window.removeEventListener('pointermove', onMove); window.removeEventListener('pointerup', onUp);
    window.removeEventListener('keydown', onKey, true); window.removeEventListener('keyup', onKey, true);
    const d = drag, wasTab = ctrl; drag = null; ctrl = false;
    if (ghost) { ghost.remove(); ghost = null; } if (ov) ov.classList.remove('on');
    if (!d || !d.moved) { if (d && d.payload && !d.payload.leafId) addWidget(d.payload.key); return; } // plain palette click
    if (!d.hit) return;
    const { payload } = d, { leafId, zone } = d.hit;
    const moving = payload.leafId ? { leafId: payload.leafId, tabIndex: payload.tabIndex } : null;
    // dropping a single-tab area onto its own centre is a no-op
    if (moving && moving.leafId === leafId) {
      const n = findLeaf(trees[ws], leafId)?.node;
      if (zone === 'center' && (!n || n.tabs.length <= 1) && !wasTab) return;
    }
    if (wasTab) addTab(leafId, zone === 'center' ? 'top' : zone, payload.key, moving);
    else if (zone === 'center') takeover(leafId, payload.key, moving);
    else splitAt(leafId, zone, payload.key, moving);
  }
  function mkGhost() {
    ghost = document.createElement('div'); ghost.className = 'dk-ghost';
    const w = widgets[drag.payload.key] || { title: drag.payload.key, icon: 'decomp' };
    ghost.innerHTML = `${svg(ICON[w.icon] || ICON.decomp, '')}<span>${w.title}</span>`;
    document.body.appendChild(ghost);
  }
  function mkOverlay() { if (!ov) { ov = document.createElement('div'); ov.className = 'dk-ov'; document.body.appendChild(ov); } }

  // =========================================================================
  // context menus (tab / tabline / pane)
  // =========================================================================
  function paneMenu(e, node) {
    const items = [{ label: 'Change what this shows', header: true }];
    for (const [k, w] of Object.entries(widgets))
      items.push({ label: w.title, icon: ICON[w.icon] || ICON.decomp, act: () => { snapshot(); node.tabs[node.active] = k; render(); save(); } });
    items.push({ sep: true }, { label: 'Close area', icon: ICON.trash, danger: true, act: () => closePane(node.id, node.active) });
    showMenu(e.clientX, e.clientY, items);
  }
  function tabMenu(e, node, i) {
    const w = widgets[node.tabs[i]] || {};
    const items = [
      { label: 'Tab · ' + (w.title || ''), header: true },
      { label: 'Activate', icon: ICON.decomp, act: () => { node.active = i; render(); save(); } },
      { label: 'Close tab', icon: ICON.trash, danger: true, act: () => closePane(node.id, i) },
      { sep: true },
      { label: 'Tab-strip', header: true },
      { label: 'Strip: top', icon: ICON.regs, act: () => setSide(node, 'top') },
      { label: 'Strip: bottom', icon: ICON.regs, act: () => setSide(node, 'bottom') },
      { label: 'Strip: left (icons)', icon: ICON.regs, act: () => setSide(node, 'left') },
      { label: 'Strip: right (icons)', icon: ICON.regs, act: () => setSide(node, 'right') },
      { sep: true },
      { label: node.tabStyle === 'name' ? 'Show: icons only' : 'Show: names', icon: ICON.strings, act: () => { snapshot(); node.tabStyle = node.tabStyle === 'name' ? 'icon' : 'name'; render(); save(); } },
    ];
    showMenu(e.clientX, e.clientY, items);
  }
  function setSide(node, s) { snapshot(); node.tabSide = s; render(); save(); }

  // =========================================================================
  // public API
  // =========================================================================
  function setWorkspace(w) { ws = w; ensure(w); render(); }
  function addWidget(key) {
    ensure(ws);
    if (!trees[ws]) { addToEmpty(key); return; }
    // add by splitting the largest area (or the root) so a click always lands somewhere
    let big = null, bigArea = 0;
    container.querySelectorAll('.dk-area').forEach(a => { const r = a.getBoundingClientRect(); if (r.width * r.height > bigArea) { bigArea = r.width * r.height; big = a.dataset.leaf; } });
    if (big) { const a = container.querySelector(`[data-leaf="${big}"]`).getBoundingClientRect(); splitAt(big, a.width >= a.height ? 'right' : 'bottom', key); }
    else addToEmpty(key);
  }
  function reset() { snapshot(); trees[ws] = null; render(); save(); toast?.('Layout cleared'); }
  function hasLayout() { ensure(ws); return !!trees[ws]; }

  load();
  return { setWorkspace, addWidget, undo, redo, reset, render, hasLayout, startPaletteDrag, onChangeHook: cb => { onChange = cb; } };
}
