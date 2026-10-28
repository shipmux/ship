use std::{
    env,
    path::{self, PathBuf},
};

use clap::{Args, Parser, Subcommand};
use etcetera::BaseStrategy;
use ship_core::{
    command::Command as ServerAction,
    endpoint::{Endpoint, ServerName},
    id::{Attachment, IdOf},
    model::NodeId,
    prelude::*,
};

#[derive(Parser)]
#[command(
    version,
    about = "Open the Ship client, run a named-socket server or script tabs and panes",
    long_about = "Open the Ship client, run a named-socket server or script tabs and panes.\n\nBare ship opens a full-screen client on the whole server; alt-q detaches.\nBare ship and tab and pane commands reuse the default-local server or start a missing one in the background.\nThat server stays running after the client and launching terminal exit."
)]
pub struct Cli {
    /// Connect only to this socket (unix:///path); also SHIP_SERVER_URL. Never starts a server
    #[arg(
        long,
        global = true,
        value_name = "URL",
        conflicts_with = "server_name"
    )]
    pub server_url: Option<Endpoint>,
    /// Select a named server, starting it if needed; also SHIP_SERVER_NAME
    #[arg(long, global = true, value_name = "NAME")]
    pub server_name: Option<ServerName>,
    /// Config file; defaults to ship/config.toml in the platform config directory
    #[arg(long, global = true, env = "SHIP_CONFIG", value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// A name may launch a server; a URL only connects to its given endpoint.
pub enum Target {
    Named(ServerName),
    Url(Endpoint),
}

impl Target {
    pub fn endpoint(&self) -> Result<Endpoint> {
        match self {
            Self::Named(name) => Endpoint::named(name),
            Self::Url(endpoint) => Ok(endpoint.clone()),
        }
    }
}

impl Cli {
    /// Flags precede variables, and the pane's URL precedes an exported name.
    pub fn target(&self) -> Result<Target> {
        if let Some(endpoint) = &self.server_url {
            return Ok(Target::Url(endpoint.clone()));
        }
        if let Some(name) = &self.server_name {
            return Ok(Target::Named(name.clone()));
        }
        if let Some(value) = env::var_os("SHIP_SERVER_URL") {
            let value = value
                .into_string()
                .map_err(|_| err!(Configuration, "SHIP_SERVER_URL must be valid UTF-8"))?;
            return value
                .parse()
                .map(Target::Url)
                .map_err(|error: AppError| error.context("invalid SHIP_SERVER_URL"));
        }
        if let Some(value) = env::var_os("SHIP_SERVER_NAME") {
            let value = value
                .into_string()
                .map_err(|_| err!(Configuration, "SHIP_SERVER_NAME must be valid UTF-8"))?;
            return value
                .parse()
                .map(Target::Named)
                .map_err(|error: AppError| error.context("invalid SHIP_SERVER_NAME"));
        }
        Ok(Target::Named(ServerName::default()))
    }

    /// `--config`, `SHIP_CONFIG`, then etcetera's config directory. Absolute,
    /// so the background server and relative settings see the same file.
    pub fn config_path(&self) -> Result<PathBuf> {
        let path = match &self.config {
            Some(path) => path.clone(),
            None => etcetera::choose_base_strategy()
                .map_err(|error| err!(Configuration, "cannot find the config directory", @external: error))?
                .config_dir()
                .join("ship/config.toml"),
        };
        path::absolute(&path).map_err(
            |error| err!(Io, "cannot resolve config path {}", path.display(), @external: error),
        )
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a foreground server on a named socket, or stop or check one
    Server(ServerArgs),
    #[command(flatten)]
    Action(ServerAction),
    /// List clients' views, or read or set one client's selection
    #[command(subcommand)]
    View(ViewCommand),
    /// Check the config file or print the defaults
    #[command(subcommand)]
    Config(ConfigCommand),
    /// Print a tab-completion script for your shell
    Completions(CompletionsArgs),
}

#[derive(Subcommand)]
pub enum ViewCommand {
    /// Print every attached client's view as JSON, keyed by attachment ID
    List,
    /// Print a client's selection as JSON: a tab or pane ID, or null
    Selected(ViewTarget),
    /// Select a tab or pane for a client, or nothing with --none
    Select {
        #[command(flatten)]
        view: ViewTarget,
        #[arg(required_unless_present = "none", conflicts_with = "none")]
        node: Option<NodeId>,
        #[arg(long)]
        none: bool,
    },
}

#[derive(Args)]
pub struct ViewTarget {
    /// Attachment ID, from ship view list
    #[arg(long, env = "SHIP_ATTACHMENT_ID", value_name = "ID")]
    pub attachment: IdOf<Attachment>,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Report every error in a config file and exit non-zero if any
    Check {
        /// File to check; defaults to the config file in use
        file: Option<PathBuf>,
    },
    /// Print every default setting, key and menu, with what each accepts
    Defaults,
}

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to complete in
    pub shell: clap_complete::Shell,
}

/// Bare `ship server` runs one; stop it with SIGINT, SIGTERM or `ship server stop`.
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ServerArgs {
    #[command(subcommand)]
    pub command: Option<ServerCommand>,
    /// Which server: picks its socket, saved tabs and logs
    #[arg(long, default_value_t)]
    pub name: ServerName,
    /// Start with one tab holding a shell in your home directory, only when
    /// there are no tabs after restoring
    #[arg(long)]
    pub starter: bool,
    /// Start without restoring this name's saved tabs. The first save
    /// replaces them and keeps the old file beside it as <name>.json.prev
    #[arg(long)]
    pub no_restore: bool,
    #[arg(long, hide = true)]
    pub background_child: bool,
    /// The launcher's first log part
    #[arg(long, hide = true, requires = "background_child")]
    pub log_file: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum ServerCommand {
    /// Stop the server, ending every program in it, and wait until it exits
    Stop,
    /// Print the server's health as JSON; never starts a server
    Status,
    /// Show or change the running server's log filter; never starts a server
    Log(LogArgs),
}

#[derive(Args)]
#[group(multiple = false)]
pub struct LogArgs {
    /// Ship's level: error, warn, info, debug or trace
    pub level: Option<tracing::Level>,
    /// A full filter, as RUST_LOG sets it
    #[arg(long)]
    pub filter: Option<String>,
    /// Return to the filter the server started with
    #[arg(long)]
    pub reset: bool,
}
