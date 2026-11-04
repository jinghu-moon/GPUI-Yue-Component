# 虚拟滚动第三轮对标：组合更新、测量身份与扩展布局

日期：2026-11-04。延续 [第二轮实施](2026-11-04-virtual-list-refinements.md)，检查当前工作区的生产接缝。本轮是源码调研和独立观察，不是六项问题的修复交付。

**后续状态：下述六项现已逐项修复，见[第三轮实施与回归](2026-11-04-virtual-list-followup-fixes.md)。本文及 28 条观察保留修复前事实；表中“当前”、后续计划和测试数字均指调研时点。历史探针故意使用旧行为的读取方式，修复后的缓存拒绝可能使它报错；验证现实现应运行新的生产回归，不通过改写旧原始输出掩盖变化。**

**结论：应先修补组合操作和扩展布局的正确性，再补 Grid 热路径与按需能力。现有测试通过，仍不能证明失效、结构更新、测量反馈与动效任意组合都正确。**

## 1. 对标范围与证据

| 对象 | 本地版本／状态 | 新一轮重点 |
| --- | --- | --- |
| TanStack Virtual | virtual-core 3.19.10 | 命令实际到达后的收尾、测量与程序滚动的交错、稳定帧测试 |
| virtua | 0.54.1 | Grid 的布局计划、行列身份、冻结区域、auto 轨道、稀疏保留的真实成本 |
| vlist | 3.3.4 | 真实窗口需求与容量的关系、内容和测量身份、Masonry 呈现、局部坐标与销毁 |
| 本项目 | HEAD `42ab5270f0c2219386e4d44f6129ff9901c78495` 加既有未提交升级 | 一维缓存、Masonry、Grid、公共 cache 和命令接缝 |

版本读取自本地 package.json，不表示上游最新版本。本轮没有运行三个 JavaScript 项目的测试或跨语言跑分；所有下列数字来自本项目的独立 Release/headless 观察。

观察源码：[follow-up 探针](virtual-list-followup-review-2026-11-04.rs)。原始输出：[28 条观察记录](virtual-list-followup-observations-2026-11-04.txt)。探针只调用生产公共入口；没有复制一个虚拟化引擎，也没有修改生产实现。观察测试的 `6 passed` 表示探针运行完成，**不表示下面六项正确性／效率问题通过验收**。

生产专项另行运行：`virtual_list` **60 passed / 7 ignored / 0 failed**。七项 ignored 为已有手动性能探针，本轮不重新跑与本次发现无关的基准。没有启动、关闭或截图真实 Showcase，真实 UI 检查仍由用户负责。

## 2. 六项已复现的问题

### 2.1 P0：先失效内容、再追加，准备缓存可能继续显示旧内容

**复现：**先构建第 0 项的内容版本 0；外部预处理输入变成版本 1，在同一次 state 更新中调用 `invalidate_items(0..1)`，随后以 Splice 追加第 100 项，最后 notify 一次。source 原有项的 key、revision 和 hint 不变；外部 factory 依赖通过文档允许的显式内容失效更新。

| 操作顺序 | Fixed | Known | Measured |
| --- | --- | --- | --- |
| 仅内容失效 | 内容版本 1，factory=1 | 内容版本 1，factory=1 | 内容版本 1，factory=1 |
| 追加 → 内容失效 | 内容版本 1，factory=1 | 内容版本 1，factory=1 | 内容版本 1，factory=1 |
| 内容失效 → 追加 | **内容版本 0，factory=0** | **内容版本 0，factory=0** | **内容版本 0，factory=0** |

根因位于 [state.rs::replace_source](../../vendor/gpui-base/src/virtualization/state.rs) 与 [element.rs 的 ViewStamp](../../vendor/gpui-base/src/virtualization/element.rs)：事务清理旧 index 域的 `content_revisions` 后，局部 epoch 回到 0，而已有 view 的 stamp 同样是 0。全局 `content_revision` 虽能拒绝旧 snapshot，却没有进入这个准备身份。source 的内容 revision 和 index 都不变时，两个不同内容状态发生缓存身份碰撞。

这不是要求调用方永远改变 source revision；`new_with_item_views` 已明确允许外部依赖用 `invalidate_items` 表达，不能在后续结构事务中丢掉已受理的失效。

**参考收获：**[vlist pipeline](../../refer/virtual/vlist-main/src/core/pipeline.ts) 将内容身份与位置分别判断；其内容引用机制不能照搬为 Rc 地址判断，但必须保留“位置事务不能撤销内容已改变这一事实”。TanStack 的混合轨迹测试也说明，应测试同一提交内的操作组合，而不只测试每个单独操作。

**最小修复方向：**清理坐标域时保留单调、不碰撞的准备失效身份。可先使用一个 reset epoch 作为缓存屏障；若清理的是非空内容失效表，让旧 stamp 不可能重新匹配。进一步按 key／Splice 坐标迁移仅在有测量收益时采用，不为局部失效建立全量 key→version 表。

**验收：**覆盖上述三顺序、三模式、删除／换序／替换与无关行复用；检查 prepared 内容值，而不只断言 global revision 增加。新结构事务之后，旧 snapshot 的迟到写入也必须继续拒绝。

### 2.2 P0：Masonry 的测量预算同时截断了可见卡片呈现

**复现：**300×100px 视口，3 列、卡片高 20px、零 gap／overscan，实际工作集 15 项。设置合法的 `measurement_budget(2)`，每帧强制 refresh，连续观察三帧。

| 尺寸模式 | 预算 | 工作集 | 每帧实际注册的卡片节点 | 三帧构建索引 |
| --- | --- | --- | --- | --- |
| Fixed | 2 | 15 | **2** | 每帧均为 `[0, 1]` |
| Measured | 2 | 15 | **2** | 每帧均为 `[0, 1]` |
| Fixed | 默认 1024 | 15 | 15 | 每帧均为 0..14 |
| Measured | 默认 1024 | 15 | 15 | 每帧均为 0..14 |

根因在 [masonry_element.rs::prepaint](../../vendor/gpui-base/src/virtualization/masonry_element.rs)：`built.len() >= budget` 的分支在动态／非动态模式共同的构建循环中。之后只 prepaint 已在 `built` 中的卡片；每帧重新创建这个 map，没有保留推进位置。预算耗尽不只是“剩余卡片使用估算高度”，而是剩余卡片根本没有呈现。增加相同 redraw 也不会补齐。

此前设计有意禁止预算耗尽后无进展地反复调度；该约束仍成立。但“测量预算有界”不能推导为“Fixed 也可以静默丢弃可见卡片”。上表预算 2 是用于揭示问题的配置，不是声称默认视口已普遍缺项；默认预算下工作集超过上限时仍需单独覆盖。

**参考收获：**[vlist pipeline 的容量处理](../../refer/virtual/vlist-main/src/core/pipeline.ts) 明确以实际窗口需求增长 capacity，避免错误估算把合法窗口截断。它也另设病态窗口上限，并发去重诊断；这两个目标需要分别处理。

**最小修复方向：**让已知几何的可见呈现与动态测量／额外校正预算分开；Fixed 不应消耗动态测量额度。动态场景先满足必需可见呈现，再限制额外测量校正工作。若必须设置呈现硬上限，应单独定义可观察的容量不足／降级合同，不能把缺项描述为完整呈现。只有保存进度且能证明推进时才考虑跨帧补齐；不靠不停 notify 重做同一前缀。

**验收：**合法工作集大于预算、密集有效小尺寸、估算偏大后实际变小、overscan、远端 retained/focus。检查真实节点覆盖与连续帧进展，不能只断言 renderer 调用数小于预算。无效零高项的止损行为另测，不取消安全上限。

### 2.3 P1：Masonry 失效未测项时，旧 snapshot 仍可回写测量

**复现：**捕获动态 Masonry 的 snapshot，随后失效第 0 项，再用旧 snapshot 提交该项 99px 的尺寸。

| 失效前状态 | 旧／新 revision | 旧测量是否受理 |
| --- | --- | --- |
| 仅有 20px 估算 | **1 → 1** | **受理** |
| 已确认真实 20px | 2 → 3 | 拒绝 |

根因：[masonry.rs::invalidate_items](../../vendor/gpui-base/src/virtualization/masonry.rs) 只在 `measured == true` 时设置 changed 并增加 revision。内容已经失效、尺寸还没测过的情况因此没有身份变化，`update_sizes_from_snapshot` 的 owner/revision 检查无法识别旧反馈。

这与第二轮一维 state 已修复的“内容版本独立于几何是否改变”是同一种事实，却没有覆盖到 Masonry。普通同步布局不等于必然触发此问题；明确受影响的是使用公开 snapshot guard 的迟到测量／资源反馈路径。

**参考收获：**[vlist autosize](../../refer/virtual/vlist-main/src/plugins/autosize/plugin.ts) 另外校验数据内容身份、节点当前 index 与测量归属；是否已有测量不能替代内容身份检查。

**最小修复方向：**成功的非空显式失效必须使之前的可写 snapshot 过期，即使旧项尚未测量、尺寸相同或已经被失效过。不必为此改变只读冻结几何，也不必加入每项独立任务。

**验收：**未测、已测、重复失效、同尺寸新内容、空输入、非法 key、外来 owner、卸载后的反馈；失败和空输入不改变版本。

### 2.4 P1：导出缓存可能把旧宽度的测量标成新宽度

**复现：**240px 跨轴下提交第 0 项真实高度 60px；调用 `prepare` 接收下一版 480px 视口，尚未进行新宽度下的测量便导出缓存。

```text
measured_cross=240, exported_cross=480, extent=60, restored=1
```

新缓存被另一个刚创建的 controller 正常接收。当前 [cache.rs::measurement_cache](../../vendor/gpui-base/src/virtualization/cache.rs) 从最新 `self.viewport.cross` 取得 cross，却从原生几何中读取仍有效的旧测量。`prepare` 接收新 viewport 不等于该批尺寸已经在新约束下产生，导出时混合了两套时间上的事实。

边界很具体：这是公共 headless／Attached 接缝在“接收新视口、尚未布局”的阶段导出缓存的缺口；未证明正常 Owned 完成新宽度布局后导出也会发生。不能据此宣称用户每次 resize 都显示错误。

**参考收获：**virtua 的 [Layout](../../refer/virtual/virtua-main/src/core/layouts/types.ts) 与 [list cache](../../refer/virtual/virtua-main/src/core/layouts/list.ts) 让尺寸快照来自 layout owner；其缓存本身是按 index 的尺寸数组，并没有本项目完整的 key/content/cross 校验，不能宣称它已经解决所有身份问题。

**最小修复方向：**记录实际测量所属的跨轴／排版身份，导出从尺寸 owner 取这份事实；遇到混合／不兼容身份时拒绝导出或先降为 hints。不能只更换缓存上的 cross 标签。已存在的 layout revision、gap、内容版本、非法数据原子校验仍须保留。

**验收：**240→480 后、首次新布局前后分别导出；横／纵轴；直接 size update 与缓存导入；跨轴尚未解析、零视口与字体 revision 改变。不兼容结果不能以 exact measured 恢复。

### 2.5 P1：无需位移的 Smooth 命令以 NoProgress 结束

**复现：**Fixed 第 0 项已经 Start 对齐，目标和实际 offset 都为 0；再次请求第 0 项 Start，Smooth duration=200ms，以 10ms 的确定时钟步长 prepare/confirm。

| 命令 | confirm 次数 | 终态 |
| --- | --- | --- |
| Instant | 1 | Settled |
| Smooth | **9** | **NoProgress** |

根因在 [state.rs::prepare / confirm](../../vendor/gpui-base/src/virtualization/state.rs)：Smooth 的 settled 绑定 `t >= 1`，但实际早已到达目标。confirm 连续观察到相同 offset、request 又未 settled，于是累加 stalled 并在第 9 次确认时报告 NoProgress。

**参考收获：**[TanStack reconcileScroll](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 以“目标未变且实际位置已到达”累计稳定帧；运动计时不是成功的唯一判据。[scroll-commands 测试](../../refer/virtual/virtual-main/packages/virtual-core/tests/scroll-commands.test.ts) 覆盖定位到已经到达的 index，以及等待测量后的命令退休。其浏览器 ResizeObserver 所需 2／3 帧不能机械变成 GPUI 固定等待。

**最小修复方向：**对于当前有效几何已确定、位移在容差内的目标直接完成；动态目标仍须先满足测量有效性。真正未到达、被宿主夹住或反复拒绝的命令继续走 NoProgress，不能通过删掉止损检查掩盖问题。

**验收：**零位移、小于容差、边界 clamp 后无位移、Nearest 已可见、短距离真实动效、动态未测项与拒绝目标的宿主；终态恰好一次，不额外报用户滚动。

### 2.6 P2：Grid 对列元数据重复支付每行查询成本

**复现：**Fixed 两轴各 100000 项，400×400px 视口，行列尺寸均为 20px，工作集 20×20=400 个 cell。source 计数包含 `key_at` 与 `index_of`，不包含 cell 业务渲染。

| 读取方案 | 行 source 调用 | 列 source 调用 | 产生的 cell／坐标配对 |
| --- | --- | --- | --- |
| 当前 `VirtualGridSnapshot::cells()` | 40 | **800** | 400 |
| 探针对两轴各取一次 `working_items()`，再组合 | 40 | **40** | 400 |

[grid.rs::cells](../../vendor/gpui-base/src/virtualization/grid.rs) 在每一行的 inner loop 中调用 `columns.item(column)`，并克隆 columns 索引 Vec。每个列项重复查询几何、获取 key 和执行反向校验。

**参考收获：**[virtua createGridPlan](../../refer/virtual/virtua-main/src/core/grid.ts) 把轨道集合、测量标记、cell／row 呈现状态显式组织成布局计划，未变行与 cell 可沿用身份。这提示我们应区分轴几何准备、cell 枚举和业务内容准备，不表示整个 Grid 可以做到 O(R+C)。

**最小修复方向：**一帧中行列各顺序物化一次，再组合；优先复用已有 `working_items()`。保留 stable pair key 和 InvalidSource 校验，不以跳过检查降低计数，也不缓存跨帧 AnyElement。

**验收：**轴元数据工作由约 `2R+2RC` 降到 `2R+2C`；cell 构建仍是 O(RC)。上表第二行为探针的等价组合计数，**不是生产实现已优化后的 A/B 时间**。后续还需测小窗口固定开销、Known 两轴、稀疏保留、分配及 cell 内容成本。

## 3. 还值得学习的能力，以及不照搬的部分

### 3.1 Grid 需要独立的消费合同，而非再套两层 List

virtua 的 [VGrid 公共入口](../../refer/virtual/virtua-main/src/react/VGrid.tsx)、[grid layout](../../refer/virtual/virtua-main/src/core/layouts/grid.ts) 与布局计划覆盖 auto 轨道、边缘冻结、分组 sticky、合并单元格、行／列 header 与单元格状态身份。这比本项目目前的 Fixed/Known 两轴组合更完整。

应借鉴的顺序是：先明确哪些尺寸能由可见单元格决定、哪些跨行／列依赖会使未呈现部分失真；再由 Grid/Table 组合层拥有冻结区域、定位避让、导航和语义。一维 `VirtualizerState` 继续只处理主轴事实，不能自动从未知离屏 cell 推断全表列宽。

不是所有参考实现都已消除相同成本：virtua `core/grid.ts` 自己保留 TODO，指出远端 kept 行列交叉枚举会产生二次方工作。本项目当前也是整轴 retained 集合的笛卡尔积；如果未来要同时保留若干远端编辑 cell，应先定义精确的 `(row_key, column_key)` 稀疏保留需求。不能把每个远端行与每个远端列的全部交点当成业务确实要求的工作集。

auto、冻结、合并和稀疏 cell 保留属于有真实 Grid/Table 消费需求后的扩展，不与本轮六项缺陷绑定交付，也不引入万能插件注册表。

### 3.2 内容准备复用应按真实热点扩展到 Grid／Masonry

vlist 的 [Masonry renderer](../../refer/virtual/vlist-main/src/plugins/masonry/renderer.ts) 分别保存内容、选择、焦点、位置和尺寸，并复用可见集合缓冲。本项目一维已有 `new_with_item_views`，Grid/Masonry 仍主要使用普通 renderer，未提供同等的可选昂贵内容准备接缝。

若图片元数据解析、文本预处理或复杂 Row 初始化在实测中重复过多，可以复用身份型 factory 的合同。Grid key 为成对身份，Masonry key 为源项身份；layout 每帧正常运行，布局相关准备依赖仍显式失效。优先共享经过测试的准备缓存机制，而不是复制两份近似实现。

vlist 为 DOM 节点保留一个 render cycle 的释放宽限，缓解边界反复回收；不直接移植到 GPUI。当前离开工作集即释放是明确的资源合同，只有测到边界震荡的真实成本后，才考虑有界 hysteresis，并同时证明资源保留上限。

### 3.3 大坐标要验证输入链，不只验证 f64 查询

vlist 的 [logical scroll 合同](../../refer/virtual/vlist-main/src/core/logical.ts) 和 [runway](../../refer/virtual/vlist-main/src/core/runway.ts) 将 logical position 与局部原点分开。值得学的是“在进入低精度物理坐标前做局部化”，不是照搬浏览器元素高度阈值、无限轮播或 synthetic wheel 接管。

本项目 Known/Fixed 的 f64 查询和局部绘制已有基础，但原生 ScrollHandle/Pixels 与 Measured 累计仍有 f32 边界，详见主文第 13.3 节。下一轮若处理精度，应测试接近大 offset 时的小滚轮增量、行内小数偏移、双轴同时移动和 scrollbar 映射；首尾可达不能替代这些测试。本轮没有新增该输入链的测量，也没有据源码阅读声称它已修复。

### 3.4 测试应同时覆盖单操作、同批组合和不同 owner

TanStack 的 [命令退休 E2E](../../refer/virtual/virtual-main/packages/react-virtual/e2e/app/test/command-retire.spec.ts) 测试“请求定位后，同一个 handler 中上方行缩小”，以及“offset 写回到达时，上方行变大”；[adjustment 测试](../../refer/virtual/virtual-main/packages/virtual-core/tests/adjustment.test.ts) 又区分首次估算校正、已测项重测、跨视口上缘项增长与多帧 reflow。

本项目的 6400 步几何 oracle 很有价值，但没有呈现内容 factory，无法发现第 2.1 节的缓存身份碰撞；预算测试只证明构建数有界，也无法证明可见覆盖。下一步应补两个不同维度：同批组合的最终内容／几何一致性，以及每帧实际呈现集合是否满足声明的合同。

现有 64 周期释放 gate 使用一维 Fixed＋准备 view。它没有自动证明 Grid/Masonry 的 mount、focus handles 与 source 更换同样无保留。可沿用既有 WeakEntity gate 和故意保留 owner 的自测，扩展到两种生产布局，不复制浏览器 heap 阈值。这里是覆盖空缺，**不是本轮已确认内存泄漏**。

## 4. 后续实施顺序与约束

| 顺序 | 工作 | 改前证据 | 改后必须证明 |
| --- | --- | --- | --- |
| 1 | 内容失效跨结构事务保持有效 | 第 2.1 节三模式／三顺序 | 当前内容正确、无关内容合理复用、旧反馈拒绝 |
| 2 | Masonry 呈现覆盖与测量预算分离 | 第 2.2 节缺 13 项且重复同前缀 | Fixed 覆盖完整；动态预算行为明确、有界且不永久饿死后项 |
| 3 | Masonry 内容失效身份 | 第 2.3 节未测项旧写入受理 | 任意成功非空内容失效拒绝旧测量，空／失败无副作用 |
| 4 | 测量缓存的实际约束身份 | 第 2.4 节 240px 测量冒充 480px | 来源／身份一致，重挂恢复不引入伪精确尺寸 |
| 5 | 无需移动的命令正确收尾 | 第 2.5 节 9 次确认后 NoProgress | 已到达则一次成功；未到达仍可止损和取消 |
| 6 | Grid 轴准备合批 | 第 2.6 节列 source 调用 800 次 | 单轴准备一次、cell 结果不变，补时间／分配 A/B |
| 随上述改动 | 同批组合、覆盖 oracle、Grid/Masonry 释放测试 | 当前几何专项全通过但观察仍暴露问题 | 测试能实际发现旧问题，而不只检查 revision 或调用上限 |
| 按消费需求 | Grid auto／冻结／稀疏 cell、布局内容准备、Masonry 生命周期、精度扩展 | 专项负载与消费者合同 | 没有重复几何／滚动 owner，没有无界资源保留 |

每项先把观察转为期望正确行为的失败回归，再修改唯一 owner，补升级前后证据。六项以内的修复不需要为三库 API 做形式上的一一对齐。稀疏 I/O、取消、背压和图像字节预算继续留在消费者／独立适配层；没有新实窗测量时不改变 overscan 默认，也不承诺此前任务管理器 CPU 峰值下降。

## 5. 复现和本轮检查

| 检查 | 本轮结果 |
| --- | --- |
| 独立 Release/headless 观察 | 6 个观察测试完成，输出 28 条原始记录；记录中包含已复现的问题 |
| 既有生产 virtual_list 专项 | 60 passed / 7 ignored / 0 failed |
| 独立 fixture Clippy，`-D warnings` | 通过 |
| 探针 rustfmt、`git diff --check` | 通过 |
| 文档本地链接与原始输出核对 | 三份变更文档本地链接均存在；28 条输出与最终执行日志逐条一致 |

从仓库根目录执行。fixture 独立于 workspace，依赖版本沿用现有根锁文件，生成物只放 target；不会改生产 Cargo.lock。

```powershell
$fixture = 'target/virtual-followup-review-20261104'
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
@'
[package]
name = "yue-virtual-followup-review-20261104"
version = "0.1.0"
edition = "2024"
[workspace]
[[test]]
name = "review"
path = "../../docs/research/virtual-list-followup-review-2026-11-04.rs"
[dependencies]
yue-component = { path = "../../crates/yue-component", default-features = false, features = ["headless-tests"] }
gpui-kit = { version = "0.8.0", default-features = false }
[patch.crates-io]
gpui-base = { path = "../../vendor/gpui-base" }
gpui-pre = { path = "../../vendor/gpui-pre" }
gpui-pre-windows = { path = "../../vendor/gpui-pre-windows" }
'@ | Set-Content -LiteralPath "$fixture/Cargo.toml" -Encoding UTF8
Copy-Item -LiteralPath Cargo.lock -Destination "$fixture/Cargo.lock"

# 首次运行允许独立 fixture 添加自己的 package 记录，后续加 --locked。
cargo test --manifest-path "$fixture/Cargo.toml" --offline --release --target-dir target --test review -- --nocapture --test-threads=1
cargo test --manifest-path "$fixture/Cargo.toml" --offline --release --locked --target-dir target --test review -- --nocapture --test-threads=1
cargo clippy --manifest-path "$fixture/Cargo.toml" --offline --release --locked --target-dir target --all-targets -- -D warnings
cargo test -p yue-component --locked --features headless-tests --test virtual_list -- --test-threads=1
rustfmt --edition 2024 --check docs/research/virtual-list-followup-review-2026-11-04.rs
git diff --check
```

本轮不修改生产 Rust 实现、公共 API、Showcase 或 vendor 补丁。原始观察没有时延跑分，不输出 P95／FPS／CPU 百分比。完整编译和执行日志保存在 `target/virtual-followup-review-20261104/`；持续保存的源码与 28 行原始输出如第 1 节所列。
