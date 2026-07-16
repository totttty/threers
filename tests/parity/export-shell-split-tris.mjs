#!/usr/bin/env node
/** Export shell split-phase triangle keys for s2 sphere ADDITION (JS reference). */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(__dirname, 'scenes/rust');
const port = 8099;

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
    const { installBvhCsg, Brush, Evaluator, ADDITION } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    const { collectIntersectingTriangles } = await import('/web/csg/core/operations/operationsUtils.js');
    const { hashVertex3 } = await import('/web/csg/core/utils/hashUtils.js');
    const { TriangleSplitter } = await import('/web/csg/core/TriangleSplitter.js');
    const {
        getHitSideWithCoplanarCheck,
        getHitSide,
        getOperationAction,
        SKIP_TRI,
    } = await import('/web/csg/core/operations/operationsUtils.js');
    const { ADDITION: ADD_CONST } = await import('/web/csg/core/constants.js');

    installBvhCsg(THREE);
    const g = (name, ...args) => geometryToBufferGeometry(new THREE[name](...args));
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc });

    let a = new Brush(g('BoxGeometry', 4, 2.5, 2.5), mat);
    let b = new Brush(g('BoxGeometry', 3.6, 2.1, 2.1), mat);
    let r = new Evaluator().evaluate(a, b, 1); // SUBTRACTION

    a = new Brush(r.geometry, mat);
    b = new Brush(g('SphereGeometry', 0.55, 24, 12), mat);
    b.position.set(-1.1, 0.2, 1.35);
    b.updateMatrixWorld(true);
    a.prepareGeometry();
    b.prepareGeometry();

    const { aIntersections } = collectIntersectingTriangles(a, b);
    const operations = [ADD_CONST];
    const splitter = new TriangleSplitter();

    const _matrix = new THREE.Matrix4().copy(b.matrixWorld).invert().multiply(a.matrixWorld);
    const _triA = new THREE.Triangle();
    const _triB = new THREE.Triangle();
    const _bary = new THREE.Triangle();
    const _world = new THREE.Triangle();
    const _worldSrc = new THREE.Triangle();

    const triKeyFromWorld = (tri) => {
        const ha = hashVertex3(tri.a).split(',').map(Number);
        const hb = hashVertex3(tri.b).split(',').map(Number);
        const hc = hashVertex3(tri.c).split(',').map(Number);
        return [...ha, ...hb, ...hc];
    };

    const pushWorldTri = (clipped, ia0, ia1, ia2, returnOnly = false) => {
        _triA.a.fromBufferAttribute(aPosition, ia0).applyMatrix4(_matrix);
        _triA.b.fromBufferAttribute(aPosition, ia1).applyMatrix4(_matrix);
        _triA.c.fromBufferAttribute(aPosition, ia2).applyMatrix4(_matrix);
        _triA.getBarycoord(clipped.a, _bary.a);
        _triA.getBarycoord(clipped.b, _bary.b);
        _triA.getBarycoord(clipped.c, _bary.c);
        _worldSrc.a.fromBufferAttribute(aPosition, ia0).applyMatrix4(a.matrixWorld);
        _worldSrc.b.fromBufferAttribute(aPosition, ia1).applyMatrix4(a.matrixWorld);
        _worldSrc.c.fromBufferAttribute(aPosition, ia2).applyMatrix4(a.matrixWorld);
        _world.a.set(0, 0, 0).addScaledVector(_worldSrc.a, _bary.a.x).addScaledVector(_worldSrc.b, _bary.a.y).addScaledVector(_worldSrc.c, _bary.a.z);
        _world.b.set(0, 0, 0).addScaledVector(_worldSrc.a, _bary.b.x).addScaledVector(_worldSrc.b, _bary.b.y).addScaledVector(_worldSrc.c, _bary.b.z);
        _world.c.set(0, 0, 0).addScaledVector(_worldSrc.a, _bary.c.x).addScaledVector(_worldSrc.b, _bary.c.y).addScaledVector(_worldSrc.c, _bary.c.z);
        const key = triKeyFromWorld(_world);
        if (!returnOnly) splitKeys.push(key);
        return key;
    };
    const splitKeys = [];
    const clippedKeys = [];
    let clippedTotal = 0;
    const splitIds = aIntersections.ids;
    const intersectionSet = aIntersections.intersectionSet;
    const aIndex = a.geometry.index;
    const aPosition = a.geometry.attributes.position;
    const bIndex = b.geometry.index;
    const bPosition = b.geometry.attributes.position;
    const bBVH = b.geometry.boundsTree;

    for (let i = 0, l = splitIds.length; i < l; i++) {
        const ia = splitIds[i];
        const ia3 = 3 * ia;
        const ia0 = aIndex.getX(ia3);
        const ia1 = aIndex.getX(ia3 + 1);
        const ia2 = aIndex.getX(ia3 + 2);
        _triA.a.fromBufferAttribute(aPosition, ia0).applyMatrix4(_matrix);
        _triA.b.fromBufferAttribute(aPosition, ia1).applyMatrix4(_matrix);
        _triA.c.fromBufferAttribute(aPosition, ia2).applyMatrix4(_matrix);
        splitter.reset();
        splitter.initialize(_triA);
        const neighbors = intersectionSet[ia];
        for (let j = 0, lj = neighbors.length; j < lj; j++) {
            const ib = neighbors[j];
            const ib3 = 3 * ib;
            const ib0 = bIndex.getX(ib3);
            const ib1 = bIndex.getX(ib3 + 1);
            const ib2 = bIndex.getX(ib3 + 2);
            _triB.a.fromBufferAttribute(bPosition, ib0);
            _triB.b.fromBufferAttribute(bPosition, ib1);
            _triB.c.fromBufferAttribute(bPosition, ib2);
            splitter.splitByTriangle(_triB);
        }
        const triangles = splitter.triangles;
        clippedTotal += triangles.length;
        for (let t = 0, lt = triangles.length; t < lt; t++) {
            const clipped = triangles[t];
            clippedKeys.push(pushWorldTri(clipped, ia0, ia1, ia2, true));
            const hitSide = splitter.coplanarTriangleUsed
                ? getHitSideWithCoplanarCheck(clipped, bBVH)
                : getHitSide(clipped, bBVH);
            const action = getOperationAction(operations[0], hitSide, false);
            if (action === SKIP_TRI) continue;
            pushWorldTri(clipped, ia0, ia1, ia2);
        }
    }

    return { splitKeys, clippedKeys, splitIdCount: splitIds.length, clippedTotal };
});

await browser.close();

function writeI32(buf, offset, arr) {
    for (let i = 0; i < arr.length; i++) {
        buf.writeInt32LE(arr[i], offset + i * 4);
    }
    return offset + arr.length * 4;
}

const flatKept = data.splitKeys.flat();
const flatClipped = data.clippedKeys.flat();
const header = 20;
const out = Buffer.alloc(header + flatKept.length * 4 + flatClipped.length * 4);
out.write('SSP1', 0, 'ascii');
out.writeUInt32LE(data.splitIdCount, 4);
out.writeUInt32LE(data.splitKeys.length, 8);
out.writeUInt32LE(data.clippedKeys.length, 12);
out.writeUInt32LE(data.clippedTotal, 16);
let o = writeI32(out, 20, flatKept);
writeI32(out, o, flatClipped);

const outPath = path.join(outDir, 'shell-split-sphere-addition.bin');
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(outPath, out);
console.log(`wrote ${outPath}`);
console.log('split ids:', data.splitIdCount, 'clipped total:', data.clippedTotal, 'kept tris:', data.splitKeys.length);
