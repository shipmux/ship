//! `ship update` and `ship update --check`.

#[cfg(feature = "self-update")]
use std::env;
use std::{ffi::OsString, process::ExitCode};

use ship_client::updates::{
    Checker, Freshness, Method, Sync, UpdateContext, UpdateSettings, installed, probe, shell,
};
use ship_core::{HealthResponse, prelude::*};

pub async fn check(checker: &Checker, settings: &UpdateSettings) -> Result<ExitCode> {
    let found = checker
        .check(Freshness::Force, settings.latest_command())
        .await?;
    let installed = installed();
    match found.latest {
        Some(latest) if latest > installed => println!("{installed} → {latest}"),
        Some(_) => println!("up to date"),
        None => println!("no releases yet"),
    }
    Ok(ExitCode::SUCCESS)
}

pub async fn update(
    settings: &UpdateSettings,
    context: &UpdateContext,
    server: Option<HealthResponse>,
) -> Result<ExitCode> {
    let program = match settings.method(context) {
        Method::Command(line) => {
            let status = shell(&line)
                .status()
                .await
                .map_err(|error| err!(Io, "cannot run update command", @external: error))?;
            if !status.success() {
                return Err(err!(Validation, "update command failed: {}", status));
            }
            OsString::from("ship")
        }
        #[cfg(feature = "self-update")]
        Method::SelfReplace => {
            // On Linux, current_exe() after replacement can include " (deleted)".
            let path = env::current_exe()
                .map_err(|error| err!(Io, "cannot find the installed ship", @external: error))?;
            if self_replace().await?.is_up_to_date() {
                println!("up to date");
                return Ok(ExitCode::SUCCESS);
            }
            path.into_os_string()
        }
        // `UpdateContext::self_replace` is false without the feature.
        #[cfg(not(feature = "self-update"))]
        Method::SelfReplace => return Err(no_method()),
        Method::Unavailable => return Err(no_method()),
    };
    let probed = probe(&program).await;
    match server {
        Some(server) => println!("{}", Sync::of(probed, &server).describe(&server)),
        None => match probed {
            Ok(probed) => println!(
                "installed ship {} (protocol {})",
                probed.version, probed.protocol
            ),
            Err(_) => println!("update completed; could not probe the installed ship"),
        },
    }
    Ok(ExitCode::SUCCESS)
}

fn no_method() -> AppError {
    err!(
        Unavailable,
        "this install can't update itself; update it the way you installed it (your package manager, or rebuild and reinstall from source)"
    )
}

#[cfg(feature = "self-update")]
async fn self_replace() -> Result<self_update::VersionStatus> {
    use self_update::backends::github::Update;
    use ship_client::updates::RELEASE_REPO;

    let (owner, repo) = RELEASE_REPO
        .split_once('/')
        .expect("RELEASE_REPO is checked at compile time");
    let target = self_update::get_target();
    Update::configure()
        .repo_owner(owner)
        .repo_name(repo)
        .bin_name("ship")
        .asset_identifier(format!("ship-{target}.tar.gz"))
        // upload-rust-binary-action names its sums file without .tar.gz.
        .checksum_from_asset(format!("ship-{target}.sha256"))
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        // Ship prints its own result; self_update's semver verdict would contradict the protocol one.
        .show_output(false)
        .show_download_progress(true)
        .build_async()
        .map_err(|error| err!(Configuration, "cannot initialize self-update", @external: error))?
        .update_async()
        .await
        .map_err(|error| err!(Io, "self-update failed", @external: error))
}
