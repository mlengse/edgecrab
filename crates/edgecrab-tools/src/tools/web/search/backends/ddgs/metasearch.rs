//! Metasearch orchestration — probe all engines, merge, rank (Python `DDGS.text()` quality parity+).

use std::time::Duration;

use super::engines;
use super::rank;
use super::relevance;
use super::settings::{DdgsEngine, DdgsSettings};
use super::transport::DdgsSession;
use crate::tools::web::search::backend::SearchResult;
use crate::tools::web::search::config::SearchOptions;
use crate::tools::web::search::error::SearchError;

fn aggregate_engine_failures(failures: &[(DdgsEngine, SearchError)]) -> SearchError {
    if failures.is_empty() {
        return SearchError::hard(
            "ddgs",
            "All metasearch engines failed. Set DDGS_PROXY or configure a paid backend via /web.",
        );
    }
    if failures.len() == 1 {
        return failures[0].1.clone();
    }
    let summary = failures
        .iter()
        .map(|(engine, err)| format!("{}: {}", engine.label(), err.message))
        .collect::<Vec<_>>()
        .join("; ");
    SearchError::server("ddgs", 503, summary)
}

/// Run metasearch: collect from Bing + DDG HTML + DDG lite, then rank the merged pool.
pub async fn search_text(
    query: &str,
    opts: &SearchOptions,
    settings: &DdgsSettings,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let max = opts.max_results();
    let engines_list = settings.engine_order();
    let mut hard_failures: Vec<(DdgsEngine, SearchError)> = Vec::new();
    let mut pool: Vec<SearchResult> = Vec::new();
    let mut scrape_reached = false;

    let mut session = DdgsSession::new(opts.timeout_secs)?;
    session.warm_up(backend).await?;

    'engines: for (idx, engine) in engines_list.iter().copied().enumerate() {
        if idx > 0 {
            tokio::time::sleep(Duration::from_millis(750)).await;
        }

        if rank::pool_is_satisfied(query, &pool, max) {
            tracing::debug!(backend, count = pool.len(), "ddgs pool satisfied, skipping engines");
            break;
        }

        for retry in 0..=settings.max_retries.min(1) {
            if retry > 0 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                session = DdgsSession::refresh(opts.timeout_secs)?;
                session.warm_up(backend).await?;
            }

            let mut transient_err: Option<SearchError> = None;
            let mut engine_had_hits = false;

            for variant in relevance::query_variants(query) {
                match engines::run_engine(&mut session, engine, &variant, settings, max, backend)
                    .await
                {
                    Ok(results) if !results.is_empty() => {
                        scrape_reached = true;
                        engine_had_hits = true;
                        rank::extend_pool(&mut pool, results);
                        tracing::debug!(
                            backend,
                            engine = engine.label(),
                            variant = %variant,
                            pool = pool.len(),
                            "ddgs engine contributed results"
                        );
                        if rank::pool_is_satisfied(query, &pool, max) {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(err) if err.is_fallback_eligible() => {
                        tracing::debug!(
                            backend,
                            engine = engine.label(),
                            variant = %variant,
                            error = %err,
                            "ddgs engine failed, trying next"
                        );
                        transient_err = Some(err);
                        break;
                    }
                    Err(err) => return Err(err),
                }
            }

            if rank::pool_is_satisfied(query, &pool, max) {
                break 'engines;
            }

            if let Some(err) = transient_err {
                if retry < settings.max_retries.min(1) {
                    continue;
                }
                hard_failures.push((engine, err));
                break;
            }
            if !engine_had_hits {
                // Engine responded but returned zero parseable hits — not a hard failure.
            }
            break;
        }
    }

    if scrape_reached {
        let selected = rank::rank_and_select(query, pool, max);
        tracing::debug!(
            backend,
            count = selected.len(),
            "ddgs metasearch finished"
        );
        return Ok(selected);
    }

    Err(aggregate_engine_failures(&hard_failures))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::error::SearchErrorKind;

    #[test]
    fn aggregate_engine_failures_lists_all_engines() {
        let failures = vec![
            (
                DdgsEngine::Bing,
                SearchError::server("ddgs", 503, "Bing blocked this request (bot challenge)."),
            ),
            (
                DdgsEngine::Html,
                SearchError::server(
                    "ddgs",
                    503,
                    "DuckDuckGo HTML blocked this request (bot challenge).",
                ),
            ),
        ];
        let err = aggregate_engine_failures(&failures);
        assert!(matches!(err.kind, SearchErrorKind::Server(503)));
        assert!(err.message.contains("Bing:"));
        assert!(err.message.contains("DuckDuckGo HTML:"));
    }
}
