# [OPEN] triposplat-cuda-crash

## Summary
- Symptom: `generate_3d_gaussian` path hits `ggml_cuda_compute_forward: CONT failed` followed by `CUDA error: an illegal memory access was encountered`.
- Scope: TripoSplat Gaussian decode path after recent rewrite to use flow latent.
- User goal: fix runtime crash and continue wiring Rust/FFI `decoder_path`.

## Hypotheses
1. The new decode graph feeds a tensor with incompatible shape/stride into a CUDA `CONT` op, causing illegal access during materialization.
2. The reference Gaussian decoder attention path reshapes Q/K/V incorrectly for ggml CUDA expectations, and the crash occurs in attention or a downstream contiguous conversion.
3. The decode graph is reading weights or runtime tensors from the wrong backend/buffer because the independent `gs.*` decoder weights are only partially registered.
4. The latent passed from `stable-diffusion.cpp` has a layout different from what the decoder path expects, so a later view/reshape becomes invalid on GPU.
5. The crash is not in the new `gs` path itself, but in the old placeholder octree sampling or tensor upload path that now interacts with the decode graph differently.

## Plan
- Add instrumentation only.
- Reproduce and collect runtime evidence.
- Confirm or reject hypotheses.
- Apply minimal fix.
- Verify with post-fix evidence.

## Evidence
- Debug server started at `http://127.0.0.1:7777`.
- Instrumentation added to:
  - `TripoSplatRunner::compute_gaussians`
  - `TripoSplatRunner::build_decode_graph`
  - `ReferenceDecoderAttention::forward`
- A rebuilt binary is active because runtime logs now include `build_graph: model.forward returned, out=...`.
- One reproduction occurred while the debug server was not collecting events, so `.dbg/trae-debug-log-triposplat-cuda-crash.ndjson` remained empty.
- With collector active, another reproduction still produced no decode-path events.
- This weakens the "decoder path crash" hypothesis and indicates the fault occurs earlier in the flow model compute path.
- Added new instrumentation in:
  - `RePo3DRotaryEmbedding::compute_axis_angles`
  - `TripoSplatAttention::forward`
  - specifically the `repo` branch just before `ggml_cont(v)`
- Leading suspects are now the repo attention path and the first `CONT` materialization there.
- Runtime evidence now confirms:
  - `triposplat attention entry`
  - `repo axis angle tensor shapes`
  - `repo path before ggml_cont(v)`
  - and then immediate CUDA `CONT failed`
- Confirmed hypothesis:
  - the current crash occurs at or immediately after `ggml_cont(v)` inside the repo attention branch
- Minimal fix applied:
  - removed the eager `ggml_cont(v)` materialization
  - replaced it with a merged `v` view directly from `qkv`, letting `ggml_ext_attention_ext(..., skip_reshape=false, ...)` handle the later reshape/permute path
- Post-fix symptom changed from CUDA illegal access to:
  - `ggml.c:3632: GGML_ASSERT(ggml_is_contiguous(a)) failed`
- Evidence determination:
  - `ggml.c:3632` is `ggml_reshape_3d()`
  - this matches the temporary follow-up code path that still did `ggml_reshape_3d(v_merged, ...)`
  - `v_merged` is a `ggml_view_3d(...)`, so it is expected to be non-contiguous
- Second minimal follow-up fix applied:
  - stop reshaping `v_merged`
  - pass the merged `[hidden_size, L, N]` view directly into `ggml_ext_attention_ext(..., skip_reshape=false, ...)`
- Post-fix symptom changed again to:
  - `ggml.c:3652: GGML_ASSERT(ggml_is_contiguous(a)) failed`
- Evidence determination:
  - `ggml.c:3652` is `ggml_reshape_4d()`
  - after the previous change, the strongest candidate is `ggml_ext_attention_ext(..., skip_reshape=false, ...)` reshaping `v`
  - `v` was still a non-contiguous merged view from `qkv`
- Third minimal follow-up fix applied:
  - keep the merged `v` layout
  - materialize contiguity only on the merged `[hidden_size, L, N]` view via `ggml_cont(v_merged_view)`
  - avoid the earlier crashing `ggml_cont()` on the head-split `[d_head, n_head, L, N]` form
- Post-fix symptom then regressed to:
  - CUDA `ggml_cuda_compute_forward: CONT failed`
- Evidence determination:
  - merged-layout contiguity is still needed, but using `ggml_cont()` still routes through the same CUDA `CONT` kernel family
  - this suggests the current blocker is specifically the CUDA `CONT` path, not merely the need for a contiguous merged `v`
- Fourth minimal follow-up fix applied:
  - replace `ggml_cont(v_merged_view)` with `ggml_dup(v_merged_view)`
  - goal: preserve a contiguous merged `v` while avoiding the specific CUDA `CONT` kernel path

## Status
- Session initialized.
