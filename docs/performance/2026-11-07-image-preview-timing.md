# ImagePreview 分阶段耗时与加载优化

2026-11-07，本地目录 `C:\Users\seeyuer\桌面\新建文件夹` 的 50 张真实图片，包含 sRGB、HDR PNG、长截图；最大图片为 3420 × 25390，约 8680 万像素。以下是 Windows 本机 CPU 测量，不是实窗显示时间或 GPU 帧率。

## 开启统计

在启动进程前设置环境变量；统计默认关闭。由用户启动 Showcase 并检查实际窗口：

```powershell
$env:YUE_IMAGE_PREVIEW_TIMING = '1'
cargo run -p windows-showcase 2> target/image-preview-timing.log
```

单图、多图使用相同开关。每条日志带 `[ImagePreview timing]` 前缀，使用毫秒。宿主日志含会话 generation、线程、阶段、文件名；组件日志含操作、源尺寸、纹理尺寸及内部阶段。改变环境变量后需要重启进程。

| 环节 | 日志字段 |
| --- | --- |
| 目录与文件身份扫描 | `directory_scan` |
| 缓存命中／未命中、写入 | `cache_hit` / `cache_miss` / `cache_write` |
| 等待可用工作槽 | `thumbnail_queue_wait` / `source_queue_wait` |
| 等待后台执行器 | `*_executor_wait` |
| 读取压缩文件 | `file_read` |
| 图片解码 | `thumbnail_decode` / `source_decode`；内部 `decode` |
| PNG 头与颜色解释 | `png_rows` 的 `header_profile_ms` |
| PNG 解压、行滤波还原 | `decompress_unfilter_ms` |
| RGB/RGBA 展开、颜色转换 | `expand_color_ms` |
| 方向纠正与像素校验 | `orientation_ms` / `validate_pixels_ms` |
| 纹理预算与缩放 | `prepare_texture` 的 `budget_ms` / `resize_ms` |
| 通道与绘制资源构造 | `bgra_conversion_ms` / `render_resource_ms` / `float_resource_ms` |
| 后台结果交给 UI 任务的等待 | `*_publish_wait` |
| 写入预览状态 | `*_ui_apply`；组件 `set_document` |
| 切图到低清／主图资源就绪 | `selection_to_preview_ready` / `selection_to_full_ready` |
| 文档更新到首次有效绘制提交 | `document_to_paint` |
| 资源第一次绘制 | `first_paint` 的 `resource_age_ms` / `paint_submit_ms` |

`publish_wait` 的起点记录在后台工作结束时，终点为 UI 任务应用结果之前，包含任务恢复等待。组件 `document_to_paint` 的起点为状态更新，终点为首次成功提交有效图片绘制；已在 atlas 中的资源再次被选择也会统计。`first_paint` 每个新资源只记一次，胶片条与主画布共享此记录。

绘制耗时包含 CPU 绘制／上传调用，**不代表 GPU 已完成渲染或显示器已呈现该帧**。本轮不启动 Showcase；真实窗口由用户检查。

缩略图全部处理结束、主图预取空闲、替换图库时输出汇总：`count / sum_ms / avg_ms / p95_ms / max_ms / slowest`。P95 使用最近至多 2048 个样本，其他指标覆盖整个会话；诊断内存有界。并发任务和内外层阶段会重叠，不能将各阶段 `sum_ms` 相加当成总等待；总等待看 `wall_ms`。失败返回也计入宿主阶段，组件提前返回标记 `status=incomplete`。

## 实测与优化依据

| 变更阶段 | 50 张无缩略图缓存准备时间 |
| --- | ---: |
| 串行基准、原开发构建配置 | 15.69 秒 |
| 4 个缩略图任务、原开发构建配置 | 5.71 秒 |
| 增加细分统计后的基准 | 5.98 秒 |
| 仅优化 PNG／fdeflate 的开发编译 | 2.86 秒 |
| 增加整数传递函数查表 | 2.52 秒 |
| 同时优化组件 CPU 转换／面积滤波的开发编译 | 0.56 秒 |

最后一次 50 张已存在缩略图缓存的读取约 19.74 毫秒。每阶段各测一次，数据用于找瓶颈，**不是稳定分位数性能保证**。无缩略图缓存准备跳过应用缩略图缓存；文件系统缓存没有清理，并非冷磁盘测试。测量不含源码编译、UI、主图并发竞争或缓存写入，因此实际首次图库完整可见时间应查看运行时 `all_thumbnails_settled` 的 `wall_ms`。

最初分阶段记录中，50 张文件读取累计约 20 毫秒，解码累计约 15.67 秒，缩略图缩放累计约 1.26 秒。普通 RGBA 长图主要耗在解压，RGB PNG 还耗在通道展开，HDR PNG 耗在颜色转换与 FP16 校验。由此针对性处理：

- 缩略图由 4 个任务并行，主图单独工作；当前选择优先，随后预取邻图。主图缓存保留 6 张，淘汰后降级为缩略图，首次未完成缩略图的资源也能释放。
- 缩略图只生成 256px、有界采样的快速代理，主图仍使用精确线性光面积滤波。原尺寸、方向、透明度和 HDR 编码继续保留。
- 8 位 sRGB PNG 直接展开字节，不绕行浮点转换；面积滤波复用 256 项 sRGB 线性化表。16 位 sRGB／PQ 输入复用传递函数表，覆盖全部 65536 个码值的测试与原公式逐项一致。
- `Cargo.toml` 在开发 profile 中为 `png`、`fdeflate` 开启 `opt-level=3`，为 `yue-component` 开启 `opt-level=2`。这会增加这些包的构建优化成本，使 CPU 热路径在开发模式也接近生产性能；Showcase 宿主仍保留普通开发配置。Release 无需依赖这些覆盖项。

主图链路也单独测量了普通图、HDR 图和长图。最大长图的精确缩放从约 3402 毫秒降到 424 毫秒，解码约 229 毫秒；HDR 样本解码约 101 毫秒，主图准备约 358 毫秒。主图预算为长边不超过 2048px、存储不超过 12MiB；代理不提供原图全部细节。

本轮日志保存在 `target/image-preview-profile/`。自动化验证覆盖资源合同、全部 16 位码值颜色换算、图库优先级、缓存损坏与失效、快速连续切图和资源升级时动画连续性。

## 重跑 CPU 探针

不需要启动窗口，也不会删除既有缓存。探针依赖 Showcase 本地输入目录，默认忽略；缩略图探针跳过缓存进行解码，然后单独读取已有缓存；主图探针选择首图、HDR 样本和最大长图：

```powershell
$env:YUE_IMAGE_PREVIEW_TIMING = '1'
cargo test -p windows-showcase profile_local_gallery_ -- --ignored --nocapture --test-threads=1 2> target/image-preview-probe.log
```

检查真实交互时，先看是否缓存未命中，然后看排队、解码、缩放、UI 交接、`document_to_paint` 哪一项增长，再对那个环节调整。正常运行删除 `YUE_IMAGE_PREVIEW_TIMING` 环境变量即可关闭日志。
