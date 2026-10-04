// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The session protocol against a stand-in engine (`fake_engine.py`), so every
//! failure path can be produced on purpose: a text-only engine, a target that
//! cannot be opened, a session that crashes, a queue that outruns the engine.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use n0xis_client::{ClientError, Engine, EngineCommand, EngineStatus, MAX_CONSECUTIVE_CRASHES};

fn python() -> Option<&'static str> {
    ["python3", "python"]
        .into_iter()
        .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

/// An engine whose behaviour is picked by the target's file name.
fn fake(mode: &str) -> Option<Engine> {
    let Some(py) = python() else {
        eprintln!("SKIPPED: no Python interpreter for the stand-in engine");
        return None;
    };
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fake_engine.py");
    let engine = EngineCommand::with_prefix(py, [script]);
    Some(Engine::start(engine, PathBuf::from(mode), None))
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn wait_for(engine: &Engine, pred: impl Fn(&EngineStatus) -> bool) -> EngineStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let st = engine.status();
        if pred(&st) || Instant::now() > deadline {
            return st;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn json_requests_carry_any_argument_exactly() {
    let Some(engine) = fake("json-target") else { return };
    let value = "say \"hi\"\nand a \\ backslash";
    let env = engine.call(s(&["echo", value])).wait().expect("answer");
    assert_eq!(env.data["argv"][1], value, "{}", env.data);
    assert!(env.data["raw"].as_str().unwrap().starts_with('['), "an engine announcing json-argv is sent JSON");
    assert!(matches!(engine.status(), EngineStatus::Ready { json_argv: true, .. }));
}

#[test]
fn a_text_only_engine_gets_text_and_a_value_it_cannot_carry_is_refused() {
    let Some(engine) = fake("text-only-target") else { return };
    let env = engine.call(s(&["echo", "a b"])).wait().expect("answer");
    assert_eq!(env.data["raw"], r#"echo "a b""#);

    let err = engine.call(s(&["echo", "say \"hi\""])).wait().unwrap_err();
    assert!(matches!(err, ClientError::Unsendable(_)), "{err:?}");
    // The refused request never reached the engine, so the session is intact.
    assert!(engine.call(s(&["echo", "next"])).wait().is_ok());
}

#[test]
fn a_target_that_cannot_be_opened_is_reported_and_not_retried() {
    let Some(engine) = fake("refuse-target") else { return };
    let st = wait_for(&engine, |s| matches!(s, EngineStatus::GaveUp { .. }));
    let EngineStatus::GaveUp { reason } = st else { panic!("{st:?}") };
    assert!(reason.contains("not-a-binary"), "the engine's own reason reaches the status: {reason}");
    let err = engine.call(s(&["echo", "x"])).wait().unwrap_err();
    assert!(matches!(err, ClientError::GaveUp(_)), "{err:?}");
}

#[test]
fn a_crash_is_reported_with_its_panic_and_the_next_request_restarts_the_session() {
    let Some(engine) = fake("crashy-target") else { return };
    let err = engine.call(s(&["crash"])).wait().unwrap_err();
    let ClientError::SessionClosed(why) = &err else { panic!("{err:?}") };
    assert!(why.contains("panicked at fake: boom"), "the crash says why: {why}");
    assert!(matches!(engine.status(), EngineStatus::Crashed { crashes: 1, .. }), "{:?}", engine.status());

    let env = engine.call(s(&["echo", "after"])).wait().expect("a new session answers");
    assert_eq!(env.data["argv"][1], "after");
    assert!(matches!(engine.status(), EngineStatus::Ready { .. }));
}

#[test]
fn repeated_crashes_stop_the_restarts_until_asked() {
    let Some(engine) = fake("crashy-target") else { return };
    for _ in 0..MAX_CONSECUTIVE_CRASHES {
        assert!(matches!(engine.call(s(&["crash"])).wait(), Err(ClientError::SessionClosed(_))));
    }
    assert!(matches!(engine.status(), EngineStatus::GaveUp { .. }), "{:?}", engine.status());
    assert!(matches!(engine.call(s(&["echo", "x"])).wait(), Err(ClientError::GaveUp(_))));

    engine.restart();
    assert!(engine.call(s(&["echo", "back"])).wait().is_ok(), "restart() lets the session start again");
}

#[test]
fn only_the_latest_request_of_a_key_runs_when_the_engine_is_busy() {
    let Some(engine) = fake("busy-target") else { return };
    let busy = engine.call(s(&["sleep", "0.3"]));
    let first = engine.send_latest("view", &Echo("first"));
    let second = engine.send_latest("view", &Echo("second"));
    let third = engine.send_latest("view", &Echo("third"));
    assert!(busy.wait().is_ok());
    assert_eq!(first.wait().unwrap_err(), ClientError::Superseded);
    assert_eq!(second.wait().unwrap_err(), ClientError::Superseded);
    assert_eq!(third.wait().expect("the latest one runs"), "third");
}

#[test]
fn requests_still_queued_at_shutdown_are_answered_not_dropped() {
    let Some(engine) = fake("busy-target") else { return };
    // The session opens in the background; until it is up, every message waits
    // in the channel, and a shutdown arriving then answers all of them alike.
    // Start the slow request on a ready session so it is truly in flight.
    assert!(matches!(wait_for(&engine, |s| matches!(s, EngineStatus::Ready { .. })), EngineStatus::Ready { .. }));
    let busy = engine.call(s(&["sleep", "0.3"]));
    std::thread::sleep(Duration::from_millis(100)); // an idle worker takes it at once
    let queued = engine.call(s(&["echo", "late"]));
    drop(engine);
    assert!(busy.wait().is_ok());
    assert_eq!(queued.wait().unwrap_err(), ClientError::Shutdown);
}

/// A typed request against the stand-in: `echo <word>` answers with the word.
struct Echo(&'static str);

impl n0xis_client::Request for Echo {
    type Output = String;

    fn args(&self) -> Vec<String> {
        s(&["echo", self.0])
    }

    fn parse(envelope: n0xis_client::Envelope) -> Result<String, ClientError> {
        let (data, _) = envelope.into_parts::<serde_json::Value>("fake.echo.v1")?;
        Ok(data["argv"][1].as_str().unwrap_or_default().to_string())
    }
}
