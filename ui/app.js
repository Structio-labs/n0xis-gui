// N0xis GUI — interactive shell (framework-free; drops into Tauri's webview as-is)
import { isNative, engineInfo, engine, pickFile } from './bridge.js';
import { initDock } from './dock.js';
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

// ---------- tiny icon set (shared by menus / widgets) ----------
const ICON = {
  decomp: 'M8 6l-6 6 6 6M16 6l6 6-6 6',
  disasm: 'M4 6h16M4 12h10M4 18h13',
  graph:  'M6 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM18 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM6 9v3a3 3 0 0 0 3 3h6',
  rename: 'M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z',
  xref:   'M7 7h10v10M7 17L17 7',
  type:   'M4 7V4h16v3M9 20h6M12 4v16',
  bp:     'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z',
  watch:  'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z',
  copy:   'M9 9h11v11H9zM5 15H4V4h11v1',
  freeze: 'M12 2v20M4 7l16 10M20 7L4 17',
  trash:  'M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14',
  scan:   'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM21 21l-4-4',
  play:   'M6 4l14 8-14 8z',
  hex:    'M4 7l8-4 8 4v10l-8 4-8-4z',
  regs:   'M4 5h16v4H4zM4 13h16v6H4z',
  chat:   'M4 5h16v11H8l-4 4z',
  note:   'M6 3h9l3 3v15H6zM14 3v5h5',
  add:    'M12 5v14M5 12h14',
  reset:  'M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5',
  settings:'M4 7h16M4 12h16M4 17h16',
  strings:'M4 7V4h16v3M9 20h6M12 4v16',
  fold:   'M6 9l6 6 6-6',
};
const svg = (d, cls = 'cxi') =>
  `<span class="${cls}"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">${
    d.split(';').map(p => `<path d="${p}"/>`).join('')}</svg></span>`;

// ---------- toast (feedback for actions) ----------
let toastEl;
function toast(msg) {
  if (!toastEl) {
    toastEl = document.createElement('div');
    toastEl.style.cssText = 'position:fixed;left:50%;bottom:46px;transform:translateX(-50%);z-index:300;' +
      'background:var(--bg2);border:1px solid var(--bd2);color:var(--tx0);padding:9px 15px;border-radius:9px;' +
      'font-size:.82rem;box-shadow:var(--shadow);opacity:0;transition:opacity .15s, transform .15s;pointer-events:none;';
    document.body.appendChild(toastEl);
  }
  toastEl.innerHTML = msg;
  toastEl.style.opacity = '1';
  toastEl.style.transform = 'translateX(-50%) translateY(-4px)';
  clearTimeout(toast._t);
  toast._t = setTimeout(() => { toastEl.style.opacity = '0'; toastEl.style.transform = 'translateX(-50%)'; }, 1700);
}
// also echo to the Output console when it exists
function echo(cmd, ok) {
  const line = document.querySelector('#dockspace .oline');
  const out = line?.parentElement;
  if (!out) { toast(cmd + ' — ' + ok); return; }
  const l1 = document.createElement('div'); l1.className = 'oline'; l1.innerHTML = `<span class="p">›</span> ${cmd}`;
  const l2 = document.createElement('div'); l2.className = 'oline'; l2.style.color = 'var(--ok)'; l2.textContent = ok;
  out.append(l1, l2); out.scrollTop = out.scrollHeight;
}

// ---------- workspaces ----------
function setWorkspace(ws) {
  $$('.wsview').forEach(v => v.classList.toggle('on', v.id === 'view-' + ws));
  $$('#wsbar .ws').forEach(t => t.classList.toggle('on', t.dataset.ws === ws));
  document.documentElement.dataset.ws = ws;
  dock.setWorkspace(ws); // the workspace IS a dock layout of widgets
}
$$('#wsbar .ws[data-ws]').forEach(t => t.addEventListener('click', () => setWorkspace(t.dataset.ws)));
$$('[data-ws]').forEach(el => { if (!el.classList.contains('ws')) el.addEventListener('click', () => setWorkspace(el.dataset.ws)); });

function openTarget(kind) {
  $('#launcher').classList.add('hidden');
  const live = kind === 'dynamic' || kind === 'both';
  $('#target-state').textContent = live ? 'live' : 'static';
  $('#target-state').style.color = live ? 'var(--live)' : 'var(--acc)';
  $('#sb-dot').classList.toggle('live', live);
  $('#sb-mode').textContent = live ? 'live · pid 8124 · running' : 'static · PE x86-64';
  setWorkspace(live ? 'dynamic' : 'decompile');
}
// ---------- launcher ----------
$$('#launcher .card').forEach(card => card.addEventListener('click', () => openTarget(card.dataset.open)));
$('#btn-live').addEventListener('click', () => openTarget('dynamic'));

// ---------- delegated widget interactions (widgets are created dynamically) ----------
$('#dockspace').addEventListener('click', e => {
  // workspace jump buttons (e.g. the decompiler's "CFG ↗")
  const jump = e.target.closest('[data-ws]');
  if (jump) { setWorkspace(jump.dataset.ws); return; }
  // function / string list selection
  const row = e.target.closest('.frow');
  if (row) { row.closest('.flist,div')?.querySelectorAll('.frow.on').forEach(r => r.classList.remove('on')); row.classList.add('on'); return; }
  // cosmetic single-select groups inside a widget
  for (const sel of ['.nt', '.rt', '.pill', '.dt']) {
    const g = e.target.closest(sel);
    if (g && !g.hasAttribute('data-ws')) { g.parentElement.querySelectorAll(sel).forEach(x => x.classList.remove('on')); g.classList.add('on'); return; }
  }
});

// ---------- zoom ----------
let zoom = 1;
function setZoom(z) {
  zoom = clamp(Math.round(z * 100) / 100, 0.7, 1.6);
  document.documentElement.style.setProperty('--zoom', zoom);
  $('#sb-zoom').textContent = 'zoom ' + Math.round(zoom * 100) + '%';
  $$('#scale-seg .sg').forEach(b => b.classList.toggle('on', Math.abs(parseFloat(b.dataset.scale) - zoom) < 0.001));
}
$$('#scale-seg .sg').forEach(b => b.addEventListener('click', () => setZoom(parseFloat(b.dataset.scale))));
window.addEventListener('keydown', e => {
  if (!(e.ctrlKey || e.metaKey)) return;
  if (e.key === '=' || e.key === '+') { e.preventDefault(); setZoom(zoom + 0.1); }
  else if (e.key === '-') { e.preventDefault(); setZoom(zoom - 0.1); }
  else if (e.key === '0') { e.preventDefault(); setZoom(1); }
});

// ---------- theme (live preview) ----------
$$('#theme-grid .th').forEach(t => t.addEventListener('click', () => {
  const th = t.dataset.theme;
  if (th) document.documentElement.setAttribute('data-theme', th);
  else document.documentElement.removeAttribute('data-theme');
  $$('#theme-grid .th').forEach(x => x.classList.remove('on'));
  t.classList.add('on');
}));

// ---------- settings overlay ----------
const settings = $('#settings-ov');
$('#btn-settings').addEventListener('click', () => settings.classList.add('on'));
$('#settings-close').addEventListener('click', () => settings.classList.remove('on'));
settings.addEventListener('click', e => { if (e.target === settings) settings.classList.remove('on'); });
$$('.snav .sni').forEach(n => n.addEventListener('click', () => {
  $$('.snav .sni').forEach(x => x.classList.remove('on')); n.classList.add('on');
}));

// ---------- command palette ----------
const COMMANDS = [
  ['Decompile · pseudocode', 'SSA-recovered C for the selected function', 'F5', () => setWorkspace('decompile')],
  ['Disassembly listing', 'Annotated instruction listing', 'Space', () => setWorkspace('decompile')],
  ['Show CFG graph', 'Control-flow graph, blocks & edges', 'G', () => setWorkspace('graph')],
  ['Add widget to canvas…', 'Blender-style dockable panels', '', () => openWpal($('#btn-addw'))],
  ['Reset workspace layout', 'Clear floating widgets', '', () => resetDock()],
  ['Set decompile style…', 'structured · ssa · goto', '', () => toast('Decompile style')],
  ['Find xrefs to / from', 'Who calls this · what it calls', 'X', () => toast('Xrefs')],
  ['Set watchpoint (R / W / X)', 'Break when this address is touched', 'W', () => setWorkspace('dynamic')],
  ['Find what accesses this address', 'Cheat-Engine-style hit counter', '', () => setWorkspace('dynamic')],
  ['Memory scan…', 'Hunt a value in a live process', '', () => setWorkspace('dynamic')],
  ['Attach to a process', 'Bind a live process to this session', '', () => setWorkspace('dynamic')],
  ['Apply FLIRT signatures', 'Name statically-linked library code', '', () => echo('sig apply --flirt zlib-1.3.1.npat', 'named 118 functions')],
  ['Explain this with Copilot', 'Walk through the current function', '↵', () => toast('Copilot')],
  ['Change theme…', 'Switch the colour palette', '', () => settings.classList.add('on')],
  ['Open settings', 'Preferences and providers', ',', () => settings.classList.add('on')],
];
const palOv = $('#palette-ov'), palInput = $('#pal-input'), palList = $('#pal-list');
let palSel = 0, palShown = [];
function renderPal(q = '') {
  const s = q.toLowerCase();
  palShown = COMMANDS.filter(c => (c[0] + ' ' + c[1]).toLowerCase().includes(s));
  palSel = 0;
  palList.innerHTML = palShown.map((c, i) =>
    `<div class="cmd ${i === 0 ? 'on' : ''}" data-i="${i}">${svg(ICON.decomp, 'ci')}<div><div class="cn">${c[0]}</div><div class="cdesc">${c[1]}</div></div>${c[2] ? `<span class="csc kbd">${c[2]}</span>` : ''}</div>`
  ).join('') || `<div class="cmd" style="color:var(--tx2)"><span class="ci"></span>No matching command</div>`;
}
function openPal() { palOv.classList.add('on'); palInput.value = ''; renderPal(); palInput.focus(); }
function closePal() { palOv.classList.remove('on'); }
function runPal() { const c = palShown[palSel]; if (c) { closePal(); c[3](); } }
$('#btn-palette').addEventListener('click', openPal);
palInput.addEventListener('input', () => renderPal(palInput.value));
palOv.addEventListener('click', e => { if (e.target === palOv) closePal(); });
palList.addEventListener('click', e => { const el = e.target.closest('.cmd'); if (!el || el.dataset.i === undefined) return; palSel = +el.dataset.i; runPal(); });
palInput.addEventListener('keydown', e => {
  if (e.key === 'ArrowDown') { e.preventDefault(); palSel = Math.min(palShown.length - 1, palSel + 1); }
  else if (e.key === 'ArrowUp') { e.preventDefault(); palSel = Math.max(0, palSel - 1); }
  else if (e.key === 'Enter') { e.preventDefault(); runPal(); return; }
  else return;
  $$('#pal-list .cmd').forEach((el, i) => el.classList.toggle('on', i === palSel));
  const on = $('#pal-list .cmd.on'); if (on) on.scrollIntoView({ block: 'nearest' });
});
window.addEventListener('keydown', e => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'p') { e.preventDefault(); palOv.classList.contains('on') ? closePal() : openPal(); }
  else if (e.key === 'Escape') { closePal(); settings.classList.remove('on'); closeCtx(); closeWpal(); }
});


/* =====================================================================
   CONTROL-FLOW GRAPH — data-driven, aligned edges, pan + zoom
   ===================================================================== */
const GRAPH = {
  blocks: [
    { id: 'block_0', addr: '0x1510', tag: 'entry',        x: 360, y: 24,  w: 210, body: 'if (rsi == 0) return 0;<br>v14 = ~rdi;' },
    { id: 'block_1', addr: '0x1538', tag: '',             x: 365, y: 150, w: 200, body: 'if (rdx &lt;= 0x2e)' },
    { id: 'block_2', addr: '0x1580', tag: '',             x: 120, y: 300, w: 180, body: 'rbx = rsi;' },
    { id: 'block_3', addr: '0x1560', tag: 'align loop ↺', x: 590, y: 300, w: 195, body: 'while (rbx &amp; 7)' },
    { id: 'block_4', addr: '0x1590', tag: 'by-8 loop ↺',  x: 585, y: 445, w: 205, body: 'for (…; rdx -= 8)' },
    { id: 'block_5', addr: '0x15fa', tag: 'exit',         x: 350, y: 590, w: 210, body: 'return ~v14;' },
  ],
  edges: [
    { f: 'block_0', t: 'block_1' },
    { f: 'block_1', t: 'block_2', type: 't' },
    { f: 'block_1', t: 'block_3', type: 'f' },
    { f: 'block_3', t: 'block_3', type: 'loop' },
    { f: 'block_3', t: 'block_4' },
    { f: 'block_4', t: 'block_4', type: 'loop' },
    { f: 'block_4', t: 'block_5' },
    { f: 'block_2', t: 'block_5' },
  ],
};
// Self-contained: bound to one graph widget's own elements (many can coexist).
function initGraphWidget(root) {
  const gv = root.querySelector('.gviewport'), gc = root.querySelector('.gcanvas'), gs = root.querySelector('.gsvg');
  if (!gv || !gc || !gs) return;
  const cam = { x: 40, y: 20, s: 1 };
  const rectOf = {};
  function renderGraph() {
    gc.querySelectorAll('.gnode').forEach(n => n.remove());
    GRAPH.blocks.forEach(b => {
      const n = document.createElement('div');
      n.className = 'gnode' + (b.tag === 'entry' ? ' entry' : b.tag === 'exit' ? ' exit' : '');
      n.style.left = b.x + 'px'; n.style.top = b.y + 'px'; n.style.width = b.w + 'px';
      n.dataset.ctx = 'gnode'; n.dataset.addr = b.addr; n.dataset.id = b.id;
      const tag = b.tag ? ' · ' + b.tag : '';
      n.innerHTML = `<div class="gt">${b.id} · ${b.addr}${tag}</div>${b.body}`;
      gc.appendChild(n);
      rectOf[b.id] = { x: b.x, y: b.y, w: b.w, h: n.offsetHeight };
    });
    const defs = `<defs><marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="context-stroke"/></marker></defs>`;
    const paths = GRAPH.edges.map(e => {
      const a = rectOf[e.f], b = rectOf[e.t], cls = 'gedge' + (e.type ? ' ' + e.type : '');
      if (e.type === 'loop') { const x = a.x + a.w, y1 = a.y + a.h * 0.28, y2 = a.y + a.h * 0.72; return `<path class="${cls}" marker-end="url(#arrow)" d="M${x} ${y1} C${x + 46} ${y1 - 6},${x + 46} ${y2 + 6},${x} ${y2}"/>`; }
      const sx = a.x + a.w / 2, sy = a.y + a.h, tx = b.x + b.w / 2, ty = b.y, my = (sy + ty) / 2;
      return `<path class="${cls}" marker-end="url(#arrow)" d="M${sx} ${sy} C${sx} ${my},${tx} ${my},${tx} ${ty - 2}"/>`;
    }).join('');
    gs.innerHTML = defs + paths;
  }
  function applyCam() { gc.style.transform = `translate(${cam.x}px,${cam.y}px) scale(${cam.s})`; const l = root.querySelector('.gzl'); if (l) l.textContent = Math.round(cam.s * 100) + '%'; }
  function fitGraph() {
    const vw = gv.clientWidth, vh = gv.clientHeight; if (!vw || !vh) return;
    let maxX = 0, maxY = 0;
    GRAPH.blocks.forEach(b => { maxX = Math.max(maxX, b.x + b.w); maxY = Math.max(maxY, b.y + (rectOf[b.id]?.h || 70)); });
    cam.s = clamp(Math.min(vw / (maxX + 60), vh / (maxY + 60)), 0.4, 1.3);
    cam.x = (vw - maxX * cam.s) / 2; cam.y = Math.max(16, (vh - maxY * cam.s) / 2); applyCam();
  }
  let pan = null;
  let userAdjusted = false;                       // once the user pans/zooms, stop auto-fitting
  gv.addEventListener('pointerdown', e => { if (e.target.closest('.gzoom')) return; pan = { x: e.clientX, y: e.clientY, cx: cam.x, cy: cam.y }; gv.classList.add('panning'); gv.setPointerCapture(e.pointerId); });
  gv.addEventListener('pointermove', e => { if (!pan) return; cam.x = pan.cx + (e.clientX - pan.x); cam.y = pan.cy + (e.clientY - pan.y); userAdjusted = true; applyCam(); });
  gv.addEventListener('pointerup', () => { pan = null; gv.classList.remove('panning'); });
  gv.addEventListener('wheel', e => { e.preventDefault(); const r = gv.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top; const ns = clamp(cam.s * (e.deltaY < 0 ? 1.12 : 0.89), 0.3, 2.4); cam.x = mx - (mx - cam.x) * (ns / cam.s); cam.y = my - (my - cam.y) * (ns / cam.s); cam.s = ns; userAdjusted = true; applyCam(); }, { passive: false });
  root.querySelector('.gz-in').addEventListener('click', () => { cam.s = clamp(cam.s * 1.15, 0.3, 2.4); userAdjusted = true; applyCam(); });
  root.querySelector('.gz-out').addEventListener('click', () => { cam.s = clamp(cam.s * 0.87, 0.3, 2.4); userAdjusted = true; applyCam(); });
  root.querySelector('.gz-fit').addEventListener('click', () => { userAdjusted = false; fitGraph(); }); // fit re-enables auto-fit
  renderGraph();
  requestAnimationFrame(fitGraph);
  setTimeout(fitGraph, 120); // refit once layout settles
  try { new ResizeObserver(() => { if (!userAdjusted) fitGraph(); }).observe(gv); } catch {} // refit when the area is resized, until the user takes over
}

/* =====================================================================
   RIGHT-CLICK CONTEXT MENUS
   ===================================================================== */
const ctx = $('#ctxmenu');
function closeCtx() { ctx.classList.remove('on'); }
const item = (label, iconKey, sc, act, opt = {}) =>
  ({ label, icon: ICON[iconKey], sc, act, ...opt });
const sep = { sep: true };
const lbl = t => ({ label: t, header: true });

function menuFor(el, tgt) {
  const addr = tgt.dataset?.addr || (tgt.querySelector?.('.fa')?.textContent) || '';
  const name = tgt.querySelector?.('.nm')?.textContent || tgt.dataset?.id || '';
  switch (el) {
    case 'frow': return [
      lbl(name || 'function'),
      item('Decompile', 'decomp', 'F5', () => { tgt.click(); setWorkspace('decompile'); toast('Decompiling ' + name); }),
      item('Show disassembly', 'disasm', 'Space', () => { tgt.click(); setWorkspace('decompile'); }),
      item('Show CFG graph', 'graph', 'G', () => setWorkspace('graph')),
      sep,
      item('Rename…', 'rename', 'N', () => toast('Rename ' + name)),
      item('Find xrefs to', 'xref', 'X', () => toast('Xrefs to ' + name)),
      item('Find xrefs from', 'xref', '', () => toast('Xrefs from ' + name)),
      item('Apply signature', 'type', '', () => echo('sig apply --func ' + name, 'matched 1')),
      sep,
      item('Copy name', 'copy', '', () => copy(name)),
      item('Copy address', 'copy', '', () => copy(addr)),
    ];
    case 'cl': return [
      item('Copy line', 'copy', 'Ctrl+C', () => copy(tgt.textContent.replace(/^\d+/, '').trim())),
      item('Rename variable…', 'rename', 'N', () => toast('Rename variable')),
      item('Change type…', 'type', 'Y', () => toast('Change type')),
      sep,
      item('Toggle breakpoint', 'bp', 'F9', () => { tgt.classList.toggle('hot'); toast('Breakpoint toggled'); }),
      item('Add watchpoint', 'watch', 'W', () => { setWorkspace('dynamic'); toast('Watchpoint added'); }),
      item('Comment…', 'note', ';', () => toast('Comment')),
    ];
    case 'drow': return [
      item('Copy instruction', 'copy', '', () => copy(tgt.textContent.trim())),
      item('Toggle breakpoint', 'bp', 'F9', () => { tgt.classList.toggle('hot'); toast('Breakpoint @ ' + (tgt.querySelector('.daddr')?.textContent || '')); }),
      item('Set watchpoint', 'watch', 'W', () => setWorkspace('dynamic')),
      item('Follow in dump', 'hex', '', () => setWorkspace('dynamic')),
      sep,
      item('Copy address', 'copy', '', () => copy(tgt.querySelector('.daddr')?.textContent || '')),
    ];
    case 'wprow': return [
      lbl('watchpoint ' + (tgt.querySelector('.mono')?.textContent || '')),
      item('Enable / disable', 'watch', '', () => { tgt.style.opacity = tgt.style.opacity === '.5' ? '1' : '.5'; }),
      item('Edit condition…', 'rename', '', () => toast('Edit condition')),
      item('Reveal writer', 'xref', '', () => { setWorkspace('decompile'); toast('Jumping to writer'); }),
      sep,
      item('Copy address', 'copy', '', () => copy(tgt.querySelector('.mono')?.textContent || '')),
      item('Remove watchpoint', 'trash', 'Del', () => { tgt.remove(); toast('Watchpoint removed'); }, { danger: true }),
    ];
    case 'srow': return [
      lbl('scan hit ' + (tgt.querySelector('.saddr')?.textContent || '')),
      item('Add to watchlist', 'add', '', () => toast('Added to watchlist')),
      item('Freeze value', 'freeze', '', () => toast('Value frozen')),
      item('Set write-watchpoint', 'watch', 'W', () => toast('Watchpoint set')),
      item('Find what writes this', 'scan', '', () => toast('Tracing writers…')),
      item('Browse in memory', 'hex', '', () => toast('Opening hex view')),
      sep,
      item('Copy address', 'copy', '', () => copy(tgt.querySelector('.saddr')?.textContent || '')),
    ];
    case 'wlrow': return [
      item('Freeze / unfreeze', 'freeze', '', () => tgt.querySelector('.frz')?.classList.toggle('on')),
      item('Edit value…', 'rename', '', () => toast('Edit value')),
      item('Rename', 'rename', '', () => toast('Rename entry')),
      sep,
      item('Remove', 'trash', 'Del', () => { tgt.remove(); }, { danger: true }),
    ];
    case 'gnode': return [
      lbl(tgt.dataset.id || 'block'),
      item('Focus block', 'graph', '', () => { fitGraph(); toast('Focus ' + tgt.dataset.id); }),
      item('Decompile block', 'decomp', '', () => { setWorkspace('decompile'); toast('Decompile ' + tgt.dataset.id); }),
      item('Set breakpoint', 'bp', 'F9', () => toast('Breakpoint @ ' + tgt.dataset.addr)),
      sep,
      item('Copy block address', 'copy', '', () => copy(tgt.dataset.addr || '')),
    ];
    default: return [
      item('Add widget…', 'add', '', () => openWpal($('#btn-addw'))),
      item('Command palette', 'decomp', 'Ctrl+P', openPal),
      item('Reset layout', 'reset', '', resetDock),
      sep,
      item('Settings', 'settings', ',', () => settings.classList.add('on')),
    ];
  }
}
function copy(t) { navigator.clipboard?.writeText(t).catch(() => {}); toast('Copied <span class="mono">' + t + '</span>'); }

function openCtx(x, y, items) {
  ctx.innerHTML = items.map((it, i) => {
    if (it.sep) return '<div class="ctxsep"></div>';
    if (it.header) return `<div class="ctxlbl">${it.label}</div>`;
    return `<div class="ctxi${it.danger ? ' danger' : ''}${it.disabled ? ' dis' : ''}" data-i="${i}">${
      svg(it.icon || ICON.decomp)}${it.label}${it.sc ? `<span class="csc">${it.sc}</span>` : ''}</div>`;
  }).join('');
  ctx.classList.add('on');
  const r = ctx.getBoundingClientRect();
  ctx.style.left = Math.min(x, innerWidth - r.width - 8) + 'px';
  ctx.style.top = Math.min(y, innerHeight - r.height - 8) + 'px';
  ctx.onclick = ev => {
    const el = ev.target.closest('.ctxi'); if (!el) return;
    const it = items[+el.dataset.i]; closeCtx(); it?.act?.();
  };
}
window.addEventListener('contextmenu', e => {
  if (e.target.closest('.overlay.on, .modal, .wpal.on')) return; // let overlays keep native
  e.preventDefault();
  const known = e.target.closest('[data-ctx],.frow,.cl,.drow,.wprow,.srow,.wlrow,.gnode');
  const kind = known ? (known.dataset.ctx || known.className.split(' ').find(c => ['frow','cl','drow','wprow','srow','wlrow','gnode'].includes(c))) : 'default';
  openCtx(e.clientX, e.clientY, menuFor(kind, known || document.body));
});
window.addEventListener('pointerdown', e => { if (!e.target.closest('#ctxmenu')) closeCtx(); }, true);
window.addEventListener('blur', closeCtx);
window.addEventListener('scroll', closeCtx, true);

/* =====================================================================
   WIDGET CATALOG + TILING DOCK (Blender/AreaKit-style)
   ===================================================================== */
// Every panel — static and dynamic — is a widget. The default workspaces below
// are just dock layouts built from this same catalog (no privileged core).
const CODE_LINES = [
  '<span class="ty">uint64_t</span> <span class="fnc">crc32_z</span>(<span class="ty">uint64_t</span> rdi, <span class="ty">void</span> *rsi, <span class="ty">uint64_t</span> rdx) {',
  '&nbsp;&nbsp;<span class="k">if</span> ((rsi == <span class="nu">0x0</span>)) <span class="k">return</span> <span class="nu">0x0</span>;',
  '&nbsp;&nbsp;v14 = ~rdi;',
  '&nbsp;&nbsp;<span class="k">if</span> ((rdx &lt;= <span class="cm">/*u*/</span> <span class="nu">0x2e</span>)) {',
  '&nbsp;&nbsp;&nbsp;&nbsp;rbx = rsi;',
  '&nbsp;&nbsp;} <span class="k">else</span> {',
  '&nbsp;&nbsp;&nbsp;&nbsp;v9 = &amp;<span class="st">crc_table</span>;',
  '&nbsp;&nbsp;&nbsp;&nbsp;<span class="k">while</span> (((rbx &amp; <span class="nu">0x7</span>) != <span class="nu">0x0</span>)) {',
  '&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;v14 = (v9[(<span class="ty">uint32_t</span>)(v14 ^ rbx-&gt;<span class="pu">field_0x0</span>)] ^ (v14 &gt;&gt; <span class="nu">0x8</span>));',
  '&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;rdx = (rdx - <span class="nu">0x1</span>);',
  '&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<span class="k">if</span> ((rdx != <span class="nu">0x0</span>)) <span class="k">continue</span>; <span class="k">else</span> <span class="k">break</span>;',
  '&nbsp;&nbsp;&nbsp;&nbsp;}',
  '&nbsp;&nbsp;&nbsp;&nbsp;<span class="cm">// main CRC-by-8 loop</span>',
  '&nbsp;&nbsp;&nbsp;&nbsp;<span class="k">for</span> (; (rdx != <span class="nu">0x0</span>); rdx = (rdx - <span class="nu">0x8</span>)) {',
  '&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;v12 = (v9[(<span class="ty">uint32_t</span>)(v14 ^ *rbx)] ^ (v14 &gt;&gt; <span class="nu">0x8</span>));',
  '&nbsp;&nbsp;&nbsp;&nbsp;}',
  '&nbsp;&nbsp;}',
  '&nbsp;&nbsp;<span class="k">return</span> ~v14;',
  '}',
];
const FUNCS = [
  ['main', '0x401060', ''], ['crc32_z', '0x1510', 'sig'], ['crc32', '0x19d0', ''],
  ['compress', '0x14d0', 'sig'], ['deflate', '0x4650', ''], ['inflate_table', '0x2a10', ''],
  ['sub_180002780', '0x2780', 'sub'], ['adler32', '0x7d70', ''], ['uncompress', '0x7850', ''],
  ['sub_180003b40', '0x3b40', 'sub'],
];
const WIDGETS = {
  functions: { title: 'Functions', icon: 'strings', body: () => `<div class="wfill">
    <div class="navtabs"><button class="nt on">Functions</button><button class="nt">Imports</button><button class="nt">Strings</button><button class="nt">Types</button></div>
    <div class="search">${svg(ICON.scan,'')}Filter functions…</div>
    <div class="flist">${FUNCS.map(([n,a,b])=>`<div class="frow${n==='crc32_z'?' on':''}${b==='sub'?' sub':''}"><span class="sym">ƒ</span><span class="nm mono">${n}</span>${b==='sig'?'<span class="badge">sig</span>':''}<span class="fa mono">${a}</span></div>`).join('')}</div>
    <div class="pfoot">2,318 functions<span class="grow"></span><span style="color:var(--ok)">86% named</span></div></div>` },

  decompiler: { title: 'Decompiler', icon: 'decomp', body: () => `<div class="wfill">
    <div class="dockhead"><button class="pill on">structured</button><button class="pill">ssa</button><button class="pill">goto</button><div class="grow"></div><button class="dt" data-ws="graph">CFG ↗</button></div>
    <div class="code selectable" style="flex:1">${CODE_LINES.map((l,i)=>`<div class="cl${i===7?' hot':''}"><span class="gut">${i+1}</span><span>${l}</span></div>`).join('')}</div></div>` },

  disassembly: { title: 'Disassembly · x86-64', icon: 'disasm', body: () => `<div class="disasm selectable" style="height:100%;">${[
      ['0x1000015c8','48 89 d8','mov','rax, rbx',''],['0x1000015cb','83 e0 07','and','eax, 0x7',''],
      ['0x1000015ce','74 2a','je','0x1000015fa','hot'],['0x1000015d0','0f b6 03','movzx','eax, byte [rbx]',''],
      ['0x1000015d3','31 f0','xor','eax, esi',''],['0x1000015d8','8b 04 87','mov','eax, [rdi + rax*4]',''],
    ].map(([a,b,m,o,h])=>`<div class="drow${h?' hot':''}" data-ctx="drow"><span class="daddr">${a}</span><span class="dbytes">${b}</span><span class="dmn"${h?' style="color:var(--live)"':''}>${m}</span><span>${o}</span></div>`).join('')}</div>` },

  hex: { title: 'Hex', icon: 'hex', body: () => `<div class="hex selectable" style="height:100%"><div class="hx"><span class="hxa">7FF6C21A40</span><span class="hxb"><span class="hb-hi">57 00 00 00</span> 2a 00 00 00 5c 21 c2 f6</span><span class="hxc">W....*...\\!</span></div><div class="hx"><span class="hxa">7FF6C21A50</span><span class="hxb">64 00 00 00 00 00 80 3f 00 00 80 3f</span><span class="hxc">d......?...?</span></div></div>` },

  graph: { title: 'CFG · crc32_z', icon: 'graph', body: () => `<div class="graphwrap" style="height:100%">
    <div class="gviewport"><div class="gcanvas"><svg class="gsvg" width="920" height="700"></svg></div>
    <div class="gzoom"><button class="gz-in">+</button><div class="gzl">100%</div><button class="gz-out">−</button><button class="gz-fit">${svg('M4 9V5a1 1 0 0 1 1-1h4;M20 9V5a1 1 0 0 0-1-1h-4;M4 15v4a1 1 0 0 0 1 1h4;M20 15v4a1 1 0 0 1-1 1h-4','')}</button></div></div></div>`,
    init: root => initGraphWidget(root) },

  copilot: { title: 'Copilot', icon: 'chat', body: () => `<div class="wfill">
    <div class="chat" style="flex:1"><div class="msg"><div class="av me">ME</div><div class="bub mine">What does <span class="mono" style="color:var(--acc)">crc32_z</span> do?</div></div>
    <div class="msg"><div class="av ai">AI</div><div><div class="bub">This is the <b>zlib CRC-32</b> checksum core. It folds each input byte through a 256-entry table (<span class="mono" style="color:var(--st)">crc_table</span>) — byte-by-byte until 8-aligned, then 8 bytes per pass.<div style="margin-top:6px;color:var(--tx1)">• <span class="mono">rsi</span> = buffer, <span class="mono">rdx</span> = length, <span class="mono">rdi</span> = seed.</div></div>
    <div class="sug"><span class="sugb">Find callers</span><span class="sugb">Explain “SSA”</span><span class="sugb">Rename vars</span></div></div></div></div>
    <div class="ask"><span class="provsel">Claude · cloud</span>Ask, or “guide me through…”<span class="grow"></span><span style="color:var(--acc)">↵</span></div></div>` },

  details: { title: 'Details · Provenance', icon: 'note', body: () => `<div class="det selectable" style="height:100%">
    <div><div class="dk">Signature</div><span class="mono">uint64_t crc32_z(uint64_t, void*, uint64_t)</span></div>
    <div><div class="dk">Callers</div><div class="xr"><span class="fnc mono">crc32</span> <span style="color:var(--tx2)">→ tail-call</span><span class="hit">live 1,204</span></div><div class="xr"><span class="fnc mono">compress2</span> <span style="color:var(--tx2)">→ 0x14e6</span><span class="hit">live 3</span></div></div>
    <div><div class="dk">Provenance — line 8 ◆</div><div style="color:var(--tx1);font-size:.78rem;line-height:1.5">Written by <span class="mono" style="color:var(--live)">and eax, 0x7</span> @ <span class="mono">0x1000015cb</span> · watchpoint hit <span class="mono" style="color:var(--live)">1,204×</span></div></div></div>` },

  output: { title: 'Output · n0x console', icon: 'disasm', body: () => `<div class="selectable" style="padding:8px 0;height:100%;overflow:auto"><div class="oline"><span class="p">›</span> decomp pseudo --addr 0x1510 --style structured</div><div class="oline" style="color:var(--ok)">ok · 19 lines · schema n0x.decomp.pseudo.v1 · 7 ms</div><div class="oline"><span class="p">›</span> sig apply --flirt zlib-1.3.1.npat</div><div class="oline">named 118 functions · <span style="color:var(--live)">crc32_z, compress, adler32 …</span></div></div>` },

  registers: { title: 'Registers · thread 0', icon: 'regs', body: () => `<div class="regs selectable" style="height:100%">${[
      ['rax','00000064',''],['rbx','7ff6c21a40','ch'],['rcx','00000000',''],['rdx','00000057',''],
      ['rsi','7ff6c218e0',''],['rdi','0000002a',''],['rsp','00cff9e0',''],['rip','7ff6c1002a','ch'],
    ].map(([n,v,c])=>`<div class="reg"><span class="rn">${n}</span><span class="rv${c?' ch':''}">${v}</span></div>`).join('')}</div>` },

  watchpoints: { title: 'Watchpoints & breakpoints', icon: 'watch', body: () => `<div style="overflow:auto;height:100%">
    <div class="wprow" data-ctx="wprow"><span class="wpi">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C21A40</div><div style="color:var(--tx2);font-size:.68rem">write · health</div></div><span class="hit">1,204</span></div>
    <div class="wprow" data-ctx="wprow"><span class="wpi" style="color:var(--acc)">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C218E0</div><div style="color:var(--tx2);font-size:.68rem">read · ammo</div></div><span class="hit" style="color:var(--acc);background:rgba(63,220,196,.1)">86</span></div>
    <div class="wprow" data-ctx="wprow"><span class="wpi" style="color:var(--dgr)">${svg(ICON.bp,'')}</span><div><div class="mono">7FF6C1002A</div><div style="color:var(--tx2);font-size:.68rem">execute · sub_1002A</div></div><span class="hit" style="color:var(--dgr);background:rgba(255,107,122,.1)">hit</span></div></div>` },

  scanner: { title: 'Memory scanner', icon: 'scan', body: () => `<div class="wfill">
    <div class="scanbar"><div class="field"><input class="inp" value="87"><span class="selbox">4-byte int ▾</span><span class="selbox">exact ▾</span></div>
    <div class="field"><button class="b p" style="flex:1">Next scan</button><button class="b g" style="flex:1">New scan</button><button class="b g">Undo</button></div>
    <div style="color:var(--tx2);font-size:.75rem">Scan 4 of 5 · 2,481,003 → 3 candidates · value 100 → 87</div></div>
    <div style="overflow:auto;flex:1">${[['7FF6C21A40','87','100','on'],['7FF6C21A44','87','100',''],['02A9F0E8','87','92','']].map(([a,v,p,o])=>`<div class="srow ${o}" data-ctx="srow"><span class="saddr">${a}</span><span class="sval"${o?' style="color:var(--live)"':''}>${v}</span><span class="sprev">${p}</span></div>`).join('')}</div></div>` },

  livemem: { title: 'Live memory · 7FF6C21A40', icon: 'hex', body: () => `<div class="hex selectable" style="height:100%">
    <div class="hx"><span class="hxa">7FF6C21A40</span><span class="hxb"><span class="hb-hi">57 00 00 00</span> 2a 00 00 00 5c 21 c2 f6 ff 7f 00 00</span><span class="hxc">W....*...\\!......</span></div>
    <div class="hx"><span class="hxa">7FF6C21A50</span><span class="hxb">64 00 00 00 00 00 80 3f 00 00 80 3f cd cc 4c 3e</span><span class="hxc">d......?...?..L&gt;</span></div>
    <div class="hx"><span class="hxa">7FF6C21A60</span><span class="hxb">10 27 00 00 e8 03 00 00 01 00 00 00 00 00 00 00</span><span class="hxc">.'..............</span></div>
    <div class="hx"><span class="hxa">7FF6C21A80</span><span class="hxb">48 65 61 6c 74 68 43 6f 6d 70 6f 6e 65 6e 74 00</span><span class="hxc" style="color:var(--ok)">HealthComponent.</span></div></div>` },

  watchlist: { title: 'Watchlist', icon: 'watch', body: () => `<div style="overflow:auto;height:100%">${[
      ['health','87','on'],['max_health','100','on'],['ammo','42',''],['gold','10000','']
    ].map(([n,v,f])=>`<div class="wlrow" data-ctx="wlrow"><span class="frz ${f}"></span><span class="wname">${n}</span><span class="wval">${v}</span></div>`).join('')}</div>` },

  copilotFollow: { title: 'Copilot · following', icon: 'chat', body: () => `<div class="wfill">
    <div style="padding:11px;font-size:.82rem;line-height:1.5;flex:1;overflow:auto">Found it. <span class="mono" style="color:var(--live)">7FF6C21A40</span> is <b>health</b> — I set a write-watchpoint and traced the writer to <span class="mono" style="color:var(--acc)">HealthComponent::TakeDamage</span>.
    <div style="margin-top:8px;color:var(--vio);font-size:.72rem"><div>→ scan 87 (4-byte)</div><div>→ watch write 7FF6C21A40</div><div>→ provenance → decompile</div></div></div>
    <div class="ask" style="margin:0 10px 10px"><span class="provsel">Local · Ollama</span>Ask…<span class="grow"></span><span style="color:var(--vio)">↵</span></div></div>` },

  strings: { title: 'Strings', icon: 'strings', body: () => `<div style="overflow:auto;height:100%;padding:6px 0">${['HealthComponent','TakeDamage','crc_table','zlib 1.3.1','deflate','Assertion failed'].map(s=>`<div class="frow"><span class="nm mono selectable">"${s}"</span></div>`).join('')}</div>` },

  notes: { title: 'Notes', icon: 'note', body: () => `<textarea class="selectable" style="width:100%;height:100%;background:transparent;border:none;color:var(--tx0);padding:11px;font:inherit;resize:none;outline:none;box-sizing:border-box" placeholder="Session notes…"></textarea>` },
};

// Default workspace layouts — trees over the same widget catalog.
const L = (...tabs) => ({ t: 'leaf', tabs });
const R = (ra, a, rb, b) => ({ t: 'split', dir: 'row', ratio: [ra, rb], kids: [a, b] });
const C = (ca, a, cb, b) => ({ t: 'split', dir: 'col', ratio: [ca, cb], kids: [a, b] });
const DEFAULT_LAYOUTS = {
  decompile: C(0.82,
    R(0.2, L('functions'),
      0.8, R(0.66, C(0.68, L('decompiler'), 0.32, L('disassembly')),
                 0.34, C(0.58, L('copilot'), 0.42, L('details')))),
    0.18, L('output')),
  static: R(0.22, L('functions'),
    0.78, R(0.6, C(0.7, L('decompiler'), 0.3, L('disassembly')), 0.4, L('strings'))),
  graph: L('graph'),
  dynamic: R(0.34, C(0.4, L('registers'), 0.6, L('watchpoints')),
    0.66, R(0.5, C(0.55, L('scanner'), 0.45, L('livemem')),
               0.5, C(0.5, L('watchlist'), 0.5, L('copilotFollow')))),
};

// tiling dock instance (Blender/AreaKit-style: split · tabs · resize · undo)
const dockspace = $('#dockspace'), wpal = $('#wpal');
const dock = initDock({ container: dockspace, widgets: WIDGETS, ICON, svg, toast, showMenu: openCtx, defaults: DEFAULT_LAYOUTS });

// widget palette — click adds to the largest area; drag to place precisely
function openWpal(anchor) {
  const list = $('#wpal-list');
  list.innerHTML = Object.entries(WIDGETS).map(([k, d]) =>
    `<div class="wpi" data-k="${k}"><span class="wpc">${svg(ICON[d.icon] || ICON.decomp,'')}</span><div><div class="wpt">${d.title}</div><div class="wpd">click to add · drag to place</div></div></div>`
  ).join('');
  wpal.classList.add('on');
  const r = (anchor || $('#btn-addw')).getBoundingClientRect();
  const pr = wpal.getBoundingClientRect();
  wpal.style.left = clamp(r.left, 8, innerWidth - pr.width - 8) + 'px';
  wpal.style.top = (r.bottom + 6) + 'px';
  // pointerdown arms a drag; a release without movement is treated as a click-add by the dock
  list.onpointerdown = e => { const el = e.target.closest('.wpi'); if (!el) return; $('#launcher').classList.add('hidden'); closeWpal(); dock.startPaletteDrag(e, el.dataset.k); };
}
function closeWpal() { wpal.classList.remove('on'); }
function resetDock() { dock.reset(); }
$('#btn-addw').addEventListener('click', e => { e.stopPropagation(); wpal.classList.contains('on') ? closeWpal() : openWpal($('#btn-addw')); });
window.addEventListener('pointerdown', e => { if (!e.target.closest('#wpal,#btn-addw')) closeWpal(); }, true);

// undo / redo — buttons + Ctrl+Shift+Z / Ctrl+Shift+X
$('#btn-undo')?.addEventListener('click', () => dock.undo());
$('#btn-redo')?.addEventListener('click', () => dock.redo());
window.addEventListener('keydown', e => {
  if (!(e.ctrlKey || e.metaKey) || !e.shiftKey) return;
  const k = e.key.toLowerCase();
  if (k === 'z') { e.preventDefault(); dock.undo(); }
  else if (k === 'x') { e.preventDefault(); dock.redo(); }
});

/* =====================================================================
   DEEP LINK (?ws=graph / #dynamic) — also handy for screenshots
   ===================================================================== */
function deepLink() {
  const p = new URLSearchParams(location.search);
  let ws = p.get('ws') || (location.hash ? location.hash.slice(1) : '');
  const valid = ['decompile', 'static', 'graph', 'dynamic'];
  if (valid.includes(ws)) { openTarget(ws === 'dynamic' ? 'dynamic' : 'static'); setWorkspace(ws); }
  if (p.get('settings') === '1') settings.classList.add('on');
  if (p.get('palette') === '1') openPal();
  if (p.get('widgets') === '1') { openTarget('static'); setWorkspace('decompile'); ['decomp', 'regs', 'copilot', 'notes'].forEach(k => dock.addWidget(k)); }
}

/* =====================================================================
   ENGINE BRIDGE — hydrate from the real n0x backend when native
   ===================================================================== */
async function hydrateFromEngine() {
  if (!isNative) { console.log('web preview — running on built-in demo data'); return; }
  // frameless window controls
  try {
    const appWin = window.__TAURI__.window.getCurrentWindow();
    const winAct = { min: () => appWin.minimize(), max: () => appWin.toggleMaximize(), close: () => appWin.close() };
    $$('[data-win]').forEach(b => b.addEventListener('click', () => winAct[b.dataset.win]?.()));
  } catch (e) { console.warn('window controls unavailable', e); }
  const info = await engineInfo();
  if (info && info.ok) {
    // real engine reachable → show it in the status bar
    const sb = document.createElement('div');
    sb.className = 'sb'; sb.style.color = 'var(--ok)';
    sb.innerHTML = `● engine n0xis ${info.version || ''} · ${info.commandCount || '?'} cmds`;
    $('#statusbar').insertBefore(sb, $('#statusbar .grow'));
    toast('Connected to n0xis engine v' + (info.version || '?'));
  } else {
    toast('Engine not found on PATH — running on demo data');
  }
  // wire the launcher "Analyze a file" card to a real file dialog + engine profile
  const analyzeCard = $('#launcher .card[data-open="static"]');
  if (analyzeCard) {
    analyzeCard.addEventListener('click', async ev => {
      ev.stopImmediatePropagation();
      const path = await pickFile('Choose a binary to analyze');
      if (!path) return;
      openTarget('static');
      $('#target-name').textContent = path.split(/[\\/]/).pop();
      const res = await engine.functions(path);
      renderRealFunctions(res);
    }, true);
  }
}
function renderRealFunctions(res) {
  const list = res?.data?.functions || res?.data?.symbols || res?.data?.items;
  if (!Array.isArray(list) || !list.length) { toast('Engine returned no functions — keeping demo'); return; }
  const fl = $('#dockspace .flist'); if (!fl) { toast('Open the Functions widget to list them'); return; }
  fl.innerHTML = '';
  list.slice(0, 400).forEach(f => {
    const name = f.name || f.symbol || f.label || 'sub_' + (f.addr || f.address || '');
    const addr = f.addr || f.address || f.va || '';
    const row = document.createElement('div');
    row.className = 'frow';
    row.innerHTML = `<span class="sym">ƒ</span><span class="nm mono">${name}</span><span class="fa mono">${addr}</span>`;
    fl.appendChild(row);
  });
  toast('Loaded ' + list.length + ' functions from the engine');
}

deepLink();
dock.setWorkspace(document.documentElement.dataset.ws || 'decompile'); // seed/restore the workspace layout
hydrateFromEngine();
console.log('N0xis GUI ready · graph, context menus, dock widgets, palette (Ctrl+P), themes, zoom');
