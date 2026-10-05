// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The project's caches (`project cache`): what they take on disk, and clearing
//! them. Which directories are caches is the engine's answer, never the GUI's:
//! names, comments, types and patches sit in the same `.n0x/` and are not caches.

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// `project cache [--clear]`.
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheUsage {
    pub clear: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CacheKind {
    pub name: String,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CacheReport {
    /// The project folder the engine resolved.
    pub dir: String,
    /// `false` when the engine fell back to its global project: the session is
    /// not where the GUI meant it to be.
    pub is_local: bool,
    pub caches: Vec<CacheKind>,
    /// All caches together, before any clear.
    pub bytes: u64,
    pub cleared: bool,
    pub freed: u64,
    #[serde(default)]
    pub failures: Vec<String>,
}

impl Request for CacheUsage {
    type Output = CacheReport;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["project".to_string(), "cache".to_string()];
        if self.clear {
            a.push("--clear".into());
        }
        a
    }

    fn parse(envelope: Envelope) -> Result<CacheReport, ClientError> {
        Ok(envelope.into_parts::<CacheReport>(schema::PROJECT_CACHE)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied from a real answer (n0xis after `baf979d`).
    #[test]
    fn a_cache_report_reads_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"bytes":1324,"caches":[{"bytes":1024,"dir":"/p/.n0x/ir-cache","files":2,"name":"ir-cache"},
            {"bytes":300,"dir":"/p/.n0x/xref-index","files":1,"name":"xref-index"}],"cleared":false,"dir":"/p/.n0x",
            "failures":[],"freed":0,"is_local":true},"meta":{"schema":"n0xis.project.cache.v1","tool":"n0xis"}}"#,
        )
        .unwrap();
        let r = CacheUsage::parse(e).unwrap();
        assert_eq!((r.bytes, r.caches[0].files), (1324, 2));
        assert!(r.is_local && !r.cleared);
        assert_eq!(CacheUsage { clear: true }.args(), ["project", "cache", "--clear"]);
    }
}
