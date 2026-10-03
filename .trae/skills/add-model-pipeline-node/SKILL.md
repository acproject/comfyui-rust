---
name: add-model-pipeline-node
description: Add a comfyui-rust node for a self-contained model repo — directory scan, COMBO, backend routing, tests. Use for MiniMax/Qwen/Boogu-style pipelines, not plain weight-file loaders.
---

# Add a Self-Contained Model Pipeline Node

Use when a new model arrives as a **self-contained directory** under `COMFY_MODELS_DIR`
(own `transformer/` + `vae/` shards + `model_index.json`, e.g. Qwen-Image-2.1,
Boogu-Image-*, MiniMax-H3, Bernini-R) and needs a one-click executor node.

Do NOT use for plain weight files (`.safetensors`/`.gguf`) that existing loaders
already cover (CheckpointLoader / UNETLoader / DiffusionModelLoader + KSampler).

Reference implementations — read the closest one before writing:

| Case | File |
|---|---|
| sd.cpp native via auto rewrite (image) | `crates/comfy-executor/src/qwen.rs` |
| Same, edit variant (reference image) | `crates/comfy-executor/src/boogu.rs` |
| Python-only video/audio pipeline | `crates/comfy-executor/src/bernini.rs`, `minimax.rs` |

## 0. Verify the backend path FIRST (do not write the node first)

1. Check `crates/comfy-inference/src/sdcpp_support.rs` — is the family in the
   hardcoded registry (`id`, kind `Image`/`ImageEdit`, aliases)?
2. Check `crates/comfy-inference/src/python.rs` —
   `FallbackBackend::SDCPP_PIPELINE_FAMILIES` and `rewrite_pipeline_for_sdcpp`.
   If the family is listed, the model directory is automatically rewritten to
   native component paths (transformer shard index + bundled VAE +
   `find_qwen3vl_encoder`). **The node needs zero inference-layer changes.**
3. If neither exists: read `cpp/stable-diffusion.cpp/docs/<model>.md` for
   defaults (steps, CFG, resolution divisor, extra args), then add the family
   to `sdcpp_support.rs` + the rewrite list. Python-only models
   (Bernini/MiniMax-style custom layouts) instead need
   `py/flash_attn_v100/comfy_fallback/<x>_generate.py`, backend trait methods,
   a `*_blocking` runner in `python.rs`, and `FallbackBackend` routing.

Facts that keep recurring (verify, don't assume):

- sd.cpp loads sharded safetensors when `diffusion_model_path` points at the
  `*.safetensors.index.json` (it parses sibling shards automatically).
- Bundled VAE is often model-specific and NOT interchangeable (Qwen 2.1 VAE vs
  old Qwen VAE; Boogu uses the FLUX VAE).
- Qwen3-VL tokenizer is embedded in the encoder safetensors; no `--tokenizer`.
- All variants may share one `_class_name` (Boogu: `BooguImagePipeline` for
  Base/Edit/Turbo/Edit-Turbo) — detect edit/turbo from the **directory name**.
- Resolution divisor differs: Qwen-Image = 32, Boogu/FLUX-VAE = 16.

## 1. Write the node: `crates/comfy-executor/src/<name>.rs`

Copy the structure from `qwen.rs`/`boogu.rs`:

1. **Scanning** — `scan_<name>_dirs() -> Vec<(String, PathBuf)>`:
   - roots: `COMFY_MODELS_DIR` (or `models`), its `diffusion_models/` (or the
     relevant subdir), and `cwd.join(base)`; dedupe with `HashSet`; sort labels.
   - match on directory name AND component layout AND
     `model_index.json` `_class_name` (fallback: presence of
     `transformer/*.safetensors.index.json`). Never trust the name alone.
   - `resolve_<name>_dir(label)` re-runs the scan and finds the label.
2. **CRITICAL Fn-closure gotcha**: the executor closure is `Fn` and must
   **rescan on every invocation** — do NOT `move` a captured `Vec` of choices
   into it. Only the `default_model: String` may be captured.
3. **NodeClassDef**: descriptive `display_name`, category `image/<x>` /
   `video/<x>`; generation nodes set `not_idempotent: true`.
4. **Inputs**: required `prompt` (multiline STRING) + model `COMBO`;
   optional per model docs: `image` (IMAGE, edit reference), `negative_prompt`,
   `seed` (default per variant), `steps`, `cfg` (FLOAT), `width`/`height` with
   the correct step (16/32) and a `quantize_dim` clamp (256..2048).
5. **IMAGE input decoding** — reuse the `resolve_input_images` envelope logic
   (`{path}`, `{images:[…]}`, inline `SdImage`), resolving bare filenames
   under `input/`. Decode with `SdImage::from_png_bytes`.
6. **Backend call**: build params via `ImageGenParams::new(prompt)` builders
   (`.with_cfg_scale`, `.with_sample_method(SampleMethod::Euler)`, …), set
   `ModelConfig::new().with_model(model_dir…)` (the rewrite happens in the
   backend layer), assign `params.ref_images` for edit models, then
   `ctx.backend().generate_image(params).map_err(ExecutorError::Inference)?`.
7. **Output**: return the `{"type":"image","images":[…]}` envelope and declare
   `output_types: vec![IoType::Image]`, `is_output_node: false` (connects to
   SaveImage). File-writing nodes (like Music3) are output nodes themselves.
   Edit-model constraints (e.g. Boogu accepts only ONE reference image) are
   enforced with truncate + `tracing::warn!`.

## 2. Register (no feature gate for these nodes)

- `crates/comfy-executor/src/lib.rs`: `pub mod <name>;`
- `crates/comfy-executor/src/builtin_nodes.rs`:
  `crate::<name>::register_<name>_nodes(registry);` next to the other
  bernini/minimax/qwen/boogu registration.

## 3. Tests in the same file

Always add (mirror `qwen::tests`):

- scan filtering test under a `std::env::temp_dir()` tree: valid repo, valid
  sharded repo without class metadata, unrelated pipeline rejected, name match
  without components rejected — wrap in `crate::TEST_ENV_LOCK.lock()` and set
  `COMFY_MODELS_DIR` via `unsafe { std::env::set_var(...) }`.
- pure helper test (dimension quantization).
- registration test: build the full builtin registry, assert class_type,
  category, required/optional inputs, and (when
  `/home/acproject/usb/comfyui/models` exists) the real model label appears in
  the COMBO choices. Also takes `TEST_ENV_LOCK`.

## 4. Validate end to end

1. `cargo test -p comfy-executor --lib` and `cargo check --workspace --all-targets`.
2. Restart with `./start.sh` (builds `local-ffi` so FallbackBackend assembles
   the native/python backends); confirm `listening on` + `models_dir=`.
3. Via the MCP endpoint (`POST /mcp`, initialize → notifications/initialized):
   `list_nodes(keyword=…)` finds the node; `build_workflow` (add SaveImage /
   LoadImage for edit) then `validate_workflow` must return `valid: true`.
4. GPU smoke: `POST /prompt` with small resolution + few steps. Poll
   `/history/<prompt_id>`. In the server log confirm the architecture line
   from sd.cpp (e.g. `qwen_image_2_1: layers=32`, `BooguImageEditPipeline`)
   and that the component paths rewritten are the bundled VAE /
   `text_encoders/qwen3vl_*`. Models on the USB drive load slowly
   (~12 MB/s); 4–8 steps at 512px is enough to prove the path.
5. V100 (sm70) notes: no bf16, 32 GB; large models may need
   `offload_params_to_cpu: true` in `config/config.json` at full resolution.

After a successful GPU run, record the model facts (dir layout, defaults, arch
log line, sample output) in project memory so the next model does not require
re-investigation.
