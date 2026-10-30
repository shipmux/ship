//! Client-local commands, using the caller's environment and starting directory.

use std::env;

use mitsein::vec1::Vec1;

use ship_core::{
    model::NodeId,
    prelude::*,
    run::{Ended, Runner},
};
use tokio::process::Command;

use super::observer::Observer;
use crate::Client;

pub(super) fn start(
    runner: &Runner,
    client: &Client,
    observer: &Observer,
    command: Vec1<String>,
) -> Result<()> {
    let label = command.join(" ");
    let (program, args) = command.split_first();
    let mut child = Command::new(program);
    child.args(args);
    let bin = env::current_exe().map_err(
        |error| err!(Io, "cannot start `{}`: cannot resolve the client executable", label, @external: error),
    )?;
    child
        .env("SHIP_BIN", bin)
        .env("SHIP_SERVER_URL", client.endpoint.url());
    child
        .env_remove("SHIP_ATTACHMENT_ID")
        .env_remove("SHIP_PANE_ID");
    if let Some(attachment) = observer.attachment {
        child.env("SHIP_ATTACHMENT_ID", attachment.to_string());
    }
    if let Some(NodeId::Pane(pane)) = observer.record().and_then(|record| record.selection) {
        child.env("SHIP_PANE_ID", pane.to_string());
    }
    runner.spawn(label, child)
}

pub(super) fn describe(ended: &Ended) -> String {
    match ended.status.code() {
        Some(code) => format!("`{}` exited with {code}", ended.label),
        None => format!("`{}` {}", ended.label, ended.status),
    }
}
