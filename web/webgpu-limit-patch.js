/** Strip wgpu limits that browsers no longer recognize (must run before wasm init). */
(function () {
    if (globalThis.__threersWebGpuPatched) return;
    const STRIP = new Set(['maxInterStageShaderComponents']);
    function filterLimits(requiredLimits) {
        if (!requiredLimits) return requiredLimits;
        const out = {};
        for (const [k, v] of Object.entries(requiredLimits)) {
            if (!STRIP.has(k)) out[k] = v;
        }
        return Object.keys(out).length ? out : undefined;
    }
    function sanitizeDescriptor(descriptor) {
        if (!descriptor) return descriptor;
        const desc = { ...descriptor };
        desc.requiredLimits = filterLimits(desc.requiredLimits);
        for (const k of STRIP) delete desc[k];
        return desc;
    }
    function wrapRequestDevice(orig) {
        return function (descriptor) {
            return orig.call(this, sanitizeDescriptor(descriptor));
        };
    }
    function tryPatch() {
        const Adapter = globalThis.GPUAdapter;
        if (!Adapter?.prototype?.requestDevice) return false;
        const orig = Adapter.prototype.requestDevice;
        if (orig.__threersWebGpuWrapped) return true;
        Adapter.prototype.requestDevice = wrapRequestDevice(orig);
        Adapter.prototype.requestDevice.__threersWebGpuWrapped = true;
        globalThis.__threersWebGpuPatched = true;
        return true;
    }
    if (!tryPatch() && globalThis.navigator?.gpu?.requestAdapter) {
        const gpu = globalThis.navigator.gpu;
        const origAdapter = gpu.requestAdapter.bind(gpu);
        gpu.requestAdapter = function (...args) {
            return origAdapter(...args).then((adapter) => {
                if (adapter && !tryPatch()) {
                    const orig = adapter.requestDevice.bind(adapter);
                    adapter.requestDevice = function (descriptor) {
                        return orig(sanitizeDescriptor(descriptor));
                    };
                }
                return adapter;
            });
        };
    }
})();
