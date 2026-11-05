# 五项虚拟滚动实施记录（2026-11-05）

用户授权将[下一轮候选](2026-11-05-virtual-list-next-opportunities.md)全部实施，包括冻结表头／首列。起点为 `df0f2117aa27985e0069d3cc21788e9b93f487f7`，保持当前质量分支；参考目录只读。沿用 GPUI Kit coding/design guides 和 feat 实施流程。

本文是本轮提交与实测的历史记录。后续 Grid cell 保留、span、约束 auto、RTL 和一维 Measured 受限动画已另行交付，见[五项后续实施](2026-11-05-virtual-list-five-followups.md)；当前支持矩阵见[组件文档第 2.5–2.6 节](../28-virtual-list.md#26-布局选择与二维边界)。下列测试计数、采样和回退范围不因文档同步扩展到后续提交。

| 项目 | 状态 | 验收 |
| --- | --- | --- |
| Grid 内容准备复用 | 已实现；ABBA 两轮 | 内容身份、失效、生命周期、几何独立；复杂／轻量单元格 ABBA 与默认守卫 |
| Masonry 活动／方向／idle | 已实现；专项通过 | 用户输入与几何补偿区分、结束恰好一次、卸载取消、无逐项任务 |
| 有界方向性缓冲 | 已采用；两轮最终 ABBA | 默认不变、保留反向余量、换向／idle／双轴／焦点完整；ABBA |
| Masonry 冷恢复缓存 | 已采用为显式功能；原位路径两轮 ABBA | 纯值、排版／内容／分栏身份、原子失败、阅读锚点和重测收益 |
| 冻结前缀行／列 | 已实现；headless／原生／视觉通过 | 一个 owner、唯一 cell、分区裁剪／命中／焦点／定位、空集合和极小视口 |
| 最终集成、文档与推送 | 全部验收完成；Git 分支交付 | workspace、Clippy、contracts、Windows Release／native、来源和回退核对 |

每项独立提交。性能候选使用相同冻结 workload 与隔离 Git vendor 基线，ABBA、依赖身份、编译干扰守卫和原始样本共同判断；收益不足的方案撤回，保留记录。功能新增不伪称 CPU 提速。headless 计时不当作 GPU FPS 或物理输入延迟。

冻结区设计由 feat 要求的只读 UI/UX 设计审查辅助：body、冻结行、冻结列、corner 分区；共享唯一 native offset，冻结 cell 不复制，互斥 mask 防点穿；不永久改写调用方 axis scroll padding。冻结行列按可见范围裁剪，不能因巨大前缀创建全量元素。

## 实现与可回退提交

| 提交 | 变更 |
| --- | --- |
| `2f49338` | Grid 工作集内内容准备 View 复用；默认 renderer 无该缓存分配 |
| `4121c0d` | Masonry 活动、方向和 120ms idle；默认 idle 不分配活动对象 |
| `3778def` | Fixed/Known/Measured、Grid 两轴、Masonry 的 opt-in 方向性 overscan |
| `7805f51` | Masonry schema-1 纯值缓存、明确 lane 身份和 key 阅读锚点 |
| `2a55718` | Grid 冻结前缀、四区裁剪／命中／定位、Showcase 控件 |
| `c3d1352` | 未冻结 Grid 保留普通 frame 存储，不分配 cell mask 数组 |
| `8126b9d` / `dac852d` | 固定 workload、原生 overdraw 合同、真实 idle 与依赖／源码身份守卫 |

通过 Git 独立提交保留各实现。移除有依赖的能力时按提交逆序 revert；不使用 hard reset 或 force push。最后以独立临时 index 逐个逆向应用提交，核对完整 tree 回到起点，主 index 不变。

## 已完成的正确性与窗口验证

- 新 API 在旧源码上产生明确缺失能力的失败基线；方向性测试修正自身 ScrollbarHandle 调用后，失败仅来自缺少配置 API。
- Grid cache 三项初始接缝覆盖重绘复用、双轴局部失效、尺寸独立、外部 revision、owner 更换、工作集释放和卸载释放；冻结后补充 corner 实体跨切换／滚动保留与 unfreeze prune。
- 有界方向性专项覆盖三种 sizing、换向、idle、非法配置原子性、retained、Grid 两轴与 Masonry；原生 Measured renderer roots 实际缩小，不能只改 snapshot 范围。200px/40px 对照中 Grid 325→170 cells，3-lane Masonry 75→51；所有可见项存在。
- Masonry 活动测试覆盖真实 wheel（边界无位移也开始活动）、方向、稳定观察不续时钟、120ms 恰好一次结束、取消程序命令、卸载 task 清理；不增加稀疏工作集的伪连续 RangeChanged。
- 冷恢复覆盖 controller 销毁／新建、weak 释放、lane／key anchor、删除／新增／内容 revision、宽度／列数／layout revision、NaN／重复／非法 lane 的原子拒绝、旧 snapshot 迟到反馈。差估算头部重建的内容布局调用 165→30（内部 pass 计数，非显示帧）。
- 冻结专项覆盖 Fixed/Known、分数尺寸、边界 gap 防点穿、四区命中、唯一 Grid 语义、共享 wheel owner、focus、尺寸更新、Start/Center/End/Nearest、冻结 key 不回零、原生 offset 不变、unfreeze 后恢复原对齐；百万项前缀与空 source 有界，完全隐藏目标／零 body 正确结束。
- 最终 workspace（含 doctest、Showcase headless）：**867 passed / 0 failed / 12 ignored**。Virtual list 子集 **153 passed / 8 ignored**。Clippy workspace/all-targets，features `headless-tests,virtual-profile`，`-D warnings` 通过；fmt 通过。
- vendor 合同独立 fixture：**16 native + 2 Base 全通过**；生产 manifests/locks 保持不变。新增 native overdraw 用例先清除窗口初始 draw 的观察值，再验证 40/200px 工作量与旧 setter 恢复双侧 200px。
- Windows Release 构建通过，三项独立进程原生探针通过：冻结／缓存恢复、原有布局、smooth 采样／收敛。实际 Win32 dispatcher、字体与 renderer，不能换算为 GPU FPS 或硬件输入延迟。
- computer-use 技能操作本轮临时 Release 窗口：初始冻结、双轴 wheel、定位百万行右下角、Masonry 滚动与缓存后重建。视觉核对首行／首列／corner 位置、边界裁剪，以及卡片 8 等顶部阅读内容重建前后保持；完成后关闭该临时窗口。自动化分数尺寸／gap 测试与真实窗口视觉检查分别记录，不把固定尺寸截图说成覆盖所有缩放比例。

## 性能判定与边界

所有时间是同一台 Windows 主机的 Release CPU/headless draw 周期或冷重建计时；不是显示刷新率，也不包含物理输入设备延迟。ABBA 同一 workload、依赖版本与生产 vendor 路径隔离，采样前两次 quiet 检查，采样期间有编译干扰则丢弃重试，不终止其他项目进程。最终守卫使用固定生产源码，启用能力的比较与默认路径分开。

Grid prepared workload：payload=64 的 steady／rapid／resize／content，首轮 CPU 中位变化依次 −48.10%／−23.54%／−46.86%／−46.31%，复测 −47.00%／−22.22%／−44.76%／−44.39%。steady factory 5184→24，rapid 4000→2000，resize 5472→144，content 5184→342；64-field steady 分配 444604→94028。轻量 payload=0 rapid 则 **+2.82%／+3.38%**，因此保留为 opt-in，不宣称全场景提速。

初始 directional / directional-repeat CSV 中名为 idle 的场景实际没有推进 TestAppContext 时钟，是 stationary-active；这些原始样本原样保留，不能计为 idle 收益。修正 workload 后显式 poll idle；directional-verified 的移动 CPU 中位变化约 −47%～−49%，真实 idle −3.47%，仅作近似不变观察。最终两轮另行记录，不覆盖这一试验纠正过程。

Masonry 冷恢复重建计时包含新建 controller、restore 和实际 draw；cache export 另行记录，缓存全 source 的 lane/hint 仍是 O(N) 存储。差估算（2px hint、20px 实际）只是压力场景；另测准确 20px hint。即使少建了内容根节点，也可能被全量元数据恢复成本抵消，不能据节点计数宣称所有规模的 CPU 收益。

### 冷恢复的第二次取舍：先测出退化，再修复

原始 `masonry-cache` / `masonry-cache-repeat` 不利样本保留：300 项、2px 差估算 CPU −62.64%／−60.50%，但 10,000 项差估算 **+90.08%／+92.08%**，准确 20px 估算 **+314.08%／+340.37%**，后者工作根节点完全相同。临时峰值 496,128→2,026,976 bytes。不能据 3300→600 次 root 调用将全量目录重建判为合格。

随后独立提交 `f0250b7` 增加相同 key 顺序和 lane 映射的原位恢复：全量验证在任何写入之前完成，复用原有 lane/source 目录，只更新变化的尺寸块；source／历史分栏变化继续走完整重映射。末尾非法记录的原子失败与 historical lane fallback 都有回归。没有删掉兼容性校验换取速度。

| 冷重建场景 | 原位首轮 CPU | 原位复测 CPU | 临时峰值前→后（bytes） |
| --- | ---: | ---: | ---: |
| 300 项，2px hint / 20px 实际 | −68.21% | −64.60% | 270268→39636 |
| 10,000 项，2px hint / 20px 实际 | −41.44% | −42.99% | 496128→496128 |
| 300 项，准确 20px hint | −9.13% | −4.06% | 39636→39636 |
| 10,000 项，准确 20px hint | **+36.78%** | **+24.77%** | 496128→496128 |

采用决策：**保留显式冷恢复功能和原位优化，拒绝把恢复自动应用于每次普通重建。** 大集合、准确 hint 且无需阅读位置／历史分栏恢复时，直接构造更便宜。全量源验证仍有成本，缓存不是普遍提速开关。一般重映射含 O(N log(lanes)) 分栏及 N 次 source 查询；若反查为 O(log N)，还需计入该来源的查询成本。相同 slots 路径顺序校验／遍历 N 项后只写变化块。

导出独立于上述冷重建计时：每个整数-key slot 为 72 bytes，300／10,000 项 payload 为 21,600／720,000 bytes（不含 Vec header、anchor／字符串 key 的额外所有权）。第一轮 exporter 分配 2 次、线程内临时峰值 24,000／800,000 bytes；不同样本导出时间保留在机器记录，不能漏算或声称仅与已测项数有关。

### 方向性缓冲的最终判定

| 场景 | 最终首轮 CPU | 最终复测 CPU |
| --- | ---: | ---: |
| 前进 | −48.90% | −48.35% |
| 后退 | −49.54% | −46.89% |
| 交替换向 | −48.86% | −47.50% |
| 静止但仍 active | −50.05% | −49.91% |
| 真实 idle | −1.88% | +0.52% |

采用为 opt-in：前向提前量不缩，反向保留有界余量；真正 idle 的 build 和 allocation 计数完全相同，时间近似不变。此处是 200/40px 固定预留对照，不能推广为任意设置或所有内容均快一半。

### 状态存储和默认代价

同一 Windows x86_64 Release `size_of`：linear controller 944→968 bytes，Masonry 560→576，layout 40→48，viewport 64→80。新增 inline 字节来自方向策略、帧级冻结约束和可选活动指针；这是值尺寸，不是 RSS。默认 idle 无活动 Box／task，活动期付出一份 Box 与一个 Entity task；Grid prepared／partition masks 都显式启用，不增加逐行任务或第二滚动 owner。

默认矩阵早期 `five-default`／复测保留全部样本，不利项包括 masonry-noop +3.68%／+3.84%，其余部分场景正负波动；不能宣称默认路径普遍零回归。原位冷恢复修改后的固定最终源码再跑 `five-final-default`／复测，结果与全部 CSV 指纹收录机器记录。计数一致的场景不把偶然负百分比当作已证明加速。

### 最终默认矩阵（起点 df0f211 → 固定最终生产源码）

下表为每个 case 的样本总时间中位数变化，保留正负号，不把不同工作量的百分比汇总成一个“整体提速”。0/1/2/3 分别为 Fixed/Known/Measured/Masonry。

| case | ABBA 1 | ABBA 2 |
| --- | ---: | ---: |
| `append-10000` | +8.74% | -0.92% |
| `append-100000` | -4.78% | +7.66% |
| `append-1000000` | -12.15% | -4.63% |
| `frames-0-complex` | -2.18% | -2.16% |
| `frames-0-rapid` | -3.91% | -4.26% |
| `frames-0-resize` | +11.04% | +0.09% |
| `frames-0-steady` | -13.61% | +9.21% |
| `frames-1-complex` | -5.31% | -7.40% |
| `frames-1-rapid` | -0.66% | -2.73% |
| `frames-1-resize` | +0.21% | -2.10% |
| `frames-1-steady` | -1.81% | -7.21% |
| `frames-2-complex` | -5.93% | -2.85% |
| `frames-2-rapid` | -3.64% | -1.66% |
| `frames-2-resize` | -1.38% | -2.34% |
| `frames-2-size` | +3.46% | +0.73% |
| `frames-2-steady` | -2.86% | -3.79% |
| `frames-3-complex` | -3.56% | -2.69% |
| `frames-3-rapid` | -2.92% | -2.94% |
| `frames-3-resize` | -3.83% | +0.16% |
| `frames-3-steady` | +0.89% | -0.01% |
| `idle-2000` | -1.76% | +0.07% |
| `masonry-noop` | +3.43% | +0.33% |
| `measured-queries` | -8.69% | +9.02% |
| `work-0-false` | -0.36% | +0.18% |
| `work-0-true` | +0.11% | -3.49% |
| `work-1-false` | +2.46% | +0.06% |
| `work-1-true` | +1.89% | +0.42% |

Fixed resize 首轮 +11.04%，复测 +0.09%；其原始 before/after 样本区间相互重叠，不能据一轮认定稳定退化，也不删除该不利样本。masonry-noop 最终 +3.43%／+0.33%；Measured size +3.46%／+0.73%。部分负数在复测反号（例如 Fixed steady、Measured queries），同样不能当成可归因收益。采用判断主要依赖 opt-in 工作量减少、复杂内容两轮收益、冷恢复原位实测、行为完整性和公开成本，不包装为所有默认场景都更快。

## 复现、来源与维护

原始 CSV、每轮 source/fixture/lock/依赖记录、附加导出与 controller-size 观察、验证日志哈希和回退 tree 记录汇总在[机器证据](virtual-list-next-implementation-2026-11-05.json)。数据全量保留，包括被修正标签、优化前冷恢复和早期默认样本；只使用明确合格的场景作采用结论。

```powershell
cargo test --workspace --features headless-tests --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1
cargo clippy --workspace --all-targets --features headless-tests,virtual-profile --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- -D warnings
python tools/test-virtualization-contracts.py --offline --target-dir C:/Temp/gpui-yue-owner-perf-20261104
powershell -NoProfile -File tools/test-virtualization-native.ps1 -Offline -Release -TargetDir C:/Temp/gpui-yue-owner-perf-20261104
python tools/compare-virtualization.py --before df0f211 --label five-final-default --date 2026-11-05
python tools/compare-virtualization.py --before 4121c0d --label directional-final --date 2026-11-05 --workload docs/research/virtual-list-directional-2026-11-05.rs --filter directional_workload --after-feature directional-overscan
python tools/compare-virtualization.py --before df0f211 --label masonry-cache-inplace --date 2026-11-05 --workload docs/research/virtual-list-masonry-cache-2026-11-05.rs --filter masonry_cache_workload --after-feature masonry-cache
```

每项性能命令另以 `-repeat` label 再运行一次。Grid prepared 的两轮命令见其 CSV 对应 identity，before 为 df0f211、after-feature 为 grid-views。比较器使用仓库外 fixture，A/B 依赖 package 记录必须相同；manifest/lock SHA 和生产 vendor src SHA 均写入 identity，发现依赖不一致时不采样。生产锁不变。早期旧版 identity 没有这些附加字段，不能伪称其已运行新版守卫；相应证据以已有原始采样与当时的依赖核对为限。

对标来源仍为只读 `refer/virtual` 三项目：TanStack Virtual 的 scrolling/direction、snapshot/lanes，virtua 的缓冲／滚动活动与内容职责分离，vlist 的 pipeline／idle 事件。候选推导和已读源码位置见[本轮调研](2026-11-05-virtual-list-next-opportunities.md)与其机器来源；参考源码 LF SHA 在本轮机器记录再次核对未变。GPUI Base/Yue 保留自身一个 owner、真实测量／Entity 生命周期和既有焦点／布局合同，不移植第二套 store/runtime。

在本轮结束时，Grid auto 轨道、cell span、精确单个 cell 保留和 RTL 尚未交付；这些能力已在[后续实施](2026-11-05-virtual-list-five-followups.md)中采用。当前仍不支持通用二维／三维空间布局、Grid Measured 轴和未知尺寸长路径的连续 smooth；一维 Measured 的已测短路径及显式末段动画不能等同于全程动画。

## 最终验收与回退核对

原位恢复提交后再次完成整个 workspace：867 passed / 0 failed / 12 ignored；Clippy 严格检查、fmt、16+2 vendor 合同、Showcase Release build、三项 Windows Release 原生用例全部通过。本轮没有改生产 Cargo manifests/locks，没有改参考项目，没有终止其他项目编译。

在独立临时 Git index 从 `924acc7` 开始，按真实提交逆序逐个 `git apply --cached --reverse` 验证了本轮 **10 个实现／测试提交**。得到 tree `7dc4c275ccfda96623ebdc3900c63eea01c5ff90`，与 `df0f211^{tree}` 完全相等；主 index 哈希保持不变，临时 index 随后移除。该验证不改工作区、不切分支、不 hard reset，也不接受／清理任何用户改动。文档和数据在单独交付提交中。

分支为 `quality/virtualization-four-priorities-20261104`。需要撤销能力时先检查后续依赖并按提交逆序 `git revert`；不 force push。完整提交列表、回退验证的 head／tree、当前生产源 SHA 与每轮采样身份在机器证据中。
