//! Each attachment's view, readable and selectable by anyone holding its ID.

use axum::{Json, extract::State};
use ship_core::{
    AppError, Result,
    id::{Attachment, IdOf},
    protocol::{SelectView, ViewingRecord},
};

use crate::{
    AppState,
    body::{Body, Path},
    state::{Select, Viewers, attachment_not_found},
};

#[utoipa::path(
    get,
    path = "/api/v0/views",
    operation_id = "list_views",
    responses(
        (status = 200, description = "Every attachment's view keyed by ID, in attach order", body = IndexMap<String, ViewingRecord>),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn list_views(State(app): State<AppState>) -> Result<Json<Viewers>> {
    Ok(Json(app.replicas.current().viewers.clone()))
}

#[utoipa::path(
    get,
    path = "/api/v0/views/{attachment}",
    operation_id = "get_view",
    params(("attachment" = String, Path, description = "Attachment ID from ship view list")),
    responses(
        (status = 200, body = ViewingRecord),
        (status = 400, description = "Malformed attachment ID", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_view(
    State(app): State<AppState>,
    Path(attachment): Path<IdOf<Attachment>>,
) -> Result<Json<ViewingRecord>> {
    app.replicas
        .current()
        .viewers
        .get(&attachment)
        .cloned()
        .map(Json)
        .ok_or_else(|| attachment_not_found(attachment))
}

#[utoipa::path(
    put,
    path = "/api/v0/views/{attachment}/selection",
    operation_id = "select_view",
    params(("attachment" = String, Path, description = "Attachment ID from ship view list")),
    request_body = SelectView,
    responses(
        (status = 200, description = "The stored view; selecting a zoom-hidden pane unzooms its tab", body = ViewingRecord),
        (status = 400, description = "Malformed attachment ID or request validation failed", body = AppError),
        (status = 404, description = "Attachment or node not found; nothing changed", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn select(
    State(app): State<AppState>,
    Path(attachment): Path<IdOf<Attachment>>,
    Body(body): Body<SelectView>,
) -> Result<Json<ViewingRecord>> {
    let value = app
        .state
        .ask(Select {
            attachment,
            selection: body.selection,
        })
        .await?;
    Ok(Json(value))
}
