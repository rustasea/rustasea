//! Browser transport shared by the Dioxus and Leptos adapters.

use http::{HeaderMap, HeaderName, HeaderValue};
use rustasea_inertia::{X_INERTIA, X_INERTIA_LOCATION};
use rustasea_inertia_client::{InertiaClient, NavigationOutcome};

use crate::error::AdapterError;
use crate::registry::InstalledRegistry;
use crate::router::RouterState;

/// Perform an Inertia visit to `url` and return the navigation outcome.
///
/// The request carries the `X-Inertia` / `X-Inertia-Version` headers built from
/// `state`, so the server answers with a page object rather than the HTML shell
/// — the link never triggers a full document load.
pub async fn fetch_page(url: &str, state: &RouterState) -> Result<NavigationOutcome, AdapterError> {
    let headers = state.navigation_headers("", &[], &[]);
    let mut request = gloo_net::http::Request::get(url);
    for (name, value) in headers.iter() {
        if let Ok(value) = value.to_str() {
            request = request.header(name.as_str(), value);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|error| AdapterError::Request(error.to_string()))?;
    let status = response.status();
    let response_headers = to_header_map(&response.headers());
    let body = response
        .text()
        .await
        .map_err(|error| AdapterError::Request(error.to_string()))?;

    let client = InertiaClient::new(Box::new(InstalledRegistry), state.root_id());
    Ok(client.handle_response(status, &response_headers, &body)?)
}

/// Hard-navigate the browser to `url`.
///
/// Used for an asset-version conflict (`409` + `X-Inertia-Location`) and as the
/// fallback when the server returns non-Inertia HTML.
pub fn hard_navigate(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_href(url);
    }
}

/// Copy the Inertia-relevant response headers into an [`http::HeaderMap`].
///
/// `InertiaClient::handle_response` only inspects the marker and location
/// headers, so extracting those two is sufficient and avoids depending on the
/// browser header collection's full API.
fn to_header_map(headers: &gloo_net::http::Headers) -> HeaderMap {
    let mut map = HeaderMap::new();
    for name in [X_INERTIA, X_INERTIA_LOCATION] {
        if let Some(value) = headers.get(name) {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                map.insert(name, value);
            }
        }
    }
    map
}
