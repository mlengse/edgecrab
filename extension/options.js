/**
 * options.js — Settings persistence
 * Single Responsibility: load/save extension settings via chrome.storage.sync
 */

const DEFAULTS = {
  inclUnsorted: true,
  inclTrash: false,
  richHtml: true,
  perPage: 50,
  rememberToken: true,
};

const fields = {
  tokenInput:    { el: null, type: 'text',     key: 'token',         storage: 'local' },
  rememberToken: { el: null, type: 'checkbox', key: 'rememberToken', storage: 'sync' },
  inclUnsorted:  { el: null, type: 'checkbox', key: 'inclUnsorted',  storage: 'sync' },
  inclTrash:     { el: null, type: 'checkbox', key: 'inclTrash',     storage: 'sync' },
  richHtml:      { el: null, type: 'checkbox', key: 'richHtml',      storage: 'sync' },
  perPage:       { el: null, type: 'number',   key: 'perPage',       storage: 'sync' },
};

async function init() {
  // Bind DOM
  for (const [id, field] of Object.entries(fields)) {
    field.el = document.getElementById(id);
  }

  // Load values
  const syncData  = await chrome.storage.sync.get(Object.keys(DEFAULTS));
  const localData = await chrome.storage.local.get('token');

  for (const [id, field] of Object.entries(fields)) {
    const value = field.storage === 'local'
      ? localData[field.key]
      : (syncData[field.key] ?? DEFAULTS[field.key]);

    if (value === undefined || value === null) continue;

    if (field.type === 'checkbox') {
      field.el.checked = Boolean(value);
    } else {
      field.el.value = value;
    }
  }

  document.getElementById('btnSave').addEventListener('click', onSave);
}

async function onSave() {
  const syncUpdate  = {};
  const localUpdate = {};

  for (const [, field] of Object.entries(fields)) {
    let value;
    if (field.type === 'checkbox') {
      value = field.el.checked;
    } else if (field.type === 'number') {
      value = Math.max(1, Math.min(50, parseInt(field.el.value, 10) || 50));
    } else {
      value = field.el.value.trim();
    }

    if (field.storage === 'local') {
      localUpdate[field.key] = value;
    } else {
      syncUpdate[field.key] = value;
    }
  }

  // If "remember token" is unchecked, don't persist token
  if (!document.getElementById('rememberToken').checked) {
    delete localUpdate.token;
  }

  await Promise.all([
    chrome.storage.sync.set(syncUpdate),
    chrome.storage.local.set(localUpdate),
  ]);

  const status = document.getElementById('saveStatus');
  status.textContent = '✓ Settings saved!';
  status.className = 'status ok';
  setTimeout(() => { status.textContent = ''; }, 2000);
}

init().catch(console.error);
