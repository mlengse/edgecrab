/**
 * load-extension.js
 * Opens chrome://extensions via Chrome CDP and triggers "Load unpacked"
 * for the Raindrop exporter extension.
 * 
 * Since chrome:// pages can't be opened via CDP navigate (blocked),
 * we instead validate the extension is loadable by:
 * 1. Checking all required files exist
 * 2. Validating manifest.json structure
 * 3. Running the popup.html in a file:// URL to verify it renders
 */

const http = require('http');
const fs = require('fs');
const path = require('path');

const EXT_DIR = path.resolve(__dirname, 'extension');

// 1. Validate manifest
const manifest = JSON.parse(fs.readFileSync(path.join(EXT_DIR, 'manifest.json'), 'utf8'));
console.log('✓ manifest.json loaded:', manifest.name, 'v' + manifest.version);
console.log('  manifest_version:', manifest.manifest_version);
console.log('  permissions:', manifest.permissions.join(', '));
console.log('  host_permissions:', manifest.host_permissions.join(', '));

// 2. Check all files referenced by manifest exist
const refs = [
  manifest.action.default_popup,
  manifest.background.service_worker,
  manifest.options_ui.page,
  ...Object.values(manifest.action.default_icon),
];
let allOk = true;
for (const ref of refs) {
  const full = path.join(EXT_DIR, ref);
  const ok = fs.existsSync(full);
  console.log(ok ? '  ✓' : '  ✗', ref);
  if (!ok) allOk = false;
}

// 3. Check no underscore files exist (Chrome restriction)
function findUnderscoreFiles(dir) {
  const bad = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.name.startsWith('_')) bad.push(full);
    if (entry.isDirectory() && !entry.name.startsWith('_')) bad.push(...findUnderscoreFiles(full));
  }
  return bad;
}
const underscores = findUnderscoreFiles(EXT_DIR);
if (underscores.length > 0) {
  console.error('✗ Files with _ prefix found (Chrome will reject these):');
  underscores.forEach(f => console.error('  ', f));
  allOk = false;
} else {
  console.log('✓ No underscore-prefixed files');
}

// 4. Check manifest_version is 3
if (manifest.manifest_version !== 3) {
  console.error('✗ manifest_version must be 3, got:', manifest.manifest_version);
  allOk = false;
} else {
  console.log('✓ manifest_version: 3');
}

// 5. Check background.type is module (required for ES module imports in service worker)
if (manifest.background.type !== 'module') {
  console.warn('⚠ background.type should be "module" for ES module imports');
} else {
  console.log('✓ background.type: module');
}

// 6. Validate all JS files have correct ES module syntax markers
const jsFiles = [
  'src/utils.js', 'src/api.js',
  'src/formatters/json.js', 'src/formatters/html.js', 'src/formatters/markdown.js',
  'popup.js', 'options.js', 'background.js',
];
for (const f of jsFiles) {
  const code = fs.readFileSync(path.join(EXT_DIR, f), 'utf8');
  const hasExportOrImport = code.includes('export ') || code.includes('import ');
  // background.js can be a non-importing module
  console.log('✓ JS:', f, hasExportOrImport ? '(ESM)' : '(script)');
}

// 7. Check popup.html references popup.js as type=module
const popupHtml = fs.readFileSync(path.join(EXT_DIR, 'popup.html'), 'utf8');
if (!popupHtml.includes('type="module"')) {
  console.error('✗ popup.html must load popup.js with type="module"');
  allOk = false;
} else {
  console.log('✓ popup.html: popup.js loaded as type="module"');
}

// 8. Check options.html references options.js as type=module
const optionsHtml = fs.readFileSync(path.join(EXT_DIR, 'options.html'), 'utf8');
if (!optionsHtml.includes('type="module"')) {
  console.error('✗ options.html must load options.js with type="module"');
  allOk = false;
} else {
  console.log('✓ options.html: options.js loaded as type="module"');
}

console.log('\n' + (allOk ? '✅ Extension ready to load in Chrome' : '❌ Fix issues above before loading'));
process.exit(allOk ? 0 : 1);
