//! Persistent threers runner for the parity compare UI (`threers-runner.html`).
//!
//! Loads wasm + WebGPU once, then swaps scenes via `postMessage` from the parent
//! compare page — avoids reloading the wasm module per scene.
const _assetV = new URLSearchParams(location.search).get('v') || '';
const _shimUrl = `/web/threejs-shim.js${_assetV ? `?v=${encodeURIComponent(_assetV)}` : ''}`;
const { default: THREE, initThreers, seedRandom } = await import(_shimUrl);

const canvas = document.getElementById('c');
const errEl = document.getElementById('err');

/** @type {import('/web/threejs-shim.js').WebGLRenderer | null} */
let renderer = null;

/** Safari does not expose `AsyncFunction` as a global — derive it from async fn syntax. */
const AsyncFunction = (async function () {}).constructor;

function extractSceneSetup(html) {
    const mod = html.match(/<script[^>]*type=["']module["'][^>]*>([\s\S]*?)<\/script>/i);
    if (!mod) throw new Error('no module script in scene html');
    let body = mod[1];
    body = body.replace(/^import[\s\S]*?;\s*/m, '');
    body = body.replace(/^const errEl[\s\S]*?;\s*/m, '');
    body = body.replace(/^function showError\([\s\S]*?\}\s*/m, '');
    body = body.replace(/^\s*(?:const|let)\s+canvas\s*=\s*document\.getElementById\([^)]+\);\s*/gm, '');
    body = body.replace(/\(async \(\) => \{[\s\S]*?try \{[\s\S]*?await initThreers\(\s*['"][^'"]+['"]\s*\);\s*/m, '');
    // Scenes without try/catch (e.g. ssao-rtcopy).
    body = body.replace(/\(async \(\) => \{[\s\S]*?await initThreers\(\s*['"][^'"]+['"]\s*\);\s*/m, '');
    body = body.replace(/^\}\)\(\);\s*/m, '');
    // Non-greedy — nested parens in getElementById('c') break [^)]+ patterns.
    body = body.replace(/^\s*const r = await THREE\.WebGLRenderer\.create\([\s\S]*?\);\s*/gm, '');
    body = body.replace(/^\s*const renderer = await THREE\.WebGLRenderer\.create\([\s\S]*?\);\s*/gm, '');
    // Drop bootstrap setSize right after renderer create (runner sets size once up front).
    body = body.replace(/^\s*r\.setSize\(\s*800\s*,\s*600(?:\s*,\s*false)?\s*\);\s*/m, '');
    body = body.replace(/\bawait composer\.render\(\);\s*/g, '');
    body = body.replace(/await new Promise\(rs => requestAnimationFrame\(\(\) => rs\(\)\)\);\s*/g, '');
    body = body.replace(/document\.body\.dataset\.ready = 'true';\s*/g, '');
    body = body.replace(/} catch \(e\) \{[\s\S]*$/m, '');
    return `const r = renderer;
r.setRenderTarget(null);
r.setSize(800, 600, false);
${body.trim()}`;
}

/** True when the extracted setup already draws to the canvas (or composer does). */
function setupPresents(setup) {
    if (/\bawait\s+composer\.render\s*\(\s*\)/.test(setup)) return true;
    if (/\bcomposer\.render\s*\(\s*\)/.test(setup)) return true;
    if (/\brenderAndApply\s*\(/.test(setup)) return true;
    if (/\brenderAndOutline\s*\(/.test(setup)) return true;
    if (/\bbloom\.apply\s*\(/.test(setup)) return true;
    if (/\bssao\.apply\s*\(/.test(setup)) return true;
    if (/\bssr\.renderAndApply\s*\(/.test(setup)) return true;
    if (/\br\.applyPostFx\s*\(/.test(setup)) return true;
    if (/\br\.render\s*\(\s*scene\s*,\s*cam\s*\)\s*;/.test(setup)) return true;
    if (/\brenderer\.render\s*\(\s*scene\s*,\s*camera\s*\)\s*;/.test(setup)) return true;
    if (/\brenderer\.render\s*\(\s*scene\s*,\s*cam\s*\)\s*;/.test(setup)) return true;
    return false;
}

function buildRenderTail(setup) {
    if (/\bEffectComposer\b/.test(setup)) {
        return 'await composer.render();';
    }
    if (/\bawait\s+composer\.render\s*\(\s*\)/.test(setup)) {
        return 'await composer.render();';
    }
    if (/\bcomposer\.render\s*\(\s*\)/.test(setup)) {
        return 'composer.render();';
    }
    if (setupPresents(setup)) {
        return '';
    }
    return 'r.render(scene, cam);';
}

function buildSceneFn(setup) {
    const tail = buildRenderTail(setup);
    const body = [
        setup,
        tail,
        'await new Promise((rs) => requestAnimationFrame(rs));',
    ].filter(Boolean).join('\n');
    return new AsyncFunction('THREE', 'renderer', 'canvas', 'seedRandom', body);
}

async function runScene(slug) {
    errEl.style.display = 'none';
    errEl.textContent = '';
    document.body.dataset.ready = '';
    canvas.style.visibility = 'visible';
    // Avoid showing geometry from the previously loaded scene while the next
    // one is being constructed (common source of "wrong mesh" reports in UI).
    if (renderer) {
        renderer.setRenderTarget(null);
        renderer.setSize(800, 600, false);
        renderer.render(new THREE.Scene(), new THREE.PerspectiveCamera());
    }

    const res = await fetch(`/tests/parity/scenes/threers-${slug}.html`);
    if (!res.ok) throw new Error(`scene not found: threers-${slug}.html`);
    const html = await res.text();
    const setup = extractSceneSetup(html);
    const run = buildSceneFn(setup);
    await run(THREE, renderer, canvas, seedRandom);
    document.body.dataset.ready = 'true';
}

window.addEventListener('message', async (event) => {
    const data = event.data;
    if (!data || data.type !== 'parity-scene') return;
    const slug = data.slug;
    try {
        await runScene(slug);
        window.parent.postMessage({ type: 'parity-threers', slug, ok: true }, '*');
    } catch (e) {
        console.error(e);
        errEl.style.display = 'block';
        errEl.textContent = e?.stack || String(e);
        document.body.dataset.ready = 'error';
        // Hide stale frame from the previous scene so a failed load cannot
        // look like a mesh mismatch beside the three.js reference.
        canvas.style.visibility = 'hidden';
        window.parent.postMessage({ type: 'parity-threers', slug, ok: false, err: String(e) }, '*');
    }
});

async function wasmUrl() {
    let id = _assetV;
    if (!id) {
        try {
            const res = await fetch('/web/pkg/build-id.txt', { cache: 'no-store' });
            if (res.ok) id = (await res.text()).trim();
        } catch (_) { /* optional */ }
    }
    return `/web/pkg/threers_bg.wasm${id ? `?v=${encodeURIComponent(id)}` : ''}`;
}

try {
    await initThreers(await wasmUrl());
    renderer = await THREE.WebGLRenderer.create(canvas);
    renderer.setSize(800, 600, false);
    window.parent.postMessage({ type: 'parity-threers-boot', ok: true }, '*');
} catch (e) {
    console.error(e);
    errEl.style.display = 'block';
    errEl.textContent = e?.stack || String(e);
    document.body.dataset.ready = 'error';
    window.parent.postMessage({ type: 'parity-threers-boot', ok: false, err: String(e) }, '*');
}
