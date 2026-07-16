#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import puppeteer from 'puppeteer';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8114;
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

const r = await page.evaluate(async (s52, clip) => {
    const { ExtendedTriangle } = await import('/web/mesh-bvh-impl.js');
    const THREE = (await import('/web/threejs-shim.js')).default;
    const tri = new ExtendedTriangle(
        new THREE.Vector3(s52[0], s52[1], s52[2]),
        new THREE.Vector3(s52[3], s52[4], s52[5]),
        new THREE.Vector3(s52[6], s52[7], s52[8]),
    );
    const clipT = new ExtendedTriangle(
        new THREE.Vector3(clip[0], clip[1], clip[2]),
        new THREE.Vector3(clip[3], clip[4], clip[5]),
        new THREE.Vector3(clip[6], clip[7], clip[8]),
    );
    const edge = new THREE.Line3();
    return {
        clipHitsTri: clipT.intersectsTriangle(tri, edge, true),
        triHitsClip: tri.intersectsTriangle(clipT, edge, true),
    };
}, snap8[52], clip);

console.log(JSON.stringify(r, null, 2));
await browser.close();
