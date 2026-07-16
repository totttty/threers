#!/usr/bin/env node
/**
 * Verify default (no-flag) builds expose stubs, not impl addons.
 * Run after `web/build.sh` without MESH_BVH/BVH_CSG.
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const webDir = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');

const features = fs.readFileSync(path.join(webDir, 'features.js'), 'utf8');
const meshAddon = fs.readFileSync(path.join(webDir, 'mesh-bvh-addon.js'), 'utf8');
const csgAddon = fs.readFileSync(path.join(webDir, 'bvh-csg-addon.js'), 'utf8');
const pkg = fs.readFileSync(path.join(webDir, 'pkg', 'threers.js'), 'utf8');

let ok = true;
if (features.includes('meshBvh: true')) {
    console.error('expected features.meshBvh false in default build');
    ok = false;
}
if (features.includes('bvhCsg: true')) {
    console.error('expected features.bvhCsg false in default build');
    ok = false;
}
if (meshAddon.includes('mesh-bvh-impl')) {
    console.error('mesh-bvh-addon should point to stub in default build');
    ok = false;
}
if (csgAddon.includes('bvh-csg-impl')) {
    console.error('bvh-csg-addon should point to stub in default build');
    ok = false;
}
if (pkg.includes('WebMeshBvh')) {
    console.error('wasm pkg should not include WebMeshBvh in default build');
    ok = false;
}

if (!ok) process.exit(1);
console.log('default build feature stubs OK');
