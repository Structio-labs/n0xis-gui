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

/// Emitted by a view when the user points at an address it already shows (a
/// click on a row). The selection moves there, but it is not a step in the
/// history: Back returns to the last place gone to, not the last row clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point(pub u64);

/// How many places back the history reaches.
pub const MAX_HISTORY: usize = 300;

/// The places the user went, to go back and forward through.
#[derive(Default)]
pub struct NavHistory {
    back: Vec<Location>,
    forward: Vec<Location>,
}

impl NavHistory {
    /// The user went somewhere new from `from`: it can be gone back to, and the
    /// way forward from here is a new one.
    pub fn left(&mut self, from: Location) {
        if self.back.last() != Some(&from) {
            self.back.push(from);
            if self.back.len() > MAX_HISTORY {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    /// The place before `current`, if there is one.
    pub fn back(&mut self, current: Option<Location>) -> Option<Location> {
        let to = self.back.pop()?;
        self.forward.extend(current);
        Some(to)
    }

    /// The place after `current`, if the user came back from one.
    pub fn forward(&mut self, current: Option<Location>) -> Option<Location> {
        let to = self.forward.pop()?;
        self.back.extend(current);
        Some(to)
    }

    pub fn clear(&mut self) {
        self.back.clear();
        self.forward.clear();
    }
}

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
    use super::{Location, MAX_HISTORY, NavHistory, hex, parse_va};
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

    fn at(va: u64) -> Location {
        Location { va, function: None }
    }

    #[test]
    fn back_and_forward_retrace_the_way_taken() {
        let mut h = NavHistory::default();
        h.left(at(1));
        h.left(at(2));
        // At 3 now.
        assert_eq!(h.back(Some(at(3))), Some(at(2)));
        assert_eq!(h.back(Some(at(2))), Some(at(1)));
        assert_eq!(h.back(Some(at(1))), None, "nothing before the first place");
        assert_eq!(h.forward(Some(at(1))), Some(at(2)));
        assert_eq!(h.forward(Some(at(2))), Some(at(3)));
        assert_eq!(h.forward(Some(at(3))), None);
    }

    #[test]
    fn going_somewhere_new_ends_the_way_forward() {
        let mut h = NavHistory::default();
        h.left(at(1));
        assert_eq!(h.back(Some(at(2))), Some(at(1)));
        h.left(at(1));
        assert_eq!(h.forward(Some(at(5))), None);
        assert_eq!(h.back(Some(at(5))), Some(at(1)));
    }

    #[test]
    fn the_history_keeps_only_the_latest_places() {
        let mut h = NavHistory::default();
        for va in 0..(MAX_HISTORY as u64 + 10) {
            h.left(at(va));
        }
        let mut n = 0;
        let mut here = Some(at(u64::MAX));
        while let Some(prev) = h.back(here.clone()) {
            here = Some(prev);
            n += 1;
        }
        assert_eq!(n, MAX_HISTORY);
        assert_eq!(here, Some(at(10)), "the oldest places were dropped");
    }
}
