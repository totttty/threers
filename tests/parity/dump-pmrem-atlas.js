// Compare PMREM CubeUV atlas samples: three.js vs threers CPU (via wasm).
import puppeteer from 'puppeteer';
import { spawn } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = 8091;
const server = spawn('node', [path.join(__dirname, 'server.js'), String(port)], { stdio: 'inherit' });
await new Promise(r => setTimeout(r, 300));

const browser = await puppeteer.launch({
    executablePath: '/opt/homebrew/bin/chromium',
    headless: 'new',
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-vulkan=swiftshader', '--no-sandbox'],
});

const sampleThree = async () => {
    const page = await browser.newPage();
    await page.goto(`http://localhost:${port}/tests/parity/scenes/threejs-pmrem-dump.html`, { waitUntil: 'load' });
    await page.waitForFunction(() => document.body.dataset.ready === 'true', { timeout: 30000 });
    return page.evaluate(() => JSON.parse(document.body.dataset.samples));
};

const sampleThreers = async () => {
    const page = await browser.newPage();
    await page.goto(`http://localhost:${port}/tests/parity/scenes/threers-pmrem-dump.html`, { waitUntil: 'load' });
    await page.waitForFunction(() => document.body.dataset.ready === 'true', { timeout: 30000 });
    return page.evaluate(() => JSON.parse(document.body.dataset.samples));
};

const three = await sampleThree();
const threers = await sampleThreers();
await browser.close();
server.kill();

function rgb(arr) {
    return arr.map(v => Math.round(v * 255));
}

console.log('Direction      | three.js rgb     | threers rgb      | delta');
for (const key of Object.keys(three)) {
    const a = three[key];
    const b = threers[key] ?? [0, 0, 0];
    const da = rgb(a);
    const db = rgb(b);
    const d = da.map((v, i) => Math.abs(v - db[i]));
    console.log(`${key.padEnd(14)} | ${da.join(',').padEnd(16)} | ${db.join(',').padEnd(16)} | ${d.join(',')}`);
}
