//! Where a server listens and the names that pick it.

use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
#[cfg(unix)]
use url::Url;

use crate::prelude::*;

/// One to 32 ASCII letters, digits, `-` or `_`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ServerName(String);

impl ServerName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ServerName {
    fn default() -> Self {
        Self("default".into())
    }
}

impl FromStr for ServerName {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self> {
        value.to_owned().try_into()
    }
}

impl TryFrom<String> for ServerName {
    type Error = AppError;

    fn try_from(value: String) -> Result<Self> {
        if value.is_empty()
            || value.len() > 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(err!(
                Configuration,
                "server name must be 1 to 32 ASCII letters, digits, '-' or '_'"
            ));
        }
        Ok(Self(value))
    }
}

impl From<ServerName> for String {
    fn from(name: ServerName) -> Self {
        name.0
    }
}

impl fmt::Display for ServerName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A Unix socket path, or a named pipe path on Windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint(PathBuf);

impl Endpoint {
    /// Resolve a name and create or check its owner-only socket directory.
    #[cfg(unix)]
    pub fn named(name: &ServerName) -> Result<Self> {
        let directory = dir::socket_dir()?;
        dir::private(&directory)?;
        Ok(Self(directory.join(format!("{name}.sock"))))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    #[cfg(unix)]
    pub fn url(&self) -> String {
        let file = Url::from_file_path(&self.0).expect("endpoint paths are absolute");
        let mut url = Url::parse("unix:///").expect("valid Unix socket URL base");
        url.set_path(file.path());
        url.into()
    }

    /// Never deleted: its inode arbitrates ownership for the server's life.
    #[cfg(unix)]
    pub fn lock_path(&self) -> PathBuf {
        self.0.with_extension("lock")
    }
}

#[cfg(unix)]
impl FromStr for Endpoint {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self> {
        let url = Url::parse(value).map_err(|error| {
            err!(Configuration, "invalid server URL; expected unix:///absolute/path", @external: error)
        })?;
        if url.scheme() != "unix" {
            return Err(err!(
                Configuration,
                "server URL must be unix:///absolute/path; HTTP/HTTPS targets are no longer supported"
            ));
        }
        if !url.has_authority()
            || url.has_host()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(err!(
                Configuration,
                "server URL requires an absolute socket path without a host, query or fragment: unix:///absolute/path"
            ));
        }
        let path = url.to_file_path().map_err(|()| {
            err!(
                Configuration,
                "server URL requires an absolute socket path: unix:///absolute/path"
            )
        })?;
        Ok(Self(path))
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(formatter)
    }
}

#[cfg(unix)]
mod dir {
    use std::{
        env, fs,
        os::unix::fs::{DirBuilderExt, MetadataExt},
        path::{self, Path, PathBuf},
    };

    use nix::unistd::getuid;

    use crate::prelude::*;

    pub(super) fn socket_dir() -> Result<PathBuf> {
        let directory = env::var_os("XDG_RUNTIME_DIR").map(|base| PathBuf::from(base).join("ship"));
        #[cfg(target_os = "macos")]
        let directory = directory
            .or_else(|| env::var_os("TMPDIR").map(|base| PathBuf::from(base).join("ship")));
        let directory =
            directory.unwrap_or_else(|| PathBuf::from(format!("/tmp/ship-{}", getuid())));
        path::absolute(&directory).map_err(|error| {
            err!(Io, "cannot resolve socket directory {}", directory.display(), @external: error)
        })
    }

    pub(super) fn private(directory: &Path) -> Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
            .map_err(|error| {
                err!(Io, "cannot create socket directory {}", directory.display(), @external: error)
            })?;
        let metadata = fs::metadata(directory).map_err(|error| {
            err!(Io, "cannot inspect socket directory {}", directory.display(), @external: error)
        })?;
        if !metadata.is_dir() || metadata.uid() != getuid().as_raw() || metadata.mode() & 0o077 != 0
        {
            return Err(err!(
                Configuration,
                "socket directory {} must be owned by the current user and have no group or other permissions (0700)",
                directory.display()
            ));
        }
        Ok(())
    }
}
