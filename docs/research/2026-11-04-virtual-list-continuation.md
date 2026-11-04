# 虚拟滚动继续对标：失效、冷构造与批量反馈

日期：2026-11-04。起点 `ea44da3`，分支 `perf/virtualization-continuation-20261104`。延续上一轮授权，以隔离 Release ABBA、行为回归和 Git 独立提交判断是否吸收；不改三个参考项目，不推送。

## 本轮对照与边界

- TanStack Virtual 的 `packages/virtual-core/src/index.ts`：`measure()` 通过 cache version 使后续计算失效，resize batch 保持一批变化的锚点和计算一致性。参考的是有效身份与批次边界，不照搬 DOM observer 或扁平数组。
- virtua 的 `src/core/layouts/list.ts` 和 `masonry.ts`：以 dirty index 延迟 offset 重算，真实尺寸与估算、lane 重排和重新估算分开管理。它不提供本项目相同的持久冻结几何合同，不能直接用可变数组替代。
- vlist 的 `src/core/pipeline.ts`：计算、提交分开，预分配 TypedArray 与池负责热路径复用；在 GPUI 中只借鉴初始化／元数据成本，不跨帧缓存 AnyElement。

三个候选分别实验，不合并收益：

1. Masonry 宽度／布局失效：当前 KnownGeometry 每次改宽复制全部尺寸块。试验几何代次与块代次，保持旧尺寸为 hint、旧快照可读，首次写入一块才处理其过期标记。检查多次失效、跨块／尾块追加、重新测量与 source 替换，比较稀疏／稠密以及 Known 静态查询和内存成本。
2. Masonry 分块索引冷构造：上轮持快照追加收益已采用，但百万项冷构造约慢 10%。试验初始化阶段批量填 source→lane slots 块，保留 lane indexes 及后续追加的持久 COW push。避免给每条 lane 增加整块初始化缓冲，尤其是大量空 lane；比较构造、常驻／峰值、追加、稳定帧和无变化反馈。
3. 原生 ListState 批量尺寸反馈：目前每项独立切片并重建路径。试验排序／合并重复 index 后一次串行树构造，保留单项无分配路径、最后一次重复反馈获胜、focus、真实／hint 标记与冻结快照。比较 1／32／4096 项连续和稀疏批次，并检查原生调用者回归。

共用 [adoption workload](virtual-list-adoption-2026-11-04.rs) 和 [比较工具](../../tools/compare-virtualization.py)。`next_masonry_width` 每样本 40 次宽度变化，`next_native_batch` 每样本 20 次批量反馈；其他成本沿用上一轮 workload。每个实验单独选择 before Git 源码归档和当前 after 源码，Release 编译完成、等待同机 cargo/rustc/clippy-driver 退出后，以 before/after/after/before（ABBA）运行。改宽／帧轨迹每版本六样本，构造／批次及其他查询每版本十样本，以下为样本中位数。计量仅为主线程 headless CPU 和堆 payload，不宣称物理帧率、RSS 或 GPU 收益。

workload 提交 `ec104b6`，兼容性测试提交 `6bde597`。测试先在现有算法上验证无序／重复反馈、hint、修订号、焦点释放和冷 pack 边界，避免只测试新实现。`builds` 对普通帧代表行构建次数；`masonry-index-memory-*` 的 `builds` 字段特指计量结束时的常驻堆 payload 字节，不能解释成构建次数。

## 1. Masonry 测量代次：采用

before `ec104b6`；after 生产源码提交 `28da536`。采样时 HEAD 为测试提交 `6bde597`，after 实现当时尚未提交，随后原样收入 `28da536`。参考 TanStack 的 cache version 和 virtua 的延迟失效边界，给 KnownGeometry 与每个 128 项共享尺寸块增加代次。改宽／布局失效只提升几何代次；读时过期的真实尺寸呈现为 hint，首次写回／尾块追加时才归一化该块的标记。旧冻结快照保留自己的代次，不增加第二份尺寸缓存。

每样本 40 次改宽，持有旧冻结快照；表中“预测量项”指计量前已写入的真实尺寸数。

| 总项／预测量项 | CPU ms 前→后 | 分配次数 前→后 | 临时峰值 B 前→后 |
| --- | --- | --- | --- |
| 1,000／1 | 3.71650→3.83230 | 12,522→11,922 | 46,704→38,136 |
| 100,000／64 | 7.07180→3.78295 | 47,442→12,402 | 1,665,736→48,256 |
| 1,000,000／64 | 51.08185→4.09745 | 355,602→12,762 | 16,591,184→56,176 |
| 100,000／100,000 | 6.87560→3.72980 | 47,495→12,452 | 1,665,736→48,256 |

百万项耗时 −91.98%、分配 −96.41%；三种稀疏规模的 source 访问均为 6,724、行构建均为 1,200，优化不靠少画行。稠密 case 的计数同样前后一致。千项耗时 +3.12%，不宣称小列表 CPU 收益。

代价也保留：每块多一个 u64，百万项常驻从 36,655,320 B 增至 36,724,464 B（+0.19%）；一万项 +0.31%。追加临时峰值增加约 2.47%～6.10%，分配次数不变。冷构造耗时在一万／十万／百万项分别为 +2.99%／+6.22%／−1.31%，不是所有路径更快。17 类帧轨迹的 source／构建次数全部不变；非改宽 Masonry 稳定／快速／复杂轨迹耗时 −3.20%／+0.93%／+0.47%，普通 Known 查询无新增分配。Masonry resize 轨迹分配 −25.06%、size 轨迹 −4.52%。

行为回归覆盖三次宽度切换、128 项尺寸块边界、部分尾块追加、再测量、source 替换及 reflow，验证旧尺寸不会通过写回重新变成有效测量，且旧快照仍为真实尺寸。

原始数据：[改宽](virtual-list-series-next-masonry-width-2026-11-04.csv)、[17 类帧和查询／追加保护](virtual-list-series-next-masonry-width-guards-2026-11-04.csv)、[冷构造／内存](virtual-list-series-next-masonry-width-memory-2026-11-04.csv)。

## 2. 批量构造 source-slot 索引：拒绝并回退

before `28da536`；试验 `f51794b`，回退 `5fe38be`。只把 pack 的 source→lane slots 改成私有 `IndexVector::from_iter`，在栈上填满一个 256 项数组后一次共享，并按迭代器下界预留目录。lane 索引和后续追加仍走原有 COW push；不为每条 lane 预留整块缓冲，不增加常驻缓存。

| 冷构造规模 | 首轮 CPU ms 前→后 | 独立复测 ms 前→后 | 分配次数 前→后 |
| --- | --- | --- | --- |
| 10,000 | 0.14790→0.12925 | 0.15865→0.13710 | 243→239 |
| 100,000 | 1.76720→1.61725 | 1.64260→1.49375 | 1,746→1,739 |
| 1,000,000 | 19.75980→17.61600 | 19.39900→17.65340 | 16,494→16,484 |

冷构造改善约 8.49%～13.58%，峰值与常驻 payload 略降；append 分配／峰值、全部帧轨迹的 source／构建／分配计数不变。行为测试 107 passed / 8 ignored，含空输入、255／256／257 边界、300 条 lane、已冻结快照和后续追加。

但十万项上 10,000 批无变化反馈，首轮耗时 5.43635→5.86545 ms（+7.89%）；单独 ABBA 复测 5.21365→5.81470 ms（+11.53%），分配、source 和 revision 语义均不变。本试验未隔离出退化的具体原因，不能仅因被改代码位于构造阶段，就把两轮可重复热路径代价当作噪声。优先保留频繁滚动／反馈路径，本轮拒绝；上轮已公开的冷构造成本仍保留，未来若换实现需重跑相同保护矩阵。

保留原始数据：[构造](virtual-list-series-next-masonry-pack-2026-11-04.csv)、[构造复测](virtual-list-series-next-masonry-pack-repeat-2026-11-04.csv)、[帧／查询／追加](virtual-list-series-next-masonry-pack-guards-2026-11-04.csv)、[无变化反馈复测](virtual-list-series-next-masonry-pack-noop-repeat-2026-11-04.csv)。
