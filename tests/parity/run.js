// End-to-end parity suite: for each scene, render the three.js reference and
// the threers (wasm/wgpu) version, screenshot both, pixelmatch. Aggregate.

import puppeteer from 'puppeteer';
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { ALL_SCENES, coverageReport, APPROXIMATE_SCENES, APPROXIMATE_THRESHOLD } from './coverage-manifest.js';
import { apiCoverageReport } from './api-inventory.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(__dirname, 'out');
fs.mkdirSync(outDir, { recursive: true });

const port = 8087;
let server = null;
let spawnedServer = false;
try {
    server = spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'ignore' });
    spawnedServer = true;
    server.on('error', () => { spawnedServer = false; });
} catch (_) { /* use existing server on port */ }
await new Promise(r => setTimeout(r, 300));

const SCENES = ALL_SCENES;
const cov = coverageReport();
const apiCov = apiCoverageReport();
console.log(`Parity suite: ${cov.totalScenes} scenes (${cov.coreScenes} core + ${cov.generatedScenes} generated; ${cov.approximateScenes} approximate postfx/animation)`);
console.log(`API surface: ${apiCov.totalExports} exports, ~${apiCov.exportCoveragePct}% with parity scenes (${apiCov.sceneTestedApis} APIs mapped)`);

async function shoot(browser, url) {
    const page = await browser.newPage();
    await page.setViewport({ width: 800, height: 600 });
    const consoleMsgs = [];
    page.on('console', async m => {
        const t = m.type();
        const parts = [];
        for (const a of m.args()) {
            try {
                const d = await a.executionContext().evaluate(o => {
                    if (typeof o === 'string') return o;
                    try { return o.message || o.toString() || JSON.stringify(o); } catch { return String(o); }
                }, a);
                parts.push(d);
            } catch { parts.push(String(a)); }
        }
        consoleMsgs.push(`[${t}] ${parts.join(' ') || m.text()}`);
    });
    page.on('pageerror', e => consoleMsgs.push(`[ERROR] ${e.message}`));
    let status, errText, png;
    try {
        await page.goto(url, { waitUntil: 'load', timeout: 30000 });
        try {
            await page.waitForFunction(() => document.body.dataset.ready, { timeout: 30000 });
        } catch (e) {
            consoleMsgs.push(`[ERROR] waitForReady timed out — page never set body.dataset.ready`);
        }
        status = await page.evaluate(() => document.body.dataset.ready).catch(() => 'timeout');
        if (status !== 'true') {
            errText = await page.evaluate(() => document.getElementById('err')?.textContent || '(no #err element)').catch(() => '(eval failed)');
        }
        png = await page.screenshot({ type: 'png', clip: { x: 0, y: 0, width: 800, height: 600 } }).catch(() => null);
    } finally {
        await page.close().catch(() => {});
    }
    return { status, errText, consoleMsgs, png };
}

console.log('Launching headless Chromium (WebGPU enabled)...');
const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium',
    headless: 'new',
    args: [
        '--enable-unsafe-webgpu',
        '--enable-features=Vulkan,WebGPU',
        '--use-vulkan=swiftshader',
        '--no-sandbox',
        '--disable-dev-shm-usage',
        '--ignore-gpu-blocklist',
        '--enable-webgl',
    ],
});

const results = [];
for (const scene of SCENES) {
    process.stdout.write(`▶ ${scene.padEnd(16)} `);
    const refUrl = `http://localhost:${port}/tests/parity/scenes/threejs-${scene}.html`;
    const cmpUrl = `http://localhost:${port}/tests/parity/scenes/threers-${scene}.html`;
    const ref = await shoot(browser, refUrl);
    const cmp = await shoot(browser, cmpUrl);
    // page.screenshot() returns a Uint8Array under newer puppeteer; pngjs needs a Buffer.
    const refBuf = ref.png ? Buffer.from(ref.png) : Buffer.alloc(0);
    const cmpBuf = cmp.png ? Buffer.from(cmp.png) : Buffer.alloc(0);
    fs.writeFileSync(path.join(outDir, `${scene}-threejs.png`), refBuf);
    fs.writeFileSync(path.join(outDir, `${scene}-threers.png`), cmpBuf);

    if (ref.status !== 'true' || cmp.status !== 'true') {
        console.log('FAIL (page error)');
        if (ref.status !== 'true') console.log(`   three.js: ${ref.errText}`);
        if (cmp.status !== 'true') {
            console.log(`   threers : ${cmp.errText}`);
            cmp.consoleMsgs.filter(m => /error|warn/i.test(m)).slice(0, 8).forEach(m => console.log(`     ${m}`));
        }
        results.push({ scene, ok: false, err: cmp.errText || ref.errText });
        continue;
    }
    const refPng = PNG.sync.read(refBuf);
    const cmpPng = PNG.sync.read(cmpBuf);
    const diff = new PNG({ width: refPng.width, height: refPng.height });
    // threshold 0.2 — strict enough to catch convention/formula bugs (we caught
    // both the missing gamma encode and the ambient/π factor at this level)
    // while ignoring sub-byte precision noise from FP/GGX implementation diffs.
    const diffCount = pixelmatch(refPng.data, cmpPng.data, diff.data, refPng.width, refPng.height, { threshold: 0.2 });
    fs.writeFileSync(path.join(outDir, `${scene}-diff.png`), PNG.sync.write(diff));
    const totalPx = refPng.width * refPng.height;
    const pct = diffCount / totalPx * 100;
    const approx = APPROXIMATE_SCENES.includes(scene);
    const pass = diffCount === 0
        || (approx ? pct < APPROXIMATE_THRESHOLD : pct < 5);
    let tag = diffCount === 0 ? '✅ PERFECT'
              : pct < 0.5      ? '✅ identical'
              : pct < 5        ? '✓ matching'
              : pct < 15       ? '⚠ minor'
              :                  '❌ diverged';
    if (approx && pass && diffCount > 0) tag = '⚠ approximate';
    console.log(`${tag}  ${pct.toFixed(2)}% (${diffCount}/${totalPx})${approx ? ' [approx]' : ''}`);
    // Surface threers console messages on big divergence — helpful for debugging.
    if (pct >= 5) {
        cmp.consoleMsgs.filter(m => !/Failed to load resource|deprecated parameters/.test(m)).slice(0, 12).forEach(m => console.log(`     ${m}`));
    }
    results.push({ scene, ok: diffCount === 0, pass, pct, diffCount, approx });
}

fs.writeFileSync(
    path.join(outDir, 'compare-results.json'),
    JSON.stringify(results.map(r => ({
        scene: r.scene,
        ok: r.ok,
        pass: r.pass,
        pct: r.pct,
        diffCount: r.diffCount,
        approx: r.approx,
        err: r.err,
    })), null, 2),
);

await browser.close();
if (spawnedServer) server?.kill();

console.log('\n─── Summary ───');
console.log(`API exports: ${apiCov.totalExports} total, ${apiCov.exportCoveragePct}% with dedicated parity scenes`);
const passed = results.filter(r => r.ok).length;
console.log(`${passed}/${results.length} pixel-perfect`);
for (const r of results) {
    if (r.ok) console.log(`  ✅ ${r.scene}`);
    else if (r.err) console.log(`  ❌ ${r.scene}  (load error)`);
    else if (r.pass && r.approx) console.log(`  ⚠ ${r.scene}  ${r.pct.toFixed(2)}% (approx)`);
    else if (r.pass) console.log(`  ✓ ${r.scene}  ${r.pct.toFixed(2)}%`);
    else console.log(`  ${r.pct < 5 ? '✓' : '⚠'} ${r.scene}  ${r.pct.toFixed(2)}%`);
}
process.exit(results.every(r => r.ok || r.pass) ? 0 : 1);
