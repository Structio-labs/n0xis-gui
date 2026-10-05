// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The engine's whole-program pass (`analyze`), run beside the session as a
//! process of its own. It discovers functions, recovers class names and builds
//! the reverse-reference index into the project folder, where the session
//! reads them: the index on its first `xref to`, the names whenever their file
//! changes. Its progress arrives on stderr as `[n0x] {"phase","done","total"}`.

use std::io::{BufRead as _, BufReader, Read as _};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};

use serde::Deserialize;

use crate::{EngineCommand, Envelope, schema};

/// How much of the IR cache the pass warms up after the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmUp {
    /// None: decompiling a function caches it the first time it is shown.
    Skip,
    /// Every function: the first decompile of any of them is instant, at the
    /// cost of time and disk now.
    All,
}

/// A step of the pass, by the engine's own names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Discovering,
    ScanningRtti,
    IndexingXrefs,
    Disassembling,
    Done,
    /// A step this client does not know by name; shown as the engine wrote it.
    Other(String),
}

impl Phase {
    fn from_engine(name: &str) -> Self {
        match name {
            "starting" => Self::Starting,
            "discovering" => Self::Discovering,
            "scanning-rtti" => Self::ScanningRtti,
            "indexing-xrefs" => Self::IndexingXrefs,
            "disassembling" => Self::Disassembling,
            "done" => Self::Done,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Starting => "starting",
            Self::Discovering => "discovering functions",
            Self::ScanningRtti => "scanning RTTI",
            Self::IndexingXrefs => "indexing references",
            Self::Disassembling => "warming the decompiler cache",
            Self::Done => "finishing",
            Self::Other(name) => name,
        }
    }

    /// Whether the reverse-reference index is on disk by this step. The engine
    /// runs discovery, RTTI and the index first, then the warm-up.
    pub fn index_written(&self) -> bool {
        matches!(self, Self::Disassembling | Self::Done)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,
    pub done: u64,
    pub total: u64,
}

/// What the finished pass reports (`n0xis.analyze.v1`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AnalysisReport {
    pub functions: u64,
    pub xref_targets: u64,
    pub rtti_classes: u64,
    #[serde(default)]
    pub cached_functions: u64,
    #[serde(default)]
    pub flirt_named: u64,
}

impl AnalysisReport {
    /// Whether the pass can have changed what functions are called.
    pub fn names_changed(&self) -> bool {
        self.rtti_classes > 0 || self.flirt_named > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisState {
    Running(Progress),
    Finished(AnalysisReport),
    Failed(String),
}

impl AnalysisState {
    /// Whether `xref to` can read the index instead of building it in the session.
    pub fn index_ready(&self) -> bool {
        match self {
            Self::Running(p) => p.phase.index_written(),
            Self::Finished(_) => true,
            // A failed pass leaves the session to build the index itself.
            Self::Failed(_) => true,
        }
    }
}

/// How much of the pass's stderr is kept to explain a failure.
const STDERR_TAIL_LINES: usize = 20;

/// A running (or finished) pass. Dropping it stops the process.
pub struct Analysis {
    state: Arc<Mutex<AnalysisState>>,
    child: Arc<Mutex<Option<Child>>>,
}

impl Analysis {
    /// Start `analyze` on `file`, in `project` so that it writes where the
    /// session reads.
    pub fn start(engine: &EngineCommand, file: &Path, project: Option<&Path>, warm_up: WarmUp) -> Self {
        let state = Arc::new(Mutex::new(AnalysisState::Running(Progress { phase: Phase::Starting, done: 0, total: 0 })));
        let child = Arc::new(Mutex::new(None));
        let mut cmd = engine.command();
        cmd.arg("analyze").arg("--file").arg(file);
        if warm_up == WarmUp::Skip {
            cmd.arg("--no-cfg");
        }
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(dir) = project {
            cmd.current_dir(dir);
        }
        let mut spawned = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                *lock(&state) = AnalysisState::Failed(format!("cannot start {}: {e}", engine.program().display()));
                return Self { state, child };
            }
        };
        let (stdout, stderr) = (spawned.stdout.take(), spawned.stderr.take());
        *lock(&child) = Some(spawned);
        let (thread_state, thread_child) = (Arc::clone(&state), Arc::clone(&child));
        std::thread::spawn(move || {
            let mut tail = std::collections::VecDeque::new();
            if let Some(err) = stderr {
                for line in BufReader::new(err).lines().map_while(Result::ok) {
                    match progress_of(&line) {
                        Some(p) => *lock(&thread_state) = AnalysisState::Running(p),
                        None => {
                            tail.push_back(line);
                            if tail.len() > STDERR_TAIL_LINES {
                                tail.pop_front();
                            }
                        }
                    }
                }
            }
            let mut out = String::new();
            if let Some(mut s) = stdout {
                let _ = s.read_to_string(&mut out);
            }
            let status = lock(&thread_child).take().map(|mut c| c.wait());
            let end = finish(&out, status.map(|s| s.map(|s| s.to_string())), tail.into_iter().collect::<Vec<_>>().join("\n"));
            *lock(&thread_state) = end;
        });
        Self { state, child }
    }

    pub fn state(&self) -> AnalysisState {
        lock(&self.state).clone()
    }
}

impl Drop for Analysis {
    fn drop(&mut self) {
        if let Some(child) = lock(&self.child).as_mut() {
            let _ = child.kill();
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A `[n0x] {"phase":…,"done":…,"total":…}` line as progress; anything else
/// (a warning, a message) is not.
fn progress_of(line: &str) -> Option<Progress> {
    #[derive(Deserialize)]
    struct Line {
        phase: String,
        #[serde(default)]
        done: u64,
        #[serde(default)]
        total: u64,
    }
    let json = line.strip_prefix("[n0x] ")?;
    let l: Line = serde_json::from_str(json).ok()?;
    Some(Progress { phase: Phase::from_engine(&l.phase), done: l.done, total: l.total })
}

/// The pass's end, from its stdout envelope. No envelope at all is a failure
/// that says how the process ended and what it last wrote.
fn finish(stdout: &str, status: Option<std::io::Result<String>>, stderr_tail: String) -> AnalysisState {
    let ended = match status {
        Some(Ok(s)) => s,
        Some(Err(e)) => format!("could not be waited for ({e})"),
        None => "was stopped".to_string(),
    };
    match serde_json::from_str::<Envelope>(stdout.trim()) {
        Ok(envelope) => match envelope.into_parts::<AnalysisReport>(schema::ANALYZE) {
            Ok((report, _)) => AnalysisState::Finished(report),
            Err(e) => AnalysisState::Failed(e.to_string()),
        },
        Err(_) => AnalysisState::Failed(format!("the engine {ended} without an answer. {}", stderr_tail.trim())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines copied from a real run (n0xis 0.3.3, `analyze --no-cfg`).
    #[test]
    fn progress_lines_read_the_engines_own_shape() {
        let p = progress_of(r#"[n0x] {"done":17,"phase":"indexing-xrefs","total":17}"#).unwrap();
        assert_eq!(p, Progress { phase: Phase::IndexingXrefs, done: 17, total: 17 });
        assert_eq!(progress_of(r#"[n0x] {"warn":"persist rtti-symbols: denied"}"#), None, "a warning is not progress");
        assert_eq!(progress_of("plain text"), None);
        let other = progress_of(r#"[n0x] {"phase":"something-new","done":1,"total":2}"#).unwrap();
        assert_eq!(other.phase.label(), "something-new");
    }

    #[test]
    fn the_index_is_ready_once_its_phase_is_past() {
        let at = |phase| AnalysisState::Running(Progress { phase, done: 0, total: 0 });
        assert!(!at(Phase::Discovering).index_ready());
        assert!(!at(Phase::IndexingXrefs).index_ready(), "still being written");
        assert!(at(Phase::Disassembling).index_ready());
        assert!(AnalysisState::Failed("x".into()).index_ready(), "a failed pass must not hold queries back forever");
    }

    #[test]
    fn the_end_reads_the_report_or_says_how_the_process_ended() {
        let out = r#"{"ok":true,"data":{"cached_functions":0,"flirt_named":0,"functions":7,"rtti_classes":0,"xref_targets":17},
            "meta":{"schema":"n0xis.analyze.v1","tool":"n0xis","tool_version":"0.3.3"}}"#;
        match finish(out, Some(Ok("exit status: 0".into())), String::new()) {
            AnalysisState::Finished(r) => {
                assert_eq!((r.functions, r.xref_targets), (7, 17));
                assert!(!r.names_changed());
            }
            other => panic!("{other:?}"),
        }
        match finish("", Some(Ok("signal: 9 (SIGKILL)".into())), "thread panicked".into()) {
            AnalysisState::Failed(why) => assert!(why.contains("SIGKILL") && why.contains("panicked"), "{why}"),
            other => panic!("{other:?}"),
        }
    }
}
