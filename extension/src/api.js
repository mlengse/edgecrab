/**
 * api.js — DRY Raindrop REST API client
 * Responsibilities: HTTP calls, auth, pagination, per-page retry with back-off.
 */

const API_BASE = 'https://api.raindrop.io/rest/v1';
const PER_PAGE = 50;
const MAX_RETRIES = 5;

// ---------------------------------------------------------------------------
// Token management
// ---------------------------------------------------------------------------

export async function getToken() {
  const { token } = await chrome.storage.local.get('token');
  return token || null;
}

export async function saveToken(token) {
  await chrome.storage.local.set({ token });
}

export async function clearToken() {
  await chrome.storage.local.remove('token');
}

// ---------------------------------------------------------------------------
// Generic fetch with retry + back-off
// ---------------------------------------------------------------------------

async function apiFetch(path, token, retries = MAX_RETRIES) {
  const url = `${API_BASE}${path}`;
  let lastError;

  for (let attempt = 0; attempt < retries; attempt++) {
    if (attempt > 0) {
      const delay = Math.min(1000 * 2 ** attempt + Math.random() * 500, 16000);
      await sleep(delay);
    }

    let resp;
    try {
      resp = await fetch(url, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });
    } catch (networkErr) {
      lastError = new RetryableError(`Network error: ${networkErr.message}`);
      continue; // retry on network failure
    }

    if (resp.status === 429) {
      const retryAfter = parseInt(resp.headers.get('Retry-After') || '4', 10);
      await sleep(retryAfter * 1000);
      lastError = new RetryableError('Rate limited (429)');
      continue;
    }

    if (resp.status === 401) throw new AuthError('Unauthorized — invalid or expired token');
    if (resp.status >= 500) {
      lastError = new RetryableError(`Server error ${resp.status}`);
      continue; // retry 5xx
    }
    if (!resp.ok) throw new ApiError(`API error ${resp.status}: ${resp.statusText}`, resp.status);

    return resp.json();
  }

  throw lastError || new RetryableError('Max retries exceeded');
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

export async function fetchRootCollections(token) {
  const data = await apiFetch('/collections', token);
  return data.items || [];
}

export async function fetchChildCollections(token) {
  const data = await apiFetch('/collections/childrens', token);
  return data.items || [];
}

export async function fetchAllCollections(token) {
  const [roots, children] = await Promise.all([
    fetchRootCollections(token),
    fetchChildCollections(token),
  ]);
  const seen = new Set();
  return [...roots, ...children].filter((c) => {
    if (seen.has(c._id)) return false;
    seen.add(c._id);
    return true;
  });
}

// ---------------------------------------------------------------------------
// Raindrops — paginated with per-page checkpoint callback
// ---------------------------------------------------------------------------

/**
 * Fetch one page of raindrops.
 * @returns {Promise<{items: Raindrop[], count: number}>}
 */
export async function fetchRaindropsPage(collectionId, page, token) {
  const params = new URLSearchParams({
    page: String(page),
    perpage: String(PER_PAGE),
    sort: '-created',
  });
  const data = await apiFetch(`/raindrops/${collectionId}?${params}`, token);
  return { items: data.items || [], count: data.count || 0 };
}

/**
 * Fetch ALL raindrops with checkpointing.
 *
 * @param {string|number}  collectionId
 * @param {string}         token
 * @param {object}         opts
 * @param {number}         opts.startPage       resume from this page (default 0)
 * @param {function}       opts.onPage          called after each page: (items, page, total, totalPages)
 * @param {function}       opts.onSuspend       called if retries exhausted mid-export
 * @param {AbortSignal}    opts.signal          optional abort signal
 * @returns {Promise<{raindrops: Raindrop[], completed: boolean}>}
 */
export async function fetchAllRaindrops(collectionId, token, opts = {}) {
  const { startPage = 0, onPage, onSuspend, signal } = opts;
  const collected = [];
  let totalCount = 0;

  // First page to discover total
  if (startPage === 0) {
    let first;
    try {
      first = await fetchRaindropsPage(collectionId, 0, token);
    } catch (err) {
      if (onSuspend) onSuspend(err);
      return { raindrops: collected, completed: false };
    }
    totalCount = first.count;
    collected.push(...first.items);
    const totalPages = Math.ceil(totalCount / PER_PAGE);
    if (onPage) await onPage(first.items, 0, totalCount, totalPages);
    if (first.items.length === 0 || totalPages <= 1) {
      return { raindrops: collected, completed: true };
    }
  }

  // Re-fetch total if resuming
  if (startPage > 0) {
    try {
      const probe = await fetchRaindropsPage(collectionId, 0, token);
      totalCount = probe.count;
    } catch {
      totalCount = startPage * PER_PAGE; // best guess
    }
  }

  const totalPages = Math.ceil(totalCount / PER_PAGE);

  for (let page = Math.max(startPage, 1); page < totalPages; page++) {
    if (signal?.aborted) return { raindrops: collected, completed: false };

    let result;
    try {
      result = await fetchRaindropsPage(collectionId, page, token);
    } catch (err) {
      if (err instanceof RetryableError) {
        if (onSuspend) onSuspend(err);
        return { raindrops: collected, completed: false };
      }
      throw err;
    }

    if (result.items.length === 0) break;
    collected.push(...result.items);
    if (onPage) await onPage(result.items, page, totalCount, totalPages);
  }

  return { raindrops: collected, completed: true };
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

export class AuthError extends Error {
  constructor(msg) { super(msg); this.name = 'AuthError'; }
}

export class ApiError extends Error {
  constructor(msg, status) { super(msg); this.name = 'ApiError'; this.status = status; }
}

export class RetryableError extends Error {
  constructor(msg) { super(msg); this.name = 'RetryableError'; }
}
