//! RustaSea router — expressive Axum-backed routing with groups and domains.
//!
//! Registration yields introspectable [`RouteEntry`] metadata plus, for routes
//! bound to a concrete action, an executable axum handler. `into_axum_router`
//! compiles the table into a dispatching axum router: bound actions run for
//! real, unbound routes fall back to the stub handler, and axum supplies
//! path/query/body extraction, 404 and 405 semantics.

mod dispatch;
mod handler;
mod route;
mod router;
#[cfg(test)]
mod tests;

pub use handler::Handler;
pub use route::{ControllerRef, RouteEntry};
pub use router::Router;
