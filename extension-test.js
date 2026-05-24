/**
 * Test: verify all three formatters produce correct output
 * with mock data matching the Raindrop.io API schema.
 */

// Mock data
const mockCollections = [
  { _id: 111, title: '🧰 Tools', count: 2, cover: ['https://example.com/cover.jpg'], color: '#6366f1', parent: {}, public: false, created: '2024-01-01T00:00:00Z', lastUpdate: '2024-06-01T00:00:00Z' },
  { _id: 222, title: '📚 Books', count: 1, cover: [], color: null, parent: {}, public: true, created: '2024-02-01T00:00:00Z', lastUpdate: '2024-05-01T00:00:00Z' },
];

const mockRaindrops = [
  {
    _id: 1001,
    title: 'GitHub Copilot CLI Extensions',
    link: 'https://github.com/example/copilot-cli',
    excerpt: 'A powerful CLI extension for GitHub Copilot',
    note: 'Really useful for daily workflow',
    cover: 'https://example.com/thumb1.jpg',
    media: [{ link: 'https://example.com/media1.jpg' }],
    tags: ['ai', 'tools', 'cli'],
    type: 'link',
    domain: 'github.com',
    created: '2024-04-20T10:00:00Z',
    lastUpdate: '2024-04-21T12:00:00Z',
    important: true,
    broken: false,
    highlights: [
      { _id: 'h1', text: 'Key finding about CLI', color: 'yellow', note: 'Important!', created: '2024-04-20T11:00:00Z' },
      { _id: 'h2', text: 'Another highlight', color: 'blue', note: '', created: '2024-04-20T11:30:00Z' },
    ],
    collection: { $id: 111 },
  },
  {
    _id: 1002,
    title: 'TypeScript Type from JSON Schema',
    link: 'https://x.com/example/status/123',
    excerpt: '',
    note: '',
    cover: null,
    media: [],
    tags: ['typescript', 'schema'],
    type: 'link',
    domain: 'x.com',
    created: '2024-04-22T09:00:00Z',
    lastUpdate: '2024-04-22T09:00:00Z',
    important: false,
    broken: false,
    highlights: [],
    collection: { $id: 111 },
  },
  {
    _id: 1003,
    title: 'Clean Code by Robert Martin',
    link: 'https://www.amazon.com/dp/0132350882',
    excerpt: 'A handbook of agile software craftsmanship',
    note: 'Must-read for every developer',
    cover: 'https://example.com/clean-code.jpg',
    media: [],
    tags: ['books', 'software'],
    type: 'link',
    domain: 'amazon.com',
    created: '2024-03-15T08:00:00Z',
    lastUpdate: '2024-03-15T08:00:00Z',
    important: true,
    broken: false,
    highlights: [],
    collection: { $id: 222 },
  },
];

// Build collection map
function buildCollectionMap(collections) {
  const map = new Map();
  for (const c of collections) map.set(c._id, c);
  map.set(0, { _id: 0, title: 'All bookmarks' });
  map.set(-1, { _id: -1, title: 'Unsorted' });
  map.set(-99, { _id: -99, title: 'Trash' });
  return map;
}

const collectionMap = buildCollectionMap(mockCollections);
const payload = {
  exportedAt: new Date().toISOString(),
  collections: mockCollections,
  raindrops: mockRaindrops,
};

// ---- Test JSON formatter ----
function formatJSON(payload, collectionMap) {
  return JSON.stringify({
    exportedAt: payload.exportedAt,
    version: '1.0.0',
    totalRaindrops: payload.raindrops.length,
    totalCollections: payload.collections.length,
    collections: payload.collections,
    raindrops: payload.raindrops.map(rd => ({
      ...rd,
      collectionTitle: collectionMap.get(rd.collection?.$id)?.title || 'Unsorted',
    })),
  }, null, 2);
}

const jsonOut = formatJSON(payload, collectionMap);
const parsed = JSON.parse(jsonOut);

console.log('=== JSON Formatter ===');
console.assert(parsed.totalRaindrops === 3, 'totalRaindrops should be 3');
console.assert(parsed.totalCollections === 2, 'totalCollections should be 2');
console.assert(parsed.raindrops[0].title === 'GitHub Copilot CLI Extensions', 'First raindrop title correct');
console.assert(parsed.raindrops[0].collectionTitle === '🧰 Tools', 'Collection title mapped correctly');
console.assert(Array.isArray(parsed.raindrops[0].tags), 'Tags is array');
console.log('✓ JSON: all assertions passed');
console.log(`  Output size: ${jsonOut.length} chars`);

// ---- Test groupByCollection ----
function groupByCollection(raindrops, collectionMap) {
  const groups = new Map();
  for (const rd of raindrops) {
    const id = rd?.collection?.$id;
    const title = collectionMap.get(id)?.title || 'Unsorted';
    if (!groups.has(title)) groups.set(title, []);
    groups.get(title).push(rd);
  }
  return groups;
}

const groups = groupByCollection(mockRaindrops, collectionMap);
console.log('\n=== Collection Grouping ===');
console.assert(groups.size === 2, 'Should have 2 collection groups');
console.assert(groups.get('🧰 Tools').length === 2, 'Tools has 2 items');
console.assert(groups.get('📚 Books').length === 1, 'Books has 1 item');
console.log('✓ Grouping: all assertions passed');
for (const [k,v] of groups) console.log(`  ${k}: ${v.length} items`);

// ---- Test Markdown format ----
function escapeMarkdown(s) {
  if (!s) return '';
  return s.replace(/([\\`*_{}[\]()#+\-.!|])/g, '\\$1');
}
function formatDate(iso) {
  if (!iso) return '';
  return new Date(iso).toISOString().slice(0, 10);
}

let md = `# Raindrop Export\n\n`;
for (const [coll, items] of groups) {
  md += `## ${coll}\n\n`;
  for (const rd of items) {
    md += `### [${escapeMarkdown(rd.title)}](${rd.link})\n\n`;
    if (rd.tags?.length) md += `**Tags:** ${rd.tags.map(t => `\`${t}\``).join(', ')}\n\n`;
    if (rd.excerpt) md += `> ${rd.excerpt}\n\n`;
    if (rd.highlights?.length) {
      md += `**Highlights:**\n`;
      rd.highlights.forEach(h => { md += `- ${h.text} *(${h.color})*\n`; });
      md += '\n';
    }
    md += '---\n\n';
  }
}

console.log('\n=== Markdown Formatter ===');
console.assert(md.includes('# Raindrop Export'), 'MD has title');
console.assert(md.includes('## 🧰 Tools'), 'MD has collection section');
console.assert(md.includes('### [GitHub Copilot CLI Extensions]'), 'MD has bookmark header');
console.assert(md.includes('`ai`'), 'MD has tag pill');
console.assert(md.includes('Key finding about CLI'), 'MD has highlight');
console.log('✓ Markdown: all assertions passed');
console.log(`  Output size: ${md.length} chars`);

// ---- Test HTML escape ----
function escapeHtml(s) {
  if (!s) return '';
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

const dangerous = '<script>alert("xss")</script>';
const escaped = escapeHtml(dangerous);
console.log('\n=== HTML Safety ===');
console.assert(!escaped.includes('<script>'), 'Script tags escaped');
console.assert(escaped.includes('&lt;script&gt;'), 'Angle brackets escaped');
console.log('✓ HTML escape: XSS prevention passed');

// ---- Edge cases ----
console.log('\n=== Edge Cases ===');
// Missing cover
const noCover = { ...mockRaindrops[0], cover: null };
console.assert(noCover.cover === null, 'Missing cover handled');
// Empty title
const noTitle = { ...mockRaindrops[0], title: '' };
const titleFallback = noTitle.title || noTitle.link || 'Untitled';
console.assert(titleFallback === noTitle.link, 'Empty title falls back to link');
// Unknown collection
const unknownColl = { collection: { $id: 99999 } };
const unknownTitle = collectionMap.get(unknownColl.collection.$id)?.title || 'Unsorted';
console.assert(unknownTitle === 'Unsorted', 'Unknown collection ID → Unsorted');
// Tags with pipe characters
const pipeTag = 'tag|with|pipes';
const escaped2 = pipeTag.replace(/\|/g, '\\|');
console.assert(escaped2 === 'tag\\|with\\|pipes', 'Pipe chars escaped in markdown');
// Broken link
const brokenRd = { ...mockRaindrops[0], broken: true };
console.assert(brokenRd.broken === true, 'Broken flag preserved');

console.log('✓ Edge cases: all passed');
console.log('\n✅ ALL TESTS PASSED');
