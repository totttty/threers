#!/usr/bin/env node
/**
 * Generate web/threejs-shim.d.ts from web/threejs-shim.js exports + THREE namespace.
 * Run: node web/scripts/generate-shim-types.mjs
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(__dirname, '..');
const shimPath = path.join(webDir, 'threejs-shim.js');
const outPath = path.join(webDir, 'threejs-shim.d.ts');

const src = fs.readFileSync(shimPath, 'utf8');

const exportRe = /^export (async )?(class|function|const) (\w+)/gm;
const exports = [];
let m;
while ((m = exportRe.exec(src)) !== null) {
    exports.push({ kind: m[2], name: m[3], async: !!m[1] });
}

const threeStart = src.indexOf('const THREE = {');
const threeEnd = src.indexOf('\n};', threeStart);
const threeBlock = src.slice(threeStart, threeEnd);
const threeInner = threeBlock.replace(/^const THREE = \{/, '').replace(/\{[\s\S]*?\}/g, (m) => {
    // preserve nested inline class names
    const cls = m.match(/class\s+(\w+)/);
    return cls ? cls[1] : '';
});
const threeKeys = [];
for (const part of threeInner.split(',')) {
    const token = part.trim().split(/\s+/)[0].replace(/^class\s+/, '');
    if (/^[A-Za-z_]\w*$/.test(token) && !threeKeys.includes(token)) threeKeys.push(token);
}

const exportNames = new Set(exports.map((e) => e.name));

/** Hand-maintained signatures for threers-specific APIs. */
const MANUAL = `
/** Wasm init — pass a wasm URL string or \`{ module_or_path }\` options object. */
export function initThreers(options?: ThreersInitOptions | string): Promise<void>;

export interface ThreersInitOptions {
    module_or_path?: string | URL | Request | Response | Promise<Response>;
}

/** Marker for wasm-backed shim objects. */
export interface ThreersHandle {
    /** @internal wasm object handle */
    _w?: unknown;
}

export type ThreersColorInput = number | string | Color | { r: number; g: number; b: number };

// ---- Math (mirrors three.js; backed by wasm) ----

export class Color {
    r: number;
    g: number;
    b: number;
    constructor(color?: ThreersColorInput);
    set(color: ThreersColorInput): this;
    getHex(): number;
    copy(c: Color): this;
}

export class Vector2 {
    x: number;
    y: number;
    constructor(x?: number, y?: number);
    set(x: number, y: number): this;
    copy(v: Vector2): this;
}

export class Vector3 {
    x: number;
    y: number;
    z: number;
    constructor(x?: number, y?: number, z?: number);
    set(x: number, y?: number, z?: number): this;
    copy(v: Vector3): this;
    clone(): Vector3;
}

export class Euler {
    x: number;
    y: number;
    z: number;
    order: string;
    constructor(x?: number, y?: number, z?: number, order?: string);
}

export class Quaternion {
    x: number;
    y: number;
    z: number;
    w: number;
    constructor(x?: number, y?: number, z?: number, w?: number);
    setFromEuler(e: Euler): this;
}

// ---- Scene graph ----

export class Scene implements ThreersHandle {
    _w?: unknown;
    children: unknown[];
    background: Color | ThreersColorInput | null;
    fog: Fog | FogExp2 | null;
    environment: CubeTexture | Texture | null;
    constructor();
    add(...object: unknown[]): this;
    remove(...object: unknown[]): this;
}

export class Group implements ThreersHandle {
    _w?: unknown;
    position: Vector3;
    rotation: Euler;
    scale: Vector3;
    constructor();
    add(child: unknown): this;
}

export class Mesh implements ThreersHandle {
    _w?: unknown;
    geometry: BufferGeometry | unknown;
    material: Material | Material[] | unknown;
    position: Vector3;
    rotation: Euler;
    quaternion: Quaternion;
    scale: Vector3;
    visible: boolean;
    morphTargetInfluences: number[];
    constructor(geometry: unknown, material: unknown);
    raycast(raycaster: Raycaster, intersects: unknown[]): void;
    updateMorphTargets(): void;
}

export class PerspectiveCamera implements ThreersHandle {
    _w?: unknown;
    position: Vector3;
    aspect: number;
    near: number;
    far: number;
    fov: number;
    constructor(fov: number, aspect: number, near?: number, far?: number);
    lookAt(x: number, y: number, z: number): this;
    lookAt(v: Vector3): this;
    updateProjectionMatrix(): void;
}

export class OrthographicCamera implements ThreersHandle {
    _w?: unknown;
    position: Vector3;
    near: number;
    far: number;
    constructor(left: number, right: number, top: number, bottom: number, near?: number, far?: number);
    lookAt(x: number, y: number, z: number): this;
}

// ---- Renderer (async factory — threers-specific) ----

export class WebGLRenderer implements ThreersHandle {
    _w?: unknown;
    domElement: HTMLCanvasElement;
    shadowMap: { enabled: boolean; type: number };
    /** @deprecated use WebGLRenderer.create(canvas) */
    constructor(): never;
    static create(canvas: HTMLCanvasElement): Promise<WebGLRenderer>;
    setSize(width: number, height?: number, updateStyle?: boolean): void;
    setPixelRatio(value: number): void;
    render(scene: Scene, camera: PerspectiveCamera | OrthographicCamera): void;
    setRenderTarget(target: WebGLRenderTarget | null): void;
    getRenderTarget(): WebGLRenderTarget | null;
    applyPostFx(
        inputRT: WebGLRenderTarget,
        effectKind: number,
        time?: number,
        p2?: [number, number, number, number],
        additive?: boolean,
        depthRT?: WebGLRenderTarget | null,
        normalRT?: WebGLRenderTarget | null,
        cam?: unknown,
        p3?: [number, number, number, number],
    ): void;
    readRenderTargetPixels(
        target: WebGLRenderTarget,
        x: number,
        y: number,
        width: number,
        height: number,
        buffer?: Uint8Array,
    ): Promise<Uint8Array | void>;
}

export type WebGPURenderer = WebGLRenderer;

export class WebGLRenderTarget implements ThreersHandle {
    _w?: unknown;
    width: number;
    height: number;
    texture: Texture;
    depthBuffer: boolean;
    stencilBuffer: boolean;
    type: number;
    constructor(width?: number, height?: number, options?: WebGLRenderTargetOptions);
    setSize(width: number, height: number): void;
    clone(): WebGLRenderTarget;
    dispose(): void;
}

export interface WebGLRenderTargetOptions {
    type?: number;
    depthBuffer?: boolean;
    stencilBuffer?: boolean;
}

// ---- Controls ----

export class OrbitControls {
    constructor(camera: PerspectiveCamera, domElement?: HTMLElement | null);
    update(
        dx?: number,
        dy?: number,
        wheel?: number,
        rotating?: boolean,
        panning?: boolean,
    ): void;
}

export class TrackballControls {
    domElement: HTMLElement | null;
    enabled: boolean;
    constructor(camera: PerspectiveCamera, domElement?: HTMLElement | null);
    update(
        dx?: number,
        dy?: number,
        wheel?: number,
        rotating?: boolean,
        panning?: boolean,
    ): void;
    dispose(): void;
}

export class FirstPersonControls {
    domElement: HTMLElement | null;
    enabled: boolean;
    constructor(camera: PerspectiveCamera, domElement?: HTMLElement | null);
    update(dx?: number, dy?: number, dt?: number, rotating?: boolean): void;
}

export class DragControls implements ThreersHandle {
    _w?: unknown;
    objects: unknown[];
    camera: PerspectiveCamera;
    domElement: HTMLElement | null;
    enabled: boolean;
    constructor(objects: unknown[], camera: PerspectiveCamera, domElement?: HTMLElement | null);
    addEventListener(type: string, listener: (event: { type: string; object?: unknown }) => void): void;
    removeEventListener(type: string, listener: (event: { type: string; object?: unknown }) => void): void;
    dispose(): void;
    update(): void;
}

// ---- Post-processing ----

export class EffectComposer {
    renderer: WebGLRenderer;
    passes: Pass[];
    renderToScreen: boolean;
    renderTarget1: WebGLRenderTarget;
    renderTarget2: WebGLRenderTarget;
    writeBuffer: WebGLRenderTarget;
    readBuffer: WebGLRenderTarget;
    constructor(renderer: WebGLRenderer, renderTarget?: WebGLRenderTarget);
    addPass(pass: Pass): void;
    insertPass(pass: Pass, index: number): void;
    removePass(pass: Pass): void;
    setSize(width: number, height: number): void;
    render(deltaTime?: number): Promise<void>;
    dispose(): void;
}

export interface Pass {
    enabled: boolean;
    needsSwap: boolean;
    renderToScreen: boolean;
    render?(
        renderer: WebGLRenderer,
        writeBuffer: WebGLRenderTarget,
        readBuffer: WebGLRenderTarget,
        deltaTime?: number,
    ): void | Promise<void>;
    setSize?(width: number, height: number): void;
}

export class RenderPass implements Pass {
    scene: Scene;
    camera: PerspectiveCamera | OrthographicCamera;
    enabled: boolean;
    needsSwap: boolean;
    renderToScreen: boolean;
    clear: boolean;
    constructor(scene: Scene, camera: PerspectiveCamera | OrthographicCamera);
    render(
        renderer: WebGLRenderer,
        writeBuffer: WebGLRenderTarget,
        readBuffer: WebGLRenderTarget,
    ): void;
}

export class ShaderPass implements Pass {
    enabled: boolean;
    needsSwap: boolean;
    renderToScreen: boolean;
    uniforms: Record<string, { value: unknown }>;
    constructor(shader: { uniforms?: Record<string, { value: unknown }> }, textureID?: string);
    render(
        renderer: WebGLRenderer,
        writeBuffer: WebGLRenderTarget,
        readBuffer: WebGLRenderTarget,
    ): void;
}

export class GlitchPass implements Pass {
    enabled: boolean;
    needsSwap: boolean;
    renderToScreen: boolean;
    goWild: boolean;
    constructor(dtSize?: number);
    render(
        renderer: WebGLRenderer,
        writeBuffer: WebGLRenderTarget,
        readBuffer: WebGLRenderTarget,
    ): void | Promise<void>;
}

export function seedRandom(seed: number): void;

// ---- Loaders ----

export class GLTFLoader {
    constructor(manager?: LoadingManager);
    load(
        url: string,
        onLoad: (gltf: { scene: Group | Scene; animations: AnimationClip[] }) => void,
        onProgress?: (event: ProgressEvent) => void,
        onError?: (err: unknown) => void,
    ): void;
    loadAsync(url: string): Promise<{ scene: Group | Scene; animations: AnimationClip[] }>;
    parse(data: ArrayBuffer | string, path: string): { scene: Group | Scene; animations: AnimationClip[] };
}

export class LoadingManager {
    onStart?: (url: string, loaded: number, total: number) => void;
    onLoad?: () => void;
    onProgress?: (url: string, loaded: number, total: number) => void;
    onError?: (url: string) => void;
}

export class Raycaster {
    constructor(origin?: Vector3, direction?: Vector3, near?: number, far?: number);
    setFromCamera(coords: Vector2, camera: PerspectiveCamera | OrthographicCamera): void;
    intersectObjects(objects: unknown[], recursive?: boolean): Intersection[];
}

export interface Intersection {
    distance: number;
    point: Vector3;
    object: unknown;
}

export class Clock {
    autoStart: boolean;
    startTime: number;
    oldTime: number;
    elapsedTime: number;
    running: boolean;
    constructor(autoStart?: boolean);
    start(): void;
    getElapsedTime(): number;
    getDelta(): number;
}

export class Fog {
    color: Color;
    near: number;
    far: number;
    constructor(color: ThreersColorInput, near: number, far: number);
}

export class FogExp2 {
    isFogExp2: true;
    color: Color;
    density: number;
    constructor(color: ThreersColorInput, density: number);
}

export interface Material extends ThreersHandle {
    transparent?: boolean;
    opacity?: number;
    side?: number;
    depthWrite?: boolean;
    depthTest?: boolean;
}

export interface BufferGeometry extends ThreersHandle {
    attributes: Record<string, BufferAttribute>;
    index: BufferAttribute | null;
    morphAttributes: { position?: BufferAttribute[] };
    setAttribute(name: string, attr: BufferAttribute): this;
    setIndex(index: BufferAttribute | Uint32Array | null): this;
}

export class BufferAttribute {
    array: ArrayLike<number>;
    itemSize: number;
    count: number;
    constructor(array: ArrayLike<number>, itemSize: number);
}

export class AnimationClip {
    name: string;
    duration: number;
    tracks: KeyframeTrack[];
    constructor(name: string, duration: number, tracks: KeyframeTrack[]);
    static parse(json: unknown): AnimationClip;
}

export class AnimationMixer {
    constructor(root: unknown);
    clipAction(clip: AnimationClip, root?: unknown): AnimationAction;
    update(delta: number): this;
}

export interface AnimationAction {
    play(): this;
    stop(): this;
    reset(): this;
    setEffectiveTimeScale(value: number): this;
    setEffectiveWeight(value: number): this;
}

export interface KeyframeTrack {
    name: string;
    times: Float32Array | number[];
    values: Float32Array | number[];
}

export class Texture extends ThreersHandle {
    image?: unknown;
    wrapS: number;
    wrapT: number;
    magFilter: number;
    minFilter: number;
    flipY: boolean;
    needsUpdate: boolean;
}

export class CubeTexture extends Texture {}

export class DataTexture extends Texture {}

export class CanvasTexture extends Texture {}

export interface MeshStandardMaterialParameters {
    color?: ThreersColorInput;
    roughness?: number;
    metalness?: number;
    map?: Texture | null;
    normalMap?: Texture | null;
    emissive?: ThreersColorInput;
    emissiveIntensity?: number;
    transparent?: boolean;
    opacity?: number;
    side?: number;
}

export class MeshStandardMaterial implements Material {
    _w?: unknown;
    constructor(parameters?: MeshStandardMaterialParameters);
}

export class MeshBasicMaterial implements Material {
    _w?: unknown;
    constructor(parameters?: { color?: ThreersColorInput; map?: Texture | null; transparent?: boolean; opacity?: number; side?: number });
}

export class BoxGeometry {
    _w?: unknown;
    parameters: { width: number; height: number; depth: number };
    constructor(width?: number, height?: number, depth?: number);
}

export class SphereGeometry {
    _w?: unknown;
    constructor(radius?: number, widthSegments?: number, heightSegments?: number);
}

export class PlaneGeometry {
    _w?: unknown;
    constructor(width?: number, height?: number);
}

export class AmbientLight {
    _w?: unknown;
    constructor(color?: ThreersColorInput, intensity?: number);
}

export class DirectionalLight {
    _w?: unknown;
    position: Vector3;
    castShadow: boolean;
    shadow: unknown;
    constructor(color?: ThreersColorInput, intensity?: number);
}

export class PointLight {
    _w?: unknown;
    position: Vector3;
    constructor(color?: ThreersColorInput, intensity?: number, distance?: number, decay?: number);
}

export class SpotLight {
    _w?: unknown;
    position: Vector3;
    constructor(color?: ThreersColorInput, intensity?: number, distance?: number, angle?: number, penumbra?: number, decay?: number);
}

export class HemisphereLight {
    _w?: unknown;
    constructor(skyColor?: ThreersColorInput, groundColor?: ThreersColorInput, intensity?: number);
}

// ---- Video export (native-codec) ----
export {
    VideoFormat,
    BrowserVideoFormat,
    VideoExporter,
    VideoExportResult,
    VideoExportError,
    VideoExportErrorCode,
    VideoExportEvent,
    VideoExportProgressEvent,
    VideoEncodeWorker,
    isVideoExportAvailable,
    assertVideoExportAvailable,
    parseVideoFormat,
    formatVideoProgress,
    emitProgress,
    formatBytes,
    alignVideoSize,
    videoMimeType,
    videoFilename,
    encodeVideoFrames,
    encodeVideoFramesInWorker,
    downloadVideoBytes,
    encodeAndDownloadVideoFrames,
    encodeGifRgba,
    encodeApngRgba,
    encodeWebmRgba,
} from './video-export.js';

export type {
    VideoFormatName,
    BrowserVideoFormatName,
    VideoEncodeOptions,
    VideoExportOptions,
    VideoDownloadOptions,
    VideoExportProgress,
    VideoExportPhase,
    VideoFormatPresetOptions,
    VideoEncodeWorkerOptions,
    VideoExportStartDetail,
    VideoExportCompleteDetail,
    VideoExportErrorDetail,
} from './video-export.js';

export type SceneVideoExportOptions = Omit<VideoEncodeOptions, 'width' | 'height'> & {
    /** Defaults to canvas drawing-buffer width when omitted. */
    width?: number;
    /** Defaults to canvas drawing-buffer height when omitted. */
    height?: number;
    /** Frame count (alias: \`frames\`). Defaults to 30 for \`VideoExporter.from()\`. */
    frameCount?: number;
    frames?: number;
    /** Alternative to frameCount: \`round(duration * fps)\` seconds. */
    duration?: number;
    update?: (frameIndex: number, frameCount: number) => void;
    onFrame?: (frameIndex: number, frameCount: number) => void;
    renderTarget?: WebGLRenderTarget;
    /** Ring of targets for pipelined capture (length ≥ concurrency). */
    renderTargets?: WebGLRenderTarget[];
    /** Yield to the event loop between frames (default true). */
    yield?: boolean;
    /**
     * Pipeline GPU readbacks across multiple render targets.
     * \`true\` → 2 in flight; a number sets concurrency (1–8).
     */
    parallel?: boolean | number;
    /** Explicit in-flight readback count (1–8). Overrides \`parallel\`. */
    concurrency?: number;
    /** Encode captured frames in a Web Worker (wasm off main thread). */
    encodeInWorker?: boolean;
    worker?: boolean | VideoEncodeWorker;
    wasmUrl?: string;
    workerUrl?: string;
    /**
     * Clear a top-left corner to alpha=0 each frame (boolean or pixel size).
     * Prefer \`mapFrame\` for custom transforms.
     */
    transparentCornerPunch?: boolean | number;
};

export type SceneVideoExporter = VideoExporter & {
    frames(n: number): SceneVideoExporter;
    duration(seconds: number): SceneVideoExporter;
    update(fn: (frameIndex: number, frameCount: number) => void): SceneVideoExporter;
    yieldBetweenFrames(on?: boolean): SceneVideoExporter;
    transparentCornerPunch(on?: boolean | number): SceneVideoExporter;
    /** Pipeline readbacks: \`true\` → 2, or pass 1–8. */
    parallel(n?: boolean | number): SceneVideoExporter;
    concurrency(n: number): SceneVideoExporter;
    /** Encode on a Web Worker after capture. */
    worker(on?: boolean): SceneVideoExporter;
    on(type: string, listener: EventListenerOrEventListenerObject, options?: boolean | AddEventListenerOptions): SceneVideoExporter;
    off(type: string, listener: EventListenerOrEventListenerObject, options?: boolean | EventListenerOptions): SceneVideoExporter;
    once(type: string, listener: EventListenerOrEventListenerObject): SceneVideoExporter;
    export(overrides?: Partial<SceneVideoExportOptions>): Promise<VideoExportResult>;
    download(
        filenameOrOverrides?: string | Partial<SceneVideoExportOptions>,
    ): Promise<VideoExportResult>;
};

declare module './video-export.js' {
    namespace VideoExporter {
        function from(
            renderer: WebGLRenderer,
            scene: Scene,
            camera: PerspectiveCamera | OrthographicCamera | Camera,
            options?: Partial<SceneVideoExportOptions>,
        ): SceneVideoExporter;
    }
}

export function captureSceneFrames(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: PerspectiveCamera | OrthographicCamera | Camera,
    options?: Partial<SceneVideoExportOptions>,
): Promise<Uint8Array[]>;
export function exportSceneVideo(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: PerspectiveCamera | OrthographicCamera | Camera,
    options?: Partial<SceneVideoExportOptions>,
): Promise<VideoExportResult>;
export function exportAndDownloadSceneVideo(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: PerspectiveCamera | OrthographicCamera | Camera,
    options?: Partial<SceneVideoExportOptions>,
): Promise<VideoExportResult>;
`;

const manualNames = new Set(
    MANUAL.split('\n')
        .map((line) => line.match(/^export (?:class|interface|type|function) (\w+)/))
        .filter(Boolean)
        .map((m) => m[1]),
);
manualNames.add('ThreersInitOptions');
manualNames.add('ThreersHandle');
manualNames.add('ThreersColorInput');
manualNames.add('WebGLRenderTargetOptions');
manualNames.add('MeshStandardMaterialParameters');
manualNames.add('Intersection');
manualNames.add('AnimationAction');
manualNames.add('Pass');
manualNames.add('VideoFormat');
manualNames.add('BrowserVideoFormat');
manualNames.add('VideoFormatName');
manualNames.add('BrowserVideoFormatName');
manualNames.add('VideoEncodeWorker');
manualNames.add('VideoEncodeWorkerOptions');
manualNames.add('encodeVideoFramesInWorker');
manualNames.add('VideoExportEvent');
manualNames.add('VideoExportProgressEvent');
manualNames.add('emitProgress');
manualNames.add('VideoExportStartDetail');
manualNames.add('VideoExportCompleteDetail');
manualNames.add('VideoExportErrorDetail');
manualNames.add('VideoFormatPresetOptions');
manualNames.add('VideoExporter');
manualNames.add('VideoExportResult');
manualNames.add('VideoExportError');
manualNames.add('VideoExportErrorCode');
manualNames.add('VideoEncodeOptions');
manualNames.add('VideoExportOptions');
manualNames.add('VideoDownloadOptions');
manualNames.add('VideoExportProgress');
manualNames.add('VideoExportPhase');
manualNames.add('SceneVideoExportOptions');
manualNames.add('SceneVideoExporter');
manualNames.add('isVideoExportAvailable');
manualNames.add('assertVideoExportAvailable');
manualNames.add('parseVideoFormat');
manualNames.add('formatVideoProgress');
manualNames.add('formatBytes');
manualNames.add('alignVideoSize');
manualNames.add('videoMimeType');
manualNames.add('videoFilename');
manualNames.add('encodeVideoFrames');
manualNames.add('downloadVideoBytes');
manualNames.add('encodeAndDownloadVideoFrames');
manualNames.add('encodeGifRgba');
manualNames.add('encodeApngRgba');
manualNames.add('encodeWebmRgba');
manualNames.add('captureSceneFrames');
manualNames.add('exportSceneVideo');
manualNames.add('exportAndDownloadSceneVideo');

const stubs = exports
    .filter((e) => e.kind === 'class' && !manualNames.has(e.name))
    .map((e) => {
        return `/** @see three.js \`${e.name}\` — threers shim implementation */\nexport class ${e.name} implements ThreersHandle {\n    _w?: unknown;\n    constructor(...args: any[]);\n    [key: string]: any;\n}\n`;
    })
    .join('\n');

const funcStubs = exports
    .filter((e) => e.kind === 'function' && !manualNames.has(e.name))
    .map((e) => `export function ${e.name}(...args: any[]): any;\n`)
    .join('\n');

const constStubs = exports
    .filter((e) => e.kind === 'const' && !manualNames.has(e.name))
    .map((e) => `export const ${e.name}: any;\n`)
    .join('\n');

const namespaceEntries = [...new Set([...threeKeys, ...exportNames])].sort().map((key) => {
    if (exportNames.has(key) || manualNames.has(key)) {
        return `    ${key}: typeof ${key};`;
    }
    if (/^[A-Z][A-Z0-9_]+$/.test(key) || /Format$|Type$|Filter$|Mapping$|Side$|Blending$|Depth$|Stencil|Factor$|Equation$|Compare$|Usage$|Ending$|Mode$|Operation$/.test(key)) {
        return `    readonly ${key}: number;`;
    }
    if (key.endsWith('ColorSpace') || key.endsWith('Transfer') || key.endsWith('Encoding') || key === 'GLSL1' || key === 'GLSL3' || key.endsWith('BindMode')) {
        return `    readonly ${key}: string;`;
    }
    if (key === 'DefaultUp' || key === 'MOUSE' || key === 'TOUCH' || key === 'REVISION') {
        return `    readonly ${key}: any;`;
    }
    return `    readonly ${key}: any;`;
});

const namespaceType = `export interface ThreersNamespace {\n${namespaceEntries.join('\n')}\n}\n\ndeclare const THREE: ThreersNamespace;\nexport default THREE;\n`;

const header = `/**
 * TypeScript definitions for threers \`threejs-shim.js\` (three.js-compatible browser API).
 *
 * AUTO-GENERATED — do not edit by hand. Run: \`node web/scripts/generate-shim-types.mjs\`
 *
 * For overlapping APIs, shapes follow three.js r165. Install \`three\` for richer
 * types when porting existing three.js code:
 *   npm install three @types/three
 */
/// <reference types="three" />

export * from './pkg/threers.js';

`;

const out = [header, MANUAL.trim(), stubs, funcStubs, constStubs, namespaceType].filter(Boolean).join('\n\n');
fs.writeFileSync(outPath, out);
console.log(`==> wrote ${outPath} (${exports.length} exports, ${threeKeys.length} THREE keys)`);
