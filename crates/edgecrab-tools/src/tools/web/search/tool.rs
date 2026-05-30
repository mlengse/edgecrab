//! `web_search` tool — dispatches through [`BackendChain`].

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use edgecrab_types::{ToolError, ToolSchema};

use crate::registry::{ToolContext, ToolHandler};
use crate::tools::web::search::backend_settings::MAX_SEARCH_RESULTS;
use crate::tools::web::search::chain::BackendChain;
use crate::tools::web::search::config::{
    ResolvedChain, SearchOptions, load_web_search_config_from_disk, web_search_is_available,
};
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::response::success_payload;

pub struct WebSearchTool;

#[derive(Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    backend: Option<String>,
}

#[async_trait]
impl ToolHandler for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn toolset(&self) -> &'static str {
        "web"
    }

    fn emoji(&self) -> &'static str {
        "🔍"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "web_search".into(),
            description: "Search the web for information. Returns titles, URLs, and snippets.\n\
                          Backends are chosen automatically from configured API keys — omit \
                          `backend` unless you need a specific provider.\n\
                          Configure via `edgecrab setup web` or web_search.primary in config.yaml."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": format!("Maximum results to return (default: 5, max: {MAX_SEARCH_RESULTS})")
                    },
                    "backend": {
                        "type": "string",
                        "description": "Optional — rarely needed. Auto-selects from configured keys (firecrawl, tavily, searxng, ddgs, …)."
                    }
                },
                "required": ["query"]
            }),
            strict: None,
        }
    }

    fn is_available(&self) -> bool {
        web_search_is_available(&load_web_search_config_from_disk())
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String, ToolError> {
        let args: SearchArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs {
                tool: "web_search".into(),
                message: e.to_string(),
            })?;

        let cfg = ctx.config.web_search.clone();
        let resolved = ResolvedChain::resolve(&cfg, args.backend.as_deref())
            .map_err(|e| e.into_tool_error())?;
        let primary = resolved.names.first().cloned().unwrap_or_default();
        let chain = BackendChain::from_resolved(&resolved).map_err(|e| e.into_tool_error())?;

        let opts = SearchOptions {
            max_results: args.max_results.unwrap_or(5),
            timeout_secs: cfg.timeout_secs,
            backend_override: args.backend.clone(),
            backend_config: Default::default(),
        };

        let (results, used_backend) = chain
            .search(&args.query, opts)
            .await
            .map_err(|e: SearchError| e.into_tool_error())?;

        let fallback_from = if used_backend != primary {
            Some(primary)
        } else {
            None
        };

        let note = if used_backend == "ddgs" {
            Some(
                "DuckDuckGo (ddgs) is the no-key fallback. \
                 For reliable broad search set SEARXNG_URL, BRAVE_API_KEY, or TAVILY_API_KEY."
                    .to_string(),
            )
        } else {
            None
        };

        Ok(success_payload(
            &args.query,
            &used_backend,
            fallback_from.as_deref(),
            note.as_deref(),
            &results,
        )
        .to_string())
    }
}

inventory::submit!(&WebSearchTool as &dyn ToolHandler);
