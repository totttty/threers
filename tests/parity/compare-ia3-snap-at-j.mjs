#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

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

function loadKeys(file) {
    const j = JSON.parse(fs.readFileSync(file, 'utf8'));
    const snap = j.snap ?? j.snap42;
    return { meta: j, keys: new Set(snap.map(triKey)) };
}

const j = Number(process.argv[2] ?? 17);
const js = loadKeys(path.join(__dirname, `scenes/rust/ia3-snap-j${j}.json`));
const nat = loadKeys(path.join(__dirname, `scenes/rust/ia3-snap-j${j}-native.json`));
const shared = [...js.keys].filter((k) => nat.keys.has(k)).length;
console.log(`j=${j} js=${js.keys.size} native=${nat.keys.size} shared=${shared}`);
const onlyJs = [...js.keys].filter((k) => !nat.keys.has(k));
const onlyNat = [...nat.keys].filter((k) => !js.keys.has(k));
console.log(`only_js=${onlyJs.length} only_nat=${onlyNat.length}`);
if (onlyJs[0]) console.log(' only_js', onlyJs[0]);
if (onlyNat[0]) console.log(' only_nat', onlyNat[0]);
