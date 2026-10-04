// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! `profile`: what the image is, read from its own headers.

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// `profile`, with the full export list when `exports` is set.
#[derive(Debug, Clone, Copy, Default)]
pub struct Profile {
    pub exports: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProfileReport {
    pub image: ImageProfile,
    /// Commands that will not do what their name suggests on this image, and why.
    #[serde(default)]
    pub advisories: Vec<Advisory>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImageProfile {
    pub machine: String,
    pub module_base: String,
    pub image_end: String,
    pub sections: Vec<Section>,
    pub export_count: u64,
    pub export_distinct_addresses: u64,
    /// Exports that name a function in another module rather than code in this
    /// one. `None` when the engine does not report them: such an engine lists a
    /// forwarder at the address of its name string, which is data, not code.
    #[serde(default)]
    pub forwarded_count: Option<u64>,
    pub thunk_count: u64,
    pub pdata_present: bool,
    pub pdata_functions: u64,
    #[serde(default)]
    pub exports: Vec<Export>,
    #[serde(default)]
    pub folded: Vec<FoldedExports>,
    #[serde(default)]
    pub detoured_exports: Vec<String>,
    #[serde(default)]
    pub engine_hints: Vec<EngineHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Section {
    pub name: String,
    pub va: String,
    pub virtual_size: u64,
    pub raw_size: u64,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Export {
    pub name: String,
    /// The address the export table holds. For a forwarder that is the
    /// address of its `MODULE.Function` string, not code.
    pub va: String,
    /// Where a one-instruction branch stub at `va` leads.
    #[serde(default)]
    pub thunk_target: Option<String>,
    #[serde(default)]
    pub thunk_kind: Option<String>,
    /// The `MODULE.Function` this export forwards to.
    #[serde(default)]
    pub forwarder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FoldedExports {
    pub va: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EngineHint {
    pub engine: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Advisory {
    pub command: String,
    /// `ineffective` or `degraded`.
    pub verdict: String,
    pub reason: String,
}

impl Request for Profile {
    type Output = ProfileReport;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["profile".to_string()];
        if self.exports {
            a.push("--exports".into());
        }
        a
    }

    fn parse(envelope: Envelope) -> Result<ProfileReport, ClientError> {
        Ok(envelope.into_parts::<ProfileReport>(schema::PROFILE)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied from a real answer (n0xis 0.3.2 on an x64 PE DLL), cut down.
    #[test]
    fn a_profile_reads_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"advisories":[{"command":"decomp pseudo","reason":"45 exports are branch stubs","verdict":"degraded"}],
            "il2cpp":null,"image":{"detoured_exports":[],"engine_hints":[],"export_count":1698,"export_distinct_addresses":1645,
            "exports":[{"name":"AcquireLock","va":"0x1800aa891"},
                       {"name":"OpenThing","va":"0x180057040","thunk_kind":"indirect","thunk_target":"0x7ffb0000"}],
            "folded":[{"names":["alpha","beta"],"va":"0x1800223c0"}],"image_end":"0x1800cb000","machine":"x64",
            "module_base":"0x180000000","pdata_functions":1531,"pdata_present":true,
            "sections":[{"executable":true,"name":".text","raw_size":548864,"va":"0x180001000","virtual_size":545444}],
            "thunk_count":45}},"meta":{"schema":"n0xis.profile.v1","source":"static:x.dll"}}"#,
        )
        .unwrap();
        let p = Profile { exports: true }.args();
        assert_eq!(p, ["profile", "--exports"]);
        let r = Profile::parse(e).unwrap();
        assert_eq!(r.image.export_count, 1698);
        assert_eq!(r.image.forwarded_count, None, "an engine that does not report forwarders is not read as zero");
        assert_eq!(r.image.exports[1].thunk_kind.as_deref(), Some("indirect"));
        assert_eq!(r.image.sections[0].name, ".text");
        assert_eq!(r.advisories[0].verdict, "degraded");
    }
}
