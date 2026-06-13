//! Browser TLS/UA profiles — Python `primp.Client(impersonate="random", impersonate_os="random")` parity.
//!
//! primp uses curl-impersonate presets (Chrome / Edge / Firefox / Safari × OS). We mirror that
//! with wreq `EmulationProvider` (Apache-2.0 — no GPL `wreq-util`).

use wreq::{
    Client, EmulationProvider, SslCurve,
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT},
    tls::{AlpnProtos, AlpsProtos, TlsConfig, TlsVersion},
};

/// primp `IMPERSONATE_OS` — browser OS dimension (independent of browser id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImpersonateOs {
    Macos,
    Windows,
    Linux,
}

impl ImpersonateOs {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "macos" | "mac" => Some(Self::Macos),
            "windows" | "win" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            "random" | "" => None,
            _ => None,
        }
    }
}

/// primp `IMPERSONATE` subset — browser × OS variants in the random pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImpersonateProfile {
    Chrome131Mac,
    Chrome131Win,
    Chrome131Linux,
    Chrome133Mac,
    Chrome133Win,
    Chrome128Mac,
    Chrome124Win,
    Edge131Win,
    Edge127Mac,
    Firefox135Mac,
    Firefox135Linux,
    Firefox133Mac,
    Firefox128Win,
    Safari18Mac,
    Safari17Mac,
}

impl ImpersonateProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Chrome131Mac | Self::Chrome131Win | Self::Chrome131Linux => "chrome_131",
            Self::Chrome133Mac | Self::Chrome133Win => "chrome_133",
            Self::Chrome128Mac => "chrome_128",
            Self::Chrome124Win => "chrome_124",
            Self::Edge131Win => "edge_131",
            Self::Edge127Mac => "edge_127",
            Self::Firefox135Mac | Self::Firefox135Linux => "firefox_135",
            Self::Firefox133Mac => "firefox_133",
            Self::Firefox128Win => "firefox_128",
            Self::Safari18Mac => "safari_18",
            Self::Safari17Mac => "safari_17",
        }
    }

    pub fn user_agent(self) -> &'static str {
        match self {
            Self::Chrome131Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
            }
            Self::Chrome131Win => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
            }
            Self::Chrome131Linux => {
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
            }
            Self::Chrome133Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"
            }
            Self::Chrome133Win => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"
            }
            Self::Chrome128Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"
            }
            Self::Chrome124Win => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            }
            Self::Edge131Win => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0"
            }
            Self::Edge127Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36 Edg/127.0.0.0"
            }
            Self::Firefox135Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:135.0) Gecko/20100101 Firefox/135.0"
            }
            Self::Firefox135Linux => {
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0"
            }
            Self::Firefox133Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:133.0) Gecko/20100101 Firefox/133.0"
            }
            Self::Firefox128Win => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0"
            }
            Self::Safari18Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15"
            }
            Self::Safari17Mac => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15"
            }
        }
    }

    fn tls_family(self) -> TlsFamily {
        match self {
            Self::Firefox135Mac
            | Self::Firefox135Linux
            | Self::Firefox133Mac
            | Self::Firefox128Win => TlsFamily::Firefox,
            Self::Safari18Mac | Self::Safari17Mac => TlsFamily::Safari,
            _ => TlsFamily::Chromium,
        }
    }

    fn is_chromium_branded(self) -> bool {
        matches!(
            self,
            Self::Chrome131Mac
                | Self::Chrome131Win
                | Self::Chrome131Linux
                | Self::Chrome133Mac
                | Self::Chrome133Win
                | Self::Chrome128Mac
                | Self::Chrome124Win
        )
    }

    fn is_edge(self) -> bool {
        matches!(self, Self::Edge131Win | Self::Edge127Mac)
    }

    pub fn impersonate_os(self) -> ImpersonateOs {
        match self {
            Self::Chrome131Win
            | Self::Chrome133Win
            | Self::Chrome124Win
            | Self::Edge131Win
            | Self::Firefox128Win => ImpersonateOs::Windows,
            Self::Chrome131Linux | Self::Firefox135Linux => ImpersonateOs::Linux,
            _ => ImpersonateOs::Macos,
        }
    }

    fn accept_header(self) -> &'static str {
        match self.tls_family() {
            TlsFamily::Firefox => {
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
            }
            TlsFamily::Safari => "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            TlsFamily::Chromium => {
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
            }
        }
    }

    /// Chromium `sec-ch-ua` — omitted for Firefox/Safari (Python primp curl-impersonate contract).
    fn sec_ch_ua(self) -> Option<&'static str> {
        if self.is_chromium_branded() {
            let v = match self {
                Self::Chrome133Mac | Self::Chrome133Win => "133",
                Self::Chrome128Mac => "128",
                Self::Chrome124Win => "124",
                _ => "131",
            };
            return Some(match v {
                "133" => r#""Google Chrome";v="133", "Chromium";v="133", "Not_A Brand";v="24""#,
                "128" => r#""Google Chrome";v="128", "Chromium";v="128", "Not_A Brand";v="24""#,
                "124" => r#""Google Chrome";v="124", "Chromium";v="124", "Not_A Brand";v="99""#,
                _ => r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#,
            });
        }
        if self.is_edge() {
            let v = if matches!(self, Self::Edge127Mac) {
                "127"
            } else {
                "131"
            };
            return Some(match v {
                "127" => r#""Microsoft Edge";v="127", "Chromium";v="127", "Not_A Brand";v="24""#,
                _ => r#""Microsoft Edge";v="131", "Chromium";v="131", "Not_A Brand";v="24""#,
            });
        }
        None
    }

    fn sec_ch_ua_platform(self) -> Option<&'static str> {
        if !self.is_chromium_branded() && !self.is_edge() {
            return None;
        }
        Some(match self {
            Self::Chrome131Win
            | Self::Chrome133Win
            | Self::Chrome124Win
            | Self::Edge131Win
            | Self::Firefox128Win => "\"Windows\"",
            Self::Chrome131Linux | Self::Firefox135Linux => "\"Linux\"",
            _ => "\"macOS\"",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TlsFamily {
    Chromium,
    Firefox,
    Safari,
}

const PRIMP_RANDOM_POOL: &[ImpersonateProfile] = &[
    ImpersonateProfile::Chrome131Mac,
    ImpersonateProfile::Chrome131Win,
    ImpersonateProfile::Chrome131Linux,
    ImpersonateProfile::Chrome133Mac,
    ImpersonateProfile::Chrome133Win,
    ImpersonateProfile::Chrome128Mac,
    ImpersonateProfile::Chrome124Win,
    ImpersonateProfile::Edge131Win,
    ImpersonateProfile::Edge127Mac,
    ImpersonateProfile::Firefox135Mac,
    ImpersonateProfile::Firefox135Linux,
    ImpersonateProfile::Firefox133Mac,
    ImpersonateProfile::Firefox128Win,
    ImpersonateProfile::Safari18Mac,
    ImpersonateProfile::Safari17Mac,
];

fn pick_random_from(candidates: &[ImpersonateProfile]) -> ImpersonateProfile {
    if candidates.is_empty() {
        return pick_random_profile();
    }
    use rand::Rng;
    candidates[rand::rng().random_range(0..candidates.len())]
}

fn pool_candidates(browser_id: Option<&str>, os: Option<ImpersonateOs>) -> Vec<ImpersonateProfile> {
    PRIMP_RANDOM_POOL
        .iter()
        .copied()
        .filter(|p| browser_id.is_none_or(|id| p.id() == id))
        .filter(|p| os.is_none_or(|o| p.impersonate_os() == o))
        .collect()
}

fn pick_random_matching_id(id: &str, os: Option<ImpersonateOs>) -> ImpersonateProfile {
    pick_random_from(&pool_candidates(Some(id), os))
}

/// Resolve profile from env — `DDGS_IMPERSONATE`, `DDGS_IMPERSONATE_OS` (primp parity).
pub fn resolve_profile_from_env() -> ImpersonateProfile {
    let browser = std::env::var("DDGS_IMPERSONATE").ok();
    let os = std::env::var("DDGS_IMPERSONATE_OS")
        .ok()
        .and_then(|s| ImpersonateOs::parse(&s));
    resolve_profile_with(browser.as_deref(), os)
}

/// Resolve profile — browser pin + optional OS filter; default = primp `random`.
pub fn resolve_profile(env_override: Option<&str>) -> ImpersonateProfile {
    resolve_profile_with(env_override, None)
}

pub fn resolve_profile_with(
    browser_override: Option<&str>,
    os_override: Option<ImpersonateOs>,
) -> ImpersonateProfile {
    if let Some(raw) = browser_override.map(str::trim).filter(|s| !s.is_empty()) {
        let key = raw.to_ascii_lowercase().replace('-', "_");
        if PRIMP_RANDOM_POOL.iter().any(|p| p.id() == key) {
            return pick_random_matching_id(&key, os_override);
        }
        if key.starts_with("chrome_") {
            return pick_random_matching_id("chrome_131", os_override);
        }
        if key.starts_with("edge_") {
            return pick_random_matching_id("edge_131", os_override);
        }
        if key.starts_with("firefox_") {
            return pick_random_matching_id("firefox_135", os_override);
        }
        if key.starts_with("safari_") {
            return pick_random_matching_id("safari_18", os_override);
        }
    }
    pick_random_from(&pool_candidates(None, os_override))
}

pub fn pick_random_profile() -> ImpersonateProfile {
    pick_random_from(&pool_candidates(None, None))
}

fn chromium_tls() -> TlsConfig {
    TlsConfig::builder()
        .min_tls_version(TlsVersion::TLS_1_2)
        .max_tls_version(TlsVersion::TLS_1_3)
        .cipher_list(
            "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:\
             TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256:TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256:\
             TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384:TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384:\
             TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256:TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256:\
             TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA:TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA:\
             TLS_RSA_WITH_AES_128_GCM_SHA256:TLS_RSA_WITH_AES_256_GCM_SHA384:\
             TLS_RSA_WITH_AES_128_CBC_SHA:TLS_RSA_WITH_AES_256_CBC_SHA",
        )
        .sigalgs_list(
            "ecdsa_secp256r1_sha256:rsa_pss_rsae_sha256:rsa_pkcs1_sha256:\
             ecdsa_secp384r1_sha384:rsa_pss_rsae_sha384:rsa_pkcs1_sha384:\
             rsa_pss_rsae_sha512:rsa_pkcs1_sha512",
        )
        .curves(vec![
            SslCurve::X25519,
            SslCurve::SECP256R1,
            SslCurve::SECP384R1,
        ])
        .alpn_protos(AlpnProtos::ALL)
        .alps_protos(AlpsProtos::HTTP2)
        .grease_enabled(true)
        .permute_extensions(true)
        .enable_ech_grease(true)
        .pre_shared_key(true)
        .enable_ocsp_stapling(true)
        .build()
}

fn firefox_tls() -> TlsConfig {
    TlsConfig::builder()
        .min_tls_version(TlsVersion::TLS_1_2)
        .max_tls_version(TlsVersion::TLS_1_3)
        .cipher_list(
            "TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256:TLS_AES_256_GCM_SHA384:\
             TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256:TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256:\
             TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256:TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256:\
             TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384:TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384:\
             TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA:TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA:\
             TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA:TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA:\
             TLS_RSA_WITH_AES_128_GCM_SHA256:TLS_RSA_WITH_AES_256_GCM_SHA384:\
             TLS_RSA_WITH_AES_128_CBC_SHA:TLS_RSA_WITH_AES_256_CBC_SHA",
        )
        .sigalgs_list(
            "ecdsa_secp256r1_sha256:ecdsa_secp384r1_sha384:ecdsa_secp521r1_sha512:\
             rsa_pss_rsae_sha256:rsa_pss_rsae_sha384:rsa_pss_rsae_sha512:\
             rsa_pkcs1_sha256:rsa_pkcs1_sha384:rsa_pkcs1_sha512:ecdsa_sha1:rsa_pkcs1_sha1",
        )
        .alpn_protos(AlpnProtos::ALL)
        .alps_protos(AlpsProtos::HTTP2)
        .grease_enabled(true)
        .permute_extensions(true)
        .enable_ech_grease(true)
        .pre_shared_key(true)
        .build()
}

fn safari_tls() -> TlsConfig {
    chromium_tls()
}

fn tls_for(family: TlsFamily) -> TlsConfig {
    match family {
        TlsFamily::Chromium => chromium_tls(),
        TlsFamily::Firefox => firefox_tls(),
        TlsFamily::Safari => safari_tls(),
    }
}

fn default_headers(profile: ImpersonateProfile) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(profile.user_agent()));
    headers.insert(ACCEPT, HeaderValue::from_static(profile.accept_header()));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate, br"),
    );
    if let Some(ua) = profile.sec_ch_ua() {
        headers.insert("sec-ch-ua", HeaderValue::from_static(ua));
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        if let Some(platform) = profile.sec_ch_ua_platform() {
            headers.insert("sec-ch-ua-platform", HeaderValue::from_static(platform));
        }
    }
    headers
}

/// Build wreq client with optional Referer / UA overrides (extract/crawl legacy path).
pub fn build_wreq_client(
    timeout_secs: u64,
    profile: ImpersonateProfile,
    referer: Option<&str>,
    user_agent: Option<&str>,
    proxy_url: Option<String>,
) -> Result<Client, String> {
    use wreq::header::{HeaderValue, REFERER, USER_AGENT};

    let mut headers = default_headers(profile);
    if let Some(ua) = user_agent {
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(ua).map_err(|e| format!("Invalid User-Agent: {e}"))?,
        );
    }
    if let Some(r) = referer {
        headers.insert(
            REFERER,
            HeaderValue::from_str(r).map_err(|e| format!("Invalid Referer: {e}"))?,
        );
        headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
    }

    let provider = EmulationProvider::builder()
        .tls_config(tls_for(profile.tls_family()))
        .default_headers(headers)
        .build();

    let mut builder = Client::builder()
        .emulation(provider)
        .cookie_store(true)
        .redirect(wreq::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)));

    if let Some(proxy_url) = proxy_url.or_else(|| edgecrab_security::proxy::resolve_proxy_url(None))
        && let Ok(proxy) = wreq::Proxy::all(&proxy_url)
    {
        builder = builder.proxy(proxy);
    }

    builder.build().map_err(|e| e.to_string())
}

/// Build DDGS HTTP client — random primp profile per session, cookie jar, no redirects.
pub fn build_ddgs_client(
    timeout_secs: u64,
    proxy_url: Option<String>,
    profile: ImpersonateProfile,
) -> Result<Client, String> {
    build_wreq_client(timeout_secs, profile, None, None, proxy_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn primp_pool_has_chromium_firefox_safari_edge() {
        let ids: HashSet<_> = PRIMP_RANDOM_POOL.iter().map(|p| p.id()).collect();
        assert!(ids.contains("chrome_131"));
        assert!(ids.contains("chrome_133"));
        assert!(ids.contains("edge_131"));
        assert!(ids.contains("firefox_135"));
        assert!(ids.contains("firefox_133"));
        assert!(ids.contains("safari_18"));
    }

    #[test]
    fn random_profile_varies_over_samples() {
        let mut seen = HashSet::new();
        for _ in 0..40 {
            seen.insert(format!("{:?}", pick_random_profile()));
        }
        assert!(seen.len() >= 4, "random pool should vary: {seen:?}");
    }

    #[test]
    fn env_override_pins_chrome_131_random_os() {
        let mut seen = HashSet::new();
        for _ in 0..30 {
            seen.insert(format!("{:?}", resolve_profile(Some("chrome_131"))));
        }
        assert!(seen.len() >= 2, "chrome_131 should vary OS: {seen:?}");
    }

    #[test]
    fn chromium_profiles_emit_sec_ch_ua() {
        let p = ImpersonateProfile::Chrome131Win;
        assert!(p.sec_ch_ua().is_some());
        assert_eq!(p.sec_ch_ua_platform(), Some("\"Windows\""));
    }

    #[test]
    fn firefox_profiles_omit_sec_ch_ua() {
        assert!(ImpersonateProfile::Firefox135Mac.sec_ch_ua().is_none());
    }

    #[test]
    fn impersonate_os_env_filters_windows_profiles() {
        let p = resolve_profile_with(Some("chrome_131"), Some(ImpersonateOs::Windows));
        assert_eq!(p.impersonate_os(), ImpersonateOs::Windows);
        assert_eq!(p.id(), "chrome_131");
    }

    #[test]
    fn builds_client_for_each_tls_family() {
        for p in PRIMP_RANDOM_POOL {
            build_ddgs_client(5, None, *p).expect("client build");
        }
    }
}
