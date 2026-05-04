## Mask 节点

| # | 节点名                | 类别     | 功能                               | 输入                                                                     | 输出     |
| - | ------------------ | ------ | -------------------------------- | ---------------------------------------------------------------------- | ------ |
| 1 | LoadImageMask      | mask   | 加载图片作为 Mask                      | image (COMBO), channel (COMBO)                                         | Mask   |
| 2 | ImageToMask        | mask   | 图片转 Mask（灰度提取）                   | image (IMAGE), channel (COMBO)                                         | MASK   |
| 3 | MaskToImage        | mask   | Mask 转图片（灰度→RGB）                 | mask (MASK)                                                            | IMAGE  |
| 4 | InvertMask         | mask   | 反转 Mask（黑白互换                     | mask (MASK)                                                            | MASK   |
| 5 | MaskComposite      | mask   | Mask 合成（加/乘/减/交集/差集/除）           | destination (MASK), source (MASK), operation (COMBO), x (INT), y (INT) | MASK   |
| 6 | SolidMask          | mask   | 创建纯色 Mask                        | value (FLOAT), width (INT), height (INT)                               | MASK   |
| 7 | FeatherMask        | mask   | Mask 边缘羽化                        | mask (MASK), left (INT), top (INT), right (INT), bottom (INT)          | MASK   |
| 8 | ThresholdMask      | mask   | Mask 二值化阈值                       | mask (MASK), value (FLOAT)                                             | MASK   |
| 9 | SetLatentNoiseMask | latent | 将 Mask 应用到 Latent（用于 Inpainting） | samples (LATENT), mask (MASK)                                          | LATENT |

### 典型使用场景

```textile
LoadImage ──┬──→ ImageToMask → InvertMask → SetLatentNoiseMask → KSampler
            └──→ VAEEncode ──────────────────────────┘
```

Mask合成：

```tex
SolidMask → MaskComposite(+InvertMask) → FeatherMask → SetLatentNoiseMask#
```
**编译时开启controlnet特征**
- 启用 controlnet feature
cargo build --features "controlnet,local"
- 或者如果使用 FFI
cargo build --features "controlnet,local-ffi

