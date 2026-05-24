/**
 * cdp-load-extension.js
 * Loads unpacked extension into Chrome via the Chrome DevTools Protocol.
 * Uses the experimental loadUnpackedExtension command.
 */

const WebSocket = require('ws');
const http = require('http');

const EXT_PATH = '/Users/raphaelmansuy/Github/03-working/edgecrab/extension';

async function getDebuggerUrl() {
  return new Promise((resolve, reject) => {
    http.get('http://localhost:9222/json/version', (res) => {
      let body = '';
      res.on('data', d => body += d);
      res.on('end', () => {
        const data = JSON.parse(body);
        resolve(data.webSocketDebuggerUrl);
      });
    }).on('error', reject);
  });
}

async function cdpCommand(ws, method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = Math.floor(Math.random() * 100000);
    const msg = JSON.stringify({ id, method, params });
    const handler = (data) => {
      const resp = JSON.parse(data);
      if (resp.id === id) {
        ws.removeListener('message', handler);
        if (resp.error) reject(new Error(`CDP Error: ${resp.error.message}`));
        else resolve(resp.result);
      }
    };
    ws.on('message', handler);
    ws.send(msg);
    setTimeout(() => { ws.removeListener('message', handler); reject(new Error('timeout')); }, 8000);
  });
}

async function main() {
  const wsUrl = await getDebuggerUrl();
  console.log('Connecting to CDP:', wsUrl);

  const ws = new WebSocket(wsUrl);
  await new Promise((r, e) => { ws.on('open', r); ws.on('error', e); });
  console.log('Connected to Chrome CDP');

  try {
    // Load unpacked extension
    const result = await cdpCommand(ws, 'Extensions.loadUnpacked', { path: EXT_PATH });
    console.log('✅ Extension loaded!', JSON.stringify(result, null, 2));
  } catch (err) {
    // Try alternative method
    console.log('loadUnpacked failed:', err.message);
    console.log('Trying via chrome.management...');
    
    // Get the page tab to execute management API
    const tabs = await new Promise((resolve, reject) => {
      http.get('http://localhost:9222/json', (res) => {
        let body = '';
        res.on('data', d => body += d);
        res.on('end', () => resolve(JSON.parse(body)));
      }).on('error', reject);
    });
    
    const pageTab = tabs.find(t => t.type === 'page' && t.url.startsWith('http'));
    if (pageTab) {
      const pageWs = new WebSocket(pageTab.webSocketDebuggerUrl);
      await new Promise((r, ej) => { pageWs.on('open', r); pageWs.on('error', ej); });
      
      try {
        const evalResult = await cdpCommand(pageWs, 'Runtime.evaluate', {
          expression: `chrome.management.getAll(function(exts){ console.log(JSON.stringify(exts.map(e=>({id:e.id,name:e.name,enabled:e.enabled})))); })`,
          awaitPromise: false,
        });
        console.log('Management eval result:', JSON.stringify(evalResult, null, 2));
      } catch(e2) {
        console.log('management eval also failed:', e2.message);
      }
      pageWs.close();
    }
  } finally {
    ws.close();
  }
}

main().catch(console.error);
