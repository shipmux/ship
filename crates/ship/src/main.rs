mod cli;
mod commands;
mod diagnostics;
mod local;
mod runlog;

use std::{io, process::ExitCode, time::Duration};

use clap::{CommandFactory, Parser};
use cli::{Cli, Command, Target};
use reqwest::redirect::Policy;
use ship_client::{Client, ui};
use ship_core::{
    HealthResponse, PROTOCOL_VERSION, SERVICE, config, endpoint::Endpoint, prelude::*,
};
use tokio::runtime::Builder;
use url::Url;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            match serde_json::to_string(&error) {
                Ok(json) => eprintln!("{json}"),
                Err(_) => eprintln!("ship: {error}"),
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    if matches!(
        cli.command,
        Some(Command::Server(cli::ServerArgs {
            background_child: true,
            ..
        }))
    ) {
        detach()?;
    }
    let diagnostics = diagnostics::init(role(&cli))?;
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| err!(Internal, "cannot create runtime", @external: error))?;
    runtime.block_on(dispatch(cli, &diagnostics))
}

fn role(cli: &Cli) -> diagnostics::Role {
    match &cli.command {
        Some(Command::Server(args)) if args.command.is_none() => diagnostics::Role::Server {
            name: args.name.clone(),
            log_file: args.log_file.clone(),
        },
        None => diagnostics::Role::Client,
        _ => diagnostics::Role::Command,
    }
}

/// Leave the launching terminal's session so the background server outlives it.
#[cfg(unix)]
fn detach() -> Result<()> {
    use nix::unistd::setsid;

    setsid()
        .map(drop)
        .map_err(|error| err!(Io, "cannot establish background server session", @external: error))
}

/// Application compatibility boundary, shared by explicit requests and local readiness.
async fn health(client: &Client) -> Result<HealthResponse> {
    let response = client.health().await?;
    if response.service != SERVICE || response.protocol_version != PROTOCOL_VERSION {
        return Err(err!(
            Validation,
            "incompatible health response: expected service {} and protocol {}",
            SERVICE,
            PROTOCOL_VERSION
        ));
    }
    Ok(response)
}

/// Only client commands resolve target variables. Running a server refuses
/// target flags and uses its own `--name`, even inside another server's pane.
async fn dispatch(mut cli: Cli, diagnostics: &diagnostics::Diagnostics) -> Result<ExitCode> {
    match cli.command.take() {
        // Never start a server, so a stopped default server stays stopped.
        Some(Command::Server(cli::ServerArgs {
            command: Some(cli::ServerCommand::Stop),
            ..
        })) => {
            let target = cli.target()?;
            let (client, _) = checked(&target.endpoint()?, matches!(target, Target::Url(_))).await?;
            commands::stop_server(&client).await
        }
        Some(Command::Server(cli::ServerArgs {
            command: Some(cli::ServerCommand::Status),
            ..
        })) => {
            let target = cli.target()?;
            commands::print(&checked(&target.endpoint()?, matches!(target, Target::Url(_))).await?.1)
        }
        Some(Command::Server(cli::ServerArgs {
            command: Some(cli::ServerCommand::Log(args)),
            ..
        })) => {
            let target = cli.target()?;
            let (client, _) = checked(&target.endpoint()?, matches!(target, Target::Url(_))).await?;
            commands::server_log(&client, args).await
        }
        Some(Command::Server(_)) if cli.server_url.is_some() || cli.server_name.is_some() => {
            Err(err!(
                Configuration,
                "--server-url and --server-name select servers for clients; use ship server --name NAME to run one"
            ))
        }
        Some(Command::Server(args)) => {
            let tabs = runlog::state_dir()?.join("tabs");
            runlog::create_private_dir(&tabs)?;
            ship_server::serve(
                Endpoint::named(&args.name)?,
                args.starter,
                cli.config_path()?,
                tabs.join(format!("{}.json", args.name)),
                !args.no_restore,
                diagnostics::shutdown()?,
                diagnostics.server_log(),
            )
            .await
            // stderr is gone for a background server, so the run log is the only record.
            .inspect(|()| tracing::info!("server stopped"))
            .inspect_err(|error| tracing::error!(%error, "server failed"))
        }
        Some(Command::Config(cli::ConfigCommand::Check { file })) => {
            return commands::check_config(&match file {
                Some(file) => file,
                None => cli.config_path()?,
            });
        }
        Some(Command::Config(cli::ConfigCommand::Defaults)) => {
            print!("{}", config::DEFAULTS);
            Ok(())
        }
        Some(Command::Completions(cli::CompletionsArgs { shell })) => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                env!("CARGO_BIN_NAME"),
                &mut io::stdout(),
            );
            Ok(())
        }
        Some(Command::Action(command)) => {
            let client = connect(&cli).await?;
            commands::action(&client, command).await
        }
        Some(Command::View(command)) => {
            let client = connect(&cli).await?;
            commands::view(&client, command).await
        }
        None => {
            // A full-screen client must not misread another protocol. Only a
            // server this call launched opens on its first tab, restored or
            // the starter.
            let target = cli.target()?;
            let endpoint = target.endpoint()?;
            let (client, open_first) = match target {
                Target::Url(_) => (checked(&endpoint, true).await?.0, false),
                Target::Named(name) => {
                    let client = client(&endpoint, true)?;
                    let launched = local::ensure_running(&client, &name, &cli.config_path()?)
                        .await
                        .context(format!("server at {endpoint}"))?;
                    (client, launched)
                }
            };
            ui::run(&client, open_first, &cli.config_path()?)
                .await
                // The client log has no stderr layer, so this doesn't repeat main's message.
                .inspect_err(|error| tracing::error!(%error, "client failed"))
        }
    }
    .map(|()| ExitCode::SUCCESS)
}

/// Validate a URL without starting it; otherwise reuse or start the named server.
async fn connect(cli: &Cli) -> Result<Client> {
    let target = cli.target()?;
    let endpoint = target.endpoint()?;
    match target {
        Target::Url(_) => Ok(checked(&endpoint, true).await?.0),
        Target::Named(name) => {
            let client = client(&endpoint, true)?;
            local::ensure_running(&client, &name, &cli.config_path()?)
                .await
                .context(format!("server at {endpoint}"))?;
            Ok(client)
        }
    }
}

/// A client whose health check passed. An explicit target that fails over
/// HTTP/2 for any reason but an absent or silent server, such as a proxy that
/// speaks only HTTP/1.1, gets one more check over HTTP/1.1. Ship's own server
/// always speaks HTTP/2, so the local default target never needs it.
async fn checked(endpoint: &Endpoint, explicit_target: bool) -> Result<(Client, HealthResponse)> {
    let http2 = client(endpoint, true)?;
    match health(&http2)
        .await
        .context(format!("server at {endpoint}"))
    {
        Err(error) if explicit_target && *error.code() == ErrorCode::Network => {
            tracing::debug!(%error, "HTTP/2 health check failed; retrying over HTTP/1.1");
            let http1 = client(endpoint, false)?;
            let response = health(&http1)
                .await
                .context(format!("server at {endpoint}"))?;
            tracing::warn!(
                server = %endpoint,
                "server answers only over HTTP/1.1"
            );
            Ok((http1, response))
        }
        result => Ok((http2, result?)),
    }
}

fn client(endpoint: &Endpoint, http2: bool) -> Result<Client> {
    let url = Url::parse("http://localhost/").expect("valid internal HTTP base");
    let builder = reqwest::Client::builder()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(1));
    #[cfg(unix)]
    let builder = builder.unix_socket(endpoint.path());
    #[cfg(windows)]
    let builder = builder.windows_named_pipe(endpoint.path());
    let builder = if http2 {
        builder.http2_prior_knowledge()
    } else {
        builder.http1_only()
    };
    let http = builder
        .build()
        .map_err(|error| err!(Configuration, "cannot initialize HTTP client for {}", endpoint, @external: error.without_url()))?;
    Ok(Client::new(http, url))
}
