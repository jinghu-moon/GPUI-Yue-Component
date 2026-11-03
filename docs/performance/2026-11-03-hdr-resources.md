# HDR 分阶段实施与验证记录（2026-11-03）

对应 [27 · HDR 管线](../27-hdr-pipeline.md)。这是分批验证记录，不是完整 HDR 产品验收。实施前工作区已包含未提交修改，没有 reset 或把用户大图纳入 fixture。随后按用户授权正常提交推送了检查点（Yue `4846e89`、SnapClip `3b405c7`），接续 P3-B／P3-C 的代码和证据更新仍在工作区。

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
3. SnapClip：源像素合同、typed ROI、FP16 原始字节 tile、冻结帧来源代次、typed 选区及压缩像素母版发布已实现。WGC、D2D、生产滚动分析/原始行带分离、HDR PNG 导出、保存/复制/F3/F6/历史仍按相邻仓库 docs/53 继续。当前 Pin 仍为八位模型，明确拒绝 HDR；16 位 SDR 生成显式 SDR 派生副本。母版接口往返测试不等于用户捕获端到端验收。
4. 性能：完整资源账本和 Release 端到端测量。FP16 path 中间目标目前按视口分配，并未做 lazy 显存优化；只算源像素字节不足以估计整体占用。

无完整端到端证据前，不在应用入口标注“HDR 已支持”。

### SnapClip P3-A 的回归状态

新增 5 项像素合同、1 项 tile、1 项 pitch、1 项初始帧行数边界测试；另外单独运行 1 项 FP16 GPU ROI 测试及 3 项既有 SDR GPU/D2D 纹理、遮罩、带标注裁切测试，均通过。关联普通回归：Capture 548（另 81 ignored，最终日志 `hdr-p3a-capture-final.log`）、Clipboard 11（另 2 ignored）、History 43、Model 26；App 285（另 5 ignored）、Pin 72（另 4 ignored）。Model 的 5 项新 integration test 单列，不重复算入 26 项 lib 测试。

SnapClip 格式检查、app/pin all-targets 编译、Model/Pin all-targets Clippy 通过。**扩大至 Capture/History 的 Clippy `-D warnings` 未通过**：Capture lib/test 报告 49/79 项（后者包含前者），History lib/test 报告 2/3 项；主要落在本批未改的几何注释、原有 benchmark/test、D2D 等代码。比如 `d3d11.rs` 原有 `worst(&mut Vec<...>)` 在 HEAD 中已存在。未使用 allow 或削弱警告来制造全绿；这项门禁仍保留，不能把整个 SnapClip 的 lint 验收称为通过。记录见相邻 docs/53 与 `SnapClip/target/hdr-p3a-clippy.log`。

### SnapClip P3-B：来源与压缩母版

本批在已推送检查点 `3b405c7` 后继续，尚未提交。完整合同、版本与预算见相邻 [docs/53 §9](../../../SnapClip/docs/53-hdr-capture.md#9-第二批实施记录p3-b2026-11-03)。

- `PixelSource` 将真实 descriptor 与采集 generation 一起传播到冻结帧、typed 选区、行带接口和发布元数据；读前／读后拒绝混合来源。采集代次与导出取消、显示输出 revision 分开。
- 四／八字节 CPU 裁切和 typed ROI 共用冻结帧入口；圆角按 Alpha 合同处理，FP16 RGB 不量化。Opaque 加圆角派生 Straight Alpha，由选区成品描述记录，原冻结帧定义不变。
- 既有 History store 新增 `.scpx` v1：独立 zlib 行 tile、完整定义及 BLAKE3 校验、有界读行和原子发布。应用 writer 已接入；普通 straight SDR BGRA8 仍走既有流式 PNG，高精度来源保存原字节母版。取消清理同样覆盖母版。
- 单 raw tile ≤4 MiB、输出单次 ≤64 MiB；打开用 64 KiB scratch 验证整个编码文件，然后建立有界索引。打开仍有全文件顺序 I/O；写入／读取同时存活的压缩块、解压块与输出不能省略出资源账本。

先失败再修复：`hdr-p3b-contract-before.log`／`hdr-p3b-master-before.log` 是缺失接口的失败；`hdr-p3b-rounded-before.log` 是 Opaque 圆角发布失败，`hdr-p3b-rounded-after.log` 转绿。最终结果：

| 验证 | 本批结果 |
| --- | --- |
| 普通 workspace lib，串行 | App 268、Capture 552、Clipboard 11、History 43、Model 26、Pin 71、UI 221；共 1,192 项通过，ignored 分开不计 |
| App／Pin headless lib | 288／72 通过，另 5／4 ignored |
| 来源／像素／母版 integration | Capture 6、Model 6、History 5 通过；History Release 同样 5 项通过 |
| 明确筛选的 GPU／D2D opt-in | FP16 ROI、纹理、选区遮罩、带标注导出，各 1 项通过 |
| SDR streaming export Release 分配 probe | 3 项通过，不产生第二份全图分配 |
| all-targets check／fmt check | 通过；UI gallery 示例原有 9 项 must-use warning 仍保留 |
| Model／History strict all-targets Clippy | 通过；History 原有 3 项 lint 已修复 |
| Capture strict Clippy | 仍失败：lib 49／lib test 79，包含重叠 |
| 扩大 App／Pin strict Clippy | App lib 41／lib test 47 项失败，不能称全仓 lint 已通过；本批前没有 App 严格 lint 基线 |

最终日志均在 SnapClip `target/hdr-p3b-*`，包括 `contracts-final`、`workspace-final`、`headless-final`、`all-targets-final`、`fmt-final`、`master-release-final`、`allocations-release-final`、`gpu-final-*` 和各 `clippy-*-final`。

Release 合成样本 1024×1200 FP16 的单次记录：raw **9,830,400 B**，编码 **1,032,769 B**，最大 raw tile **4,194,304 B**；写入含 fixture 生成／校验 **21.786 ms**，打开含全文件指纹验证 **0.437 ms**。高度可压缩、未隔离缓存／并行系统负载，不能作为真实截图吞吐、45MP 或端到端性能结论。这里只测合成存储行为，未测 RSS／VRAM。

**剩余**：WGC 三处 pool、D2D 覆盖层、生产滚动源与固定分析派生图、HDR PNG 编码、保存面板／F3／F6／剪贴板／历史 DB，以及真实 HDR 显示和性能门禁。接口能逐 bit 发布 FP16，不代表生产 WGC 已采集 FP16；本批没有开启用户 HDR 入口、切换系统 HDR、实际截屏或修改剪贴板。

### SnapClip P3-C：WGC 格式合同与 D2D 内部 FP16

本批继续在工作区，未提交。完整实施记录见相邻 [docs/53 §10](../../../SnapClip/docs/53-hdr-capture.md#10-第三批实施记录p3-c2026-11-03)。Yue 本批仅同步文档；实际平台代码改动在 SnapClip。

WGC 单帧、session 创建和 resize 共用一个 descriptor→pool-format 映射；session／frame 保留原定义，真实纹理要校验。provider 将请求定义传给冻结帧，但**生产仍显式请求 SDR**，真实 HDR 捕获未测。D3D 上传、目标、swap-chain 颜色空间，以及 D2D 的 source／target／blur／export 已支持显式 FP16/scRGB；不是仅换 WGC 枚举。

捕获 scRGB 原样绘制，UI 色只做一次 EOTF 与 `SDR-white/80`，Alpha 独立。私有颜色模块统一画刷、动态颜色、标注、渐变、小型 SDR UI 缩略图与图标目标；白点变化重建资源。bitmap 属性的 COM 颜色上下文由明确生命周期释放；图标缓存使用完整颜色 bits，准备失败不发布部分结果。

新增 GPU 测试发现并修复放大镜中心被辅助色带染色的问题：修复前 R=0.1776123，源 R=0.25；修复后中心格重绘真实 FP16，不能用八位样本或辅助色覆盖。没有可确认的输出白点就拒绝 HDR overlay；旧八位 sampler、layered 倒计时和生产滚动均保持明确能力门禁。

| 验证 | 结果 |
| --- | --- |
| 普通 workspace lib | **1,196** 项通过：App 268、Capture 556、Clipboard 11、History 43、Model 26、Pin 71、UI 221；ignored 另计 |
| App／Pin headless lib | 288／72 通过，另 5／4 ignored |
| Model／Capture／History integration | 6／6／5，17 项通过 |
| 新增普通测试 | 4 项通过：WGC 格式／纹理、UI 白及能力门禁、缩略图解释 |
| 新增 HDR GPU | Debug／Release 各 **3** 项通过：原始高亮／负值、CPU／GPU source、mask／blur、FP16 标注；14 图标／Alpha／白点 80、160、240 nit 及恢复 SDR；放大镜中心原色 |
| 既有 D2D／D3D 回归 | 最终 28／6 项通过；更宽的 D2D 筛选 31 项包含 probe 与 codegen，不当作 31 项像素断言 |
| all-targets check／fmt／diff | 通过；UI gallery 原有 9 项 must-use warning，无本批新增 rustc warning |
| strict Clippy | Capture `--no-deps` 仍失败 41／71 项，较前一批 49／79 减少 8；包含依赖时在 UI 原有 16 项失败，App 既有失败未处理，没有新增 allow |
| 门禁 | 测试数量基线及 4 项脚本回归、依赖方向、desktop opt-in、注释引用、ledger／符号／PowerShell BOM 均通过 |

日志在 SnapClip `target/hdr-p3c-*`，最终 workspace、GPU Debug／Release、D2D、check／fmt 使用 `*-final.log`。单包默认 Capture 基线 550／81 与 workspace 的 552／81 差两项，是既有 `test-support` feature unification；本批新增 4 项普通、3 项 opt-in，不是删除旧测试。

这批是内部数值验收，**没有测真实 HDR 屏、WGC HDR 捕获、DWM 呈现、DPI／跨屏、45MP、RSS／VRAM 或 20 次资源替换残留**。FP16 每像素八字节，source、双缓冲、blur、export、staging 和 CPU 结果可能同时存活；不能只按最终选区大小宣称低占用。没有性能提升结论，也没有开启 HDR 用户入口、切系统 HDR、实际截屏或改剪贴板。

下一步接真实源能力／输出白快照、来源变化策略，再推进生产滚动的原始行带／固定分析派生、HDR PNG 及保存／贴图／历史消费。HDR 倒计时与 HEX 映射仍需实现；旧宿主的明确拒绝是能力边界，不是完成了该功能。
