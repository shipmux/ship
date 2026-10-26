//! Request decoding: serde decides, the schema explains a refusal.

use std::{collections::HashMap, fmt::Display, sync::LazyLock};

use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Path as AxumPath, Query as AxumQuery, Request},
    http::{HeaderMap, header::CONTENT_TYPE, request::Parts},
};
use boon::{Compiler, Draft, ErrorKind, SchemaIndex, Schemas, ValidationError};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use serde_path_to_error::Segment;
use ship_core::{AppError, Result, err, protocol::Problem};
use utoipa::ToSchema;

/// A JSON body decoded by serde; the schema explains only a refusal.
pub(crate) struct Body<T>(pub T);

impl<T, S> FromRequest<S> for Body<T>
where
    T: DeserializeOwned + ToSchema,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self> {
        if !is_json(request.headers()) {
            return Err(invalid(BODY, vec![whole("expected application/json")]));
        }
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|error| invalid("unreadable request body", vec![whole(error)]))?;
        decode(&bytes).map(Self)
    }
}

/// A query string decoded by serde, reporting its first problem.
pub(crate) struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let AxumQuery(value) = AxumQuery::from_request_parts(parts, state)
            .await
            .map_err(|error| invalid("invalid query string", vec![whole(error.body_text())]))?;
        Ok(Self(value))
    }
}

/// Path parameters, such as a tab or pane ID, decoded by serde.
pub(crate) struct Path<T>(pub T);

impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let AxumPath(value) = AxumPath::from_request_parts(parts, state)
            .await
            .map_err(|error| invalid("invalid path", vec![whole(error.body_text())]))?;
        Ok(Self(value))
    }
}

const BODY: &str = "invalid request body";

/// Decode one complete JSON document. Also used for input stream lines.
pub(crate) fn decode<T: DeserializeOwned + ToSchema>(bytes: &[u8]) -> Result<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let refusal = match serde_path_to_error::deserialize(&mut deserializer) {
        Ok(value) => {
            return deserializer
                .end()
                .map(|()| value)
                .map_err(|error| invalid(BODY, vec![whole(error)]));
        }
        Err(refusal) => refusal,
    };
    // Not JSON at all: there is nothing for the schema to read.
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| invalid(BODY, vec![whole(error)]))?;
    let mut problems = explain(&T::name(), &value);
    if problems.is_empty() {
        problems.push(Problem {
            pointer: pointer(refusal.path()),
            detail: refusal.into_inner().to_string(),
        });
    }
    Err(invalid(BODY, problems))
}

fn invalid(what: &'static str, problems: Vec<Problem>) -> AppError {
    err!(Validation, what, @data: { errors: problems })
}

/// A problem with the request as a whole rather than one field.
fn whole(detail: impl Display) -> Problem {
    Problem {
        pointer: None,
        detail: detail.to_string(),
    }
}

/// Like axum's `Json`, accept `application/json` and `application/*+json`.
fn is_json(headers: &HeaderMap) -> bool {
    let Some(essence) = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
    else {
        return false;
    };
    let essence = essence.trim().to_ascii_lowercase();
    essence
        .strip_prefix("application/")
        .is_some_and(|subtype| subtype == "json" || subtype.ends_with("+json"))
}

/// serde's path as a JSON Pointer, or `None` when it knows no field.
fn pointer(path: &serde_path_to_error::Path) -> Option<String> {
    let pointer: String = path
        .iter()
        .filter_map(|segment| match segment {
            Segment::Seq { index } => Some(format!("/{index}")),
            Segment::Map { key } | Segment::Enum { variant: key } => Some(child_pointer("", key)),
            Segment::Unknown => None,
        })
        .collect();
    (!pointer.is_empty()).then_some(pointer)
}

fn child_pointer(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// The document's compiled schemas and the index of each named component.
struct ValidatorMap {
    schemas: Schemas,
    indices: HashMap<String, SchemaIndex>,
}

/// Compile once, when the router is built. The URI names an in-memory
/// resource; local component references resolve within it, without fetching
/// a document.
static SCHEMAS: LazyLock<ValidatorMap> = LazyLock::new(|| {
    const ROOT: &str = "https://ship.invalid/openapi";
    let components = crate::openapi().components.expect("the API has components");
    let mut compiler = Compiler::new();
    compiler.set_default_draft(Draft::V2020_12);
    compiler
        .add_resource(ROOT, json!({ "components": components }))
        .expect("the schema document has a valid URI");
    let mut schemas = Schemas::new();
    let indices = components
        .schemas
        .into_keys()
        .map(|name| {
            let index = compiler
                .compile(&format!("{ROOT}#/components/schemas/{name}"), &mut schemas)
                .expect("the server's component schemas compile");
            (name, index)
        })
        .collect();
    ValidatorMap { schemas, indices }
});

/// Compile the schemas now, so one that doesn't compile panics at startup
/// rather than on the first refused request.
pub(crate) fn compile_schemas() {
    LazyLock::force(&SCHEMAS);
}

fn explain(schema: &str, body: &Value) -> Vec<Problem> {
    let mut problems = Vec::new();
    if let Some(&index) = SCHEMAS.indices.get(schema)
        && let Err(error) = SCHEMAS.schemas.validate(body, index)
    {
        flatten(&error, &mut problems);
    }
    problems
}

/// Whether an alternative fails on the shape of the value at `depth` itself
/// (a missing key or wrong type) rather than on something inside it. Causes
/// only ever point at or below their parent, so depth identifies the value.
fn shape_error_at(error: &ValidationError<'_, '_>, depth: usize) -> bool {
    (error.instance_location.tokens.len() == depth
        && matches!(
            error.kind,
            ErrorKind::Required { .. } | ErrorKind::Type { .. }
        ))
        || error
            .causes
            .iter()
            .any(|cause| shape_error_at(cause, depth))
}

/// Unwrap groups and references, keep relevant alternatives and give each
/// missing property its own pointer. This is independent of request types.
fn flatten(error: &ValidationError<'_, '_>, out: &mut Vec<Problem>) {
    let here = error.instance_location.to_string();
    match &error.kind {
        // An enum variant the caller didn't send fails on shape; report the
        // ones that got further, or all of them when none did.
        ErrorKind::OneOf(None) => {
            let depth = error.instance_location.tokens.len();
            let relevant: Vec<_> = error
                .causes
                .iter()
                .filter(|cause| !shape_error_at(cause, depth))
                .collect();
            let causes = if relevant.is_empty() {
                error.causes.iter().collect()
            } else {
                relevant
            };
            for cause in causes {
                flatten(cause, out);
            }
        }
        ErrorKind::Required { want } => out.extend(want.iter().map(|field| Problem {
            pointer: Some(child_pointer(&here, field)),
            detail: format!("missing property {field:?}"),
        })),
        _ if error.causes.is_empty() => out.push(Problem {
            pointer: Some(here),
            detail: error.kind.to_string(),
        }),
        _ => {
            for cause in &error.causes {
                flatten(cause, out);
            }
        }
    }
}
