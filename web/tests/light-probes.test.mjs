import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeGltfAccessor } from '../gltf-accessor.js';
import { LightProbeGrid } from '../light-probe-grid.js';
import {
    evaluateIrradiance,
    halfToFloat,
    interpolateProbeCoefficient,
    projectCubeToSH,
} from '../light-probe-math.js';

function floatToHalf(value) {
    const bits = new Uint32Array(new Float32Array([value]).buffer)[0];
    const sign = (bits >>> 16) & 0x8000;
    let exponent = ((bits >>> 23) & 0xff) - 127 + 15;
    let fraction = bits & 0x7fffff;
    if (exponent <= 0) return sign;
    if (exponent >= 31) return sign | 0x7c00;
    fraction = (fraction + 0x1000) >>> 13;
    if (fraction === 0x400) { fraction = 0; exponent++; }
    return sign | (exponent << 10) | fraction;
}

function constantCube(size, rgb) {
    const bytes = new Uint8Array(6 * size * size * 8);
    for (let pixel = 0; pixel < 6 * size * size; pixel++) {
        for (let channel = 0; channel < 4; channel++) {
            const half = floatToHalf(channel < 3 ? rgb[channel] : 1);
            bytes[pixel * 8 + channel * 2] = half & 0xff;
            bytes[pixel * 8 + channel * 2 + 1] = half >>> 8;
        }
    }
    return bytes;
}

test('half-float decoding covers normal, subnormal, infinity and sign', () => {
    assert.equal(halfToFloat(0x3c00), 1);
    assert.equal(halfToFloat(0xc000), -2);
    assert.equal(halfToFloat(0x0001), 2 ** -24);
    assert.equal(halfToFloat(0x7c00), Number.POSITIVE_INFINITY);
    assert.ok(Number.isNaN(halfToFloat(0x7e00)));
});

test('constant cube projects to constant pi irradiance', () => {
    const coefficients = projectCubeToSH(constantCube(16, [1, 0.5, 0.25]), 16);
    const irradiance = evaluateIrradiance(coefficients, [0, 1, 0]);
    assert.ok(Math.abs(irradiance[0] - Math.PI) < 0.025, `${irradiance[0]} != pi`);
    assert.ok(Math.abs(irradiance[1] - Math.PI * 0.5) < 0.025);
    assert.ok(Math.abs(irradiance[2] - Math.PI * 0.25) < 0.025);
    for (let coefficient = 1; coefficient < 9; coefficient++) {
        assert.ok(Math.abs(coefficients[coefficient * 4]) < 0.015);
    }
});

test('probe interpolation is position dependent and clamps outside bounds', () => {
    const coefficients = new Float32Array(2 * 2 * 2 * 9 * 4);
    for (let probe = 0; probe < 8; probe++) coefficients[probe * 36] = probe;
    assert.deepEqual(interpolateProbeCoefficient(coefficients, [2, 2, 2], [0, 0, 0], [1, 1, 1], [0, 0, 0]), [0, 0, 0]);
    assert.deepEqual(interpolateProbeCoefficient(coefficients, [2, 2, 2], [0, 0, 0], [1, 1, 1], [1, 1, 1]), [7, 0, 0]);
    assert.deepEqual(interpolateProbeCoefficient(coefficients, [2, 2, 2], [0, 0, 0], [1, 1, 1], [0.5, 0.5, 0.5]), [3.5, 0, 0]);
    assert.deepEqual(interpolateProbeCoefficient(coefficients, [2, 2, 2], [0, 0, 0], [1, 1, 1], [-5, 9, 2]), [6, 0, 0]);
});

test('glTF byte-strided accessors decode interleaved Sponza-style data', () => {
    const buffer = new ArrayBuffer(48);
    const view = new DataView(buffer);
    const values = [[1, 2, 3], [4, 5, 6]];
    for (let vertex = 0; vertex < 2; vertex++) {
        for (let component = 0; component < 3; component++) {
            view.setFloat32(vertex * 24 + component * 4, values[vertex][component], true);
        }
        view.setFloat32(vertex * 24 + 12, 99, true);
    }
    const decoded = decodeGltfAccessor(
        { componentType: 5126, type: 'VEC3', count: 2, byteOffset: 0 },
        { buffer, byteOffset: 0, byteStride: 24 },
    );
    assert.deepEqual([...decoded.array], [1, 2, 3, 4, 5, 6]);
});

test('light-probe cache helpers preserve coefficients and bake metadata', () => {
    const coefficients = new Float32Array(2 * 2 * 2 * 9 * 4);
    for (let i = 0; i < coefficients.length; i++) coefficients[i] = i / 17;
    const settings = { cacheKey: 'unit-grid-v1', cubemapSize: 16, near: 0.05, far: 12, bounces: 1 };
    let encodedMetadata;
    let uploaded;
    const renderer = {
        encodeLightProbeGridCache(values, resolution, min, max, bakeSettings, cacheKey) {
            encodedMetadata = { values, resolution, min, max, bakeSettings, cacheKey };
            return new Uint8Array([1, 2, 3, 4]);
        },
        decodeLightProbeGridCache(bytes, cacheKey) {
            assert.deepEqual([...bytes], [1, 2, 3, 4]);
            assert.equal(cacheKey, settings.cacheKey);
            return {
                coefficients,
                resolution: encodedMetadata.resolution,
                min: encodedMetadata.min,
                max: encodedMetadata.max,
                settings: encodedMetadata.bakeSettings,
            };
        },
        setLightProbeGrid(values, resolution, min, max) {
            uploaded = { values, resolution, min, max };
        },
        clearLightProbeGrid() {},
    };

    const baked = new LightProbeGrid(2, 2, 2, [2, 2, 2]).setCoefficients(coefficients);
    const bytes = baked.toCacheBytes(renderer, settings);
    assert.deepEqual([...bytes], [1, 2, 3, 4]);
    assert.equal(encodedMetadata.cacheKey, settings.cacheKey);

    const loaded = new LightProbeGrid(2, 2, 2, [2, 2, 2]);
    loaded.loadCacheBytes(renderer, bytes, settings);
    assert.deepEqual([...loaded.coefficients], [...coefficients]);
    assert.equal(uploaded.values, loaded.coefficients);
});

test('invalid bundled probe data falls back to a live bake', async () => {
    const grid = new LightProbeGrid(2, 2, 2, [2, 2, 2]);
    const coefficients = new Float32Array(2 * 2 * 2 * 9 * 4);
    let bakeCalls = 0;
    grid.bake = async () => {
        bakeCalls++;
        grid.setCoefficients(coefficients);
        return grid;
    };
    const renderer = {
        decodeLightProbeGridCache() {
            throw new Error('checksum mismatch');
        },
    };
    const warnings = [];
    const previousWarn = console.warn;
    console.warn = (...args) => warnings.push(args);
    try {
        await grid.bakeCached(renderer, {}, {
            cacheKey: 'invalid-asset-v1',
            cacheUrl: 'data:application/octet-stream;base64,AQIDBA==',
            useIndexedDB: false,
            cubemapSize: 8,
            near: 0.1,
            far: 10,
            bounces: 0,
        });
    } finally {
        console.warn = previousWarn;
    }
    assert.equal(bakeCalls, 1);
    assert.equal(warnings.length, 1);
    assert.equal(grid.cacheSource, 'baked');
    assert.equal(grid.coefficients, coefficients);
});
