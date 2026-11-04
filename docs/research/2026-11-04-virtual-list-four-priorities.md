# 虚拟滚动四项逐项验证（2026-11-04）

本轮接续细微尺寸反馈与单项分配优化。起点 `3795d40` 已先提交并推送到 `quality/virtualization-next-review-20261104`，远端 SHA 已核对；新工作隔离在 `quality/virtualization-four-priorities-20261104`。参考目录仍为 `refer/virtual/{virtua-main,virtual-main,vlist-main}`，没有修改参考项目。

## 方法与约束

- 一个滚动位置 owner、一份测量几何。Base 维护行为，Yue 继续只呈现；不引入持久 scratch 或另一份高度/位置表。
- Release 对比隔离 Git 归档与当前 vendor，使用同一 workload、ABBA 顺序（before/after/after/before）。每轮等待两次无编译观察；轮询检测到同机编译时保留日志、丢弃该轮并重试。不终止其他项目。
- 比较运行时冻结生产源码和 workload。分配/峰值仅为测量线程、测量窗口内的 allocator payload，不是 RSS。`ns` 是工作段墙钟，不是 CPU 使用率。
- 新接口按行为合同验收，不把功能增加包装成性能收益。拒绝的试验保留提交和数据，再用 `git revert` 撤销。

## 1. 实窗证据：补齐实际 draw 分段采样

`99abe6d` 增加可选 `windows-showcase/virtual-profile` 和忽略的实窗探针，运行真正的 Application、Windows 平台、字体和 renderer，驱动生产 `VirtualLayoutsPage`。四条轨迹分别为 Grid 跳转、Masonry 跳转、Masonry 重排加跳转、横向 Measured 定位；各测 rem 16/20，每 case 108 次，去掉 12 次预热，至少保留 96 帧。

首轮 smoke 基线共 768 条，环境为 150% 缩放、refresh interval 6.25ms（160Hz）。其 draw p50/p95/p99（ms）如下。该轮未经过编译干扰保护，只是功能与环境基线，不作为采用性能优化的证据。

| case | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Grid rem16 | 3.983 | 6.221 | 7.479 |
| Masonry rem16 | 1.854 | 3.332 | 4.361 |
| Masonry reflow rem16 | 2.289 | 4.074 | 5.259 |
| Horizontal rem16 | 1.327 | 2.353 | 2.894 |
| Grid rem20 | 4.836 | 6.839 | 9.080 |
| Masonry rem20 | 2.226 | 3.681 | 4.267 |
| Masonry reflow rem20 | 2.058 | 3.363 | 4.327 |
| Horizontal rem20 | 1.082 | 1.997 | 2.552 |

数据：[smoke 原始帧](virtual-native-frame-baseline-2026-11-04.csv)。profiling 特性额外启用上游 profiler 已有依赖 `hdrhistogram`，正常构建不启用；没有改变正常发布特性。

探针通过原生 owner/controller 发出定位；不是物理滚轮/触控板录制。Draw、layout、prepaint、paint、Present 只覆盖对应应用工作/提交阶段，发起 reflow 等命令时发生在 Draw 外的逻辑没有并入 draw_ns。这不是输入到显示延迟、GPU 完成时间、扫描输出 FPS 或硬件级丢帧数。大字号 Grid 的长尾值得持续观察，但当前证据没有把 renderer、字形、内容复杂度和组件几何成本分别隔离，不能据此重写 Grid。

最终受保护 ABBA 的 3072 帧（两轮）仍显示混合方向：相对 `3795d40` 的 draw 中位变化按场景为约 -12.6%～+21.8%，另一轮约 -15.8%～+32.5%。更关键的是同源码 `HEAD` 对 `HEAD` 的控制对比本身也产生约 -0.2%～+14.5% 的变化，`fc3929c`（同 vendor、仅 fixture 特性对齐）对当前源码的对比也有约 -21.8%～+19.5% 波动。因此不把实窗耗时差异归因给本轮组件改动；它保留了环境基线和后续真实设备验收入口。

## 2. 采用：多项尺寸反馈复用索引缓冲完成去重

优化前合同/工作量提交 `03aca2e`，候选 `b6f8f5c`。原实现同时建立 HashSet 和 indexed Vec；现在先验证所有 key/正尺寸、写入同一个 indexed Vec，按索引排序后检查重复，再过滤真正的 no-op，最后复用已排序缓冲提交原 backend。单项专用路径不变。

无变化项也要保留到重复检查后，不能因 no-op 先过滤而接受重复 key。整批验证完成之前不修改测量几何、命令、锚点或快照版本。既有 stored f32 slot/exact/cross 身份判定继续复用，没有为减少分配跳过验证。多处不同故障同时出现时，优先返回哪一个错误可能随验证顺序改变；未承诺多故障的错误优先级，单一故障类型保持不变。

新增回归在优化前后均通过：双轴、Known/实际 Measured/hints、2/31/32/33/256/4096 项、乱序、全部 no-op 重复、有效更新后的末尾 NaN、冻结快照和失败原子性。候选完整 virtual_list 为 118 passed / 8 ignored。

两轮独立 ABBA，各含 72 case：三种后端/反馈、2/32/256/4096 项、正序/倒序/确定性乱序、no-op/40↔41px 真更新。每样本 2000 次调用；4096 项为 64 次；每版本每 case 十样本。

| case | 首轮 ms 前→后 | 复测 ms 前→后 | 分配次数前→后 | 临时峰值 B 前→后 |
| --- | --- | --- | --- | --- |
| Known，32，正序，无变化，2000 次 | 3.00035→1.39390 | 3.03675→1.38480 | 12000→2000 | 1408→512 |
| Measured，256，乱序，无变化，2000 次 | 162.60935→152.41295 | 165.98715→153.93630 | 18000→2000 | 11040→4096 |
| hint，4096，乱序，真更新，64 次 | 324.47725→323.06665 | 329.49375→322.91695 | 1347583→1346689 | 10020880→9881600 |

两轮全部 case 的分配和临时峰值均下降，source 查询次数的逐样本计数集合一致。无变化 case 中位耗时分别下降 4.69%～53.64%、4.31%～54.40%；真更新分别下降 0.43%～34.22%、1.01%～40.22%。最小的 CPU 差别不作为显著加速依据；采用理由首先是确定的分配下降、验证和几何合同完整。批量仍有一个 Vec，不宣称零分配，也不把 API 微基准扩大成实窗帧率改善。

数据：[首轮](virtual-list-series-priorities-batch-2026-11-04.csv)、[独立复测](virtual-list-series-priorities-batch-repeat-2026-11-04.csv)。首轮曾丢弃受其他项目编译影响的 `before-0-attempt-1.txt`，有效 CSV 不含该轮。

## 3. 分层 Masonry 索引目录试验

候选 `d1ae715` 只改变私有 `IndexVector`：256 项叶块不变，每 32 叶块增加共享目录组。读取仍为常数层级，持有冻结快照时的根目录复制从 O(N/256) 降到 O(N/8192)，另复制最多 32 指针的尾目录及尾叶。它仍有与 N 成比例的根目录，不宣称变成对数树。

完整 118 项组件回归通过，包括原实现先通过的 8192 source-slot、24576 lane-index 边界和多代冻结快照。测试 workload 分别测 1 万/10 万/100 万项的冷构造，以及 20 次每次追加 10 项、保留全部 20 代快照。原模型的构造不计入追加窗口；冷构造窗口含新模型构造与释放。source 读取均为零，因为输入为固定尺寸算术源，不能把这个结果推广到任意业务 source。

首轮数据如下，全部数值为每 case 十样本的中位数。

| N / 路径 | ms 前→后 | 耗时变化 | 分配次数前→后 | 临时峰值 B 前→后 |
| --- | --- | ---: | --- | --- |
| 1 万 / 冷构造 | 0.15930→0.17015 | +6.81% | 243→253 | 496128→496264 |
| 1 万 / 20 次追加 | 0.06950→0.07105 | +2.23% | 883→1043 | 425712→424592 |
| 10 万 / 冷构造 | 1.63350→1.81760 | +11.27% | 1746→1855 | 5219208→5216712 |
| 10 万 / 20 次追加 | 0.13870→0.10975 | -20.87% | 942→1102 | 571752→453480 |
| 100 万 / 冷构造 | 19.44050→21.00660 | +8.06% | 16494→17701 | 48960424→48953656 |
| 100 万 / 20 次追加 | 0.86200→0.21965 | -74.52% | 1122→1282 | 1777720→550008 |

数据：[首轮](virtual-list-series-priorities-directory-2026-11-04.csv)。

结论：**不吸收，`e273df9` 已显式 revert `d1ae715`**，当前索引源码与试验前一致。百万项持续保留快照时的追加收益确实存在，但当前 Showcase 万项卡片场景没有获得收益，三档冷构造和所有 case 的分配数反而增加。没有明确的大规模持续追加消费者时，不把这项代价加到默认实现。拒绝后未继续为候选运行整帧保护矩阵；最终保护矩阵使用恢复后的实现。

独立复测在最后一个 before 轮次之前一直等待同机其他项目编译，主动停止的只是本任务采样器；没有完成 ABBA，所以[不完整数据](virtual-list-series-priorities-directory-incomplete-2026-11-04.csv)明确排除，不拼接成“第二轮通过”。拒绝判断来自已经完成的首轮对比和确定的分配/结构成本。

## 4. 采用：Masonry instant 绝对偏移定位

`84195c0` 新增 `VirtualMasonryState::scroll_to_offset`，复用原先的一个 pending 命令和原生 `ScrollHandle`，仅把私有目标扩展为 Key/Offset。key 的旧 align 入口保持兼容；options 校验、ID、事件、输入中断、卸载和 `CommandProgress` 都是原路径，不另建滚动状态机。

契约如下：

- offset 必须是有限非负 f64。key/offset/动效失败保留旧 pending、ID 和事件，不写原生位置。
- 可在挂载前排队，按当前 viewport/总高 clamp；源收缩或同帧测量改变几何时重新解析。空源 clamp 到零，offset 没有 Missing-key 语义。
- 正常布局/校正完成且原生读回位置与目标 clamp 相符才报告一次 Settled；32 帧/2 秒活动采样、8 次无位置进展继续有界收尾为 NoProgress。
- offset 的 align 和 scroll padding 不参与计算；离屏高度可仍是 estimated。定位完成后不持续追随尾部，不承诺以后追加仍贴底。
- 边界滚轮、原生拖动/外部位置变化、显式取消和卸载继续终止旧命令。失败的 smooth 不隐式取消；合法 reduced-motion 请求仍为 instant。

六项新增生产接缝测试通过：非法/非有限输入与旧命令原子性；挂载前、空源、最大有限值、`16777217.125` 精确位置；Known/Measured 校正与 source shrink；用户接管及卸载/重挂；边界无位移 wheel 恰好一次取消；有限预算下的 NoProgress 恰好一次收尾。实体事件通过订阅验证，纯 owner 事件通过 `take_events` 验证；不为测试保留第二份生产事件队列。最终完整组件目标为 124 passed / 8 ignored。

参考项目的启发是向唯一平台 owner 提交 offset 意图：virtua `src/core/driver.ts` 的归一化 scrollTo、TanStack `packages/virtual-core/src/index.ts` 的 offset/target 解析、vlist 的公共位置与滚动模块。这里没有复制 DOM 驱动、scroll classifier 或浏览器坐标归一化。本轮结束时 Masonry 的 Attached/follow-tail/smooth、RTL/反向布局和未知全局 Grid 行高仍未实现，capabilities 只把 offset 改为 true。后续 follow-tail 的采用与验证见[尾部跟随对标](2026-11-04-virtual-list-follow-tail-review.md)，本报告的数据仍对应本轮提交。

## 最终验证与回退

验证针对 `84195c0` 的采用源码；`fc3929c` 只使 native 性能 fixture 不启用 headless test-support，匹配生产 Showcase 的依赖特性。正常窗口验收和 profiling probe 分进程运行。

- 完整 workspace + doctest：838 passed / 10 ignored / 0 failed，其中 virtual_list 为 124 passed / 8 ignored。
- 原生层 15 + Base 层 2 项合同通过；合同工具核对生产 manifests/locks 没有被它修改。
- workspace all-targets Clippy 同时启用 headless-tests 与 virtual-profile，`-D warnings` 通过；workspace fmt、改动 vendor 文件/workload rustfmt 通过。
- Windows Showcase Release 构建通过。实际 Windows/GPU 窗口验收 1 passed，验证 Grid 尾行/尾列、Masonry 重排和横向 Measured 定位/收尾，renderer 调用 264/49/28；调用数不是 FPS。
- 六项新 offset 测试是生产 GPUI 接缝的 headless 布局/输入测试。实窗探针不包含该新增方法的硬件滚轮重放，不将两者混称。

| 内容 | 提交/回退 |
| --- | --- |
| 本轮前已推送的起点 | `3795d40` |
| 实窗分段采样与 opt-in 特性 | `99abe6d` |
| 优化前通过的批次/冻结快照合同 | `03aca2e` |
| 采用的多项缓冲优化 | `b6f8f5c`；可 `git revert b6f8f5c` |
| 分层目录试验及明确撤销 | `d1ae715` → `e273df9` |
| 采用的 offset 能力、测试与能力文档 | `84195c0`；可 `git revert 84195c0` |
| native fixture 特性对齐 | `fc3929c` |

`git apply --reverse --check` 已分别验证 `b6f8f5c`、`84195c0` 可在当前工作树逆向应用，仅检查没有实际回退。索引文件与 `b6f8f5c` 的 LF 规范化 SHA-256 相同，确认被拒绝的目录没有残留。需要回到整个旧版本时，从干净工作区新建 review 分支指向上述提交；不使用 hard reset 覆盖现有工作。退回 offset 提交会同时恢复其 capability、测试和接口文档；退回缓冲提交保留以前的精度修正和本轮原子性测试。
