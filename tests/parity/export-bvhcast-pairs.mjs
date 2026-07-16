#!/usr/bin/env node
/** Export bvhcast pairs + BVH buffers for shell+sphere (JS CSG reference). */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(__dirname, 'scenes/rust');
const port = 8098;

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
    const { installBvhCsg, Brush, Evaluator, SUBTRACTION } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    const { collectIntersectingTriangles } = await import('/web/csg/core/operations/operationsUtils.js');
    installBvhCsg(THREE);
    const g = (name, ...args) => geometryToBufferGeometry(new THREE[name](...args));
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc });
    const evaluator = new Evaluator();
    evaluator.useGroups = false;

    let a = new Brush(g('BoxGeometry', 4, 2.5, 2.5), mat);
    let b = new Brush(g('BoxGeometry', 3.6, 2.1, 2.1), mat);
    let r = evaluator.evaluate(a, b, SUBTRACTION);

    a = new Brush(r.geometry, mat);
    b = new Brush(g('SphereGeometry', 0.55, 24, 12), mat);
    b.position.set(-1.1, 0.2, 1.35);
    b.updateMatrixWorld(true);

    a.prepareGeometry();
    b.prepareGeometry();

    const matrix = new THREE.Matrix4()
        .copy(a.matrixWorld)
        .invert()
        .multiply(b.matrixWorld);

    const pairs = [];
    a.geometry.boundsTree.bvhcast(b.geometry.boundsTree, matrix, {
        intersectsTriangles(triangleA, triangleB, ia, ib) {
            pairs.push(ia, ib);
            return false;
        },
    });

    const { aIntersections, bIntersections } = collectIntersectingTriangles(a, b);

    const serialize = (bvh) => {
        const parts = bvh._w.serialize();
        return {
            node_buffer: Array.from(parts[1]),
            triangle_order: Array.from(parts[2]),
            triangle_indices: Array.from(parts[3]),
            positions: Array.from(parts[4]),
        };
    };

    const neighborLists = (map) => {
        const out = [];
        for (const id of map.ids) {
            out.push(map.intersectionSet[id].length, ...map.intersectionSet[id]);
        }
        return out;
    };

    return {
        pairs,
        aIds: aIntersections.ids,
        bIds: bIntersections.ids,
        aNeighbors: neighborLists(aIntersections),
        bNeighbors: neighborLists(bIntersections),
        shellBvh: serialize(a.geometry.boundsTree),
        sphereBvh: serialize(b.geometry.boundsTree),
    };
});

await browser.close();

function writeF32(buf, offset, arr) {
    for (let i = 0; i < arr.length; i++) {
        buf.writeFloatLE(arr[i], offset + i * 4);
    }
    return offset + arr.length * 4;
}

function writeU32(buf, offset, arr) {
    for (let i = 0; i < arr.length; i++) {
        buf.writeUInt32LE(arr[i], offset + i * 4);
    }
    return offset + arr.length * 4;
}

function packBvh(bvh) {
    const header = Buffer.alloc(20);
    header.write('BVH1', 0, 'ascii');
    header.writeUInt32LE(bvh.node_buffer.length, 4);
    header.writeUInt32LE(bvh.triangle_order.length, 8);
    header.writeUInt32LE(bvh.triangle_indices.length, 12);
    header.writeUInt32LE(bvh.positions.length, 16);
    const nb = Buffer.from(new Float32Array(bvh.node_buffer).buffer);
    const to = Buffer.from(new Uint32Array(bvh.triangle_order).buffer);
    const ti = Buffer.from(new Uint32Array(bvh.triangle_indices).buffer);
    const pos = Buffer.from(new Float32Array(bvh.positions).buffer);
    return Buffer.concat([header, nb, to, ti, pos]);
}

const shellBin = packBvh(data.shellBvh);
const sphereBin = packBvh(data.sphereBvh);
const pairBytes = data.pairs.length * 4;
const aIdBytes = data.aIds.length * 4;
const bIdBytes = data.bIds.length * 4;
const aNeighborBytes = data.aNeighbors.length * 4;
const bNeighborBytes = data.bNeighbors.length * 4;
const header = 24;
const total = header + pairBytes + aIdBytes + bIdBytes + aNeighborBytes + bNeighborBytes + shellBin.length + sphereBin.length;
const out = Buffer.alloc(total);
let o = 0;
out.write('BCP1', 0, 'ascii');
o = 4;
out.writeUInt32LE(data.pairs.length / 2, o); o += 4;
out.writeUInt32LE(data.aIds.length, o); o += 4;
out.writeUInt32LE(data.bIds.length, o); o += 4;
out.writeUInt32LE(data.aNeighbors.length, o); o += 4;
out.writeUInt32LE(data.bNeighbors.length, o); o += 4;
o = writeU32(out, o, data.pairs);
o = writeU32(out, o, data.aIds);
o = writeU32(out, o, data.bIds);
o = writeU32(out, o, data.aNeighbors);
o = writeU32(out, o, data.bNeighbors);
shellBin.copy(out, o); o += shellBin.length;
sphereBin.copy(out, o);

const outPath = path.join(outDir, 'bvhcast-shell-sphere.bin');
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(outPath, out);
console.log(`wrote ${outPath}`);
console.log('pairs:', data.pairs.length / 2, 'aIds:', data.aIds.length, 'bIds:', data.bIds.length);
