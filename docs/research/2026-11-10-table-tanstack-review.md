# YueTable 源码调研 02：TanStack Table

日期：2026-11-10。状态：本地源码、文档和定向测试代码阅读完成，结论已补入 [YueTable 设计](../45-table.md)。本轮只做设计，不实现组件、不安装 JS 依赖、不运行参考测试或性能采样。

上一项：[Tabulator](2026-11-10-table-tabulator-review.md)。按用户要求逐个推进，其余参考项目不在本轮结论内。

## 1. 快照与范围

| 项目 | 核对结果 |
| --- | --- |
| 本地目录 | `refer/DataTable-Libs/table`，独立 Git 仓库；阅读时工作区干净 |
| HEAD | `c14e0221b1e656c7bb35e7b3d4cdf188719a3599` |
| 包版本 | [table-core/package.json](../../refer/DataTable-Libs/table/packages/table-core/package.json) 为 `@tanstack/table-core` **9.3.1** |
| 许可 | [LICENSE](../../refer/DataTable-Libs/table/LICENSE)，MIT，版权行 2016 Tanner Linsley |
| 核心依赖 | `@tanstack/store` `^0.11.2`；本地是 v9，不用 v8 的实现路径和状态合同代替 |
| 仓库规则 | 已阅读根 AGENTS.md 及其要求的 CONTRIBUTING.md；参考仓库只读，没有开展上游功能修改或 PR |

调研主线为 constructTable → 状态所有者／memo → core、filter、sort、expand、page 行模型 → 列大小／固定／拖动 → 范围选择与候选值；核对相应实现测试。没有遍历所有框架适配器、worker 实验包、电子表格范围选择或全部功能。

通过 Context7 检索 `/tanstack/table` 的 v9 受控状态和 memo 资料，返回源码／文档与本地重点一致：v9 采用 per-slice 回调或 external atoms，v8 顶层 `onStateChange` 已移除。在线检索用于交叉理解，不推导本地 HEAD 等于最新发布版。

## 2. Headless 的可取之处是职责边界

证据：[constructTable.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/table/constructTable.ts)、[row-models 指南](../../refer/DataTable-Libs/table/docs/guide/row-models.md)、[virtualization 指南](../../refer/DataTable-Libs/table/docs/framework/react/guide/virtualization.md)。

核心组织列、行模型、状态与逻辑方法，不决定 cell 的视觉渲染。features、row model factories 和函数注册各有明确位置；只启用需要的能力。文档明确 Table 不内建虚拟化 API，Table 提供最终行／列，Virtual 决定滚动范围与测量。

Yue 吸收这一分工：业务记录／异步结果由宿主持有，Table 管列／查询／选择和导航，Base 管滚动／虚拟化，已有 Yue 控件负责输入与外观。不引入 React、TanStack Store 或 TanStack Virtual，也不将 YueTable 拆成另一个通用跨框架 Headless 产品。

按职责组织不等于为每项能力创建公共插件 trait。Rust 方法本身不需要 JS 的 prototype 分配方案；只有实际成本与复用场景成立时才拆内部模块。参考的 tree-shaking／包大小约束不能直接作为原生构建／滚动性能结论。

## 3. 状态所有者：连续操作与无变化更新

证据：[constructTable.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/table/constructTable.ts)、[coreTablesFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/table/coreTablesFeature.utils.ts)、[utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/utils.ts) 的 `setStateSlice`，以及[状态指南](../../refer/DataTable-Libs/table/docs/framework/react/guide/table-state.md)。

v9 为注册状态建立 baseAtoms 与派生 atoms，可从内部状态、外部 atoms、受控 state 读取。`table_publishExternalState` 区分 staged 选项与宿主已经提交的快照；React 等适配器不能把未 commit 的 render 临时值发布成外部事实。

`setStateSlice` 不在 Table 的潜在旧快照上提前执行 updater。它把包装后的 updater 交给 owner，在 owner 实际应用时得到最新 current，再计算 next 并判断无变化。即使 Table 局部看来是 no-op，用户回调仍可能收到 updater，因为排队的 owner 状态未必相同。大量选择集合和高频拖动不走通用深比较路径，源码明确说明其成本取舍。

最有价值的回归例子是：当前受控 sorting 仍显示空，先请求排序，再请求重置；若组件提前判断“空重置为空”而丢掉第二个动作，owner 最终会错误保留排序。

Yue 转化：

- 保持一条 `TableEvent` 请求通道及宿主 accepted view，不复制 atom 图。事件携带 key／目标／必要类型化意图和基础 revision，由宿主针对最新事实接受或拒绝。
- 同步接受本地 UI 状态与异步保存分开；同域 pending 的交互策略明确，不让两份旧目标快照互相覆盖。
- 无变化的接受复用缓存并抑制无效 notify，但仍结算 pending 请求。比较在 owner 处理路径，不能在 render 中深比全量记录／选择。
- 沿用 GPUI 的 update／render 阶段，不引入 React 的 commit hook 或两个镜像事实。

设计落点：第 5.2、19.3 节；M1 状态骨架，M3 查询与选择流程。

## 4. 行模型：显式依赖、稳定身份、缓存边界

证据：[createCoreRowModel.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/row-models/createCoreRowModel.ts)、[coreRowModelsFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/row-models/coreRowModelsFeature.utils.ts)、[coreRowsFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/rows/coreRowsFeature.utils.ts)、[constructColumn.ts](../../refer/DataTable-Libs/table/packages/table-core/src/core/columns/constructColumn.ts)。

核心模型递归创建 rows，维护 flatRows 与 rowsById。`memo` 逐项比较依赖元组引用；core 依赖 data 引用，filter 依赖上游模型和 filter 状态，sort 依赖上游模型和排序状态。结果在输入相同时复用，不逐帧做全量转换。manualFiltering／manualSorting／manualPagination 等在对应阶段返回上游模型。

身份方面，参考未提供 getRowId 时用 index／parent.index，列 id 还可从字段路径替换点号或 header 字符串推导。Yue 不采用这些默认值：显式业务 key、结构化 cell key；标题翻译或数据重排不改变身份。

行 `getValue` 按 column id 惰性缓存 accessor 值，`getAllCells` 用按 Column 实例索引的 WeakMap 复用 cell；后者仍会为该行的全部叶列创建／读取 cell。这些做法依赖各自对象生命周期，不能直接当成双轴 GPUI 工作集方案。

吸收与限制：

- 使用显式数据／结构、列语义、query、几何 revision；原地修改数据也必须推进版本，不能只依赖容器地址。
- 同 key 列更换 accessor／比较器／解析规则时失效相关缓存；只改几何不失效业务查询。依赖读取发生在计算内不等于自动进入 memo 元组，必须逐项设计和测试。
- 结构索引、显示投影、虚拟工作集分别命名；结构中存在的 key 不自动成为当前页显示行。
- Cells 仅按工作列＋必要保留项建立内容，不调用“每行全部 cell”路径；值缓存的查询成本与真实控件内存分开报告，不给全量行列建立 Entity。

这里没有复现并判断参考在动态 accessor 下存在缺陷；结论是借鉴它清晰的缓存结构，同时为 Yue 明确更完整的版本合同。

设计落点：第 5.1、5.4、5.5、7、21 节；M1／M3／M5。

## 5. 过滤、排序与树投影：复用输入，不能复用失效结构

证据：[createFilteredRowModel.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-filtering/createFilteredRowModel.ts)、[filterRowsUtils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-filtering/filterRowsUtils.ts)、[createSortedRowModel.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/row-sorting/createSortedRowModel.ts)。

排序先解析有效排序条目及比较器，多字段逐级比较，同值以原 index 保持稳定；undefined 的 first／last 与随方向变化的数值策略分别处理。同级递归排序，只有子树顺序变化的分支才需要新的结构对象，不修改上游父子顺序。复制分支时明确排除 `_memo_*` 和 `_cellsCache`，避免克隆继承仍引用原分支的派生方法缓存。

过滤有 root-first 与 leaf-first 路径，并处理深度边界；“父不匹配但叶匹配是否保留祖先”是可观察语义，不能靠泛化递归偶然决定。过滤／排序的 flatRows 需保持先父后子、唯一且对应本阶段结构，不只看最终文本顺序。

Yue 保持既定规则：树表过滤保留匹配节点和祖先路径，同级稳定排序，不增加按字段分组产品范围。未变业务 Row 与结构可共享；本阶段顺序／反查／索引只指向本阶段结果。空值、零、false 与比较方向按明确类型规则验收，不照搬 JS 隐式类型转换。

设计落点：第 5.5、11、13、22 节；M3／M4。

## 6. 分页与展开：不能把 flatRows 当屏幕行

证据：[createExpandedRowModel.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/row-expanding/createExpandedRowModel.ts)、[createPaginatedRowModel.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/row-pagination/createPaginatedRowModel.ts)。

通常顺序为 core → filter → group → sort → expand → page。默认 paginateExpandedRows 为 true，展开子行占 pageSize，可被切到不同页；设为 false 时延迟展开，在分页的根／未展开序列后附加子行。manualPagination 路径还需考虑展开由谁完成，不能仅看一张流水线箭头图。

参考 paginated 模型的 rowsById 透传分页前索引，flatRows 重建会遍历页内父节点的子孙，即使子孙折叠也可能存在；seen id 集合用于防止子孙同时经 inline rows 和 subRows 重复加入。其测试明确固定了这些行为。

Yue 不用这些集合名称推导 UI：根分页总数、当页展开后的显示行数、结构节点数和虚拟工作集各有独立口径。保持根分页、子孙跟随根，折叠子孙不进入渲染／UIA 数量／页内全选。显示 key 和反查一起接受，唯一且先父后子。Remote 结果不重复分页，不开放一组 manual bool 让同一操作同时由两方执行。

设计落点：第 5.5、11、12、15、18 节；M3／M4。

## 7. 列几何与拖动：一次累计，处理最后一个坐标

证据：[columnSizingFeature.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-sizing/columnSizingFeature.ts)、[columnSizingFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-sizing/columnSizingFeature.utils.ts)、[columnOrderingFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-ordering/columnOrderingFeature.utils.ts)、[orderByColumnPinning.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-pinning/orderByColumnPinning.ts)、[columnResizingFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-resizing/columnResizingFeature.utils.ts)。

列 committed size 与 transient resizing 分离。getColumnOffsets 在 Table 级缓存全体／start／center／end 区域的前后偏移，getStart／getAfter 做 O(1) 查询；尺寸、可见性、顺序与 pin 等输入共同使其失效。header 组宽来自叶列之和。列显示、header 和 row cells 的排序依赖须一致。

累计偏移本身是线性循环；不可据此称所有顺序处理都线性：pin 归并 helper 仍有对 start／end 的 includes 查找，成本随 pinned 数量变化。Yue 在一次列快照解析中建立 key 查找／成员集合，继续复用 Base 轨道，不在每行重复累计或搜索。

拖动按绝对起点计算，保存瞬时 delta，支持 onChange／onEnd。move 首次及时更新，后续合并到 requestAnimationFrame；正常 up 取消尚未执行的帧，以释放坐标最终提交，并合批更新拖动态与尺寸。还覆盖零起始宽度避免除零。

Yue 采用 GPUI 帧调度、起始几何＋绝对位移、末次坐标结算和待执行工作清理。保持既定“预览＋释放一次请求”；不增加另一个持久宽度事实。取消必须撤销，而参考 touchcancel 调用 onEnd 属于结束提交路径，不照搬到 Escape／失捕获。参考提供的是逻辑尺寸和顺序，不能替代本项目末端冻结的裁剪、绘制、命中、焦点或 UIA 实现。

设计落点：第 7、8、22 节；M2。

## 8. 选择与筛选候选：来源范围必须说清楚

证据：[rowSelectionFeature.utils.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/row-selection/rowSelectionFeature.utils.ts)、[createFacetedRowModel.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-faceting/createFacetedRowModel.ts)、[createFacetedUniqueValues.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-faceting/createFacetedUniqueValues.ts)、[createFacetedMinMaxValues.ts](../../refer/DataTable-Libs/table/packages/table-core/src/features/column-faceting/createFacetedMinMaxValues.ts)。

范围选择在最新 logical display order 中反查两个端点，检查索引仍对应正确 row，跳过不可选节点，并用一次 selection updater 修改整个区间。普通 click 不解析全部显示顺序。select-all 与 select-page 使用不同来源，selected key 与从已加载模型投影得到的 selected records 也不同。

参考范围可跨 client-side pages，默认 child recursion 也可选中折叠子孙。Yue 只学习端点校验、一次请求和普通 click 的低成本；初期范围限当前页已展开投影，父子默认独立，跨页保留不等于跨页范围，服务端全部结果选择仍需独立域协议。

Faceted row model 在计算本列候选时排除本列筛选，保留其他筛选，从而避免候选被当前选择永久收窄；unique／minmax 派生结果按输入缓存。它不是随便从可见工作集取 distinct，也不能从远程一页得出全域选项。

Yue 补全既有筛选 panel 的候选来源合同：固定枚举、已加载、其他条件过滤后的结果域或服务端选项；必要时由宿主提供候选／计数。不要求新建 faceting 引擎或全局搜索功能。参考 minmax 使用 Number 转换，会把 null 转为 0；Yue 不继承此隐式规则。

设计落点：第 12、13.2、22 节；M3，树范围在 M4。

## 9. 已阅读的测试逻辑与 Yue 验收

以下为定向阅读的测试与断言，不代表本轮执行通过；涉及参考有而 Yue 没有的 grouping 等能力时，只借鉴投影一致性的测试方法，不扩大产品范围。

| 参考测试 | 有价值的断言 | Yue 对应 |
| --- | --- | --- |
| [setStateSlice.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/unit/setStateSlice.test.ts) | updater 仅由 owner 执行一次；结构 no-op 保留引用；真实更新后接局部看来无变化的 reset 仍正确 | 同一旧快照下连续设置／重置／选择不丢失；接受路径去重，确认仍结算 |
| [renderPhaseReactivity.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/unit/core/renderPhaseReactivity.test.ts) | 只按显式 commit 发布；推测性读取不推进通知比较基线 | GPUI render 不发布业务值；失败提交不让部分来源／query／轨道可见 |
| [createCoreRowModel.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/core/row-models/createCoreRowModel.test.ts) | 重复调用保留模型；data 引用改变后重算，无关呈现选项不重算 | 换数据 revision 必重算，视觉变化不重跑查询；额外测试原地记录更新与列语义变更 |
| [createSortedRowModel.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/features/row-sorting/createSortedRowModel.test.ts) | 仅变化分支需要克隆；不继承旧 memo／cell cache；同值稳定、多列、undefined 策略 | 同级排序不改源结构；用热缓存后重排，检查值、反查、次序及焦点 |
| [createFilteredRowModel.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/features/column-filtering/createFilteredRowModel.test.ts) | 匹配叶子时祖先保留、结构顺序／索引一致 | 树表采用既定保留路径规则，过滤隐藏节点不泄入显示投影 |
| [rowModelFlatRowsOrder.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/core/row-models/rowModelFlatRowsOrder.test.ts) | 多级 pipeline 中逐阶段验证 preorder 和 key 唯一 | 不只检查最终 rows 文本；每个会改变结构的阶段检查不变量 |
| [createPaginatedRowModel.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/features/row-pagination/createPaginatedRowModel.test.ts) | 两种展开分页口径、flatRows 去重、rowsById 可含其他页 | Yue 固定根分页；结构索引与当前显示／选择域分别断言 |
| [columnSizingFeature.utils.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/unit/features/column-sizing/columnSizingFeature.utils.test.ts) | 先热缓存，再改变列顺序与状态，header／cell／offset 同步更新 | 宽度、隐藏、pin、重排逐项失效；四区总宽和同列边界一致 |
| [columnResizingFeature.utils.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/unit/features/column-resizing/columnResizingFeature.utils.test.ts) | 多次 move 合帧；up 取消待执行帧，最终坐标仍提交；状态批次仅一次通知；零宽不 NaN | GPUI 事件注入和帧推进；最终位置不丢、旧帧不回写、取消零提交 |
| [rowSelectionRange.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/features/row-selection/rowSelectionRange.test.ts) | 普通 click 不解析显示序列，范围一次更新；变排序／过滤／锚点删除后正确处理 | 一次 SelectionRequested；当前页端点与 disabled／折叠规则；不复制跨页默认值 |
| [createFacetedRowModels.test.ts](../../refer/DataTable-Libs/table/packages/table-core/tests/implementation/features/column-faceting/createFacetedRowModels.test.ts) | 本列候选排除自身条件、保留其他条件；空集、unique 计数、空值转换 | 仅在宿主提供联动候选时验收来源和完整性；空值不隐式变为 0 |

测试不搬 React atom 通知次数、DOM 类名或 JS prototype 等实现细节。Yue 验证真实 accepted view、阶段执行计数、GPUI bounds／scene、焦点 Entity 与业务事件；性能必须另做相同 workload 的前后采样。

## 10. 吸收清单与下一项

| 编号 | 吸收决定 | 阶段 |
| --- | --- | --- |
| TS01 | 最新 owner 上应用类型化意图；无变化接受不产生额外失效 | M1／M3 |
| TS02 | 数据／列语义／query／几何分别版本化，热缓存后测失效 | M1／M2／M3 |
| TS03 | 结构索引、显示投影、工作集分开；先父后子且显示 key 唯一 | M1／M3／M4 |
| TS04 | typed 数据列与展示／组列分开；显式 key 不取标题／index | M1 |
| TS05 | 共享四区列偏移／索引；查找 O(1)，归并避免重复扫描 | M2 |
| TS06 | 起始绝对位移、帧合批、末次释放坐标、取消待执行任务 | M2 |
| TS07 | 默认根分页／当页范围／独立父子选择，明确偏离参考默认值 | M3／M4 |
| TS08 | 一次范围请求；selected key 与已加载记录子集分开 | M3／M4 |
| TS09 | 联动筛选候选来源、完整性与自身条件排除合同 | M3 宿主组合 |
| TS10 | 业务值与 UI Entity 缓存分别记账，Cells 不先创建全列内容 | M1／M5 |

不增加动态插件、worker 平台、跨框架适配器、按字段分组、电子表格选择／填充柄等新范围。复用既有 Base 轨道／虚拟化、YuePagination、Tree 结构、Input／Form 及筛选浮层，保持 M1–M6 的产品边界。

下一项为 `refer/DataTable-Libs/glide-data-grid`（Glide Data Grid），重点应核对惰性取值、绘制与损伤更新、缓存／资源生命周期和编辑覆层。该项尚未开始，不能据 Canvas 百万行宣传为 Yue 承诺性能或引入第二套渲染器。

后续进展：第三项已完成，见 [Glide Data Grid 专项记录](2026-11-10-table-glide-review.md)；上段保留为本轮结束时的调研边界。
