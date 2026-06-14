#!/usr/bin/env node
/** Glitch parity isolation. */
import puppeteer from 'puppeteer';
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8126;

const SCENE = `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101010);
scene.add(new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial({ color: 0x00ff88 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
`;

const THREE_IMPORTS = `
import * as THREE from 'three';
import { EffectComposer } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/RenderPass.js';
import { GlitchPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/GlitchPass.js';
`;

function shellThree(body) {
    return `<!DOCTYPE html><html><body><canvas id="c" width="800" height="600"></canvas>
<script type="importmap">{"imports":{"three":"https://unpkg.com/three@0.165.0/build/three.module.js"}}</script>
<script type="module">${THREE_IMPORTS}
const r = new THREE.WebGLRenderer({ canvas: document.getElementById('c'), antialias: false });
r.setSize(800, 600, false); r.setPixelRatio(1);
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
  ${body.split('// __RENDER__')[0]}
  const r = await THREE.WebGLRenderer.create(document.getElementById('c'));
  r.setSize(800, 600);
  ${body.includes('// __RENDER__') ? body.split('// __RENDER__')[1] : ''}
  await new Promise(rs => requestAnimationFrame(rs));
  document.body.dataset.ready = 'true';
})();
</script></body></html>`;
}

const SEED = `(function(s){let st=s>>>0;Math.random=()=>{st=(Math.imul(st,1664525)+1013904223)>>>0;return st/4294967296;};})(42);`;
const SEED_T = `THREE.seedRandom(42);`;
const SCENE_WITH_SEED = `
${SCENE}
${SEED}
`;
const SCENE_WITH_SEED_T = `
${SCENE}
${SEED_T}
`;
const THREE_GLITCH = `
${SCENE_WITH_SEED}
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
const glitchPass = new GlitchPass();
glitchPass.material.fragmentShader = glitchPass.material.fragmentShader.replace(
  /\\/\\/add noise[\\s\\S]*?gl_FragColor = gl_FragColor\\+ snow;/,
  '// snow omitted',
);
glitchPass.material.needsUpdate = true;
composer.addPass(glitchPass);
composer.render();
window.__glitchDebug = { seed: glitchPass.uniforms.seed.value, byp: glitchPass.uniforms.byp.value, amount: glitchPass.uniforms.amount.value, angle: glitchPass.uniforms.angle.value, seed_x: glitchPass.uniforms.seed_x.value, seed_y: glitchPass.uniforms.seed_y.value, distortion_x: glitchPass.uniforms.distortion_x.value, distortion_y: glitchPass.uniforms.distortion_y.value, col_s: glitchPass.uniforms.col_s.value, randX: glitchPass.randX, curF: glitchPass.curF, h0: glitchPass.heightMap.image.data[0], h1: glitchPass.heightMap.image.data[1], h4095: glitchPass.heightMap.image.data[4095] };`;

const THREERS_GLITCH = `
${SCENE_WITH_SEED_T}
// __RENDER__
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
const glitchPass = new THREE.GlitchPass();
glitchPass._skipSnow = true;
composer.addPass(glitchPass);
await composer.render();
window.__glitchDebug = globalThis.__glitchDebug;`;

const BENCHES = [
    { id: 'A-rt-copy', three: `${SCENE_WITH_SEED}\nconst c=new EffectComposer(r);c.addPass(new RenderPass(scene,cam));c.addPass(new ShaderPass(CopyShader));c.render();`, threers: `${SCENE_WITH_SEED_T}\n// __RENDER__\nconst c=new THREE.EffectComposer(r);c.addPass(new THREE.RenderPass(scene,cam));c.addPass({enabled:true,needsSwap:true,renderToScreen:true,render(renderer,wb,rb){renderer.applyPostFx(rb,0,0,[1,0,0,0]);}});await c.render();` },
    { id: 'B-three-glitch-vs-copy', three: THREE_GLITCH, threers: `${SCENE_WITH_SEED_T}\n// __RENDER__\nconst c=new THREE.EffectComposer(r);c.addPass(new THREE.RenderPass(scene,cam));c.addPass({enabled:true,needsSwap:true,renderToScreen:true,render(renderer,wb,rb){renderer.applyPostFx(rb,0,0,[1,0,0,0]);}});await c.render();` },
    { id: 'C-full', three: THREE_GLITCH, threers: THREERS_GLITCH },
];

const benchDir = path.join(__dirname, 'scenes', 'glitch-bench');
fs.mkdirSync(benchDir, { recursive: true });
for (const b of BENCHES) {
    fs.writeFileSync(path.join(benchDir, `three-${b.id}.html`), shellThree(b.three));
    fs.writeFileSync(path.join(benchDir, `threers-${b.id}.html`), shellThreers(b.threers));
}

const server = spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'pipe' });
await new Promise(r => setTimeout(r, 400));
const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium', headless: 'new',
    args: ['--enable-unsafe-webgpu','--enable-features=Vulkan,WebGPU','--use-vulkan=swiftshader','--no-sandbox','--disable-dev-shm-usage','--ignore-gpu-blocklist','--enable-webgl'],
});

for (const b of BENCHES) {
    const base = `http://localhost:${port}/tests/parity/scenes/glitch-bench`;
    const refPage = await browser.newPage();
    await refPage.setViewport({ width: 800, height: 600 });
    await refPage.goto(`${base}/three-${b.id}.html`, { waitUntil: 'load', timeout: 60000 });
    await refPage.waitForFunction(() => document.body.dataset.ready === 'true', { timeout: 60000 });
    const ref = PNG.sync.read(Buffer.from(await refPage.screenshot({ type: 'png' })));
    await refPage.close();
    const cmpPage = await browser.newPage();
    await cmpPage.setViewport({ width: 800, height: 600 });
    await cmpPage.goto(`${base}/threers-${b.id}.html`, { waitUntil: 'load', timeout: 60000 });
    await cmpPage.waitForFunction(() => document.body.dataset.ready === 'true', { timeout: 60000 });
    const cmp = PNG.sync.read(Buffer.from(await cmpPage.screenshot({ type: 'png' })));
    await cmpPage.close();
    const diff = new PNG({ width: 800, height: 600 });
    const n = pixelmatch(ref.data, cmp.data, diff.data, 800, 600, { threshold: 0.2 });
    console.log(`${b.id.padEnd(24)} ${(n / 480000 * 100).toFixed(2).padStart(6)}%  ${b.id}`);
}

await browser.close();
server.kill();
