#!/usr/bin/env node
/** Export per-neighbor clipped counts for shell sphere split (JS reference). */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(__dirname, 'scenes/rust');
const port = 8105;

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

const data = await page.evaluate(async () => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { installBvhCsg, Brush, Evaluator } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    const { collectIntersectingTriangles } = await import('/web/csg/core/operations/operationsUtils.js');
    const { TriangleSplitter } = await import('/web/csg/core/TriangleSplitter.js');
    const { getHitSideWithCoplanarCheck, getHitSide, getOperationAction, SKIP_TRI } =
        await import('/web/csg/core/operations/operationsUtils.js');
    const { ADDITION } = await import('/web/csg/core/constants.js');

    installBvhCsg(THREE);
    const g = (name, ...args) => geometryToBufferGeometry(new THREE[name](...args));
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc });

    let a = new Brush(g('BoxGeometry', 4, 2.5, 2.5), mat);
    let b = new Brush(g('BoxGeometry', 3.6, 2.1, 2.1), mat);
    let r = new Evaluator().evaluate(a, b, 1);

    a = new Brush(r.geometry, mat);
    b = new Brush(g('SphereGeometry', 0.55, 24, 12), mat);
    b.position.set(-1.1, 0.2, 1.35);
    b.updateMatrixWorld(true);
    a.prepareGeometry();
    b.prepareGeometry();

    const { aIntersections } = collectIntersectingTriangles(a, b);
    const splitter = new TriangleSplitter();
    const _matrix = new THREE.Matrix4().copy(b.matrixWorld).invert().multiply(a.matrixWorld);
    const _triA = new THREE.Triangle();
    const _triB = new THREE.Triangle();
    const aIndex = a.geometry.index;
    const aPosition = a.geometry.attributes.position;
    const bIndex = b.geometry.index;
    const bPosition = b.geometry.attributes.position;
    const bBVH = b.geometry.boundsTree;

    const perIa = [];
    for (const ia of aIntersections.ids) {
        const ia3 = 3 * ia;
        const ia0 = aIndex.getX(ia3);
        const ia1 = aIndex.getX(ia3 + 1);
        const ia2 = aIndex.getX(ia3 + 2);
        _triA.a.fromBufferAttribute(aPosition, ia0).applyMatrix4(_matrix);
        _triA.b.fromBufferAttribute(aPosition, ia1).applyMatrix4(_matrix);
        _triA.c.fromBufferAttribute(aPosition, ia2).applyMatrix4(_matrix);
        splitter.reset();
        splitter.initialize(_triA);
        const initialSnap = ia === 15 || ia === 3
            ? {
                matrix: Array.from(_matrix.elements),
                triA: [
                    _triA.a.x, _triA.a.y, _triA.a.z,
                    _triA.b.x, _triA.b.y, _triA.b.z,
                    _triA.c.x, _triA.c.y, _triA.c.z,
                ],
            }
            : null;
        const neighbors = aIntersections.intersectionSet[ia];
        const steps = [];
        const snapshots = ia === 15 || ia === 3 ? [] : null;
        for (let j = 0; j < neighbors.length; j++) {
            const ib = neighbors[j];
            const ib3 = 3 * ib;
            _triB.a.fromBufferAttribute(bPosition, bIndex.getX(ib3));
            _triB.b.fromBufferAttribute(bPosition, bIndex.getX(ib3 + 1));
            _triB.c.fromBufferAttribute(bPosition, bIndex.getX(ib3 + 2));
            splitter.splitByTriangle(_triB);
            steps.push({ ib, count: splitter.triangles.length });
            if (snapshots && (ia !== 3 || j === 42 || j === 43)) {
                const snap = splitter.triangles.map((t) => [
                    t.a.x, t.a.y, t.a.z,
                    t.b.x, t.b.y, t.b.z,
                    t.c.x, t.c.y, t.c.z,
                ]);
                snapshots.push({ j, ib, snap });
            }
        }
        let kept = 0;
        let hitMismatch = 0;
        for (const clipped of splitter.triangles) {
            const hitSide = splitter.coplanarTriangleUsed
                ? getHitSideWithCoplanarCheck(clipped, bBVH)
                : getHitSide(clipped, bBVH);
            const action = getOperationAction(ADDITION, hitSide, false);
            if (action !== SKIP_TRI) kept++;
        }
        perIa.push({ ia, steps, final: splitter.triangles.length, kept, snapshots, initialSnap });
    }
    return perIa;
});

await browser.close();

const outPath = path.join(outDir, 'shell-split-cascade.json');
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(data, null, 0));
console.log(`wrote ${outPath}`);
