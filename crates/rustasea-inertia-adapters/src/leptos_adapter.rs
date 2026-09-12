//! Leptos adapter for the `vue` starter-kit variant (ADR-0002 decision 4).

use leptos::prelude::*;

use rustasea_inertia_client::NavigationOutcome;

use crate::browser::{fetch_page, hard_navigate};
use crate::router::RouterState;

/// Leptos context carrying the shared Inertia [`RouterState`].
#[derive(Clone, Copy)]
pub struct RouterContext {
    state: RwSignal<RouterState>,
}

impl RouterContext {
    /// The reactive router state signal.
    pub fn state(&self) -> RwSignal<RouterState> {
        self.state
    }
}

/// Provide the Inertia router to a Leptos subtree.
///
/// `initial_page` is the JSON the server embedded in the root document's
/// `data-page` attribute. It is hydrated once, mounting the initial component
/// through the registry installed via [`crate::install_registry`].
#[component]
pub fn RouterProvider(initial_page: String, children: Children) -> impl IntoView {
    let state = RwSignal::new({
        let mut router = RouterState::new();
        let _ = router.hydrate(&initial_page);
        router
    });

    provide_context(RouterContext { state });
    children()
}

/// Read the nearest [`RouterContext`].
///
/// Panics when called outside a [`RouterProvider`] subtree, matching Leptos's
/// `expect_context` contract.
pub fn use_router() -> RouterContext {
    expect_context::<RouterContext>()
}

/// A Leptos anchor that performs an Inertia visit instead of a full page load.
///
/// Clicking prevents the browser's default navigation, fetches `to` with the
/// `X-Inertia` headers, and swaps the page in place. A version conflict
/// (`409` + `X-Inertia-Location`) or a non-Inertia response hard-navigates.
#[component]
pub fn Link(to: String, children: Children) -> impl IntoView {
    let context = use_router();
    let href = to.clone();

    view! {
        <a
            href=href
            on:click=move |event| {
                event.prevent_default();
                let to = to.clone();
                leptos::task::spawn_local(async move {
                    let snapshot = context.state().get();
                    match fetch_page(&to, &snapshot).await {
                        Ok(NavigationOutcome::Mount(page)) => {
                            context.state().update(|router| router.set_page(page));
                        }
                        Ok(NavigationOutcome::HardNavigate(location)) => hard_navigate(&location),
                        Ok(NavigationOutcome::FullReload) | Err(_) => hard_navigate(&to),
                    }
                });
            }
        >
            {children()}
        </a>
    }
}
