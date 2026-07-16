#!/usr/bin/env node
/** Export ia=3 clipped snap after neighbor j (0-based, after applying neighbors[0..j]). */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const jEnd = Number(process.argv[2] ?? 18);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8111;
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

const data = await page.evaluate(async (jEnd) => {
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
    for (let j = 0; j <= jEnd; j++) {
        const ib = neighbors[j];
        const ib3 = 3 * ib;
        const bp = b.geometry.attributes.position;
        const bi = b.geometry.index;
        const triB = new THREE.Triangle();
        triB.a.fromBufferAttribute(bp, bi.getX(ib3));
        triB.b.fromBufferAttribute(bp, bi.getX(ib3 + 1));
        triB.c.fromBufferAttribute(bp, bi.getX(ib3 + 2));
        splitter.splitByTriangle(triB);
    }
    const snap = splitter.triangles.map((t) => [
        t.a.x, t.a.y, t.a.z, t.b.x, t.b.y, t.b.z, t.c.x, t.c.y, t.c.z,
    ]);
    const ib = neighbors[jEnd];
    const ib3 = 3 * ib;
    const bp = b.geometry.attributes.position;
    const bi = b.geometry.index;
    const clip = [
        bp.getX(bi.getX(ib3)), bp.getY(bi.getX(ib3)), bp.getZ(bi.getX(ib3)),
        bp.getX(bi.getX(ib3 + 1)), bp.getY(bi.getX(ib3 + 1)), bp.getZ(bi.getX(ib3 + 1)),
        bp.getX(bi.getX(ib3 + 2)), bp.getY(bi.getX(ib3 + 2)), bp.getZ(bi.getX(ib3 + 2)),
    ];
    return {
        j: jEnd,
        ib,
        count: snap.length,
        coplanar: splitter.coplanarTriangleUsed,
        clip,
        snap,
    };
}, jEnd);

await browser.close();

const out = path.join(__dirname, 'scenes/rust', `ia3-snap-j${jEnd}.json`);
fs.writeFileSync(out, JSON.stringify(data));
console.log(`wrote ${out} count=${data.count} ib=${data.ib} coplanar=${data.coplanar}`);
