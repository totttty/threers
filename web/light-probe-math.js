const SH_BASIS_COUNT = 9;

export function halfToFloat(value) {
    const sign = (value & 0x8000) ? -1 : 1;
    const exponent = (value >>> 10) & 0x1f;
    const fraction = value & 0x03ff;
    if (exponent === 0) return sign * fraction * 2 ** -24;
    if (exponent === 0x1f) return fraction ? Number.NaN : sign * Number.POSITIVE_INFINITY;
    return sign * (1 + fraction / 1024) * 2 ** (exponent - 15);
}

export function cubeTexelDirection(face, x, y, size) {
    const column = (2 * (x + 0.5) / size) - 1;
    const row = 1 - (2 * (y + 0.5) / size);
    let dx, dy, dz;
    switch (face) {
        case 0: [dx, dy, dz] = [1, row, -column]; break;
        case 1: [dx, dy, dz] = [-1, row, column]; break;
        case 2: [dx, dy, dz] = [column, 1, -row]; break;
        case 3: [dx, dy, dz] = [column, -1, row]; break;
        case 4: [dx, dy, dz] = [column, row, 1]; break;
        case 5: [dx, dy, dz] = [-column, row, -1]; break;
        default: throw new RangeError(`Cube face must be 0..5, got ${face}`);
    }
    const inverseLength = 1 / Math.hypot(dx, dy, dz);
    return [dx * inverseLength, dy * inverseLength, dz * inverseLength];
}

function shBasis(x, y, z) {
    return [
        0.282095,
        0.488603 * y,
        0.488603 * z,
        0.488603 * x,
        1.092548 * x * y,
        1.092548 * y * z,
        0.315392 * (3 * z * z - 1),
        1.092548 * x * z,
        0.546274 * (x * x - y * y),
    ];
}

function halfChannel(bytes, offset) {
    return halfToFloat(bytes[offset] | (bytes[offset + 1] << 8));
}

/** Project six layer-major RGBA16F cube faces into nine RGB SH coefficients. */
export function projectCubeToSH(bytes, size) {
    const expected = 6 * size * size * 8;
    if (!(bytes instanceof Uint8Array) || bytes.byteLength !== expected) {
        throw new Error(`Expected ${expected} RGBA16F bytes, got ${bytes?.byteLength ?? 'none'}`);
    }
    const coefficients = new Float32Array(SH_BASIS_COUNT * 4);
    let totalWeight = 0;
    for (let face = 0; face < 6; face++) {
        for (let y = 0; y < size; y++) {
            for (let x = 0; x < size; x++) {
                const column = (2 * (x + 0.5) / size) - 1;
                const row = 1 - (2 * (y + 0.5) / size);
                const lengthSquared = column * column + row * row + 1;
                const weight = 4 / (Math.sqrt(lengthSquared) * lengthSquared);
                const [nx, ny, nz] = cubeTexelDirection(face, x, y, size);
                const basis = shBasis(nx, ny, nz);
                const pixel = ((face * size * size) + (y * size + x)) * 8;
                const r = halfChannel(bytes, pixel);
                const g = halfChannel(bytes, pixel + 2);
                const b = halfChannel(bytes, pixel + 4);
                for (let coefficient = 0; coefficient < SH_BASIS_COUNT; coefficient++) {
                    const scale = basis[coefficient] * weight;
                    const offset = coefficient * 4;
                    coefficients[offset] += r * scale;
                    coefficients[offset + 1] += g * scale;
                    coefficients[offset + 2] += b * scale;
                }
                totalWeight += weight;
            }
        }
    }
    const normalization = (4 * Math.PI) / totalWeight;
    for (let i = 0; i < coefficients.length; i++) coefficients[i] *= normalization;
    return coefficients;
}

/** Evaluate diffuse irradiance from radiance SH, using Three.js's convolution constants. */
export function evaluateIrradiance(coefficients, normal) {
    const [x, y, z] = normal;
    const weights = [
        0.886227,
        2 * 0.511664 * y,
        2 * 0.511664 * z,
        2 * 0.511664 * x,
        2 * 0.429043 * x * y,
        2 * 0.429043 * y * z,
        0.743125 * z * z - 0.247708,
        2 * 0.429043 * x * z,
        0.429043 * (x * x - y * y),
    ];
    const result = [0, 0, 0];
    for (let coefficient = 0; coefficient < SH_BASIS_COUNT; coefficient++) {
        const offset = coefficient * 4;
        for (let channel = 0; channel < 3; channel++) {
            result[channel] += coefficients[offset + channel] * weights[coefficient];
        }
    }
    return result;
}

/** Position-dependent coefficient interpolation matching the renderer's clamped grid lookup. */
export function interpolateProbeCoefficient(coefficients, resolution, min, max, position, coefficient = 0) {
    const [nx, ny, nz] = resolution;
    const grid = position.map((value, axis) => {
        const extent = max[axis] - min[axis];
        const cells = resolution[axis] - 1;
        return Math.max(0, Math.min(cells, ((value - min[axis]) / extent) * cells));
    });
    const base = grid.map((value, axis) => Math.min(Math.floor(value), resolution[axis] - 2));
    const fraction = grid.map((value, axis) => value - base[axis]);
    const out = [0, 0, 0];
    for (let z = 0; z <= 1; z++) for (let y = 0; y <= 1; y++) for (let x = 0; x <= 1; x++) {
        const weight = (x ? fraction[0] : 1 - fraction[0])
            * (y ? fraction[1] : 1 - fraction[1])
            * (z ? fraction[2] : 1 - fraction[2]);
        const probe = (base[0] + x) + nx * ((base[1] + y) + ny * (base[2] + z));
        const offset = (probe * SH_BASIS_COUNT + coefficient) * 4;
        for (let channel = 0; channel < 3; channel++) out[channel] += coefficients[offset + channel] * weight;
    }
    return out;
}
