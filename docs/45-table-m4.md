# YueTable M4：复杂记录与真实编辑

本阶段在 Rows 中提供 Fixed／Known／Measured、独立详情、Base keyed tree 投影和受控编辑会话。使用现有虚拟 Grid 的 Known 轴更新自然行高；不增加第二套滚动 owner，也不宣称 Cells 支持自然测量。

`TableRowSizing::known` 接受稳定 row key 与正有限 rem 高度，必须覆盖当前显示投影；发布后不可修改。`measured(estimate)` 以合法估计启动，只构建工作行的所有可显示列，按整行最大自然内容高合批反馈。列／字体／rem／宽度／密度及 source row revision 隔离旧反馈，零测量不覆盖有效尺寸。尺寸缓存最多 4096 项；Base Known 几何仍是 O(N)，不是整个表只有 4096 项内存。Base 保留冻结表头下有效正文边缘的 key 与像素阅读位置。

`TableSource::row_revision` 默认使用 source revision，精确更新的宿主可按记录覆盖。更换内容必须发布新不可变 source。列语义或渲染规则改变应发布新的列声明快照；颜色主题不改变文字几何时无须重建行高。

## 结构投影

`TableOutline::new(source)` 不复制 Row，保存 Base KeyedTreeModel 与独立 detail keys。`set_tree` 验证业务记录存在；`toggle_tree`、`filter_tree` 复用 Base 展开与保祖先路径，`sort_siblings` 稳定排序各级兄弟，保持子树连续。`snapshot(revision, page)` 对根切页，root_total／page_len 与显示行数／业务记录数分别计数。

`set_details([(record_key, detail_key)])` 要求显式稳定且与业务域不冲突的 key，`toggle_detail` 独立于树展开。仅打开的详情进入投影，只有工作集内详情调用 `TableDelegate::render_detail`；详情不参与选择或普通单元格导航。Showcase 复用 Descriptions，并为每个打开的嵌套表保留独立状态与滚动 owner，关闭释放该实例。

`request_children`、`apply_children`、`reject_children` 使用 Base owner／代次票据。过期成功在转换及校验前拒绝，当前失败可重试；新 source 需要同 scope 和递增 revision，来源缺记录或非法子树不发布。

默认选择独立。创建 outline 后、set_tree 前调用 `with_check_mode(TreeCheckMode::Cascade)`，再由宿主将请求交给 `apply_selection(request, accepted_view)` 才开启级联。Base 的加载不完整与 disabled 边界仍然有效，不把未加载子树标成全选；范围操作以显示范围的叶项为输入。禁用边界应在 TreeNode 与 Table disabled_rows 一致声明，拒绝不会发布半套选择。跨页／折叠仍保留业务 key。

## 编辑会话

列 `.editable(true)` 与导航控件 `.interactive(true)` 分开；默认双击、F2、Enter 只发送 `EditRequested`。宿主先初始化真实 retained Input／Select 等控件，然后 `start_edit(key, Cell/Row, window, cx)`，注册实际 focus handles。Table 不持有一份文本草稿，不替代编辑器的 IME 与选区。

宿主观察实际控件值，并在内容改变时 `edit_draft_changed`；立即保存／Tab 应先读取并比较所有真实值，不能等待可能迟到的 Change 事件。`request_edit_save_with_move` 每次创建唯一尝试版本，依次 Validating→Saving→commit_edit；验证／保存失败调用 reject_edit，保留输入和错误。重试未改变的文本也使用新 token，旧成功／失败不能结束新会话。

`commit_edit` 在唯一接受入口校验完整候选；保存目标缺失、源错误或列非法时保留旧 view 和会话。成功最多接受一次，Next／Previous 请求下一可编辑格（行模式跳下一行）。隐藏字段／修改查询前须结束会话；同 key 的外部记录或规则修订进入 Conflict 并保留草稿，删除／换 scope／卸载取消而不隐式保存。编辑行通过 Grid 稀疏保留，未把源与视口间所有行都挂载。

示例 `pages/table/editing.rs` 使用 YueInput、YueSelect、YueForm、YueField。行内 Tab 先在当前控件间移动，末端保存后导航；IME 标记文本和打开的 Select 优先。Enter／Escape 不抢实际编辑器先处理的动作。真实 Windows 输入法仍由用户验收，不以 headless 标记文本或键盘模拟冒充原生 IME。

功能测试覆盖 TC01／02／04／08／10／13／14／17：Known 重排及原子拒绝、自然换行与同行居中、宽度恢复、深处内容修订锚点、稳定尺寸、根分页与同级排序、节点失败重试／过期、详情零构建与释放、真实输入立即 Enter／Tab、行多字段／离屏输入、异步倒序与失败重试、冲突与删行焦点修复。性能证据见 M4 性能报告；GPU、Narrator、跨屏 DPI 和真实 IME 单列，不冒充自动通过。
