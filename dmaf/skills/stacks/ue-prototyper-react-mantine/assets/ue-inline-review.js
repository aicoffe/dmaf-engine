/*!
 * ue-inline-review.js — inline review runtime for hi-fi UE prototypes
 * Version: 1.1.1  |  Schema: ue-inline-review/comments@1 (frozen format)
 * Zero-dependency vanilla JS. IIFE, no imports, no network, no build.
 * All UI classes prefixed `uer-`. Vendored into proto/review/ by the
 * ue-prototyper skills; loaded ONLY in review sessions (?uer=1) through
 * the ProtoLayout dynamic import — never in business entry files.
 * Global review session: client-side SPA navigation (History API) is
 * detected and the page identity re-resolves from
 * window.__UE_REVIEW_META__.pages (registry, exact then longest-prefix
 * match) with a last-path-segment fallback — comments, pins and exports
 * follow the active page automatically; picking/panel state is kept.
 */
(function () {
  'use strict';

  if (window.__ueInlineReviewLoaded) return;
  window.__ueInlineReviewLoaded = true;

  var VERSION = 'uer-VERSION-1.1.1';
  var SCHEMA = 'ue-inline-review/comments@1';
  var Z_BASE = 2147483000; // overlay band (design-drafts pattern: reserved high band)
  var META = window.__UE_REVIEW_META__ || {};
  var PAGES = (META.pages && META.pages.length ? META.pages : []);

  /* --------------------------------- page identity (global review session) */
  // Identity is route-driven and re-resolved on every client-side navigation.
  // Registry (META.pages) matches exact first, then longest prefix (covers
  // parameterized routes); unregistered routes fall back to the last path
  // segment. Search params are identity flags (?uer / ?mockError), never
  // part of the identity. Hash routers (#/path) are normalized.

  function routeString() {
    var h = location.hash;
    var s = (h.length > 1 && h.charAt(1) === '/') ? h.slice(1) : location.pathname;
    var q = s.indexOf('?');
    return q >= 0 ? s.slice(0, q) : s;
  }
  function normRoute(s) {
    return (s.length > 1 && s.charAt(s.length - 1) === '/') ? s.slice(0, -1) : s;
  }
  function resolveIdentity() {
    var route = normRoute(routeString());
    var best = null, bestLen = -1, i, m;
    for (i = 0; i < PAGES.length; i++) {
      if (!PAGES[i] || !PAGES[i].match || !PAGES[i].page) continue;
      m = normRoute(String(PAGES[i].match));
      if (m === route) { best = PAGES[i]; break; } // exact match wins
      if (route.indexOf(m + '/') === 0 && m.length > bestLen) { best = PAGES[i]; bestLen = m.length; }
    }
    if (best) {
      return { page: String(best.page), pageSource: best.pageSource || 'unversioned',
               controls: (best.controls && best.controls.length ? best.controls : []),
               registered: true };
    }
    var segs = route.split('/').filter(function (s) { return s; });
    return { page: segs.length ? segs[segs.length - 1] : 'page',
             pageSource: 'unversioned', controls: [], registered: false };
  }

  var identity = resolveIdentity();
  var PAGE = identity.page;
  var PAGE_SOURCE = identity.pageSource;
  var CONTROLS = identity.controls;
  var STORE_KEY = 'uer:comments:' + PAGE;
  var currentRoute = normRoute(routeString());

  /* ---------------------------------------------------------------- helpers */

  function $(sel, root) { return (root || document).querySelector(sel); }
  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  }
  function esc(s) { return CSS.escape(String(s)); }
  function uid() {
    return 'c-' + Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 7);
  }
  function nowISO() { return new Date().toISOString(); }
  function pageCoords(r) {
    return { x: Math.round(r.left + window.scrollX), y: Math.round(r.top + window.scrollY),
             width: Math.round(r.width), height: Math.round(r.height) };
  }
  function viewport() {
    return { width: window.innerWidth, height: window.innerHeight };
  }

  /* ------------------------------------------------- anchoring (spec R3) */
  // Selector bundle: id -> unique class combo -> structural path (nth-of-type),
  // CSS and XPath built symmetrically, uniqueness verified at capture time.

  function looksAuthoredId(id) {
    if (/^[0-9]+$/.test(id)) return false;            // bare digits
    if (/^:r[a-z0-9:]*$/i.test(id)) return false;     // React useId pattern
    if (/^__.*__$/.test(id)) return false;            // dunder generated
    return true;
  }

  function sameTagIndex(elm) {
    var i = 1, sib = elm;
    while ((sib = sib.previousElementSibling)) { if (sib.tagName === elm.tagName) i++; }
    return i;
  }
  function hasSameTagSiblings(elm) {
    var p = elm.parentElement; if (!p) return false;
    var tag = elm.tagName, n = 0;
    for (var k = 0; k < p.children.length; k++) if (p.children[k].tagName === tag) n++;
    return n > 1;
  }

  function classCandidates(elm) {
    return Array.prototype.filter.call(elm.classList, function (c) {
      return c && c.indexOf('uer-') !== 0;
    });
  }

  function tryIdAnchor(elm) {
    if (!elm.id || !looksAuthoredId(elm.id)) return null;
    var css = '#' + esc(elm.id);
    if (document.querySelectorAll(css).length === 1) return css;
    return null;
  }

  function tryClassAnchor(elm) {
    var cs = classCandidates(elm).slice(0, 4); // verbatim, capped (S4 rationale)
    var i, j;
    for (i = 0; i < cs.length; i++) {
      var css = elm.tagName.toLowerCase() + '.' + esc(cs[i]);
      if (document.querySelectorAll(css).length === 1) return css;
    }
    for (i = 0; i < cs.length; i++) for (j = i + 1; j < cs.length; j++) {
      var css2 = elm.tagName.toLowerCase() + '.' + esc(cs[i]) + '.' + esc(cs[j]);
      if (document.querySelectorAll(css2).length === 1) return css2;
    }
    return null;
  }

  function structuralFrom(rootEl, elm) {
    // Build CSS + XPath in one walk: only levels with same-tag siblings get an index.
    var chain = [], n = elm;
    while (n && n !== rootEl) { chain.unshift(n); n = n.parentElement; }
    if (n !== rootEl) return null; // elm not under rootEl
    var css, xpath;
    if (rootEl.id) {
      css = '#' + esc(rootEl.id);
      xpath = '//*[@id="' + rootEl.id + '"]';
    } else if (rootEl === document.body) {
      css = 'body';
      xpath = '//body';
    } else { return null; }
    for (var i = 0; i < chain.length; i++) {
      var node = chain[i], tag = node.tagName.toLowerCase();
      var cssSeg = tag, xpSeg = tag;
      if (hasSameTagSiblings(node)) {
        cssSeg += ':nth-of-type(' + sameTagIndex(node) + ')';
        xpSeg += '[' + sameTagIndex(node) + ']';
      }
      css += ' > ' + cssSeg;
      xpath += '/' + xpSeg;
    }
    return { css: css, xpath: xpath };
  }

  function anchorBundle(elm) {
    var bundle = { css: null, xpath: null, unique: false };
    var idCss = tryIdAnchor(elm);
    if (idCss) {
      bundle.css = idCss;
      bundle.xpath = '//*[@id="' + elm.id + '"]';
    } else {
      var cls = tryClassAnchor(elm);
      if (cls) bundle.css = cls;
    }
    // Structural fallback (and XPath symmetry) from nearest authored-id ancestor or body.
    var root = elm, structural = null;
    while (root && root !== document.body) {
      if (root.id && looksAuthoredId(root.id) &&
          document.querySelectorAll('#' + esc(root.id)).length === 1) break;
      root = root.parentElement;
    }
    root = root || document.body;
    structural = structuralFrom(root, elm);
    if (structural) {
      if (!bundle.css) bundle.css = structural.css;
      bundle.xpath = bundle.xpath || structural.xpath; // keep the pair recorded
    }
    var unique = false;
    if (bundle.css) {
      try { unique = document.querySelectorAll(bundle.css).length === 1; } catch (e) { unique = false; }
    }
    bundle.unique = unique;
    if (!bundle.xpath && elm.tagName.toLowerCase() === 'body') bundle.xpath = '//body';
    return bundle;
  }

  function matchControl(elm) {
    for (var i = 0; i < CONTROLS.length; i++) {
      var c = CONTROLS[i];
      try {
        if (elm.matches(c.selector) || elm.closest(c.selector)) return c.ueControlId;
      } catch (e) { /* invalid selector in meta — skip */ }
    }
    return null;
  }

  function snapshot(elm, comment, author) {
    var r = elm.getBoundingClientRect();
    var c = classCandidates(elm).slice(0, 4);
    var ctrl = matchControl(elm);
    var b = anchorBundle(elm);
    var item = {
      id: uid(),
      status: 'open',
      selector: { css: b.css, xpath: b.xpath, unique: b.unique },
      anchor: {
        tag: elm.tagName.toLowerCase(),
        classes: c,
        textSnippet: (elm.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80),
        boundingBox: pageCoords(r)
      },
      comment: comment || '',
      category: 'unclassified',
      createdAt: nowISO(),
      author: author || null
    };
    if (ctrl) item.anchor.ueControlId = ctrl; // R7: best-effort, never blocks export
    return item;
  }

  /* ------------------------------------------------------ storage (spec R5) */

  var memoryStore = null; // fallback when localStorage unavailable

  function loadDrafts() {
    try {
      var raw = localStorage.getItem(STORE_KEY);
      if (!raw) return [];
      var parsed = JSON.parse(raw);
      if (parsed && parsed.schema === SCHEMA && Array.isArray(parsed.comments)) return parsed.comments;
      return [];
    } catch (e) { return memoryStore || []; }
  }
  function saveDrafts() {
    try { localStorage.setItem(STORE_KEY, JSON.stringify(exportPayload())); }
    catch (e) { memoryStore = exportPayload(); } // degrade to memory (C5)
  }

  /* ---------------------------------------------------------- state */

  var comments = loadDrafts();
  var picking = false;
  var composer = null;      // open composer state {comment, pin, elm}
  var hoverBox = null;
  var armedClear = 0;       // two-click clear

  /* ------------------------------------------------------ export (spec R4) */

  function exportPayload() {
    return {
      schema: SCHEMA,
      page: PAGE,
      pageSource: PAGE_SOURCE,
      exportedAt: nowISO(),
      viewport: viewport(),
      userAgent: navigator.userAgent,
      comments: comments
    };
  }

  function markdownView() {
    var p = exportPayload();
    var lines = [];
    lines.push('# Design review comments');
    lines.push('');
    lines.push('- Page: `' + p.page + '`' + (p.pageSource !== 'unversioned' ? ' (source: `' + p.pageSource + '`)' : ''));
    lines.push('- Exported: ' + p.exportedAt);
    lines.push('- Viewport: ' + p.viewport.width + 'x' + p.viewport.height);
    lines.push('- Comments: ' + p.comments.length);
    lines.push('');
    if (!p.comments.length) { lines.push('(no comments)'); return lines.join('\n'); }
    p.comments.forEach(function (c, i) {
      var label = c.anchor.ueControlId ? ' · ' + c.anchor.ueControlId : '';
      lines.push('## ' + (i + 1) + '. [' + c.status + '/' + c.category + '] ' + c.anchor.tag + label);
      lines.push('');
      if (c.comment) { lines.push('> ' + c.comment.replace(/\n/g, '\n> ')); lines.push(''); }
      lines.push('- CSS: `' + (c.selector.css || '(none)') + '`' + (c.selector.unique ? '' : ' (**not unique** — verify before applying)'));
      lines.push('- XPath: `' + (c.selector.xpath || '(none)') + '`');
      if (c.anchor.classes && c.anchor.classes.length) lines.push('- Classes: `' + c.anchor.classes.join(' ') + '`');
      if (c.anchor.textSnippet) lines.push('- Text: “' + c.anchor.textSnippet + '”');
      var bb = c.anchor.boundingBox;
      if (bb) lines.push('- Box: x=' + bb.x + ' y=' + bb.y + ' w=' + bb.width + ' h=' + bb.height);
      if (c.screenshot) lines.push('- Screenshot: ' + c.screenshot);
      lines.push('- Created: ' + c.createdAt + (c.author ? ' by ' + c.author : ''));
      lines.push('');
    });
    return lines.join('\n');
  }

  function download(name, text, type) {
    var blob = new Blob([text], { type: type || 'application/json' });
    var url = URL.createObjectURL(blob); // standard object-URL download path (S5)
    var a = el('a');
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(function () { URL.revokeObjectURL(url); }, 1000);
  }

  function exportJSON() { download('comments-' + PAGE + '.json', JSON.stringify(exportPayload(), null, 2)); toast('Exported comments-' + PAGE + '.json'); }
  function exportMD() { download('comments-' + PAGE + '.md', markdownView(), 'text/markdown'); toast('Exported comments-' + PAGE + '.md'); }

  function copyText(text, okMsg) {
    // Convenience only; download is the reliable path (spec C4).
    function fallback() {
      try {
        var ta = el('textarea');
        ta.value = text;
        ta.style.position = 'fixed'; ta.style.left = '-9999px';
        document.body.appendChild(ta); ta.select();
        document.execCommand('copy'); ta.remove();
        toast(okMsg);
      } catch (e) { toast('Copy failed — use Export (download) instead'); }
    }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(function () { toast(okMsg); }, fallback);
    } else fallback();
  }

  /* -------------------------------------------------------------- UI root */

  var root = el('div', 'uer-root');
  root.setAttribute('data-uer', VERSION);
  document.documentElement.appendChild(root);

  var style = el('style');
  style.textContent =
    '.uer-root,.uer-root *{all:revert;box-sizing:border-box;font-family:system-ui,-apple-system,Segoe UI,Roboto,sans-serif;}' +
    '.uer-layer{position:absolute;top:0;left:0;width:100%;height:0;pointer-events:none;z-index:' + Z_BASE + ';}' +
    '.uer-box{position:absolute;border:2px dashed #6366f1;background:rgba(99,102,241,.08);border-radius:4px;pointer-events:none;}' +
    '.uer-flash{position:absolute;border:2px solid #f59e0b;background:rgba(245,158,11,.15);border-radius:4px;pointer-events:none;transition:opacity .6s;}' +
    '.uer-pin{position:absolute;width:22px;height:22px;border-radius:50%;background:linear-gradient(135deg,#6366f1,#a78bfa);border:2px solid #fff;box-shadow:0 1px 6px rgba(0,0,0,.35);pointer-events:auto;cursor:pointer;display:flex;align-items:center;justify-content:center;color:#fff;font-size:11px;font-weight:700;}' +
    '.uer-pin.uer-open::after{content:"";position:absolute;top:-2px;right:-2px;width:8px;height:8px;border-radius:50%;background:#10b981;border:1px solid #fff;}' +
    '.uer-pill{position:fixed;top:12px;right:12px;z-index:' + (Z_BASE + 2) + ';background:#111827;color:#f9fafb;border:1px solid #374151;border-radius:9999px;padding:8px 14px;font-size:13px;cursor:pointer;box-shadow:0 2px 10px rgba(0,0,0,.3);display:flex;gap:8px;align-items:center;touch-action:none;user-select:none;-webkit-user-select:none;}' +
    '.uer-pill .uer-dot{width:9px;height:9px;border-radius:50%;background:#4b5563;}' +
    '.uer-pill.uer-on .uer-dot{background:#6366f1;box-shadow:0 0 6px #6366f1;}' +
    '.uer-panel{position:fixed;top:0;right:0;height:100vh;width:340px;max-width:92vw;background:#111827;color:#e5e7eb;border-left:1px solid #374151;z-index:' + (Z_BASE + 2) + ';display:none;flex-direction:column;font-size:13px;}' +
    '.uer-panel.uer-show{display:flex;}' +
    '.uer-panel header{padding:12px 14px;border-bottom:1px solid #374151;display:flex;align-items:center;gap:8px;font-weight:600;}' +
    '.uer-panel .uer-list{flex:1;overflow:auto;padding:8px;}' +
    '.uer-item{border:1px solid #374151;border-radius:8px;padding:8px;margin-bottom:8px;background:#1f2937;cursor:pointer;}' +
    '.uer-item .uer-meta{color:#9ca3af;font-size:11px;margin-top:4px;word-break:break-all;}' +
    '.uer-item .uer-row{display:flex;gap:6px;align-items:center;margin-top:6px;}' +
    '.uer-chip{font-size:10px;border-radius:9999px;padding:2px 8px;border:1px solid #4b5563;color:#d1d5db;}' +
    '.uer-chip.uer-open{border-color:#6366f1;color:#a5b4fc;}' +
    '.uer-chip.uer-resolved{border-color:#10b981;color:#6ee7b7;}' +
    '.uer-chip.uer-wontfix{border-color:#6b7280;color:#9ca3af;}' +
    '.uer-ibtn{background:#374151;border:none;border-radius:6px;color:#e5e7eb;font-size:11px;padding:3px 8px;cursor:pointer;}' +
    '.uer-ibtn:hover{background:#4b5563;}' +
    '.uer-item textarea{width:100%;background:#111827;color:#e5e7eb;border:1px solid #4b5563;border-radius:6px;padding:6px;font-size:12px;margin-top:6px;resize:vertical;}' +
    '.uer-panel footer{padding:10px 12px;border-top:1px solid #374151;display:flex;flex-wrap:wrap;gap:6px;}' +
    '.uer-btn{background:#4f46e5;border:none;border-radius:8px;color:#fff;font-size:12px;padding:7px 12px;cursor:pointer;}' +
    '.uer-btn.uer-ghost{background:#374151;}' +
    '.uer-btn.uer-danger.uer-armed{background:#dc2626;}' +
    '.uer-composer{position:fixed;z-index:' + (Z_BASE + 3) + ';width:280px;background:#111827;color:#e5e7eb;border:1px solid #4b5563;border-radius:10px;padding:10px;box-shadow:0 8px 30px rgba(0,0,0,.45);font-size:13px;}' +
    '.uer-composer textarea{width:100%;height:70px;background:#1f2937;color:#f9fafb;border:1px solid #4b5563;border-radius:6px;padding:6px;font-size:13px;resize:vertical;}' +
    '.uer-composer .uer-hint{color:#9ca3af;font-size:11px;margin-top:6px;}' +
    '.uer-composer .uer-row{display:flex;gap:6px;margin-top:8px;justify-content:flex-end;}' +
    '.uer-toast{position:fixed;bottom:18px;left:50%;transform:translateX(-50%);background:#111827;color:#f9fafb;border:1px solid #374151;border-radius:9999px;padding:8px 16px;font-size:12px;z-index:' + (Z_BASE + 4) + ';opacity:0;transition:opacity .25s;pointer-events:none;}' +
    '.uer-toast.uer-show{opacity:1;}' +
    'html.uer-picking, html.uer-picking body{cursor:crosshair !important;}';
  root.appendChild(style);

  var layer = el('div', 'uer-layer');
  root.appendChild(layer);

  var pill = el('button', 'uer-pill');
  pill.appendChild(el('span', 'uer-dot'));
  pill.appendChild(document.createTextNode('Review'));
  root.appendChild(pill);

  // Pill is draggable to avoid covering page elements; position is remembered in localStorage and survives reloads / SPA page switches
  try {
    var savedPill = JSON.parse(localStorage.getItem('uer-pill-pos') || 'null');
    if (savedPill && typeof savedPill.x === 'number' && typeof savedPill.y === 'number' && pill.offsetWidth > 0) {
      pill.style.right = 'auto';
      pill.style.left = Math.max(4, Math.min(window.innerWidth - pill.offsetWidth - 4, savedPill.x)) + 'px';
      pill.style.top = Math.max(4, Math.min(window.innerHeight - pill.offsetHeight - 4, savedPill.y)) + 'px';
    }
  } catch (e) {}

  var toastNode = el('div', 'uer-toast');
  root.appendChild(toastNode);
  var toastTimer = 0;
  function toast(msg) {
    toastNode.textContent = msg;
    toastNode.classList.add('uer-show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { toastNode.classList.remove('uer-show'); }, 2200);
  }

  /* ------------------------------------------------------------ panel */

  var panel = el('div', 'uer-panel');
  var pHead = el('header');
  var pTitle = el('span', null, 'Comments');
  var pCount = el('span', 'uer-chip', '0');
  var pClose = el('button', 'uer-ibtn', '✕');
  pClose.style.marginLeft = 'auto';
  pHead.appendChild(pTitle); pHead.appendChild(pCount); pHead.appendChild(pClose);
  var pList = el('div', 'uer-list');
  var pFoot = el('footer');
  var bJSON = el('button', 'uer-btn', 'Export JSON');
  var bMD = el('button', 'uer-btn uer-ghost', 'Export MD');
  var bCopy = el('button', 'uer-btn uer-ghost', 'Copy JSON');
  var bClear = el('button', 'uer-btn uer-ghost uer-danger', 'Clear');
  pFoot.appendChild(bJSON); pFoot.appendChild(bMD); pFoot.appendChild(bCopy); pFoot.appendChild(bClear);
  panel.appendChild(pHead); panel.appendChild(pList); panel.appendChild(pFoot);
  root.appendChild(panel);

  function renderPanel() {
    pTitle.textContent = 'Comments · ' + PAGE;
    pCount.textContent = String(comments.length);
    pList.textContent = '';
    comments.forEach(function (c, i) {
      var item = el('div', 'uer-item');
      var head = el('div', null, (i + 1) + '. ' + (c.comment ? c.comment.split('\n')[0].slice(0, 60) : '(no text)'));
      head.style.fontWeight = '600';
      item.appendChild(head);
      item.appendChild(el('div', 'uer-meta', (c.selector.css || '?') + (c.anchor.ueControlId ? '  → ' + c.anchor.ueControlId : '') + (c.selector.unique ? '' : '  ⚠ not unique')));
      var row = el('div', 'uer-row');
      var chip = el('span', 'uer-chip uer-' + c.status, c.status);
      var chipC = el('span', 'uer-chip', c.category);
      var bEdit = el('button', 'uer-ibtn', '✎');
      var bDel = el('button', 'uer-ibtn', '✕');
      row.appendChild(chip); row.appendChild(chipC);
      row.appendChild(bEdit); row.appendChild(bDel);
      row.style.marginLeft = 'auto';
      item.appendChild(row);
      var editor = null;
      function openEditor() {
        if (editor) return;
        editor = el('textarea');
        editor.value = c.comment;
        item.appendChild(editor);
        editor.focus();
        editor.addEventListener('keydown', function (ev) {
          ev.stopPropagation(); // keep keystrokes out of page hotkeys (S4 lesson)
          if (ev.key === 'Enter' && !ev.shiftKey) { ev.preventDefault(); c.comment = editor.value; saveDrafts(); renderPanel(); renderPins(); }
          if (ev.key === 'Escape') { editor.remove(); editor = null; }
        });
      }
      bEdit.addEventListener('click', function (ev) { ev.stopPropagation(); openEditor(); });
      bDel.addEventListener('click', function (ev) {
        ev.stopPropagation();
        comments = comments.filter(function (x) { return x.id !== c.id; });
        saveDrafts(); renderPanel(); renderPins();
      });
      chip.addEventListener('click', function (ev) {
        ev.stopPropagation();
        c.status = c.status === 'open' ? 'resolved' : (c.status === 'resolved' ? 'wontfix' : 'open');
        saveDrafts(); renderPanel(); renderPins();
      });
      chipC.addEventListener('click', function (ev) {
        ev.stopPropagation();
        var cats = ['unclassified', 'visual', 'rule', 'new-requirement', 'question'];
        c.category = cats[(cats.indexOf(c.category) + 1) % cats.length];
        saveDrafts(); renderPanel();
      });
      item.addEventListener('click', function () { focusComment(c); });
      pList.appendChild(item);
    });
  }

  function focusComment(c) {
    try {
      var elm = document.querySelector(c.selector.css);
      if (!elm && c.selector.xpath) {
        var r = document.evaluate(c.selector.xpath, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
        elm = r.singleNodeValue;
      }
      if (!elm) { toast('Element not found — anchor may be stale'); return; }
      elm.scrollIntoView({ block: 'center' });
      var b = el('div', 'uer-flash');
      positionBox(b, elm);
      layer.appendChild(b);
      setTimeout(function () { b.style.opacity = '0'; }, 300);
      setTimeout(function () { b.remove(); }, 1000);
    } catch (e) { toast('Anchor resolution failed'); }
  }

  /* ---------------------------------------------------------- pins */

  function renderPins() {
    Array.prototype.slice.call(layer.querySelectorAll('.uer-pin')).forEach(function (p) { p.remove(); });
    comments.forEach(function (c, i) {
      var elm = null;
      try { elm = c.selector.css ? document.querySelector(c.selector.css) : null; } catch (e) {}
      if (!elm) return;
      var pin = el('div', 'uer-pin' + (c.status === 'open' ? ' uer-open' : ''));
      pin.textContent = String(i + 1);
      pin.title = c.comment ? c.comment.slice(0, 120) : 'comment ' + (i + 1);
      var r = elm.getBoundingClientRect();
      pin.style.left = (r.left + window.scrollX + r.width - 11) + 'px';
      pin.style.top = (r.top + window.scrollY - 11) + 'px';
      pin.addEventListener('click', function (ev) {
        ev.stopPropagation(); ev.preventDefault();
        focusComment(c);
      });
      layer.appendChild(pin);
    });
  }

  function positionBox(box, elm) {
    var r = elm.getBoundingClientRect();
    box.style.left = (r.left + window.scrollX - 2) + 'px';
    box.style.top = (r.top + window.scrollY - 2) + 'px';
    box.style.width = (r.width + 4) + 'px';
    box.style.height = (r.height + 4) + 'px';
  }

  var rafPending = false;
  function reposition() {
    if (rafPending) return;
    rafPending = true;
    requestAnimationFrame(function () { rafPending = false; renderPins(); });
  }
  window.addEventListener('scroll', reposition, true);
  window.addEventListener('resize', reposition);

  /* ------------------------------------------------------- composer */

  function closeComposer(discard) {
    if (!composer) return;
    if (discard) {
      comments = comments.filter(function (x) { return x.id !== composer.comment.id; });
      saveDrafts();
    }
    if (composer.node) composer.node.remove();
    composer = null;
    renderPins(); renderPanel();
  }

  function openComposer(elm, evX, evY) {
    var c = snapshot(elm);
    comments.push(c);
    saveDrafts();
    var node = el('div', 'uer-composer');
    node.style.left = Math.min(evX, window.innerWidth - 300) + 'px';
    node.style.top = Math.min(evY + 14, window.innerHeight - 180) + 'px';
    var ta = el('textarea');
    ta.placeholder = 'What should change?';
    var hint = el('div', 'uer-hint', 'Enter save · Shift+Enter newline · Esc cancel');
    var row = el('div', 'uer-row');
    var bSave = el('button', 'uer-btn', 'Save');
    var bCancel = el('button', 'uer-btn uer-ghost', 'Cancel');
    row.appendChild(bCancel); row.appendChild(bSave);
    node.appendChild(ta); node.appendChild(hint); node.appendChild(row);
    root.appendChild(node);
    composer = { comment: c, node: node, elm: elm };
    setTimeout(function () { ta.focus(); }, 0);

    function commit() {
      c.comment = ta.value.trim();
      if (!c.comment) { closeComposer(true); return; }
      composer = null; node.remove();
      saveDrafts(); renderPins(); renderPanel();
      toast('Saved #' + comments.length + ' — press Export JSON when done');
    }
    bSave.addEventListener('click', commit);
    bCancel.addEventListener('click', function () { closeComposer(true); });
    ta.addEventListener('keydown', function (ev) {
      ev.stopPropagation(); // containment (spec R8)
      if (ev.key === 'Enter' && !ev.shiftKey) { ev.preventDefault(); commit(); }
      if (ev.key === 'Escape') { closeComposer(true); }
    });
  }

  /* ------------------------------------------------------- pick mode */

  function setPicking(on) {
    picking = on;
    pill.classList.toggle('uer-on', on);
    document.documentElement.classList.toggle('uer-picking', on);
    if (!on) { if (hoverBox) { hoverBox.remove(); hoverBox = null; } closeComposer(true); }
  }

  function inUerUI(node) { return !!(node && node.closest && node.closest('.uer-root')); }

  document.addEventListener('mouseover', function (ev) {
    if (!picking || inUerUI(ev.target)) return;
    if (!hoverBox) { hoverBox = el('div', 'uer-box'); layer.appendChild(hoverBox); }
    positionBox(hoverBox, ev.target);
  }, true);

  document.addEventListener('click', function (ev) {
    if (!picking || inUerUI(ev.target)) return;
    ev.preventDefault(); ev.stopPropagation(); // inert while annotating (spec R8)
    if (composer) closeComposer(true);
    openComposer(ev.target, ev.clientX, ev.clientY);
  }, true);

  document.addEventListener('keydown', function (ev) {
    if (composer || inUerUI(ev.target)) return; // typing in composer stays there
    if (ev.key === 'Escape' && picking) { setPicking(false); return; }
    if ((ev.key === 'm' || ev.key === 'M') && !ev.metaKey && !ev.ctrlKey && !ev.altKey) {
      var t = ev.target;
      if (t && (t.tagName === 'TEXTAREA' || t.tagName === 'INPUT' || t.isContentEditable)) return;
      setPicking(!picking);
    }
  });

  // Drag the pill (pointer capture, clamped to the viewport; >3px counts as a drag and swallows the synthetic click that follows)
  var pillDrag = null;
  var pillSuppressClick = false;
  pill.addEventListener('pointerdown', function (ev) {
    if (ev.button !== 0) return;
    var r = pill.getBoundingClientRect();
    pill.style.right = 'auto';
    pill.style.left = r.left + 'px';
    pill.style.top = r.top + 'px';
    pillDrag = { id: ev.pointerId, dx: ev.clientX - r.left, dy: ev.clientY - r.top, x0: r.left, y0: r.top, moved: false };
    try { pill.setPointerCapture(ev.pointerId); } catch (e) {}
    ev.preventDefault();
  });
  pill.addEventListener('pointermove', function (ev) {
    if (!pillDrag || ev.pointerId !== pillDrag.id) return;
    var rawX = ev.clientX - pillDrag.dx, rawY = ev.clientY - pillDrag.dy;
    if (!pillDrag.moved && (Math.abs(rawX - pillDrag.x0) > 3 || Math.abs(rawY - pillDrag.y0) > 3)) pillDrag.moved = true;
    pill.style.left = Math.max(4, Math.min(window.innerWidth - pill.offsetWidth - 4, rawX)) + 'px';
    pill.style.top = Math.max(4, Math.min(window.innerHeight - pill.offsetHeight - 4, rawY)) + 'px';
  });
  function finishPillDrag() {
    if (!pillDrag) return;
    if (pillDrag.moved) {
      try { localStorage.setItem('uer-pill-pos', JSON.stringify({ x: pill.offsetLeft, y: pill.offsetTop })); } catch (e) {}
      pillSuppressClick = true;
      setTimeout(function () { pillSuppressClick = false; }, 0);
    }
    pillDrag = null;
  }
  pill.addEventListener('pointerup', function (ev) {
    if (!pillDrag || ev.pointerId !== pillDrag.id) return;
    finishPillDrag();
  });
  pill.addEventListener('pointercancel', function () {
    finishPillDrag();
  });

  pill.addEventListener('click', function () {
    if (pillSuppressClick) { pillSuppressClick = false; return; } // synthetic click after a drag must not open the panel
    panel.classList.toggle('uer-show');
    renderPanel(); renderPins();
  });
  pClose.addEventListener('click', function () { panel.classList.remove('uer-show'); });

  bJSON.addEventListener('click', exportJSON);
  bMD.addEventListener('click', exportMD);
  bCopy.addEventListener('click', function () { copyText(JSON.stringify(exportPayload(), null, 2), 'JSON copied'); });
  bClear.addEventListener('click', function () {
    var t = Date.now();
    if (t - armedClear < 4000) {
      comments = []; saveDrafts(); renderPanel(); renderPins();
      bClear.textContent = 'Clear'; bClear.classList.remove('uer-armed'); armedClear = 0;
      toast('Cleared (this page)');
    } else {
      armedClear = t;
      bClear.textContent = 'Clear ' + comments.length + '?';
      bClear.classList.add('uer-armed');
      setTimeout(function () { bClear.textContent = 'Clear'; bClear.classList.remove('uer-armed'); }, 4000);
    }
  });

  renderPanel(); renderPins();

  /* ------------------------------------------- SPA navigation resync */
  // vue-router / react-router both drive the History API: patch push/replace
  // and listen to popstate/hashchange/pageshow, then re-resolve identity.
  // The patch only exists inside a review session (?uer=1), scoped to this
  // page lifetime. resync() is idempotent — no-op when the route is unchanged
  // (query toggles, router scroll restoration).

  function resync() {
    var route = normRoute(routeString());
    if (route === currentRoute) return;
    currentRoute = route;
    var next = resolveIdentity();
    if (!next.page || next.page === PAGE) return;
    var keptDraft = false;
    if (composer) {
      // half-typed composer text is saved onto the old page's comment; an
      // empty one is discarded (Esc semantics)
      var taEl = composer.node ? composer.node.querySelector('textarea') : null;
      var text = taEl ? taEl.value.trim() : '';
      if (text) { composer.comment.comment = text; keptDraft = true; }
      closeComposer(keptDraft ? false : true);
    }
    PAGE = next.page;
    PAGE_SOURCE = next.pageSource;
    CONTROLS = next.controls;
    STORE_KEY = 'uer:comments:' + PAGE;
    comments = loadDrafts();
    renderPanel();
    renderPins(); // DOM may still be swapping; re-render below once settled
    requestAnimationFrame(function () { requestAnimationFrame(renderPins); });
    setTimeout(renderPins, 300);
    if (keptDraft) toast('Switched to ' + PAGE + ' — draft comment kept on previous page');
    else if (next.registered) toast('Switched to ' + PAGE + ' — ' + comments.length + ' comment(s)');
    else toast('Switched to ' + PAGE + ' (route not in review registry)');
  }

  ['pushState', 'replaceState'].forEach(function (name) {
    var orig = history[name];
    if (typeof orig !== 'function') return;
    history[name] = function () {
      var r = orig.apply(this, arguments);
      resync();
      return r;
    };
  });
  window.addEventListener('popstate', resync);
  window.addEventListener('hashchange', resync);
  window.addEventListener('pageshow', resync);
})();
