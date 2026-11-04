# 平滑定位与布局边界：对标、测试与取舍（2026-11-05）

本轮采用 Masonry Fixed／Known 的短距离程序定位动画，补齐 Grid 的动效偏好与失活窗口策略，同时明确二维布局的现有边界。基线为 `4230ee50be48cf94c4ad6cceb83dac4e99a41dec`；最终源码为 `b83013b8c0f7293d98df99f0636cebad5a5c0b4f`，分支为 `quality/virtualization-four-priorities-20261104`。以下实验于 2026-11-05（Asia/Shanghai）完成。没有修改参考目录。

## 参考机制与采用范围

| 参考项目 | 本地源码 | 采用／保留的边界 |
| --- | --- | --- |
| TanStack Virtual | [virtual-core](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) | 目标变化后重新解析定位；区分动画过程与最终几何收敛；不照搬浏览器 scroll classifier |
| virtua | [driver](../../refer/virtual/virtua-main/src/core/driver.ts) | 动态测量与 smooth 有冲突，不能为动画挂载全路径；双轴／动态尺寸限制须明示 |
| vlist | [motion](../../refer/virtual/vlist-main/src/synthetic/motion.ts)、[apply-scroll-behavior](../../refer/virtual/vlist-main/src/utils/apply-scroll-behavior.ts) | 帧时间、动态目标、cubic-out 和 reduced motion；继续沿用本项目唯一 native ScrollHandle |

这里只对标本地源码机制，没有运行三个 JavaScript 项目的 benchmark，也没有比较跨框架绝对 FPS。参考项目存在某项能力，不代表本项目的不同几何／所有权模型可以直接支持它。

## 当前布局支持

| 布局 | 当前支持 | 明确限制 |
| --- | --- | --- |
| 纵向／横向列表 | VirtualList，Fixed／Known／Measured | 沿一个主轴排列 |
| 二维表格／矩阵 | VirtualGrid，同时虚拟化行列 | 两根 Attached Fixed／Known 轴共享 binding、宿主与一个 ScrollHandle；行高／列宽分别显式提供，可不等长 |
| 多列瀑布流 | VirtualMasonry，支持真实测量与显式 reflow 改列数 | 一个纵向滚动轴；稳定分栏，非横纵独立滚动的矩阵 |
| 自动轨道、跨格、冻结行列 | 尚无现成布局合同 | 需另行定义尺寸依赖、命中、裁剪与语义 |
| 任意二维坐标画布、节点图、三维空间 | 尚不支持 | 需要独立空间索引与裁剪策略 |

Grid 构建的是保留行列工作集的交点 `W_rows × W_columns`，不创建全量 `N × M`。不依据当前可见列自动推断所有行的真实高度；动态内容若要影响行高，调用方须先提供一致的 Known 行尺寸。双轴基础不自动带来自动轨道、跨格或冻结区支持。

## Smooth 的合同

这里的 smooth 是显式 key／offset 程序定位的短距离缓动。滚轮／触控板继续走原生输入路径，不额外叠加惯性。该能力默认不启用，由 `VirtualizerMotion::Smooth { duration, max_distance }` 请求。

- Masonry Fixed／Known 支持 key、offset 动画。距离同时受调用方上限和一倍视口限制，复用一维 ease-out cubic 采样器。
- 首次有效布局开始计时；同帧几何校正复用一个 elapsed。key 每帧按当前几何重新解析；Nearest 使用命令起点的参考视口，避免目标随动画自身漂移。
- Measured 的 `supports_smooth()` 仍为 false。合法 Smooth 选项接受后安全即时定位；reduced motion、超距、失活／隐藏／无可见区域也使用即时策略。动态测量仍需正常收敛，估算位置不冒充最终实测对齐。
- 新命令从唯一 native precise offset 续接。wheel（包括边界无位移输入）、native offset 变化、显式取消与卸载结束旧意图，终态通知恰好一次。
- 显式命令暂停已有 follow-tail 会话，不创建或擅自恢复会话；尾部跟随和阅读锚点补偿不播放位移动画。
- Grid 两轴共用同一帧时间，一次提交 x/y，再分别确认结果；补齐 reduced motion、裁剪后不可见和窗口失活时的安全收尾。wheel 取消两轴主动命令。

实现提交分别为：

1. `173c80bd47a08a969ac96347c0ee620128fe1643`：Masonry 有界 smooth。
2. `aa5b7f8bf5af5951b0ea5678d1ce9ea1d130c43a`：Grid 动效偏好、微距确认及实窗验证。
3. `b83013b8c0f7293d98df99f0636cebad5a5c0b4f`：可选 motion 状态隔离与饱和有界计数。

没有增加滚动 owner、动画 Task 或逐项几何镜像。

## 改动前后行为测试

将当前新增测试放入隔离的基线 fixture：Masonry 7 项全部失败，Grid 1 项失败。Masonry 基线不支持所请求的动画；Grid 基线在失活后没有按预期结束动画。最终源码新增 8 项全部通过。生产 manifests／locks 没有为 fixture 改写。

| 测试入口 | 验证内容 |
| --- | --- |
| [Masonry smooth](../../crates/yue-component/tests/virtual_list/masonry_smooth.rs) | Fixed／Known 的 key 和 offset 中间帧、终点、恰好一次事件；新命令续接与 follow 暂停；几何变化、Nearest、即时降级、卸载、高刷新率、微距、亚像素与边界 wheel |
| [Grid layouts](../../crates/yue-component/tests/virtual_list/layouts.rs) | 双轴同帧 smooth、输入取消、动效偏好和失活收尾；保留既有宿主拒绝移动得到 NoProgress 的断言 |
| [Windows native tests](../../examples/windows-showcase/src/pages/virtual_layouts/native_tests.rs) | 真实活动窗口里的布局、Grid x/y 与 Masonry 中间位置和最终定位 |

实窗验证曾发现正常动画帧被计入 32 次几何重试：360ms 动画提前停在约 72.15px，未达到 80px。最终区分动画推进和几何重试，正常推进不消耗该预算；64 帧、每帧 2ms 的用例不再提前终止。命令仍受两秒期限、几何尝试与停滞保护限制。

每帧不足 0.01px 的位移也曾可能被误计停滞。最终只有时钟推进且宿主实际接受当前采样位置，才视为动画进展；冻结时钟／拒绝移动的宿主仍有界结束，未通过放宽宿主拒绝测试来消除误报。

## 状态尺寸与分配取舍

初版内联 motion 扩大了默认控制器值，因此另做隔离方案。最终 `MasonryCommand` 仅为显式可动画的 Fixed／Known Smooth 请求持有一个 `Box<MasonryMotion>`；Instant、Measured、选项 `reduced_motion(true)` 不分配此对象。系统 reduced-motion 偏好由首帧处理，可能在分配后才降级。运动状态自身因此多一次命令级分配，至多保留一份，终止／取消时释放。

32 次重试／8 次停滞使用饱和 u8 计数，既不溢出，也降低内联状态大小。本机 Windows Release 探针结果：

| `size_of`，字节 | 基线 4230ee5 | 初版 aa5b7f8 | 最终 b83013b |
| --- | ---: | ---: | ---: |
| VirtualizerState | 944 | 952 | 944 |
| VirtualMasonryState | 560 | 616 | 560 |

这是 Rust 控制器值尺寸，不包含拥有的 heap、allocator overhead 或 RSS，不是 ABI 承诺。探针表达式与来源记录在[最终机器记录](virtual-list-smooth-layout-2026-11-05.json)。

隔离方案单独比较 aa5b7f8 → b83013b：Fixed steady +0.30%、Masonry no-op +0.31%、Masonry steady +0.28%，没有测出明确 CPU 改善。保留依据是恢复默认本体尺寸，并接受显式动画的命令级分配代价；不能将其描述成 CPU 加速。

## 六轮 Release 前后成本守卫

使用冻结的[既有 workload](virtual-list-optimization-series-2026-11-04.rs)，LF SHA-256 为 `6d87d2892f997ced2c238fb53be0c6ec28ebcde21d926e8cf67ea08f70e5f9e0`。每轮使用相同 workload 和隔离 vendor fixture，按 before／after／after／before（ABBA）执行。编译守卫在每段前等待两次安静观察，并在运行中和结束时检测；本轮没有被检测到编译干扰而丢弃的样本，没有删除不利数据。

每轮 27 cases、404 行，共六轮 2424 行。`frames-0/1/2/3` 分别是 Fixed／Known／Masonry Measured／一维 Measured；轨迹逐样本去除 20 次预热，记录 120 次工作段。表中 frames 数值是工作段合计耗时的样本中位数，非单帧延迟；正数表示耗时增加，负数表示减少。每版本六个样本，不是统计显著性结论。

### 初版三轮，不删去退化

4230ee5 → aa5b7f8 的完整数据保留在[初版机器记录](virtual-list-smooth-layout-initial-2026-11-05.json)，其中实现结论标记为已被 b83013b 替代：

| case | 首轮 | 复测 | 确认轮 |
| --- | ---: | ---: | ---: |
| Fixed steady | +14.56% | +2.28% | +5.11% |
| Masonry no-op | +10.74% | +8.50% | +8.96% |
| Masonry steady | +1.69% | +2.84% | −0.36% |
| append-1000000 | +2.93% | +3.67% | +3.49% |

不能因为后续时段的绝对耗时变小，就推断状态隔离消除了上述 CPU 成本；直接隔离 A/B 没有给出这种证据。

### 最终两轮，4230ee5 → b83013b

| case | 首轮前 → 后，ms | 变化 | 复测前 → 后，ms | 变化 |
| --- | ---: | ---: | ---: | ---: |
| Fixed steady | 4.34525 → 4.35190 | +0.15% | 3.99680 → 4.03775 | +1.02% |
| Known steady | 3.95710 → 3.90695 | −1.27% | 3.80680 → 3.86390 | +1.50% |
| 一维 Measured steady | 5.55030 → 5.43740 | −2.03% | 5.47055 → 5.37935 | −1.67% |
| Masonry steady | 9.39885 → 9.52370 | +1.33% | 9.41725 → 9.63995 | +2.36% |
| Masonry rapid | 8.82665 → 8.86280 | +0.41% | 8.76210 → 8.77055 | +0.10% |
| Masonry complex | 21.34835 → 21.24565 | −0.48% | 21.21215 → 20.51135 | −3.30% |
| Masonry resize | 10.50370 → 10.44105 | −0.60% | 10.27440 → 10.53630 | +2.55% |
| Masonry size | 8.41395 → 8.73320 | +3.79% | 8.40565 → 8.29230 | −1.35% |
| Masonry no-op | 5.35055 → 5.29340 | −1.07% | 5.32185 → 5.24070 | −1.52% |
| append-1000000 | 0.10470 → 0.09215 | −11.99% | 0.09280 → 0.09625 | +3.72% |

最终两轮 Masonry steady 同方向增加约 1.33%／2.36%，Fixed steady 也增加约 0.15%／1.02%；这是保留时接受的实测成本，不能宣称默认路径零退化或整体加速。其他 case 方向不一，尤其短工作段 append 的大百分比不能外推为稳定收益。六轮全部 27 cases 的分配次数、测量窗口内临时峰值、source reads 和 builds 的逐样本取值集合一致；计数不增加不等于 CPU 耗时不增加。

这些矩阵默认不启用 Smooth，是默认路径成本守卫；基线没有对应 Masonry 动画，不能把它们说成相同动画功能的提速对照。allocator 数据仅限测量线程窗口的 payload，排除了控制器创建，不是 RSS。headless 墙钟工作段受后台负载、热态和调度影响，不代表 CPU 使用率、GPU 完成时间、实际 FPS 或输入到显示延迟。实窗测试另证位置与收敛，不提供这些计时指标。

原始数据：

- 初版：[首轮](virtual-list-series-smooth-guards-2026-11-05.csv)、[复测](virtual-list-series-smooth-guards-repeat-2026-11-05.csv)、[确认轮](virtual-list-series-smooth-guards-confirm-2026-11-05.csv)。
- 状态隔离：[aa5b7f8 → b83013b](virtual-list-series-smooth-isolation-guards-2026-11-05.csv)。
- 最终：[首轮](virtual-list-series-smooth-final-guards-2026-11-05.csv)、[复测](virtual-list-series-smooth-final-guards-repeat-2026-11-05.csv)。

[最终机器记录](virtual-list-smooth-layout-2026-11-05.json)包含来源提交、12 个源码／测试文件 LF 哈希、CSV 原始字节及 LF 哈希、各轮身份、全部 cases 的中位数与计数对比。复现最终两轮（当前生产源码须保持 b83013b）：

```powershell
python tools/compare-virtualization.py --before 4230ee5 --label smooth-final-guards --date 2026-11-05
python tools/compare-virtualization.py --before 4230ee5 --label smooth-final-guards-repeat --date 2026-11-05
```

## 最终验证与回退

最终验证源码均对应 b83013b，未在采样后继续修改生产实现或测试：

- workspace + doctest：850 passed／11 ignored／0 failed；其中 virtual_list 为 136 passed／8 ignored，新增 8 项通过。ignored 未当作通过。
- workspace all-targets Clippy，features `headless-tests,virtual-profile`、`-D warnings` 通过；workspace fmt 与改动 vendor/native test rustfmt 通过。
- vendor native 合同 15 项、Base 合同 2 项通过；生产 manifests／locks 未改动。
- Windows Showcase Release 构建通过；两个实窗测试各 1 passed。布局测试 Grid／Masonry／Horizontal renderer calls 为 264／49／28；smooth 测试实际采到 Grid x/y 与 Masonry 中间位置，最终 Grid 为 (-100,-80)、Masonry offset 为 80。它们不是硬件输入录制或 GPU 性能证据。
- 用独立临时 Git index 从 b83013b 逆向应用 b83013b、aa5b7f8、173c80b，结果 tree 与 4230ee5 一致：`dc42628398a81fd1d67a64aa8f3289b0a40dc9dd`。检查没有改主 index 或工作区，也没有实际回退分支。

采用上述三个实现提交，文档与六轮数据另作一份提交。需要撤销本轮时，先 revert 该文档数据提交，再依次 revert `b83013b`、`aa5b7f8`、`173c80b`，保留已推送历史，不使用 hard reset。当前未交付的自动轨道／跨格／冻结区／通用空间布局需独立需求与消费者验证；Measured smooth 仍使用安全即时策略。
