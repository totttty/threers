#!/usr/bin/env node
// Generate tests/parity/scenes/rust/{slug}.rs from threers-{slug}.html pairs.

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { threersHtmlToRust } from './js-to-rust-scene.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const scenesDir = path.join(__dirname, 'scenes');
const rustDir = path.join(scenesDir, 'rust');

function loadManifestSlugs() {
    const slugs = new Set();
    const main = JSON.parse(fs.readFileSync(path.join(__dirname, 'scenes-manifest.json'), 'utf8'));
    for (const s of main.scenes) slugs.add(s);
    for (const name of ['scenes-manifest-mesh-bvh.json', 'scenes-manifest-bvh-csg.json']) {
        const p = path.join(__dirname, name);
        if (!fs.existsSync(p)) continue;
        const part = JSON.parse(fs.readFileSync(p, 'utf8'));
        for (const s of part.scenes || []) slugs.add(s);
    }
    return [...slugs];
}

const NATIVE_RUST_SCENES = new Set(['bvh-csg-hierarchy']);

function featureStub(slug, feature) {
    if (NATIVE_RUST_SCENES.has(slug)) return null;
    if (feature === 'bvh-csg') {
        return `//! Parity scene \`${slug}\` — CSG is JavaScript, not native Rust.
//!
//! Boolean evaluation: web/csg/ (three-bvh-csg@0.0.16 port)
//! Scene: scenes/threers-${slug}.html
`;
    }
    if (feature === 'mesh-bvh') {
        return `//! Parity scene \`${slug}\` — BVH in wasm; scene setup in JS.
//! Scene: scenes/threers-${slug}.html
`;
    }
    return null;
}

function featureForSlug(slug) {
    for (const name of ['scenes-manifest-mesh-bvh.json', 'scenes-manifest-bvh-csg.json']) {
        const p = path.join(__dirname, name);
        if (!fs.existsSync(p)) continue;
        const part = JSON.parse(fs.readFileSync(p, 'utf8'));
        if ((part.scenes || []).includes(slug)) return part.feature || 'mesh-bvh';
    }
    return null;
}

const manifest = { scenes: loadManifestSlugs() };

fs.mkdirSync(rustDir, { recursive: true });

let wrote = 0;
let stubbed = 0;
let missing = 0;

for (const slug of manifest.scenes) {
    const htmlPath = path.join(scenesDir, `threers-${slug}.html`);
    const rustPath = path.join(rustDir, `${slug}.rs`);
    const feat = featureForSlug(slug);
    if (NATIVE_RUST_SCENES.has(slug) && fs.existsSync(rustPath)) {
        console.log('keep native:', slug);
        continue;
    }
    const stub = feat ? featureStub(slug, feat) : null;
    if (stub) {
        fs.writeFileSync(path.join(rustDir, `${slug}.rs`), stub);
        stubbed += 1;
        continue;
    }
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

console.log(`wrote ${wrote} rust scene snippets, ${stubbed} feature stubs (${missing} missing threers html)`);
