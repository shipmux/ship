//! Fire-and-forget commands with empty input and output sent directly to a
//! log file. Nothing stops a run; it outlives its owner.

use std::{
    fs::File,
    process::{ExitStatus, Stdio},
};

use tokio::{process::Command, sync::mpsc};

use crate::prelude::*;

/// How a run ended.
pub struct Ended {
    pub label: String,
    pub status: ExitStatus,
}

/// Every exit is logged; clients also show failures in their status line.
#[derive(Clone)]
pub enum OnExit {
    Log,
    Report(mpsc::UnboundedSender<Ended>),
}

pub struct Runner {
    log_file: Box<dyn Fn() -> Result<File> + Send + Sync>,
    on_exit: OnExit,
}

impl Runner {
    pub fn new(
        log_file: impl Fn() -> Result<File> + Send + Sync + 'static,
        on_exit: OnExit,
    ) -> Self {
        Self {
            log_file: Box::new(log_file),
            on_exit,
        }
    }

    /// Returns once the process starts. Dropping the waiter never kills it.
    pub fn spawn(&self, label: String, mut command: Command) -> Result<()> {
        let output = (self.log_file)().context(format!("cannot start `{label}`"))?;
        let stderr = output.try_clone().map_err(
            |error| err!(Io, "cannot start `{}`: cannot clone run output", label, @external: error),
        )?;
        command
            .stdin(Stdio::null())
            .stdout(output)
            .stderr(stderr)
            .kill_on_drop(false);
        detach(&mut command);
        let mut child = command
            .spawn()
            .map_err(|error| err!(Io, "cannot start `{}`", label, @external: error))?;
        tracing::info!(%label, pid = child.id(), "run started");
        let on_exit = self.on_exit.clone();
        crate::spawn_traced(async move {
            match child.wait().await {
                Ok(status) => {
                    tracing::info!(%label, %status, "run ended");
                    if !status.success()
                        && let OnExit::Report(sender) = on_exit
                    {
                        sender.send(Ended { label, status }).ok();
                    }
                }
                Err(error) => tracing::warn!(%label, %error, "cannot wait for run"),
            }
        });
        Ok(())
    }
}

/// Terminal signals aimed at the owner must not reach its runs.
#[cfg(unix)]
fn detach(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn detach(_: &mut Command) {}
