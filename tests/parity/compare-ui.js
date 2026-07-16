const BASE = '/tests/parity/scenes';

const THEME_KEY = 'parity-theme';

function resolveTheme(mode) {
    if (mode === 'light') return 'light';
    if (mode === 'dark') return 'dark';
    return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function refreshAllCodePanes() {
    refreshCodePane('three');
    refreshCodePane('threers');
}

function canvasChromeColor() {
    return getComputedStyle(document.documentElement).getPropertyValue('--canvas-chrome').trim() || '#101418';
}

function resolvedUiTheme() {
    return document.documentElement.dataset.theme || 'dark';
}

function applyIframeChrome(frame) {
    if (!frame?.contentDocument) return;
    try {
        const doc = frame.contentDocument;
        let style = doc.getElementById('parity-ui-chrome');
        if (!style) {
            style = doc.createElement('style');
            style.id = 'parity-ui-chrome';
            (doc.head || doc.documentElement).appendChild(style);
        }
        const chrome = canvasChromeColor();
        style.textContent = `html,body{background:${chrome} !important;}`;
        if (doc.body) doc.body.style.background = chrome;
    } catch (_) { /* cross-origin or not ready */ }
}

function syncIframeThemes() {
    applyIframeChrome(els.threeFrame);
    applyIframeChrome(els.threersFrame);
    els.grid?.querySelectorAll('iframe').forEach((frame) => applyIframeChrome(frame));
}

function applyTheme(mode) {
    const resolved = resolveTheme(mode);
    document.documentElement.dataset.theme = resolved;
    document.documentElement.dataset.themeMode = mode;
    try { localStorage.setItem(THEME_KEY, mode); } catch (_) { /* private mode */ }
    document.querySelectorAll('.theme-btn').forEach((btn) => {
        btn.classList.toggle('active', btn.dataset.themeMode === mode);
        btn.setAttribute('aria-pressed', btn.dataset.themeMode === mode ? 'true' : 'false');
    });
    refreshAllCodePanes();
    syncIframeThemes();
}

function initTheme() {
    const mode = (() => {
        try { return localStorage.getItem(THEME_KEY) || 'system'; } catch (_) { return 'system'; }
    })();
    applyTheme(mode);
    matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        let current = 'system';
        try { current = localStorage.getItem(THEME_KEY) || 'system'; } catch (_) { /* ignore */ }
        if (current === 'system') applyTheme('system');
    });
    document.querySelectorAll('.theme-btn').forEach((btn) => {
        btn.addEventListener('click', () => applyTheme(btn.dataset.themeMode));
    });
}

let SCENES = [];
let SCENE_API_MAP = {};
let APPROXIMATE_SCENES = [];
/** Scene slugs that need optional wasm/shim features (e.g. mesh-bvh). */
let FEATURE_SCENES = new Map();
/** @type {string | null} */
let meshBvhBuildEnabled = null;
let bvhCsgBuildEnabled = null;

let stats = new Map();
/** Latest iframe boot/render result — overrides stale compare-results device errors. */
const liveSceneStatus = new Map();

let index = 0;
let gridMode = false;
let syncOrbitEnabled = true;
/** @type {'three' | 'threers'} */
let activeOrbitSide = 'three';
let orbitSyncRaf = 0;

function orbitStateKey(state) {
    if (!state?.position || !state?.target) return '';
    const p = state.position;
    const t = state.target;
    return `${p.x.toFixed(4)},${p.y.toFixed(4)},${p.z.toFixed(4)}|${t.x.toFixed(4)},${t.y.toFixed(4)},${t.z.toFixed(4)}`;
}

const DEFAULT_ORBIT_STATE = {
    position: { x: 0, y: 0, z: 5 },
    target: { x: 0, y: 0, z: 0 },
};

/** @type {string} */
let lastOrbitRelayKey = '';
/** @type {string} */
let iframeCacheBust = '';
/** Bump when compare scene iframes need a cache bust (shim, scenes, orbit sync). */
const SCENE_CACHE_REV = 24;

async function fetchAssetVersion() {
    try {
        const res = await fetch('/web/pkg/build-id.txt', { cache: 'no-store' });
        if (res.ok) return `${(await res.text()).trim()}-r${SCENE_CACHE_REV}`;
    } catch (_) { /* optional */ }
    return `${Date.now()}-r${SCENE_CACHE_REV}`;
}
/** @type {Map<string, { path: string, display: string, full: string, url?: string }>} */
const sourceCache = new Map();
/** @type {{ three: string, threers: string, threeLang: string, threersLang: string }} */
const codeLang = { three: 'javascript', threers: 'javascript', threeLang: 'javascript', threersLang: 'javascript' };
/** @type {{ three?: object, threers?: object, rust?: object }} */
const currentSources = {};

function extractSceneSource(html) {
    const blocks = [...html.matchAll(/<script[^>]*type=["']module["'][^>]*>([\s\S]*?)<\/script>/gi)];
    if (blocks.length === 0) return html.trim();
    const importmap = html.match(/<script[^>]*type=["']importmap["'][^>]*>([\s\S]*?)<\/script>/i);
    const parts = [];
    if (importmap) {
        try {
            const json = JSON.stringify(JSON.parse(importmap[1].trim()), null, 2);
            parts.push(`// import map (from scene HTML)\nconst __importMap__ = ${json};\n`);
        } catch {
            parts.push(`// import map (from scene HTML)\n${importmap[1].trim()}\n`);
        }
    }
    blocks.forEach((m, i) => {
        if (blocks.length > 1) parts.push(`// --- script ${i + 1} ---\n`);
        parts.push(m[1].trim());
    });
    return parts.join('\n\n');
}

async function fetchSceneSource(prefix, slug) {
    const rel = `scenes/${prefix}-${slug}.html`;
    const url = `${BASE}/${prefix}-${slug}.html`;
    const cacheKey = url;
    if (sourceCache.has(cacheKey)) return sourceCache.get(cacheKey);

    const res = await fetch(url);
    if (!res.ok) throw new Error(`not found: ${rel}`);
    const full = await res.text();
    const entry = {
        path: rel,
        url,
        full,
        display: extractSceneSource(full),
    };
    sourceCache.set(cacheKey, entry);
    return entry;
}

function jsToThreersTs(js) {
    let ts = js.replace(
        /import THREE, \{ initThreers \} from '([^']+)';/,
        `import THREE, {
  initThreers,
  type WebGLRenderer,
  type PerspectiveCamera,
} from '$1';`,
    );
    ts = ts.replace(
        /const r = await THREE\.WebGLRenderer\.create\(document\.getElementById\('c'\)\);/,
        "const r: WebGLRenderer = await THREE.WebGLRenderer.create(document.getElementById('c')!);",
    );
    ts = ts.replace(
        /const cam = new THREE\.PerspectiveCamera/,
        'const cam: PerspectiveCamera = new THREE.PerspectiveCamera',
    );
    return ts;
}

function jsToThreeTs(js) {
    let ts = js;
    if (!/import type/.test(ts)) {
        ts = ts.replace(
            /import \* as THREE from 'three';/,
            "import * as THREE from 'three';\n// npm i three@0.165 @types/three",
        );
    }
    ts = ts.replace(
        /const r = new THREE\.WebGLRenderer\(\{ canvas: document\.getElementById\('c'\)/,
        "const canvas = document.getElementById('c') as HTMLCanvasElement;\nconst r = new THREE.WebGLRenderer({ canvas",
    );
    ts = ts.replace(
        /const cam = new THREE\.PerspectiveCamera/,
        'const cam: THREE.PerspectiveCamera = new THREE.PerspectiveCamera',
    );
    return ts;
}

const HLJS_LANG = {
    javascript: 'javascript',
    typescript: 'typescript',
    rust: 'rust',
};

function highlightSource(text, language) {
    if (typeof hljs === 'undefined') return null;
    const lang = HLJS_LANG[language] || language;
    try {
        if (lang && hljs.getLanguage(lang)) {
            return hljs.highlight(text, { language: lang, ignoreIllegals: true }).value;
        }
    } catch (_) { /* fall through */ }
    try {
        return hljs.highlightAuto(text, [lang, 'javascript', 'typescript', 'rust']).value;
    } catch (_) {
        return null;
    }
}

function codeBlockEl(preEl) {
    let code = preEl.querySelector('code');
    if (!code) {
        preEl.textContent = '';
        code = document.createElement('code');
        preEl.appendChild(code);
    }
    return code;
}

function renderCode(preEl, text, { loading = false, error = false, language = 'javascript' } = {}) {
    const code = codeBlockEl(preEl);
    preEl.dataset.source = text;
    preEl.classList.toggle('loading', loading);
    preEl.classList.toggle('is-error', error);

    if (loading) {
        code.className = '';
        code.textContent = text;
        return;
    }

    if (error) {
        code.className = '';
        code.textContent = text;
        return;
    }

    const lang = HLJS_LANG[language] || language || 'javascript';

    code.className = `hljs language-${lang}`;
    code.removeAttribute('data-highlighted');
    const html = highlightSource(text, language);
    if (html) {
        code.innerHTML = html;
    } else {
        code.textContent = text;
    }
}

function displayTextForPane(pane, entry) {
    if (!entry) return '';
    if (pane && codeLang[`${pane}Lang`] === 'typescript') {
        return pane === 'three' ? jsToThreeTs(entry.display) : jsToThreersTs(entry.display);
    }
    return entry.display;
}

function refreshCodePane(pane) {
    const pre = pane === 'three' ? els.threeCode : els.threersCode;
    const pathEl = pane === 'three' ? els.threeCodePath : els.threersCodePath;
    const lang = codeLang[`${pane}Lang`];

    if (pane === 'threers' && lang === 'rust') {
        const entry = currentSources.rust;
        if (!entry || entry.error) {
            pathEl.textContent = entry?.path || '—';
            renderCode(pre, entry?.error || 'Rust source not available', { error: true, language: 'rust' });
            return;
        }
        pathEl.innerHTML = `<a href="${entry.url}" target="_blank" rel="noopener">${entry.path}</a>`;
        renderCode(pre, entry.display, { language: 'rust' });
        return;
    }

    const entry = currentSources[pane];
    if (!entry) return;
    if (entry.error) {
        pathEl.textContent = entry.path || '—';
        renderCode(pre, entry.error, {
            error: true,
            language: lang === 'typescript' ? 'typescript' : 'javascript',
        });
        return;
    }
    pathEl.innerHTML = `<a href="${entry.url}" target="_blank" rel="noopener">${entry.path}</a>`;
    renderCode(pre, displayTextForPane(pane, entry), { language: lang });
}

function setCodeTabsActive(tabsEl, lang) {
    tabsEl.querySelectorAll('.code-tab').forEach((btn) => {
        btn.classList.toggle('active', btn.dataset.lang === lang);
    });
}

function rustStubForFeatureScene(slug, feature) {
    if (slug === 'bvh-csg-hierarchy') return null;
    if (feature === 'bvh-csg') {
        return `//! Parity scene \`${slug}\` — CSG is JavaScript, not native Rust.
//!
//! Boolean evaluation: \`web/csg/\` (port of three-bvh-csg@0.0.16)
//!   Evaluator.evaluate / evaluateHierarchy on JS BufferGeometry
//! BVH acceleration: wasm mesh-bvh (\`web/mesh-bvh-impl.js\`)
//! Final draw: threers wgpu renderer uploads mesh via BufferGeometry._syncWasmFromJs()
//!
//! There is no \`examples/${slug}.rs\` — use the JavaScript tab:
//!   scenes/threers-${slug}.html
//! Hierarchy ops: web/csg/core/operations/Operation.js, OperationGroup.js
//!                 web/csg/core/Evaluator.js (evaluateHierarchy)

// CSG runs in the browser; wasm only renders the evaluated triangle soup.
`;
    }
    if (feature === 'mesh-bvh') {
        return `//! Parity scene \`${slug}\` — scene graph is JavaScript; BVH queries are wasm.
//!
//! See scenes/threers-${slug}.html and web/mesh-bvh-impl.js
//! Native Rust example generation does not apply to mesh-bvh parity scenes.

`;
    }
    return null;
}

async function fetchRustSceneSource(slug) {
    const rel = `scenes/rust/${slug}.rs`;
    const url = `${BASE}/rust/${slug}.rs`;
    const cacheKey = url;
    if (sourceCache.has(cacheKey)) return sourceCache.get(cacheKey);

    const feature = FEATURE_SCENES.get(slug);
    const stub = feature ? rustStubForFeatureScene(slug, feature) : null;
    if (stub) {
        const entry = { path: rel, url, full: stub, display: stub, stub: true };
        sourceCache.set(cacheKey, entry);
        return entry;
    }

    const res = await fetch(url);
    if (!res.ok) throw new Error(`not found: ${rel} — run node generate-rust-scenes.js`);
    const display = await res.text();
    const entry = { path: rel, url, full: display, display };
    sourceCache.set(cacheKey, entry);
    return entry;
}

async function loadSceneSources(slug) {
    renderCode(els.threeCode, 'Loading source…', { loading: true });
    renderCode(els.threersCode, 'Loading source…', { loading: true });

    const [threeRes, threersRes, rustRes] = await Promise.allSettled([
        fetchSceneSource('threejs', slug),
        fetchSceneSource('threers', slug),
        fetchRustSceneSource(slug),
    ]);

    if (threeRes.status === 'fulfilled') {
        currentSources.three = threeRes.value;
    } else {
        currentSources.three = { path: `scenes/threejs-${slug}.html`, error: String(threeRes.reason) };
    }

    if (threersRes.status === 'fulfilled') {
        currentSources.threers = threersRes.value;
    } else {
        currentSources.threers = { path: `scenes/threers-${slug}.html`, error: String(threersRes.reason) };
    }

    if (rustRes.status === 'fulfilled') {
        currentSources.rust = rustRes.value;
    } else {
        currentSources.rust = { path: `scenes/rust/${slug}.rs`, error: String(rustRes.reason) };
    }

    refreshCodePane('three');
    refreshCodePane('threers');
    setCodeTabsActive(els.tabsThree, codeLang.threeLang);
    setCodeTabsActive(els.tabsThreers, codeLang.threersLang);
}

async function copyText(text, btn) {
    try {
        await navigator.clipboard.writeText(text);
        const prev = btn.textContent;
        btn.textContent = 'Copied';
        setTimeout(() => { btn.textContent = prev; }, 1200);
    } catch {
        btn.textContent = 'Failed';
        setTimeout(() => { btn.textContent = 'Copy'; }, 1200);
    }
}

const els = {
    list: document.getElementById('scene-list'),
    search: document.getElementById('search'),
    threeFrame: document.getElementById('frame-three'),
    threersFrame: document.getElementById('frame-threers'),
    threeCode: document.getElementById('code-three'),
    threersCode: document.getElementById('code-threers'),
    threeCodePath: document.getElementById('code-path-three'),
    threersCodePath: document.getElementById('code-path-threers'),
    tabsThree: document.getElementById('tabs-three'),
    tabsThreers: document.getElementById('tabs-threers'),
    btnCopyThree: document.getElementById('btn-copy-three'),
    btnCopyThreers: document.getElementById('btn-copy-threers'),
    title: document.getElementById('scene-title'),
    counter: document.getElementById('scene-counter'),
    stat: document.getElementById('scene-stat'),
    apis: document.getElementById('scene-apis'),
    statusThree: document.getElementById('status-three'),
    statusThreers: document.getElementById('status-threers'),
    viewSingle: document.getElementById('view-single'),
    viewGrid: document.getElementById('view-grid'),
    grid: document.getElementById('grid'),
    btnPrev: document.getElementById('btn-prev'),
    btnNext: document.getElementById('btn-next'),
    btnReload: document.getElementById('btn-reload'),
    btnOpenScene: document.getElementById('btn-open-scene'),
    syncOrbit: document.getElementById('sync-orbit'),
};

function slugFromHash() {
    const h = decodeURIComponent(location.hash.replace(/^#/, ''));
    if (!h) return null;
    const i = SCENES.indexOf(h);
    return i >= 0 ? i : null;
}

function setHash(slug) {
    const next = `#${slug}`;
    if (location.hash !== next) history.replaceState(null, '', next);
}

function shortStatErr(err) {
    if (!err) return err;
    const line = String(err).split('\n')[0];
    return line.length > 140 ? `${line.slice(0, 137)}…` : line;
}

function statLabel(slug) {
    const live = liveSceneStatus.get(slug);
    const r = stats.get(slug);
    if (live?.ok && r?.err && /RequestDeviceError|maxInterStageShaderComponents/.test(r.err)) {
        if (r.pct != null && r.ok) return { text: `pixel-perfect · 0.00%`, cls: 'perfect' };
        if (r.pct != null && r.pass !== false) return { text: `matching · ${r.pct.toFixed(2)}%`, cls: 'ok' };
        if (r.pct != null) return { text: `diverged · ${r.pct.toFixed(2)}%`, cls: 'bad' };
        return { text: 'live OK · compare stats stale — press ↻ or run node run.js', cls: 'ok' };
    }
    if (!r) return { text: 'no stats — run node run.js', cls: 'muted' };
    if (r.err) return { text: `error: ${shortStatErr(r.err)}`, cls: 'bad' };
    if (r.ok) return { text: `pixel-perfect · 0.00%`, cls: 'perfect' };
    if (r.pct == null) return { text: '—', cls: 'muted' };
    if (r.pct < 0.5) return { text: `identical · ${r.pct.toFixed(2)}%`, cls: 'good' };
    if (r.pass !== false && r.pct < 5) return { text: `matching · ${r.pct.toFixed(2)}%`, cls: 'ok' };
    return { text: `diverged · ${r.pct.toFixed(2)}%`, cls: 'bad' };
}

function listItemClass(slug, active) {
    const r = stats.get(slug);
    let tag = 'pending';
    if (r && r.err) tag = 'err';
    else if (r && r.ok) tag = 'perfect';
    else if (r && r.pct != null && r.pct < 0.5) tag = 'identical';
    else if (r && r.pass !== false && r.pct != null && r.pct < 5) tag = 'match';
    else if (r && r.pct != null) tag = 'warn';
    return `scene-item${active ? ' active' : ''} tag-${tag}`;
}

function renderList(filter = '') {
    const q = filter.trim().toLowerCase();
    els.list.innerHTML = '';
    SCENES.forEach((slug, i) => {
        if (q && !slug.includes(q)) return;
        const li = document.createElement('button');
        li.type = 'button';
        li.className = listItemClass(slug, i === index);
        li.dataset.index = String(i);
        const r = stats.get(slug);
        const pct = r && r.pct != null ? `${r.pct.toFixed(2)}%` : '—';
        const feat = FEATURE_SCENES.has(slug) ? ` · ${FEATURE_SCENES.get(slug)}` : '';
        li.innerHTML = `<span class="slug">${slug}${feat}</span><span class="pct">${pct}</span>`;
        li.addEventListener('click', () => goTo(i));
        els.list.appendChild(li);
    });
}

function renderGrid(filter = '') {
    const q = filter.trim().toLowerCase();
    els.grid.innerHTML = '';
    SCENES.forEach((slug, i) => {
        if (q && !slug.includes(q)) return;
        const card = document.createElement('button');
        card.type = 'button';
        card.className = 'grid-card';
        card.innerHTML = `
            <div class="grid-label">${slug}</div>
            <div class="grid-pair">
                <iframe loading="lazy" title="three.js ${slug}" src="${BASE}/threejs-${slug}.html"></iframe>
                <iframe loading="lazy" title="threers ${slug}" src="${BASE}/threers-${slug}.html"></iframe>
            </div>`;
        card.querySelectorAll('iframe').forEach((frame) => {
            frame.addEventListener('load', () => {
                applyIframeChrome(frame);
                if (syncOrbitEnabled) {
                    try {
                        frame.contentWindow?.postMessage({ type: 'parity-orbit-sync-enabled', enabled: true }, '*');
                    } catch (_) { /* not ready */ }
                }
            });
        });
        card.addEventListener('click', () => { gridMode = false; syncViewMode(); goTo(i); });
        els.grid.appendChild(card);
    });
}

function syncViewMode() {
    els.viewSingle.hidden = gridMode;
    els.viewGrid.hidden = !gridMode;
    document.body.classList.toggle('mode-grid', gridMode);
}

function broadcastOrbitSyncEnabled(enabled) {
    const msg = { type: 'parity-orbit-sync-enabled', enabled };
    const frames = gridMode
        ? [...document.querySelectorAll('.grid-pair iframe')]
        : [els.threeFrame, els.threersFrame].filter(Boolean);
    for (const frame of frames) {
        try { frame?.contentWindow?.postMessage(msg, '*'); } catch (_) { /* not ready */ }
    }
}

function pushOrbitSyncToPeer(sourceSide, state) {
    if (!syncOrbitEnabled || gridMode || !state) return;
    const targetFrame = sourceSide === 'three' ? els.threersFrame : els.threeFrame;
    try {
        targetFrame?.contentWindow?.postMessage({
            type: 'parity-orbit-sync',
            from: sourceSide,
            state,
        }, '*');
    } catch (_) { /* not ready */ }
}

function readOrbitStateFromFrame(frame) {
    try {
        const win = frame?.contentWindow;
        const ctx = win?.__parityOrbitCtx;
        const fn = win?.__parityExportOrbitState;
        if (ctx && typeof fn === 'function') return fn(ctx);
    } catch (_) { /* not ready */ }
    return null;
}

function startContinuousOrbitSync() {
    if (orbitSyncRaf) cancelAnimationFrame(orbitSyncRaf);
    const tick = () => {
        if (syncOrbitEnabled && !gridMode) {
            const sourceFrame = activeOrbitSide === 'threers' ? els.threersFrame : els.threeFrame;
            const state = readOrbitStateFromFrame(sourceFrame);
            if (state) {
                const key = orbitStateKey(state);
                if (key && key !== lastOrbitRelayKey) {
                    lastOrbitRelayKey = key;
                    pushOrbitSyncToPeer(activeOrbitSide, state);
                }
            }
        }
        orbitSyncRaf = requestAnimationFrame(tick);
    };
    orbitSyncRaf = requestAnimationFrame(tick);
}

function orbitRelayTargetFrame(event, source) {
    if (gridMode) {
        const srcFrame = [...document.querySelectorAll('.grid-pair iframe')]
            .find((f) => f.contentWindow === event.source);
        if (srcFrame?.parentElement) {
            return [...srcFrame.parentElement.querySelectorAll('iframe')]
                .find((f) => f !== srcFrame) || null;
        }
        return null;
    }
    return source === 'three' ? els.threersFrame : els.threeFrame;
}

function requestOrbitState(frame, source) {
    return new Promise((resolve) => {
        const onMsg = (event) => {
            const d = event.data;
            if (!d || d.type !== 'parity-orbit-state' || d.source !== source) return;
            window.removeEventListener('message', onMsg);
            resolve(d.state || null);
        };
        window.addEventListener('message', onMsg);
        setTimeout(() => {
            window.removeEventListener('message', onMsg);
            resolve(null);
        }, 5000);
        try {
            frame?.contentWindow?.postMessage({ type: 'parity-orbit-get-state' }, '*');
        } catch (_) {
            window.removeEventListener('message', onMsg);
            resolve(null);
        }
    });
}

async function alignOrbitsFromThree() {
    let state = await requestOrbitState(els.threeFrame, 'three');
    if (!state) state = DEFAULT_ORBIT_STATE;
    lastOrbitRelayKey = orbitStateKey(state);
    activeOrbitSide = 'three';
    broadcastOrbitSyncEnabled(true);
    pushOrbitSyncToPeer('three', state);
}

function scheduleOrbitAlign() {
    if (!syncOrbitEnabled) return;
    alignOrbitsFromThree();
    for (const delay of [500, 1200, 2500]) {
        setTimeout(() => { if (syncOrbitEnabled) alignOrbitsFromThree(); }, delay);
    }
}

function handleOrbitRelay(event) {
    const d = event.data;
    if (!d || d.type !== 'parity-orbit-change' || !syncOrbitEnabled) return;
    const key = orbitStateKey(d.state);
    if (!key || key === lastOrbitRelayKey) return;
    lastOrbitRelayKey = key;
    if (d.source === 'three' || d.source === 'threers') activeOrbitSide = d.source;
    const target = orbitRelayTargetFrame(event, d.source);
    try {
        target?.contentWindow?.postMessage({
            type: 'parity-orbit-sync',
            from: d.source,
            state: d.state,
        }, '*');
    } catch (_) { /* not ready */ }
}

function assertSceneFeature(slug) {
    if (!FEATURE_SCENES.has(slug)) return;
    const feat = FEATURE_SCENES.get(slug);
    if (feat === 'bvh-csg' && bvhCsgBuildEnabled === false) {
        throw new Error('bvh-csg not in wasm build — run: BVH_CSG=1 web/build.sh');
    }
    if (feat === 'mesh-bvh' && meshBvhBuildEnabled === false) {
        throw new Error('mesh-bvh not in wasm build — run: MESH_BVH=1 web/build.sh');
    }
    if (meshBvhBuildEnabled === false && feat === 'bvh-csg') {
        throw new Error('bvh-csg requires mesh-bvh — run: BVH_CSG=1 web/build.sh');
    }
}

function sceneFrameUrl(side, slug) {
    const bust = iframeCacheBust ? `?v=${encodeURIComponent(iframeCacheBust)}` : '';
    return `${BASE}/${side}-${slug}.html${bust}`;
}

function waitFrameReady(frame, timeoutMs = 45000) {
    return new Promise((resolve) => {
        const start = Date.now();
        const tick = () => {
            try {
                const doc = frame.contentDocument;
                const ready = doc && doc.body && doc.body.dataset && doc.body.dataset.ready;
                if (ready === 'true') { resolve({ ok: true }); return; }
                if (ready === 'error') {
                    const errEl = doc.getElementById('err');
                    const err = (errEl && errEl.textContent) || 'render error';
                    resolve({ ok: false, err });
                    return;
                }
            } catch (_) { /* not loaded yet */ }
            if (Date.now() - start > timeoutMs) {
                resolve({ ok: false, err: 'timeout' });
                return;
            }
            requestAnimationFrame(tick);
        };
        frame.addEventListener('load', () => tick(), { once: true });
        if (frame.contentDocument && frame.contentDocument.readyState === 'complete') tick();
    });
}

async function pollFrames() {
    els.statusThree.textContent = 'loading…';
    els.statusThreers.textContent = 'loading…';
    els.statusThree.className = 'pill loading';
    els.statusThreers.className = 'pill loading';

    const slug = SCENES[index];
    let featureErr = null;
    try { assertSceneFeature(slug); } catch (e) { featureErr = String(e); }

    const threeReady = featureErr
        ? Promise.resolve({ ok: false, err: featureErr })
        : waitFrameReady(els.threeFrame, 60000);
    const threersReady = featureErr
        ? Promise.resolve({ ok: false, err: featureErr })
        : waitFrameReady(els.threersFrame, 120000);

    const [a, b] = await Promise.all([threeReady, threersReady]);
    applyIframeChrome(els.threeFrame);
    syncIframeThemes();
    els.statusThree.textContent = a.ok ? 'ready' : shortStatErr(a.err);
    els.statusThree.className = `pill ${a.ok ? 'ok' : 'bad'}`;
    els.statusThreers.textContent = b.ok ? 'ready' : shortStatErr(b.err);
    els.statusThreers.className = `pill ${b.ok ? 'ok' : 'bad'}`;

    liveSceneStatus.set(slug, { ok: b.ok, err: b.err });
    if (b.ok) {
        const r = stats.get(slug);
        if (r?.err && /RequestDeviceError|maxInterStageShaderComponents/.test(r.err)) {
            const { err: _drop, ...rest } = r;
            stats.set(slug, rest);
        }
        const st = statLabel(slug);
        els.stat.textContent = st.text;
        els.stat.className = `stat ${st.cls}`;
        renderList(els.search.value);
    }
    if (syncOrbitEnabled) scheduleOrbitAlign();
}

function loadScene(i, { forceReload = false } = {}) {
    lastOrbitRelayKey = '';
    index = ((i % SCENES.length) + SCENES.length) % SCENES.length;
    const slug = SCENES[index];
    setHash(slug);
    els.title.textContent = slug;
    els.counter.textContent = `${index + 1} / ${SCENES.length}`;
    const st = statLabel(slug);
    els.stat.textContent = st.text;
    els.stat.className = `stat ${st.cls}`;
    const apis = SCENE_API_MAP[slug];
    els.apis.textContent = apis && apis.length ? apis.join(', ') : 'core renderer / scene graph';
    if (APPROXIMATE_SCENES.includes(slug)) els.apis.textContent += ' · approximate';
    const feat = FEATURE_SCENES.get(slug);
    if (feat) {
        const hint = feat === 'bvh-csg' ? 'BVH_CSG=1 web/build.sh' : 'MESH_BVH=1 web/build.sh';
        els.apis.textContent += ` · feature:${feat} (${hint})`;
        if (feat === 'bvh-csg' && bvhCsgBuildEnabled === false) {
            els.apis.textContent += ' · wasm build missing bvh-csg';
        } else if (feat === 'mesh-bvh' && meshBvhBuildEnabled === false) {
            els.apis.textContent += ' · wasm build missing mesh-bvh';
        }
    }

    if (forceReload) {
        iframeCacheBust = String(Date.now());
    }
    els.threeFrame.src = sceneFrameUrl('threejs', slug);
    els.threersFrame.src = sceneFrameUrl('threers', slug);

    if (els.btnOpenScene) els.btnOpenScene.href = `${BASE}/threejs-${slug}.html`;
    renderList(els.search.value);
    loadSceneSources(slug);
    pollFrames();
}

function goTo(i) {
    loadScene(i);
    const active = els.list.querySelector('.scene-item.active');
    if (active) active.scrollIntoView({ block: 'nearest' });
}

function step(delta) {
    goTo(index + delta);
}

async function loadManifest() {
    const [mainRes, bvhRes, csgRes] = await Promise.all([
        fetch('/tests/parity/scenes-manifest.json'),
        fetch('/tests/parity/scenes-manifest-mesh-bvh.json'),
        fetch('/tests/parity/scenes-manifest-bvh-csg.json'),
    ]);
    if (!mainRes.ok) throw new Error('scenes-manifest.json missing — run node generate-scenes.js');
    const data = await mainRes.json();
    SCENES = [...data.scenes];
    SCENE_API_MAP = { ...(data.apiMap || {}) };
    APPROXIMATE_SCENES = data.approximate || [];

    for (const res of [bvhRes, csgRes]) {
        if (!res.ok) continue;
        const part = await res.json();
        const flag = part.feature || 'mesh-bvh';
        for (const slug of part.scenes || []) {
            if (!SCENES.includes(slug)) SCENES.push(slug);
            FEATURE_SCENES.set(slug, flag);
        }
        Object.assign(SCENE_API_MAP, part.apiMap || {});
    }
}

async function loadMeshBvhBuildFlag() {
    try {
        const bust = iframeCacheBust ? `?v=${encodeURIComponent(iframeCacheBust)}` : '';
        const res = await fetch(`/web/features.js${bust}`);
        if (!res.ok) { meshBvhBuildEnabled = false; bvhCsgBuildEnabled = false; return; }
        const text = await res.text();
        meshBvhBuildEnabled = /meshBvh:\s*true/.test(text);
        bvhCsgBuildEnabled = /bvhCsg:\s*true/.test(text);
    } catch (_) {
        meshBvhBuildEnabled = false;
        bvhCsgBuildEnabled = false;
    }
}

async function loadStats() {
    const rows = [];
    const bust = iframeCacheBust ? `?v=${encodeURIComponent(iframeCacheBust)}` : '';
    for (const url of [
        `/tests/parity/out/compare-results.json${bust}`,
        `/tests/parity/out/compare-results-mesh-bvh.json${bust}`,
        `/tests/parity/out/compare-results-bvh-csg.json${bust}`,
    ]) {
        try {
            const res = await fetch(url);
            if (!res.ok) continue;
            const ct = res.headers.get('content-type') || '';
            if (!ct.includes('json')) continue;
            const part = await res.json();
            if (Array.isArray(part)) rows.push(...part);
        } catch (_) { /* optional */ }
    }
    if (rows.length) {
        stats = new Map(rows.map((r) => {
            const pct = r.pct ?? r.diffPct;
            const ok = r.ok ?? (r.pass && pct != null && pct < 0.01);
            return [r.scene, { ...r, pct, ok }];
        }));
    }
}

function bind() {
    els.btnPrev.addEventListener('click', () => step(-1));
    els.btnNext.addEventListener('click', () => step(1));
    els.btnReload.addEventListener('click', () => loadScene(index, { forceReload: true }));
    els.btnCopyThree.addEventListener('click', () => {
        if (els.threeCode.classList.contains('loading')) return;
        copyText(els.threeCode.dataset.source || '', els.btnCopyThree);
    });
    els.btnCopyThreers.addEventListener('click', () => {
        if (els.threersCode.classList.contains('loading')) return;
        copyText(els.threersCode.dataset.source || '', els.btnCopyThreers);
    });

    function bindCodeTabs(tabsEl, pane) {
        tabsEl.addEventListener('click', (e) => {
            const btn = e.target.closest('.code-tab');
            if (!btn) return;
            const lang = btn.dataset.lang;
            codeLang[`${pane}Lang`] = lang;
            setCodeTabsActive(tabsEl, lang);
            refreshCodePane(pane);
        });
    }
    bindCodeTabs(els.tabsThree, 'three');
    bindCodeTabs(els.tabsThreers, 'threers');
    els.search.addEventListener('input', () => {
        renderList(els.search.value);
        if (gridMode) renderGrid(els.search.value);
    });
    document.getElementById('btn-grid').addEventListener('click', () => {
        gridMode = true;
        syncViewMode();
        renderGrid(els.search.value);
    });
    document.getElementById('btn-single').addEventListener('click', () => {
        gridMode = false;
        syncViewMode();
    });
    window.addEventListener('hashchange', () => {
        const i = slugFromHash();
        if (i != null && i !== index) goTo(i);
    });
    window.addEventListener('keydown', (e) => {
        if (e.target.matches('input, textarea')) return;
        if (e.key === 'ArrowLeft' || e.key === 'k') { e.preventDefault(); step(-1); }
        if (e.key === 'ArrowRight' || e.key === 'j') { e.preventDefault(); step(1); }
        if (e.key === 'r') { e.preventDefault(); loadScene(index, { forceReload: true }); }
    });

    if (els.syncOrbit) {
        els.syncOrbit.addEventListener('change', async () => {
            syncOrbitEnabled = els.syncOrbit.checked;
            try { localStorage.setItem('parity-sync-orbit', syncOrbitEnabled ? '1' : '0'); } catch (_) { /* private */ }
            broadcastOrbitSyncEnabled(syncOrbitEnabled);
            if (syncOrbitEnabled) scheduleOrbitAlign();
        });
    }
    window.addEventListener('message', handleOrbitRelay);
    startContinuousOrbitSync();
}

async function initCompare() {
    initTheme();
    syncOrbitEnabled = (() => {
        try {
            const v = localStorage.getItem('parity-sync-orbit');
            if (v === '0') return false;
            return true;
        } catch (_) { return true; }
    })();
    if (els.syncOrbit) els.syncOrbit.checked = syncOrbitEnabled;
    iframeCacheBust = await fetchAssetVersion();
    await loadManifest();
    await loadMeshBvhBuildFlag();
    await loadStats();
    bind();
    syncViewMode();
    const fromHash = slugFromHash();
    const start = fromHash != null ? fromHash : 0;
    renderList();
    goTo(start);
}

window.initCompare = initCompare;
