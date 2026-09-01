// N0xis GUI — interactive shell (framework-free; drops into Tauri's webview as-is)
import { isNative, engineInfo, engine, pickFile, n0x, initialTarget, listProcesses, processIcons } from './bridge.js';
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
  { id: 'open', label: 'Open file / process', group: 'General', def: 'Ctrl+O', scope: 'global', act: () => openFileTarget() },
  { id: 'goto', label: 'Go to address / symbol', group: 'General', def: 'Ctrl+G', scope: 'target', act: () => doGoto() },
  { id: 'settings', label: 'Settings', group: 'General', def: 'Ctrl+,', scope: 'global', act: () => openSettings() },
  { id: 'zoomin', label: 'Zoom in', group: 'General', def: 'Ctrl+=', scope: 'global', act: () => setZoom(zoom + 0.1) },
  { id: 'zoomout', label: 'Zoom out', group: 'General', def: 'Ctrl+-', scope: 'global', act: () => setZoom(zoom - 0.1) },
  { id: 'zoomreset', label: 'Reset zoom', group: 'General', def: 'Ctrl+0', scope: 'global', act: () => setZoom(1) },
  { id: 'closewin', label: 'Close window', group: 'General', def: 'Ctrl+Q', scope: 'global', act: () => winCtl('close') },
  { id: 'decompile', label: 'Decompile / Continue', group: 'Analysis', def: 'F5', scope: 'target', act: () => { if (isLive()) toast('Continue'); else { setWorkspace('decompile'); toast('Decompiling'); } } },
  { id: 'rename', label: 'Rename symbol', group: 'Analysis', def: 'F2', scope: 'target', act: () => doRename() },
  { id: 'comment', label: 'Comment', group: 'Analysis', def: 'Ctrl+/', scope: 'target', act: () => doComment() },
  { id: 'xrefs', label: 'Find references (xrefs)', group: 'Analysis', def: 'Shift+F12', scope: 'target', act: () => toast('Xrefs') },
  { id: 'stepover', label: 'Step over', group: DBG, def: 'F10', scope: 'live', act: () => toast('Step over') },
  { id: 'stepinto', label: 'Step into', group: DBG, def: 'F11', scope: 'live', act: () => toast('Step into') },
  { id: 'breakpoint', label: 'Toggle breakpoint', group: DBG, def: 'F9', scope: 'target', act: () => toast('Toggle breakpoint') },
  { id: 'undo', label: 'Undo layout change', group: 'Layout', def: 'Ctrl+Shift+Z', scope: 'global', act: () => dock.undo() },
  { id: 'redo', label: 'Redo layout change', group: 'Layout', def: 'Ctrl+Shift+X', scope: 'global', act: () => dock.redo() },
];
const keyOf = id => (id in prefs.keys ? prefs.keys[id] : (KEYDEFS.find(d => d.id === id)?.def ?? null));
function comboOf(e) {                              // normalize an event → "Ctrl+Shift+P" / "F2" / "Ctrl+/"
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return null;
  let s = '';
  if (e.ctrlKey || e.metaKey) s += 'Ctrl+';
  if (e.shiftKey) s += 'Shift+';
  if (e.altKey) s += 'Alt+';
  let k = e.key; if (k.length === 1) k = k.toUpperCase();
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
  ['Find what accesses this address', 'Cheat-Engine-style hit counter', '', () => setWorkspace('dynamic')],
  ['Memory scan…', 'Hunt a value in a live process', '', () => { setWorkspace('dynamic'); if (!isLive()) pickProcess(); }],
  ['Attach to a process', 'Bind a live process to this session', '', () => pickProcess()],
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
  const cam = { x: 40, y: 20, s: 1 };
  let blocks = GRAPH.blocks.map(b => ({ ...b }));  // local, independently draggable positions
  const rectOf = {};
  const rankOf = {};

  // layered auto-layout (Sugiyama-lite): rank by longest path from entry, then place each
  // node at the BARYCENTRE x of its neighbours (parents + loop partners) and push apart on
  // overlap — so a child sits under its parent, and rows never overlap.
  function layout() {
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
    blocks.forEach(b => b.y = 24 + rankOf[b.id] * PITCH);
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
  function drawEdges() {
    // explicit per-colour markers (context-stroke is unreliable across themes)
    const M = { n: mid + '-n', t: mid + '-t', f: mid + '-f', loop: mid + '-l', u: mid + '-u' };
    const mk = (id, c) => `<marker id="${id}" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="${c}"/></marker>`;
    const defs = `<defs>${mk(M.n, 'var(--bd2)')}${mk(M.t, 'var(--ok)')}${mk(M.f, 'var(--dgr)')}${mk(M.loop, 'var(--live)')}${mk(M.u, 'var(--fn)')}</defs>`;
    const paths = GRAPH.edges.map(e => {
      const a = rectOf[e.f], b = rectOf[e.t]; if (!a || !b) return '';
      const cls = 'gedge' + (e.type ? ' ' + e.type : '');
      const arw = e.type === 't' ? M.t : e.type === 'f' ? M.f : e.type === 'loop' ? M.loop : e.type === 'u' ? M.u : M.n;
      if (e.type === 'loop') { // back-edge: out the body's right, up, back into the header's right side
        const sx = a.x + a.w, sy = a.y + a.h * 0.5, tx2 = b.x + b.w, ty2 = b.y + b.h * 0.5, bx = Math.max(sx, tx2) + 44;
        return `<path class="${cls}" marker-end="url(#${arw})" d="M${sx} ${sy} C${bx} ${sy},${bx} ${ty2},${tx2} ${ty2}"/>`;
      }
      const sx = a.x + a.w / 2, sy = a.y + a.h, tx = b.x + b.w / 2, ty = b.y, my = (sy + ty) / 2;
      const gap = (rankOf[e.t] ?? 0) - (rankOf[e.f] ?? 0);
      // adaptive: only bow around when the straight corridor is actually blocked by a block
      if (gap > 1 && corridorBlocked(sx, tx, rankOf[e.f], rankOf[e.t], [e.f, e.t])) {
        const leftOff = Math.min(sx, tx) - 130, rightOff = Math.max(sx, tx) + 130;
        const off = !corridorBlocked(leftOff, leftOff, rankOf[e.f], rankOf[e.t], [e.f, e.t]) ? leftOff : rightOff;
        return `<path class="${cls}" marker-end="url(#${arw})" d="M${sx} ${sy} C${off} ${sy + 40},${off} ${ty - 40},${tx} ${ty - 2}"/>`;
      }
      return `<path class="${cls}" marker-end="url(#${arw})" d="M${sx} ${sy} C${sx} ${my},${tx} ${my},${tx} ${ty - 2}"/>`;
    }).join('');
    gs.innerHTML = defs + paths;
  }
  function renderGraph() { buildNodes(); drawEdges(); }
  function applyCam() { gc.style.transform = `translate(${cam.x}px,${cam.y}px) scale(${cam.s})`; const l = root.querySelector('.gzl'); if (l) l.textContent = Math.round(cam.s * 100) + '%'; }
  function fitGraph() {
    const vw = gv.clientWidth, vh = gv.clientHeight; if (!vw || !vh) return;
    let maxX = 0, maxY = 0;
    blocks.forEach(b => { maxX = Math.max(maxX, b.x + b.w); maxY = Math.max(maxY, b.y + (rectOf[b.id]?.h || 70)); });
    cam.s = clamp(Math.min(vw / (maxX + 60), vh / (maxY + 60)), 0.4, 1.3);
    cam.x = (vw - maxX * cam.s) / 2; cam.y = Math.max(16, (vh - maxY * cam.s) / 2); applyCam();
  }
  let pan = null, userAdjusted = false;           // once the user pans/zooms, stop auto-fitting
  gv.addEventListener('pointerdown', e => { if (e.target.closest('.gzoom, .gnode, .glegend')) return; pan = { x: e.clientX, y: e.clientY, cx: cam.x, cy: cam.y }; gv.classList.add('panning'); gv.setPointerCapture(e.pointerId); });
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
  function trimLegend() { if (blocks.length <= 12) root.querySelector('.glegend')?.remove(); }
  trimLegend();
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
  const r = await n0x(['ir', 'build', '--file', curPath, '--addr', selAddr]);
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
      item('Find xrefs to', 'xref', 'Shift+F12', () => toast('Xrefs to ' + name)),
      item('Find xrefs from', 'xref', '', () => toast('Xrefs from ' + name)),
      item('Apply signature', 'type', '', () => echo('sig apply --func ' + name, 'matched 1')),
      sep,
      item('Copy name', 'copy', '', () => copy(name)),
      item('Copy address', 'copy', '', () => copy(addr)),
    ];
    case 'cl': return [
      item('Copy line', 'copy', 'Ctrl+C', () => copy(tgt.textContent.replace(/^\d+/, '').trim())),
      item('Rename variable…', 'rename', 'F2', () => doRename(selAddr, '')),
      item('Change type…', 'type', '', () => doRetype(selAddr)),
      sep,
      item('Toggle breakpoint', 'bp', 'F9', () => { tgt.classList.toggle('hot'); toast('Breakpoint toggled'); }),
      item('Add watchpoint', 'watch', '', () => { setWorkspace('dynamic'); toast('Watchpoint added'); }),
      item('Comment…', 'note', 'Ctrl+/', () => doComment(selAddr)),
    ];
    case 'drow': return [
      item('Copy instruction', 'copy', '', () => copy(tgt.textContent.trim())),
      item('Toggle breakpoint', 'bp', 'F9', () => { tgt.classList.toggle('hot'); toast('Breakpoint @ ' + (tgt.querySelector('.daddr')?.textContent || '')); }),
      item('Set watchpoint', 'watch', '', () => setWorkspace('dynamic')),
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
async function runAnnotate(kind, addr, val, okverb) {
  if (!isNative) { toast(`${okverb} <span class="mono">${escH(addr)}</span> → ${escH(val || '(cleared)')} · demo`); return; }
  const args = ['annotate', kind, '--addr', addr]; if (val !== '') args.push('--value', val);
  const r = await n0x(args);
  toast(r?.ok ? `${okverb} <span class="mono">${escH(addr)}</span> → ${escH(val || 'cleared')}`
              : `annotate failed: ${escH(r?.error?.message || '?')}`);
}
async function doRename(addr = selAddr, cur = selName) {
  const v = await askInput(`Rename ${cur || addr}`, cur || '', 'new function/variable name');
  if (v !== null) runAnnotate('name', addr, v.trim(), 'Renamed');
}
async function doComment(addr = selAddr) {
  const v = await askInput(`Comment @ ${addr}`, '', 'free-text note');
  if (v !== null) runAnnotate('comment', addr, v, 'Commented');
}
async function doRetype(addr = selAddr, cur = '') {
  const v = await askInput(`Change type @ ${addr}`, cur, 'e.g.  int(char*, size_t)');
  if (v !== null) runAnnotate('type', addr, v.trim(), 'Type set on');
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
    // prefer the host /proc names (real exe names for Proton/Wine games — the
    // engine's `process ps` reports 'wine-preloader' for those), and resolve real
    // per-app icons in parallel (freedesktop + Steam appid).
    const [host, ic] = await Promise.all([listProcesses(), processIcons()]);
    if (host?.ok && host.data.processes?.length) procs = host.data.processes.slice();
    else { const r = await n0x(['process', 'ps']); if (!r?.ok) { toast('process list failed: ' + (r?.error?.message || '?')); return; } procs = (r.data.processes || []).slice(); }
    icons = ic?.data?.icons || {};
  } else {   // web preview — demo list so the flow is exercisable
    procs = [{ name: 'game.exe', pid: 8124 }, { name: 'chrome.exe', pid: 4410 }, { name: 'explorer.exe', pid: 1200 }, { name: 'discord.exe', pid: 9931 }, { name: 'steam.exe', pid: 5567 }];
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
      item('Change type…', 'type', '', () => doRetype()),
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
      item('Redo layout', 'redo', 'Ctrl+Shift+X', () => dock.redo()),
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
const funcRows = () => FUNCLIST || FUNCS.map(([name, addr, tag]) => ({ name, addr, tag }));
function funcView() {                       // full list filtered by the search box
  const q = funcQuery.trim().toLowerCase(), rows = funcRows();
  return q ? rows.filter(f => (f.name + ' ' + f.addr).toLowerCase().includes(q)) : rows;
}
const frowHTML = f => `<div class="frow${f.tag === 'sub' ? ' sub' : ''}${f.addr === selAddr ? ' on' : ''}" data-addr="${escH(f.addr)}"><span class="sym">ƒ</span><span class="nm mono">${escH(f.name)}</span>${f.tag === 'sig' ? '<span class="badge">sig</span>' : ''}<span class="fa mono">${escH(f.addr)}</span></div>`;
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
    <div class="flist">${funcRows().map(frowHTML).join('')}</div>
    <div class="pfoot">${funcFootHTML()}</div></div>` },

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

  hex: { title: 'Hex', icon: 'hex', body: () => `<div class="hex selectable" style="height:100%"><div class="hx"><span class="hxa">7FF6C21A40</span><span class="hxb"><span class="hb-hi">57 00 00 00</span> 2a 00 00 00 5c 21 c2 f6</span><span class="hxc">W....*...\\!</span></div><div class="hx"><span class="hxa">7FF6C21A50</span><span class="hxb">64 00 00 00 00 00 80 3f 00 00 80 3f</span><span class="hxc">d......?...?</span></div></div>` },

  graph: { title: 'Control-flow graph', icon: 'graph', body: () => `<div class="graphwrap" style="height:100%">
    <div class="gviewport"><div class="gcanvas"><svg class="gsvg" width="920" height="700"></svg></div>
    <div class="gzoom"><button class="gz-in">+</button><div class="gzl">100%</div><button class="gz-out">−</button><button class="gz-fit">${svg('M4 9V5a1 1 0 0 1 1-1h4;M20 9V5a1 1 0 0 0-1-1h-4;M4 15v4a1 1 0 0 0 1 1h4;M20 15v4a1 1 0 0 1-1 1h-4','')}</button></div>
    <div class="glegend"><span class="lg"><span class="ln t"></span>true</span><span class="lg"><span class="ln f"></span>false</span><span class="lg"><span class="ln u"></span>uncond</span><span class="lg"><span class="ln loop"></span>loop</span><span class="lg" style="color:var(--tx2)">drag blocks · scroll = zoom</span></div></div></div>`,
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
    <div class="wprow" data-ctx="wprow"><span class="wpico">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C21A40</div><div style="color:var(--tx2);font-size:.68rem">write · health</div></div><span class="hit">1,204</span></div>
    <div class="wprow" data-ctx="wprow"><span class="wpico" style="color:var(--acc)">${svg(ICON.watch,'')}</span><div><div class="mono">7FF6C218E0</div><div style="color:var(--tx2);font-size:.68rem">read · ammo</div></div><span class="hit" style="color:var(--acc);background:rgba(63,220,196,.1)">86</span></div>
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
    <div class="hx"><span class="hxa">7FF6C21A80</span><span class="hxb">48 65 61 6c 74 68 43 6f 6d 70 6f 6e 65 6e 74 00</span><span class="hxc" style="color:var(--ok)">HealthComponent.</span></div></div>` },

  watchlist: { title: 'Watchlist', icon: 'watch', body: () => `<div style="overflow:auto;height:100%">${[
      ['health','87','on'],['max_health','100','on'],['ammo','42',''],['gold','10000','']
    ].map(([n,v,f])=>`<div class="wlrow" data-ctx="wlrow"><span class="frz ${f}"></span><span class="wname">${n}</span><span class="wval">${v}</span></div>`).join('')}</div>` },

  strings: { title: 'Strings', icon: 'strings', body: () => `<div style="overflow:auto;height:100%;padding:6px 0">${['HealthComponent','TakeDamage','crc_table','zlib 1.3.1','deflate','Assertion failed'].map(s=>`<div class="frow"><span class="nm mono selectable">"${s}"</span></div>`).join('')}</div>` },

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
    ${adv.length ? `<div class="tg-adv">${adv.map(a => `<div class="tg-adv-i"><span class="mono tg-ac">${escH(a.code || 'note')}</span>${escH(a.message || a)}</div>`).join('')}</div>` : ''}
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
let XREF = {
  to: [{ from: '0x14e6', to: '0x1510', kind: 'call', text: 'call crc32_z' }, { from: '0x1a70', to: '0x1510', kind: 'call', text: 'call crc32_z' }],
  from: [{ from: '0x1510', to: '0x1002a', kind: 'call', text: 'call sub_1002A' }, { from: '0x1510', to: '0x1560', kind: 'cond_jmp', text: 'je short 0x1560' }],
};
const xrefRow = (r, dir) => { const a = dir === 'to' ? r.from : r.to; return `<div class="xr-r" data-ctx="frow" data-addr="${escH(a)}"><span class="xr-k">${escH(r.kind || '')}</span><span class="fnc mono">${escH(a)}</span><span class="xr-tx mono">${escH(r.text || '')}</span></div>`; };
function paintXrefs(scope) {                         // scope: one widget root, or all mounted
  const roots = scope ? [scope] : $$('.xrefs');
  roots.forEach(r => {
    const addr = r.querySelector('.xr-addr'), to = r.querySelector('.xr-to'), from = r.querySelector('.xr-from');
    if (addr) addr.textContent = selAddr;
    if (to) to.innerHTML = XREF.to.length ? XREF.to.map(x => xrefRow(x, 'to')).join('') : '<div class="xr-none">no references</div>';
    if (from) from.innerHTML = XREF.from.length ? XREF.from.map(x => xrefRow(x, 'from')).join('') : '<div class="xr-none">no references</div>';
  });
}
function initXrefs(root) { paintXrefs(root.querySelector('.xrefs') || root); root.querySelector('.xr-refresh')?.addEventListener('click', loadXrefs); }
async function loadXrefs(seq) {
  if (!isNative || !curPath) { paintXrefs(); return; }
  const [t, f] = await Promise.all([n0x(['xref', 'to', '--file', curPath, '--addr', selAddr]), n0x(['xref', 'from', '--file', curPath, '--addr', selAddr])]);
  if (seq !== undefined && seq !== selSeq) return;
  if (t?.ok) XREF.to = t.data.refs || []; if (f?.ok) XREF.from = f.data.refs || [];
  paintXrefs();
}

let VARS = [{ n: 'rdi', t: 'uint64_t', k: 'param' }, { n: 'rsi', t: 'void*', k: 'param' }, { n: 'rdx', t: 'uint64_t', k: 'param' }, { n: 'v14', t: '—', k: 'local' }, { n: 'v9', t: '—', k: 'local' }];
function paintVars(el) {
  if (!el) { $$('.vars').forEach(paintVars); return; }
  el.innerHTML = VARS.length ? VARS.map(v => `<div class="var-r"><span class="var-k ${v.k}">${v.k[0].toUpperCase()}</span><span class="ty mono">${escH(v.t)}</span><span class="fnc mono">${escH(v.n)}</span></div>`).join('') : '<div class="xr-none">no variables</div>';
}
function parseVars(sig, pseudo) {
  const out = [], seen = new Set();
  const m = /\(([^)]*)\)/.exec(sig || '');
  if (m && m[1].trim()) m[1].split(',').forEach(p => { const parts = p.trim().split(/\s+/); const n = parts.pop(); const t = parts.join(' ') || '?'; if (n && !seen.has(n)) { seen.add(n); out.push({ n, t, k: 'param' }); } });
  (pseudo || []).join('\n').replace(/\b(var_[0-9a-fA-F]+|v\d+)\b/g, x => { if (!seen.has(x)) { seen.add(x); out.push({ n: x, t: '—', k: 'local' }); } return x; });
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
async function loadDecomp(seq) {
  if (!isNative || !curPath) return;                 // keep the demo pseudo in the web preview
  paintDecompMsg('decompiling ' + (selName || selAddr) + '…');
  const r = await n0x(['decomp', 'pseudo', '--file', curPath, '--addr', selAddr, '--style', decompStyle]);
  if (seq !== undefined && seq !== selSeq) return;    // a newer selection superseded this one
  if (r?.ok && Array.isArray(r.data.pseudo)) { paintDecomp(r.data.pseudo); VARS = parseVars(r.data.signature, r.data.pseudo); paintVars(); }
  else paintDecompMsg('// no decompilation for ' + selAddr + (r?.error ? ' — ' + r.error.message : ''));
}
function paintDisasm(insns) {
  const html = insns.map(i => {
    const op = (i.text || '').slice((i.mnemonic || '').length).trim();
    return `<div class="drow" data-ctx="drow" data-addr="${escH(i.va || '')}"><span class="daddr">${escH(i.va || '')}</span><span class="dbytes">${escH(i.bytes || '')}</span><span class="dmn">${escH(i.mnemonic || '')}</span><span>${escH(op)}</span></div>`;
  }).join('');
  $$('.disasm').forEach(d => { d.innerHTML = html; });
}
async function loadDisasm(seq) {
  if (!isNative || !curPath) return;
  const r = await engine.disasm(curPath, selAddr);
  if (seq !== undefined && seq !== selSeq) return;
  if (r?.ok && Array.isArray(r.data.insns)) paintDisasm(r.data.insns);
  else $$('.disasm').forEach(d => { d.innerHTML = `<div class="drow" style="color:var(--tx2)">no disassembly for ${escH(selAddr)}</div>`; });
}

// one place that refreshes every symbol-following widget when the selection moves
function onSymbolSelect() {
  const my = ++selSeq;
  const a = $('#sb-addr'); if (a) a.textContent = selAddr;
  const n = $('#sb-name'); if (n) n.textContent = selName || '—';
  paintXrefs(); loadXrefs(my); loadDecomp(my); loadDisasm(my); loadGraph();
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
      0.8, R(0.66, C(0.68, L('decompiler'), 0.32, L('disassembly')),
                 0.34, C(0.58, L('copilot'), 0.42, L('details', 'variables', 'xrefs')))),
    0.18, L('output', 'console')),
  static: R(0.22, L('functions'),
    0.78, R(0.6, C(0.7, L('decompiler'), 0.3, L('disassembly')),
      0.4, C(0.5, L('triage'), 0.5, L('strings')))),
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

// undo / redo — buttons + Ctrl+Shift+Z / Ctrl+Shift+X
$('#btn-undo')?.addEventListener('click', () => dock.undo());
$('#btn-redo')?.addEventListener('click', () => dock.redo());
// (Ctrl+Shift+Z / Ctrl+Shift+X handled by the data-driven dispatcher)

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
  $$('.ffilter').forEach(i => { i.value = ''; });
  paintFunctions();                            // clear the old list immediately
  paintDecompMsg('analyzing ' + name + '…'); paintTriage();
  openTarget('static');
  $('#target-name').textContent = name;
  setWorkspace('static');
  toast('Analyzing <span class="mono">' + escH(name) + '</span>…');
  setTarget(path);                             // → Triage via profile
  loadFunctions(path);
}

// Stream the function list in chunks so a huge binary (AyuGram = 371k functions)
// never lands as one 16 MB blob that crashes the webview. --pdata gives exact
// starts, a real total, and O(1) paging; falls back to prologue scan.
const FUNC_CHUNK = 20000, FUNC_CAP = 400000;   // cap only guards against pathological OOM
async function loadFunctions(path) {
  FUNCLIST = []; FUNCMETA = { total: 0, shown: 0, named: 0 };
  const mapf = f => { const addr = f.addr || f.address || f.va || ''; return { addr, name: f.name || f.symbol || f.label || 'sub_' + String(addr).replace(/^0x/, ''), tag: '' }; };
  let offset = 0, first = true, pdata = true, total = null;
  while (offset < FUNC_CAP) {
    let res = await engine.functions(path, FUNC_CHUNK, offset, pdata);
    if (path !== curPath) return;              // a newer open superseded this one
    // if --pdata isn't applicable (non-x64-PE), retry this offset with prologue scan
    if (pdata && (!res?.ok || !(res.data?.functions?.length))) { pdata = false; res = await engine.functions(path, FUNC_CHUNK, offset, false); if (path !== curPath) return; }
    const list = res?.data?.functions;
    if (!res?.ok) { if (first) { $$('#dockspace .flist').forEach(fl => { fl.innerHTML = `<div class="fl-empty">${escH(res?.error?.message || 'analysis failed')}</div>`; }); toast('Engine: ' + (res?.error?.message || 'failed')); } break; }
    if (total == null) total = res?.meta?.total ?? null;
    if (!Array.isArray(list) || !list.length) break;
    list.forEach(f => FUNCLIST.push(mapf(f)));
    FUNCMETA = { total: total ?? FUNCLIST.length, shown: FUNCLIST.length, named: Math.round(FUNCLIST.filter(f => !/^sub_/i.test(f.name)).length / FUNCLIST.length * 100) };
    paintFunctions();
    if (first) { first = false; $('#dockspace .flist .frow')?.click(); }  // decompile the first ASAP
    if (list.length < FUNC_CHUNK) break;       // last page
    offset += FUNC_CHUNK;
    await new Promise(r => setTimeout(r, 0));   // yield so the UI stays responsive
  }
  if (FUNCLIST.length) { FUNCMETA.total = total ?? FUNCLIST.length; paintFunctions(); toast('Loaded ' + FUNCLIST.length.toLocaleString() + ' functions'); }
}

deepLink();
dock.setWorkspace(document.documentElement.dataset.workspace || 'decompile'); // seed/restore the workspace layout
hydrateFromEngine();
console.log('N0xis GUI ready · graph, context menus, dock widgets, palette (Ctrl+P), themes, zoom');
