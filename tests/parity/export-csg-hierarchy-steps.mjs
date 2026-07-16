#!/usr/bin/env node
/** Export s1–s4 hierarchy CSG step geometry bins (JS `web/csg/` reference). */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(__dirname, 'scenes/rust');
const port = 8096;

spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 400));

const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium',
    headless: 'new',
    args: ['--no-sandbox'],
});

const page = await browser.newPage();
await page.goto(`http://127.0.0.1:${port}/tests/parity/scenes/threers-bvh-csg-hierarchy.html`, {
    waitUntil: 'networkidle0',
    timeout: 120000,
});

const steps = await page.evaluate(async () => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { installBvhCsg, Brush, Evaluator, ADDITION, SUBTRACTION } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    installBvhCsg(THREE);
    const g = (name, ...args) => geometryToBufferGeometry(new THREE[name](...args));
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc });
    const evaluator = new Evaluator();
    evaluator.useGroups = false;
    const posOf = (geom) => Array.from(geom.attributes.position.array);

    let a = new Brush(g('BoxGeometry', 4, 2.5, 2.5), mat);
    let b = new Brush(g('BoxGeometry', 3.6, 2.1, 2.1), mat);
    let r = evaluator.evaluate(a, b, SUBTRACTION);
    const s1 = { verts: r.geometry.attributes.position.count, pos: posOf(r.geometry) };

    a = new Brush(r.geometry, mat);
    b = new Brush(g('SphereGeometry', 0.55, 24, 12), mat);
    b.position.set(-1.1, 0.2, 1.35);
    b.updateMatrixWorld(true);
    r = evaluator.evaluate(a, b, ADDITION);
    const s2 = { verts: r.geometry.attributes.position.count, pos: posOf(r.geometry) };

    a = new Brush(r.geometry, mat);
    b = new Brush(g('BoxGeometry', 1.2, 1.0, 0.5), mat);
    b.position.set(0.8, 0.15, 1.35);
    b.updateMatrixWorld(true);
    r = evaluator.evaluate(a, b, SUBTRACTION);
    const s3 = { verts: r.geometry.attributes.position.count, pos: posOf(r.geometry) };

    a = new Brush(r.geometry, mat);
    b = new Brush(g('BoxGeometry', 1.2, 1.0, 0.12), mat);
    b.position.set(0.8, 0.15, 1.35);
    b.updateMatrixWorld(true);
    r = evaluator.evaluate(a, b, ADDITION);
    const s4 = { verts: r.geometry.attributes.position.count, pos: posOf(r.geometry) };

    return { s1, s2, s3, s4 };
});

await browser.close();

function writeBin(name, { verts, pos }) {
    const f32 = new Float32Array(pos);
    const buf = Buffer.alloc(8 + f32.byteLength);
    buf.write('TCG1', 0, 'ascii');
    buf.writeUInt32LE(verts, 4);
    Buffer.from(f32.buffer).copy(buf, 8);
    const out = path.join(outDir, name);
    fs.mkdirSync(outDir, { recursive: true });
    fs.writeFileSync(out, buf);
    console.log(`wrote ${out} — ${verts} verts`);
}

writeBin('bvh-csg-step1-shell.geom.bin', steps.s1);
writeBin('bvh-csg-after-sphere.geom.bin', steps.s2);
writeBin('bvh-csg-step3-wincut.geom.bin', steps.s3);
writeBin('bvh-csg-hierarchy.geom.bin', steps.s4);

console.log('JS steps:', {
    s1: steps.s1.verts,
    s2: steps.s2.verts,
    s3: steps.s3.verts,
    s4: steps.s4.verts,
});
