use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ship_core::{AppError, Result, err, id::*, model::*, protocol::*};

use tokio::sync::oneshot;

use crate::{
    AppState,
    body::{Body, Path, Query},
    pane::PaneCommand,
    state::*,
};

type App = State<AppState>;

#[utoipa::path(
    get,
    path = "/api/v0/server/log",
    operation_id = "get_server_log",
    responses((status = 200, body = LogFilter), (status = 500, body = AppError)),
)]
pub(crate) async fn get_server_log(State(app): App) -> Result<Json<LogFilter>> {
    Ok(Json((app.log)(None)?))
}

#[utoipa::path(
    put,
    path = "/api/v0/server/log",
    operation_id = "set_server_log",
    request_body = LogChange,
    responses(
        (status = 200, body = LogFilter),
        (status = 400, description = "Request validation failed", body = AppError),
        (status = 422, description = "Invalid filter", body = AppError),
        (status = 500, body = AppError),
    ),
)]
pub(crate) async fn set_server_log(State(app): App, Body(change): Body<LogChange>) -> Response {
    match (app.log)(Some(change)) {
        Ok(filter) => Json(filter).into_response(),
        Err(error) if *error.code() == ship_core::ErrorCode::Validation => {
            let mut response = error.into_response();
            *response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
            response
        }
        Err(error) => error.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v0/tabs",
    operation_id = "list_tabs",
    responses(
        (status = 200, description = "Top-level tabs keyed by ID, in order, with their descendants", body = IndexMap<String, Tab>),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn list_tabs(State(app): App) -> Result<Response> {
    let value = app.state.ask(ListTabs).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/tabs",
    operation_id = "create_tab",
    request_body = CreateTab,
    responses(
        (status = 201, body = Tab),
        (status = 404, description = "Parent or sibling not found", body = AppError),
        (status = 400, description = "Request validation failed", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn create_tab(State(app): App, Body(body): Body<CreateTab>) -> Result<Response> {
    let tab = app.state.ask(body).await?;
    Ok((StatusCode::CREATED, Json(tab)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/tabs/{id}",
    operation_id = "get_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    responses(
        (status = 200, body = Tab),
        (status = 400, description = "Malformed tab ID", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Result<Response> {
    let value = app.state.ask(Get::<Tab>(id)).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v0/tabs/{id}",
    operation_id = "set_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    request_body = SetTab,
    responses(
        (status = 200, body = Tab),
        (status = 400, description = "Malformed tab ID or request validation failed", body = AppError),
        (status = 404, description = "Tab or zoomed pane not found", body = AppError),
        (status = 422, description = "The zoomed pane is in another tab", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn set_tab(
    State(app): App,
    Path(id): Path<IdOf<Tab>>,
    Body(set): Body<SetTab>,
) -> Result<Response> {
    let value = app.state.ask(Set { id, set }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/tabs/{id}",
    operation_id = "remove_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    responses(
        (status = 204, description = "Tab and its descendants removed"),
        (status = 400, description = "Malformed tab ID", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn remove_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Result<Response> {
    app.state.ask(Remove::<Tab>(id)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/tabs/{id}/move",
    operation_id = "move_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    request_body = MoveTabBody,
    responses(
        (status = 200, description = "The moved tab with its subtree", body = Tab),
        (status = 400, description = "Malformed tab ID or request validation failed", body = AppError),
        (status = 404, description = "Tab, parent or sibling not found", body = AppError),
        (status = 422, description = "Move into itself or a descendant", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn move_tab(
    State(app): App,
    Path(id): Path<IdOf<Tab>>,
    Body(body): Body<MoveTabBody>,
) -> Result<Response> {
    let value = app.state.ask(Move { id, to: body.at }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes",
    operation_id = "create_pane",
    request_body = CreatePane,
    responses(
        (status = 201, body = Pane),
        (status = 400, description = "Request validation failed, or invalid program or starting directory", body = AppError),
        (status = 404, description = "Tab or anchor pane not found", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn create_pane(State(app): App, Body(body): Body<CreatePane>) -> Result<Response> {
    let pane = app.state.ask(body).await?;
    Ok((StatusCode::CREATED, Json(pane)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/panes/{id}",
    operation_id = "get_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    responses(
        (status = 200, body = Pane),
        (status = 400, description = "Malformed pane ID", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_pane(State(app): App, Path(id): Path<IdOf<Pane>>) -> Result<Response> {
    let value = app.state.ask(Get::<Pane>(id)).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v0/panes/{id}",
    operation_id = "set_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = SetPane,
    responses(
        (status = 200, body = Pane),
        (status = 400, description = "Malformed pane ID or request validation failed", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn set_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Body(set): Body<SetPane>,
) -> Result<Response> {
    let value = app.state.ask(Set { id, set }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/panes/{id}",
    operation_id = "remove_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    responses(
        (status = 204, description = "Pane removed"),
        (status = 400, description = "Malformed pane ID", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn remove_pane(State(app): App, Path(id): Path<IdOf<Pane>>) -> Result<Response> {
    app.state.ask(Remove::<Pane>(id)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/swap",
    operation_id = "swap_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = SwapPane,
    responses(
        (status = 200, description = "The panes' tab", body = Tab),
        (status = 400, description = "Malformed pane ID or request validation failed", body = AppError),
        (status = 404, description = "Either pane not found", body = AppError),
        (status = 422, description = "Panes in different tabs or the same pane twice", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn swap_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Body(swap): Body<SwapPane>,
) -> Result<Response> {
    let value = app.state.ask(Swap { id, swap }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/resize",
    operation_id = "resize_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = ResizePane,
    responses(
        (status = 200, description = "The pane's tab, unchanged when the edge can't move", body = Tab),
        (status = 400, description = "Malformed pane ID or request validation failed", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn resize_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Body(resize): Body<ResizePane>,
) -> Result<Response> {
    let value = app.state.ask(Resize { id, resize }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/scroll",
    operation_id = "scroll_pane",
    params(("id" = String, Path, description = "Pane ID")),
    request_body = ScrollPane,
    responses(
        (status = 204, description = "Scroll queued, including for exited panes"),
        (status = 400, description = "Malformed pane ID or request validation failed", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn scroll_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Body(body): Body<ScrollPane>,
) -> Result<StatusCode> {
    let live = app.live.borrow();
    let pane = live
        .get(&id)
        .ok_or_else(|| err!(NotFound, "pane {} not found", id))?;
    pane.commands
        .send(PaneCommand::Scroll(body))
        .map_err(|_| err!(Internal, "pane {} task ended", id))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v0/panes/{id}/text",
    operation_id = "read_pane",
    params(
        ("id" = String, Path, description = "Pane ID"),
        ("lines" = Option<u32>, Query, description = "Last N rows including history"),
        ("unwrap" = Option<bool>, Query, description = "Join soft-wrapped lines"),
        ("ansi" = Option<bool>, Query, description = "Keep colors and styles"),
    ),
    responses(
        (status = 200, body = String, content_type = "text/plain"),
        (status = 400, description = "Malformed pane ID or query validation failed", body = AppError),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn read_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Query(read): Query<ReadPane>,
) -> Result<String> {
    let (reply, text) = oneshot::channel();
    {
        let live = app.live.borrow();
        let pane = live
            .get(&id)
            .ok_or_else(|| err!(NotFound, "pane {} not found", id))?;
        pane.commands
            .send(PaneCommand::Read(read, reply))
            .map_err(|_| err!(Internal, "pane {} task ended", id))?;
    }
    text.await
        .map_err(|_| err!(Internal, "pane {} read failed", id))
}

#[utoipa::path(
    post,
    path = "/api/v0/server/stop",
    operation_id = "stop_server",
    responses(
        (status = 202, description = "Shutdown started, as on SIGTERM. The server keeps \
            answering until its panes are torn down, then exits."),
    ),
)]
pub(crate) async fn stop_server(State(app): App) -> StatusCode {
    app.stop.cancel();
    StatusCode::ACCEPTED
}
