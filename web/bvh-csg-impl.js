/**
 * bvh-csg implementation (three-bvh-csg@0.0.16 port).
 * Loaded only when features.bvhCsg is true (BVH_CSG=1 web/build.sh).
 */
import { isBvhCsgEnabled } from '/web/features.js';
import { installMeshBvh } from '/web/mesh-bvh-impl.js';

const _FEATURE_ERR = 'bvh-csg feature is disabled. Rebuild with: BVH_CSG=1 web/build.sh';

if (!isBvhCsgEnabled()) {
    throw new Error(_FEATURE_ERR);
}

export * from './csg/index.js';

function _requireBvhCsg() {
    if (!isBvhCsgEnabled()) throw new Error(_FEATURE_ERR);
}

/** Patch threers for three-bvh-csg compatibility (implies installMeshBvh). */
export function installBvhCsg(THREE = {}) {
    _requireBvhCsg();
    installMeshBvh(THREE);
    return { enabled: true };
}

export default { installBvhCsg };
