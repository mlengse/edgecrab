/**
 * formatters/json.js — JSON export formatter
 * Single Responsibility: convert raw API data to a JSON string
 */

/**
 * Format the export payload as pretty-printed JSON.
 * @param {object} payload  { exportedAt, collections, raindrops }
 * @param {Map}    collectionMap
 * @returns {string}  JSON string
 */
export function formatJSON(payload, collectionMap) {
  const output = {
    exportedAt: payload.exportedAt,
    version: '1.0.0',
    totalRaindrops: payload.raindrops.length,
    totalCollections: payload.collections.length,
    collections: payload.collections.map((c) => ({
      _id: c._id,
      title: c.title,
      count: c.count,
      cover: c.cover?.[0] || null,
      color: c.color || null,
      parent: c.parent?.$id ?? null,
      public: c.public || false,
      created: c.created,
      lastUpdate: c.lastUpdate,
    })),
    raindrops: payload.raindrops.map((rd) => ({
      _id: rd._id,
      title: rd.title || '',
      link: rd.link || '',
      excerpt: rd.excerpt || '',
      note: rd.note || '',
      cover: rd.cover || null,
      media: rd.media || [],
      tags: rd.tags || [],
      type: rd.type || 'link',
      domain: rd.domain || '',
      created: rd.created,
      lastUpdate: rd.lastUpdate,
      important: rd.important || false,
      broken: rd.broken || false,
      highlights: (rd.highlights || []).map((h) => ({
        _id: h._id,
        text: h.text,
        color: h.color || 'yellow',
        note: h.note || '',
        created: h.created,
      })),
      collection: {
        $id: rd.collection?.$id ?? -1,
        title: collectionMap.get(rd.collection?.$id)?.title || 'Unsorted',
      },
    })),
  };

  return JSON.stringify(output, null, 2);
}
