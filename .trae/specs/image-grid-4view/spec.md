# Spec: Combine 4 Character Views into 2×2 Grid Image

## Problem
The `QwenImageMultiViewPose` workflow generates 4 separate character view images (front/side/back/top-down). The user wants these combined into a **single 2×2 grid image** instead of 4 separate outputs.

## Solution
A new `ImageGrid` builtin node takes a list of images and arranges them into a grid. Placed between `QwenImageMultiViewPose` and `SaveImage` in the workflow.

## Implementation (Completed)

### 1. ImageGrid Node
- **File**: [builtin_nodes.rs](file:///home/acproject/workspace/rust_projects/comfyui-rust/crates/comfy-executor/src/builtin_nodes.rs) (~L2637-2758)
- **Registration**: line 268 — `register_image_grid(registry);`
- **Inputs**: `images` (IMAGE list), `rows` (INT, default 0=auto), `columns` (INT, default 0=auto)
- **Auto layout**: `cols = ceil(sqrt(n))`, `rows = ceil(n/cols)`
- **Workflow config**: `columns: 2` → forces 2×2 grid for 4 images
- **Output**: single IMAGE `{"images": [grid_image]}`
- **Grid assembly**: copies pixel data row-by-row from each source image into the correct position
- **Log**: `ImageGrid: {} images -> {}x{} grid ({}x{})`

### 2. Workflow
- **File**: `将人物切换为4视图.json`
- Node 3: `LoadImage` (OK_033_00000.png)
- Node 1: `QwenImageMultiViewPose` → 4 views
- Node 4: `ImageGrid` (columns: 2) → 2×2 grid
- Node 2: `SaveImage` (filename_prefix: OK_041) → saves grid

### 3. Server
- **Binary**: `target/release/comfy-server` (compiled, running in background)
- **Server log**: `~/.config/Trae CN/ModularData/ai-agent/command-execution/423a393d15f34ccc41af1b981d3a3ab0/output.log`

## Generation Status (In Progress)
| View | Status | Output |
|------|--------|--------|
| Front | ✅ Done | py_img_42.png |
| Side | ✅ Done | py_img_43.png |
| Back | ✅ Done | py_img_44.png |
| Top-down | 🔄 20/40 (50%) | py_img_45.png (pending) |
| ImageGrid + Save | ⏳ Pending | OK_041_00000.png |

- Each view: 40 steps, ~102s/step, ~68 min
- Top-down ETA: ~03:25 CST
- Grid assembly after: seconds

## Acceptance Criteria
1. Top-down view completes successfully (40/40 steps)
2. `ImageGrid: 4 images -> 2x2 grid` appears in server log
3. `output/OK_041_00000.png` exists with dimensions **2496×1664** (2×1248 × 2×832)
4. The image is a valid 2×2 grid containing all 4 views (not noise/corruption)
5. Only 1 output file (OK_041_00000.png), not 4 separate images

## Verification Commands
```bash
# Check grid log
grep "ImageGrid" <server-log>

# Check dimensions
python3 -c "from PIL import Image; print(Image.open('output/OK_041_00000.png').size)"

# Expected: (2496, 1664)
```
