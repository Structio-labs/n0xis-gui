// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{ClientError, EngineCommand, EngineError, Envelope};

/// Schema of the banner a session prints before it reads requests.
pub const SERVE_READY_SCHEMA: &str = "n0xis.serve.ready.v1";

/// The request form that carries every argument exactly. An engine announces it
/// in `data.request_formats` of its banner; one that does not is sent text lines.
const JSON_ARGV: &str = "json-argv";

/// How much of the engine's stderr is kept to explain a crash.
const STDERR_TAIL_BYTES: usize = 8 * 1024;

/// How long a closing session gets to exit on its own before it is killed.
const CLOSE_GRACE: Duration = Duration::from_millis(300);

/// One `n0xis serve --file <target>` process: the image is loaded once, then
/// every request is one line in and one JSON envelope out. Blocking; the
/// [`Engine`](crate::Engine) worker owns it on its own thread.
pub struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr_tail: Arc<Mutex<String>>,
    json_argv: bool,
    label: String,
}

impl Session {
    /// Start a session for `file`, running in `project_dir` (the directory whose
    /// `.n0x/` holds this target's names and caches), and read its banner.
    pub fn open(engine: &EngineCommand, file: &Path, project_dir: Option<&Path>) -> Result<Self, ClientError> {
        let mut cmd = engine.command();
        cmd.arg("serve")
            .arg("--quiet")
            .arg("--file")
            .arg(file)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = project_dir {
            cmd.current_dir(dir);
        }
        let mut child = cmd.spawn().map_err(|e| ClientError::Spawn(format!("{}: {e}", engine.program().display())))?;
        let (Some(stdin), Some(stdout), Some(stderr)) = (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            let _ = child.kill();
            return Err(ClientError::Spawn("the engine's standard streams were not available".into()));
        };

        // Drain stderr on its own thread so the engine can never block on a full
        // pipe, and keep its tail to say why a session ended.
        let stderr_tail = Arc::new(Mutex::new(String::new()));
        let tail = Arc::clone(&stderr_tail);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(&line);
                t.push('\n');
                if t.len() > STDERR_TAIL_BYTES {
                    let mut cut = t.len() - STDERR_TAIL_BYTES;
                    while !t.is_char_boundary(cut) {
                        cut += 1;
                    }
                    t.drain(..cut);
                }
            }
        });

        let mut session =
            Self { child, stdin, stdout: BufReader::new(stdout), stderr_tail, json_argv: false, label: String::new() };
        let banner = session.read_envelope(false)?;
        if !banner.ok {
            return Err(ClientError::Engine(banner.error.unwrap_or_else(|| EngineError {
                code: "unknown".into(),
                message: "the session failed to start without saying why".into(),
                hint: None,
            })));
        }
        if banner.meta.schema.as_deref() != Some(SERVE_READY_SCHEMA) {
            return Err(ClientError::Protocol(format!(
                "expected the {SERVE_READY_SCHEMA} banner, got {}",
                banner.meta.schema.as_deref().unwrap_or("a line without a schema")
            )));
        }
        session.json_argv = banner
            .data
            .get("request_formats")
            .and_then(|v| v.as_array())
            .is_some_and(|forms| forms.iter().any(|f| f == JSON_ARGV));
        session.label = banner.data.get("label").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        Ok(session)
    }

    /// What the engine calls the loaded target.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether this engine reads JSON requests, which carry any argument exactly.
    pub fn reads_json_argv(&self) -> bool {
        self.json_argv
    }

    /// Send one request (the subcommand and its arguments; the session supplies
    /// `--file`) and read its answer.
    pub fn call(&mut self, args: &[String]) -> Result<Envelope, ClientError> {
        let line = if self.json_argv {
            serde_json::to_string(args).map_err(|e| ClientError::Protocol(e.to_string()))?
        } else {
            text_request(args)?
        };
        self.send_line(&line, false)
    }

    /// Send a line exactly as typed, for the engine to read as it reads any
    /// request (text, or a JSON argument array when it starts with `[`). The
    /// console uses this so that how a typed command splits into arguments is
    /// the engine's rule, not a second copy of it here.
    pub fn call_line(&mut self, line: &str) -> Result<Envelope, ClientError> {
        // A blank line is how a session is told to end, so it is not a request.
        if line.trim().is_empty() {
            return Err(ClientError::Unsendable("an empty request would end the session".into()));
        }
        if line.contains(['\n', '\r']) {
            return Err(ClientError::Unsendable("a request is one line; this one holds a line break".into()));
        }
        self.send_line(line, true)
    }

    fn send_line(&mut self, line: &str, keep_raw: bool) -> Result<Envelope, ClientError> {
        if writeln!(self.stdin, "{line}").and_then(|()| self.stdin.flush()).is_err() {
            return Err(self.closed());
        }
        self.read_envelope(keep_raw)
    }

    fn read_envelope(&mut self, keep_raw: bool) -> Result<Envelope, ClientError> {
        let mut line = String::new();
        match self.stdout.read_line(&mut line) {
            Ok(0) | Err(_) => Err(self.closed()),
            Ok(_) => {
                let not_an_envelope = |e: serde_json::Error| {
                    let shown: String = line.trim().chars().take(200).collect();
                    ClientError::Protocol(format!("not an envelope ({e}): {shown}"))
                };
                if keep_raw {
                    let raw: serde_json::Value = serde_json::from_str(line.trim()).map_err(not_an_envelope)?;
                    let mut envelope: Envelope = serde_json::from_value(raw.clone()).map_err(not_an_envelope)?;
                    envelope.raw = Some(raw);
                    Ok(envelope)
                } else {
                    serde_json::from_str(line.trim()).map_err(not_an_envelope)
                }
            }
        }
    }

    /// The session is gone: say how it ended and what it printed last.
    fn closed(&mut self) -> ClientError {
        let deadline = std::time::Instant::now() + CLOSE_GRACE;
        let status = loop {
            match self.child.try_wait() {
                Ok(Some(s)) => break Some(s),
                Ok(None) if std::time::Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
                _ => break None,
            }
        };
        let tail = self.stderr_tail.lock().unwrap_or_else(|e| e.into_inner()).trim().to_string();
        let how = status.map_or_else(|| "still running but not answering".to_string(), |s| s.to_string());
        if tail.is_empty() { ClientError::SessionClosed(how) } else { ClientError::SessionClosed(format!("{how}: {tail}")) }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // A blank line ends a session cleanly; give it a moment, then make sure.
        let _ = writeln!(self.stdin);
        let _ = self.stdin.flush();
        let deadline = std::time::Instant::now() + CLOSE_GRACE;
        while std::time::Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The text request form: arguments joined by spaces, an argument that is empty
/// or holds whitespace wrapped in `"`. The form has no escape, so an argument
/// holding `"` or a line break cannot be written in it; any rewrite would send a
/// different value, so such a request is refused.
fn text_request(args: &[String]) -> Result<String, ClientError> {
    let mut parts = Vec::with_capacity(args.len());
    for (i, a) in args.iter().enumerate() {
        if a.contains(['"', '\n', '\r']) {
            return Err(ClientError::Unsendable(format!(
                "argument {i} holds a quote or a line break, which this engine's text requests cannot carry \
                 (an engine that reads JSON requests can)"
            )));
        }
        parts.push(if a.is_empty() || a.chars().any(char::is_whitespace) { format!("\"{a}\"") } else { a.clone() });
    }
    Ok(parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn text_requests_quote_what_needs_quoting() {
        assert_eq!(text_request(&s(&["annotate", "name", "--value", "a b", ""])).unwrap(), r#"annotate name --value "a b" """#);
    }

    #[test]
    fn a_value_the_text_form_cannot_carry_is_refused_not_rewritten() {
        for bad in [r#"say "hi""#, "two\nlines", "cr\r"] {
            let err = text_request(&s(&["annotate", "comment", "--value", bad])).unwrap_err();
            assert!(matches!(err, ClientError::Unsendable(_)), "{bad:?}: {err:?}");
        }
    }
}
