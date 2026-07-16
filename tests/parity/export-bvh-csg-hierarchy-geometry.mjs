#!/usr/bin/env node
/** Export evaluated bvh-csg-hierarchy mesh for native Rust parity / examples. */
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8091;
const outBin = path.join(__dirname, 'scenes/rust/bvh-csg-hierarchy.geom.bin');

spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 400));

const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium',
    headless: 'new',
    args: ['--no-sandbox', '--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-vulkan=swiftshader'],
});

const page = await browser.newPage();
await page.goto(
    `http://127.0.0.1:${port}/tests/parity/scenes/threers-bvh-csg-hierarchy.html`,
    { waitUntil: 'networkidle0', timeout: 120000 },
);
await page.waitForFunction(() => document.body.dataset.ready === 'true', { timeout: 120000 });

const arrays = await page.evaluate(async () => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const {
        installBvhCsg, Brush, Evaluator, Operation, OperationGroup, ADDITION, SUBTRACTION,
    } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    installBvhCsg(THREE);
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc });
    const evaluator = new Evaluator();
    evaluator.useGroups = false;
    const g = (geo) => geometryToBufferGeometry(new THREE[geo.split('(')[0]](...geo.match(/[\d.]+/g).map(Number)));
    const root = new Operation(geometryToBufferGeometry(new THREE.BoxGeometry(4, 2.5, 2.5)), mat);
    root.operation = ADDITION;
    const cut = new Operation(geometryToBufferGeometry(new THREE.BoxGeometry(3.6, 2.1, 2.1)), mat);
    cut.operation = SUBTRACTION;
    const sphere = new Operation(geometryToBufferGeometry(new THREE.SphereGeometry(0.55, 24, 12)), mat);
    sphere.operation = ADDITION;
    sphere.position.set(-1.1, 0.2, 1.35);
    const windowGroup = new OperationGroup();
    const winCut = new Operation(geometryToBufferGeometry(new THREE.BoxGeometry(1.2, 1.0, 0.5)), mat);
    winCut.operation = SUBTRACTION;
    const winFrame = new Operation(geometryToBufferGeometry(new THREE.BoxGeometry(1.2, 1.0, 0.12)), mat);
    winFrame.operation = ADDITION;
    windowGroup.add(winCut, winFrame);
    windowGroup.position.set(0.8, 0.15, 1.35);
    root.add(cut, sphere, windowGroup);
    root.updateMatrixWorld(true);
    const result = new Brush();
    evaluator.evaluateHierarchy(root, result);
    result.geometry.computeVertexNormals();
    const pos = result.geometry.attributes.position.array;
    const nor = result.geometry.attributes.normal.array;
    const draw = result.geometry.drawRange.count;
    const vertCount = draw === Infinity ? pos.length / 3 : draw;
    return {
        vertCount,
        triCount: (vertCount / 3) | 0,
        position: Array.from(pos.slice(0, vertCount * 3)),
        normal: Array.from(nor.slice(0, vertCount * 3)),
    };
});

await browser.close();

const vertCount = arrays.vertCount;
const pos = new Float32Array(arrays.position);
const nor = new Float32Array(arrays.normal);
if (pos.length !== vertCount * 3 || nor.length !== vertCount * 3) {
    throw new Error(`length mismatch: verts=${vertCount} pos=${pos.length} nor=${nor.length}`);
}

const buf = Buffer.alloc(8 + pos.byteLength + nor.byteLength);
buf.write('TCG1', 0, 'ascii');
buf.writeUInt32LE(vertCount, 4);
Buffer.from(pos.buffer).copy(buf, 8);
Buffer.from(nor.buffer).copy(buf, 8 + pos.byteLength);

fs.mkdirSync(path.dirname(outBin), { recursive: true });
fs.writeFileSync(outBin, buf);
console.log(`wrote ${outBin} — ${vertCount} verts, ${arrays.triCount} tris, ${buf.length} bytes`);
