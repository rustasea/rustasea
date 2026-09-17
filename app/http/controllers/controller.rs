//! Base controller for the application.
//!
//! This is the RustaSea analogue of Hypervel 0.4's
//! `app/Http/Controllers/Controller.php`. Every scaffolded controller implements
//! the re-exported [`Controller`](rustasea::http::Controller) trait to inherit
//! the framework response helpers (`json`, `json_status`, `validation_error`,
//! `redirect`, `see_other`) from one app-local import path.
//!
//! Add shared application helpers here (authorization gates, response shaping,
//! or common extractors) so they are available to every controller through this
//! single extension point.

pub use rustasea::http::Controller;
