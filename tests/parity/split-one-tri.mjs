#!/usr/bin/env node
/** Split one triangle by clip; args: path-to-json with {tri, clip} */
import fs from 'fs';
import puppeteer from 'puppeteer';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const input = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const port = 8112;
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

const result = await page.evaluate(async (input) => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { TriangleSplitter } = await import('/web/csg/core/TriangleSplitter.js');
    const { isTriDegenerate } = await import('/web/csg/core/utils/triangleUtils.js');

    function hashCoord(v) {
        return (v * 1e6 + 0.5) | 0;
    }
    function ordKey(t) {
        return [
            hashCoord(t.a.x), hashCoord(t.a.y), hashCoord(t.a.z),
            hashCoord(t.b.x), hashCoord(t.b.y), hashCoord(t.b.z),
            hashCoord(t.c.x), hashCoord(t.c.y), hashCoord(t.c.z),
        ].join(',');
    }

    const tri = new THREE.Triangle();
    const s = input.tri;
    tri.a.set(s[0], s[1], s[2]);
    tri.b.set(s[3], s[4], s[5]);
    tri.c.set(s[6], s[7], s[8]);
    const clip = new THREE.Triangle();
    const c = input.clip;
    clip.a.set(c[0], c[1], c[2]);
    clip.b.set(c[3], c[4], c[5]);
    clip.c.set(c[6], c[7], c[8]);

    const splitter = new TriangleSplitter();
    splitter.initialize(tri);
    splitter.splitByTriangle(clip);
    return {
        count: splitter.triangles.length,
        keys: splitter.triangles.map((t) => ordKey(t)),
        degen: splitter.triangles.map((t) => isTriDegenerate(t)),
    };
}, input);

console.log(JSON.stringify(result, null, 2));
await browser.close();
process.exit(0);
