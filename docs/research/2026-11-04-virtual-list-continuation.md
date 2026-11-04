# 虚拟滚动继续对标：失效、冷构造与批量反馈

日期：2026-11-04。起点 `ea44da3`，分支 `perf/virtualization-continuation-20261104`。延续上一轮授权，以隔离 Release ABBA、行为回归和 Git 独立提交判断是否吸收；不改三个参考项目，不推送。

## 本轮对照与边界

- TanStack Virtual 的 `packages/virtual-core/src/index.ts`：`measure()` 通过 cache version 使后续计算失效，resize batch 保持一批变化的锚点和计算一致性。参考的是有效身份与批次边界，不照搬 DOM observer 或扁平数组。
- virtua 的 `src/core/layouts/list.ts` 和 `masonry.ts`：以 dirty index 延迟 offset 重算，真实尺寸与估算、lane 重排和重新估算分开管理。它不提供本项目相同的持久冻结几何合同，不能直接用可变数组替代。
- vlist 的 `src/core/pipeline.ts`：计算、提交分开，预分配 TypedArray 与池负责热路径复用；在 GPUI 中只借鉴初始化／元数据成本，不跨帧缓存 AnyElement。

三个候选分别实验，不合并收益：

1. Masonry 宽度／布局失效：当前 KnownGeometry 每次改宽复制全部尺寸块。试验几何代次与块代次，保持旧尺寸为 hint、旧快照可读，首次写入一块才处理其过期标记。检查多次失效、跨块／尾块追加、重新测量与 source 替换，比较稀疏／稠密以及 Known 静态查询和内存成本。
2. Masonry 分块索引冷构造：上轮持快照追加收益已采用，但百万项冷构造约慢 10%。试验初始化阶段批量填块，保留后续持久 COW push；比较构造、常驻／峰值、追加、稳定帧和无变化反馈。
3. 原生 ListState 批量尺寸反馈：目前每项独立切片并重建路径。试验排序／合并重复 index 后一次串行树构造，保留单项无分配路径、最后一次重复反馈获胜、focus、真实／hint 标记与冻结快照。比较 1／32／4096 项连续和稀疏批次，并检查原生调用者回归。

共用 [adoption workload](virtual-list-adoption-2026-11-04.rs) 和 [比较工具](../../tools/compare-virtualization.py)。`next_masonry_width` 每样本 40 次宽度变化，`next_native_batch` 每样本 20 次批量反馈；其他成本沿用上一轮 workload。计量仅为主线程 headless CPU 和堆 payload，不宣称物理帧率、RSS 或 GPU 收益。最终采用／拒绝、代价、原始 CSV 与回退提交在实验后补全。
