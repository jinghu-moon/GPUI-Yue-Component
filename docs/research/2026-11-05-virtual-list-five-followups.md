# 虚拟化五项后续实施（2026-11-05）

本轮起点 `7fb2c29`；按冻结组合、稀疏 cell、span、Measured smooth、auto/content/RTL 顺序实施。参考代码只读，生产依赖清单和 lock 不变；每项独立提交。headless draw/CPU 不是 GPU FPS 或输入延迟。

## 1. 冻结组合与成本

新增 24 轮连续双轴 Known 反馈、视口 resize、冻结切换、命令与真实 wheel 接管回归，并检查四区内部 row-major 绘制顺序。冻结专项旧实现与候选均 10 项通过。已有 source 变更、正文消失、焦点和准备生命周期测试继续保留。

原冻结路径收集全部 R×C cells 后稳定排序。改为只暂存 R+C 轨道，分割冻结前缀，以四个矩形按原顺序流式遍历。普通未冻结路径不增加轨道 buffer。

用固定 [workload](virtual-list-frozen-anchor-2026-11-05.rs)、基线 `7fb2c29` 两次 Release ABBA。每版每场景 6 samples，sample 为 160 个计时 draw 周期，中位数 CPU 变化如下，负数为改善。

| 场景 | 第一轮 | 复测 |
| --- | ---: | ---: |
| 未冻结 steady | +0.92% | +0.77% |
| 未冻结 scroll | +1.80% | -2.24% |
| 未冻结 prefix | -3.71% | +0.45% |
| 未冻结 hidden | -0.68% | +0.77% |
| 冻结 steady | +1.72% | -1.72% |
| 冻结 scroll | -1.28% | -3.21% |
| 冻结 prefix | +1.06% | -1.14% |
| 冻结 hidden | -2.62% | +0.10% |

冻结四场景分配均减少 960，窗口内增量 peak 均减少 3328 bytes。所有场景 reads/builds、reading_error 完全一致（误差均 0）；非冻结分配及 peak 完全相同。按有确定证据的空间收益吸收，不声称 CPU 普遍加速或零回归。

完整不利样本见 [第一轮](virtual-list-series-frozen-stream-2026-11-05.csv) 和 [复测](virtual-list-series-frozen-stream-repeat-2026-11-05.csv)。重现时选择新的唯一 label，例如：`python tools/compare-virtualization.py --before 7fb2c29 --label frozen-stream-reproduce --date 2026-11-05 --workload docs/research/virtual-list-frozen-anchor-2026-11-05.rs --filter frozen_anchor_workload`；复测再次替换 label，工具会拒绝覆盖既有证据。隔离构建 identity 和编译静默检查由比较器记录在 `C:/Temp/gpui-yue-series-20261104/` 对应目录。

## 2. 精确单 cell 保留

`VirtualGridCellKey` 以两轴稳定 key 标识；`retained_cells` 为普通／冻结／旧轴保留工作集添加稀疏并集。重复、已可见、已冻结的 cell 去重，删除任一轴 key 后忽略。不把这些 key 写入两轴 retained，因此两个离散 cell 不产生额外交叉。只缓冲 K 个 extras，与原四区有序迭代器合并；默认不分配额外 cell map。

`cell_focus_handle` 只在 handle 包含焦点时保留该 cell，不能把同一 handle 登记给两处。GridMount 记住上一帧注册，在删除、注销或更换 owner 时先聚焦显式 `focus_fallback`（否则 blur），再修剪 View。Prepared stamp 仍遵循原 content revision/index 合同；持久编辑数据／实体由消费者按 domain key 持有，不能依靠任意工作集缓存永久保存业务数据。

新增稀疏并集、重复／缺失、冻结去重、离屏编辑 Entity 与本地状态、删除／注销焦点修复、释放回归。既有 prepared 测试覆盖 content revision、owner 替换、geometry-only 复用和卸载。另补冻结时双轴插入／删除／重排保留正文 key 的组合回归。

固定 [workload](virtual-list-cell-retention-2026-11-05.rs) 对照 `a6c6265`：before 两轴各保留两个远端 key，after 精确保留两对所需 cell。两边都保留需求中的两个编辑位置；旧实现还创建额外行列交点。两轮 Release ABBA：

| 场景 | 第一轮 CPU | 复测 CPU | 160 周期 builds | 分配 |
| --- | ---: | ---: | --- | --- |
| 默认未冻结 | +0.30% | -0.01% | 8000→8000 | 55520→55520 |
| 默认冻结 | +0.71% | +0.51% | 8000→8000 | 58720→58720 |
| 两个远端 cell | -42.87% | -42.87% | 15680→8640 | 107040→61280 |
| 冻结＋两个远端 cell | -41.97% | -40.74% | 15680→8640 | 109280→65120 |

空间 peak 分别 80048→45920、81504→46944 bytes；reads 19200→11520。保留 [原始](virtual-list-series-cell-retention-2026-11-05.csv)／[复测](virtual-list-series-cell-retention-repeat-2026-11-05.csv)。这是减少不必要工作量的收益，不能当成相同 cell 数的遍历提速。比较器新增 `grid-cells` 仅用于临时 fixture feature；生产 manifests/locks 不变。

## 3. 显式列跨格与行跨格

`VirtualGridSpan::new(stable_origin, rows, columns)` 和 Grid 的 fallible `spans(&descriptors,cx)`，按列跨度、再双轴跨度验证。Fixed/Known 轨道尺寸仍由调用方提供；`cell.extent()` 是合并矩形（含内部 gap），`row()/column()` 保持 origin 原轨道，另有跨度 readers。只有 origin 创建一个 View／命中／语义节点，覆盖 slot 不再生成。离屏 origin 只要跨度与工作轨道相交仍会进入工作集；稀疏 retained 落入跨度时映射到 origin。

非正跨度、溢出／越界、重叠、缺失 key、Measured 轴和跨冻结边界，在 mount/滚动修改前返回类型化 `VirtualizerError`。配置先设置冻结再验证 spans；source 事务后每次 render 按稳定 key 重新解析。生成后的 span 配置若在 render 与 request_layout 之间失效会 fail closed，不能静默画旧索引。descriptor 不保留 source，也不建立 N×M map。

验证采用 row 扫线＋活跃列区间 O(S log S)，包含查询用 row-max interval tree 剪枝；每帧扫描 S 个稀疏矩形与轴工作索引交集，不扫描跨度覆盖面积。极端同一 row 上有很多不相交跨度时单次包含查询仍可能访问 O(S)，这里不承诺完整二维索引的 O(log S)。没有把这项新能力宣传为速度优化。

回归覆盖列 span、行 span、offscreen origin、唯一 click、冻结区域、原子拒绝、稳定 key splice 与 Known 尺寸反馈；百万行列中一个 100000×100000 span 只产生一个 cell，source reads 有界。默认路径两轮 Release ABBA 的 allocations/peak/reads/builds 全部相同，CPU 有不利项：未冻结 prefix +0.24%/+5.36%，冻结 hidden +1.58%/+1.96%，冻结 prefix +0.39%/+3.78%。其余场景及全部原始样本见 [第一轮](virtual-list-series-grid-span-default-2026-11-05.csv)／[复测](virtual-list-series-grid-span-default-repeat-2026-11-05.csv)。按明确功能需求与默认空间成本不增长采用，不能声称零 CPU 代价。

## 4. Measured smooth 与独立末段策略

一维 Measured 的 Smooth 只播放已测、有界短路径（viewport 与 max_distance 中较小值，几何检查另受 measurement_budget 限制）。未测或过密路径即时收敛，不扫描／冻结全部沿途项。capability 代表存在受限动画能力。Masonry Measured 仍不播放动画。

可选 `JumpThenSmooth` 仅用于一维 Measured：先跳至目标前方半个有界距离，下一个时间样本在已测路径播放末段，否则即时收敛。Fixed/Known 和 Masonry 类型化拒绝此策略；普通 Smooth 的长距即时行为保留。它不是全程连续动画。

动画状态只在请求时分配；相同 now 重求复用样本，动画不消耗几何重试预算，保留 8 次拒绝和 2 秒总截止。几何改变目标／校正实际位置时，从宿主实际位置按剩余截止时间衔接。五项新增 UI integration tests 覆盖横纵轴 1ms 高刷新率、同 now 重求、尺寸失效、未知尺寸降级、真实 wheel、目标删除、no-op、reduced motion、隐藏、拒绝宿主、超时与一次终态。长距测试检查 builds 有界，不构建沿途 900 行。

试验先后暴露“拒绝宿主仍重设起点返回原地”和“等尺寸新增测量重启段落”问题；拒绝／高刷新率用例先失败后修正。[trial 原始采样](virtual-list-series-measured-motion-trial-2026-11-05.csv) 保留，仅是被拒候选。阶段候选重跑两轮 ABBA，baseline `7237b55`，终点误差均小于 0.01px；[固定 workload](virtual-list-measured-motion-2026-11-05.rs)、[阶段第一轮](virtual-list-series-measured-motion-final-2026-11-05.csv)、[复测](virtual-list-series-measured-motion-final-repeat-2026-11-05.csv) 完整保存。文件名中的 final 是当时的阶段标签；本轮最终源码另见第 6 节 accepted 样本。

第一轮 CPU：idle +3.53%，reduced +1.30%，short +13.64%，tail +12.78%。short builds 1610→1938、分配约 20230→22414、peak 16440→17488 bytes；tail builds 1610→1941、分配 21381→23538、peak 19864→27400。idle 各空间／工作量计数相同，reduced 两个命令多两个 motion 分配。动画的中间位置不同、工作量更多，这是功能成本，不是等画面提速。按用户要求采用并公开代价。阶段专项全量为 170 passed / 0 failed / 8 ignored。

后续组合审查再发现一个截止边界：动画完成的同 now 最后测量反馈不能继续复用旧的“最终对齐位置”。直接 `prepare→update_sizes→prepare` 回归先复现 40px（错误）而不是 50px，再修正为只冻结中间动画采样、完成后仍重求实际目标；另补 Owned 最后一帧真实布局反馈用例。最终整轮验证／重新采样覆盖此修正。

最终审查又补两项先失败的回归：动画开始后，主轴视口从 100px 缩至 20px 时仍播放超出新边界的路径；已测路径随后失效时停在 0px 等待截止而不是即时请求 40px。`e7fa10d` 把已测／距离／视口条件提升为逐时间样本校验，失效则退回现有即时收敛，同 now 中间采样仍只计算一次。新增用例覆盖横纵两轴缩放和直接 prepare 的后续失效；Measured smooth 模块 9 项通过。

## 5a. 约束驱动 auto tracks

采用显式 `VirtualGridTrack(minimum,maximum,weight)` 及纯函数 `resolve`。调用方传入实际可用 extent、gap、两端 padding；按权重分配剩余空间，达到 max 后再分配给其余轨道，不足时保留 min 并允许滚动。零权重固定在 min，达到全部 max 后留白，非法／非有限输入整体拒绝。解析采用 breakpoint water filling，O(T log T)，不查询 cell。结果交给 Known sizing 或现有 `update_sizes` 合批；消费者在约束变化时重算，不能每帧重建百万行 Known 数组。

测试覆盖权重、max 饱和、min 溢出滚动、留白、零轨道、非法输入，以及解析后 Known resize 保持原阅读 key、同值更新不使旧 snapshot 过期。默认 Grid 不调用解析器。这是约束驱动布局，不声称能推断未见内容的 intrinsic 最大宽高。

## 5b. RTL

Grid 的 `right_to_left(true)` 镜像列矩形、冻结列（右侧）、mask 和 hit testing。数据／keys 不反序，列 state 的逻辑 offset 仍从阅读 start 向后递增，Start/End、方向性 overscan、阅读锚点和命令共用此坐标。唯一 ScrollHandle 仍为物理从左坐标，映射为 `physical = max_scroll - logical`。mount 保存的上一 applied pair 只是转换观察，不接收另一套输入／滚动意图；方向切换、宽度／总尺寸变化保持逻辑位置，native resize clamp 不误认成 wheel。

两项专项覆盖四区真实 click、冻结 gap 不穿透、双向 wheel、resize、LTR↔RTL、外部 scrollbar 的物理写入、四种 align、双轴 smooth，以及第 800000000 列上的 0.25px 输入。只有 Grid 获得此镜像适配；没有宣称一维 List／Masonry 或文本输入 Left/Right 语义已整体 RTL 化。消费者继续负责文本方向与键盘语义。

同一逻辑 trace 下 LTR 基线 `69a7008` 与 RTL 两轮 ABBA，各场景 reads/builds 与阅读误差相同（误差 0）。RTL 每 160 周期多 320 次配置分配、增量 peak 多 168 bytes，这是 opt-in builder 配置的实际成本。CPU 以负值为主，唯一反号项为复测未冻结 prefix +0.09%，不据镜像差异宣称普遍速度收益。[workload](virtual-list-rtl-grid-2026-11-05.rs)、[第一轮](virtual-list-series-grid-rtl-2026-11-05.csv)、[复测](virtual-list-series-grid-rtl-repeat-2026-11-05.csv) 保留。Showcase 新增真实 RTL 和 2×3 正文合并开关，headless 测试点击开关并核对唯一合并节点及 432×72px 矩形；原生 motion probe 组合 RTL、冻结、跨格和 Measured 动画。

## 5c. 内容自适应实验与不采用结论

不吸收“每帧对所有工作 cell 再做一遍 intrinsic layout”的通用方案。[实验](virtual-list-content-probe-2026-11-05.rs) 给同一真实 cell 根增加约束前 MaxContent 计算，再由正常 Grid 布局同一根；没有复制隐藏 cell 或测量离屏全集。对照场景的最终轨道、renderer 次数与 source reads 相同；probe 只是测量成本微实验，不是完整生产自适应控制器。

两轮 CPU 分别 **+20.23% / +19.83%**，分配 **135520→159520**、peak **48792→51736 bytes**，160 周期 builds 均 8000，多 8000 次 intrinsic layout；关闭 probe 的控制组 +0.80%/-0.74%，所有计数相同。滚至之前未见的第 900 行后，观测最大宽度增加 **12163px**，实证当前工作集无法保证全局内容最大值。[第一轮](virtual-list-series-content-intrinsic-2026-11-05.csv)／[复测](virtual-list-series-content-intrinsic-repeat-2026-11-05.csv) 保存完整不利结果。开发时先拒绝了错误地在已约束布局之后调用的 probe（只重复得到 50px）；修正为 request_layout 内约束前测量后才采纳实验数据。

因此采用约束 auto，内容尺寸仍由业务显式 hints／版本失效和 `update_sizes` 合批控制。若消费者要“已观察内容的最大值”，必须明确 epoch、预算和容量；epoch 内增长，重置后才允许收缩，不能承诺所有离屏内容已适配。此结论只拒绝本轮逐帧通用测量候选，不声称所有内容适配策略都不值得做。

## 6. 最终接受源码与整轮验证

最终实现提交为 `e7fa10d`；后续归档提交只整理文档与样本。以下六份 accepted CSV 的 after vendor source fingerprint 均与最终测试工作区一致，Git 的 LF 归一化后整套 vendor source 与该提交一致；before/after 的依赖记录也相同。完整身份、日志哈希和分场景统计见 [机器证据](virtual-list-five-followups-2026-11-05.json)。

每组均为 Release ABBA，两轮独立比较，逐轮保留全部样本。默认／Grid 的 baseline 为本轮起点 `7fb2c29`；Measured baseline 为加入动效前的 `7237b55`，两边中间画面和工作量不同。CPU 表格为 sample 总耗时的中位数变化，负数为改善，不是显著性检验，也不是显示帧率。

### 6.1 默认路径

[第一轮](virtual-list-series-followups-accepted-default-2026-11-05.csv)／[复测](virtual-list-series-followups-accepted-default-repeat-2026-11-05.csv) 共 27 类场景。两轮每个场景的 allocations、peak、reads、builds 样本分布与 before 完全一致；CPU 的正负结果全部列出。

| 原始 case | 第一轮 CPU | 复测 CPU |
| --- | ---: | ---: |
| append-10000 | +4.81% | -7.27% |
| append-100000 | -27.36% | -9.13% |
| append-1000000 | -8.90% | -2.36% |
| frames-0-complex | -0.37% | +0.13% |
| frames-0-rapid | +2.32% | -0.01% |
| frames-0-resize | -0.85% | -0.11% |
| frames-0-steady | -6.06% | -13.51% |
| frames-1-complex | +0.80% | -2.43% |
| frames-1-rapid | -1.07% | -0.07% |
| frames-1-resize | +1.09% | +0.50% |
| frames-1-steady | +0.80% | -0.15% |
| frames-2-complex | -1.02% | +0.53% |
| frames-2-rapid | +4.28% | -1.09% |
| frames-2-resize | -0.50% | -2.09% |
| frames-2-size | -4.38% | -3.43% |
| frames-2-steady | +3.20% | +0.11% |
| frames-3-complex | -3.20% | -1.83% |
| frames-3-rapid | -2.99% | -1.39% |
| frames-3-resize | -0.14% | +0.95% |
| frames-3-steady | -0.93% | -0.40% |
| idle-2000 | -0.78% | -4.60% |
| masonry-noop | +3.19% | +7.08% |
| measured-queries | +0.63% | +1.05% |
| work-0-false | -0.11% | -2.04% |
| work-0-true | -2.36% | -1.02% |
| work-1-false | +0.85% | +1.90% |
| work-1-true | -1.73% | +0.03% |

此前中间版本的 append-100000 也出现过 +20.00%／+69.06%，另有 Grid 冻结 hidden +6.89% 等不利观测；这些数据仍完整保存在 `followups-final-*` CSV，不因最新样本不同而删除，也不把计数一致解释为 CPU 零回归。

### 6.2 冻结 Grid 最终守卫

[第一轮](virtual-list-series-followups-accepted-grid-2026-11-05.csv)／[复测](virtual-list-series-followups-accepted-grid-repeat-2026-11-05.csv) 保持全部 reads/builds 和 0 阅读误差。冻结四场景每 160 周期仍少 960 allocations、3328 bytes 增量 peak；未冻结的空间／工作量计数不变。

| 原始 case | 第一轮 CPU | 复测 CPU |
| --- | ---: | ---: |
| anchor-grid-0-hidden | +1.35% | -2.63% |
| anchor-grid-0-prefix | +1.55% | +0.44% |
| anchor-grid-0-scroll | +0.28% | +0.77% |
| anchor-grid-0-steady | +0.98% | +2.72% |
| anchor-grid-3-hidden | -1.23% | -1.82% |
| anchor-grid-3-prefix | +0.94% | -0.33% |
| anchor-grid-3-scroll | -2.99% | -3.72% |
| anchor-grid-3-steady | -3.54% | -1.43% |

### 6.3 Measured 动画最终成本

[第一轮](virtual-list-series-followups-accepted-measured-2026-11-05.csv)／[复测](virtual-list-series-followups-accepted-measured-repeat-2026-11-05.csv) 覆盖最后的逐采样边界修正。下列计数为单 sample 中位数，before 为旧即时行为。

| 场景 | 第一轮 CPU | 复测 CPU | builds before→after | allocations before→after | peak bytes before→after |
| --- | ---: | ---: | ---: | ---: | ---: |
| measured-idle | +1.36% | +5.11% | 1600→1600 | 20160→20160 | 15960→15960 |
| measured-reduced | -4.35% | -0.89% | 1610→1610 | 20226→20228 | 16440→16440 |
| measured-short | +15.10% | +12.57% | 1610→1938 | 20226→22408 | 16440→17488 |
| measured-tail | +10.39% | +17.59% | 1610→1941 | 21381→23538 | 19864→27400 |

idle/reduced 终点误差为 0；short 最大约 0.0049994px，tail 最大约 0.0062504px，均小于既有 0.01px 验收阈值，不能写成精确零误差。动画增加中间布局、source reads 和 View 构建，这是按明确功能需求采用的成本；没有把动画新增工作量当成等画面提速。

### 6.4 验证、产物与回退

- 最终 workspace（含 doctests）为 **892 passed / 0 failed / 12 ignored**，其中 virtual_list 为 **178 passed / 0 failed / 8 ignored**。忽略项未伪装成通过；另行运行的 Windows Release 原生用例为 **3/3**，底层 native/base 必需契约为 **16+2**。
- 严格 workspace/all-targets Clippy（headless-tests,virtual-profile，-D warnings）、cargo fmt check、git diff check 通过。生产 Cargo manifests/locks 与本轮起点一致；参考源码只读且指纹未变。
- 原生测试覆盖真实 Windows loop 的布局、中间动画和单 owner 坐标；没有新增人工截图、跨 GPU/DPI 或屏幕阅读器验收，headless CPU 也不表示 GPU FPS 或输入延迟。
- `followups-pre-final-default-*` 保存截止反馈修正前的样本，其临时目录仍叫 `five-final-default*`。曾复用该 label，旧的已跟踪历史 CSV 已恢复；本轮 CSV 改为上述唯一名称。比较器现拒绝已有 CSV 或 identity 的 label。
- 新防覆盖检查一度与 archive 解压复用路径变量，四份 `followups-final-{default,measured}*` CSV 因此写错临时位置；`1325ed5` 修正为独立 output_path。这四份 CSV 从四轮规范原始日志逐行恢复并校验，无采样数值修改；其后的 final-grid 与全部 accepted 运行验证了正常输出位置。
- 阶段性的 `after_head` 可早于未提交候选，实际测试版本以 vendor source hash 为准；被拒 trial 不冒充最终实现。accepted 六组与最终工作区逐字节匹配，并按 Git LF 归一化核对 `e7fa10d` 的完整 vendor 源码；保留 raw 和 LF 两种指纹，不修改采样后的源文件。
- 起点 `7fb2c29`，实现按独立提交保留；逆补丁在临时 Git index 中逐提交核对 parent tree，并验证整轮恢复为起点 tree，主 index 和工作区不参与回退。归档提交后再验证最终交付 HEAD；正式回退应考虑后续依赖并按逆序 git revert，不改写历史。

| 能力／修正 | 提交 |
| --- | --- |
| 四区流式遍历 | a6c6265 |
| 精确 cell 与焦点生命周期 | 3a5d031 |
| 显式 row/column span | 7237b55 |
| Measured 短路径及末段策略 | b5af010 |
| min/max/weight auto | 69a7008 |
| 截止反馈重求 | b449e82 |
| Grid RTL | 732fce3 |
| Showcase 与原生组合验证 | d2dfa56 |
| 内容测量拒绝实验与证据保护 | 895a88d |
| 公共导出格式／产物路径修复 | 28db5f3 / 1325ed5 |
| 动画中 resize／失效的逐采样边界 | e7fa10d |
