//! Shared-type HTTP client and the full-screen client; identity validation and
//! startup belong to the application.

mod api;
mod automation;
mod execute;
pub mod keymap;
pub mod sound;
pub mod ui;
mod watch;

use std::{error::Error, io, iter, time::Duration};

use api::Order;
pub use api::{Client, Resource};
pub use execute::{KeyScope, Outcome, Scope, current_dir};
use reqwest::{Error as HttpError, Method, RequestBuilder, Response, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use ship_core::{
    id::{Attachment, IdOf},
    prelude::*,
    protocol::{ATTACHMENT_HEADER, ORDER_HEADER, REVISION_HEADER},
};
use tokio::time::timeout;
use url::Url;

/// `request` body argument for bodiless requests.
const NO_BODY: Option<&()> = None;

const TIMEOUT: Duration = Duration::from_secs(2);

impl Client {
    /// Send one request and decode the expected success body. The two-second
    /// timeout covers the whole exchange.
    async fn request<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        attachment: Option<IdOf<Attachment>>,
        expected_status: StatusCode,
    ) -> Result<T> {
        self.numbered(
            method,
            path,
            body,
            attachment,
            self.next_order(),
            expected_status,
        )
        .await
    }

    /// `request`, carrying `order` when there is one.
    async fn numbered<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        attachment: Option<IdOf<Attachment>>,
        order: Option<Order>,
        expected_status: StatusCode,
    ) -> Result<T> {
        let (request, safe) = self.build(method, path, body, attachment, order);
        let exchange = async {
            let response = open(request, expected_status).await?;
            if let Some(revision) = response
                .headers()
                .get(REVISION_HEADER)
                .and_then(|value| value.to_str().ok()?.parse().ok())
            {
                self.wrote(revision);
            }
            let bytes = response.bytes().await.map_err(http_error)?;
            // A 204 has no body; decode it as JSON null so `()` succeeds.
            let bytes: &[u8] = if bytes.is_empty() { b"null" } else { &bytes };
            serde_json::from_slice(bytes)
                .map_err(|error| err!(Serialization, "cannot decode response", @external: error))
        };
        timeout(TIMEOUT, exchange)
            .await
            .unwrap_or_else(|_| Err(err!(Timeout, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }

    /// Start a streaming request. The two-second timeout covers only the
    /// response head (and an error body); the returned body is unbounded.
    async fn stream<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let (request, safe) = self.build(method, path, body, None, self.next_order());
        timeout(TIMEOUT, open(request, StatusCode::OK))
            .await
            .unwrap_or_else(|_| Err(err!(Timeout, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }

    /// The request, and its URL without credentials for error messages.
    /// Callers take `order` when first polled, so numbers follow the order
    /// requests start.
    fn build<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        attachment: Option<IdOf<Attachment>>,
        order: Option<Order>,
    ) -> (RequestBuilder, Url) {
        let mut url = self.url.clone();
        url.set_path(path);
        url.set_query(None);
        url.set_fragment(None);
        let mut safe = url.clone();
        safe.set_username("").ok();
        safe.set_password(None).ok();

        let mut request = self.http.request(method, url);
        if let Some(body) = body {
            request = request.json(body);
        }
        if let Some(attachment) = attachment {
            request = request.header(ATTACHMENT_HEADER, attachment.to_string());
        }
        if let Some(order) = order {
            request = request.header(ORDER_HEADER, order.to_string());
        }
        (request, safe)
    }
}

/// Send and check the status. Any other status returns the server's
/// `AppError` when the body is one, so server messages reach the caller;
/// otherwise the status and body text.
async fn open(request: RequestBuilder, expected_status: StatusCode) -> Result<Response> {
    let response = request.send().await.map_err(http_error)?;
    let status = response.status();
    if status == expected_status {
        return Ok(response);
    }
    let bytes = response.bytes().await.map_err(http_error)?;
    Err(
        serde_json::from_slice::<AppError>(&bytes).unwrap_or_else(|_| {
            let text = String::from_utf8_lossy(&bytes);
            let text = text.trim();
            err!(
                ErrorCode::UpstreamHttpStatus(status.as_u16()),
                "expected HTTP {}, received HTTP {}{}{}",
                expected_status.as_u16(),
                status.as_u16(),
                if text.is_empty() { "" } else { ": " },
                text
            )
        }),
    )
}

fn http_error(error: HttpError) -> AppError {
    // Classify retained foreign causes before AppError snapshots them as text.
    // Body/decode failures cannot establish an absent listener.
    let connection_refused = error.is_connect()
        && iter::successors(error.source(), |source| (*source).source()).any(|source| {
            source.downcast_ref::<io::Error>().is_some_and(|error| {
                matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
                )
            })
        });
    // reqwest only says "error sending request"; this is what it means.
    if connection_refused {
        return err!(ConnectionRefused, "no server running");
    }
    let code = if error.is_timeout() {
        ErrorCode::Timeout
    } else if error.is_decode() {
        ErrorCode::Serialization
    } else {
        ErrorCode::Network
    };
    AppError::external(code, error.without_url())
}
