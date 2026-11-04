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

表中 CPU ms 指 `Instant::elapsed` 测得的主线程工作段墙钟耗时，不是进程 CPU 使用率。临时峰值为计量窗口内净新增堆 payload 的峰值，已有模型／冻结快照不纳入起始值，不能当作完整进程堆或 RSS。构造 case 从创建之前开始计量。完整的 baseline、采样时 HEAD、随后收入的 after 源码提交、workload SHA256 和运行顺序见[12 组来源记录](virtual-list-continuation-2026-11-04.json)。

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
| 1,000,000 | 19.75980→17.61605 | 19.39905→17.65340 | 16,494→16,484 |

冷构造改善约 8.49%～13.58%，峰值与常驻 payload 略降；append 分配／峰值、全部帧轨迹的 source／构建／分配计数不变。行为测试 107 passed / 8 ignored，含空输入、255／256／257 边界、300 条 lane、已冻结快照和后续追加。

但十万项上 10,000 批无变化反馈，首轮耗时 5.43635→5.86545 ms（+7.89%）；单独 ABBA 复测 5.21365→5.81470 ms（+11.53%），分配、source 和 revision 语义均不变。本试验未隔离出退化的具体原因，不能仅因被改代码位于构造阶段，就把两轮可重复热路径代价当作噪声。优先保留频繁滚动／反馈路径，本轮拒绝；上轮已公开的冷构造成本仍保留，未来若换实现需重跑相同保护矩阵。

保留原始数据：[构造](virtual-list-series-next-masonry-pack-2026-11-04.csv)、[构造复测](virtual-list-series-next-masonry-pack-repeat-2026-11-04.csv)、[帧／查询／追加](virtual-list-series-next-masonry-pack-guards-2026-11-04.csv)、[无变化反馈复测](virtual-list-series-next-masonry-pack-noop-repeat-2026-11-04.csv)。

## 3. 原生尺寸反馈：采用有界批次，撤销整批版本

before `f1e9061`（含倒序／重复输入 workload 扩展）；after 采用源码 `ed7fe17`。参考 virtua 的批量尺寸写入和 vlist 的计算／提交分离，保留 ListState 作为唯一尺寸 owner。先校验整个输入，顺序输入直接借用；只有无序输入才复制并稳定排序。保留每个重复索引的最后输入，每个非空调用只增加一次修订号，保持 hint／真实标记、焦点和冻结快照。

第一版用一个前向游标提交全部变化，试验 `b759fd2`、回退 `26fcdfd`。虽然连续／稀疏耗时均有收益，但 4096 项稀疏批次临时峰值从约 29.38 MB 增至 56.69 MB（+92.95%）。旧树和整批新稀疏路径同时保留，不能只看速度采用。保留[整批原始数据](virtual-list-series-next-native-batch-2026-11-04.csv)和[帧保护](virtual-list-series-next-native-batch-guards-2026-11-04.csv)。

最终每八个唯一索引作一次前向拼接，及时释放旧树路径；RefCell 写借用覆盖整个调用，校验在所有写入之前，外部只观察最终修订号。上限约束临时新旧路径数量，不是固定字节数；无序输入仍有 O(输入项数) 排序缓冲。单项不增加排序缓冲，不增加 public API 或并行尺寸缓存。

百万项模型，持有原始冻结几何，每样本连续提交 20 次。下表是相同尺寸反馈的十样本中位数，时间是整个 20 次调用总和，不是每帧时间：

| 批次／顺序 | CPU ms 前→后 | 分配次数 前→后 | 临时峰值 B 前→后 |
| --- | --- | --- | --- |
| 1 项 | 0.11725→0.11370 | 402→402 | 35,840→35,840 |
| 32 连续，升序 | 2.76075→0.48460 | 12,223→3,290 | 40,960→41,984 |
| 32 稀疏，升序 | 6.14280→4.60815 | 15,771→11,408 | 500,736→600,064 |
| 4096 连续，升序 | 388.22715→68.13810 | 1,650,874→433,220 | 728,064→731,136 |
| 4096 稀疏，升序 | 718.51655→352.30715 | 2,025,630→944,170 | 29,380,608→29,319,168 |
| 32 稀疏，倒序 | 6.21190→4.65565 | 15,992→11,428 | 504,832→600,576 |
| 4096 稀疏，倒序 | 866.92055→351.88315 | 2,187,212→944,210 | 29,773,824→29,384,704 |
| 4096 稀疏，含 2048 次重复覆盖 | 1125.47010→352.06805 | 3,102,421→944,210 | 29,574,144→29,444,096 |

采用代价：32 项稀疏升序峰值仍多 99,328 B（约 97 KiB，+19.84%），倒序多 95,744 B（+18.97%）。4096 连续峰值 +0.42%，单项不变；大稀疏批次回到基线附近。相对整批版本，主动放弃部分连续写入速度以限制临时路径保留。收益主要是尺寸写回，不能宣称稳定帧有同等比例改善：17 类轨迹的 source／行构建／分配数全部不变；原生 Measured 稳定／快速／复杂／resize 轨迹 CPU 分别 +4.03%／−1.17%／+1.86%／+3.15%。

另加[真实尺寸变动 workload](virtual-list-batch-churn-2026-11-04.rs)，每次调用把所有反馈高度在 30／31px（重复覆盖 40／41px）之间交替，输入准备开销计在前后两边，避免只测重复提交相同值。4096 连续 391.91540→68.59995 ms（−82.50%）；4096 稀疏升序 723.23820→351.05685 ms（−51.46%）；倒序 −59.82%、含重复覆盖 −68.64%。各 case 分配／峰值与上表一致。单项 0.11360→0.11475 ms（+1.01%），分配／峰值不变，不宣称单项 CPU 收益。

回归包括无序／重复最后获胜、true/false 标记、空批次不改 revision、非空批次一次 revision、冻结几何及 focus 最终释放。5000 行六轮混合批次逐项对照独立数组模型的精确 prefix、尺寸和 measured 标记。新增非法输入整体拒绝：越界、零、NaN、无穷高均不留下部分尺寸或未增加 revision 的写入。最终原生集成 15 passed，完整 virtual_list 为 107 passed / 8 ignored。

原始数据：[有界批次](virtual-list-series-next-native-bounded-2026-11-04.csv)、[持续尺寸变化](virtual-list-series-next-native-bounded-churn-2026-11-04.csv)、[帧／查询／追加保护](virtual-list-series-next-native-bounded-guards-2026-11-04.csv)。

## 回退索引与验证

| 内容 | 决策 | 提交 |
| --- | --- | --- |
| Workload 起点／扩展 | 保留 | `ec104b6`／`f1e9061` |
| 兼容性行为测试 | 保留 | `6bde597` |
| Masonry 测量代次失效 | 采用 | `28da536` |
| source-slot 冷构造 | 拒绝；试验后回退 | `f51794b` → `5fe38be` |
| 原生整批前向反馈 | 拒绝峰值；试验后回退 | `b759fd2` → `26fcdfd` |
| 原生每八项有界反馈 | 采用 | `ed7fe17` |

分支 `perf/virtualization-continuation-20261104`，不推送、不改参考项目或依赖清单。撤销采用项可按依赖逆序执行 `git revert ed7fe17`、`git revert 28da536`；证据和独立兼容性测试可保留。不要使用 hard reset 覆盖起点之前的工作。

以下检查均针对最终采用源码通过：

- `cargo test --workspace --features windows-showcase/headless-tests --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1`：全部 workspace 测试及 doctests 通过，含 Showcase 和其他组件；virtual_list 最终 **107 passed / 8 ignored**。ignored 为既有手工探针，以上 ABBA workload 另行运行。
- 原生 fixture `cargo test --manifest-path target/gpui-virtual-upgrades-contract/Cargo.toml --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --test virtualization -- --test-threads=1`：**15 passed**。
- Base fixture `cargo test --manifest-path target/gpui-base-contract/Cargo.toml --features test-support --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --lib virtualization -- --test-threads=1`：**2 passed**。
- `cargo clippy --workspace --all-targets --features windows-showcase/headless-tests --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- -D warnings`：通过；原生 fixture 的 virtualization test target Clippy `-D warnings` 也通过。
- `cargo fmt --all -- --check`、修改 vendor／workload 的 `rustfmt --edition 2024 --check`、`git diff --check`：通过。
- `cargo build --release -p windows-showcase --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104`：通过。
- 12 组 CSV 行数、before/after、workload 快照 SHA256、after 源码提交和报告链接均已核对；`ed7fe17`、`28da536` 的 `git apply --reverse --check` 均通过，只检查可反向应用，未改工作区。

时间证据来自隔离 Release ABBA，采样前等待编译退出；机器调度和缓存仍会影响耗时，计数和行为回归提供独立证据。本轮没有执行实窗 GPU／物理帧率或图片网络时序验收。
