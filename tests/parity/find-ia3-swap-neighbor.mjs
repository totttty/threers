#!/usr/bin/env node
/** Find first neighbor where ia=3 clipped set diverges from JS (canonical tri keys). */
import puppeteer from 'puppeteer';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8108;
spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 400));

function triKey(t) {
    const verts = [];
    for (let i = 0; i < 3; i++) {
        verts.push([
            Math.round(t[i * 3] * 1e6),
            Math.round(t[i * 3 + 1] * 1e6),
            Math.round(t[i * 3 + 2] * 1e6),
        ]);
    }
    verts.sort((a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2]);
    return verts.flat().join(',');
}

const REF = '-173205,-507380,-100000,-139921,-518412,-100000,-101161,-531259,-100000';
const NAT = '-536835,0,-100000,-531259,-42350,-100000,-526229,-80558,-100000';

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

const r = await page.evaluate(async (REF, NAT) => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { installBvhCsg, Brush, Evaluator } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    const { collectIntersectingTriangles } = await import('/web/csg/core/operations/operationsUtils.js');
    const { TriangleSplitter } = await import('/web/csg/core/TriangleSplitter.js');
    installBvhCsg(THREE);
    const g = (n, ...a) => geometryToBufferGeometry(new THREE[n](...a));
    const mat = new THREE.MeshStandardMaterial();
    let a = new Brush(g('BoxGeometry', 4, 2.5, 2.5), mat);
    let b = new Brush(g('BoxGeometry', 3.6, 2.1, 2.1), mat);
    a = new Brush(new Evaluator().evaluate(a, b, 1).geometry, mat);
    b = new Brush(g('SphereGeometry', 0.55, 24, 12), mat);
    b.position.set(-1.1, 0.2, 1.35);
    b.updateMatrixWorld(true);
    a.prepareGeometry();
    b.prepareGeometry();
    const { aIntersections } = collectIntersectingTriangles(a, b);
    const splitter = new TriangleSplitter();
    const _matrix = new THREE.Matrix4().copy(b.matrixWorld).invert().multiply(a.matrixWorld);
    const ia = 3;
    const ia3 = 3 * ia;
    const idx = a.geometry.index;
    const pos = a.geometry.attributes.position;
    const triA = new THREE.Triangle();
    triA.a.fromBufferAttribute(pos, idx.getX(ia3)).applyMatrix4(_matrix);
    triA.b.fromBufferAttribute(pos, idx.getX(ia3 + 1)).applyMatrix4(_matrix);
    triA.c.fromBufferAttribute(pos, idx.getX(ia3 + 2)).applyMatrix4(_matrix);
    splitter.reset();
    splitter.initialize(triA);
    const neighbors = aIntersections.intersectionSet[ia];
    const out = [];
    function triKey(t) {
        const verts = [
            [t.a.x, t.a.y, t.a.z],
            [t.b.x, t.b.y, t.b.z],
            [t.c.x, t.c.y, t.c.z],
        ];
        verts.sort((a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2]);
        return verts.flat().map((v) => Math.round(v * 1e6)).join(',');
    }
    for (let j = 0; j < neighbors.length; j++) {
        const ib = neighbors[j];
        const ib3 = 3 * ib;
        const bp = b.geometry.attributes.position;
        const bi = b.geometry.index;
        const triB = new THREE.Triangle();
        triB.a.fromBufferAttribute(bp, bi.getX(ib3));
        triB.b.fromBufferAttribute(bp, bi.getX(ib3 + 1));
        triB.c.fromBufferAttribute(bp, bi.getX(ib3 + 2));
        splitter.splitByTriangle(triB);
        let hasRef = false;
        let hasNat = false;
        for (const t of splitter.triangles) {
            const k = triKey(t);
            if (k === REF) hasRef = true;
            if (k === NAT) hasNat = true;
        }
        if (hasRef || hasNat) {
            out.push({ j, ib, count: splitter.triangles.length, hasRef, hasNat });
        }
    }
    return out;
}, REF, NAT);

console.log(JSON.stringify(r, null, 2));
await browser.close();
