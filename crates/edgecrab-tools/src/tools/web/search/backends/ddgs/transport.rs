//! HTTP transport — primp random impersonation, referer chain, pacing, DDGS_PROXY (Python parity).

use std::time::{Duration, Instant};

use super::error::{map_engine_http_status, map_transport_error};
use super::fingerprint;
use super::settings::DdgsEngine;
use crate::tools::web::search::error::SearchError;

fn resolve_ddgs_proxy() -> Option<String> {
    std::env::var("DDGS_PROXY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(expand_proxy_tb_alias)
}

/// Python `utils._expand_proxy_tb_alias` — `tb` → Tor Browser SOCKS5.
fn expand_proxy_tb_alias(proxy: String) -> String {
    if proxy == "tb" {
        "socks5://127.0.0.1:9150".into()
    } else {
        proxy
    }
}

struct HttpRequest<'a> {
    engine: DdgsEngine,
    backend: &'a str,
    method: &'a str,
    url: &'a str,
    referer: Option<&'a str>,
    query: Option<&'a [(&'a str, &'a str)]>,
    form: Option<&'a [(&'a str, &'a str)]>,
    cookie: Option<&'a str>,
}

/// Build the outbound URL including query pairs (primp referer chain uses request URL).
fn build_request_url(base: &str, query: Option<&[(&str, &str)]>) -> Result<String, SearchError> {
    let Some(params) = query.filter(|p| !p.is_empty()) else {
        return Ok(base.to_string());
    };
    let mut url = url::Url::parse(base)
        .map_err(|e| SearchError::hard("ddgs", format!("Invalid engine URL {base}: {e}")))?;
    {
        let mut pairs = url.query_pairs_mut();
        for (k, v) in params {
            pairs.append_pair(k, v);
        }
    }
    Ok(url.to_string())
}

/// One `DDGS()` client — primp random profile + `referer=True`, reused until [`Self::refresh`].
pub struct DdgsSession {
    client: wreq::Client,
    last_request: Instant,
    profile_id: &'static str,
    /// Python `primp.Client(referer=True)` — previous request URL for the Referer header.
    last_request_url: Option<String>,
}

impl DdgsSession {
    pub fn new(timeout_secs: u64) -> Result<Self, SearchError> {
        let profile = fingerprint::resolve_profile_from_env();
        let profile_id = profile.id();
        let client = fingerprint::build_ddgs_client(timeout_secs, resolve_ddgs_proxy(), profile)
            .map_err(|e| SearchError::hard("ddgs", e))?;
        tracing::debug!(profile = profile_id, "ddgs session fingerprint");
        Ok(Self {
            client,
            last_request: Instant::now() - Duration::from_secs(30),
            profile_id,
            last_request_url: None,
        })
    }

    /// Fresh client + new random profile (Python new `DDGS()` / primp retry).
    pub fn refresh(timeout_secs: u64) -> Result<Self, SearchError> {
        Self::new(timeout_secs)
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
        self.request(HttpRequest {
            engine,
            backend,
            method: "GET",
            url,
            referer: None,
            query: Some(query),
            form: None,
            cookie,
        })
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
        self.request(HttpRequest {
            engine,
            backend,
            method: "POST",
            url,
            referer: Some(referer),
            query: None,
            form: Some(form),
            cookie: None,
        })
        .await
    }

    async fn request(&mut self, req: HttpRequest<'_>) -> Result<String, SearchError> {
        use wreq::header::REFERER;

        let HttpRequest {
            engine,
            backend,
            method,
            url,
            referer,
            query,
            form,
            cookie,
        } = req;
        validate_search_url(url)?;
        self.pace().await;

        let request_url = build_request_url(url, query)?;
        let effective_referer = referer.or(self.last_request_url.as_deref());
        tracing::trace!(
            profile = self.profile_id,
            method,
            url = request_url,
            referer = ?effective_referer,
            "ddgs request"
        );

        let mut req = match method {
            "POST" => self.client.post(url),
            _ => self.client.get(url),
        };

        if method == "GET" {
            req = req.header("Upgrade-Insecure-Requests", "1");
        }
        let fetch_site = if effective_referer.is_some() {
            "same-origin"
        } else {
            "none"
        };
        req = req
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", fetch_site)
            .header("Sec-Fetch-User", "?1");

        if let Some(r) = effective_referer {
            req = req.header(REFERER, r);
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

        let resp = req
            .send()
            .await
            .map_err(|e| map_transport_error(backend, engine, &e.to_string()))?;

        let code = resp.status().as_u16();
        let ok = match engine {
            DdgsEngine::Bing => code == 200,
            DdgsEngine::Html | DdgsEngine::Lite => code == 200 || code == 202,
        };
        if ok {
            self.last_request_url = Some(request_url);
            return resp
                .text()
                .await
                .map_err(|e| map_transport_error(backend, engine, &e.to_string()));
        }
        Err(map_engine_http_status(backend, engine, code))
    }
}

use crate::tools::web::search::http::validate_search_url;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_proxy_tb_alias_matches_python() {
        assert_eq!(
            expand_proxy_tb_alias("tb".into()),
            "socks5://127.0.0.1:9150"
        );
        assert_eq!(
            expand_proxy_tb_alias("http://127.0.0.1:8080".into()),
            "http://127.0.0.1:8080"
        );
    }

    #[test]
    fn build_request_url_appends_query() {
        let url = build_request_url(
            "https://www.bing.com/search",
            Some(&[("q", "rust"), ("first", "11")]),
        )
        .expect("url");
        assert!(url.contains("q=rust"));
        assert!(url.contains("first=11"));
    }

    #[test]
    fn session_picks_valid_profile() {
        let _session = DdgsSession::new(10).expect("session");
    }
}
