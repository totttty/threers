export function halfToFloat(value: number): number;
export function cubeTexelDirection(face: number, x: number, y: number, size: number): [number, number, number];
export function projectCubeToSH(bytes: Uint8Array, size: number): Float32Array;
export function evaluateIrradiance(coefficients: ArrayLike<number>, normal: [number, number, number]): [number, number, number];
export function interpolateProbeCoefficient(
    coefficients: ArrayLike<number>,
    resolution: [number, number, number],
    min: [number, number, number],
    max: [number, number, number],
    position: [number, number, number],
    coefficient?: number,
): [number, number, number];
