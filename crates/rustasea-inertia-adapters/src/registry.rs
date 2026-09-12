//! Component mounting through a [`ComponentRegistry`].
//!
//! A generated app installs its registry once at boot; the reactive adapters
//! then mount every incoming page through [`mount_installed`]. WASM is
//! single-threaded, so the installed registry lives in a thread-local slot.
//! Tests and embedders that need isolation call [`mount_page`] with an explicit
//! registry instead.

use std::cell::RefCell;

use rustasea_inertia_client::{ClientError, ComponentRegistry, Page, Value};

thread_local! {
    /// Registry installed for the current thread (the WASM app thread).
    static INSTALLED: RefCell<Option<Box<dyn ComponentRegistry>>> = const { RefCell::new(None) };
}

/// Install the generated component registry used by [`mount_installed`].
///
/// Call once during boot, before hydrating the initial page. Installing again
/// replaces the previous registry.
pub fn install_registry(registry: Box<dyn ComponentRegistry>) {
    INSTALLED.with(|slot| {
        *slot.borrow_mut() = Some(registry);
    });
}

/// Mount `component` through the registry installed by [`install_registry`].
///
/// Returns [`ClientError::Mount`] when no registry has been installed yet.
pub fn mount_installed(component: &str, props: &Value) -> Result<(), ClientError> {
    INSTALLED.with(|slot| match slot.borrow().as_ref() {
        Some(registry) => registry.mount(component, props),
        None => Err(ClientError::Mount {
            component: component.to_string(),
            message: "no component registry installed".to_string(),
        }),
    })
}

/// Parse `page_json` and mount its component through `registry`.
///
/// This is the registry-agnostic mounting path used by tests and by embedders
/// that manage the registry themselves. On success it returns the parsed page.
pub fn mount_page(
    registry: &dyn ComponentRegistry,
    page_json: &str,
) -> Result<Page<Value>, ClientError> {
    let page: Page<Value> =
        serde_json::from_str(page_json).map_err(|source| ClientError::ParsePage { source })?;
    registry.mount(&page.component, &page.props)?;
    Ok(page)
}

/// Registry adapter that resolves components through the installed thread-local.
///
/// Lets [`crate::RouterState`] and [`crate::fetch_page`] reuse
/// [`rustasea_inertia_client::InertiaClient`] without owning the registry.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct InstalledRegistry;

impl ComponentRegistry for InstalledRegistry {
    /// Mount `component` through the installed registry.
    fn mount(&self, component: &str, props: &Value) -> Result<(), ClientError> {
        mount_installed(component, props)
    }
}
