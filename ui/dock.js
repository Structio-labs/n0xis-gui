// dock.js — a Blender/AreaKit-style tiling dock. EVERYTHING is a widget.
//
// Model: a binary tree of nodes.
//   split = { t:'split', dir:'row'|'col', ratio:[a,b], kids:[node,node] }
//   leaf  = { t:'leaf', id, tabs:[{key,inst},...], active, tabSide, tabStyle }
// Each tab is a widget INSTANCE (stable `inst` id). The dock keeps a pool of
// instance DOM elements, created once (body()+init) and MOVED between areas on
// re-render, so a widget's live state and event handlers survive splitting,
// tabbing and resizing. One tree per workspace, persisted, every op undoable.
//
// Gestures (docs/panes.md, adapted):
//   • drag an area header / palette widget onto another area:
//       edge → the target splits, the pane lands on that side
//       centre → the target is taken over (content replaced)
//   • hold Ctrl while dragging → insert as a TAB (drop side places the strip;
//       a vertical strip shows icons)
//   • drag the line between two areas → resize (proportions preserved)
//   • click the type marker (top-left) → change what the area shows
//   • Ctrl+Shift+Z / Ctrl+Shift+X → undo / redo   (all changes auto-saved)

let uid = 1;
const nid = () => 'n' + (uid++);
const clone = o => JSON.parse(JSON.stringify(o));

export function initDock(deps) {
  const { container, widgets, ICON, svg, toast, showMenu, defaults } = deps;
  const SKEY = 'n0xis.dock.v2';

  let ws = 'decompile';
  const trees = {};                 // ws -> root node | null
  const hist = {}, fut = {};        // undo / redo stacks per ws
  const pool = new Map();           // inst id -> { el, key }
  const ensure = w => { if (!(w in trees)) { trees[w] = null; hist[w] = []; fut[w] = []; } };

  // ---- persistence ----
  let onChange = () => {};
  function save() { try { localStorage.setItem(SKEY, JSON.stringify(trees)); } catch {} onChange(); }
  function load() {
    try { const raw = localStorage.getItem(SKEY); if (raw) { const o = JSON.parse(raw); for (const k in o) { trees[k] = o[k]; hist[k] = []; fut[k] = []; } } } catch {}
  }

  // ---- undo/redo ----
  function snapshot() { ensure(ws); hist[ws].push(clone(trees[ws])); if (hist[ws].length > 80) hist[ws].shift(); fut[ws] = []; }
  function undo() { ensure(ws); if (!hist[ws].length) return toast?.('Nothing to undo'); fut[ws].push(clone(trees[ws])); trees[ws] = hist[ws].pop(); render(); save(); }
  function redo() { ensure(ws); if (!fut[ws].length) return toast?.('Nothing to redo'); hist[ws].push(clone(trees[ws])); trees[ws] = fut[ws].pop(); render(); save(); }

  // ---- node / instance helpers ----
  const mkInst = key => ({ key, inst: nid() });
  const leaf = key => ({ t: 'leaf', id: nid(), tabs: [mkInst(key)], active: 0, tabSide: 'top', tabStyle: 'auto' });
  function findLeaf(node, id) {
    if (!node) return null;
    if (node.t === 'leaf') return node.id === id ? { node } : null;
    return findLeaf(node.kids[0], id) || findLeaf(node.kids[1], id);
  }
  function replaceNode(oldLeafId, newNode) {
    const p = parentOfLeaf(trees[ws], oldLeafId);
    if (!p || !p.parent) trees[ws] = newNode; else p.parent.kids[p.side] = newNode;
  }
  function parentOfLeaf(node, id, parent = null, side = -1) {
    if (!node) return null;
    if (node.t === 'leaf') return node.id === id ? { parent, side } : null;
    return parentOfLeaf(node.kids[0], id, node, 0) || parentOfLeaf(node.kids[1], id, node, 1);
  }
  function parentOfNode(node, target, parent = null, side = -1) {
    if (!node || node.t === 'leaf') return null;
    if (node === target) return { parent, side };
    return parentOfNode(node.kids[0], target, node, 0) || parentOfNode(node.kids[1], target, node, 1);
  }
  function removeLeaf(id) {
    const p = parentOfLeaf(trees[ws], id);
    if (!p || !p.parent) { trees[ws] = null; return; } // was the root leaf → empty workspace
    const sibling = p.parent.kids[1 - p.side];
    const gp = parentOfNode(trees[ws], p.parent);
    if (!gp || !gp.parent) trees[ws] = sibling; else gp.parent.kids[gp.side] = sibling;
  }
  function collectInsts(node, out = []) {
    if (!node) return out;
    if (node.t === 'leaf') node.tabs.forEach(t => out.push(t.inst));
    else { collectInsts(node.kids[0], out); collectInsts(node.kids[1], out); }
    return out;
  }
  function dropInst(inst) { const e = pool.get(inst); if (e) { e.el.remove(); pool.delete(inst); } }

  // ---- operations ----
  function addToEmpty(key) { snapshot(); trees[ws] = leaf(key); render(); save(); }
  function splitAt(targetId, side, entry, movingFrom) {
    snapshot();
    const nl = { t: 'leaf', id: nid(), tabs: [entry], active: 0, tabSide: 'top', tabStyle: 'auto' };
    const t = findLeaf(trees[ws], targetId).node;
    const dir = (side === 'left' || side === 'right') ? 'row' : 'col';
    const kids = (side === 'left' || side === 'top') ? [nl, t] : [t, nl];
    replaceNode(targetId, { t: 'split', dir, ratio: [0.5, 0.5], kids });
    if (movingFrom) detach(movingFrom);
    render(); save();
  }
  function takeover(targetId, entry, movingFrom) {
    snapshot();
    const t = findLeaf(trees[ws], targetId).node;
    t.tabs.forEach(x => { if (!movingFrom || x.inst !== entry.inst) dropInst(x.inst); });
    t.tabs = [entry]; t.active = 0;
    if (movingFrom) detach(movingFrom, entry.inst);
    render(); save();
  }
  function addTab(targetId, side, entry, movingFrom) {
    snapshot();
    const t = findLeaf(trees[ws], targetId).node;
    t.tabs.push(entry); t.active = t.tabs.length - 1;
    if (['left', 'right', 'top', 'bottom'].includes(side)) t.tabSide = side;
    if (movingFrom) detach(movingFrom, entry.inst);
    render(); save();
  }
  // remove the dragged instance from its old leaf (keep its pooled DOM — it moved)
  function detach(mv, keepInst) {
    const f = findLeaf(trees[ws], mv.leafId); if (!f) return;
    const n = f.node, i = n.tabs.findIndex(x => x.inst === mv.inst);
    if (i < 0) return;
    if (n.tabs.length <= 1) { const p = parentOfLeaf(trees[ws], n.id); if (p || trees[ws] === n) removeLeafKeep(n.id, keepInst); }
    else { n.tabs.splice(i, 1); n.active = Math.max(0, Math.min(n.active, n.tabs.length - 1)); }
  }
  function removeLeafKeep(id, keepInst) {
    const p = parentOfLeaf(trees[ws], id);
    const node = findLeaf(trees[ws], id)?.node;
    if (node) node.tabs.forEach(t => { if (t.inst !== keepInst) dropInst(t.inst); });
    if (!p || !p.parent) { trees[ws] = null; return; } // was the root leaf
    const sibling = p.parent.kids[1 - p.side];
    const gp = parentOfNode(trees[ws], p.parent);
    if (!gp || !gp.parent) trees[ws] = sibling; else gp.parent.kids[gp.side] = sibling;
  }
  function closePane(leafId, tabIndex) {
    snapshot();
    const f = findLeaf(trees[ws], leafId); if (!f) return;
    const n = f.node, entry = n.tabs[tabIndex];
    if (entry) dropInst(entry.inst);
    if (n.tabs.length <= 1) removeLeaf(leafId);
    else { n.tabs.splice(tabIndex, 1); n.active = Math.max(0, Math.min(n.active, n.tabs.length - 1)); }
    render(); save();
  }

  // ---- instance bodies (created once, then reused) ----
  function bodyFor(entry) {
    let rec = pool.get(entry.inst);
    if (!rec) {
      const def = widgets[entry.key] || { body: () => '' };
      const el = document.createElement('div');
      el.className = 'dk-widget-body';
      try { el.innerHTML = def.body ? def.body() : ''; } catch { el.innerHTML = ''; }
      try { def.init?.(el); } catch (e) { console.warn('widget init failed', entry.key, e); }
      rec = { el, key: entry.key };
      pool.set(entry.inst, rec);
    }
    return rec.el;
  }

  // =========================================================================
  // rendering (moves pooled bodies rather than recreating them)
  // =========================================================================
  function render() {
    ensure(ws);
    const root = trees[ws];
    container.classList.toggle('dk-has', !!root);
    // detach all pooled bodies so we can re-place the active ones
    pool.forEach(r => r.el.remove());
    container.innerHTML = '';
    if (!root) {
      garbageCollect();
      container.style.pointerEvents = 'auto';
      const e = document.createElement('div'); e.className = 'dk-empty';
      e.innerHTML = `<div class="dk-empty-t">This workspace is empty</div><div class="dk-empty-s">Add a widget, or restore the default layout.</div><div class="dk-empty-a"><button data-a="add">+ Add widget</button><button data-a="reset">Reset to default</button></div>`;
      e.querySelector('[data-a="add"]').addEventListener('click', () => document.getElementById('btn-addw')?.click());
      e.querySelector('[data-a="reset"]').addEventListener('click', () => reset());
      container.appendChild(e);
      return;
    }
    container.style.pointerEvents = 'auto';
    const wrap = document.createElement('div'); wrap.className = 'dk-root';
    wrap.appendChild(renderNode(root));
    container.appendChild(wrap);
    garbageCollect();
  }
  function garbageCollect() {
    const live = new Set(collectInsts(trees[ws]));
    [...pool.keys()].forEach(k => { if (!live.has(k)) dropInst(k); });
  }
  function renderNode(node) {
    if (node.t === 'split') {
      const d = document.createElement('div'); d.className = 'dk-split ' + node.dir;
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
    area.className = 'dk-area side-' + node.tabSide; area.dataset.leaf = node.id;
    const vertical = node.tabSide === 'left' || node.tabSide === 'right';
    const iconOnly = node.tabStyle === 'icon' || (node.tabStyle === 'auto' && vertical);
    const multi = node.tabs.length > 1;

    let strip = null;
    if (multi || node.tabSide !== 'top') {
      strip = document.createElement('div');
      strip.className = 'dk-tabs ' + (vertical ? 'v' : 'h');
      node.tabs.forEach((entry, i) => {
        const w = widgets[entry.key] || { title: entry.key, icon: 'decomp' };
        const tab = document.createElement('div');
        tab.className = 'dk-tab' + (i === node.active ? ' on' : ''); tab.title = w.title;
        tab.innerHTML = `<span class="dk-ti">${svg(ICON[w.icon] || ICON.decomp, '')}</span>` +
          (iconOnly ? '' : `<span class="dk-tn">${w.title}</span>`) +
          `<span class="dk-tx" title="Close">${svg('M7 7l10 10M17 7l-10 10', '')}</span>`;
        tab.addEventListener('click', e => { if (e.target.closest('.dk-tx')) { closePane(node.id, i); return; } node.active = i; render(); save(); });
        tab.addEventListener('pointerdown', e => { if (e.target.closest('.dk-tx')) return; startDrag(e, { entry, leafId: node.id }); });
        tab.addEventListener('contextmenu', e => { e.preventDefault(); e.stopPropagation(); tabMenu(e, node, i); });
        strip.appendChild(tab);
      });
    }

    const active = node.tabs[node.active];
    const w = widgets[active.key] || { title: active.key, icon: 'decomp' };
    const head = document.createElement('div'); head.className = 'dk-head';
    head.innerHTML =
      `<span class="dk-marker" title="Change what this shows">${svg(ICON[w.icon] || ICON.decomp, '')}</span>` +
      `<span class="dk-title">${w.title}</span><span class="dk-grow"></span>` +
      `<span class="dk-close" title="Close area">${svg('M6 6l12 12M18 6l-12 12', '')}</span>`;
    head.querySelector('.dk-marker').addEventListener('click', e => { e.stopPropagation(); paneMenu(e, node); });
    head.querySelector('.dk-close').addEventListener('click', e => { e.stopPropagation(); closePane(node.id, node.active); });
    head.addEventListener('pointerdown', e => { if (e.target.closest('.dk-close,.dk-marker')) return; startDrag(e, { entry: active, leafId: node.id }); });
    head.addEventListener('contextmenu', e => { e.preventDefault(); e.stopPropagation(); paneMenu(e, node); });

    const body = document.createElement('div'); body.className = 'dk-body';
    body.appendChild(bodyFor(active));

    if (strip && node.tabSide === 'top') area.append(strip, head, body);
    else if (strip && node.tabSide === 'bottom') area.append(head, body, strip);
    else if (strip && node.tabSide === 'left') { const row = document.createElement('div'); row.className = 'dk-vrow'; row.append(strip, col(head, body)); area.append(row); }
    else if (strip && node.tabSide === 'right') { const row = document.createElement('div'); row.className = 'dk-vrow'; row.append(col(head, body), strip); area.append(row); }
    else area.append(head, body);
    return area;
  }
  function col(a, b) { const c = document.createElement('div'); c.className = 'dk-col'; c.append(a, b); return c; }

  // ---- gutter resize (proportions preserved) ----
  function gutterDrag(g, node, splitEl) {
    g.addEventListener('pointerdown', e => {
      e.preventDefault(); g.setPointerCapture(e.pointerId); g.classList.add('drag');
      const horiz = node.dir === 'row', rect = splitEl.getBoundingClientRect();
      const total = horiz ? rect.width : rect.height;
      const move = ev => {
        const pos = horiz ? ev.clientX - rect.left : ev.clientY - rect.top;
        const r = Math.max(0.08, Math.min(0.92, pos / total));
        node.ratio = [r, 1 - r];
        splitEl.children[0].style.flex = node.ratio[0];
        splitEl.children[2].style.flex = node.ratio[1];
      };
      const up = () => { g.releasePointerCapture(e.pointerId); g.classList.remove('drag'); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); save(); };
      window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
    });
  }

  // =========================================================================
  // drag & drop (split / takeover / tab) — Ctrl toggles tab mode
  // =========================================================================
  let drag = null, ghost = null, ov = null, ctrl = false;
  function startDrag(e, payload) {   // payload: { entry, leafId? }  (leafId absent = from palette)
    e.preventDefault();
    drag = { payload, x: e.clientX, y: e.clientY, moved: false, hit: null };
    ctrl = e.ctrlKey || e.metaKey;
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('keyup', onKey, true);
  }
  function startPaletteDrag(e, key) { startDrag(e, { entry: mkInst(key) }); }
  function onKey(e) { if (e.key === 'Control' || e.key === 'Meta') { ctrl = e.type === 'keydown'; if (drag && drag.hit) paint(drag.hit); } }
  function onMove(e) {
    if (!drag) return;
    if (!drag.moved && Math.hypot(e.clientX - drag.x, e.clientY - drag.y) < 5) return;
    if (!drag.moved) { drag.moved = true; mkGhost(); mkOverlay(); }
    ctrl = e.ctrlKey || e.metaKey;
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
    const { r, zone } = hit, tab = ctrl;
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
    if (!d || !d.moved) { if (d && d.payload && !d.payload.leafId) addEntry(d.payload.entry); return; } // plain palette click
    if (!d.hit) return;
    const { payload } = d, { leafId, zone } = d.hit;
    const moving = payload.leafId ? { leafId: payload.leafId, inst: payload.entry.inst } : null;
    if (moving && moving.leafId === leafId) {
      const n = findLeaf(trees[ws], leafId)?.node;
      if (zone === 'center' && (!n || n.tabs.length <= 1) && !wasTab) return; // onto itself = no-op
    }
    if (wasTab) addTab(leafId, zone === 'center' ? 'top' : zone, payload.entry, moving);
    else if (zone === 'center') takeover(leafId, payload.entry, moving);
    else splitAt(leafId, zone, payload.entry, moving);
  }
  function mkGhost() {
    ghost = document.createElement('div'); ghost.className = 'dk-ghost';
    const w = widgets[drag.payload.entry.key] || { title: drag.payload.entry.key, icon: 'decomp' };
    ghost.innerHTML = `${svg(ICON[w.icon] || ICON.decomp, '')}<span>${w.title}</span>`;
    document.body.appendChild(ghost);
  }
  function mkOverlay() { if (!ov) { ov = document.createElement('div'); ov.className = 'dk-ov'; document.body.appendChild(ov); } }

  // ---- context menus ----
  function paneMenu(e, node) {
    const items = [{ label: 'Change what this shows', header: true }];
    for (const [k, w] of Object.entries(widgets))
      items.push({ label: w.title, icon: ICON[w.icon] || ICON.decomp, act: () => { snapshot(); const old = node.tabs[node.active]; dropInst(old.inst); node.tabs[node.active] = mkInst(k); render(); save(); } });
    items.push({ sep: true }, { label: 'Close area', icon: ICON.trash, danger: true, act: () => closePane(node.id, node.active) });
    showMenu(e.clientX, e.clientY, items);
  }
  function tabMenu(e, node, i) {
    const w = widgets[node.tabs[i].key] || {};
    showMenu(e.clientX, e.clientY, [
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
    ]);
  }
  function setSide(node, s) { snapshot(); node.tabSide = s; render(); save(); }

  // ---- seed a default layout (instances get fresh ids) ----
  function instantiate(spec) {
    if (spec.t === 'split') return { t: 'split', dir: spec.dir, ratio: spec.ratio ? spec.ratio.slice() : [0.5, 0.5], kids: spec.kids.map(instantiate) };
    return { t: 'leaf', id: nid(), tabs: spec.tabs.map(k => mkInst(k)), active: spec.active || 0, tabSide: spec.tabSide || 'top', tabStyle: spec.tabStyle || 'auto' };
  }

  // ---- public API ----
  function setWorkspace(w) {
    ws = w; ensure(w);
    if (!trees[w] && defaults && defaults[w]) trees[w] = instantiate(defaults[w]); // seed default layout
    render();
  }
  function addEntry(entry) {
    ensure(ws);
    if (!trees[ws]) { snapshot(); trees[ws] = { t: 'leaf', id: nid(), tabs: [entry], active: 0, tabSide: 'top', tabStyle: 'auto' }; render(); save(); return; }
    let big = null, area = 0;
    container.querySelectorAll('.dk-area').forEach(a => { const r = a.getBoundingClientRect(); if (r.width * r.height > area) { area = r.width * r.height; big = a.dataset.leaf; } });
    if (big) { const r = container.querySelector(`[data-leaf="${big}"]`).getBoundingClientRect(); splitAt(big, r.width >= r.height ? 'right' : 'bottom', entry); }
    else addToEmpty(entry.key);
  }
  function addWidget(key) { addEntry(mkInst(key)); }
  function reset() { snapshot(); collectInsts(trees[ws]).forEach(dropInst); trees[ws] = defaults && defaults[ws] ? instantiate(defaults[ws]) : null; render(); save(); toast?.('Layout reset to default'); }
  function clearAll() { snapshot(); collectInsts(trees[ws]).forEach(dropInst); trees[ws] = null; render(); save(); toast?.('Layout cleared'); }

  load();
  return { setWorkspace, addWidget, undo, redo, reset, clearAll, render, startPaletteDrag, onChangeHook: cb => { onChange = cb; } };
}
