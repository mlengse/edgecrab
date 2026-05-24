# 027 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-tools/src/tools/x_search.rs (NEW)                 │
   │                                                                  │
   │   pub struct XSearch;                                            │
   │                                                                  │
   │   #[derive(Deserialize)]                                         │
   │   struct Args {                                                  │
   │       query: String,                                             │
   │       #[serde(default = "default_max")] max_results: u32,        │
   │       lang: Option<String>,                                      │
   │       since_id: Option<String>,                                  │
   │       time_range: Option<String>, // "1h", "24h", "7d"          │
   │   }                                                              │
   │                                                                  │
   │   async fn execute(&self, args, ctx) -> Result<String,...> {     │
   │       let token = env::var("X_BEARER_TOKEN")?;                   │
   │       let url = build_v2_recent_url(&args)?;                     │
   │       ssrf::is_safe_url(&url)?;                                  │
   │       let resp = client.get(url).bearer(token).send().await?;    │
   │       handle_rate_limit(&resp)?;                                 │
   │       let tweets = parse_v2(resp).await?;                        │
   │       Ok(serde_json::to_string(&tweets)?)                        │
   │   }                                                              │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Tool file** | `crates/edgecrab-tools/src/tools/x_search.rs` |
| **Register** | `crates/edgecrab-tools/src/tools/mod.rs` + add to a toolset (`social`) |
| **Auth** | `X_BEARER_TOKEN` env var; surface clear error if missing |
| **Rate-limit handling** | retry on 429 with `Retry-After`; max 3 retries; surface remaining quota in tool output footer |
| **Result shape** | strict JSON schema with documented fields |
| **Cost surfacing** | log API call cost to `usage.json` (X has paid tiers; if user provides cost-per-1000-tweets in config, track it) |
| **Tests** | `wiremock` based; rate-limit retry test; SSRF protection test |
| **Optional** | future backends (Bluesky AT Protocol, Mastodon search) via `SocialSearchBackend` trait — leave a single-implementation trait for OCP |

## DRY / SOLID Notes

- **OCP:** define `SocialSearchBackend` trait now; `x_search` is the
  first impl; future Bluesky/Mastodon backends slot in.
- **DRY:** SSRF check + retry logic from existing `web.rs` reused.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
