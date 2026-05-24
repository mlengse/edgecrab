/**
 * formatters/offline.js — Self-contained offline SPA
 *
 * Produces a single HTML file that replicates the Raindrop.io UX:
 *  - Sidebar: collection navigation + counts
 *  - List / Card (grid) / Compact view modes with persistent preference
 *  - Full-text search (tokenised, no external deps)
 *  - Tag filtering, sort (date/title/domain), favourites/broken toggles
 *  - Slide-in detail panel: highlights, notes, media, all fields
 *  - Dark mode (auto via prefers-color-scheme + manual toggle)
 *  - Keyboard shortcuts: / = search, Esc = close/clear
 *  - Virtual pagination (50 items, "Load more")
 *  - Zero external dependencies — fully offline, file:// safe
 *
 * Single Responsibility: convert API payload → offline SPA HTML string.
 */

import { escapeHtml, buildCollectionMap, getCollectionTitle } from '../utils.js';

export function formatOfflineSPA(payload) {
  const { exportedAt, collections, raindrops } = payload;
  const collMap = buildCollectionMap(collections);

  const enriched = raindrops.map((rd) => ({
    ...rd,
    _ct: getCollectionTitle(rd, collMap),
    _cid: rd.collection?.$id ?? -1,
  }));

  const allTags = [...new Set(enriched.flatMap((r) => r.tags || []))].sort();

  // JSON-encode with < > & escaped to Unicode escapes so no raw HTML tags
  // appear in the embedded <script> block (defence-in-depth XSS mitigation).
  const DATA = JSON.stringify({
    exportedAt,
    total: enriched.length,
    collections: collections.map((c) => ({
      _id: c._id, title: c.title, count: c.count || 0,
      color: c.color || null,
    })),
    raindrops: enriched,
    allTags,
  }).replace(/</g, '\\u003c').replace(/>/g, '\\u003e').replace(/&/g, '\\u0026');

  return `<!DOCTYPE html>
<html lang="en" data-theme="auto">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Raindrop — Offline Archive</title>
<style>${CSS}</style>
</head>
<body>
<div id="app">
  <nav id="sidebar" aria-label="Collections">
    <div class="sb-head">
      <div class="sb-logo">🔖 <strong>Raindrop</strong></div>
      <button id="btnTheme" class="ico-btn" title="Toggle theme">◑</button>
    </div>
    <div id="collList" class="coll-list"></div>
    <div class="sb-foot" id="sbFoot"></div>
  </nav>
  <div id="main">
    <div id="toolbar">
      <div class="search-row">
        <label class="search-box">
          <span class="si">🔍</span>
          <input id="q" type="search" placeholder="Search… (press / to focus)" autocomplete="off" spellcheck="false">
          <button id="btnClearQ" class="ico-btn" style="display:none">✕</button>
        </label>
      </div>
      <div class="ctrl-row">
        <div class="filter-grp">
          <button id="fFav"    class="flt-btn" title="Favourites">⭐</button>
          <button id="fBroken" class="flt-btn" title="Broken links">⚠️</button>
        </div>
        <select id="sortSel" class="sort-sel">
          <option value="nd">Newest first</option>
          <option value="na">Oldest first</option>
          <option value="ta">Title A→Z</option>
          <option value="da">Domain A→Z</option>
        </select>
        <div class="view-grp">
          <button id="vList"    class="v-btn" data-v="list"    title="List">☰</button>
          <button id="vGrid"    class="v-btn" data-v="grid"    title="Cards">⊞</button>
          <button id="vCompact" class="v-btn" data-v="compact" title="Compact">≡</button>
        </div>
      </div>
    </div>
    <div id="tagStrip"></div>
    <div id="content">
      <div id="collHead"></div>
      <div id="countLine"></div>
      <div id="list"></div>
      <button id="btnMore" class="btn-more" style="display:none">Load more</button>
    </div>
  </div>
  <div id="overlay" class="overlay" style="display:none" role="dialog" aria-modal="true">
    <div id="panel" class="panel">
      <button id="btnClose" class="ico-btn panel-close">✕</button>
      <div id="panelBody"></div>
    </div>
  </div>
</div>
<script>
const D=${DATA};
const PG=50;
const S={view:ls('rd_v')||'list',sort:ls('rd_s')||'nd',coll:'all',tag:null,q:'',fav:false,brk:false,page:1};

// ── search index ──────────────────────────────────────────────────────────
function buildIdx(){
  return D.raindrops.map((r,i)=>({
    i,
    s:[r.title,r.excerpt,r.note,r.domain,r._ct,...(r.tags||[])].filter(Boolean).join(' ').toLowerCase()
  }));
}
let IDX=buildIdx();

function search(q){
  if(!q)return D.raindrops;
  const terms=q.toLowerCase().split(/\\s+/).filter(Boolean);
  return IDX.filter(({s})=>terms.every(t=>s.includes(t))).map(({i})=>D.raindrops[i]);
}

// ── filtering + sorting ───────────────────────────────────────────────────
function getFiltered(){
  let items=S.q?search(S.q):[...D.raindrops];
  if(S.coll!=='all') items=items.filter(r=>r._cid===S.coll);
  if(S.tag) items=items.filter(r=>(r.tags||[]).includes(S.tag));
  if(S.fav) items=items.filter(r=>r.important);
  if(S.brk) items=items.filter(r=>r.broken);
  switch(S.sort){
    case'na': items.sort((a,b)=>new Date(a.created)-new Date(b.created)); break;
    case'ta': items.sort((a,b)=>(a.title||'').localeCompare(b.title||'')); break;
    case'da': items.sort((a,b)=>(a.domain||'').localeCompare(b.domain||'')); break;
    default:  items.sort((a,b)=>new Date(b.created)-new Date(a.created));
  }
  return items;
}

// ── sidebar ───────────────────────────────────────────────────────────────
function buildSidebar(){
  const el=document.getElementById('collList');
  const sys=[
    {_id:'all',label:'📚 All bookmarks',n:D.raindrops.length},
    {_id:-1, label:'📥 Unsorted',       n:D.raindrops.filter(r=>r._cid===-1).length},
    {_id:-99,label:'🗑️ Trash',          n:D.raindrops.filter(r=>r._cid===-99).length},
  ];
  const usr=[...D.collections].sort((a,b)=>a.title.localeCompare(b.title));
  let h='<div class="cs">';
  sys.forEach(c=>h+=ci(c._id,c.label,c.n));
  h+='</div><div class="cd"></div><div class="cs">';
  usr.forEach(c=>h+=ci(c._id,c.title,c.count));
  h+='</div>';
  el.innerHTML=h;
  el.addEventListener('click',e=>{
    const b=e.target.closest('[data-c]'); if(!b)return;
    const id=b.dataset.c; S.coll=id==='all'?'all':+id; S.page=1;
    el.querySelectorAll('[data-c]').forEach(x=>x.classList.remove('act'));
    b.classList.add('act'); render();
  });
  el.querySelector('[data-c="all"]').classList.add('act');
  document.getElementById('sbFoot').textContent=
    'Exported '+new Date(D.exportedAt).toLocaleDateString()+' · '+D.total.toLocaleString()+' bookmarks';
}
function ci(id,label,n){
  return \`<button class="ci" data-c="\${id}"><span class="ci-lbl">\${x(label)}</span><span class="ci-n">\${(n||0).toLocaleString()}</span></button>\`;
}

// ── tag strip ──────────────────────────────────────────────────────────────
function buildTags(){
  const el=document.getElementById('tagStrip');
  if(!D.allTags.length){el.style.display='none';return;}
  el.innerHTML=D.allTags.map(t=>\`<button class="tc" data-t="\${x(t)}">\${x(t)}</button>\`).join('');
  el.addEventListener('click',e=>{
    const b=e.target.closest('.tc'); if(!b)return;
    const t=b.dataset.t;
    if(S.tag===t){S.tag=null;b.classList.remove('act');}
    else{el.querySelectorAll('.tc').forEach(c=>c.classList.remove('act'));S.tag=t;b.classList.add('act');}
    S.page=1; render();
  });
}

// ── render ────────────────────────────────────────────────────────────────
function render(){
  const f=getFiltered();
  // header
  let hdr='All bookmarks';
  if(S.coll!=='all'){const c=D.collections.find(c=>c._id===S.coll);hdr=c?.title||'Collection';}
  if(S.q) hdr=\`Search: "\${x(S.q)}"\`;
  document.getElementById('collHead').innerHTML=\`<h2 class="ch">\${hdr}</h2>\`;
  document.getElementById('countLine').textContent=f.length.toLocaleString()+' bookmark'+(f.length!==1?'s':'');
  renderList(f);
}

function renderList(f){
  const el=document.getElementById('list');
  el.className='bl '+S.view;
  const slice=f.slice(0,S.page*PG);
  el.innerHTML=slice.map(r=>card(r)).join('');
  el.querySelectorAll('[data-id]').forEach(el=>{
    el.addEventListener('click',e=>{if(e.target.tagName==='A')return;openPanel(+el.dataset.id);});
  });
  const hasMore=f.length>slice.length;
  const btn=document.getElementById('btnMore');
  btn.style.display=hasMore?'block':'none';
  btn.onclick=()=>{S.page++;renderList(f);};
}

// ── card template ─────────────────────────────────────────────────────────
function card(r){
  const cov=r.cover
    ? \`<div class="cov"><img src="\${x(r.cover)}" alt="" loading="lazy" onerror="this.parentElement.classList.add('nc')"></div>\`
    : \`<div class="cov nc"><span class="cf">\${x((r.domain||'?')[0].toUpperCase())}</span></div>\`;
  const tags=(r.tags||[]).slice(0,5).map(t=>\`<span class="tp\${t===S.tag?' act':''}">\${x(t)}</span>\`).join('');
  const bdg=[
    r.important?'<span class="bdg fv" title="Favourite">⭐</span>':'',
    r.broken   ?'<span class="bdg bk" title="Broken">⚠️</span>':'',
    r.type&&r.type!=='link'?\`<span class="bdg tp-b">\${x(r.type)}</span>\`:'',
  ].join('');
  const exc=r.excerpt?\`<p class="exc">\${x(r.excerpt.slice(0,140))}\${r.excerpt.length>140?'…':''}</p>\`:'';
  const meta=[
    \`<span class="mc">\${x(r._ct)}</span>\`,
    r.domain?\`<span class="ms">·</span><span class="md">\${x(r.domain)}</span>\`:'',
    r.created?\`<span class="ms">·</span><time>\${fd(r.created)}</time>\`:'',
  ].join('');
  return \`<article class="bk\${r.important?' fv':''}\${r.broken?' bk-br':''}" data-id="\${r._id}">
\${cov}<div class="bdy"><div class="tr"><a class="tt" href="\${x(r.link||'#')}" target="_blank" rel="noopener">\${x(r.title||'Untitled')}</a>\${bdg}</div>\${exc}<div class="tgs">\${tags}</div><div class="meta">\${meta}</div></div></article>\`;
}

// ── detail panel ──────────────────────────────────────────────────────────
function openPanel(id){
  const r=D.raindrops.find(r=>r._id===id); if(!r)return;
  const hl=(r.highlights||[]).map(h=>
    \`<blockquote class="hl hl-\${x(h.color||'yellow')}">\${x(h.text)}\${h.note?\`<footer>\${x(h.note)}</footer>\`:''}</blockquote>\`
  ).join('');
  const media=(r.media||[]).filter(m=>m.link).map(m=>
    \`<img src="\${x(m.link)}" loading="lazy" onerror="this.remove()">\`
  ).join('');
  const tags=(r.tags||[]).map(t=>\`<span class="tp">\${x(t)}</span>\`).join('');
  document.getElementById('panelBody').innerHTML=\`
\${r.cover?\`<div class="p-cov"><img src="\${x(r.cover)}" onerror="this.parentElement.remove()"></div>\`:''}
<h2 class="p-ttl"><a href="\${x(r.link||'#')}" target="_blank" rel="noopener">\${x(r.title||'Untitled')}</a></h2>
<div class="p-meta">\${[r.domain,fd(r.created),r.important?'⭐':'',r.broken?'⚠️ broken':''].filter(Boolean).join(' · ')}</div>
\${tags?\`<div class="p-tags">\${tags}</div>\`:''}
\${r.excerpt?\`<div class="p-sect"><h4>Description</h4><p>\${x(r.excerpt)}</p></div>\`:''}
\${r.note?\`<div class="p-sect"><h4>Note</h4><p>\${x(r.note)}</p></div>\`:''}
\${hl?\`<div class="p-sect"><h4>Highlights</h4>\${hl}</div>\`:''}
\${media?\`<div class="p-sect"><h4>Media</h4><div class="mg">\${media}</div></div>\`:''}
<div class="p-coll">Collection: <strong>\${x(r._ct)}</strong></div>
\${r.type?\`<div class="p-coll">Type: \${x(r.type)}</div>\`:''}
\${r.created?\`<div class="p-coll">Created: \${x(new Date(r.created).toLocaleString())}</div>\`:''}
  \`;
  document.getElementById('overlay').style.display='flex';
  document.getElementById('panel').scrollTop=0;
}
function closePanel(){document.getElementById('overlay').style.display='none';}

// ── theme ──────────────────────────────────────────────────────────────────
function applyTheme(){
  const saved=ls('rd_th');
  if(saved)document.documentElement.setAttribute('data-theme',saved);
  document.getElementById('btnTheme').addEventListener('click',()=>{
    const cur=document.documentElement.getAttribute('data-theme');
    const n=cur==='dark'?'light':'dark';
    document.documentElement.setAttribute('data-theme',n);
    ss('rd_th',n);
  });
}

// ── controls ──────────────────────────────────────────────────────────────
function initControls(){
  // search
  const qi=document.getElementById('q'), qc=document.getElementById('btnClearQ');
  qi.addEventListener('input',()=>{
    S.q=qi.value.trim(); S.page=1;
    qc.style.display=S.q?'inline-block':'none'; render();
  });
  qc.addEventListener('click',()=>{qi.value='';S.q='';S.page=1;qc.style.display='none';render();});
  // view
  document.querySelectorAll('.v-btn').forEach(b=>{
    b.addEventListener('click',()=>{
      S.view=b.dataset.v; ss('rd_v',S.view); S.page=1;
      document.querySelectorAll('.v-btn').forEach(x=>x.classList.remove('act'));
      b.classList.add('act'); render();
    });
    if(b.dataset.v===S.view) b.classList.add('act');
  });
  // sort
  const ss2=document.getElementById('sortSel');
  ss2.value=S.sort;
  ss2.addEventListener('change',()=>{S.sort=ss2.value;ss('rd_s',S.sort);S.page=1;render();});
  // filters
  document.getElementById('fFav').addEventListener('click',e=>{
    S.fav=!S.fav; S.page=1; e.currentTarget.classList.toggle('act',S.fav); render();
  });
  document.getElementById('fBroken').addEventListener('click',e=>{
    S.brk=!S.brk; S.page=1; e.currentTarget.classList.toggle('act',S.brk); render();
  });
  // detail panel close
  document.getElementById('btnClose').addEventListener('click',closePanel);
  document.getElementById('overlay').addEventListener('click',e=>{
    if(e.target===document.getElementById('overlay'))closePanel();
  });
}

// ── keyboard ──────────────────────────────────────────────────────────────
document.addEventListener('keydown',e=>{
  if(e.key==='/'&&document.activeElement.id!=='q'){e.preventDefault();document.getElementById('q').focus();}
  if(e.key==='Escape'){
    if(document.getElementById('overlay').style.display!=='none'){closePanel();return;}
    const qi=document.getElementById('q');
    if(qi.value){qi.value='';S.q='';S.page=1;document.getElementById('btnClearQ').style.display='none';render();}
  }
});

// ── utils ─────────────────────────────────────────────────────────────────
function x(s){if(!s)return'';return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');}
function fd(iso){try{return new Date(iso).toLocaleDateString(undefined,{year:'numeric',month:'short',day:'numeric'});}catch{return iso||'';}}
function ls(k){try{return localStorage.getItem(k);}catch{return null;}}
function ss(k,v){try{localStorage.setItem(k,v);}catch{}}

// ── boot ──────────────────────────────────────────────────────────────────
buildSidebar(); buildTags(); initControls(); applyTheme(); render();
</script>
</body>
</html>`;
}

// ── CSS ─────────────────────────────────────────────────────────────────────
// Defined outside the function so it's not re-created on each call.
const CSS = `
:root{--sb:260px;--bg:#f5f5f7;--sbg:#f0f0f2;--cbg:#fff;--tx:#1a1a2e;--txm:#6b7280;--br:#e5e7eb;--ac:#0066ff;--acl:#eff6ff;--tgbg:#ede9fe;--tgtx:#5b21b6;--tbg:#fff;--ibg:#f9fafb;--sh:0 1px 3px rgba(0,0,0,.08);--shm:0 4px 20px rgba(0,0,0,.14);}
@media(prefers-color-scheme:dark){:root:not([data-theme="light"]){--bg:#111827;--sbg:#1f2937;--cbg:#1f2937;--tx:#f9fafb;--txm:#9ca3af;--br:#374151;--ac:#60a5fa;--acl:#1e3a5f;--tgbg:#312e81;--tgtx:#c4b5fd;--tbg:#1f2937;--ibg:#374151;--sh:0 1px 3px rgba(0,0,0,.3);--shm:0 4px 20px rgba(0,0,0,.5);}}
[data-theme="dark"]{--bg:#111827;--sbg:#1f2937;--cbg:#1f2937;--tx:#f9fafb;--txm:#9ca3af;--br:#374151;--ac:#60a5fa;--acl:#1e3a5f;--tgbg:#312e81;--tgtx:#c4b5fd;--tbg:#1f2937;--ibg:#374151;--sh:0 1px 3px rgba(0,0,0,.3);--shm:0 4px 20px rgba(0,0,0,.5);}
[data-theme="light"]{--bg:#f5f5f7;--sbg:#f0f0f2;--cbg:#fff;--tx:#1a1a2e;--txm:#6b7280;--br:#e5e7eb;--ac:#0066ff;--acl:#eff6ff;--tgbg:#ede9fe;--tgtx:#5b21b6;--tbg:#fff;--ibg:#f9fafb;--sh:0 1px 3px rgba(0,0,0,.08);--shm:0 4px 20px rgba(0,0,0,.14);}
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0;}
html,body{height:100%;overflow:hidden;background:var(--bg);color:var(--tx);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;font-size:14px;}
#app{display:flex;height:100vh;overflow:hidden;}
/* sidebar */
#sidebar{width:var(--sb);min-width:var(--sb);background:var(--sbg);border-right:1px solid var(--br);display:flex;flex-direction:column;overflow:hidden;}
.sb-head{padding:14px 12px 10px;display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid var(--br);}
.sb-logo{font-size:14px;}
.coll-list{flex:1;overflow-y:auto;padding:6px 0;}
.cs{padding:2px 0;}
.cd{height:1px;background:var(--br);margin:5px 8px;}
.ci{display:flex;align-items:center;justify-content:space-between;width:calc(100% - 10px);margin:1px 5px;padding:6px 10px;border:none;background:transparent;color:var(--tx);cursor:pointer;border-radius:6px;text-align:left;transition:background .1s;}
.ci:hover{background:var(--br);}
.ci.act{background:var(--ac);color:#fff;}
.ci.act .ci-n{background:rgba(255,255,255,.22);color:#fff;}
.ci-lbl{font-size:12.5px;flex:1;overflow:hidden;white-space:nowrap;text-overflow:ellipsis;}
.ci-n{font-size:11px;background:var(--br);color:var(--txm);padding:1px 6px;border-radius:99px;margin-left:5px;flex-shrink:0;}
.sb-foot{padding:8px 12px;border-top:1px solid var(--br);font-size:11px;color:var(--txm);}
/* main */
#main{flex:1;display:flex;flex-direction:column;overflow:hidden;min-width:0;}
/* toolbar */
#toolbar{background:var(--tbg);border-bottom:1px solid var(--br);padding:8px 14px 6px;box-shadow:var(--sh);z-index:10;}
.search-row{margin-bottom:6px;}
.search-box{display:flex;align-items:center;background:var(--ibg);border:1px solid var(--br);border-radius:8px;padding:0 10px;gap:6px;}
.si{color:var(--txm);font-size:12px;}
#q{flex:1;border:none;background:transparent;color:var(--tx);font-size:13px;padding:7px 0;outline:none;}
.ctrl-row{display:flex;align-items:center;gap:8px;flex-wrap:wrap;}
.filter-grp,.view-grp{display:flex;gap:3px;}
.flt-btn,.v-btn{padding:4px 9px;border:1px solid var(--br);border-radius:6px;background:var(--cbg);color:var(--txm);cursor:pointer;font-size:13px;transition:all .1s;}
.flt-btn:hover,.v-btn:hover{background:var(--br);}
.flt-btn.act{border-color:var(--ac);background:var(--acl);color:var(--ac);}
.v-btn.act{background:var(--ac);color:#fff;border-color:var(--ac);}
.sort-sel{padding:5px 7px;border:1px solid var(--br);border-radius:6px;background:var(--cbg);color:var(--tx);font-size:12px;cursor:pointer;}
.ico-btn{background:transparent;border:none;cursor:pointer;font-size:13px;color:var(--txm);padding:3px 5px;border-radius:4px;}
.ico-btn:hover{color:var(--tx);background:var(--br);}
/* tag strip */
#tagStrip{padding:5px 14px;display:flex;flex-wrap:wrap;gap:4px;border-bottom:1px solid var(--br);background:var(--tbg);max-height:50px;overflow-y:auto;}
.tc{padding:2px 8px;border-radius:99px;font-size:11px;cursor:pointer;border:1px solid var(--br);background:var(--cbg);color:var(--txm);white-space:nowrap;transition:all .1s;}
.tc:hover,.tc.act{background:var(--tgbg);color:var(--tgtx);border-color:var(--tgtx);}
/* content */
#content{flex:1;overflow-y:auto;padding:14px 16px;}
.ch{font-size:17px;font-weight:700;margin-bottom:4px;}
#countLine{font-size:12px;color:var(--txm);margin-bottom:10px;}
/* list view */
.bl.list .bk{display:flex;gap:10px;align-items:flex-start;background:var(--cbg);border:1px solid var(--br);border-radius:8px;margin-bottom:7px;padding:9px 11px;cursor:pointer;transition:box-shadow .15s;}
.bl.list .bk:hover{box-shadow:var(--shm);}
.bl.list .cov{width:54px;height:40px;flex-shrink:0;border-radius:5px;overflow:hidden;background:var(--br);}
.bl.list .cov img{width:100%;height:100%;object-fit:cover;display:block;}
.bl.list .nc{display:flex;align-items:center;justify-content:center;}
.bl.list .cf{font-size:16px;font-weight:700;color:var(--txm);}
.bl.list .bdy{flex:1;min-width:0;}
.bl.list .tr{display:flex;align-items:baseline;gap:5px;margin-bottom:1px;}
.bl.list .tt{font-weight:600;font-size:13px;color:var(--tx);text-decoration:none;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;flex:1;}
.bl.list .tt:hover{color:var(--ac);}
.bl.list .exc{font-size:11.5px;color:var(--txm);margin:2px 0 3px;overflow:hidden;display:-webkit-box;-webkit-line-clamp:1;-webkit-box-orient:vertical;}
.bl.list .tgs{display:flex;gap:3px;flex-wrap:wrap;margin-bottom:2px;}
.bl.list .meta{font-size:11px;color:var(--txm);display:flex;align-items:center;gap:4px;flex-wrap:wrap;}
.bl.list .ms{opacity:.35;}
/* grid view */
.bl.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:12px;}
.bl.grid .bk{display:flex;flex-direction:column;background:var(--cbg);border:1px solid var(--br);border-radius:10px;overflow:hidden;cursor:pointer;transition:box-shadow .15s;}
.bl.grid .bk:hover{box-shadow:var(--shm);}
.bl.grid .cov{width:100%;height:120px;background:var(--br);flex-shrink:0;}
.bl.grid .cov img{width:100%;height:100%;object-fit:cover;display:block;}
.bl.grid .nc{display:flex;align-items:center;justify-content:center;height:120px;}
.bl.grid .cf{font-size:30px;font-weight:700;color:var(--br);}
.bl.grid .bdy{padding:9px 11px;flex:1;display:flex;flex-direction:column;gap:3px;}
.bl.grid .tr{display:flex;align-items:flex-start;gap:5px;}
.bl.grid .tt{font-weight:600;font-size:12.5px;color:var(--tx);text-decoration:none;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;flex:1;line-height:1.4;}
.bl.grid .tt:hover{color:var(--ac);}
.bl.grid .exc{font-size:11.5px;color:var(--txm);display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;}
.bl.grid .tgs{display:flex;gap:3px;flex-wrap:wrap;}
.bl.grid .meta{font-size:11px;color:var(--txm);display:flex;align-items:center;gap:4px;flex-wrap:wrap;margin-top:auto;}
.bl.grid .ms{opacity:.35;}
/* compact view */
.bl.compact .bk{display:flex;align-items:center;gap:8px;padding:5px 8px;border-bottom:1px solid var(--br);cursor:pointer;transition:background .1s;}
.bl.compact .bk:hover{background:var(--acl);}
.bl.compact .cov{display:none;}
.bl.compact .bdy{flex:1;display:flex;align-items:center;gap:8px;min-width:0;}
.bl.compact .tr{flex:1;min-width:0;}
.bl.compact .tt{font-size:12.5px;font-weight:500;color:var(--tx);text-decoration:none;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;display:block;}
.bl.compact .tt:hover{color:var(--ac);}
.bl.compact .exc,.bl.compact .tgs{display:none;}
.bl.compact .meta{font-size:11px;color:var(--txm);white-space:nowrap;flex-shrink:0;display:flex;gap:3px;}
.bl.compact .ms{opacity:.35;}
/* shared card bits */
.bk.fv{border-left:3px solid #fbbf24;}
.bk.bk-br{opacity:.6;}
.tp{font-size:11px;padding:2px 7px;border-radius:99px;background:var(--tgbg);color:var(--tgtx);white-space:nowrap;}
.tp.act{filter:brightness(.85);}
.bdg{font-size:10px;padding:1px 5px;border-radius:4px;flex-shrink:0;line-height:1.5;}
.bdg.fv{background:#fef9c3;}
.bdg.bk{background:#fee2e2;}
.bdg.tp-b{background:var(--tgbg);color:var(--tgtx);text-transform:uppercase;}
/* load more */
.btn-more{display:block;width:100%;margin-top:12px;padding:9px;background:var(--cbg);border:1px solid var(--br);border-radius:8px;color:var(--ac);font-size:13px;cursor:pointer;}
.btn-more:hover{background:var(--acl);}
/* overlay */
.overlay{position:fixed;inset:0;background:rgba(0,0,0,.45);z-index:100;display:flex;justify-content:flex-end;}
.panel{width:420px;max-width:100%;height:100%;overflow-y:auto;background:var(--cbg);box-shadow:var(--shm);padding:18px;position:relative;animation:si .2s ease;}
@keyframes si{from{transform:translateX(30px);opacity:0}to{transform:none;opacity:1}}
.panel-close{position:sticky;top:0;float:right;z-index:1;}
.p-cov img{width:100%;border-radius:8px;margin-bottom:10px;max-height:180px;object-fit:cover;}
.p-ttl{font-size:15px;font-weight:700;margin-bottom:5px;line-height:1.4;}
.p-ttl a{color:var(--tx);text-decoration:none;}
.p-ttl a:hover{color:var(--ac);}
.p-meta{font-size:12px;color:var(--txm);margin-bottom:8px;}
.p-tags{display:flex;gap:4px;flex-wrap:wrap;margin-bottom:10px;}
.p-sect{margin-bottom:12px;}
.p-sect h4{font-size:11px;text-transform:uppercase;letter-spacing:.05em;color:var(--txm);margin-bottom:5px;}
.p-sect p{font-size:13px;line-height:1.6;}
.p-coll{font-size:12px;color:var(--txm);margin-top:3px;}
.hl{margin-bottom:7px;padding:7px 10px;border-radius:6px;font-size:13px;line-height:1.5;}
.hl footer{font-size:11px;color:var(--txm);margin-top:3px;}
.hl-yellow{background:#fef9c3;border-left:3px solid #f59e0b;}
.hl-blue{background:#dbeafe;border-left:3px solid #3b82f6;}
.hl-green{background:#dcfce7;border-left:3px solid #22c55e;}
.hl-red{background:#fee2e2;border-left:3px solid #ef4444;}
.hl-purple{background:#f3e8ff;border-left:3px solid #a855f7;}
.hl-orange{background:#ffedd5;border-left:3px solid #f97316;}
.hl-pink{background:#fce7f3;border-left:3px solid #ec4899;}
.hl-cyan{background:#cffafe;border-left:3px solid #06b6d4;}
.hl-indigo{background:#e0e7ff;border-left:3px solid #6366f1;}
.hl-teal{background:#ccfbf1;border-left:3px solid #14b8a6;}
.hl-gray{background:#f3f4f6;border-left:3px solid #9ca3af;}
.hl-brown{background:#fef3c7;border-left:3px solid #b45309;}
.mg{display:grid;grid-template-columns:repeat(3,1fr);gap:5px;}
.mg img{width:100%;border-radius:5px;aspect-ratio:1;object-fit:cover;}
::-webkit-scrollbar{width:5px;}
::-webkit-scrollbar-thumb{background:var(--br);border-radius:3px;}
@media(max-width:640px){#sidebar{display:none;}.panel{width:100%;}}
`;
