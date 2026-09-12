//! Dioxus adapter for the `react` starter-kit variant (ADR-0002 decision 4).

use dioxus::prelude::*;

use rustasea_inertia_client::NavigationOutcome;

use crate::browser::{fetch_page, hard_navigate};
use crate::router::RouterState;

/// Dioxus context carrying the shared Inertia [`RouterState`].
#[derive(Clone, Copy)]
pub struct RouterContext {
    state: Signal<RouterState>,
}

impl RouterContext {
    /// The reactive router state signal.
    pub fn state(&self) -> Signal<RouterState> {
        self.state
    }
}

/// Provide the Inertia router to a Dioxus subtree.
///
/// `initial_page` is the JSON the server embedded in the root document's
/// `data-page` attribute. It is hydrated once, mounting the initial component
/// through the registry installed via [`crate::install_registry`].
#[component]
pub fn RouterProvider(initial_page: String, children: Element) -> Element {
    let _context = use_context_provider(|| {
        let mut router = RouterState::new();
        let _ = router.hydrate(&initial_page);
        RouterContext {
            state: Signal::new(router),
        }
    });

    rsx! {
        {children}
    }
}

/// Read the nearest [`RouterContext`].
///
/// Panics when called outside a [`RouterProvider`] subtree, matching Dioxus's
/// `use_context` contract.
pub fn use_router() -> RouterContext {
    use_context::<RouterContext>()
}

/// A Dioxus anchor that performs an Inertia visit instead of a full page load.
///
/// Clicking prevents the browser's default navigation, fetches `to` with the
/// `X-Inertia` headers, and swaps the page in place. A version conflict
/// (`409` + `X-Inertia-Location`) or a non-Inertia response hard-navigates.
#[component]
pub fn Link(to: String, children: Element) -> Element {
    let context = use_router();
    let href = to.clone();

    rsx! {
        a {
            href: "{href}",
            onclick: move |event| {
                event.prevent_default();
                let to = to.clone();
                let mut state = context.state();
                spawn(async move {
                    let snapshot = (*state.read()).clone();
                    match fetch_page(&to, &snapshot).await {
                        Ok(NavigationOutcome::Mount(page)) => {
                            state.write().set_page(page);
                        }
                        Ok(NavigationOutcome::HardNavigate(location)) => hard_navigate(&location),
                        Ok(NavigationOutcome::FullReload) | Err(_) => hard_navigate(&to),
                    }
                });
            },
            {children}
        }
    }
}
