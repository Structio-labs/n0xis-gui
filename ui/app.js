// N0xis GUI — interactive shell (framework-free; drops into Tauri's webview as-is)
import { isNative, engineInfo, engine, pickFile } from './bridge.js';
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
  const out = $('#view-decompile .out div');
  if (!out) return;
  const l1 = document.createElement('div'); l1.className = 'oline'; l1.innerHTML = `<span class="p">›</span> ${cmd}`;
  const l2 = document.createElement('div'); l2.className = 'oline'; l2.style.color = 'var(--ok)'; l2.textContent = ok;
  out.append(l1, l2); out.scrollTop = out.scrollHeight;
}

// ---------- workspaces ----------
function setWorkspace(ws) {
  $$('.wsview').forEach(v => v.classList.toggle('on', v.id === 'view-' + ws));
  $$('#wsbar .ws').forEach(t => t.classList.toggle('on', t.dataset.ws === ws));
  document.documentElement.dataset.ws = ws;
  showDockFor(ws);
  if (ws === 'graph') requestAnimationFrame(() => { renderGraph(); fitGraph(); });
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

// ---------- function list selection ----------
$('#flist').addEventListener('click', e => {
  const row = e.target.closest('.frow'); if (!row) return;
  $$('#flist .frow').forEach(r => r.classList.remove('on'));
  row.classList.add('on');
});

// ---------- beginner / pro ----------
let pro = true;
$('#mode-toggle').addEventListener('click', () => {
  pro = !pro;
  $('#mode-label').innerHTML = pro ? 'Beginner / <b>Pro</b>' : '<b>Beginner</b> / Pro';
  toast(pro ? 'Pro mode — full docking &amp; every panel' : 'Beginner mode — guided, simplified layout');
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

// ---------- cosmetic group toggles ----------
function groupToggle(sel, cls = 'on') {
  $$(sel).forEach(g => g.addEventListener('click', () => {
    g.parentElement.querySelectorAll(sel.split(' ').pop()).forEach(x => x.classList.remove(cls));
    g.classList.add(cls);
  }));
}
groupToggle('.dockhead .pill');
groupToggle('.rtabs .rt');
groupToggle('.navtabs .nt');
$$('.dockhead .dt:not([data-ws])').forEach(d => d.addEventListener('click', () => {
  d.parentElement.querySelectorAll('.dt').forEach(x => x.classList.remove('on')); d.classList.add('on');
}));

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
const gv = $('#gviewport'), gc = $('#gcanvas'), gs = $('#gsvg');
const gcam = { x: 40, y: 20, s: 1 };
const rectOf = {};
function renderGraph() {
  if (!gc) return;
  // nodes
  gc.querySelectorAll('.gnode').forEach(n => n.remove());
  GRAPH.blocks.forEach(b => {
    const n = document.createElement('div');
    n.className = 'gnode' + (b.tag === 'entry' ? ' entry' : b.tag === 'exit' ? ' exit' : '');
    n.style.left = b.x + 'px'; n.style.top = b.y + 'px'; n.style.width = b.w + 'px';
    n.dataset.ctx = 'gnode'; n.dataset.addr = b.addr; n.dataset.id = b.id;
    const tag = b.tag && b.tag !== 'entry' && b.tag !== 'exit' ? ' · ' + b.tag : (b.tag ? ' · ' + b.tag : '');
    n.innerHTML = `<div class="gt">${b.id} · ${b.addr}${tag}</div>${b.body}`;
    gc.appendChild(n);
    rectOf[b.id] = { x: b.x, y: b.y, w: b.w, h: n.offsetHeight };
  });
  // edges (measured, so they always meet the boxes)
  const defs = `<defs>
    <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="context-stroke"/>
    </marker></defs>`;
  const paths = GRAPH.edges.map(e => {
    const a = rectOf[e.f], b = rectOf[e.t];
    const cls = 'gedge' + (e.type ? ' ' + e.type : '');
    if (e.type === 'loop') {
      const x = a.x + a.w, y1 = a.y + a.h * 0.28, y2 = a.y + a.h * 0.72;
      return `<path class="${cls}" marker-end="url(#arrow)" d="M${x} ${y1} C${x + 46} ${y1 - 6},${x + 46} ${y2 + 6},${x} ${y2}"/>`;
    }
    const sx = a.x + a.w / 2, sy = a.y + a.h, tx = b.x + b.w / 2, ty = b.y;
    const my = (sy + ty) / 2;
    return `<path class="${cls}" marker-end="url(#arrow)" d="M${sx} ${sy} C${sx} ${my},${tx} ${my},${tx} ${ty - 2}"/>`;
  }).join('');
  gs.innerHTML = defs + paths;
}
function applyCam() {
  gc.style.transform = `translate(${gcam.x}px,${gcam.y}px) scale(${gcam.s})`;
  const l = $('#gz-lvl'); if (l) l.textContent = Math.round(gcam.s * 100) + '%';
}
function fitGraph() {
  if (!gv) return;
  const vw = gv.clientWidth, vh = gv.clientHeight;
  let maxX = 0, maxY = 0;
  GRAPH.blocks.forEach(b => { maxX = Math.max(maxX, b.x + b.w); maxY = Math.max(maxY, b.y + (rectOf[b.id]?.h || 70)); });
  gcam.s = clamp(Math.min(vw / (maxX + 60), vh / (maxY + 60)), 0.4, 1.3);
  gcam.x = (vw - maxX * gcam.s) / 2;
  gcam.y = Math.max(16, (vh - maxY * gcam.s) / 2);
  applyCam();
}
if (gv) {
  let pan = null;
  gv.addEventListener('pointerdown', e => {
    if (e.target.closest('.gzoom')) return;
    pan = { x: e.clientX, y: e.clientY, cx: gcam.x, cy: gcam.y };
    gv.classList.add('panning'); gv.setPointerCapture(e.pointerId);
  });
  gv.addEventListener('pointermove', e => {
    if (!pan) return;
    gcam.x = pan.cx + (e.clientX - pan.x); gcam.y = pan.cy + (e.clientY - pan.y); applyCam();
  });
  gv.addEventListener('pointerup', e => { pan = null; gv.classList.remove('panning'); });
  gv.addEventListener('wheel', e => {
    e.preventDefault();
    const r = gv.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top;
    const ns = clamp(gcam.s * (e.deltaY < 0 ? 1.12 : 0.89), 0.3, 2.4);
    gcam.x = mx - (mx - gcam.x) * (ns / gcam.s); gcam.y = my - (my - gcam.y) * (ns / gcam.s);
    gcam.s = ns; applyCam();
  }, { passive: false });
  $('#gz-in').addEventListener('click', () => { gcam.s = clamp(gcam.s * 1.15, 0.3, 2.4); applyCam(); });
  $('#gz-out').addEventListener('click', () => { gcam.s = clamp(gcam.s * 0.87, 0.3, 2.4); applyCam(); });
  $('#gz-fit').addEventListener('click', fitGraph);
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
   BLENDER-STYLE DOCK — floating widgets + add-widget palette
   ===================================================================== */
const dockspace = $('#dockspace'), snaphint = $('#snaphint'), wpal = $('#wpal');
const WIDGETS = {
  decomp:   { title: 'Decompiler',      icon: 'decomp',  w: 460, h: 300, body: () => `<div class="code selectable" style="padding:10px 0"><div class="cl"><span class="gut">1</span><span><span class="ty">uint64_t</span> <span class="fnc">crc32_z</span>(<span class="ty">uint64_t</span>, <span class="ty">void</span> *, <span class="ty">uint64_t</span>) {</span></div><div class="cl"><span class="gut">2</span><span>&nbsp;&nbsp;<span class="k">return</span> ~v14;</span></div><div class="cl"><span class="gut">3</span><span>}</span></div></div>` },
  disasm:   { title: 'Disassembly',     icon: 'disasm',  w: 380, h: 220, body: () => `<div class="disasm" style="height:100%">${['48 89 d8|mov|rax, rbx','83 e0 07|and|eax, 0x7','74 2a|je|0x15fa','0f b6 03|movzx|eax, [rbx]'].map(r=>{const[b,m,o]=r.split('|');return `<div class="drow" data-ctx="drow"><span class="daddr">0x1000015c8</span><span class="dbytes">${b}</span><span class="dmn">${m}</span><span>${o}</span></div>`;}).join('')}</div>` },
  hex:      { title: 'Hex / memory',    icon: 'hex',     w: 420, h: 190, body: () => `<div class="hex" style="height:100%"><div class="hx"><span class="hxa">7FF6C21A40</span><span class="hxb"><span class="hb-hi">57 00 00 00</span> 2a 00 00 00 5c 21 c2 f6</span><span class="hxc">W....*...\\!</span></div><div class="hx"><span class="hxa">7FF6C21A50</span><span class="hxb">64 00 00 00 00 00 80 3f 00 00 80 3f</span><span class="hxc">d......?...?</span></div></div>` },
  regs:     { title: 'Registers',       icon: 'regs',    w: 260, h: 180, body: () => `<div class="regs" style="height:100%">${['rax|00000064','rbx|7ff6c21a40 ch','rcx|00000000','rdx|00000057','rsi|7ff6c218e0','rdi|0000002a'].map(r=>{const[n,v,c]=r.split(' ');const[rn,rv]=n.split('|');return `<div class="reg"><span class="rn">${rn}</span><span class="rv${c?' ch':''}">${rv}</span></div>`;}).join('')}</div>` },
  watch:    { title: 'Watchpoints',     icon: 'watch',   w: 300, h: 170, body: () => `<div style="overflow:auto;height:100%"><div class="wprow" data-ctx="wprow"><span class="wpi">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C21A40</div><div style="color:var(--tx2);font-size:.68rem">write · health</div></div><span class="hit">1,204</span></div></div>` },
  scan:     { title: 'Memory scanner',  icon: 'scan',    w: 320, h: 200, body: () => `<div class="scanbar"><div class="field"><input class="inp" value="87"><span class="selbox">4-byte ▾</span></div><div class="field"><button class="b p" style="flex:1">Next scan</button><button class="b g" style="flex:1">New scan</button></div></div><div class="srow on" data-ctx="srow"><span class="saddr">7FF6C21A40</span><span class="sval" style="color:var(--live)">87</span><span class="sprev">100</span></div>` },
  copilot:  { title: 'Copilot',         icon: 'chat',    w: 320, h: 240, body: () => `<div class="chat" style="height:100%"><div class="msg"><div class="av ai">AI</div><div class="bub">Ask me to explain, rename, or trace a value. I can drive n0x for you.</div></div></div>` },
  strings:  { title: 'Strings',         icon: 'strings', w: 300, h: 200, body: () => `<div style="overflow:auto;height:100%;padding:6px 0">${['HealthComponent','TakeDamage','crc_table','zlib 1.3.1'].map(s=>`<div class="frow"><span class="nm mono selectable">"${s}"</span></div>`).join('')}</div>` },
  notes:    { title: 'Notes',           icon: 'note',    w: 300, h: 180, body: () => `<textarea class="selectable" style="width:100%;height:100%;background:transparent;border:none;color:var(--tx0);padding:11px;font:inherit;resize:none;outline:none;box-sizing:border-box" placeholder="Session notes…"></textarea>` },
  output:   { title: 'Output console',  icon: 'disasm',  w: 400, h: 160, body: () => `<div style="padding:8px 0;height:100%;overflow:auto"><div class="oline"><span class="p">›</span> decomp pseudo --addr 0x1510</div><div class="oline" style="color:var(--ok)">ok · 19 lines · 7 ms</div></div>` },
};
let widgetZ = 42, widgetSeq = 0;
function spawnWidget(key, opt = {}) {
  const def = WIDGETS[key]; if (!def) return;
  $('#launcher').classList.add('hidden');
  const el = document.createElement('div');
  el.className = 'widget'; el.dataset.ws = document.documentElement.dataset.ws || 'decompile';
  const dr = dockspace.getBoundingClientRect();
  const w = opt.w || def.w, h = opt.h || def.h;
  el.style.width = w + 'px'; el.style.height = h + 'px';
  el.style.left = (opt.x ?? (40 + (widgetSeq % 5) * 34)) + 'px';
  el.style.top = (opt.y ?? (40 + (widgetSeq % 5) * 30)) + 'px';
  el.style.zIndex = ++widgetZ; widgetSeq++;
  el.innerHTML =
    `<div class="whead"><span class="wtitle"><span class="wico">${svg(ICON[def.icon] || ICON.decomp,'')}</span>${def.title}</span>` +
    `<span class="grow" style="flex:1"></span>` +
    `<button class="wbtn wclose" title="Close">${svg('M4 4l8 8M12 4l-8 8','')}</button></div>` +
    `<div class="wbody">${def.body()}</div><div class="wgrip"></div>`;
  dockspace.appendChild(el);
  focusWidget(el);
  makeDraggable(el);
  el.querySelector('.wclose').addEventListener('click', () => { el.remove(); refreshDockEmpty(); });
  refreshDockEmpty();
  return el;
}
function focusWidget(el) {
  $$('.widget', dockspace).forEach(w => w.classList.remove('focus'));
  el.classList.add('focus'); el.style.zIndex = ++widgetZ;
}
function makeDraggable(el) {
  const head = el.querySelector('.whead'), grip = el.querySelector('.wgrip');
  el.addEventListener('pointerdown', () => focusWidget(el), true);
  // drag
  head.addEventListener('pointerdown', e => {
    if (e.target.closest('.wbtn')) return;
    e.preventDefault();
    const dr = dockspace.getBoundingClientRect();
    const sx = e.clientX, sy = e.clientY, ox = el.offsetLeft, oy = el.offsetTop;
    head.setPointerCapture(e.pointerId);
    let snap = null;
    const move = ev => {
      let nx = clamp(ox + ev.clientX - sx, 0, dr.width - el.offsetWidth);
      let ny = clamp(oy + ev.clientY - sy, 0, dr.height - el.offsetHeight);
      el.style.left = nx + 'px'; el.style.top = ny + 'px';
      snap = edgeSnap(ev.clientX - dr.left, ev.clientY - dr.top, dr);
    };
    const up = ev => {
      head.releasePointerCapture(e.pointerId);
      window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up);
      snaphint.classList.remove('on');
      if (snap) { el.style.left = snap.x + 'px'; el.style.top = snap.y + 'px'; el.style.width = snap.w + 'px'; el.style.height = snap.h + 'px'; }
    };
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
  });
  // resize
  grip.addEventListener('pointerdown', e => {
    e.preventDefault(); e.stopPropagation();
    const sx = e.clientX, sy = e.clientY, ow = el.offsetWidth, oh = el.offsetHeight;
    grip.setPointerCapture(e.pointerId);
    const move = ev => {
      el.style.width = clamp(ow + ev.clientX - sx, 180, 1400) + 'px';
      el.style.height = clamp(oh + ev.clientY - sy, 110, 1000) + 'px';
    };
    const up = () => { grip.releasePointerCapture(e.pointerId); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); };
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
  });
}
function edgeSnap(mx, my, dr) {
  const T = 46, W = dr.width, H = dr.height;
  let s = null;
  if (my < T) s = { x: 0, y: 0, w: W, h: H / 2 };          // top → top half
  else if (my > H - T) s = { x: 0, y: H / 2, w: W, h: H / 2 }; // bottom half
  else if (mx < T) s = { x: 0, y: 0, w: W / 2, h: H };      // left half
  else if (mx > W - T) s = { x: W / 2, y: 0, w: W / 2, h: H }; // right half
  if (s) {
    const b = dockspace.getBoundingClientRect();
    snaphint.style.left = (b.left + s.x) + 'px'; snaphint.style.top = (b.top + s.y) + 'px';
    snaphint.style.width = s.w + 'px'; snaphint.style.height = s.h + 'px';
    snaphint.classList.add('on');
  } else snaphint.classList.remove('on');
  return s;
}
function showDockFor(ws) {
  $$('.widget', dockspace).forEach(w => { w.style.display = w.dataset.ws === ws ? '' : 'none'; });
  refreshDockEmpty();
}
function refreshDockEmpty() {
  const anyVisible = $$('.widget', dockspace).some(w => w.style.display !== 'none' && w.dataset.ws === (document.documentElement.dataset.ws || 'decompile'));
  dockspace.classList.toggle('empty', false); // empty hint only when explicitly in dock mode; keep off to avoid covering workspaces
  dockspace.style.pointerEvents = anyVisible ? 'none' : 'none'; // container passthrough; widgets capture their own events
}
function resetDock() {
  $$('.widget', dockspace).forEach(w => w.remove());
  toast('Layout reset');
}
// widget palette
function openWpal(anchor) {
  const list = $('#wpal-list');
  list.innerHTML = Object.entries(WIDGETS).map(([k, d]) =>
    `<div class="wpi" data-k="${k}"><span class="wpc">${svg(ICON[d.icon] || ICON.decomp,'')}</span><div><div class="wpt">${d.title}</div><div class="wpd">Floating · dockable</div></div></div>`
  ).join('');
  wpal.classList.add('on');
  const r = (anchor || $('#btn-addw')).getBoundingClientRect();
  const pr = wpal.getBoundingClientRect();
  wpal.style.left = clamp(r.left, 8, innerWidth - pr.width - 8) + 'px';
  wpal.style.top = (r.bottom + 6) + 'px';
  list.onclick = e => { const el = e.target.closest('.wpi'); if (!el) return; spawnWidget(el.dataset.k); closeWpal(); };
}
function closeWpal() { wpal.classList.remove('on'); }
$('#btn-addw').addEventListener('click', e => { e.stopPropagation(); wpal.classList.contains('on') ? closeWpal() : openWpal($('#btn-addw')); });
window.addEventListener('pointerdown', e => { if (!e.target.closest('#wpal,#btn-addw')) closeWpal(); }, true);

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
  if (p.get('widgets') === '1') { openTarget('static'); setWorkspace('decompile'); spawnWidget('regs', { x: 60, y: 60 }); spawnWidget('copilot', { x: 360, y: 120 }); spawnWidget('notes', { x: 720, y: 80 }); }
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
  const fl = $('#flist'); fl.innerHTML = '';
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

renderGraph();
deepLink();
hydrateFromEngine();
console.log('N0xis GUI ready · graph, context menus, dock widgets, palette (Ctrl+P), themes, zoom');
