/**
 * popup.js — Popup UI orchestration (v2.1 — quota-safe, cursor-only checkpoint)
 *
 * Key change from v2.0: raindrop data is NEVER stored in chrome.storage.
 * Only the page cursor is persisted. On resume, pages 0..fetchedPages-1
 * are re-fetched from the API (fast, cheap) before continuing from the cursor.
 * This eliminates the kQuotaBytes error completely.
 */

import {
  fetchAllCollections, fetchRaindropsPage,
  getToken, saveToken, AuthError, RetryableError,
} from './src/api.js';
import {
  loadCheckpoint, startCheckpoint, advanceCursor,
  suspendCheckpoint, markResumed, completeCheckpoint, clearCheckpoint,
  isResumable, resumeLabel, acquireMutex, releaseMutex,
} from './src/checkpoint.js';
import { buildCollectionMap, buildFilename } from './src/utils.js';
import { formatJSON }                        from './src/formatters/json.js';
import { formatNetscapeHTML, formatRichHTML }from './src/formatters/html.js';
import { formatMarkdown }                    from './src/formatters/markdown.js';
import { formatOfflineSPA }                  from './src/formatters/offline.js';

// ── DOM refs ──────────────────────────────────────────────────────────────
const $ = (id) => document.getElementById(id);
const elToken        = $('inputToken');
const elSaveToken    = $('btnSaveToken');
const elTokenSt      = $('tokenStatus');
const elScope        = $('selectScope');
const elProgress     = $('progressWrap');
const elBar          = $('progressBar');
const elLabel        = $('progressLabel');
const elStatus       = $('statusMsg');
const elExport       = $('btnExport');
const elSettings     = $('btnSettings');
const elResumeBanner = $('resumeBanner');
const elResumeLabel  = $('resumeLabel');
const elBtnResume    = $('btnResume');
const elBtnDiscard   = $('btnDiscard');
const elAbort        = $('btnAbort');

// ── State ──────────────────────────────────────────────────────────────────
let isExporting    = false;
let abortRequested = false;
const selectedFormats = new Set(['json', 'offline']);

// ── Init ───────────────────────────────────────────────────────────────────
async function init() {
  const token = await getToken();
  if (token) {
    elToken.value = token;
    setTokenStatus('ok', '✓ Token saved');
  } else {
    setTokenStatus('warn', 'Paste your Raindrop token above');
  }

  // Format buttons
  document.querySelectorAll('.fmt-btn').forEach((btn) => {
    const fmt = btn.dataset.fmt;
    btn.classList.toggle('active', selectedFormats.has(fmt));
    btn.addEventListener('click', () => {
      if (selectedFormats.has(fmt)) {
        if (selectedFormats.size > 1) { selectedFormats.delete(fmt); btn.classList.remove('active'); }
      } else {
        selectedFormats.add(fmt); btn.classList.add('active');
      }
    });
  });

  elSaveToken.addEventListener('click', async () => {
    const t = elToken.value.trim();
    if (!t) { setTokenStatus('err', '✗ Token is empty'); return; }
    await saveToken(t);
    setTokenStatus('ok', '✓ Token saved');
  });

  elExport.addEventListener('click', () => startExport());

  elAbort.addEventListener('click', () => {
    abortRequested = true;
    setStatus('⚠️ Pausing after this page…');
    elAbort.disabled = true;
  });

  elSettings.addEventListener('click', () => chrome.runtime.openOptionsPage());

  await checkResumeBanner();
}

// ── Resume Banner ──────────────────────────────────────────────────────────
async function checkResumeBanner() {
  const cp = await loadCheckpoint();
  if (!isResumable(cp)) { elResumeBanner.style.display = 'none'; return; }
  elResumeBanner.style.display = 'flex';
  elResumeLabel.textContent = resumeLabel(cp);
  elBtnResume.onclick  = () => { elResumeBanner.style.display = 'none'; startExport({ resume: true }); };
  elBtnDiscard.onclick = async () => { await clearCheckpoint(); elResumeBanner.style.display = 'none'; setStatus(''); };
}

// ── Export Flow ────────────────────────────────────────────────────────────
async function startExport({ resume = false } = {}) {
  if (isExporting) return;
  const locked = await acquireMutex();
  if (!locked) { setStatus('⚠️ Export already running in another tab'); return; }

  const token = elToken.value.trim() || await getToken();
  if (!token) { setTokenStatus('err', '✗ No token — paste one above'); releaseMutex(); return; }
  await saveToken(token);

  isExporting    = true;
  abortRequested = false;
  elExport.disabled = true;
  elAbort.style.display = 'inline-block';
  elAbort.disabled = false;
  elProgress.style.display = 'block';

  try {
    await runExport(token, resume);
  } catch (err) {
    if (err instanceof AuthError) {
      setTokenStatus('err', '✗ Invalid token');
      setStatus('Authentication failed — check your token.');
    } else {
      setStatus(`❌ Export error: ${err.message}`);
    }
  } finally {
    isExporting = false;
    elExport.disabled = false;
    elAbort.style.display = 'none';
    await releaseMutex();
  }
}

async function runExport(token, resume) {
  const PER_PAGE = 50;

  // ── Step 1: Collections ────────────────────────────────────────────────
  setStatus('Fetching collections…');
  setProgress(0, 0, 'Loading collections…');

  let cp = resume ? await loadCheckpoint() : null;

  let collections;
  if (cp?.collections?.length) {
    collections = cp.collections;         // cached from previous run
  } else {
    collections = await fetchAllCollections(token);
  }
  const collMap = buildCollectionMap(collections);

  const scope        = elScope.value;
  const collectionId = scope === 'all' ? 0 : Number(scope);

  // ── Step 2: Checkpoint setup ───────────────────────────────────────────
  if (!cp || !resume) {
    cp = await startCheckpoint(collectionId, collections);
  } else {
    cp = await markResumed(cp);
    setStatus(`↺ Resuming — re-fetching ${cp.fetchedPages} page(s) from API…`);
  }

  // ── Step 3: Discover total ─────────────────────────────────────────────
  // Probe page 0 to get totalCount/totalPages (always needed, even on resume)
  let totalCount = cp.totalCount || 0;
  let totalPages = cp.totalPages || 0;

  if (totalCount === 0 || totalPages === 0) {
    const probe = await fetchRaindropsPage(collectionId, 0, token);
    totalCount  = probe.count;
    totalPages  = Math.ceil(totalCount / PER_PAGE);
    // Don't advance cursor yet — page 0 is included in the re-fetch loop below
  } else {
    // Already know totals from the saved checkpoint
    setProgress(cp.fetchedPages, totalPages, `Resuming from page ${cp.fetchedPages + 1} of ${totalPages}…`);
  }

  // ── Step 4: Fetch all pages (resume skips no storage — just re-fetches) ──
  // All raindrops live only in this in-memory array.
  const raindrops = [];

  // Dedup map for the re-fetch phase (resume may re-fetch pages 0..cursor-1)
  const seen = new Set();

  const addItems = (items) => {
    for (const r of items) {
      if (!seen.has(r._id)) { seen.add(r._id); raindrops.push(r); }
    }
  };

  const resumeCursor = resume ? (cp.fetchedPages) : 0;

  for (let page = 0; page < totalPages; page++) {
    if (abortRequested) {
      cp = await suspendCheckpoint(cp);
      setStatus('⏸ Export paused — reopen popup to resume.');
      await checkResumeBanner();
      return;
    }

    const isReplay = page < resumeCursor; // re-fetching already-seen pages
    setProgress(
      page + 1, totalPages,
      isReplay
        ? `Re-fetching page ${page + 1} of ${totalPages} (resuming)…`
        : `Fetching page ${page + 1} of ${totalPages}…`,
    );

    let result;
    try {
      result = await fetchRaindropsPage(collectionId, page, token);
    } catch (err) {
      if (err instanceof RetryableError) {
        // Save cursor so we know where we got to
        cp = await advanceCursor(cp, totalCount, page - 1, totalPages);
        cp = await suspendCheckpoint(cp);
        setStatus(`⏸ Network error on page ${page + 1} — reopen popup to resume.`);
        await checkResumeBanner();
        return;
      }
      throw err;
    }

    if (!result.items.length) break;
    addItems(result.items);

    // Only advance the checkpoint cursor for pages we're fetching fresh
    // (not replay pages — resuming from the old cursor is already correct)
    if (!isReplay) {
      cp = await advanceCursor(cp, totalCount, page, totalPages);
    }
  }

  // ── Step 5: Format ────────────────────────────────────────────────────
  setStatus(`Formatting ${raindrops.length.toLocaleString()} bookmarks…`);
  setProgress(totalPages, totalPages, 'Formatting…');

  const payload = { exportedAt: new Date().toISOString(), collectionId, collections, raindrops };

  const downloads = [];
  if (selectedFormats.has('json'))
    downloads.push({ content: formatJSON(payload, collMap),          name: buildFilename('raindrop-export', 'json'),      type: 'application/json' });
  if (selectedFormats.has('html')) {
    downloads.push({ content: formatNetscapeHTML(payload, collMap),  name: buildFilename('raindrop-bookmarks', 'html'),   type: 'text/html' });
    downloads.push({ content: formatRichHTML(payload, collMap),      name: buildFilename('raindrop-gallery', 'html'),     type: 'text/html' });
  }
  if (selectedFormats.has('markdown'))
    downloads.push({ content: formatMarkdown(payload, collMap),      name: buildFilename('raindrop-export', 'md'),        type: 'text/markdown' });
  if (selectedFormats.has('offline'))
    downloads.push({ content: formatOfflineSPA(payload),             name: buildFilename('raindrop-offline', 'html'),     type: 'text/html' });

  // ── Step 6: Download ──────────────────────────────────────────────────
  setStatus(`Downloading ${downloads.length} file(s)…`);
  for (const dl of downloads) {
    await triggerDownload(dl.content, dl.name, dl.type);
  }

  // ── Step 7: Done ──────────────────────────────────────────────────────
  await completeCheckpoint();
  setStatus(`✅ Done! ${raindrops.length.toLocaleString()} bookmarks → ${downloads.length} file(s) in ~/Downloads`);
  setProgress(totalPages, totalPages, 'Complete ✓');
  elBar.style.width = '100%';
}

// ── Download helper ────────────────────────────────────────────────────────
function triggerDownload(content, filename, mimeType) {
  return new Promise((resolve) => {
    const blob = new Blob([content], { type: mimeType });
    const url  = URL.createObjectURL(blob);
    chrome.downloads.download({ url, filename, saveAs: false, conflictAction: 'uniquify' }, (dlId) => {
      URL.revokeObjectURL(url);
      resolve(dlId);
    });
  });
}

// ── UI helpers ─────────────────────────────────────────────────────────────
function setTokenStatus(state, msg) { elTokenSt.textContent = msg; elTokenSt.className = 'token-status ' + state; }
function setStatus(msg) { elStatus.textContent = msg; }
function setProgress(current, total, label) {
  const pct = total > 0 ? Math.round((current / total) * 100) : 0;
  elBar.style.width = pct + '%';
  elLabel.textContent = label || `${pct}%`;
}

document.addEventListener('DOMContentLoaded', init);
