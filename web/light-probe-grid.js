import {
    Color,
    CubeCamera,
    HalfFloatType,
    InstancedMesh,
    Matrix4,
    MeshBasicMaterial,
    Object3D,
    SphereGeometry,
    Vector3,
    WebGLCubeRenderTarget,
} from './threejs-shim.js';
import { projectCubeToSH } from './light-probe-math.js';

const COEFFICIENTS_PER_PROBE = 9 * 4;
const MAX_PROBES = 2048;
const CACHE_DATABASE = 'threers-light-probes-v1';
const CACHE_STORE = 'grids';

function normalizedBakeSettings(options = {}) {
    return {
        cubemapSize: Math.max(2, Math.floor(options.cubemapSize ?? 8)),
        near: Number(options.near ?? 0.1),
        far: Number(options.far ?? 100),
        bounces: Math.max(0, Math.floor(options.bounces ?? 0)),
    };
}

function valuesMatch(a, b) {
    return Math.abs(a - b) <= 1e-5 * Math.max(1, Math.abs(a), Math.abs(b));
}

function openCacheDatabase() {
    if (!globalThis.indexedDB) return Promise.reject(new Error('IndexedDB is unavailable'));
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(CACHE_DATABASE, 1);
        request.onupgradeneeded = () => {
            if (!request.result.objectStoreNames.contains(CACHE_STORE)) {
                request.result.createObjectStore(CACHE_STORE);
            }
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error || new Error('Could not open probe cache'));
    });
}

async function readPersistentCache(cacheKey) {
    const database = await openCacheDatabase();
    try {
        return await new Promise((resolve, reject) => {
            const request = database.transaction(CACHE_STORE, 'readonly').objectStore(CACHE_STORE).get(cacheKey);
            request.onsuccess = () => resolve(request.result ? new Uint8Array(request.result) : null);
            request.onerror = () => reject(request.error || new Error('Could not read probe cache'));
        });
    } finally {
        database.close();
    }
}

async function writePersistentCache(cacheKey, bytes) {
    const database = await openCacheDatabase();
    try {
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(CACHE_STORE, 'readwrite');
            transaction.objectStore(CACHE_STORE).put(bytes.slice().buffer, cacheKey);
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error('Could not write probe cache'));
            transaction.onabort = () => reject(transaction.error || new Error('Probe cache write aborted'));
        });
    } finally {
        database.close();
    }
}

async function deletePersistentCache(cacheKey) {
    const database = await openCacheDatabase();
    try {
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(CACHE_STORE, 'readwrite');
            transaction.objectStore(CACHE_STORE).delete(cacheKey);
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error('Could not delete probe cache'));
        });
    } finally {
        database.close();
    }
}

function checkedResolution(value) {
    const vector = value instanceof Vector3
        ? value.clone()
        : new Vector3(value?.x ?? value?.[0] ?? 2, value?.y ?? value?.[1] ?? 2, value?.z ?? value?.[2] ?? 2);
    vector.set(Math.floor(vector.x), Math.floor(vector.y), Math.floor(vector.z));
    if (vector.x < 2 || vector.y < 2 || vector.z < 2) {
        throw new Error('LightProbeGrid resolution must be at least 2 on every axis');
    }
    if (vector.x * vector.y * vector.z > MAX_PROBES) {
        throw new Error(`LightProbeGrid supports at most ${MAX_PROBES} probes`);
    }
    return vector;
}

export class LightProbeGrid extends Object3D {
    constructor(width = 5, height = 5, depth = 5, resolution = new Vector3(4, 4, 4)) {
        super();
        this.isLightProbeGrid = true;
        this.type = 'LightProbeGrid';
        this.width = width;
        this.height = height;
        this.depth = depth;
        this.resolution = checkedResolution(resolution);
        this.coefficients = null;
        this.boundingBox = { min: new Vector3(), max: new Vector3() };
        this._revision = 0;
        this._uploadedRevision = new WeakMap();
        this._baking = false;
        this.cacheSource = null;
        this.updateBoundingBox();
    }

    get count() {
        return this.resolution.x * this.resolution.y * this.resolution.z;
    }

    setResolution(resolution) {
        this.resolution = checkedResolution(resolution);
        this.coefficients = null;
        this._revision++;
        return this;
    }

    setSize(width, height, depth) {
        if (![width, height, depth].every((value) => Number.isFinite(value) && value > 0)) {
            throw new Error('LightProbeGrid size must be finite and positive');
        }
        this.width = width;
        this.height = height;
        this.depth = depth;
        this.updateBoundingBox();
        this._revision++;
        return this;
    }

    updateBoundingBox() {
        const half = new Vector3(this.width / 2, this.height / 2, this.depth / 2);
        this.boundingBox.min.copy(this.position).sub(half);
        this.boundingBox.max.copy(this.position).add(half);
        return this.boundingBox;
    }

    getProbePosition(index, target = new Vector3()) {
        const nx = this.resolution.x;
        const ny = this.resolution.y;
        const x = index % nx;
        const y = Math.floor(index / nx) % ny;
        const z = Math.floor(index / (nx * ny));
        const min = this.boundingBox.min;
        target.set(
            min.x + (x / (nx - 1)) * this.width,
            min.y + (y / (ny - 1)) * this.height,
            min.z + (z / (this.resolution.z - 1)) * this.depth,
        );
        return target;
    }

    setCoefficients(coefficients) {
        const expected = this.count * COEFFICIENTS_PER_PROBE;
        if (coefficients.length !== expected) {
            throw new Error(`Expected ${expected} light-probe values, got ${coefficients.length}`);
        }
        this.coefficients = coefficients instanceof Float32Array
            ? coefficients
            : new Float32Array(coefficients);
        this._revision++;
        return this;
    }

    toCacheBytes(renderer, options = {}) {
        if (!this.coefficients) throw new Error('LightProbeGrid has no baked coefficients');
        if (typeof options.cacheKey !== 'string' || options.cacheKey.length === 0) {
            throw new Error('Light-probe cacheKey must be a non-empty string');
        }
        this.updateBoundingBox();
        return renderer.encodeLightProbeGridCache(
            this.coefficients,
            this.resolution,
            this.boundingBox.min,
            this.boundingBox.max,
            normalizedBakeSettings(options),
            options.cacheKey,
        );
    }

    loadCacheBytes(renderer, bytes, options = {}) {
        if (typeof options.cacheKey !== 'string' || options.cacheKey.length === 0) {
            throw new Error('Light-probe cacheKey must be a non-empty string');
        }
        const decoded = renderer.decodeLightProbeGridCache(
            bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes),
            options.cacheKey,
        );
        const expectedSettings = normalizedBakeSettings(options);
        const expectedResolution = this.resolution;
        this.updateBoundingBox();
        if (decoded.resolution.x !== expectedResolution.x
            || decoded.resolution.y !== expectedResolution.y
            || decoded.resolution.z !== expectedResolution.z) {
            throw new Error('Cached light-probe resolution does not match this grid');
        }
        for (const axis of ['x', 'y', 'z']) {
            if (!valuesMatch(decoded.min[axis], this.boundingBox.min[axis])
                || !valuesMatch(decoded.max[axis], this.boundingBox.max[axis])) {
                throw new Error('Cached light-probe bounds do not match this grid');
            }
        }
        if (decoded.settings.cubemapSize !== expectedSettings.cubemapSize
            || decoded.settings.bounces !== expectedSettings.bounces
            || !valuesMatch(decoded.settings.near, expectedSettings.near)
            || !valuesMatch(decoded.settings.far, expectedSettings.far)) {
            throw new Error('Cached light-probe bake settings do not match');
        }
        this.setCoefficients(decoded.coefficients);
        if (this.visible) this._applyToRenderer(renderer, true);
        else renderer.clearLightProbeGrid();
        return this;
    }

    async bakeCached(renderer, scene, options = {}) {
        const settings = normalizedBakeSettings(options);
        const cacheKey = options.cacheKey;
        if (typeof cacheKey !== 'string' || cacheKey.length === 0) {
            throw new Error('Light-probe cacheKey must be a non-empty string');
        }
        const onProgress = typeof options.onProgress === 'function' ? options.onProgress : () => {};
        const candidates = [];
        if (!options.forceBake && options.cacheUrl) {
            try {
                const response = await fetch(options.cacheUrl, { cache: 'force-cache' });
                if (response.ok) candidates.push(['asset', new Uint8Array(await response.arrayBuffer())]);
            } catch {
                // A bundled cache is an optimization; baking remains the fallback.
            }
        }
        if (!options.forceBake && options.useIndexedDB !== false) {
            try {
                const persisted = await readPersistentCache(cacheKey);
                if (persisted) candidates.push(['indexeddb', persisted]);
            } catch {
                // Private browsing and restricted contexts may disable IndexedDB.
            }
        }
        for (const [source, bytes] of candidates) {
            try {
                this.loadCacheBytes(renderer, bytes, { ...settings, cacheKey });
                this.cacheSource = source;
                onProgress({
                    phase: 'cache', source, pass: settings.bounces + 1,
                    passes: settings.bounces + 1, probe: this.count,
                    total: this.count, ratio: 1,
                });
                return this;
            } catch (error) {
                console.warn(`Ignoring invalid ${source} light-probe cache`, error);
                if (source === 'indexeddb') {
                    try { await deletePersistentCache(cacheKey); } catch { /* ignore cache cleanup */ }
                }
            }
        }

        await this.bake(renderer, scene, { ...options, ...settings });
        this.cacheSource = 'baked';
        if (options.useIndexedDB !== false) {
            try {
                const bytes = this.toCacheBytes(renderer, { ...settings, cacheKey });
                await writePersistentCache(cacheKey, bytes);
            } catch {
                // The completed bake remains usable even if persistence is unavailable.
            }
        }
        return this;
    }

    _applyToRenderer(renderer, force = false) {
        if (!this.coefficients || this.visible === false) return;
        this.updateBoundingBox();
        const bounds = this.boundingBox;
        const uploadKey = `${this._revision}:${bounds.min.x}:${bounds.min.y}:${bounds.min.z}:${bounds.max.x}:${bounds.max.y}:${bounds.max.z}`;
        if (!force && this._uploadedRevision.get(renderer) === uploadKey) return;
        renderer.setLightProbeGrid(this.coefficients, this.resolution, this.boundingBox.min, this.boundingBox.max);
        this._uploadedRevision.set(renderer, uploadKey);
    }

    async bake(renderer, scene, options = {}) {
        if (this._baking) throw new Error('This LightProbeGrid is already baking');
        this._baking = true;
        const settings = normalizedBakeSettings(options);
        const cubeSize = settings.cubemapSize;
        const bounceCount = settings.bounces;
        const passCount = bounceCount + 1;
        const onProgress = typeof options.onProgress === 'function' ? options.onProgress : () => {};
        const signal = options.signal;
        const previousVisible = this.visible;
        const target = new WebGLCubeRenderTarget(cubeSize, { type: HalfFloatType });
        const camera = new CubeCamera(settings.near, settings.far, target);
        const probePosition = new Vector3();
        this.updateBoundingBox();

        try {
            let coefficients = new Float32Array(this.count * COEFFICIENTS_PER_PROBE);
            for (let pass = 0; pass < passCount; pass++) {
                if (signal?.aborted) throw signal.reason || new DOMException('Probe bake aborted', 'AbortError');
                if (pass === 0) {
                    this.visible = false;
                    renderer.clearLightProbeGrid();
                } else {
                    this.coefficients = coefficients;
                    this._revision++;
                    this.visible = true;
                    this._applyToRenderer(renderer, true);
                }

                const next = new Float32Array(coefficients.length);
                for (let probe = 0; probe < this.count; probe++) {
                    if (signal?.aborted) throw signal.reason || new DOMException('Probe bake aborted', 'AbortError');
                    camera.position.copy(this.getProbePosition(probe, probePosition));
                    camera.update(renderer, scene);
                    const bytes = await renderer.readProbeCube(target);
                    next.set(projectCubeToSH(bytes, cubeSize), probe * COEFFICIENTS_PER_PROBE);
                    const completed = pass * this.count + probe + 1;
                    onProgress({
                        phase: 'bake', pass: pass + 1, passes: passCount,
                        probe: probe + 1, total: this.count,
                        ratio: completed / (passCount * this.count),
                    });
                    if ((probe & 1) === 1) await new Promise((resolve) => requestAnimationFrame(resolve));
                }
                coefficients = next;
            }
            this.setCoefficients(coefficients);
            this.cacheSource = 'baked';
            this.visible = previousVisible;
            if (this.visible) this._applyToRenderer(renderer, true);
            else renderer.clearLightProbeGrid();
            onProgress({ phase: 'complete', pass: passCount, passes: passCount, probe: this.count, total: this.count, ratio: 1 });
            return this;
        } finally {
            target.dispose();
            this._baking = false;
            if (!this.coefficients) this.visible = previousVisible;
        }
    }

    dispose(renderer) {
        this.coefficients = null;
        this._revision++;
        if (renderer?._activeLightProbeGrid === this) renderer.clearLightProbeGrid();
    }
}

export class LightProbeGridHelper extends InstancedMesh {
    constructor(grid, size = 0.08) {
        super(
            new SphereGeometry(size, 10, 6),
            new MeshBasicMaterial({ color: 0xffffff }),
            grid.count,
        );
        this.grid = grid;
        this.type = 'LightProbeGridHelper';
        this.update();
    }

    update() {
        const matrix = new Matrix4();
        const position = new Vector3();
        const color = new Color();
        for (let probe = 0; probe < this.grid.count; probe++) {
            this.grid.getProbePosition(probe, position);
            matrix.identity().setPosition(position);
            this.setMatrixAt(probe, matrix);
            if (this.grid.coefficients) {
                const offset = probe * COEFFICIENTS_PER_PROBE;
                const exposure = 0.282095;
                color.setRGB(
                    Math.max(0, this.grid.coefficients[offset] * exposure),
                    Math.max(0, this.grid.coefficients[offset + 1] * exposure),
                    Math.max(0, this.grid.coefficients[offset + 2] * exposure),
                );
            } else {
                color.setRGB(0.08, 0.35, 1);
            }
            this.setColorAt(probe, color);
        }
        return this;
    }
}

export { projectCubeToSH } from './light-probe-math.js';
