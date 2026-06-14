#!/usr/bin/env node
/** Dotscreen parity isolation — find which stage owns the ~4% diff. */
import puppeteer from 'puppeteer';
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8125;
const outDir = path.join(__dirname, 'out', 'dotscreen-bench');
fs.mkdirSync(outDir, { recursive: true });

const SCENE = `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x334455);
scene.add(new THREE.Mesh(new THREE.TorusKnotGeometry(0.5, 0.15, 64, 8), new THREE.MeshBasicMaterial({ color: 0xaaccff })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
`;

const THREE_IMPORTS = `
import * as THREE from 'three';
import { EffectComposer } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/RenderPass.js';
import { ShaderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/ShaderPass.js';
import { CopyShader } from 'https://unpkg.com/three@0.165.0/examples/jsm/shaders/CopyShader.js';
import { DotScreenPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/DotScreenPass.js';
`;

function shellThree(body) {
    return `<!DOCTYPE html><html><body><canvas id="c" width="800" height="600"></canvas>
<script type="importmap">{"imports":{"three":"https://unpkg.com/three@0.165.0/build/three.module.js"}}</script>
<script type="module">${THREE_IMPORTS}
const r = new THREE.WebGLRenderer({ canvas: document.getElementById('c'), antialias: false });
r.setSize(800, 600, false); r.setPixelRatio(1);
${SCENE}
${body}
document.body.dataset.ready = 'true';
</script></body></html>`;
}

function shellThreers(body) {
    return `<!DOCTYPE html><html><body><canvas id="c" width="800" height="600"></canvas>
<script type="module">
import THREE, { initThreers } from '/web/threejs-shim.js';
(async () => {
  await initThreers('/web/pkg/threers_bg.wasm');
  ${SCENE}
  const r = await THREE.WebGLRenderer.create(document.getElementById('c'));
  r.setSize(800, 600);
  ${body}
  await new Promise(rs => requestAnimationFrame(rs));
  document.body.dataset.ready = 'true';
})();
</script></body></html>`;
}

const BENCHES = [
    {
        id: 'A-rt-copy',
        label: 'Base RT: three copy vs threers copy',
        three: `const c=new EffectComposer(r); c.addPass(new RenderPass(scene,cam)); c.addPass(new ShaderPass(CopyShader)); c.render();`,
        threers: `const c=new THREE.EffectComposer(r); c.addPass(new THREE.RenderPass(scene,cam));
c.addPass({enabled:true,needsSwap:true,renderToScreen:true,render(renderer,wb,rb){renderer.applyPostFx(rb,0,0,[1,0,0,0]);}});
await c.render();`,
    },
    {
        id: 'B-three-dot-vs-copy',
        label: 'three dotscreen vs threers copy (effect size on ref)',
        three: `const c=new EffectComposer(r); c.addPass(new RenderPass(scene,cam)); c.addPass(new DotScreenPass(new THREE.Vector2(0.5,0.5),1.2,1.4)); c.render();`,
        threers: `const c=new THREE.EffectComposer(r); c.addPass(new THREE.RenderPass(scene,cam));
c.addPass({enabled:true,needsSwap:true,renderToScreen:true,render(renderer,wb,rb){renderer.applyPostFx(rb,0,0,[1,0,0,0]);}});
await c.render();`,
    },
    {
        id: 'C-full',
        label: 'Full dotscreen production',
        three: `const c=new EffectComposer(r); c.addPass(new RenderPass(scene,cam)); c.addPass(new DotScreenPass(new THREE.Vector2(0.5,0.5),1.2,1.4)); c.render();`,
        threers: `const c=new THREE.EffectComposer(r); c.addPass(new THREE.RenderPass(scene,cam)); c.addPass(new THREE.DotScreenPass(new THREE.Vector2(0.5,0.5),1.2,1.4)); await c.render();`,
    },
];

const benchDir = path.join(__dirname, 'scenes', 'dotscreen-bench');
fs.mkdirSync(benchDir, { recursive: true });
for (const b of BENCHES) {
    fs.writeFileSync(path.join(benchDir, `three-${b.id}.html`), shellThree(b.three));
    fs.writeFileSync(path.join(benchDir, `threers-${b.id}.html`), shellThreers(b.threers));
}

const server = spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'pipe' });
await new Promise(r => setTimeout(r, 400));

const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium',
    headless: 'new',
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-vulkan=swiftshader',
        '--no-sandbox', '--disable-dev-shm-usage', '--ignore-gpu-blocklist', '--enable-webgl'],
});

for (const b of BENCHES) {
    const base = `http://localhost:${port}/tests/parity/scenes/dotscreen-bench`;
    const refPage = await browser.newPage();
    await refPage.setViewport({ width: 800, height: 600 });
    await refPage.goto(`${base}/three-${b.id}.html`, { waitUntil: 'load' });
    await refPage.waitForFunction(() => document.body.dataset.ready === 'true');
    const ref = PNG.sync.read(Buffer.from(await refPage.screenshot({ type: 'png' })));
    await refPage.close();
    const cmpPage = await browser.newPage();
    await cmpPage.setViewport({ width: 800, height: 600 });
    await cmpPage.goto(`${base}/threers-${b.id}.html`, { waitUntil: 'load' });
    await cmpPage.waitForFunction(() => document.body.dataset.ready === 'true');
    const cmp = PNG.sync.read(Buffer.from(await cmpPage.screenshot({ type: 'png' })));
    await cmpPage.close();
    const diff = new PNG({ width: 800, height: 600 });
    const n = pixelmatch(ref.data, cmp.data, diff.data, 800, 600, { threshold: 0.2 });
    const pct = n / 480000 * 100;
    fs.writeFileSync(path.join(outDir, `${b.id}-diff.png`), PNG.sync.write(diff));
    console.log(`${b.id.padEnd(22)} ${pct.toFixed(2).padStart(6)}%  ${String(n).padStart(6)}  ${b.label}`);
}

await browser.close();
server.kill();
