import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

const canvas = document.getElementById('canvas') as HTMLCanvasElement;
const dpr = Math.min(window.devicePixelRatio || 1, 2);
canvas.width = Math.floor(window.innerWidth * dpr);
canvas.height = Math.floor(window.innerHeight * dpr);

const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setSize(canvas.width, canvas.height, false);

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

const camera = new THREE.PerspectiveCamera(60, canvas.width / canvas.height, 0.1, 100);
camera.position.set(2.5, 2, 3.5);

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;

window.addEventListener('resize', () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.floor(window.innerWidth * dpr);
    const h = Math.floor(window.innerHeight * dpr);
    canvas.width = w;
    canvas.height = h;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
});

function animate(): void {
    requestAnimationFrame(animate);
    controls.update();
    renderer.render(scene, camera);
}
animate();
