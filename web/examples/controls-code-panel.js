/** Load tabbed source files into a code panel. */
export async function initCodePanel(root, sources) {
    const tabsEl = root.querySelector('.tabs');
    const preEl = root.querySelector('pre');
    const loadingEl = root.querySelector('.loading');

    const cache = new Map();

    async function load(key) {
        if (cache.has(key)) return cache.get(key);
        const entry = sources.find((s) => s.id === key);
        if (!entry) return '';
        const text = await fetch(entry.url).then((r) => {
            if (!r.ok) throw new Error(`Failed to load ${entry.url}`);
            return r.text();
        });
        cache.set(key, text);
        return text;
    }

    async function show(key) {
        tabsEl.querySelectorAll('.tab').forEach((btn) => {
            btn.classList.toggle('active', btn.dataset.tab === key);
        });
        if (loadingEl) loadingEl.hidden = false;
        preEl.textContent = '';
        try {
            preEl.textContent = await load(key);
        } catch (e) {
            preEl.textContent = String(e);
        }
        if (loadingEl) loadingEl.hidden = true;
    }

    for (const src of sources) {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'tab' + (src.id === sources[0].id ? ' active' : '');
        btn.dataset.tab = src.id;
        btn.textContent = src.label;
        btn.addEventListener('click', () => show(src.id));
        tabsEl.appendChild(btn);
    }

    await show(sources[0].id);
}
