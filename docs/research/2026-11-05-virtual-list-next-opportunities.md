# 下一轮虚拟滚动优化候选（2026-11-05）

审查基线为 `f0f7583b93bd0a9041fda08a1debf7b38d77d9c0`，包含此前已采用的 smooth、Grid 动效策略与可选 motion 状态隔离。本轮重新阅读本地三个参考项目、当前 Base 源码及历史实验，并增加两个观察探针；未修改生产实现、能力声明或默认值。下列内容是候选与验收条件，不是已获得的性能提升。

## 建议顺序

| 顺序 | 候选 | 当前事实 | 预期价值与限制 |
| --- | --- | --- | --- |
| 1 | Grid 单元格内容准备复用 | 列表／Masonry 已有 item views，Grid 只有逐次 renderer | 先减少复杂单元格的重复准备；轻量 cell 可能不值得增加缓存 |
| 2 | 有界方向性 overscan | 列表与 Masonry 前后对称；Grid 两轴工作集相乘 | 先在已有方向状态的列表／Grid 试验 opt-in，保留反向余量；默认 overscan=0 的路径不会自动获益 |
| 3 | Masonry 滚动活动／结束通知 | 命令取消与 follow-tail 已有，尚无一维同等的活动／方向／idle 公共合同 | 支持昂贵内容延迟准备与业务通知，也是将方向缓冲推广到 Masonry 的前提 |
| 4 | Masonry 纯值测量缓存与阅读锚点恢复 | 一维有 portable measurement cache，Masonry 只有活的几何快照 | 控制器销毁后返回页面时减少重测；若应用一直保留 Entity，价值较低 |

先实验 1，再在列表／Grid 实验 2；Masonry 的方向策略须先完成 3。第 4 项应以真实的页面销毁／恢复消费者为依据。布局增强与热路径性能分别验收，不将缺少功能直接认定为算法缺陷。

## 1. Grid 内容准备复用：最值得先测

当前 [grid.rs](../../vendor/gpui-base/src/virtualization/grid.rs) 的 `prepaint` 遍历 `snapshot.cells()`，每个 cell 都调用 `self.render`，构造本帧元素。一维 [element.rs](../../vendor/gpui-base/src/virtualization/element.rs) 和 [masonry_element.rs](../../vendor/gpui-base/src/virtualization/masonry_element.rs) 已有 `new_with_item_views`，按内容身份复用 `AnyView`，保留本帧 layout／prepaint／paint；Grid 尚无对应入口。

[vlist pipeline](../../refer/virtual/vlist-main/src/core/pipeline.ts) 分别追踪 `_lastItem`、`_lastOffset`、`_lastSize`，内容不变时不重复调用 template，位置和尺寸各自更新。可借鉴内容与几何分离，不能直接移植 DOM 池或跨帧保存 `AnyElement`。

本轮生产接缝观察：每轴 100 万项，240×100px 视口，行高 20px、列宽 50px，overscan 为默认 0。初次挂载后，连续 10 次 owner notify + draw，数据、尺寸和视口均未改变：

| 观察 | 首次锁定依赖运行 | 独立重复运行 |
| --- | ---: | ---: |
| 不同单元格身份 | 25 | 25 |
| renderer 总调用 | 500 | 500 |
| 每次显式 draw 周期调用 | 50 | 50 |

一个显式 draw 周期包含 GPUI 的内部处理，不能将它等同于一个显示帧。这里没有测 factory 的业务耗时、分配、CPU 提交或 FPS；500 次回调本身不证明有 500 次昂贵准备，应用也可能在 renderer 外已有缓存。

候选方案是增加 opt-in 的 cell view 准备入口，身份由行 key、列 key 组成；内容 stamp 覆盖两轴的 source revision、内容失效身份、索引及消费方外部内容 revision。几何仍逐帧输入，不把位置／尺寸塞进不可变准备身份。缓存只保留工作集及明确保留项，退出或卸载后释放，复用现有唯一 scroll owner。接口名和具体形状须在实现时设计，此处不声明存在新的可调用 API。

吸收门槛：

- 稳定内容的重复 draw 不再重做昂贵 factory，但正常 reactive Render、布局与绘制继续运行。
- 单元格进入工作集才准备；离开后释放；row／column key、内容 revision、显式失效、source replacement、同 ID 换 owner 均正确。
- 单纯位置变化复用内容；受宽度、主题或外部数据影响的内容遵守明确的 Entity notify／revision 合同。
- 焦点保留、空集合、卸载和消费者持有子 Entity 的生命周期有回归。
- 同一复杂 cell 的旧 renderer 路径与新 opt-in 路径做 Release ABBA，另测轻量 cell、快速双轴滚动、resize、局部更新。比较准备次数、分配／峰值、耗时分布与冷启动；轻量默认路径不能被无谓加重。

## 2. 方向性缓冲：二维放大使它更值得测

[virtua store 的 `$getRange`](../../refer/virtual/virtua-main/src/core/store.ts) 在向下滚动时只扩展后方、向上时只扩展前方，idle 时扩展两侧，估算尚在进行时还会减少缓冲。其意图是控制额外工作，不是保证任意速度都无空白。

本项目 [state.rs::snapshot](../../vendor/gpui-base/src/virtualization/state.rs) 使用 `offset - overscan` 与 `extent + 2 * overscan`；[Masonry working_indices](../../vendor/gpui-base/src/virtualization/masonry.rs) 相同。Grid 将两轴工作集相乘，所以非零 overscan 对复杂单元格会有乘积成本。

本轮纯几何探针保持与上节相同的行列尺寸和视口，横纵 offset 均为 1000px，无额外 retained／focus 项：

| 每轴对称 overscan | 工作集行 | 工作集列 | 工作集单元格 |
| --- | ---: | ---: | ---: |
| 0px，当前默认 | 5 | 5 | 25 |
| 100px | 15 | 9 | 135 |
| 200px | 25 | 13 | 325 |

两次运行结果相同。325 是这组输入的实际工作集计数，不是全量 N×M，也不是性能退化倍数；它来自调用方显式设置缓冲，不能称为默认路径缺陷。

先保持默认策略不变，试验“运动方向保持足够提前量、反向保留小余量、idle 恢复两侧”的 opt-in 策略。列表／Grid 已有活动和方向状态；Masonry 应先补第 3 项。不能通过裁掉必需可见项或焦点项来满足总 cell 预算，也不能在方向变化的首帧沿用过期缓冲。

吸收门槛：相同前向提前量下，往返、突然换向、滚动条跳转、双轴斜向、边缘 clamp、动态改高和 retained／focus 场景完整覆盖；工作集、renderer／测量次数、CPU 工作段 p95／p99 与临时峰值有净收益，实窗不增加可见空白。没有证据就保留 opt-in，不改变默认，不引入无依据的速度预测或第二调度器。

## 3. Masonry 生命周期：为昂贵内容调度提供真实信号

[一维 state](../../vendor/gpui-base/src/virtualization/state.rs) 已提供 `is_scrolling`、`scroll_direction`、`ScrollStateChanged`、`ScrollEnd` 及单个可取消的 idle task。[Masonry](../../vendor/gpui-base/src/virtualization/masonry.rs) 的 `user_scroll_activity` 当前负责取消定位、触发 follow-tail 输入确认；它没有同等的活动／方向／idle 查询与通知。新增 smooth 并不补齐这一能力。

[virtua store](../../refer/virtual/virtua-main/src/core/store.ts) 区分 scroll、scroll-end 通知；[TanStack Virtual](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 保留 `isScrolling`／`scrollDirection`；[vlist events](../../refer/virtual/vlist-main/src/types.ts) 提供 `scroll:idle`。这些信号使消费者可以在滚动中显示轻量内容、结束后准备重内容，业务无需为每张卡片各开 timer。

可吸收共同语义，不为 API 对称强行合并两种几何算法。Masonry 的多 lane 工作集可能是稀疏索引，不能直接冒充一段连续的 `RangeChanged`。若将来需要工作集通知，应先定义准确的数据形状。

吸收门槛：用户 wheel、原生位置变化、程序命令和测量锚点补偿区分清楚；结束事件次数与顺序稳定，稳定帧不重复通知，换向及时，卸载／换 owner 后旧 task 不再回调。idle 由一个 Entity 级可取消任务负责，不给每个条目增加任务。收益按消费者实际避免的准备工作和通知量测量，不声称几何计算因此自动加速。

## 4. Masonry 冷恢复缓存：只在需要销毁控制器时做

[cache.rs](../../vendor/gpui-base/src/virtualization/cache.rs) 已有一维的纯值测量缓存：key、内容 revision、尺寸与排版约束，并恢复阅读锚点；Masonry 尚无同类 portable cache。现有 `VirtualMasonrySnapshot` 保留 source 和持久几何的活快照身份，不能直接当成跨控制器的存储格式。

[TanStack `takeSnapshot`](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 导出已测项，重挂载时与 scrollOffset 配对。源码仍可能遍历 measurements，不能总结为导出成本只与已测项数相关，也不能忽略 lanes 对恢复几何的影响。

候选缓存只保存纯值，不带 Entity、ScrollHandle、owner token 或旧 source。身份至少要考虑 key／内容 revision、列数、列实际宽度、主轴／列 gap、排版 revision，以及恢复时的分栏／reflow 规则；不兼容先拒绝或显式降为 hints，不能把别的列宽测量重贴标签。key 锚点比单独绝对 offset 更适合内容变化后的阅读恢复。

吸收门槛：控制器销毁再建、删除／追加／改内容、列数／列宽／字体变化、重复或非法记录、原子失败、旧快照迟到反馈全部覆盖；测量复用率、重测次数、恢复耗时和缓存大小可量化。冷恢复不等于免除当前 Masonry 构造／打包的 O(N) 元数据成本；不默认永久保存全部卡片或资源。

## 暂不优先的方向

- 之前已拒绝的 Masonry 栈缓冲／SmallVec：见[四处差距的实验记录](2026-11-04-virtual-list-four-gaps.md)。单项快速路径已采用，多项临时 Vec 仍在，但旧方案没有稳定净收益，不能只因能减少分配就重新推荐为已成立的优化。
- Measured 长距离 smooth：沿途尺寸仍不确定，扩大挂载区可能抵消虚拟化收益；继续安全即时定位，不把动画覆盖率当性能目标。
- 自动轨道、跨格、任意二维画布／三维：属于新的布局合同，优先级低于已观测到的重复内容准备与缓冲成本。
- 冻结表头／首列：若实际表格消费者需要，这是合理的下一项布局增强；须共享 scroll owner，明确交叉冻结区、裁剪、命中、焦点与语义，不用四个彼此校正 offset 的独立网格拼接。它不会自动提高滚动速度。

## 本轮证据、复现与 Git

探针源码见 [virtual-list-next-review-2026-11-05.rs](virtual-list-next-review-2026-11-05.rs)，原始计数摘录和来源／fixture 身份见[机器记录](virtual-list-next-review-2026-11-05.json)。两个观察测试在固定依赖后运行两次，均为 2 passed／0 failed；不是前后优化测试，没有生产行为已被修复的含义。本轮未重跑上轮的 850 项 workspace 测试，也不把其数字算作新验证。

最初观察把一次 draw 当成一次单元格遍历，预期 250、实际 500；随后改为记录各 draw 周期的实际次数，避免混淆内部 pass 与显示帧。首次临时 fixture 未复制已有锁，解析到另一组依赖；正式记录改用上一轮 after fixture 的锁并只替换测试包名，后续全部 `--locked`，两次计数一致。另核对上一轮 before／after 的依赖包记录相同，没有据此发现上一轮 A/B 依赖不一致。

复现可在仓库外创建独立 Cargo 包，test 指向上述探针，依赖 `gpui-kit = { version = "0.8.0", default-features = false }` 与开启 `test-support` 的 `gpui-base`，三个 patch 路径与 `tools/compare-virtualization.py` 保持一致；从根 Cargo.lock 建立 fixture 锁，记录其身份后用 `--locked` 重跑。此次实际命令：

```powershell
cargo test --manifest-path C:/Temp/yue-virtual-next-review-20261105/Cargo.toml --release --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --test perf review_ -- --nocapture --test-threads=1
```

本轮只提交调研与观察文件，生产源码、生产 manifests／locks、参考项目均未修改；撤销本轮调研提交不会撤销已采用的 smooth。下一轮每个候选独立建立失败回归／固定 workload、实现提交和 ABBA 证据，未满足净收益或行为门槛则 revert 该候选，保留原始样本。
