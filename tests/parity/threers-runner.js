//! Persistent threers runner for the parity compare UI (`threers-runner.html`).
//!
//! Loads wasm + WebGPU once, then swaps scenes via `postMessage` from the parent
//! compare page — avoids reloading the wasm module per scene.
const _assetV = new URLSearchParams(location.search).get('v') || '';
const _shimUrl = '/web/threejs-shim.js';
const { default: THREE, initThreers, seedRandom } = await import(_shimUrl);
import { buildInitialRender, buildThreersOrbitTail, detectCamVar, stripInlineOrbit } from '/tests/parity/parity-interact.js';
import { installOrbitSyncBridge } from '/tests/parity/parity-orbit-sync.js';

installOrbitSyncBridge();

const canvas = document.getElementById('c');
const errEl = document.getElementById('err');

/** @type {import('/web/threejs-shim.js').WebGLRenderer | null} */
let renderer = null;

/** mesh-bvh addon exports when wasm was built with MESH_BVH=1 (null otherwise). */
let meshBvh = null;

/** bvh-csg addon exports when wasm was built with BVH_CSG=1 (null otherwise). */
let bvhCsg = null;

/** Safari does not expose `AsyncFunction` as a global — derive it from async fn syntax. */
const AsyncFunction = (async function () {}).constructor;

function sceneNeedsMeshBvh(html) {
    return /\bMeshBVH\b/.test(html)
        || /\bNOT_INTERSECTED\b/.test(html)
        || /\bINTERSECTED\b/.test(html)
        || /\bacceleratedRaycast\b/.test(html)
        || /\bshapecast\s*\(/.test(html);
}

function sceneNeedsBvhCsg(html) {
    return /\bBrush\b/.test(html)
        || /\bEvaluator\b/.test(html)
        || /\bADDITION\b/.test(html)
        || /\bSUBTRACTION\b/.test(html)
        || /\bevaluateHierarchy\b/.test(html)
        || /\bOperation\b/.test(html)
        || /\binstallBvhCsg\b/.test(html);
}

function bvhCsgUrl() {
    return '/web/bvh-csg-addon.js';
}

async function ensureBvhCsg() {
    if (bvhCsg) return bvhCsg;
    await ensureMeshBvh();
    const featMod = await import(featuresUrl());
    if (!featMod.features?.bvhCsg) {
        throw new Error('bvh-csg scenes require BVH_CSG=1 web/build.sh (features.bvhCsg is false)');
    }
    bvhCsg = await import(bvhCsgUrl());
    bvhCsg.installBvhCsg(THREE);
    return bvhCsg;
}

function bvhCsgBindings() {
    if (!bvhCsg) return '';
    return [
        'const Brush = bvhCsg.Brush;',
        'const Evaluator = bvhCsg.Evaluator;',
        'const Operation = bvhCsg.Operation;',
        'const OperationGroup = bvhCsg.OperationGroup;',
        'const ADDITION = bvhCsg.ADDITION;',
        'const SUBTRACTION = bvhCsg.SUBTRACTION;',
        'const REVERSE_SUBTRACTION = bvhCsg.REVERSE_SUBTRACTION;',
        'const INTERSECTION = bvhCsg.INTERSECTION;',
        'const DIFFERENCE = bvhCsg.DIFFERENCE;',
        'const HOLLOW_SUBTRACTION = bvhCsg.HOLLOW_SUBTRACTION;',
        'const HOLLOW_INTERSECTION = bvhCsg.HOLLOW_INTERSECTION;',
        'const geometryToBufferGeometry = THREE.geometryToBufferGeometry;',
    ].join('\n');
}

function meshBvhUrl() {
    return '/web/mesh-bvh-addon.js';
}

function featuresUrl() {
    return '/web/features.js';
}

async function ensureMeshBvh() {
    if (meshBvh) return meshBvh;
    const featMod = await import(featuresUrl());
    if (!featMod.features?.meshBvh) {
        throw new Error('mesh-bvh scenes require MESH_BVH=1 web/build.sh (features.meshBvh is false)');
    }
    meshBvh = await import(meshBvhUrl());
    meshBvh.installMeshBvh(THREE);
    return meshBvh;
}

function meshBvhBindings() {
    if (!meshBvh) return '';
    return [
        'const MeshBVH = meshBvh.MeshBVH;',
        'const NOT_INTERSECTED = meshBvh.NOT_INTERSECTED;',
        'const INTERSECTED = meshBvh.INTERSECTED;',
        'const CONTAINED = meshBvh.CONTAINED;',
        'const acceleratedRaycast = meshBvh.acceleratedRaycast;',
        'const computeBoundsTree = meshBvh.computeBoundsTree;',
        'const disposeBoundsTree = meshBvh.disposeBoundsTree;',
        'const StaticGeometryGenerator = meshBvh.StaticGeometryGenerator;',
    ].join('\n');
}

function extractSceneSetup(html) {
    const mod = html.match(/<script[^>]*type=["']module["'][^>]*>([\s\S]*?)<\/script>/i);
    if (!mod) throw new Error('no module script in scene html');
    let body = mod[1];
    body = body.replace(/^import\s[\s\S]*?;\s*/gm, '');
    body = body.replace(/^const errEl[\s\S]*?;\s*/m, '');
    body = body.replace(/^function showError\([\s\S]*?\}\s*/m, '');
    body = body.replace(/^\s*(?:const|let)\s+canvas\s*=\s*document\.getElementById\([^)]+\);\s*/gm, '');
    body = body.replace(/\(async \(\) => \{[\s\S]*?try \{[\s\S]*?await initThreers\(\s*['"][^'"]+['"]\s*\);\s*/m, '');
    // Scenes without try/catch (e.g. ssao-rtcopy).
    body = body.replace(/\(async \(\) => \{[\s\S]*?await initThreers\(\s*['"][^'"]+['"]\s*\);\s*/m, '');
    body = body.replace(/^\s*installMeshBvh\(THREE\);\s*/gm, '');
    body = body.replace(/^\s*installBvhCsg\(THREE\);\s*/gm, '');
    body = body.replace(/^\}\)\(\);\s*/m, '');
    body = body.replace(/^\s*const r = await THREE\.WebGLRenderer\.create\([\s\S]*?\);\s*/gm, '');
    body = body.replace(/^\s*const renderer = await THREE\.WebGLRenderer\.create\([\s\S]*?\);\s*/gm, '');
    // Drop bootstrap setSize right after renderer create (runner sets size once up front).
    body = body.replace(/^\s*r\.setSize\(\s*800\s*,\s*600(?:\s*,\s*false)?\s*\);\s*/m, '');
    body = body.replace(/\bawait composer\.render\(\);\s*/g, '');
    body = body.replace(/await new Promise\(rs => requestAnimationFrame\(\(\) => rs\(\)\)\);\s*/g, '');
    body = body.replace(/document\.body\.dataset\.ready = 'true';\s*/g, '');
    body = body.replace(/} catch \(e\) \{[\s\S]*$/m, '');
    body = stripInlineOrbit(body);
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
    const expr = buildInitialRender(setup, { threers: true });
    if (expr) return expr;
    if (setupPresents(setup)) return '';
    const cam = detectCamVar(setup);
    return cam ? `r.render(scene, ${cam});` : '';
}

function buildSceneFn(setup) {
    const tail = buildRenderTail(setup);
    const orbitTail = buildThreersOrbitTail(setup);
    const body = [
        meshBvhBindings(),
        bvhCsgBindings(),
        setup,
        tail,
        'await new Promise((rs) => requestAnimationFrame(rs));',
        orbitTail,
    ].filter(Boolean).join('\n');
    return new AsyncFunction('THREE', 'renderer', 'canvas', 'seedRandom', 'meshBvh', 'bvhCsg', body);
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
    if (sceneNeedsMeshBvh(html)) await ensureMeshBvh();
    if (sceneNeedsBvhCsg(html)) await ensureBvhCsg();
    const setup = extractSceneSetup(html);
    const run = buildSceneFn(setup);
    await run(THREE, renderer, canvas, seedRandom, meshBvh, bvhCsg);
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
    try {
        const featMod = await import(featuresUrl());
        if (featMod.features?.meshBvh) {
            meshBvh = await import(meshBvhUrl());
            meshBvh.installMeshBvh(THREE);
        }
        // bvh-csg: lazy-loaded per scene via ensureBvhCsg() (BVH_CSG=1 only)
    } catch (e) {
        console.warn('mesh-bvh addon not loaded at boot', e);
    }
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
