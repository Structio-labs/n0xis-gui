// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The text an image holds (`strings`), one page at a time.

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// `strings [--contains] --limit --offset`: a page of the image's strings in
/// address order, with how many match in all.
#[derive(Debug, Clone)]
pub struct ListStrings {
    /// Keep only the strings that hold this text, in any case.
    pub contains: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StringEncoding {
    Utf8,
    Utf16le,
    /// One this client does not know yet.
    #[serde(other)]
    Other,
}

impl StringEncoding {
    pub fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16le => "UTF-16",
            Self::Other => "?",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ImageString {
    pub address: String,
    pub section: String,
    pub encoding: StringEncoding,
    /// Characters.
    pub length: u64,
    /// Bytes in the image.
    pub size: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringsPage {
    /// How many strings match in all, not only on this page.
    pub total: u64,
    pub strings: Vec<ImageString>,
}

#[derive(Deserialize)]
struct Data {
    strings: Vec<ImageString>,
}

impl Request for ListStrings {
    type Output = StringsPage;

    fn args(&self) -> Vec<String> {
        let mut a = vec!["strings".to_string(), "--limit".into(), self.limit.to_string(), "--offset".into(), self.offset.to_string()];
        if let Some(text) = self.contains.as_ref().filter(|t| !t.is_empty()) {
            a.extend(["--contains".into(), text.clone()]);
        }
        a
    }

    fn parse(envelope: Envelope) -> Result<StringsPage, ClientError> {
        let (data, meta) = envelope.into_parts::<Data>(schema::STRINGS)?;
        let total = meta.total.ok_or_else(|| ClientError::Decode("the engine did not say how many strings match".into()))?;
        Ok(StringsPage { total, strings: data.strings })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied from a real answer (`strings --contains ПРИВІТ` on the engine's
    /// planted-strings fixture), with the list of ranges read shortened.
    #[test]
    fn a_page_reads_the_engines_own_shape() {
        let e: Envelope = serde_json::from_str(
            r#"{"ok":true,"data":{"count":1,"min":4,"ranges":[{"address":"0x2a8","name":".note.gnu.build-id","size":36},{"address":"0x2000","name":".rodata","size":105}],"strings":[{"address":"0x2030","encoding":"utf8","length":13,"section":".rodata","size":19,"text":"Привіт, n0xis"}]},"meta":{"schema":"n0xis.strings.v1","tool":"n0xis","tool_version":"0.3.3","source":"static:strings_planted.so","returned":1,"total":1,"truncated":false}}"#,
        )
        .unwrap();
        let page = ListStrings::parse(e).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.strings[0].text, "Привіт, n0xis");
        assert_eq!((page.strings[0].encoding, page.strings[0].length, page.strings[0].size), (StringEncoding::Utf8, 13, 19));
        let args = ListStrings { contains: Some("key".into()), limit: 500, offset: 1000 }.args();
        assert_eq!(args, ["strings", "--limit", "500", "--offset", "1000", "--contains", "key"]);
        assert!(!ListStrings { contains: Some(String::new()), limit: 1, offset: 0 }.args().contains(&"--contains".to_string()));
    }
}
