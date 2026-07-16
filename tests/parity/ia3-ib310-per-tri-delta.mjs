#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import puppeteer from 'puppeteer';
import { spawn } from 'child_process';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8112;
spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 400));

const snap8 = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'scenes/rust/ia3-snap-j8.json'), 'utf8'),
).snap;
const clip = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'scenes/rust/ia3-snap-j9.json'), 'utf8'),
).clip;

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

const deltas = await page.evaluate(async (snap8, clip) => {
    const { TriangleSplitter } = await import('/web/csg/core/TriangleSplitter.js');
    const THREE = (await import('/web/threejs-shim.js')).default;
    const clipTri = new THREE.Triangle(
        new THREE.Vector3(clip[0], clip[1], clip[2]),
        new THREE.Vector3(clip[3], clip[4], clip[5]),
        new THREE.Vector3(clip[6], clip[7], clip[8]),
    );
    const out = [];
    for (let i = 0; i < snap8.length; i++) {
        const s = snap8[i];
        const tri = new THREE.Triangle(
            new THREE.Vector3(s[0], s[1], s[2]),
            new THREE.Vector3(s[3], s[4], s[5]),
            new THREE.Vector3(s[6], s[7], s[8]),
        );
        const splitter = new TriangleSplitter();
        splitter.initialize(tri);
        splitter.splitByTriangle(clipTri);
        const d = splitter.triangles.length - 1;
        if (d !== 0) out.push({ i, d, count: splitter.triangles.length });
    }
    return out;
}, snap8, clip);

console.log('js deltas', deltas);
console.log('total delta', deltas.reduce((a, x) => a + x.d, 0));
await browser.close();
