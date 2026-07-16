#!/usr/bin/env node
/** Compare native ia=3 snap42 (after 43 neighbors) against JS reference fixture. */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'scenes/rust/ia3-split-336.json'), 'utf8'),
);
const nativePath = path.join(__dirname, 'scenes/rust/ia3-snap42-native.json');

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

function loadKeys(snaps) {
    return new Set(snaps.map(triKey));
}

if (!fs.existsSync(nativePath)) {
    console.error('missing', nativePath, '- run: cargo test --features bvh-csg export_ia3_snap42_native -- --nocapture');
    process.exit(1);
}

const native = JSON.parse(fs.readFileSync(nativePath, 'utf8'));
const jsKeys = loadKeys(fixture.snap42);
const natKeys = loadKeys(native.snap42);
const shared = [...jsKeys].filter((k) => natKeys.has(k)).length;
console.log(`snap42: js=${jsKeys.size} native=${natKeys.size} shared=${shared}`);
const onlyJs = [...jsKeys].filter((k) => !natKeys.has(k));
const onlyNat = [...natKeys].filter((k) => !jsKeys.has(k));
console.log(`only_js=${onlyJs.length} only_native=${onlyNat.length}`);
if (onlyJs.length <= 3) for (const k of onlyJs) console.log('  only_js', k);
if (onlyNat.length <= 3) for (const k of onlyNat) console.log('  only_nat', k);
