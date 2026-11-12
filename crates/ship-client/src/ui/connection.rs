//! The attach stream and everything that keeps it open: the first attach,
//! reconnecting with backoff, cutting a broken connection and the
//! throttled area PUT.

use std::{cell::RefCell, mem, time::Duration};

use futures_util::FutureExt;
use ship_core::{
    HealthResponse, PROTOCOL_VERSION,
    id::{Attachment, IdOf},
    prelude::*,
    protocol::{AttachRequest, EndReason, Ended, SseEvent, ViewInput},
    screen::Size,
};
use tokio::time;

use super::{Events, Request, finish, next};
use crate::Client;

const FIRST_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

pub(super) struct Connection<'a> {
    client: &'a Client,
    /// The span every poll of the client enters, swapped on each attach.
    span: &'a RefCell<tracing::Span>,
    /// What the next attach sends; its area is refreshed before each retry.
    request: AttachRequest,
    events: Option<Events>,
    reconnect: Option<Request<'a, Reattached>>,
    backoff: Duration,
    /// Attached, but no event since. The backoff resets on the first one, so a
    /// server whose events don't decode can't pin reconnects at the shortest delay.
    unproven: bool,
    putting: Option<Request<'a>>,
}

/// A new area or focus for this client's record, never a selection that
/// could overwrite a script's edit.
pub(super) struct ViewPut {
    pub attachment: IdOf<Attachment>,
    pub view: ViewInput,
}

/// What the connection has to say to the loop.
pub(super) enum Link {
    /// Any event but `ended`. On `attached`, numbering has started and the
    /// span names the new attachment.
    Event(SseEvent),
    /// `ended` with `server_shutdown`.
    Shutdown,
    /// The stream broke, fell behind, or ended without notice. Logged here;
    /// the loop cuts.
    Lost,
    /// /health before a reattach reported another protocol.
    Incompatible(HealthResponse),
    /// The area PUT failed.
    PutFailed(AppError),
}

/// What a reattach found.
enum Reattached {
    Events(Events),
    Incompatible(HealthResponse),
}

/// What a cut connection reattaches with.
pub(super) enum Reattach {
    /// The last record's selection at the current area, from
    /// `Observer::remembered`.
    Remembered(AttachRequest),
    /// The request the connection last sent, as before the first
    /// `attached` event.
    Previous,
}

impl<'a> Connection<'a> {
    /// The first attach, without delay, so an unreachable server fails
    /// before the client takes the terminal.
    pub async fn open(
        client: &'a Client,
        request: AttachRequest,
        span: &'a RefCell<tracing::Span>,
    ) -> Result<Self> {
        let events = Box::pin(client.attach(&request).await?);
        Ok(Self {
            client,
            span,
            request,
            events: Some(events),
            reconnect: None,
            backoff: FIRST_BACKOFF,
            unproven: false,
            putting: None,
        })
    }

    /// Waits on the stream, a reconnect attempt and the area PUT together.
    /// A failed reconnect retries after the doubled backoff with `area`.
    ///
    /// Safe to drop at any await, as the loop's `select!` does whenever
    /// another branch wins: the stream, the reconnect and the PUT live in
    /// `self`, so a dropped call loses nothing and the next one resumes them.
    pub async fn next(&mut self, area: Size) -> Link {
        loop {
            tokio::select! {
                event = next(&mut self.events) => return self.read(event),
                result = finish(&mut self.reconnect) => match result {
                    Ok(Reattached::Events(stream)) => {
                        self.reconnect = None;
                        self.events = Some(stream);
                    }
                    Ok(Reattached::Incompatible(server)) => {
                        self.reconnect = None;
                        return Link::Incompatible(server);
                    }
                    Err(error) => {
                        tracing::debug!(%error, backoff = ?self.backoff, "reconnect failed");
                        self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
                        self.request.area = Some(area);
                        self.retry();
                    }
                },
                result = finish(&mut self.putting) => {
                    self.putting = None;
                    if let Err(error) = result {
                        return Link::PutFailed(error);
                    }
                }
            }
        }
    }

    /// An event that has already arrived, without waiting for one.
    pub fn ready(&mut self) -> Option<Link> {
        let event = next(&mut self.events).now_or_never()?;
        Some(self.read(event))
    }

    fn read(&mut self, event: Option<Result<SseEvent>>) -> Link {
        match event {
            Some(Ok(SseEvent::Ended(Ended {
                reason: EndReason::ServerShutdown,
            }))) => Link::Shutdown,
            Some(Ok(event)) if !matches!(event, SseEvent::Ended(_)) => {
                if mem::take(&mut self.unproven) {
                    self.backoff = FIRST_BACKOFF;
                }
                if let SseEvent::Attached(attached) = &event {
                    let span = tracing::info_span!(
                        parent: None,
                        "client",
                        attachment = %attached.attachment
                    );
                    span.in_scope(|| tracing::info!("attached"));
                    self.span.replace(span);
                    self.client.order(Some(attached.attachment));
                    self.unproven = true;
                }
                Link::Event(event)
            }
            lost => {
                match lost {
                    Some(Err(error)) => tracing::warn!(%error, "disconnected; reconnecting"),
                    Some(Ok(SseEvent::Ended(Ended {
                        reason: EndReason::Lost,
                    }))) => tracing::warn!("event stream fell behind; reconnecting"),
                    _ => tracing::warn!("event stream ended without notice; reconnecting"),
                }
                Link::Lost
            }
        }
    }

    /// Drop the stream, the PUT and numbering, and start reattaching.
    pub fn cut(&mut self, reattach: Reattach) {
        (self.events, self.putting) = (None, None);
        self.client.forget();
        if mem::take(&mut self.unproven) {
            self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
        }
        if let Reattach::Remembered(request) = reattach {
            self.request = request;
        }
        self.retry();
    }

    fn retry(&mut self) {
        self.reconnect = Some(Box::pin(reattach_after(
            self.client,
            self.request.clone(),
            self.backoff,
        )));
    }

    /// No PUT in flight.
    pub fn idle(&self) -> bool {
        self.putting.is_none()
    }

    pub fn put(&mut self, ViewPut { attachment, view }: ViewPut) {
        let client = self.client;
        self.putting = Some(Box::pin(async move {
            client.set_view(attachment, &view).await.map(|_| ())
        }));
    }
}

/// After `delay`, check /health, then attach if the protocol matches.
async fn reattach_after(
    client: &Client,
    request: AttachRequest,
    delay: Duration,
) -> Result<Reattached> {
    time::sleep(delay).await;
    let server = client.health().await?;
    if server.protocol_version != PROTOCOL_VERSION {
        return Ok(Reattached::Incompatible(server));
    }
    Ok(Reattached::Events(Box::pin(client.attach(&request).await?)))
}

/// The server refused a number: this connection's sequence is broken.
pub(super) fn refused(error: &AppError) -> bool {
    matches!(error.code(), ErrorCode::OutOfOrder)
}
