// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! A window of the target's bytes (`mem span`), for the hex view. Every
//! address in the window either holds a byte the engine read or nothing it can
//! read (unmapped, or a zero-fill tail); a gap never ends the window.

use serde::Deserialize;

use crate::{ClientError, Envelope, Request, schema};

/// The widest window the engine reads at once (its `MAX_SPAN`). An answer
/// claiming more is refused, never allocated.
pub const MAX_WINDOW: u32 = 64 * 1024;

/// `mem span --addr --size`: every readable stretch of `size` bytes from `addr`.
#[derive(Debug, Clone)]
pub struct MemSpan {
    pub addr: String,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: u64,
    /// One entry per address of the window: `None` where the engine has no byte.
    pub bytes: Vec<Option<u8>>,
}

impl Span {
    /// How many addresses of the window hold a byte.
    pub fn read(&self) -> usize {
        self.bytes.iter().filter(|b| b.is_some()).count()
    }

    pub fn contains(&self, va: u64) -> bool {
        va >= self.start && va - self.start < self.bytes.len() as u64
    }

    /// The byte at `va`; `None` outside the window or where nothing can be read.
    pub fn byte_at(&self, va: u64) -> Option<u8> {
        let i = usize::try_from(va.checked_sub(self.start)?).ok()?;
        self.bytes.get(i).copied().flatten()
    }
}

#[derive(Deserialize)]
struct Data {
    address: String,
    size: u64,
    read: u64,
    runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    address: String,
    read: u64,
    hex: String,
}

fn address(s: &str) -> Result<u64, ClientError> {
    let digits = s.strip_prefix("0x").ok_or_else(|| ClientError::Decode(format!("address {s:?} is not hex")))?;
    u64::from_str_radix(digits, 16).map_err(|e| ClientError::Decode(format!("address {s:?}: {e}")))
}

impl Request for MemSpan {
    type Output = Span;

    fn args(&self) -> Vec<String> {
        vec!["mem".into(), "span".into(), "--addr".into(), self.addr.clone(), "--size".into(), self.size.to_string()]
    }

    fn parse(envelope: Envelope) -> Result<Span, ClientError> {
        let d = envelope.into_parts::<Data>(schema::MEM_SPAN)?.0;
        let start = address(&d.address)?;
        if d.size > u64::from(MAX_WINDOW) {
            return Err(ClientError::Decode(format!("a window of {} bytes is wider than the engine reads", d.size)));
        }
        let mut bytes = vec![None; d.size as usize];
        let mut total = 0u64;
        for run in &d.runs {
            let at = address(&run.address)?;
            let run_bytes = run
                .hex
                .split_whitespace()
                .map(|b| u8::from_str_radix(b, 16).map_err(|e| ClientError::Decode(format!("byte {b:?}: {e}"))))
                .collect::<Result<Vec<u8>, _>>()?;
            // Every count the engine states must be the count it sent, and every
            // byte must fall in the window it was asked for.
            if run_bytes.len() as u64 != run.read {
                return Err(ClientError::Decode(format!("a run at {} says {} bytes and holds {}", run.address, run.read, run_bytes.len())));
            }
            let offset = at
                .checked_sub(start)
                .filter(|o| o.saturating_add(run.read) <= d.size)
                .ok_or_else(|| ClientError::Decode(format!("a run at {} lies outside the window", run.address)))?;
            for (i, b) in run_bytes.into_iter().enumerate() {
                bytes[offset as usize + i] = Some(b);
            }
            total += run.read;
        }
        if total != d.read {
            return Err(ClientError::Decode(format!("the engine says it read {} bytes and sent {total}", d.read)));
        }
        Ok(Span { start, bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(json: &str) -> Envelope {
        serde_json::from_str(json).unwrap()
    }

    /// Copied from a real answer: `mem span` across the gap between `.init` and
    /// `.plt` of the engine's ELF fixture (`tls_overlap.so`, 0x1018, 12 bytes).
    #[test]
    fn a_span_places_each_run_and_leaves_the_gaps_empty() {
        let e = envelope(
            r#"{"ok":true,"data":{"address":"0x1018","read":7,"runs":[{"address":"0x1018","hex":"c4 08 c3","read":3},{"address":"0x1020","hex":"ff 35 ca 2f","read":4}],"size":12},"meta":{"schema":"n0xis.mem.span.v1","tool":"n0xis","tool_version":"0.3.3","source":"static:tls_overlap.so"}}"#,
        );
        let s = MemSpan::parse(e).unwrap();
        assert_eq!(s.start, 0x1018);
        assert_eq!(s.bytes, [Some(0xc4), Some(0x08), Some(0xc3), None, None, None, None, None, Some(0xff), Some(0x35), Some(0xca), Some(0x2f)]);
        assert_eq!((s.read(), s.byte_at(0x101b), s.byte_at(0x1020), s.byte_at(0x2000)), (7, None, Some(0xff), None));
        assert_eq!(MemSpan { addr: "0x10".into(), size: 64 }.args(), ["mem", "span", "--addr", "0x10", "--size", "64"]);
    }

    #[test]
    fn an_answer_that_contradicts_itself_is_refused() {
        let outside = envelope(
            r#"{"ok":true,"data":{"address":"0x1000","size":4,"read":4,"runs":[{"address":"0x1002","read":4,"hex":"00 01 02 03"}]},"meta":{"schema":"n0xis.mem.span.v1"}}"#,
        );
        assert!(MemSpan::parse(outside).is_err(), "a run past the window's end");
        let miscounted = envelope(
            r#"{"ok":true,"data":{"address":"0x1000","size":4,"read":2,"runs":[{"address":"0x1000","read":3,"hex":"00 01 02"}]},"meta":{"schema":"n0xis.mem.span.v1"}}"#,
        );
        assert!(MemSpan::parse(miscounted).is_err(), "a total that is not the runs' sum");
        let huge = envelope(
            r#"{"ok":true,"data":{"address":"0x1000","size":18446744073709551615,"read":0,"runs":[]},"meta":{"schema":"n0xis.mem.span.v1"}}"#,
        );
        assert!(MemSpan::parse(huge).is_err(), "a window wider than the engine reads is never allocated");
    }
}
