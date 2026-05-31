//! Metasearch orchestration — Python `DDGS.text()` parity (9.0.0: Bing-only `auto`).

use std::time::{Duration, Instant};

use super::engines;
use super::selection;
use super::settings::{DdgsEngine, DdgsSettings};
use super::transport::DdgsSession;
use crate::tools::web::search::backend::SearchResult;
use crate::tools::web::search::config::SearchOptions;
use crate::tools::web::search::error::SearchError;

/// Wall-clock budget — Python uses `self.timeout` per HTTP call; cap total probe time the same way.
pub fn metasearch_budget(timeout_secs: u64) -> Duration {
    Duration::from_secs(timeout_secs.max(3))
}

fn budget_exhausted(start: Instant, budget: Duration) -> bool {
    start.elapsed() >= budget
}

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

/// Run metasearch — Python contract: try engines in order, **return on first success** (even empty).
pub async fn search_text(
    query: &str,
    opts: &SearchOptions,
    settings: &DdgsSettings,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let max = opts.max_results();
    let engines_list = settings.engine_order();
    let mut hard_failures: Vec<(DdgsEngine, SearchError)> = Vec::new();

    let budget = metasearch_budget(opts.timeout_secs);
    let started = Instant::now();
    let mut session = DdgsSession::new(opts.timeout_secs)?;

    for (idx, engine) in engines_list.iter().copied().enumerate() {
        if budget_exhausted(started, budget) {
            tracing::debug!(backend, "ddgs metasearch budget exhausted");
            break;
        }
        if idx > 0 {
            tokio::time::sleep(Duration::from_millis(750)).await;
        }

        let mut attempts = settings.max_retries.saturating_add(1);
        while attempts > 0 {
            attempts -= 1;
            if attempts < settings.max_retries {
                tokio::time::sleep(Duration::from_millis(500)).await;
                session = DdgsSession::refresh(opts.timeout_secs)?;
            }

            match engines::run_engine(&mut session, engine, query, settings, max, backend).await {
                Ok(results) => {
                    let selected =
                        selection::select_results(settings.selection_mode, query, results, max);
                    tracing::debug!(backend, count = selected.len(), "ddgs metasearch finished");
                    return Ok(selected);
                }
                Err(err) if err.is_fallback_eligible() => {
                    tracing::debug!(
                        backend,
                        engine = engine.label(),
                        error = %err,
                        "ddgs engine failed"
                    );
                    if attempts > 0 {
                        continue;
                    }
                    hard_failures.push((engine, err));
                    break;
                }
                Err(err) => return Err(err),
            }
        }
    }

    if hard_failures.is_empty() {
        tracing::debug!(backend, "ddgs metasearch finished with empty pool");
        return Ok(Vec::new());
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

    #[test]
    fn metasearch_budget_uses_full_timeout() {
        assert_eq!(metasearch_budget(10).as_secs(), 10);
        assert_eq!(metasearch_budget(2).as_secs(), 3);
    }
}
