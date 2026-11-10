# YueTable 源码调研 04：RevoGrid

日期：2026-11-10。状态：本地源码、类型和定向测试代码阅读完成，结论已补入 [YueTable 设计](../45-table.md)。仅调研与设计，没有实现组件、安装依赖、启动浏览器或运行参考测试／性能采样。

上一项：[Glide Data Grid](2026-11-10-table-glide-review.md)。按用户要求一次一个项目，本轮不研究其余仓库。

## 1. 快照、许可和范围

| 项目 | 核对结果 |
| --- | --- |
| 目录 | `refer/DataTable-Libs/revogrid`，独立 Git 仓库，阅读时工作区干净 |
| HEAD | `e5ea5e019aedb2db2db50ddadba579f81ec77bdf` |
| 版本 | [package.json](../../refer/DataTable-Libs/revogrid/package.json)：`@revolist/revogrid` **4.31.4** |
| 许可 | [LICENSE](../../refer/DataTable-Libs/revogrid/LICENSE)：MIT，2024 Revolist OU |
| 实现 | Stencil Web Component、`@stencil/store`、DOM 虚拟窗口和分区；不是 Canvas 引擎 |
| Core／Pro | [readme.md](../../refer/DataTable-Libs/revogrid/readme.md) 另列 Pro 能力；本轮只以本地 Core 源码中实际存在的机制为证据，不把 Pro 功能介绍当成开源实现 |
| 仓库规则 | 已阅读 [AGENTS.md](../../refer/DataTable-Libs/revogrid/AGENTS.md)，遵循源码／类型优先及 Core／Pro 区分；无可用的 RevoGrid 专用 MCP，本轮使用本地源码与 Context7 |

主线：ViewportStore／DimensionStore → 三区行列 provider → 滚动与 resize → source／proxy／trim → 编辑输入交接 → 生命周期和回归测试。未穷尽全部插件、分组、导出、框架适配器或 Pro。

Context7 查询 `/revolist/revogrid-docs` 的 `virtualX`、固定列和视口生命周期。在线文档与本地重点一致：默认只虚拟化普通列，固定列可显式加入 `virtualX`。检索不证明该 HEAD 是最新发布，也不替代本地实现。

## 2. 数据、尺寸、窗口分开，少量滚动复用窗口

证据：[viewport.store.ts](../../refer/DataTable-Libs/revogrid/src/store/vp/viewport.store.ts) 的 `setViewPortCoordinate`、[viewport.helpers.ts](../../refer/DataTable-Libs/revogrid/src/store/vp/viewport.helpers.ts) 的 `getUpdatedItemsByPosition`／`recombineByOffset`、[dimension.helpers.ts](../../refer/DataTable-Libs/revogrid/src/store/dimension/dimension.helpers.ts)。

Dimension 管总长度、默认尺寸、自定义尺寸和位置查找；Viewport 管工作项、环形 start／end、可见尺寸与最后位置。小幅滚动时轮换离开的项并补齐进入窗口的项，大跨度才重建窗口。轮换仍复制数组、创建被替换的几何对象，不等于完全无分配，更不等于保留业务对象身份。

自定义尺寸保存稀疏例外，按排序后的索引／位置建立查询；均匀尺寸可以通过算式得到总量，不为每行填一份尺寸。该策略在大量均匀行、少量例外时有意义；重建例外索引、trim 移位与窗口更新各有成本，不宜概括成所有操作 O(1)。

Yue 直接复用已有 Base [geometry.rs](../../vendor/gpui-base/src/virtualization/geometry.rs)：Fixed 算术、Known 索引、Measured SumTree，以及现有预算／工作集／stable key。没有必要再写环形 DOM 池或第二套尺寸索引。吸收连续滚动正反向、边界和大跳转的测试方式；重用槽位不能导致 Checkbox、Input、选择和资源身份串到别的 row key。

参考用默认尺寸的一项作为两侧缓冲，并对取窗起点另行 clamp。Yue 沿用 Base 范围预算，不复制该常量；尤其不能把带缓冲的取窗起点当作真实 owner 滚动位置。

## 3. 固定区的成本必须算进去，但不复制多滚动区结构

证据：[store/index.ts](../../refer/DataTable-Libs/revogrid/src/store/index.ts)、[viewport.service.ts](../../refer/DataTable-Libs/revogrid/src/components/revoGrid/viewport.service.ts)、[viewport.helpers.ts](../../refer/DataTable-Libs/revogrid/src/components/revoGrid/viewport.helpers.ts)、[dimension.provider.ts](../../refer/DataTable-Libs/revogrid/src/services/dimension.provider.ts)、[viewport.scrolling.service.ts](../../refer/DataTable-Libs/revogrid/src/components/revoGrid/viewport.scrolling.service.ts)。

行与列各分 Start／普通／End；组合为多个显示区，分区共享各自轴的 dimension／viewport。`DEFAULT_VIRTUAL_X=['rgCol']`，固定列默认非虚拟，显式配置后才能虚拟化固定列区域。滚动服务在多个元素之间传播并等待更新，固定区还可拥有独立横向滚动或转交 delta。

Yue 吸收分区的几何、预算和身份要求，不吸收多份 scroll position 的同步链。保留一份 accepted 列轨道、一个滚动 owner、单个可交互内容实例；冻结／裁剪通过 Base 最小扩展实现。不能因列叫“固定”就渲染该区全部行，不能把固定列的完整内容排除在工作集统计之外。冻结量过大沿用设计的受限布局规则，不另增加“固定区独立滚动”的公共能力。

参考 `pinnedTopSource／pinnedBottomSource` 是独立数据源；Yue 首尾冻结是同一视图内的记录，不新增两个业务来源来绕开身份。`pinning.spec.ts` 还验证短正文时尾部行紧接正文，避免无意义大空洞：Yue 在内容不足一屏时保持自然顺序，内容超出才钉住尾边；不把数据尾行当作总是贴窗底的 summary／affix。

## 4. Resize 要保证覆盖连续，不能依赖下一个 scroll 来补格子

证据：`ViewportService` 的 `onResizeviewport`、[viewport.resize.service.ts](../../refer/DataTable-Libs/revogrid/src/components/revoGrid/viewport.resize.service.ts)、[viewport.resize.spec.ts](../../refer/DataTable-Libs/revogrid/test/viewport.resize.spec.ts)。

直接改变 viewport virtualSize 默认会清空工作项；服务因此在 resize 期间携带旧 items，等坐标同步重算覆盖。缩小时主动缩减工作集；虚拟固定列增大时立即补齐，因为该区未必收到后续 scroll。重复同尺寸不清空窗口。尾部扩容需要夹紧实际位置，不能只把旧范围画到更大的容器里。

Yue 的具体合同：先用新约束计算合法轨道、范围与 owner 位置，再作为一致布局结果呈现，不发布“新几何＋空工作集＋下一帧补齐”的可见中间态。旧窗口仅可作为仍一致的过渡缓存，不能用旧 query 的格子填新 query 的洞。无实际变化不重置尺寸或 Entity；扩大覆盖在下一次合法布局完成，不要求用户再滚动触发。

验收宽／高分别变化及同时变化，包含右下边界、固定区域、横向溢出、零尺寸恢复。检查视口四角内部采样点对应的 cell 身份和实际命中，表头／正文／焦点同轨；只断言“格子数量大于零”无法发现局部空洞和错误索引。采样点须排除边框、合法空白与装饰，不把短内容本来没有数据的区域当缺格。

参考 resize throttle 为 40ms；Yue 采用现有 GPUI 帧／布局合批，不增加 Web 定时器常量，也不为了过渡冻结整个表格。

## 5. 筛选／折叠撤销后，尺寸不能跟错记录

证据：[data.store.ts](../../refer/DataTable-Libs/revogrid/src/store/dataSource/data.store.ts)、[data.proxy.ts](../../refer/DataTable-Libs/revogrid/src/store/dataSource/data.proxy.ts)、[trimmed.plugin.ts](../../refer/DataTable-Libs/revogrid/src/store/dataSource/trimmed.plugin.ts)、[dimension.trim.plugin.ts](../../refer/DataTable-Libs/revogrid/src/store/dimension/dimension.trim.plugin.ts)、[dimension.store.ts](../../refer/DataTable-Libs/revogrid/src/store/dimension/dimension.store.ts)。

参考区分原始 source、排序 proxyItems 与移除隐藏项后的 items；trim 按命名原因合并，任一层隐藏都不显示。尺寸 trim 先保留未裁剪尺寸，再移位，避免多次隐藏／恢复不断压缩已经移位过的 map。新输入尺寸会重置旧恢复基线；调用方尺寸 map 被复制，外部后续修改不会偷偷改变内部几何。

`updateData` 明示物理索引可能随整批数据变化，保留 trim 要显式 opt-in。`setItemsPending` 则刻意令 items 为空，直到异步操作结束，并在中途 proxy／trim 改变时保持为空。该行为不作为 Yue 的刷新默认值：已有有效 accepted view 时继续显示它及 busy 状态，完成后原子接受新来源，不能用 pending 期间全空代替一致提交。

Yue 不增加公共任意 trim 插件图。过滤／分页／展开各有既有语义阶段，保留其他阶段仍在施加的限制；解除筛选不自动展开折叠分支。Known 尺寸按 stable key 映射，Measured 复用按 key＋有效约束版本判断；同位置换成新 key 不继承旧高度，暂离投影的 key 可在有界缓存内恢复，超过预算或约束变化后重新测量。

尺寸输入与恢复缓存不能与宿主可变缓冲区共用可写所有权。不可变共享快照无需额外复制；发布后不得原地修改。拒绝非有限／非法尺寸按 Yue／Base 类型化合同处理，不照搬 RevoGrid“过滤坏值继续”的具体行为，也不为隐藏行制造零高槽位。复用 Base 更新事务与已有 Tree 投影。

## 6. 深处滚动：数据位置、绘制、焦点和命中必须同源

证据：[scroll.dimension.helpers.ts](../../refer/DataTable-Libs/revogrid/src/services/scroll.dimension.helpers.ts)、[local.scroll.service.ts](../../refer/DataTable-Libs/revogrid/src/services/local.scroll.service.ts)、[local.scroll.timer.ts](../../refer/DataTable-Libs/revogrid/src/services/local.scroll.timer.ts)、`DimensionProvider.setViewPortCoordinate`。

浏览器的最大滚动尺寸有限，参考检测物理范围上限，对超长数据映射 logical／physical 坐标，用 renderOffset 补偿。带缓冲的取窗 clamp 和真实滚动 clamp 不同；末尾自定义行高时 renderOffset 必须用真实 logical end，不能用被缓冲拉回的位置。

LocalScrollService 分别记录精确逻辑值与浏览器实际写入／读回值，抑制同一写入的反馈；wheel delta 累加到逻辑位置，不反复经浏览器取整往返。测试还覆盖新位置待提交时，旧反馈不应取消它，以及接受中间位置后快速返回旧位置不能被永久去重。参考本身仍有 Math.ceil 和定时器，不能称作原生亚像素滚动的完整样板。

本项目 [ScrollOffset](../../vendor/gpui-pre/src/elements/div.rs) 已有单 owner 的 f64 值与 Pixels 投影，Base geometry 使用 f64，Grid 先计算局部坐标再转 Pixels。没有浏览器 DOM 最大高度问题，故本轮不移植压缩映射或另建 logical owner。

吸收验收：大偏移下连续小 delta、快速往返、程序定位＋用户滚动交错；只在最终局部坐标边界转换像素。除了显示目标文本，还要点击正确 key、检查 active／focus bounds、编辑和 Popup anchor／resize 命中。两轴都测，包含末行／末列自定义尺寸；容差依据 GPUI 基线，不能照搬浏览器 1px／2px。纯几何或某个大数值断言不证明所有原生布局路径都已通过。

## 7. 已接受编辑的输入交接，复用真实 Input

证据：[keyboard.service.ts](../../refer/DataTable-Libs/revogrid/src/components/overlay/keyboard.service.ts)、[revogr-overlay-selection.tsx](../../refer/DataTable-Libs/revogrid/src/components/overlay/revogr-overlay-selection.tsx)、[revogr-edit.tsx](../../refer/DataTable-Libs/revogrid/src/components/editors/revogr-edit.tsx)、[text-editor.ts](../../refer/DataTable-Libs/revogrid/src/components/editors/text-editor.ts)。

参考处理编辑开始后、输入框尚未挂载／拿焦点时的后续字符：尚无 editor 就暂存在 edit 值；已有真实 input 则直接交给它，避免下一帧用旧 store 文本覆盖用户随后原生输入。selection／edit 外部改变会使待处理键盘变更代次失效。TextEditor 在 composition 期间不因 Enter／Tab 保存，销毁前协调失焦。

Yue 已有宿主保留的 Input Entity、文本替换与 IME 路径，不照搬 DOM value 拼接或手工 key→字符转换。用户通过既定 F2／Enter／按钮进入后，优先同步衔接真实编辑器焦点与输入，保证连续输入一次且按序消费，重渲染不拿旧初值 set_value。必要的交接缓存只属于已接受的同一 cell／会话，有数量边界，取消、拒绝或换目标后清理；未接受编辑请求前不拦截用户文字。

已经送到 Input 的文本不能再由 Table 重放；组合输入、死键、Windows AltGr 与快捷键按真实文本／命令通道区分。不用 Ctrl+Alt 的修饰键猜测代替系统文字输入。参考支持直接打印字符启编和 Space 等入口；本轮不增加这些入口，不改变第 18 节 Space 选择／控件行为，只吸收现有编辑过程不丢输入的保证。

## 8. 挂载、移除、再挂载也要验收

证据：[revogr-viewport-scroll.tsx](../../refer/DataTable-Libs/revogrid/src/components/scroll/revogr-viewport-scroll.tsx)、`GridResizeService.destroy`、[viewport-lifecycle.spec.ts](../../refer/DataTable-Libs/revogrid/test/viewport-lifecycle.spec.ts)、[lifecycle.spec.ts](../../refer/DataTable-Libs/revogrid/e2e/lifecycle.spec.ts)。

参考 componentDidLoad／reconnect 初始化 ResizeObserver，避免重复建立；disconnect 销毁服务并置空，destroy 取消尾随 throttle。测试用同一个 grid 实例移动到新容器，重新伸缩后与未移动对照一致，并检查滚动后的记录仍正确。

Yue 复用 Base attachment／mount revision、GPUI Subscription／Task 与 Popup owner，不引入 DOM observer。短暂移除与真正释放区分：同一 state 在合法新挂载恢复 accepted view、keyed 会话与锚点；窗口绑定、输入捕获和浮层须按当前宿主重建，旧 mount 的延迟尺寸／滚动／焦点任务不能写入新 mount。重复挂载或释放不得重复订阅、双重事件或残留捕获。

用户已编辑的草稿保留仍按第 14 节宿主流程处理，不能因为 element 暂离树就隐式保存。原生跨窗口再挂载是否支持以 Base 的实际合同为准；无合法绑定时返回既有失败结果，不把 Web reconnect 等同于任意 GPUI 窗口迁移能力。

## 9. 读到的测试及可移植验收

以下逐项阅读了相关测试断言，未执行。`test/` 是 Stencil／Jest 路径，部分直接构造 service 并模拟 observer／rAF；`e2e/` 是 [Playwright 配置](../../refer/DataTable-Libs/revogrid/playwright.config.ts) 下的浏览器测试，不是本轮已跑的结果。部分 E2E 也使用合成 KeyboardEvent、手改 input.value 或内部 provider，不能当作真实扫描枪／IME 设备验收。

| 参考 | 实际断言 | Yue 的测试落点 |
| --- | --- | --- |
| [viewport.spec.ts](../../refer/DataTable-Libs/revogrid/test/viewport.spec.ts) | 多轮正／反向 recombine 后范围有序、位置非负、end 不越界 | 复用 Base 范围／缓存测试；另验同槽换 key 不继承控件状态，不复制环形实现 |
| [dimension.spec.ts](../../refer/DataTable-Libs/revogrid/test/dimension.spec.ts) | 均匀百万行总长、自定义 delta、非有限尺寸处理、重复 trim 恢复、调用方 map 后续改动不污染内部 | key＋约束版本恢复；非法输入原子拒绝；不可变尺寸来源与投影一致 |
| [viewport.resize.spec.ts](../../refer/DataTable-Libs/revogrid/test/viewport.resize.spec.ts) | 重复 resize 保持 items；收缩无空发布；末端增长夹紧；虚拟固定列增长无需后续 scroll | 真实布局同帧一致、四角覆盖、重复尺寸不失效；固定区按同一 owner 验收 |
| [resize-trim-regressions.spec.ts](../../refer/DataTable-Libs/revogrid/e2e/resize-trim-regressions.spec.ts) | 多次隐藏／恢复实际行高；右下角扩缩后 scroll 与命中采样内容匹配，过程中无空 items | 真实 GPUI bounds／命中／scene，包含自然短内容；不能只看最终元素数量 |
| [pinning.spec.ts](../../refer/DataTable-Libs/revogrid/e2e/pinning.spec.ts) | 四边固定可见、短正文尾行无大间隙、同分区局部坐标区分、活动列 pin 后焦点映射 | 保持 stable cell key 的单实例和单语义身份；首尾区切换不丢草稿 |
| [scroll.dimension.spec.ts](../../refer/DataTable-Libs/revogrid/test/scroll.dimension.spec.ts) | 压缩映射起中末、wheel 逻辑累加、取整反馈去重、待执行 scroll 不被旧反馈取消、自定义尾部偏移 | 复用 f64 owner；测实际应用位置和局部坐标；不照搬 DOM 映射或整像素常量 |
| [virtualization.spec.ts](../../refer/DataTable-Libs/revogrid/e2e/virtualization.spec.ts) | 有界窗口、深处目标文本、focus bounds 对齐及深处点击／编辑／拖动 | Table 真实 key、焦点、编辑和命中联合验收；不为匹配参考另加电子表格复制／填充 |
| [viewport.scrolling.spec.ts](../../refer/DataTable-Libs/revogrid/test/viewport.scrolling.spec.ts) | 多元素 setScroll 依次完成后发布，固定 delta 转交和独立分区路由 | 参考用于理解其结构成本；Yue 断言一次输入只写一次 owner，无互相回写环 |
| [keyboard.service.spec.ts](../../refer/DataTable-Libs/revogrid/test/keyboard.service.spec.ts) 与 [editing.spec.ts](../../refer/DataTable-Libs/revogrid/e2e/editing.spec.ts) | editor 未挂载时暂存；已有 input 不更新旧 store；条码字符串和缓冲＋原生输入保留；合成 AltGr | 已接受会话的快速连续文本、IME 优先、Input 实体唯一来源、取消不重放旧输入 |
| [viewport-lifecycle.spec.ts](../../refer/DataTable-Libs/revogrid/test/viewport-lifecycle.spec.ts) 与 [lifecycle.spec.ts](../../refer/DataTable-Libs/revogrid/e2e/lifecycle.spec.ts) | 重连新建一个 observer、重复 disconnect 只释放一次；同实例移动后跟随新尺寸 | 多次挂载／卸载／重绑、旧 mount 回调隔离、无重复订阅、真实焦点与锚点修复 |
| [data.store.spec.ts](../../refer/DataTable-Libs/revogrid/test/data.store.spec.ts) | pending 中 proxy／trim／source 变化仍保持空 items，释放才恢复 | 不复制清空显示的默认；验证 Yue busy 时保留有效 accepted view，新来源原子接受 |

性能仍需在本项目相同 workload 下前后采样；参考存在百万行 E2E 代码不代表本轮通过该测试，也不代表 GPUI 已验证同规模。

## 10. 吸收清单与下一项

| 编号 | 吸收决定 | 阶段 |
| --- | --- | --- |
| RG01 | 复用 Base 工作集／尺寸索引，借鉴反复正反向／大跳转测试，不另建 DOM 环形池 | M1／M5 |
| RG02 | 固定区计入预算、按 stable key 切换；短内容尾冻结保持自然顺序 | M2／M5 |
| RG03 | resize 不发布不一致空窗口，扩大立即补覆盖、收缩减工作集，四角内容与命中对齐 | M1／M2／M5 |
| RG04 | 尺寸按 key＋约束版本恢复；筛选／折叠互不解除，尺寸快照不受外部可变别名污染 | M3／M4 |
| RG05 | 深处位移、最后一行列、点击／焦点／editor／Popup 同源；复用精确 owner，不移植浏览器压缩 | M1／M2／M4／M5 |
| RG06 | 已接受编辑焦点交接期间输入不丢、不重放、不用旧初值覆盖；不新增打印字符启编入口 | M4 |
| RG07 | 重新挂载恢复合法绑定，旧 mount 任务隔离，释放／重绑幂等 | 各阶段 |
| RG08 | pending 刷新保留有效 accepted view，不照搬参考清空 items | M3 |

不新增多滚动 owner、固定区独立滚动、trim 插件平台、Web Component／跨框架包装器、浏览器滚动尺寸压缩、Pro 专用特性或额外电子表格入口。新增内容集中在既有 M1–M6 功能的正确性和验收。

下一项为 `refer/DataTable-Libs/mui-x`（MUI X Data Grid），须先区分 Community／Pro／Premium 的源码与许可，再研究状态选择器、布局协调、更新批次与交互回归。本轮尚未开展该项源码调研。

后续进展：第 5 项 [MUI X Data Grid 调研](2026-11-10-table-mui-review.md)已完成，以上仍保留本轮的阅读范围。
