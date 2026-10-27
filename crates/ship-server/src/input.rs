//! Input: keys, pastes and pointers, routed to panes. Pointers need the
//! sending attachment, so a mouse selection's copy reaches it.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ship_core::{
    AppError, Result, err,
    id::{Attachment, IdOf},
    protocol::*,
};
use ship_macros::traced;

use crate::{
    AppState,
    attach::AttachmentHeader,
    body::Body,
    pane::{LivePanes, PaneCommand, PaneMouse},
    state::CheckAttachment,
};

#[utoipa::path(
    post,
    path = "/api/v0/attach/input",
    operation_id = "input",
    params(("x-ship-attachment-id" = Option<String>, Header,
        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a.... \
            Required for mouse frames.")),
    request_body(content = InputBody,
        description = "Frames in the order they should reach their panes. Removed panes ignore \
            frames; exited panes still accept mouse input."),
    responses(
        (status = 204, description = "Every frame was routed"),
        (status = 400, description = "Malformed attachment header or invalid body; nothing was \
            routed", body = AppError),
        (status = 404, description = "Attachment not found; nothing was routed", body = AppError),
        (status = 422, description = "A mouse frame without an attachment; nothing was routed",
            body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn input(
    State(app): State<AppState>,
    attachment: Option<AttachmentHeader>,
    Body(InputBody { frames }): Body<InputBody>,
) -> Result<Response> {
    let attachment = attachment.map(|AttachmentHeader(id)| id);
    match attachment {
        Some(id) => {
            app.state.ask(CheckAttachment(id)).await?;
        }
        None if frames
            .iter()
            .any(|frame| matches!(frame, InputFrame::Mouse(_))) =>
        {
            let mut response =
                err!(Validation, "mouse input requires an attachment").into_response();
            *response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
            return Ok(response);
        }
        None => {}
    }
    let live = app.live.borrow();
    for frame in frames {
        route(&live, frame, attachment);
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[traced(fields(attachment = ?from))]
/// Mouse frames come only with an attachment; `input` checked.
fn route(live: &LivePanes, frame: InputFrame, from: Option<IdOf<Attachment>>) {
    let (pane, command) = match frame {
        InputFrame::Key(KeyInput { pane, key }) => (pane, PaneCommand::Key(key)),
        InputFrame::Paste(PasteInput { pane, text }) => (pane, PaneCommand::Paste(text)),
        InputFrame::Mouse(input) => {
            let Some(from) = from else { return };
            (input.pane, PaneCommand::Mouse(PaneMouse { input, from }))
        }
    };
    match live.get(&pane) {
        Some(handle) => {
            handle.commands.send(command).ok();
        }
        None => tracing::debug!(%pane, "input for a removed pane dropped"),
    }
}
