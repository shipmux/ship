//! Owning a server name for the life of the process.

#[cfg(unix)]
use std::fs::{self, File};

use ship_core::{endpoint::Endpoint, prelude::*};
#[cfg(unix)]
use tokio::net::UnixListener;

#[cfg(unix)]
pub(crate) type Listener = UnixListener;

pub(crate) struct Claim {
    endpoint: Endpoint,
    #[cfg(unix)]
    _lock: File,
}

impl Claim {
    #[cfg(unix)]
    pub(crate) fn take(endpoint: Endpoint) -> Result<(Self, Listener)> {
        use std::{
            fs::{OpenOptions, TryLockError},
            io::ErrorKind,
            os::unix::fs::OpenOptionsExt,
        };

        let lock_path = endpoint.lock_path();
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&lock_path)
            .map_err(|error| {
                err!(Io, "cannot open server lock {}", lock_path.display(), @external: error)
            })?;
        lock.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => {
                err!(Conflict, "a server is already running at {}", endpoint)
            }
            TryLockError::Error(error) => {
                err!(Io, "cannot lock server name at {}", endpoint, @external: error)
            }
        })?;
        match fs::remove_file(endpoint.path()) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(err!(Io, "cannot remove stale socket {}", endpoint, @external: error));
            }
        }
        let listener = Listener::bind(endpoint.path())
            .map_err(|error| err!(Io, "cannot bind server at {}", endpoint, @external: error))?;
        Ok((
            Self {
                endpoint,
                _lock: lock,
            },
            listener,
        ))
    }

    pub(crate) fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        // Unlink before releasing the lock, so a successor's socket stays intact.
        #[cfg(unix)]
        fs::remove_file(self.endpoint.path()).ok();
    }
}
