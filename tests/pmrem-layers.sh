#!/usr/bin/env bash
# PMREM pipeline layer runner — verify each stage before full parity.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== PMREM layers L01–L07 (CPU: geometry → textures → pack → blur → sampling) ==="
cargo test pmrem_layer_0 -- --nocapture

echo ""
echo "=== PMREM layer L08 (GPU texture upload) ==="
cargo test gpu_cube_uv_atlas_upload_matches_cpu -- --nocapture

echo ""
echo "=== PMREM layer L10 (GPU shader sampling) ==="
cargo test gpu_cube_uv_shader_sample_matches_cpu -- --nocapture

echo ""
echo "=== PMREM layers L11–L13 (IBL + grazing + row convention) ==="
cargo test pmrem_layer_11_ibl_front_sphere_diagnostic -- --nocapture
cargo test pmrem_layer_12_grazing_env_probes -- --nocapture
cargo test pmrem_layer_13_grazing_plus_z_not_wrong_face -- --nocapture

echo ""
echo "=== PMREM blur/L09 diagnostics (always print gap) ==="
cargo test pmrem_layer_05_mip1_green_diagnostic -- --nocapture
cargo test pmrem_layer_05_first_blur_column_parity_size32 -- --nocapture
cargo test pmrem_layer_09_report_current_probe_gap -- --nocapture
cargo test pmrem_layer_09_per_mip_plus_z_diagnostic -- --nocapture

echo ""
echo "=== PMREM gates L05 + L09 (three.js atlas probes) ==="
cargo test pmrem_layer_05_mip1_plus_z_not_neon_green -- --nocapture
cargo test pmrem_layer_05_first_blur_matches_threejs_column40 -- --nocapture
cargo test pmrem_layer_09_threejs_atlas_probes -- --nocapture
echo "L05 + L09 gates PASSED"

echo ""
echo "=== PMREM scene parity (final gate) ==="
(cd tests/parity && node run.js pmrem)
echo "PMREM pipeline complete (layers + scene parity)"
