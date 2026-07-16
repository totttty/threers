#!/usr/bin/env node
/** Export non-indexed sphere soup for parity (0.55, 24, 12). */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(__dirname, 'scenes/rust');
const port = 8097;

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

const pos = await page.evaluate(async () => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    const g = geometryToBufferGeometry(new THREE.SphereGeometry(0.55, 24, 12));
    return Array.from(g.attributes.position.array);
});

await browser.close();

const f32 = new Float32Array(pos);
const verts = f32.length / 3;
const buf = Buffer.alloc(8 + f32.byteLength);
buf.write('TCG1', 0, 'ascii');
buf.writeUInt32LE(verts, 4);
Buffer.from(f32.buffer).copy(buf, 8);
const out = path.join(outDir, 'sphere-0.55-24-12.geom.bin');
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(out, buf);
console.log(`wrote ${out} — ${verts} verts, ${verts / 3} tris`);
