#!/usr/bin/env node
/** Shared CSG parity scene body (operation name for three.js / threers templates). */
export function csgSceneScript(operation, { hierarchy = false } = {}) {
    const opImport = hierarchy
        ? `ADDITION, SUBTRACTION, Operation`
        : operation;
    const evalBlock = hierarchy ? `
        const inner = new Operation(new THREE.SphereGeometry(1, 32, 16), mat);
        inner.operation = ADDITION;
        const outer = new Brush(new THREE.BoxGeometry(1.4, 1.4, 1.4), mat);
        outer.position.set(0.45, 0, 0);
        const root = new Operation(new THREE.BoxGeometry(2.2, 0.6, 0.6), mat);
        root.operation = SUBTRACTION;
        root.add(inner);
        root.add(outer);
        inner.updateMatrixWorld(true);
        outer.updateMatrixWorld(true);
        root.updateMatrixWorld(true);
        const result = new Brush();
        evaluator.evaluateHierarchy(root, result);
` : `
        const brush1 = new Brush(new THREE.SphereGeometry(1, 32, 16).toNonIndexed(), mat);
        const brush2 = new Brush(new THREE.BoxGeometry(1.2, 1.2, 1.2).toNonIndexed(), mat);
        brush2.position.set(0.5, 0, 0);
        brush1.updateMatrixWorld(true);
        brush2.updateMatrixWorld(true);
        const result = new Brush();
        evaluator.evaluate(brush1, brush2, ${operation}, result);
`;
    return { opImport, evalBlock };
}
