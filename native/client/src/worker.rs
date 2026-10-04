// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

use std::collections::{HashSet, VecDeque};
use std::future::Future;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, JoinHandle};

use futures_channel::oneshot;

use crate::{ClientError, EngineCommand, Envelope, Request, Session};

/// Consecutive crashes after which the session is not started again until
/// [`Engine::restart`]. A file that kills the parser on every open would
/// otherwise be re-opened forever.
pub const MAX_CONSECUTIVE_CRASHES: u32 = 3;

/// What the engine is doing, for the status bar. Every state a user can be in
/// is its own case, so the UI never has to guess from an error string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    Starting,
    Ready { label: String, json_argv: bool },
    /// The session ended; the next request starts a new one.
    Crashed { crashes: u32, reason: String },
    /// Not restarted any more until [`Engine::restart`].
    GaveUp { reason: String },
    Stopped,
}

enum Msg {
    Call(Job),
    Restart,
    Shutdown,
}

/// What a job sends: an argument list the client encodes, or a line typed by
/// the user that the engine reads as it is.
enum Payload {
    Args(Vec<String>),
    Line(String),
}

struct Job {
    payload: Payload,
    key: Option<String>,
    reply: oneshot::Sender<Result<Envelope, ClientError>>,
}

/// One engine session for one target, owned by a worker thread. Requests queue
/// up and run one at a time; the UI thread never blocks on the engine.
pub struct Engine {
    tx: Sender<Msg>,
    status: Arc<Mutex<EngineStatus>>,
    worker: Option<JoinHandle<()>>,
}

impl Engine {
    /// Start the worker and open the session for `file` right away, so a missing
    /// engine or an unreadable file shows in [`Engine::status`] before the first
    /// request.
    pub fn start(engine: EngineCommand, file: PathBuf, project_dir: Option<PathBuf>) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(EngineStatus::Starting));
        let worker_status = Arc::clone(&status);
        let worker = thread::Builder::new()
            .name("n0xis-engine".into())
            .spawn(move || run(&rx, &worker_status, &engine, &file, project_dir.as_deref()))
            .expect("spawn the engine worker thread");
        Self { tx, status, worker: Some(worker) }
    }

    pub fn status(&self) -> EngineStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Send raw arguments; the answer is the untyped envelope.
    pub fn call(&self, args: Vec<String>) -> Pending {
        self.submit(None, Payload::Args(args))
    }

    /// Send a line as typed; the engine splits it into arguments by its own
    /// rule. The answer is the untyped envelope.
    pub fn call_line(&self, line: String) -> Pending {
        self.submit(None, Payload::Line(line))
    }

    /// Send a typed request.
    pub fn send<R: Request>(&self, request: &R) -> TypedPending<R> {
        TypedPending { inner: self.submit(None, Payload::Args(request.args())), _request: PhantomData }
    }

    /// Send a typed request that replaces any not-yet-started request with the
    /// same key. Each view uses its own key, so when the selection moves faster
    /// than the engine answers, only the latest selection is computed.
    pub fn send_latest<R: Request>(&self, key: &str, request: &R) -> TypedPending<R> {
        TypedPending { inner: self.submit(Some(key.to_string()), Payload::Args(request.args())), _request: PhantomData }
    }

    /// Allow the session to start again after [`EngineStatus::GaveUp`].
    pub fn restart(&self) {
        let _ = self.tx.send(Msg::Restart);
    }

    fn submit(&self, key: Option<String>, payload: Payload) -> Pending {
        let (reply, rx) = oneshot::channel();
        if let Err(mpsc::SendError(Msg::Call(job))) = self.tx.send(Msg::Call(Job { payload, key, reply })) {
            let _ = job.reply.send(Err(ClientError::Shutdown));
        }
        Pending(rx)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Shutdown);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// An answer that is not there yet. Await it from async code, or [`Pending::wait`].
pub struct Pending(oneshot::Receiver<Result<Envelope, ClientError>>);

impl Future for Pending {
    type Output = Result<Envelope, ClientError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.get_mut().0).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(oneshot::Canceled)) => Poll::Ready(Err(ClientError::Shutdown)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Pending {
    /// Block the calling thread until the answer arrives. Never call it on the UI thread.
    pub fn wait(self) -> Result<Envelope, ClientError> {
        block_on(self)
    }
}

/// A typed answer that is not there yet.
pub struct TypedPending<R: Request> {
    inner: Pending,
    _request: PhantomData<fn() -> R>,
}

impl<R: Request> Future for TypedPending<R> {
    type Output = Result<R::Output, ClientError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.get_mut().inner).poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.and_then(R::parse)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R: Request> TypedPending<R> {
    /// Block the calling thread until the answer arrives. Never call it on the UI thread.
    pub fn wait(self) -> Result<R::Output, ClientError> {
        block_on(self)
    }
}

fn block_on<F: Future>(fut: F) -> F::Output {
    struct ThreadWaker(thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        thread::park();
    }
}

/// Keep only the newest queued job of each key; answer the older ones.
fn drop_superseded(queue: &mut VecDeque<Job>) {
    let mut seen = HashSet::new();
    let mut kept = VecDeque::with_capacity(queue.len());
    while let Some(job) = queue.pop_back() {
        match &job.key {
            Some(k) if !seen.insert(k.clone()) => {
                let _ = job.reply.send(Err(ClientError::Superseded));
            }
            _ => kept.push_front(job),
        }
    }
    *queue = kept;
}

fn run(
    rx: &Receiver<Msg>,
    status: &Mutex<EngineStatus>,
    engine: &EngineCommand,
    file: &std::path::Path,
    project_dir: Option<&std::path::Path>,
) {
    let set = |s: EngineStatus| *status.lock().unwrap_or_else(|e| e.into_inner()) = s;
    let open = || Session::open(engine, file, project_dir);

    let mut session: Option<Session> = None;
    let mut crashes = 0u32;
    let mut gave_up: Option<String> = None;
    match open() {
        Ok(s) => {
            set(EngineStatus::Ready { label: s.label().to_string(), json_argv: s.reads_json_argv() });
            session = Some(s);
        }
        // Not a crash: a missing engine or an unreadable file fails the same way
        // every time, so it is not retried until asked.
        Err(e) => {
            gave_up = Some(e.to_string());
            set(EngineStatus::GaveUp { reason: e.to_string() });
        }
    }

    let mut queue: VecDeque<Job> = VecDeque::new();
    loop {
        let mut incoming = Vec::new();
        if queue.is_empty() {
            match rx.recv() {
                Ok(m) => incoming.push(m),
                Err(_) => break,
            }
        }
        incoming.extend(rx.try_iter());
        for m in incoming {
            match m {
                Msg::Call(job) => queue.push_back(job),
                Msg::Restart => {
                    crashes = 0;
                    if gave_up.take().is_some() {
                        set(EngineStatus::Starting);
                    }
                }
                Msg::Shutdown => {
                    for job in queue.drain(..) {
                        let _ = job.reply.send(Err(ClientError::Shutdown));
                    }
                    set(EngineStatus::Stopped);
                    return;
                }
            }
        }
        drop_superseded(&mut queue);
        let Some(job) = queue.pop_front() else { continue };
        if job.reply.is_canceled() {
            continue; // nobody is waiting for this answer any more
        }
        if let Some(reason) = &gave_up {
            let _ = job.reply.send(Err(ClientError::GaveUp(reason.clone())));
            continue;
        }
        if session.is_none() {
            match open() {
                Ok(s) => {
                    set(EngineStatus::Ready { label: s.label().to_string(), json_argv: s.reads_json_argv() });
                    session = Some(s);
                }
                Err(e) => {
                    gave_up = Some(e.to_string());
                    set(EngineStatus::GaveUp { reason: e.to_string() });
                    let _ = job.reply.send(Err(e));
                    continue;
                }
            }
        }
        let Some(s) = session.as_mut() else { continue };
        let result = match &job.payload {
            Payload::Args(args) => s.call(args),
            Payload::Line(line) => s.call_line(line),
        };
        if let Err(ClientError::SessionClosed(reason)) = &result {
            // The request that ended the session is answered with that, never
            // re-sent: it may be exactly the input that kills the parser.
            session = None;
            crashes += 1;
            if crashes >= MAX_CONSECUTIVE_CRASHES {
                gave_up = Some(reason.clone());
                set(EngineStatus::GaveUp { reason: reason.clone() });
            } else {
                set(EngineStatus::Crashed { crashes, reason: reason.clone() });
            }
        } else {
            crashes = 0;
        }
        let _ = job.reply.send(result);
    }
    set(EngineStatus::Stopped);
}
