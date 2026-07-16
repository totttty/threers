#!/usr/bin/env node
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import path from 'path';

const webDir = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const features = readFileSync(path.join(webDir, 'features.js'), 'utf8');
const pkgPath = path.join(webDir, 'pkg/threers.js');

if (!features.includes('meshBvh: true')) {
    console.error('mesh-bvh check failed: features.meshBvh is false in web/features.js');
    console.error('Rebuild with: MESH_BVH=1 web/build.sh');
    process.exit(1);
}
const pkg = readFileSync(pkgPath, 'utf8');
if (!pkg.includes('WebMeshBvh')) {
    console.error('mesh-bvh check failed: WebMeshBvh not found in web/pkg/threers.js');
    console.error('Rebuild with: MESH_BVH=1 web/build.sh');
    process.exit(1);
}
const addon = readFileSync(path.join(webDir, 'mesh-bvh-addon.js'), 'utf8');
if (!addon.includes('mesh-bvh-impl')) {
    console.error('mesh-bvh check failed: mesh-bvh-addon.js does not re-export impl');
    process.exit(1);
}
console.log('mesh-bvh feature flag OK (features.js + wasm + addon)');
