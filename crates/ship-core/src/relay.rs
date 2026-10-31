use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::ControlFlow,
};

use kameo::prelude::*;
use ship_macros::Actor;
use tokio::sync::mpsc;

#[derive(Actor, Default)]
pub struct RelayBus {
    subscriptions: HashMap<TypeId, Vec<Box<dyn Any + Send>>>,
}

/// Subscribe a typed sink to receive publications of type `T`.
pub struct Subscribe<T> {
    pub sink: Box<dyn Sink<T> + 'static>,
}

/// Clone `T` into every sink subscribed to it.
pub struct Publish<T>(pub T);

impl<T: Clone + Send + 'static> Message<Subscribe<T>> for RelayBus {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Subscribe<T>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let type_id = TypeId::of::<T>();
        self.subscriptions
            .entry(type_id)
            .or_default()
            .push(Box::new(msg.sink));
    }
}

impl<T: Clone + Send + 'static> Message<Publish<T>> for RelayBus {
    type Reply = ();

    /// A sink that answers `Break` is removed.
    async fn handle(
        &mut self,
        msg: Publish<T>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let type_id = TypeId::of::<T>();
        if let Some(sinks) = self.subscriptions.get_mut(&type_id) {
            sinks.retain_mut(|any_sink| {
                if let Some(sink) = any_sink.downcast_mut::<Box<dyn Sink<T> + 'static>>() {
                    matches!(sink.try_send(msg.0.clone()), ControlFlow::Continue(_))
                } else {
                    true
                }
            });
            if sinks.is_empty() {
                self.subscriptions.remove(&type_id);
            }
        }
    }
}

pub trait Sink<T>: Send + 'static {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    Sent,
    Closed,
}

/// `Closed` once the receiver is gone. For publications that must not
/// drop, such as exit statuses.
impl<T: Send + 'static> Sink<T> for mpsc::UnboundedSender<T> {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        match self.send(value) {
            Ok(()) => ControlFlow::Continue(SendResult::Sent),
            Err(_) => ControlFlow::Break(SendResult::Closed),
        }
    }
}
