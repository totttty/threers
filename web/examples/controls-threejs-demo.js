import * as THREE from 'three';
import { OrbitControls } from 'https://unpkg.com/three@0.165.0/examples/jsm/controls/OrbitControls.js';

const canvas = document.getElementById('canvas');
const preview = canvas.parentElement;

function resizeCanvas() {
    const dpr = Math.min(devicePixelRatio || 1, 2);
    const w = Math.floor(preview.clientWidth * dpr);
    const h = Math.floor(preview.clientHeight * dpr);
    canvas.width = w;
    canvas.height = h;
    return { w, h, dpr };
}

let { w, h } = resizeCanvas();

const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setSize(w, h, false);

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x202030);
scene.add(new THREE.AmbientLight(0xffffff, 0.35));
const key = new THREE.DirectionalLight(0xffffff, 1);
key.position.set(3, 5, 2);
scene.add(key);

scene.add(new THREE.Mesh(
    new THREE.BoxGeometry(1, 1, 1),
    new THREE.MeshStandardMaterial({ color: 0xff6633, roughness: 0.4, metalness: 0.1 }),
));

const camera = new THREE.PerspectiveCamera(60, w / h, 0.1, 100);
camera.position.set(2.5, 2, 3.5);

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;

addEventListener('resize', () => {
    ({ w, h } = resizeCanvas());
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
});

function animate() {
    requestAnimationFrame(animate);
    controls.update();
    renderer.render(scene, camera);
}
animate();
