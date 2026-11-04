//! Release checks and the cache shared by every client on this machine.

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use ship_core::prelude::*;

use super::{RELEASE_REPO, core_version, installed, shell};

pub const FRESH_FOR: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Cached,
    Force,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Release {
    pub version: Version,
    pub published: Option<String>,
    pub url: String,
    pub notes: String,
}

/// Independent of the reader's version, so old and new clients share it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Found {
    pub checked_at: SystemTime,
    pub latest: Option<Version>,
    /// Stable releases from GitHub's first page, newest version first.
    pub releases: Vec<Release>,
    pub announced: Option<Version>,
}

/// Returned only when a TUI client claims the one-time notice.
pub struct Checked {
    pub found: Found,
    pub announce: bool,
}

#[derive(Clone)]
pub struct Checker {
    http: reqwest::Client,
    cache: PathBuf,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    html_url: String,
    body: Option<String>,
}

impl Checker {
    pub fn new(cache: PathBuf) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("ship/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(
                |error| err!(Configuration, "cannot initialize release checker", @external: error),
            )?;
        Ok(Self { http, cache })
    }

    /// Missing, corrupt or otherwise unreadable means no cached result.
    pub fn cached(&self) -> Option<Found> {
        serde_json::from_slice(&fs::read(&self.cache).ok()?).ok()
    }

    /// Checks never consume a notice; the TUI claims it with `announce`.
    pub async fn check(&self, freshness: Freshness, latest_command: Option<&str>) -> Result<Found> {
        let cached = self.cached();
        if freshness == Freshness::Cached
            && let Some(found) = &cached
            && found.checked_at.elapsed().is_ok_and(|age| age < FRESH_FOR)
        {
            return Ok(found.clone());
        }

        tracing::info!(command = latest_command, "asking for the latest release");
        let from_command = match latest_command {
            Some(line) => {
                let output = shell(line)
                    .output()
                    .await
                    .map_err(|error| err!(Io, "cannot run latest_command", @external: error))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let why = match stderr.trim() {
                        "" => String::new(),
                        stderr => format!(": {stderr}"),
                    };
                    return Err(err!(
                        Validation,
                        "latest_command failed: {}{why}",
                        output.status
                    ));
                }
                let text = String::from_utf8(output.stdout).map_err(|error| {
                    err!(Validation, "latest_command output is not UTF-8", @external: error)
                })?;
                Some(core_version(&text).ok_or_else(|| {
                    err!(
                        Validation,
                        "latest_command returned no version: {}",
                        text.trim()
                    )
                })?)
            }
            None => None,
        };
        // Commands determine the version, but notes always come from GitHub.
        let response = self
            .http
            .get(format!(
                "https://api.github.com/repos/{RELEASE_REPO}/releases?per_page=30"
            ))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| err!(Network, "cannot fetch GitHub releases", @external: error))?;
        let releases: Vec<GithubRelease> = response
            .json()
            .await
            .map_err(|error| err!(Validation, "cannot read GitHub releases", @external: error))?;
        let mut releases: Vec<Release> = releases
            .into_iter()
            .filter(|release| !release.draft && !release.prerelease)
            .filter_map(|release| {
                let tag = release
                    .tag_name
                    .strip_prefix('v')
                    .unwrap_or(&release.tag_name);
                let version = Version::parse(tag).ok()?;
                if !version.pre.is_empty() {
                    return None;
                }
                Some(Release {
                    version: core_version(tag)?,
                    published: release.published_at,
                    url: release.html_url,
                    notes: release.body.unwrap_or_default(),
                })
            })
            .collect();
        releases.sort_by(|a, b| b.version.cmp(&a.version));
        let found = Found {
            checked_at: SystemTime::now(),
            latest: from_command
                .or_else(|| releases.first().map(|release| release.version.clone())),
            releases,
            // Re-read so a notice claimed during the fetch isn't forgotten.
            announced: self.cached().or(cached).and_then(|found| found.announced),
        };
        self.store(&found)?;
        Ok(found)
    }

    /// Claim a note for a TUI client, not for a CLI check. Called when the
    /// client is ready to display it, including when it reused a cached check.
    pub fn announce(&self, mut found: Found) -> Result<Checked> {
        let announced = self.cached().and_then(|cached| cached.announced);
        found.announced = found.announced.max(announced);
        let announce = found.latest.as_ref().is_some_and(|latest| {
            latest > &installed()
                && found
                    .announced
                    .as_ref()
                    .is_none_or(|announced| latest > announced)
        });
        if announce {
            found.announced = found.latest.clone();
            self.store(&found)?;
        }
        Ok(Checked { found, announce })
    }

    /// NamedTempFile supplies private, unique files and atomic replacement.
    fn store(&self, found: &Found) -> Result<()> {
        let bytes = serde_json::to_vec(found)
            .map_err(|error| err!(Internal, "cannot encode update cache", @external: error))?;
        let write = || -> io::Result<()> {
            let parent = self.cache.parent().unwrap_or(Path::new("."));
            fs::create_dir_all(parent)?;
            let mut file = tempfile::NamedTempFile::new_in(parent)?;
            file.write_all(&bytes)?;
            file.persist(&self.cache).map_err(|error| error.error)?;
            Ok(())
        };
        write().map_err(|error| {
            err!(Io, "cannot write update cache {}", self.cache.display(), @external: error)
        })
    }
}

impl Found {
    pub fn newer<'a>(&'a self, than: &'a Version) -> impl Iterator<Item = &'a Release> {
        self.releases
            .iter()
            .filter(move |release| &release.version > than)
    }

    pub fn release(&self, version: &Version) -> Option<&Release> {
        self.releases
            .iter()
            .find(|release| &release.version == version)
    }
}
