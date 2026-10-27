use std::{
    fmt,
    future::ready,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::{Stream, StreamExt, TryStreamExt};
use indexmap::IndexMap;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use ship_core::{
    HEALTH_PATH, HealthResponse, LogChange, LogFilter,
    id::{Attachment, Id, IdOf, Identified, Prefixed},
    model::{Pane, Tab},
    prelude::*,
    protocol::{
        AttachRequest, CreatePane, CreateTab, InputBody, InputFrame, MoveTabBody, REVISION_HEADER,
        ReadPane, ResizePane, ScrollPane, SelectView, SseEvent, SwapPane, ViewInput, ViewingRecord,
    },
    tree::Tabs,
};
use sse_stream::SseByteStream;
use tokio::time::timeout;
use url::Url;

use crate::{NO_BODY, http_error};

pub struct Client {
    pub http: reqwest::Client,
    pub url: Url,
    /// Highest `x-ship-revision` a response carried; 0 before any.
    written: AtomicU64,
    /// The last number taken, while the full-screen client is attached.
    sequence: Mutex<Option<Order>>,
}

/// A request's place in its attachment's sequence, sent as `x-ship-order`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Order {
    pub attachment: IdOf<Attachment>,
    pub n: u64,
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.attachment, self.n)
    }
}

/// Collection path for the generic entity methods.
pub trait Resource: Identified<Id = Id<Self>> + Prefixed + DeserializeOwned {
    const COLLECTION: &'static str;
}

impl Resource for Tab {
    const COLLECTION: &'static str = "/api/v0/tabs";
}

impl Resource for Pane {
    const COLLECTION: &'static str = "/api/v0/panes";
}

impl Client {
    pub fn new(http: reqwest::Client, url: Url) -> Self {
        Self {
            http,
            url,
            written: AtomicU64::new(0),
            sequence: Mutex::new(None),
        }
    }

    /// The highest revision a response reported. A replica at or past it
    /// includes every change this client's requests made.
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Acquire)
    }

    /// Raise `written` to `revision`; a late response never lowers it.
    pub(crate) fn wrote(&self, revision: u64) {
        self.written.fetch_max(revision, Ordering::AcqRel);
    }

    /// End a connection: `written` goes back to 0, for a server that may
    /// have restarted and counts afresh, and numbering stops.
    pub fn forget(&self) {
        self.written.store(0, Ordering::Release);
        self.order(None);
    }

    /// Number this client's requests for `attachment` from 1, or stop
    /// numbering with `None`.
    pub fn order(&self, attachment: Option<IdOf<Attachment>>) {
        *self.sequence.lock().expect("sequence lock") =
            attachment.map(|attachment| Order { attachment, n: 0 });
    }

    /// Take the next number, while numbering.
    pub(crate) fn next_order(&self) -> Option<Order> {
        let mut sequence = self.sequence.lock().expect("sequence lock");
        let order = sequence.as_mut()?;
        order.n += 1;
        Some(*order)
    }

    /// The last number taken, while numbering.
    pub(crate) fn last_order(&self) -> Option<Order> {
        *self.sequence.lock().expect("sequence lock")
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        self.request(Method::GET, HEALTH_PATH, NO_BODY, None, StatusCode::OK)
            .await
    }

    /// GET when showing the filter, PUT when changing it.
    pub async fn server_log(&self, change: Option<&LogChange>) -> Result<LogFilter> {
        let method = if change.is_some() {
            Method::PUT
        } else {
            Method::GET
        };
        self.request(method, "/api/v0/server/log", change, None, StatusCode::OK)
            .await
    }

    /// Starts the server's shutdown; it exits once its panes are torn down.
    pub async fn stop_server(&self) -> Result<()> {
        self.request(
            Method::POST,
            "/api/v0/server/stop",
            NO_BODY,
            None,
            StatusCode::ACCEPTED,
        )
        .await
    }

    /// Top-level tabs with their descendants, in order. Each viewed tab
    /// carries server-computed geometry; unviewed tabs omit it.
    pub async fn tabs(&self) -> Result<Tabs> {
        self.request(Method::GET, Tab::COLLECTION, NO_BODY, None, StatusCode::OK)
            .await
    }

    pub async fn create_tab(&self, body: &CreateTab) -> Result<Tab> {
        self.request(
            Method::POST,
            Tab::COLLECTION,
            Some(body),
            None,
            StatusCode::CREATED,
        )
        .await
    }

    pub async fn create_pane(&self, body: &CreatePane) -> Result<Pane> {
        self.request(
            Method::POST,
            Pane::COLLECTION,
            Some(body),
            None,
            StatusCode::CREATED,
        )
        .await
    }

    pub async fn get<T: Resource>(&self, id: IdOf<T>) -> Result<T> {
        let path = format!("{}/{id}", T::COLLECTION);
        self.request(Method::GET, &path, NO_BODY, None, StatusCode::OK)
            .await
    }

    /// Change the tab or pane's attributes and return it.
    pub async fn set<T: Resource>(&self, id: IdOf<T>, body: &impl serde::Serialize) -> Result<T> {
        let path = format!("{}/{id}", T::COLLECTION);
        self.request(Method::PATCH, &path, Some(body), None, StatusCode::OK)
            .await
    }

    pub async fn remove<T: Resource>(&self, id: IdOf<T>) -> Result<()> {
        let path = format!("{}/{id}", T::COLLECTION);
        self.request(Method::DELETE, &path, NO_BODY, None, StatusCode::NO_CONTENT)
            .await
    }

    pub async fn move_tab(&self, id: IdOf<Tab>, to: &MoveTabBody) -> Result<Tab> {
        let path = format!("{}/{id}/move", Tab::COLLECTION);
        self.request(Method::POST, &path, Some(to), None, StatusCode::OK)
            .await
    }

    /// Exchange the pane with `body.other`; returns their tab.
    pub async fn swap(&self, pane: IdOf<Pane>, body: &SwapPane) -> Result<Tab> {
        self.pane_edit(pane, "swap", body).await
    }

    /// Move the pane's edge; returns its tab.
    pub async fn resize(&self, pane: IdOf<Pane>, body: &ResizePane) -> Result<Tab> {
        self.pane_edit(pane, "resize", body).await
    }

    /// Scroll the shared viewport, including after program exit.
    pub async fn scroll(&self, pane: IdOf<Pane>, body: &ScrollPane) -> Result<()> {
        let path = format!("{}/{pane}/scroll", Pane::COLLECTION);
        self.request(
            Method::POST,
            &path,
            Some(body),
            None,
            StatusCode::NO_CONTENT,
        )
        .await
    }

    /// Read terminal text without changing the shared viewport.
    pub async fn read(&self, pane: IdOf<Pane>, query: &ReadPane) -> Result<String> {
        let path = format!("{}/{pane}/text", Pane::COLLECTION);
        let (request, safe) = self.build(Method::GET, &path, NO_BODY, None, self.next_order());
        let exchange = async {
            let response = crate::open(request.query(query), StatusCode::OK).await?;
            if let Some(revision) = response
                .headers()
                .get(REVISION_HEADER)
                .and_then(|value| value.to_str().ok()?.parse().ok())
            {
                self.wrote(revision);
            }
            response.text().await.map_err(http_error)
        };
        timeout(crate::TIMEOUT, exchange)
            .await
            .unwrap_or_else(|_| Err(err!(Timeout, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }

    async fn pane_edit(
        &self,
        pane: IdOf<Pane>,
        edit: &str,
        body: &impl serde::Serialize,
    ) -> Result<Tab> {
        let path = format!("{}/{pane}/{edit}", Pane::COLLECTION);
        self.request(Method::POST, &path, Some(body), None, StatusCode::OK)
            .await
    }

    /// Open the attach stream. The two-second timeout covers only the
    /// response head. The returned stream yields decoded events and ends on EOF.
    ///
    /// `sse-stream` scans each received chunk once; `eventsource-stream`
    /// rescanned a partial line on every chunk, which was quadratic in the size
    /// of a screen event. Blocks without data are skipped.
    pub async fn attach(
        &self,
        request: &AttachRequest,
    ) -> Result<impl Stream<Item = Result<SseEvent>> + use<>> {
        let response = self
            .stream(Method::POST, "/api/v0/attach", Some(request))
            .await?;
        let body = response
            .bytes_stream()
            .inspect_ok(|chunk| tracing::debug!(bytes = chunk.len(), "attach chunk"));
        Ok(SseByteStream::new(body)
            .try_filter_map(|block| ready(Ok(block.data)))
            .map(|data| match data {
                Ok(data) => {
                    let _decode = tracing::debug_span!("decode", bytes = data.len()).entered();
                    serde_json::from_str(&data).map_err(
                        |error| err!(Serialization, "cannot decode attach event", @external: error),
                    )
                }
                Err(sse_stream::Error::Body(error)) => match error.downcast::<reqwest::Error>() {
                    Ok(error) => Err(http_error(*error)),
                    Err(error) => {
                        Err(err!(Serialization, "malformed attach stream", @external: error))
                    }
                },
                Err(error) => Err(err!(Serialization, "malformed attach stream", @external: error)),
            }))
    }

    /// Send `frames` to their panes in one request; nothing is routed if the
    /// server refuses any of them. Mouse frames need `attachment`.
    /// `order` is a number taken earlier; without one, the request takes
    /// the next number like any other.
    pub(crate) async fn input(
        &self,
        attachment: Option<IdOf<Attachment>>,
        order: Option<Order>,
        frames: &[InputFrame],
    ) -> Result<()> {
        let order = order.or_else(|| self.next_order());
        self.numbered(
            Method::POST,
            "/api/v0/attach/input",
            Some(&InputBody {
                frames: frames.to_vec(),
            }),
            attachment,
            order,
            StatusCode::NO_CONTENT,
        )
        .await
    }

    /// Every attachment's view, in attach order.
    pub async fn views(&self) -> Result<IndexMap<IdOf<Attachment>, ViewingRecord>> {
        self.request(Method::GET, "/api/v0/views", NO_BODY, None, StatusCode::OK)
            .await
    }

    pub async fn view(&self, attachment: IdOf<Attachment>) -> Result<ViewingRecord> {
        let path = format!("/api/v0/views/{attachment}");
        self.request(Method::GET, &path, NO_BODY, None, StatusCode::OK)
            .await
    }

    pub async fn select(
        &self,
        attachment: IdOf<Attachment>,
        body: &SelectView,
    ) -> Result<ViewingRecord> {
        let path = format!("/api/v0/views/{attachment}/selection");
        self.request(Method::PUT, &path, Some(body), None, StatusCode::OK)
            .await
    }

    /// Update the attachment's area; returns the record as stored.
    pub async fn set_view(
        &self,
        attachment: IdOf<Attachment>,
        view: &ViewInput,
    ) -> Result<ViewingRecord> {
        self.request(
            Method::PUT,
            "/api/v0/attach/view",
            Some(view),
            Some(attachment),
            StatusCode::OK,
        )
        .await
    }
}
