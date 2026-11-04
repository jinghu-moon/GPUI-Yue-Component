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

完整不利样本见 [第一轮](virtual-list-series-frozen-stream-2026-11-05.csv) 和 [复测](virtual-list-series-frozen-stream-repeat-2026-11-05.csv)。重现：`python tools/compare-virtualization.py --before 7fb2c29 --label frozen-stream --date 2026-11-05 --workload docs/research/virtual-list-frozen-anchor-2026-11-05.rs --filter frozen_anchor_workload`；复测替换 label。隔离构建 identity 和编译静默检查由比较器记录在 `C:/Temp/gpui-yue-series-20261104/` 对应目录。

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

试验先后暴露“拒绝宿主仍重设起点返回原地”和“等尺寸新增测量重启段落”问题；拒绝／高刷新率用例先失败后修正。[trial 原始采样](virtual-list-series-measured-motion-trial-2026-11-05.csv) 保留，仅是被拒候选。最终源码重跑两轮 ABBA，baseline `7237b55`，最终误差均为 0；[固定 workload](virtual-list-measured-motion-2026-11-05.rs)、[最终第一轮](virtual-list-series-measured-motion-final-2026-11-05.csv)、[复测](virtual-list-series-measured-motion-final-repeat-2026-11-05.csv) 完整保存。

第一轮 CPU：idle +3.53%，reduced +1.30%，short +13.64%，tail +12.78%。short builds 1610→1938、分配约 20230→22414、peak 16440→17488 bytes；tail builds 1610→1941、分配 21381→23538、peak 19864→27400。idle 各空间／工作量计数相同，reduced 两个命令多两个 motion 分配。动画的中间位置不同、工作量更多，这是功能成本，不是等画面提速。按用户要求采用并公开代价。阶段专项全量为 170 passed / 0 failed / 8 ignored。

后续组合审查再发现一个截止边界：动画完成的同 now 最后测量反馈不能继续复用旧的“最终对齐位置”。直接 `prepare→update_sizes→prepare` 回归先复现 40px（错误）而不是 50px，再修正为只冻结中间动画采样、完成后仍重求实际目标；另补 Owned 最后一帧真实布局反馈用例。最终整轮验证／重新采样覆盖此修正。

## 5a. 约束驱动 auto tracks

采用显式 `VirtualGridTrack(minimum,maximum,weight)` 及纯函数 `resolve`。调用方传入实际可用 extent、gap、两端 padding；按权重分配剩余空间，达到 max 后再分配给其余轨道，不足时保留 min 并允许滚动。零权重固定在 min，达到全部 max 后留白，非法／非有限输入整体拒绝。解析采用 breakpoint water filling，O(T log T)，不查询 cell。结果交给 Known sizing 或现有 `update_sizes` 合批；消费者在约束变化时重算，不能每帧重建百万行 Known 数组。

测试覆盖权重、max 饱和、min 溢出滚动、留白、零轨道、非法输入，以及解析后 Known resize 保持原阅读 key、同值更新不使旧 snapshot 过期。默认 Grid 不调用解析器。这是约束驱动布局，不声称能推断未见内容的 intrinsic 最大宽高。
