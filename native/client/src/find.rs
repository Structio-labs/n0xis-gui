// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! `find`: search the image's bytes for a string or a byte pattern.

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// What to look for. The engine reads each form; the client only names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindQuery {
    /// Literal text, as UTF-8 or as UTF-16LE.
    Text { text: String, utf16: bool },
    /// Hex bytes with `?`/`??` wildcards, e.g. `48 8B ?? C3`.
    Bytes(String),
    /// Text with `\xNN`, `\n`, `\t`, `\r`, `\0`, `\\`, `\"` escapes.
    Escaped(String),
}

#[derive(Debug, Clone)]
pub struct Find {
    pub query: FindQuery,
    /// At most this many matches come back (0 lets the engine return them all).
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FindMatch {
    pub va: String,
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FindResult {
    pub count: u64,
    /// More matches exist than were returned.
    pub truncated: bool,
    pub bytes_scanned: u64,
    #[serde(default)]
    pub pattern_len: Option<u64>,
    pub matches: Vec<FindMatch>,
}

impl Request for Find {
    type Output = FindResult;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["find".to_string()];
        match &self.query {
            FindQuery::Text { text, utf16 } => {
                a.extend(["--string".into(), text.clone()]);
                if *utf16 {
                    a.push("--utf16".into());
                }
            }
            FindQuery::Bytes(p) => a.extend(["--bytes".into(), p.clone()]),
            FindQuery::Escaped(s) => a.extend(["--escaped".into(), s.clone()]),
        }
        a.extend(["--limit".into(), self.limit.to_string()]);
        a
    }

    fn parse(envelope: Envelope) -> Result<FindResult, ClientError> {
        Ok(envelope.into_parts::<FindResult>(schema::FIND)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied from a real answer (n0xis 0.3.2).
    #[test]
    fn a_find_result_reads_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"bytes_scanned":827392,"count":1,"matches":[{"section":".rdata","va":"0x1800b9a70"}],
            "pattern_len":6,"truncated":false},"meta":{"schema":"n0xis.find.v1"}}"#,
        )
        .unwrap();
        let r = Find::parse(e).unwrap();
        assert_eq!(r.matches[0].section.as_deref(), Some(".rdata"));
        assert!(!r.truncated);
    }

    #[test]
    fn each_query_form_names_its_own_flag() {
        let f = |query| Find { query, limit: 300 }.args();
        assert_eq!(f(FindQuery::Text { text: "a b".into(), utf16: true }), ["find", "--string", "a b", "--utf16", "--limit", "300"]);
        assert_eq!(f(FindQuery::Bytes("48 ?? c3".into()))[1], "--bytes");
        assert_eq!(f(FindQuery::Escaped(r"\x00".into()))[1], "--escaped");
    }
}
