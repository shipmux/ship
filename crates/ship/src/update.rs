//! The update CLI. Checking changes only the shared release cache.

use std::process::ExitCode;

use ship_client::updates::{Checker, Freshness, UpdateSettings, installed};
use ship_core::prelude::*;

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
