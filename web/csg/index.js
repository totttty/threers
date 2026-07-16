/**
 * three-bvh-csg@0.0.16 port — only load via bvh-csg-addon when BVH_CSG=1.
 */
import { isBvhCsgEnabled } from '../features.js';

if (!isBvhCsgEnabled()) {
    throw new Error('bvh-csg feature is disabled. Rebuild with: BVH_CSG=1 web/build.sh');
}

export * from './core/Brush.js';
export * from './core/Evaluator.js';
export * from './core/operations/Operation.js';
export * from './core/operations/OperationGroup.js';
export * from './core/TriangleSplitter.js';
export * from './core/HalfEdgeMap.js';
export * from './materials/GridMaterial.js';

export * from './core/constants.js';
export * from './core/debug/debugUtils.js';

export * from './objects/TriangleSetHelper.js';
export * from './objects/EdgesHelper.js';
export * from './objects/PointsHelper.js';
export * from './objects/HalfEdgeHelper.js';

export * from './utils/computeMeshVolume.js';
