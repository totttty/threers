/**
 * Stub mesh-bvh addon — active when wasm was built without MESH_BVH=1.
 * Rebuild with: MESH_BVH=1 web/build.sh
 */

import { features, isMeshBvhEnabled, isBvhCsgEnabled } from './features.js';

const ERR = 'mesh-bvh feature is disabled. Rebuild wasm with: MESH_BVH=1 web/build.sh';

function disabled(name) {
    const err = () => { throw new Error(`${name}: ${ERR}`); };
    err.enabled = false;
    return err;
}

export { features, isMeshBvhEnabled, isBvhCsgEnabled };

export const NOT_INTERSECTED = 0;
export const INTERSECTED = 1;
export const CONTAINED = 2;
export const CENTER = 0;
export const AVERAGE = 1;
export const SAH = 2;

export class MeshBVH {
    constructor() {
        throw new Error(`MeshBVH: ${ERR}`);
    }
    static serialize() { throw new Error(`MeshBVH.serialize: ${ERR}`); }
    static deserialize() { throw new Error(`MeshBVH.deserialize: ${ERR}`); }
}

export class GenerateMeshBVHWorker {
    constructor() { throw new Error(`GenerateMeshBVHWorker: ${ERR}`); }
}

export class StaticGeometryGenerator {
    constructor() {
        throw new Error(`StaticGeometryGenerator: ${ERR}`);
    }
    generate() {
        throw new Error(`StaticGeometryGenerator.generate: ${ERR}`);
    }
}

export const acceleratedRaycast = disabled('acceleratedRaycast');
export const computeBoundsTree = disabled('computeBoundsTree');
export const disposeBoundsTree = disabled('disposeBoundsTree');
export const installMeshBvh = disabled('installMeshBvh');

export default {
    MeshBVH,
    StaticGeometryGenerator,
    acceleratedRaycast,
    computeBoundsTree,
    disposeBoundsTree,
    installMeshBvh,
    features,
    isMeshBvhEnabled,
    NOT_INTERSECTED,
    INTERSECTED,
    CONTAINED,
    CENTER,
};
