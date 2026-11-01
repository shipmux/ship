//! Published replicas: the latest state and a bounded every-revision feed.

use std::sync::Arc;

use ship_core::protocol::Replica;
use tokio::sync::{broadcast, watch};

/// Revisions an every-revision stream can fall behind before it is lost.
pub(crate) const RING: usize = 256;

/// Read handles on the published replicas.
#[derive(Clone)]
pub struct Replicas {
    latest: watch::Receiver<Arc<Replica>>,
    // Only the state actor's sink keeps publication alive, so shutdown closes both feeds.
    ring: broadcast::WeakSender<Arc<Replica>>,
}

impl Replicas {
    pub fn latest(&self) -> watch::Receiver<Arc<Replica>> {
        self.latest.clone()
    }

    /// Subscribe before the attach commit. None once publication has stopped.
    pub(crate) fn every(&self) -> Option<broadcast::Receiver<Arc<Replica>>> {
        Some(self.ring.upgrade()?.subscribe())
    }

    /// The replica published last.
    pub(crate) fn current(&self) -> Arc<Replica> {
        Arc::clone(&self.latest.borrow())
    }
}

/// The publishing side, owned by the state actor. Dropping it ends both feeds.
pub(crate) struct ReplicaSink {
    latest: watch::Sender<Arc<Replica>>,
    ring: broadcast::Sender<Arc<Replica>>,
}

impl ReplicaSink {
    pub(crate) fn new(first: Arc<Replica>) -> Self {
        Self {
            latest: watch::Sender::new(first),
            ring: broadcast::Sender::new(RING),
        }
    }

    /// Never fails: `send_replace`, and no ring readers is normal.
    pub(crate) fn publish(&self, replica: Arc<Replica>) {
        self.ring.send(Arc::clone(&replica)).ok();
        self.latest.send_replace(replica);
    }

    pub(crate) fn latest(&self) -> Arc<Replica> {
        Arc::clone(&self.latest.borrow())
    }

    /// A read handle for routes, streams and the saver.
    pub(crate) fn subscribe(&self) -> Replicas {
        Replicas {
            latest: self.latest.subscribe(),
            ring: self.ring.downgrade(),
        }
    }
}
