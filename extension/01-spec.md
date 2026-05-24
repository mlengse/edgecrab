# Raindrop.io Export — Chrome Extension Specification

**Version:** 2.0.0
**Date:** 2026-04-25
**Target App:** https://app.raindrop.io (Manifest V3)

---

## 1. Purpose & Goals

Export all Raindrop.io bookmarks with full metadata to local files in five formats:
**JSON**, **HTML (Netscape)**, **HTML (Rich Gallery)**, **Markdown**, and **Offline SPA** — a
self-contained single HTML file that fully replicates the Raindrop.io UX for offline browsing
(sidebar navigation, card/list/grid views, full-text search, tag filtering, sort, covers,
highlights, notes, broken-link flags, favourites).

The export is **resilient**: it checkpoints after every page of API results so that a
network failure, rate-limit, or browser close can be **resumed** from where it stopped,
not restarted from scratch. All in-progress state is held in `chrome.storage.local`.

---

## 2. Site & API Reference

### 2.1 URL Pattern
```
https://app.raindrop.io/my/{collectionId}
  0    → All bookmarks
 -1   → Unsorted
 -99  → Trash
 {n}  → Named collection
```

### 2.2 Raindrop Fields (complete)
| Field | Type | Notes |
|-------|------|-------|
| `_id` | Integer | Unique ID |
| `title` | String | max 1 000 chars |
| `link` | String | Target URL |
| `excerpt` | String | Description, max 10 000 |
| `note` | String | User note, max 10 000 |
| `cover` | String | Thumbnail URL |
| `media` | Array | `[{link}]` additional images |
| `tags` | Array | String tags |
| `type` | String | link/article/image/video/document/audio |
| `domain` | String | Hostname |
| `created` | ISO String | |
| `lastUpdate` | ISO String | |
| `important` | Boolean | Favourite ⭐ |
| `broken` | Boolean | URL unreachable ⚠️ |
| `highlights` | Array | `[{_id,text,color,note,created}]` |
| `collection.$id` | Integer | Parent collection |
| `cache.status` | String | ready/retry/failed/… |
| `file` | Object | `{name,size,type}` for uploads |

### 2.3 Collection Fields
`_id`, `title`, `count`, `cover[]`, `color`, `parent.$id`, `public`, `created`,
`lastUpdate`, `view`, `sort`, `access.level`, `collaborators`

### 2.4 API Endpoints
```
GET /rest/v1/collections           → root collections
GET /rest/v1/collections/childrens → nested collections
GET /rest/v1/raindrops/{id}?page={n}&perpage=50&sort=-created
```

---

## 3. Architecture (v2)

### 3.1 File Tree
```
extension/
├── manifest.json
├── background.js              # Service worker: downloads + message bus
├── popup.html                 # Main UI (360×520px)
├── popup.js                   # Orchestration + resume UI
├── options.html               # Settings page
├── options.js                 # Settings persistence
├── src/
│   ├── api.js                 # DRY REST client + per-page retry
│   ├── checkpoint.js          # Resilient export state machine  ← NEW
│   ├── utils.js               # Pure helpers
│   └── formatters/
│       ├── json.js
│       ├── html.js            # Netscape + Rich gallery
│       ├── markdown.js
│       └── offline.js         # Full offline SPA replica         ← NEW
└── icons/
```

### 3.2 Checkpoint State Machine

```
IDLE → STARTED → FETCHING_COLLECTIONS → FETCHING_PAGES → FORMATTING → DOWNLOADING → DONE
                                              ↑                ↓
                                         resume here ←── network error/close
```

State stored in `chrome.storage.local` under key `exportCheckpoint`:
```json
{
  "version": 1,
  "collectionId": "0",
  "startedAt": "ISO",
  "resumedAt": "ISO",
  "totalCount": 1500,
  "fetchedPages": 12,
  "totalPages": 30,
  "collections": [...],
  "raindrops": [...],
  "status": "fetching_pages"
}
```

On **every successfully fetched page**, the state is written atomically to storage.
On popup open, if `status !== "done"` and `status !== "idle"`, a **Resume banner** is shown.

### 3.3 Component Responsibilities (SOLID)

| Component | Single Responsibility |
|-----------|----------------------|
| `api.js` | HTTP + auth + pagination + per-page retry with back-off |
| `checkpoint.js` | Read/write/clear checkpoint; resume logic |
| `formatters/json.js` | Convert payload → JSON string |
| `formatters/html.js` | Convert payload → Netscape HTML or Rich gallery HTML |
| `formatters/markdown.js` | Convert payload → Markdown |
| `formatters/offline.js` | Convert payload → self-contained offline SPA HTML |
| `background.js` | File downloads; keepalive alarm |
| `popup.js` | UX state machine; progress; resume |
| `options.js` | Settings via chrome.storage.sync |
| `utils.js` | Pure helpers (no side effects) |

---

## 4. Resilient Export Engine

### 4.1 Per-page Retry
Each page fetch retries up to **5 times** with exponential back-off (1s, 2s, 4s, 8s, 16s)
+ ±500ms jitter. After all retries exhausted the export is suspended (not aborted) and the
checkpoint is saved so the user can resume.

### 4.2 Checkpointing Contract
- Checkpoint written **after every successful page** (not just at end)
- Checkpoint written **before any download** (format phase is idempotent — safe to re-run)
- Checkpoint cleared only on explicit user action ("Clear & Restart") or on successful DONE
- Partial raindrops from a prior run are **merged** by `_id` deduplication before continuing

### 4.3 Resume Flow
1. Popup opens → `checkpoint.load()` → if `status` ∈ {`fetching_pages`, `suspended`}:
   - Show Resume banner: "↺ Resume interrupted export — {n}/{total} fetched"
   - Buttons: **Resume** | **Discard & restart**
2. Resume: skip already-fetched pages (use `fetchedPages` cursor), merge into existing data
3. Continue fetch from page `fetchedPages` onward

### 4.4 Concurrency Guard
`chrome.storage.session` mutex flag `exportRunning`. If popup opens while another instance
is exporting (unlikely but possible), show "Export already running" and disable button.

---

## 5. Export Formats

### 5.1 JSON — full fidelity raw data
### 5.2 HTML (Netscape) — browser-importable bookmarks
### 5.3 HTML (Rich Gallery) — visual with covers/tags/highlights
### 5.4 Markdown — sectioned by collection
### 5.5 Offline SPA — **new, primary feature**

#### Offline SPA Specification

A **single self-contained `.html` file** (~2–8 MB typical) with zero external dependencies.
All CSS, JS, and fallback icons are inlined. Opens in any browser from any local path.

##### Features

| Feature | Implementation |
|---------|---------------|
| **Sidebar** | Fixed left panel (290px), collection list with counts, system collections |
| **List view** | Horizontal rows: thumbnail · title · meta (collection, domain, date) |
| **Card/Grid view** | 3-column responsive masonry, cover on top, title, tags, meta below |
| **Compact view** | Dense single-line rows (title + domain + date, no thumbnails) |
| **View toggle** | 3-button toggle (List / Grid / Compact) in toolbar, persisted to localStorage |
| **Full-text search** | Client-side Fuse.js (inlined, ~30KB) over title+excerpt+note+tags+domain |
| **Tag filter** | Click tag pill to filter; active tag highlighted; clear button |
| **Sort** | By date desc/asc, by title A–Z, by domain |
| **Collection nav** | Sidebar click filters to collection; "All" shows everything |
| **Broken filter** | Toggle to show only broken links |
| **Favourites filter** | Toggle to show only ⭐ favourites |
| **Pagination** | Virtual window: render 50 at a time, "Load more" / infinite scroll |
| **Card detail** | Click card → slide-in detail panel: full excerpt, note, all highlights, media list |
| **Thumbnail** | `<img>` with `loading=lazy` + `onerror` fallback to domain favicon placeholder |
| **Dark mode** | Auto via `prefers-color-scheme`, toggle button overrides |
| **Keyboard nav** | `/` focuses search; `Esc` clears; `←` `→` pages detail panel |
| **Export metadata** | Footer: exported-at, total counts, version |

##### Data embedding strategy
All bookmark data embedded as a `const DATA = {...}` JS variable in a `<script>` tag.
This avoids CORS/XHR restrictions when opening from `file://`.

---

## 6. Settings (Options Page)

| Setting | Default |
|---------|---------|
| Export formats (multi) | JSON + Offline SPA |
| Export scope | All collections |
| Include Unsorted | true |
| Include Trash | false |
| Rich HTML | true |
| Per-page count | 50 (max) |
| Remember token | true |

---

## 7. Edge Cases & Mitigations

| Risk | Mitigation |
|------|-----------|
| Token extraction fails | Manual paste + save |
| Rate limiting (429) | Back-off + suspend + checkpoint |
| Network drop mid-export | Auto-checkpoint → resume banner on next open |
| Service worker killed (MV3) | `chrome.alarms` keepalive every 25s |
| 15 000+ bookmarks | Paginated fetch; checkpoint every page; UI shows page-by-page progress |
| `chrome.storage.local` quota (10MB) | Chunked storage: split `raindrops` array across multiple keys |
| Cover image CDN 404 | `onerror` fallback to letter-avatar; never breaks layout |
| Offline SPA > 100MB | Warn user; chunk into multiple files if >50MB |
| `_` prefixed files in extension dir | No such files — checked by load-extension.js validator |
| Broken links in export | `broken: true` flag → ⚠️ badge in all formats |
| Duplicate `_id` across pages | Dedup by `_id` in merge step |
| Tags with special markdown chars | `escapeMarkdownTable()` |
| XSS in titles/excerpts | `escapeHtml()` on all user content in HTML outputs |
| Chrome storage quota exceeded | Flush partial data to download, clear checkpoint, continue |

---

## 8. Permissions (unchanged)

```json
["downloads","activeTab","scripting","storage","alarms"]
host_permissions: ["https://api.raindrop.io/*","https://app.raindrop.io/*"]
```

---

## 9. Testing Plan

1. Syntax check all JS with `node --check` ✓
2. Unit tests: formatters, groupByCollection, escapeHtml, checkpoint state transitions
3. Integration: load unpacked, open raindrop.io, enter token, export → verify files appear in Downloads
4. Resume test: kill popup mid-export, reopen → resume banner shows, resume works
5. Offline SPA: open generated file in Chrome, Firefox, Safari → search, filter, view modes
6. Large library test: 1 000+ items → pagination correct, no memory crash
