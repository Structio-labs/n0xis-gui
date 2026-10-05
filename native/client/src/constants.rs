// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Known constants in a function (`const identify --addr`): every numeric
//! literal of its decompilation that the engine's table names, with the
//! algorithm it belongs to.

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// `const identify --addr`: the literals of the function at `addr` the
/// engine recognises.
#[derive(Debug, Clone)]
pub struct IdentifyConstants {
    pub addr: String,
}

/// What one recognised value is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConstantMatch {
    pub algorithm: String,
    /// What the value is in that algorithm (a polynomial, an initial value…).
    pub role: String,
    pub value: String,
    pub width: u32,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub formula: Option<String>,
}

/// A literal of the function, and every algorithm it could belong to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConstantHit {
    pub literal: String,
    pub matches: Vec<ConstantMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Constants {
    pub addr: String,
    pub hits: Vec<ConstantHit>,
    /// How many literals were named.
    pub identified: u64,
    /// How many literals the decompilation holds.
    pub literals_scanned: u64,
}

impl Request for IdentifyConstants {
    type Output = Constants;

    fn args(&self) -> Vec<String> {
        vec!["const".into(), "identify".into(), "--addr".into(), self.addr.clone()]
    }

    fn parse(envelope: Envelope) -> Result<Constants, ClientError> {
        let (data, _) = envelope.into_parts::<Constants>(schema::CONST_IDENTIFY)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied from a real answer: `const identify --addr 0xbe70` on the
    /// system's zlib (`crc32_combine`).
    #[test]
    fn an_answer_reads_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"addr":"0xbe70","hits":[{"literal":"0xedb88320","matches":[{"algorithm":"CRC-32","decimal":"3988292384","formula":"crc = (crc>>1) ^ (0xEDB88320 & -(crc&1))","note":"zlib/PNG/Ethernet CRC-32, reflected form","role":"reversed polynomial","value":"0xedb88320","width":32}]}],"identified":1,"literals_scanned":66},"meta":{"schema":"n0xis.const.identify.v1","tool":"n0xis","tool_version":"0.3.3","source":"static:libz.so.1"}}"#,
        )
        .unwrap();
        let found = IdentifyConstants::parse(e).unwrap();
        assert_eq!((found.identified, found.literals_scanned), (1, 66));
        assert_eq!(found.hits[0].literal, "0xedb88320");
        assert_eq!(found.hits[0].matches[0].algorithm, "CRC-32");
        assert_eq!(found.hits[0].matches[0].role, "reversed polynomial");
        assert_eq!(IdentifyConstants { addr: "0xbe70".into() }.args(), ["const", "identify", "--addr", "0xbe70"]);
    }

    /// The engine answers a single `--value` under the same schema with
    /// another shape; that answer is refused, not misread as an empty list.
    #[test]
    fn the_single_value_shape_is_refused() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"match_count":1,"matches":[],"value":"0xedb88320"},"meta":{"schema":"n0xis.const.identify.v1","tool":"n0xis","tool_version":"0.3.3"}}"#,
        )
        .unwrap();
        assert!(IdentifyConstants::parse(e).is_err());
    }
}
