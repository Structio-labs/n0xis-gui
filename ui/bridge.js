// bridge.js — the thin-client seam between the web UI and the n0x engine.
//
// In the Tauri window, window.__TAURI__ is present (withGlobalTauri=true) and the
// Rust backend exposes commands that shell out to the real `n0xis` binary.
// In a plain browser (design preview) there is no backend, so every call resolves
// to null and the UI keeps its built-in demo data — the frontend stays self-sufficient.

const T = (typeof window !== 'undefined') && (window.__TAURI__ || window.__TAURI_INTERNALS__);
export const isNative = !!T;

function invoke(cmd, args) {
  const core = window.__TAURI__?.core;
  if (!core?.invoke) return Promise.resolve(null);
  return core.invoke(cmd, args).catch(err => ({ ok: false, error: { message: String(err) } }));
}

/** Run any n0x subcommand: n0x(['guide','--brief']) -> {ok,data,meta} | null (web) */
export function n0x(args) {
  if (!isNative) return Promise.resolve(null);
  return invoke('n0x_run', { args });
}

/** Is the engine binary reachable? -> {ok, version, commandCount} | null */
export function engineInfo() {
  if (!isNative) return Promise.resolve(null);
  return invoke('engine_info', {});
}

/** Open a native file picker for a target binary -> path | null */
export function pickFile(title) {
  if (!isNative) return Promise.resolve(null);
  return invoke('pick_file', { title: title || 'Choose a target' });
}

/** A path passed on the command line (n0xis-gui /path) -> path | null */
export function initialTarget() {
  if (!isNative) return Promise.resolve(null);
  return invoke('initial_target', {});
}

/** Host process list with real names (/proc/comm on Linux) -> {ok,data:{processes}} | null */
export function listProcesses() {
  if (!isNative) return Promise.resolve(null);
  return invoke('list_processes', {});
}

/** Convenience wrappers over the CLI's verbs. */
export const engine = {
  guide: (topic) => n0x(topic ? ['guide', topic, '--brief'] : ['guide', '--brief']),
  // profile takes --file (not a positional); --exports pulls the export table too.
  profile: (path) => n0x(['profile', '--file', path, '--exports']),
  // the real verb is `function discover`; --limit 0 = every function (BN/Ghidra
  // show them all; the UI virtualizes the list so huge counts stay smooth).
  functions: (path, limit = 0) =>
    n0x(['function', 'discover', '--file', path, '--limit', String(limit)]),
  decompile: (path, addr, style = 'structured') =>
    n0x(['decomp', 'pseudo', '--file', path, '--addr', addr, '--style', style]),
  disasm: (path, addr, count = 40) =>
    n0x(['disasm', '--file', path, '--addr', addr, '--count', String(count)]),
};
