#!/usr/bin/env node
/**
 * Port three-bvh-csg@0.0.16 sources into web/csg/ with threers import paths.
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(__dirname, '..');
const srcRoot = process.argv[2] || '/tmp/three-bvh-csg-0.0.16/src';
const outRoot = path.join(webDir, 'csg');

const THREE_IMPORTS = new Set([
    'Mesh', 'Group', 'Matrix4', 'Matrix3', 'Triangle', 'Vector2', 'Vector3', 'Vector4',
    'BufferAttribute', 'BufferGeometry', 'Ray', 'Line3', 'Plane', 'DoubleSide',
    'Color', 'MathUtils', 'LineSegments', 'LineBasicMaterial', 'MeshPhongMaterial',
    'MeshBasicMaterial', 'InstancedMesh', 'SphereGeometry',
]);

function shimPath(fromFile) {
    const rel = path.relative(path.dirname(fromFile), webDir).replace(/\\/g, '/');
    return `${rel || '.'}/threejs-shim.js`;
}

function meshBvhPath(fromFile) {
    const rel = path.relative(path.dirname(fromFile), webDir).replace(/\\/g, '/');
    return `${rel || '.'}/mesh-bvh-impl.js`;
}

function transform(content, filePath) {
    const shim = shimPath(filePath);
    const bvh = meshBvhPath(filePath);

    content = content.replace(/from\s+['"]three['"]/g, `from '${shim}'`);
    content = content.replace(/from\s+['"]three-mesh-bvh['"]/g, (m, offset, str) => {
        if (str.includes('ExtendedTriangle')) {
            return `from '${bvh}'`;
        }
        return `from '${bvh}'`;
    });

    // Brush imports MeshBVH from three-mesh-bvh — already handled.
    return content;
}

function walk(dir, base = '') {
    for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
        const src = path.join(dir, ent.name);
        const rel = path.join(base, ent.name);
        if (ent.isDirectory()) {
            walk(src, rel);
        } else if (ent.name.endsWith('.js')) {
            const out = path.join(outRoot, rel);
            fs.mkdirSync(path.dirname(out), { recursive: true });
            const raw = fs.readFileSync(src, 'utf8');
            fs.writeFileSync(out, transform(raw, out));
        }
    }
}

if (!fs.existsSync(srcRoot)) {
    console.error('Source not found:', srcRoot);
    process.exit(1);
}
if (fs.existsSync(outRoot)) {
    fs.rmSync(outRoot, { recursive: true });
}
walk(srcRoot);
console.log('==> ported three-bvh-csg to', outRoot);
