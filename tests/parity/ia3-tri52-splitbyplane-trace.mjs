#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import puppeteer from 'puppeteer';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8118;
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
    const { ExtendedTriangle } = await import('/web/mesh-bvh-impl.js');
    const { isTriDegenerate } = await import('/web/csg/core/utils/triangleUtils.js');
    const THREE = (await import('/web/threejs-shim.js')).default;
    const EPSILON = 1e-10;
    const COPLANAR_EPSILON = 1e-10;
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
    const _splittingTriangle = new ExtendedTriangle();
    _splittingTriangle.copy(clipTri);
    const plane = clipTri.getPlane(new THREE.Plane());
    const _edge = new THREE.Line3();
    const _foundEdge = new THREE.Line3();
    const _vec = new THREE.Vector3();
    const gate = _splittingTriangle.intersectsTriangle(tri, _edge, true);
    const arr = [tri.a, tri.b, tri.c];
    let intersects = 0;
    let coplanarEdge = false;
    let vertexSplitEnd = -1;
    const posSideVerts = [];
    const negSideVerts = [];
    for (let t = 0; t < 3; t++) {
        const tNext = (t + 1) % 3;
        _edge.start.copy(arr[t]);
        _edge.end.copy(arr[tNext]);
        const startDist = plane.distanceToPoint(_edge.start);
        const endDist = plane.distanceToPoint(_edge.end);
        if (Math.abs(startDist) < COPLANAR_EPSILON && Math.abs(endDist) < COPLANAR_EPSILON) {
            coplanarEdge = true;
            break;
        }
        if (startDist > 0) posSideVerts.push(t);
        else negSideVerts.push(t);
        if (Math.abs(startDist) < COPLANAR_EPSILON) continue;
        let didIntersect = !!plane.intersectLine(_edge, _vec);
        if (!didIntersect && Math.abs(endDist) < COPLANAR_EPSILON) {
            _vec.copy(_edge.end);
            didIntersect = true;
        }
        if (didIntersect && !(_vec.distanceTo(_edge.start) < EPSILON)) {
            if (_vec.distanceTo(_edge.end) < EPSILON) vertexSplitEnd = t;
            if (intersects === 0) _foundEdge.start.copy(_vec);
            else _foundEdge.end.copy(_vec);
            intersects++;
        }
    }
    const edgeDist = _foundEdge.distance();
    const willSplit = !coplanarEdge && intersects === 2 && edgeDist > COPLANAR_EPSILON;
    let splitResult = null;
    if (willSplit && vertexSplitEnd === -1) {
        const singleVert = posSideVerts.length >= 2 ? negSideVerts[0] : posSideVerts[0];
        if (singleVert === 0) {
            const tmp = _foundEdge.start.clone();
            _foundEdge.start.copy(_foundEdge.end);
            _foundEdge.end.copy(tmp);
        }
        const nextVert1 = (singleVert + 1) % 3;
        const nextVert2 = (singleVert + 2) % 3;
        const fs = _foundEdge.start;
        const fe = _foundEdge.end;
        const useFirst =
            arr[nextVert1].distanceToSquared(fs) < arr[nextVert2].distanceToSquared(fe);
        const nextTri1 = new THREE.Triangle();
        const nextTri2 = new THREE.Triangle();
        if (useFirst) {
            nextTri1.a.copy(arr[nextVert1]);
            nextTri1.b.copy(fs);
            nextTri1.c.copy(fe);
            nextTri2.a.copy(arr[nextVert1]);
            nextTri2.b.copy(arr[nextVert2]);
            nextTri2.c.copy(fs);
        } else {
            nextTri1.a.copy(arr[nextVert2]);
            nextTri1.b.copy(fs);
            nextTri1.c.copy(fe);
            nextTri2.a.copy(arr[nextVert1]);
            nextTri2.b.copy(arr[nextVert2]);
            nextTri2.c.copy(fe);
        }
        const main = new THREE.Triangle();
        main.a.copy(arr[singleVert]);
        main.b.copy(fe);
        main.c.copy(fs);
        const ab = new THREE.Vector3().subVectors(main.b, main.a);
        const ac = new THREE.Vector3().subVectors(main.c, main.a);
        const cb = new THREE.Vector3().subVectors(main.b, main.c);
        const a1 = ab.angleTo(ac);
        const a2 = ab.angleTo(cb);
        const a3 = Math.PI - a1 - a2;
        const ab1 = new THREE.Vector3().subVectors(nextTri1.b, nextTri1.a);
        const ac1 = new THREE.Vector3().subVectors(nextTri1.c, nextTri1.a);
        const cb1 = new THREE.Vector3().subVectors(nextTri1.b, nextTri1.c);
        const t1a1 = ab1.angleTo(ac1);
        const t1a2 = ab1.angleTo(cb1);
        const t1a3 = Math.PI - t1a1 - t1a2;
        const dist1Ab = nextTri1.a.distanceToSquared(nextTri1.b);
        const dist1Ac = nextTri1.a.distanceToSquared(nextTri1.c);
        const dist1Bc = nextTri1.b.distanceToSquared(nextTri1.c);
        const distMainAb = main.a.distanceToSquared(main.b);
        const distMainAc = main.a.distanceToSquared(main.c);
        const distMainBc = main.b.distanceToSquared(main.c);
        const dist2Ab = nextTri2.a.distanceToSquared(nextTri2.b);
        const dist2Ac = nextTri2.a.distanceToSquared(nextTri2.c);
        const dist2Bc = nextTri2.b.distanceToSquared(nextTri2.c);
        const ab2 = new THREE.Vector3().subVectors(nextTri2.b, nextTri2.a);
        const ac2 = new THREE.Vector3().subVectors(nextTri2.c, nextTri2.a);
        const cb2 = new THREE.Vector3().subVectors(nextTri2.b, nextTri2.c);
        const na1 = ab2.angleTo(ac2);
        const na2 = ab2.angleTo(cb2);
        const na3 = Math.PI - na1 - na2;
        splitResult = {
            useFirst,
            singleVert,
            degen1: isTriDegenerate(nextTri1),
            degen2: isTriDegenerate(nextTri2),
            degenMain: isTriDegenerate(main),
            tri1Angles: [t1a1, t1a2, t1a3],
            tri1Dists: [dist1Ab, dist1Ac, dist1Bc],
            tri1Verts: [
                [nextTri1.a.x, nextTri1.a.y, nextTri1.a.z],
                [nextTri1.b.x, nextTri1.b.y, nextTri1.b.z],
                [nextTri1.c.x, nextTri1.c.y, nextTri1.c.z],
            ],
            mainAngles: [a1, a2, a3],
            mainDists: [distMainAb, distMainAc, distMainBc],
            next2Angles: [na1, na2, na3],
            next2Dists: [dist2Ab, dist2Ac, dist2Bc],
            mainVerts: [
                [main.a.x, main.a.y, main.a.z],
                [main.b.x, main.b.y, main.b.z],
                [main.c.x, main.c.y, main.c.z],
            ],
            next2Verts: [
                [nextTri2.a.x, nextTri2.a.y, nextTri2.a.z],
                [nextTri2.b.x, nextTri2.b.y, nextTri2.b.z],
                [nextTri2.c.x, nextTri2.c.y, nextTri2.c.z],
            ],
        };
    }
    return {
        gate,
        intersects,
        coplanarEdge,
        vertexSplitEnd,
        edgeDist,
        willSplit,
        posSideVerts,
        negSideVerts,
        splitResult,
    };
}, snap8[52], clip);

console.log(JSON.stringify(r, null, 2));
await browser.close();
