/**
 * extension-test-v2.js — v2 functional tests (cursor-only checkpoint)
 * Run: node --input-type=module < extension-test-v2.js
 */

// ── Minimal chrome API stub ────────────────────────────────────────────────
const store = {};
const sessionStore = {};
globalThis.chrome = {
  storage: {
    local: {
      get:    async (keys) => {
        if (typeof keys === 'string') return { [keys]: store[keys] };
        const res = {};
        (Array.isArray(keys) ? keys : Object.keys(keys)).forEach(k => { res[k] = store[k]; });
        return res;
      },
      set:    async (obj) => Object.assign(store, obj),
      remove: async (keys) => { (Array.isArray(keys) ? keys : [keys]).forEach(k => delete store[k]); },
    },
    session: {
      get:    async (k)   => ({ [k]: sessionStore[k] }),
      set:    async (obj) => Object.assign(sessionStore, obj),
      remove: async (k)   => { delete sessionStore[k]; },
    },
  },
};

// ── Test helpers ───────────────────────────────────────────────────────────
let passed = 0, failed = 0;
function assert(cond, msg) {
  if (cond) { console.log(`  ✓ ${msg}`); passed++; }
  else       { console.error(`  ✗ FAIL: ${msg}`); failed++; }
}
async function run(name, fn) {
  console.log(`\n=== ${name} ===`);
  try { await fn(); }
  catch(e) { console.error('  ✗ Threw:', e.message, e.stack?.split('\n')[1]||''); failed++; }
}

// ── Imports ────────────────────────────────────────────────────────────────
const {
  loadCheckpoint, startCheckpoint, advanceCursor,
  suspendCheckpoint, markResumed, completeCheckpoint, clearCheckpoint,
  isResumable, resumeLabel, acquireMutex, releaseMutex,
} = await import('./src/checkpoint.js');

const { formatOfflineSPA }  = await import('./src/formatters/offline.js');
const { AuthError, RetryableError, ApiError } = await import('./src/api.js');

// ── Checkpoint: basic lifecycle ────────────────────────────────────────────
await run('Checkpoint: start', async () => {
  const collections = [{ _id: 1, title: 'Tech', count: 10 }];
  const cp = await startCheckpoint('0', collections);
  assert(cp.version === 2,               'version is 2');
  assert(cp.status === 'fetching_pages', 'status is fetching_pages');
  assert(cp.fetchedPages === 0,          'fetchedPages starts at 0');
  assert(cp.collectionId === '0',        'collectionId stored as string');
  assert(cp.collections.length === 1,    'collections cached in checkpoint');
  assert(!isResumable(cp),               'brand-new checkpoint (0 pages) is not resumable');
});

await run('Checkpoint: advanceCursor', async () => {
  const cp0 = await loadCheckpoint();
  const cp1 = await advanceCursor(cp0, 300, 0, 6);  // page 0 done
  assert(cp1.fetchedPages === 1, 'fetchedPages = 1 after page 0');
  assert(cp1.totalCount === 300, 'totalCount stored');
  assert(cp1.totalPages === 6,   'totalPages stored');
  assert(isResumable(cp1),       'checkpoint with 1 page is resumable');

  const cp2 = await advanceCursor(cp1, 300, 3, 6);  // page 3 done
  assert(cp2.fetchedPages === 4, 'fetchedPages = 4 after page 3');

  // Confirm storage only holds the cursor key (no raindrop keys)
  const keys = Object.keys(store);
  const raindropKeys = keys.filter(k => k.startsWith('exportRaindrops'));
  assert(raindropKeys.length === 0, `NO raindrop data written to storage (${raindropKeys.length} keys)`);
});

await run('Checkpoint: suspend + isResumable + resumeLabel', async () => {
  const cp0 = await loadCheckpoint();
  const cp1 = await suspendCheckpoint(cp0);
  assert(cp1.status === 'suspended', 'suspended status set');
  assert(isResumable(cp1),           'suspended checkpoint is resumable');
  const lbl = resumeLabel(cp1);
  assert(lbl.includes('%'),    `label includes percentage: ${lbl}`);
  assert(lbl.includes('300'), `label mentions totalCount: ${lbl}`);
});

await run('Checkpoint: markResumed', async () => {
  const cp0 = await loadCheckpoint();
  const cp1 = await markResumed(cp0);
  assert(cp1.resumedAt !== null,         'resumedAt timestamp set');
  assert(cp1.status === 'fetching_pages','status reset to fetching_pages');
});

await run('Checkpoint: completeCheckpoint clears key', async () => {
  await completeCheckpoint();
  const cp = await loadCheckpoint();
  assert(cp === null, 'checkpoint gone after complete');
  const keys = Object.keys(store);
  assert(keys.length === 0, `storage fully empty after complete (${keys.length} keys)`);
});

await run('Checkpoint: isResumable edge cases', () => {
  assert(!isResumable(null),                          'null → not resumable');
  assert(!isResumable({ status: 'done' }),            'done → not resumable');
  assert(!isResumable({ status: 'fetching_pages', fetchedPages: 0 }), '0 pages → not resumable');
  assert( isResumable({ status: 'fetching_pages', fetchedPages: 5 }), '5 pages → resumable');
  assert( isResumable({ status: 'suspended',      fetchedPages: 1 }), 'suspended+1page → resumable');
});

await run('Checkpoint: mutex prevents double-run', async () => {
  const a = await acquireMutex();
  const b = await acquireMutex();   // second acquire should fail
  assert(a === true,  'first acquire succeeds');
  assert(b === false, 'second acquire blocked');
  await releaseMutex();
  const c = await acquireMutex();
  assert(c === true,  'acquire succeeds after release');
  await releaseMutex();
});

// ── Offline SPA ────────────────────────────────────────────────────────────
await run('Offline SPA: basic structure', () => {
  const payload = {
    exportedAt: new Date().toISOString(),
    collections: [{ _id: 1, title: 'Dev', count: 2, color: '#0066ff' }],
    raindrops: [
      { _id: 1, title: 'GitHub', link: 'https://github.com', domain: 'github.com',
        excerpt: 'Code hosting', tags: ['dev','git'], important: false, broken: false,
        created: '2024-01-01T00:00:00Z', collection: { $id: 1 }, highlights: [], media: [] },
      { _id: 2, title: '<script>XSS</script>', link: 'https://evil.com', domain: 'evil.com',
        excerpt: '', tags: [], important: true, broken: true,
        created: '2024-02-01T00:00:00Z', collection: { $id: 1 }, highlights: [], media: [] },
    ],
  };
  const html = formatOfflineSPA(payload);
  assert(html.startsWith('<!DOCTYPE html>'),          'valid DOCTYPE');
  assert(html.includes('</html>'),                    'has closing tag');
  assert(html.includes('const D='),                   'embedded data present');
  assert(html.includes('GitHub'),                     'bookmark title present');
  assert(!html.includes('<script>XSS</script>'),      'raw <script> tag absent');
  assert(html.includes('\\u003cscript\\u003e') || html.includes('\\u003c'), 'XSS unicode-escaped');
  assert(html.includes('Dev'),                        'collection name present');
  assert(html.includes('github.com'),                 'domain present');
  assert(html.includes('--bg'),                       'CSS custom properties present');
  assert(html.includes('prefers-color-scheme'),       'dark-mode media query present');
});

await run('Offline SPA: highlights + media', () => {
  const payload = {
    exportedAt: new Date().toISOString(),
    collections: [],
    raindrops: [{
      _id: 10, title: 'Annotated', link: 'https://example.com', domain: 'example.com',
      excerpt: '', tags: ['research'], important: false, broken: false,
      created: '2024-03-01T00:00:00Z', collection: { $id: -1 },
      highlights: [
        { _id: 'h1', text: 'Key insight', color: 'yellow', note: 'Important!' },
        { _id: 'h2', text: 'Another point', color: 'blue', note: '' },
      ],
      media: [{ link: 'https://example.com/img.jpg' }],
    }],
  };
  const html = formatOfflineSPA(payload);
  assert(html.includes('.hl-yellow'), 'yellow highlight CSS');
  assert(html.includes('.hl-blue'),   'blue highlight CSS');
  assert(html.includes('"Key insight"') || html.includes('Key insight'), 'highlight text in data');
});

await run('Offline SPA: 1000 items performance', () => {
  const raindrops = Array.from({ length: 1000 }, (_, i) => ({
    _id: i + 1, title: `Bookmark ${i + 1}`, link: `https://example.com/${i}`,
    domain: 'example.com', excerpt: `Excerpt ${i}`,
    tags: [`tag${i % 20}`], important: i % 7 === 0, broken: i % 50 === 0,
    created: new Date(Date.now() - i * 3600000).toISOString(),
    collection: { $id: (i % 5) + 1 }, highlights: [], media: [],
  }));
  const collections = Array.from({ length: 5 }, (_, i) => ({ _id: i+1, title: `Col ${i+1}`, count: 200 }));
  const t0 = Date.now();
  const html = formatOfflineSPA({ exportedAt: new Date().toISOString(), collections, raindrops });
  const ms = Date.now() - t0;
  assert(html.includes('"total":1000'),         '1000 items in embedded data');
  assert(html.length > 50_000,                 `output > 50KB (${(html.length/1024).toFixed(0)}KB)`);
  assert(ms < 3000,                            `formatted in < 3s (${ms}ms)`);
});

// ── Error types ────────────────────────────────────────────────────────────
await run('Error types', () => {
  const ae = new AuthError('bad token');
  assert(ae instanceof AuthError,    'AuthError instanceof');
  assert(ae.name === 'AuthError',    'AuthError.name');
  const re = new RetryableError('timeout');
  assert(re instanceof RetryableError, 'RetryableError instanceof');
  const ap = new ApiError('404', 404);
  assert(ap.status === 404,           'ApiError.status');
});

// ── Summary ────────────────────────────────────────────────────────────────
console.log(`\n${'─'.repeat(52)}`);
console.log(`Tests: ${passed + failed}   ✓ ${passed}   ✗ ${failed}`);
if (failed === 0) console.log('✅ ALL V2 TESTS PASSED');
else { console.error(`❌ ${failed} test(s) FAILED`); process.exit(1); }
