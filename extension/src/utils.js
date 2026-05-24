/**
 * utils.js — Pure helper functions (no side effects, no imports)
 * Single Responsibility: data transformation utilities
 */

/**
 * Format an ISO date string to YYYY-MM-DD.
 * @param {string} iso
 * @returns {string}
 */
export function formatDate(iso) {
  if (!iso) return '';
  try {
    return new Date(iso).toISOString().slice(0, 10);
  } catch {
    return iso;
  }
}

/**
 * Format an ISO date string to a human-readable string.
 * @param {string} iso
 * @returns {string}
 */
export function formatDateTime(iso) {
  if (!iso) return '';
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

/**
 * Sanitize a string to be safe as a filename.
 * @param {string} name
 * @returns {string}
 */
export function sanitizeFilename(name) {
  return name
    .replace(/[/\\?%*:|"<>]/g, '-')
    .replace(/\s+/g, '_')
    .replace(/-+/g, '-')
    .slice(0, 100);
}

/**
 * Escape a string for safe HTML output.
 * @param {string} str
 * @returns {string}
 */
export function escapeHtml(str) {
  if (!str) return '';
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Escape special Markdown characters in a string.
 * @param {string} str
 * @returns {string}
 */
export function escapeMarkdown(str) {
  if (!str) return '';
  return str.replace(/([\\`*_{}[\]()#+\-.!|])/g, '\\$1');
}

/**
 * Escape pipe chars in a Markdown table cell.
 * @param {string} str
 * @returns {string}
 */
export function escapeMarkdownTable(str) {
  if (!str) return '';
  return String(str).replace(/\|/g, '\\|').replace(/\n/g, ' ');
}

/**
 * Build a filename with timestamp, e.g. "raindrop-export_2026-04-25T12-00-00.json"
 * @param {string} base
 * @param {string} ext
 * @returns {string}
 */
export function buildFilename(base, ext) {
  const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  return `${base}_${ts}.${ext}`;
}

/**
 * Build a lookup map of collection _id → collection object.
 * @param {Array} collections
 * @returns {Map<number, object>}
 */
export function buildCollectionMap(collections) {
  const map = new Map();
  for (const c of collections) {
    map.set(c._id, c);
  }
  // Add system collections
  map.set(0,   { _id: 0,   title: 'All bookmarks' });
  map.set(-1,  { _id: -1,  title: 'Unsorted' });
  map.set(-99, { _id: -99, title: 'Trash' });
  return map;
}

/**
 * Get collection title for a raindrop.
 * @param {object} raindrop
 * @param {Map} collectionMap
 * @returns {string}
 */
export function getCollectionTitle(raindrop, collectionMap) {
  const id = raindrop?.collection?.$id;
  if (id === undefined || id === null) return 'Unsorted';
  return collectionMap.get(id)?.title || `Collection #${id}`;
}

/**
 * Group an array of raindrops by collection title.
 * @param {Array} raindrops
 * @param {Map} collectionMap
 * @returns {Map<string, Array>}
 */
export function groupByCollection(raindrops, collectionMap) {
  const groups = new Map();
  for (const rd of raindrops) {
    const title = getCollectionTitle(rd, collectionMap);
    if (!groups.has(title)) groups.set(title, []);
    groups.get(title).push(rd);
  }
  return groups;
}

/**
 * Convert Unix timestamp (seconds) to ISO string.
 * Used for Netscape HTML bookmark format.
 * @param {string} iso
 * @returns {number}
 */
export function toUnixTimestamp(iso) {
  if (!iso) return 0;
  try {
    return Math.floor(new Date(iso).getTime() / 1000);
  } catch {
    return 0;
  }
}

/**
 * Highlight color → CSS hex colour map.
 */
export const HIGHLIGHT_COLORS = {
  yellow:  '#fef9c3',
  blue:    '#dbeafe',
  brown:   '#fef3c7',
  cyan:    '#cffafe',
  gray:    '#f3f4f6',
  green:   '#dcfce7',
  indigo:  '#e0e7ff',
  orange:  '#ffedd5',
  pink:    '#fce7f3',
  purple:  '#f3e8ff',
  red:     '#fee2e2',
  teal:    '#ccfbf1',
};
