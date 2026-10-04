// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Running processes and value scans over their memory (`process ps`,
//! `mem map`, `scan value`, `scan filter`). These name their own source
//! (`--pid`), so they need no session: [`EngineCommand::run`] runs them once,
//! in a directory whose `.n0x/` keeps the scan results a later scan narrows.

use std::io::Read as _;
use std::path::Path;
use std::process::Stdio;

use serde::Deserialize;

use crate::{ClientError, EngineCommand, Envelope, Request, schema};

/// How much of the engine's stderr is kept to explain a failed run.
const STDERR_TAIL_BYTES: usize = 8 * 1024;

impl EngineCommand {
    /// Run one command to completion and read its envelope. Blocking: call it
    /// from a background thread.
    pub fn run(&self, args: &[String], dir: Option<&Path>) -> Result<Envelope, ClientError> {
        let mut cmd = self.command();
        cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }
        let mut child = cmd.spawn().map_err(|e| ClientError::Spawn(format!("{}: {e}", self.program().display())))?;
        // Drain stderr on its own thread, so a chatty run cannot block on a full pipe.
        let stderr = child.stderr.take();
        let tail = std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut s) = stderr {
                let _ = s.read_to_string(&mut text);
            }
            let cut = text.len().saturating_sub(STDERR_TAIL_BYTES);
            let cut = (cut..text.len()).find(|&i| text.is_char_boundary(i)).unwrap_or(text.len());
            text[cut..].trim().to_string()
        });
        let mut out = String::new();
        if let Some(mut s) = child.stdout.take() {
            let _ = s.read_to_string(&mut out);
        }
        let status = child.wait().map_err(|e| ClientError::Spawn(e.to_string()))?;
        let tail = tail.join().unwrap_or_default();
        // A failing command still prints its envelope; only no envelope at all
        // is a broken run.
        serde_json::from_str::<Envelope>(out.trim()).map_err(|e| {
            let shown: String = out.trim().chars().take(200).collect();
            ClientError::Protocol(format!("the engine exited ({status}) without an envelope ({e}): {shown} {tail}"))
        })
    }

    /// [`EngineCommand::run`] for a typed request.
    pub fn run_request<R: Request>(&self, request: &R, dir: Option<&Path>) -> Result<R::Output, ClientError> {
        R::parse(self.run(&request.args(), dir)?)
    }
}

/// `process ps`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListProcesses;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Processes {
    pub processes: Vec<ProcessInfo>,
}

impl Request for ListProcesses {
    type Output = Processes;

    fn args(&self) -> Vec<String> {
        vec!["process".into(), "ps".into()]
    }

    fn parse(envelope: Envelope) -> Result<Processes, ClientError> {
        Ok(envelope.into_parts::<Processes>(schema::PROCESS_PS)?.0)
    }
}

/// `mem map --pid`: the process's regions; reading them is the access check.
#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    pub pid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Region {
    pub base: String,
    pub end: String,
    pub protect: String,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Regions {
    pub regions: Vec<Region>,
}

impl Request for MemoryMap {
    type Output = Regions;

    fn args(&self) -> Vec<String> {
        vec!["mem".into(), "map".into(), "--pid".into(), self.pid.to_string()]
    }

    fn parse(envelope: Envelope) -> Result<Regions, ClientError> {
        Ok(envelope.into_parts::<Regions>(schema::MEM_MAP)?.0)
    }
}

/// The value types a scan reads, by the engine's names.
pub const SCAN_TYPES: [&str; 10] = ["i32", "u32", "i64", "u64", "f32", "f64", "i8", "u8", "i16", "u16"];

/// `scan value`: every address in the process's writable memory that holds
/// `value` as `value_type`, saved as `save_as` for a later narrowing scan.
#[derive(Debug, Clone)]
pub struct ScanValue {
    pub pid: u32,
    pub value_type: String,
    /// As typed; the engine parses it for the type.
    pub value: String,
    pub save_as: String,
}

/// What a narrowing scan keeps of the previous result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Narrow {
    Exact(String),
    Changed,
    Unchanged,
    Increased,
    Decreased,
}

/// `scan filter`: narrow the result saved as `from`.
#[derive(Debug, Clone)]
pub struct ScanFilter {
    pub pid: u32,
    pub from: String,
    pub narrow: Narrow,
    pub save_as: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScanMatch {
    pub addr: String,
    /// Kept as the engine wrote it: a 64-bit integer must not pass through a float.
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScanResult {
    pub matches: Vec<ScanMatch>,
    pub total_matches: u64,
    pub shown: u64,
    /// Every region of the scan set was read.
    pub exhaustive: bool,
    pub regions: u64,
    pub value_type: String,
}

impl Request for ScanValue {
    type Output = ScanResult;

    fn args(&self) -> Vec<String> {
        vec![
            "scan".into(),
            "value".into(),
            "--pid".into(),
            self.pid.to_string(),
            "--type".into(),
            self.value_type.clone(),
            "--value".into(),
            self.value.clone(),
            "--save-as".into(),
            self.save_as.clone(),
        ]
    }

    fn parse(envelope: Envelope) -> Result<ScanResult, ClientError> {
        Ok(envelope.into_parts::<ScanResult>(schema::SCAN)?.0)
    }
}

impl Request for ScanFilter {
    type Output = ScanResult;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["scan".into(), "filter".into(), "--pid".into(), self.pid.to_string(), "--from".into(), self.from.clone()];
        match &self.narrow {
            Narrow::Exact(v) => a.extend(["--criterion".into(), "exact".into(), "--value".into(), v.clone()]),
            Narrow::Changed => a.extend(["--criterion".into(), "changed".into()]),
            Narrow::Unchanged => a.extend(["--criterion".into(), "unchanged".into()]),
            Narrow::Increased => a.extend(["--criterion".into(), "increased".into()]),
            Narrow::Decreased => a.extend(["--criterion".into(), "decreased".into()]),
        }
        a.extend(["--save-as".into(), self.save_as.clone()]);
        a
    }

    fn parse(envelope: Envelope) -> Result<ScanResult, ClientError> {
        Ok(envelope.into_parts::<ScanResult>(schema::SCAN)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(json: &str) -> Envelope {
        serde_json::from_str(json).expect("envelope")
    }

    /// Copied from real answers (n0xis 0.3.2) on a process that printed the
    /// address of the value it holds.
    #[test]
    fn a_scan_reads_the_engines_own_shape_and_keeps_big_values_exact() {
        let r = ScanValue::parse(env(
            r#"{"ok":true,"data":{"exhaustive":true,"matches":[{"addr":"0x561fd3d5d030","value":1592594996},
            {"addr":"0x561fd3d5d038","value":18446744073709551615}],"regions":1,"shown":2,"total_matches":2,"value_type":"u64"},
            "meta":{"schema":"n0xis.scan.v1","source":"live:142246:holder"}}"#,
        ))
        .unwrap();
        assert_eq!(r.matches[0].addr, "0x561fd3d5d030");
        assert_eq!(r.matches[1].value.to_string(), "18446744073709551615", "no float in between");
        assert!(r.exhaustive);
    }

    #[test]
    fn a_narrowing_scan_names_its_criterion() {
        let f = |narrow| ScanFilter { pid: 7, from: "a".into(), narrow, save_as: "b".into() }.args();
        assert_eq!(f(Narrow::Changed), ["scan", "filter", "--pid", "7", "--from", "a", "--criterion", "changed", "--save-as", "b"]);
        assert!(f(Narrow::Exact("5".into())).windows(2).any(|w| w == ["--value", "5"]));
    }

    #[test]
    fn processes_and_regions_read_the_engines_own_shape() {
        let p = ListProcesses::parse(env(
            r#"{"ok":true,"data":{"count":1,"processes":[{"name":"holder","pid":142246}]},"meta":{"schema":"n0xis.process.ps.v1"}}"#,
        ))
        .unwrap();
        assert_eq!(p.processes[0].pid, 142246);
        let m = MemoryMap::parse(env(
            r#"{"ok":true,"data":{"count":1,"regions":[{"base":"0x561fd3d59000","end":"0x561fd3d5a000","kind":"image","protect":"r--","size":4096,"state":"commit"}]},"meta":{"schema":"n0xis.mem.map.v1"}}"#,
        ))
        .unwrap();
        assert_eq!(m.regions[0].protect, "r--");
    }
}
