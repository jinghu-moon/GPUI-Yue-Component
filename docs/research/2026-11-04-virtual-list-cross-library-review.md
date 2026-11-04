# 虚拟滚动第二轮跨库对标

后续实施：第 5.1–5.4 的正确性与更新路径、紧凑几何及验证机制已改造，当前合同、A/B 样本与测试见 [第二轮实施记录](2026-11-04-virtual-list-refinements.md)。下文保留本次调研的修复前状态；Masonry 生命周期扩展、方向性缓冲默认与数据适配仍须按消费需求和测量决定。

日期：2026-11-04。对象为完成上一轮 1–8 项升级后的本项目，以及本地 TanStack Virtual、virtua、vlist 源码。本轮继续调研并补可复现探针，未实施下文建议的生产改造。

**结论：下一步首先修正内容失效与定位请求的正确性，再优化结构更新和昂贵内容准备；不应先扩充更多布局或配置项。** virtua 的分工和批量状态更新、vlist 的内容／位置分离与生命周期验证，均值得在已有 GPUI 原生路径上吸收。

## 1. 依据与测量边界

| 对象 | 本地版本 | 本轮关注 |
| --- | --- | --- |
| TanStack Virtual | `@tanstack/virtual-core 3.19.10` | lazy materialization、resize 批次、滚动补偿与混合动作 trace |
| virtua | `0.54.1` | layout/store/driver 分工、分类通知、方向性缓冲、估算与批量锚点 |
| vlist | `3.3.4` | 两阶段呈现、分块尺寸、autosize、稀疏数据和 heap gate |
| 本项目 | `HEAD 5f552affa797e022f4ddaaec3a49cc26d9c60636` 加当前工作区升级 | 已补丁的 Base/GPUI；不是仅检查 HEAD 文件 |

版本来自本地 package.json，不代表上游最新版本。此前的 [源码调研](2026-11-03-virtual-list-source-review.md) 与 [升级记录](2026-11-03-virtual-list-upgrades.md) 保留历史语境，本文记录升级后重新检查的结果。

源码阅读不等于跨库跑分：本轮未运行三个 JavaScript 项目的测试或 benchmark，不比较它们与 GPUI 的绝对 FPS、内存或 CPU。运行的是本项目独立 Release/headless 观察探针，`rustc 1.98.1`，串行执行；没有启动或操作真实 Showcase。实窗体验由用户检查。

## 2. virtua：值得借鉴的分工与反馈控制

### 2.1 布局、状态、平台驱动各自拥有事实

入口：[Layout 合同](../../refer/virtual/virtua-main/src/core/layouts/types.ts)、[list layout](../../refer/virtual/virtua-main/src/core/layouts/list.ts)、[store](../../refer/virtual/virtua-main/src/core/store.ts)、[driver](../../refer/virtual/virtua-main/src/core/driver.ts)。

- layout 管尺寸、偏移和范围；store 管滚动活动、锚点、跳转及订阅；driver 接实际容器滚动和测量。
- store 的 `UPDATE_VIRTUAL_STATE`、`UPDATE_SIZE_EVENT`、`UPDATE_SCROLL_EVENT`、`UPDATE_SCROLL_END_EVENT` 用 bitmask 表达变化类别，订阅者只接收关心的类别。
- `ACTION_ITEM_RESIZE` 对一批尺寸读取一次锚点旧 offset，应用整批，再计算一次补偿。
- 二维驱动把两根轴的目标合到一次原生 `scrollTo`；不要让两根控制器轮流覆盖同一个宿主 offset。

**本项目可吸收：**一维 state 已有事务式尺寸批量更新，Grid 已合并双轴请求，这些保留。后续把内容身份、几何、工作集、滚动活动的变化显式区分，避免只因 offset 变化便触发业务准备。Masonry 的公共滚动生命周期应与一维 state 对齐，复用已经证实共同的合同；不为此引入 Redux 式 action runtime 或万能 layout 插件总线。

### 2.2 方向性缓冲与首屏估算阶段

`store.ts::$getRange` 在向下滚动时主要向后扩展、向上时主要向前扩展，idle 时两侧扩展；初始自动估算尚在进行时暂不加缓冲。其目的在于控制本帧额外工作，而不是保证所有速度下都没有空洞。

本项目已有方向状态，但默认 overscan 仍是对称的。可增加一种**有界的方向性策略**，保留少量反向余量，换向及时更新；retained/focus 项继续独立计入预算。先比较相同卡片、相同拖动轨迹下的构建量、空白和 CPU 提交耗时，再决定默认值。不要在没有证据时加入速度预测、多个调度器或宣称能降低实窗 CPU。

### 2.3 自动估算有价值，也有冷成本

`layouts/list.ts::$setItemSizes` 在已测总尺寸覆盖视口后，对有效 measured sizes 求中位数，更新默认估计，然后停止自动估计。中位数能降低少量大卡片对初始估计的影响。

代价也必须保留：源码 `sizes.filter` 扫描数组，随后排序；估计更新使 offset 缓存失效。`computedOffsetIndex` 延迟计算有效前缀，但 `$getTotalSize = getOffset(length)` 仍可能计算整个未缓存后缀。不能把它总结成“只算可见项”或“始终 O(log N)”。

**本项目可吸收：**未来可选、样本数有界、一次性冻结的估计校准；调用方提供可靠 per-item hints 时不启用。估算变化也是几何事务，须保锚点。不全量测文本，不在每次反馈后追踪均值而让滚动条反复变化。优先级低于已确认的正确性和追加性能问题。

## 3. vlist：值得借鉴的内容更新与资源边界

### 3.1 内容准备和几何更新分离

[pipeline.ts](../../refer/virtual/vlist-main/src/core/pipeline.ts) 的呈现阶段分别保存 `_lastItem`、`_lastOffset`、`_lastSize`：内容引用改变才替换内容；位置和尺寸改变各自写对应属性。先 acquire 新节点，再 release 离开窗口的节点，避免更新中出现短暂空缺。

**迁移到 GPUI 的含义：**复用工作集内昂贵内容准备，让正常 Render/layout/paint 继续运行；不是复用 DOM，也不是跨帧缓存 AnyElement。当前 `new_with_item_views` 已经走这条路，但 stamp 仍含 start/extent/measured，几何改变会触发内容 factory，见第 5 节。

不能直接删掉这些字段：当前 factory 收到完整 `VirtualizerItem`，可能依赖位置／尺寸。应先明确“不可变内容准备”和“本帧几何输入”的合同，允许依赖几何的渲染正常更新，再用测试证明未依赖几何的昂贵准备不会被重做。

### 3.2 紧凑分块尺寸索引

[sizes.ts](../../refer/virtual/vlist-main/src/rendering/sizes.ts) 的 variable cache 使用 512 项一块的 `inner`、`blockTotals`、`blockOffsets`：offset 读取 O(1)，范围查询两级二分。更新只重读触及块的 sizeFn，随后重算受影响的 block offsets。

这仍有后缀成本：k 个触及块的成本约 `O(k × 512 + 后续块数)`，不是与 N 无关的 O(1) 更新。它是可变数组，也不提供本项目当前的冻结 snapshot 合同。

**本项目可吸收：**研究“紧凑叶块＋持久上层索引”、批量构造和顺序工作集迭代，用数据决定块大小。当前 Known 百万项几何 payload 约 31.3 MB、逐项查询为 O(log N)，有压缩与局部性优化空间；但不能退回可变前缀数组而破坏旧快照。先对比内存、单项更新、批量更新、顺序读取及存活旧快照的成本，再选结构，不能只比较查一次 offset 的速度。

### 3.3 测量观察也有生命周期

[autosize/plugin.ts](../../refer/virtual/vlist-main/src/plugins/autosize/plugin.ts) 在 observer 一批反馈中累计 changedLow/high，刷新一次尺寸并补偿一次；用 stable id、内容引用和节点 `data-index` 校验归属，避免回收后的反馈写到另一项。测量引用随已离开的业务内容清理。

它测一次后会 unobserve；图片 load/error 或显式 `remeasure` 重新触发。因此它并非自动覆盖任意内容变化，不能照搬后宣称无需失效协议。

**本项目可吸收：**保留当前同步原生测量的快照版本校验，并扩大迟到资源反馈、复用条目和卸载清理测试；对稳定内容避免无意义重测。动态换行／展开内容仍由原生测量与显式失效负责，不引入 ResizeObserver 仿真层。

### 3.4 虚拟渲染与虚拟数据加载分层

正确入口是 [plugins/data/sparse.ts](../../refer/virtual/vlist-main/src/plugins/data/sparse.ts) 与 [plugins/data/manager.ts](../../refer/virtual/vlist-main/src/plugins/data/manager.ts)。前者按 chunk 稀疏保存并淘汰缓存；后者按范围寻找缺失块，用 `activeLoads` 去重请求、`AbortController` 取消远端请求，并支持并发上限。

**本项目可吸收：**由消费者或单独数据适配层订阅工作集，执行预取、去重、取消、背压和缓存淘汰。列表几何只需要逻辑长度、稳定 key、估算和内容版本，不负责网络／数据库 I/O。数据条数上限不等于图片资源字节上限，解码图像与 GPU 纹理另行预算。

### 3.5 证明检测器能发现泄漏

[scripts/heap.ts](../../refer/virtual/vlist-main/scripts/heap.ts) 预热后执行 200 次 create/destroy，排空 frames/microtasks 并采样；结合增长率与绝对增长量，区分暖机成本和持续保留。`--self-test` 故意持有实例，要求检测失败，验证 gate 真的有效。

**本项目可吸收：**反复创建、滚动、切源、卸载，排空 headless 任务，检查 WeakEntity、view/cache handle、计时任务与业务资源引用；在独立探针中故意保留一个 owner，确认检查会失败。Rust 不依赖浏览器 GC，不能复制其 JS 字节阈值。现有单次 WeakEntity 释放测试有价值，但不能替代多周期保留趋势测试。

## 4. TanStack：进一步可学的验证方法

入口：[virtual-core/index.ts](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts)、[lazy-measurements.ts](../../refer/virtual/virtual-main/packages/virtual-core/src/lazy-measurements.ts)、[scroll-trace.test.ts](../../refer/virtual/virtual-main/packages/virtual-core/tests/scroll-trace.test.ts)。

1. **分清数值几何与对象物化。** 单 lane 的 Float64Array 配合 lazy VirtualItem，减少未访问对象；但 `getMeasurements` 仍循环计算失效后缀的 key/estimate。值得学顺序批量读取和工作集对象物化，不是移植 Proxy，或声称不需要 O(N) 元数据。
2. **测量补偿按场景验证。** `resizeItem` 区分初次估算校正、已测项重测、跨越视口上缘的流式增长和反向滚动；`_resizeBatch` 合并通知，补偿后的判断使用更新几何。本项目已有绝对锚点，不能简单替换默认策略，应加这些差分用例。
3. **混合动作 trace。** 固定 seed 组合滚动、尺寸反馈、插入、换序和命令，记录/hash 状态轨迹。可借其可复现性，同时以朴素几何 oracle 验证范围、锚点和终态，不能只更新 golden hash 就把回归变成通过。

测量缓存导出仍可能遍历 measurements；源码中的惰性不意味着导出成本只与已测条目数相关。iOS 惯性滚动补丁和浏览器同步更新调度没有直接移植到 Windows/GPUI 的依据。virtua driver 也明确警告长距离 smooth 扩大／冻结沿途范围的性能风险：不要为平滑定位把起点到终点全变成工作集。

## 5. 升级后新确认的问题

本节区分合同缺陷、现有合同带来的成本和未来能力扩展。探针源码见 [独立观察测试](virtual-list-review-2026-11-04.rs)，不是以错误行为为期望的生产回归测试。

### 5.1 内容失效未覆盖 Fixed/Known：正确性缺陷

[state.rs::invalidate_items](../../vendor/gpui-base/src/virtualization/state.rs) 目前只在存在 measured ListState 时改变状态。Fixed/Known 调用成功但没有改变准备缓存身份。

探针：1000 项、20px，factory 创建持有内容版本的 Row，先显示版本 0，再把宿主版本改为 1，调用 `invalidate_items(0..1)` 并 notify。两种模式重新 draw 后均为 `prepared=0, factories=0`，仍显示旧准备结果。设计文档允许这条失效路径，与实现不一致。

修复方向：内容版本与测量版本分离；明确失效目标如何触发工作集缓存的内容重建，所有尺寸模式一致。范围内失效不得为无关行重建，Measured 也须测试同尺寸内容变化。以 source revision 或外部 content_revision 更新是当前可用的显式替代路径，不是该缺陷已修复。

### 5.2 位置变化导致昂贵准备重做：缓存合同待改进

[element.rs::ViewStamp](../../vendor/gpui-base/src/virtualization/element.rs) 包含 index/start/extent/measured。探针滚至 Known 第 50 项，再将第 0 项从 20px 改到 40px；锚点使 visible range 仍为 `50..55`，但内容未变的 5 个条目全部重新调用 factory。

这符合当前 factory 能读取几何的合同，不能称为简单实现错误。可借 vlist 的独立身份解决，但需要同步调整准备输入／依赖合同，证明内容更新、布局相关更新和普通 Render 均正确，不能为了 factory=0 隐藏陈旧几何。

### 5.3 scroll padding 改变后旧请求仍有效：版本校验缺口

`state.rs::set_layout` 的 geometry_changed 不包含 scroll_padding；后者改变定位目标，却没有使旧请求失效。

探针：20px 的第 10 项 Start 定位生成 offset 200；设置 scroll padding start=15/end=10，新请求目标为 185，但 `validate_request(old_request, viewport)` 仍返回成功。旧目标如果被延迟应用，会采用过期的对齐策略。

修复方向：令定位策略变化使旧定位请求过期；不应因此重新测量全量内容。明确区分内容、几何、定位策略、工作集预算的版本影响，并覆盖同一宿主两轴配对请求与用户输入取消。

### 5.4 单项追加仍可能支付全量工作

**一维逐项估算：**10000 项末尾 splice 一项，使用 `measured_with_estimates`，`replace_source` 实际调用估算 callback **10001 次**。这是完整评估新 hints 以保持原子校验的现有行为；新 callback 可能改变旧项估算，不能在没有合同保证时擅自只算新增项。可定义可信的未变区间／局部 hint 更新路径，让调用方表达“只追加”；泛化 Replace 仍需完整校验。

**动态 Masonry：**`replace_source` 先调用 `Self::new` 打包，迁移旧测量后再打包，并对每个未测项单独更新持久树的 measured 标记。即使没有实际测量、只是末尾多一项，也支付重复构造和逐项树更新。source 查找复杂度还取决于调用方的 `index_of` 实现。

三列、统一 40px 估算、全部未测、源 key/index 查找 O(1)，每规模 3 个样本，中位数如下；原始样本见 [CSV](virtual-list-masonry-cold-review-2026-11-04.csv)。

| 原项数 | 初次构造 | replace_source 追加一项 |
| --- | --- | --- |
| 1,000 | 0.0302 ms | 1.5941 ms |
| 10,000 | 0.2422 ms | 25.0499 ms |
| 100,000 | 3.4054 ms | 341.0327 ms |

这是 CPU 几何事务微基准，不是 draw/FPS 或任务管理器 CPU。样本没有专门预热，三样本也不构成统计上限；构造较快不能证明挂载和滚动同样快。即便排除首样本，追加路径仍明显更贵。

修复方向：先去掉二次打包与逐项反复建树，在一次构造时写好 extent/measured；再为声明未变前缀的 append/splice 建增量路径。顺序遍历树而非对 N 项分别查找。必须保留原子失败、旧快照、stable lane、测量迁移和锚点合同，不能只为基准特判未测样本。

### 5.5 Masonry 公共生命周期仍弱于一维 state

当前 Masonry 有范围、动态测量、焦点和 key 定位，但使用独立的一次性 pending 目标；没有与一维 state 同等的命令终态、scroll idle/direction、follow-tail 和纯值 cache 合同。这是适配覆盖范围差距，不是上一轮一维功能失效。

可参考 virtua 共享状态生命周期的分工，但只提取有共同事实与测试的部分。布局分配和稳定分栏仍留在 Masonry，不为了 API 对称而增加无用能力。

## 6. 建议的实施顺序与验收

| 顺序 | 改造 | 应先补的失败／基线证据 | 改后验收 |
| --- | --- | --- | --- |
| P0 | 各模式内容失效；对齐策略请求版本 | 第 5.1、5.3 探针转为正确行为断言，观察旧实现失败 | Fixed/Known/Measured 内容都更新；旧目标拒绝，新目标正确；不全量重测 |
| P1 | 内容准备与几何依赖分离 | 第 5.2，外加 reorder、宽度变化、同高度内容变化 | 纯坐标变化不重做内容；依赖布局的内容不陈旧；工作集退出释放 |
| P1 | 结构更新与 Masonry 批量构造 | 追加/删除/换序的时间、分配、估算调用数；旧快照存活 | 原子变更、保锚点；避免双重打包、逐项重建；输出 A/B 原始样本 |
| P1 | 混合轨迹和多周期释放 gate | 固定 seed；故意保留实例的检测器自测 | oracle 对照通过；泄漏自测必失败、正常路径稳定；覆盖迟到资源 |
| P2 | 紧凑几何与工作集顺序迭代 | 当前约 31.3 MB Known payload 及查询/更新基线 | 减少内存／查询成本且保留持久快照；不引入第二套几何 owner |
| P2 | 按需方向性缓冲和分类通知 | 同轨迹 factory、测量、通知、空白和 CPU 提交量 | 未降低正确性；工作集仍有界；未证明收益则不改默认 |
| P2 | 对齐 Masonry 生命周期 | 明确每项公共合同的消费需求 | 命令取消、idle、资源卸载及快照恢复分别有测试 |
| 消费需求驱动 | 稀疏数据适配、可选估计校准 | 远端迟到／取消和缓存字节预算；估计误差 | 不把 I/O 放入几何核心；无无界样本、请求或资源保留 |

每次行为改动先留升级前证据，再完成升级后专项及工作区回归。不用“新增配置数”衡量基础层完成度。不能搬运的机制包括 DOM 节点池、JS 响应式运行时、浏览器测量调度、无必要的平台 hack，以及为长距离 smooth 保留全部沿途条目。

## 7. 探针复现

探针共 3 个 test，打印观察结果；`3 passed` 只表示成功执行，不表示第 5 节的问题已经修复。生产源文件本轮未作更改。以下从仓库根目录执行，生成物只写 target；使用根锁文件和已有离线依赖：

```powershell
$fixture = 'target/virtual-review-20261104'
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
@'
[package]
name = "yue-virtual-review-20261104"
version = "0.1.0"
edition = "2024"
[workspace]
[[test]]
name = "review"
path = "../../docs/research/virtual-list-review-2026-11-04.rs"
[dependencies]
yue-component = { path = "../../crates/yue-component", default-features = false, features = ["headless-tests"] }
gpui-kit = { version = "0.8.0", default-features = false }
[patch.crates-io]
gpui-base = { path = "../../vendor/gpui-base" }
gpui-pre = { path = "../../vendor/gpui-pre" }
gpui-pre-windows = { path = "../../vendor/gpui-pre-windows" }
'@ | Set-Content -LiteralPath "$fixture/Cargo.toml" -Encoding utf8
Copy-Item -LiteralPath Cargo.lock -Destination "$fixture/Cargo.lock"
cargo test --manifest-path "$fixture/Cargo.toml" --target-dir target --release --offline --test review -- --nocapture --test-threads=1
```

本轮阅读文件标识（SHA-256），便于区分后续生产改造：

```text
virtua src/core/store.ts
8256F04615BE57B5BCB1B3473C39037B5E9536EEF3637F8A637AD9B89E4C912C
vlist src/core/pipeline.ts
5A704DEC25B3908EB5273CB310B5DE8F006CF69CBCCCB0889FC230BCA6977E51
vlist src/rendering/sizes.ts
56E2CB1066A5EA49F36E6517E4FBDC1EA19DFFFD0A9487167BCE986241E1F498
Base virtualization/state.rs
4D5D559DF677D3DCCB38EF3B05F1679588D5DE7D7C9B7B966B51C179DBE6AD99
Base virtualization/element.rs
AF03EB8A73240E2B2F1C3A11232B7473643B3D46087DA5E3E5BA50898DDED13A
Base virtualization/masonry.rs
1F3A5C5E66F01A5D25C66A7C53510B1D00E18D3C8E9F2DCC2164D1931C94A1A1
```
