//! Ordered requests: a request with `x-ship-order: {attachment}/{n}` runs
//! only after that attachment's request `n - 1` has finished. Handlers and
//! unnumbered callers don't know this exists.

use std::{
    collections::HashMap,
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use ship_core::{
    Result, err,
    id::{Attachment, IdOf},
    protocol::ORDER_HEADER,
    spawn_traced,
};
use tokio::{sync::watch, time::timeout};

use crate::{AppState, state::ListViews};

/// How long a numbered request waits for a missing predecessor while
/// nothing is running. Under the client's 2 s timeout, so the client sees
/// the refusal rather than its own timeout.
const ORDER_WAIT: Duration = Duration::from_secs(1);

/// Each active attachment's lane, created by its first numbered request.
pub(crate) type Lanes = Arc<Mutex<HashMap<IdOf<Attachment>, Arc<Lane>>>>;

#[derive(Default)]
struct Progress {
    /// The last number that finished.
    done: u64,
    /// The last number that claimed its turn.
    started: u64,
    running: bool,
}

#[derive(Default)]
pub(crate) struct Lane(watch::Sender<Progress>);

impl Lane {
    /// Wait until request `n - 1` has finished, then claim `n`.
    async fn turn(&self, n: u64) -> Result<()> {
        let mut progress = self.0.subscribe();
        loop {
            let running = {
                let progress = progress.borrow_and_update();
                if progress.started >= n {
                    return Err(err!(OutOfOrder, "request {} already ran", n));
                }
                progress.running
            };
            // Atomic, so a second copy of `n` finds it started.
            if self.0.send_if_modified(|progress| {
                let ready = progress.done == n - 1 && progress.started < n;
                if ready {
                    progress.started = n;
                    progress.running = true;
                }
                ready
            }) {
                return Ok(());
            }
            // A slow predecessor isn't a missing one: only idle time counts.
            if running {
                progress.changed().await.ok();
            } else if timeout(ORDER_WAIT, progress.changed()).await.is_err() {
                let missing = n - 1;
                tracing::warn!(
                    n,
                    missing,
                    "numbered request refused: its predecessor never arrived"
                );
                return Err(err!(OutOfOrder, "request {} never arrived", missing));
            }
        }
    }

    fn finish(&self, n: u64) {
        self.0.send_modify(|progress| {
            progress.done = n;
            progress.running = false;
        });
    }
}

/// Finishes the turn even if the handler panics, so the lane isn't stuck.
struct Turn(Arc<Lane>, u64);

impl Drop for Turn {
    fn drop(&mut self) {
        self.0.finish(self.1);
    }
}

pub(crate) async fn order(
    State(app): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response> {
    let Some((attachment, n)) = parse(request.headers())? else {
        return Ok(next.run(request).await);
    };
    let lane = lane(&app, attachment).await?;
    lane.turn(n).await?;
    let turn = Turn(lane, n);
    // Spawned so a dropped connection can't cancel a request that started.
    spawn_traced(async move {
        let response = next.run(request).await;
        drop(turn);
        response
    })
    .await
    .map_err(|error| err!(Internal, "numbered request failed", @external: error))
}

/// `None` without the header; 400 when it's malformed or `n` is 0.
fn parse(headers: &HeaderMap) -> Result<Option<(IdOf<Attachment>, u64)>> {
    let Some(value) = headers.get(ORDER_HEADER) else {
        return Ok(None);
    };
    let malformed = || {
        err!(
            Validation,
            "malformed {} header, expected {{attachment}}/{{n}}",
            ORDER_HEADER
        )
    };
    let (attachment, n) = value
        .to_str()
        .ok()
        .and_then(|value| value.rsplit_once('/'))
        .ok_or_else(malformed)?;
    let n: NonZeroU64 = n.parse().map_err(|_| malformed())?;
    Ok(Some((attachment.parse()?, n.get())))
}

/// The attachment's lane, created once the state actor confirms it is
/// active. `AttachmentGuard` removes it when the attachment ends.
async fn lane(app: &AppState, attachment: IdOf<Attachment>) -> Result<Arc<Lane>> {
    if let Some(lane) = app.lanes.lock().expect("lanes lock").get(&attachment) {
        return Ok(Arc::clone(lane));
    }
    let viewers = app.state.ask(ListViews).await?;
    if !viewers.contains_key(&attachment) {
        return Err(err!(NotFound, "attachment {} not found", attachment));
    }
    let mut lanes = app.lanes.lock().expect("lanes lock");
    Ok(Arc::clone(lanes.entry(attachment).or_default()))
}
