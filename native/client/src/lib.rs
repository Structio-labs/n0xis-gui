// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Typed client for the n0xis engine's persistent session (`n0xis serve`).
//!
//! The engine runs as a separate process on purpose: a parser that dies on a
//! hostile file takes its own process down, not the GUI. This crate owns that
//! process seam: finding the engine, the session protocol, a worker thread that
//! queues requests, and the typed answers the GUI renders. It has no UI
//! dependency, so every part of it is testable without a window.

mod envelope;
mod find;
mod live;
mod locate;
mod notes;
mod profile;
mod refs;
mod request;
mod session;
mod worker;

pub use envelope::{EngineError, Envelope, Meta};
pub use find::{Find, FindMatch, FindQuery, FindResult};
pub use live::{
    ListProcesses, MemoryMap, Narrow, ProcessInfo, Processes, Region, Regions, SCAN_TYPES, ScanFilter, ScanMatch,
    ScanResult, ScanValue,
};
pub use locate::{ENGINE_ENV, EngineCommand};
pub use notes::{
    Annotate, AnnotationRecord, Annotations, DefineEnum, DefineStruct, EnumDef, EnumMember, ListAnnotations, ListTypes,
    Note, RETURN_KEY, RemoveType, RenameVariable, SetBookmark, SetVariableType, ShowAnnotations, StructDef, StructField,
    TypeLibrary, TypeSubject, Written,
};
pub use profile::{Advisory, EngineHint, Export, FoldedExports, ImageProfile, Profile, ProfileReport, Section};
pub use refs::{Block, BuildCfg, CallSite, Cfg, CfgInstruction, Edge, Xref, Xrefs, XrefsTo};
pub use request::{
    DecompStyle, Decompile, Decompiled, Disassemble, Disassembly, DiscoverFunctions, FunctionEntry, FunctionsPage,
    Instruction, Request, Variable, VariableKind, schema,
};
pub use session::{SERVE_READY_SCHEMA, Session};
pub use worker::{Engine, EngineStatus, MAX_CONSECUTIVE_CRASHES, Pending, TypedPending};

/// Everything that can go wrong between the GUI and an answer. Each case is a
/// different thing for the user to do, so none of them is folded into another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// No engine binary was found. Says where the client looked.
    EngineNotFound(String),
    /// The engine process could not be started.
    Spawn(String),
    /// The session process ended. Carries the tail of its stderr, which holds the
    /// panic message when it crashed.
    SessionClosed(String),
    /// The engine sent something outside the protocol: a line that is not an
    /// envelope, or a payload of a different shape than the one requested.
    Protocol(String),
    /// The engine answered `ok:false`.
    Engine(EngineError),
    /// The payload did not have the shape its schema promises.
    Decode(String),
    /// This engine reads only text requests, and an argument holds a quote or a
    /// line break that the text form cannot carry. Sending it anyway would send a
    /// different value, so the request is refused instead.
    Unsendable(String),
    /// A newer request with the same key replaced this one before it ran.
    Superseded,
    /// The engine is not restarted any more; the reason is the last failure.
    GaveUp(String),
    /// The client was shut down before the request ran.
    Shutdown,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EngineNotFound(s) => write!(f, "engine not found ({s})"),
            Self::Spawn(s) => write!(f, "cannot start the engine: {s}"),
            Self::SessionClosed(s) => write!(f, "the engine session ended: {s}"),
            Self::Protocol(s) => write!(f, "unexpected engine output: {s}"),
            Self::Engine(e) => write!(f, "{}: {}", e.code, e.message),
            Self::Decode(s) => write!(f, "unexpected payload shape: {s}"),
            Self::Unsendable(s) => write!(f, "cannot send this request: {s}"),
            Self::Superseded => write!(f, "replaced by a newer request"),
            Self::GaveUp(s) => write!(f, "the engine is stopped after repeated failures: {s}"),
            Self::Shutdown => write!(f, "the engine client was shut down"),
        }
    }
}

impl std::error::Error for ClientError {}
