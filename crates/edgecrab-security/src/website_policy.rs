//! Website access policy — domain blocklist for URL-capable tools.
//!
//! Mirrors Hermes `tools/website_policy.py`: loads `security.website_blocklist`
//! from `~/.edgecrab/config.yaml` (or `EDGECRAB_HOME`) with optional shared list files.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use thiserror::Error;
use url::Url;

const CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRule {
    pub pattern: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WebsiteBlockPolicy {
    pub enabled: bool,
    pub rules: Vec<BlockRule>,
}

/// Metadata returned when a URL is blocked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebsiteBlockInfo {
    pub url: String,
    pub host: String,
    pub rule: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum WebsitePolicyError {
    #[error("Invalid config YAML at {path}: {source}")]
    InvalidYaml {
        path: PathBuf,
        source: serde_yml::Error,
    },
    #[error("Failed to read config file {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{0}")]
    InvalidShape(String),
}

struct PolicyCache {
    path_tag: String,
    loaded_at: Instant,
    policy: WebsiteBlockPolicy,
}

static CACHE: OnceLock<Mutex<Option<PolicyCache>>> = OnceLock::new();

fn cache_lock() -> std::sync::MutexGuard<'static, Option<PolicyCache>> {
    CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Drop cached policy so the next check reloads config.
pub fn invalidate_cache() {
    *cache_lock() = None;
}

fn edgecrab_home() -> PathBuf {
    std::env::var("EDGECRAB_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".edgecrab")))
        .unwrap_or_else(|_| PathBuf::from(".edgecrab"))
}

fn default_config_path() -> PathBuf {
    edgecrab_home().join("config.yaml")
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .to_ascii_lowercase()
        .trim_end_matches('.')
        .to_string()
}

fn normalize_rule(raw: &str) -> Option<String> {
    let mut value = raw.trim().to_ascii_lowercase();
    if value.is_empty() || value.starts_with('#') {
        return None;
    }
    if let Ok(parsed) = Url::parse(&value) {
        if let Some(host) = parsed.host_str() {
            value = host.to_string();
        } else if !parsed.path().is_empty() {
            value = parsed.path().trim_start_matches('/').to_string();
        }
    }
    value = value
        .split('/')
        .next()
        .unwrap_or("")
        .trim_end_matches('.')
        .to_string();
    if value.starts_with("www.") {
        value = value[4..].to_string();
    }
    if value.is_empty() { None } else { Some(value) }
}

fn read_shared_rules(path: &Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        tracing::warn!(
            "Shared blocklist file not found (skipping): {}",
            path.display()
        );
        return Vec::new();
    };
    raw.lines().filter_map(normalize_rule).collect()
}

fn load_policy_config(config_path: &Path) -> Result<WebsiteBlockPolicy, WebsitePolicyError> {
    if !config_path.exists() {
        return Ok(WebsiteBlockPolicy::default());
    }

    let content =
        std::fs::read_to_string(config_path).map_err(|source| WebsitePolicyError::ReadConfig {
            path: config_path.to_path_buf(),
            source,
        })?;
    let raw: serde_yml::Value =
        serde_yml::from_str(&content).map_err(|source| WebsitePolicyError::InvalidYaml {
            path: config_path.to_path_buf(),
            source,
        })?;

    let security = raw
        .get("security")
        .cloned()
        .unwrap_or(serde_yml::Value::Null);
    if !security.is_null() && !security.is_mapping() {
        return Err(WebsitePolicyError::InvalidShape(
            "security must be a mapping".into(),
        ));
    }

    let blocklist = security
        .get("website_blocklist")
        .cloned()
        .unwrap_or(serde_yml::Value::Null);
    if !blocklist.is_null() && !blocklist.is_mapping() {
        return Err(WebsitePolicyError::InvalidShape(
            "security.website_blocklist must be a mapping".into(),
        ));
    }

    let enabled = blocklist
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let domains = blocklist
        .get("domains")
        .cloned()
        .unwrap_or(serde_yml::Value::Null);
    if !domains.is_null() && !domains.is_sequence() {
        return Err(WebsitePolicyError::InvalidShape(
            "security.website_blocklist.domains must be a list".into(),
        ));
    }

    let shared_files = blocklist
        .get("shared_files")
        .cloned()
        .unwrap_or(serde_yml::Value::Null);
    if !shared_files.is_null() && !shared_files.is_sequence() {
        return Err(WebsitePolicyError::InvalidShape(
            "security.website_blocklist.shared_files must be a list".into(),
        ));
    }

    let mut rules = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(items) = domains.as_sequence() {
        for item in items {
            if let Some(normalized) = item.as_str().and_then(normalize_rule)
                && seen.insert(("config".to_string(), normalized.clone()))
            {
                rules.push(BlockRule {
                    pattern: normalized,
                    source: "config".into(),
                });
            }
        }
    }

    if let Some(items) = shared_files.as_sequence() {
        for item in items {
            let Some(path_str) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            let mut path = PathBuf::from(path_str);
            if !path.is_absolute() {
                path = edgecrab_home().join(path);
            }
            let path = path;
            for normalized in read_shared_rules(&path) {
                let key = (path.display().to_string(), normalized.clone());
                if seen.insert(key) {
                    rules.push(BlockRule {
                        pattern: normalized,
                        source: path.display().to_string(),
                    });
                }
            }
        }
    }

    Ok(WebsiteBlockPolicy { enabled, rules })
}

/// Load blocklist policy from disk (explicit path bypasses cache).
pub fn load_website_blocklist(
    config_path: Option<&Path>,
) -> Result<WebsiteBlockPolicy, WebsitePolicyError> {
    let path = config_path
        .map(Path::to_path_buf)
        .unwrap_or_else(default_config_path);
    let path_tag = path.display().to_string();

    if config_path.is_none() {
        let guard = cache_lock();
        if let Some(cache) = guard.as_ref()
            && cache.path_tag == path_tag
            && cache.loaded_at.elapsed() < CACHE_TTL
        {
            return Ok(cache.policy.clone());
        }
    }

    let policy = load_policy_config(&path)?;

    if config_path.is_none() {
        *cache_lock() = Some(PolicyCache {
            path_tag,
            loaded_at: Instant::now(),
            policy: policy.clone(),
        });
    }

    Ok(policy)
}

fn extract_host(url: &str) -> Option<String> {
    if let Ok(parsed) = Url::parse(url)
        && let Some(host) = parsed.host_str()
    {
        return Some(normalize_host(host));
    }
    if !url.contains("://")
        && let Ok(parsed) = Url::parse(&format!("//{url}"))
        && let Some(host) = parsed.host_str()
    {
        return Some(normalize_host(host));
    }
    None
}

fn match_host_against_rule(host: &str, pattern: &str) -> bool {
    if host.is_empty() || pattern.is_empty() {
        return false;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return host != suffix && host.ends_with(&format!(".{suffix}"));
    }
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

/// Returns block metadata when disallowed, `None` when allowed.
///
/// Fail-open on load errors (logs warning) unless an explicit `config_path`
/// is passed (tests propagate errors).
pub fn check_website_access(
    url: &str,
    config_path: Option<&Path>,
) -> Result<Option<WebsiteBlockInfo>, WebsitePolicyError> {
    let host = match extract_host(url) {
        Some(host) => host,
        None => return Ok(None),
    };

    let policy = match load_website_blocklist(config_path) {
        Ok(policy) => policy,
        Err(err) if config_path.is_some() => return Err(err),
        Err(err) => {
            tracing::warn!("Website policy config error (failing open): {err}");
            return Ok(None);
        }
    };

    if !policy.enabled {
        return Ok(None);
    }

    for rule in &policy.rules {
        if match_host_against_rule(&host, &rule.pattern) {
            let message = format!(
                "Blocked by website policy: '{host}' matched rule '{}' from {}",
                rule.pattern, rule.source
            );
            tracing::info!(
                url = url,
                host = host,
                rule = rule.pattern,
                source = rule.source,
                "URL blocked by website policy"
            );
            return Ok(Some(WebsiteBlockInfo {
                url: url.to_string(),
                host,
                rule: rule.pattern.clone(),
                source: rule.source.clone(),
                message,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, yaml: &str) -> PathBuf {
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, yaml).expect("write config");
        path
    }

    #[test]
    fn merges_config_and_shared_file() {
        invalidate_cache();
        let dir = TempDir::new().expect("tempdir");
        let shared = dir.path().join("community-blocklist.txt");
        std::fs::write(&shared, "# comment\nexample.org\nsub.bad.net\n").expect("shared");

        let config = write_config(
            &dir,
            &format!(
                r#"
security:
  website_blocklist:
    enabled: true
    domains:
      - example.com
      - https://www.evil.test/path
    shared_files:
      - "{shared}"
"#,
                shared = shared.display()
            ),
        );

        let policy = load_website_blocklist(Some(&config)).expect("load");
        assert!(policy.enabled);
        let patterns: std::collections::HashSet<_> =
            policy.rules.iter().map(|r| r.pattern.as_str()).collect();
        assert!(patterns.contains("example.com"));
        assert!(patterns.contains("evil.test"));
        assert!(patterns.contains("example.org"));
        assert!(patterns.contains("sub.bad.net"));
    }

    #[test]
    fn blocks_subdomains_of_parent_rule() {
        invalidate_cache();
        let dir = TempDir::new().expect("tempdir");
        let config = write_config(
            &dir,
            r#"
security:
  website_blocklist:
    enabled: true
    domains: [example.com]
"#,
        );

        let blocked = check_website_access("https://docs.example.com/page", Some(&config))
            .expect("check")
            .expect("blocked");
        assert_eq!(blocked.host, "docs.example.com");
        assert_eq!(blocked.rule, "example.com");
    }

    #[test]
    fn wildcard_subdomain_rule() {
        invalidate_cache();
        let dir = TempDir::new().expect("tempdir");
        let config = write_config(
            &dir,
            r#"
security:
  website_blocklist:
    enabled: true
    domains: ["*.tracking.example"]
"#,
        );

        assert!(
            check_website_access("https://a.tracking.example", Some(&config))
                .expect("check")
                .is_some()
        );
        assert!(
            check_website_access("https://tracking.example", Some(&config))
                .expect("check")
                .is_none()
        );
    }

    #[test]
    fn disabled_by_default_when_section_missing() {
        invalidate_cache();
        let dir = TempDir::new().expect("tempdir");
        let config = write_config(&dir, "display:\n  tool_progress: all\n");
        let policy = load_website_blocklist(Some(&config)).expect("load");
        assert!(!policy.enabled);
        assert!(policy.rules.is_empty());
    }

    #[test]
    fn invalid_domains_type_errors_with_explicit_path() {
        invalidate_cache();
        let dir = TempDir::new().expect("tempdir");
        let config = write_config(
            &dir,
            r#"
security:
  website_blocklist:
    enabled: true
    domains: example.com
"#,
        );
        let err = load_website_blocklist(Some(&config)).expect_err("invalid");
        assert!(err.to_string().contains("domains must be a list"));
    }
}
