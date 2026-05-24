/**
 * checkpoint.js — Resilient export state machine (v2 — cursor-only storage)
 *
 * DESIGN: Only the *progress cursor* is written to chrome.storage.local.
 * Raindrop data is NEVER stored — it accumulates in memory in popup.js.
 * This means the storage footprint is a flat ~2 KB regardless of library size,
 * completely eliminating the kQuotaBytes error.
 *
 * On resume from a fresh popup, pages 0..fetchedPages-1 are re-fetched via
 * the API (fast: ~1 req/s, no quota cost) before continuing from the cursor.
 *
 * Checkpoint schema (chrome.storage.local key: "exportCheckpoint"):
 * {
 *   version:      2,
 *   collectionId: string,
 *   startedAt:    ISO,
 *   resumedAt:    ISO | null,
 *   totalCount:   number,
 *   fetchedPages: number,      ← the only cursor we need
 *   totalPages:   number,
 *   collections:  Collection[], ← cached so resume skips an extra API call
 *   status:       'fetching_pages' | 'suspended' | 'done'
 * }
 *
 * Single Responsibility: checkpoint cursor read/write/clear/resume helpers.
 */

const CHECKPOINT_KEY = 'exportCheckpoint';
const MUTEX_KEY = 'exportRunning';

// ── Public API ─────────────────────────────────────────────────────────────

/**
 * Load checkpoint cursor. Returns null if none exists.
 * @returns {Promise<object|null>}
 */
export async function loadCheckpoint() {
  const data = await chrome.storage.local.get(CHECKPOINT_KEY);
  return data[CHECKPOINT_KEY] || null;
}

/**
 * Start a brand-new checkpoint, wiping any previous one.
 * @param {string|number} collectionId
 * @param {object[]}      collections
 * @returns {Promise<object>}
 */
export async function startCheckpoint(collectionId, collections) {
  const cp = {
    version: 2,
    collectionId: String(collectionId),
    startedAt: new Date().toISOString(),
    resumedAt: null,
    totalCount: 0,
    fetchedPages: 0,
    totalPages: 0,
    // Store collections so resume doesn't need an extra API round-trip
    collections: collections.map((c) => ({
      _id: c._id, title: c.title, count: c.count || 0,
      color: c.color || null, parent: c.parent || null,
    })),
    status: 'fetching_pages',
  };
  await chrome.storage.local.set({ [CHECKPOINT_KEY]: cp });
  return cp;
}

/**
 * Advance the page cursor. No raindrop data is written to storage.
 * @param {object} cp         current checkpoint
 * @param {number} totalCount from API
 * @param {number} page       0-based page index just successfully fetched
 * @param {number} totalPages
 * @returns {Promise<object>} updated checkpoint
 */
export async function advanceCursor(cp, totalCount, page, totalPages) {
  const updated = {
    ...cp,
    totalCount,
    fetchedPages: page + 1,
    totalPages,
    status: 'fetching_pages',
  };
  await chrome.storage.local.set({ [CHECKPOINT_KEY]: updated });
  return updated;
}

/**
 * Mark checkpoint as suspended (retries exhausted or user paused).
 * @param {object} cp
 * @returns {Promise<object>}
 */
export async function suspendCheckpoint(cp) {
  const updated = { ...cp, status: 'suspended' };
  await chrome.storage.local.set({ [CHECKPOINT_KEY]: updated });
  return updated;
}

/**
 * Record that the export was resumed now.
 * @param {object} cp
 * @returns {Promise<object>}
 */
export async function markResumed(cp) {
  const updated = { ...cp, resumedAt: new Date().toISOString(), status: 'fetching_pages' };
  await chrome.storage.local.set({ [CHECKPOINT_KEY]: updated });
  return updated;
}

/**
 * Clear the checkpoint from storage (call on success or user-initiated discard).
 */
export async function clearCheckpoint() {
  await chrome.storage.local.remove(CHECKPOINT_KEY);
}

/**
 * Convenience: clear checkpoint on successful completion.
 */
export async function completeCheckpoint() {
  await clearCheckpoint();
}

/**
 * Returns true if the checkpoint can be resumed.
 * @param {object|null} cp
 * @returns {boolean}
 */
export function isResumable(cp) {
  return cp !== null &&
    (cp.status === 'fetching_pages' || cp.status === 'suspended') &&
    cp.fetchedPages > 0;
}

/**
 * Human-readable resume label for the banner.
 * @param {object} cp
 * @returns {string}
 */
export function resumeLabel(cp) {
  const fetched = cp.fetchedPages * 50; // approximate items
  const total   = cp.totalCount || '?';
  const pct     = cp.totalPages > 0
    ? Math.round((cp.fetchedPages / cp.totalPages) * 100)
    : 0;
  return `~${fetched.toLocaleString()} of ${total.toLocaleString()} items fetched (${pct}%) — resume or discard`;
}

// ── Mutex ──────────────────────────────────────────────────────────────────

/**
 * Acquire export mutex. Returns false if another export is already running.
 * Uses chrome.storage.session (in-memory, cleared on browser restart).
 * @returns {Promise<boolean>}
 */
export async function acquireMutex() {
  const data = await chrome.storage.session.get(MUTEX_KEY).catch(() => ({}));
  if (data[MUTEX_KEY]) return false;
  await chrome.storage.session.set({ [MUTEX_KEY]: true }).catch(() => {});
  return true;
}

/** Release the running mutex. */
export async function releaseMutex() {
  await chrome.storage.session.remove(MUTEX_KEY).catch(() => {});
}
