# 虚拟滚动继续对标：细微尺寸反馈与单项成本

起点 `6022bdf`，分支 `quality/virtualization-next-review-20261104`。本轮继续读取本地 `refer/virtual/{virtua-main,virtual-main,vlist-main}`；没有更新这些参考源码，结论对应仓库中的参考快照，并非对上游最新版本的声明。

## 对标依据与取舍

| 参考项目 | 实际机制 | 本项目的取舍 |
| --- | --- | --- |
| virtua | `src/core/layouts/list.ts` 的 `$isSizeEqual` 用值相等判断尺寸，`$setItemSizes` 提交实际差值 | 不让逐项 epsilon 吞掉可表示的尺寸变化，保留本地几何 owner 和批次原子性 |
| TanStack Virtual | `packages/virtual-core/src/index.ts` 的 `resizeItem` 先读已有尺寸，仅在 `delta !== 0` 时写入、更新缓存版本；单 lane 直接读尺寸 | 单项反馈无需创建去重集合和索引 Vec；仍先验证 key、尺寸、exact/hint 和 cross 身份 |
| vlist | `src/plugins/autosize/plugin.ts` 把 gap 加到尺寸后，比较 `sizeWithGap !== oldSize`；`src/rendering/sizes.ts` 使用分块前缀 | 本地原生 Measured 也按存储的 f32 含 gap slot 比较；不先减 gap 后做近似比较，不另建尺寸表 |

本轮同时检查了 measurement cache 恢复、尺寸批次和阅读锚点。已有 cache schema/content/cross 校验和三种区域锚点已有测试覆盖，未找到需要在本轮修改的缓存缺陷；这不等于穷尽证明。选择有确定失败测试的尺寸精度问题和有分配证据的单项热点，没有为追求接口数量复制参考项目全部选项。

## 1. 采用：可表示的细微尺寸变化不得被丢弃

实现前，一维 `apply_sizes` 把差值小于 `0.001px` 当作无变化；Masonry 的单项、多项及原生布局反馈使用 `0.01px` 容差。三项新增回归在原实现上全部失败，原始输出保留在 `virtual-list-feedback-before-2026-11-04.txt`。

- 8192 项从 `20px` 增加到 `20 + 1/2048px`，期望总高 `163844px`，原实现仍为 `163840px`。各项的小差值累计成了 `4px`，并影响列表中部的阅读锚点。
- hints 的可表示变化被吞掉，旧 writable snapshot 也没有失效。
- Masonry 从 `20px` 变成 `20 + 1/256px`，原单项反馈仍返回 `20px`；批次路径有相同判定。

修正 `3fc5dd2` 只改变尺寸 no-op 的判定，不改变滚动命令的收敛容差：Known/Masonry 比较存储的 f32 尺寸；原生 Measured 比较含 gap 的 f32 slot，避免先减 gap 引入舍入差而把重复值当成变化。exact/hint 标记和真实跨轴 constraint 仍必须相同，才能判为 no-op。Masonry 原生布局使用相同的值变化规则。

五项专项回归覆盖双轴、Known/Measured/hints、单项与倒序批次、128 项块边界、总尺寸/前缀、冻结快照、锚点、重复提交、非法输入整批拒绝和 Fixed 后端拒绝。原生 GPUI 布局测试从稍有误差的已测尺寸开始，验证实际布局能校正、离屏项不被全量测量、稳定后不再增加 revision 或产生持续帧循环。精度测试先证明行为，性能比较使用前后语义一致的普通 40/41px 反馈，不把错误地跳过写入作为“优化”。

精度修正的 Release ABBA 普通反馈矩阵包含 Known/真实测量/hints、空/1/32/256 项、无变化/实际变化，每样本 2000 次调用、每版本十样本。全部分配、临时峰值和 source 访问计数相同。非空 case 中位耗时变化为 `-2.24%～+3.97%`，不据此声称 CPU 收益。17 类帧轨迹以及查询、追加、工作集保护的计数也全部相同；帧轨迹耗时为 `-3.24%～+3.38%`。采用理由是修正可复现的累计位置错误，不是帧率提升。

数据：[普通反馈](virtual-list-series-next-feedback-exact-linear-guarded-2026-11-04.csv)、[帧与工作集保护](virtual-list-series-next-feedback-exact-guards-2026-11-04.csv)。

## 2. 采用：一维单项反馈不创建去重/索引缓冲

行为基线 `b016579`（已包含精度修正），采用实现 `5b813d5`。单项输入没有重复项，在原状态 owner 内直接校验 key 和尺寸，再复用相同的 stored-size/exact/cross 判定。Known 的真实更新仍由原持久前缀树提交；Measured 使用栈上的一个 native tuple 交给 ListState。没有持久 scratch、尺寸镜像或新依赖。

独立 helper 不内联进多项反馈主体，多项仍先校验整个批次再一次提交；单项、hint、空批次、非法 key/零/NaN/无穷、重复拒绝、旧快照和锚点合同在优化前后均通过。

两个独立 Release ABBA，每样本 2000 次调用、每版本十样本，普通高度在 40/41px 间交替，包含输入构造成本：

| 单项 case | 首轮 ms 前→后 | 独立复测 ms 前→后 | 分配次数前→后 | 临时峰值 B 前→后 |
| --- | --- | --- | --- | --- |
| Known，无变化 | 0.19235→0.04185 | 0.17205→0.04230 | 4000→0 | 68→0 |
| Measured，真实测量，无变化 | 0.29365→0.17560 | 0.30035→0.17970 | 4000→0 | 68→0 |
| Measured，hint，无变化 | 0.30120→0.18020 | 0.31180→0.18155 | 4000→0 | 68→0 |
| Known，实际变化 | 4.07485→3.89465 | 4.09375→3.95000 | 31986→27986 | 9804→9736 |
| Measured，真实测量，实际变化 | 8.34970→8.09790 | 8.25815→8.21685 | 37983→33983 | 27716→27648 |
| Measured，hint，实际变化 | 8.36205→8.11135 | 8.26860→8.08655 | 37983→33983 | 27716→27648 |

无变化单项 Known 耗时下降 `75.41%～78.24%`，Measured/hints 下降 `40.17%～41.77%`。真实变化减少分配 `4000` 次，Known 约 `12.51%`、Measured/hints 约 `10.53%`；真实 Measured 复测耗时仅 `-0.50%`，不把它表述为显著 CPU 收益。source 查询次数在前后相同：无变化单项为 2000，实际变化为 3999（额外查询来自相同的锚点恢复）。

代价：32/256 项 Known 无变化批次，首轮耗时分别 `+2.03%/+2.89%`，复测 `+1.24%/+1.35%`；不隐藏这项小幅重复观测。其余多项 case 在复测 `-1.56%～+0.81%`；全部多项分配、峰值和 source 次数不变。采用的是单项调用的明确分配收益，不能宣称所有反馈路径更快。

17 类帧轨迹的 source/构建/分配/峰值计数仍全部相同。该轮耗时 `-10.19%～+3.86%`，既有正也有负；Fixed 稳定帧并未修改却出现较大下降，更说明不能把它归因于单项接口优化。Known 的四类轨迹为 `+3.86%/+2.13%/+2.24%/+1.37%`，明确未观察到整体帧收益。没有减少呈现内容，也没有通过漏测获得收益。

数据：[单项与批量首轮](virtual-list-series-next-feedback-single-2026-11-04.csv)、[独立复测](virtual-list-series-next-feedback-single-repeat-2026-11-04.csv)、[帧与工作集保护](virtual-list-series-next-feedback-single-guards-2026-11-04.csv)。

## 测量方法与限制

沿用隔离 Git 归档的 Release ABBA，before/after/after/before，工作量和 workload 完全相同。`ns` 是主线程工作段的墙钟耗时，heap 只计该线程测量窗口内的分配 payload；不等于进程 CPU 使用率、RSS 或 GPU 帧率，不包含预存模型。

首轮 `next-feedback-exact-linear` 采样期间发现同机其他项目开始编译，该轮耗时数据明确排除，不用于收益判断。随后 `131eba9` 加强采样器：每轮之前重新等待两次无编译观察，运行期间每秒及结束时检查；观测到编译则保留原日志、丢弃该轮并重试，最多三次。没有中断其他项目。受保护的重测实际丢弃了 `after-1-attempt-1.txt`，随后完成四轮。轮询不能保证发现所有极短进程，也不能排除系统调度或非编译负载；分配计数、行为回归和独立重复测量仍需一起判断。

本轮没有增加新 UI、依赖、持久缓存或第二个滚动 owner。原生 GPUI headless 布局和实际 Windows/GPU 窗口验收分别报告；不把 headless 结果当作跨显卡、DPI 或物理帧节奏结论。

六组数据（五组采用、一组排除）的 CSV 行数/来源提交、源码和 workload 哈希、逐 case 计数核对、17 项原生/Base 合同及最终验证摘要见[来源与验收记录](virtual-list-feedback-2026-11-04.json)。CSV/源码哈希按 CRLF→LF 规范化；原工具记录的 workload 原始字节哈希另外保留，已与本轮实际输入核对。受干扰的[首轮数据](virtual-list-series-next-feedback-exact-linear-2026-11-04.csv)保留用于审计，不进入性能结论。

## 最终验证

以下针对采用源码 `5b813d5` 完成：

- 完整 workspace + doctest：`830 passed / 10 ignored / 0 failed`；其中 virtual_list 为 `116 passed / 8 ignored`。新增五项专项测试均通过；ignored 为既有手动探针，性能 workload 和实窗探针另行执行。
- `tools/test-virtualization-contracts.py --offline`：15 项 native + 2 项 Base 合同通过，生产 manifests/locks 未改动。
- workspace all-targets headless Clippy `-D warnings` 通过；workspace fmt、三份改动 vendor 源码及 workload 的 rustfmt、Python AST 语法和 `git diff --check` 通过。
- Windows Showcase Release 构建通过；`tools/test-virtualization-native.ps1 -Offline -Release` 实际窗口探针 `1 passed`，覆盖 Grid 尾行/尾列、Masonry 重排、横向 Measured 定位/收尾。真实 renderer 调用分别为 264/49/28，不作为帧率指标。
- 五组有效 ABBA 的 before/after 来源、workload 哈希和计数核对通过；两组完整保护矩阵的每个 case，在全部样本上 source/构建/分配/峰值计数集合一致，不只核对中位数。采用的 singleton 源码与提交 `5b813d5` 一致。

## 保留的边界与下一步价值判断

- f32 单项尺寸和原生含 gap slot 的舍入边界仍在，不能把“保留可表示变化”扩大成任意精度承诺。极端大 gap/微小内容尺寸或超大绝对坐标仍须按主设计第 13.3 节审查。
- Known/Masonry 冷构造仍为 O(N)，Masonry 追加仍复制 O(N/256) 的共享块目录。本轮没有重新引入之前已因热路径/峰值代价被拒绝的整批方案。
- RTL/反向布局、Masonry Attached/offset/follow-tail/smooth、由可见单元格推断全局 Grid 动态行高仍未实现。这些需要新的所有权/布局合同和场景验证，不能靠扩大 capabilities 声明补齐。
- 继续微优化时应优先依据真实调用频率选择路径；本轮 32/256 项无变化批量仍创建临时去重/索引缓冲，不能从单项零分配推断整批也已零分配。

## 复现与 Git 回退

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list feedback:: --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1
python tools/compare-virtualization.py --before 5c80863 --label feedback-exact-recheck --filter next_linear_feedback --workload docs/research/virtual-list-adoption-2026-11-04.rs
python tools/compare-virtualization.py --before b016579 --label feedback-single-recheck --filter next_linear_feedback --workload docs/research/virtual-list-adoption-2026-11-04.rs
python tools/compare-virtualization.py --before b016579 --label feedback-single-guards-recheck --filter series_
```

| 作用 | 提交 |
| --- | --- |
| 本轮起点 | `6022bdf` |
| 先失败的三项精度回归/workload | `532c254` |
| 原生 Masonry 精度/稳定帧回归 | `5c80863` |
| 精度修正 | `3fc5dd2` |
| 采样期间的编译干扰检查 | `131eba9` |
| 优化前已通过的线性批次合同 | `b016579` |
| 采用的线性单项优化 | `5b813d5` |

`git revert 5b813d5` 可只撤销性能优化，保留精度修正和回归测试；已通过 `git apply --reverse --check` 检查，未实际改动工作区。若需撤销精度修正，先撤销依赖它的单项实现，并相应处理新回归测试；不应在保留新合同测试的同时期待旧缺陷通过。完整历史审查可从干净工作区执行 `git switch -c review/virtual-feedback-before 6022bdf`，不覆盖当前分支，不使用 hard reset 丢弃证据。

没有推送，也没有修改参考项目和生产依赖清单/锁文件。
