# YueTreeSelect 树选择器设计

- 状态：首版已实现；实际 API、测试证据与待人工验收项见第 14 节。
- 设计：2026-11-08；实现验证：2026-11-09。
- 平台：Windows / GPUI，复用当前仓库的 Base、Yue 组件与 token 同步链路。
- 目标：从层级候选中填写一个或多个字段值；完整复用 [YueTree](37-tree.md) 的树行为，并复用现有 Select 字段零件，而非复制两套控制器。
- 关联：[Select](20-select.md)、[Input / Field / Form](03-input.md)、[Tag](19-tag.md)、[Popup](12-popup.md)、[组件架构](24-component-architecture.md)。

## 1. 四份参考的吸收与取舍

| 来源 | 已核实场景 | Yue 设计 |
| --- | --- | --- |
| [图 1：Arco TreeSelect](assets/tree-select-arco-reference.png) | 基础、对象值、自定义回填、前置文字、受控、异步、本地／远程搜索、尺寸、触发器、多选／勾选、回填策略、弹层插槽、虚拟列表、标签拖排 | 采用字段与树的主要能力，类型化值和票据；标签拖排另列后续，不把拖排当树移动 |
| [图 2：TDesign TreeSelect](assets/tree-select-tdesign-reference.png) | 无 Checkbox 多选、过滤、自定义数据值、树与弹层配置、异步、折叠标签、前后缀、加载／只读／禁用／无边框 | 采用三种互斥选择模式、折叠和统一字段状态；配置用 typed 白名单，不透传任意 treeProps |
| [图 3：TDesign Tree](assets/tree-tdesign-reference.png) | 激活与勾选分离、过滤恢复、异步叶子陷阱、受控和虚拟滚动 | TreeSelect 直接继承同一树合同，不维护弹层专用简化树 |
| [图 4：Arco Tree](assets/tree-arco-reference.png) | 箭头与节点动作、级联、选中策略、连线、图标和额外内容 | 复用树行结构和勾选投影；禁止插槽覆盖行语义与确认路由 |

对象值只意味着宿主可通过 key 找到对象；组件不复制完整业务对象。JS 的 labelInValue、fieldNames、DOM 挂载容器与任意属性透传均不直接搬到 Rust。

TreeSelect 是树形字段，不是 Dropdown 命令菜单、Cascader 多列选择、可自由创建文本的 Combobox，也不是 TreeGrid。首版不内置创建节点、改名、节点拖放或拖动 Tag 排序。

## 2. 可直接复用与必须抽取的能力

| 来源 | 复用内容 | 不能直接假定的部分 |
| --- | --- | --- |
| YueTree / Base Tree 增强 | 节点、展开、active、选择／勾选唯一真值、过滤、异步票据、虚拟视口、TreeItem 语义 | 当前 Base Tree 仍缺这些完整合同；须先按 Tree 设计实现 |
| [Base Select](../vendor/gpui-base/src/select.rs) | ComboBox 根、trigger/content focus 桥、开合请求、确认与取消路由 | 不使用 YueSelectState 保存另一份平铺候选和已选值 |
| [YueSelect 字段](../crates/yue-component/src/select/element.rs) | Input 尺度、字段宽度、前缀、清空、箭头、Tag 及折叠布局的已验证规则 | 当前实现依赖 Select state / Presentation / Option，不能未经抽取就在兄弟模块调用私有 renderer |
| [Input surface](../crates/yue-component/src/input/frame.rs) | 现有 surface_style、状态边框／背景、ControlLayout 与字体尺度 | TreeSelect 只组合真实外框，不再包一个有独立值的 YueInput 当 trigger |
| YueInput / InputState | 弹层搜索、完整编辑、清除、选区、IME | 不重写文本编辑器；query 不是字段值 |
| YueTag / YueOverflowTooltip | 值标签、关闭、溢出说明 | TreeSelect 决定树分支标签的删除范围与锁定规则 |
| 既有 Popup | Anchor、碰撞、可用高度、Presence、父子浮层、Escape 和外部点击 | 一个主 Popup，折叠标签面板是其关联子面板；不另写全局监听 |
| YueField / YueForm | 类型化选择值适配、标签、错误、必填、提交失败聚焦 | 新增 tree_select typed adapter，不伪装成字符串 Input |
| Empty / Spinner / Button / Icon / Tooltip | 无结果、加载、重试、清空、装饰及说明 | 不为一个树字段新造一组同职能控件 |

### 2.1 字段零件的具体共享方案

实施时从 Select 中抽取 crate-private `choice_field` 零件：

1. 字段 frame／尾部操作布局：接收已求出的字段状态、尺寸、宽度、前缀和内容槽；调用现有 Input surface，不拥有选择或 open。
2. Tag 布局／折叠测量：接收稳定身份的展示条目、可关闭标记及删除请求；复用 YueTag 与既有测量策略，不自己保存选中集合。
3. 主 Popup 和折叠子 Popup 继续使用既有基础层；只提取确认相同的几何／宽度 helper，不预先抽象两套不同的键盘控制器。

Select 先迁移并通过原有交互／几何回归，再接 TreeSelect。共享零件只保存必要测量缓存，不创建 InputState、TreeState、SelectState 或独立 Popup runtime。

TreeSelect 不把 TreeNode 平铺成 SelectOption 再塞进 YueSelect；那会丢层级语义、重复勾选逻辑并产生两个 active／value owner。Select 的 ListBox 内容不参与 TreeSelect 的树弹层。

## 3. 状态所有权与模式

```text
宿主：业务对象／远程查询和节点加载任务／表单规则
                              ↓
Entity<YueTreeSelectState>
  ├─ 一个 YueTreeState / Base owner：唯一 selected 或 checked value、模型、active、展开
  ├─ 一个可选 InputState：query 的唯一文本
  ├─ open／查询会话与票据／已选标题路径快照／字段焦点
  └─ 测量缓存、Popup 注册与保留订阅
                              ↓
Base Select ComboBox 根 + choice_field + YueTag
                              ↓
既有 Popup + YueInput 搜索栏 + 相同的 YueTree 语义与虚拟视口
```

宿主持有 `Entity<YueTreeSelectState>`，渲染 `YueTreeSelect::new(id, &state)`。一个 state 只挂载一个字段。TreeSelect 的 value reader 读取同一树 owner，不保留一份可写的 committed keys 镜像；用于 Change 的 before 快照不构成第二个真值。

嵌入树不再启用独立 Tree 的搜索栏／InputState。唯一搜索输入由 TreeSelect 保留，其规范化文本交给同一 Base 过滤入口；该入口只维护派生匹配投影，不是第二个可编辑 query。

| 模式 | 值来源 | 默认交互 |
| --- | --- | --- |
| Single | Base selected 单 key | 选择可用行后立即提交并关闭 |
| Multiple | Base selected key 集合，父子独立 | 点击／Enter 切换该项；保持弹层 |
| Checked(Strict) | Base 独立 checked keys | Checkbox 或标题切换；父子独立，保持弹层 |
| Checked(Cascade) | Base 规范叶子集合及其投影 | Checkbox 或标题级联；保持弹层，Parents 标签压缩 |

模式在构造时固定，用枚举排除 multiple + checkable 等布尔组合冲突。Checked 模式禁用独立行选择；活动行与勾选分离。Multiple 是字段式独立开关，不照搬独立 Tree 的“普通点击替换全部”多选模型；此选择策略作为 Base 的显式配置，而不是在两层分别处理点击。

默认 Single、Medium、Outline、无搜索、clearable(false)、无数量上限；disabled／readonly／loading 均为 false。多值默认 Collapse、最多 2 个标签；Checked(Cascade) 默认 Parents。模式及行为配置在 state 中持有，不能与外观 builder 各存一份真值。

Single / Multiple 默认任何 selectable 节点都可作为值；可配置 LeavesOnly，分支标题此时只开合，不提交值。箭头始终只开合；selectable(false) 仍可导航／展开。Checked 模式由 checkable／check_disabled 控制，不能再用 selectable 隐式改变级联范围。

## 4. 公共值、标签快照与同步

| 类型 | 含义 |
| --- | --- |
| TreeSelectMode | Single / Multiple / Checked(TreeCheckMode) |
| TreeSelectValue | Single(可空 key)、Multiple(有序 keys)、Checked(Vec<SharedString>)；模式必须匹配 |
| TreeSelectItemSnapshot | key、label、已知祖先路径、disabled／locked、解析状态；字段私有，reader 读取 |
| TreeSelectChange | before / after 规范值、trigger key、source、选中展示快照；不附整棵树 |
| TreeSelectSearch | None / Local / Remote，Remote 提供防抖及票据交付 |
| TreeSelectValueDisplay | Label / Path；默认 Label，Path 使用已知祖先文本和分隔符 |

普通多选／Strict 保留选入顺序；取消再选排末尾。Cascade 的规范叶子与投影按模型前序稳定排列，原因是祖先压缩不等于独立标签顺序。模式差异在事件与文档示例中可见。

### 4.1 静默同步

| 操作 | 合同 |
| --- | --- |
| set_value，同值 | 静默，无关闭／清 query 等副作用，支持宿主回写 |
| set_value，不同值 | 原子同步树 owner，关闭会话并清 query、失效远程查询票据，修复 Tag 焦点；不发送用户 Change |
| 完整 set_nodes | 按 Tree 的 authoritative 更新规则；返回被删除／不再有效项的报告，不暗中保留确认删除的 key |
| label／路径更新 | 刷新展示快照，值 key 不变，不发送 Change |
| set_selected_items | 只补当前已选项的 label／路径／锁定快照，不新增候选、不擅自变值 |
| query 结果缺少已选 key | 保留值和快照；搜索未命中不能代表删除 |
| 未知 key | Single／Multiple／Strict 可显式接受待解析值；Cascade 只接受已知规范叶子值，拒绝未知 key 和把分支标签当值回写 |
| 失败 | 重复 key、模式不符、非法结构等返回 typed 错误，旧值与会话完整保留 |

未解析值有快照时用快照，否则显示 key 并标注“待解析”；不伪造节点、祖先或叶子属性。路径不完整时显示已知标题并提供未解析说明，不拼出假的完整路径。

远程结果缓存仅保留当前结果与当前已选项快照；不无限保留历次查询整棵树。业务要求选中对象仍存在、路径完整或权限有效时由 Form 规则校验，组件不能由一页搜索结果推断全集。

## 5. 勾选、回填与删除

完整级联／禁用合同以 [Tree 第 6 节](37-tree.md#6-复选框级联与回填策略) 为唯一来源；TreeSelect 不重新解释半选、禁用边界或异步未知子树。

- Checked(Cascade) 默认标签投影 Parents，可改 All / Leaves；切换只改展示，不改变提交值或发送 Change。
- Checked(Strict) 标签始终是独立值；父子不会因为 Parents 配置而被压缩隐藏。
- 表单、value reader 和 Change 永远返回规范值。例：P 下 A、B 全选，Parents 显示 P 一个标签，提交仍是叶子 A、B；All 可显示 P、A、B，但不是新增了第三个业务值。
- 删除 P 标签取消其传播范围内可修改的叶子；删除 All 中的 A 则只取消 A，P 随即半选并从全勾投影消失。半选 P 不作为独立标签。
- 禁用边界外的选择和锁定值不被父标签删除；锁定叶子必须有独立可读标签／说明，不能被一个可关闭父标签藏住。
- 清空取消所有可修改值，保留锁定值；实际值变化只发一次 Change。全锁定时不显示有效清空动作。
- max_selected 首版仅用于 Multiple／Strict，按规范 key 数量计算；到上限仍可移除。Cascade 不提供含糊的“标签数等于选择数”上限，业务可用 Form 规则约束叶子数量。

折叠与选择上限不同：默认最多展示 2 个投影标签，宽度不足继续折叠为 +N，N 统计隐藏展示标签数。需要得知实际叶子数时提供独立说明，不能把两个数字混用。

删除／清空都复用树控制器的原子操作，不能先删除 Tag 再尝试同步树。Tag 的 on_close 阻止触发字段开合，关闭后的焦点回到邻近标签或 trigger。

## 6. 字段结构与尺寸

```text
字段：[前缀] [单值／Tag…／+N／占位符] [清空] [箭头／加载]
                     ↓ Anchor
Popup ┌──────────────────────────────────────┐
      │ [可选 YueInput 查询]                 │
      │ [可选 header]                        │
      ├──────────────────────────────────────┤
      │ [箭头] [Checkbox] [图标] 节点标题     │
      │    [箭头] [Checkbox] [图标] 子节点    │
      │ … 单一树视口                Scrollbar│
      ├──────────────────────────────────────┤
      │ 加载／失败重试／说明／可选 footer    │
      └──────────────────────────────────────┘
```

- 三档 Small / Medium / Large，字段最低高度复用 Input 的 28 / 32 / 40px，Tag 对应现有 20 / 24 / 30px。树行默认跟随同档 TreeSize。
- 默认 Outline，可选 Text；Text 仍预留边框和焦点环，不能因无边框失去键盘反馈。颜色、内边距、字体、图标沿用现有字段零件。
- 默认字段宽度复用 Select 的 240px，显式 block 才铺满；内容区 min_w_0，操作区不收缩。固定／填满／内容宽度为互斥策略，不累加覆盖。
- 默认面板 MatchTrigger，树视口最大高度复用 Select 的 288px；同时受 Popup 可用空间限制。可以显式指定面板宽度，但不可超出窗口碰撞边界。
- 树面板垂直空间先扣除搜索／header／footer，再交给唯一树视口；不让整个 Popup 与树各有一条纵向滚动条。
- 深层节点与长路径按 Tree 的缩进／省略合同；不让选项标题撑宽 Popup。默认显示 Label，Path 为显式选择。
- 标签默认 Collapse，可选 Wrap；Wrap 改变字段高度时同步 Anchor，尾部操作保持第一行中心，不覆盖下一行 Tag。
- prefix_text 是上下文提示，不是字段名称；字段名仍来自 YueField 或 accessibility_label。图标复用 Tabler，不新增树专属 SVG 集。

搜索输入放在 Popup 顶部，与当前 YueSelect 保持一致，直接使用现有 YueInput。trigger 始终显示已提交值；首版不同时增加内联 query／Tag 编辑器／自定义任意 trigger 三套布局。

需要新增树字段 token 时，在邻仓 Yue-Design-System 维护来源并同步；相同的 field／Tag／Popup token 直接复用，不能复制一份仅改前缀的色板。

## 7. 打开、关闭与提交

TreeSelect 采用即时提交，不引入 DatePicker 式候选确认事务。

| 动作 | 结果 |
| --- | --- |
| 点击字段／关闭态 Enter、Space、Alt+Down | 可编辑时打开；优先聚焦 query，否则树 |
| 打开定位 | 优先 reveal 当前已解析选中项；无选中则第一可用项；未知初值不触发无界加载 |
| Single 选择 | 更新值，发一次 Change，再关闭并恢复 trigger 焦点 |
| Multiple／Checked 选择 | 更新值并发 Change，弹层保持，query 保留 |
| 点同一 Single 项 | 值不变，不发 Change；关闭 |
| Escape | 先关最上层子 Popup，再关主 Popup；保留已经提交的值 |
| 外部点击 | 关闭且保留值；不抢回用户刚点击的外部焦点 |
| Tab／Shift+Tab | 走显式内部焦点序列，越界后关闭并交给 Root 向前／后移动 |
| 关闭 | 清 query，清临时过滤，恢复持久展开；查询票据失效，关闭动画由既有 Presence 持有 |
| disabled／readonly 动态变为 true | 立即关闭、阻止新的修改与延迟提交；语义状态当帧更新 |

query 清除不等于字段清空，query 不是值。主 Popup 的退出保留期间不接受旧树的点击提交；不能出现视觉已关闭却仍可选中的帧。

## 8. 本地搜索、远程结果与懒加载

### 8.1 本地搜索

Local 复用 Tree 的过滤：label／keywords、自定义纯过滤函数、保留祖先路径、临时展开、清除恢复。只搜索已加载节点；搜索期间勾选分支依然作用于完整已知模型，示例明确说明隐藏子项也可能受影响。

搜索不会自动加载全部 Unloaded 分支。无结果时用紧凑 YueEmpty 显示“未找到匹配节点”，不清除原有选中值；未查询的真正空树显示“暂无选项”。

### 8.2 远程搜索

Remote 为宿主驱动，首版支持 Single / Multiple / Checked(Strict)。Checked(Cascade) + Remote 配置返回明确错误：查询结果常是部分子树，无法安全推导规范叶子集合或“全选分支”。完整树 Cascade 可使用 Local；服务端全集子树选择协议另行设计。

- 非 composing 的有效文本更新按默认 200ms 防抖发 QueryRequested；空 query 也有自己的结果集，不隐式等于关闭。
- ticket 绑定 state、打开会话、query generation 与请求序号；接受最新匹配票据，旧响应丢弃。
- 宿主返回带完整已知祖先链的 TreeNode 结果树；单页内 key 唯一、路径一致。只返回散乱叶子时宿主先适配，不由组件猜父级。
- `apply_search_results(ticket, nodes)` 使用保留缺失选择的更新策略；不会把此次缺席的已选 key 当删除。结果替换后 active 按 key 修复，不按行号保留。
- 请求进行中旧结果可保留显示，但禁用提交和节点加载，防止用户把旧 query 项误当新结果；仍允许关闭、编辑 query 和移除字段标签。可用父级明确的“正在搜索”状态提示。
- 失败保留已选值和查询，显示可访问的重试按钮；当前失败结果不冒充成功无结果。重试创建新票据。
- 旧结果从新请求开始到当前请求成功前均不可提交，包括失败后的重试等待；错误面板不能重新启用上一 query 的候选。
- 清 query 请求基准结果；关闭／更换搜索模式／完整 set_nodes／不同 set_value 使旧查询票据失效。结果错误不使用全局 Toast 代替面板反馈。

首版每次返回一个完整结果快照，不增加滚动分页协议。大型搜索由宿主限制候选集并提示缩小范围；需要分页时另定带路径合并和去重规则，不能直接搬 Select 的平铺分页合并。

### 8.3 节点懒加载

节点 LoadRequested 直接复用 Tree 的 ticket，不另造 TreeSelect children 请求状态。Remote 结果发生替换时，对应结果模型 generation 改变，旧节点加载返回失效。

普通关闭不取消已知模型的节点加载，结果可缓存，但不能重新打开面板／提交值；销毁、reload、节点删除重建使票据失效。Cascade 的未知分支批量勾选限制严格继承 Tree；Single / Multiple / Strict 不受该级联限制。

## 9. 键盘、焦点与 IME

树中的 Up / Down / Left / Right / Home / End / Page 键直接复用 Tree。箭头导航只改 active，绝不自动提交字段；Single 以 Enter／Space 或点击提交，Multiple／Checked 以同样入口切换。

| 焦点位置／按键 | 行为 |
| --- | --- |
| 关闭态 trigger 的 Down | 打开并聚焦树中的已有活动／选中项 |
| Query 的 Down／Enter | 非 composing 时交给树；第一次只迁移焦点，不同时选中 |
| Query 的 Left／Right／Home／End／Space／Ctrl+A | 保留真实文本编辑行为，不触发树动作 |
| 树的 Ctrl+F | 有搜索时返回查询框 |
| 树的 Escape | 关闭当前最上层 Popup；不清字段值 |
| 字段 Backspace | query 不在编辑且可删除时先聚焦最后可关闭标签，第二次确认删除；不删锁定项 |
| 标签 Delete／Backspace | 聚焦标签且允许时删除对应值／分支覆盖范围 |
| +N 的 Enter／Space | 打开关联已选项面板，支持键盘查看与移除隐藏标签 |
| Tab／Shift+Tab | query → 登记的 header 控件 → 树停靠点 → footer 控件；边界交给 Root |

树内 Checkbox 不新增独立 Tab 停靠。TreeSelect 的节点 extra 首版只接受非交互装饰，命令放 header／footer，避免字段内部变成难以退出的节点编辑器；独立 Tree 的行操作模式不自动启用。

IME composing 时 Enter／Escape 和候选键不得提交／关闭／切换树，防抖远程请求等组合结束后再发。按真实 InputState 路径仲裁，不用“最近一次键盘 key”猜测组合状态。

主 Popup 挂在既有 Root 下；Dialog 中的字段继承 modal-aware 遍历和父子 Popup 路由，不创建自己的 focus trap。header／footer 交互内容必须显式登记 FocusHandle，删除时修复焦点，不能仅附任意 AnyElement 后假定 Tab 可用。

## 10. 字段状态、表单与可访问性

| 状态 | 行为与反馈 |
| --- | --- |
| disabled | 禁用配色、移出 Tab、不能打开／清空／删标签；已有值仍可读 |
| readonly | 保留 Tab 停靠与可读值，不打开、不修改；不把 readonly 当 disabled 灰化处理 |
| loading | 字段尾部 loading；默认仍可打开和关闭、可移除已有可修改值；候选是否可提交按当前请求状态决定 |
| error | 外框与错误说明，焦点仍可辨，不隐式禁止选择 |
| open / focus | 分别表达展开与实际焦点，不能关闭时留下伪 focus |

disabled 优先决定交互；readonly 决定写入许可；error 和 loading 可并存。全局 loading、远程搜索 loading、单分支 loading 分开建模，不能用一个 bool 覆盖全部请求状态。

新增拟议适配：`YueField.tree_select(...)`、`YueForm.require_tree_select(...)`、`rule_tree_select(...)`。必填读取规范值是否为空；错误聚焦字段 trigger；自定义规则读取 typed value 和解析状态，不读格式化标签字符串。Cascade 半选父节点本身不是值，但已有叶子可使必填成立；严格权限校验仍由业务负责。

- 字段使用 Base Select 的 ComboBox 根，提供名称、值、expanded、required／invalid／disabled 等状态；Popup 的主要内容为 Tree，不能继续误报 ListBox。
- “拥有 Tree 弹层”的关系、活动 TreeItem、层级、checked／mixed 由实际树语义提供；如 Base Select 当前只表达默认弹层类型，应在 Base 补 typed popup kind，不在 Yue 假装已有能力。
- 已选值的可读文本与字段名分开；Parents 标签可说明覆盖的已选叶子数量，避免一个父标签被误读为只选择一个对象。
- +N 是真实可操作按钮，交互面板使用 Popup；Tooltip 只用于无交互短说明。
- 清空、关闭标签、重试、箭头都有合适名称，装饰图标不重复朗读；焦点路径不依赖图标本身获得焦点。
- 虚拟树的离屏可访问能力遵循当前 GPUI／Windows 平台边界，不新增独立 COM provider。真实 Narrator 和 IME 验收由用户执行，设计稿不宣称通过。

## 11. 自定义边界与 API 草案

| 分类 | 拟议入口 |
| --- | --- |
| state | new(mode, window, cx)、set_nodes、set_value / value、set_selected_items、set_disabled / set_readonly / set_loading |
| 树 | expanded keys、reveal、node load ticket 交付、show_lines、show_icons、选择范围策略 |
| 搜索 | set_search、set_query / query、apply_search_results、reject_search、retry_search |
| 外观 | size、variant、theme、placeholder、prefix_text / prefix_icon、width / block、panel_width、max_height |
| 已选展示 | value_display、check_projection、tag_layout、collapse_count、clearable、max_selected（适用模式） |
| 插槽 | render_label、render_extra（非交互）、render_value、render_tag、header / footer 与其 focus handles、empty |
| 语义／表单 | accessibility_label、accessibility_description、status、required |

render_label／extra 只替换行内部内容，保留 Base 行根、固定行高、语义与动作。自定义 Tag 接收稳定 key、可关闭状态及标准删除请求；不能回调修改私有集合。render_value 仍须提供可读文本，不能仅返回一个无名称图标。

首版不提供任意 trigger 替换和任意 popupProps／treeProps 透传；明确入口足以覆盖参考中的前后缀、尺寸、标签和树装饰，同时保护焦点和角色合同。

事件拟为 `TreeSelectEvent`：

- Change：规范 before／after、trigger key、Pointer／Keyboard／Accessibility／TagRemove／Clear 来源；实际值改变才发一次。
- OpenChanged(bool)：当前 open；Single 提交时先完成 Change，再通知关闭。首版事件不附关闭原因。
- QueryRequested：远程 query 和不可伪造搜索票据；本地 query 通过保留的 InputState 观察，首版没有独立 QueryChanged 事件。
- LoadRequested：直接转交同一树 owner 的加载请求；不复制票据或再次发起任务。

setter 静默；加载／label 解析／投影切换／过滤／方向导航都不是用户值变更。先将状态更新到 after，再发送事件；订阅必须由宿主保留。

## 12. 虚拟化与性能约束

TreeSelect 使用相同 Tree 投影和固定行高视口，不递归创建整棵树的 element，也不逐行建立 entity。列表滚动 owner 只有一个，Popup 只负责容器约束。

- 模型变更时建立 key／parent 索引；展开／过滤变更时更新投影；滚动帧只处理可见范围及 overscan。
- active、Tag 和语义身份使用 key，结果换序或折叠不会串项；滚动定位使用模型坐标，不手算 y 偏移。
- Tag 测量缓存与字体／尺寸／宽度／标签版本关联；窗口变窄后重算折叠，不能在每个滚动帧测量所有已选 label。
- 勾选分支的成本可与受影响子树大小相关，但必须在一次状态更新中完成，不能为每个叶子发送一次 Change／notify。
- 打开／关闭不深克隆完整树；保留必要模型，关闭释放 Popup 挂载和会话资源。远程缓存按当前结果与已选项限定。
- 首版不支持任意变高富内容行；需要时单独验证现有 measured virtualization、语义和定位，再扩展行合同。

参考中的虚拟列表阈值不构成 Yue 性能结论。实施前后使用相同一万节点数据、相同可见投影和构建配置，对比打开、过滤、展开、级联和滚动分配；记录实际结果再决定局部优化，不以功能数量提高评分。

## 13. Showcase、测试与实施顺序

以下为设计验收场景清单；首版已提供独立 **组件 → TreeSelect** 页面，实际示例见第 14 节，未覆盖的实窗和性能场景仍待验收：

1. 组织／分类单选、LeavesOnly、路径回填、三尺寸、Outline／Text 与状态。
2. 无 Checkbox 多选、折叠／换行、+N 查看与删除、锁定值、清空与上限。
3. Strict／Cascade，对比 Parents／All／Leaves，现场显示“标签投影”和“规范提交值”。
4. 本地搜索、祖先路径、无结果、清除恢复和隐藏后代勾选提示。
5. 远程查询竞争、延迟、失败重试、未知初值、label 解析和路径变化。
6. 懒加载并发、空成功、过期返回；Cascade 未知分支不能批量选择的解释。
7. Form 必填／自定义规则、Dialog 内焦点、header／footer 控件、一万节点与长标签。

| 验证 | 必须断言 |
| --- | --- |
| 单值／多值 | 方向键不提交；单选关闭、多选保持；同值不发 Change；一次动作一次事件 |
| 级联／标签 | 删除父标签只影响覆盖范围；锁定值保留；切投影不变值；半选不进入规范值 |
| 数据同步 | authoritative 删除与搜索缺席不同；未知初值／路径快照；非法更新原子拒绝 |
| 竞态 | A/B 查询乱序、关后返回、重开新会话、节点删除重建和不同模型加载全部被票据正确区分 |
| 焦点／输入 | 真实按键、composition、Tag 删除、+N、Tab 边界、外部点击、Dialog 和退出动画期间不提交 |
| 语义 | ComboBox → Tree → TreeItem，checked／mixed 与 level 一致，无重复 ListBox／Checkbox Tab 停靠 |
| 共享回归 | Select 原有字段、折叠、Tag、Popup 与表单测试保持；独立 Checkbox 不因抽取 mark 改行为 |
| 布局／性能 | 窄字段、Wrap Anchor、上方翻转、有限高度、长路径、虚拟回收、批量事件及分配比较 |

先完成 Tree 文档规定的 Base 增强与树核心；再抽取共享字段零件并回归 Select；随后实现 TreeSelect 单选／多选、Checked、搜索／异步、Form 与 Showcase。本轮改动保留在工作区，未自动提交或推送；工作区同时存在其他组件的既有修改，后续提交需按范围整理。

Windows 实窗视觉、真实 IME、Narrator、当前设备 DPI 和输入设备验收由用户负责；自动交互测试已执行，详见下节；实机性能采样未执行。本文保留设计合同，同时以实际 API 章节说明实现选择。

## 14. 首版实现与用法（2026-11-09）

源码：[入口](../crates/yue-component/src/tree_select/mod.rs)、[状态](../crates/yue-component/src/tree_select/state.rs)、[字段共享零件](../crates/yue-component/src/choice_field.rs)、[Showcase](../examples/windows-showcase/src/pages/tree_select.rs)。

```rust,ignore
let categories = cx.new(|cx| {
    let mut state = YueTreeSelectState::new(TreeSelectMode::Single, window, cx).unwrap();
    state.set_nodes([
        TreeNode::new("docs", "文档").children([
            TreeNode::new("guide", "使用指南"),
            TreeNode::new("api", "API"),
        ]),
    ], cx).unwrap();
    state.set_search(TreeSelectSearch::Local, window, cx).unwrap();
    state
});
YueField::new(theme).label("分类").tree_select(
    YueTreeSelect::new("category", &categories).clearable(true),
);
```

构造器返回 `Result`，拒绝 `Checked(None)`；Remote + Cascade 返回 `TreeDataError::Mode`。`value(cx)` 从同一树 owner 返回 typed 快照，Single / Multiple / Checked 的 keys 是真实提交值；Parents／All／Leaves 仅影响展示。没有第二个平铺 SelectState，也没有独立树引擎。

`set_expanded_keys`、`expanded_keys(cx)`、`reveal`、`set_node_icon` 和懒加载交付转给内部树。`set_max_selected` 适用于 Multiple／Strict，`set_leaves_only` 适用于 Single／Multiple。搜索使用 `set_search`、`set_query`、`query(cx)`、`set_filter`、`set_search_debounce`；远程事件携带 ticket，必须用 `apply_search_results`／`reject_search` 交付。

`set_value` 同值回写保持会话，不同值关闭并清 query；两者都不发用户 Change。完整 `set_nodes` 会清理确认删除的值；查询结果替换保留缺席值及快照。`set_selected_items` 可补未解析项的标题、路径和锁定标记；清空及 Tag 删除尊重锁定快照。`TreeSelectChange::trigger_key()` 在清空时为 None，其余用户入口携带对应 key。

插槽已提供 `render_label / render_extra / render_value / render_tag / empty / header / footer`。`render_tag` 装饰组件提供的 YueTag，需保留其关闭回调与 focus handle。header／footer 回调接收 `(interactive, window, cx)`，退出保留期间必须禁用控件；交互项需登记 focus handles 并设 `tab_stop(false)`。实际槽位高度经布局测量，再扣除树视口预算。`SelectPanelWidth::Content` 测量已知可见节点标题与层级槽位，按模型版本、字体和尺寸缓存，并受窗口碰撞约束；任意自定义 renderer 的自然宽度不参与文本估算，可显式指定 Fixed。

字段使用 Base ComboBox，新增 opt-in `popup_kind(HasPopup::Tree)`，树内部为 Tree / TreeItem，不套 ListBox 或独立 Checkbox 语义。`+N` 面板与主面板互斥，键盘可查看和移除隐藏标签。查询输入用专属 GPUI actions 在 Base Input 前处理 Enter／Escape／Tab，组合状态读真实 InputState 的 marked range。

树内开合默认复用 YueTree 的箭头旋转与子树收展，完整策略、128 行预算和 reduced motion 边界见 [Tree 动效](37-tree.md#91-动效)；字段值和事件仍在动作发生时立即更新，不等待动画结束。

Field 与 Form 接入为 `YueField::tree_select`、`YueForm::require_tree_select`、`rule_tree_select`；规则读取 typed value，错误聚焦字段。Showcase 覆盖三尺寸、单／多／Strict／Cascade、本地／远程查询、乱序与失败、换行、只读／禁用、LeavesOnly、数量上限、Text 面板插槽和 Form。

自动验证及完整 token 审计的既有阻塞见 [实施记录](research/2026-11-09-tree-implementation.md)。真实 Windows 视觉、IME、Narrator 和 DPI 由用户验收；未引入额外 Windows COM provider。首版未提供 Tag 拖排、节点拖放、分页结果合并或任意可变行高。
