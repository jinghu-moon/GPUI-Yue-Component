# HDR 分阶段实施与验证记录（2026-11-03）

对应 [27 · HDR 管线](../27-hdr-pipeline.md)。这是工作区验证记录，不是完整 HDR 产品验收。实施前工作区已包含未提交修改；没有 reset、提交或推送，也没有把用户大图纳入 fixture。

## 已实现的合同

- 显示资源明确区分 BGRA8/sRGB、RGBA16F/相对线性 SDR、RGBA16F/绝对 scRGB（1=80 nit）。FP16 保留负 RGB 和大于 1 的高光；CPU 输入为 straight alpha，目标读回为 associated alpha。
- PNG 文件流按行解码、解析 cICP 优先级和 matrix/TRC ICC。PQ 在进入工作空间前完成 EOTF 与原色转换；16 位 SDR 不冒充 HDR。未知或未经验证的色彩定义返回错误。
- `moxcms 0.8.1` 启用 `extended_range`，F32 transform 使用 `prefer_fixed_point=false` 和 `allow_extended_range_rgb_xyz=true`。Display-P3 红色测试验证转换后的负分量和大于 1 分量没有被裁掉。PQ ICC 的通用 PCS 转换只产生相对值，不能据此推定绝对 nit；已知 CICP/PQ 走显式绝对转换。
- SDR 显示派生采用固定 BT.2390 EETF 与明确的色域压缩。CPU 与生产 HLSL 使用同一组独立生成参考向量；不根据代理重新估计源峰值。
- Windows opt-in `WindowColorPolicy::AutomaticHdr` 使用真实 FP16 swapchain 和检查过的 G10/P709 色彩空间；普通窗口默认仍为原 SDR。系统当前类型、HDR 支持和窗口实际模式分别报告。
- `DisplayInformation.GetForWindow` 运行时探测，事件回调合并到 HWND 所在线程；窗口取消订阅、平台拥有 DispatcherQueue/WinRT 生命周期。跨屏/设置/设备事件触发刷新，不轮询显示器。
- UI、渐变、文字覆盖率、4× MSAA path、浮点 backdrop 与图像在线性目标内合成。读回按格式和 pitch 处理，`render_to_image` 明确重绘 SDR straight-alpha 派生，不把 FP16 当 RGBA8。
- 提供 `hdr_preview` 原生诊断示例。示例不承担异步加载 UX；45MP 文件只显示代理，不从代理导出或制造“原始细节”。

## 修改前后与可复现证据

日志均在本地忽略的 `target/` 下；CI/其他机器应重新运行命令，不把下表的历史数字当作实时结果。通过数包括对应命令输出的 doctest；ignored 不计入通过。

| 验证 | 修改前 | 修改后 |
| --- | --- | --- |
| `cargo test --workspace --locked --offline` | 215 通过 | 232 通过 |
| 同上加 `--features headless-tests` | 685 通过 | 702 通过 |
| `yue-component --no-default-features --features headless-tests` | 原有入口 | 682 通过，使用系统字体 |
| 两组 workspace Clippy `--all-targets -- -D warnings` | 原有入口 | 均通过 |
| standalone Windows backend Clippy `--all-targets --features test-support -- -D warnings` | 新增路径 | 通过，显式使用同一 core patch |
| `cargo fmt --all -- --check` | 原有入口 | 通过，只格式化本轮涉及文件 |
| core snapshot/backdrop 合同 | 原有入口 | 5 + 6 通过，含 FP16、快照持有与 20 次替换回收 |
| Windows DirectX 回归，Debug/Release | 新增 FP16 合同不能满足 | 各 31 通过；默认跳过硬件计时及隐藏原生窗口测试 |
| Windows 输出策略 | 无独立合同 | Debug/Release 各 1 通过 |
| 原创 fixture 与 token 同步 | 原有 token 入口 | fixture `--check`、两种 token `--check` 通过 |
| SnapClip app/pin | app 285 / pin 71 通过，另 5 / 4 ignored | app 285 / pin 72 通过，另 5 / 4 ignored |

先失败后修复的日志包括：`hdr-p1-alpha-before.log`（目标 alpha 错误累加）、`hdr-p2-ui-before.log`（UI/采样线性化）、`hdr-p2-backdrop-before.log`（FP16 capture）、`hdr-p2-glyph-before.log`（文字 alpha）。最终日志：`hdr-current-workspace.log`、`hdr-current-headless.log`、`hdr-current-system-fonts.log`、`hdr-current-clippy*.log`、`hdr-current-snapshot.log`、`hdr-current-gpu-{debug,release}.log`。新增测试没有删除或放松原有 SDR 断言。

### 隐藏原生窗口集成

`hdr-p2-native.log`、`hdr-p2-native-release.log`：在本机创建隐藏、无激活 HWND；建立真实硬件 swapchain、读取 Windows 当前显示能力，再注入输出快照验证 SDR/FP16/HDR 模式切换、反复 resize、SDR 重绘读回后资源恢复、SDR white generation 和生产 device-recovery 重建路径。Debug/Release 各 1 项通过。没有 ShowWindow、Present 或改变系统 HDR。

实际系统快照：`display_kind=Sdr`、`hdr_supported=true`、SDR white 80 nit、报告峰值 400 nit。因此**只证明本机 API/资源路径可用，不证明显示器已经显示 HDR**。注入 HDR 快照不是实际开启 HDR；调用 recovery 方法也不是模拟真实驱动拔除。该测试保持 opt-in/ignored：

```powershell
pwsh -NoProfile -File tools/test-gpui-hdr.ps1 -Native
# Release 验证同一生产 shader 的嵌入编译路径
pwsh -NoProfile -File tools/test-gpui-hdr.ps1 -Native -Release
```

## 45MP 文件准备测量

系统记录：Windows 11 IoT Enterprise LTSC，10.0.26100 / build 26100；NVIDIA 驱动 32.0.16.2101。系统还枚举到 GameViewer Virtual Display Adapter（15.6.5.199），不能把该适配器当作已测试的第二块 HDR 显示器。

Windows，Intel Core i5-13600KF / RTX 4070 Ti SUPER，Release，使用用户本地两张 8256×5504 PNG。输入文件预算由诊断显式设为 256 MiB，组件默认仍为 64 MiB；代理预算仍为 64 MiB。每张只运行一次，无中位数或显著性结论。

```powershell
cargo run -p yue-component --example image_color_probe --release --locked --offline -- D:/100_Projects/110_Daily/SnapClip/HDR-IMAGES/1A_PNG_HDR.png D:/100_Projects/110_Daily/SnapClip/HDR-IMAGES/1B_PNG_SDR.png
cargo run -p yue-component --example hdr_resource_probe --release --locked --offline -- D:/100_Projects/110_Daily/SnapClip/HDR-IMAGES/1A_PNG_HDR.png D:/100_Projects/110_Daily/SnapClip/HDR-IMAGES/1B_PNG_SDR.png
```

| 来源 | 解码后语义 | 源像素字节 | 代理 | 代理字节 | 解码 | 代理准备 |
| --- | --- | ---: | --- | ---: | ---: | ---: |
| HDR PNG | scRGB FP16，源峰值 981.25 nit | 363,528,192 | 3547×2364 | 67,080,864 | 4829.635 ms | 788.719 ms |
| 16 位 SDR PNG | 相对线性 SDR FP16 | 363,528,192 | 3547×2364 | 67,080,864 | 1871.791 ms | 808.828 ms |

日志：`hdr-current-icc-probe.log`、`hdr-current-large-resources.log`。解码包含文件读取与颜色准备，代理计时包含校验/统计/缩图/资源构造；没有隔离冷/热磁盘缓存。没有测 GPU 上传、窗口首显、帧率、进程 RSS 或显存峰值，不能据此宣称低占用或实时准备。PNG 避免额外全图 F32，但 FP16 源和代理在准备期间仍同时存活。

## 待通过的门禁

1. AVIF：当前生产依赖没有已验证的高精度解码/许可/性能证据，继续不启用 HDR AVIF；历史文档的 codec 声明不能代替源码证据。
2. 显示：系统 HDR 真开/关、SDR 白变化、WCG、SDR/HDR 双屏、DPI、真实设备丢失及恢复。当前隐藏窗口不提供 UI 观感、Narrator 或实际峰值证据。
3. SnapClip：第一批源像素合同、typed ROI readback 和 FP16 原始字节 tile 已测试；生产冻结帧/generation、WGC、D2D、滚动分析/原始行带分离、耐久浮点母版、HDR PNG 导出、保存/复制/F3/F6/历史仍按相邻仓库 docs/53 继续。当前 Pin 仍为八位模型，明确拒绝 HDR；16 位 SDR 生成显式 SDR 派生副本。原始 tile 的逐 bit 测试不等于耐久母版端到端验收。
4. 性能：完整资源账本和 Release 端到端测量。FP16 path 中间目标目前按视口分配，并未做 lazy 显存优化；只算源像素字节不足以估计整体占用。

无完整端到端证据前，不在应用入口标注“HDR 已支持”。

### SnapClip P3-A 的回归状态

新增 5 项像素合同、1 项 tile、1 项 pitch、1 项初始帧行数边界测试；另外单独运行 1 项 FP16 GPU ROI 测试及 3 项既有 SDR GPU/D2D 纹理、遮罩、带标注裁切测试，均通过。关联普通回归：Capture 548（另 81 ignored，最终日志 `hdr-p3a-capture-final.log`）、Clipboard 11（另 2 ignored）、History 43、Model 26；App 285（另 5 ignored）、Pin 72（另 4 ignored）。Model 的 5 项新 integration test 单列，不重复算入 26 项 lib 测试。

SnapClip 格式检查、app/pin all-targets 编译、Model/Pin all-targets Clippy 通过。**扩大至 Capture/History 的 Clippy `-D warnings` 未通过**：Capture lib/test 报告 49/79 项（后者包含前者），History lib/test 报告 2/3 项；主要落在本批未改的几何注释、原有 benchmark/test、D2D 等代码。比如 `d3d11.rs` 原有 `worst(&mut Vec<...>)` 在 HEAD 中已存在。未使用 allow 或削弱警告来制造全绿；这项门禁仍保留，不能把整个 SnapClip 的 lint 验收称为通过。记录见相邻 docs/53 与 `SnapClip/target/hdr-p3a-clippy.log`。
