// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

// N0xis GUI — interactive shell (framework-free; drops into Tauri's webview as-is)
import { isNative, engineInfo, engine, pickFile, pickFolder, n0x, initialTarget, listProcesses, processIcons, fncacheGet, fncachePut, sessionOpen, sessionQuery, sessionClose, analyzeStart, analyzeStatus, cacheInfo, clearCache, setDiscardOnClose } from './bridge.js';
import { initDock } from './dock.js';
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

// ---------- tiny icon set (shared by menus / widgets) ----------
// Icon set — Lucide (ISC license, https://lucide.dev), inlined as 24×24 markup.
const ICON = {
  decomp: "<path d='m16 18 6-6-6-6' /> <path d='m8 6-6 6 6 6' />",
  disasm: "<path d='M3 5h.01' /> <path d='M3 12h.01' /> <path d='M3 19h.01' /> <path d='M8 5h13' /> <path d='M8 12h13' /> <path d='M8 19h13' />",
  graph: "<path d='M15 6a9 9 0 0 0-9 9V3' /> <circle cx='18' cy='6' r='3' /> <circle cx='6' cy='18' r='3' />",
  rename: "<path d='M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z' /> <path d='m15 5 4 4' />",
  xref: "<path d='M8 3 4 7l4 4' /> <path d='M4 7h16' /> <path d='m16 21 4-4-4-4' /> <path d='M20 17H4' />",
  type: "<path d='M12 4v16' /> <path d='M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2' /> <path d='M9 20h6' />",
  bp: "<circle cx='12' cy='12' r='1' /> <circle cx='12' cy='12' r='10' />",
  watch: "<path d='M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0' /> <circle cx='12' cy='12' r='3' />",
  copy: "<rect width='14' height='14' x='8' y='8' rx='2' ry='2' /> <path d='M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2' />",
  freeze: "<path d='m10 20-1.25-2.5L6 18' /> <path d='M10 4 8.75 6.5 6 6' /> <path d='m14 20 1.25-2.5L18 18' /> <path d='m14 4 1.25 2.5L18 6' /> <path d='m17 21-3-6h-4' /> <path d='m17 3-3 6 1.5 3' /> <path d='M2 12h6.5L10 9' /> <path d='m20 10-1.5 2 1.5 2' /> <path d='M22 12h-6.5L14 15' /> <path d='m4 10 1.5 2L4 14' /> <path d='m7 21 3-6-1.5-3' /> <path d='m7 3 3 6h4' />",
  trash: "<path d='M10 11v6' /> <path d='M14 11v6' /> <path d='M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6' /> <path d='M3 6h18' /> <path d='M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2' />",
  scan: "<path d='m21 21-4.34-4.34' /> <circle cx='11' cy='11' r='8' />",
  play: "<path d='M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z' />",
  hex: "<rect x='14' y='14' width='4' height='6' rx='2' /> <rect x='6' y='4' width='4' height='6' rx='2' /> <path d='M6 20h4' /> <path d='M14 10h4' /> <path d='M6 14h2v6' /> <path d='M14 4h2v6' />",
  regs: "<path d='M12 3v18' /> <rect width='18' height='18' x='3' y='3' rx='2' /> <path d='M3 9h18' /> <path d='M3 15h18' />",
  chat: "<path d='M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z' />",
  note: "<path d='M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z' /> <path d='M14 2v5a1 1 0 0 0 1 1h5' /> <path d='M10 9H8' /> <path d='M16 13H8' /> <path d='M16 17H8' />",
  add: "<path d='M5 12h14' /> <path d='M12 5v14' />",
  reset: "<path d='M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8' /> <path d='M3 3v5h5' />",
  settings: "<path d='M10 5H3' /> <path d='M12 19H3' /> <path d='M14 3v4' /> <path d='M16 17v4' /> <path d='M21 12h-9' /> <path d='M21 19h-5' /> <path d='M21 5h-7' /> <path d='M8 10v4' /> <path d='M8 12H3' />",
  strings: "<path d='M12 4v16' /> <path d='M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2' /> <path d='M9 20h6' />",
  fold: "<path d='m6 9 6 6 6-6' />",
  x: "<path d='M18 6 6 18' /> <path d='m6 6 12 12' />",
  follow: "<path d='M4 16v-2.38C4 11.5 2.97 10.5 3 8c.03-2.72 1.49-6 4.5-6C9.37 2 10 3.8 10 5.5c0 3.11-2 5.66-2 8.68V16a2 2 0 1 1-4 0Z' /> <path d='M20 20v-2.38c0-2.12 1.03-3.12 1-5.62-.03-2.72-1.49-6-4.5-6C14.63 6 14 7.8 14 9.5c0 3.11 2 5.66 2 8.68V20a2 2 0 1 0 4 0Z' /> <path d='M16 17h4' /> <path d='M4 13h4' />",
  open: "<path d='m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2' />",
  comment: "<path d='M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z' /> <path d='M12 11h.01' /> <path d='M16 11h.01' /> <path d='M8 11h.01' />",
  bug: "<path d='M12 20v-9' /> <path d='M14 7a4 4 0 0 1 4 4v3a6 6 0 0 1-12 0v-3a4 4 0 0 1 4-4z' /> <path d='M14.12 3.88 16 2' /> <path d='M21 21a4 4 0 0 0-3.81-4' /> <path d='M21 5a4 4 0 0 1-3.55 3.97' /> <path d='M22 13h-4' /> <path d='M3 21a4 4 0 0 1 3.81-4' /> <path d='M3 5a4 4 0 0 0 3.55 3.97' /> <path d='M6 13H2' /> <path d='m8 2 1.88 1.88' /> <path d='M9 7.13V6a3 3 0 1 1 6 0v1.13' />",
  redo: "<path d='m15 14 5-5-5-5' /> <path d='M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13' />",
  bookmark: "<path d='m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z' />",
  undo: "<path d='M9 14 4 9l5-5' /> <path d='M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11' />",
  step: "<path d='m6 17 5-5-5-5' /> <path d='m13 17 5-5-5-5' />",
  min: "<path d='M5 12h14' />",
  max: "<path d='M15 3h6v6' /> <path d='m21 3-7 7' /> <path d='m3 21 7-7' /> <path d='M9 21H3v-6' />",
  identify: "<path d='m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72' /> <path d='m14 7 3 3' /> <path d='M5 6v4' /> <path d='M19 14v4' /> <path d='M10 2v2' /> <path d='M7 8H3' /> <path d='M21 16h-4' /> <path d='M11 3H9' />",
  patch: "<path d='m3 7 3 3 3-3' /> <path d='M6 10V5a2 2 0 0 1 2-2h2' /> <rect x='3' y='14' width='7' height='7' rx='1' />",
  about: "<circle cx='12' cy='12' r='10' /> <path d='M12 16v-4' /> <path d='M12 8h.01' />",
  book: "<path d='M12 7v14' /> <path d='M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z' />",
  term: "<path d='m4 17 6-6-6-6' /> <path d='M12 19h8' />",
  chip: "<path d='M12 20v2' /> <path d='M12 2v2' /> <path d='M17 20v2' /> <path d='M17 2v2' /> <path d='M2 12h2' /> <path d='M2 17h2' /> <path d='M2 7h2' /> <path d='M20 12h2' /> <path d='M20 17h2' /> <path d='M20 7h2' /> <path d='M7 20v2' /> <path d='M7 2v2' /> <rect x='4' y='4' width='16' height='16' rx='2' /> <rect x='8' y='8' width='8' height='8' rx='1' />",
};
const svg = (d, cls = 'cxi') =>
  `<span class="${cls}"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${
    d.includes('<') ? d : d.split(';').map(p => `<path d="${p}"/>`).join('')}</svg></span>`;

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
  document.documentElement.dataset.workspace = ws; // state attr — NOT data-ws (that selects jump buttons)
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
  recordRecent($('#target-name')?.textContent, kind); // remember for Open Recent
}
// ---------- launcher ----------
$$('#launcher .card').forEach(card => card.addEventListener('click', () => {
  if (card.dataset.open === 'dynamic' || card.dataset.open === 'both') pickProcess();
  else openTarget(card.dataset.open);
}));
$('#btn-live').addEventListener('click', () => openTarget('dynamic'));

// ---------- delegated widget interactions (widgets are created dynamically) ----------
// the currently-selected symbol — Edit/Analyze actions operate on this address
let selAddr = '0x100001510', selName = 'crc32_z';
$('#dockspace').addEventListener('click', e => {
  // workspace jump buttons (e.g. the decompiler's "CFG ↗")
  const jump = e.target.closest('[data-ws]');
  if (jump) { setWorkspace(jump.dataset.ws); return; }
  // function / string list selection (single-select within the same list)
  const row = e.target.closest('.frow');
  if (row) {
    row.parentElement?.querySelectorAll(':scope > .frow.on').forEach(r => r.classList.remove('on')); row.classList.add('on');
    const a = row.querySelector('.fa')?.textContent, n = row.querySelector('.nm')?.textContent;
    if (a) { selAddr = a.trim(); selName = (n || '').trim(); onSymbolSelect(); }   // track + follow the current symbol
    return;
  }
  // decompiler style pills → re-decompile with that style
  const pill = e.target.closest('.pill[data-style]');
  if (pill) { pill.parentElement.querySelectorAll('.pill').forEach(x => x.classList.remove('on')); pill.classList.add('on'); decompStyle = pill.dataset.style; loadDecomp(); return; }
  // cosmetic single-select groups inside a widget
  for (const sel of ['.nt', '.rt', '.pill', '.dt']) {
    const g = e.target.closest(sel);
    if (g && !g.hasAttribute('data-ws')) { g.parentElement.querySelectorAll(sel).forEach(x => x.classList.remove('on')); g.classList.add('on'); return; }
  }
  // give every other actionable control feedback (Copilot suggestions, scanner buttons, dropdowns…)
  const act = e.target.closest('.sugb, .b, .selbox, .cbtn, .provsel');
  if (act) { const t = act.textContent.trim(); if (act.classList.contains('b')) echo('scan ' + t.toLowerCase(), 'ok · 3 candidates'); else toast(t); return; }
  // freeze toggle in the watchlist
  const frz = e.target.closest('.frz'); if (frz) { frz.classList.toggle('on'); return; }
});
// live filter for the function list — filters the DATA, so it works with the
// virtualized list and matches functions that aren't currently rendered.
$('#dockspace').addEventListener('input', e => {
  const f = e.target.closest('.ffilter'); if (!f) return;
  funcQuery = f.value;
  paintFunctions();
});

// ---------- shell buttons (launcher links, new-workspace, target chip) ----------
$$('#launcher a').forEach(a => a.addEventListener('click', () => toast(a.textContent.trim())));
$('#wsbar .ws[style]')?.addEventListener('click', () => toast('New workspace — build your own from + Widget')); // the "+" tab
$('#target-chip')?.addEventListener('click', e => { const r = $('#target-chip').getBoundingClientRect(); openCtx(r.left, r.bottom + 4, menuItems('File')); });

// ---------- lightweight prefs persistence ----------
const PREF = 'n0xis.prefs.v1';
const prefs = (() => { try { return JSON.parse(localStorage.getItem(PREF)) || {}; } catch { return {}; } })();
const savePrefs = () => { try { localStorage.setItem(PREF, JSON.stringify(prefs)); } catch {} };

// ---------- zoom ----------
let zoom = 1;
function setZoom(z) {
  zoom = clamp(Math.round(z * 100) / 100, 0.7, 1.6);
  document.documentElement.style.setProperty('--zoom', zoom);
  $('#sb-zoom').textContent = 'zoom ' + Math.round(zoom * 100) + '%';
  $$('#scale-seg .sg').forEach(b => b.classList.toggle('on', Math.abs(parseFloat(b.dataset.scale) - zoom) < 0.001));
  prefs.zoom = zoom; savePrefs();
}
$$('#scale-seg .sg').forEach(b => b.addEventListener('click', () => setZoom(parseFloat(b.dataset.scale))));
// (zoom / palette / layout keys are all handled by the data-driven dispatcher below)

// ---------- theme (live preview, persisted) ----------
function applyTheme(th) {
  if (th) document.documentElement.setAttribute('data-theme', th);
  else document.documentElement.removeAttribute('data-theme');
  $$('#theme-grid .th').forEach(x => x.classList.toggle('on', (x.dataset.theme || '') === (th || '')));
  prefs.theme = th || ''; savePrefs();
}
$$('#theme-grid .th').forEach(t => t.addEventListener('click', () => applyTheme(t.dataset.theme)));
// restore saved prefs
if (prefs.theme) applyTheme(prefs.theme);
if (prefs.zoom) setZoom(prefs.zoom);

// ---------- keybindings: data-driven, editable, persisted (prefs.keys) ----------
// KEYDEFS is the single source of truth: the Settings editor renders it, the
// runtime dispatcher reads it, and per-command overrides live in prefs.keys.
if (!prefs.keys) prefs.keys = {};
const DBG = 'Debug (live target)';
const KEYDEFS = [
  { id: 'palette', label: 'Command palette', group: 'General', def: 'Ctrl+Shift+P', scope: 'global', act: () => togglePal() },
  { id: 'palette2', label: 'Command palette · quick', group: 'General', def: 'Ctrl+P', scope: 'global', act: () => togglePal() },
  { id: 'palette3', label: 'Command palette · F1', group: 'General', def: 'F1', scope: 'global', act: () => openPal() },
  { id: 'open', label: 'Open file / process', group: 'General', def: 'Ctrl+O', scope: 'global', act: () => openFileTarget() },
  { id: 'goto', label: 'Go to address / symbol', group: 'General', def: 'Ctrl+G', scope: 'target', act: () => doGoto() },
  { id: 'find', label: 'Find in image (bytes / string)', group: 'General', def: 'Ctrl+F', scope: 'target', act: () => openFind() },
  { id: 'navback', label: 'Navigate back', group: 'General', def: 'Alt+ArrowLeft', scope: 'target', act: () => navBackward() },
  { id: 'navfwd', label: 'Navigate forward', group: 'General', def: 'Alt+ArrowRight', scope: 'target', act: () => navForward() },
  { id: 'settings', label: 'Settings', group: 'General', def: 'Ctrl+,', scope: 'global', act: () => openSettings() },
  { id: 'zoomin', label: 'Zoom in', group: 'General', def: 'Ctrl+=', scope: 'global', act: () => setZoom(zoom + 0.1) },
  { id: 'zoomout', label: 'Zoom out', group: 'General', def: 'Ctrl+-', scope: 'global', act: () => setZoom(zoom - 0.1) },
  { id: 'zoomreset', label: 'Reset zoom', group: 'General', def: 'Ctrl+0', scope: 'global', act: () => setZoom(1) },
  { id: 'closewin', label: 'Close window', group: 'General', def: 'Ctrl+Q', scope: 'global', act: () => winCtl('close') },
  { id: 'decompile', label: 'Decompile / Continue', group: 'Analysis', def: 'F5', scope: 'target', act: () => { if (isLive()) toast('Continue'); else { setWorkspace('decompile'); toast('Decompiling'); } } },
  { id: 'rename', label: 'Rename symbol', group: 'Analysis', def: 'F2', scope: 'target', act: () => doRename() },
  { id: 'comment', label: 'Comment', group: 'Analysis', def: 'Ctrl+/', scope: 'target', act: () => doComment() },
  { id: 'announdo', label: 'Undo rename / comment', group: 'Analysis', def: 'Ctrl+Z', scope: 'target', act: () => annotationUndo() },
  { id: 'annoredo', label: 'Redo rename / comment', group: 'Analysis', def: 'Ctrl+Y', scope: 'target', act: () => annotationRedo() },
  { id: 'xrefs', label: 'Find references (xrefs)', group: 'Analysis', def: 'Shift+F12', scope: 'target', act: () => toast('Xrefs') },
  { id: 'stepover', label: 'Step over', group: DBG, def: 'F10', scope: 'live', act: () => toast('Step over') },
  { id: 'stepinto', label: 'Step into', group: DBG, def: 'F11', scope: 'live', act: () => toast('Step into') },
  { id: 'breakpoint', label: 'Toggle breakpoint', group: DBG, def: 'F9', scope: 'target', act: () => toast('Toggle breakpoint') },
  { id: 'undo', label: 'Undo layout change', group: 'Layout', def: 'Ctrl+Shift+Z', scope: 'global', act: () => dock.undo() },
  { id: 'redo', label: 'Redo layout change', group: 'Layout', def: 'Ctrl+Shift+Y', scope: 'global', act: () => dock.redo() },
];
const keyOf = id => (id in prefs.keys ? prefs.keys[id] : (KEYDEFS.find(d => d.id === id)?.def ?? null));
function comboOf(e) {                              // normalize an event → "Ctrl+Shift+P" / "F2" / "Ctrl+/"
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return null;
  let s = '';
  if (e.ctrlKey || e.metaKey) s += 'Ctrl+';
  if (e.shiftKey) s += 'Shift+';
  if (e.altKey) s += 'Alt+';
  // Letters/digits come from the PHYSICAL key code, not e.key — otherwise a
  // non-Latin keyboard layout (Ukrainian, Russian, …) reports the Cyrillic char
  // for the Z key ('я'), so Ctrl+Z would never match a 'Ctrl+Z' binding. Function
  // keys, arrows and punctuation keep e.key (layout-independent enough, and lets
  // '/' , ',' etc. bind naturally).
  let k;
  if (/^Key[A-Z]$/.test(e.code)) k = e.code.slice(3);          // KeyZ -> Z
  else if (/^Digit[0-9]$/.test(e.code)) k = e.code.slice(5);   // Digit0 -> 0
  else { k = e.key; if (k.length === 1) k = k.toUpperCase(); }
  return s + k;
}
let recordingId = null;
function setKey(id, combo) { prefs.keys[id] = combo; savePrefs(); renderKeybindings(); }
function clearOverride(id) { delete prefs.keys[id]; savePrefs(); renderKeybindings(); }
function resetAllKeys() { prefs.keys = {}; savePrefs(); renderKeybindings(); toast('Keybindings reset to defaults'); }
function startRecording(id) {                      // capture the next chord and bind it
  recordingId = id; renderKeybindings();
  const cap = e => {
    e.preventDefault(); e.stopPropagation();
    if (e.key === 'Escape') { recordingId = null; window.removeEventListener('keydown', cap, true); renderKeybindings(); return; }
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;   // wait for the non-modifier key
    const combo = comboOf(e); if (!combo) return;
    recordingId = null; window.removeEventListener('keydown', cap, true); setKey(id, combo);
  };
  window.addEventListener('keydown', cap, true);
}
function renderKeybindings() {
  const el = $('#keybind-list'); if (!el) return;
  const groups = [...new Set(KEYDEFS.map(d => d.group))];
  el.innerHTML = `<div class="kb-top"><span class="kb-hint">Click a shortcut to rebind</span><button class="sg" data-kb="reset-all">Reset all to defaults</button></div>` +
    groups.map(g => `<div class="kb-grp">${escH(g)}</div>` + KEYDEFS.filter(d => d.group === g).map(d => {
      const cur = keyOf(d.id), custom = d.id in prefs.keys, rec = recordingId === d.id;
      const chip = rec ? `<span class="kbd rec">press keys…</span>`
        : (cur === null ? `<span class="kbd unbound">unbound</span>` : `<span class="kbd">${escH(cur)}</span>`);
      return `<div class="kb-row" data-id="${d.id}"><span class="kb-c">${escH(d.label)}</span><span class="kb-keys">${chip}` +
        `<button class="kb-act" data-kb="edit" title="Rebind">${svg(ICON.rename, '')}</button>` +
        `<button class="kb-act" data-kb="del" title="Remove binding">${svg(ICON.x, '')}</button>` +
        `<button class="kb-act${custom ? '' : ' hide'}" data-kb="revert" title="Reset to default">${svg(ICON.reset, '')}</button></span></div>`;
    }).join('')).join('');
}
$('#keybind-list')?.addEventListener('click', e => {
  if (e.target.closest('[data-kb="reset-all"]')) return resetAllKeys();
  const row = e.target.closest('.kb-row'); if (!row) return;
  const id = row.dataset.id, btn = e.target.closest('[data-kb]');
  const kb = btn ? btn.dataset.kb : (e.target.closest('.kbd') ? 'edit' : null);
  if (kb === 'edit') startRecording(id);
  else if (kb === 'del') setKey(id, null);
  else if (kb === 'revert') clearOverride(id);
});

// ---------- settings overlay (real pane switching) ----------
const settings = $('#settings-ov');
let advWired = false;
async function refreshAdvanced() {
  const eng = $('#adv-engine');
  if (eng) {
    if (!isNative) eng.textContent = 'web preview — engine offline';
    else { const i = await engineInfo(); eng.textContent = i && i.ok ? `${i.bin || 'n0xis'} · v${i.version} · ${i.commandCount} cmds` : 'engine not found on PATH'; }
  }
  if (!advWired) { advWired = true; $('#adv-reset')?.addEventListener('click', () => { try { localStorage.clear(); } catch {} toast('Preferences reset — reload the window to apply'); }); }
}
function showSettingsPane(name) {
  $$('.snav .sni').forEach(x => x.classList.toggle('on', x.dataset.pane === name));
  $$('.scontent .spane').forEach(p => { p.hidden = p.dataset.pane !== name; });
  if (name === 'keybindings') renderKeybindings();
  if (name === 'advanced') refreshAdvanced();
}
function openSettings(pane) { settings.classList.add('on'); if (pane) showSettingsPane(pane); }
$('#btn-settings').addEventListener('click', () => openSettings());
$('#settings-close').addEventListener('click', () => settings.classList.remove('on'));
settings.addEventListener('click', e => { if (e.target === settings) settings.classList.remove('on'); });
$$('.snav .sni').forEach(n => n.addEventListener('click', () => showSettingsPane(n.dataset.pane)));

// ---------- command palette ----------
const COMMANDS = [
  ['Decompile · pseudocode', 'SSA-recovered C for the selected function', 'F5', () => setWorkspace('decompile')],
  ['Disassembly listing', 'Annotated instruction listing', 'Space', () => setWorkspace('decompile')],
  ['Show CFG graph', 'Control-flow graph, blocks & edges', '', () => setWorkspace('graph')],
  ['Add widget to canvas…', 'Blender-style dockable panels', '', () => openWpal($('#btn-addw'))],
  ['Reset workspace layout', 'Clear floating widgets', '', () => resetDock()],
  ['Set decompile style…', 'structured · ssa · goto', '', () => toast('Decompile style')],
  ['Find xrefs to / from', 'Who calls this · what it calls', 'Shift+F12', () => toast('Xrefs')],
  ['Set watchpoint (R / W / X)', 'Break when this address is touched', '', () => setWorkspace('dynamic')],
  ['Find what accesses this address', 'count the hits on an address', '', () => setWorkspace('dynamic')],
  ['Memory scan…', 'Hunt a value in a live process', '', () => { setWorkspace('dynamic'); if (!isLive()) pickProcess(); }],
  ['Attach to a process', 'Bind a live process to this session', '', () => pickProcess()],
  ['Apply FLIRT signatures', 'Name statically-linked library code', '', () => echo('sig apply --flirt zlib-1.3.1.npat', 'named 118 functions')],
  ['Explain this with Copilot', 'Walk through the current function', '↵', () => toast('Copilot')],
  ['Change theme…', 'Switch the colour palette', '', () => settings.classList.add('on')],
  ['Open settings', 'Preferences and providers', ',', () => settings.classList.add('on')],
];

/* ---- settings registry: every option is a searchable palette entry (F1) ----
   getSet/setSet persist to localStorage; the command palette lists each setting
   with its current value and toggles/cycles it, so any preference is findable by
   typing — the VS Code F1 model. Docs live in docs/SETTINGS.md for AI/user. */
const SETTINGS = [
  { id: 'cache.mode', label: 'Cache location', desc: 'Where analysis caches (IR + xref index) are stored', def: 'central',
    choices: [['central', 'Central (~/.local/share)'], ['project', 'Beside the binary'], ['custom', 'Custom folder…']] },
  { id: 'cache.keepOnClose', label: 'Keep cache on close', desc: 'Off = delete the derived cache when the window closes (names are always kept)', def: true, toggle: true },
  { id: 'analyze.autorun', label: 'Auto-analyze on open', desc: 'Run discover + RTTI + xref index in the background when a binary opens', def: true, toggle: true },
  { id: 'analyze.warmCfg', label: 'Warm decompilation cache', desc: 'Also pre-decode every function (more disk + time; first decompile view is instant)', def: false, toggle: true },
];
function getSet(id, def) { try { const v = localStorage.getItem('n0x.set.' + id); return v === null ? def : JSON.parse(v); } catch { return def; } }
function setSet(id, val) { try { localStorage.setItem('n0x.set.' + id, JSON.stringify(val)); } catch {} }
function settingById(id) { return SETTINGS.find(s => s.id === id); }
function settingValueLabel(s) {
  const v = getSet(s.id, s.def);
  if (s.toggle) return v ? 'On' : 'Off';
  return (s.choices.find(c => c[0] === v) || [, v])[1];
}
async function runSetting(s) {
  if (s.toggle) { setSet(s.id, !getSet(s.id, s.def)); }
  else {
    const vals = s.choices.map(c => c[0]);
    const next = vals[(vals.indexOf(getSet(s.id, s.def)) + 1) % vals.length];
    if (next === 'custom') { const p = await pickFolder('Choose a cache folder'); if (!p) return; setSet('cache.custom', p); }
    setSet(s.id, next);
  }
  applySetting(s.id);
  toast(s.label + ': ' + settingValueLabel(s));
}
// where the derived cache lives for `path`, honoring cache.mode (null = central)
function projectDir(path) {
  const mode = getSet('cache.mode', 'central');
  if (mode === 'project') return path.replace(/[\/\\][^\/\\]*$/, '') || null;
  if (mode === 'custom') return getSet('cache.custom', '') || null;
  return null;
}
function applySetting(id) {
  if (id === 'cache.keepOnClose' && curPath) setDiscardOnClose(getSet('cache.keepOnClose', true) ? null : curPath, projectDir(curPath));
}
// one-shot actions (not stateful settings)
const SET_ACTIONS = [
  ['Clear analysis cache', 'Delete the IR + xref-index cache for this binary (keeps names)', '', async () => {
    if (!curPath) return toast('Open a binary first');
    const r = await clearCache(curPath, projectDir(curPath));
    toast(r?.ok ? 'Cache cleared · freed ' + fmtBytes(r.freed || 0) : 'Clear failed');
  }],
  ['Show cache size', 'How much disk the analysis cache uses for this binary', '', async () => {
    if (!curPath) return toast('Open a binary first');
    const r = await cacheInfo(curPath, projectDir(curPath));
    toast(r?.ok ? 'Cache: ' + fmtBytes(r.bytes || 0) : 'No cache');
  }],
  ['Re-run analysis', 'Discover + RTTI + xref index again (resumes from cache)', '', () => { if (curPath) startAnalysis(curPath); }],
];
function fmtBytes(n) { if (n < 1024) return n + ' B'; if (n < 1048576) return (n / 1024).toFixed(0) + ' KB'; if (n < 1073741824) return (n / 1048576).toFixed(1) + ' MB'; return (n / 1073741824).toFixed(2) + ' GB'; }
// the full palette list = static commands + settings (with live value) + actions
function palEntries() {
  const setCmds = SETTINGS.map(s => [`${s.label}: ${settingValueLabel(s)}`, s.desc, 'setting', () => runSetting(s)]);
  const actCmds = SET_ACTIONS.map(a => [a[0], a[1], a[2] || 'action', a[3]]);
  return COMMANDS.concat(setCmds, actCmds);
}

const palOv = $('#palette-ov'), palInput = $('#pal-input'), palList = $('#pal-list');
let palSel = 0, palShown = [];
function renderPal(q = '') {
  const s = q.toLowerCase();
  palShown = palEntries().filter(c => (c[0] + ' ' + c[1]).toLowerCase().includes(s));
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
function togglePal() { palOv.classList.contains('on') ? closePal() : openPal(); }
window.addEventListener('keydown', e => {   // Escape is not a bindable command — handled directly
  if (e.key === 'Escape') { closePal(); settings.classList.remove('on'); closeCtx(); closeWpal(); }
});


/* =====================================================================
   CONTROL-FLOW GRAPH — data-driven, aligned edges, pan + zoom
   ===================================================================== */
let GRAPH = {
  blocks: [
    { id: 'block_0', addr: '0x1510', tag: 'entry',     x: 355, y: 24,  w: 210, body: 'if (rsi == 0) return 0;<br>v14 = ~rdi;' },
    { id: 'block_1', addr: '0x1538', tag: '',          x: 360, y: 150, w: 200, body: 'if (rdx &lt;= 0x2e)' },
    { id: 'block_2', addr: '0x1580', tag: '',          x: 110, y: 300, w: 185, body: 'rbx = rsi;' },
    { id: 'block_3', addr: '0x1560', tag: 'loop head', x: 600, y: 290, w: 205, body: 'while (rbx &amp; 7)' },
    { id: 'block_4', addr: '0x156e', tag: 'loop body', x: 600, y: 430, w: 205, body: 'v14 = tbl[v14 ^ *rbx];<br>rdx--;' },
    { id: 'block_5', addr: '0x1590', tag: '',          x: 420, y: 450, w: 200, body: 'v9 = &amp;crc_table;' },
    { id: 'block_6', addr: '0x15fa', tag: 'exit',      x: 300, y: 600, w: 210, body: 'return ~v14;' },
  ],
  // edge types (Binary-Ninja convention): t=true(green) · f=false(red) · u=unconditional(blue) · loop=back-edge(amber)
  edges: [
    { f: 'block_0', t: 'block_1', type: 'u' },   // single successor → unconditional
    { f: 'block_1', t: 'block_2', type: 't' },   // rdx small → skip the align loop
    { f: 'block_1', t: 'block_3', type: 'f' },   // else enter the align loop
    { f: 'block_3', t: 'block_4', type: 't' },   // condition holds → run the body
    { f: 'block_3', t: 'block_5', type: 'f' },   // condition fails → leave the loop
    { f: 'block_4', t: 'block_3', type: 'loop' },// back-edge: body loops back to the header
    { f: 'block_2', t: 'block_6', type: 'u' },
    { f: 'block_5', t: 'block_6', type: 'u' },
  ],
};
// Self-contained: bound to one graph widget's own elements (many can coexist).
let graphSeq = 0;
const graphRebuilds = [];                          // every mounted graph re-reads GRAPH on demand
function initGraphWidget(root) {
  const gv = root.querySelector('.gviewport'), gc = root.querySelector('.gcanvas'), gs = root.querySelector('.gsvg');
  if (!gv || !gc || !gs) return;
  const mid = 'arrow-' + (graphSeq++);            // unique marker id per graph instance
  // edge style: 'ortho' (default — hard 90° right angles) | 'curved' (beta — rounded corners)
  let edgeMode = (() => { try { return localStorage.getItem('n0x.graph.edges') || 'ortho'; } catch { return 'ortho'; } })();
  const cam = { x: 40, y: 20, s: 1 };
  let blocks = GRAPH.blocks.map(b => ({ ...b }));  // local, independently draggable positions
  const rectOf = {};
  const rankOf = {};

  // layered auto-layout (Sugiyama-lite): rank by longest path from entry, then place each
  // node at the BARYCENTRE x of its neighbours (parents + loop partners) and push apart on
  // overlap — so a child sits under its parent, and rows never overlap.
  function layout() {
    if (blocks.length > 160) return;            // skip the O(blocks·edges) layout for capped graphs
    const byId = {}; blocks.forEach(b => byId[b.id] = b);
    const fwd = GRAPH.edges.filter(e => e.type !== 'loop');
    for (const k in rankOf) delete rankOf[k];
    rankOf[blocks[0].id] = 0;
    for (let it = 0; it < 80; it++) {
      let ch = false;
      fwd.forEach(e => { if (rankOf[e.f] == null) return; const nr = rankOf[e.f] + 1; if (rankOf[e.t] == null || rankOf[e.t] < nr) { rankOf[e.t] = nr; ch = true; } });
      if (!ch) break;
    }
    blocks.forEach(b => { if (rankOf[b.id] == null) rankOf[b.id] = 0; });
    const rows = {}; blocks.forEach(b => (rows[rankOf[b.id]] ||= []).push(b));
    const ranks = Object.keys(rows).map(Number).sort((a, c) => a - c);
    const CX = 470, GAPX = 46, PITCH = 158;
    // A block that many edges converge on gets extra headroom ABOVE its row, so the
    // incoming arrows fan out in the band and turn DOWN into its top — instead of
    // cramming into its side at a shallow angle. Extra ≈ one lane-gap per extra parent.
    const inCount = {}; GRAPH.edges.forEach(e => { inCount[e.t] = (inCount[e.t] || 0) + 1; });
    const rowY = {}; let yAcc = 24;
    ranks.forEach((r, idx) => {
      let maxIn = 0; rows[r].forEach(b => maxIn = Math.max(maxIn, inCount[b.id] || 0));
      const extra = Math.min(96, Math.max(0, maxIn - 2) * 16);   // headroom for a heavily-targeted row
      if (idx > 0) yAcc += PITCH;
      yAcc += extra;
      rowY[r] = yAcc;
    });
    blocks.forEach(b => b.y = rowY[rankOf[b.id]]);
    ranks.forEach(r => {
      const row = rows[r];
      if (r === 0) { const tot = row.reduce((s, b) => s + b.w, 0) + GAPX * (row.length - 1); let x = CX - tot / 2; row.forEach(b => { b.x = Math.round(x); x += b.w + GAPX; }); return; }
      // desired centre = barycentre of already-placed neighbours (parents, and loop partner for a body block)
      row.forEach(b => {
        const nb = GRAPH.edges
          .filter(e => (e.t === b.id && rankOf[e.f] < r) || (e.f === b.id && e.type === 'loop' && rankOf[e.t] < r))
          .map(e => e.t === b.id ? byId[e.f] : byId[e.t]).filter(n => n)
          .map(n => n.x + n.w / 2);
        b._d = nb.length ? nb.reduce((a, c) => a + c, 0) / nb.length : CX;
      });
      row.sort((a, b) => a._d - b._d);
      row.forEach(b => b.x = Math.round(b._d - b.w / 2));
      for (let i = 1; i < row.length; i++) { const minX = row[i - 1].x + row[i - 1].w + GAPX; if (row[i].x < minX) row[i].x = minX; }
    });
  }
  layout();
  // does the direct edge from (sx) to (tx) spanning ranks (rf..rt) hit an intermediate block?
  function corridorBlocked(sx, tx, rf, rt, ids) {
    const lo = Math.min(sx, tx) - 8, hi = Math.max(sx, tx) + 8;
    return blocks.some(b => rankOf[b.id] > rf && rankOf[b.id] < rt && !ids.includes(b.id) && b.x < hi && b.x + b.w > lo);
  }

  function buildNodes() {
    gc.querySelectorAll('.gnode').forEach(n => n.remove());
    blocks.forEach(b => {
      const n = document.createElement('div');
      n.className = 'gnode' + (b.tag === 'entry' ? ' entry' : b.tag === 'exit' ? ' exit' : '');
      n.style.left = b.x + 'px'; n.style.top = b.y + 'px'; n.style.width = b.w + 'px';
      n.dataset.ctx = 'gnode'; n.dataset.addr = b.addr; n.dataset.id = b.id;
      n.innerHTML = `<div class="gt">${b.id} · ${b.addr}${b.tag ? ' · ' + b.tag : ''}</div>${b.body}`;
      gc.appendChild(n);
      rectOf[b.id] = { x: b.x, y: b.y, w: b.w, h: n.offsetHeight };
      // drag the block to reposition it — edges follow live
      n.addEventListener('pointerdown', e => {
        if (e.button !== 0) return;               // leave right-click for the context menu
        e.stopPropagation();                      // don't pan the viewport
        const sx = e.clientX, sy = e.clientY, ox = b.x, oy = b.y;
        n.setPointerCapture(e.pointerId); n.classList.add('dragging');
        const move = ev => {
          b.x = ox + (ev.clientX - sx) / cam.s; b.y = oy + (ev.clientY - sy) / cam.s;
          n.style.left = b.x + 'px'; n.style.top = b.y + 'px';
          rectOf[b.id].x = b.x; rectOf[b.id].y = b.y; drawEdges();
        };
        const up = () => { n.releasePointerCapture(e.pointerId); n.classList.remove('dragging'); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); };
        window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
      });
    });
  }
  // ---- orthogonal edge routing (Binary-Ninja-style) ----------------------
  // A polyline through `pts`; radius>0 rounds every 90° corner (the beta mode),
  // radius=0 keeps hard right angles (the default). No diagonals either way.
  function polyPath(pts, radius) {
    // drop consecutive duplicates so a zero-length segment can't break the round
    const p = pts.filter((q, i) => i === 0 || q[0] !== pts[i - 1][0] || q[1] !== pts[i - 1][1]);
    if (p.length < 2) return '';
    if (radius <= 0 || p.length < 3) return 'M' + p.map(q => q[0] + ' ' + q[1]).join(' L');
    const dist = (a, b) => Math.hypot(b[0] - a[0], b[1] - a[1]);
    let d = `M${p[0][0]} ${p[0][1]}`;
    for (let i = 1; i < p.length - 1; i++) {
      const p0 = p[i - 1], p1 = p[i], p2 = p[i + 1];
      const r = Math.min(radius, dist(p0, p1) / 2, dist(p1, p2) / 2);
      const u1 = [(p1[0] - p0[0]) / (dist(p0, p1) || 1), (p1[1] - p0[1]) / (dist(p0, p1) || 1)];
      const u2 = [(p2[0] - p1[0]) / (dist(p1, p2) || 1), (p2[1] - p1[1]) / (dist(p1, p2) || 1)];
      d += ` L${(p1[0] - u1[0] * r).toFixed(1)} ${(p1[1] - u1[1] * r).toFixed(1)}`;
      d += ` Q${p1[0]} ${p1[1]} ${(p1[0] + u2[0] * r).toFixed(1)} ${(p1[1] + u2[1] * r).toFixed(1)}`;
    }
    const last = p[p.length - 1];
    return d + ` L${last[0]} ${last[1]}`;
  }
  // spread N ports evenly across a block edge span [lo..hi] so incoming arrows
  // land in a row (never stacked at the centre); a lone port sits at the middle.
  function portAt(lo, hi, idx, count) {
    if (count <= 1) return (lo + hi) / 2;
    const m = Math.min(20, (hi - lo) / (count + 1));
    return lo + m + ((hi - lo) - 2 * m) * (idx / (count - 1));
  }

  // Greedy track allocator: pack non-overlapping intervals on the same track,
  // push overlapping ones onto the next — so parallel runs keep a fixed gap and
  // never merge, but two runs that only touch at a point may share a line.
  function allocTracks(items, gap) {
    items.sort((a, b) => a.lo - b.lo);
    const end = [];
    items.forEach(it => { let t = 0; while (t < end.length && end[t] > it.lo - gap) t++; it.track = t; end[t] = it.hi; it.tracks = end; });
    return end.length;
  }
  function drawEdges() {
    // explicit per-colour markers (context-stroke is unreliable across themes)
    const M = { n: mid + '-n', t: mid + '-t', f: mid + '-f', loop: mid + '-l', u: mid + '-u' };
    const mk = (id, c) => `<marker id="${id}" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="${c}"/></marker>`;
    const defs = `<defs>${mk(M.n, 'var(--bd2)')}${mk(M.t, 'var(--ok)')}${mk(M.f, 'var(--dgr)')}${mk(M.loop, 'var(--live)')}${mk(M.u, 'var(--fn)')}</defs>`;
    const round = edgeMode === 'curved' ? 8 : 0;

    // Rows & bands: horizontal runs live ONLY in the empty y-bands between block
    // rows; vertical runs live ONLY in gutters — so no segment ever crosses a block.
    const rowsByRank = {};
    blocks.forEach(b => { const r = rectOf[b.id]; if (r) (rowsByRank[rankOf[b.id]] ||= []).push(r); });
    const ranks = Object.keys(rowsByRank).map(Number).sort((a, c) => a - c);
    const rTop = {}, rBot = {};
    ranks.forEach(rk => { rTop[rk] = Math.min(...rowsByRank[rk].map(r => r.y)); rBot[rk] = Math.max(...rowsByRank[rk].map(r => r.y + r.h)); });
    const bandBelow = rk => { const i = ranks.indexOf(rk), n = ranks[i + 1]; return n == null ? rBot[rk] + 42 : (rBot[rk] + rTop[n]) / 2; };
    const bandAbove = rk => { const i = ranks.indexOf(rk), p = ranks[i - 1]; return p == null ? rTop[rk] - 42 : (rBot[p] + rTop[rk]) / 2; };

    let minBX = Infinity, maxBX = -Infinity;
    blocks.forEach(b => { const r = rectOf[b.id]; if (r) { minBX = Math.min(minBX, r.x); maxBX = Math.max(maxBX, r.x + r.w); } });
    if (!isFinite(minBX)) { minBX = 0; maxBX = 0; }
    const midX = (minBX + maxBX) / 2;
    // Is the vertical column at x (over [y0..y1]) free of blocks? — lets an edge drop
    // straight down the target's own column instead of detouring out to a gutter.
    const vClear = (x, y0, y1, ids) => {
      const lo = Math.min(y0, y1), hi = Math.max(y0, y1);
      return !blocks.some(bl => { const r = rectOf[bl.id]; return r && !ids.includes(bl.id) && x > r.x - 8 && x < r.x + r.w + 8 && r.y < hi - 3 && r.y + r.h > lo + 3; });
    };
    // Pre-classify (port-independent) so ports can be ordered by TRUE approach.
    // 'gut' = must detour to a side gutter; otherwise the edge drops the target column.
    const meta = GRAPH.edges.map((e, i) => {
      const a = rectOf[e.f], b = rectOf[e.t]; if (!a || !b) return null;
      const rf = rankOf[e.f], rt = rankOf[e.t];
      const fc = a.x + a.w / 2, tc = b.x + b.w / 2;
      const side = (fc + tc) / 2 <= midX ? 'L' : 'R';
      let gut;
      if (rt === rf + 1) gut = false;                      // adjacent — always a clean drop
      else if (rt <= rf) gut = true;                       // back-edge / same rank — round the side
      else gut = !vClear(tc, a.y + a.h - 2, b.y + 2, [e.f, e.t]);  // forward: drop only if column clear
      return { e, i, a, b, rf, rt, fc, tc, side, gut };
    });
    const metaByI = {}; meta.forEach(m => { if (m) metaByI[m.i] = m; });

    // Ports: exits spread across each block's bottom, entries across its top. Order by
    // APPROACH so arrows don't cross needlessly — a gutter edge takes the far port on
    // its side; a direct edge sorts by the neighbour's centre (this is the swap that
    // keeps a left-source on the left port and a right-source on the right).
    const BIG = 1e9;
    const outG = {}, inG = {};
    meta.forEach(m => { if (!m) return; (outG[m.e.f] ||= []).push(m.i); (inG[m.e.t] ||= []).push(m.i); });
    const outPort = {}, inPort = {};
    const exitKey = i => { const m = metaByI[i]; return m.gut ? (m.side === 'L' ? -BIG : BIG) : m.tc; };
    const entryKey = i => { const m = metaByI[i]; return m.gut ? (m.side === 'L' ? -BIG : BIG) : m.fc; };
    for (const id in outG) { const r = rectOf[id]; const a = outG[id]; a.sort((i, j) => exitKey(i) - exitKey(j)); a.forEach((ei, k) => outPort[ei] = portAt(r.x, r.x + r.w, k, a.length)); }
    for (const id in inG) { const r = rectOf[id]; const a = inG[id]; a.sort((i, j) => entryKey(i) - entryKey(j)); a.forEach((ei, k) => inPort[ei] = portAt(r.x, r.x + r.w, k, a.length)); }

    // Pass 1 — skeleton. 'zig' = drop the target column (across the band below the
    // source, then straight down); 'gut' = detour out to a side gutter.
    const bandRuns = {};                 // rounded band-y → [hrun]
    const gutRuns = { L: [], R: [] };    // gutter side → [route]
    const routes = meta.map(m => {
      if (!m) return null;
      const e = m.e, i = m.i, a = m.a, b = m.b;
      const sx = outPort[i], sBot = a.y + a.h, tx = inPort[i], tTop = b.y - 1;
      const R = { e, i, sx, sBot, tx, tTop };
      if (!m.gut) {
        R.kind = 'zig';
        const base = bandBelow(m.rf);
        R.h1 = { lo: Math.min(sx, tx), hi: Math.max(sx, tx), base, r: R };
        (bandRuns[Math.round(base)] ||= []).push(R.h1);
      } else {
        R.kind = 'gut';
        R.side = m.side;
        R.b1 = bandBelow(m.rf); R.b2 = bandAbove(m.rt);
        gutRuns[R.side].push(R);
      }
      return R;
    });
    // Pass 2a — gutter columns. Route each edge (loops, back-edges, blocked spans)
    // through the NEAREST clear vertical corridor beside its OWN blocks — a tight loop
    // hugging the block — instead of swinging out to the global margin. Then give runs
    // that share a corridor and overlap in y separate tracks so none of them merge.
    // A gutter lane keeps a fixed GAP from every block — the SAME gap it keeps from
    // other lanes — so an arrow can never touch or cross a block it isn't attached to.
    const GAP = 14;
    const clearAt = (x, lo, hi, ids) => !blocks.some(bl => { const r = rectOf[bl.id]; return r && !ids.includes(bl.id) && r.y < hi - 3 && r.y + r.h > lo + 3 && x > r.x - GAP && x < r.x + r.w + GAP; });
    const nearestGut = (side, x0, y0, y1, ids) => {
      let x = x0; const lo = Math.min(y0, y1), hi = Math.max(y0, y1);
      for (let guard = 0; guard < 400; guard++) {
        if (clearAt(x, lo, hi, ids)) return x;
        let nx = side === 'R' ? x + 8 : x - 8;
        blocks.forEach(bl => { const r = rectOf[bl.id]; if (!r || ids.includes(bl.id)) return;
          if (r.y < hi - 3 && r.y + r.h > lo + 3 && x > r.x - GAP && x < r.x + r.w + GAP) nx = side === 'R' ? Math.max(nx, r.x + r.w + GAP) : Math.min(nx, r.x - GAP); });
        x = nx;
      }
      return x;
    };
    for (const side of ['L', 'R']) {
      const rs = gutRuns[side], dir = side === 'R' ? 1 : -1;
      rs.forEach(R => {
        const a = rectOf[R.e.f], b = rectOf[R.e.t];
        const x0 = side === 'R' ? Math.max(a.x + a.w, b.x + b.w) + GAP : Math.min(a.x, b.x) - GAP;
        R.baseX = nearestGut(side, x0, R.b1, R.b2, [R.e.f, R.e.t]);
      });
      const byCorr = {};                              // separate tracks only WITHIN a shared corridor
      rs.forEach(R => (byCorr[Math.round(R.baseX / GAP)] ||= []).push(R));
      for (const k in byCorr) {
        const items = byCorr[k].map(R => ({ lo: Math.min(R.b1, R.b2), hi: Math.max(R.b1, R.b2), R }));
        allocTracks(items, 4);
        // each successive track is GAP further out, then snapped clear of any block it meets
        items.forEach(it => { it.R.laneX = nearestGut(side, it.R.baseX + dir * it.track * GAP, it.R.b1, it.R.b2, [it.R.e.f, it.R.e.t]); });
      }
      gutRuns[side].forEach(R => {
        R.h1 = { lo: Math.min(R.sx, R.laneX), hi: Math.max(R.sx, R.laneX), base: R.b1, r: R, isB1: true };
        R.h2 = { lo: Math.min(R.tx, R.laneX), hi: Math.max(R.tx, R.laneX), base: R.b2, r: R, isB1: false };
        (bandRuns[Math.round(R.b1)] ||= []).push(R.h1);
        (bandRuns[Math.round(R.b2)] ||= []).push(R.h2);
      });
    }
    // Pass 2b — band tracks: horizontal runs sharing a band get parallel y-lines.
    for (const k in bandRuns) {
      const g = bandRuns[k];
      const n = allocTracks(g, 4);
      g.forEach(h => { h.y = h.base + (h.track - (n - 1) / 2) * 7; });
    }

    const paths = routes.map(R => {
      if (!R) return '';
      const e = R.e, cls = 'gedge' + (e.type ? ' ' + e.type : '');
      const arw = e.type === 't' ? M.t : e.type === 'f' ? M.f : e.type === 'loop' ? M.loop : e.type === 'u' ? M.u : M.n;
      let pts;
      if (R.kind === 'zig') {
        const my = R.h1.y;
        pts = [[R.sx, R.sBot], [R.sx, my], [R.tx, my], [R.tx, R.tTop]];
      } else {                            // gutter: down into band, across, up/down the gutter, across, into top
        const y1 = R.h1.y, y2 = R.h2.y, lx = R.laneX;
        pts = [[R.sx, R.sBot], [R.sx, y1], [lx, y1], [lx, y2], [R.tx, y2], [R.tx, R.tTop]];
      }
      return `<path class="${cls}" marker-end="url(#${arw})" d="${polyPath(pts, round)}"/>`;
    }).join('');
    gs.innerHTML = defs + paths;
  }
  function renderGraph() {
    // our Sugiyama-lite layout is fine for typical functions but O(blocks·edges);
    // a giant CFG would freeze the webview, so cap it (a real ELK/dagre engine is
    // the production path). Show a note instead of hanging.
    if (blocks.length > 160) {
      gc.querySelectorAll('.gnode').forEach(n => n.remove()); gs.innerHTML = '';
      let m = gv.querySelector('.gtoobig'); if (!m) { m = document.createElement('div'); m.className = 'gtoobig'; gv.appendChild(m); }
      m.innerHTML = `Large control-flow graph — <b>${blocks.length}</b> blocks.<br>Rendering is capped for performance; use the Decompiler / Disassembly for this function.`;
      return;
    }
    gv.querySelector('.gtoobig')?.remove();
    buildNodes(); drawEdges();
  }
  function applyCam() { gc.style.transform = `translate(${cam.x}px,${cam.y}px) scale(${cam.s})`; const l = root.querySelector('.gzl'); if (l) l.textContent = Math.round(cam.s * 100) + '%'; }
  function fitGraph() {
    const vw = gv.clientWidth, vh = gv.clientHeight; if (!vw || !vh) return;
    let maxX = 0, maxY = 0;
    blocks.forEach(b => { maxX = Math.max(maxX, b.x + b.w); maxY = Math.max(maxY, b.y + (rectOf[b.id]?.h || 70)); });
    cam.s = clamp(Math.min(vw / (maxX + 60), vh / (maxY + 60)), 0.4, 1.3);
    cam.x = (vw - maxX * cam.s) / 2; cam.y = Math.max(16, (vh - maxY * cam.s) / 2); applyCam();
  }
  let pan = null, userAdjusted = false;           // once the user pans/zooms, stop auto-fitting
  gv.addEventListener('pointerdown', e => { if (e.target.closest('.gzoom, .gctl, .gnode, .glegend')) return; legend?.classList.remove('on'); legBtn?.classList.remove('on'); pan = { x: e.clientX, y: e.clientY, cx: cam.x, cy: cam.y }; gv.classList.add('panning'); gv.setPointerCapture(e.pointerId); });
  gv.addEventListener('pointermove', e => { if (!pan) return; cam.x = pan.cx + (e.clientX - pan.x); cam.y = pan.cy + (e.clientY - pan.y); userAdjusted = true; applyCam(); });
  gv.addEventListener('pointerup', () => { pan = null; gv.classList.remove('panning'); });
  gv.addEventListener('wheel', e => { e.preventDefault(); const r = gv.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top; const ns = clamp(cam.s * (e.deltaY < 0 ? 1.12 : 0.89), 0.3, 2.4); cam.x = mx - (mx - cam.x) * (ns / cam.s); cam.y = my - (my - cam.y) * (ns / cam.s); cam.s = ns; userAdjusted = true; applyCam(); }, { passive: false });
  function zoomBy(factor) { // zoom about the centre of the viewport, not the corner
    const r = gv.getBoundingClientRect(), cx = r.width / 2, cy = r.height / 2;
    const ns = clamp(cam.s * factor, 0.3, 2.4);
    cam.x = cx - (cx - cam.x) * (ns / cam.s); cam.y = cy - (cy - cam.y) * (ns / cam.s);
    cam.s = ns; userAdjusted = true; applyCam();
  }
  root.querySelector('.gz-in').addEventListener('click', () => zoomBy(1.15));
  root.querySelector('.gz-out').addEventListener('click', () => zoomBy(0.87));
  root.querySelector('.gz-fit').addEventListener('click', () => { userAdjusted = false; fitGraph(); }); // fit re-enables auto-fit
  // edge-style toggle (beta): ortho ⇄ curved, persisted; re-draws in place
  const curveBtn = root.querySelector('.gc-curve');
  const syncCurve = () => { curveBtn?.classList.toggle('on', edgeMode === 'curved'); };
  syncCurve();
  curveBtn?.addEventListener('click', () => {
    edgeMode = edgeMode === 'curved' ? 'ortho' : 'curved';
    try { localStorage.setItem('n0x.graph.edges', edgeMode); } catch {}
    syncCurve(); drawEdges();
    toast(edgeMode === 'curved' ? 'Rounded edges (beta)' : 'Orthogonal edges');
  });
  // legend popover — opened on demand from its button (hidden by default)
  const legend = root.querySelector('.glegend'), legBtn = root.querySelector('.gc-legend');
  legBtn?.addEventListener('click', e => { e.stopPropagation(); legend?.classList.toggle('on'); legBtn.classList.toggle('on', legend?.classList.contains('on')); });
  renderGraph();
  requestAnimationFrame(fitGraph);
  setTimeout(fitGraph, 120); // refit once layout settles
  try { new ResizeObserver(() => { if (!userAdjusted) fitGraph(); }).observe(gv); } catch {}
  // rebuild from the current GRAPH (a new function was selected) — re-read, re-layout, re-fit
  function rebuild() {
    if (!gc.isConnected) return;                  // widget was closed
    blocks = GRAPH.blocks.map(b => ({ ...b }));
    for (const k in rectOf) delete rectOf[k];
    for (const k in rankOf) delete rankOf[k];
    layout(); renderGraph(); userAdjusted = false; fitGraph(); setTimeout(fitGraph, 60);
  }
  graphRebuilds.push(rebuild);
  if (isNative && curPath) loadGraph();          // fetch the current function's CFG on mount
}
// convert the engine's `ir build` CFG into the graph widget's block/edge model
function cfgToGraph(d) {
  const idByStart = {};
  (d.blocks || []).forEach(b => { idByStart[b.start] = 'block_' + b.id; });
  const val = a => { try { return parseInt(a, 16); } catch { return 0; } };
  const blocks = (d.blocks || []).map((b, i) => {
    const ins = b.insns || [];
    const body = ins.slice(0, 6).map(x => escH(x.text || x.mnemonic || '')).join('<br>') + (ins.length > 6 ? `<br><span style="color:var(--tx2)">… +${ins.length - 6}</span>` : '');
    const isExit = !(b.successors && b.successors.length);
    return { id: 'block_' + b.id, addr: b.start, tag: i === 0 ? 'entry' : (isExit ? 'exit' : ''), x: 0, y: 0, w: 250, body: body || '—' };
  });
  const edges = [];
  (d.blocks || []).forEach(b => (b.successors || []).forEach(s => {
    const to = idByStart[s.to]; if (!to) return;
    let type = s.kind === 'cjmp-true' ? 't' : s.kind === 'cjmp-false' ? 'f' : 'u';
    if (val(s.to) <= val(b.start)) type = 'loop';   // backward edge = loop back-edge
    edges.push({ f: 'block_' + b.id, t: to, type });
  }));
  return { blocks, edges };
}
async function loadGraph() {
  if (!isNative || !curPath) return;               // keep the demo CFG in the web preview
  const r = await n0xCached('ir|' + curPath + '|' + selAddr, ['ir', 'build', '--file', curPath, '--addr', selAddr]);
  if (r?.ok && Array.isArray(r.data.blocks) && r.data.blocks.length) {
    GRAPH = cfgToGraph(r.data);
    graphRebuilds.forEach(fn => { try { fn(); } catch {} });
  }
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
      item('Show disassembly', 'disasm', '', () => { tgt.click(); setWorkspace('decompile'); }),
      item('Show CFG graph', 'graph', '', () => setWorkspace('graph')),
      sep,
      item('Rename…', 'rename', 'F2', () => doRename(addr || selAddr, name)),
      item('Set return type…', 'type', '', () => doRetypeReturn(addr || selAddr)),
      item('Bookmark this', 'bookmark', '', () => toggleBookmark(addr || selAddr)),
      item('Find xrefs to', 'xref', 'Shift+F12', () => toast('Xrefs to ' + name)),
      item('Find xrefs from', 'xref', '', () => toast('Xrefs from ' + name)),
      item('Apply signature', 'type', '', () => echo('sig apply --func ' + name, 'matched 1')),
      sep,
      item('Copy name', 'copy', '', () => copy(name)),
      item('Copy address', 'copy', '', () => copy(addr)),
    ];
    case 'cl': return [
      item('Copy line', 'copy', 'Ctrl+C', () => copy(tgt.textContent.replace(/^\d+/, '').trim())),
      item('Rename function…', 'rename', 'F2', () => doRename(selAddr, selName)),
      item('Set return type…', 'type', '', () => doRetypeReturn(selAddr)),
      item('Comment…', 'note', 'Ctrl+/', () => doComment(selAddr)),
      item('Bookmark this', 'bookmark', '', () => toggleBookmark(selAddr)),
      sep,
      item('Toggle breakpoint', 'bp', 'F9', () => { tgt.classList.toggle('hot'); toast('Breakpoint toggled'); }),
      item('Add watchpoint', 'watch', '', () => { setWorkspace('dynamic'); toast('Watchpoint added'); }),
    ];
    case 'varrow': {
      const vn = tgt.dataset?.var || '';
      return [
        lbl(vn || 'variable'),
        item('Rename variable…', 'rename', 'F2', () => doRenameVar(addr || selAddr, vn)),
        item('Set type…', 'type', '', () => doRetypeVar(addr || selAddr, vn)),
        sep,
        item('Copy name', 'copy', '', () => copy(vn)),
      ];
    }
    case 'drow': return [
      item('Copy instruction', 'copy', '', () => copy(tgt.textContent.trim())),
      item('Comment…', 'note', 'Ctrl+/', () => doComment(tgt.dataset?.addr || tgt.querySelector('.daddr')?.textContent || selAddr)),
      item('Bookmark this line', 'bookmark', '', () => toggleBookmark(tgt.dataset?.addr || tgt.querySelector('.daddr')?.textContent || selAddr)),
      item('Toggle breakpoint', 'bp', 'F9', () => { tgt.classList.toggle('hot'); toast('Breakpoint @ ' + (tgt.querySelector('.daddr')?.textContent || '')); }),
      item('Set watchpoint', 'watch', '', () => setWorkspace('dynamic')),
      item('Follow in dump', 'hex', '', () => setWorkspace('dynamic')),
      sep,
      item('Copy address', 'copy', '', () => copy(tgt.dataset?.addr || tgt.querySelector('.daddr')?.textContent || '')),
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
      item('Decompile block', 'decomp', '', () => { setWorkspace('decompile'); toast('Decompile ' + tgt.dataset.id); }),
      item('Rename label…', 'rename', 'F2', () => doRename(tgt.dataset.addr || selAddr, tgt.dataset.id)),
      item('Invert branch logic', 'reset', '', () => doPatch('Invert branch', tgt.dataset.addr || selAddr)),
      item('Set breakpoint', 'bp', 'F9', () => toast('Breakpoint @ ' + tgt.dataset.addr)),
      sep,
      item('Find xrefs to block', 'xref', 'Shift+F12', () => toast('Xrefs → ' + tgt.dataset.addr)),
      item('Open in new tab', 'add', '', () => toast('Open ' + tgt.dataset.id + ' in a new pane')),
      item('Comment…', 'note', 'Ctrl+/', () => doComment(tgt.dataset.addr || selAddr)),
      sep,
      item('Copy block address', 'copy', '', () => copy(tgt.dataset.addr || '')),
    ];
    case 'typrow': {
      const tn = tgt.dataset?.name || '';
      const isStruct = tgt.dataset?.kind === 'struct';
      return [
        lbl(tgt.dataset?.kind + ' ' + tn),
        ...(isStruct ? [item('Add field…', 'add', '', () => addField(tn))] : []),
        item('Copy name', 'copy', '', () => copy(tn)),
        sep,
        item('Delete type', 'trash', '', () => rmType(tn), { danger: true }),
      ];
    }
    case 'bmkrow': {
      const a = tgt.dataset?.addr || '';
      return [
        lbl(a),
        item('Go to', 'xref', '', () => findGoto(a)),
        item('Rename…', 'rename', 'F2', () => doRename(a, tgt.querySelector('.fnc')?.textContent || '')),
        item('Comment…', 'note', 'Ctrl+/', () => doComment(a)),
        item('Toggle bookmark', 'bookmark', '', () => toggleBookmark(a)),
        sep,
        item('Copy address', 'copy', '', () => copy(a)),
        item('Remove all annotations here', 'trash', '', () => rmAnno(a), { danger: true }),
      ];
    }
    default: return [
      item('Add widget…', 'add', '', () => openWpal($('#btn-addw'))),
      item('Command palette', 'decomp', 'Ctrl+P', openPal),
      item('Reset layout', 'reset', '', resetDock),
      sep,
      item('Settings', 'settings', 'Ctrl ,', () => settings.classList.add('on')),
    ];
  }
}
function copy(t) { navigator.clipboard?.writeText(t).catch(() => {}); toast('Copied <span class="mono">' + t + '</span>'); }

/* =====================================================================
   REAL EDIT ACTIONS — annotate / const identify over the engine seam.
   `window.prompt` is unreliable in the Tauri webview, so use a small modal.
   ===================================================================== */
function askInput(title, value = '', placeholder = '') {
  return new Promise(resolve => {
    const ov = document.createElement('div');
    ov.className = 'ask-ov';
    ov.innerHTML = `<div class="ask-box"><div class="ask-t">${escH(title)}</div>` +
      `<input class="ask-in mono" value="${escH(value)}" placeholder="${escH(placeholder)}">` +
      `<div class="ask-a"><button class="ask-cancel">Cancel</button><button class="ask-ok">OK</button></div></div>`;
    document.body.appendChild(ov);
    const inp = ov.querySelector('.ask-in');
    const done = v => { ov.remove(); resolve(v); };
    ov.querySelector('.ask-ok').onclick = () => done(inp.value);
    ov.querySelector('.ask-cancel').onclick = () => done(null);
    ov.addEventListener('mousedown', e => { if (e.target === ov) done(null); });
    inp.addEventListener('keydown', e => { e.stopPropagation(); if (e.key === 'Enter') done(inp.value); else if (e.key === 'Escape') done(null); });
    setTimeout(() => { inp.focus(); inp.select(); }, 20);
  });
}
/* =====================================================================
   Annotation edits with a real undo/redo stack (Ctrl+Z / Ctrl+Y).
   The engine already keeps full versioned history in .n0x/annotations.json;
   this is the live stack that lets a rename/comment/type be reversed with a
   keystroke. Every edit records (before → after) so undo re-applies `before`.
   ===================================================================== */
const annoUndo = [], annoRedo = [];
const ANNO_MAX = 200;
async function readRecord(addr) { const r = await eng(['annotate', 'show', '--addr', addr]); return r?.ok ? r.data : null; }
function fieldOf(rec, kind, key) {
  if (!rec) return null;
  if (kind === 'name') return rec.name ?? null;
  if (kind === 'comment') return rec.comment ?? null;
  if (kind === 'type') return rec.type_note ?? null;
  if (kind === 'var') return (rec.var_names && rec.var_names[key]) ?? null;
  if (kind === 'vartype') return (rec.var_types && rec.var_types[key]) ?? null;
  return null;
}
// A variable's stable key is its SYNTHESISED name. After a first rename the user
// sees the new name, so translate a displayed name back to the underlying key —
// otherwise a second rename (or an undo) would target a dead slot.
function effectiveVarKey(rec, displayed) {
  if (!rec || !rec.var_names) return displayed;
  if (displayed in rec.var_names) return displayed;
  for (const [k, v] of Object.entries(rec.var_names)) if (v === displayed) return k;
  return displayed;
}
function annoArgs(kind, addr, key, value) {
  // `var` and `vartype` carry a per-variable key; the rest are address-scoped.
  const a = (kind === 'var' || kind === 'vartype') ? ['annotate', kind, '--addr', addr, '--var', key] : ['annotate', kind, '--addr', addr];
  if (value != null && value !== '') a.push('--value', value);
  return a;
}
// Apply a value WITHOUT recording it — shared by edits and by undo/redo. Routes
// through the session so the write lands in THIS target's .n0x/ (the serve
// process runs in the project dir); a bare n0x() would walk up from the GUI cwd.
async function applyAnno(kind, addr, key, value) {
  const r = await eng(annoArgs(kind, addr, key, value));
  if (r?.ok) { engCache.clear(); refreshAfterAnno(kind, addr, value); }
  return r;
}
function refreshAfterAnno(kind, addr, value) {
  if (kind === 'name') {
    const k = (addr || '').toLowerCase();
    const shown = value || ('sub_' + (addr || '').replace(/^0x/i, '').toUpperCase());
    funcByAddr.set(k, shown);
    if (Array.isArray(FUNCLIST)) { const f = FUNCLIST.find(f => (f.addr || '').toLowerCase() === k); if (f) f.name = shown; }
    if ((selAddr || '').toLowerCase() === k) selName = value || selName;
    try { paintFunctions(); } catch {}
  }
  onSymbolSelect();   // re-run every open view (decomp/disasm/xref/graph/linear)
  try { refreshBookmarks(); } catch {}   // the Notes panel reflects the new name/comment
}
// User-facing edit: capture the current value, apply, push onto the undo stack.
async function editAnno(kind, addr, displayed, value, okverb) {
  value = value || '';
  if (!isNative) { toast(`${okverb} <span class="mono">${escH(displayed || addr)}</span> → ${escH(value || '(cleared)')} · demo`); return; }
  const rec = await readRecord(addr);
  // `var`/`vartype` are keyed by the variable's stable synthesized name (translate
  // the displayed name back); `@return` passes straight through.
  const key = (kind === 'var' || kind === 'vartype') ? effectiveVarKey(rec, displayed) : null;
  const before = fieldOf(rec, kind, key);
  if ((before ?? '') === value) { toast('No change'); return; }   // nothing to record
  const r = await applyAnno(kind, addr, key, value);
  if (!r?.ok) { toast(`annotate failed: ${escH(r?.error?.message || '?')}`); return; }
  annoUndo.push({ kind, addr, key, before, after: value || null, label: displayed || addr });
  if (annoUndo.length > ANNO_MAX) annoUndo.shift();
  annoRedo.length = 0;
  toast(`${okverb} <span class="mono">${escH(displayed || addr)}</span> → ${escH(value || 'cleared')} · Ctrl+Z to undo`);
}
async function annotationUndo() {
  const e = annoUndo.pop();
  if (!e) { toast('Nothing to undo'); return; }
  await applyAnno(e.kind, e.addr, e.key, e.before || '');
  annoRedo.push(e);
  toast(`Undo <span class="mono">${escH(e.label)}</span> ↺ ${escH(e.before || 'cleared')}`);
}
async function annotationRedo() {
  const e = annoRedo.pop();
  if (!e) { toast('Nothing to redo'); return; }
  await applyAnno(e.kind, e.addr, e.key, e.after || '');
  annoUndo.push(e);
  toast(`Redo <span class="mono">${escH(e.label)}</span> → ${escH(e.after || 'cleared')}`);
}
async function doRename(addr = selAddr, cur = selName) {
  const v = await askInput(`Rename ${cur || addr}`, cur || '', 'new function/variable name');
  if (v !== null) editAnno('name', addr, cur, v.trim(), 'Renamed');
}
async function doComment(addr = selAddr) {
  const cur = isNative ? fieldOf(await readRecord(addr), 'comment', null) : null;
  const v = await askInput(`Comment @ ${addr}`, cur || '', 'free-text note');
  if (v !== null) editAnno('comment', addr, addr, v, 'Commented');
}
// Rename a decompiled variable by its displayed name (`local_78`, `rcx`, `v3`) —
// scoped to the current function (fnAddr). WYSIWYG: you rename what you see.
async function doRenameVar(fnAddr, varName) {
  if (!varName) { toast('No variable to rename'); return; }
  const v = await askInput(`Rename variable ${varName}`, varName, 'new variable name');
  if (v !== null) editAnno('var', fnAddr, varName, v.trim(), 'Renamed');
}
// Set the C type of a decompiled variable/parameter — applied live in the
// decompiler's signature + declarations (keyed by the var's displayed name).
async function doRetypeVar(fnAddr, varName) {
  if (!varName) { toast('No variable to type'); return; }
  const cur = isNative ? (fieldOf(await readRecord(fnAddr), 'vartype', effectiveVarKey(await readRecord(fnAddr), varName)) || '') : '';
  const v = await askInput(`Set type of ${varName}`, cur, 'e.g.  int   ·   char *   ·   MyStruct *');
  if (v !== null) editAnno('vartype', fnAddr, varName, v.trim(), 'Type set on');
}
// Set the function's return type (@return); empty / "void" → void.
async function doRetypeReturn(fnAddr = selAddr) {
  const cur = isNative ? (fieldOf(await readRecord(fnAddr), 'vartype', '@return') || '') : '';
  const v = await askInput(`Set return type of ${selName || fnAddr}`, cur, 'e.g.  int   ·   bool   ·   void   ·   Foo *');
  if (v !== null) editAnno('vartype', fnAddr, '@return', v.trim(), 'Return type set on');
}
async function doIdentify(addr = selAddr) {
  if (!isNative || !curPath) { toast('Scanning for known algorithms… · demo'); return; }
  toast('Identifying algorithms @ ' + addr + '…');
  const r = await n0x(['const', 'identify', '--file', curPath, '--addr', addr]);
  const d = r?.data || {};
  const hits = d.matches || d.hits || d.identified || d.constants || [];
  toast(r?.ok ? `Identify: ${Array.isArray(hits) ? hits.length + ' match(es)' : 'done'} — see Console for detail`
              : `identify failed: ${escH(r?.error?.message || '?')}`);
}
function doPatch(kind, addr = selAddr) {          // invert branch / NOP — a live-memory write
  if (!isLive()) { toast('Patch needs a live target — attach a process first (Debug ▸ Attach)'); return; }
  toast(`${kind} @ ${addr} — journaled patch (live) …`);   // full pid+bytes flow lands with the Debugger tier
}
async function doGoto() {                          // navigate to an address or symbol (like VS Code's Go to Line)
  const v = await askInput('Go to address or symbol', '', 'e.g.  0x140001510   or   crc32_z');
  if (v === null || !v.trim()) return;
  const q = v.trim(), rows = $$('#dockspace .flist .frow');
  const hit = rows.find(r => (r.querySelector('.nm')?.textContent || '').toLowerCase() === q.toLowerCase())
    || rows.find(r => (r.querySelector('.fa')?.textContent || '').toLowerCase() === q.toLowerCase());
  if (hit) { hit.click(); hit.scrollIntoView({ block: 'nearest' }); toast('Jumped to ' + escH(hit.querySelector('.nm')?.textContent || q)); }
  else { selAddr = /^0x/i.test(q) ? q : selAddr; selName = q; onSymbolSelect(); toast('Go to ' + escH(q)); }
}

/* ---- Find in image (Ctrl+F): byte / string / escaped-string search over the
   engine's `find` command, results navigate to the address. ---- */
let findState = { mode: 'string', utf16: false, q: '' };
async function openFind() {
  if (!isNative || !curPath) { toast('Open a binary to search'); return; }
  document.querySelector('.find-ov')?.remove();
  const ov = document.createElement('div');
  ov.className = 'find-ov';
  ov.innerHTML = `<div class="find-box"><div class="find-top">` +
    `<input class="find-in mono" placeholder="text, bytes (48 8B ?? C3), or \\x48…" value="${escH(findState.q)}">` +
    `<div class="find-modes">` +
    ['string', 'bytes', 'escaped'].map(m => `<button class="find-mode${m === findState.mode ? ' on' : ''}" data-mode="${m}">${m === 'string' ? 'Text' : m === 'bytes' ? 'Bytes' : 'Escaped'}</button>`).join('') +
    `<label class="find-w"><input type="checkbox" class="find-utf16"${findState.utf16 ? ' checked' : ''}> wide</label></div></div>` +
    `<div class="find-res"></div><div class="find-foot"><span class="find-stat">Enter to search · Esc to close</span></div></div>`;
  document.body.appendChild(ov);
  const inp = ov.querySelector('.find-in'), res = ov.querySelector('.find-res'), stat = ov.querySelector('.find-stat');
  const close = () => ov.remove();
  ov.addEventListener('mousedown', e => { if (e.target === ov) close(); });
  ov.querySelectorAll('.find-mode').forEach(b => b.onclick = () => { findState.mode = b.dataset.mode; ov.querySelectorAll('.find-mode').forEach(x => x.classList.toggle('on', x === b)); inp.focus(); });
  ov.querySelector('.find-utf16').onchange = e => { findState.utf16 = e.target.checked; };
  async function run() {
    const q = inp.value; findState.q = q; if (!q.trim()) return;
    stat.textContent = 'searching…';
    const flag = findState.mode === 'bytes' ? '--bytes' : findState.mode === 'escaped' ? '--escaped' : '--string';
    const args = ['find', '--file', curPath, flag, q, '--limit', '300'];
    if (findState.mode === 'string' && findState.utf16) args.push('--utf16');
    const r = await eng(args);
    if (!r?.ok) { stat.textContent = 'error: ' + escH(r?.error?.message || '?'); res.innerHTML = ''; return; }
    const d = r.data, ms = d.matches || [];
    stat.textContent = `${d.count} match${d.count === 1 ? '' : 'es'}${d.truncated ? ' (capped)' : ''} · ${(d.bytes_scanned || 0).toLocaleString()} bytes scanned`;
    res.innerHTML = ms.map(m => `<div class="find-r" data-addr="${escH(m.va)}"><span class="fnc mono">${escH(m.va)}</span><span class="find-sec">${escH(m.section || '')}</span></div>`).join('') || '<div class="xr-none">no matches</div>';
    res.querySelectorAll('.find-r').forEach(row => row.onclick = () => { findGoto(row.dataset.addr); close(); });
  }
  inp.addEventListener('keydown', e => { e.stopPropagation(); if (e.key === 'Enter') { e.preventDefault(); run(); } else if (e.key === 'Escape') { e.preventDefault(); close(); } });
  setTimeout(() => { inp.focus(); inp.select(); }, 20);
}
function findGoto(addr) {
  if (!addr) return;
  selAddr = addr; selName = ''; selSeq++;
  const sb = $('#sb-addr'); if (sb) sb.textContent = addr;
  onSymbolSelect();                                   // re-center disasm/linear/decomp on the hit
  linears.forEach(r => { if (r.isConnected) r._linJump?.(addr); });
  toast('Jumped to <span class="mono">' + escH(addr) + '</span>');
}

/* ---- Bookmarks / Notes widget: every address the user annotated (name / comment
   / type / var-rename / bookmark), read from `.n0x/annotations.json` via
   `annotate list`. Click a row → jump. This is the "saved work" index. ---- */
function initBookmarks(root) {
  const box = root.querySelector('.bmk') || root;
  box.querySelector('.bmk-refresh')?.addEventListener('click', () => loadBookmarks(box));
  loadBookmarks(box);
}
function refreshBookmarks() { $$('.bmk').forEach(loadBookmarks); }
async function loadBookmarks(box) {
  const el = box.querySelector('.bmk-list'); if (!el) return;
  if (!isNative || !curPath) { el.innerHTML = '<div class="xr-none">open a binary to see its notes</div>'; return; }
  const r = await eng(['annotate', 'list']);
  const recs = (r?.ok && r.data && r.data.records) ? r.data.records : [];
  // Only records the USER created (names, comments, types, var renames, bookmarks)
  // — the recovered 371k RTTI names live elsewhere and would drown the list.
  const rows = recs.filter(x => x.name || x.comment || x.type_note || x.bookmark || (x.var_names && Object.keys(x.var_names).length));
  rows.sort((a, b) => (b.bookmark ? 1 : 0) - (a.bookmark ? 1 : 0) || vaNum(a.va) - vaNum(b.va));
  if (!rows.length) { el.innerHTML = '<div class="xr-none">no notes yet — right-click ▸ “Bookmark this”, or rename / comment something</div>'; return; }
  el.innerHTML = rows.map(x => {
    const va = x.va;
    const nm = x.name || ('sub_' + String(va).replace(/^0x/i, '').toUpperCase());
    const cmt = x.comment ? `<span class="bmk-cmt">${escH(String(x.comment).split('\n')[0])}</span>` : '';
    const star = x.bookmark ? '<span class="bmk-star on" title="bookmark">★</span>' : '<span class="bmk-star"></span>';
    return `<div class="bmk-r" data-ctx="bmkrow" data-addr="${escH(va)}">${star}<span class="fnc mono">${escH(nm)}</span>${cmt}<span class="bmk-va mono">${escH(va)}</span></div>`;
  }).join('');
  el.querySelectorAll('.bmk-r').forEach(row => row.addEventListener('click', () => findGoto(row.dataset.addr)));
}
async function toggleBookmark(addr = selAddr) {
  if (!isNative || !addr) { toast(`Bookmark <span class="mono">${escH(addr)}</span> · demo`); return; }
  const rec = await readRecord(addr);
  const on = !(rec && rec.bookmark);
  const args = ['annotate', 'bookmark', '--addr', addr]; if (!on) args.push('--off');
  const r = await eng(args);
  if (!r?.ok) { toast(`bookmark failed: ${escH(r?.error?.message || '?')}`); return; }
  toast(on ? `Bookmarked <span class="mono">${escH(addr)}</span>` : `Removed bookmark <span class="mono">${escH(addr)}</span>`);
  refreshBookmarks();
}
async function rmAnno(addr = selAddr) {
  if (!isNative || !addr) return;
  const r = await eng(['annotate', 'rm', '--addr', addr]);
  if (r?.ok) { toast(`Cleared annotations @ <span class="mono">${escH(addr)}</span>`); engCache.clear(); onSymbolSelect(); refreshBookmarks(); }
  else toast(`clear failed: ${escH(r?.error?.message || '?')}`);
}

/* ---- Types panel: define struct/enum types (`.n0x/types.json`). A struct's
   named fields make the decompiler render `p->count` for a pointer typed to it
   (right-click a variable ▸ Set type). ---- */
function initTypes(root) {
  const box = root.querySelector('.typ') || root;
  box.querySelector('.typ-refresh')?.addEventListener('click', () => loadTypes(box));
  box.querySelector('.typ-add-s')?.addEventListener('click', () => addStruct());
  box.querySelector('.typ-add-e')?.addEventListener('click', () => addEnum());
  loadTypes(box);
}
function refreshTypes() { $$('.typ').forEach(loadTypes); }
async function loadTypes(box) {
  const el = box.querySelector('.typ-list'); if (!el) return;
  if (!isNative || !curPath) { el.innerHTML = '<div class="xr-none">open a binary to define types</div>'; return; }
  const r = await eng(['type', 'list']);
  const d = (r && r.ok && r.data) ? r.data : { structs: [], enums: [] };
  const struct = s => `<div class="typ-s"><div class="typ-srow" data-ctx="typrow" data-name="${escH(s.name)}" data-kind="struct"><span class="typ-k">struct</span><span class="fnc mono">${escH(s.name)}</span><button class="typ-addf" title="Add field">＋</button></div>` +
    (s.fields || []).slice().sort((a, b) => a.offset - b.offset).map(f => `<div class="typ-f mono">+0x${(f.offset >>> 0).toString(16)} <b>${escH(f.name)}</b>${f.ctype ? ' : ' + escH(f.ctype) : ''}</div>`).join('') + `</div>`;
  const enm = e => `<div class="typ-s"><div class="typ-srow" data-ctx="typrow" data-name="${escH(e.name)}" data-kind="enum"><span class="typ-k">enum</span><span class="fnc mono">${escH(e.name)}</span></div>` +
    (e.members || []).map(m => `<div class="typ-f mono">${escH(m.name)} = ${m.value}</div>`).join('') + `</div>`;
  el.innerHTML = ((d.structs || []).map(struct).join('') + (d.enums || []).map(enm).join('')) ||
    '<div class="xr-none">no types yet — “+ Struct”, add fields, then right-click a variable ▸ Set type</div>';
  el.querySelectorAll('.typ-addf').forEach(b => b.addEventListener('click', e => { e.stopPropagation(); addField(b.closest('.typ-srow').dataset.name); }));
}
async function addStruct() {
  const n = await askInput('New struct name', '', 'e.g.  Session');
  if (n && n.trim()) { await eng(['type', 'struct', '--name', n.trim()]); refreshTypes(); }
}
async function addEnum() {
  const n = await askInput('New enum name', '', 'e.g.  State');
  if (n && n.trim()) { await eng(['type', 'enum', '--name', n.trim()]); refreshTypes(); }
}
async function addField(structName) {
  const spec = await askInput(`Add field to ${structName}`, '', 'offset name [ctype]   e.g.  0x68 count int');
  if (!spec || !spec.trim()) return;
  const p = spec.trim().split(/\s+/);
  if (p.length < 2) { toast('Need: offset name [ctype]'); return; }
  // put replaces the struct, so re-send the existing fields + the new one.
  const r = await eng(['type', 'list']);
  const s = ((r && r.data && r.data.structs) || []).find(x => x.name === structName);
  const args = ['type', 'struct', '--name', structName];
  (s && s.fields || []).forEach(f => args.push('--field', `${f.offset}:${f.name}${f.ctype ? ':' + f.ctype : ''}`));
  args.push('--field', `${p[0]}:${p[1]}${p[2] ? ':' + p[2] : ''}`);
  const w = await eng(args);
  if (!w?.ok) { toast(`add field failed: ${escH(w?.error?.message || '?')}`); return; }
  refreshTypes(); engCache.clear(); onSymbolSelect();   // re-decompile: the field name may now render
}
async function rmType(name) {
  if (!name) return;
  const r = await eng(['type', 'rm', '--name', name]);
  if (r?.ok) { toast(`Removed type <span class="mono">${escH(name)}</span>`); refreshTypes(); engCache.clear(); onSymbolSelect(); }
}

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
   PROJECTS / RECENT TARGETS + the top menu bar
   ===================================================================== */
const RECENT_KEY = 'n0xis.recent.v1';
function getRecent() { try { return JSON.parse(localStorage.getItem(RECENT_KEY)) || []; } catch { return []; } }
function recordRecent(name, kind) {
  if (!name) return;
  let r = getRecent().filter(x => x.name !== name);
  r.unshift({ name, kind, ts: Date.now() });
  try { localStorage.setItem(RECENT_KEY, JSON.stringify(r.slice(0, 8))); } catch {}
}
function fmtAgo(ts) { const s = (Date.now() - ts) / 1000; if (s < 60) return 'just now'; if (s < 3600) return Math.floor(s / 60) + 'm ago'; if (s < 86400) return Math.floor(s / 3600) + 'h ago'; return Math.floor(s / 86400) + 'd ago'; }
let curPid = 0, curPidName = '';
const isLive = () => curPid > 0;
const winCtl = a => $(`[data-win="${a}"]`)?.click();
function closeTarget() { curPid = 0; curPidName = ''; $('#launcher').classList.remove('hidden'); $('#sb-dot').classList.remove('live'); toast('Target closed'); }

/* ---- attach to a live process (real: `process ps` → pick → validate) ---- */
function pickFromList(title, items, opts = {}) {
  return new Promise(resolve => {
    const ov = document.createElement('div'); ov.className = 'ask-ov';
    ov.innerHTML = `<div class="ask-box lst"><div class="ask-t">${escH(title)}</div>` +
      `<input class="ask-in lst-f mono" placeholder="${escH(opts.placeholder || 'Filter…')}">` +
      `<div class="lst-items"></div><div class="ask-a"><button class="ask-cancel">Cancel</button></div></div>`;
    document.body.appendChild(ov);
    const filt = ov.querySelector('.lst-f'), box = ov.querySelector('.lst-items');
    const done = v => { ov.remove(); resolve(v); };
    const render = q => {
      const s = (q || '').toLowerCase();
      const rows = items.filter(it => !s || (it.label + ' ' + (it.sub || '')).toLowerCase().includes(s)).slice(0, 400);
      box.innerHTML = rows.map(it => `<div class="lst-row" data-i="${items.indexOf(it)}">${
        it.iconUri ? `<img class="lst-img" src="${it.iconUri}" alt="">` : (opts.icon ? `<span class="lst-i">${svg(opts.icon, '')}</span>` : '')
        }<span class="lst-l">${escH(it.label)}</span><span class="lst-s mono">${escH(it.sub || '')}</span></div>`).join('') || '<div class="xr-none">no matches</div>';
    };
    render('');
    filt.addEventListener('input', () => render(filt.value));
    box.addEventListener('click', e => { const r = e.target.closest('.lst-row'); if (r) done(items[+r.dataset.i].value); });
    ov.querySelector('.ask-cancel').onclick = () => done(null);
    ov.addEventListener('mousedown', e => { if (e.target === ov) done(null); });
    filt.addEventListener('keydown', e => { e.stopPropagation(); if (e.key === 'Escape') done(null); });
    setTimeout(() => filt.focus(), 20);
  });
}
async function pickProcess() {
  let procs;
  let icons = {};
  if (isNative) {
    toast('Enumerating processes…');
    // prefer the host /proc names (real exe names for programs under Wine or Proton — the
    // engine's `process ps` reports 'wine-preloader' for those), and resolve real
    // per-app icons in parallel (freedesktop + Steam appid).
    const [host, ic] = await Promise.all([listProcesses(), processIcons()]);
    if (host?.ok && host.data.processes?.length) procs = host.data.processes.slice();
    else { const r = await n0x(['process', 'ps']); if (!r?.ok) { toast('process list failed: ' + (r?.error?.message || '?')); return; } procs = (r.data.processes || []).slice(); }
    icons = ic?.data?.icons || {};
  } else {   // web preview — demo list so the flow is exercisable
    procs = [{ name: 'app.exe', pid: 8124 }, { name: 'editor.exe', pid: 4410 }, { name: 'shell.exe', pid: 1200 }, { name: 'server.exe', pid: 9931 }, { name: 'worker.exe', pid: 5567 }];
  }
  procs.sort((a, b) => a.name.localeCompare(b.name));
  const items = procs.map(p => ({ label: p.name, sub: 'pid ' + p.pid, value: p, iconUri: icons[p.pid] }));
  const pick = await pickFromList(`Attach to a process · ${procs.length} running`, items, { placeholder: 'Filter by name…', icon: ICON.chip });
  if (pick) attachProcess(pick.pid, pick.name);
}
function attachProcess(pid, name) {
  curPid = pid; curPidName = name;
  // a live process is a SEPARATE target from any static file — clear the static
  // context so the function list can't show a different binary's functions.
  curPath = ''; FUNCLIST = null; FUNCMETA = null; funcQuery = ''; selAddr = ''; selName = '';
  $$('#dockspace .flist').forEach(fl => { fl.innerHTML = `<div class="fl-empty">Live process · pid ${pid}. Static analysis needs the on-disk binary — open it with <span class="mono">File ▸ Open</span>.</div>`; });
  $$('#dockspace .pfoot').forEach(pf => { pf.innerHTML = `<span class="fcount">live process</span>`; });
  $('#launcher').classList.add('hidden');
  $('#target-name').textContent = name;
  const st = $('#target-state'); if (st) { st.textContent = 'live'; st.style.color = 'var(--live)'; }
  $('#sb-dot').classList.add('live');
  $('#sb-mode').textContent = `live · pid ${pid} · ${name}`;
  setWorkspace('dynamic');
  recordRecent(name, 'dynamic');
  toast(`Attached to <span class="mono">${escH(name)}</span> · pid ${pid}`);
  // validate access up front so permission problems surface immediately, not as silent dead panels
  n0x(['mem', 'map', '--pid', String(pid)]).then(m => { if (m && !m.ok) toast('⚠ ' + escH(m.error?.message || 'cannot read this process')); });
}
function reopenRecent(rec) { $('#target-name').textContent = rec.name; openTarget(rec.kind === 'dynamic' || rec.kind === 'both' ? 'dynamic' : 'static'); }
async function openFileTarget() {
  if (isNative) {
    const p = await pickFile('Choose a binary to analyze'); if (p) openBinary(p);
  } else { $('#launcher').classList.remove('hidden'); toast('Choose a target on the launcher'); }
}
function newProject() { if (isNative) openFileTarget(); else { $('#launcher').classList.remove('hidden'); toast('New project — pick a target'); } }

function recentItems() {
  const r = getRecent();
  if (!r.length) return [{ label: 'No recent targets', disabled: true, act: () => {} }];
  return r.map(rec => item(`${rec.name}  ·  ${fmtAgo(rec.ts)}`, rec.kind === 'dynamic' || rec.kind === 'both' ? 'play' : 'decomp', '', () => reopenRecent(rec)));
}
// Menu bar — one clear job per menu (no workspace duplication; workspaces are the tabs).
function menuItems(name) {
  switch (name) {
    case 'File': return [            // target lifecycle
      item('New project', 'add', '', newProject),
      item('Open file…', 'open', 'Ctrl+O', openFileTarget),
      item('Attach to process…', 'play', '', () => pickProcess()),
      item('Launch & attach…', 'play', '', () => pickProcess()),
      sep, lbl('Open recent'), ...recentItems(), sep,
      item('Settings', 'settings', 'Ctrl ,', () => openSettings()),
      item('Keyboard shortcuts', 'type', '', () => openSettings('keybindings')),
      sep,
      item('Close target', 'x', '', closeTarget),
      item('Exit', 'x', 'Ctrl+Q', () => winCtl('close'), { danger: true }),
    ];
    case 'Edit': return [            // edits to the analysis database (engine-backed)
      item('Go to address…', 'xref', 'Ctrl+G', () => doGoto()),
      sep,
      item('Rename…', 'rename', 'F2', () => doRename()),
      item('Comment…', 'comment', 'Ctrl+/', () => doComment()),
      item('Set return type…', 'type', '', () => doRetypeReturn()),
      item('Invert branch logic', 'redo', '', () => doPatch('Invert branch')),
      item('Patch → NOP…', 'patch', '', () => doPatch('NOP')),
      sep,
      item('Command palette', 'scan', 'Ctrl+Shift+P', openPal),
    ];
    case 'View': return [            // presentation only
      item('Command palette', 'scan', 'Ctrl+Shift+P', openPal),
      sep,
      item('Zoom in', 'add', 'Ctrl +', () => setZoom(zoom + 0.1)),
      item('Zoom out', 'min', 'Ctrl −', () => setZoom(zoom - 0.1)),
      item('Reset zoom', 'reset', 'Ctrl 0', () => setZoom(1)),
      sep,
      item('Themes & appearance…', 'settings', '', () => openSettings('appearance')),
    ];
    case 'Analyze': return [         // analysis actions
      item('Decompile', 'decomp', 'F5', () => { setWorkspace('decompile'); toast('Decompiling'); }),
      item('Apply FLIRT signatures', 'type', '', () => echo('sig apply --flirt zlib-1.3.1.npat', 'named 118 functions')),
      item('Find xrefs', 'xref', 'Shift+F12', () => toast('Xrefs')),
      item('Identify algorithms', 'identify', '', () => doIdentify()),
      item('Re-run analysis', 'reset', '', () => toast('Re-analyzing…')),
    ];
    case 'Debug': return [           // control of the running target
      item('Attach to process…', 'play', '', () => pickProcess()),
      item('Memory scan…', 'scan', '', () => { setWorkspace('dynamic'); if (!isLive()) pickProcess(); }),
      item('Set watchpoint', 'watch', '', () => setWorkspace('dynamic')),
      item('Find what writes…', 'xref', '', () => setWorkspace('dynamic')),
      sep,
      item('Continue', 'play', 'F5', () => toast('Continue'), { disabled: !isLive() }),
      item('Step over', 'step', 'F10', () => toast('Step over'), { disabled: !isLive() }),
      item('Step into', 'step', 'F11', () => toast('Step into'), { disabled: !isLive() }),
      item('Toggle breakpoint', 'bp', 'F9', () => toast('Toggle breakpoint')),
    ];
    case 'Window': return [          // window & pane management
      item('Add widget…', 'add', '', () => openWpal($('#btn-addw'))),
      sep,
      item('Undo layout', 'undo', 'Ctrl+Shift+Z', () => dock.undo()),
      item('Redo layout', 'redo', 'Ctrl+Shift+Y', () => dock.redo()),
      item('Reset layout', 'reset', '', () => dock.reset()),
      sep,
      item('Minimize', 'min', '', () => winCtl('min')),
      item('Maximize', 'max', '', () => winCtl('max')),
      item('Close window', 'x', '', () => winCtl('close'), { danger: true }),
    ];
    case 'Help': return [
      item('Getting started', 'book', '', () => toast('Getting started')),
      item('Glossary of terms', 'strings', '', () => toast('Glossary of RE terms')),
      item('Keyboard shortcuts', 'type', '', () => openSettings('keybindings')),
      sep,
      item('Documentation', 'book', '', () => toast('Docs')),
      item('About N0xis', 'about', '', () => toast('N0xis GUI · a thin client over the n0xis engine')),
    ];
    default: return [];
  }
}
$$('.tbar .menu').forEach(btn => btn.addEventListener('click', e => {
  e.stopPropagation();
  const r = btn.getBoundingClientRect();
  openCtx(r.left, r.bottom + 3, menuItems(btn.textContent.trim()));
}));

// ---------- global keyboard accelerators (VS Code-style) ----------
// Mainstream editor muscle-memory: F2 rename, Ctrl+/ comment, F5 run/continue,
// F10/F11 step, Shift+F12 references, Ctrl+Shift+P palette, Ctrl+O/Ctrl+,.
// Ignored inside text fields; the debugger keys need a live target.
const isEditable = t => !!t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));
const targetOpen = () => $('#launcher')?.classList.contains('hidden');
window.addEventListener('keydown', e => {   // data-driven dispatch — reads the (editable) keymap
  if (recordingId) return;                  // the rebind recorder is capturing this chord
  if (isEditable(e.target)) return;          // never hijack typing
  const combo = comboOf(e); if (!combo) return;
  for (const d of KEYDEFS) {
    if (keyOf(d.id) !== combo) continue;     // conflicts deferred: first match wins
    e.preventDefault();
    if (d.scope === 'target' && !targetOpen()) return;
    if (d.scope === 'live' && !isLive()) return;
    d.act(); return;
  }
});
// Mouse back/forward buttons (button 3 = back, 4 = forward), like every browser
// and disassembler — mirror Alt+←/→. Only when a target is open.
window.addEventListener('mouseup', e => {
  if (!targetOpen()) return;
  if (e.button === 3) { e.preventDefault(); navBackward(); }
  else if (e.button === 4) { e.preventDefault(); navForward(); }
});

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
// shared render pieces (used by the decompiler/disassembly widgets AND the switchable Code view)
const DISASM = [
  ['0x1000015c8', '48 89 d8', 'mov', 'rax, rbx', ''], ['0x1000015cb', '83 e0 07', 'and', 'eax, 0x7', ''],
  ['0x1000015ce', '74 2a', 'je', '0x1000015fa', 'hot'], ['0x1000015d0', '0f b6 03', 'movzx', 'eax, byte [rbx]', ''],
  ['0x1000015d3', '31 f0', 'xor', 'eax, esi', ''], ['0x1000015d8', '8b 04 87', 'mov', 'eax, [rdi + rax*4]', ''],
];
const pseudoInner = () => `<div class="code selectable" style="flex:1;min-height:0">${CODE_LINES.map((l, i) => `<div class="cl${i === 7 ? ' hot' : ''}"><span class="gut">${i + 1}</span><span>${l}</span></div>`).join('')}</div>`;
const disasmInner = () => `<div class="disasm selectable" style="flex:1;min-height:0;height:auto">${DISASM.map(([a, b, m, o, h]) => `<div class="drow${h ? ' hot' : ''}" data-ctx="drow"><span class="daddr">${a}</span><span class="dbytes">${b}</span><span class="dmn"${h ? ' style="color:var(--live)"' : ''}>${m}</span><span>${o}</span></div>`).join('')}</div>`;
const FUNCS = [
  ['main', '0x401060', ''], ['crc32_z', '0x1510', 'sig'], ['crc32', '0x19d0', ''],
  ['compress', '0x14d0', 'sig'], ['deflate', '0x4650', ''], ['inflate_table', '0x2a10', ''],
  ['sub_180002780', '0x2780', 'sub'], ['adler32', '0x7d70', ''], ['uncompress', '0x7850', ''],
  ['sub_180003b40', '0x3b40', 'sub'],
];
// function list is DATA (survives re-render / workspace switch / project change),
// not a one-off DOM patch. null → the built-in demo list.
let FUNCLIST = null, FUNCMETA = null, funcQuery = '';
let funcByAddr = new Map();               // normalized addr → name, for linear-view headers
const funcRows = () => FUNCLIST || FUNCS.map(([name, addr, tag]) => ({ name, addr, tag }));
function funcView() {                       // full list filtered by the search box
  const q = funcQuery.trim().toLowerCase(), rows = funcRows();
  return q ? rows.filter(f => (f.name + ' ' + f.addr).toLowerCase().includes(q)) : rows;
}
const frowHTML = f => `<div class="frow${f.tag === 'sub' ? ' sub' : ''}${f.addr === selAddr ? ' on' : ''}" data-addr="${escH(f.addr)}"><span class="sym">ƒ</span><span class="nm mono" title="${escH(f.name)}">${escH(f.name)}</span>${f.tag === 'sig' ? '<span class="badge">sig</span>' : ''}<span class="fa mono">${escH(f.addr)}</span></div>`;
const FROW_H = 27;                          // px — must match .frow height in CSS
function renderFlist(fl) {                  // virtualized: no cap, only visible rows in the DOM
  const rows = funcView(), total = rows.length;
  if (total <= 300) { fl.onscroll = null; fl.innerHTML = rows.map(frowHTML).join(''); return; }
  fl.innerHTML = `<div class="flist-vp" style="height:${total * FROW_H}px;position:relative"><div class="flist-win" style="position:absolute;left:0;right:0;top:0"></div></div>`;
  const win = fl.querySelector('.flist-win');
  const draw = () => {
    const st = fl.scrollTop, vh = fl.clientHeight || 400;
    const start = Math.max(0, Math.floor(st / FROW_H) - 8), end = Math.min(total, Math.ceil((st + vh) / FROW_H) + 8);
    win.style.transform = `translateY(${start * FROW_H}px)`;
    win.innerHTML = rows.slice(start, end).map(frowHTML).join('');
  };
  fl.onscroll = draw; draw();
}
function funcFootHTML() {
  const total = FUNCMETA ? FUNCMETA.total : funcRows().length;
  const named = FUNCMETA ? FUNCMETA.named : 86;
  const loading = FUNCMETA && FUNCMETA.shown < FUNCMETA.total;
  const extra = loading ? ` · loading ${FUNCMETA.shown.toLocaleString()}…` : (funcQuery ? ` · ${funcView().length.toLocaleString()} shown` : '');
  return `<span class="fcount">${total.toLocaleString()} functions${extra}</span><span class="grow"></span><span class="fnamed" style="color:var(--ok)">${named}% named</span>`;
}
function paintFunctions() {
  $$('#dockspace .flist').forEach(renderFlist);
  $$('#dockspace .pfoot').forEach(pf => { pf.innerHTML = funcFootHTML(); });
}
const WIDGETS = {
  functions: { title: 'Functions', icon: 'strings', body: () => `<div class="wfill">
    <div class="navtabs"><button class="nt on">Functions</button><button class="nt">Imports</button><button class="nt">Strings</button><button class="nt">Types</button></div>
    <div class="search">${svg(ICON.scan,'')}<input class="ffilter" placeholder="Filter functions…" spellcheck="false"></div>
    <div class="flist"></div>
    <div class="pfoot">${funcFootHTML()}</div></div>`,
    // NEVER dump the whole list into body() — a 371k-function binary would build
    // 371k DOM nodes at once and freeze/crash the webview. Render virtualized on mount.
    init: root => { renderFlist(root.querySelector('.flist')); root.querySelector('.pfoot').innerHTML = funcFootHTML(); } },

  decompiler: { title: 'Decompiler', icon: 'decomp', body: () => `<div class="wfill">
    <div class="dockhead"><button class="pill on" data-style="structured">structured</button><button class="pill" data-style="ssa">ssa</button><button class="pill" data-style="goto">goto</button><div class="grow"></div><button class="dt" data-ws="graph">CFG ↗</button></div>
    ${pseudoInner()}</div>` },

  disassembly: { title: 'Disassembly · x86-64', icon: 'disasm', body: () => `<div style="height:100%;display:flex;flex-direction:column">${disasmInner()}</div>` },

  // Switchable Code view (Binary Ninja-style): one pane, one dropdown, three
  // representations of the SAME function — Pseudo-C / Disassembly / Graph.
  // (Side-by-side dual view = split this pane in the tiling dock and pick another.)
  codeview: { title: 'Code', icon: 'decomp', body: () => `<div class="wfill cvw">
    <div class="dockhead cv-head"><button class="cv-sel">Pseudo-C ▾</button><div class="grow"></div><button class="dt" data-ws="graph">CFG ↗</button></div>
    <div class="cv-body" style="flex:1;min-height:0;display:flex;flex-direction:column;overflow:hidden"></div></div>`, init: root => initCodeView(root) },

  // Linear view (Binary Ninja-style): the WHOLE binary as one continuous listing,
  // functions flowing into one another, lazily disassembled as you scroll, with a
  // VS Code-style minimap on the right.
  linear: { title: 'Linear · listing', icon: 'disasm', body: () => `<div class="lin-outer"><div class="lin-bar"><button data-mode="asm" class="on">Asm</button><button data-mode="pseudo">Pseudo</button><span class="lin-bar-hint">continuous listing · whole program</span></div><div class="lin-wrap"><div class="linear selectable mono"></div><canvas class="lin-map" width="72"></canvas></div></div>`, init: root => initLinear(root) },

  hex: { title: 'Hex', icon: 'hex', body: () => `<div class="hex selectable" style="height:100%"><div class="hx"><span class="hxa">7FF6C21A40</span><span class="hxb"><span class="hb-hi">57 00 00 00</span> 2a 00 00 00 5c 21 c2 f6</span><span class="hxc">W....*...\\!</span></div><div class="hx"><span class="hxa">7FF6C21A50</span><span class="hxb">64 00 00 00 00 00 80 3f 00 00 80 3f</span><span class="hxc">d......?...?</span></div></div>` },

  graph: { title: 'Control-flow graph', icon: 'graph', body: () => `<div class="graphwrap" style="height:100%">
    <div class="gviewport"><div class="gcanvas"><svg class="gsvg" width="920" height="700"></svg></div>
    <div class="glegend"><div class="lg-h">Legend</div><span class="lg"><span class="ln t"></span>true branch</span><span class="lg"><span class="ln f"></span>false branch</span><span class="lg"><span class="ln u"></span>unconditional</span><span class="lg"><span class="ln loop"></span>loop back-edge</span><span class="lg" style="color:var(--tx2)">drag blocks · scroll = zoom</span></div>
    <div class="gctl">
      <button class="gc-legend" title="Legend">${svg('M8 6h13;M8 12h13;M8 18h13;M3 6h.01;M3 12h.01;M3 18h.01','')}</button>
      <button class="gc-curve" title="Rounded edges (beta)">${svg('M4 19 C 9 5, 15 5, 20 19','')}<span class="gc-beta">β</span></button>
    </div>
    <div class="gzoom"><button class="gz-in">+</button><div class="gzl">100%</div><button class="gz-out">−</button><button class="gz-fit">${svg('M4 9V5a1 1 0 0 1 1-1h4;M20 9V5a1 1 0 0 0-1-1h-4;M4 15v4a1 1 0 0 0 1 1h4;M20 15v4a1 1 0 0 1-1 1h-4','')}</button></div></div></div>`,
    init: root => initGraphWidget(root) },

  copilot: { title: 'Copilot', icon: 'chat', body: () => `<div class="wfill">
    <div class="chat" style="flex:1"><div class="msg"><div class="av me">ME</div><div class="bub mine">What does <span class="mono" style="color:var(--acc)">crc32_z</span> do?</div></div>
    <div class="msg"><div class="av ai">AI</div><div><div class="bub">This is the <b>zlib CRC-32</b> checksum core. It folds each input byte through a 256-entry table (<span class="mono" style="color:var(--st)">crc_table</span>) — byte-by-byte until 8-aligned, then 8 bytes per pass.<div style="margin-top:6px;color:var(--tx1)">• <span class="mono">rsi</span> = buffer, <span class="mono">rdx</span> = length, <span class="mono">rdi</span> = seed.</div></div>
    <div class="sug"><span class="sugb">Find callers</span><span class="sugb">Explain “SSA”</span><span class="sugb">Rename vars</span></div></div></div></div>
    <div class="ai-scopebar" title="What the AI can see: the selected function, or the lines you select in the Linear view">${svg('M12 2a10 10 0 1 0 0 20a10 10 0 0 0 0-20;M12 8v4;M12 16h.01','')}<span class="ai-scope">whole binary</span></div>
    <div class="ask"><span class="provsel">Claude · cloud</span>Ask, or “guide me through…”<span class="grow"></span><span style="color:var(--acc)">↵</span></div></div>`,
    init: () => refreshAiScope() },

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
    <div class="wprow" data-ctx="wprow"><span class="wpico">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C21A40</div><div style="color:var(--tx2);font-size:.68rem">write · retries</div></div><span class="hit">1,204</span></div>
    <div class="wprow" data-ctx="wprow"><span class="wpico" style="color:var(--acc)">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C218E0</div><div style="color:var(--tx2);font-size:.68rem">read · timeout_ms</div></div><span class="hit" style="color:var(--acc);background:rgba(63,220,196,.1)">86</span></div>
    <div class="wprow" data-ctx="wprow"><span class="wpico" style="color:var(--dgr)">${svg(ICON.bp,'')}</span><div><div class="mono">7FF6C1002A</div><div style="color:var(--tx2);font-size:.68rem">execute · sub_1002A</div></div><span class="hit" style="color:var(--dgr);background:rgba(255,107,122,.1)">hit</span></div></div>` },

  scanner: { title: 'Memory scanner', icon: 'scan', body: () => `<div class="wfill scanw">
    <div class="scanbar">
      <div class="field"><input class="inp scan-val" placeholder="value…" value=""><select class="scan-type selbox">${['i8', 'u8', 'i16', 'u16', 'i32', 'u32', 'i64', 'u64', 'f32', 'f64'].map(t => `<option${t === 'i32' ? ' selected' : ''}>${t}</option>`).join('')}</select></div>
      <div class="field"><button class="b p scan-new" style="flex:1">New scan</button><button class="b g scan-next" style="flex:1">Next scan</button></div>
      <div class="scan-stat" style="color:var(--tx2);font-size:.75rem">not scanning — attach a process to begin</div>
    </div>
    <div class="scan-res" style="overflow:auto;flex:1"></div></div>`, init: root => initScanner(root) },

  livemem: { title: 'Live memory · 7FF6C21A40', icon: 'hex', body: () => `<div class="hex selectable" style="height:100%">
    <div class="hx"><span class="hxa">7FF6C21A40</span><span class="hxb"><span class="hb-hi">57 00 00 00</span> 2a 00 00 00 5c 21 c2 f6 ff 7f 00 00</span><span class="hxc">W....*...\\!......</span></div>
    <div class="hx"><span class="hxa">7FF6C21A50</span><span class="hxb">64 00 00 00 00 00 80 3f 00 00 80 3f cd cc 4c 3e</span><span class="hxc">d......?...?..L&gt;</span></div>
    <div class="hx"><span class="hxa">7FF6C21A60</span><span class="hxb">10 27 00 00 e8 03 00 00 01 00 00 00 00 00 00 00</span><span class="hxc">.'..............</span></div>
    <div class="hx"><span class="hxa">7FF6C21A80</span><span class="hxb">43 6f 6e 6e 65 63 74 69 6f 6e 50 6f 6f 6c 00 00</span><span class="hxc" style="color:var(--ok)">ConnectionPool..</span></div></div>` },

  watchlist: { title: 'Watchlist', icon: 'watch', body: () => `<div style="overflow:auto;height:100%">${[
      ['retries','3','on'],['max_retries','5','on'],['timeout_ms','30000',''],['buffer_len','4096','']
    ].map(([n,v,f])=>`<div class="wlrow" data-ctx="wlrow"><span class="frz ${f}"></span><span class="wname">${n}</span><span class="wval">${v}</span></div>`).join('')}</div>` },

  strings: { title: 'Strings', icon: 'strings', body: () => `<div style="overflow:auto;height:100%;padding:6px 0">${['ConnectionPool','ParseHeader','crc_table','zlib 1.3.1','deflate','Assertion failed'].map(s=>`<div class="frow"><span class="nm mono selectable">"${s}"</span></div>`).join('')}</div>` },

  notes: { title: 'Notes', icon: 'note', body: () => `<textarea class="selectable" style="width:100%;height:100%;background:transparent;border:none;color:var(--tx0);padding:11px;font:inherit;resize:none;outline:none;box-sizing:border-box" placeholder="Session notes…"></textarea>` },

  // Triage — a view over `profile`: image facts, sections, exports, runtime hints,
  // and the advisories that say which commands will be ineffective and why.
  triage: { title: 'Triage · overview', icon: 'chip', body: () => `<div class="triage-root selectable" style="height:100%;overflow:auto">${triageHTML(TRIAGE)}</div>` },

  // Console — the process seam made interactive: run ANY n0xis command, see the
  // {ok,data,meta} envelope pretty-printed (bounded so a huge result can't flood).
  console: { title: 'Console · n0xis', icon: 'term', body: () => `<div class="nxc"><div class="nxc-log selectable"></div><div class="nxc-inp"><span class="nxc-ps mono">n0xis›</span><input class="nxc-in mono" spellcheck="false" placeholder="profile · guide scan · xref to --addr … · function discover"></div></div>`, init: root => initConsole(root) },

  // Cross-references — `xref to` (who reaches this address) + `xref from` (where
  // it goes). Follows the selected symbol; refreshes on demand.
  xrefs: { title: 'Cross-references', icon: 'xref', body: () => `<div class="xrefs" style="height:100%;display:flex;flex-direction:column"><div class="xr-hd"><span class="dk-mk">${svg(ICON.xref, '')}</span><span class="xr-addr mono">${escH(selAddr)}</span><span class="dk-grow" style="flex:1"></span><button class="xr-refresh" title="Refresh">${svg(ICON.reset, '')}</button></div><div class="xr-cols selectable"><div class="xr-col"><div class="xr-t">referenced by</div><div class="xr-to"></div></div><div class="xr-col"><div class="xr-t">refers to</div><div class="xr-from"></div></div></div></div>`, init: root => initXrefs(root) },

  // Variables — typed locals/params recovered by the decompiler for the selection.
  variables: { title: 'Variables', icon: 'type', body: () => `<div class="vars selectable" style="height:100%;overflow:auto"></div>`, init: root => { paintVars(root.querySelector('.vars')); } },

  // Stack — a call frame view. Live backtrace (`stack backtrace --pid`) once a
  // process is attached; a static frame sketch until then.
  stack: { title: 'Stack', icon: 'regs', body: () => `<div class="stk selectable" style="height:100%;overflow:auto"></div>`, init: root => { paintStack(root.querySelector('.stk')); } },

  // Bookmarks / Notes — every address the user has annotated (renamed, commented,
  // typed, or bookmarked). Click a row to jump there. The saved-work index; it
  // reads the same `.n0x/annotations.json` everything else writes to.
  bookmarks: { title: 'Bookmarks', icon: 'bookmark', body: () => `<div class="bmk" style="height:100%;display:flex;flex-direction:column"><div class="bmk-hd"><span class="dk-mk">${svg(ICON.bookmark, '')}</span><span class="bmk-t">names · comments · bookmarks</span><span style="flex:1"></span><button class="bmk-refresh" title="Refresh">${svg(ICON.reset, '')}</button></div><div class="bmk-list selectable" style="flex:1;overflow:auto"></div></div>`, init: root => initBookmarks(root) },

  // Types — user-defined struct / enum definitions the decompiler uses to render
  // named struct fields (`p->count`). Define one, add fields, then right-click a
  // variable ▸ Set type to a pointer to it.
  types: { title: 'Types', icon: 'type', body: () => `<div class="typ" style="height:100%;display:flex;flex-direction:column"><div class="typ-hd"><button class="typ-add-s">+ Struct</button><button class="typ-add-e">+ Enum</button><span style="flex:1"></span><button class="typ-refresh" title="Refresh">${svg(ICON.reset, '')}</button></div><div class="typ-list selectable" style="flex:1;overflow:auto"></div></div>`, init: root => initTypes(root) },
};

/* ---- Triage: render the `profile` envelope (demo data until a real target) ---- */
const escH = s => String(s).replace(/[&<>]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
const hx = n => typeof n === 'number' ? '0x' + n.toString(16) : (n ?? '');
let TRIAGE = {
  source: 'demo · no target open', image: {
    machine: 'x64', module_base: '0x140000000', image_end: '0x14a2c000',
    pdata_present: true, pdata_functions: 48213, thunk_count: 1120,
    export_count: 3, export_distinct_addresses: 3,
    sections: [
      { name: '.text', va: '0x140001000', raw_size: 0x9fc00, virtual_size: 0x9fb42 },
      { name: '.rdata', va: '0x1400a1000', raw_size: 0x2a000, virtual_size: 0x29e10 },
      { name: '.data', va: '0x1400cb000', raw_size: 0x1a000, virtual_size: 0x4c200 },
      { name: '.pdata', va: '0x140117000', raw_size: 0x24000, virtual_size: 0x23f40 },
      { name: '.rsrc', va: '0x14013b000', raw_size: 0x1200, virtual_size: 0x1180 },
    ],
    exports: [{ name: 'CreateResetEvent', va: '0x1800080f0' }, { name: 'DllCanUnloadNow', va: '0x1800017f0' }, { name: 'GetHandleVerifier', va: '0x180002210' }],
    engine_hints: ['msvc-2022', 'statically-linked zlib 1.3.1'], folded: [], detoured_exports: [],
  },
  advisories: [{ code: 'demo', message: 'Open a target (File → Open file) to profile it for real.' }],
};
function triageHTML(p) {
  const im = p.image || {};
  const secs = im.sections || [], exps = im.exports || [], hints = im.engine_hints || [], adv = p.advisories || [];
  const kb = n => (typeof n === 'number' ? (n >= 1024 ? (n / 1024).toFixed(n >= 1048576 ? 0 : 1) + ' KB' : n + ' B') : n);
  const fact = (k, v) => `<div class="tg-fact"><span class="tg-fk">${k}</span><span class="tg-fv mono">${v}</span></div>`;
  const secRows = secs.map(s => `<div class="tg-sec"><span class="mono tg-sn">${escH(s.name || '—')}</span><span class="mono">${s.va}</span><span class="mono tg-ss">${kb(s.raw_size)}</span><span class="mono tg-ss">${kb(s.virtual_size)}</span></div>`).join('');
  const expCap = exps.slice(0, 40);
  const expRows = expCap.map(e => `<div class="tg-exp"><span class="fnc mono">${escH(e.name)}</span><span class="mono tg-ea">${e.va}</span></div>`).join('') +
    (exps.length > expCap.length ? `<div class="tg-more">+${exps.length - expCap.length} more · Console: <span class="mono">profile --exports</span></div>` : '');
  return `
    <div class="tg-head"><span class="chip-b mono">${escH(im.machine || '?')}</span><span class="tg-src mono">${escH(p.source || '')}</span></div>
    ${adv.length ? `<div class="tg-adv">${adv.map(a => { const msg = typeof a === 'string' ? a : (a.message || a.detail || a.text || JSON.stringify(a)); return `<div class="tg-adv-i"><span class="mono tg-ac">${escH(typeof a === 'object' ? (a.code || 'note') : 'note')}</span>${escH(msg)}</div>`; }).join('')}</div>` : ''}
    <div class="tg-grid">
      ${fact('Module base', im.module_base || '?')}
      ${fact('Image end', im.image_end || '?')}
      ${fact('.pdata funcs', (im.pdata_functions ?? '?') + (im.pdata_present ? '' : ' (none)'))}
      ${fact('Exports', im.export_count ?? exps.length)}
      ${fact('Thunks', im.thunk_count ?? '?')}
      ${fact('Distinct addrs', im.export_distinct_addresses ?? '?')}
    </div>
    ${hints.length ? `<div class="tg-hints">${hints.map(h => `<span class="tg-chip">${escH(h)}</span>`).join('')}</div>` : ''}
    <div class="tg-t">Sections</div>
    <div class="tg-sec tg-sh"><span>name</span><span>va</span><span>raw</span><span>virtual</span></div>
    ${secRows || '<div class="tg-more">no section table (x86 fallback)</div>'}
    ${exps.length ? `<div class="tg-t">Exports</div>${expRows}` : ''}`;
}
function paintTriage() { $$('.triage-root').forEach(el => { el.innerHTML = triageHTML(TRIAGE); }); }

/* ---- current target + real profile load ---- */
let curPath = '';
function setTarget(path) { curPath = path || ''; loadProfile(); }
async function loadProfile() {
  if (!isNative || !curPath) return;
  const r = await engine.profile(curPath);
  if (r && r.ok && r.data) {
    TRIAGE = { ...r.data, source: r.meta?.source || curPath.split(/[\\/]/).pop() }; paintTriage();
    const m = $('#sb-mode'); if (m && TRIAGE.image?.machine) m.textContent = `static · PE ${TRIAGE.image.machine}`;
    toast('Profiled ' + TRIAGE.source);
  }
  else if (r && !r.ok) toast('Profile failed: ' + (r.error?.message || 'unknown'));
}

/* ---- Console: interactive n0xis REPL (bounded output) ---- */
function tokenize(line) {                       // minimal shell-ish split with quotes
  const out = []; const re = /"([^"]*)"|'([^']*)'|(\S+)/g; let m;
  while ((m = re.exec(line))) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}
const CONSOLE_CAP = 16000;                      // never flood the pane / the model's eyes
function initConsole(root) {
  const log = root.querySelector('.nxc-log'), inp = root.querySelector('.nxc-in');
  const line = (html, cls = '') => { const d = document.createElement('div'); d.className = 'oline ' + cls; d.innerHTML = html; log.appendChild(d); log.scrollTop = log.scrollHeight; };
  if (!log.dataset.init) {
    log.dataset.init = '1';
    line(isNative ? 'n0xis engine · type a command, or <span class="mono">guide</span> for the catalog' : 'web preview — engine offline; commands echo only', 'dim');
  }
  const hist = []; let hi = -1;
  inp.addEventListener('keydown', async e => {
    if (e.key === 'ArrowUp') { if (hi < hist.length - 1) inp.value = hist[++hi] || ''; e.preventDefault(); return; }
    if (e.key === 'ArrowDown') { if (hi > 0) inp.value = hist[--hi] || ''; else { hi = -1; inp.value = ''; } e.preventDefault(); return; }
    if (e.key !== 'Enter') return;
    const raw = inp.value.trim(); if (!raw) return;
    inp.value = ''; hist.unshift(raw); hi = -1;
    let args = tokenize(raw);
    // convenience: a static command with no target gets the current --file
    const bare = !args.some(a => a === '--file' || a === '--pid') && curPath && !/^(guide|doctor|--version|version)$/.test(args[0] || '');
    if (bare) args = [...args, '--file', curPath];
    line(`<span class="p">›</span> ${escH(raw)}${bare ? ` <span class="dim">--file ${escH(curPath.split(/[\\/]/).pop())}</span>` : ''}`);
    if (!isNative) { line('engine offline — native app only', 'dim'); return; }
    line('<span class="dim">running…</span>', 'run-tmp');
    const res = await n0x(args);
    root.querySelector('.run-tmp')?.remove();
    if (res === null) { line('no engine', 'dim'); return; }
    let js = JSON.stringify(res, null, 2);
    const clipped = js.length > CONSOLE_CAP;
    if (clipped) js = js.slice(0, CONSOLE_CAP);
    line(`<pre class="nxc-json ${res.ok ? 'ok' : 'err'}">${escH(js)}</pre>${clipped ? '<div class="dim">… output clipped (' + CONSOLE_CAP + ' chars) — narrow it with flags/--limit</div>' : ''}`);
  });
}

/* ---- Xrefs / Variables / Stack + live decompiler (engine-backed) ---- */
// The reverse-xref index is built by `analyze`. Querying `xref to` before it
// exists makes the ENGINE build it inside the session — 18.6 s on AyuGram
// (2.2 M targets, measured) — and because `session_query` holds one mutex for the
// whole request, every decompile queues behind it and the whole UI sits in
// "Loading". So we don't ask until the index is there.
let xrefIndexReady = false;
let XREF = {
  to: [{ from: '0x14e6', to: '0x1510', kind: 'call', text: 'call crc32_z' }, { from: '0x1a70', to: '0x1510', kind: 'call', text: 'call crc32_z' }],
  from: [{ from: '0x1510', to: '0x1002a', kind: 'call', text: 'call sub_1002A' }, { from: '0x1510', to: '0x1560', kind: 'cond_jmp', text: 'je short 0x1560' }],
};
const xrefRow = (r, dir) => { const a = dir === 'to' ? r.from : r.to; const nm = (dir === 'from' && r.sym) ? ` <span class="dsym" style="color:var(--acc)">${escH(r.sym)}</span>` : ''; return `<div class="xr-r" data-ctx="frow" data-addr="${escH(a)}"><span class="xr-k">${escH(r.kind || '')}</span><span class="fnc mono">${escH(a)}${nm}</span><span class="xr-tx mono">${escH(r.text || '')}</span></div>`; };
function paintXrefs(scope) {                         // scope: one widget root, or all mounted
  const roots = scope ? [scope] : $$('.xrefs');
  roots.forEach(r => {
    const addr = r.querySelector('.xr-addr'), to = r.querySelector('.xr-to'), from = r.querySelector('.xr-from');
    if (addr) addr.textContent = selAddr;
    if (to) to.innerHTML = XREF.to.length ? XREF.to.map(x => xrefRow(x, 'to')).join('') : '<div class="xr-none">no references</div>';
    if (from) from.innerHTML = XREF.from.length ? XREF.from.map(x => xrefRow(x, 'from')).join('') : '<div class="xr-none">no references</div>';
  });
}
function initXrefs(root) { paintXrefs(root.querySelector('.xrefs') || root); root.querySelector('.xr-refresh')?.addEventListener('click', loadXrefs); if (isNative && curPath) loadXrefs(); }
async function loadXrefs(seq) {
  if (!isNative || !curPath) { paintXrefs(); return; }
  // Index not built yet — skip rather than block the session (see `xrefIndexReady`).
  // `startAnalysis` re-runs this the moment the index is ready.
  if (!xrefIndexReady) { XREF = { to: [], from: [] }; paintXrefs(); return; }
  const [t, f] = await Promise.all([
    n0xCached('xt|' + curPath + '|' + selAddr, ['xref', 'to', '--file', curPath, '--addr', selAddr]),
    n0xCached('xf|' + curPath + '|' + selAddr, ['xref', 'from', '--file', curPath, '--addr', selAddr]),
  ]);
  if (seq !== undefined && seq !== selSeq) return;
  if (t?.ok) XREF.to = t.data.refs || []; if (f?.ok) XREF.from = f.data.refs || [];
  paintXrefs();
}

let VARS = [{ n: 'rdi', t: 'uint64_t', k: 'param' }, { n: 'rsi', t: 'void*', k: 'param' }, { n: 'rdx', t: 'uint64_t', k: 'param' }, { n: 'v14', t: '—', k: 'local' }, { n: 'v9', t: '—', k: 'local' }];
const VAR_BADGE = { arg: 'A', stack: 'S', reg: 'R' };   // Argument · Stack · Register (Binary-Ninja taxonomy)
function paintVars(el) {
  if (!el) { $$('.vars').forEach(paintVars); return; }
  const n = { arg: 0, stack: 0, reg: 0 }; VARS.forEach(v => n[v.k] = (n[v.k] || 0) + 1);
  const head = VARS.length ? `<div class="vars-h">${n.arg} args · ${n.stack} stack · ${n.reg} reg</div>` : '';
  el.innerHTML = head + (VARS.length ? VARS.map(v => `<div class="var-r" data-ctx="varrow" data-addr="${escH(selAddr)}" data-var="${escH(v.n)}"><span class="var-k ${v.k}" title="${v.k}">${VAR_BADGE[v.k] || '?'}</span><span class="ty mono">${escH(v.t)}</span><span class="fnc mono">${escH(v.n)}</span></div>`).join('') : '<div class="xr-none">no variables</div>');
}
// Classify variables the way BN does: Arguments (signature), Stack (var_XXXX =
// frame offsets), Register (vN / rax_N = SSA temporaries), with types lifted from
// the pseudocode declarations where the engine emits them.
function parseVars(sig, pseudo) {
  const out = [], at = new Map();          // name → index in out
  const text = (pseudo || []).join('\n');
  const add = (n, t, k) => { if (at.has(n)) { const v = out[at.get(n)]; if ((v.t === '—' || v.t === '?') && t && t !== '—') v.t = t; } else { at.set(n, out.length); out.push({ n, t: t || '—', k }); } };
  const m = /\(([^)]*)\)/.exec(sig || '');
  if (m && m[1].trim() && !/^\s*void\s*$/.test(m[1])) m[1].split(',').forEach(p => { const parts = p.trim().split(/\s+/); const nm = parts.pop(); const t = parts.join(' ') || '?'; if (nm) add(nm.replace(/^[*&]+/, ''), t, 'arg'); });
  // typed declarations: "int32_t var_10c8", "int64_t* rsi = …", "char var_1004_1"
  const typeRe = /\b((?:u?int(?:8|16|32|64|128)_t|void|char|bool|float|double|long|short|int)(?:\s*\*+)?)\s+(var_[0-9a-fA-F]+(?:_\d+)?|[a-z][a-z0-9]*_\d+|v\d+)\b/g;
  let g; while ((g = typeRe.exec(text))) { const t = g[1].replace(/\s+/g, ''), n = g[2]; add(n, t, /^var_/.test(n) ? 'stack' : 'reg'); }
  // remaining bare tokens with no declared type
  text.replace(/\b(var_[0-9a-fA-F]+(?:_\d+)?|v\d+)\b/g, x => { add(x, '—', /^var_/.test(x) ? 'stack' : 'reg'); return x; });
  return out;
}

let STACK = [{ off: '-0x08', t: 'void*', n: '__return_addr' }, { off: '-0x18', t: 'uint64_t', n: '__saved_rbx' }, { off: '-0x40', t: 'char[32]', n: 'var_40' }, { off: '-0x48', t: 'uint64_t', n: 'v14' }];
function paintStack(el) {
  if (!el) { $$('.stk').forEach(paintStack); return; }
  el.innerHTML = `<div class="stk-note">${isLive() ? 'live backtrace' : 'static frame · attach a process for a live backtrace'}</div>` +
    STACK.map(s => `<div class="stk-r"><span class="stk-o mono">${escH(s.off)}</span><span class="ty mono">${escH(s.t)}</span><span class="fnc mono">${escH(s.n)}</span></div>`).join('');
}

// Render engine pseudocode readably: PRESERVE indentation (HTML would eat the
// leading spaces — that's the 'everything on one plane' bug), colourize, and
// turn unsigned wrap offsets (field_0xffff…f0) back into signed (-0x10).
function highlightPseudo(raw) {
  let s = raw.replace(/0x(f{4,}[0-9a-fA-F]{0,4})\b/g, m => {
    try { const hex = m.slice(2), bits = hex.length * 4, v = BigInt('0x' + hex) - (1n << BigInt(bits)); if (v < 0n && v > -0x100000000n) return '-0x' + (-v).toString(16); } catch {}
    return m;
  });
  let comment = ''; const ci = s.indexOf('//');
  if (ci >= 0) { comment = s.slice(ci); s = s.slice(0, ci); }
  s = escH(s)
    .replace(/\b(if|else|while|for|do|switch|case|default|return|goto|break|continue)\b/g, '<span class="k">$1</span>')
    .replace(/\b(uint(?:8|16|32|64)_t|int(?:8|16|32|64)_t|void|unsigned|signed|char|bool|float|double|long|short|int|struct|DWORD|QWORD|WORD|BYTE)\b/g, '<span class="ty">$1</span>')
    .replace(/(-?0x[0-9a-fA-F]+|\b\d+\b)/g, '<span class="nu">$1</span>');
  return s + (comment ? `<span class="cm">${escH(comment)}</span>` : '');
}
function paintDecomp(lines) {
  const html = lines.map((l, i) => `<div class="cl"><span class="gut">${i + 1}</span><span class="cc mono">${highlightPseudo(l)}</span></div>`).join('');
  $$('.code').forEach(code => { code.innerHTML = html; });
}
let decompStyle = 'structured', selSeq = 0;
function paintDecompMsg(msg) { $$('.code').forEach(code => { code.innerHTML = `<div class="cl"><span class="gut"></span><span class="mono" style="color:var(--tx2)">${escH(msg)}</span></div>`; }); }
// per-(command,file,addr) result cache → revisiting a function is instant, no re-spawn
const engCache = new Map(); const ENG_CACHE_MAX = 800;
let sessionOn = false;   // a persistent `n0xis serve` process is loaded for curPath
// route through the resident session when available (image loaded once), else one-shot
// global "Loading" indicator — honest feedback ONLY for a slow interactive wait.
// Debounced so it never strobes: it appears only after a query has been in flight
// ~180ms continuously (so instant/cached calls and the fast background-analysis
// stream — which the status bar already reports — never flash it), and once shown
// it stays put for at least 320ms so it fades cleanly instead of blinking.
let inflight = 0, loadEl = null, loadTimer = null, loadShownAt = 0;
function setLoading(on) {
  inflight = Math.max(0, inflight + (on ? 1 : -1));
  if (!loadEl) { loadEl = document.createElement('div'); loadEl.id = 'loading-ind'; loadEl.innerHTML = '<span class="spin"></span><span>Loading…</span>'; document.body.appendChild(loadEl); }
  if (inflight > 0) {
    if (!loadEl.classList.contains('on') && !loadTimer)
      loadTimer = setTimeout(() => { loadTimer = null; if (inflight > 0) { loadEl.classList.add('on'); loadShownAt = Date.now(); } }, 180);
  } else {
    if (loadTimer) { clearTimeout(loadTimer); loadTimer = null; }
    if (loadEl.classList.contains('on')) {
      const held = Date.now() - loadShownAt, minOn = 320;
      if (held >= minOn) loadEl.classList.remove('on');
      else setTimeout(() => { if (inflight === 0) loadEl.classList.remove('on'); }, minOn - held);
    }
  }
}
async function eng(args) {
  setLoading(true);
  try {
    if (!(sessionOn && isNative)) return await n0x(args);
    const r = await sessionQuery(args);
    // The Rust side kills the session on ANY io error, but `sessionOn` never got
    // cleared — so every later call kept asking a dead session and returned
    // `{ok:false,"no session"}` instead of degrading. Fall back to a one-shot
    // (and stop using the session) whenever the SESSION is what failed, not the
    // command. Slower per call, but the app keeps working.
    if (r && r.ok === false && /no session|session (closed|write|read)/i.test(r.error?.message || '')) {
      sessionOn = false;
      return await n0x(args);
    }
    return r;
  }
  finally { setLoading(false); }
}
async function n0xCached(key, args) {
  if (engCache.has(key)) { const v = engCache.get(key); engCache.delete(key); engCache.set(key, v); return v; }  // LRU touch
  const r = await eng(args);
  if (r && r.ok) { if (engCache.size >= ENG_CACHE_MAX) engCache.delete(engCache.keys().next().value); engCache.set(key, r); }
  return r;
}
async function loadDecomp(seq) {
  if (!isNative || !curPath) return;                 // keep the demo pseudo in the web preview
  const key = 'decomp|' + curPath + '|' + selAddr + '|' + decompStyle, cached = engCache.has(key);
  if (!cached) paintDecompMsg('decompiling ' + (selName || selAddr) + '…');   // no flash on a cache hit
  const r = await n0xCached(key, ['decomp', 'pseudo', '--file', curPath, '--addr', selAddr, '--style', decompStyle]);
  if (seq !== undefined && seq !== selSeq) return;    // a newer selection superseded this one
  if (r?.ok && Array.isArray(r.data.pseudo)) { paintDecomp(r.data.pseudo); VARS = parseVars(r.data.signature, r.data.pseudo); paintVars(); }
  else paintDecompMsg('// no decompilation for ' + selAddr + (r?.error ? ' — ' + r.error.message : ''));
}
// A branch/call whose target is a *named* function (recovered RTTI, a user
// rename) renders `→ Name` beside the operand — the engine now names the whole
// function list, so this resolves against it. A bare `sub_…` adds nothing, so it
// is left off. Best-effort: a target not in the currently-loaded page stays plain.
function calleeName(target) {
  if (!target) return '';
  const n = funcByAddr.get(String(target).toLowerCase());
  return (n && !/^sub_/i.test(n)) ? n : '';
}
const symArrow = target => { const n = calleeName(target); return n ? ` <span class="dsym" style="color:var(--acc)">→ ${escH(n)}</span>` : ''; };
// A user comment on this address (from `annotate comment`) rides on the row as
// `; text`, muted, like a source comment.
const cmtSpan = c => c ? ` <span class="dcmt" style="color:var(--tx2);font-style:italic">; ${escH(c)}</span>` : '';
function paintDisasm(insns) {
  const html = insns.map(i => {
    const op = (i.text || '').slice((i.mnemonic || '').length).trim();
    return `<div class="drow" data-ctx="drow" data-addr="${escH(i.va || '')}"><span class="daddr">${escH(i.va || '')}</span><span class="dbytes">${escH(i.bytes || '')}</span><span class="dmn">${escH(i.mnemonic || '')}</span><span>${escH(op)}${symArrow(i.target)}${cmtSpan(i.comment)}</span></div>`;
  }).join('');
  $$('.disasm').forEach(d => { d.innerHTML = html; });
}
async function loadDisasm(seq) {
  if (!isNative || !curPath) return;
  const r = await n0xCached('disasm|' + curPath + '|' + selAddr, ['disasm', '--file', curPath, '--addr', selAddr, '--count', '40']);
  if (seq !== undefined && seq !== selSeq) return;
  if (r?.ok && Array.isArray(r.data.insns)) paintDisasm(r.data.insns);
  else $$('.disasm').forEach(d => { d.innerHTML = `<div class="drow" style="color:var(--tx2)">no disassembly for ${escH(selAddr)}</div>`; });
}

/* ---- Linear view: the whole binary as one continuous listing + a minimap ---- */
const linears = [];
// The Linear view shows the WHOLE program as one continuous listing. It never
// truncates on selection — it keeps a sliding window of whole functions that grows
// UP (scroll up / prepend) and DOWN (scroll down / append), anchored on the sorted
// function table, and simply centres on the selected function. The minimap maps the
// ENTIRE function table at once (full extent immediately) and fills in as bodies load.
const vaNum = a => parseInt(String(a || '').replace(/[^0-9a-fx]/gi, ''), 16) || 0;

// ---- AI context scope: the AI sees ONLY the selected function, or, when lines are
// selected in the Linear view, exactly that address range. Shown as a chip in Copilot.
let linSel = null;                          // {a,b} numeric addresses, or null
const linWarmed = new Set();                // numeric fn addrs whose disassembly is cached (fills the minimap)
function aiScopeText() {
  if (linSel) { const a = Math.min(linSel.a, linSel.b), b = Math.max(linSel.a, linSel.b); return a === b ? `line @ 0x${a.toString(16)}` : `lines 0x${a.toString(16)}–0x${b.toString(16)}`; }
  if (selName || selAddr) return (selName || selAddr) + (selAddr && selName ? ' · ' + selAddr : '');
  return 'whole binary';
}
function refreshAiScope() { $$('.ai-scope').forEach(el => { el.textContent = aiScopeText(); }); }

function initLinear(root) {
  const view = root.querySelector('.linear'), map = root.querySelector('.lin-map'), wrap = root.querySelector('.lin-wrap');
  let FN = [], fnLen = -1;                  // sorted anchor table — rebuilt when the function list grows
  const fnCache = new Map();               // fi → [rowObj]  (whole-function disassembly, cached)
  let loFi = 0, hiFi = -1, rows = [], loading = false;
  let mapDrag = false;                      // true while the minimap thumb is being dragged (see the handler)
  let linMode = 'asm';                       // 'asm' = disassembly listing | 'pseudo' = continuous decompiled C
  // The widget may mount before the function list has finished streaming (it's a tab
  // built up front). Re-sync from the live FUNCLIST whenever it changes size.
  function syncFN() {
    const src = FUNCLIST || [];
    if (src.length === fnLen) return false;
    fnLen = src.length;
    FN = src.map(f => ({ addr: f.addr, name: f.name, n: vaNum(f.addr), end: vaNum(f.end) })).filter(f => f.n).sort((a, b) => a.n - b.n);
    fnCache.clear(); loFi = 0; hiFi = -1; rows = [];
    return true;
  }
  const MAXROWS = 5000;                    // sliding-window cap so the DOM stays light
  const fiOf = n => { let lo = 0, hi = FN.length - 1, r = 0; while (lo <= hi) { const m = (lo + hi) >> 1; if (FN[m].n <= n) { r = m; lo = m + 1; } else hi = m - 1; } return r; };
  const lineHTML = (ins, fnName) => {
    const va = ins.va || '', op = (ins.text || '').slice((ins.mnemonic || '').length).trim();
    return (fnName ? `<div class="lin-fn" data-addr="${escH(va)}" title="${escH(fnName)}">${escH(fnName)}</div>` : '') +
      `<div class="lin-row${fnName ? ' fnstart' : ''}" data-ctx="drow" data-addr="${escH(va)}"><span class="daddr">${escH(va)}</span><span class="dmn">${escH(ins.mnemonic || '')}</span><span class="lin-op">${escH(op)}${symArrow(ins.target)}${cmtSpan(ins.comment)}</span></div>`;
  };
  async function fetchFn(fi) {
    if (fnCache.has(fi)) return fnCache.get(fi);
    const f = FN[fi]; if (!f || !isNative || !curPath) return [];
    if (linMode === 'pseudo') return fetchFnPseudo(fi, f);
    // Stop at the function's REAL end. `.pdata` gives an exact `end`; without it,
    // the next function's start. This is what keeps the listing from disassembling
    // the int3/zero padding after `ret` as bogus `add [rax],al` — and stops the
    // LAST function from running 0x400 bytes into the section's zero tail.
    const nextN = FN[fi + 1]?.n ?? Infinity;
    const stop = (f.end && f.end > f.n) ? Math.min(f.end, nextN) : (Number.isFinite(nextN) ? nextN : f.n + 0x400);
    const approx = Math.max(24, Math.min(2000, Math.ceil((stop - f.n) / 2)));
    const r = await n0xCached('lin|' + curPath + '|' + f.addr, ['disasm', '--file', curPath, '--addr', f.addr, '--count', String(approx)]);
    let ins = (r?.ok && Array.isArray(r.data.insns)) ? r.data.insns : [];
    ins = ins.filter(x => vaNum(x.va) < stop);                          // trim the over-fetch + any padding past the end
    const rws = ins.map((x, idx) => ({ va: x.va, fi, html: lineHTML(x, idx === 0 ? (f.name || 'sub_' + f.addr) : null) }));
    if (!rws.length) rws.push({ va: f.addr, fi, html: lineHTML({ va: f.addr, mnemonic: '—', text: '' }, f.name || 'sub_' + f.addr) });
    fnCache.set(fi, rws); linWarmed.add(f.n);
    return rws;
  }
  // Pseudocode mode: decompile the whole function (same engine call as the Decompiler
  // tab — cached + vtable-memoised, so revisits are instant) and lay its C lines out
  // as rows, so the Linear view flows decompiled C for the whole program.
  async function fetchFnPseudo(fi, f) {
    const r = await n0xCached('linp|' + curPath + '|' + f.addr + '|' + decompStyle,
      ['decomp', 'pseudo', '--file', curPath, '--addr', f.addr, '--style', decompStyle]);
    const lines = (r?.ok && Array.isArray(r.data.pseudo) && r.data.pseudo.length) ? r.data.pseudo : ['// no decompilation'];
    const name = f.name || 'sub_' + f.addr;
    const hdr = `<div class="lin-fn" data-addr="${escH(f.addr)}" title="${escH(name)}">${escH(name)}</div>`;
    const rws = lines.map((ln, idx) => ({
      va: f.addr, fi,
      html: (idx === 0 ? hdr : '') + `<div class="lin-prow" data-addr="${escH(f.addr)}"><span class="lin-pgut">${idx + 1}</span><span class="lin-pc">${escH(ln)}</span></div>`,
    }));
    fnCache.set(fi, rws); linWarmed.add(f.n);
    return rws;
  }
  const rebuild = () => { rows = []; for (let i = loFi; i <= hiFi; i++) rows = rows.concat(fnCache.get(i) || []); };
  const render = () => { view.innerHTML = rows.map(r => r.html).join(''); markSel(); drawMap(); };
  // keep whatever the user is looking at fixed in place across a window change (add/trim)
  const anchor = () => { const el = [...view.querySelectorAll('.lin-row[data-addr],.lin-prow[data-addr]')].find(c => c.offsetTop >= view.scrollTop); return el ? { va: el.getAttribute('data-addr'), off: el.offsetTop - view.scrollTop } : null; };
  const restore = a => { if (!a) return; const el = [...view.querySelectorAll('.lin-row[data-addr],.lin-prow[data-addr]')].find(c => c.getAttribute('data-addr') === a.va); if (el) view.scrollTop = el.offsetTop - a.off; };
  async function appendNext() {
    if (loading || hiFi >= FN.length - 1) return; loading = true;
    const a = anchor();
    await fetchFn(hiFi + 1); hiFi++;
    rebuild(); while (rows.length > MAXROWS && loFi < hiFi) { loFi++; rebuild(); }
    render(); restore(a); loading = false;
  }
  async function prependPrev() {
    if (loading || loFi <= 0) return; loading = true;
    const a = anchor();
    await fetchFn(loFi - 1); loFi--;
    rebuild(); while (rows.length > MAXROWS && hiFi > loFi) { hiFi--; rebuild(); }
    render(); restore(a); loading = false;
  }
  function centerOn(addr) {
    const t = vaNum(addr);
    const el = [...view.querySelectorAll('[data-addr]')].find(c => vaNum(c.getAttribute('data-addr')) === t);
    if (el) view.scrollTop = el.offsetTop - view.clientHeight / 2 + el.offsetHeight;
    markSel(); drawVp();
  }
  function markSel() {
    const t = vaNum(selAddr);
    view.querySelectorAll('.lin-row.cur').forEach(n => n.classList.remove('cur'));
    view.querySelectorAll('.lin-row[data-addr]').forEach(n => { if (vaNum(n.getAttribute('data-addr')) === t) n.classList.add('cur'); });
    // paint the AI line-selection band
    view.querySelectorAll('.lin-row.selline').forEach(n => n.classList.remove('selline'));
    if (linSel) { const a = Math.min(linSel.a, linSel.b), b = Math.max(linSel.a, linSel.b); view.querySelectorAll('.lin-row[data-addr]').forEach(n => { const v = vaNum(n.getAttribute('data-addr')); if (v >= a && v <= b) n.classList.add('selline'); }); }
  }
  async function jump(addr) {
    syncFN();
    if (!FN.length || !isNative || !curPath) { view.innerHTML = '<div class="fl-empty">Open a binary to see the continuous listing.</div>'; return; }
    const fi = fiOf(vaNum(addr) || FN[0].n);
    if (fi < loFi || fi > hiFi || !rows.length) {   // re-anchor: load this function, then a small buffer each way
      loFi = hiFi = fi; await fetchFn(fi); rebuild(); render();
      const buf = linMode === 'pseudo' ? 1 : 2;         // decompiling is heavier — load a tighter buffer
      for (let k = 0; k < buf; k++) { await appendNext(); await prependPrev(); }
    }
    centerOn(addr);
  }
  function drawMap() {
    const h = map.clientHeight || wrap.clientHeight || 400; if (map.height !== h) map.height = h;
    const ctx = map.getContext('2d'); ctx.clearRect(0, 0, map.width, h);
    const N = FN.length || 1;
    for (let y = 0; y < h; y++) {                    // full extent immediately; brighter where bodies are loaded
      const fi = Math.floor(y / h * N), f = FN[fi];
      const loaded = f && (fnCache.has(fi) || linWarmed.has(f.n)), named = f && !/^sub_/i.test(f.name);
      ctx.fillStyle = loaded ? (named ? '#3fdcc4' : '#8ab4ff') : (named ? 'rgba(63,220,196,.28)' : 'rgba(150,160,180,.14)');
      ctx.fillRect(8, y, map.width - 16, 1);
    }
    if (hiFi >= loFi) { ctx.fillStyle = 'rgba(255,255,255,.06)'; ctx.fillRect(0, loFi / N * h, map.width, Math.max(2, (hiFi - loFi + 1) / N * h)); }
    drawVp();
  }
  function drawVp() {
    if (mapDrag) return;                     // while dragging, the thumb follows the cursor (see mapThumbAt) — don't fight it
    let box = wrap.querySelector('.lin-vp'); if (!box) { box = document.createElement('div'); box.className = 'lin-vp'; wrap.appendChild(box); }
    const N = FN.length || 1, h = map.clientHeight || 400, span = (hiFi - loFi + 1) || 1;
    const frac = view.scrollHeight ? view.scrollTop / view.scrollHeight : 0, vfrac = view.scrollHeight ? view.clientHeight / view.scrollHeight : 1;
    box.style.top = ((loFi + frac * span) / N * h) + 'px';
    box.style.height = Math.max(10, (vfrac * span) / N * h) + 'px';
  }
  view.addEventListener('scroll', () => {
    if (view.scrollTop < 240) prependPrev();
    if (view.scrollTop + view.clientHeight > view.scrollHeight - 320) appendNext();
    drawVp();
  });
  // click a row → select for the AI; shift-click → extend a line range (addresses passed to the AI)
  view.addEventListener('click', e => {
    // Clicking a function NAME header in the listing = select & decompile it — the
    // way to decompile after scrolling there with the minimap (which is scroll-only).
    const hdr = e.target.closest('.lin-fn[data-addr]');
    if (hdr) { const a = hdr.getAttribute('data-addr'); const f = FN.find(x => x.addr === a); selAddr = f ? f.addr : a; selName = f ? f.name : ''; linSel = null; onSymbolSelect(); return; }
    const row = e.target.closest('.lin-row[data-addr]'); if (!row) return;
    const v = vaNum(row.getAttribute('data-addr'));
    if (e.shiftKey && linSel) linSel.b = v; else linSel = { a: v, b: v };
    markSel(); refreshAiScope();
  });
  // Minimap = a PURE global scrollbar over the whole program, fully decoupled from
  // the decompiler. Click or drag only SCROLLS the listing; it never decompiles
  // (that's a separate action: click a function name in the listing). The thumb
  // follows the cursor instantly (visual only); the actual re-anchor (`jump`) is
  // COALESCED — a single one runs at a time, always to the latest target — so any
  // amount of fast clicking/dragging never queues work or piles up.
  let mapTargetFi = 0, mapJumping = false, mapPending = false;
  const mapFiAt = clientY => { const r = map.getBoundingClientRect(); return clamp(Math.round((clientY - r.top) / r.height * ((FN.length || 1) - 1)), 0, (FN.length || 1) - 1); };
  function mapThumbTo(clientY) {                    // move the thumb to the cursor — instant, no load
    const r = map.getBoundingClientRect();
    let box = wrap.querySelector('.lin-vp'); if (!box) { box = document.createElement('div'); box.className = 'lin-vp'; wrap.appendChild(box); }
    box.style.top = clamp(clientY - r.top, 0, r.height - box.offsetHeight) + 'px';
  }
  function mapKick() {                              // scroll to the latest target, one jump at a time
    mapPending = true;
    if (mapJumping) return;
    mapJumping = true;
    (async () => {
      while (mapPending) { mapPending = false; const f = FN[mapTargetFi]; if (f) { try { await jump(f.addr); } catch {} } }
      mapJumping = false;
      if (!mapDrag) drawVp();
    })();
  }
  const mapAt = e => { mapThumbTo(e.clientY); mapTargetFi = mapFiAt(e.clientY); mapKick(); };
  map.addEventListener('pointerdown', e => { e.preventDefault(); mapDrag = true; map.setPointerCapture?.(e.pointerId); map.style.cursor = 'grabbing'; mapAt(e); });
  map.addEventListener('pointermove', e => { if (!mapDrag) return; e.preventDefault(); mapAt(e); });
  const mapEnd = e => { if (!mapDrag) return; mapDrag = false; map.releasePointerCapture?.(e.pointerId); map.style.cursor = 'grab'; mapAt(e); };
  map.addEventListener('pointerup', mapEnd);
  map.addEventListener('pointercancel', () => { mapDrag = false; map.style.cursor = 'grab'; drawVp(); });
  map.style.cursor = 'grab';
  // Keep the minimap sized to the widget: redraw (which resyncs the canvas buffer
  // to its clientHeight and repositions the thumb) whenever the panel is resized,
  // so it never overflows into its own scrollbar or leaves an empty strip below.
  try { new ResizeObserver(() => { try { drawMap(); } catch {} }).observe(wrap); } catch {}
  // Asm ⇄ Pseudo: same windowed loader, different per-function fetch. Switching
  // drops the cached rows (different content) and re-anchors on the current spot.
  root.querySelector('.lin-bar')?.addEventListener('click', e => {
    const b = e.target.closest('button[data-mode]'); if (!b || b.classList.contains('on')) return;
    root.querySelectorAll('.lin-bar button').forEach(x => x.classList.toggle('on', x === b));
    linMode = b.dataset.mode;
    fnCache.clear(); loFi = 0; hiFi = -1; rows = [];
    jump(selAddr || (FN[0] && FN[0].addr) || '');
  });
  root._linJump = addr => jump(addr);
  // analysis ticks call this — if the function list only just arrived, populate now
  root._linRedraw = () => { if (!rows.length && syncFN()) jump(selAddr || (FN[0] && FN[0].addr) || ''); else try { drawMap(); } catch {} };
  linears.push(root);
  jump(selAddr || (FUNCLIST && FUNCLIST[0] && FUNCLIST[0].addr) || '');
}

// Background analysis — drives the REAL engine `analyze` pass (discover → RTTI →
// xref index → IR-cache warm) as a separate process, and shows its live phases in
// the status bar (like BN). It's what pre-builds the xref index so `xref to` is
// instant, recovers MSVC class names, and persists to the per-target `.n0x/`.
let analyzePollTimer = null;
const PHASE_LABEL = { starting: 'starting', discovering: 'discovering', 'scanning-rtti': 'scanning RTTI', 'indexing-xrefs': 'indexing xrefs', disassembling: 'disassembling', done: 'ready' };
// How many functions to pre-decode in the (optional) warm phase. null = skip it
// (decompilation still caches lazily on first view; avoids ~371k tiny cache files).
function fmtPhase(s) {
  const p = PHASE_LABEL[s.phase] || s.phase || 'analyzing';
  const done = +s.done || 0, total = +s.total || 0;
  if (s.phase === 'disassembling' && total > 0) return `${p} ${done.toLocaleString()} / ${total.toLocaleString()}`;
  if (done > 0) return `${p} ${done.toLocaleString()}`;
  return p + '…';
}
async function startAnalysis(path) {
  if (!isNative) return;
  clearInterval(analyzePollTimer);
  xrefIndexReady = false;                       // the index is being (re)built
  const val = $('#sb-aval'), spin = $('#sb-analysis .sb-aspin');
  const set = (txt, col) => { if (val) { val.textContent = txt; val.style.color = col || 'var(--acc)'; } };
  if (spin) spin.hidden = false;
  set('starting…');
  const limit = getSet('analyze.warmCfg', false) ? 0 : null;   // 0 = warm every fn · null = skip
  await analyzeStart(path, limit, projectDir(path));
  analyzePollTimer = setInterval(async () => {
    if (path !== curPath) { clearInterval(analyzePollTimer); return; }
    const s = await analyzeStatus();
    if (!s) return;
    if (s.running === false) {
      clearInterval(analyzePollTimer);
      if (spin) spin.hidden = true;
      const r = s.result?.data;
      if (r) {
        const parts = [`${(+r.functions || 0).toLocaleString()} fns`];
        if (r.rtti_classes) parts.push(`${(+r.rtti_classes).toLocaleString()} classes`);
        set('ready · ' + parts.join(' · '), 'var(--ok)');
      } else set('ready', 'var(--ok)');
      xrefIndexReady = true;                    // built on disk — in-session queries are now ~2 ms
      loadXrefs();                              // fill the panel that was skipped while indexing
      linears.forEach(x => { if (x.isConnected) x._linRedraw?.(); });
      return;
    }
    set(fmtPhase(s));
  }, 300);
}

// one place that refreshes every symbol-following widget when the selection moves.
// Only spawn the engine for panes that are actually visible (a click shouldn't
// run xrefs + CFG if those widgets aren't shown) — big latency win.
// ---- Navigation history (Alt+← / Alt+→), like every disassembler. A stack of
// visited locations: any user navigation pushes the previous spot onto `navBack`;
// going back/forward replays without re-recording (guarded).
const navBack = [], navFwd = [];
let navPrev = null, navGuard = false;
const NAV_MAX = 300;
function navGoTo(loc) {                       // replay a remembered location
  navGuard = true;
  selAddr = loc.addr; selName = loc.name || '';
  onSymbolSelect();
  linears.forEach(r => { if (r.isConnected) r._linJump?.(selAddr); });
  navGuard = false;
}
function navBackward() {
  if (!navBack.length) { toast('No previous location'); return; }
  navFwd.push({ addr: selAddr, name: selName });
  navGoTo(navBack.pop());
}
function navForward() {
  if (!navFwd.length) { toast('Nothing to go forward to'); return; }
  navBack.push({ addr: selAddr, name: selName });
  navGoTo(navFwd.pop());
}
function onSymbolSelect() {
  const my = ++selSeq;
  linSel = null;                             // a function selection supersedes any Linear line-selection
  // Record the jump for back/forward — the PREVIOUS location, unless this select
  // is itself a back/forward replay (navGuard) or a no-op re-select.
  if (!navGuard && navPrev && navPrev.addr && navPrev.addr !== selAddr) {
    navBack.push(navPrev);
    if (navBack.length > NAV_MAX) navBack.shift();
    navFwd.length = 0;                        // a fresh navigation forks history
  }
  navPrev = { addr: selAddr, name: selName };
  const a = $('#sb-addr'); if (a) a.textContent = selAddr;
  const n = $('#sb-name'); if (n) n.textContent = selName || '—';
  if ($('.code')) loadDecomp(my);
  if ($('.disasm')) loadDisasm(my);
  if ($('.xrefs')) { paintXrefs(); loadXrefs(my); }
  if ($('.graphwrap')) loadGraph();
  linears.forEach(r => { if (r.isConnected) r._linJump?.(selAddr); });
  refreshAiScope();                          // a new function became the AI's context
}

/* ---- memory scanner: real `scan value` / `scan filter` against the attached pid ---- */
let scanPass = 0;
function initScanner(root) {
  const valEl = root.querySelector('.scan-val'), typeEl = root.querySelector('.scan-type');
  const stat = root.querySelector('.scan-stat'), res = root.querySelector('.scan-res');
  const setStat = (t, c) => { stat.textContent = t; stat.style.color = c || 'var(--tx2)'; };
  if (!curPid) setStat('not scanning — attach a process to begin');
  const renderMatches = d => {
    const ms = d.matches || [];
    res.innerHTML = ms.slice(0, 300).map(m => `<div class="srow" data-ctx="srow" data-addr="${escH(m.addr)}"><span class="saddr">${escH(m.addr)}</span><span class="sval" style="color:var(--live)">${escH(String(m.value))}</span></div>`).join('') || '<div class="xr-none">no matches</div>';
    const total = d.total_matches ?? ms.length;
    setStat(`pass ${scanPass} · ${total.toLocaleString()} match${total === 1 ? '' : 'es'}` + (d.shown < total ? ` · showing ${d.shown}` : ''), total ? 'var(--ok)' : 'var(--tx2)');
  };
  async function scan(kind) {
    if (!curPid) { setStat('attach a process first (Debug ▸ Attach)', 'var(--live)'); return; }
    const type = typeEl.value, val = valEl.value.trim();
    if (val === '') { setStat('enter a value to search for', 'var(--live)'); return; }
    if (!isNative) {   // web preview — demo matches so the scanner is exercisable
      scanPass = kind === 'next' ? scanPass + 1 : 1;
      const n = kind === 'next' ? 2 : 6;
      renderMatches({ matches: Array.from({ length: n }, (_, i) => ({ addr: '0x7FF6C21A' + (40 + i * 4).toString(16).toUpperCase(), value: val })), total_matches: n, shown: n });
      return;
    }
    setStat(kind === 'next' ? 'narrowing…' : 'scanning memory…');
    const args = kind === 'next'
      ? ['scan', 'filter', '--pid', String(curPid), '--from', 'gui', '--criterion', 'exact', '--value', val, '--save-as', 'gui']
      : ['scan', 'value', '--pid', String(curPid), '--type', type, '--criterion', 'exact', '--value', val, '--save-as', 'gui'];
    const r = await n0x(args);
    if (r?.ok) { scanPass = kind === 'next' ? scanPass + 1 : 1; renderMatches(r.data); }
    else setStat('scan failed · ' + (r?.error?.message || 'unknown'), 'var(--dgr)');
  }
  root.querySelector('.scan-new').addEventListener('click', () => scan('new'));
  root.querySelector('.scan-next').addEventListener('click', () => scan('next'));
}

/* ---- switchable Code view (Pseudo-C / Disassembly / Graph in one pane) ---- */
function initCodeView(root) {
  const body = root.querySelector('.cv-body'), sel = root.querySelector('.cv-sel');
  if (!root.dataset.cvmode) root.dataset.cvmode = 'pseudo';
  const label = { pseudo: 'Pseudo-C', disasm: 'Disassembly', graph: 'Graph' };
  const draw = () => {
    const m = root.dataset.cvmode;
    sel.textContent = (label[m] || 'Pseudo-C') + ' ▾';
    if (m === 'graph') { body.innerHTML = WIDGETS.graph.body(); try { WIDGETS.graph.init(body); } catch (e) { console.warn('graph init', e); } }
    else if (m === 'disasm') { body.innerHTML = disasmInner(); }
    else { body.innerHTML = pseudoInner(); if (isNative && curPath) loadDecomp(); }
  };
  draw();
  sel.addEventListener('click', e => {
    e.stopPropagation();
    const r = sel.getBoundingClientRect();
    openCtx(r.left, r.bottom + 4, [
      { label: 'Pseudo-C', icon: ICON.decomp, act: () => { root.dataset.cvmode = 'pseudo'; draw(); } },
      { label: 'Disassembly', icon: ICON.disasm, act: () => { root.dataset.cvmode = 'disasm'; draw(); } },
      { label: 'Graph', icon: ICON.graph, act: () => { root.dataset.cvmode = 'graph'; draw(); } },
    ]);
  });
}

// Default workspace layouts — trees over the same widget catalog.
const L = (...tabs) => ({ t: 'leaf', tabs });
const R = (ra, a, rb, b) => ({ t: 'split', dir: 'row', ratio: [ra, rb], kids: [a, b] });
const C = (ca, a, cb, b) => ({ t: 'split', dir: 'col', ratio: [ca, cb], kids: [a, b] });
const DEFAULT_LAYOUTS = {
  decompile: C(0.82,
    R(0.2, L('functions'),
      0.8, R(0.66, C(0.68, L('decompiler', 'linear'), 0.32, L('disassembly')),
                 0.34, C(0.58, L('copilot'), 0.42, L('details', 'variables', 'xrefs', 'bookmarks')))),
    0.18, L('output', 'console')),
  static: R(0.22, L('functions'),
    0.78, R(0.6, C(0.7, L('decompiler', 'linear'), 0.3, L('disassembly')),
      0.4, C(0.5, L('variables', 'triage', 'types'), 0.5, L('strings', 'xrefs', 'bookmarks')))),
  graph: L('graph'),
  dynamic: R(0.34, C(0.4, L('registers', 'stack'), 0.6, L('watchpoints')),
    0.66, R(0.5, C(0.55, L('scanner'), 0.45, L('livemem')),
               0.5, C(0.5, L('watchlist'), 0.5, L('copilot')))),
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

// undo / redo — buttons + Ctrl+Shift+Z / Ctrl+Shift+Y
$('#btn-undo')?.addEventListener('click', () => dock.undo());
$('#btn-redo')?.addEventListener('click', () => dock.redo());
// (Ctrl+Shift+Z / Ctrl+Shift+Y handled by the data-driven dispatcher)

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
      if (path) openBinary(path);
    }, true);
  }
  // opened from a terminal / file manager: n0xis-gui /path/to/binary
  const initPath = await initialTarget();
  if (initPath) openBinary(initPath);
}

// One path opens a target everywhere (launcher card, File menu, CLI arg).
// Opening a project fully RESETS state — no stale merge from the previous one.
async function openBinary(path) {
  if (!path) return;
  const name = path.split(/[\\/]/).pop();
  curPid = 0; curPidName = '';                 // leave any live session
  selAddr = ''; selName = ''; FUNCLIST = null; FUNCMETA = null; funcQuery = '';
  engCache.clear();                            // drop the previous target's cached results
  $$('.ffilter').forEach(i => { i.value = ''; });
  paintFunctions();                            // clear the old list immediately
  paintDecompMsg('analyzing ' + name + '…'); paintTriage();
  openTarget('static');
  $('#target-name').textContent = name;
  setWorkspace('static');
  toast('Analyzing <span class="mono">' + escH(name) + '</span>…');
  setTarget(path);                             // → Triage via profile
  sessionOn = false;
  if (isNative) {
    const proj = projectDir(path);
    const s = await sessionOpen(path, proj); if (path !== curPath) return; sessionOn = !!(s && s.ok);
    // arm the close behavior for the current cache-location + keep-on-close choice
    setDiscardOnClose(getSet('cache.keepOnClose', true) ? null : path, proj);
  }
  loadFunctions(path);
}

// Stream the function list in chunks so a huge binary (~370k functions is real)
// never lands as one 16 MB blob that crashes the webview. --pdata gives exact
// starts, a real total, and O(1) paging; falls back to prologue scan.
const FUNC_CHUNK = 20000, FUNC_CAP = 400000;   // cap only guards against pathological OOM
async function loadFunctions(path) {
  FUNCLIST = []; FUNCMETA = { total: 0, shown: 0, named: 0 }; funcByAddr = new Map();
  // instant path: a validated on-disk cache (survives restarts) — no re-streaming
  try {
    const cached = await fncacheGet(path);
    if (path !== curPath) return;
    if (cached) {
      const c = JSON.parse(cached);
      if (Array.isArray(c.list) && c.list.length) {
        FUNCLIST = c.list; FUNCLIST.forEach(f => funcByAddr.set((f.addr || '').toLowerCase(), f.name));
        FUNCMETA = { total: c.total ?? FUNCLIST.length, shown: FUNCLIST.length, named: c.named ?? 0 };
        paintFunctions(); $('#dockspace .flist .frow')?.click();
        linears.forEach(r => { if (r.isConnected) r._linRedraw?.(); });   // populate a Linear tab that mounted early
        toast('Loaded ' + FUNCMETA.total.toLocaleString() + ' functions · cached');
        if (getSet("analyze.autorun", true)) startAnalysis(path); else xrefIndexReady = true;
        return;
      }
    }
  } catch {}
  const mapf = f => { const addr = f.addr || f.address || f.va || ''; return { addr, name: f.name || f.symbol || f.label || 'sub_' + String(addr).replace(/^0x/, ''), end: f.end || f.end_va || '', tag: '' }; };
  const fargs = (off, pd) => ['function', 'discover', '--file', path, ...(pd ? ['--pdata'] : []), '--limit', String(FUNC_CHUNK), '--offset', String(off)];
  let offset = 0, first = true, pdata = true, total = null;
  while (offset < FUNC_CAP) {
    let res = await eng(fargs(offset, pdata));   // resident session → the image is loaded once, not per chunk
    if (path !== curPath) return;              // a newer open superseded this one
    // if --pdata isn't applicable (non-x64-PE), retry this offset with prologue scan
    if (pdata && (!res?.ok || !(res.data?.functions?.length))) { pdata = false; res = await eng(fargs(offset, false)); if (path !== curPath) return; }
    const list = res?.data?.functions;
    if (!res?.ok) { if (first) { $$('#dockspace .flist').forEach(fl => { fl.innerHTML = `<div class="fl-empty">${escH(res?.error?.message || 'analysis failed')}</div>`; }); toast('Engine: ' + (res?.error?.message || 'failed')); } break; }
    if (total == null) total = res?.meta?.total ?? null;
    if (!Array.isArray(list) || !list.length) break;
    list.forEach(f => { const m = mapf(f); FUNCLIST.push(m); funcByAddr.set(m.addr.toLowerCase(), m.name); });
    FUNCMETA = { total: total ?? FUNCLIST.length, shown: FUNCLIST.length, named: Math.round(FUNCLIST.filter(f => !/^sub_/i.test(f.name)).length / FUNCLIST.length * 100) };
    paintFunctions();
    if (first) { first = false; $('#dockspace .flist .frow')?.click(); }  // decompile the first ASAP
    if (list.length < FUNC_CHUNK) break;       // last page
    offset += FUNC_CHUNK;
    await new Promise(r => setTimeout(r, 0));   // yield so the UI stays responsive
  }
  if (FUNCLIST.length) {
    FUNCMETA.total = total ?? FUNCLIST.length; paintFunctions();
    toast('Loaded ' + FUNCLIST.length.toLocaleString() + ' functions');
    // persist for an instant re-open next time (validated by file mtime)
    try { fncachePut(path, JSON.stringify({ list: FUNCLIST, total: FUNCMETA.total, named: FUNCMETA.named })); } catch {}
    linears.forEach(r => { if (r.isConnected) r._linRedraw?.(); });        // populate a Linear tab that mounted early
    if (getSet("analyze.autorun", true)) startAnalysis(path); else xrefIndexReady = true;
  }
}

deepLink();
dock.setWorkspace(document.documentElement.dataset.workspace || 'decompile'); // seed/restore the workspace layout
hydrateFromEngine();
console.log('N0xis GUI ready · graph, context menus, dock widgets, palette (Ctrl+P), themes, zoom');
