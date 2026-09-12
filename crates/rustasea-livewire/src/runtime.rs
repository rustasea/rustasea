//! The livewire runtime: component registry, action execution, rendering, and
//! realtime fan-out.

use std::collections::HashMap;
use std::sync::Arc;

use rustasea_broadcast::{BroadcastHub, WsMessage};
use rustasea_view::{ViewEngine, ViewResponse};
use serde_json::Value;

use crate::authorizer::{ActionAuthorizer, Actor};
use crate::component::Component;
use crate::error::LivewireError;
use crate::state::{ComponentState, StatePatch};

/// Result of a successfully executed component action.
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Re-rendered HTML fragment for the HTMX swap.
    pub fragment: ViewResponse,
    /// Patched component state after the action ran.
    pub state: Value,
    /// Whether the broadcast hub accepted the realtime fan-out.
    ///
    /// Delivery is best-effort: a channel with no subscribers still reports
    /// `true` because the hub accepted the message.
    pub published: bool,
}

/// Server-side runtime holding components, the view engine, the action
/// authorizer, and the broadcast hub.
///
/// The runtime is `Clone` (all fields are cheap handles) so it can be used as
/// axum router state while remaining shareable with background publishers.
#[derive(Clone)]
pub struct Livewire {
    /// Engine used to render component templates.
    engine: Arc<dyn ViewEngine>,
    /// Gate consulted before every action.
    authorizer: Arc<dyn ActionAuthorizer>,
    /// Realtime fan-out hub.
    hub: BroadcastHub,
    /// Registered components by name.
    components: HashMap<String, Arc<Component>>,
}

impl Livewire {
    /// Create a runtime from a view engine, an action authorizer, and a hub.
    pub fn new(
        engine: Arc<dyn ViewEngine>,
        authorizer: Arc<dyn ActionAuthorizer>,
        hub: BroadcastHub,
    ) -> Self {
        Self {
            engine,
            authorizer,
            hub,
            components: HashMap::new(),
        }
    }

    /// Register a component, replacing any previous component of that name.
    pub fn register(&mut self, component: Component) {
        self.components
            .insert(component.name().to_string(), Arc::new(component));
    }

    /// Borrow the broadcast hub (e.g. to subscribe a test receiver).
    pub fn hub(&self) -> &BroadcastHub {
        &self.hub
    }

    /// Borrow the view engine.
    pub fn engine(&self) -> &Arc<dyn ViewEngine> {
        &self.engine
    }

    /// Look up a registered component.
    pub fn component(&self, name: &str) -> Option<&Component> {
        self.components.get(name).map(Arc::as_ref)
    }

    /// Render a component's full-page template.
    pub fn render_page(&self, name: &str, state: &Value) -> Result<ViewResponse, LivewireError> {
        let template = self
            .component(name)
            .and_then(Component::page_template)
            .ok_or_else(|| LivewireError::ComponentNotFound {
                name: name.to_string(),
            })?;
        self.engine
            .render_value(template, state)
            .map_err(|source| LivewireError::Render {
                component: name.to_string(),
                source,
            })
    }

    /// Render a component's HTMX fragment template.
    pub fn render_fragment(
        &self,
        name: &str,
        state: &Value,
    ) -> Result<ViewResponse, LivewireError> {
        let template = self
            .component(name)
            .map(Component::fragment_template)
            .ok_or_else(|| LivewireError::ComponentNotFound {
                name: name.to_string(),
            })?;
        self.engine
            .render_value(template, state)
            .map_err(|source| LivewireError::Render {
                component: name.to_string(),
                source,
            })
    }

    /// Authorize and run `action`, returning the patched state and fragment.
    ///
    /// Order is deliberate: component lookup, then authorization, then action
    /// lookup, then state mutation. An unauthorized caller cannot distinguish a
    /// missing action from a denied one. On success the fragment is fanned out
    /// to the component's channel (best-effort — a broadcast failure never
    /// fails the HTTP response).
    pub async fn run_action(
        &self,
        name: &str,
        action: &str,
        actor: Option<&Actor>,
        state: Value,
        patch: &StatePatch,
    ) -> Result<ActionResult, LivewireError> {
        let component = self
            .component(name)
            .ok_or_else(|| LivewireError::ComponentNotFound {
                name: name.to_string(),
            })?;

        if !self.authorizer.is_authorized(actor, name, action) {
            return Err(LivewireError::Unauthorized {
                component: name.to_string(),
                action: action.to_string(),
            });
        }

        let handler = component
            .handler(action)
            .ok_or_else(|| LivewireError::ActionNotFound {
                component: name.to_string(),
                action: action.to_string(),
            })?;

        let mut state = ComponentState::new(state)?;
        state.apply(patch)?;
        handler(&mut state)?;

        let value = state.into_value();
        let fragment = self.render_fragment(name, &value)?;
        let published = self.publish(component, action, fragment.body()).await;

        Ok(ActionResult {
            fragment,
            state: value,
            published,
        })
    }

    /// Publish a rendered fragment to the component's channel.
    async fn publish(&self, component: &Component, event: &str, fragment: &str) -> bool {
        let channel = component.channel().auth_channel();
        let message = WsMessage::new(event, &channel, fragment.to_string());
        self.hub.publish(&channel, message).await.is_ok()
    }
}

impl axum::extract::FromRef<Livewire> for BroadcastHub {
    /// Expose the hub so `rustasea_broadcast::ws_route::<Livewire>()` can mount
    /// the WebSocket endpoint with this runtime as router state.
    fn from_ref(state: &Livewire) -> Self {
        state.hub.clone()
    }
}
