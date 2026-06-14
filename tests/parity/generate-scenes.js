#!/usr/bin/env node
// Generate threejs-*.html / threers-*.html pairs from shared scene bodies.
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const scenesDir = path.join(__dirname, 'scenes');

const THREE_IMPORTS = `import * as THREE from 'three';`;
const THREE_POSTFX_IMPORTS = `${THREE_IMPORTS}
import { EffectComposer } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/RenderPass.js';
import { ShaderPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/ShaderPass.js';
import { FXAAShader } from 'https://unpkg.com/three@0.165.0/examples/jsm/shaders/FXAAShader.js';
import { FilmPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/FilmPass.js';
import { DotScreenPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/DotScreenPass.js';
import { HalftonePass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/HalftonePass.js';
import { GlitchPass } from 'https://unpkg.com/three@0.165.0/examples/jsm/postprocessing/GlitchPass.js';`;
const THREE_PARAMETRIC_IMPORT = `${THREE_IMPORTS}
import { ParametricGeometry } from 'https://unpkg.com/three@0.165.0/examples/jsm/geometries/ParametricGeometry.js';`;
const THREE_TUBE_IMPORT = `${THREE_IMPORTS}
import { CatmullRomCurve3 } from 'https://unpkg.com/three@0.165.0/examples/jsm/curves/CatmullRomCurve3.js';
import { TubeGeometry } from 'https://unpkg.com/three@0.165.0/examples/jsm/geometries/TubeGeometry.js';`;

function threeShell(body, { extraImports = '' } = {}) {
    const [setup, postfx = ''] = body.split('// __POSTFX__');
    return `<!DOCTYPE html>
<html><head><meta charset="UTF-8"><style>body{margin:0;background:#101418}canvas{display:block}</style></head>
<body><canvas id="c" width="800" height="600"></canvas>
<script type="importmap">{"imports":{"three":"https://unpkg.com/three@0.165.0/build/three.module.js"}}</script>
<script type="module">
${extraImports || THREE_IMPORTS}
${setup.trim()}
const r = new THREE.WebGLRenderer({ canvas: document.getElementById('c'), antialias: false });
r.setSize(800, 600, false); r.setPixelRatio(1);
${postfx.trim() || 'r.render(scene, cam);'}
document.body.dataset.ready = 'true';
</script></body></html>`;
}

function threersShell(body) {
    const [setup, postfx = ''] = body.split('// __POSTFX__');
    return `<!DOCTYPE html>
<html><head><meta charset="UTF-8"><style>body{margin:0;background:#101418}canvas{display:block}#err{position:absolute;top:0;left:0;padding:12px;color:#fdd;background:#500;font-family:monospace;white-space:pre-wrap}</style></head>
<body><canvas id="c" width="800" height="600"></canvas>
<div id="err" style="display:none"></div>
<script type="module">
import THREE, { initThreers } from '/web/threejs-shim.js';
const errEl = document.getElementById('err');
(async () => {
    try {
        await initThreers('/web/pkg/threers_bg.wasm');
        ${setup.trim()}
        const r = await THREE.WebGLRenderer.create(document.getElementById('c'));
        r.setSize(800, 600);
        ${(postfx.trim() || 'r.render(scene, cam);').replace(/composer\.render\(\)/g, 'await composer.render()')}
        await new Promise(rs => requestAnimationFrame(() => rs()));
        document.body.dataset.ready = 'true';
    } catch (e) {
        console.error(e);
        errEl.style.display = 'block';
        errEl.textContent = e?.stack || String(e);
        document.body.dataset.ready = 'error';
    }
})();
</script></body></html>`;
}

/** @type {Array<{slug:string, threeExtra?:string, threeBody?:string, threersBody?:string, body?:string}>} */
export const GENERATED_SCENES = [
    {
        slug: 'parametric',
        threeExtra: THREE_PARAMETRIC_IMPORT,
        threeBody: `
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.4));
const dl = new THREE.DirectionalLight(0xffffff, 1.0); dl.position.set(2, 3, 4); scene.add(dl);
const func = (u, v, target) => {
    target.set((u - 0.5) * 2, (v - 0.5) * 2, 0.4 * Math.sin(Math.PI * u) * Math.sin(Math.PI * v));
};
const mesh = new THREE.Mesh(new ParametricGeometry(func, 12, 12),
    new THREE.MeshStandardMaterial({ color: 0x77aaff, roughness: 0.45, metalness: 0, side: THREE.DoubleSide }));
mesh.rotation.set(-0.2, 0.3, 0); scene.add(mesh);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 4); cam.lookAt(0, 0, 0);`,
        threersBody: `
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.4));
const dl = new THREE.DirectionalLight(0xffffff, 1.0); dl.position.set(2, 3, 4); scene.add(dl);
const func = (u, v, target) => {
    target.set((u - 0.5) * 2, (v - 0.5) * 2, 0.4 * Math.sin(Math.PI * u) * Math.sin(Math.PI * v));
};
const mesh = new THREE.Mesh(new THREE.ParametricGeometry(func, 12, 12),
    new THREE.MeshStandardMaterial({ color: 0x77aaff, roughness: 0.45, metalness: 0, side: THREE.DoubleSide }));
mesh.rotation.set(-0.2, 0.3, 0); scene.add(mesh);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 4); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'circle',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.5));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(1, 2, 3); scene.add(dl);
scene.add(new THREE.Mesh(new THREE.CircleGeometry(0.85, 48),
    new THREE.MeshStandardMaterial({ color: 0x66aaff, side: THREE.DoubleSide })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'ring',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.5));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(2, 2, 3); scene.add(dl);
scene.add(new THREE.Mesh(new THREE.RingGeometry(0.35, 0.85, 48),
    new THREE.MeshStandardMaterial({ color: 0xff8844, side: THREE.DoubleSide })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'capsule',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.5));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(2, 3, 4); scene.add(dl);
scene.add(new THREE.Mesh(new THREE.CapsuleGeometry(0.35, 0.9, 8, 16),
    new THREE.MeshStandardMaterial({ color: 0x88cc66, roughness: 0.4 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3.5); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'dodecahedron',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.5));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(3, 4, 2); scene.add(dl);
scene.add(new THREE.Mesh(new THREE.DodecahedronGeometry(0.9, 0),
    new THREE.MeshStandardMaterial({ color: 0xaa77ff, flatShading: true })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'fog-exp2',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x88aacc);
scene.fog = new THREE.FogExp2(0x88aacc, 0.12);
for (let i = 0; i < 3; i++) {
    const m = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial({ color: 0xff5544 }));
    m.position.set(0, 0, -i * 3); scene.add(m);
}
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0.6, 0.2, 2); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'doubleside',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.4));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(2, 1, 3); scene.add(dl);
const geo = new THREE.PlaneGeometry(1.4, 1.4);
const plane = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ color: 0x66aaff, side: THREE.DoubleSide }));
plane.rotation.y = Math.PI * 0.35; scene.add(plane);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'backside',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x223344);
scene.add(new THREE.Mesh(new THREE.SphereGeometry(0.9, 24, 16),
    new THREE.MeshBasicMaterial({ color: 0xff6644, side: THREE.BackSide })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 2.5); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'alphatest',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0xffffff);
const tex = new THREE.DataTexture(new Uint8Array([0, 0, 0, 255, 255, 255, 255, 0, 0, 255, 255, 255, 255, 255, 255, 255]), 2, 2);
tex.needsUpdate = true; tex.magFilter = THREE.NearestFilter; tex.minFilter = THREE.NearestFilter;
scene.add(new THREE.Mesh(new THREE.PlaneGeometry(1.6, 1.6),
    new THREE.MeshBasicMaterial({ map: tex, transparent: true, alphaTest: 0.5 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'wireframe-geom',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
const box = new THREE.BoxGeometry(0.9, 0.9, 0.9);
scene.add(new THREE.LineSegments(new THREE.WireframeGeometry(box),
    new THREE.LineBasicMaterial({ color: 0x44aaff })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(1, 1, 2.5); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'tube-curve',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.4));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(2, 3, 4); scene.add(dl);
const curve = new THREE.CatmullRomCurve3([
    new THREE.Vector3(-1, -0.5, 0), new THREE.Vector3(-0.5, 0.5, 0),
    new THREE.Vector3(0.5, -0.5, 0), new THREE.Vector3(1, 0.5, 0),
]);
scene.add(new THREE.Mesh(new THREE.TubeGeometry(curve, 32, 0.15, 8, false),
    new THREE.MeshStandardMaterial({ color: 0xff9966 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3.5); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'extrude-bevel',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.AmbientLight(0xffffff, 0.45));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(2, 3, 4); scene.add(dl);
const shape = new THREE.Shape();
shape.moveTo(-0.5, -0.5); shape.lineTo(0.5, -0.5); shape.lineTo(0.5, 0.5); shape.lineTo(-0.5, 0.5); shape.closePath();
scene.add(new THREE.Mesh(new THREE.ExtrudeGeometry(shape, { depth: 0.35, bevelEnabled: true, bevelSize: 0.08, bevelThickness: 0.08, bevelSegments: 2 }),
    new THREE.MeshStandardMaterial({ color: 0x66ccaa })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(1.2, 1, 2.8); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'render-order',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x222233);
const a = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, 0.8), new THREE.MeshBasicMaterial({ color: 0xff0000, transparent: true, opacity: 0.6 }));
a.position.x = -0.2; a.renderOrder = 1; scene.add(a);
const b = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, 0.8), new THREE.MeshBasicMaterial({ color: 0x0088ff, transparent: true, opacity: 0.6 }));
b.position.x = 0.2; b.renderOrder = 0; scene.add(b);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'axes-grid',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
scene.add(new THREE.GridHelper(4, 8, 0x444466, 0x222233));
scene.add(new THREE.AxesHelper(1.2));
scene.add(new THREE.Mesh(new THREE.BoxGeometry(0.5, 0.5, 0.5), new THREE.MeshBasicMaterial({ color: 0x66aaff })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(2, 2, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'quat-track',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x1c1c22);
scene.add(new THREE.AmbientLight(0xffffff, 0.4));
const dl = new THREE.DirectionalLight(0xffffff, 1); dl.position.set(2, 3, 4); scene.add(dl);
const cube = new THREE.Mesh(new THREE.BoxGeometry(0.7, 0.7, 0.7), new THREE.MeshStandardMaterial({ color: 0x66aaff }));
cube.name = 'Box'; scene.add(cube);
const q0 = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), 0);
const q1 = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), Math.PI / 2);
const track = new THREE.QuaternionKeyframeTrack('Box.quaternion', [0, 1], [...q0.toArray(), ...q1.toArray()]);
const clip = new THREE.AnimationClip('spin', 1, [track]);
const mixer = new THREE.AnimationMixer(scene);
mixer.clipAction(clip).play();
mixer.update(0.5);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'color-track',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101418);
const mat = new THREE.MeshBasicMaterial({ color: 0xff0000 });
const cube = new THREE.Mesh(new THREE.BoxGeometry(0.9, 0.9, 0.9), mat);
cube.name = 'Box'; scene.add(cube);
const track = new THREE.ColorKeyframeTrack('Box.material.color', [0, 1], [1, 0, 0, 0, 0, 1]);
const mixer = new THREE.AnimationMixer(scene);
mixer.clipAction(new THREE.AnimationClip('color', 1, [track])).play();
mixer.update(0.5);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);`,
    },
    {
        slug: 'shadow-mat',
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x888888);
const floor = new THREE.Mesh(new THREE.PlaneGeometry(4, 4), new THREE.ShadowMaterial({ opacity: 0.4 }));
floor.rotation.x = -Math.PI / 2; floor.position.y = -0.6; floor.receiveShadow = true; scene.add(floor);
const cube = new THREE.Mesh(new THREE.BoxGeometry(0.6, 0.6, 0.6), new THREE.MeshStandardMaterial({ color: 0x66aaff }));
cube.position.y = 0; cube.castShadow = true; scene.add(cube);
const dl = new THREE.DirectionalLight(0xffffff, 1.2); dl.position.set(2, 4, 2); dl.castShadow = true;
dl.shadow.camera.left = -3; dl.shadow.camera.right = 3; dl.shadow.camera.top = 3; dl.shadow.camera.bottom = -3;
dl.shadow.mapSize.set(1024, 1024); scene.add(dl);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 1.2, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
if (r.shadowMap) r.shadowMap.enabled = true;
r.render(scene, cam);`,
    },
    {
        slug: 'fxaa',
        threeExtra: THREE_POSTFX_IMPORTS,
        threeBody: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x111111);
const cube = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, 0.8), new THREE.MeshBasicMaterial({ color: 0xffffff }));
cube.rotation.set(0.4, 0.6, 0); scene.add(cube);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new ShaderPass(FXAAShader));
composer.render();`,
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x111111);
const cube = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, 0.8), new THREE.MeshBasicMaterial({ color: 0xffffff }));
cube.rotation.set(0.4, 0.6, 0); scene.add(cube);
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.ShaderPass(THREE.FXAAShader));
composer.render();`,
    },
    {
        slug: 'film',
        threeExtra: THREE_POSTFX_IMPORTS,
        threeBody: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x222222);
scene.add(new THREE.Mesh(new THREE.SphereGeometry(0.7, 24, 16), new THREE.MeshBasicMaterial({ color: 0xff6622 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new FilmPass(0.35, false));
composer.render();`,
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x222222);
scene.add(new THREE.Mesh(new THREE.SphereGeometry(0.7, 24, 16), new THREE.MeshBasicMaterial({ color: 0xff6622 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.FilmPass(0.35, false));
composer.render();`,
    },
    {
        slug: 'dotscreen',
        threeExtra: THREE_POSTFX_IMPORTS,
        threeBody: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x334455);
scene.add(new THREE.Mesh(new THREE.TorusKnotGeometry(0.5, 0.15, 64, 8), new THREE.MeshBasicMaterial({ color: 0xaaccff })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new DotScreenPass(new THREE.Vector2(0.5, 0.5), 1.2, 1.4));
composer.render();`,
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x334455);
scene.add(new THREE.Mesh(new THREE.TorusKnotGeometry(0.5, 0.15, 64, 8), new THREE.MeshBasicMaterial({ color: 0xaaccff })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
composer.addPass(new THREE.DotScreenPass(new THREE.Vector2(0.5, 0.5), 1.2, 1.4));
composer.render();`,
    },
    {
        slug: 'halftone-postfx',
        threeExtra: THREE_POSTFX_IMPORTS,
        threeBody: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0xeeeeee);
scene.add(new THREE.Mesh(new THREE.SphereGeometry(0.75, 24, 16), new THREE.MeshBasicMaterial({ color: 0x333333 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
composer.addPass(new HalftonePass(800, 600, { radius: 6, scatter: 0, shape: 1 }));
composer.render();`,
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0xeeeeee);
scene.add(new THREE.Mesh(new THREE.SphereGeometry(0.75, 24, 16), new THREE.MeshBasicMaterial({ color: 0x333333 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const sceneRT = new THREE.WebGLRenderTarget(800, 600);
r.setRenderTarget(sceneRT); r.render(scene, cam); r.setRenderTarget(null);
new THREE.HalftonePass(800, 600, { radius: 6, scatter: 0, shape: 1 }).apply(r, sceneRT);`,
    },
    {
        slug: 'glitch',
        threeExtra: THREE_POSTFX_IMPORTS,
        threeBody: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101010);
scene.add(new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial({ color: 0x00ff88 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new EffectComposer(r);
composer.addPass(new RenderPass(scene, cam));
(function(s){let st=s>>>0;Math.random=()=>{st=(Math.imul(st,1664525)+1013904223)>>>0;return st/4294967296;};})(42);
const glitchPass = new GlitchPass();
glitchPass.material.fragmentShader = glitchPass.material.fragmentShader.replace(
    /\\/\\/add noise[\\s\\S]*?gl_FragColor = gl_FragColor\\+ snow;/,
    '// snow omitted for parity (GLSL sin noise)',
);
glitchPass.material.needsUpdate = true;
composer.addPass(glitchPass);
composer.render();`,
        body: `
const scene = new THREE.Scene(); scene.background = new THREE.Color(0x101010);
scene.add(new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial({ color: 0x00ff88 })));
const cam = new THREE.PerspectiveCamera(45, 800/600, 0.1, 100);
cam.position.set(0, 0, 3); cam.lookAt(0, 0, 0);
// __POSTFX__
const composer = new THREE.EffectComposer(r);
composer.addPass(new THREE.RenderPass(scene, cam));
THREE.seedRandom(42);
const glitchPass = new THREE.GlitchPass();
glitchPass._skipSnow = true;
composer.addPass(glitchPass);
composer.render();`,
    },
];

function buildThree(def) {
    const body = def.threeBody || def.body || '';
    return threeShell(body, { extraImports: def.threeExtra });
}

function buildThreers(def) {
    const body = def.threersBody || def.body || '';
    return threersShell(body);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
    import('./coverage-manifest.js').then(async ({ ALL_SCENES, SCENE_API_MAP, APPROXIMATE_SCENES }) => {
        fs.writeFileSync(
            path.join(__dirname, 'scenes-manifest.json'),
            JSON.stringify({ scenes: ALL_SCENES, apiMap: SCENE_API_MAP, approximate: APPROXIMATE_SCENES }, null, 2),
        );
        for (const def of GENERATED_SCENES) {
            fs.writeFileSync(path.join(scenesDir, `threejs-${def.slug}.html`), buildThree(def));
            fs.writeFileSync(path.join(scenesDir, `threers-${def.slug}.html`), buildThreers(def));
            console.log('wrote', def.slug);
        }
        await import('./generate-rust-scenes.js');
    });
}
