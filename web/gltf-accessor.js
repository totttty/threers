const COMPONENT_INFO = {
    5120: { bytes: 1, array: Int8Array, get: 'getInt8', signed: true },
    5121: { bytes: 1, array: Uint8Array, get: 'getUint8' },
    5122: { bytes: 2, array: Int16Array, get: 'getInt16', signed: true },
    5123: { bytes: 2, array: Uint16Array, get: 'getUint16' },
    5125: { bytes: 4, array: Uint32Array, get: 'getUint32' },
    5126: { bytes: 4, array: Float32Array, get: 'getFloat32', float: true },
};

export const GLTF_TYPE_SIZES = {
    SCALAR: 1,
    VEC2: 2,
    VEC3: 3,
    VEC4: 4,
    MAT2: 4,
    MAT3: 9,
    MAT4: 16,
};

function normalizedValue(value, componentType) {
    switch (componentType) {
        case 5120: return Math.max(value / 127, -1);
        case 5121: return value / 255;
        case 5122: return Math.max(value / 32767, -1);
        case 5123: return value / 65535;
        case 5125: return value / 4294967295;
        default: return value;
    }
}

/** Decode a glTF accessor, including byte-strided/interleaved buffer views. */
export function decodeGltfAccessor(accessor, bufferView) {
    const info = COMPONENT_INFO[accessor.componentType];
    if (!info) throw new Error(`Unsupported glTF component type ${accessor.componentType}`);
    const itemSize = GLTF_TYPE_SIZES[accessor.type];
    if (!itemSize) throw new Error(`Unsupported glTF accessor type ${accessor.type}`);

    const count = accessor.count >>> 0;
    const elementBytes = itemSize * info.bytes;
    const byteStride = bufferView?.byteStride || elementBytes;
    if (byteStride < elementBytes) {
        throw new Error(`glTF accessor stride ${byteStride} is smaller than ${elementBytes}`);
    }

    const output = accessor.normalized
        ? new Float32Array(count * itemSize)
        : new info.array(count * itemSize);
    if (!bufferView) return { array: output, itemSize, count };

    const start = (bufferView.byteOffset || 0) + (accessor.byteOffset || 0);
    const required = count === 0 ? 0 : (count - 1) * byteStride + elementBytes;
    if (start < 0 || start + required > bufferView.buffer.byteLength) {
        throw new Error('glTF accessor exceeds its buffer');
    }

    const view = new DataView(bufferView.buffer);
    for (let i = 0; i < count; i++) {
        const elementStart = start + i * byteStride;
        for (let component = 0; component < itemSize; component++) {
            const offset = elementStart + component * info.bytes;
            const value = info.bytes === 1
                ? view[info.get](offset)
                : view[info.get](offset, true);
            output[i * itemSize + component] = accessor.normalized
                ? normalizedValue(value, accessor.componentType)
                : value;
        }
    }
    return { array: output, itemSize, count };
}
