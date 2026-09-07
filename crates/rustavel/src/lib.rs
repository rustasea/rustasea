//! Rustavel umbrella crate — re-exports foundation and config.

pub use rustavel_config as config;
pub use rustavel_foundation as foundation;

pub use foundation::{Application, Container, ServiceProvider};
pub use config::ConfigLoader;
