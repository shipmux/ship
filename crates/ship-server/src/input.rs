//! The input stream: keys, pastes and pointers, routed to panes. Pointers
//! need the sending attachment, so a mouse selection's copy reaches it.

use std::io;

use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{StreamExt, TryStreamExt};
use ship_core::{
    AppError, Result, err,
    id::{Attachment, IdOf},
    protocol::*,
};
use ship_macros::traced;
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};

use crate::{
    AppState,
    attach::AttachmentHeader,
    body,
    pane::{LivePanes, PaneCommand, PaneMouse},
    state::CheckAttachment,
};

#[utoipa::path(
    post,
    path = "/api/v0/attach/input",
    operation_id = "input",
    params(("x-ship-attachment-id" = Option<String>, Header,
        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a.... \
            Required for mouse frames; without it the request ends with its body or the server.")),
    request_body(content = InputFrame, content_type = "application/x-ndjson",
        description = "One `InputFrame` per line, each at most 64 KiB, read as they arrive. \
            Removed panes ignore frames; exited panes still accept mouse input."),
    responses(
        (status = 204, description = "The body ended, or the attachment or server did"),
        (status = 400, description = "Malformed attachment header, and nothing was read; or a \
            line failed validation, and reading stopped there", body = AppError),
        (status = 404, description = "Attachment not found; nothing was read", body = AppError),
        (status = 422, description = "Oversized or unreadable line, or a mouse frame without \
            an attachment; reading stopped there", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn input(
    State(app): State<AppState>,
    attachment: Option<AttachmentHeader>,
    stream: Body,
) -> Result<Response> {
    let attachment = attachment.map(|AttachmentHeader(id)| id);
    let revision = match attachment {
        Some(id) => app.state.ask(CheckAttachment(id)).await?,
        None => 0,
    };
    let mut replicas = app.replicas.latest();
    let stream = StreamReader::new(stream.into_data_stream().map_err(io::Error::other));
    let mut lines = FramedRead::new(stream, LinesCodec::new_with_max_length(INPUT_LINE_MAX));
    loop {
        tokio::select! {
            line = lines.next() => {
                let Some(line) = line else { break };
                // A line that doesn't decode answers 400 like any body; the
                // stream's other stops keep 422.
                let (status, error) = match line.map(|line| frame(&line)) {
                    Ok(Ok(None)) => continue,
                    Ok(Ok(Some(frame))) => match route(&app.live.borrow(), frame, attachment) {
                        Ok(()) => continue,
                        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, error),
                    },
                    Ok(Err(error)) => (StatusCode::BAD_REQUEST, error),
                    Err(error) => (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        err!(Validation, "unreadable input line", @external: error),
                    ),
                };
                // Never the line itself: a paste can hold secrets.
                tracing::warn!(?attachment, %error, "input stream stopped at a bad line");
                let mut response = error.into_response();
                *response.status_mut() = status;
                return Ok(response);
            }
            // Replicas older than the check may still be on their way and
            // lack the attachment, so only newer ones can end the stream.
            changed = replicas.changed() => {
                if changed.is_err() {
                    break;
                }
                let replica = replicas.borrow_and_update();
                if attachment.is_some_and(|id| {
                    replica.revision >= revision && !replica.viewers.contains_key(&id)
                }) {
                    break;
                }
            }
        }
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// A blank line is skipped.
fn frame(line: &str) -> Result<Option<InputFrame>> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    body::decode(line.as_bytes()).map(Some)
}

#[traced(fields(attachment = ?from))]
fn route(live: &LivePanes, frame: InputFrame, from: Option<IdOf<Attachment>>) -> Result<()> {
    let (pane, command) = match frame {
        InputFrame::Key(KeyInput { pane, key }) => (pane, PaneCommand::Key(key)),
        InputFrame::Paste(PasteInput { pane, text }) => (pane, PaneCommand::Paste(text)),
        InputFrame::Mouse(input) => {
            let from =
                from.ok_or_else(|| err!(Validation, "mouse input requires an attachment"))?;
            (input.pane, PaneCommand::Mouse(PaneMouse { input, from }))
        }
    };
    match live.get(&pane) {
        Some(handle) => {
            handle.commands.send(command).ok();
        }
        None => tracing::debug!(%pane, "input for a removed pane dropped"),
    }
    Ok(())
}
