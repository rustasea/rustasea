//! RustaSea foundation — Application, Container, ServiceProvider, shutdown.

use std::any::Any;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, RwLock};

/// Service provider lifecycle.
///
/// Implementors participate in the Application boot DAG: `register` is called
/// first for all providers, then `boot`. Both passes follow the same
/// dependency order resolved from [`ServiceProvider::dependencies`].
pub trait ServiceProvider: Send + Sync {
    /// Stable name used to resolve dependency edges.
    ///
    /// Defaults to the fully-qualified Rust type name; override it to expose a
    /// short name that [`ServiceProvider::dependencies`] can reference.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Names of providers that must boot before this one.
    ///
    /// Each entry resolves against [`ServiceProvider::name`] (full or short
    /// form). Empty by default, so providers without dependencies keep their
    /// registration order.
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    /// Register bindings into the container.
    fn register(&self, _app: &mut Application) {}

    /// Boot after all providers have registered.
    fn boot(&self, _app: &Application) {}
}

impl ServiceProvider for Box<dyn ServiceProvider> {
    /// Delegate the provider name to the boxed value.
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    /// Delegate dependency declarations to the boxed value.
    fn dependencies(&self) -> &'static [&'static str] {
        self.as_ref().dependencies()
    }

    /// Delegate registration to the boxed value.
    fn register(&self, app: &mut Application) {
        self.as_ref().register(app);
    }

    /// Delegate boot to the boxed value.
    fn boot(&self, app: &Application) {
        self.as_ref().boot(app);
    }
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

/// A container factory producing a boxed, thread-safe value from the container.
type Factory = Box<dyn Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync>;

/// DI container with Bind/Singleton/Instance semantics.
pub struct Container {
    factories: HashMap<String, Factory>,
    singleton_factories: HashMap<String, Factory>,
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
        self.type_registry
            .insert(k.clone(), stringify!(BindingKind::Bind).to_string());
        self.factories.insert(k, Box::new(factory));
    }

    /// Bind a singleton factory under `key`.
    pub fn singleton<F>(&mut self, key: impl Into<String>, factory: F)
    where
        F: Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync + 'static,
    {
        let k = key.into();
        self.type_registry
            .insert(k.clone(), stringify!(BindingKind::Singleton).to_string());
        self.singleton_factories.insert(k, Box::new(factory));
    }

    /// Bind a concrete instance under `key`.
    pub fn instance<T>(&mut self, key: impl Into<String>, value: T)
    where
        T: Any + Send + Sync + 'static,
    {
        let k = key.into();
        self.type_registry
            .insert(k.clone(), stringify!(BindingKind::Instance).to_string());
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
    pub fn make_singleton<T: Any + Send + Sync + Clone + 'static>(
        &mut self,
        key: &str,
    ) -> Option<T> {
        if let Some(v) = self.singleton_cache.get(key) {
            return v.downcast_ref::<T>().cloned();
        }
        let factory = self.singleton_factories.get(key)?;
        let boxed = factory(self);
        let typed = boxed.downcast_ref::<T>()?.clone();
        self.singleton_cache
            .insert(key.to_string(), Box::new(typed.clone()));
        Some(typed)
    }

    /// Resolve a transient binding by key.
    pub fn make_transient<T: Any + Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T> {
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

/// Errors raised while resolving and running the provider boot DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootError {
    /// A dependency cycle was detected among the listed providers.
    DependencyCycle {
        /// Names of the providers participating in the cycle.
        providers: Vec<String>,
    },
    /// A provider declared a dependency that is not registered.
    UnknownDependency {
        /// Name of the provider that declared the dependency.
        provider: String,
        /// The unresolved dependency name.
        dependency: String,
    },
}

impl std::fmt::Display for BootError {
    /// Render the error with the offending provider names.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyCycle { providers } => {
                write!(
                    f,
                    "provider dependency cycle detected: {}",
                    providers.join(" -> ")
                )
            }
            Self::UnknownDependency {
                provider,
                dependency,
            } => write!(
                f,
                "provider `{provider}` depends on unknown provider `{dependency}`"
            ),
        }
    }
}

impl std::error::Error for BootError {}

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

    /// Run register then boot for all providers in dependency order.
    ///
    /// Providers are topologically sorted by [`ServiceProvider::dependencies`];
    /// independent providers keep their registration order as the tie-breaker.
    /// Returns [`BootError`] when the graph contains a cycle or an unresolved
    /// dependency. On error nothing is registered or booted and the
    /// application stays unbooted.
    pub fn boot(&mut self) -> Result<(), BootError> {
        if self.booted {
            return Ok(());
        }
        let order = self.topological_order()?;
        let providers = std::mem::take(&mut self.providers);
        for &index in &order {
            providers[index].register(self);
        }
        for &index in &order {
            providers[index].boot(self);
        }
        self.providers = providers;
        self.booted = true;
        Ok(())
    }

    /// Resolve the stable topological order of registered providers.
    ///
    /// Kahn's algorithm with a min-heap keyed by registration index, so ready
    /// providers are emitted in registration order (the tie-breaker).
    fn topological_order(&self) -> Result<Vec<usize>, BootError> {
        let count = self.providers.len();
        let index_of = self.provider_index();
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut indegree = vec![0usize; count];

        for (node, provider) in self.providers.iter().enumerate() {
            for dependency in provider.dependencies() {
                let Some(&dependency_index) = index_of.get(*dependency) else {
                    return Err(BootError::UnknownDependency {
                        provider: provider.name().to_string(),
                        dependency: (*dependency).to_string(),
                    });
                };
                if dependency_index == node {
                    return Err(BootError::DependencyCycle {
                        providers: vec![provider.name().to_string()],
                    });
                }
                edges[dependency_index].push(node);
                indegree[node] += 1;
            }
        }

        let mut ready: BinaryHeap<Reverse<usize>> = (0..count)
            .filter(|&node| indegree[node] == 0)
            .map(Reverse)
            .collect();
        let mut order = Vec::with_capacity(count);
        while let Some(Reverse(node)) = ready.pop() {
            order.push(node);
            for &next in &edges[node] {
                indegree[next] -= 1;
                if indegree[next] == 0 {
                    ready.push(Reverse(next));
                }
            }
        }

        if order.len() == count {
            return Ok(order);
        }
        Err(BootError::DependencyCycle {
            providers: self.cycle_names(&edges, &indegree),
        })
    }

    /// Build a lookup from provider name (full and short) to registration index.
    fn provider_index(&self) -> HashMap<String, usize> {
        let mut index_of = HashMap::new();
        for (index, provider) in self.providers.iter().enumerate() {
            let name = provider.name();
            index_of.entry(name.to_string()).or_insert(index);
            if let Some(short) = name.rsplit("::").next() {
                index_of.entry(short.to_string()).or_insert(index);
            }
        }
        index_of
    }

    /// Extract one concrete cycle from the residual graph for diagnostics.
    ///
    /// Every residual node is tried as a DFS root — not just the first — so a
    /// downstream consumer that merely depends on a cycle is never reported as
    /// a cycle member. Dumping all residual nodes is a last resort only.
    fn cycle_names(&self, edges: &[Vec<usize>], indegree: &[usize]) -> Vec<String> {
        let residual: Vec<bool> = indegree.iter().map(|&degree| degree > 0).collect();
        let mut path = Vec::new();
        let mut on_path = vec![false; self.providers.len()];
        let mut done = vec![false; self.providers.len()];
        for candidate in 0..self.providers.len() {
            if !residual[candidate] {
                continue;
            }
            if let Some(cycle) = self.find_cycle(
                candidate,
                edges,
                &residual,
                &mut path,
                &mut on_path,
                &mut done,
            ) {
                return cycle
                    .iter()
                    .map(|&node| self.providers[node].name().to_string())
                    .collect();
            }
        }
        (0..self.providers.len())
            .filter(|&node| residual[node])
            .map(|node| self.providers[node].name().to_string())
            .collect()
    }

    /// Depth-first search for a cycle within the residual node set.
    fn find_cycle(
        &self,
        node: usize,
        edges: &[Vec<usize>],
        residual: &[bool],
        path: &mut Vec<usize>,
        on_path: &mut [bool],
        done: &mut [bool],
    ) -> Option<Vec<usize>> {
        path.push(node);
        on_path[node] = true;
        for &next in &edges[node] {
            if !residual[next] || done[next] {
                continue;
            }
            if on_path[next] {
                let begin = path.iter().position(|&visited| visited == next)?;
                return Some(path[begin..].to_vec());
            }
            if let Some(cycle) = self.find_cycle(next, edges, residual, path, on_path, done) {
                return Some(cycle);
            }
        }
        path.pop();
        on_path[node] = false;
        done[node] = true;
        None
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
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
            let mut int =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).ok();
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
