# Table 源码调研 09：SlickGrid

- 日期：2026-11-10；对应 [Table 设计](../45-table.md)，接续 [Grid.js 调研](2026-11-10-table-gridjs-review.md)。
- 本地目录：`refer/DataTable-Libs/SlickGrid`；HEAD：`dee9256d986f5305caa91eacfef95a5f9e1cf146`；[package.json](../../refer/DataTable-Libs/SlickGrid/package.json) 版本 **6.0.0**，根 LICENSE 为 MIT。
- 已阅读该仓库 AGENTS.md：不编辑 dist 构建产物；本轮只修改 Yue 的设计与调研文档，未修改参考仓库。
- 依据为核心源码、定向测试片段及本地性能文档；另用 Context7 核对 6pac/SlickGrid 文档。没有安装依赖、运行参考测试／示例或进行性能实测。

## 1. 本轮研究的是 6pac/SlickGrid 6.0.0

不能把旧版多 viewport／冻结 pane 的印象直接套到当前快照。[slick.grid.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.grid.ts) 已有 unified pinning／docking：行分区、列分区、共享布局结果、正文虚拟工作集，以及按需保留的固定项。选择、编辑、行详情和列菜单等另有明确部件；DataView 负责数据处理，Grid 消费投影与 metadata。

它具备真实行／列虚拟渲染，与上一项 Grid.js 对当前结果逐行渲染不同。Yue 可学习其几何、工作集与数据变化的接线，但继续复用 Base VirtualList／VirtualGrid、ScrollHandle、ResizeHandle 和已有 Yue 控件。SlickGrid 的分组聚合、复制填充、任意行 pinning、滚动触发 sticky 与插件体系不因此进入 Yue 新范围。

Slickgrid-Universal 是下一项独立仓库，版本、能力和测试不能与本轮混写。

## 2. 虚拟工作集：及时覆盖新视口，避免用延时遮掩空白

证据：[slick.grid.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.grid.ts) 的 getRenderedRange、cleanupRows、cleanUpAndRenderCells、renderRows、render、_handleScroll。

getRenderedRange 根据纵向方向分配前后缓冲，横向扩展像素范围。render 清理窗口外行、补现有行缺失的列、创建新行，再补固定行；固定行中间区域的 cell 仍按横向范围虚拟化。普通无 dock／特殊 metadata 路径使用列 lower-bound 跳过左侧离屏列，不能据此称所有分支都有同样复杂度。

当前源码默认大纵向跳转及时 render，允许显式启用节流；横向先更新滚动变换，再合并昂贵 cell 更新。它说明滚动位置、当前覆盖范围、构建调度必须一起考虑，而不是“少 render 就一定快”。

Yue 使用现有 Base 范围／预算，不复制一整屏缓冲、0.8 倍阈值或固定毫秒参数。滚动超过现有覆盖、快速反向或程序跳转时，下一次提交的 GPUI 场景须覆盖有效视口；不能直到输入停止才补内容，也不能将旧窗口变换到新位置后显示错误 key。连续输入合批仍读最新 owner 位置，失效旧调度不得回滚新工作集。具体调度服从 GPUI 更新／布局阶段，不承诺每个 native scroll event 同步 paint。

cleanupRows 会保留 activeRow 和覆盖当前范围的 rowspan origin。Yue 吸收必需项不被回收的目的，但沿既有 Cells 精确 cell key／span 索引，不为一个屏外活动格或 origin 保留整条行、扫描全部列或扩大其他行的窗口。

## 3. 坐标、冻结区与命中必须来自同一份几何

证据：[slick.grid.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.grid.ts) 的 setScroller、scrollTo、getPageOffset、getRowPosition／getRowFromPosition、getCellFromPoint／getCellFromDockedPoint、render 中 rowDockingStale 处理。

SlickGrid 区分自然位置、虚拟页偏移和实际渲染位置。scrollTo 先求目标页偏移，写滚动 owner 后回读浏览器实际夹紧位置，再更新行坐标。固定区命中按已解析 docking 几何计算，不要求屏外目标有 DOM；同数量排序／过滤也可能改变行身份，需要重解成员，而不是只在 row count 改变时更新。

Yue 继续用同一个双轴 ScrollHandle 和 Base f64／局部 Pixels 坐标，不移植浏览器最大 CSS 高度的分页压缩映射，也不引入横纵两个位置事实。程序定位与数据缩短后采用实际合法 owner 位置，不拿请求值代替结果值。

补几何联合验收：从冻结／正文的可见 cell 内部点回查，命中 key 须等于该 cell，bounds、焦点和 editor／Popup anchor 同源；同时用模型坐标解析尚未构建的目标，再滚入验证。覆盖首尾冻结交叉、横纵大偏移、列宽变化及同数量重排／换 key。边界共享统一归属，裁剪外的底层格不能接到覆盖区点击。

这不改变 Yue 的首尾数量冻结合同，不引入参考的任意离散行 pinning。受限布局、短内容尾行和非法跨冻结 span 仍按主设计明确处理。

## 4. DataView：有用的是变化信息，而非第二份数据所有权

证据：[slick.dataview.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.dataview.ts) 的 idxById／rowsById、updateItems、beginUpdate／endUpdate、getRowDiffs、recalc、refresh。

DataView 将源 items、过滤结果、当前显示 rows 与 key 索引分开；diff 用当前位置的 ID 变化和 updated 标志找需要刷新的行。refresh 分别通知计数、行变化，并提供合并的 onRowsOrCountChanged 事件，避免消费者猜测两类通知的关系。updateItems 合并后 refresh，bulk update 可推迟索引／删除处理，文档明确批次期间部分读取未必一致。

Yue 借鉴一批数据变化同时发布数量、key 投影和内容变化范围的方式，沿单次 accepted view 和版本化失效，不额外建立可独立修改的 DataView。验收须区分“行数未变但顺序／内容变了”和“数量改变且内容也变了”，两个区域不能各自调一次完整查询／布局。参考部分测试只是断言 refresh 被调用，不能据此证明只完成一次重算。

参考 sort 会原地排列 items，updateItems 顺序修改且末尾才 refresh；这些不是原子回滚范本。Yue 保持宿主先校验、来源与 view 同批发布、失败保留原状，不暴露成对 begin/end 公共锁，不依赖异常后使用者手动解锁，也不把显示行索引当业务身份。

## 5. 过滤收窄／扩张只在可证明时使用

证据：[DataViewHints](../../refer/DataTable-Libs/SlickGrid/src/models/dataViewHints.interface.ts)、[slick.dataview.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.dataview.ts) 的 getFilteredAndPagedItems／filterItemsWithCaching、[performance.md](../../refer/DataTable-Libs/SlickGrid/docs/in-depth/performance.md) 的 refresh hints。

- Narrowing 只重新过滤上一次结果；Expanding 遍历来源但复用此前成功匹配项；Unchanged 复用现有过滤结果。
- ignoreDiffsBefore／After 限制 diff 计算窗口；这减少比较，不代表窗口之外的业务事实没有变化。
- hints 每次 refresh 后清除。源码使用普通函数保留闭包，本地测试还明确断言不调用运行时代码生成；在线旧示例的 inlineFilters 不能当作该版本仍依赖函数编译的证据。

Yue 不新增公开 hint 开关，也不立即实现第二套过滤缓存。只有采样证明 Local 过滤确为瓶颈时，才在现有本地适配器考虑有证明的内部优化：同一来源／列语义／筛选规则下，明确阈值或字面包含关系才可推导新旧集合。输入变长、删字、粘贴、替换、IME 提交都不自动等于收窄／扩张；来源更新、自定义函数或字段范围变化则保守重算。

候选优化必须与完整基线比较完整结果 key 顺序、total、分页及选择范围，不能只比较当前窗口；已有资源／离屏内容版本也须正确失效。没有足够依赖信息继续使用主设计的完整过滤路径，未测得收益就不吸收实现。

## 6. 重内容分期与清理：不要新增后处理引擎

证据：[slick.grid.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.grid.ts) 的 asyncPostProcessRows、queuePostProcessedRowForCleanup、asyncPostProcessCleanupRows。

SlickGrid 让同步 formatter 先展示轻内容，再按行分批调用 asyncPostRender；已处理项标记 R，待清理／刷新项标记 C，清理队列按组释放节点和自定义资源。这里的 async 主要是定时分期，回调调用处并不 await；名称不表示已经提供网络完成代次隔离。一整行可能含很多复杂格，“每批一行”也不保证帧成本有界。

Yue 将其转成已有复杂 renderer／图片资源的成本验收：有实测必要时，由稳定内容 owner 和既有 Task 分期计算可选重内容，前景保留必要文本、语义、交互与稳定占位几何。后台可计算纯数据，不把 GPUI Element／Entity 的窗口操作搬到任意线程。结果仍按 cell key、内容与 mount 版本接受，滚出、换列、换来源、失败和卸载都结算使用者与资源。

预算按实际格数／工作耗时及保留字节衡量，清理积压也要计入，不只数当前工作集。沿已有 ImageCache、Task、内容 stamp 和按需帧，不为普通文字表新增公共 asyncPostRender 钩子或后处理队列；若无性能证据，只补测试合同。

## 7. 编辑提交后的重入与订阅归属

证据：[slick.core.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.core.ts) 的 SlickEditorLock、[slick.grid.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.grid.ts) 的 commitCurrentEdit／navigate。

参考在导航前尝试完成编辑，校验失败保留 editor 并阻止前进；保存命令执行后还检查事件回调是否重新取得编辑 lock。这处理了“保存处理器同步开启另一编辑器”的重入，不能只看旧保存返回成功就继续旧导航。

Yue 不移植全局 EditorLock、execute／undo 历史系统或直接修改业务记录。沿第 14 节的会话／请求合同，保存接受、校验、外部事件或 Dialog 完成后，继续导航／结构操作前重验 owner、来源、session 和目标。若宿主已启动新会话、换 query 或转移焦点，旧动作停止，不关闭新 editor、不再次保存或抢焦点；异步路径同样适用。失败仍使用同一真实输入 Entity 和既有 Field／Form 错误。

证据：[slick.dataview.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.dataview.ts) 的 destroy／syncGridSelection。选择通过 ID 与投影行号协调；DataView 清理自己绑定到 Grid 的 handler，已有测试检查其他订阅仍被通知。Yue 已有 stable-key 选择合同，不再镜像一份 index 选择；补“同一 delegate 被两表或业务订阅共享，释放一表后另一使用者仍正常”的验收。只释放本 owner 的 Task／Subscription，不对宿主共享事件源执行全局清空。

## 8. 跨冻结 span 的边界仍保持明确

证据：[slick.grid.ts](../../refer/DataTable-Libs/SlickGrid/src/slick.grid.ts) 的 appendColspanFragments／createColspanContinuationContent。当前参考按 docking 段绘制 continuation，去除重复语义属性，并深拷贝内容作为展示副本；也对会拆开 span 的部分非连续 pinning 明确拒绝。

可学习唯一语义主体、拒绝先于发布、回收时清理全部片段的原则，但深拷贝 Input／Button 内容不符合 Yue 单一可操作实例合同。Yue 不因此放宽跨冻结合并限制，不复制真实控件树，不为兼容参考另建呈现片段系统；若既有组合未实现就继续类型化拒绝。离屏 span 也必须参加配置合法性检查，不能等滚入才发现并破坏半张表。

## 9. 已阅读的定向测试与证据边界

本轮没有逐条审阅整套测试。Vitest 配置采用 jsdom，相关几何测试常替换 clientWidth／Height 或直接设置内部布局；它们能验证规则和接线，不能替代真实浏览器／GPUI 布局。下表用例均未运行。

| 测试／已读片段 | 实际断言与 Yue 落点 |
| --- | --- |
| [slickDataView.spec.ts](../../refer/DataTable-Libs/SlickGrid/src/__tests__/slickDataView.spec.ts) 的批量 CRUD、Filtering 片段 | 合批最终记录与删除结果、过滤保留闭包且不 new Function、Expanding 第二次只多检查未匹配项。Yue 借鉴完整 key 结果与调用计数；不把 refresh spy 的“被调用”误写成仅重算一次，也不宣称所有 hint 组合已覆盖。 |
| 同文件的选择与 destroy 片段 | 分页后的 selected IDs／filtered IDs、相同选择归一、批量选择缓存命中／失效；销毁后原业务订阅仍正常。Yue 检查稳定选择域、当前投影与共享订阅归属，不复制 selection 模型双向同步。 |
| [slickGrid.spec.ts](../../refer/DataTable-Libs/SlickGrid/src/__tests__/slickGrid.spec.ts) 的 scroll throttle 片段 | 两次大跳默认调用 render 两次；显式节流改为 enqueue；更改节流配置后旧任务不额外执行。Yue 验证新工作集覆盖与旧任务隔离，不要求相同 render 次数或公开对应开关。 |
| 同文件的 rowspan、校验失败与 async cell 片段 | 强制保留 span origin、计数／节点存在；校验失败回 false 并通知；延后 renderer 更新文本。Yue 使用稀疏 span、真实 Input／焦点和场景内容验收，不照搬固定格数或 DOM innerHTML。 |
| [slickGrid-pinning.spec.ts](../../refer/DataTable-Libs/SlickGrid/src/__tests__/slickGrid-pinning.spec.ts) 的配置拒绝／语义片段 | setColumns 被拒后调用者输入和当前配置不被改坏、无提前事件；装饰 wrapper 不重复语义，列索引一致。Yue 维持原子拒绝与单一语义树，不据 ARIA 属性推导 Windows UIA 完整。 |
| 同文件的重排、owner、命中片段 | 反转同数来源后 ID pin 重映射；owner 将请求 900 夹到 850 后内部同为 850；几何命中不调用 elementFromPoint，覆盖 top／center／bottom 和左右列。Yue 只借鉴同数变化重算、实际 owner 与正反坐标，不增加任意行 pinning。 |
| 同文件的异步清理片段 | 两次 enqueue 合为一次回调；同组 R 节点移除、C 清理回调参数与队列清空；离屏 colspan 的主格和 continuation 同时移除。Yue 检查有界任务与资源释放，不将 fake timer 通过当帧耗时通过。 |
| [quirk-pinning-hit-testing-geometry.cy.ts](../../refer/DataTable-Libs/SlickGrid/cypress/e2e/quirk-pinning-hit-testing-geometry.cy.ts) | 浏览器滚动后从真实 cell 矩形取点，验证左右固定列、头尾行和未渲染行的几何反查。Yue 转成 GPUI bounds→key 与模型→滚入后的联合断言；不采用非连续 pin 功能。 |
| [quirk-pinning-bottom-hit-testing.cy.ts](../../refer/DataTable-Libs/SlickGrid/cypress/e2e/quirk-pinning-bottom-hit-testing.cy.ts) | 两表 harness 验证底固定行与正文、顶固定对照的点击行身份；[Cypress 配置](../../refer/DataTable-Libs/SlickGrid/cypress/cypress.config.ts) 关闭 testIsolation，后续用例沿用前一用例页面。Yue 用独立可重复 fixture，不复制这种跨用例依赖。 |
| [quirk-scroll-rendering-performance.cy.ts](../../refer/DataTable-Libs/SlickGrid/cypress/e2e/quirk-scroll-rendering-performance.cy.ts) 的首批跳转／节流用例 | 50k／500k 数据大跳、反向和边界夹紧后检查缺行／缺格、内容与列对齐；延后任务用 clock／tick 控制。Yue 学习几何覆盖断言，不能从文件名推导 FPS／分配收益，也不照搬 1px 容差或五次 render 数量。 |

## 10. 吸收清单与下一项

| 编号 | 补充的合同／回归 | 阶段 |
| --- | --- | --- |
| SG01 | 大跳、快速反向、超出缓冲后的下一场景有效覆盖；合批始终读取最新 owner，必要保留项仍稀疏 | M1／M5 |
| SG02 | bounds→命中 key、模型坐标→滚入、冻结交叉与夹紧后实际 owner 联合验证；同数投影变化也更新 | M2／M5 |
| SG03 | 行数未变的内容／顺序变化、数量与内容同时变化，一次一致提交与失效，不重复全管线 | M1／M3 |
| SG04 | 有采样需求才考虑可证明的 Local 集合复用；与完整基线核对完整 key／total／页／选择，未知依赖退回完整重算 | M3 |
| SG05 | 已有复杂资源分期守格数／时间／字节预算，旧结果隔离且清理不积压，必要语义／交互不延后 | M1／M5 |
| SG06 | 保存回调同步／异步重入后重验会话，旧导航不破坏新 editor／query／焦点 | M4 |
| SG07 | 释放一表只解自己的订阅／任务，共享 delegate 和业务订阅仍有效；独立 fixture | M1–M4 |
| SG08 | 离屏非法 span 配置也原子拒绝；唯一内容与语义、不拷贝交互控件、不因参考放宽能力 | M2／M5 |

本轮没有实现上述优化；条件过滤复用、复杂内容分期均须有实测需求才决定是否实现，其他项补入既有阶段验收。未引入分组聚合／undo／电子表格、动态插件、额外滚动 owner 或跨冻结控件副本。

下一项为 `refer/DataTable-Libs/slickgrid-universal`，先核对版本、包分层和与 SlickGrid 的关系，再单独判断服务／扩展组合的价值。本轮尚未开展该项。

后续进展：[Slickgrid-Universal 11.0.0 专项调研](2026-11-10-table-slickgrid-universal-review.md)已完成，十个参考仓库现已全部完成。以上“下一项”保留为本轮交付时的历史范围。
