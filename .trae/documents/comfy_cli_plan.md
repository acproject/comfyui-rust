# 多媒体资源 CLI（comfy）实施计划

## Repository Research

### 现状
- 工作区现有两个交付二进制：`comfy-server`（HTTP/WS，crates/comfy-api）与 `comfy-mcp`（MCP stdio，crates/comfy-mcp，且它是 server 的 HTTP 客户端）。**没有**供外部程序直接调用的独立多媒体 CLI。
- `crates/comfy-inference/src/cli.rs` 里的 `CliBackend` 不是用户命令行，而是对外部 `sd-cli` 可执行文件的内部封装。
- 所有推理能力已收敛在 `InferenceBackend` trait（backend.rs），`PythonBackend`（python.rs，直连 `py/flash_attn_v100/comfy_fallback` 脚本）实现了全部多媒体模态：

| 模态 | trait 方法 | Python 脚本 | 产物 |
|---|---|---|---|
| 图片 t2i/i2i/编辑 | `generate_image(ImageGenParams)` | img_generate.py | 内存 `SdImage` → PNG |
| 音乐 Music3 | `generate_music3(Music3Params)` | music3_generate.py | WAV 文件（`Music3Output.output_file`） |
| 视频 Bernini t2v/v2v | `generate_bernini_video(BerniniVideoParams)` | bernini_generate.py | MP4 文件 |
| 音视频 H3 t2va/i2va/ref2va/mr2va | `generate_av(H3Params)` | h3_generate.py | `SdVideo`（帧+音轨）→ ffmpeg 复用为 MP4 |
| 3D 高斯 TripoSplat | `generate_3d_gaussian(Gaussian3DParams)` | triposplat_generate.py | PLY/SPLAT 文件 |
| 能力查询 | `get_capabilities()` | — | `BackendCapabilities` |

- 媒体 I/O 原语齐备：`SdImage::from_image_bytes/to_png_bytes`、`SdVideo::encode_with_ffmpeg/decode_with_ffmpeg`、`SdAudio::to_wav_bytes`。
- `PythonBackend::new(PythonInferConfig, workspace_root)` 可独立构造；脚本目录解析顺序：显式参数 → `COMFY_PYTHON_SCRIPT_DIR` → workspace 相对路径 → cwd 相对路径。`FallbackBackend::new(primary: Arc<dyn InferenceBackend>, python)` 可在给定 `sd-cli` 时做混合后端。
- 仓库**无 clap 依赖**，`comfy_mcp.rs` 采用手写参数解析；本计划沿用该风格，不引入新依赖。
- 依赖中已有 serde / serde_json / tracing / tracing-subscriber（comfy-mcp 在用）。

### 已确认的决策（用户选择）
1. 首版覆盖**全部多媒体能力**：image / music / video / av / gaussian3d + capabilities。
2. **独立直连后端**：进程内构造 `PythonBackend`（可选 `FallbackBackend` 包 sd-cli），不依赖 comfy-server 运行。
3. 新建 crate **`comfy-cli`**，二进制名 **`comfy`**（lib + bin，镜像 comfy-mcp 结构），加入 workspace。

## Files and Modules

- `Cargo.toml`（workspace 根）：members 增加 `crates/comfy-cli`。
- `crates/comfy-cli/Cargo.toml`（新）：package `comfy-cli`，`[lib]` + `[[bin]] name="comfy"`；依赖 comfy-inference(path)、serde、serde_json、tracing、tracing-subscriber。
- `crates/comfy-cli/src/lib.rs`（新）：模块声明与 `run(args: impl Iterator<Item=String>) -> i32` 入口。
- `crates/comfy-cli/src/bin/comfy.rs`（新）：薄 main，初始化 stderr tracing 后调用 `comfy_cli::run(std::env::args())`。
- `crates/comfy-cli/src/args.rs`（新）：手写解析器。`GlobalOpts`、`Subcommand` 枚举（Image/Music/Video/Av/Gaussian3d/Capabilities/Help/Version）、各子命令参数 struct、`--flag value` / `--flag=value` / 可重复 flag / 用法错误（exit 2）。
- `crates/comfy-cli/src/backend.rs`（新）：由 GlobalOpts 构造 `Arc<dyn InferenceBackend>`；workspace 根用 `env!("CARGO_MANIFEST_DIR")` 上溯两级做脚本目录编译期兜底；`--sd-cli` 存在时包 `FallbackBackend`；提供各模态 `supports_*` 前置检查。
- `crates/comfy-cli/src/commands.rs`（新）：6 个子命令处理函数，负责把 CLI 参数映射为既有 params 结构、调用后端、落盘产物。
- `crates/comfy-cli/src/output.rs`（新）：统一结果信封 `{"ok":true,"command":...,"outputs":[{"path","media_type","format"}],"meta":{...}}`（stdout，单行 JSON）；失败信封 `{"ok":false,"error":...}`；默认输出路径与目录创建。
- `crates/comfy-cli/tests/cli_tests.rs`（新）：纯解析器/契约单测，不需要 GPU。
- `build.sh`：产物提示补充 `target/release/comfy`（仅 echo 文案，构建命令已是 `--workspace` 自动覆盖）。

## 命令面（详细）

全局选项（子命令之前）：`--python PATH`、`--script-dir DIR`、`--device auto|cuda|cpu`（默认 auto）、`--dtype auto|fp16|bf16|fp32`（默认 auto）、`--sd-cli PATH`（可选，启用混合后端）、`--output-dir DIR`（默认 `COMFY_OUTPUT_DIR` 或 `output/cli`）、`-h/--help`、`-V/--version`。

1. `comfy image --model PATH --prompt TEXT [--negative-prompt] [--width 512] [--height 512] [--steps 20] [--guidance-scale 7.0] [--seed 42] [--init-image PATH --strength 0.75] [--ref-image PATH]... [--lora PATH]... [--lora-scale F]... [--output PATH]`
   - 映射 `ImageGenParams`（`ModelConfig.with_model`，LoRA 按序配对，缺省 scale=1.0）；多张产物按 `_1/_2` 后缀写 PNG。
2. `comfy music --model PATH --prompt TEXT [--lyrics TEXT | --lyrics-file PATH] [--duration 60] [--seed 0] [--max-new-tokens N] [--output x.wav]`
   - 映射 `Music3Params`（duration 自动 clamp 1..360），直接把目标路径塞进 `output_path` 让脚本一次写到位。
3. `comfy video --model PATH --prompt TEXT [--negative-prompt] [--width 832] [--height 480] [--frames 81] [--fps 16] [--steps 40] [--seed 42] [--max-image-size 848] [--input-video PATH] [--guidance-mode auto|t2v_apg|v2v_apg] [--output x.mp4]`
   - 映射 `BerniniVideoParams`；带 `--input-video` 即 v2v。
4. `comfy av --model PATH [--mode t2va|i2va|ref2va|mr2va] --prompt TEXT [--negative-prompt] [--width 832] [--height 480] [--frames 124] [--fps 24] [--steps 50] [--cfg 7.0] [--seed 42] [--reference PATH]... [--sfx/--no-sfx] [--bgm/--no-bgm] [--variant fl2va|ref2va] [--output x.mp4]`
   - 映射 `H3Params`；引用图经 `SdImage::from_image_bytes` 读入；产物 `SdVideo` 用 `encode_with_ffmpeg` 落 MP4（ffmpeg 缺失时给可操作报错）。
5. `comfy gaussian3d --ckpt PATH --decoder PATH --dinov3 PATH --vae-encoder PATH --rmbg PATH --input-image PATH [--steps 20] [--guidance-scale 3.0] [--seed 42] [--num-gaussians 262144] [--erode-radius 1] [--format ply|splat] [--output PATH]`
   - 映射 `Gaussian3DParams`/`ModelConfig`（clip_vision=dinov3、vae=vae-encoder 等），后端要求的五个模型路径全部做必填校验。
6. `comfy capabilities`：打印解释器、脚本目录与 `BackendCapabilities` JSON，不触发任何模型加载。

### 对外契约（便于其它程序集成）
- stdout：成功时恰好一行 JSON 结果信封；失败时一行 `{"ok":false,...}`。
- stderr：所有 tracing 日志与人读错误。
- 退出码：0 成功；1 运行/推理失败；2 参数用法错误。
- 输出路径未指定时统一落到 `--output-dir` 下，文件名含子命令与 seed。

## Implementation Steps

1. 根 `Cargo.toml` 加 member；建 `crates/comfy-cli/Cargo.toml` 与目录骨架。
2. `args.rs`：手写 tokenizer + 全局/子命令结构 + help/version 文案 + 解析单测。
3. `backend.rs`：后端构造、workspace 编译期兜底、能力前置检查。
4. `output.rs`：JSON 信封、默认路径、目录创建。
5. `commands.rs`：按 image → capabilities → music → video → av → gaussian3d 顺序接入（image/capabilities 可立即真机验证）。
6. `lib.rs` + `bin/comfy.rs` 串联：解析→构造后端→分发→落盘→JSON 输出/退出码。
7. `tests/cli_tests.rs`：六类命令的参数解析、默认值、重复 flag、用法错误码、help/version。
8. `build.sh` 产物提示补一行。

## Dependencies and Considerations

- 不新增第三方解析依赖：手写 parser 与 comfy-mcp 保持一致，减少供应链与编译成本。
- comfy-inference 默认 features 即可获得 PythonBackend/CliBackend/FallbackBackend（FFI/bridge 均为可选 feature，本 crate 不启用）。
- `PythonInferConfig.device/dtype` 原样透传给脚本；`auto` 即进入 img_generate.py 的多卡分片分支（本次已验证的 Edit-2511 路径）。
- H3 的 `ref2va` 后端仅支持图像参考（python.rs 现有约束），CLI 不开放视频参考参数，避免传入即报错。
- 长文本（歌词/提示词）支持 `--lyrics-file/--prompt-file` 从文件读取，规避 shell 长度与换行问题。

## Validation

- `cargo build -p comfy-cli` 与 `cargo test -p comfy-cli`（解析器单测全绿）。
- `cargo check --workspace` 确认无破坏性影响。
- 真机轻量验证（本机脚本与 venv 存在）：
  - `./target/debug/comfy --help`、`--version`、无参数退出码 2。
  - `./target/debug/comfy capabilities` 返回合法 JSON 且 six 个能力位与脚本存在性一致。
  - `comfy image` 用 Qwen-Image-Edit-2511 跑 512px/4 steps 端到端（复用已验证的 img_generate.py 分片路径），确认 PNG 落盘与 stdout JSON 契约。
- music/video/av/gaussian3d 受限于本机模型是否齐备：缺失时验证报错信息可操作；模型存在时按同样契约手动跑一次。

## Risks

- **重模型 E2E 成本高/模型缺失**：解析器与参数映射用单测覆盖；image 做真机 E2E，其余模态在模型就绪后按文档命令复验，运行期错误统一走 JSON 失败信封。
- **ffmpeg 缺失导致 av 落盘失败**：调用前用 `SdVideo::is_ffmpeg_available()` 预检并给出安装提示。
- **任意 cwd 下找不到 Python 脚本**：三级解析（`--script-dir`/`COMFY_PYTHON_SCRIPT_DIR` → 编译期 workspace 兜底 → cwd 相对），并在 capabilities 中暴露最终解析结果便于排障。
- **手写 parser 回归**：所有解析规则进 `cli_tests.rs` 表驱动测试，用法错误稳定返回退出码 2。
