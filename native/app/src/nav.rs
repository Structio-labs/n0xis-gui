// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Where the user is: one address, and the function it lies in when the
//! engine's function list says so. Every view that can send the user somewhere
//! emits [`Navigate`]; the workbench turns it into a [`Location`] and shows it
//! everywhere.

use n0xis_client::FunctionEntry;

/// A place in the target. `function` is set only when the engine's list puts
/// `va` inside a function: at its start, or within the extent it states. An
/// address the list does not cover gets no function, rather than a guess.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub va: u64,
    pub function: Option<FunctionEntry>,
}

impl Location {
    /// The function's start when there is one, else the address itself: what a
    /// view about "the selected function" should ask about.
    pub fn subject(&self) -> u64 {
        self.function.as_ref().and_then(|f| parse_va(&f.va)).unwrap_or(self.va)
    }

    /// `name` at a function start, `name+0x12` inside one, the bare address elsewhere.
    pub fn label(&self) -> String {
        match &self.function {
            Some(f) => match parse_va(&f.va) {
                Some(start) if start != self.va => format!("{}+{:#x}", f.name, self.va.wrapping_sub(start)),
                _ => f.name.clone(),
            },
            None => hex(self.va),
        }
    }
}

/// Emitted by a view when the user asks to go to an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Navigate(pub u64);

/// An engine address (`0x1800aa891`) as a number.
pub fn parse_va(s: &str) -> Option<u64> {
    let s = s.trim();
    let digits = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u64::from_str_radix(digits, 16).ok()
}

/// The engine's own spelling of an address: lower-case hex with `0x`.
pub fn hex(va: u64) -> String {
    format!("{va:#x}")
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{Location, hex, parse_va};
    use n0xis_client::FunctionEntry;

    #[test]
    fn addresses_read_and_print_in_the_engines_spelling() {
        assert_eq!(parse_va("0x1800AA891"), Some(0x1800aa891));
        assert_eq!(parse_va("1000"), Some(0x1000));
        assert_eq!(parse_va("not hex"), None);
        assert_eq!(hex(0x1800aa891), "0x1800aa891");
    }

    #[test]
    fn a_location_inside_a_function_is_labelled_by_its_offset() {
        let f = FunctionEntry { name: "parse".into(), va: "0x1000".into(), end: Some("0x1080".into()) };
        let inside = Location { va: 0x1012, function: Some(f.clone()) };
        assert_eq!(inside.label(), "parse+0x12");
        assert_eq!(inside.subject(), 0x1000);
        assert_eq!(Location { va: 0x1000, function: Some(f) }.label(), "parse");
        assert_eq!(Location { va: 0x9000, function: None }.label(), "0x9000");
    }
}
