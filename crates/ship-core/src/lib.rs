//! Shared types, contextual diagnostics and feature-gated mechanisms.

use tokio::task::JoinHandle;

pub mod command;
pub mod config;
pub mod endpoint;
pub mod error;
pub mod geometry;
pub mod id;
pub mod layout;
pub mod model;
#[cfg(feature = "kameo")]
pub mod relay;
#[cfg(feature = "run")]
pub mod run;
pub mod screen;
pub mod tree;

/// Common operation errors and construction helpers.
pub mod prelude {
    pub use crate::{AppError, ErrorCode, Result, ResultExt, err};
}

pub use error::{AppError, ErrorCode, ExternalError, InternalError, Result, ResultExt};

/// Spawns short work inside the caller's span. Long-lived tasks should have
/// their own root spans instead.
pub fn spawn_traced<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    use tracing::Instrument;
    tokio::spawn(future.instrument(tracing::Span::current()))
}

pub mod protocol;
pub use protocol::{HEALTH_PATH, HealthResponse, LogChange, LogFilter, PROTOCOL_VERSION, SERVICE};
