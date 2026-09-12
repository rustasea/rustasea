/// Typed listener-job wrapper bridging `Listener<E>` into the queue.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::dispatcher;
use crate::event::Event;

/// Serializable queue job that re-invokes a registered listener for `E`.
///
/// The dispatcher enqueues this wrapper when a listener opts into queue-backed
/// execution (`Queue { enable: true }`). The body carries the listener's stable
/// type key plus the serialized event; a worker rebuilds it and resolves the
/// listener from the process-wide registry before invoking `handle`.
#[derive(Serialize)]
pub struct ListenerJob<E: Event> {
    /// Stable listener type key (`std::any::type_name::<L>()`).
    pub listener: String,
    /// Event payload delivered to the listener.
    pub event: E,
}

impl<'de, E: Event> Deserialize<'de> for ListenerJob<E> {
    /// Deserialize the listener key and the event body.
    ///
    /// Hand-written because deriving `Deserialize` over an `E: Event` bound
    /// (which already carries `DeserializeOwned`) is ambiguous for serde's
    /// generated lifetime-bound; the event is reconstructed from its JSON body
    /// through the `DeserializeOwned` supertrait instead.
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            listener: String,
            event: serde_json::Value,
        }
        let raw = Raw::deserialize(deserializer)?;
        let event = serde_json::from_value::<E>(raw.event).map_err(serde::de::Error::custom)?;
        Ok(Self {
            listener: raw.listener,
            event,
        })
    }
}

impl<E: Event> ListenerJob<E> {
    /// Build a listener job for `listener` carrying `event`.
    pub fn new(listener: impl Into<String>, event: E) -> Self {
        Self {
            listener: listener.into(),
            event,
        }
    }
}

#[async_trait]
impl<E: Event> rustasea_queue::Job for ListenerJob<E> {
    /// Resolve the registered listener by key and invoke it inline.
    ///
    /// A missing listener (not registered in the worker process) surfaces as a
    /// typed `JobError::Exception`, so the worker retries then dead-letters the
    /// payload instead of silently dropping it.
    async fn handle(self) -> std::result::Result<(), rustasea_queue::JobError> {
        dispatcher::invoke_listener_by_key::<E>(&self.listener, self.event)
            .await
            .map_err(|error| rustasea_queue::JobError::Exception(error.to_string()))
    }
}

/// Register the worker handler for [`ListenerJob<E>`].
///
/// Called automatically by `Dispatcher::listen` for every queue-enabled
/// listener; applications that build their listener registry elsewhere can call
/// it explicitly at boot so `queue:work` can resolve the job kind.
pub fn register_listener_job<E: Event>() {
    rustasea_queue::register_job::<ListenerJob<E>>();
}
