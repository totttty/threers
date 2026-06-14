#!/usr/bin/env node
// Generate tests/parity/scenes/rust/{slug}.rs from threers-{slug}.html pairs.

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { threersHtmlToRust } from './js-to-rust-scene.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const scenesDir = path.join(__dirname, 'scenes');
const rustDir = path.join(scenesDir, 'rust');

const manifest = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'scenes-manifest.json'), 'utf8'),
);

fs.mkdirSync(rustDir, { recursive: true });

let wrote = 0;
let missing = 0;

for (const slug of manifest.scenes) {
    const htmlPath = path.join(scenesDir, `threers-${slug}.html`);
    if (!fs.existsSync(htmlPath)) {
        console.warn('skip (no threers html):', slug);
        missing += 1;
        continue;
    }
    const html = fs.readFileSync(htmlPath, 'utf8');
    const rust = threersHtmlToRust(html, slug);
    fs.writeFileSync(path.join(rustDir, `${slug}.rs`), rust);
    wrote += 1;
}

console.log(`wrote ${wrote} rust scene snippets (${missing} missing threers html)`);
