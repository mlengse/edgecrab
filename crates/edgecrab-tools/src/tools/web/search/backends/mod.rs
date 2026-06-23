//! Built-in web search backend implementations.

pub mod mock;

#[cfg(feature = "brave")]
pub mod brave;
#[cfg(feature = "exa")]
pub mod exa;
#[cfg(feature = "firecrawl")]
pub mod firecrawl;
#[cfg(feature = "parallel")]
pub mod parallel;
#[cfg(feature = "searxng")]
pub mod searxng;
#[cfg(feature = "tavily")]
pub mod tavily;
#[cfg(feature = "xai")]
pub mod xai;

#[cfg(feature = "brave")]
pub use brave::BraveBackend;
#[cfg(feature = "exa")]
pub use exa::ExaBackend;
#[cfg(feature = "firecrawl")]
pub use firecrawl::FirecrawlBackend;
#[cfg(feature = "parallel")]
pub use parallel::ParallelBackend;
#[cfg(feature = "searxng")]
pub use searxng::SearxngBackend;
#[cfg(feature = "tavily")]
pub use tavily::TavilyBackend;
#[cfg(feature = "xai")]
pub use xai::XaiBackend;
