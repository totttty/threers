/** Shared orbit-control helpers for parity scene runners (three.js + threers). */

const THREE_ORBIT_URL = 'https://unpkg.com/three@0.165.0/examples/jsm/controls/OrbitControls.js';

export function detectCamVar(source) {
    if (/\b(?:const|let|var)\s+cam\b/.test(source)) return 'cam';
    if (/\b(?:const|let|var)\s+camera\b/.test(source)) return 'camera';
    return null;
}

/** Renderer variable used for scene/cam draws (avoid CSG locals like `const r = r1`). */
function detectRenderVar(setup, { threers = false } = {}) {
    if (/\b(?:const|let|var)\s+renderer\b/.test(setup)) return 'renderer';
    if (threers) return 'r';
    if (/\br\.render\s*\(\s*scene\s*,/.test(setup) && /\bconst\s+r\s*=/.test(setup)) return 'r';
    return null;
}

export function sceneHasOrbitTarget(source) {
    return /\b(?:const|let|var)\s+scene\b/.test(source) && detectCamVar(source) != null;
}

/** First-frame render (may await composer). */
export function buildInitialRender(setup, { threers = false } = {}) {
    if (/\bEffectComposer\b/.test(setup)) return 'await composer.render();';
    if (/\bawait\s+composer\.render\s*\(\s*\)/.test(setup)) return 'await composer.render();';
    if (/\bcomposer\.render\s*\(\s*\)/.test(setup)) return 'composer.render();';
    if (setupPresents(setup)) return '';
    const cam = detectCamVar(setup);
    if (!cam) return '';
    const draw = detectRenderVar(setup, { threers });
    if (draw) return `${draw}.render(scene, ${cam});`;
    return threers ? `r.render(scene, ${cam});` : `renderer.render(scene, ${cam});`;
}

/** Per-frame render inside the orbit loop (no await). */
export function buildLoopRender(setup, { threers = false } = {}) {
    if (/\bEffectComposer\b/.test(setup)) return 'composer.render();';
    const cam = detectCamVar(setup);
    if (!cam) return '';
    if (setupPresents(setup) && !/\b(?:r|renderer)\.render\s*\(\s*scene\s*,/.test(setup)) {
        return '';
    }
    const draw = detectRenderVar(setup, { threers });
    if (draw) return `${draw}.render(scene, ${cam});`;
    return '';
}

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

/** Unwrap `(async () => { try { … } catch … })();` so runner orbit tail shares scene scope. */
export function unwrapAsyncSceneIife(source) {
    const unwrapped = source.replace(
        /\(async\s*\(\)\s*=>\s*\{\s*try\s*\{([\s\S]*)\}\s*catch\s*\([^)]*\)\s*\{[\s\S]*?\}\s*\}\)\(\)\s*;?/,
        (_, body) => body.trim(),
    );
    if (unwrapped !== source) return unwrapped;
    return source.replace(
        /\(async\s*\(\)\s*=>\s*\{([\s\S]*)\}\s*\)\(\)\s*;?/,
        (_, body) => body.trim(),
    );
}

/** Remove inline orbit loops from scene HTML (runners inject sync-aware controls). */
export function stripInlineOrbit(source) {
    return source
        .replace(/import\s*\{[^}]*installOrbitSyncBridge[^}]*\}\s*from\s*['"][^'"]+['"];\s*/g, '')
        .replace(/const\s*\{\s*installOrbitSyncBridge\s*\}\s*=\s*await\s*import\([^)]+\);\s*/g, '')
        .replace(/installOrbitSyncBridge\(\);\s*/g, '')
        .replace(/const\s*\{\s*OrbitControls\s*\}\s*=\s*await\s*import\([^)]+\);\s*/g, '')
        .replace(/const\s+controls\s*=\s*new\s+(?:THREE\.)?OrbitControls\([^)]+\);\s*/g, '')
        .replace(/window\.__parityOrbit\s*=[^;]+;\s*/g, '')
        .replace(/window\.__parityOrbitCtx\s*=[^;]+;\s*/g, '')
        .replace(/\(function\s+orbitLoop\s*\(\)\s*\{[\s\S]*?requestAnimationFrame\(orbitLoop\);\s*\}\)\(\);\s*/g, '')
        .replace(/if \(typeof window\.__parityPrimeOrbitCtx === ['"]function['"]\) window\.__parityPrimeOrbitCtx\(window\.__parityOrbitCtx\);\s*/g, '')
        .replace(/document\.body\.dataset\.triCount\s*=[^;]+;\s*/g, '');
}

export function buildThreersOrbitTail(setup) {
    const cam = detectCamVar(setup);
    const loopRender = buildLoopRender(setup, { threers: true });
    if (!cam || !loopRender) return '';
    return [
        'if (window.__parityOrbit?.dispose) window.__parityOrbit.dispose();',
        'window.__parityOrbitLoopGen = (window.__parityOrbitLoopGen || 0) + 1;',
        'const __parityOrbitLoopId = window.__parityOrbitLoopGen;',
        `window.__parityOrbit = new THREE.OrbitControls(${cam}, canvas);`,
        `const __parityOrbit = window.__parityOrbit;`,
        `window.__parityOrbitCtx = { side: 'threers', orbit: __parityOrbit, cam: ${cam}, render: () => { ${loopRender} } };`,
        'if (typeof window.__parityPrimeOrbitCtx === "function") window.__parityPrimeOrbitCtx(window.__parityOrbitCtx);',
        'canvas?.focus?.();',
        'document.body.dataset.ready = "true";',
        '(function __parityOrbitLoop() {',
        '  if (__parityOrbitLoopId !== window.__parityOrbitLoopGen) return;',
        '  __parityOrbit.update();',
        `  ${loopRender}`,
        '  if (window.__parityOrbitSyncHook) window.__parityOrbitSyncHook();',
        '  requestAnimationFrame(__parityOrbitLoop);',
        '})();',
    ].join('\n');
}

export function buildThreejsOrbitTail(setup) {
    const cam = detectCamVar(setup);
    const loopRender = buildLoopRender(setup, { threers: false });
    if (!cam || !loopRender) {
        return "document.body.dataset.ready = 'true';";
    }
    return [
        'if (window.__parityOrbit?.dispose) window.__parityOrbit.dispose();',
        'window.__parityOrbitLoopGen = (window.__parityOrbitLoopGen || 0) + 1;',
        'const __parityOrbitLoopId = window.__parityOrbitLoopGen;',
        `const { OrbitControls } = await import('${THREE_ORBIT_URL}');`,
        "const __cv = document.getElementById('c') || document.querySelector('canvas');",
        `window.__parityOrbit = new OrbitControls(${cam}, __cv);`,
        'const __parityOrbit = window.__parityOrbit;',
        `window.__parityOrbitCtx = { side: 'three', orbit: __parityOrbit, cam: ${cam}, render: () => { ${loopRender} } };`,
        'if (typeof window.__parityPrimeOrbitCtx === "function") window.__parityPrimeOrbitCtx(window.__parityOrbitCtx);',
        '__cv?.focus?.();',
        "document.body.dataset.ready = 'true';",
        '(function __parityOrbitLoop() {',
        '  if (__parityOrbitLoopId !== window.__parityOrbitLoopGen) return;',
        '  __parityOrbit.update();',
        `  ${loopRender}`,
        '  if (window.__parityOrbitSyncHook) window.__parityOrbitSyncHook();',
        '  requestAnimationFrame(__parityOrbitLoop);',
        '})();',
    ].join('\n');
}

export function prepareThreejsModule(html) {
    const mod = html.match(/<script[^>]*type=["']module["'][^>]*>([\s\S]*?)<\/script>/i);
    if (!mod) throw new Error('no module script in scene html');
    let body = mod[1];
    body = body.replace(/document\.getElementById\(['"]canvas['"]\)/g, "document.getElementById('c')");
    body = body.replace(/document\.body\.dataset\.ready\s*=\s*['"]true['"];\s*/g, '');
    body = stripInlineOrbit(body);
    body = unwrapAsyncSceneIife(body);
    return body.trim();
}

export function extractImportMap(html) {
    const m = html.match(/<script[^>]*type=["']importmap["'][^>]*>([\s\S]*?)<\/script>/i);
    return m ? m[1].trim() : null;
}
