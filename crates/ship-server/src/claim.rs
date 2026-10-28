//! Owning a server name for the life of the process.

#[cfg(unix)]
use std::fs::{self, File};

use ship_core::{endpoint::Endpoint, prelude::*};
#[cfg(unix)]
use tokio::net::UnixListener;

#[cfg(unix)]
pub(crate) type Listener = UnixListener;
#[cfg(windows)]
pub(crate) type Listener = pipe::PipeListener;

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

    /// The first pipe instance owns the name until the process exits.
    #[cfg(windows)]
    pub(crate) fn take(endpoint: Endpoint) -> Result<(Self, Listener)> {
        use std::io::ErrorKind;

        let listener = pipe::create(&endpoint).map_err(|error| match error.kind() {
            ErrorKind::PermissionDenied => {
                err!(Conflict, "a server is already running at {}", endpoint)
            }
            _ => err!(Io, "cannot create server pipe {}", endpoint, @external: error),
        })?;
        Ok((Self { endpoint }, listener))
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

#[cfg(windows)]
mod pipe {
    use std::{io, time::Duration};

    use axum::serve;

    use interprocess::{
        local_socket::{
            GenericFilePath, ListenerOptions, ToFsName,
            tokio::{Listener, Stream, prelude::*},
        },
        os::windows::{
            ToWtf16, local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
        },
    };
    use ship_core::endpoint::Endpoint;
    use tokio::time::sleep;

    /// Full access for the pipe's owner, nobody else.
    const OWNER_ONLY: &str = "D:P(A;;GA;;;OW)";

    pub(crate) struct PipeListener(Listener);

    /// interprocess creates the first instance with `FILE_FLAG_FIRST_PIPE_INSTANCE`,
    /// so a pipe someone else already holds fails with `PermissionDenied`.
    pub(super) fn create(endpoint: &Endpoint) -> io::Result<PipeListener> {
        let descriptor =
            SecurityDescriptor::deserialize(&OWNER_ONLY.to_wtf_16().expect("SDDL has no nul"))?;
        ListenerOptions::new()
            .name(endpoint.path().to_fs_name::<GenericFilePath>()?)
            .security_descriptor(descriptor)
            .create_tokio()
            .map(PipeListener)
    }

    impl serve::Listener for PipeListener {
        type Io = Stream;
        type Addr = ();

        async fn accept(&mut self) -> (Stream, ()) {
            loop {
                match self.0.accept().await {
                    Ok(stream) => return (stream, ()),
                    // As axum's own listeners: a client that left is not fatal,
                    // anything else waits a second before retrying.
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::ConnectionAborted
                                | io::ErrorKind::ConnectionReset
                                | io::ErrorKind::BrokenPipe
                        ) => {}
                    Err(error) => {
                        tracing::error!("pipe accept error: {error}");
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }

        fn local_addr(&self) -> io::Result<()> {
            Ok(())
        }
    }
}
