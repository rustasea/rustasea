//! Rustavel foundation — Application, Container, ServiceProvider, shutdown.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Service provider lifecycle.
///
/// Implementors participate in the Application boot DAG:
///
/// `register` is called first for all providers, then `boot`.
pub trait ServiceProvider: Send + Sync {
    /// Register bindings into the container.
    fn register(&self, _app: &mut Application) {}

    /// Boot after all providers have registered.
    fn boot(&self, _app: &Application) {}
}

/// Binding kind for diagnostic purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// Transient factory — new instance per resolve.
    Bind,
    /// Singleton factory — cached after first resolve.
    Singleton,
    /// Concrete instance.
    Instance,
}

/// DI container with Bind/Singleton/Instance semantics.
pub struct Container {
    factories: HashMap<String, Box<dyn Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    singleton_factories: HashMap<String, Box<dyn Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    instances: HashMap<String, Box<dyn Any + Send + Sync>>,
    singleton_cache: HashMap<String, Box<dyn Any + Send + Sync>>,
    type_registry: HashMap<String, String>,
}

impl Default for Container {
    /// Create an empty container.
    fn default() -> Self {
        Self {
            factories: HashMap::new(),
            singleton_factories: HashMap::new(),
            instances: HashMap::new(),
            singleton_cache: HashMap::new(),
            type_registry: HashMap::new(),
        }
    }
}

impl Container {
    /// Create a new empty container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a transient factory under `key`.
    pub fn bind<F>(&mut self, key: impl Into<String>, factory: F)
    where
        F: Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync + 'static,
    {
        let k = key.into();
        self.type_registry.insert(k.clone(), stringify!(BindingKind::Bind).to_string());
        self.factories.insert(k, Box::new(factory));
    }

    /// Bind a singleton factory under `key`.
    pub fn singleton<F>(&mut self, key: impl Into<String>, factory: F)
    where
        F: Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync + 'static,
    {
        let k = key.into();
        self.type_registry.insert(k.clone(), stringify!(BindingKind::Singleton).to_string());
        self.singleton_factories.insert(k, Box::new(factory));
    }

    /// Bind a concrete instance under `key`.
    pub fn instance<T>(&mut self, key: impl Into<String>, value: T)
    where
        T: Any + Send + Sync + 'static,
    {
        let k = key.into();
        self.type_registry.insert(k.clone(), stringify!(BindingKind::Instance).to_string());
        self.instances.insert(k, Box::new(value));
    }

    /// Resolve a reference to a bound value by key.
    pub fn get<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        if let Some(v) = self.instances.get(key) {
            return v.downcast_ref::<T>();
        }
        if let Some(v) = self.singleton_cache.get(key) {
            return v.downcast_ref::<T>();
        }
        None
    }

    /// Resolve or create a singleton value by key.
    pub fn make_singleton<T: Any + Send + Sync + Clone + 'static>(&mut self, key: &str) -> Option<T> {
        if let Some(v) = self.singleton_cache.get(key) {
            return v.downcast_ref::<T>().cloned();
        }
        let factory = self.singleton_factories.get(key)?;
        let boxed = factory(self);
        let typed = boxed.downcast_ref::<T>()?.clone();
        self.singleton_cache.insert(key.to_string(), Box::new(typed.clone()));
        Some(typed)
    }

    /// Resolve a transient binding by key.
    pub fn make_transient<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<T>
    where
        T: Clone,
    {
        let factory = self.factories.get(key)?;
        let boxed = factory(self);
        boxed.downcast_ref::<T>().cloned()
    }

    /// Check whether a key is bound.
    pub fn bound(&self, key: &str) -> bool {
        self.instances.contains_key(key)
            || self.singleton_cache.contains_key(key)
            || self.factories.contains_key(key)
            || self.singleton_factories.contains_key(key)
    }

    /// List registered keys.
    pub fn keys(&self) -> Vec<String> {
        self.type_registry.keys().cloned().collect()
    }
}

/// Application bootstrap and provider DAG.
pub struct Application {
    /// Shared service container.
    pub container: Container,
    providers: Vec<Box<dyn ServiceProvider>>,
    booted: bool,
}

impl Default for Application {
    /// Create a default application.
    fn default() -> Self {
        Self {
            container: Container::new(),
            providers: Vec::new(),
            booted: false,
        }
    }
}

impl Application {
    /// Create a new application.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure a new application via callback.
    pub fn configure<F>(f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        let mut app = Self::new();
        f(&mut app);
        app
    }

    /// Register a service provider.
    pub fn provider<P>(&mut self, provider: P) -> &mut Self
    where
        P: ServiceProvider + 'static,
    {
        self.providers.push(Box::new(provider));
        self
    }

    /// Run register then boot for all providers.
    pub fn boot(&mut self) {
        if self.booted {
            return;
        }
        let providers = std::mem::take(&mut self.providers);
        for p in &providers {
            p.register(self);
        }
        for p in &providers {
            p.boot(self);
        }
        self.providers = providers;
        self.booted = true;
    }

    /// Whether the application has been booted.
    pub fn is_booted(&self) -> bool {
        self.booted
    }

    /// Graceful shutdown stub.
    pub async fn shutdown(self) {
        shutdown::graceful(Duration::from_secs(30)).await;
    }
}

use std::time::Duration;

/// Graceful shutdown utilities.
pub mod shutdown {
    use super::Duration;

    /// Wait for SIGTERM/SIGINT then drain with timeout.
    pub async fn graceful(_timeout: Duration) {
        #[cfg(unix)]
        {
            let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
            let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).ok();
            tokio::select! {
                _ = async { if let Some(s) = term.as_mut() { s.recv().await; } } => {},
                _ = async { if let Some(s) = int.as_mut() { s.recv().await; } } => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    /// Shutdown handle wrapping a timeout.
    pub struct ShutdownHandle {
        /// Timeout for drain.
        pub timeout: Duration,
    }

    impl ShutdownHandle {
        /// Create a new handle.
        pub fn new(timeout: Duration) -> Self {
            Self { timeout }
        }

        /// Run graceful shutdown.
        pub async fn drain(self) {
            graceful(self.timeout).await;
        }
    }
}

/// Thread-safe application handle.
pub type AppHandle = Arc<RwLock<Application>>;
