---
name: comfyui-sdcpp-gpu-validation
description: 在 comfyui-rust 工作区验证模型走 sd.cpp 原生路径并在 GPU 上完成 sd-cli 与 comfy-server 双重验证。用于新增/切换模型管线、CUDA 报 OS call failed (error 301) 回退 CPU、或需要确认服务端工作流出图时。
---

# comfyui-rust 模型 GPU 验证流程

验证目标：模型走 sd.cpp 原生路径（非 Python fallback），且 sd-cli 与 comfy-server 工作流均在 CUDA 上出图正常。

## 1. 关键环境

- 仓库：`/home/acproject/workspace/rust_projects/comfyui-rust`
- 模型库：`/home/acproject/usb/comfyui/models`（`COMFY_MODELS_DIR`）
- GPU：3×Tesla V100 32GB，推理只用 GPU0；CUDA 12.8 在 `/usr/local/cuda-12.8`
- sd-cli：`cpp/stable-diffusion.cpp/build/bin/sd-cli`（GGML_CUDA=ON、GGML_CUDA_FA=ON）
- 服务器：`target/debug/comfy-server`，监听 `127.0.0.1:8188`，配置在 `config/config.json`（gitignore；优先于默认值）
- 磁盘紧张：`/` 仅约 20G，产物放 /tmp 或 USB

## 2. 沙箱与 CUDA 301（最重要的坑）

现象：日志出现 `ggml_cuda_init: failed to initialize CUDA: OS call failed`，随后 `auto-fit: no GPU devices ... #0: CPU`，模型在 CPU 上跑且推理后常 SIGSEGV（exit 139）。

已确认的触发规律：**同一条 Shell 调用里先执行 `pkill`/`kill`（或 `fuser -k` 等）再启动 CUDA 进程，新进程会进入沙箱受限态导致 CUDA 301。** 与链接符号、seccomp、AppArmor、命名空间、init_array 均无关；用 strace 包裹启动会"碰巧成功"（时序差异），不要依赖它。

操作规则：

1. 所有 GPU 相关 Shell 调用必须 `dangerouslyDisableSandbox: true`。
2. **停进程和启动进程必须拆成两次独立的 Shell 调用**：
   - 调用 A：`pkill -x comfy-server; sleep 2; pgrep -x comfy-server || echo stopped`
   - 调用 B（不含任何 kill/pkill）：`cd <repo> && (setsid env COMFY_MODELS_DIR=... target/debug/comfy-server > /tmp/xxx.log 2>&1 < /dev/null &)`
3. 若仍怀疑处于坏窗口，用独立调用跑探测确认（无残留后新建临时 example 调 `comfy_inference::get_system_info()`，验证后删除）：出现 `found 3 CUDA devices` 即为好窗口。
4. 后台 `run_in_background` 方式启动服务器也可能落入坏窗口；优先 setsid 脱离式。
5. 不要用 ptrace attach（Operation not permitted）；只能 strace 包裹启动。

## 3. 标准验证步骤

### A. sd-cli 先验证（链路最快）

- 从模型的 `docs/<model>.md` 或已成功输出 PNG 的 metadata 还原参数（见第 4 节）。
- 文生图直接出图；图像编辑加 `-r <input.png>`。
- 管线目录（HF/diffusers 结构）必须显式传组件路径：
  - `--diffusion-model <dir>/transformer/diffusion_pytorch_model.safetensors.index.json`
  - `--vae <dir>/vae/diffusion_pytorch_model.safetensors`
  - `--llm <base>/text_encoders/qwen3vl_8b_fp8_scaled.safetensors`
  - 编辑任务加 `--llm_vision <base>/text_encoders/qwen3vl_8b_visual.safetensors`
  - Qwen3VL/Boogu 类加 `--diffusion-fa`
- 管线自带的 HF 分片 text_encoder/mllm（`model.language_model.*`/`model.visual.*` 键）与 sd.cpp 不兼容，勿用。视觉塔键名必须是裸 `visual.*`。

### B. Rust 侧路由确认

- 白名单家族在 `crates/comfy-inference/src/python.rs` 的 `SDCPP_PIPELINE_FAMILIES` + `rewrite_pipeline_for_sdcpp`：命中管线目录后改写为 `model_path=None` + diffusion/vae/llm 路径；有 ref_images 且无 llm_vision 时自动选视觉塔。
- `local.rs` 把 ref_images 填入 CImgGenParams；KSampler（builtin_nodes.rs `register_ksampler`）的 optional 输入 `reference_image` 接收 IMAGE。
- LoadImage 节点输出 `{"type":"image","path":"<input 相对路径>"}`，不是内联 SdImage；KSampler 用 `parse_sd_image_from_value` + `controlnet::load_image_from_value`（controlnet feature 下）兜底从输入目录读图。
- `diffusion_flash_attn` 是 context 级配置（config.json），默认 false；Boogu/Qwen3VL 类需设 true（等价 `--diffusion-fa`）。
- 家族→ModelType 映射 `sdcpp_family_to_model_type` 缺新家族时落 Other；改写器直写 vae_path 可绕过 VAE 探测退化，必要时补映射。

### C. 服务端工作流验证

1. 按第 2 节方式启动服务器，`sleep 6` 后先看日志是否出现 `ggml_cuda_init: found 3 CUDA devices`；若是 301，停掉重来，不要在 CPU 上浪费时间。
2. 提交 `/prompt`（API 格式 `{"prompt": {节点 id: {"class_type":..., "inputs":...}}}`，连线用 `[目标id, 输出槽]`）。
3. 关键日志锚点：
   - `matched sd.cpp family <id> ... rewritten to native component paths`（路由正确）
   - `reference images present, auto-selected vision weights ...`
   - `Using flash attention in the diffusion model`
   - `... on CUDA0` / `prepared params backend buffers (... VRAM) on CUDA0`
4. HTTP 在推理期间会长时间不响应，不要短超时反复重发；直接轮询日志与 `output/` 目录（轮询间隔 ≥ 10s）。
5. 完成后必须用 Read 工具肉眼检查输出 PNG：非噪声、语义正确（编辑任务还要核对保留构图 + 编辑指令生效）。

### D. Boogu-Image-Edit 参考参数（V100 实测）

steps 20、CFG 6.0、guidance 3.5（默认即 3.5）、euler/discrete、strength 0.75（默认）、seed 任意；1024² 下 V100 约 54s/it，20 步约 18 分钟，属正常（GPU0 应 100% 利用率）。Boogu-Turbo 文生图则是 4 步 lcm、cfg≈1.0。

## 4. 从输出 PNG 还原 sd.cpp 参数

sd-cli/FFI 输出的 PNG 把完整参数写在 `parameters` 文本块和 `SDCPP:` JSON 里：

```bash
python3 - <<'EOF'
from PIL import Image
im = Image.open('/tmp/x.png')
print(im.size)
print(im.info.get('parameters','')[:2000])
EOF
```

可还原 steps/cfg/guidance/sampler/scheduler/seed/尺寸/strength/各组件文件名，用于构造等价的服务端工作流。

## 5. 收尾

- 删除一切临时诊断：临时 examples、`eprintln!` 调试行、/tmp 工作流、input 测试图、临时日志。
- config/config.json、output/、input/ 均被 gitignore，改动不进版本库。
- 停止临时服务器（独立调用 pkill）。
- 未经用户明确要求不要 commit；提交前跑 `cargo build -p comfy-api --features "local-ffi,controlnet,flash-attn"`，并额外 `cargo check -p comfy-executor`（无 controlnet feature 时 cfg 块内变量易触发 unused_mut）。
