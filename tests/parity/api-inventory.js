// Compare threers THREE namespace exports against parity scene coverage.
import path from 'path';
import { fileURLToPath } from 'url';
import THREE from '../../web/threejs-shim.js';
import {
    SCENE_API_MAP, STUB_APIS, PARTIAL_APIS, APPROXIMATE_SCENES, ALL_SCENES,
} from './coverage-manifest.js';

/** All symbols on the default THREE namespace object. */
export function listThreeExports() {
    return Object.keys(THREE).sort();
}

/** APIs referenced by at least one parity scene slug. */
export function apisFromScenes() {
    const covered = new Set();
    for (const apis of Object.values(SCENE_API_MAP)) {
        for (const a of apis) covered.add(a);
    }
    return [...covered].sort();
}

/** Rough classification for reporting. */
export function classifyExport(name) {
    if (STUB_APIS.includes(name)) return 'stub';
    if (PARTIAL_APIS.includes(name)) return 'partial';
    const sceneCovered = apisFromScenes().includes(name);
    if (sceneCovered) return APPROXIMATE_SCENES.some(s => SCENE_API_MAP[s]?.includes(name)) ? 'approximate' : 'tested';
    // Heuristic: geometry/material/light classes without dedicated scenes.
    if (/Geometry$/.test(name) || /Material$/.test(name) || /Light$/.test(name)) return 'implemented';
    if (/Loader$/.test(name) || /Pass$/.test(name) || /Controls$/.test(name)) return 'partial';
    return 'exported';
}

export function apiCoverageReport() {
    const exports = listThreeExports();
    const byStatus = {};
    for (const name of exports) {
        const status = classifyExport(name);
        (byStatus[status] ||= []).push(name);
    }
    const tested = (byStatus.tested?.length ?? 0) + (byStatus.approximate?.length ?? 0);
    const pct = exports.length ? Math.round((tested / exports.length) * 100) : 0;
    return {
        totalExports: exports.length,
        totalScenes: ALL_SCENES.length,
        sceneTestedApis: apisFromScenes().length,
        exportCoveragePct: pct,
        byStatus,
        stubs: STUB_APIS.length,
        partial: PARTIAL_APIS.length,
    };
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
    const r = apiCoverageReport();
    console.log(JSON.stringify(r, null, 2));
}
