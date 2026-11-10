# YueTable M6：重排、列配置与完整接入

日期：2026-11-11。当前公开 API；自动化和性能结果在实施进度及证据报告中单列。

## 行列重排

默认关闭。行手柄使用显式 `TableColumn::row_drag_handle(true)` 列，表格启用 `row_drag(TableRowDrag::Handle)`。`Row` 是整行拖动的 opt-in：只从普通非编辑文本启动，不劫持 Checkbox、Button、Input 或编辑器。列手柄由 `.reorderable(true)` 开启，与排序按钮和伸缩手柄分别命中；复用 YueButton 和 Tabler GripVertical。

指针移动超过4px才进入拖动，插入线只是预览，不发布数据。松手重新使用最后绝对坐标命中，至多发一个请求。Escape、窗口失活、丢失按键、卸载、source／query／列布局变化取消；键盘手柄失焦也取消。边缘24px区域通过原 ScrollHandle 自动滚动，不创建第二 owner；每次只留一个带会话序号和 WeakEntity 的后续帧回调，结束后失效。

聚焦手柄后，空格开始、方向键选目标、Enter／空格确认，Escape取消。确认使用按钮的原生 key-up，不在 key-down 后又由 click 启动第二次会话。行键盘目标从 source 当前索引逐步查找，不为一次按键分配全表 key 数组。有限视口下，行目标通过原虚拟轴定位，列目标按共享列轨道／固定区余量滚入视野；不移动键盘焦点或提前提交。Static的外层页面滚动仍由宿主持有。Static 使用实际自然行 bounds；Rows／Cells 使用同一几何索引。离屏源只保留稳定 key 和结算元数据，不保留整行控件或扩大双轴工作集。

`TableEvent::RowMoveRequested`／`ColumnMoveRequested` 携带 owner 票据、scope、view revision、稳定源 key、目标 key 和 `TableMovePlacement::{Before,After}`；行请求还携带 source revision。无变化的位置不发请求。宿主接受后才改变顺序：

- `TableRowMoveRequest::apply_to_rows(&view)` 创建新的 TableRows 索引，复用原 `Rc<[R]>`，不要求 R: Clone，也不保留旧 source 链；自定义来源使用 `ordered_keys(&view)` 映射业务存储。
- `TableColumnMoveRequest::apply_to(&view)` 更新列偏好顺序，保留原始声明。
- 最终 `commit_view(next, Some(request.id()), cx)` 重验；拒绝调用 `reject_request`。未结算请求有64项上限。

默认拒绝排序激活、树／范围占位／详情来源的平面行重排。自定义 source 需显式实现 `can_move_rows`。过滤或分页还需 `row_move_scope(TableRowMoveScope::MappedVisibleResult)`，由宿主映射结果域；不声称修改了后台全局顺序。禁用行、详情、占位及首尾冻结数据行不能作为源或目标。列移动限制在同一完整分组路径及pin区，不能穿过受保护列；跨pin的调整使用列设置请求。

## 列配置与偏好

`TableView` 分别保存 `declared_columns()`、`column_preferences()` 和有效 `columns()`。原声明不被拖动宽度或临时窄容器覆盖。`TableColumnPreferences` 是宿主可读写／可持久化的值模型；组件不决定配置文件位置或序列化框架。

偏好按稳定 column key 保存可选 visible、pin、原始 Rems width 和 order，并带 schema 及每列 semantic version。`TableColumn::semantic_version(u64)` 表示同名列的业务意义版本；与 typed query 的规则身份不同。`reconcile(&new_view)` 清掉删除／失权／语义变化列的旧设置，补新声明并固定受保护位置。`apply_to` 严格拒绝未协调的旧schema、非法key、无效宽度和越权请求。

`allowed(false)` 与 `visible(false)` 分开：前者禁止通过偏好重新显示，也不能用于查询；后者只是显示偏好。`configurable(false)` 列继承声明，不能被配置移动、隐藏、固定或改宽度。权限仍由业务宿主决定，不能把展示层当作数据访问控制。

对话框使用独立 `TableColumnDraft::new(&view)`。`set_visible`、`set_pin`、`set_width`、`set_order` 只改变草稿，`reset()` 清除偏好、恢复声明。确认调用 `request_column_configuration(&draft, cx)`，重验scope、schema、列集合、语义版本和权限后只发一个 `ColumnConfigurationRequested`。宿主用 `request.apply_to(&latest_view)` 和一次 commit 接受；取消不发请求。

Showcase 复用 YueDialog、YueCheckbox、YueInput、YueButton；未修改的宽度输入不会把Flex列变成固定宽度。刷新查询／分页／批量操作后，宿主用旧偏好的 `reconcile(&next).apply_to(&next)` 恢复设置；不能仅把当前有效 columns 当作新声明。配置持久化可使用 getter／builder 接入现有业务设置服务。

## 尾游标加载

`tail_loading(cursor, distance: Rems, has_more)` 与已知范围 `range_loading(prefetch_rows)`、已知总数分页互斥。range与分页也互斥；两种预取都要求Rows／Cells真实虚拟视口，Static明确拒绝。空白cursor拒绝；高度、行高和预取距离的非有限／非法数值由数值builder前置断言拒绝。触发基于真实viewport到内容末端的距离，不比较近似行数；空结果可引导首次请求。首尾冻结和固定footer参与可用空间计算。

事件继续复用 `RangeRequested`，`request.tail_cursor()` 区分未知尾游标；此时 `range()` 为旧长度的空区间。相同cursor去重，每表至多一个加载请求。宿主异步完成后调用 `commit_range`，候选必须保持scope／query，长度增加且原前缀key顺序不变。cursor由宿主在接受追加后推进，has_more=false停止自动请求。未推进cursor不会重复追加；新内容远离末端也不会循环请求。

`reject_range` 保留旧行并停止自动重试，`retry_range` 生成新票据。旧成功／失败不能清除当前请求；换query／scope、外部替换source、布局取消或卸载使旧请求失效。宿主负责取消网络任务，示例以完成／失败按钮模拟真实异步边界，不冒称连接网络。

## 自然高度合并与最终校验

Static＋rowspan现在使用有界自然轨道布局：每个origin只构建一次，按共享列宽测量内容，先取单行最大高度，再将跨行内容不足的高度分配给所跨行。轨道按设备像素统一取整，全部同排行相邻格伸展居中；covered格不构建、不命中、不重复语义。长内容真实撑开布局，不切换到Cells，也不估算离屏高度。Static有意构建全部origin，适合确实有界的短表。

Static不接受冻结数据行或固定列footer（自然表头／footer随内容）；需冻结时选择Rows／Cells。Rows仅接受单行colspan；Cells接受Fixed／Known的rowspan，Measured／全宽详情继续使用Rows。`TableSpanPlan`只描述正文：分组表头自行计算跨度；表外完整总结通过footer slot组合。正文计划拒绝占位／详情、越界、重叠、responsive列或跨pin／冻结边界。跨pin分组表头的唯一语义片段报告完整逻辑组跨度。

`YueTable::validate(cx)` 验证完整builder组合；`capabilities(cx)` 返回只读 `TableCapabilities`，包含模式、rowspan、Measured、行列重排、编辑及宿主滚动能力。组合校验失败不会发布半份presentation或修改accepted数据，呈现明确配置错误；宿主可在挂载前用Result处理。`try_render_mode` 仍可单独检验模式，但最终应校验完整builder。

## 示例与验收入口

- 重排／完整列配置／尾部追加：[customization.rs](../examples/windows-showcase/src/pages/table/customization.rs)、[configuration.rs](../examples/windows-showcase/src/pages/table/configuration.rs)。
- 综合工单：查询、分页、跨页选择、批量核验、列设置与真实记录详情／编辑同用；负责人保存后重新执行当前query，保留selection和偏好：[query.rs](../examples/windows-showcase/src/pages/table/query.rs)。
- 日期／时间行与单元格编辑：复用DatePicker／TimePicker；保存先flush实际输入，再在picker变化事件结算后创建编辑token，校验失败保留草稿、可重试：[temporal.rs](../examples/windows-showcase/src/pages/table/temporal.rs)。已有Input／Select、Form／Field路径继续保留。
- 自然高度合并、双轴、宿主双表与范围加载：[advanced.rs](../examples/windows-showcase/src/pages/table/advanced.rs)。所有大型例子按需挂载。

自动化、固定轨迹、性能证据与每项功能映射见[最终交付核对](research/2026-11-11-table-completion.md)。真实IME、Narrator、GPU、当前显示器滚动手感及跨屏DPI保持独立验收边界，不用headless测试代替。
