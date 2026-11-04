//! Ask an installed Ship for its version and protocol, and explain server sync.

use std::ffi::OsStr;

use semver::Version;
use ship_core::{HealthResponse, PROTOCOL_VERSION, prelude::*};
use tokio::process::Command;

use super::{core_version, installed};

pub const RESTART: &str =
    "restart the server: ship server stop && ship (ends running programs; tabs are restored)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probed {
    pub version: Version,
    pub protocol: u32,
}

/// What an installed binary means for the running server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sync {
    Compatible(Probed),
    Restart(Probed),
    Unknown,
}

/// Runs `<program> --version`; a bare `ship` searches PATH.
pub async fn probe(program: &OsStr) -> Result<Probed> {
    let output = Command::new(program)
        .arg("--version")
        .output()
        .await
        .map_err(|error| err!(Io, "cannot probe {} --version", program.to_string_lossy(), @external: error))?;
    if !output.status.success() {
        return Err(err!(
            Validation,
            "{} --version failed: {}",
            program.to_string_lossy(),
            output.status
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| err!(Validation, "version output is not UTF-8", @external: error))?;
    let parsed = text
        .trim()
        .strip_prefix("ship ")
        .and_then(|text| text.split_once(" (protocol "))
        .and_then(|(version, protocol)| {
            Some(Probed {
                version: core_version(version)?,
                protocol: protocol.strip_suffix(')')?.parse().ok()?,
            })
        });
    parsed.ok_or_else(|| {
        err!(
            Validation,
            "unrecognized ship version output: {}",
            text.trim()
        )
    })
}

impl Sync {
    pub fn of(probed: Result<Probed>, server: &HealthResponse) -> Self {
        match probed {
            Ok(probed) if probed.protocol == server.protocol_version => Self::Compatible(probed),
            Ok(probed) => Self::Restart(probed),
            Err(_) => Self::Unknown,
        }
    }

    /// Whether the server must restart, for the update CLI and the panel.
    pub fn guidance(&self) -> &'static str {
        match self {
            Self::Compatible(_) => "server is compatible; restart when convenient",
            Self::Restart(_) => "server must restart to stay in sync",
            Self::Unknown => "could not probe the installed ship; the server may need a restart",
        }
    }

    /// The versions, the guidance and the restart's cost.
    pub fn describe(&self, server: &HealthResponse) -> String {
        match self {
            Self::Compatible(probed) | Self::Restart(probed) => format!(
                "installed ship {} (protocol {}); server ship {} (protocol {})\n{}\n{RESTART}",
                probed.version,
                probed.protocol,
                server.version,
                server.protocol_version,
                self.guidance()
            ),
            Self::Unknown => format!(
                "could not probe the installed ship; server ship {} (protocol {}) may need a restart to stay in sync\n{RESTART}",
                server.version, server.protocol_version
            ),
        }
    }
}

/// Attach-time mismatch, naming both sides and the restart command.
pub fn mismatch(server: &HealthResponse) -> String {
    format!(
        "incompatible protocol: client ship {} (protocol {}); server ship {} (protocol {})\n{RESTART}",
        installed(),
        PROTOCOL_VERSION,
        server.version,
        server.protocol_version
    )
}
