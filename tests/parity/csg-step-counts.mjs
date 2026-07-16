#!/usr/bin/env node
import puppeteer from 'puppeteer';
import { spawn } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8094;
spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 500));

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

const steps = await page.evaluate(async () => {
    const THREE = (await import('/web/threejs-shim.js')).default;
    const { installBvhCsg, Brush, Evaluator, ADDITION, SUBTRACTION } = await import('/web/bvh-csg-addon.js');
    const { geometryToBufferGeometry } = await import('/web/threejs-shim.js');
    installBvhCsg(THREE);
    const g = (name, ...args) => geometryToBufferGeometry(new THREE[name](...args));
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc });
    const evaluator = new Evaluator();
    evaluator.useGroups = false;
    const vc = (geom) => geom.attributes.position.count;

    let a = new Brush(g('BoxGeometry', 4, 2.5, 2.5), mat);
    let b = new Brush(g('BoxGeometry', 3.6, 2.1, 2.1), mat);
    let r = evaluator.evaluate(a, b, SUBTRACTION);
    const s1 = vc(r.geometry);

    a = new Brush(r.geometry, mat);
    b = new Brush(g('SphereGeometry', 0.55, 24, 12), mat);
    b.position.set(-1.1, 0.2, 1.35);
    b.updateMatrixWorld(true);
    r = evaluator.evaluate(a, b, ADDITION);
    const s2 = vc(r.geometry);

    a = new Brush(r.geometry, mat);
    b = new Brush(g('BoxGeometry', 1.2, 1.0, 0.5), mat);
    b.position.set(0.8, 0.15, 1.35);
    b.updateMatrixWorld(true);
    r = evaluator.evaluate(a, b, SUBTRACTION);
    const s3 = vc(r.geometry);

    a = new Brush(r.geometry, mat);
    b = new Brush(g('BoxGeometry', 1.2, 1.0, 0.12), mat);
    b.position.set(0.8, 0.15, 1.35);
    b.updateMatrixWorld(true);
    r = evaluator.evaluate(a, b, ADDITION);
    const s4 = vc(r.geometry);

    return { s1, s2, s3, s4 };
});

console.log('JS threers CSG steps:', steps);
await browser.close();
