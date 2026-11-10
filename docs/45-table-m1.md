# YueTable M1 使用与验收

日期：2026-11-10。基础实现已进入代码；[主设计](45-table.md)仍是 M1–M6 的完整目标，不能把后续阶段的拟定 API 当作已实现能力。

## 1. 当前入口

公共类型从 `yue_component` 导入；最小完整程序见 [examples/minimal/src/bin/table.rs](../examples/minimal/src/bin/table.rs)。运行：

```powershell
cargo run -p yue-minimal --bin table
```

宿主保留呈现 delegate 和 `Entity<YueTableState<D>>`。`TableDelegate` 只关联 `Source`，Row 从 Source 推导；默认 `render_cell` 使用列 renderer，不必为空壳 delegate 重写方法。`TableView::shared_columns()` 提供共享列声明，`selection_kind()` 读取当前选择模式，builder 使用 `selection_mode(...)`。State 以弱引用调用 delegate，宿主须保留它，避免宿主持 State、State 又强持宿主形成环。

```rust,ignore
impl TableDelegate for Orders {
    type Source = TableRows<Order>;
}
let delegate = cx.new(|_| Orders);
let table = cx.new(|cx| YueTableState::new(&delegate, cx));
table.update(cx, |state, cx| {
    state.commit_view(TableView::new(source, columns), None, cx)
})?;
```

这里的 `cx` 为宿主 Context；完整程序提供定义及数据。该段用于说明接线，不作为独立编译单元。

## 2. 已实现能力

| 能力 | 实际行为 |
| --- | --- |
| 来源 | `TableSource` 不可变窄读取；`TableRows::try_new` 共享 `Rc<[R]>`，不要求 Row: Clone；空 key、重复 key、错误反查拒绝 |
| 接受 | `commit_view(candidate, response_to, cx)` 唯一发布；校验失败不换 accepted Rc；`based_on` 校验基础 view revision；同作用域的新 source revision 必须递增 |
| 列 | 文本／自定义 renderer、rem 宽度、正文／独立表头对齐、垂直对齐、Wrap／真实溢出 Tooltip、自定义表头、列 footer |
| 模式 | 默认 Rows＋Fixed，默认有限高度 20rem；Static 自然高度。Rows 每工作行渲染全部列，列总数很多时不能当作 Cells 模式 |
| 滚动 | 复用 Base VirtualGrid，以一个横向整行轨道承载各列；一个 ScrollHandle 统管两轴，表头与正文共享横向位置；YueScrollbar 提供浮动滚动条 |
| 表头 | Rows 内默认固定，支持隐藏／取消固定；空结果仍保留表头；Static 随自然内容滚动 |
| 外观 | 三档密度；None／Rows／Grid 分隔；可选外框、斑马纹与 hover；普通格透明，冻结表头／footer 有区域底板 |
| 选择 | 默认关闭；单选 Radio、多选 Checkbox、当前结果全选／半选、禁选行、空格、Shift 范围及 Ctrl+A；单选重复选择保持选中，清空走独立请求 |
| 状态 | Ready、Loading、Refreshing、Failed；刷新保留旧行，busy 禁发选择；失败保留旧结果；业务空／无可见列独立提示；自定义 empty／toolbar／整表 footer |
| 键盘 | 表格导航焦点；上下左右、Home／End、Ctrl+Home／End、PageUp／PageDown；Enter／F2 进入声明的真实控件，Escape 在子控件／浮层未消费时返回导航 |
| 身份与生命周期 | scope＋row key＋column key；同 key 重排保留活动身份；焦点控件只稀疏保留其一行，删除／清空后修复焦点、释放旧 source；不抢外部焦点 |
| 语义 | Table／Row／ColumnHeader／Cell、零基行列索引、逻辑数量、选择／禁用／只读状态；不声明完整 Windows UIA Grid／GridItem／离屏 Realize |

密度复用已有 `SIZE_32／40／48`、`FONT_SIZE_*`、`SPACE_*` 等 primitive 与语义颜色；不改生成 token 文件，不借 Descriptions 的部件 token 形成耦合。表头区域底板默认 `theme::keys::SURFACE`，特殊 Card 表面用 `.surface_background(token_key)` 明确指定。Rows 的行高包含分隔和内容，文本默认单行省略；要显示自然多行使用 Static，或显式提供足够 `.row_height(rems(...))`。

`scroll_to_row(key, options, cx)` 只负责 Rows 自有视口。Static 由包含页面滚动，当前返回 `UnsupportedOperation`，不把未执行的定位报告为成功。`readonly` 保留浏览和选择；M1 没有编辑器，因此不把它描述为已完成编辑权限管理。

## 3. 请求与冲突规则

TableEvent 是唯一事件通道。`SelectionRequested` 包含实例票据、基础 revision 和类型化意图；`apply_to` 在宿主读取的最新 accepted view 上重基，候选随即带该 view revision。不要保存旧整份目标再覆盖新选择。

Showcase 的真实订阅见 [table.rs](../examples/windows-showcase/src/pages/table.rs)。顺序为：读取 accepted → `request.apply_to(&accepted)` → `commit_view(next, Some(request.id()), cx)`；失败后结算所属 `reject_request`。外部 commit 传 None，不回发用户请求。无变化返回 Unchanged，不更新滚动几何。

同一更新内连续两次选择各自有票据，可以依次在最新 view 上接受；旧完成／拒绝只结算自己的票据，不能清除较新的 pending。全量 source 替换使未完成的旧选择票据失效。跨实例票据、重复结算、过期基础 revision、同作用域倒退／复用 source revision 均拒绝。未结算请求上限 64；达到上限返回 `TooManyPendingRequests`，宿主必须接受或拒绝已有请求。

M1 只重基选择意图。后续 query、异步加载、编辑与列配置的跨领域规则在其实施阶段补失败用例，不对尚不存在的请求类型作完成承诺。

## 4. 自定义控件的焦点合同

自定义内容返回完整拥有数据的 Element，不能保留 `&Row`。Input、Select 等真实 Entity 继续由宿主按 cell key 保留。

有真实操作的列声明 `.interactive(true)`；其主控件仅在 `cell.is_active()` 时进入 Tab 顺序，并在 `cell.control_focus()` 为 Some 时挂接该 handle。这样 Enter／F2 可以直接进入控件，而不是猜测子树、寻找按钮或增加所有行的 Tab 停靠点。Showcase 的“复制编号”是完整范例。renderer 还需依据 `cell.size()`、`is_disabled()`、`is_busy()`、`is_readonly()` 设置真实控件；Table 不遍历任意自定义子树来覆写其属性。

未声明交互的 Avatar／Tag 等静态内容不参与焦点导航；行内 Button 的点击和 Space 不触发行选择。内置选择控件通过新增的 `YueCheckbox::tab_stop(false)`／既有 Radio API 退出重复 Tab 顺序；表格本身的空格／Ctrl+A 提供键盘等价操作。弹层和输入先处理自己的击键，Table 只处理冒泡到自己的导航／Escape。

## 5. 验收与后续

自动化入口：[Table 合同与 UI 测试](../crates/yue-component/tests/table.rs)、[Showcase 页面测试](../examples/windows-showcase/src/pages/table.rs)、[TB01 CPU 探针](../crates/yue-component/tests/table/benchmark.rs)。证据与命令见[实施记录](research/2026-11-10-table-m1-implementation.md)及[12,000 帧 headless 规模基线](performance/2026-11-10-table-m1-baseline.md)。

后续按 M2–M6 实施左右固定列／尾行、伸缩与分组表头、排序筛选与分页、展开和树形、Cells／动态尺寸、编辑、拖拽及完整列配置。M1 的 CPU 证据是 headless 初始基线；真实窗口滚动、Narrator、IME、GPU、跨 DPI／多显示器验收仍独立列出，不由测试通过数量替代。
