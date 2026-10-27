//! Input requests, numbered when their first frame is queued.

use std::{collections::VecDeque, future::pending};

use futures_util::{StreamExt, stream::FuturesUnordered};
use ship_core::{prelude::*, protocol::InputFrame};

use super::Request;
use crate::{Client, api::Order};

/// Input requests in flight at once; more input waits and goes out together.
const IN_FLIGHT: usize = 4;
/// A waiting batch takes no more frames past this, well under the server's
/// 2 MB body limit.
const BATCH_BYTES: usize = 1 << 20;

struct Batch {
    order: Order,
    frames: Vec<InputFrame>,
    bytes: usize,
}

pub(super) struct Outbox<'a> {
    client: &'a Client,
    waiting: VecDeque<Batch>,
    sending: FuturesUnordered<Request<'a>>,
}

impl<'a> Outbox<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            waiting: VecDeque::new(),
            sending: FuturesUnordered::new(),
        }
    }

    /// Join the waiting batch while no other request has taken a number
    /// since it, or else take the next number for a new one. Dropped while
    /// not numbering, as there's no attachment to send it to.
    pub fn push(&mut self, frame: InputFrame) {
        let bytes = serde_json::to_vec(&frame).map_or(0, |json| json.len());
        match self.waiting.back_mut() {
            Some(batch)
                if self.client.last_order() == Some(batch.order)
                    && batch.bytes + bytes <= BATCH_BYTES =>
            {
                batch.frames.push(frame);
                batch.bytes += bytes;
            }
            _ => {
                let Some(order) = self.client.next_order() else {
                    return;
                };
                self.waiting.push_back(Batch {
                    order,
                    frames: vec![frame],
                    bytes,
                });
            }
        }
        self.pump();
    }

    fn pump(&mut self) {
        while self.sending.len() < IN_FLIGHT
            && let Some(Batch { order, frames, .. }) = self.waiting.pop_front()
        {
            tracing::debug!(
                order = order.n,
                frames = frames.len(),
                in_flight = self.sending.len() + 1,
                "input sent"
            );
            let client = self.client;
            self.sending.push(Box::pin(async move {
                client
                    .input(Some(order.attachment), Some(order), &frames)
                    .await
            }));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty() && self.sending.is_empty()
    }

    /// Abandon everything waiting or in flight.
    pub fn clear(&mut self) {
        self.waiting.clear();
        self.sending.clear();
    }

    /// The next finished request's result, or never while none is in flight.
    pub async fn next(&mut self) -> Result<()> {
        let Some(result) = self.sending.next().await else {
            return pending().await;
        };
        self.pump();
        result
    }
}
