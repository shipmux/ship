//! Versions shared by the update CLI and the full-screen client.

mod probe;

pub use probe::{Probed, Sync, mismatch, probe};

use semver::Version;

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
