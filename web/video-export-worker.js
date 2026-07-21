/**
 * Web Worker that loads threers wasm (`native-codec`) and encodes GIF/APNG/WebM.
 *
 * Main thread captures frames (optionally pipelined); this worker encodes so the
 * UI stays responsive.
 *
 * Messages in:
 *   { id, type: 'init', wasmUrl }
 *   { id, type: 'encode', wasmUrl?, format, width, height, fps, transparent, gifColors, frames: ArrayBuffer[] }
 *
 * Messages out:
 *   { id, ok: true }
 *   { id, ok: true, bytes: ArrayBuffer, mime, format }
 *   { id, ok: false, error: string }
 */

import init, {
  encodeGifRgba,
  encodeApngRgba,
  encodeWebmRgba,
} from './pkg/threers.js';

let initPromise = null;
let wasmUrlCached = null;

function ensureInit(wasmUrl) {
  const url = wasmUrl || wasmUrlCached;
  if (!url) {
    return Promise.reject(new Error('video-export-worker: wasmUrl required on first message'));
  }
  if (!initPromise || wasmUrlCached !== url) {
    wasmUrlCached = url;
    initPromise = init({ module_or_path: url });
  }
  return initPromise;
}

function asU8(bytes) {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}

self.onmessage = async (event) => {
  const msg = event.data || {};
  const { id, type } = msg;
  try {
    if (type === 'init') {
      await ensureInit(msg.wasmUrl);
      if (typeof encodeGifRgba !== 'function') {
        throw new Error('native-codec bindings missing — rebuild with NATIVE_CODEC=1');
      }
      self.postMessage({ id, ok: true, type: 'init' });
      return;
    }

    if (type === 'encode') {
      await ensureInit(msg.wasmUrl);
      const width = msg.width | 0;
      const height = msg.height | 0;
      const fps = Math.max(1, msg.fps | 0 || 30);
      const transparent = !!msg.transparent;
      const gifColors = Math.min(256, Math.max(2, msg.gifColors | 0 || 256));
      const format = String(msg.format || 'gif').toLowerCase();
      const frames = (msg.frames || []).map((buf) => {
        if (buf instanceof Uint8Array) return buf;
        if (buf instanceof ArrayBuffer) return new Uint8Array(buf);
        if (ArrayBuffer.isView(buf)) {
          return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
        }
        throw new Error('frame must be ArrayBuffer or TypedArray');
      });

      let encoded;
      if (format === 'gif') {
        encoded = encodeGifRgba(width, height, fps, gifColors, transparent, frames);
      } else if (format === 'apng') {
        encoded = encodeApngRgba(width, height, fps, transparent, frames);
      } else if (format === 'webm') {
        encoded = encodeWebmRgba(width, height, fps, transparent, frames);
      } else {
        throw new Error(`unsupported format: ${format}`);
      }

      const u8 = asU8(encoded);
      const copy = u8.buffer.slice(u8.byteOffset, u8.byteOffset + u8.byteLength);
      const mime = format === 'gif' ? 'image/gif' : format === 'apng' ? 'image/png' : 'video/webm';
      self.postMessage({ id, ok: true, type: 'encode', bytes: copy, mime, format }, [copy]);
      return;
    }

    throw new Error(`unknown worker message type: ${type}`);
  } catch (err) {
    self.postMessage({
      id,
      ok: false,
      type: type || 'error',
      error: String(err?.message || err),
    });
  }
};
