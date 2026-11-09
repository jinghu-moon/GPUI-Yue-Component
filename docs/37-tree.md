# YueTree 树组件设计

- 状态：首版已实现；实际 API、测试证据与仍待人工验收项见第 16 节。
- 设计：2026-11-08；实现验证：2026-11-09。
- 平台：Windows / GPUI，沿用当前锁定的 GPUI Kit / Base 0.8.0 与仓库补丁。
- 目标：提供文件目录、组织结构、分类和权限树的通用层级浏览、选择与勾选能力，同时作为 [YueTreeSelect](38-tree-select.md) 的唯一树行为基础。
- 约束：[组件架构](24-component-architecture.md)、[Checkbox](06-checkbox.md)、[Popup](12-popup.md)、[虚拟滚动](28-virtual-list.md)、[List](29-list.md)。

## 1. 四份参考与采用范围

四张用户截图已归档，按用户提供顺序对应：

| 参考 | 重点 | Yue 的采用方式 |
| --- | --- | --- |
| [图 1：Arco TreeSelect](assets/tree-select-arco-reference.png) | 树形字段、异步、搜索、多选、勾选回填、虚拟列表、自定义弹层 | 与 TreeSelect 共用节点、勾选、过滤及加载合同 |
| [图 2：TDesign TreeSelect](assets/tree-select-tdesign-reference.png) | 无复选框多选、标签折叠、自定义值、前后缀和字段状态 | 区分行选择与复选框勾选；保留同一组稳定 key |
| [图 3：TDesign Tree](assets/tree-tdesign-reference.png) | 激活与勾选分离、级联、过滤展开恢复、连线、异步、节点操作及虚拟滚动 | 采用行为区分与场景覆盖，明确未知子树和禁用边界 |
| [图 4：Arco Tree](assets/tree-arco-reference.png) | 整行命中、受控状态、拖放、勾选策略、尺寸、图标、额外内容、定位 | 保留树行语义根，采用 typed 配置和内容插槽；拖放单独分期 |

参考用于确定功能与信息结构，不照搬网页卡片、React 属性、DOM 容器配置、四档网页尺寸或截图缩放后的像素。

Tree 处理层级浏览及选择；TreeSelect 处理字段与树弹层。横向逐级面板属于 Cascader，多列单元格属于 TreeGrid，均不混入本组件。

## 2. 已有能力与真实缺口

| 当前源码 | 已提供 | 设计中的复用与补齐 |
| --- | --- | --- |
| [Base Tree](../vendor/gpui-base/src/tree.rs) | TreeItem / TreeEntry / TreeState；展开、折叠、可见条目、焦点、上下左右、reveal_item、uniform_list；Tree / TreeItem role | 在同一 Base 树基础上补齐稳定 key、独立活动项、选择、勾选、过滤和加载；不在 Yue 再写一套树引擎 |
| 当前 Tree 的限制 | selected_ix / right_clicked_ix 按位置保存；set_items 重置选择；点击同时选择与展开；is_folder 仅判断 children 非空；事件仅 Expanded / Collapsed | 新行为显式 opt-in；允许未加载分支；新合同不依赖可见索引作为身份 |
| 当前 Tree 的投影 | root 移入 entries；add_entry 会 clone TreeItem，包含其 children；rebuild_entries 重建可见投影 | 不能因已有虚拟列表就宣称整棵树无克隆成本；新路径使用共享模型和轻量可见行 |
| [Base virtualization](../vendor/gpui-base/src/virtualization/mod.rs) | 固定／已知／测量尺寸、稳定身份相关配套、锚定、定位、滚动命令和失效合同 | 增强 Tree 的行视口复用该基础；不新建滚动算法 |
| [YueVirtualList](../crates/yue-component/src/virtual_list.rs) | 垂直虚拟列表外观、Scrollbar、可配置容器和条目语义 | 借用相同呈现／滚动零件；Tree 不能额外套一层 List / ListItem 角色或第二个视口 |
| [YueCheckbox](../crates/yue-component/src/checkbox/mod.rs) | Checked / Unchecked / Indeterminate、三尺寸、禁用、命名及请求事件 | 复用三态绘制与反馈；树控制器决定级联结果 |
| YueInput / Icon / Tooltip / Button | 完整搜索输入、Tabler 图标、溢出提示、命令及焦点 | 搜索栏和操作栏直接组合；不复制编辑器、图标或按钮 |
| YueEmpty / Spinner / Skeleton | 空态与加载反馈 | 区分整树空、过滤无结果、根加载、单分支加载与失败 |
| YueDropdown / Popup / motion | 菜单、浮层、关闭路由、动效策略 | 节点菜单和提示直接组合；行不创建窗口级 owner |
| YueList / YueCollapse | 内容布局、独立折叠面板 | 不作为树状态机；不逐节点嵌 Collapse，避免深层布局与动画叠加 |

当前上下导航会回绕，左右只开合当前分支；这与本文增强行为不同。不能只改外观，就认为新键盘与多选合同已经实现。

## 3. 架构与唯一所有者

```text
宿主：业务对象、网络任务、持久化、权限判断、节点增删改与移动
                         ↓ typed 节点／更新
YueTreeState：持有一个 Base KeyedTreeModel、订阅和 Yue 呈现缓存
                         ↓
Base 树行为：模型索引／展开／active／selected／checked／过滤／加载票据
                         ↓ 轻量可见行
Base Tree 语义根 + 现有 virtualization 视口
                         ↓
YueTree：缩进、箭头、Checkbox 外观、图标、标题、extra、连线、Scrollbar
```

- 宿主持有 `Entity<YueTreeState>`，使用 `YueTree::new(id, &state)`；一个 state 同时只挂载一个树视图。
- Base 是节点模型、展开集合、活动 key、选择集合、勾选集合和加载状态的唯一所有者；Yue 不按事件再复制一份可写选择。
- Yue 持有可选的搜索 InputState、搜索订阅、主题与布局配置；InputState 是 query 文本唯一真值，规范化 query / match 索引只是派生缓存。
- 模式和交互许可在 state 初始化／update 中设置；size、theme、显示图标、连线和内容 renderer 属于呈现。render 不创建 entity、任务或订阅，不回写业务值。
- TreeSelect 内部组合同一 YueTreeState / Base owner，不再创建第二棵树来同步选中项。

### 3.1 Base 扩展边界

实现采用 Base `keyed_tree::KeyedTreeModel` 的独立 opt-in 入口，供 YueTree 与 TreeSelect 共用。旧 `Tree / TreeState` 的索引、点击展开及导航合同保持原状，旧路径尚未迁移到新模型；两者不是同一实例的两套呈现。此实现选择避免改写旧状态语义，后续上游升级时可独立迁移 keyed 模块。

增强行根由 Base 保留：稳定 ElementId、TreeItem role、活动语义、选择／勾选／展开及统一动作分发。Yue 只供应内部槽位；箭头、勾选和行点击必须进入同一控制器，一次事件只能提交一次。

旧 Tree 继续使用 uniform_list；YueTree 的 keyed 模型使用 virtualization，一次挂载只有一个滚动 owner。新增通用行为放 Base，Yue token 和 Tabler 图标留在 Yue。补丁必须记录来源、兼容测试、迁移方式与移除条件，更新 [YUE-PATCH](../vendor/gpui-base/YUE-PATCH.md)；使用方仍须保留根 Cargo patch。

## 4. 节点、身份与数据更新

以下为首版实际数据合同；公共结构字段私有，使用构造器、builder 和 reader。

| 类型 | 合同 |
| --- | --- |
| TreeNode | 稳定非空 key、非空可读 label、子节点状态；可附 keywords、disabled、selectable、checkable、check_disabled。图标由 YueTreeState::set_node_icon 配置，额外说明用 render_extra |
| TreeChildren | Leaf、Loaded(children)、Unloaded；禁止用空 Vec 同时表示叶子和待加载分支 |
| TreeSelectionMode | None / Single / Multiple；创建 state 时固定 |
| TreeCheckMode | None / Strict / Cascade；与行选择独立，创建 state 时固定 |
| TreeCheckProjection | All / Parents / Leaves；勾选结果展示投影，不是另一份值 |
| checked_keys / TreeMark | Strict 下保存独立 key；Cascade 下保存规范化的可勾选叶子 key；提供只读投影与半选查询，没有独立 TreeCheckedValue 类型 |
| TreeDataError / TreeApply | 空 key／label、重复／未知 key、模式不符、非法值、不完整子树、数量上限返回 typed 错误；过期加载返回 TreeApply::Stale |

key 在整棵树全局唯一且区分大小写；不同父级也不能复用相同 key。label 可重复；序号、可变祖先路径和标题均不能充当 key。业务对象由宿主用 key 映射，不在组件里保存第二份泛型业务模型。

输入可采用嵌套 builder；Base 内部建立共享、带版本的节点索引，保存 parent、children 顺序及必要遍历信息。可见条目只含 key / depth / 层级元数据等，不复制整棵子树。任何扁平导入适配都需拒绝环和多父；嵌套输入亦须拒绝重复 key。

### 4.1 更新规则

| 操作 | 行为 |
| --- | --- |
| set_nodes（完整模型） | 原子验证；按 key 保留有效展开、选择与勾选，删除已不存在项；返回 reconciliation 报告，不伪装成用户操作 |
| 改 label、顺序或父级 | key 不变则身份不变；刷新路径／元数据，不按原索引迁移状态 |
| 删除 active | 优先移到仍在的最近可用祖先，再找原位置附近可用行；无可用行则清空 active |
| 折叠包含 active 的分支 | active 移到该分支；后代选择／勾选保留，焦点不落在隐藏行 |
| 新增子节点 | 默认不选中新子项；原“全勾”父节点按新全集重算，必要时变半选 |
| 已勾叶子改为分支 | Cascade 原叶子 key 失去规范值资格，移除并在报告中列出；不自动授权新后代 |
| disabled / check_disabled 改变 | 已有值不擅自清除；成为锁定选择；其与可传播集合的关系按第 6 节重算 |
| 静默 setter | 成功更新不发送用户 Change；同值无副作用；错误完整保留旧模型、焦点和票据 |
| 整体 reload | 增加模型 generation，旧加载／搜索票据失效 |

普通 Tree 的 key setter 只接受当前模型存在的项。为 TreeSelect 的远程 Single / Multiple / Strict 模式提供显式“保留未解析选择”配置与展示快照；它只保留值，不制造可导航的假节点。Cascade 不接受未知 key 或不完整分支作为赋值种子。

Cascade 的 `set_checked` 只接收规范叶子值，不能把 Parents／All 投影直接回写为值；分支批量选择通过同一控制器的子树勾选命令完成，并先检查完整性。宿主需要从父级业务配置导入时，先在完整模型上显式求得叶子集合，再静默赋值。

完整替换与远程搜索结果替换必须是不同更新入口：前者可确认删除，后者不能推断全集。TreeSelect 的结果入口、保留规则与会话失效见其设计文档。

## 5. 活动项、行选择、展开与勾选

| 状态 | 含义 | 典型视觉 |
| --- | --- | --- |
| active_key | 当前键盘目标，与视图焦点一起决定活动反馈 | 焦点轮廓／活动边界 |
| selected_keys | 用户选择的行，如当前文件、多个资源 | 选中填充 |
| checked value | Checkbox 表达的选择，如批量权限 | 勾号／半选横线 |
| expanded_keys | 持久展开意图 | 展开箭头与后代可见性 |
| hover / context target | 指针反馈与菜单目标 | 轻量 hover／菜单目标反馈 |

方向键只移动 active，不改变 selected 或 checked。右键只设置菜单目标，不默认改选择；已选择多行时右键菜单可继续针对当前集合。宿主决定菜单业务作用域。

默认配置为 Single 行选择、None 勾选、无搜索、整行命中、全部折叠、无连线和无默认节点图标；按需启用 Checkbox 或多选。初始化值通过 state 设置，不在每次 render 的 builder 中重复应用。

### 5.1 默认点击合同

- 箭头：只开合；不选择、不勾选、不触发 Activate。
- 标题／行留白：在 Single 模式选中该行；Multiple 模式按修饰键处理；None 模式只移动 active。
- Checkbox：只请求勾选；不再冒泡触发行选择或展开。
- 首版整行命中，选中填充可延伸到行尾；缩进仍表示层级，暂不提供内容区命中开关。
- 显式 `row_action(Check)` 适用于权限树：标题／留白同 Checkbox；`row_action(Expand)` 适用于纯目录浏览。它是单一动作枚举，不支持一次点击同时 select + check + expand 的组合数组。
- 双击产生 `Activate`，不内置打开文件／路由／加载任务；独立单击选择可以先发生，但不额外切换展开。
- disabled 行不能选择、勾选或开合；不自动把 disabled 继承给所有后代。已展开的可用后代仍可导航。selectable(false) 只禁止行选择，仍可展开；checkable(false) 只隐藏本项 Checkbox。

### 5.2 无复选框多选

普通点击替换选择，Ctrl+点击切换单项；Shift+点击按当前可见顺序选择锚点到目标的范围，Ctrl+Shift 添加范围。范围只包含可选择、非禁用行；折叠／过滤隐藏项不被范围选入。

普通方向键保持选择；Ctrl+A 选择当前可见的可选择行，隐藏的已有选择保留。范围锚点使用 key；锚点不可见时以当前 active 为起点。None / Single 模式不消费多选快捷键。

## 6. 复选框、级联与回填策略

### 6.1 Strict

每个可勾选节点独立切换，父子互不影响；规范值是显式勾选的 key，维持选入顺序。组件不推导半选。首版不提供可写的 half_checked_keys：业务要展示独立状态可用非选择性的节点装饰，不把展示半选混进已提交值。

Strict 下 All / Parents / Leaves 不进行层级压缩，均使用独立选中项，防止隐藏仍有独立语义的值。

### 6.2 Cascade

规范值保存可勾选叶子的 key；分支状态与 All / Parents / Leaves 都从该集合推导，半选永远不进入规范值。

1. 勾选分支会选中其可传播范围内全部可用叶子；取消会移除这些叶子。半选分支再按 Space／点击视为请求全选。
2. `checkable(false)` 的中间节点为透明层级：隐藏自己的 Checkbox，但祖先仍可影响其可用后代。
3. disabled 或 check_disabled 节点为传播边界：祖先批量操作不修改该节点及其子树，该子树也不参与祖先全选分母。其可用后代可单独操作，形成独立传播范围。
4. 变为禁用的已勾叶子保留为锁定值；不参与可用祖先的全选判定；不能被祖先取消、清空或标签删除间接移除。宿主可显式 set_checked 修正。
5. 一个完整分支有至少一个可用叶子且这些叶子全部选中才显示 Checked；部分选中显示 Indeterminate；零选中显示 Unchecked。没有可用叶子的分支不可批量勾选，不用“空集全真”显示全勾。
6. Leaf 本身 checkable(false) 或禁用时不能新选入；Loaded([]) 在成功更新时规范化为 Leaf。

例如 `P → A、B`，两叶子可用：选 A 后 P 半选；再选 B 后 P 全勾。若 `B` 为禁用边界，P 只控制 A；B 子树中的独立选择必须单独保留，不能被 P 标签遮蔽。

### 6.3 已加载数据与未知子树

首版不保存“选择未来所有后代”的隐式权限意图。

- Unloaded 或仍含未知后代的分支可以展开，但 Cascade 的整支勾选入口不可用，并说明“加载完整子节点后可批量勾选”。不自动递归发网络请求。
- 已加载的可用叶子仍可单独勾选；有未知后代时祖先不能判为全勾，有已选叶子则半选，无已选叶子则未选。
- 批量勾选前检查整个可传播范围是否完整，不能只检查直接 children。禁用传播边界内的未知子树不阻塞外部祖先。
- 成功加载新后代只更新模型与派生状态，不自动选中新增叶子、不产生用户 CheckChanged。
- 需要先选父级再自动选中未来后代的业务，应另行设计子树选择意图协议；该能力不作为本轮实施的隐含需求。

此限制同时适用于 Tree 和 TreeSelect，避免异步返回悄悄扩大授权范围。Strict 与懒加载可以直接组合。

### 6.4 投影合同

| 投影 | Cascade 输出 |
| --- | --- |
| Leaves | 规范叶子 key，按当前模型前序排列；锁定叶子仍单独出现 |
| All | 全勾的可勾选分支及叶子；不含半选项 |
| Parents | 用最高层的全勾可用分支压缩其传播范围内叶子；边界另一侧的选择独立保留 |

切换投影不修改规范值、不发送 CheckChanged。Tree 默认 All 供结果预览；TreeSelect 默认 Parents 用于标签压缩。删除 Parents 中一个分支标签代表取消它覆盖的叶子范围；不可作用于边界外或锁定叶子。

## 7. 展开、过滤与搜索

- 默认全部折叠；初始化可给 expanded keys、首层或全部“已加载分支”。初始化展开全部不会遍历网络。
- `reveal(key)` 展开已知祖先并定位；未知 key 返回 TreeDataError::UnknownKey，不悄悄加载未知路径。宿主拿到路径并补齐数据后再 reveal。
- 展开与过滤展开分开保存。过滤只影响可见投影，不改规范模型、选择和勾选。
- 本地默认对 label / keywords 做不区分大小写的子串匹配；不隐式增加拼音／模糊检索依赖。可替换纯过滤函数，结果按 key 缓存，不在每行 render 中执行搜索。
- 非空 query 展示命中节点和祖先路径，自动临时展开必要祖先；命中父级不默认展示全部未命中后代，避免“搜到目录就展示整棵树”。
- 用户可在当前 query 中折叠结果路径；query 改变后重新计算临时展开。清空 query 恢复过滤前持久展开及可恢复的阅读锚点。
- 搜索上下文祖先仍可操作；Cascade 勾选作用于完整模型中的传播范围，不仅是屏幕匹配项。示例必须明确提示“勾选分支将作用于未显示的子项”。
- 本地搜索只覆盖已加载数据，懒加载示例显示该说明；不宣称能搜索服务器全集。

独立 Tree 的搜索栏是可选组合，复用 YueInput。远程搜索会话由 TreeSelect 适配，独立 Tree 宿主亦可按相同票据方式交付模型，但 Cascade 不接受缺失子树的全选推断。

互斥展开、逐层自动展开和大量节点展开动画不是首版必要项；先保证常规开合、过滤恢复和 reveal 的合同。

## 8. 异步加载

Base 保存每节点 Idle / Loading / Failed / Loaded 状态及票据；网络任务由宿主创建并保留。首次展开 Unloaded 节点发出一次 `LoadRequested`，不在 render 发请求。

| 情况 | 行为 |
| --- | --- |
| 同节点反复展开 | Loading 中去重；折叠不丢已启动任务，成功结果可缓存，但不强制重新展开 |
| 不同节点加载 | 可并行，互不覆盖 |
| 成功 children 为空 | 规范化 Leaf，移除箭头和 loading；只修正展开状态，不改变用户值 |
| 失败 | 节点保留，显示简短错误和可访问的重试操作；不删除旧值 |
| 重试 | 新票据，旧返回不可覆盖新结果 |
| 删除节点、整体 reload、销毁 | 票据失效；宿主尽力取消任务，即使未取消也不能写回旧状态 |
| 非法 children | 原子拒绝；保持原模型并进入可重试失败状态 |

`TreeLoadTicket` 绑定模型身份、generation、节点 target 与请求序号；key 删除后重建也不能接受旧返回。`apply_children(ticket, nodes)` 返回 `Result<TreeApply, TreeDataError>`，`reject_load(ticket, error)` 返回 `TreeApply`；合法／过期结果分别为 Applied / Stale，非法数据为 Err 并保留可重试状态。旧返回不弹 Toast、不产生业务选择事件。

根加载复用同一票据机制的 root target，视觉位于树容器；分支加载以箭头位 Spinner 和节点内短状态表达，不用全树遮罩。重试控件不属于业务节点，不污染 key 集合、选择范围和层级计数。

## 9. 布局、尺寸与外观

```text
[可选搜索栏／工具栏：固定]
┌─ 单一树视口 ────────────────────────────┐
│ [箭头] [Checkbox] [图标] 标题       extra │
│   │ [箭头] [Checkbox] [图标] 子节点      │
│   └        [Checkbox] [图标] 叶子        │
└──────────────────────────── Scrollbar ┘
[可选底部说明：固定]
```

树自身不生成外层卡片、标题或独立边框；宿主负责业务容器。默认填满给定宽度，默认视口高度由 `--tree-max-height` 提供，基准为 288px；可显式设置 height。首版不提供 fill 高度模式，宿主需给出有限视口；搜索栏与外部工具栏不随行滚动。

| 项目 | 已接入 token／复用 |
| --- | --- |
| 三尺寸 | TreeSize::Small / Medium / Large；行高 28 / 32 / 40px，复用来源 Input token |
| 缩进 | `--tree-indent-sm/md/lg`，基准为 16 / 20 / 24px；按逻辑层级计算 |
| 行内容 | `--tree-padding-inline`、`--tree-gap`；字体复用同档 Input／正文 token |
| 箭头／图标 | 复用 Tabler ChevronRight 与现有 icon 尺度；箭头槽位固定，Leaf 留对齐空间 |
| Checkbox | 对应现有 Checkbox 三尺寸；复用其色彩、半选笔画和状态，不自绘第二套勾选框 |
| hover / selected / focus / disabled | 以 Yue 语义色为来源补 `--tree-*` 别名，焦点与选择可同时辨认 |
| 连线 | `--tree-line-color/width`，基准宽度 2px（随 rem 缩放），默认关闭，装饰性且不参与命中 |
| 溢出 | 标题单行省略，YueOverflowTooltip 展示完整文本；extra 不挤入文字命中区 |

整树某行有节点图标时默认统一保留图标列；全树无图标时不占列。checkbox 列按 check mode 占位；个别隐藏 Checkbox 仍保留该列，保持同级标题对齐。

深层缩进受当前可用宽度约束，最多占可用宽度三分之一或 12 级缩进，保留箭头、Checkbox 与标题区域。默认 Tooltip 展示完整标题；完整祖先路径由宿主通过 ancestors / render_extra 提供，TreeSelect 也可用 Path 回填。首版不引入横向虚拟化或自动横向滚动。

show_lines 使用行上下文的祖先“是否还有后续兄弟”信息绘制；不能仅查看当前渲染行的前后邻居，否则虚拟截断会断线或多画。竖线中心与对应祖先展开箭头的向下尖端对齐，共用缩进、槽位间距与图标槽尺寸；绘制时以槽位中心向左右各扩半个线宽，避免小数线宽的布局取整造成中心偏移，随三档尺寸和 rem 同步缩放。

来源已在邻仓 `../Yue-Design-System` 注册 18 个 Tree token，并运行同步工具生成 generated.rs / inventory；后续修改仍先改来源，禁止手改生成文件。尺寸随窗口 rem 比例缩放。

### 9.1 动效

2026-11-09 动效增补：原先箭头和子节点即时切换，现默认增加箭头 90° 旋转及子树高度收展。复用既有 `MotionDuration / MotionEase`：箭头 Interaction + Standard（当前 100ms），展开 Enter + Enter（当前 200ms），折叠 Exit + Stow（当前 100ms）。没有新增 token、依赖或公共 API，hover／选择颜色仍即时切换。

逻辑投影先完成更新；Yue 呈现层通过现有 Known virtualization 的 keyed size batch 过渡子树占位，保持一个滚动 owner。行内容保持正常尺寸，以裁剪区域逐步揭示／收回；后续行位置、滚动范围同步变化。折叠后立即卸载后代交互与语义，只保留上一帧已挂载行的绘制快照，不能在收起过程中点选残影。连续反向开合从当前高度继续，按剩余距离缩短时间。

每次最多过渡 128 个变化行；超大分支仍旋转箭头，行布局直接更新，避免让成千上万行参与每帧尺寸反馈。初始展开、搜索／数据替换、reveal、尺寸／主题／DPI 变化直接定位；显式展开 setter 和已展开分支的加载完成可触发收展。键盘进入正在揭示的子行前，先完成当前高度过渡，保证活动行完整可见。reduced motion、窗口隐藏／失活时直接完成过渡；不为视口外的箭头持续请求帧。TreeSelect 复用相同树动效，Popup 动效继续由既有 Popup 处理。

## 10. Windows 键盘与焦点

树为一个 Tab 停靠点，内部用 active key 导航；禁用行跳过，但其已展开的可用后代仍参与顺序。上下不回绕。首次进入优先已有可见 active，再选中可见项，再第一可用行。

| 按键（树拥有焦点时） | 增强路径行为 |
| --- | --- |
| Up / Down | 前／后一可用可见节点，只移动 active |
| Right | 折叠分支展开；已展开分支移到第一可用可见子代；Unloaded 展开并请求加载 |
| Left | 展开分支折叠；否则移到最近可用可见祖先 |
| Home / End | 第一／最后可用可见节点 |
| PageUp / PageDown | 按视口行数移动并确保目标可见 |
| Enter | 应用行选择策略并发 Activate；不兼作开合；TreeSelect 适配为提交／切换字段值 |
| Space | 有 Checkbox 时切换勾选；否则应用行选择策略；阻止同时滚动页面 |
| Ctrl+Space | Multiple 行选择中切换 active 的选择，保持其它项；勾选模式只切 Checkbox |
| Shift+Up / Down | Multiple 模式从范围锚点扩选；Ctrl+Shift 追加；勾选不按范围扩散 |
| Ctrl+A | Multiple 行选择中选当前可见可用项；不隐式变成“勾选全树” |
| Ctrl+F | 有搜索栏时聚焦现有 Input |
| Shift+F10 | 为 active 请求上下文菜单；菜单由宿主用既有 Dropdown 提供；首版未额外绑定硬件菜单键 |
| Tab / Shift+Tab | 离开树；不逐个遍历所有行、箭头和 Checkbox |
| Escape | 先退出节点操作／菜单，由最上层 Popup 消费；独立树不清空选择 |

查询 Input 中左右、Home / End、Space、Ctrl+A 属于文本编辑；非 composing 的 Down / Enter 将焦点交给树，第一次不提交值。IME 组合时 Enter／Escape 由输入法处理。TreeSelect 的关闭和 Tab 路由见下一文档。

extra 默认是非交互的计数／Badge／文本。需要操作时使用明确的行操作模式：F2 进入该行已登记的 Button / 菜单按钮 FocusHandle 顺序，Tab 在这些操作间前进，边界离开树；Escape 返回树 active。操作区不冒泡选择。行回收、过滤或删除前必须先迁移焦点，不能把焦点留给已卸载控件。

## 11. 可访问性

- 一个 Tree 容器和按 key 命名的 TreeItem 行；行提供可读 label、disabled、selected、分支 expanded、checked / mixed，以及真实 level / sibling position / sibling count。
- 虚拟化时层级、同级数量来自过滤后的逻辑投影，而不是当前挂载范围；计数仅包含已知节点，不把未知后代报为“全部”。
- Checkbox 的视觉复用需要支持树行内嵌模式：不额外加入全局 Tab，不重复朗读两份 checked。优先由 TreeItem 承载勾选语义和动作，内嵌 mark 仅负责绘制；若需抽取现有 Checkbox 的 crate-private mark／surface，应同步回归独立 Checkbox。
- 为 Row 的选择、Toggle 和 Expand 提供统一可访问动作；不是只给图标加 label。额外 Button 有自己的名称，但不替代 TreeItem 角色。
- active 行需要在视口内，焦点语义与稳定 key 一致；数据替换不能把旧行的语义 ID 赋给另一个节点。
- 当前 Base Tree 只提供部分语义。level、sibling 和虚拟行可访问目标是否有完整底层接口，实施前须逐项核查；不足时在现有语义层补齐，不能把设计清单写成已支持。
- 不把 Windows UIA 离屏 Realize 或自建 COM provider 列为本组件交付条件；Narrator 实窗检查明确记录当前平台实际边界。

## 12. 公共 API 与事件

| 表面 | 入口 |
| --- | --- |
| 构造 | YueTreeState::new(selection_mode, check_mode, window, cx)；YueTree::new(id, &state) |
| 数据 | set_nodes、node / parent / ancestors、apply_children、reject_load |
| 状态 | selected_keys / set_selected_keys、checked_keys / set_checked、projected_checked、mark、expanded_keys / set_expanded_keys、active_key |
| 导航 | reveal、focus；返回不存在／未加载等可区分结果 |
| 搜索 | searchable（builder）、query、set_query、set_filter；组合保留的 InputState，过滤只更新投影 |
| 呈现 | size、theme、height、show_lines、show_icons、render_label、render_extra、render_actions、empty |
| 行动作 | set_row_action、set_row_action_handles 登记额外操作的 FocusHandle；不暴露可任意覆盖语义根的 render_row |

事件采用保留订阅，不以每次 render 的回调保存业务状态：

| 事件 | 内容与时机 |
| --- | --- |
| SelectionChanged | before / after keys、trigger key、Pointer / Keyboard / Accessibility；值实际改变才发送 |
| CheckChanged | before / after 规范值、trigger key、source；半选／投影为可读派生结果，不复制整树 |
| ExpansionChanged | key、expanded；仅用户开合发送，reveal／过滤不冒充用户动作 |
| Activate | 节点 key 与来源；宿主决定打开／导航行为 |
| LoadRequested | 节点／根目标与不可伪造 ticket |
| ContextRequested | target key；宿主按当前行／指针上下文放置菜单 |

一次交互先完成原子状态更新，再发送值事件；订阅者读取 state 时已是 after。程序同步、加载完成或过滤不发送用户选择事件；数据修复由 setter 返回的报告交给宿主处理。

## 13. 拖放和节点编辑的分期边界

参考中的增加、删除、重命名不需要组件内置文件系统 CRUD。首版示例用现有 Button／Dropdown 操作宿主数据，再通过 set_nodes 更新；同一个 key 的状态连续性由树保证。

节点拖放列为后续独立阶段，只有确认业务需要后实施。预留合同：Before / Inside / After typed 位置；禁止自身／后代形成环；disabled、can_drag、can_drop 明确；稳定 key 参与跨行回收；边缘自动滚动、悬停展开、Escape 取消及键盘移动替代；DropRequested 携带模型版本，宿主接受后提交新数据。拖动不先修改业务结构再等待宿主撤销。

TreeSelect 首版不启用树节点拖放，标签排序也不是树节点移动。跨窗口／文件系统拖放、TreeGrid、任意行高富文档节点与自动加载全集不进入本轮范围。

## 14. Showcase 与验收

以下是设计验收场景清单；首版已提供独立 **组件 → Tree** 页面，当前实际示例见第 16 节。尚未把全部模型边界都做成可点的示例：

1. 文件目录：基础、三尺寸、图标／连线、长标题、深层节点及 reveal。
2. 行选择：None／Single／Multiple，Ctrl／Shift、选择与焦点分离、上下文菜单。
3. 权限树：Strict／Cascade、半选、三种结果投影、不可选／不可勾／禁用传播边界。
4. 搜索：祖先路径、临时展开、清除恢复、未加载范围说明、过滤中勾选全模型的提示。
5. 异步：根／分支、并发、失败重试、折叠中返回、删除重建同 key、空成功变叶子。
6. 节点操作：宿主增加、插入、重命名、移动、删除，展示保留和 reconciliation。
7. 一万节点：宽树／深树、局部展开、全已加载展开、键盘快速导航和滚动锚点。

| 验收层 | 必须覆盖的真实合同 |
| --- | --- |
| Base 模型测试 | 唯一 key、非法更新原子性、选择／勾选独立、禁用边界、规范值与投影等价、未知子树不能全勾 |
| Base 兼容回归 | 现有 tree 测试继续通过，旧点击／回绕默认不变，增强配置单独测试 |
| GPUI 事件测试 | 点击与拖离、Checkbox 不冒泡、真实 actions、多选修饰键、焦点回收、语义节点与层级 |
| 异步测试 | 同节点去重、不同节点并发、过期票据、删除重建、防止加载后悄悄选中新叶子 |
| 虚拟滚动测试 | 按 key 锚定、投影改变、焦点行挂载、跨视口连线、只创建可见范围加 overscan 行 |
| 性能前后对比 | 相同构建／数据／展开状态，比较模型建立、展开、过滤和滚动；记录耗时、分配与挂载数，不把截图流畅等同于性能证据 |
| 实窗人工 | 用户负责 Windows 视觉、真实 IME、Narrator、当前设备 DPI 与输入设备检查；尚未执行，不承诺多显示器验收 |

可借鉴参考项目的 key 唯一、过滤恢复和级联边界用例逻辑；本地现有 Base Tree / Select / virtual_list 测试作为起点，不写只重复实现细节的快照测试。

## 15. 实施顺序与交付门槛

1. Base 增强：共享节点索引、显式分支、key 身份、独立 active／selected、兼容行为配置；先通过旧 Tree 回归。
2. YueTree：token、三尺寸、箭头／图标／Checkbox 内嵌零件、虚拟视口、基础语义和键盘；通过文件目录和多选示例。
3. 勾选／过滤／异步：级联、投影、边界、搜索恢复、加载票据和错误反馈；通过权限与异步用例。
4. TreeSelect：按 [后续文档](38-tree-select.md) 抽取确实共用的字段零件，组合相同树 owner；先回归 Select 再接新字段。
5. Showcase、性能比较、文档同步与实窗待验收记录；节点拖放不阻塞前四阶段。

首版实现及本文交付记录列出的自动验证已完成。性能前后耗时／分配基准和实窗项目尚未完成，不能将上表全部视为通过；具体证据见下节。

## 16. 首版实现与用法（2026-11-09）

源码：[Base keyed model](../vendor/gpui-base/src/tree/keyed.rs)、[YueTree](../crates/yue-component/src/tree/mod.rs)、[状态](../crates/yue-component/src/tree/state.rs)、[Showcase](../examples/windows-showcase/src/pages/tree.rs)。

```rust,ignore
let tree = cx.new(|cx| {
    let mut state = YueTreeState::new(
        TreeSelectionMode::Single, TreeCheckMode::None, window, cx,
    );
    state.set_nodes([
        TreeNode::new("source", "源代码").children([
            TreeNode::new("main", "main.rs"),
            TreeNode::new("remote", "远程目录").unloaded(),
        ]),
    ], cx).unwrap();
    state
});
// 在宿主 render 中组合；同一个 state 只挂载一次。
YueTree::new("files", &tree)
    .show_lines(true).show_icons(true).searchable(true)
    .height(px(288.));
```

`TreeNode` 为嵌套导入数据；模型存储后是无子树克隆的节点元数据，遍历实际子项使用 Base `children_keys(key)`。`child_state()` 在存储节点上只表示 Leaf / Loaded / Unloaded 类别，Loaded 的 Vec 不再重复保存已被扁平索引接管的数据。`TreeFilter` 为同步纯匹配器，可由 `set_filter(Some(Rc::new(...)), cx)` 配置，祖先保留和展开恢复仍由模型完成。

实际勾选值是 `checked_keys()` 的只读切片，没有另建可写 `TreeCheckedValue`。Strict 维持选入顺序；Cascade 按模型前序保存规范叶子集合。程序赋值可保留显式禁用项作为锁定值，但拒绝非 selectable／非 checkable 项与 Cascade 分支值。`set_nodes` 返回 reconciliation，错误不部分修改旧模型。

`render_extra` 是非交互装饰。行操作使用 `render_actions` 与 `set_row_action_handles(key, handles, cx)`，控件需 `tab_stop(false)`；F2 进入、Tab 遍历、Escape 返回树。活动行按稳定 key 保留挂载，过滤／删除操作行时在下一次布局前将焦点迁回树。菜单通过 `ContextRequested(key)` 交由宿主的 Dropdown 处理。

Showcase 包含三档尺寸、单／多选、Strict／Cascade、搜索恢复、reveal、标题更新保留 key、F2 复制路径、首次失败的分支重试，以及 100×100 子节点的虚拟树。默认开合动效见第 9.1 节。业务布局仍只支持固定行高，过渡期内部尺寸反馈不开放为可变行高 API；首版整行命中，未增加内容区命中开关。

验证证据集中在 [Tree / TreeSelect 实施记录](research/2026-11-09-tree-implementation.md)。一万节点测试断言只挂载少量视口行、End 定位末项、导航和选择复用相同投影；一万层深树测试断言连线索引共享且存储为 O(N)。这些是复杂度与交互证据，不是实机帧率或已完成的前后耗时／分配基准。

仍待用户真机验收：Windows 像素外观、真实 IME 候选窗口、Narrator、当前设备 DPI、滚轮／触控板。节点拖放、TreeGrid、可变行高不在首版范围。
