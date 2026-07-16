#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import puppeteer from 'puppeteer';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8115;
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
    const THREE = (await import('/web/threejs-shim.js')).default;
    const tri = new THREE.Triangle(
        new THREE.Vector3(s52[0], s52[1], s52[2]),
        new THREE.Vector3(s52[3], s52[4], s52[5]),
        new THREE.Vector3(s52[6], s52[7], s52[8]),
    );
    const clipTri = new THREE.Triangle(
        new THREE.Vector3(clip[0], clip[1], clip[2]),
        new THREE.Vector3(clip[3], clip[4], clip[5]),
        new THREE.Vector3(clip[6], clip[7], clip[8]),
    );
    const plane = clipTri.getPlane(new THREE.Plane());
    const n = plane.normal;
    const arr = [tri.a, tri.b, tri.c];
    const EPS = 1e-10;
    const edges = [];
    let intersects = 0;
    const _vec = new THREE.Vector3();
    const _edge = new THREE.Line3();
    for (let t = 0; t < 3; t++) {
        const tNext = (t + 1) % 3;
        _edge.start.copy(arr[t]);
        _edge.end.copy(arr[tNext]);
        const sd = plane.distanceToPoint(_edge.start);
        const ed = plane.distanceToPoint(_edge.end);
        if (Math.abs(sd) < EPS && Math.abs(ed) < EPS) {
            edges.push({ t, coplanarEdge: true });
            break;
        }
        if (Math.abs(sd) < EPS) {
            edges.push({ t, skip: true, sd, ed });
            continue;
        }
        const tParam = sd / (sd - ed);
        let did = !!plane.intersectLine(_edge, _vec);
        if (!did && Math.abs(ed) < EPS) {
            _vec.copy(_edge.end);
            did = true;
        }
        const useHit = did && !(_vec.distanceTo(_edge.start) < EPS);
        if (useHit) intersects++;
        edges.push({ t, sd, ed, tParam, did, useHit, hit: useHit ? [_vec.x, _vec.y, _vec.z] : null });
    }
    return {
        plane: { n: [n.x, n.y, n.z], constant: plane.constant },
        edges,
        intersects,
    };
}, snap8[52], clip);

console.log(JSON.stringify(r, null, 2));
await browser.close();
