# 虚拟滚动四项验收（2026-11-05）

本轮按观察尺寸消费者、组合回归、大量 span、Windows 实窗依次验证。每项独立提交；参考目录只读，生产 manifests / locks 不变。当前公共能力仍以[组件文档](../28-virtual-list.md#26-布局选择与二维边界)为准。

## 1. Grid 观察尺寸的等价消费者

已完成，保留私有消费者案例，没有新增 `VirtualGridObservedSizes` 公共 API。源码为 [grid_observed.rs](../../crates/yue-component/tests/virtual_list/grid_observed.rs)，计时入口为 [performance.rs](../../crates/yue-component/tests/virtual_list/performance.rs) 的 `grid_observed_equivalent_evidence`。

两个实现使用相同 `VirtualGridCellKey`、有限 track 域、唯一 cell 预算、默认尺寸、完整输出及批量原子校验。Cells 路径保存 cell 最大值再按轴聚合；Axes 路径保存已见 key 和轴最大值。同 epoch 只增长；显式新 epoch 清空样本并恢复默认，允许收缩；过期反馈、缺失 key、非法几何、超预算整批拒绝。业务内容、源域或布局改变需要消费者开启新 epoch（域改变还需重建域索引）。这不是内容 intrinsic layout，存储为 O(R+C+budget)，不适用于无元数据的百万轨道默认路径。

借鉴 TanStack 的完整输出等价及独立 oracle 思路，普通测试用独立历史扫描核对全部行列，覆盖 1/7/64 轨道、重复反馈、三个 epoch、缩小及原子失败。实际 Known 两轴消费者测试覆盖增长、收缩、阅读 key 相对位置和迟到快照拒绝。每轴分别原子，不声称双轴联合事务。

两次 Release / offline / locked 运行，每种场景按 Cells–Axes–Axes–Cells 两轮执行，所有样本及源码身份见[机器记录](virtual-list-four-acceptance-observed-2026-11-05.json)。每个样本 40 轮、64×64 cells；warm/reset 各轮含 16 帧稀疏反馈，warm 保持同 epoch，reset 复用容量并开始新 epoch；warm 的首轮初始化也计时。两路径都校验全批、生成完整有序向量、消费所有值，计时外逐帧断言相等。

| 场景 | Cells 全部样本 | Axes 全部样本 |
| --- | ---: | ---: |
| 冷启动 | 34.28–35.79 ms | 32.86–37.24 ms |
| 跨帧复用 | 99.29–102.51 ms | 45.62–52.98 ms |
| epoch 重置并复用容量 | 105.77–117.37 ms | 51.09–54.44 ms |

峰值暂存／保留 payload 为 Cells 1,180,496 B、Axes 1,082,192 B；相同场景分配次数相同。冷启动 Axes 的 37.24ms 不利样本保留；不能宣称所有场景都更快，更不能换算成 FPS。新结果修正[旧微实验的非等价限制](2026-11-05-virtual-list-next-opportunities.md#grid-已观察尺寸-epoch-候选不吸收2026-11-05)，不覆盖或删除旧证据。

决策：吸收“消费者按轴保存最大值、显式 epoch 失效”的案例与回归，暂不提升为 Base API。这里的有限域索引、默认值、失效时机属于应用合同；没有第二个真实消费者证明通用边界。无需为该实验增加 Grid 默认路径缓存。

验证：3 个普通回归通过；等价 Release 手动探针两次各 1 passed；目标 Clippy `-D warnings` 通过。

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list observed_consumer --offline --locked -- --nocapture --test-threads=1
cargo test -p yue-component --features headless-tests --test virtual_list grid_observed_equivalent_evidence --release --offline --locked -- --ignored --nocapture --test-threads=1
```

## 2. 多能力组合回归

已完成，新增 [grid_combinations.rs](../../crates/yue-component/tests/virtual_list/grid_combinations.rs) 的生产组件组合轨迹。32 次挂载交替 LTR/RTL；同一个 Grid 同时开启冻结首行／列、2×2 span、稀疏保留及远端聚焦 editor，然后交错双轴定位、Known 冻结尺寸反馈、resize、源重排、50ms 动画采样、真实 `ScrollWheelEvent` 取消动画、撤销焦点注册、带待执行动画和 idle timer 卸载。

每轮 5 个阶段分别通过独立线性几何 oracle 检查四区共 20 次原生指针命中（合计 640 次），不是仅检查 renderer 调用数；语义 cell key 全部唯一，工作集小于 120。锚点检查使用 stable key 到冻结 body 边缘的相对位置；重排后未变 key 的编辑内容仍保留，变动索引的旧内容释放。每轮卸载后两轴及所有 cell view 的 weak 引用失效。第一轮故意保留一个强引用作负控制，确认它仍存活，再释放并验证归零。

借鉴 TanStack 的命令／尺寸／重排／frame 混合轨迹、virtua 的阅读锚点、vlist 的 scoped 生命周期和真实 hit 断言，没有运行参考项目的浏览器测试来代替 GPUI 测试。初始化类型／借用编译错误均已修正；初次运行还暴露测试自身两个错误假设（点击会正常移走焦点、未变内容身份应复用），已改成在指针输入前检查焦点，并只要求变动身份释放。最终组合测试 1 passed，目标 Clippy `-D warnings` 通过；生产代码无需修改。

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list mixed_grid_trace --offline --locked -- --nocapture --test-threads=1
```

## 3. 大量 span 成本

已完成并采用两处局部优化：同起始行的已验证非重叠 spans 按列有序，可以剪枝列区间；呈现时复用现有 row-max 树访问与稀疏工作轨道相交的 spans，替换每帧扫描所有描述符。没有增加索引、缓存或公共 API，配置验证仍为 O(S log S)，构建成本没有被消除。一般二维分布的极端查询仍可能为 O(S)，不声称最坏情况保证为 O(log S)。

先提交 `3347e14` 固定 workload，再建立 before 可执行文件；生产优化随后测试。Base 两个普通 oracle 测试分别覆盖点查询和稀疏矩形相交：空集、半开边界、间隙、反序输入、同起始行但不同高度，以及冻结区与远端工作集之间的大空洞。两项通过；完整 virtualization 目标 182 passed / 10 ignored。既有离屏 span origin 唯一呈现、冻结边界、重排、巨大 span 和新组合生命周期测试也都通过。

独立微探针的两轮 ABBA（640 行原始样本）见[CSV](virtual-list-four-acceptance-span-queries-2026-11-05.csv)和[可执行文件／锁身份](virtual-list-four-acceptance-span-queries-2026-11-05.json)。每样本配置 20 次，或点查询 2,000 次，二者分开计时。10,000 个同起始行 spans 的查询中位数从 24.400ms 降为 0.147ms；分散行从 0.169ms 变为 0.188ms，增加约 0.019ms / 2,000 次查询。该不利结果保留，不宣称每个微场景都更快。

完整生产 Grid [工作负载](virtual-list-span-cost-2026-11-05.rs)使用同一源码、同一依赖记录、隔离 vendor 和 Release ABBA，重复两次。每样本 64 次双轴移动＋notify＋draw，5 个样本／进程。每次 draw 在计时外与独立几何 oracle 比较完整 cell 集合，span 尺寸也逐项断言；不是仅消费数量。配置时间在真实 render 内单独累计，剩余时间包含其他 render/layout/prepaint/paint 工作。保留全部时序及 p50/p95/p99：[第一轮](virtual-list-series-span-index-2026-11-05.csv)、[重复轮](virtual-list-series-span-index-repeat-2026-11-05.csv)、[身份与分场景统计](virtual-list-four-acceptance-spans-2026-11-05.json)。

| 分布／span 数 | 第一轮总 CPU 变化 | 重复轮总 CPU 变化 |
| --- | ---: | ---: |
| 分散行／0 | −3.2% | −2.3% |
| 分散行／100 | −3.1% | +0.8% |
| 分散行／1,000 | −2.6% | −1.9% |
| 分散行／10,000 | −2.3% | +0.6% |
| 同起始行／0 | −2.3% | −1.3% |
| 同起始行／100 | −5.1% | −7.6% |
| 同起始行／1,000 | −9.2% | −13.2% |
| 同起始行／10,000 | −9.8% | −10.7% |

10,000 同行 spans 的第一轮总量为 210.85→190.11ms / 64 次 draw，扣配置后的工作为 35.65→11.35ms；配置仍占主要成本。普通分散行总量基本持平，不把小幅波动解释为稳定收益。1,000 同行 spans 的首轮 p99 中位数曾增加 15.2%，原始样本保留；采用理由是热点场景两轮总工作稳定下降、默认零 span 无新增持有结构、回归和完整输出通过，而不是所有尾部分位都下降。本次没有测 draw 工作负载堆分配或真实显示 FPS。

```powershell
python tools/compare-virtualization.py --before 3347e14 --label <new-label> --date 2026-11-05 --filter series_span_frames --workload docs/research/virtual-list-span-cost-2026-11-05.rs
```

## 4. Windows 实窗验收

已完成当前 Windows 环境可执行的原生窗口、输入、UIA 和时序检查，并修复实窗发现的 Showcase 消费者语义缺口。跨 DPI／多显示器迁移和 Narrator 实际朗读仍未覆盖，不能把本项写成所有硬件条件均已验收。

新增 [native_acceptance.rs](../../examples/windows-showcase/src/pages/virtual_layouts/native_acceptance.rs)：自动探针在真实 Windows 窗口运行 24 轮 1100×760／900×680 resize、16／20 rem、LTR／RTL、span 与双轴定位，检查逻辑位置、RTL 原生投影、冻结工作集有界；实际最小化至不可见后激活恢复，阅读偏移保持。手动 helper 只打开生产 `VirtualLayoutsPage`，记录状态并设定约 300 秒的等待循环，实际退出受系统调度影响；其 `1 passed` 仅代表窗口流程结束，不代表自动通过视觉／读屏断言。原生 runner 新增 `-IncludeFrameProfile`，5 个探针各自独立进程，避免共享 Win32 全局状态。

本机 GPUI 只枚举到 1 个显示环境，逻辑 2560×1440，真实 scale=1.5，刷新间隔 6.25ms。系统枚举的活动显示适配器为 RTX 4070 Ti SUPER，另有 GameViewer 虚拟适配器；没有独立查询 GPUI 最终渲染 adapter 的身份。Windows、CPU、驱动、编译器、锁文件和源码身份见[机器记录](virtual-list-four-acceptance-native-2026-11-05.json)。改变 rem 是字体／界面缩放测试，不是更换显示器或真实 DPI 迁移。

### 原生输入与可访问性修复

通过原生窗口注入键盘／滚轮并观察 UIA 和截图：Grid 的冻结首行首列、RTL 镜像、2×3 合并 origin 唯一呈现、合并覆盖项不重复，以及滚动后的命名 cell 工作集均符合观察。修复后双轴滚动的 UIA 为 108 个 cell，包含冻结条带；工具注入距离和实际逻辑位移并不相同，实际 `x=700.0052490234375,y=1750` 已记录，不能拿请求的滚轮数值声称同等像素位移或输入延迟。

本轮发现两处真实缺口：Masonry 显示卡片但 UIA 的 List 没有条目；横向 Measured 有 ListItem 却没有名称。先新增生产页回归，得到 0 passed／2 failed，再在 renderer 根补 `ListItem`、复用可见标题的名称、内容说明及集合位置／总数；横向关闭默认 item role，避免重复无名包装。没有给 Base 增加业务命名 API，也没有修改可见布局。两个回归随后通过，覆盖滚动后的名称替换、Masonry 缓存重建／五列 reflow、横向第 2000 项和语义工作集有界／名称唯一。

原生 UIA 复验中，Masonry 初始／滚后分别有 14 个命名条目，重建后 17 个，五列 reflow 后 24 个；名称随卡片与列归属更新。横向初始有 7 个命名条目，末端 6 个，原生向前滚动后为“节点 1987”至“节点 1993”共 7 个。键盘 Right 切换标签、按钮操作、窗口关闭退出均已实际观察。前后 [UIA 摘录](virtual-list-four-acceptance-uia-2026-11-05.txt)及[失败／通过、实窗状态记录](virtual-list-four-acceptance-native-transcript-2026-11-05.txt)保留；截图当场检查，未归档成图片文件。

这些证据确认 UIA 条目可识别，不等于 Narrator 语音、浏览模式或全文朗读均已通过。本轮未启动 Narrator，也没有采集／试听声音。仍需在带声音的读屏环境完成该项；不将 UIA 名称检查改写为“已通过屏幕阅读器验收”。

### 原生 draw／present 工作样本

修复后 Release、offline、locked 的 5 个独立原生探针全部通过。Frame 探针含 Grid、Masonry 滚动、Masonry 滚动中 reflow、横向 Measured，分别在 rem 16／20 下运行；每组丢弃 12 个预热 step，记录 96 个 draw，共 768 个最终样本。修复前的另一组 768 个样本一并保留，[原始 CSV](virtual-list-four-acceptance-native-2026-11-05.csv)合计 1,536 行，不挑选有利轮次。解析使用行内 `FRAME,case=`，包含 libtest 前缀后的首条数据，避免每进程漏掉首样本。

| 场景／rem | draw P50 | draw P95 | draw P99 | draw 超过 6.25ms |
| --- | ---: | ---: | ---: | ---: |
| Grid／16 | 3.515 ms | 5.303 ms | 6.633 ms | 1 / 96 |
| Grid／20 | 3.867 ms | 5.577 ms | 5.957 ms | 0 / 96 |
| Masonry 滚动／16 | 1.654 ms | 2.575 ms | 3.145 ms | 0 / 96 |
| Masonry 滚动／20 | 1.383 ms | 2.432 ms | 3.675 ms | 0 / 96 |
| Masonry reflow／16 | 1.665 ms | 2.629 ms | 3.255 ms | 0 / 96 |
| Masonry reflow／20 | 1.776 ms | 2.730 ms | 3.241 ms | 0 / 96 |
| 横向 Measured／16 | 1.109 ms | 1.544 ms | 1.803 ms | 0 / 96 |
| 横向 Measured／20 | 1.106 ms | 1.654 ms | 2.040 ms | 0 / 96 |

P50 为中位数，P95／P99 为 nearest rank；全部分段与 CPU present 时长见 JSON／CSV。这里是原生应用侧 draw／present 工作，输入由 owner 定时写入；不是 GPU 完成时间、实际呈现 FPS 或硬件输入至显示延迟。Grid 的不利尾样本保留，因此不承诺稳定 160 FPS。可访问性修复前后的两轮并非 ABBA，语义输出也不同，且旧二进制重建前未另存哈希：旧样本只作历史环境记录，不作为“语义修复零开销”或性能采用依据。

```powershell
& tools/test-virtualization-native.ps1 -Release -Offline -IncludeFrameProfile
# 手动验收 helper：只在明确授权真实桌面操作时运行。
cargo test -p windows-showcase --release --features virtual-profile --offline --locked native_interactive_acceptance_surface -- --ignored --nocapture --test-threads=1
```

## 最终验证与回退

- workspace（`headless-tests,virtual-profile`）：898 passed／17 ignored，包含 36 个 doctest；其中 virtual_list 182 passed／10 ignored。ignored 的性能／实窗探针不算普通自动覆盖，实窗 5 项另行运行并通过。
- workspace all-targets Clippy `-D warnings`：通过。
- 生产 pin 隔离 fixture：16 个 GPUI 原生合同＋4 个 Base 合同通过，生产 manifests／locks 不变；报告身份已纳入 native JSON。
- yue-component 无默认特性＋`headless-tests`：866 passed／10 ignored；`cargo fmt --all -- --check`、`git diff --check` 及 99 个本地文档链接／锚点检查通过。原始样本、源码哈希和归档文件哈希已核对；提交推送状态由最终 Git 检查确认。

```powershell
cargo test --workspace --features headless-tests,virtual-profile --offline --locked -- --test-threads=1
cargo clippy --workspace --all-targets --features headless-tests,virtual-profile --offline --locked -- -D warnings
python tools/test-virtualization-contracts.py --offline
cargo test -p yue-component --no-default-features --features headless-tests --offline --locked -- --test-threads=1
cargo fmt --all -- --check
```

| 项目 | 独立 Git 点 | 回退范围 |
| --- | --- | --- |
| 等价观察尺寸消费者 | `dbd9512` | 私有案例、等价计时与第 1 项记录 |
| 组合回归 | `a4a7bb5` | 混合轨迹及生命周期回归，不改生产行为 |
| span 基线 | `3347e14` | 固定优化前 workload |
| span 优化 | `954a94e` | 两处局部树剪枝与 oracle；可单独 revert 恢复旧生产路径 |
| Windows 验收与语义修复 | 本文同批提交 | 原生探针、runner、Showcase 命名条目与第 4 项证据；可按提交独立 revert |

参考目录保持只读；不执行 reset／force push。后续真正剩下的是跨 DPI／多显示器与 Narrator 声音验收，以及由真实消费者证明是否需要通用观察尺寸 API；不是再次把已完成的四项从头实施。
