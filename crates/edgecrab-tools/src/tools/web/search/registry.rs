//! Runtime registry for built-in and plugin-registered search backends.

use std::sync::{Arc, OnceLock, RwLock};

use super::backend::WebSearchBackend;
use super::config::ExtractOptions;
use super::content_extract::{ExtractHttpError, RawExtractPage};

static REGISTRY: OnceLock<RwLock<BackendRegistry>> = OnceLock::new();

static TEST_REGISTRY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialize integration tests that mutate the global backend registry.
#[doc(hidden)]
pub fn test_registry_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_REGISTRY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Central map of search backends — plugins call [`register_web_search_backend`].
#[derive(Default)]
pub struct BackendRegistry {
    entries: Vec<Arc<dyn WebSearchBackend>>,
}

impl BackendRegistry {
    fn global() -> &'static RwLock<Self> {
        REGISTRY.get_or_init(|| RwLock::new(Self::register_builtins()))
    }

    fn register_builtins() -> Self {
        let mut reg = Self::default();
        macro_rules! register {
            ($backend:expr) => {
                reg.entries.push(Arc::new($backend));
            };
        }
        #[cfg(feature = "searxng")]
        register!(super::backends::SearxngBackend);
        #[cfg(feature = "brave")]
        register!(super::backends::BraveBackend);
        #[cfg(feature = "ddgs")]
        register!(super::backends::DdgsBackend);
        #[cfg(feature = "firecrawl")]
        register!(super::backends::FirecrawlBackend);
        #[cfg(feature = "tavily")]
        register!(super::backends::TavilyBackend);
        #[cfg(feature = "exa")]
        register!(super::backends::ExaBackend);
        #[cfg(feature = "parallel")]
        register!(super::backends::ParallelBackend);
        #[cfg(feature = "xai")]
        register!(super::backends::XaiBackend);
        reg
    }

    pub fn register(&mut self, backend: Arc<dyn WebSearchBackend>) {
        let name = backend.name().to_string();
        self.entries.retain(|b| b.name() != name);
        self.entries.push(backend);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn WebSearchBackend>> {
        let key = name.trim().to_ascii_lowercase();
        // Aliases for backward compatibility
        let key = match key.as_str() {
            "duckduckgo" | "ddg" | "brave-free" => {
                if key == "brave-free" {
                    "brave"
                } else {
                    "ddgs"
                }
            }
            other => other,
        };
        self.entries.iter().find(|b| b.name() == key).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn WebSearchBackend>> {
        self.entries.clone()
    }

    #[doc(hidden)]
    pub fn reset_for_tests() {
        if let Some(lock) = REGISTRY.get() {
            *lock.write().expect("registry lock") = Self::register_builtins();
        }
    }
}

/// Register a plugin-provided backend (overwrites same name).
pub fn register_web_search_backend(backend: Arc<dyn WebSearchBackend>) {
    BackendRegistry::global()
        .write()
        .expect("registry lock")
        .register(backend);
}

pub fn get_web_search_backend(name: &str) -> Option<Arc<dyn WebSearchBackend>> {
    BackendRegistry::global()
        .read()
        .expect("registry lock")
        .get(name)
}

pub fn list_web_search_backends() -> Vec<Arc<dyn WebSearchBackend>> {
    BackendRegistry::global()
        .read()
        .expect("registry lock")
        .list()
}

/// Hermes-style picker rows for all registered backends (built-ins + plugins).
pub fn list_web_provider_setup_schemas() -> Vec<(String, super::setup_schema::SetupSchema)> {
    list_web_search_backends()
        .into_iter()
        .map(|b| {
            let name = b.name().to_string();
            let schema = b.setup_schema();
            (name, schema)
        })
        .collect()
}

/// Dispatch `web_extract` to a registered backend (Hermes registry extract path).
pub async fn extract_with_backend(
    name: &str,
    url: &str,
    opts: &ExtractOptions,
) -> Result<RawExtractPage, ExtractHttpError> {
    let backend = get_web_search_backend(name).ok_or_else(|| {
        ExtractHttpError::hard(format!("Web backend '{name}' is not registered."))
    })?;
    if !backend.supports_extract() {
        return Err(ExtractHttpError::hard(format!(
            "{name} does not support extract"
        )));
    }
    backend.extract(url, opts).await
}

/// Reset built-in backends after tests register mocks (integration-test helper).
#[doc(hidden)]
pub fn reset_registry_for_tests() {
    BackendRegistry::reset_for_tests();
}
