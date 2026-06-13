//! `web_search` tool — dispatches through [`BackendChain`].

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use edgecrab_types::{ToolError, ToolSchema};

use crate::artifact_spill::{
    SpillConfig, web_search_inline_threshold, web_search_spilled_json, write_artifact_proactive,
};
use crate::registry::{ToolContext, ToolHandler};
use crate::tool_progress_tail::ToolProgressTail;
use crate::tools::web::search::backend_settings::MAX_SEARCH_RESULTS;
use crate::tools::web::search::chain::BackendChain;
use crate::tools::web::search::config::{
    ResolvedChain, SearchOptions, effective_web_search_config, load_web_search_config_from_disk,
    web_search_is_available,
};
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::response::{build_web_search_agent_notes, success_payload};

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
                        "description": "Optional — omit unless required. Unconfigured backends are ignored and the saved chain is used instead."
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

        let cfg = effective_web_search_config(&ctx.config.web_search);
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

        let progress = ToolProgressTail::progress_fn_from_context(ctx);
        let (results, used_backend) = chain
            .search_with_progress(&args.query, opts, progress)
            .await
            .map_err(|e: SearchError| e.into_tool_error())?;

        let fallback_from = if used_backend != primary {
            Some(primary)
        } else {
            None
        };

        let chain_summary = resolved.names.join(" → ");
        let note = build_web_search_agent_notes(
            &used_backend,
            fallback_from.as_deref(),
            resolved.skipped_tool_override.as_deref(),
            &chain_summary,
            &cfg,
        );

        let payload = success_payload(
            &args.query,
            &used_backend,
            fallback_from.as_deref(),
            resolved.skipped_tool_override.as_deref(),
            note.as_deref(),
            &results,
        );

        let spill_config = SpillConfig::from(&ctx.config);
        let inline_threshold = web_search_inline_threshold(&spill_config);
        let json_str = payload.to_string();
        if json_str.len() > inline_threshold
            && let Some(written) = write_artifact_proactive(
                "web_search",
                &json_str,
                &ctx.session_id,
                &ctx.cwd,
                &spill_config,
                None,
            )
        {
            return Ok(web_search_spilled_json(
                &args.query,
                &used_backend,
                fallback_from.as_deref(),
                resolved.skipped_tool_override.as_deref(),
                note.as_deref(),
                &results,
                &written,
            )
            .to_string());
        }

        Ok(json_str)
    }
}

inventory::submit!(&WebSearchTool as &dyn ToolHandler);
