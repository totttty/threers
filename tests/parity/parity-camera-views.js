/** Camera poses for multi-view pixel parity (position + lookAt target). */

const DEFAULT_TARGET = [0, 0, 0];

function view(name, pos, target = DEFAULT_TARGET) {
    return { name, pos: [...pos], target: [...target] };
}

function spherical(radius, elevDeg, azimDeg, target = DEFAULT_TARGET) {
    const elev = (elevDeg * Math.PI) / 180;
    const azim = (azimDeg * Math.PI) / 180;
    const sinE = Math.sin(elev);
    return view(
        `orb-${elevDeg}-${azimDeg}`,
        [
            target[0] + radius * sinE * Math.sin(azim),
            target[1] + radius * Math.cos(elev),
            target[2] + radius * sinE * Math.cos(azim),
        ],
        target,
    );
}

/** Views every bvh-csg scene is compared at (worst diff wins). */
const BVH_CSG_VIEWS = [
    view('front', [0, 0, 5]),
    view('back', [0, 0, -5]),
    view('right', [5, 0, 0]),
    view('left', [-5, 0, 0]),
    spherical(5, 52, 38),
    spherical(5, 38, 142),
    spherical(5, 28, 218),
];

const SCENE_EXTRA_VIEWS = {
    'bvh-csg-hierarchy': [
        view('slab', [-3.8, 2.9, 1.25]),
        view('slab-low', [-4.5, 1.8, 0.8]),
    ],
    'bvh-csg-hollow': [
        spherical(4.5, 45, 60),
    ],
    'bvh-csg-complex': [
        spherical(6, 48, 25),
    ],
};

export function viewsForScene(slug) {
    const extra = SCENE_EXTRA_VIEWS[slug] || [];
    const seen = new Set();
    const out = [];
    for (const v of [...BVH_CSG_VIEWS, ...extra]) {
        const key = `${v.pos.join(',')}|${v.target.join(',')}`;
        if (seen.has(key)) continue;
        seen.add(key);
        out.push(v);
    }
    return out;
}
