//! HTTP transport — cookie jar, pacing, sanitized errors (single responsibility).

use std::time::{Duration, Instant};

use super::error::{map_engine_http_status, map_transport_error};
use super::settings::DdgsEngine;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_chrome_client_with_headers, validate_search_url};

/// One `DDGS()` client instance — reused across engine attempts in a single search.
pub struct DdgsSession {
    client: wreq::Client,
    last_request: Instant,
}

impl DdgsSession {
    pub fn new(timeout_secs: u64) -> Result<Self, SearchError> {
        Ok(Self {
            client: build_chrome_client_with_headers(timeout_secs, None, None)?,
            last_request: Instant::now() - Duration::from_secs(30),
        })
    }

    /// Fresh client with a new TLS/UA fingerprint (Python `DDGS()` per retry).
    pub fn refresh(timeout_secs: u64) -> Result<Self, SearchError> {
        Self::new(timeout_secs)
    }

    /// Best-effort cookie seed — Python `primp` + DDG expect a prior visit to duckduckgo.com.
    pub async fn warm_up(&mut self, backend: &str) -> Result<(), SearchError> {
        let _ = self
            .get(
                super::settings::DdgsEngine::Html,
                backend,
                "https://duckduckgo.com/",
                &[],
                None,
            )
            .await;
        Ok(())
    }

    async fn pace(&mut self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < Duration::from_secs(20) && elapsed > Duration::ZERO {
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
        self.last_request = Instant::now();
    }

    pub async fn get(
        &mut self,
        engine: DdgsEngine,
        backend: &str,
        url: &str,
        query: &[(&str, &str)],
        cookie: Option<&str>,
    ) -> Result<String, SearchError> {
        self.request(engine, backend, "GET", url, None, Some(query), None, cookie)
            .await
    }

    pub async fn post_form(
        &mut self,
        engine: DdgsEngine,
        backend: &str,
        url: &str,
        referer: &str,
        form: &[(&str, &str)],
    ) -> Result<String, SearchError> {
        self.request(engine, backend, "POST", url, Some(referer), None, Some(form), None)
            .await
    }

    async fn request(
        &mut self,
        engine: DdgsEngine,
        backend: &str,
        method: &str,
        url: &str,
        referer: Option<&str>,
        query: Option<&[(&str, &str)]>,
        form: Option<&[(&str, &str)]>,
        cookie: Option<&str>,
    ) -> Result<String, SearchError> {
        validate_search_url(url)?;
        self.pace().await;

        let mut req = match method {
            "POST" => self.client.post(url),
            _ => self.client.get(url),
        };

        if let Some(r) = referer {
            req = req.header("Referer", r).header("Sec-Fetch-User", "?1");
        }
        if let Some(q) = query {
            req = req.query(q);
        }
        if let Some(f) = form {
            req = req.form(f);
        }
        if let Some(c) = cookie {
            req = req.header("Cookie", c);
        }

        let resp = req.send().await.map_err(|e| {
            map_transport_error(backend, engine, &e.to_string())
        })?;

        let code = resp.status().as_u16();
        // DDG HTML/lite often return HTTP 202 with a full HTML body (results or bot challenge).
        if code == 200 || code == 202 {
            return resp
                .text()
                .await
                .map_err(|e| map_transport_error(backend, engine, &e.to_string()));
        }
        Err(map_engine_http_status(backend, engine, code))
    }
}
