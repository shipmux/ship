//! Shared-type HTTP client and the full-screen client; identity validation and
//! startup belong to the application.

mod api;
mod automation;
mod execute;
pub mod keymap;
pub mod sound;
pub mod ui;
pub mod updates;
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

/// `call` body argument for bodiless requests; a bare `None` can't name
/// the body type.
const NO_BODY: Option<&()> = None;

const TIMEOUT: Duration = Duration::from_secs(2);

/// A request ready to send.
struct Prepared {
    request: RequestBuilder,
    /// The URL without credentials, for errors.
    safe: Url,
}

impl Client {
    /// JSON in, JSON out, numbered while attached. Most endpoints.
    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
        expect: StatusCode,
    ) -> Result<T> {
        let mut prepared = self.build(method, path, None, self.next_order());
        if let Some(body) = body {
            prepared.request = prepared.request.json(body);
        }
        self.exchange(prepared, expect, json).await
    }

    /// The request with `x-ship-order` and, when given, the attachment
    /// header. Callers take `order` when first polled, so numbers follow the
    /// order requests start.
    fn build(
        &self,
        method: Method,
        path: &str,
        attachment: Option<IdOf<Attachment>>,
        order: Option<Order>,
    ) -> Prepared {
        let mut url = self.url.clone();
        url.set_path(path);
        url.set_query(None);
        url.set_fragment(None);
        let mut safe = url.clone();
        safe.set_username("").ok();
        safe.set_password(None).ok();

        let mut request = self.http.request(method, url);
        if let Some(attachment) = attachment {
            request = request.header(ATTACHMENT_HEADER, attachment.to_string());
        }
        if let Some(order) = order {
            request = request.header(ORDER_HEADER, order.to_string());
        }
        Prepared { request, safe }
    }

    /// Send within `TIMEOUT`, check the status, record `x-ship-revision`,
    /// then `read` the response. The one copy of the timeout and context.
    ///
    /// Any other status returns the server's `AppError` when the body is
    /// one, so server messages reach the caller; otherwise the status and
    /// body text.
    async fn exchange<T>(
        &self,
        Prepared { request, safe }: Prepared,
        expect: StatusCode,
        read: impl AsyncFnOnce(Response) -> Result<T>,
    ) -> Result<T> {
        let exchange = async {
            let response = request.send().await.map_err(http_error)?;
            let status = response.status();
            if status != expect {
                let bytes = response.bytes().await.map_err(http_error)?;
                return Err(
                    serde_json::from_slice::<AppError>(&bytes).unwrap_or_else(|_| {
                        let text = String::from_utf8_lossy(&bytes);
                        let text = text.trim();
                        err!(
                            ErrorCode::UpstreamHttpStatus(status.as_u16()),
                            "expected HTTP {}, received HTTP {}{}{}",
                            expect.as_u16(),
                            status.as_u16(),
                            if text.is_empty() { "" } else { ": " },
                            text
                        )
                    }),
                );
            }
            if let Some(revision) = response
                .headers()
                .get(REVISION_HEADER)
                .and_then(|value| value.to_str().ok()?.parse().ok())
            {
                self.wrote(revision);
            }
            read(response).await
        };
        timeout(TIMEOUT, exchange)
            .await
            .unwrap_or_else(|_| Err(err!(Timeout, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }
}

/// Decode a JSON body; a 204's empty body decodes as null, so `()` succeeds.
async fn json<T: DeserializeOwned>(response: Response) -> Result<T> {
    let bytes = response.bytes().await.map_err(http_error)?;
    let bytes: &[u8] = if bytes.is_empty() { b"null" } else { &bytes };
    serde_json::from_slice(bytes)
        .map_err(|error| err!(Serialization, "cannot decode response", @external: error))
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
    // Callers read raw bytes and decode JSON themselves, so reqwest's
    // "decode" kind here only means reading the body failed: a cut
    // connection, such as an attach stream ending without `Ended`.
    let code = if error.is_timeout() {
        ErrorCode::Timeout
    } else {
        ErrorCode::Network
    };
    AppError::external(code, error.without_url())
}
