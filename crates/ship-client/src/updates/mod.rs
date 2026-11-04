//! Update settings, versions and method selection, shared by the CLI and TUI.

mod check;
mod probe;

pub use check::{Checked, Checker, FRESH_FOR, Found, Freshness, Release};
pub use probe::{Probed, Sync, mismatch, probe};

use std::path::PathBuf;

use semver::Version;
use tokio::process::Command;

/// `[client.updates]` with defaults applied. Replaced whole on reload.
#[derive(Clone)]
pub struct UpdateSettings {
    pub check: bool,
    /// Overrides the build's update method with a shell line.
    pub command: Option<String>,
    /// Overrides the build's latest-version source with a shell line.
    pub latest_command: Option<String>,
}

/// Cache location and self-replace support, supplied by the binary.
pub struct UpdateContext {
    pub cache: PathBuf,
    pub self_replace: bool,
}

pub const UPDATE_COMMAND: Option<&str> = option_env!("SHIP_UPDATE_COMMAND");
pub const LATEST_COMMAND: Option<&str> = option_env!("SHIP_LATEST_COMMAND");
/// GitHub `owner/name`; release builds leave this unset.
pub const RELEASE_REPO: &str = match option_env!("SHIP_RELEASE_REPO") {
    Some(repo) => repo,
    None => "shipmux/ship",
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Command(String),
    SelfReplace,
    Unavailable,
}

impl UpdateSettings {
    /// Config command, build-time command, self-replace, then unavailable.
    pub fn method(&self, context: &UpdateContext) -> Method {
        match self.command.as_deref().or(UPDATE_COMMAND) {
            Some(command) => Method::Command(command.to_owned()),
            None if context.self_replace => Method::SelfReplace,
            None => Method::Unavailable,
        }
    }

    pub fn latest_command(&self) -> Option<&str> {
        self.latest_command.as_deref().or(LATEST_COMMAND)
    }
}

#[cfg(unix)]
fn shell(line: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", line]).kill_on_drop(true);
    command
}

#[cfg(not(unix))]
fn shell(line: &str) -> Command {
    let mut command = Command::new("cmd");
    command.args(["/C", line]).kill_on_drop(true);
    command
}

/// The major.minor.patch in a release tag, packager version or ship --version.
pub fn core_version(text: &str) -> Option<Version> {
    text.split_whitespace().find_map(|word| {
        let word = word.strip_prefix('v').unwrap_or(word);
        Version::parse(word.split(['-', '+']).next()?).ok()
    })
}

/// This binary's version.
pub fn installed() -> Version {
    core_version(env!("CARGO_PKG_VERSION")).expect("Cargo supplies a valid package version")
}
