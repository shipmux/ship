use std::{
    io::{self, Write},
    path::Path,
    process::ExitCode,
    time::Duration,
};

use futures_util::StreamExt;
use serde::Serialize;
use ship_client::{Client, Outcome, Scope, keymap::ClientSettings};
use ship_core::{
    HealthResponse,
    command::Command,
    config::ServerSettings,
    prelude::*,
    protocol::{FeedMode, SelectView},
};

use crate::{
    cli::{LogArgs, ViewCommand},
    diagnostics,
};

use tokio::time::{Instant, sleep};

/// Panes get two seconds to exit and the drain five, so ten covers a stop.
const STOP_WAIT: Duration = Duration::from_secs(10);

/// `ship tab …` and `ship pane …`: execute under `Scope::Cli`, then print
/// the outcome as one JSON line, or nothing for close.
pub async fn action(client: &Client, command: Command) -> Result<()> {
    match client.execute(command, &Scope::Cli).await? {
        Outcome::Closed => Ok(()),
        Outcome::Text(text) => io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .map_err(|error| err!(Io, "cannot print pane text", @external: error)),
        outcome => print(&outcome),
    }
}

/// Read or steer an attachment's selection, printing one JSON line.
pub async fn view(client: &Client, command: ViewCommand) -> Result<()> {
    match command {
        ViewCommand::List => print(&client.views().await?),
        ViewCommand::Selected(view) => print(&client.view(view.attachment).await?.selection),
        ViewCommand::Select { view, node, .. } => print(
            &client
                .select(view.attachment, &SelectView { selection: node })
                .await?,
        ),
    }
}

/// `ship watch`: the health answer, then every attach line as the server sent
/// it. The server closes the stream after `ended`, which says why; watch then
/// fails. A closed stdout is a normal end.
pub async fn watch(client: &Client, health: &HealthResponse, feed: FeedMode) -> Result<()> {
    let mut out = io::stdout();
    let mut lines = std::pin::pin!(client.observe_lines(feed).await?);
    if closed(line(&mut out, health))? {
        return Ok(());
    }
    loop {
        match lines.next().await {
            Some(Ok(data)) => {
                if closed(writeln!(out, "{data}"))? {
                    return Ok(());
                }
            }
            Some(Err(error)) => return Err(error.context("lost the connection to the server")),
            None => return Err(err!(Unavailable, "the server ended the stream")),
        }
    }
}

/// One JSON line. Stdout is line-buffered, so it reaches a pipe at once.
/// `println!` would panic on a closed pipe.
fn line(out: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)
}

/// Whether the reader closed stdout, which ends `watch` normally.
fn closed(written: io::Result<()>) -> Result<bool> {
    match written {
        Ok(()) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(true),
        Err(error) => Err(err!(Io, "cannot print state", @external: error)),
    }
}

/// `ship config check`: loads each part of the file the way its process
/// does, short of reading sound files, and prints each part's first error
/// on stderr. Returns failure after findings, without a further error line.
/// No warnings.
pub fn check_config(path: &Path) -> Result<ExitCode> {
    let mut errors: Vec<AppError> = ServerSettings::load(path).err().into_iter().collect();
    errors.extend(ClientSettings::check(path).err());
    // A syntax error fails both parts the same way.
    errors.dedup_by_key(|error| error.to_string());
    for error in &errors {
        eprintln!("{error}");
    }
    match errors.is_empty() {
        true => Ok(ExitCode::SUCCESS),
        false => Ok(ExitCode::FAILURE),
    }
}

/// `ship server stop`: start shutdown, then wait until the server refuses
/// connections. Prints nothing, like removal.
pub async fn stop_server(client: &Client) -> Result<()> {
    client.stop_server().await?;
    let deadline = Instant::now() + STOP_WAIT;
    loop {
        match client.health().await {
            Err(error) if error.code() == ErrorCode::ConnectionRefused => return Ok(()),
            _ if Instant::now() >= deadline => {
                return Err(err!(Network, "server still running after ten seconds"));
            }
            _ => sleep(Duration::from_millis(50)).await,
        }
    }
}

/// Show or change the live filter, printing one JSON line.
pub async fn server_log(client: &Client, args: LogArgs) -> Result<()> {
    use ship_core::LogChange;

    let change = if let Some(level) = args.level {
        Some(LogChange::Set(diagnostics::level_filter(level)))
    } else if let Some(filter) = args.filter {
        Some(LogChange::Set(filter))
    } else if args.reset {
        Some(LogChange::Reset)
    } else {
        None
    };
    print(&client.server_log(change.as_ref()).await?)
}

pub(crate) fn print(value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|error| err!(Serialization, "cannot serialize result", @external: error))?;
    println!("{json}");
    Ok(())
}
