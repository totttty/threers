import { InstancedMesh, Object3D, Vector3, WebGLRenderer } from './threejs-shim.js';

export interface LightProbeBakeProgress {
    phase: 'bake' | 'cache' | 'complete';
    source?: 'asset' | 'indexeddb';
    pass: number;
    passes: number;
    probe: number;
    total: number;
    ratio: number;
}

export interface LightProbeCacheOptions extends LightProbeBakeOptions {
    cacheKey: string;
    cacheUrl?: string | null;
    useIndexedDB?: boolean;
    forceBake?: boolean;
}

export interface LightProbeBakeOptions {
    cubemapSize?: number;
    near?: number;
    far?: number;
    bounces?: number;
    signal?: AbortSignal;
    onProgress?: (progress: LightProbeBakeProgress) => void;
}

export class LightProbeGrid extends Object3D {
    readonly isLightProbeGrid: true;
    width: number;
    height: number;
    depth: number;
    resolution: Vector3;
    coefficients: Float32Array | null;
    cacheSource: 'asset' | 'indexeddb' | 'baked' | null;
    readonly count: number;
    readonly boundingBox: { min: Vector3; max: Vector3 };
    constructor(width?: number, height?: number, depth?: number, resolution?: Vector3 | [number, number, number]);
    setResolution(resolution: Vector3 | [number, number, number]): this;
    setSize(width: number, height: number, depth: number): this;
    updateBoundingBox(): { min: Vector3; max: Vector3 };
    getProbePosition(index: number, target?: Vector3): Vector3;
    setCoefficients(coefficients: Float32Array | ArrayLike<number>): this;
    toCacheBytes(renderer: WebGLRenderer, options: LightProbeCacheOptions): Uint8Array;
    loadCacheBytes(renderer: WebGLRenderer, bytes: Uint8Array | ArrayBuffer, options: LightProbeCacheOptions): this;
    bakeCached(renderer: WebGLRenderer, scene: unknown, options: LightProbeCacheOptions): Promise<this>;
    bake(renderer: WebGLRenderer, scene: unknown, options?: LightProbeBakeOptions): Promise<this>;
    dispose(renderer?: WebGLRenderer): void;
}

export class LightProbeGridHelper extends InstancedMesh {
    grid: LightProbeGrid;
    constructor(grid: LightProbeGrid, size?: number);
    update(): this;
}

export { projectCubeToSH } from './light-probe-math.js';
