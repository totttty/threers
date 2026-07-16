/**
 * Stub bvh-csg addon — active when wasm was built without BVH_CSG=1.
 */
import { features, isBvhCsgEnabled } from './features.js';

const ERR = 'bvh-csg feature is disabled. Rebuild wasm with: BVH_CSG=1 web/build.sh';

function disabled(name) {
    const err = () => { throw new Error(`${name}: ${ERR}`); };
    err.enabled = false;
    return err;
}

export { features, isBvhCsgEnabled };

export class Brush { constructor() { throw new Error(`Brush: ${ERR}`); } }
export class Evaluator { constructor() { throw new Error(`Evaluator: ${ERR}`); } }
export class Operation { constructor() { throw new Error(`Operation: ${ERR}`); } }
export class OperationGroup { constructor() { throw new Error(`OperationGroup: ${ERR}`); } }
export class HalfEdgeMap { constructor() { throw new Error(`HalfEdgeMap: ${ERR}`); } }
export class TriangleSplitter { constructor() { throw new Error(`TriangleSplitter: ${ERR}`); } }
export class GridMaterial { constructor() { throw new Error(`GridMaterial: ${ERR}`); } }

export const ADDITION = 0;
export const SUBTRACTION = 1;
export const REVERSE_SUBTRACTION = 2;
export const INTERSECTION = 3;
export const DIFFERENCE = 4;
export const HOLLOW_SUBTRACTION = 5;
export const HOLLOW_INTERSECTION = 6;

export const installBvhCsg = disabled('installBvhCsg');
export const computeMeshVolume = disabled('computeMeshVolume');

export default { installBvhCsg, features, isBvhCsgEnabled };
