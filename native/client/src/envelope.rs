// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::ClientError;

/// One engine answer: `{"ok":true,"data":…,"meta":…}` or
/// `{"ok":false,"error":{"code":…,"message":…}}`.
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    pub ok: bool,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub error: Option<EngineError>,
    #[serde(default)]
    pub meta: Meta,
    /// The answer exactly as the engine wrote it, kept only where it is shown
    /// as is (the console); typed views read the fields above.
    #[serde(skip)]
    pub raw: Option<Value>,
}

/// The engine's own report of a failure.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EngineError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub hint: Option<String>,
}

/// Which payload shape `data` has, and the paging facts some commands add.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub truncated: Option<bool>,
}

impl Envelope {
    /// The payload as `T` together with `meta`, after checking that the answer
    /// is a success of the shape that was asked for. Another `meta.schema` is a
    /// protocol error rather than a best-effort decode: the engine names the shape
    /// of every answer, and the client goes by that name.
    pub fn into_parts<T: DeserializeOwned>(self, schema: &str) -> Result<(T, Meta), ClientError> {
        if !self.ok {
            return Err(ClientError::Engine(self.error.unwrap_or_else(|| EngineError {
                code: "unknown".into(),
                message: "the engine reported a failure without saying why".into(),
                hint: None,
            })));
        }
        if self.meta.schema.as_deref() != Some(schema) {
            return Err(ClientError::Protocol(format!(
                "expected a {schema} answer, got {}",
                self.meta.schema.as_deref().unwrap_or("one without a schema")
            )));
        }
        let data = serde_json::from_value(self.data).map_err(|e| ClientError::Decode(format!("{schema}: {e}")))?;
        Ok((data, self.meta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(json: &str) -> Envelope {
        serde_json::from_str(json).expect("envelope")
    }

    #[test]
    fn a_failure_keeps_the_engines_own_code_and_message() {
        let e = env(r#"{"ok":false,"error":{"code":"missing-source","message":"no file"}}"#);
        let err = e.into_parts::<Value>("n0xis.x.v1").unwrap_err();
        assert_eq!(
            err,
            ClientError::Engine(EngineError { code: "missing-source".into(), message: "no file".into(), hint: None })
        );
    }

    #[test]
    fn an_answer_of_another_shape_is_refused_not_decoded() {
        let e = env(r#"{"ok":true,"data":{"pseudo":[]},"meta":{"schema":"n0xis.decode.v1"}}"#);
        let err = e.into_parts::<Value>("n0x.decomp.pseudo.v1").unwrap_err();
        assert!(matches!(err, ClientError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn an_answer_without_a_schema_is_refused() {
        let e = env(r#"{"ok":true,"data":{}}"#);
        assert!(matches!(e.into_parts::<Value>("n0xis.x.v1"), Err(ClientError::Protocol(_))));
    }
}
