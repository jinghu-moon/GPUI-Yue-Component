# Table 源码调研 07：Material React Table

- 日期：2026-11-10；对应[Table 设计](../45-table.md)，接续 [react-data-grid 调研](2026-11-10-table-react-data-grid-review.md)。
- 本地目录：`refer/DataTable-Libs/material-react-table`；HEAD：`60080592eaefd57f53abec383f449fa7d59168a0`。
- 包版本 **3.2.1**；根及包内 LICENSE 为 MIT。版本依据是本地 package.json，不使用介绍文档中的版本替代快照。
- 本轮是定向源码、文档和示例阅读，没有安装依赖、运行 Storybook／参考测试、测量性能或实现 YueTable。

## 1. 先区分底层与组合层

[package.json](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/package.json) 声明 `@tanstack/react-table` **8.20.6**、`@tanstack/react-virtual` **3.11.2**，并依赖 match-sorter-utils 和 highlight-words；MUI 6、Date Pickers 7 等为 peer 依赖。它把表格模型、虚拟化、MUI 控件、工具栏和编辑流程组合成完整产品，不是另一套独立的底层虚拟化引擎。

此前独立调研的 `refer/DataTable-Libs/table` 是 TanStack Table **9.3.1**，与此处 V8 不同。MRT 的封装代码、初始化规则及 V8 API 不直接代表 V9 行为。

[useMRT_TableOptions.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_TableOptions.ts) 接入 core／filter／sort／group／expanded／pagination 模型；展示列默认禁用数据列能力。虚拟化开关在初始化时固定，并会调整 sticky header 和布局模式；无数据时还会自动开启若干 manual 模式。

Yue 吸收展示列与数据列能力分离、已有部件组合的方向，继续使用 Base-only 和显式 Local／Remote／Rows／Cells 合同。空结果不改变查询所有权，不照搬初始化后冻结的模式、Web 依赖或大量可写布尔开关。分组聚合、导出和全屏不因此进入新范围。

## 2. 列配置按 key 协调，批量操作明确范围

证据：[displayColumn.utils.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/utils/displayColumn.utils.ts)、[useMRT_TableInstance.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_TableInstance.ts)、[useMRT_Effects.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_Effects.ts)。

MRT 按启用能力插入具名展示列，并将已有 columnOrder 与当前叶列 key 归并；但 effect 自动修复顺序的入口主要看列数量，拖动／伸缩期间也会复用旧列定义。Yue 应保留稳定描述与避免热路径重建，不能只比较数量或因会话冻结而忽略外部合法修订。

Yue 在列配置 revision 改变时按当前 key 集合、能力和权限协调：同数量 A→B 换列也要识别，保留存续列的用户顺序，清理删除列，按声明规则插入新列。选择／展开／操作列的增减也参加同一次协调，不另维护隐式轨道。

证据：[MRT_ShowHideColumnsMenu.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/menus/MRT_ShowHideColumnsMenu.tsx)、[MRT_ShowHideColumnsMenuItems.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/menus/MRT_ShowHideColumnsMenuItems.tsx)。

MRT 的全部显示／隐藏只修改允许隐藏的叶列，列顺序重置与取消固定分开；分组菜单递归组织子列。Yue 用已有 Checkbox／Dialog／Dropdown：

- 批量操作仅作用于有权限且允许修改的叶列，保留必显列；分组勾选按同一可操作范围派生全选／半选。半选是 Yue 需要补齐的合同，不宣称参考已完善实现。
- “重置顺序”“重置宽度”“取消固定”各有明确作用范围，不用一个含糊的重置抹掉其他偏好。
- 面板打开期间外部更新列或权限，草稿确认前按新 revision 重验；不能复活已删列或失权列。有冲突时留在面板提示，取消不回写。
- 确认只发一次完整 `ColumnsRequested`。参考的拖列随后单独设置 pin 不能照搬成两个可被宿主分别接受的请求。

配置持久化仍归宿主，不新增通用设置平台。

## 3. 筛选算子与有效值是同一次意图

证据：[MRT_FilterOptionMenu.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/menus/MRT_FilterOptionMenu.tsx)、[MRT_FilterTextField.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/inputs/MRT_FilterTextField.tsx) 的输入初始化部分。

MRT 根据单值、多值、区间和无值算子切换输入及筛选值；无值条件使用空格字符串哨兵，部分初始化依赖 truthiness。其优势是控件随条件变化，不要求用户重新配置整个列；Yue 不移植哨兵或多 setter 的中间状态。

已有 typed 筛选支持切换算子时，算子、合法载荷或清除值、分页策略形成一次完整 `QueryRequested`。无值算子用类型表达，0／false 与空值不同；不兼容旧值先清除或转换为合法草稿，不能拿旧 scalar 执行新 range。候选来源和解析规则变化照常使缓存失效，不因稳定函数引用永远使用旧候选。

[useMRT_Effects.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_Effects.ts) 会在 ranked global filter 时清除／恢复排序；[row.utils.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/utils/row.utils.ts) 中排名路径对取得的行数组排序。Yue 保持已接受排序与上游投影不变，不为复刻搜索体验暗改排序，也不新增 fuzzy ranking 引擎。

## 4. 缓存不能以冻结交互换性能

证据：[MRT_TableBody.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/body/MRT_TableBody.tsx)、[MRT_TableBodyRow.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/body/MRT_TableBodyRow.tsx)、[MRT_TableBodyCell.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/body/MRT_TableBodyCell.tsx) 的 memo 比较器，以及 [memoization.mdx](../../refer/DataTable-Libs/material-react-table/apps/material-react-table-docs/pages/docs/guides/memoization.mdx)。

参考分别用 data、row／index、cell 引用作为主要比较条件。文档明确警告：memoMode 为 cells 会影响密度切换，table-body 会影响多数功能，包括虚拟化；也不推荐一开始给所有 renderer 包 useCallback。

Yue 不新增类似的危险公开优化档。业务内容未变并不表示元素无需更新：主题／rem／密度、选择／busy、编辑状态、列几何和语义变化仍须到达相应控件。优化只缩小实际无关的失效，不能令表头已变而正文停在旧状态。现有第 21 节的 factory／Entity Render／layout／paint／分配观测继续分开，不把 React 引用比较直接翻译成 Rust 性能结论。

MRT 的密度变化后用 150ms 定时重绘修复 pinned 行位置，不是 Yue 的布局收敛方案；共享几何应在同一布局／尺寸更新合同中完成。

## 5. 虚拟拖动、详情与加载行必须保留真实身份

证据：[virtualization.utils.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/utils/virtualization.utils.ts)、[useMRT_RowVirtualizer.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_RowVirtualizer.ts)、[useMRT_ColumnVirtualizer.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_ColumnVirtualizer.ts)。

MRT 将拖动源索引加入虚拟范围，避免源元素被移除后 dragEnd 无法触发；行源 key 会回查过滤后的行。列虚拟范围也加入固定列并去重。这说明工作集切换不能破坏交互结算，但不意味着 GPUI 需要保持整行／整列的所有格子。

Yue 复用 GPUI 捕获／会话，以 stable key 映射最新的显示投影和 RowKind；仅保留完成／取消所需的源控制、预览或精确项。源离屏后松手／Escape／失捕获仍结算一次，回到正常工作集去重。来源被过滤／删除、query 改变则按既有规则取消，不把旧位置重新解释成另一条记录。列保留也随最新可见列／pin 配置重验，不只依赖拖动源 key 没变。

证据：[MRT_TableDetailPanel.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/body/MRT_TableDetailPanel.tsx)。参考启用详情后虚拟 count 为业务行数乘二，详情用奇数槽；非加载时会先调用 renderDetailPanel，再按展开决定显示。虚拟路径没有使用非虚拟路径的 Collapse。

Yue 沿用只为实际展开项插入具名 detail 行的投影，不采用 `index * 2 + 1` 恒定公式，不为折叠行保留永久零高槽或启动详情任务。详情内容在展开且需要时创建；折叠／重开缓存及任务沿既有 owner／代次规则。虚拟展开动效仍是 Yue 待实现／验证的尺寸反馈合同，不能称从参考取得了已验证动画方案。

[useMRT_TableInstance.ts](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/hooks/useMRT_TableInstance.ts) 在空数据加载时生成少量字段为 null 的伪记录。Yue 的 Skeleton／loading 使用独立展示身份，绝不交给业务 accessor、查询、选择、编辑或 summary；不能因暂无数据自动改变 Local／Remote 模式。

## 6. 编辑表单组合已有控件，权限不等于列可见性

证据：[MRT_EditActionButtons.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/buttons/MRT_EditActionButtons.tsx)、[MRT_EditCellTextField.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/inputs/MRT_EditCellTextField.tsx)、[MRT_EditRowModal.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/modals/MRT_EditRowModal.tsx)、[editing-crud-row 示例的保存片段](../../refer/DataTable-Libs/material-react-table/apps/material-react-table-docs/examples/editing-crud-row/sandbox/src/TS.tsx)。

MRT 的保存回调由宿主校验与更新，宿主显式退出编辑；示例校验失败时保留编辑，保存成功后退出。行 Dialog 组合 TextField 和保存／取消，允许自定义内容。Yue 借鉴完整消费示例，使用已有 Dialog＋Field／Form＋Input／Select／DatePicker／TimePicker，不额外建立 TableEditForm、平行草稿引擎或提前创建所有 cell 输入。

编辑记录的字段由宿主明确选择、校验权限；当前隐藏列不自动排除合法编辑字段，可见列也不自动获得编辑权。真实控件 Entity 保有 typed 草稿，沿第 14 节的 session／数据／draft revision、IME 和显式保存流程。不能借业务取值缓存存草稿，也不通过拼接或拆分 input name 反查身份；参考默认 blur 写缓存并退出的方式不改变 Yue 默认失焦不保存的规则。

## 7. 工具区、选择计数与远程示例的边界

证据：[MRT_TopToolbar.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/toolbar/MRT_TopToolbar.tsx)、[MRT_ToolbarInternalButtons.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/toolbar/MRT_ToolbarInternalButtons.tsx)、[MRT_ToolbarAlertBanner.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/src/components/toolbar/MRT_ToolbarAlertBanner.tsx)。

业务操作与表格工具分区、只展示有效能力值得学习；MRT 的 window 媒体查询、绝对定位和补偿 margin 不适合直接移植。Yue 按真实容器宽度换行／收纳菜单，关键操作始终有入口，复用 Button／Dropdown／Tag／Text。不因参考有按钮就默认新增全屏、搜索等公共开关。

选择计数明确范围，例如“已选 12 项，其中当前页 3 项”；跨页 key 计数与本页已加载记录数分别读取，查询域选择沿协议表达，不能用后者冒充总选择。busy、失败和选择状态分别呈现其语义，不一律塞入警告条。

[remote 示例](../../refer/DataTable-Libs/material-react-table/apps/material-react-table-docs/examples/remote/sandbox/src/TS.tsx) 展示 manualFiltering／Sorting／Pagination、rowCount，以及首次 loading 与 refetch 的区分。但该示例 fetch effect 未见请求代次隔离，catch 提前 return 跳过后方 loading 清理；它是接线示例，不作为生产异步正确性模板。Yue 继续由宿主在全部完成路径校验代次并结算忙碌态，结果／总数同批接受，分页直接复用 YuePagination。

## 8. 证据等级与应补的回归

本地快照按 `.test.*`／`.spec.*`、`__tests__`／cypress／playwright 路径检索未找到独立测试文件；检出 **54 个 `.stories.tsx`**。组件包 TypeScript 中检索 expect／describe／it／test 调用和 `play:` 未找到匹配，包脚本也没有 test 入口。这限定于本次检索范围，不代表上游线上没有其他验证。

实际阅读 [Memod.stories.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/stories/features/Memod.stories.tsx) 与 [dragging-virtual-when-filtered.stories.tsx](../../refer/DataTable-Libs/material-react-table/packages/material-react-table/stories/fixed-bugs/dragging-virtual-when-filtered.stories.tsx)：它们提供 memo 档位、过滤＋虚拟拖动的复现配置，未见独立断言。本轮没有运行这些场景，不能写成参考测试通过。

| 编号 | 转成 Yue 的设计／验收合同 | 阶段 |
| --- | --- | --- |
| MRT01 | 同数量换列 key、展示列增减、面板打开时删列／改权限；确认不得复活旧列，取消无回写 | M2／M6 |
| MRT02 | 必显列不受批量隐藏影响；分组半选与操作范围一致；各类重置互不误清；确认一次完整请求 | M2／M6 |
| MRT03 | scalar／list／range／无值算子切换仅一次查询，0／false 保真，分页同步且排序不暗改 | M3 |
| MRT04 | 同一业务数据版本下切换主题／rem／密度／selection／busy／几何／语义仍正确；优化前后联合核对性能与交互 | M1–M5 |
| MRT05 | 首次加载不调用伪业务 accessor、不参与选择／查询／summary；折叠 detail 不构建内容或启动任务 | M1／M4 |
| MRT06 | 过滤结果＋详情投影下拖动源离屏、回窗去重、末次释放／Escape；来源失效取消，工作集仍有界 | M4／M5／M6 |
| MRT07 | 宿主记录编辑 Dialog 的 typed 控件、隐藏／可见字段权限、校验失败、保存失败／取消及迟到结果 | M4 |
| MRT08 | 窄容器工具区仍可操作、跨页选择计数范围正确；远程失败也结算 busy，旧响应不覆盖新结果 | M1／M3 |

测试须落到 GPUI 输入、状态、bounds 和绘制，不能只统计回调或 factory。以上均为待实现的验收项，不是已经通过的测试。

## 9. 吸收范围与下一项

本轮补全已有 M1–M6 的列配置、筛选、编辑组合、虚拟交互、状态展示与性能正确性合同；没有增加 React／MUI 依赖、groupBy／导出引擎、公共 memoMode、定时几何修复或存盘平台。

下一项为 `refer/DataTable-Libs/gridjs`。先核对本地版本、许可和真实流水线，再区分数据处理／异步／渲染能力，避免把轻量表格与虚拟化 Data Grid 混为一谈。本轮尚未开展该项调研。

后续进展：第 8 项已完成，见 [Grid.js 6.2.0 专项记录](2026-11-10-table-gridjs-review.md)。上段“尚未开展”记录该轮结束时的范围，当前总进度以主设计第 2.2 节为准。
