# Showcase 多页面滚动：开发构建配置优化

## 问题与处理范围

用户反馈多个页面滚动卡顿。排查时正在使用 `target/debug/windows-showcase.exe`。此前根 workspace 已将 Yue 组件编译为 opt-level=2，但 GPUI、Base、Windows 后端和 Taffy 仍使用默认 opt-level=0；这些依赖共同承担各页面的事件处理、布局和绘制工作。

本轮联合调整开发构建中的 `gpui-pre`、`gpui-base`、`gpui-pre-windows` 为 opt-level=2，`taffy` 为 opt-level=3。应用仍为 opt-level=0；保留 debug assertions 和 overflow checks。未改滚动速度、滚动条动画、布局、数据、虚拟工作集或交互代码。该配置只作用于本 workspace 的 dev profile，不改变 release profile，也不会自动传递到依赖本组件库的其他应用。

前后对照证明这组配置能降低所测页面的滚动 CPU 成本；未拆分四个依赖各自的边际贡献，也不能据此认定原生卡顿的所有原因已经排除。首次重编这些依赖会增加开发构建时间。

## 固定采样与正确性约束

使用真实 Showcase 的 Button、Table、List 页面，在 headless GPUI 窗口中执行真实 `ScrollWheelEvent`。窗口为1280×800，rem=16，固定 light theme，并在两版中都开启 reduce motion，以隔离动效时间差。轨迹每步12px，连续40步向下、40步反向回到起点。80步预热后，计量240步。

计时包含事件 dispatch、完整 `window.draw` 与 `clear`，effects 在计时外排空。仅初始化时 refresh，没有每帧用 refresh 强迫无效重绘。测量前检查页面实际控件的 y 坐标向上移动，最终检查反向滚动回原位置，误差不超过1px。每个有效步骤的绘制 quad 数在所有版本、所有轮次逐点一致，避免因少画内容得到虚假加速。

两个 EXE 先分别构建、复制并记录 SHA-256。采样器不编译；每页面三组 ABBA，共36个独立进程、8640个有效计时样本，固定 SEED=0、ITERATIONS=1。每轮必须实际执行恰好一个成功的 ignored probe；编译、Clippy 或原生 Showcase 干扰时，整轮弃样并保留日志。门禁不能排除机器上的所有其他负载。

采样开始前，接受门限已固化在 `tools/sample-showcase-scroll.py`：**每页面、每组的 P50 与 P95 均至少下降20%**。先分别计算每进程的 floor((240−1)q) 分位数，再取同组两个同版进程的中位数进行门禁比较；下表展示每版本六个进程分位数的中位数，单位ms。

| 页面 | P50 前 / 后 | P95 前 / 后 | P95 降幅 |
| --- | --- | --- | --- |
| Button | 16.91 / 2.82 | 22.54 / 5.46 | 75.8% |
| Table | 54.08 / 13.37 | 66.07 / 19.47 | 70.5% |
| List | 68.03 / 11.43 | 97.28 / 20.37 | 79.1% |

18个页面／组／分位数门限全部通过，最小降幅为70.0%。正式采样共弃样5轮，原日志全部保留；外部 Cargo 测试导致等待和弃样，没有结束外部进程。

原始初诊只测原配置，Button／Table／List 的 P95 分别为22.72／65.31／82.39ms；初诊不混入正式 ABBA 汇总。

## 验证与实际使用

滚动条专项14项通过；Showcase 76项通过、10项 opt-in 测试忽略；严格 Clippy all-targets 通过，覆盖组件、Showcase 与最小消费者。默认构建的原生 Showcase 也在正式采样结束后重新构建，核对其框架与应用的有效 dev profile 与所测候选一致。

重新运行 `cargo run -p windows-showcase`，或启动本次更新后的 `target/debug/windows-showcase.exe`，才能使用新配置。无需修改消费者的 API。

**尚未验收：原生 GPU 完成、输入到屏幕延迟、呈现 FPS、真实鼠标滚轮／触控板／拖动滚动条手感。**本测试使用真实页面和真实事件路径，但运行于 headless 平台，包含测试观测成本；结果是 CPU 工作量对照，不能换算成真实 FPS。普通滚轮的离散步进与 CPU 耗时也应区分，本轮没有新增平滑滚动引擎。真机体验由用户验证；若仍卡顿，应按具体输入方式采原生链路，继续定位，不能直接套用本表宣称问题已全部解决。

## 证据与回退

原始字节源码集包含测试、采样器、Cargo 配置及相关框架源码。原配置只替换根 Cargo.toml，重算后与初诊源码身份一致；正式比较同时冻结源码、两个 EXE 和构建记录，结束时复核未变化。证据包含初诊、正式原始日志／CSV、构建有效 profiles、测试／Clippy日志、固定门限、源文件与离线复核器，不打包大体积 EXE。

初诊原配置源码集 SHA-256 `617c7e3be9321669855a867d9e3b17ce7461ad0c9570af44ca622d4fcfd551a0`；正式候选源码集 SHA-256 `57bf0765466ac7e2b174a60ff26d441dd07d66238770816317b01851fab245ed`。

[完整证据归档](evidence/2026-11-11-shared-scroll-dev.zip)，SHA-256 `dc526b6734b4069bfff6141622bbb0c59359e8a267001a42986c673dd8324e5b`。归档前及解压后均已实际运行离线复核，通过。

解压证据归档后运行 `python tools/verify-showcase-scroll.py <解压目录>`，核对文件／源码／构建身份、初诊、36进程序列、完整样本、CSV对账、逐点 quad 工作量、原生开发构建配置及18个改善门限。优化提交可独立 `git revert`；回退配置后也需重新构建原生 EXE。
