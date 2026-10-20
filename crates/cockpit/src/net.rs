//! Background I/O: a tokio runtime, `Loader<T>` slots the UI polls, and a callback channel
//! for write actions. The UI thread never awaits.

use camunda::{ApiError, Client};
use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub type Callback<A> = Box<dyn FnOnce(&mut A) + Send + 'static>;

pub struct Net<A> {
    rt: tokio::runtime::Runtime,
    pub client: Arc<Client>,
    ctx: egui::Context,
    tx: Sender<Callback<A>>,
    rx: Receiver<Callback<A>>,
    in_flight: Arc<Mutex<usize>>,
}

impl<A: 'static> Net<A> {
    pub fn new(client: Client, ctx: egui::Context) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().thread_name("flightdeck-io").build().expect("tokio runtime");
        let (tx, rx) = channel();
        Self { rt, client: Arc::new(client), ctx, tx, rx, in_flight: Arc::new(Mutex::new(0)) }
    }

    pub fn in_flight(&self) -> usize {
        *self.in_flight.lock().unwrap()
    }

    /// Start a fetch into a loader slot.
    pub fn fetch<T, F, Fut>(&self, loader: &mut Loader<T>, f: F)
    where
        T: Send + 'static,
        F: FnOnce(Arc<Client>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, ApiError>> + Send + 'static,
    {
        let slot: Arc<Mutex<Option<Result<T, ApiError>>>> = Arc::new(Mutex::new(None));
        let gen = loader.generation.wrapping_add(1);
        loader.generation = gen;
        loader.pending = Some(Pending { slot: slot.clone() });
        let client = self.client.clone();
        let ctx = self.ctx.clone();
        let inflight = self.in_flight.clone();
        *inflight.lock().unwrap() += 1;
        self.rt.spawn(async move {
            let r = f(client).await;
            *slot.lock().unwrap() = Some(r);
            *inflight.lock().unwrap() -= 1;
            ctx.request_repaint();
        });
    }

    /// Run an action; `done` is invoked on the UI thread with the result.
    pub fn run<T, F, Fut, D>(&self, f: F, done: D)
    where
        T: Send + 'static,
        F: FnOnce(Arc<Client>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, ApiError>> + Send + 'static,
        D: FnOnce(&mut A, Result<T, ApiError>) + Send + 'static,
    {
        let client = self.client.clone();
        let ctx = self.ctx.clone();
        let tx = self.tx.clone();
        let inflight = self.in_flight.clone();
        *inflight.lock().unwrap() += 1;
        self.rt.spawn(async move {
            let r = f(client).await;
            *inflight.lock().unwrap() -= 1;
            let cb: Callback<A> = Box::new(move |app: &mut A| done(app, r));
            let _ = tx.send(cb);
            ctx.request_repaint();
        });
    }

    /// Drain completed action callbacks. Call at the start of each frame.
    pub fn drain(&self) -> Vec<Callback<A>> {
        let mut v = vec![];
        while let Ok(cb) = self.rx.try_recv() {
            v.push(cb);
        }
        v
    }

}

struct Pending<T> {
    slot: Arc<Mutex<Option<Result<T, ApiError>>>>,
}

/// Async data slot: keeps the last good value while reloading or after an error.
pub struct Loader<T> {
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub fetched_at: Option<Instant>,
    pending: Option<Pending<T>>,
    generation: u64,
}

impl<T> Default for Loader<T> {
    fn default() -> Self {
        Self { data: None, error: None, fetched_at: None, pending: None, generation: 0 }
    }
}

impl<T> Loader<T> {
    pub fn is_loading(&self) -> bool {
        self.pending.is_some()
    }
    pub fn age(&self) -> Option<std::time::Duration> {
        self.fetched_at.map(|t| t.elapsed())
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_none() && self.pending.is_none() && self.error.is_none()
    }
    /// Should we (re)fetch? True when never fetched or older than `max_age` and not in flight.
    pub fn stale(&self, max_age: std::time::Duration) -> bool {
        if self.pending.is_some() {
            return false;
        }
        match self.fetched_at {
            None => true,
            Some(t) => t.elapsed() >= max_age,
        }
    }
    /// Poll the pending slot. Returns true when new data or an error arrived this frame.
    pub fn poll(&mut self) -> bool {
        let Some(p) = &self.pending else { return false };
        let taken = p.slot.lock().unwrap().take();
        match taken {
            None => false,
            Some(Ok(v)) => {
                self.data = Some(v);
                self.error = None;
                self.fetched_at = Some(Instant::now());
                self.pending = None;
                true
            }
            Some(Err(e)) => {
                self.error = Some(e);
                self.fetched_at = Some(Instant::now());
                self.pending = None;
                true
            }
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn invalidate(&mut self) {
        self.fetched_at = None;
    }
}
