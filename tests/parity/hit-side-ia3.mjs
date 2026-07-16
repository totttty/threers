#!/usr/bin/env node
import puppeteer from 'puppeteer';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8107;
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

const r = await page.evaluate(async () => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { installBvhCsg, Brush, Evaluator } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    const { collectIntersectingTriangles, getHitSide, getHitSideWithCoplanarCheck, getOperationAction } =
        await import('/web/csg/core/operations/operationsUtils.js');
    const { TriangleSplitter } = await import('/web/csg/core/TriangleSplitter.js');
    const { ADDITION } = await import('/web/csg/core/constants.js');
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
    }
    const bBVH = b.geometry.boundsTree;
    const out = [];
    const targets = [
        {
            label: 'native',
            verts: [
                [-0.531259, -0.04235, -0.1],
                [-0.526229, -0.080558, -0.1],
                [-0.536835, 0.0, -0.1],
            ],
        },
        {
            label: 'ref',
            verts: [
                [-0.13992, -0.518411, -0.1],
                [-0.173204, -0.507379, -0.1],
                [-0.10116, -0.531258, -0.1],
            ],
        },
    ];
    for (const clipped of splitter.triangles) {
        for (const { label, verts: targetVerts } of targets) {
            let m = 0;
            for (const v of [clipped.a, clipped.b, clipped.c]) {
                for (const [tx, ty, tz] of targetVerts) {
                    if (Math.hypot(v.x - tx, v.y - ty, v.z - tz) < 0.002) {
                        m++;
                        break;
                    }
                }
            }
            if (m >= 3) {
                const hs = splitter.coplanarTriangleUsed
                    ? getHitSideWithCoplanarCheck(clipped, bBVH)
                    : getHitSide(clipped, bBVH);
                const action = getOperationAction(ADDITION, hs, false);
                out.push({
                    label,
                    m,
                    hs,
                    action,
                    cop: splitter.coplanarTriangleUsed,
                    verts: [
                        clipped.a.x, clipped.a.y, clipped.a.z,
                        clipped.b.x, clipped.b.y, clipped.b.z,
                        clipped.c.x, clipped.c.y, clipped.c.z,
                    ],
                });
            }
        }
    }
    return { count: splitter.triangles.length, out };
});

console.log(JSON.stringify(r, null, 2));
await browser.close();
