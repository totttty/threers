//! three.js parity runner — loads a scene module and attaches OrbitControls for compare UI.
import {
    buildThreejsOrbitTail,
    extractImportMap,
    prepareThreejsModule,
} from '/tests/parity/parity-interact.js';
import { installOrbitSyncBridge } from '/tests/parity/parity-orbit-sync.js';

installOrbitSyncBridge();

const errEl = document.getElementById('err');
const params = new URLSearchParams(location.search);
const slug = params.get('slug');

function showError(e) {
    errEl.style.display = 'block';
    errEl.textContent = (e && e.stack) ? e.stack : String(e);
    document.body.dataset.ready = 'error';
}

async function run() {
    if (!slug) throw new Error('missing ?slug= query param');

    const res = await fetch(`/tests/parity/scenes/threejs-${slug}.html`);
    if (!res.ok) throw new Error(`scene not found: threejs-${slug}.html`);
    const html = await res.text();

    const importMap = extractImportMap(html);
    if (importMap) {
        let el = document.querySelector('script[type="importmap"]');
        if (!el) {
            el = document.createElement('script');
            el.type = 'importmap';
            document.head.prepend(el);
        }
        el.textContent = importMap;
    }

    const setup = prepareThreejsModule(html);
    const orbitTail = buildThreejsOrbitTail(setup);
    const script = document.createElement('script');
    script.type = 'module';
    script.textContent = [
        setup,
        'try {',
        orbitTail,
        '} catch (e) {',
        "  const errEl = document.getElementById('err');",
        "  if (errEl) { errEl.style.display = 'block'; errEl.textContent = (e && e.stack) ? e.stack : String(e); }",
        "  document.body.dataset.ready = 'error';",
        '  console.error(e);',
        '}',
    ].join('\n');
    document.body.appendChild(script);
}

try {
    await run();
} catch (e) {
    console.error(e);
    showError(e);
}
