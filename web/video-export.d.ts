/**
 * Browser video / animated-image export (requires wasm built with `native-codec`).
 *
 * Preferred entry: {@link VideoExporter}.
 */

export type VideoFormatName = 'gif' | 'apng' | 'webm';

/** @deprecated Use {@link VideoFormatName}. */
export type BrowserVideoFormatName = VideoFormatName;

export declare const VideoFormat: {
  readonly Gif: 'gif';
  readonly Apng: 'apng';
  readonly Webm: 'webm';
};

/** @deprecated Use {@link VideoFormat}. */
export declare const BrowserVideoFormat: typeof VideoFormat;

export declare const VideoExportErrorCode: {
  readonly Unavailable: 'Unavailable';
  readonly InvalidFormat: 'InvalidFormat';
  readonly InvalidOption: 'InvalidOption';
  readonly InvalidDimensions: 'InvalidDimensions';
  readonly EmptyFrames: 'EmptyFrames';
  readonly InvalidFrame: 'InvalidFrame';
  readonly FrameSize: 'FrameSize';
  readonly EncodeFailed: 'EncodeFailed';
  readonly Aborted: 'Aborted';
};

/** Event names dispatched by {@link VideoExporter}. */
export declare const VideoExportEvent: {
  readonly Start: 'start';
  readonly Progress: 'progress';
  readonly Capture: 'capture';
  readonly Encode: 'encode';
  readonly Complete: 'complete';
  readonly Error: 'error';
  readonly Abort: 'abort';
};

export type VideoExportPhase = 'capture' | 'encode' | 'done';

export interface VideoExportProgress {
  phase: VideoExportPhase;
  /** 1-based capture progress, or encode cursor. */
  frame: number;
  /** Total frames in this export. */
  frames: number;
  /** 0..1 overall progress estimate. */
  ratio: number;
  format: VideoFormatName;
  /** Ready-to-display status line from {@link formatVideoProgress}. */
  message: string;
}

/** Progress CustomEvent — also exposes `phase` / `frame` / `ratio` / `message`. */
export declare class VideoExportProgressEvent extends CustomEvent<VideoExportProgress> {
  readonly progress: VideoExportProgress;
  readonly phase: VideoExportPhase;
  readonly frame: number;
  readonly frames: number;
  readonly ratio: number;
  readonly format: VideoFormatName;
  readonly message: string;
  constructor(type: string, progress: VideoExportProgress);
}

export interface VideoExportStartDetail {
  phase: 'capture' | 'encode';
  format?: VideoFormatName | string;
  frames?: number;
  concurrency?: number;
}

export interface VideoExportCompleteDetail {
  result: VideoExportResult;
}

export interface VideoExportErrorDetail {
  error: unknown;
}

export interface VideoFormatPresetOptions {
  transparent?: boolean;
  /** GIF only. */
  colors?: number;
  gifColors?: number;
  /** WebM only — alias for `transparent`. */
  alpha?: boolean;
}

export interface VideoEncodeOptions {
  width: number;
  height: number;
  format: VideoFormatName | string;
  /** Frames per second (default 30). */
  fps?: number;
  /** Preserve alpha where the format supports it (default false). */
  transparent?: boolean;
  /** GIF palette size `2..=256` (default 256). */
  gifColors?: number;
  /** Suggested download filename (extension optional). */
  filename?: string;
  /** Used when `filename` is omitted (default `"export"`). */
  basename?: string;
  /**
   * When true, WebM rejects non-multiple-of-8 sizes instead of snapping
   * (scene capture path). Buffer encode always rejects misaligned sizes.
   */
  strictSize?: boolean;
  onProgress?: (info: VideoExportProgress) => void;
  /** Optional EventTarget that also receives progress / lifecycle events. */
  eventTarget?: EventTarget;
  signal?: AbortSignal;
  /** Optional per-frame RGBA transform before encode. */
  mapFrame?: (rgba: Uint8Array, frameIndex: number, frameCount: number) => Uint8Array;
}

export interface VideoDownloadOptions {
  format: VideoFormatName | string;
  filename?: string;
  basename?: string;
  transparent?: boolean;
  width?: number;
  height?: number;
  fps?: number;
  frameCount?: number;
}

/** @deprecated Use {@link VideoEncodeOptions}. */
export type VideoExportOptions = VideoEncodeOptions;

export declare class VideoExportError extends Error {
  code: string;
  hint?: string;
  constructor(message: string, code?: string, options?: { cause?: unknown; hint?: string });
  static is(err: unknown): boolean;
}

export declare class VideoExportResult {
  readonly bytes: Uint8Array;
  readonly format: VideoFormatName;
  readonly width: number;
  readonly height: number;
  readonly fps: number;
  readonly frameCount: number;
  readonly transparent: boolean;
  filename: string;
  readonly mime: string;
  /** Seconds (`frameCount / fps`). */
  readonly duration: number;
  readonly byteLength: number;
  /** e.g. `"320×240"`. */
  readonly sizeLabel: string;
  /** Short human summary for logs / UI status. */
  readonly summary: string;
  toBlob(): Blob;
  toObjectURL(): string;
  revokeObjectURL(): this;
  download(filename?: string): this;
  preview(parent?: ParentNode | null): HTMLImageElement | HTMLVideoElement;
}

export declare class VideoExporter extends EventTarget {
  constructor(options?: Partial<VideoEncodeOptions>);
  /** Chainable `addEventListener`. */
  on(type: string, listener: EventListenerOrEventListenerObject, options?: boolean | AddEventListenerOptions): this;
  /** Chainable `removeEventListener`. */
  off(type: string, listener: EventListenerOrEventListenerObject, options?: boolean | EventListenerOptions): this;
  /** Chainable one-shot listener. */
  once(type: string, listener: EventListenerOrEventListenerObject): this;
  configure(options: Partial<VideoEncodeOptions>): this;
  format(format: VideoFormatName | string): this;
  gif(options?: VideoFormatPresetOptions): this;
  apng(options?: VideoFormatPresetOptions): this;
  webm(options?: VideoFormatPresetOptions): this;
  size(width: number, height: number): this;
  fps(fps: number): this;
  transparent(on?: boolean): this;
  gifColors(n: number): this;
  filename(name: string): this;
  basename(name: string): this;
  strictSize(on?: boolean): this;
  onProgress(cb: (info: VideoExportProgress) => void): this;
  signal(signal: AbortSignal | null | undefined): this;
  timeout(ms: number): this;
  mapFrame(
    fn: (rgba: Uint8Array, frameIndex: number, frameCount: number) => Uint8Array,
  ): this;
  encode(frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>): VideoExportResult;
  encodeAndDownload(
    frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
    filename?: string,
  ): VideoExportResult;
  encodeInWorker(
    frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
    workerOptions?: VideoEncodeWorkerOptions,
  ): Promise<VideoExportResult>;
  static encode(
    frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
    options: VideoEncodeOptions,
  ): VideoExportResult;
  static encodeAndDownload(
    frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
    options: VideoEncodeOptions,
  ): VideoExportResult;
  static encodeInWorker(
    frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
    options: VideoEncodeOptions,
    workerOptions?: VideoEncodeWorkerOptions,
  ): Promise<VideoExportResult>;
}

export interface VideoEncodeWorkerOptions {
  /** Absolute or page-relative URL to `threers_bg.wasm` (default: beside this module). */
  wasmUrl?: string;
  /** URL of `video-export-worker.js` (default: beside this module). */
  workerUrl?: string;
  /** Reuse an existing worker instance instead of the shared pool. */
  worker?: VideoEncodeWorker;
  onProgress?: (info: VideoExportProgress) => void;
  signal?: AbortSignal;
  mapFrame?: (rgba: Uint8Array, frameIndex: number, frameCount: number) => Uint8Array;
  eventTarget?: EventTarget;
}

export declare class VideoEncodeWorker {
  constructor(options?: VideoEncodeWorkerOptions);
  init(): Promise<void>;
  encode(
    frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
    options: VideoEncodeOptions,
  ): Promise<VideoExportResult>;
  terminate(): void;
}

/** True when the loaded wasm exposes native-codec encode bindings. */
export function isVideoExportAvailable(): boolean;

/** Throw a helpful error when native-codec wasm is missing. */
export function assertVideoExportAvailable(): void;

/** Parse UI strings like `"gif"`, `"webm-alpha"`, `"apng.png"`. */
export function parseVideoFormat(input: string | VideoFormatName): {
  format: VideoFormatName;
  transparent: boolean;
};

/** Ready-to-display progress line. */
export function formatVideoProgress(info: VideoExportProgress): string;

/** Fire callback + EventTarget progress events. */
export function emitProgress(
  cb: ((info: VideoExportProgress) => void) | null | undefined,
  info: VideoExportProgress,
  eventTarget?: EventTarget | null,
): VideoExportProgress;

/** Snap width/height down to multiples of 8 (VP9). */
export function alignVideoSize(
  width: number,
  height: number,
): { width: number; height: number; snapped: boolean };

export function formatBytes(n: number): string;

export function videoMimeType(format: VideoFormatName | string): string;

export function videoFilename(
  format: VideoFormatName | string,
  basename?: string,
  transparent?: boolean,
): string;

/** Prefer {@link VideoExporter.encode}; returns raw bytes only. */
export function encodeVideoFrames(
  frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
  options: VideoEncodeOptions,
): Uint8Array;

export function downloadVideoBytes(
  bytes: Uint8Array | ArrayBuffer,
  options: VideoDownloadOptions,
): string;

export function encodeAndDownloadVideoFrames(
  frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
  options: VideoEncodeOptions,
): VideoExportResult;

export function encodeVideoFramesInWorker(
  frames: ArrayLike<Uint8Array | ArrayBuffer | ArrayBufferView>,
  options: VideoEncodeOptions,
  workerOptions?: VideoEncodeWorkerOptions,
): Promise<VideoExportResult>;

export function encodeGifRgba(
  width: number,
  height: number,
  fps: number,
  colors: number,
  transparent: boolean,
  frames: Uint8Array[],
): Uint8Array;

export function encodeApngRgba(
  width: number,
  height: number,
  fps: number,
  transparent: boolean,
  frames: Uint8Array[],
): Uint8Array;

export function encodeWebmRgba(
  width: number,
  height: number,
  fps: number,
  transparent: boolean,
  frames: Uint8Array[],
): Uint8Array;
