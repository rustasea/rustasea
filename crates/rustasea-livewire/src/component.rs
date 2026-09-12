//! Component definitions: templates, state-mutating actions, and the broadcast
//! channel used for realtime updates.

use std::collections::HashMap;
use std::sync::Arc;

use rustasea_broadcast::Channel;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::LivewireError;
use crate::state::{ComponentState, StatePatch};

/// Handler invoked to mutate a component's state.
///
/// The handler receives the current state and mutates it in place; returning
/// `Err` aborts the request and no fragment is rendered or broadcast.
pub type ActionHandler =
    Arc<dyn Fn(&mut ComponentState) -> Result<(), LivewireError> + Send + Sync>;

/// A server-rendered component.
///
/// A component binds a fragment template (returned for HTMX partial swaps), an
/// optional full-page template, named state-mutating actions, and the broadcast
/// channel its updates are published on. It holds no per-request state: the
/// browser owns the state and posts it back with each action (ADR-0002 §D5).
#[derive(Clone)]
pub struct Component {
    /// Registry/URL name.
    name: String,
    /// Template rendered for partial swaps.
    fragment_template: String,
    /// Optional template rendered for the initial full page.
    page_template: Option<String>,
    /// Channel updates are broadcast on.
    channel: Channel,
    /// Named action handlers.
    actions: HashMap<String, ActionHandler>,
}

impl Component {
    /// Create a component that renders `fragment_template` for partial swaps.
    ///
    /// The default broadcast channel is `Channel::Public(name)`.
    pub fn new(name: impl Into<String>, fragment_template: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            channel: Channel::Public(name.clone()),
            name,
            fragment_template: fragment_template.into(),
            page_template: None,
            actions: HashMap::new(),
        }
    }

    /// Set the template used for the initial full-page render.
    pub fn with_page_template(mut self, template: impl Into<String>) -> Self {
        self.page_template = Some(template.into());
        self
    }

    /// Broadcast updates on `channel` instead of the default public channel.
    pub fn on_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    /// Register a named state-mutating action.
    pub fn action<F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(&mut ComponentState) -> Result<(), LivewireError> + Send + Sync + 'static,
    {
        self.actions.insert(name.into(), Arc::new(handler));
        self
    }

    /// Register an action that applies a fixed shallow state patch.
    pub fn patch_action(self, name: impl Into<String>, patch: StatePatch) -> Self {
        self.action(name, move |state| {
            state.apply(&patch).map_err(LivewireError::from)
        })
    }

    /// Component name (registry key and URL segment).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Template name rendered for HTMX partial swaps.
    pub fn fragment_template(&self) -> &str {
        &self.fragment_template
    }

    /// Template name rendered for the initial full page, if any.
    pub fn page_template(&self) -> Option<&str> {
        self.page_template.as_deref()
    }

    /// Broadcast channel this component publishes updates on.
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    /// Handler registered for `action`, if any.
    pub fn handler(&self, action: &str) -> Option<&ActionHandler> {
        self.actions.get(action)
    }
}

/// Body of an HTMX action request.
///
/// `state` is the browser-held component state; `patch` is an optional shallow
/// merge applied before the action runs. Both are untrusted — the runtime
/// authorizes the action before reading either.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionRequest {
    /// Current client-held component state (JSON object).
    #[serde(default = "empty_state")]
    pub state: Value,
    /// Shallow patch merged before the action runs.
    #[serde(default)]
    pub patch: StatePatch,
}

impl ActionRequest {
    /// Create a request from state and an empty patch.
    pub fn new(state: Value) -> Self {
        Self {
            state,
            patch: StatePatch::default(),
        }
    }
}

/// Default state for a request that omits `state`: an empty object.
fn empty_state() -> Value {
    Value::Object(Map::new())
}
