#!/usr/bin/env node
/**
 * FXAA parity micro-benchmarks — isolate each pipeline stage.
 *
 * Usage: node bench-fxaa.js
 *
 * Each row compares three.js vs threers for one slice of the FXAA pipeline.
 * Goal: find which stage owns the ~2% residual diff so fixes can be targeted.
 */

import puppeteer from 'puppeteer';
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..', '..');
const outDir = path.join(__dirname, 'out', 'fxaa-bench');
fs.mkdirSync(outDir, { recursive: true });

const port = 8120;
const SCENE = `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x111111);
const cube = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, 0.8), new THREE.MeshBasicMaterial({ color: 0xffffff }));
cube.rotation.set(0.4, 0.6, 0); scene.add(cube);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
`;

const THREE_POSTFX = `
import * as THREE from 'three';
import { EffectComposer } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/RenderPass.js';
import { ShaderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/ShaderPass.js';
import { CopyShader } from 'https://unpkg.com/three@0.165.0/examples/jsm/shaders/CopyShader.js';
import { FXAAShader } from 'https://unpkg.com/three@0.165.0/examples/jsm/shaders/FXAAShader.js';
`;

function shellThree(body) {
    return `<!DOCTYPE html><html><head><meta charset="UTF-8"><style>body{margin:0;background:#101418}canvas{display:block}</style></head>
<body><canvas id="c" width="800" height="600"></canvas>
<script type="importmap">{"imports":{"three":"https://unpkg.com/three@0.165.0/build/three.module.js"}}</script>
<script type="module">${THREE_POSTFX}
${SCENE}
const r = new THREE.WebGLRenderer({ canvas: document.getElementById('c'), antialias: false });
r.setSize(800, 600, false); r.setPixelRatio(1);
${body}
document.body.dataset.ready = 'true';
</script></body></html>`;
}

function shellThreers(body) {
    return `<!DOCTYPE html><html><head><meta charset="UTF-8"><style>body{margin:0;background:#101418}canvas{display:block}#err{position:absolute;top:0;left:0;padding:8px;color:#fdd;background:#500;font:12px monospace;white-space:pre-wrap}</style></head>
<body><canvas id="c" width="800" height="600"></canvas><div id="err" style="display:none"></div>
<script type="module">
import THREE, { initThreers } from '/web/threejs-shim.js';
(async () => {
  try {
    await initThreers('/web/pkg/threers_bg.wasm');
    ${SCENE}
    const r = await THREE.WebGLRenderer.create(document.getElementById('c'));
    r.setSize(800, 600);
    ${body}
    await new Promise(rs => requestAnimationFrame(() => rs()));
    document.body.dataset.ready = 'true';
  } catch (e) {
    document.getElementById('err').style.display = 'block';
    document.getElementById('err').textContent = e?.stack || String(e);
    document.body.dataset.ready = 'error';
  }
})();
</script></body></html>`;
}

/** @type {Array<{id:string, label:string, three:string, threers:string}>} */
const BENCHES = [
    {
        id: 'A-direct',
        label: 'A. Direct render (no composer)',
        three: 'r.render(scene, cam);',
        threers: 'r.render(scene, cam);',
    },
    {
        id: 'B-rt-copy',
        label: 'B. Composer RenderPass → copy to canvas',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(CopyShader));
composer.render();`,
        threers: `
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass({ enabled: true, needsSwap: false, renderToScreen: true,
  render(renderer, wb, rb) { renderer.applyPostFx(rb, 0, 0, [1,0,0,0]); } });
await composer.render();`,
    },
    {
        id: 'C-fxaa-full',
        label: 'C. Full FXAA composer (production path)',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(FXAAShader));
composer.render();`,
        threers: `
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.ShaderPass(THREE.FXAAShader));
await composer.render();`,
    },
    {
        id: 'D-fxaa-wgsl',
        label: 'D. FXAA composer — threers WGSL only (WebGL fallback off)',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(FXAAShader));
composer.render();`,
        threers: `
window.__FXAA_FORCE = 'wgsl';
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.ShaderPass(THREE.FXAAShader));
await composer.render();`,
    },
    {
        id: 'E-fxaa-webgl',
        label: 'E. FXAA composer — threers WebGL f16 bake only',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(FXAAShader));
composer.render();`,
        threers: `
window.__FXAA_FORCE = 'webgl';
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.ShaderPass(THREE.FXAAShader));
await composer.render();`,
    },
    {
        id: 'F-three-fxaa-vs-copy',
        label: 'F. three.js FXAA vs copy (how much FXAA moves pixels on reference)',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(FXAAShader));
composer.render();`,
        threers: `
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass({ enabled: true, needsSwap: false, renderToScreen: true,
  render(renderer, wb, rb) { renderer.applyPostFx(rb, 0, 0, [1,0,0,0]); } });
await composer.render();`,
    },
    {
        id: 'G-readback-blit',
        label: 'G. f16 readback → RGBA8 blit vs wgpu copy (readback path alone)',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(CopyShader));
composer.render();`,
        threers: `
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass({ enabled: true, needsSwap: false, renderToScreen: true,
  async render(renderer, wb, rb) {
    const f16 = await renderer.readRenderTargetF16(rb, 0, 0, 800, 600);
    const w=800,h=600, row=w*8;
    const flip=(b,rb,h)=>{const o=new Uint8Array(b.length);for(let y=0;y<h;y++){o.set(b.subarray(y*rb,y*rb+rb),(h-1-y)*rb);}return o;};
    const f=(h)=>{const s=(h&0x8000)>>15,e=(h&0x7C00)>>10,fv=h&0x3FF;if(e===0)return(s?-1:1)*2**-14*(fv/1024);if(e===0x1F)return fv?NaN:(s?-Infinity:Infinity);return(s?-1:1)*2**(e-15)*(1+fv/1024);};
    const u16=new Uint16Array(flip(new Uint8Array(f16),row,h).buffer);
    const rgba=new Uint8Array(w*h*4);
    for(let i=0;i<w*h;i++){rgba[i*4]=Math.round(f(u16[i*4])*255);rgba[i*4+1]=Math.round(f(u16[i*4+1])*255);rgba[i*4+2]=Math.round(f(u16[i*4+2])*255);rgba[i*4+3]=Math.round(f(u16[i*4+3])*255);}
    renderer._w.blitRgba8ToCanvas(rgba,w,h);
  } });
await composer.render();`,
    },
    {
        id: 'H-f16-gl-copy',
        label: 'H. f16 → RGBA16F GL texture copy vs wgpu copy (GPU sample path)',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(CopyShader));
composer.render();`,
        threers: `
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass({ enabled: true, needsSwap: false, renderToScreen: true,
  async render(renderer, wb, rb) {
    const w=800,h=600;
    const f16 = await renderer.readRenderTargetF16(rb, 0, 0, w, h);
    const row=w*8;
    const flip=(b,rb,h)=>{const o=new Uint8Array(b.length);for(let y=0;y<h;y++){o.set(b.subarray(y*rb,y*rb+rb),(h-1-y)*rb);}return o;};
    const input=flip(new Uint8Array(f16),row,h);
    const c=document.createElement('canvas'); c.width=w; c.height=h;
    const gl=c.getContext('webgl2',{antialias:false,preserveDrawingBuffer:true});
    const vs='#version 300 es\\nprecision highp float;\\nconst vec2 uvs[3]=vec2[3](vec2(0,2),vec2(0,0),vec2(2,0));\\nconst vec2 pos[3]=vec2[3](vec2(-1,3),vec2(-1,-1),vec2(3,-1));\\nout vec2 vUv;\\nvoid main(){vUv=uvs[gl_VertexID];gl_Position=vec4(pos[gl_VertexID],0,1);}';
    const fs='#version 300 es\\nprecision highp float;\\nuniform sampler2D t;\\nin vec2 vUv;\\nout vec4 o;\\nvoid main(){o=texture(t,vUv);}';
    const sh=(t,s)=>{const o=gl.createShader(t);gl.shaderSource(o,s);gl.compileShader(o);return o;};
    const p=gl.createProgram(); gl.attachShader(p,sh(gl.VERTEX_SHADER,vs)); gl.attachShader(p,sh(gl.FRAGMENT_SHADER,fs)); gl.linkProgram(p);
    const tex=gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D,tex);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA16F,w,h,0,gl.RGBA,gl.HALF_FLOAT,new Uint16Array(input.buffer,input.byteOffset,input.byteLength/2));
    const out=gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D,out);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,w,h,0,gl.RGBA,gl.UNSIGNED_BYTE,null);
    const fb=gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER,fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER,gl.COLOR_ATTACHMENT0,gl.TEXTURE_2D,out,0);
    gl.useProgram(p); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D,tex);
    gl.uniform1i(gl.getUniformLocation(p,'t'),0); gl.viewport(0,0,w,h); gl.drawArrays(gl.TRIANGLES,0,3);
    const rgba=new Uint8Array(w*h*4); gl.readPixels(0,0,w,h,gl.RGBA,gl.UNSIGNED_BYTE,rgba);
    renderer._w.blitRgba8ToCanvas(flip(rgba,w*4,h),w,h);
  } });
await composer.render();`,
    },
    {
        id: 'I-f16-gl-nearest',
        label: 'I. f16 → RGBA16F GL copy (NEAREST) vs wgpu copy',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(CopyShader));
composer.render();`,
        threers: `
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass({ enabled: true, needsSwap: false, renderToScreen: true,
  async render(renderer, wb, rb) {
    const w=800,h=600;
    const f16 = await renderer.readRenderTargetF16(rb, 0, 0, w, h);
    const row=w*8;
    const flip=(b,rb,h)=>{const o=new Uint8Array(b.length);for(let y=0;y<h;y++){o.set(b.subarray(y*rb,y*rb+rb),(h-1-y)*rb);}return o;};
    const input=flip(new Uint8Array(f16),row,h);
    const c=document.createElement('canvas'); c.width=w; c.height=h;
    const gl=c.getContext('webgl2',{antialias:false,preserveDrawingBuffer:true});
    const vs='#version 300 es\\nprecision highp float;\\nconst vec2 uvs[3]=vec2[3](vec2(0,2),vec2(0,0),vec2(2,0));\\nconst vec2 pos[3]=vec2[3](vec2(-1,3),vec2(-1,-1),vec2(3,-1));\\nout vec2 vUv;\\nvoid main(){vUv=uvs[gl_VertexID];gl_Position=vec4(pos[gl_VertexID],0,1);}';
    const fs='#version 300 es\\nprecision highp float;\\nuniform sampler2D t;\\nin vec2 vUv;\\nout vec4 o;\\nvoid main(){o=texture(t,vUv);}';
    const sh=(t,s)=>{const o=gl.createShader(t);gl.shaderSource(o,s);gl.compileShader(o);return o;};
    const p=gl.createProgram(); gl.attachShader(p,sh(gl.VERTEX_SHADER,vs)); gl.attachShader(p,sh(gl.FRAGMENT_SHADER,fs)); gl.linkProgram(p);
    const tex=gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D,tex);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA16F,w,h,0,gl.RGBA,gl.HALF_FLOAT,new Uint16Array(input.buffer,input.byteOffset,input.byteLength/2));
    const out=gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D,out);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,w,h,0,gl.RGBA,gl.UNSIGNED_BYTE,null);
    const fb=gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER,fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER,gl.COLOR_ATTACHMENT0,gl.TEXTURE_2D,out,0);
    gl.useProgram(p); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D,tex);
    gl.uniform1i(gl.getUniformLocation(p,'t'),0); gl.viewport(0,0,w,h); gl.drawArrays(gl.TRIANGLES,0,3);
    const rgba=new Uint8Array(w*h*4); gl.readPixels(0,0,w,h,gl.RGBA,gl.UNSIGNED_BYTE,rgba);
    renderer._w.blitRgba8ToCanvas(flip(rgba,w*4,h),w,h);
  } });
await composer.render();`,
    },
    {
        id: 'J-fxaa-rgba8-input',
        label: 'J. FXAA on CPU-decoded RGBA8 input (fix hypothesis)',
        three: `
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(FXAAShader));
composer.render();`,
        threers: `
globalThis.__FXAA_RGBA8_INPUT = true;
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.ShaderPass(THREE.FXAAShader));
await composer.render();`,
    },
];

const benchDir = path.join(__dirname, 'scenes', 'fxaa-bench');
fs.mkdirSync(benchDir, { recursive: true });
for (const b of BENCHES) {
    fs.writeFileSync(path.join(benchDir, `three-${b.id}.html`), shellThree(b.three));
    fs.writeFileSync(path.join(benchDir, `threers-${b.id}.html`), shellThreers(b.threers));
}

// Patch threejs-shim for __FXAA_FORCE before server starts — inject via evaluate instead.
const server = spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'pipe' });
await new Promise(r => setTimeout(r, 400));

const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium',
    headless: 'new',
    args: [
        '--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-vulkan=swiftshader',
        '--no-sandbox', '--disable-dev-shm-usage', '--ignore-gpu-blocklist', '--enable-webgl',
    ],
});

function analyzeDiff(ref, cmp) {
    let hard = 0, soft = 0, maxd = 0;
    const hardAtEdge = { bg: 0, fg: 0 };
    for (let i = 0; i < ref.data.length; i += 4) {
        const dr = Math.abs(ref.data[i] - cmp.data[i]);
        const dg = Math.abs(ref.data[i + 1] - cmp.data[i + 1]);
        const db = Math.abs(ref.data[i + 2] - cmp.data[i + 2]);
        const d = Math.max(dr, dg, db);
        if (d === 0) continue;
        if (d >= 200) hard++; else soft++;
        if (d > maxd) maxd = d;
        if (d >= 200) {
            const lum = ref.data[i] + ref.data[i + 1] + ref.data[i + 2];
            if (lum < 80 || lum > 640) hardAtEdge.bg++;
            else hardAtEdge.fg++;
        }
    }
    return { hard, soft, maxd, hardAtEdge };
}

async function shoot(url, force) {
    const page = await browser.newPage();
    await page.setViewport({ width: 800, height: 600 });
    if (force) {
        await page.goto(`http://localhost:${port}/tests/parity/scenes/fxaa-bench/threers-C-fxaa-full.html`);
        await page.waitForFunction(() => typeof window.__patchFxaa === 'function' || document.body.dataset.ready, { timeout: 5000 }).catch(() => {});
    }
    await page.goto(url, { waitUntil: 'load', timeout: 30000 });
    if (force) {
        await page.evaluate((mode) => {
            window.__FXAA_FORCE = mode;
        }, force);
    }
    await page.waitForFunction(() => document.body.dataset.ready === 'true', { timeout: 30000 });
    const png = await page.screenshot({ type: 'png' });
    await page.close();
    return PNG.sync.read(Buffer.from(png));
}

console.log('FXAA micro-benchmarks (three.js vs threers)\n');
console.log('ID'.padEnd(14) + 'Diff%'.padStart(8) + '  Pixels'.padStart(12) + '  hard/soft'.padStart(14) + '  Description');
console.log('─'.repeat(90));

const rows = [];
for (const b of BENCHES) {
    const base = `http://localhost:${port}/tests/parity/scenes/fxaa-bench`;
    let force = null;
    if (b.id === 'D-fxaa-wgsl') force = 'wgsl';
    if (b.id === 'E-fxaa-webgl') force = 'webgl';

    const refUrl = `${base}/three-${b.id}.html`;
    const cmpUrl = `${base}/threers-${b.id}.html`;

    let ref, cmp;
    if (force) {
        const page = await browser.newPage();
        await page.setViewport({ width: 800, height: 600 });
        await page.goto(refUrl, { waitUntil: 'load' });
        await page.waitForFunction(() => document.body.dataset.ready === 'true');
        ref = PNG.sync.read(Buffer.from(await page.screenshot({ type: 'png' })));
        await page.goto(cmpUrl, { waitUntil: 'load' });
        await page.evaluate((mode) => { window.__FXAA_FORCE = mode; }, force);
        await page.waitForFunction(() => document.body.dataset.ready === 'true');
        cmp = PNG.sync.read(Buffer.from(await page.screenshot({ type: 'png' })));
        await page.close();
    } else {
        const refPage = await browser.newPage();
        await refPage.setViewport({ width: 800, height: 600 });
        await refPage.goto(refUrl, { waitUntil: 'load' });
        await refPage.waitForFunction(() => document.body.dataset.ready === 'true');
        ref = PNG.sync.read(Buffer.from(await refPage.screenshot({ type: 'png' })));
        await refPage.close();
        const cmpPage = await browser.newPage();
        await cmpPage.setViewport({ width: 800, height: 600 });
        await cmpPage.goto(cmpUrl, { waitUntil: 'load' });
        await cmpPage.waitForFunction(() => document.body.dataset.ready === 'true');
        cmp = PNG.sync.read(Buffer.from(await cmpPage.screenshot({ type: 'png' })));
        await cmpPage.close();
    }

    const diff = new PNG({ width: 800, height: 600 });
    const n = pixelmatch(ref.data, cmp.data, diff.data, 800, 600, { threshold: 0.2 });
    const pct = n / 480000 * 100;
    const stats = analyzeDiff(ref, cmp);
    fs.writeFileSync(path.join(outDir, `${b.id}-diff.png`), PNG.sync.write(diff));
    rows.push({ ...b, pct, n, stats });
    console.log(
        b.id.padEnd(14) +
        pct.toFixed(2).padStart(7) + '%' +
        String(n).padStart(12) +
        `${stats.hard}/${stats.soft}`.padStart(14) +
        '  ' + b.label
    );
}

await browser.close();
server.kill();

console.log('\n─── Decomposition ───\n');
const a = rows.find(r => r.id === 'A-direct');
const b = rows.find(r => r.id === 'B-rt-copy');
const c = rows.find(r => r.id === 'C-fxaa-full');
const d = rows.find(r => r.id === 'D-fxaa-wgsl');
const e = rows.find(r => r.id === 'E-fxaa-webgl');
const f = rows.find(r => r.id === 'F-three-fxaa-vs-copy');
const g = rows.find(r => r.id === 'G-readback-blit');
const h = rows.find(r => r.id === 'H-f16-gl-copy');
const i = rows.find(r => r.id === 'I-f16-gl-nearest');
const j = rows.find(r => r.id === 'J-fxaa-rgba8-input');

console.log(`Stage 1 — Base renderer:           ${a?.pct.toFixed(2)}%  (must be 0%)`);
console.log(`Stage 2 — Half-float RT + copy:    ${b?.pct.toFixed(2)}%  (must be 0%)`);
console.log(`Stage 3 — FXAA effect magnitude:   ${f?.pct.toFixed(2)}%  (three.js FXAA vs threers copy)`);
console.log(`Stage 4 — End-to-end production:   ${c?.pct.toFixed(2)}%`);
console.log(`Stage 5a — WGSL FXAA only:         ${d?.pct.toFixed(2)}%`);
console.log(`Stage 5b — WebGL f16 FXAA only:    ${e?.pct.toFixed(2)}%`);
console.log(`Stage 6 — f16 CPU decode blit:     ${g?.pct.toFixed(2)}%  (bytes OK)`);
console.log(`Stage 7 — f16 GL half-float copy:  ${h?.pct.toFixed(2)}%  (ROOT: cross-API rehydrate)`);
console.log(`Stage 8 — f16 GL NEAREST copy:     ${i?.pct.toFixed(2)}%`);
console.log(`Stage 9 — FXAA on RGBA8 input:     ${j?.pct.toFixed(2)}%  (target fix)`);
console.log(`\nDiff images → ${outDir}`);

const cStats = c?.stats;
if (cStats) {
    console.log(`\nProduction diff pixel profile: ${cStats.hard} hard (≥200), ${cStats.soft} soft, max Δ=${cStats.maxd}`);
    console.log(`Hard flips: ~${cStats.hardAtEdge.fg} on foreground edges, ~${cStats.hardAtEdge.bg} on background edges`);
}
