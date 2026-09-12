//! Framework-agnostic Inertia router state.

use http::HeaderMap;
use rustasea_inertia_client::{
    ClientError, ComponentRegistry, InertiaClient, NavigationOutcome, Page, Value,
};

use crate::registry::{mount_page, InstalledRegistry};

/// The current Inertia page plus the asset version used for navigations.
///
/// The state is `Send + Sync` and holds only owned JSON, so both Dioxus
/// `Signal` and Leptos `RwSignal` can wrap it. Mounting is delegated to the
/// registry installed through [`crate::install_registry`].
#[derive(Debug, Clone, PartialEq)]
pub struct RouterState {
    page: Option<Page<Value>>,
    version: Option<String>,
    root_id: String,
}

impl Default for RouterState {
    /// Start with no page, no version, and the default `app` mount element.
    fn default() -> Self {
        Self {
            page: None,
            version: None,
            root_id: "app".to_string(),
        }
    }
}

impl RouterState {
    /// Create an empty router state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the asset version sent with navigations.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the id of the element the app mounts into.
    pub fn with_root_id(mut self, root_id: impl Into<String>) -> Self {
        self.root_id = root_id.into();
        self
    }

    /// The configured asset version, if any.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The mount element id.
    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    /// The current page, if one has been hydrated.
    pub fn page(&self) -> Option<&Page<Value>> {
        self.page.as_ref()
    }

    /// The current component key, if a page is loaded.
    pub fn component(&self) -> Option<&str> {
        self.page.as_ref().map(|page| page.component.as_str())
    }

    /// The current page props, if a page is loaded.
    pub fn props(&self) -> Option<&Value> {
        self.page.as_ref().map(|page| &page.props)
    }

    /// The current request URL, if a page is loaded.
    pub fn url(&self) -> Option<&str> {
        self.page.as_ref().map(|page| page.url.as_str())
    }

    /// Replace the current page, adopting its version when none is configured.
    pub fn set_page(&mut self, page: Page<Value>) {
        if self.version.is_none() {
            self.version = Some(page.version.clone());
        }
        self.page = Some(page);
    }

    /// Drop the current page.
    pub fn clear(&mut self) {
        self.page = None;
    }

    /// Build the `X-Inertia*` headers for a visit to `component`.
    ///
    /// `component` is only required for partial reloads; an empty value with
    /// empty `only`/`except` produces a full-visit header set.
    pub fn navigation_headers(&self, component: &str, only: &[&str], except: &[&str]) -> HeaderMap {
        self.client().navigation_headers(component, only, except)
    }

    /// Parse `page_json`, mount through the installed registry, and store it.
    pub fn hydrate(&mut self, page_json: &str) -> Result<Page<Value>, ClientError> {
        let page = self.client().hydrate(page_json)?;
        self.set_page(page.clone());
        Ok(page)
    }

    /// Parse `page_json`, mount through `registry`, and store it.
    ///
    /// Registry-explicit variant of [`RouterState::hydrate`], used by tests and
    /// embedders that do not install a global registry.
    pub fn hydrate_with(
        &mut self,
        registry: &dyn ComponentRegistry,
        page_json: &str,
    ) -> Result<Page<Value>, ClientError> {
        let page = mount_page(registry, page_json)?;
        self.set_page(page.clone());
        Ok(page)
    }

    /// Apply an HTTP response, mounting the page on success.
    ///
    /// A `409` yields [`NavigationOutcome::HardNavigate`]; an Inertia-marked
    /// response is parsed and mounted; anything else is a full reload.
    pub fn handle_response(
        &mut self,
        status: u16,
        headers: &HeaderMap,
        body: &str,
    ) -> Result<NavigationOutcome, ClientError> {
        let outcome = self.client().handle_response(status, headers, body)?;
        if let NavigationOutcome::Mount(page) = &outcome {
            self.set_page(page.clone());
        }
        Ok(outcome)
    }

    /// Build an Inertia client that mounts through the installed registry.
    fn client(&self) -> InertiaClient {
        let mut client = InertiaClient::new(Box::new(InstalledRegistry), self.root_id.clone());
        if let Some(version) = &self.version {
            client = client.with_version(version.clone());
        }
        client
    }
}
