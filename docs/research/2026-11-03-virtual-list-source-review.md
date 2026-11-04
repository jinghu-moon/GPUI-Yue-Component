# 虚拟滚动基础层源码调研记录

后续实施：原生接缝、横向动态、增量尺寸与多布局的当前能力见 [1–8 项升级记录](2026-11-03-virtual-list-upgrades.md)。本文保留调研时源码语境，历史限制不代表当前实现。

日期：2026-11-03。用途：为 [VirtualList 设计](../28-virtual-list.md) 及本项目虚拟滚动基础层提供可以回查的架构、算法和测试依据。本次补充重点追踪 TanStack Virtual 的布局组合、复杂条目、几何/内容更新和文本尺寸推导，并核对本地 GPUI 三条路径的轴、外部容器与语义接缝。

本文保存实现前的源码调研快照；GPUI/Base 的后续源码改动、运行测试与性能样本见 [实现与验证记录](2026-11-03-virtual-list-implementation.md)。原 `vendor/gpui-base/src/virtual_list.rs` 已迁至 `virtualization/{geometry,state,element}.rs`，下文对旧实现的事实描述与 hash 均为改造前证据。

本记录区分三类信息：**源码事实**、**根据源码推导的成本**、**本项目的设计决定**。阅读过测试不等于运行过测试；调研阶段没有运行三个前端项目的测试或性能基准，也未在当时测量本项目性能；实施后的测量单独记录。官网的体积、FPS 和内存数字不作为跨运行时比较结果。

## 1. 调研快照与范围

| 对象 | 本地版本 | 本轮重点入口 |
| --- | --- | --- |
| TanStack Virtual | `@tanstack/virtual-core 3.19.10` | `Virtualizer`、measurement、range、scroll command、React adapter、fixed/variable/dynamic/pretext 示例与 benchmark workload |
| virtua | `0.54.1` | list layout、store、container driver、React Virtualizer |
| vlist | `3.3.4` | size cache、two-phase pipeline、pool、scroll、autosize/data/a11y plugins、React adapter |
| GPUI Kit | 锁定 `0.8.0` | Kit 的 GPUI re-export、Base seam；没有启用另一套 Component |
| 本地 Base / GPUI core | `gpui-base 0.8.0` / `gpui-pre 0.3.9`，带仓库补丁 | `virtual_list.rs`、`uniform_list.rs`、`list.rs`、`scrollbar.rs`、`div.rs` |

这是本地导出版本的调研，不将其称为各项目的最新版本。项目基线提交为 `4846e89`；本地参考导出未据此推断其上游 commit。用户提供的 [跨框架虚拟滚动库项目说明文档](../../refer/virtual/跨框架虚拟滚动库项目说明文档.md) 作为导航，结论以以下源码为准。

关键文件 SHA-256，用于识别本次阅读的源码内容：

```text
TanStack packages/virtual-core/src/index.ts
EC73AE69DF7F60BC5A29F7650AA3EE9FA27764D01E76B01EFCA184A6090C5C8D
virtua src/core/store.ts
8256F04615BE57B5BCB1B3473C39037B5E9536EEF3637F8A637AD9B89E4C912C
vlist src/core/create.ts
B5E4FBDB32DB8050147C2591CE26ACCD6144A7818B6A99BC1E474994A5FB592F
Base src/virtual_list.rs
E3D267980D927B26299E564108B078FA49CC8AAA16FFE54062DCBA30215C876B
GPUI src/elements/list.rs
42704328BF8F1F5C75B0129DFAF4052E475F2DE1BCFEC3A2C6F92F518C6CB573
GPUI src/elements/uniform_list.rs
92C0BCE00A70AD078E9E94BB93B3A49270FE2D76816B971295745ED6C1ED66C3
```

## 2. TanStack Virtual：可组合的 headless 控制器

### 2.1 架构与 headless 边界

[virtual-core/src/index.ts](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 中的 `Virtualizer` 持有跨帧状态：尺寸缓存、几何缓存、范围、滚动方向、滚动命令和校正。消费者通过 `count`、`estimateSize`、`getItemKey` 提供模型，通过 `getVirtualItems()` 取得应呈现的条目；并不获得一个默认样式列表。

平台接缝包括 `observeElementRect`、`observeElementOffset`、`measureElement`、`scrollToFn`。core 同时提供 DOM 默认实现，内部也使用 Element、ResizeObserver 等类型。因此它的 headless 主要指**不强制组件外观和呈现树**，不能理解为整个源文件与浏览器完全无关。

`rangeExtractor` 允许更改呈现索引集合，默认实现根据 overscan 扩展范围。固定、可变、横向和多 lane 共享控制器。默认 `getItemKey(index) => index` 便于入门，但对可插入、重排的数据，不能作为本项目的默认身份策略。

### 2.2 尺寸与范围的真实成本

`getMeasurements` 以尺寸缓存版本等依赖进行 memoization；`pendingMin` 记录最早脏索引，重建其后的几何。稳定几何下，单 lane 范围定位是二分起点，再向前走到视口末端；变化后仍可能重建很长的后缀。不能把“二分查找”表述成整个动态更新都是 `O(log N)`。

[lazy-measurements.ts](../../refer/virtual/virtual-main/packages/virtual-core/src/lazy-measurements.ts) 将单 lane 几何存为 `Float64Array` 的 `[start, size]`，另有 key/item 数组；通过 Proxy 延迟生成 `VirtualItem` 对象。其收益是少创建未被读取的对象，**不是少保存所有条目的几何**：单 lane 数值 backing 本身即约 `16 × N` 字节，还不含 key 数组、Map、对象和框架成本。

这启发 Rust 实现使用紧凑几何、共享不可变 backing，只为工作窗口组装呈现数据；没有理由移植 JavaScript Proxy。

### 2.3 测量批次与锚点

ResizeObserver callback 将多个尺寸变化合并，批次结束再通知；如果本批已经改变滚动位置，后续条目的锚点判断会先读取更新后的几何，避免拿旧 start 与新 scroll offset 比较。

默认 `measureElement` 优先使用 observer entry 的 `borderBoxSize`。没有 entry 的同步调用若已有尺寸缓存则先返回缓存，避免重复读取 `offsetWidth/offsetHeight`；无缓存时才走实际读取。`useCachedMeasurements` 是显式选项，启用后使用缓存/估算代替测量，不能自动保证动态内容的最新尺寸。

`Virtualizer.measureElement` 在普通用户滚动时跳过同步 ref 测量，让 ResizeObserver 异步提供尺寸；初始/idle 或程序定位仍可能同步测量。smooth 阶段还约束可测量的目标邻域。不能将其描述为“所有测量一律推迟到下一次 rAF”；`useAnimationFrameWithResizeObserver` 是另一项可选策略，额外调度也可能增加延迟。本项目应按 GPUI 实际阶段合批，不能照搬浏览器时序。

`resizeItem` 区分首次估算纠正和后续重测。后续重测默认只补偿完全位于视口上方的项；跨过视口顶边的行在底部继续增长，不应让阅读位置跟着向下漂移。源码还针对向上滚动、smooth command、end anchoring 分别处理，不能简化为“任何上方 start 都加上 delta”。

`setOptions` 在 end-anchor 场景识别边界 key 变化，用 `(anchorKey, offsetInItem)` 在新序列恢复位置；包含 prepend、append-with-trim 和尾部跟随逻辑。恢复 key 的路径含线性查找，故稳定身份正确性与查找成本是两个问题。

### 2.4 滚动命令是状态机

[scroll-classifier.ts](../../refer/virtual/virtual-main/packages/virtual-core/src/scroll-classifier.ts) 区分重复事件、自身写回、被边界 clamp 的写回和外部滚动。命令含 queued、smooth、smoothLanded、smoothFinishing、instant 等阶段；测量改变目标时重新求目标，稳定后结束，并有超时止损。

源码中与 iOS touch/momentum 相关的 hold/deferral 是浏览器平台问题。Windows GPUI 应复用输入系统、采用自己的命令取消和布局时序，不复制 UA 判断及其常量。可以吸收的基本规则是：**自身校正不是用户输入；用户输入有权中断定位；命令必须最终收敛或结束。**

### 2.5 React adapter 的新优化

[react-virtual/src/index.tsx](../../refer/virtual/virtual-main/packages/react-virtual/src/index.tsx) 的 `directDomUpdates` 默认关闭；启用后，适配器的 onChange 可直接更新位置与容器尺寸，仅在范围／滚动态变化时触发对应的组件重新渲染。last-position/last-size 防止重复写；容器先增大，再同步 scroll offset，避免目标被旧 scrollHeight 截断。

这不阻止业务 props/state 更新，也不消除新条目挂载、内容布局和绘制成本，不是任意复杂子树的自动缓存。跨轴位置仍由消费者负责。这项模式要求主轴位置和尺寸的写入所有权集中，不能让消费方的 transform 与引擎互相覆盖。GPUI 本来有 layout/prepaint/paint 生命周期，借鉴的是**降低语义通知噪声及同帧提交几何**，不是在 GPUI 外另建直接绘制后门。

### 2.6 查阅的测试与基准

- [measurement.test.ts](../../refer/virtual/virtual-main/packages/virtual-core/tests/measurement.test.ts)：同尺寸 no-op、最早脏索引、乱序测量、过期节点、批次只通知一次；特别阅读了 `#1218` 与 `#1036` 的批次断言。
- [anchor-end.test.ts](../../refer/virtual/virtual-main/packages/virtual-core/tests/anchor-end.test.ts)：尾部增长、远离尾部不跟随、被旧容器尺寸截断的写入、真实手势取消待重试校正。
- [scroll-commands.test.ts](../../refer/virtual/virtual-main/packages/virtual-core/tests/scroll-commands.test.ts) 与 [scroll-classifier.ts](../../refer/virtual/virtual-main/packages/virtual-core/src/scroll-classifier.ts)：命令与写回的分工。
- [bench.bench.ts](../../refer/virtual/virtual-main/packages/virtual-core/tests/bench.bench.ts)：区分冷启动、尺寸风暴、每次测量都读布局与批次读取、真实 onChange 成本。其部分注释叙述历史优化，成本判断仍以当前执行代码为准。
- [React adapter tests](../../refer/virtual/virtual-main/packages/react-virtual/tests/index.test.tsx)：direct DOM 模式下子组件自行挂载条目、条目先于容器挂载等生命周期案例；证明几何接管的正确性，不证明复杂卡片帧率。
- [TanstackPage.tsx](../../refer/virtual/virtual-main/benchmarks/src/pages/TanstackPage.tsx) 与 [dataset.ts](../../refer/virtual/virtual-main/benchmarks/src/lib/dataset.ts)：该 benchmark 页面每行主要呈现 `item.text`，包含不同文本长度的负载。它不能覆盖本项目包含输入框、图片、深子树或模糊效果的卡片成本，也不能用其结果代替这些场景的原生基线。

### 2.7 校准概念：Virtualizer、viewport 与布局组合

[introduction.md](../../refer/virtual/virtual-main/docs/introduction.md) 说明 core 不提供默认 markup/style。实际共享抽象是 `Virtualizer`：从滚动容器/窗口获得 viewport 几何与 offset，管理一个主轴上的条目范围、尺寸和位置，可配置多个 lanes。viewport 是可见几何输入，不是仓库中的另一个通用列表/网格 UI 类。

| 已核对的示例 | 源码事实 | 可移植边界 |
| --- | --- | --- |
| [fixed/main.tsx](../../refer/virtual/virtual-main/examples/react/fixed/src/main.tsx) 的 `GridVirtualizerFixed` | 行、列两个 Virtualizer 的 getScrollElement 指向同一个 parentRef；以行工作集 × 列工作集创建单元格 | 双轴可组合；两个控制器不等于两个嵌套同轴滚动所有者 |
| [variable/main.tsx](../../refer/virtual/virtual-main/examples/react/variable/src/main.tsx) 的 `MasonryVerticalVirtualizerVariable` | 一个 Virtualizer 设置 lanes=4，消费者根据 lane 放置横向位置，并沿主轴放置 start | 多列分配与跨轴位置有明确职责；不是自由二维空间索引 |
| [dynamic/main.tsx](../../refer/virtual/virtual-main/examples/react/dynamic/src/main.tsx) 的动态条目 | measureElement 绑定条目外层；内部正文由消费者创建 | 条目根是测量与虚拟化单位，不自动识别内部可虚拟化的后代 |

推导：一维列表、二维网格和瀑布流可以共享部分机制，但范围表示和尺寸依赖并不完全相同。例如动态网格的行高若受未呈现列影响，仍需消费方定义可靠策略，不能仅取当前可见列的最大值。对本项目，明确这些设计边界比先增加 `VirtualViewport` 名称或万能布局插件更有价值。

### 2.8 复杂卡片：限制数量，不自动消除内部工作

`getVirtualItems().map(...)` 只创建工作范围内的条目，卡片内部的多个文本块、按钮、Input 和图片仍按调用方的 renderer 构建。整个卡片进入工作集后，子元素的排版/绘制以及资源准备并不因外层虚拟化而自动消失。即使某些内部内容被裁剪，裁剪也不等于跳过其构建与布局。

因此有两个独立变量：参与工作的条目数量，以及每个条目的实际工作量。单条目包含数千条记录时，核心不会递归拆分它；应由拥有语义的组件评估展平为独立 key 的标题、分组和记录，保留组关系、焦点、编辑值和阅读锚点。确有独立视口才考虑内部虚拟区域，不以嵌套滚动作为默认方案。

对 GPUI 的采用要求：基础层声明条目根粒度、提供稳定身份与局部失效、避免不必要的宿主通知，并允许按阶段诊断复杂条目成本。真实布局仍可能多次调用 renderer，不能承诺同一范围完全不构建，也不能用永久缓存 Element 树冒充内容优化。

### 2.9 文本高度推导：Pretext 示例的收益与代价

[pretext.md](../../refer/virtual/virtual-main/docs/pretext.md) 和 [pretext/main.tsx](../../refer/virtual/virtual-main/examples/react/pretext/src/main.tsx) 使用额外依赖 `@chenglou/pretext`，不是 Virtualizer 内置的文本引擎。其处理顺序为：

1. 按正文及字体/字距等输入缓存 `prepare()` 结果。
2. 用当前可用文本宽度和 line-height 调用 `layout()`，得到换行高度，再加已知的元信息、边框和内边距。
3. 将计算值交给 estimateSize，并让呈现槽位使用 virtualRow.size；该示例没有再给同一行附加 measureElement。
4. 宽度或字体版本变化后重置虚拟化尺寸；字体准备缓存也有相应的失效处理。

适用条件是高度主要由可推导的文本排版决定，且推导输入与实际呈现一致。图片、嵌入内容、任意组件布局不在纯文本推导的保证内；应实际测量或由可靠元数据显式给出尺寸。一行必须有确定的尺寸优先级，不能让估算、measureElement、resizeItem 各自独立争夺最终高度。

收益是减少未知高度引起的测量与滚动校正，不是消除可见文本的实际渲染。还须注意 `getMeasurements` 可能从脏起点遍历到 count 并调用 estimateSize；该示例的 preparedCache 也不是仅保留可见项的有界缓存。不能从“虚拟行只渲染 W 项”推断首次文本预处理只花 W 的成本，或所有缓存都是恒定内存。

对本项目：优先验证 GPUI 自身文本能力能否复用；近似推导只作 Measured 估算，精确且版本一致的已知结果才进入 Known。缓存必须有内容/排版身份、容量和清理规则。首版不增加第四种尺寸模式、不全量预排版百万条，也不因这个例子引入 JavaScript 运行时或新的通用文本排版层。

## 3. virtua：layout / store / driver 分离

### 3.1 架构

[layouts/types.ts](../../refer/virtual/virtua-main/src/core/layouts/types.ts) 描述几何能力；[store.ts](../../refer/virtual/virtua-main/src/core/store.ts) 拥有视口、scroll offset、方向、jump 与 pending jump；[driver.ts](../../refer/virtual/virtua-main/src/core/driver.ts) 观察容器和条目、写滚动位置；[React Virtualizer](../../refer/virtual/virtua-main/src/react/Virtualizer.tsx) 连接框架和呈现树。

core 有 list/grid/masonry layout，包也导出了 `unstable_core`。因此它有可研究、可复用的无样式机制，但不能把 unstable 入口当作无需适配的稳定 headless 公共承诺。

### 3.2 list layout

[layouts/list.ts](../../refer/virtual/virtua-main/src/core/layouts/list.ts) 保存 `sizes[N]` 和 `offsets[N+1]`；未测尺寸用默认值，`computedOffsetIndex` 标记已算前缀。查询更远的 offset 时通过循环补全前缀；尺寸变化将脏点回退到最早变化处。

因此，已缓存 offset 的读取便宜，首次深跳或前部失效后的远端查询可能补算很长的前缀。二分中的 getter 本身有工作量，不能只数二分次数。初始未指定 itemSize 时，测量总量超过视口后，用已测非零尺寸的**中位数**更新默认估算；这是一次估算策略，不是保证每一行准确。

尺寸缓存主要按 index 管理，长度更新的 `shift` 明确表示从头部增删。React child key 可以稳定组件身份，但不意味着内部所有尺寸状态都自动按业务 key 重排。本项目应将身份恢复作为独立合同。

### 3.3 低噪声通知与像素缓冲

store 用 bitmask 区分 virtual state、size、scroll、scroll-end 订阅。无关订阅直接跳过；一个 action 做完一致的状态更新后再通知。

`$getRange(bufferSize = 200)` 使用**像素缓冲**并偏向滚动方向，idle 时两侧都有；估算初期不额外扩大窗口。冻结目标范围、SSR 和校正写回尚未完成时保留前一范围。这不是越快就无限扩大渲染窗口。

### 3.4 测量与滚动校正

driver 将同一 ResizeObserver 批次整理成 resize 列表，区分被重新绑定 index 的元素；跳过隐藏导致的零尺寸。store 先记录 anchor offset，更新尺寸，再计算锚点差值。scroll driver 在边界用绝对写入，普通校正用相对 scrollBy，避免覆盖并发滚动。

手动定位保留一个可重求的目标函数，订阅尺寸变化并在 microtask 合并重写。smooth 的 frozen range 可能覆盖沿途大量项；其 API 注释明确提醒长距离 smooth 会破坏虚拟化收益。这直接支持本项目“远距离立即定位、短距离才可动画”的策略。

### 3.5 不机械移植的行为

React adapter 将未测项隐藏至可定位，并在滚动期间把容器 `pointerEvents` 置为 none。这是它选择的交互与浏览器性能折衷，不适合无条件复制到有按钮、输入、上下文菜单的桌面列表。

driver 的 iOS overflow 临时切换、ResizeObserver delivery workaround 均保留在浏览器平台层；GPUI 的 Base 需要解决真实布局阶段的问题，而不是模拟浏览器 API。

[ListItem.tsx](../../refer/virtual/virtua-main/src/react/ListItem.tsx) 用 React.memo 包装条目并 memoize 样式，设置 layout/style containment；[useChildren.ts](../../refer/virtual/virtua-main/src/react/useChildren.ts) 可缓存静态 children 展平结果，但 render-function 模式仍会调用消费者提供的内容函数。这些措施减少不必要的更新，并不保证任意复杂卡片的后代完全不重建。

### 3.6 查阅的测试

- [layouts/list.spec.ts](../../refer/virtual/virtua-main/src/core/layouts/list.spec.ts)：snapshot 独立性、长度变化、shift、估算与缓存。
- [compensation.browser.spec.tsx](../../refer/virtual/virtua-main/src/compensation.browser.spec.tsx)：快滚进入未测区域、jump 写回、视口上方／顶边／跨顶边／下方尺寸变化。逐段阅读了 idle 条件下这些断言，证明“跨顶边行变化不补偿”的区别。
- [scroll-to.browser.spec.tsx](../../refer/virtual/virtua-main/src/scroll-to.browser.spec.tsx)：定位相关测试入口，作为后续原生场景矩阵的参考。

## 4. vlist：DOM 核心与按功能扩展

### 4.1 它不是纯计算 headless 引擎

[create.ts](../../refer/virtual/vlist-main/src/core/create.ts) 直接建立 DOM、size cache、pool、engine state 和事件系统；[React adapter](../../refer/virtual/vlist-main/src/react.ts) 负责 mount/destroy 与 setItems，DOM 仍由 core 拥有。样式可替换、插件可组合，不等于不拥有渲染树。

[AGENTS.md](../../refer/virtual/vlist-main/AGENTS.md) 和 [types.ts](../../refer/virtual/vlist-main/src/core/types.ts) 中的 PluginContext 按 dom、scroll、items、sizes、render、hooks、nav 等 capability 分组；插件有 priority、conflicts 和单一导航 owner 的约定。这个设计解决了它支持很多布局／交互时的扩展问题，同时产生索引空间转换与插件时序成本。

本项目当前不需要照抄 plugin registry。先用普通 Rust 模块和明确输入输出；真实扩展冲突出现后再引入更复杂机制。

### 4.2 两阶段 pipeline

[pipeline.ts](../../refer/virtual/vlist-main/src/core/pipeline.ts) 将计算结果放入复用的 typed arrays，随后 commit：获取节点、绑定内容与身份、更新位置和尺寸、批量附加，再释放离窗节点。位置、尺寸、选中／焦点状态都有相等判断。

这减少稳定滚动路径的重复工作，但并不意味着完整 commit 零分配：新条目的 template、字符串、DocumentFragment、DOM 和回调仍有成本。连续窗口释放可按范围检查；任意稀疏集合的释放路径用线性 membership 扫描，不能对所有插件组合宣称同一复杂度。

源码还给渲染窗口加保护上限，防止零尺寸／极小尺寸将全量数据拉入工作集。它是报错并止损，不是静默裁掉内容。对本项目的启发是：尺寸前置条件和窗口预算必须可诊断。

### 4.3 fixed 与 variable size cache 不同

[rendering/sizes.ts](../../refer/virtual/vlist-main/src/rendering/sizes.ts) 的 fixed cache 只需 size、count 等少量标量，offset 乘法、index 除法即可。

variable cache 使用每块 512 项的两层前缀：`inner[N]`、`blockTotals[B]`、`blockOffsets[B+1]`。offset 读取 `O(1)`，两级二分定位 `O(log N)`；单个局部变更重读触及块的 sizeFn，并重算后续块 offset。

推导：一个连续变更范围触及 k 个条目、q 个块时，大致是 `O(q × 512 + 后续块数)`，不是对 N 完全无关；其尺寸函数读取次数可与总 N 无关，但后续块累加仍随 `N/512` 增长。内存为 `O(N)`，不是恒定内存。

[sizes.test.ts](../../refer/virtual/vlist-main/test/rendering/sizes.test.ts) 既有 700 项随机编辑对照 full rebuild，也有 10K/100K 下 sizeFn 读取次数相等的测试。后者证明“尺寸函数调用有界”，不证明总更新时间恒定。部分名为 multi-block 的旧测试用 500 项，在当前 512 块大小下并未跨块；本项目测试必须用真实边界数据，不能只沿用名称。

### 4.4 pool、autosize 与数据缓存

[pool.ts](../../refer/virtual/vlist-main/src/core/pool.ts) 释放时清理 class/style/id/role/ARIA/data 属性与内容，空闲时 trim，避免历史最大窗口永远保留。它复用的是 DOM 容器，不是跨业务对象保留所有行的状态。

[pipeline.ts](../../refer/virtual/vlist-main/src/core/pipeline.ts) 在已挂载条目的 `_lastItem` 引用未变化时避免替换内容，`_lastOffset`/`_lastSize` 避免重复几何写入；新获取的容器仍须调用 template 创建内容。结合 pool 释放清理可知，其收益是保留当前不变内容、少做重复工作，不能宣称回收池永久保存了所有昂贵子树，更不能照搬为 GPUI Element 池。

[autosize/plugin.ts](../../refer/virtual/vlist-main/src/plugins/autosize/plugin.ts) 的测量按 item id 缓存并保留内容对象作版本依据；测后 pin 尺寸、unobserve。图像 load/error 或显式 remeasure 才重新测量；因此不能称为所有内容变化都自动持续观察。回收节点测量要核对其当前绑定，数据变化要清理离开集合的测量；这些是可吸收的不变量。

[data/sparse.ts](../../refer/virtual/vlist-main/src/plugins/data/sparse.ts) 的 chunk storage 有 maxCachedItems、evictionBuffer、LRU 信息；[data/manager.ts](../../refer/virtual/vlist-main/src/plugins/data/manager.ts) 管理异步请求、AbortController 和已加载项的 id→index。它能限制**已加载数据工作集**，这与 variable geometry 的 `O(N)` 内存是不同账目。普通静态入口在 create.ts 中仍执行 `[...rawConfig.items]` 复制引用数组。

### 4.5 滚动与大坐标

[scroll.ts](../../refer/virtual/vlist-main/src/core/scroll.ts) 既处理 native scroll，也可拦截 wheel 同步 commit；不能概括为“一律 rAF 节流”。[scroll-source.ts](../../refer/virtual/vlist-main/src/core/scroll-source.ts) 的 smooth 每帧重新求目标并读取浏览器实际应用的值，处理尺寸上限导致的 clamp。

[adapter.ts](../../refer/virtual/vlist-main/src/core/adapter.ts) 把逻辑位置表示为 `(index, offsetPx)`；[runway.ts](../../refer/virtual/vlist-main/src/core/runway.ts) 的小 runway 与 baseOffset 维护局部呈现原点。native/synthetic 切换解决浏览器元素高度上限，carousel 还引入 lap index。这些浏览器上限数字不能变成 GPUI 的阈值。

可吸收的是“模型位置／累计坐标／局部绘制坐标分开”“校正读取实际应用结果”。GPUI 自己已存在 ListOffset，无需再平行发明一套逻辑滚动模型。

### 4.6 性能宣称的证据边界

[README Performance](../../refer/virtual/vlist-main/README.md) 给出约 0.1 MB、2 ms、120 FPS 等数字；[scripts/bench.ts](../../refer/virtual/vlist-main/scripts/bench.ts) 实际委托相邻 `vlist.io` 项目的 benchmark runner，本导出中的 README 表格不足以还原全部测量口径。

[scripts/heap.ts](../../refer/virtual/vlist-main/scripts/heap.ts) 则是另一种可查阅的 gate：预热、重复创建销毁、drain frame、主动 GC、比较增长速率，并有故意保留实例的 self-test。它证明的目标是泄漏检测，不是百万条完整数据仅占 0.1 MB。

本项目借鉴它分账和校验测量工具的办法，不采用其 README 数字作验收标准。

### 4.7 可访问性与坐标空间

[a11y/plugin.ts](../../refer/virtual/vlist-main/src/plugins/a11y/plugin.ts) 在容器 focus、活动项、selection 和 DOM/数据索引间进行显式转换；groups、grid、carousel 带来的 data/layout/virtual index 不能混用。

本项目首版只做单列，已经扁平化的 group header 也属于 source 中的一个呈现项；选择逻辑只在可选业务项上运行。没有实际 Grid/Tree 消费者前，不增加三重 index mapping 框架。

## 5. GPUI / Base 的实际能力与缺口

| 路径 | 已有事实 | 成本／边界 |
| --- | --- | --- |
| [uniform_list.rs](../../vendor/gpui-pre/src/elements/uniform_list.rs) | 代表行测量、按固定行高计算范围、只构建可见项、UniformListScrollHandle、strict/nearest 定位；局部 item_top 用 f64 差值计算 | 固定高度路径不必分配 N 个 Size；不能给不等高行冒充固定高度；没有通用显式 overscan/pinned-row 合同 |
| 改造前 Base `virtual_list.rs`（基线 `4846e89`） | 已知尺寸向量，纵／横轴；sizes/origins 为 Rc slices；可见边界二分；尾部额外一项；deferred scroll | `O(N)` 几何初始化，稳定 frame clone 共享 backing；几何失效目前比较 item_sizes，未将解析后的 gap 纳入条件；须用回归测试覆盖 gap/rem 变化 |
| [GPUI list.rs](../../vendor/gpui-pre/src/elements/list.rs) | SumTree、逻辑 ListOffset、按需测量、splice、remeasure_items、uniform hint、follow-tail、离屏焦点行保留 | `O(N)` 元数据；可见行仍会实际构建／layout；宽度变化会全量失效；不是持久化业务 key 的控制器 |
| [scrollbar.rs](../../vendor/gpui-base/src/scrollbar.rs) | ScrollbarHandle 已适配三种 handle/state；ListState 支持 drag start/end 冻结高度 | 应复用这条接缝，不能再套 overflow owner 或复制 thumb 计算 |
| [div.rs](../../vendor/gpui-pre/src/elements/div.rs) | role、aria_label、aria_selected、aria_active_descendant、aria_position_in_set、aria_size_of_set、synthetic children | 活动后代声明在子节点；需要真实可访问节点；不能机械搬网页容器端 ARIA 字符串或声称 Narrator 已验收 |

进一步需要在实现前证明的点：

1. `ListState::with_uniform_item_height` 的 hint 与首次／宽度变化时 prepaint 全量清空 hint 的路径必须联合测试；不能仅从方法存在推导首帧 scrollbar 准确。
2. `splice` 新增的是无 hint 项，新增项应继承估算，且旧测量应保留。`remeasure()` 采用 proportional anchor，`remeasure_items()` 采用 absolute；本项目的默认绝对行内偏移合同不能误用前者。
3. 零高度连片可能让原生测量循环走过很多项；应在源/尺寸合同和原生循环保护中止损，不能只在 Yue 画空白。
4. Base 的 known-size 路径 gap 失效、强制对齐与离屏焦点保留，需要最小范围原生补齐；不创建第四种滚动引擎。
5. Pixels 和累计 height 的精度、巨大列表 scrollbar 映射需要实测；浏览器不存在于本项目，但有限精度仍存在。

### 5.1 主轴能力不能由名称推断

| 本地入口 | 已核对的实际行为 | 普适化限制 |
| --- | --- | --- |
| [geometry.rs](../../vendor/gpui-pre/src/geometry.rs) 的 Axis/Along | 已有 Vertical/Horizontal 和 along(axis) 投影 | 基础合同可复用现有类型，不需增加 VirtualAxis；具体引擎能否换轴须另验 |
| [uniform_list.rs](../../vendor/gpui-pre/src/elements/uniform_list.rs) 的 prepaint | item_height × count 求总高，按 scroll_offset.y 求范围，以 y 放置各项 | horizontal_sizing_behavior/横向 overflow 是跨轴能力，不是横向条目虚拟化 |
| 改造前 Base `virtual_list.rs`（基线 `4846e89`） 的 virtual_list | 接收 Axis，ItemSizeLayout 沿对应轴取 sizes/origins，visible_range 用二分定位 | 已知尺寸支持两轴；核心几何仍多为私有实现并与 Element 阶段耦合，尚非公开快照合同 |
| [list.rs](../../vendor/gpui-pre/src/elements/list.rs) 的 layout_items | available_width 约束内容，available_height/size.height 驱动测量和推进，逻辑位置为 ListOffset | 动态测量当前绑定纵向；不得仅增加 Axis 参数便宣称支持横向 |

本项目设计以主轴术语统一合同，首版明确支持纵向 Fixed/Known/Measured 及横向 Known；横向 Fixed/Measured 构造失败，不默默换引擎或分配 N 项尺寸数组。该限制是首版能力矩阵，不能表述为所有布局/轴组合均已实现。

### 5.2 共享旧 handle 不等于共享一个可靠滚动所有者

Base 的 `VirtualList::with_scroll_handle` 是为 Table 提供的隐藏方法：它移除默认 overflow 包装并借用传入 handle，证明外部容器组合已有实际消费基础。但 prepaint 仍会改写 handle state 的 axis、items_count、last_content_size，deferred 定位与 clamp 仍可能调用 set_offset 写完整 Point。

因此，两个列表只拿同一个旧 VirtualListScrollHandle 并不能自动得到独立两轴控制：轴信息会被后一次 prepaint 覆盖，单轴按自己的内容范围 clamp 完整 Point 也可能影响另一轴。应将范围/几何与滚动写入分开，各轴只提出自身意图，唯一宿主合并总内容宽高和 offset 后提交。

GPUI List 的 paint 直接注册 ScrollWheelEvent，ListState 内持有逻辑滚动位置；附着外部宿主时不能同时保留其独立 wheel/写入路径。需要让相同原生测量、SumTree、锚点和焦点算法接受当前宿主视口，输出校正意图；Owned 保留原生所有权，Attached 不再创建第二份输入或实际滚动真值。

[ScrollHandle::set_offset](../../vendor/gpui-pre/src/elements/div.rs) 的注释还明确：向内容后方滚动时绘制 offset 更负。公共正向 main_offset 与原生符号、padding 和局部 origin 的转换应只发生在 binding 边界，不能让每个消费者自行猜测。

以上是新增接缝的源码依据，不是已完成的补丁。优先提取原有算法使 Owned/Attached 共用；不另写一套前缀树、文本测量、焦点树或全局 driver runtime。

### 5.3 从源码事实到可检验的 headless 合同

普适化需要同时满足三个条件：已知几何不依赖呈现；动态测量可以接回真实原生布局；几何结果可以进入不同的容器与语义组合。只把现有 Element 改名为 Virtualizer 不足以证明这些条件。

设计据此将 retained 行为命名为 Virtualizer*，保留 VirtualList 为一维便利适配。viewport/snapshot/capabilities 是表达既有事实的值；不再增加有镜像滚动状态的 Viewport Entity。原始机制不创建 role，列表、Select 或 Grid 的语义由对应消费者选择。

Attached 的定位/校正以本批同步返回的请求值交给 owner，提交后确认实际位置；帧后 range 等事件只作语义通知，不承载本帧校正。总尺寸仍有估算、某项已测量、某次目标定位已收敛也需分别报告，不能等待全部离屏内容测量才结束一个定位命令。

验收同时使用普通复杂卡片流、Select 和单个宿主里的双轴最小组合；还覆盖 Measured Attached、无 renderer 的 Fixed/Known 查询、能力失败无副作用与过期快照拒绝。双轴样例先使用独立已知行列尺寸，不把未知列影响行高的问题隐藏在测试数据里。这些门槛在后续实施中进入生产接缝测试；运行结果与性能测量单独记录，不改写本节的历史源码快照。

## 6. 本项目采用与舍弃

| 调研收获 | 采用方式 | 不采用的部分 |
| --- | --- | --- |
| TanStack 的 headless seam | VirtualizerSource/State、不可变 viewport/snapshot 与呈现接缝分开，按主轴表达 | DOM observer、Proxy、浏览器事件分类常量 |
| virtua 的职责分离 | 原生 layout + Base 合同 + Yue surface；事件按用途去重 | 另一套 store/action runtime、滚动时全局禁止 pointer |
| vlist 的 compact buffers / commit discipline | 共享几何、同帧一致性、预算与清理、测量身份校验 | DOM pool、插件总线、Wheel 接管、浏览器 synthetic engine |
| 三者的锚点校正 | key + 行内偏移，尺寸变化和用户输入来源分开 | 全场景固定使用 index 或像素总 offset |
| Virtualizer 与布局组合 | Owned/Attached、唯一滚动写入者、能力矩阵和双轴最小验收；布局语义由消费者拥有 | 仅凭概念增加 Viewport Entity，或共享旧 handle 即宣称两轴安全 |
| 复杂条目与内容更新 | 明确条目根粒度，减少无效内容工作，同时测量 W 与单条目成本 | 自动拆任意卡片、仅靠裁剪/池化就宣称内部成本消失 |
| Pretext 尺寸推导 | 可靠尺寸来源、明确失效与单一真值；预处理成本和缓存另记 | 新增尺寸模式、JS 文本引擎、无界预排版、近似值覆盖实测 |
| 性能工具 | 先基线、按阶段分账、测量工具 self-check、前后相同 workload | README 的跨平台 FPS／KB 保证 |

## 7. 调研结论落实为基础层要求

以下要求已进入 [设计文档](../28-virtual-list.md)。章节号用于回查合同与后续验收，不表示对应实现或测试已经完成。

| 收获 | 基础层必须考虑的约束 | 设计与验证入口 |
| --- | --- | --- |
| 概念与 headless 边界 | viewport 是几何值，VirtualizerState 是每轴机制，VirtualList 是一维适配；无镜像 Entity | §2.4、§4、§5.1 |
| 布局可组合但并非全等 | 两轴只提出本轴意图；宿主统一应用；先验收已知行列组合，再定义动态 Grid/lanes 的额外尺寸依赖 | §2.4、§5.8、§15.6 |
| 主轴与支持能力 | 复用 Axis，统一正向位置和项内锚点；逐项验证 axis/sizing/binding，不支持时先失败 | §2.5、§5.7、§15.2、§15.6 |
| 语义归属 | 原始机制无 role；Yue 列表默认可覆盖；Select/Grid/Tree 各自拥有语义和导航 | §11.1、§15.6 |
| 无呈现查询与真实测量 | Fixed/Known 无 renderer 求几何，Measured 走原生布局并返回估算/实测状态 | §5.4、§5.8、§7.2、§15.6 |
| 虚拟化粒度 | 一个 source key 对应条目根与主轴尺寸；内部后代由内容所有者负责 | §5.6；§15.3 的复杂卡片/超高条目测试 |
| 几何与内容更新 | 避免逐像素广播业务更新；相同 range 的内容变化仍须生效；不承诺 renderer 零调用 | §10.4；§15.3 的范围/内容更新测试 |
| 测量缓存与合批 | 复用实际布局阶段，缓存按版本失效，批次几何与锚点一致，过期反馈不能回写 | §7；§15.3 的批次与失效测试 |
| 预计算高度 | 输入与真实排版一致；近似/精确来源及写入优先级明确，不制造第四种模式 | §6.5；§15.3 的推导尺寸失效测试 |
| 预处理与资源成本 | estimate/prepare 可能遍历 N；缓存、文本、图片、GPU 分账并有清理规则 | §13.2、§13.4、§13.5 |
| 两个独立性能变量 | 固定工作集改变 N；固定 N/槽位改变复杂度；另测真实增长高度与展平对照 | §13.5、§15.1、§15.4 |
| 证据与迁移 | 复杂内容的原生改造前基线、改造后同等功能对照；不将简单文本 benchmark 当作复杂卡片保证 | §15、§16 |

三个库均带 MIT 许可。当前方案仅研究机制、不复制其实现；若后续移植具体代码，须保留对应来源与许可证，并在 vendor 补丁记录说明改动和测试。公共资料的追溯入口为各包 package.json 所列官方仓库：`https://github.com/TanStack/virtual`、`https://github.com/inokawa/virtua`、`https://github.com/floor/vlist`；API 概念可对照 `https://tanstack.com/virtual/latest/docs/api/virtualizer`，但本记录的版本与算法判断均以本地快照为准。
