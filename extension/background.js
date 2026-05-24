/**
 * background.js — Service worker
 * Responsibilities:
 *   - Receive download requests from popup
 *   - Trigger chrome.downloads.download()
 *   - Keep alive during long exports via alarms
 *   - Route status messages back to popup
 */

// ---------------------------------------------------------------------------
// Keepalive alarm (prevents MV3 service worker from terminating mid-export)
// ---------------------------------------------------------------------------

chrome.alarms.create('keepalive', { periodInMinutes: 0.4 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'keepalive') {
    // No-op: just prevents termination
  }
});

// ---------------------------------------------------------------------------
// Message handler
// ---------------------------------------------------------------------------

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.action === 'download') {
    handleDownload(msg)
      .then((result) => sendResponse({ ok: true, result }))
      .catch((err) => sendResponse({ ok: false, error: err.message }));
    return true; // async response
  }

  if (msg.action === 'openOptions') {
    chrome.runtime.openOptionsPage();
    sendResponse({ ok: true });
    return false;
  }
});

// ---------------------------------------------------------------------------
// Download handler
// ---------------------------------------------------------------------------

/**
 * Trigger a file download from a content string.
 * @param {object} msg  { content, filename, mimeType }
 */
async function handleDownload({ content, filename, mimeType }) {
  // Build a data URL (avoids blob URL cross-origin issues in service workers)
  const encoded = encodeURIComponent(content);
  const dataUrl = `data:${mimeType};charset=utf-8,${encoded}`;

  return new Promise((resolve, reject) => {
    chrome.downloads.download(
      {
        url: dataUrl,
        filename: filename, // relative path inside ~/Downloads or user-set dir
        saveAs: false,
        conflictAction: 'uniquify',
      },
      (downloadId) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else {
          resolve(downloadId);
        }
      }
    );
  });
}
