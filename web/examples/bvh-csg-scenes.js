/**
 * Deterministic CSG scenario builders for the bvh-csg interactive demo.
 * Parity scenes mirror these setups in tests/parity/scenes/threers-bvh-csg-*.html.
 */
import {
    Brush,
    Evaluator,
    Operation,
    OperationGroup,
    ADDITION,
    SUBTRACTION,
    INTERSECTION,
    DIFFERENCE,
    REVERSE_SUBTRACTION,
    HOLLOW_SUBTRACTION,
    HOLLOW_INTERSECTION,
} from '/web/bvh-csg-addon.js';

/** @param {typeof import('/web/threejs-shim.js').default} THREE */
export function geom(THREE, primitive) {
    return THREE.geometryToBufferGeometry(primitive);
}

/** Deterministic triangle soup (fixed layout, no Math.random). */
export function makeTriangleSoupGeometry(THREE) {
    const triangles = 40;
    const positions = new Float32Array(triangles * 9);
    const normals = new Float32Array(triangles * 9);
    const n = 200;
    const n2 = n / 2;
    const d = 80;
    const d2 = d / 2;
    const pA = new THREE.Vector3();
    const pB = new THREE.Vector3();
    const pC = new THREE.Vector3();
    const cb = new THREE.Vector3();
    const ab = new THREE.Vector3();

    for (let t = 0; t < triangles; t++) {
        const i = t * 9;
        const x = Math.sin(t * 0.31) * n - n2;
        const y = Math.cos(t * 0.47) * n - n2;
        const z = Math.sin(t * 0.19) * n - n2;
        const ax = x + Math.sin(t * 1.1) * d - d2;
        const ay = y + Math.cos(t * 1.3) * d - d2;
        const az = z + Math.sin(t * 0.9) * d - d2;
        const bx = x + Math.cos(t * 0.7) * d - d2;
        const by = y + Math.sin(t * 1.7) * d - d2;
        const bz = z + Math.cos(t * 1.1) * d - d2;
        const cx = x + Math.sin(t * 0.5) * d - d2;
        const cy = y + Math.cos(t * 0.3) * d - d2;
        const cz = z + Math.sin(t * 1.5) * d - d2;
        positions[i] = ax; positions[i + 1] = ay; positions[i + 2] = az;
        positions[i + 3] = bx; positions[i + 4] = by; positions[i + 5] = bz;
        positions[i + 6] = cx; positions[i + 7] = cy; positions[i + 8] = cz;
        pA.set(ax, ay, az);
        pB.set(bx, by, bz);
        pC.set(cx, cy, cz);
        cb.subVectors(pC, pB);
        ab.subVectors(pA, pB);
        cb.cross(ab).normalize();
        normals[i] = normals[i + 3] = normals[i + 6] = cb.x;
        normals[i + 1] = normals[i + 4] = normals[i + 7] = cb.y;
        normals[i + 2] = normals[i + 5] = normals[i + 8] = cb.z;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
    geometry.scale(0.006, 0.006, 0.006);
    return geometry;
}

export const SCENARIOS = [
    { id: 'simple', label: 'Simple' },
    { id: 'complex', label: 'Complex' },
    { id: 'hollow', label: 'Hollow' },
    { id: 'multimaterial', label: 'Multi-material' },
    { id: 'multiop', label: 'Multi-op' },
    { id: 'hierarchy', label: 'Hierarchy' },
];

const SIMPLE_OPS = [
    { id: 'union', label: 'Union', value: ADDITION },
    { id: 'subtract', label: 'Subtract', value: SUBTRACTION },
    { id: 'intersect', label: 'Intersect', value: INTERSECTION },
    { id: 'difference', label: 'Difference', value: DIFFERENCE },
];

const HOLLOW_OPS = [
    { id: 'hollow-intersect', label: 'Hollow ∩', value: HOLLOW_INTERSECTION },
    { id: 'hollow-subtract', label: 'Hollow −', value: HOLLOW_SUBTRACTION },
];

/** @param {typeof import('/web/threejs-shim.js').default} THREE */
export function getOpsForScenario(scenarioId) {
    if (scenarioId === 'hollow') return HOLLOW_OPS;
    if (scenarioId === 'simple') return SIMPLE_OPS;
    return [];
}

/**
 * @param {typeof import('/web/threejs-shim.js').default} THREE
 * @param {string} scenarioId
 * @param {number} [op]
 */
export function evaluateScenario(THREE, scenarioId, op = ADDITION) {
    const evaluator = new Evaluator();
    const mat = new THREE.MeshStandardMaterial({ color: 0x4488cc, roughness: 0.45, metalness: 0.1 });

    switch (scenarioId) {
        case 'simple': {
            const brushA = new Brush(geom(THREE, new THREE.SphereGeometry(1, 32, 16)), mat);
            const brushB = new Brush(geom(THREE, new THREE.BoxGeometry(1.2, 1.2, 1.2)), mat);
            brushB.position.set(0.5, 0, 0);
            brushA.updateMatrixWorld(true);
            brushB.updateMatrixWorld(true);
            const result = new Brush();
            evaluator.evaluate(brushA, brushB, op, result);
            result.geometry.computeVertexNormals();
            return { result, triCount: triCountOf(result.geometry), label: opLabel(op, SIMPLE_OPS) };
        }

        case 'complex': {
            const knotMat = new THREE.MeshStandardMaterial({ color: 0x4dd0e1, roughness: 0.45, metalness: 0.1 });
            const sphereMat = new THREE.MeshStandardMaterial({ color: 0xffab40, roughness: 0.45, metalness: 0.1 });
            const knot = new Brush(geom(THREE, new THREE.TorusKnotGeometry(0.8, 0.25, 64, 8)), knotMat);
            knot.position.y = -0.2;
            const spheres = [
                { pos: [0.35, 0.55, 0.2], scale: 0.18 },
                { pos: [-0.45, 0.15, -0.25], scale: 0.14 },
                { pos: [0.1, -0.35, 0.4], scale: 0.16 },
            ];
            let merged = null;
            for (const s of spheres) {
                const b = new Brush(geom(THREE, new THREE.SphereGeometry(1, 20, 12)), sphereMat);
                b.position.set(s.pos[0], s.pos[1], s.pos[2]);
                b.scale.setScalar(s.scale);
                b.updateMatrixWorld(true);
                if (!merged) merged = b;
                else merged = evaluator.evaluate(merged, b, ADDITION);
            }
            knot.updateMatrixWorld(true);
            const result = new Brush();
            evaluator.evaluate(knot, merged, SUBTRACTION, result);
            result.geometry.computeVertexNormals();
            return { result, triCount: triCountOf(result.geometry), label: 'Complex subtract' };
        }

        case 'hollow': {
            evaluator.attributes = ['position', 'normal'];
            evaluator.useGroups = false;
            const soupMat = new THREE.MeshStandardMaterial({
                color: 0x88aacc,
                side: THREE.DoubleSide,
                roughness: 0.3,
            });
            const brushA = new Brush(makeTriangleSoupGeometry(THREE), soupMat);
            const brushB = new Brush(geom(THREE, new THREE.SphereGeometry(1, 32, 16)), mat);
            brushB.position.set(0, 0.8, 0);
            brushB.scale.setScalar(1.6);
            brushA.updateMatrixWorld(true);
            brushB.updateMatrixWorld(true);
            const result = new Brush();
            evaluator.evaluate(brushA, brushB, op, result);
            result.geometry.computeVertexNormals();
            return { result, triCount: triCountOf(result.geometry), label: opLabel(op, HOLLOW_OPS) };
        }

        case 'multimaterial': {
            const red = new THREE.MeshStandardMaterial({ color: 0xff1744, roughness: 0.25 });
            const green = new THREE.MeshStandardMaterial({ color: 0x76ff03, roughness: 0.25 });
            const blue = new THREE.MeshStandardMaterial({ color: 0x2979ff, roughness: 0.25 });
            // Box axes avoid a wasm BVH hang seen with chained cylinder CSG + useGroups.
            const c1 = new Brush(geom(THREE, new THREE.BoxGeometry(0.9, 6, 0.9)), blue);
            const c2 = new Brush(geom(THREE, new THREE.BoxGeometry(6, 0.9, 0.9)), blue);
            const c3 = new Brush(geom(THREE, new THREE.BoxGeometry(0.9, 0.9, 6)), blue);
        const sphere = new Brush(geom(THREE, new THREE.SphereGeometry(1, 32, 16)), green);
            const box = new Brush(geom(THREE, new THREE.BoxGeometry(1.5, 1.5, 1.5)), red);
            [c1, c2, c3, sphere, box].forEach(b => b.updateMatrixWorld(true));
            let result = evaluator.evaluate(c1, c2, ADDITION);
            result = evaluator.evaluate(result, c3, ADDITION);
            result = evaluator.evaluate(sphere, result, SUBTRACTION);
            result = evaluator.evaluate(box, result, INTERSECTION);
            result.geometry.computeVertexNormals();
            const groups = result.geometry.groups?.length ?? 0;
            const mats = Array.isArray(result.material) ? result.material.length : 1;
            return { result, triCount: triCountOf(result.geometry), label: `Multi-material · ${groups} groups · ${mats} mats` };
        }

        case 'multiop': {
            evaluator.attributes = ['position', 'normal'];
            const mat1 = new THREE.MeshStandardMaterial({ color: 0xfff8e1, roughness: 0.9, side: THREE.DoubleSide });
            const mat2 = new THREE.MeshStandardMaterial({ color: 0xff9800, roughness: 0.9, side: THREE.DoubleSide });
            const brush1 = new Brush(geom(THREE, new THREE.IcosahedronGeometry(1, 1)), mat1);
            const brush2 = new Brush(geom(THREE, new THREE.CylinderGeometry(0.5, 0.5, 2.5, 24)), mat2);
            brush1.rotation.set(0.35, 0.55, 0.2);
            brush2.rotation.set(-0.25, -0.4, -0.65);
            brush1.updateMatrixWorld(true);
            brush2.updateMatrixWorld(true);
            const r1 = new Brush();
            const r2 = new Brush();
            const r3 = new Brush();
            const r4 = new Brush();
            evaluator.evaluate(
                brush1, brush2,
                [SUBTRACTION, INTERSECTION, ADDITION, REVERSE_SUBTRACTION],
                [r1, r2, r3, r4],
            );
            for (const r of [r1, r2, r3, r4]) r.geometry.computeVertexNormals();
            return {
                results: [
                    { mesh: r1, position: [-1.8, 0, 1.8] },
                    { mesh: r2, position: [1.8, 0, 1.8] },
                    { mesh: r3, position: [-1.8, 0, -1.8] },
                    { mesh: r4, position: [1.8, 0, -1.8] },
                ],
                triCount: [r1, r2, r3, r4].reduce((n, r) => n + triCountOf(r.geometry), 0),
                label: 'Batch · 4 ops',
            };
        }

        case 'hierarchy': {
            evaluator.useGroups = false;
            const root = new Operation(geom(THREE, new THREE.BoxGeometry(4, 2.5, 2.5)), mat);
            root.operation = ADDITION;
            const cut = new Operation(geom(THREE, new THREE.BoxGeometry(3.6, 2.1, 2.1)), mat);
            cut.operation = SUBTRACTION;
            const sphere = new Operation(geom(THREE, new THREE.SphereGeometry(0.55, 24, 12)), mat);
            sphere.operation = ADDITION;
            sphere.position.set(-1.1, 0.2, 1.35);
            const windowGroup = new OperationGroup();
            const winCut = new Operation(geom(THREE, new THREE.BoxGeometry(1.2, 1.0, 0.5)), mat);
            winCut.operation = SUBTRACTION;
            const winFrame = new Operation(geom(THREE, new THREE.BoxGeometry(1.2, 1.0, 0.12)), mat);
            winFrame.operation = ADDITION;
            windowGroup.add(winCut, winFrame);
            windowGroup.position.set(0.8, 0.15, 1.35);
            root.add(cut, sphere, windowGroup);
            root.updateMatrixWorld(true);
            const result = new Brush();
            evaluator.evaluateHierarchy(root, result);
            result.geometry.computeVertexNormals();
            return { result, triCount: triCountOf(result.geometry), label: 'Hierarchy · OperationGroup' };
        }

        default:
            throw new Error(`unknown scenario: ${scenarioId}`);
    }
}

function triCountOf(geometry) {
    const dr = geometry.drawRange;
    return ((dr.count !== Infinity ? dr.count : geometry.attributes.position.count) / 3) | 0;
}

function opLabel(op, list) {
    return list.find(o => o.value === op)?.label || 'CSG';
}
