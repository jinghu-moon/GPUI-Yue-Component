# 20 Select 下拉选择器设计

状态：首版已实现（2026-11-02）。基于本仓库锁定的 `gpui-kit 0.8.0`、受维护的 `gpui-base 0.8.0`、`gpui-pre 0.3.9`。实现与无头回归见第 19 节；Windows 实窗、真实 IME、DPI 和 Narrator 仍待用户验收，第 17 节的实窗清单不表示已经通过。

## 1. 结论与目标

采用 **Base Select 语义根 + Yue Select 状态与呈现 + 现有 YueInput / YueTag + 既有 Popup 基础层**。搜索直接组合项目现有 YueInput，不重写输入框或文本编辑状态；不复用 Dropdown 的菜单状态机，不新建第二套 Positioner、浮层注册器、退出动画或应用 Root。

组件解决“从具有稳定身份的候选集合中选择一个或多个字段值”。选中值、正在编辑的搜索文本、键盘活动项和 Popup 的存在性是四件事，必须分开建模。默认是非搜索单选；搜索和多选是显式能力。

关联文档：[Input / Form](03-input.md)、[Dropdown](07-dropdown.md)、[Tooltip](10-tooltip.md)、[Popup 基础层](12-popup.md)、[Tag](19-tag.md)。参考用户提供的 Select 页面能力及 [Element Plus Select 官方文档](https://element-plus.org/zh-CN/component/select.html)，借鉴场景覆盖，不照搬 Vue API、DOM 挂载选项或网页默认尺寸。

### 1.1 首版能力范围

| 能力 | 设计决策 |
| --- | --- |
| 单选 / 多选 | 都支持；模式在 state 创建时确定，选项以稳定 key 关联 |
| 三尺寸 | Small / Medium / Large；与 YueInput 的字号、高度及焦点语言一致 |
| 字段状态 | 正常、hover、focus、open、disabled、readonly、loading、error |
| 清空 / 删除 | 单选清空，多选删标签和清空全部；保留禁用选项的锁定选择 |
| 分组 / 自定义选项 | 组标题、图标、说明、末端元信息与自定义非交互内容 |
| 搜索 | 本地 label / keywords 匹配、自定义过滤、宿主驱动的远程结果 |
| 创建选项 | 明确的创建请求；宿主决定 key，组件不把任意文本静默变成值 |
| 多选标签 | 显示、折叠、数量上限、折叠项查看；复用源码已有的 YueTag |
| 浮层 | 默认下方起始侧、碰撞翻转、宽度策略、可用高度、圆角与统一退出 |
| 大列表 | 固定行高虚拟化、稳定活动 key、模型坐标滚动定位 |
| 表单 / 可访问性 | 必填、校验描述、字段焦点、ComboBox / ListBox / ListBoxOption 语义 |
| 诊断与验收 | Showcase 独立页，真实事件测试；IME、UIA / Narrator、DPI 单独实窗验收 |

### 1.2 组件边界

- Dropdown 的选项执行命令，Select 的选项改变字段值；不把菜单 check 状态当作 Select 的选择模型。
- 搜索输入是 query，不是提交值。首版只接受候选 key；创建也先向宿主请求，再用宿主确认的 option 提交。
- 树形选择、级联选择、任意文本自动补全和可拖动标签排序不进入本组件；它们分别需要自己的数据和键盘合同。
- Tooltip 负责短帮助；带滚动、删除或其它交互的折叠标签面板使用 Popup，而不是把交互内容塞进 Tooltip。

## 2. 已核实的源码基础

| 当前来源 | 实际提供 | 本组件仍须负责 |
| --- | --- | --- |
| `vendor/gpui-base/src/select.rs` | 受控 `Select`、ComboBox role、expanded / value、trigger/content FocusHandle、Confirm / Cancel 和开合请求 | 候选集合、选择值、active key、方向键移动、搜索、选项角色、列表及皮肤 |
| `vendor/gpui-base/src/combobox.rs` | 复用 Select 的另一键盘上下文和回调入口 | 不提供现成的搜索引擎、远程请求、选择集合或已发布的 Base SelectState |
| `crates/yue-component/src/popup/` | Anchor、PopupSpec / Host、定位、Presence、退出保留、父子关系和关闭路由 | open 真值、焦点目标、确认与取消的语义 |
| Base `Positioner` / `virtual_list` | 当前布局定位、受维护约束接口和虚拟视口工具 | listbox 内容、固定行高、active 项定位及测量失效 |
| Base `InputState` / YueInput | 直接作为搜索框使用；已有 SearchInput 语义、三尺寸、前缀图标、清空、主题和输入外观；编辑与 IME 继续由其 Base state 承载 | query 会话、搜索防抖、选项导航与 IME 确认仲裁；不另造 SelectInput |
| `src/tag.rs`、`src/tag/element.rs` | 已导出的 YueTag / YueTagGroup，含独立 on_close、close_label、close_tab_stop、三尺寸与溢出提示 | Select 拥有删除规则、折叠、值顺序及焦点恢复 |

Base Select 的 `.on_confirm()` 只在打开时运行；关闭态 Confirm 请求打开。Cancel / 无障碍关闭会先通知 dismiss，再请求关闭并恢复 trigger focus。Base 当前不是一个完整的 styled Select，也不是 GPUI Component 层的 SelectState。后续不得从在线新版本示例直接推断 0.8.0 支持的接口。

## 3. 责任与依赖

```text
宿主：选项业务数据 / 网络与取消 / 持久化 / 表单规则
                 ↓ options、selected keys、查询结果
YueSelectState：一个选中值真值 + query / active key / 请求世代
                 ↓
Base Select：ComboBox 根语义 / 键盘开合 / FocusHandle 桥接
                 ↓
YueSelect：token 外观 / 现有 YueInput 搜索框 / 选择列表 / YueTag 组合
                 ↓
既有 Popup：Anchor → Positioner → Presence → Runtime 路由
```

公共组件采用保留 `Entity<YueSelectState>` 的 `RenderOnce` 外观；宿主持有 entity 和订阅。Popup 仍为 crate-private，不新增公共 YuePopup，不启用 `gpui-kit/component` 或另一 GPUI 后端。选项导航与值变更作为 Select state/controller 的明确定义，不藏在行渲染闭包里。

## 4. 参考图映射与默认配置

原图留档：[Select 参考图](assets/select-reference.png)。采用功能与信息结构；不复制网页演示卡片、覆盖率、配置侧栏和贡献者区域，也不按缩放图片估算像素。

| 参考图场景 | Yue 首版方案 |
| --- | --- |
| 单选、多选、禁用选项 | Single / Multiple，稳定 key，逐项 disabled |
| 分组 | 单层组标题与组内选项；组本身不可选，不引入树形选择 |
| 普通、禁用、加载、不同尺寸 | 独立字段状态和 Small / Medium / Large |
| 自定义选项、选中项、顶部和底部 | 保留语义和行为的内容插槽，见第 13 节 |
| 折叠选中项、自定义折叠项 | 按数量或可用宽度折叠，+N 打开已选项子面板 |
| 过滤、远程搜索、创建新条目 | 显式 SelectSearch、带世代的结果交付、CreateRequested |
| 限制可选数量 | 多选 max_selected；到达上限仍能取消已有选择 |
| 前缀图标、前缀文字 | Tabler Icon、静态 prefix_text；不把业务前缀当占位符 |
| 文字 / 无边框选择器 | SelectVariant::Text；保留 hover 与键盘焦点，不另造 Button Select |
| 已选值为对象 | 组件只保存 key，事件附选中项快照；宿主通过 key 映射业务对象 |
| 自定义宽度 | trigger 与 panel 独立策略；不默认铺满卡片 |
| 滚动加载、虚拟滚动 | 带 ticket 的 LoadMoreRequested + 模型坐标虚拟列表 |

搜索与图片有一个明确差异：**首版搜索框位于 Popup 顶部，直接使用现有 YueInput**，关闭时 trigger 始终显示已提交的选择。复用 YueInput 与搜索框放在哪里是两个决定；现有 Input 并不限制必须放在弹层内。此处采用弹层顶部布局，避免在字段内同时放查询光标、Tag、清除和开合四种竞争交互。暂不增加第二套“trigger 内联搜索”模式。

| 配置 | 默认值 |
| --- | --- |
| mode / size / variant | Single / Medium / Outline |
| placeholder | “请选择”，可由调用方替换 |
| trigger 宽度 | --select-width，240px；不 flex_grow、不隐式 w_full |
| panel 宽度 / 列表最大高度 | MatchTrigger / --select-list-max-height，288px，并受窗口可用空间约束 |
| search / clearable / creatable | None / false / false |
| 标签布局 | Collapse，最多显示 2 项，宽度不足继续折叠；可改 Wrap |
| max_selected | None，不限制；仅适用于多选 |
| 远程防抖 | 200ms 的可配置行为参数，不做设计 token |
| 方向键边界 | 不循环，停在首 / 尾可用项 |
| 保留查询 | 同一打开会话内多选后保留；关闭清空；不新增永久保留查询开关 |
| 虚拟化 | Auto，投影行数超过 100 使用虚拟化；阈值为实现参数，可配置 |

## 5. 结构、尺寸与外观

```text
字段外壳： [可选前缀] [已选文本 / Tag… / +N / 占位符] [清空] [箭头或加载]
                         ↓ 现有 PopupAnchor
Popup： ┌─────────────────────────────────────────┐
        │ 可选查询输入框                         │
        │ 可选固定 header                       │
        ├─────────────────────────────────────────┤
        │ 组标题                                │
        │ [图标] 选项标题              [选中标记] │
        │        可选说明                        │
        │ … 只有此区域滚动                      │
        ├─────────────────────────────────────────┤
        │ 加载更多 / 错误重试 / 可选 footer       │
        └─────────────────────────────────────────┘
```

### 5.1 字段布局

- 三尺寸单行最低高度直接使用 `--input-height-sm/md/lg`，当前分别为 **28 / 32 / 40px**；字体、前后内边距、图标尺寸、边框与焦点环沿用 Input token。
- 文本左对齐、垂直居中；选中项、占位符使用相同基线。图标、Tag 关闭区、清空区、箭头区均不收缩，只有文本区域允许省略。
- 选择框与弹层显式采用左对齐。纯文本选项从行内边距开始，不为不存在的图标留空列；当前结果集含图标时才为所有普通选项保留统一图标列，选中标记始终位于右侧。普通列表和虚拟列表遵循同一规则。
- 采用正常 flex 槽位分配宽度，文字 `min_w_0`；禁止绝对定位清空图标覆盖长文本，禁止修改高度来模拟按压动画。
- 多选 Tag 按尺寸对应 Small / Medium / Large。当前 Tag 高度 20 / 24 / 30px；分别与字段的 28 / 32 / 40px 检查边框、上下余量。装不下时调整 Select 容器内距或最低高度，不缩放文字。
- Collapse 保持单行；Wrap 可增高，箭头区固定于第一行垂直中心，Tag 区独立换行。宽度或 Tag 行数改变后当帧更新 Anchor，而不是留下旧的 Popup 位置。
- 默认宽度 240px，可传 width / min_width / max_width；block 显式填满父级，auto_width 按当前显示内容测量并受 min/max 约束。三者代表同一个外宽策略，后设置者替换前者，不同时叠加。
- 过窄时先省略文本、折叠 Tag，保留必要操作区域；若连最小操作宽度都不足，外壳保持有效最小宽度，由父布局处理溢出，不让命中区互相覆盖。
- `prefix_text` 是可见静态前缀，例如“产品：”；真正字段名仍由 YueField / accessibility_label 提供。前缀图标是装饰，不额外获得焦点。

### 5.2 两种外观

| 外观 | 常态 | hover / focus / open |
| --- | --- | --- |
| Outline | Input 背景、边框与圆角 | Input hover；focus 为可见外环；open 保持活动边界 |
| Text | 透明背景与透明预留边框，内容宽度可选 | 轻量 hover 填充；键盘焦点仍有外环，错误仍可见 |

Text 不等于移除上下 padding；两种外观相同尺寸的内容基线与命中高度一致。切换状态不增减边框厚度，不改变文字栅格位置。

### 5.3 状态叠加规则

| 状态 | 字段表现与行为 |
| --- | --- |
| disabled | 禁用配色、无可编辑动作、无 hover / 按压反馈、移出 Tab；已聚焦后禁用也不能操作 |
| readonly | 保留可读值和键盘停靠，可复制文本；不打开选择面板，不清空、不删除 |
| loading | 尾部旋转标记与“正在加载选项”；默认仍能打开、查询、关闭和移除已有值 |
| error | 错误边界与描述；不靠红色单独表达，不隐含禁用 |
| open | 展开箭头和活动边界；不伪装成 focus，关闭后按实际焦点恢复视觉 |

优先级：disabled 决定交互，readonly 决定是否允许写入；error 与 loading 可以并存。error 的边界高于普通 hover；焦点环始终可辨。远程列表 loading 不等于整个 Select disabled，宿主若需冻结字段必须显式 disabled。

## 6. 数据与状态合同

以下类型为拟议 API，字段保持私有，使用构造器、builder 和 reader；枚举只表达不可混淆的状态。

| 类型 | 内容与约束 |
| --- | --- |
| SelectMode | Single / Multiple，创建 state 时固定，不能在 render 中切换 |
| SelectValue | Single(Option<SharedString>) / Multiple(Vec<SharedString>) |
| SelectOption | 必填 key / label，可选 disabled、leading_icon、description、keywords、meta |
| SelectGroup | 必填稳定 key / label，内部是 Option；不递归嵌套 |
| SelectEntry | Option 或 Group；禁止同一集合内跨组重复 option key |
| SelectChange | before、after、source、选中项快照；通过 reader 暴露 |
| SelectRequestTicket | state 身份、打开会话、query 世代及请求身份的不可伪造票据 |

key 区分大小写，非空；label 必须有非空可读文本。相同 label 允许出现，key 不同即为不同项；group key 在组之间也必须唯一，并使用不同于 option 的元素身份前缀。业务对象由宿主保存并用 key 索引，不为了适配 JS 的 valueType 复制一份泛型对象状态。

### 6.1 一个真值，多个投影

`YueSelectState` 持有 committed value、候选模型、已选项展示缓存、可选的搜索 `Entity<InputState>`、active_key、open、搜索结果状态与请求世代。entity 在宿主构造时创建；首次启用搜索时在 update 路径创建并保留 InputState 和订阅，不在 render 里重建。

query 的唯一文本真值是该 InputState 的 value，Select 的 query reader 读取它。过滤用的规范化文本、上次已提交查询与请求世代只是派生快照，不能成为另一份可编辑字符串。关闭清空通过现有 InputState 的更新路径完成；区分用户编辑与程序清空，避免额外发送远程搜索请求。

- 模式、选项、搜索模式、上限和交互许可保存在 state，不能在 builder 和 state 各留一份 disabled / readonly 真值。需要变化时用 state setter，setter 能同步关闭与失效任务。
- 外观 builder 负责 size、variant、theme、placeholder、宽度、Tag 折叠、内容插槽和可访问名称等呈现配置。
- `hovered_key` 与 `active_key` 分离：鼠标移入只改变指针反馈；真实移动鼠标才切换输入模式。键盘改变 active 后，静止鼠标不能在下一次重绘中抢回旧行。
- active 表示本次可确认的项，selected 表示已经提交的值。移动方向键不 emit Change，不改选择；单选不能用 hover 代替 selected。
- 内部滚动偏移、行测量是视图缓存；不是另一份候选或选择真值。一个 state 同时只挂载一个 Select 字段。

### 6.2 设置值、选项替换与缺失项

| 操作 | 必须遵守 |
| --- | --- |
| set_value，相同值 | 静默且无副作用；保留 open / query / active，支持宿主回写事件值 |
| set_value，不同值 | 静默同步；关闭会话、清空查询、失效票据，清理不存在的标签焦点；不 emit 用户 Change |
| set_options | 原子验证后替换，失效旧请求并重建投影；按 key 保留仍有效的 active；不按 index 保留 |
| 更改 label / 顺序 / 分组 | 选择 key 不变；刷新展示缓存，不 emit Change |
| 当前结果没有已选 key | 保留值与缓存标题；远程分页或搜索未命中不代表用户取消选择 |
| 程序设置未知 key | 明确接受为待解析选择；可同时提供已选项快照，没标题时显示 key 并提供未解析说明，不伪造 option |
| 重复 key / 模式不符 / 超上限 | 返回明确错误，旧 state 完整保留；不截断、不覆盖、不 panic 到一半 |
| 新上限小于当前数量 | 拒绝配置更新；宿主先显式调整值，组件不擅自删除已有选择 |

多选值维持选入顺序；重新选入的项排在末尾。重复 key 的 Multiple 值也是无效输入，不在事件中静默去重。已选项缓存只保留当前选中 key；不长期缓存全部历史远程结果。

缺失 key 既不自动失效也不自动通过业务校验。若业务要求“必须仍在可选全集中”，宿主提供规则或明确更新选择；组件无法从一页远程结果判断全集。

## 7. 选择、清空、标签与上限

### 7.1 单选

激活 enabled option 后立即提交 key，值真实变化时 emit 一次 Change，然后关闭并恢复 trigger focus。重复确认同一项关闭面板但不重复发 Change。未确认的 active 不在 outside / Escape / Tab 时提交。

### 7.2 多选

激活 enabled option 即切换已提交集合，emit 一次 Change，面板保持打开。无需另设“确定”按钮；Escape 关闭但不回滚已经完成的多选。可选项末端绘制勾选标记，不在整行内部再放一个独立 Checkbox 焦点和点击源。

`max_selected` 为 None 或正整数。达到上限时：

- 已选 enabled 项仍可取消、Tag 仍可删除；未选项显示“最多选择 N 项”，不接受新增。
- 上限造成的阻塞与业务 disabled 分开记录，取消一项后立即恢复。
- 不通过反复触发 Toast 解释限制；提示在字段描述 / 列表固定区域可见，键盘与鼠标行为一致。
- 创建请求也受相同上限约束，不能通过创建绕过。

### 7.3 清空与锁定

- `clearable` 控制是否显示整体清空入口；有可移除选择且未 disabled / readonly 才可用。hover 或 focus 时可见，布局始终预留空间，不造成箭头跳动。
- option 的 disabled 同时锁定其已选状态，禁止用户通过行、Tag、折叠面板或“清空全部”移除；外部 set_value 仍可更改。
- clear 保留锁定项；若没有可移除项，清空入口不出现。可访问名称在有锁定项时为“清除可移除的选择”。
- 缺失项沿用最后已知 disabled 状态；宿主刷新该 key 的快照才能更新锁定。未知 key 默认可移除。
- 多项批量清空只 emit 一次 Change。关闭按钮只执行删除，不打开面板、不冒泡成字段激活。

### 7.4 Tag、折叠与键盘移除

复用真实的 `YueTag::new(...).on_close(...).close_label(...)`，不重复实现 Tag 配色 / SVG / 关闭动画。Tag body 是静态文本，关闭按钮才是独立交互点。实际截断才启用 OverflowTooltip。

`SelectTagLayout::Collapse` 配合 `collapse_count(n)` 按数量折叠，默认 2 项；`SelectTagLayout::Auto` 按可用宽度折叠，Collapse 同样受宽度约束。始终为尾部操作区预留宽度；+N 按实际隐藏数计算。空间极小时只显示 +N，不渲染半个 Tag。

- +N 是按钮，点击或 Enter / Space 打开已选项子 Popup；其中按选择顺序列出隐藏项，可单项移除，锁定项没有关闭动作。
- 子 Popup 与主列表面板互斥：按 +N 时若主面板打开，先结束主会话，再打开折叠面板；它们都归当前 Select owner，嵌在其它 Popup 内时继承该父层。不得把主列表关掉后又把子面板挂到已经关闭的主列表 id。
- Escape 只关闭当前顶层并回到 +N；若删除后 +N 消失，回到 trigger。外部点击不抢焦点。
- Tag 关闭按钮和 +N 可用 Tab 到达；query/list 的临时焦点不加入全局 Tab 顺序。state 记录稳定关闭焦点，删除后聚焦下一个可移除 Tag，末尾则前一个，没有则 trigger。
- trigger 聚焦且多选时 Backspace 先聚焦最后可移除 Tag 的关闭按钮，不立即删除；关闭按钮 Enter / Space / Delete 执行移除。搜索输入中的 Backspace 始终编辑 query，不偷删 Tag。

## 8. 搜索、远程结果与创建

### 8.1 本地搜索

`SelectSearch::None / Local / Remote` 互斥。Local 默认：query trim 后与 label / keywords 做 Unicode 小写化后的子串匹配；不自动加拼音、模糊排序或网络依赖。空 query 显示完整候选；保留源顺序；无结果的组不渲染。description / meta 默认不参与匹配。

可提供纯函数 filter(option, query) 覆盖匹配；不得在 filter 或 render 中发请求、更新 state。中文与大小写规则有特殊要求的宿主自行替换 filter。

搜索框在弹层顶部固定，拥有独立可读名称“搜索选项”；列表节点的描述提供结果数，分页时明确是已加载数量，不把组标题或创建行计为选项；无匹配时显示空结果说明。关闭会话清空 query，下次打开不会残留不相关过滤。多选本次会话确认后保留查询；单选确认立即关闭。

#### 现有 YueInput 的直接组合

| 搜索框需求 | 复用项目已有能力 |
| --- | --- |
| 构造与文本真值 | YueInput::new(id, theme, &input_state)，使用 Select state 保留的 Base InputState |
| 搜索语义与名称 | input_type(InputType::Search)、accessibility_label、accessibility_description |
| 搜索图标与清空 | leading_icon 使用现有 Tabler Search；clearable(true)，清空产生的真实文本变化进入同一过滤 / 防抖路径 |
| 尺寸、主题和布局 | size 映射到现有 InputSize；跟随 Select 解析后的主题；block 只铺满弹层的搜索槽位，不改变外部 Select 宽度策略 |
| 输入编辑 | 光标、选择、复制粘贴、撤销与 IME 继续由 YueInput 所用 Base InputState 处理 |
| Select 特有交互 | 在组合边界处理方向键、Tab、Escape 和结果确认；保留 Input 的普通编辑行为 |

不新增 YueSelectInput，不手绘搜索框，也不只拿 Input token 后复制一套输入实现。查询 Input 不绑定 Form.attach，它收集搜索词，Form 校验的是 SelectValue。

关闭态 trigger 的职责是展示选择并开合，语义是 ComboBox；多选还包含 Tag 与独立删除动作，因此不把它强行包装成一个 readonly YueInput。trigger 与 YueInput 若在实施时确认存在重复的外壳绘制，提取现有 input 模块的最小私有 frame / metrics helper 共同使用；不增加第二个公开 Input 组件，不为只展示 label 再维护一个 InputState。

### 8.2 远程搜索与加载更多

网络、Task、超时与重试业务由宿主拥有。组件只发事件并验收结果，不能自己选 HTTP 客户端、阻塞 render 或在 Drop 后访问 Window。

```text
已提交 query 改变
  → 立即增加 query 世代，旧票据失效
  → 防抖到期且会话仍有效 → SearchRequested(query, ticket)
  → 宿主持有并取消旧 Task
  → state.apply_page(ticket, page)
  → 身份 / 会话 / 世代 / 请求均匹配才原子接收
```

- 开启 Remote 面板时对空 query 也请求一次；输入期间立即撤销旧候选的可选资格，不能让旧搜索结果被误确认。已提交的 Tag / 文本不因此清空。
- 列表区有 InitialLoading / Ready / Empty / Failed；追加页另有 idle / loading / failed。首次失败可重试；追加失败保留当前结果并在底部重试。
- `SelectPage` 包含条目与可选 next_cursor；cursor 是宿主的 opaque 值。一次会话同一个 cursor 最多一个未完成加载请求；滚动到底和底部“加载更多”共用同一入口。
- 只有 has_more、非 loading、到达末端时发 LoadMoreRequested。结果后仍未填满视口可以请求下一页，但一次仅一个；重复 cursor 或无进展页面停止自动请求并显示可重试状态，避免空页死循环。
- 第一页替换候选；后续页追加，跨页重复 key 返回错误且整页拒绝，不由组件猜测覆盖优先级。宿主负责去重。
- 查询改变、关闭、禁用、只读、外部重置、权威 set_options 和卸载会失效相关票据；旧成功与旧错误都不能覆盖当前状态。
- apply_page 返回 Applied / Stale / Invalid；过期是正常竞态，不能作为错误 Toast。宿主取消任务只是优化，票据验收才是正确性保障。
- 选中缓存独立于当前页结果，已选项不随搜索抖动；远程模式不再对服务端结果二次本地过滤。

### 8.3 创建选项

creatable 只在 Local / Remote 搜索模式有效；query trim 后非空、不是组合输入、没有可用的同 label 精确匹配时显示独立“创建「…」”候选。大小写比较沿用默认搜索规范；有同名但 disabled 的既有项也不提供绕过入口。

创建是一个带票据的请求，不是合成一个 label=key 的 option：

1. 用户激活创建行，emit CreateRequested(query, ticket)，创建行进入 pending 并防重复。
2. 宿主校验、持久化并分配稳定 key，通过 resolve_create(ticket, option) 回传。
3. 票据仍有效且未超过上限，先原子合入新 option，再按单 / 多选规则提交，emit 一次 Change。
4. 失败显示局部原因和重试，不清空原来的选择。迟到结果不能重新打开弹层或覆盖新选择；业务端已创建成功的资源由宿主后续刷新呈现。

创建票据另外记录 selection revision；等待创建期间若用户通过其它行、Tag 或 clear 更改选择，该创建票据也失效，避免旧创建结果追加到一份用户已经修改的值。普通远程查询票据不因多选切换而失效，两种请求不能混用一个全局取消标志。

创建行有独立身份并进入同一导航投影；不得把它画成普通 option 后同时触发 on_confirm 和创建请求。远程首次 loading 中不提前判定“没有同名项”；结果就绪后才决定能否创建。

### 8.4 IME 边界

Base Input 的 `ime_marked_range` 是私有字段；`InputEvent` 只有 Change / PressEnter / Focus / Blur，没有 CompositionStart / End。已核实 `InputBaseState<M>` 实现公开的 `EntityInputHandler::marked_text_range(window, cx)`，可在事件更新路径读取，不能编造 event.is_composing。

- 组合期间不确认选项、不创建、不用方向键抢走候选窗操作；本地显示可保留上一次已提交过滤，远程不发送未提交组合文本。
- 组合提交后重新读取 value 再过滤 / 开始防抖。IME 用来确认候选字的 Enter 不得继续变成 Select Confirm 或 Form submit。
- 仅检查回调时 marked_range 是否为 None 不足以证明安全，因为平台可能先提交组合再派发键。实施先做真实事件顺序测试；若需要，给 Base Input 增加最小“本次 Enter 已用于组合确认”语义或 composition 事件适配，并补 Base 回归。
- 不能用固定延时、拼音字符串猜测或吞掉所有 Enter 规避问题；输入法真实窗口验收是交付门槛。

## 9. 键盘、焦点与关闭策略

### 9.1 焦点模型

非搜索打开后焦点进入 listbox 容器；行本身不逐项加入 Tab，active option 标记真实的可访问活动节点。搜索打开后先聚焦顶部 query；Up / Down 将焦点转入列表，再移动或定位 active。列表内 Ctrl+F 回到搜索，保留文本与选择范围。

这是有意选择的分区焦点：当前 GPUI 的 `aria_active_descendant()` 只认**真实聚焦祖先下的节点**。query 与 listbox 是兄弟，不能在 query 持有焦点时给兄弟 option 加该标记，就声称读屏已读到当前项。首版不增加跨兄弟焦点关系补丁，也不截取普通字符后手工注入 Base Input。搜索 query 持焦时 Enter 进入当前可用结果，列表持焦后 Enter 确认；明确在搜索框帮助文案中提示。

### 9.2 按键表

| 上下文 | 按键 | 行为 |
| --- | --- | --- |
| trigger，关闭 | Enter / Space / Up / Down | 打开；优先定位已选的 enabled 项，否则首个；首次按键不额外移动一次 |
| trigger，关闭且不可搜索 | 可打印字符 | 500ms 前缀 typeahead 定位候选并打开，不提交 |
| query | 普通文字 / Space / Backspace / Delete / Home / End | 完全交给 Base Input 编辑，不执行选择或 Tag 删除 |
| query，未组合 | Up / Down / Enter | 聚焦 listbox 的 active 或首个 enabled 项；此次仅进入列表，不提交 |
| listbox | Up / Down | 跳过禁用、组头、分隔及状态行，逐项移动，不循环 |
| listbox | Home / End | 首 / 尾可用项 |
| listbox | PageUp / PageDown | 按实际可见行数移动，落到最近可用项并滚动 |
| listbox | Enter | 单选提交并关闭，多选切换并保留面板；创建行发创建请求 |
| listbox | Space | 非搜索选择与 Enter 一致；搜索列表也可选择，输入空格需回 query |
| listbox | Ctrl+F | 有搜索框时返回 query；无搜索框时传播给宿主 |
| listbox，非搜索 | 可打印字符 | typeahead，仅定位、不提交；重复同字符可在匹配项中依次定位 |
| query / listbox / 自定义区 | Escape | 非 IME 时关闭当前顶层，保留已提交值，恢复 trigger 或子面板 owner |
| 默认 query / listbox | Tab / Shift+Tab | 不确认；关闭后从 trigger 的全局位置向后 / 前移动，不陷在浮层中 |
| trigger，多选 | Backspace | 聚焦最后一个可移除 Tag 的关闭按钮 |
| Tag 关闭 / +N | Enter / Space | 删除 / 打开折叠项面板；不转成字段开合 |

没有可用项时 active=None，方向键和 Enter 不提交；Escape / Tab 仍有效。typeahead 是 Select 的前缀导航，不借用 Dropdown 的 menu checked/submenu 状态；时间阈值是行为参数，不做颜色或尺寸 token。

搜索 query 的 Tab 必须用局部 SelectQuery key context 覆盖 Base Input 的 IndentInline / OutdentInline。实现使用已核实的 Window::focus_next / focus_prev，但先将焦点基点恢复到 trigger，再向对应方向走一次；不能对临时 query 的 tab_stop(false) 直接调用后就声称字段顺序正确。

若有可交互 header / footer，Tab 先按可见结构在浮层操作间移动，经过边界才关闭并越过整个字段；不做无限 focus trap。默认无这些操作时按表格行为退出。所有路径须按实际焦点树回归。

### 9.3 关闭原因与焦点

| 原因 | 是否提交未确认 active | 焦点策略 |
| --- | --- | --- |
| 单选确认 / 清空后显式关闭 | 仅确认路径提交 | trigger |
| Escape / 无障碍开合关闭 | 否 | trigger；子层优先回其 owner |
| Tab / Shift+Tab | 否 | 下 / 上一个字段，不能最终停回 trigger |
| OutsidePress | 否 | 保留外部点击结果，不回抢 trigger |
| trigger 再次点击 | 否 | trigger |
| AnchorInvalid / ParentClosed / ScopeInvalid / Unplaceable | 否 | 不聚焦隐藏 trigger；仍持有内层焦点时交由有效父 owner 接管 |
| WindowInactive | 否 | 关闭会话，不激活窗口、不强制焦点 |
| 动态 disabled / readonly / 卸载 / 外部重置 | 否 | 撤销内层交互，若已失效则不强制恢复；遵守上层焦点接管 |

焦点策略由 Select owner 按原因执行，不让 PopupHost 主动 focus。退出动画中的旧行已不可交互、不可确认、不可继续处理滚动。

## 10. Base 的实施前置修补

这是源码发现的具体关卡，**本轮未修改**，必须先写失败回归再做最小 Base 修补并登记 `vendor/gpui-base/YUE-PATCH.md`。

| 当前源码行为 | 风险 | 最小处理与回归 |
| --- | --- | --- |
| Select Confirm 回调后无条件 focus content | 单选回调关闭后又聚焦隐藏列表 | 仅关闭态请求打开时转焦点；打开态 Confirm 不在回调后抢焦点。验收单选 / 多选 / 回调禁用及卸载 |
| Up / Down 关闭态开面板后仍 propagate 同一 action | 初始化 active 后再移动一格；第一次 Down 跳过首项 | 关闭态打开并消费；打开态交 controller 导航，不强制切回 query；每次按键只处理一次 |
| Base Select 强制 handle.tab_stop(true) | 调用方 tab_stop(false) 失效 | 增加明确 tab_stop / tab_index，并对传入 handle 生效；disabled 优先 |
| Base 只有 accessible label / value builders | 描述、readonly 等状态无法完整落在同一个语义根 | 在 Base 增加必要的语义适配，避免另包第二个 ComboBox 假装修复 |
| Base 没有 readonly 行为入口，closed 请求可能被 owner 拒绝 | 拒绝打开后仍把焦点移到未显示 content；以 disabled 代替又会丢失 Tab 停靠 | 增加保留焦点的只读行为，并让开合被拒绝或回调中失效时不转移 content focus；分别回归 readonly / disabled |
| Input Enter / Tab 绑定与 Select query 不同 | 组合确认误选、Tab 缩进、表单误提交 | 局部 action 路由及 IME 验证；只在确认确有缺口时补 Input 的通用行为接口 |

Base 继续承载通用行为和语义；不得在这些补丁里加入 Yue token、颜色或 Popup 皮肤。现有 Base Select / Combobox 测试也要回归；不能只测 Yue 新代码。

## 11. Popup 复用与大列表

### 11.1 接入既有 Popup

使用当前 crate-private PopupAnchor、PopupSpec、PopupHost、PopupRuntime；只增加一个 Select consumer。位置默认 Bottom / Start，无箭头，surface gap 拟用 4px 的空间 token。外部点击和生命周期使用 PopupRuntime；主面板 Escape 由 Select 的局部键盘上下文仲裁，避免组合输入时误关闭。折叠面板的 Escape 直接由 Runtime 处理。关闭事件区分 Escape、OutsidePress 与 Lifecycle。

- 继承调用环境的 parent 和 modal scope，Dialog 内的 Select 不越出模态层；嵌套 Select 的子面板不会被父层当外部点击。
- 使用现有 Positioner 同帧 Anchor 与碰撞翻转；位置最终结果可能在上方，进出方向按实际结果调整，不能只看请求方向。
- 通过现有 `PopupHost::available_size(...)` 约束自然排版，再结合候选侧实际可用高度设置 listbox 视口；保留面板表面和 margin。Select 的扩散阴影允许在窗口边缘裁切，不把阴影视为需要完整避让的内容，否则靠右的正常字段会被阴影预算推离锚点。不能只按整个窗口高度排版后任其遮挡 trigger。
- MatchTrigger 默认按字段完整 border box 测量宽度与锚点，不使用扣掉边框的内部 content box；Content 是至少 trigger 宽度的受限自然宽；Fixed 是显式面板宽。都 clamp 到视口，长行省略，实际截断才显示帮助。
- Content / 虚拟列表不逐帧测量全部候选。允许初次用文本测量与已知槽位计算自然宽度并按模型、字体、主题修订失效；不得只测第一个虚拟行就把长行截成“自然宽”。
- Panel 的 max height 是整体可用高度减去 query、header、footer、内距后的列表高度，列表单独滚动；滚动条位于列表边缘，不吞没选中标记。
- 复用 Presence / motion 和 pointer policy。注册只在正确生命周期有效；关闭后移除 hitbox 与活动节点，视觉退出可继续。快速重开采样当前值，不重复叠加残影。
- 不假设 overflow_hidden 自动做圆角后代裁剪。搜索背景、hover 填充、滚动内容、底部状态区域必须在圆角外轮廓内，外侧焦点环与阴影单独保留。

### 11.2 虚拟化与投影

将业务条目投影成带稳定 key 的 Option / GroupHeader / Create / Status 行，并建立 key→投影 index 映射。默认单行 option 各尺寸固定高度；说明行模式使用统一的双行高度。组头、分隔与状态行有显式已知高度。

已核实 Base `v_virtual_list(view, id, Rc<Vec<Size<Pixels>>>, render_range)` 要求 view 实现 Render。若 public state 保持纯 controller，就用一个私有 panel view 作虚拟列表渲染适配，持弱 state 引用；它不复制选择状态，不形成强引用环。只在所用列表 API 确实需要时引入该视图。

- 导航先按模型更新 active，再调用 VirtualListScrollHandle::scroll_to_item(index, strategy)。不能依赖候选已挂载的布局 bounds 计算远端索引。
- 目标进入已挂载区后才能发布真实 option 的 active 标记；不把 key 当 DOM ID 传给并不存在的 container API。
- 行 ElementId 以 option key 为身份，不能按 visible index 生成；过滤与滚动复用不继承上一项 hover、选择或 tooltip。
- 固定高度模式中自定义内容必须服从行槽位；若启用已知双行布局，整份投影使用相应高度。首版不承诺任意动态行高与无测量虚拟化同时成立。
- 过滤重建、options 替换、theme / font / DPI / size 变化重新计算需要的测量；滚动偏移 clamp，不能落在空白区域。
- 10,000 项用虚拟化呈现，渲染项数应受可见区和预取限制；过滤可以 O(n)，不为每次鼠标移动重建所有字符串、SVG 和布局。

## 12. 可访问性与 Form

### 12.1 语义表

| 元素 | 语义合同 |
| --- | --- |
| trigger 根 | 一个 ComboBox，名称是字段名，value 是可读 committed label，expanded 是逻辑 open |
| query | 独立搜索输入名称与帮助；不是 committed value，不重复输出第二个 ComboBox |
| 列表容器 | ListBox；行的 selected 与 active 分开；多选需正确表达 multiselectable |
| option | ListBoxOption、完整 label、selected、禁用语义；可访问激活走同一 commit 路径 |
| active option | 列表祖先真实聚焦且该行实际挂载时标记 aria_active_descendant |
| Tag / 清空 / +N / 重试 | 独立可读操作名称，例如“移除 Rust”；装饰图标不重复播报 |
| loading / 空 / 错误 / 上限 | 明确的文字状态；不伪装成可选项，不每帧公告相同消息 |

GPUI 当前有 role / aria_label / aria_description / aria_selected / descendant-side active 标记；没有对应 DOM 全套 ARIA builder。Base 的 A11yDisabledExt 已公开导出，本轮在同一节点转发适配上补充 SelectionA11y / A11ySelectionExt，将 disabled、readonly、required、invalid、multiselectable 写入真实 AccessKit 节点；不增加额外布局节点或焦点。

上述原生状态有节点级回归。选项的可访问 Click 使用同一个 activate 路径，但当前无头工具不能从系统 UIA 分发该动作；实际激活与播报仍列入 Windows 验收，不把红边或节点属性等同于读屏通过。

图形节点测试能证明 role / value / expanded / selected / description 的输出，不能证明 Narrator 实际播报。Windows UIA / Narrator 是单独的真实窗口验收，尤其验证搜索分区焦点、虚拟行、已选项缺失和多选上限。

### 12.2 YueField / YueForm 扩展

实现前 YueForm 的 require / rule / attach 只接受 Base InputBaseState；本轮保留 Input 接口，增加 SelectValue 类型化校验。YueField 的 `.child(select)` 仍是任意元素；使用新增 `.select(select)` 才会自动转发 label、required、description、error。

已落地的改法：

1. FormField 内部校验改为读取实时 state 的 `Fn(&App) -> Option<SharedString>` 和独立 focus 闭包；现有 Input 的 require / rule 适配到这层，不改它的文本真值。
2. 增加 require_select / rule_select 的强类型适配，规则读 SelectValue；required 的含义为单选非 None、多选非空。disabled 是否跳过校验必须由显式规则或字段策略决定，不能暗中更改既有 Form 行为。
3. YueField 增加 `.select(YueSelect)` 的可触达分支，和 Input 一样转发字段名、必填说明及 live error；用户显式描述与错误合并，不互相覆盖丢失。
4. Form submit 聚焦首个失败 Select 的 trigger，且先关闭其旧查询会话。修正选中值后可立即消除错误，无需再次提交。
5. query **不得**接入 Form.attach；弹层 Enter 只执行本节定义的进入列表 / 选择，不向父 Form 提交。关闭态 Enter 先打开 Select，也不提交。

不新增第二个 YueSelectForm 或把业务规则藏进 Option renderer。

## 13. 自定义内容与公共 API

下表对应 `src/select/` 当前公开 API。完整 retained state 消费者示例位于 YueSelect Rustdoc，并作为 doctest 编译；独立使用方式见第 19 节。

| 对象 | 当前入口 |
| --- | --- |
| YueSelectState | new(mode, window, cx)、value、selected_items、options、query、active_key、is_open |
| State 更新 | set_value、set_selected_items（仅补齐当前选择展示快照）、set_options、set_disabled、set_readonly、set_loading、set_max_selected、set_search_mode、set_creatable、set_search_debounce、set_open |
| 远程交付 | apply_page(ticket, page)、reject_request(ticket, message)、resolve_create(ticket, option)、retry_request（为当前失败请求发新票据） |
| YueSelect | new(id, &state)；继承默认主题，theme / use_default_theme 与现有 Button 一致 |
| 外观与布局 | size、variant、placeholder、prefix_text、leading_icon、width / min_width / max_width、block、auto_width、margin、flex_grow |
| 列表呈现 | panel_width、max_panel_height、row_layout、virtualization、empty_text、loading_text |
| 多选呈现 | tag_layout、collapse_count、clearable、可读清空 / 删除文案 |
| 焦点与辅助 | track_focus、tab_index、tab_stop、key_context、accessibility_label、accessibility_description |
| 内容插槽 | render_option、render_value、render_tag、render_collapsed、panel_header、panel_footer；panel_header_focus / panel_footer_focus 登记临时焦点 |
| Form | require_select / rule_select；YueField.select |

模式只在创建时确定；disabled / readonly 等交互许可通过 state 更新。传入 focus handle 必须进入同一 Base 语义根，tab policy 对调用方 handle 同样生效；不额外叠 track_focus 生成第二个字段停靠点。

### 13.1 事件与重入

使用保留 state 的事件订阅，不再加一份 builder on_change，避免重复通知：

| SelectEvent | 发生时机 |
| --- | --- |
| Change(SelectChange) | 用户确实改变 committed value；source 分 pointer / keyboard / tag_remove / clear / create |
| OpenChange(SelectOpenChange) | 用户或生命周期导致开合变化，附原因；静默外部同步不当作用户事件 |
| SearchRequested | 已提交 query 的防抖请求；包含 ticket |
| LoadMoreRequested | 当前 query 的追加页请求；包含 ticket 与 cursor |
| CreateRequested | 显式创建请求，包含 query 与 ticket |

Change 的读取值必须已经是 after；同值不重复发。单选确认时先提交、再关闭，事件队列顺序为 Change → OpenChange(false)。外部 setter 不发 Change / OpenChange；由它引起的关闭仍须失效票据并更新视图，宿主已知道该次同步。用户开合和 Popup 生命周期关闭才发 OpenChange。回调通过正常 GPUI update 运行，不在借用 state 的 renderer 中同步调用任意宿主闭包；宿主可同值回写而不重入 panic。

### 13.2 插槽不得夺走语义

- render_option 只替换行内容，收到只读 OptionContext（option、selected、active、disabled、size、query）；外层高度、角色、选择和鼠标/键盘监听仍由组件拥有。
- 图标 / 说明 / meta 默认先用内建槽位，不强迫每个消费者自建行布局。option 内不允许再嵌输入框、按钮或其它有焦点控件；有这些需求使用 footer 或其它组件。
- render_value 只替换已选文本区，仍须提供完整可读 label。render_tag 可调整内容但必须复用组件提供的 remove 语义；不能自行通过 index 删除或忽略锁定。
- render_collapsed 自定义 +N 的内容，不替换其计数、按钮语义、开合与命中区。
- header / footer 通过 `Fn(interactive: bool, &mut Window, &mut App)` factory 生成新元素，固定于列表外。交互控件须 `.disabled(!interactive).tab_stop(false)`，使用同一 FocusHandle 并按可见顺序登记到 panel_header_focus / panel_footer_focus。Select 管理临时 Tab 顺序；动态隐藏或禁用的操作不能登记。插槽按实际排版测高，每个插槽最多占可用高度的三分之一，超出单独滚动。异步业务仍在事件处理层。
- 自定义内容仍服从行高、宽度和主题；不允许 `Styled` 任意覆盖内部 token 后再声称尺寸可预测。

## 14. Token 来源与最小新增项

唯一源仍为 `D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens`。Select 专用变量已写入源包并生成到本仓库；现有 Input / Menu / Popover / Tag 变量继续直接复用。

### 14.1 直接复用

| 视觉部分 | 已有来源 |
| --- | --- |
| 字段尺寸、字号、图标、内距、边框、disabled / readonly / invalid、焦点环 | input.css 的对应 `--input-*` |
| 浮层背景、边框、圆角、阴影、内距、进出场 | `--popover-*` |
| 行 hover 填充、行圆角、文字与渐变 | `--menu-item-background-hover`、`--menu-item-border-radius`、`--menu-item-color`、`--menu-item-duration` / ease |
| 标签、关闭区、标签动效 | `--tag-*`，使用现有 YueTag 适配 |
| 选中色、组标题、说明、间距 | `--accent-subtle` / text、`--text-secondary`、既有 space / font token |

本方案不为每个 Input 变量复制一个同值 select 别名，也不使用 `--menu-width` 作为字段宽度。

### 14.2 仅新增选择器独有变量

已写入外部设计系统源包 `src/component-tokens/select.css` 并通过 `_index.css` 导出，共 12 个变量；本仓库快照与 Rust 常量均由工具生成：

| 已实现 token | 当前值 / 关系 |
| --- | --- |
| `--select-width` | 240px，默认字段宽度 |
| `--select-list-max-height` | 288px，列表上限；还需受真实可用高度 clamp |
| `--select-panel-gap` | 引用 space-4 |
| `--select-option-height-sm/md/lg` | 引用 input-height-sm/md/lg，即 28 / 32 / 40px |
| `--select-option-description-height-sm/md/lg` | 48 / 56 / 64px，统一双行模式 |
| `--select-option-background-selected` | 引用 accent-subtle；与 hover 为独立状态 |
| `--select-option-color-selected` | 引用 accent-subtle-text；不默认白字 |
| `--select-tag-gap` | 引用 Tag / space-4；控制多选容器，不修改 Tag 内距 |

组标题、search padding、panel footer、清空命中区优先用已有 text / input / space 值；测量证明需要独立调整时再增加专用 token。防抖、最大选择数、折叠数量、虚拟化阈值是行为参数，不加入 token。

新增量不代表只使用这些变量；组件还消费现有 Input / Menu / Popover / Tag 的完整相关参数。Light/Dark × Azure/Neutral 四种配置及中性色覆盖都要同步验证，不能只填浅色 Azure。

```powershell
# 修改设计系统源定义后，重新同步并检查：
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --write
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check
node tools/sync-yue-theme.mjs --check
```

不手改 inventory payload 哈希、generated.rs 或生成常量。不通过无法解析时 fallback 透明 / 0 掩盖源变量缺失。

## 15. 动画与绘制

- 字段边框 / 背景使用当前 Input motion，已有 input-duration 为 100ms；行 hover 直接采用 Menu 当前 **300ms、cubic-bezier(.33, 0, .67, 1)**，与本项目先前调校一致，不另硬编码“300”。
- Popup 与 Dropdown 共用 `motion::dropdown_popup`：Popover enter 200ms / exit 100ms，分别使用 `--motion-ease-enter` / `--motion-ease-exit`；入场同时淡入并从靠近锚点 4px 的位置归位，退出反向位移并淡出，通过现有 Presence 执行。主选项面板和 +N 已选项面板采用同一策略。箭头朝向动画共用 Input 时长，不另建动画循环。
- hover 插值针对背景层；selected 勾选、值和上限状态立即响应，不能等 300ms 才确认。hover 与 selected 合成规则清晰：选中底色为基底，再叠 hover wash；不能相互覆盖消失。
- 每项动效以 option key 分通道，换过滤结果、离开虚拟视口或退出完成时清理悬挂状态。异步重绘不能重新提交旧 panel / row 布局，避免重现 Toast 隐藏卡片闪烁路径。
- 不对文字、Tag 或整个 Select 做尺寸缩放 / 布局动画；只改需要过渡的视觉层，避免文本栅格抖动和相邻字段位移。
- reduce_motion 立即落到目标状态，清理在途动画，不保留透明 hitbox。远程 loading spinner 在关闭、不可见或 reduced motion 下不持续申请无效帧。

## 16. 模块与实施顺序

当前模块 `src/select/`：

```text
mod.rs       公共外观 builder、导出与 RenderOnce 组合
model.rs     key / option / group / value / 请求与错误类型
state.rs     committed 值、查询会话、投影、票据及事件
behavior.rs  导航、typeahead、确认、关闭原因与焦点策略
panel.rs     现有 Popup / YueInput 接入、列表、固定头尾与虚拟适配
tests.rs     纯模型与模块内测试；达到维护需要时再拆文件
```

不为每个 enum 建文件，不在一开始创建通用 DataSource / Repository / Selector 引擎。tokens 沿用现有 typed token 适配，不另发一个 Select theme crate。

| 顺序 | 可核验产物 |
| --- | --- |
| 1. 基线与 Base 关卡 | 现有 workspace / headless 基线；新增焦点、首次方向键、Tab policy 失败测试后修补 Base |
| 2. 单选最小闭环 | 模型、受控 state、三尺寸、Outline/Text、Popup、焦点及键盘、清空、状态 |
| 3. 多选 | 复用 YueTag、锁定 / 上限、折叠面板、删除焦点和宽度测量 |
| 4. 查询与数据 | 本地、远程 / 分页票据、创建、IME 路由；结果状态和过期响应测试 |
| 5. 组合完整性 | 分组 / 插槽、虚拟列表、Field / Form、节点语义与 token 同步 |
| 6. 验收与文档 | Showcase 全矩阵、无头回归、真实 UI 清单、消费者 doctest、补丁与 README 更新 |

首版交付覆盖本文所有已纳入能力，分步是实施与验证顺序，不表示做到单选就结束。对未达到的 UIA / IME 能力必须明确记录，不能跳过测试就标“全功能完成”。

## 17. Showcase 与验收

### 17.1 展示页

在组件分组增加 **Select**，继续按首字母升序；不新增一级导航类别。左侧目录、右侧滚动示例遵循当前布局。页顶用受限宽度控制面板切换配置，不把每个 Select 默认撑满卡片。

示例至少覆盖：

1. 基础单选、清空、同名不同 key、禁用项与缺失已选项。
2. 单 / 多选 × 三尺寸 × Outline / Text；并排展示相同尺寸的基线。
3. 多选 Wrap / Count / Auto 折叠、删除 / 锁定 / 上限、极窄宽度与长中文 Tag。
4. 本地搜索、分组、Tabler 图标、说明、前缀文字、自定义行与固定头尾。
5. 远程延迟、乱序响应、失败 / 重试、滚动追加、创建成功 / 失败 / 迟到；可用确定性模拟数据，不将演示伪装成真实 API。
6. 10,000 项虚拟列表，首 / 中 / 尾跳转、过滤后的 active 与滚动位置。
7. disabled / readonly / loading / error，四主题，外部 set_value / set_options，实际 Change 计数。
8. Form 提交与首个无效项聚焦；Dialog 内、父 Dropdown / Popup 内、屏幕边缘与窗口缩放。

### 17.2 自动测试矩阵

| 层级 | 必测内容 |
| --- | --- |
| 纯模型 | 模式和值验证；跨组 / 跨页重复 key；同名；顺序；未知 key 缓存；选项替换；上限原子性；禁用锁定 |
| 搜索与请求 | 默认过滤、空查询、分组投影；防抖；旧成功 / 旧错误；close/reopen；取消；重复分页；空页无进展；创建竞态 |
| Input 组合回归 | 搜索确实渲染 YueInput；清空触发一次过滤 / 防抖；同值同步不重建 InputState；复制粘贴、撤销、主题与三尺寸正常；局部键盘覆盖不改变独立 YueInput 行为 |
| Base 回归 | Confirm 后不聚焦隐藏节点；关闭态 Up/Down 一步；打开态不抢 query；调用方焦点和 tab policy；原 Select / Combobox 仍正常 |
| 真实事件无头 | 鼠标点行、清空、Tag、+N、外部点击；按下后移出再释放；Enter / Space / Up / Down / Page / Escape / Tab；父 Form 不误提交 |
| 禁用与焦点 | 先取得焦点再 disabled / readonly；事件为零；外部点击不回抢；删除后目标消失；anchor / modal invalid |
| 测量与绘制 | 三尺寸、三位宽数值 label、长中英文字、极窄宽度；Tag 不重叠；圆角边缘；首中尾虚拟行；实际可见 item 数有界 |
| 动画 | 假时钟推进起点 / 中间 / 终点；hover 反转从当前采样继续；reduced motion；快速重开；卸载行不重放旧 hover |
| 节点 | ComboBox / ListBox / Option，名称、值、expanded、selected、disabled；query 焦点时不谎报兄弟 active；关闭与虚拟卸载无幽灵节点 |
| Form 回归 | 原 Input 的 require / rule / attach 行为保留；Select typed rule；第一失败字段聚焦；修正后错误消除；query 不接提交 |
| 消费者 | 公共 API doctest，默认字体与系统字体，两种主题策略，调用方 focus handle，既有 Popup 与 Kit 一份依赖 |

所有修补先增加能失败的测试，再验证修复后的结果。不能让 disabled 测试一开始就没有焦点，再用“未发生事件”证明键盘安全；不能用直接调用私有 commit 代替所有真实输入测试。

实施前后记录实际执行结果，不预填本轮通过数。最低回归命令：

```powershell
node tools/sync-yue-theme.mjs --check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
pwsh -File tools/test-gpui-base.ps1 -Offline -Filter select::tests
```

Base 补丁涉及 Input / Combobox，已追加对应模块回归。生成 token 同时执行源包对照检查；真实执行结果记录在第 19 节。

### 17.3 Windows 真实窗口门槛

- 100 / 125 / 150 / 200% DPI，Light / Dark、Azure / Neutral；真实字体与系统字体路径。
- 鼠标滚轮 / 触控板、拖动滚动条、长列表、上下翻转、窗口移动与失焦；Popup 无裁切和透明挡板。
- 微软拼音等中文 IME：组合、候选方向键、Enter、Escape、连续输入与 query 切换；绝不误选或误提交 Form。
- Narrator / UIA：字段名、当前值、已展开、禁用 / 只读、选中项、分区导航、上限与加载 / 错误的实际播报。
- 多选长 Tag、省略提示、删除焦点、+N 消失和加载并发；查看已选标签与主列表不互相穿透关闭。
- release 窗口核对 hover 的真实中间帧、滚动帧率与快速重开；无头逻辑像素测试不能替代 GPU 渲染结论。

真实窗口由用户验收。未实测就明确写“未验收”，不能把无头通过扩展成已覆盖 DPI、读屏或输入法。

## 18. 调研出处与交付范围

- [用户参考图](assets/select-reference.png)：首版功能场景与组合参考。
- [Base Select 源码](../vendor/gpui-base/src/select.rs)、[Combobox](../vendor/gpui-base/src/combobox.rs)：0.8.0 当前真实的键盘、焦点、语义与回调。
- [Base 虚拟列表](../vendor/gpui-base/src/virtual_list.rs)、[Input state](../vendor/gpui-base/src/input/base/state.rs)：虚拟行尺寸、滚动入口、InputEvent 和 EntityInputHandler 实现。
- [Popup Host](../crates/yue-component/src/popup/host.rs)、[Runtime](../crates/yue-component/src/popup/runtime.rs)、[12 号文档](12-popup.md)：定位、Presence、父子关系、关闭原因和作用域。
- [YueTag](../crates/yue-component/src/tag/mod.rs)、[Tag 绘制](../crates/yue-component/src/tag/element.rs)、[YueForm](../crates/yue-component/src/form/mod.rs)、[YueInput](../crates/yue-component/src/input/mod.rs)、[YueField](../crates/yue-component/src/field/mod.rs)：复用点与需要扩展的类型边界。
- [token 快照](../crates/yue-component/tokens/inventory.json)、[同步工具](../tools/sync-yue-theme.mjs)：当前已解析参数与生成流程；最终变量定义以外部设计系统源包为准。
- [Element Plus Select 官方文档](https://element-plus.org/zh-CN/component/select.html)：辅助对照搜索、多选、创建与远程等场景，不作为本仓库 Rust API 的证据。

本轮包含 Select Rust 实现、Base 机制修补、源 token 与生成产物、Showcase 和回归测试。没有启动真实窗口，也没有提交 Git。


## 19. 首版实现与使用

### 19.1 入口

运行 `cargo run -p windows-showcase`，进入 **组件 → Select**，位于 Segmented 和 Slider 之间。页面覆盖基础/搜索单选、禁用/只读/加载、三尺寸 Outline/Text 多选、Auto/Collapse/Wrap、锁定/上限、分组图标说明、远程乱序与失败、分页、创建、一万项虚拟列表、自定义行/固定页脚、Form 和 Dialog 内选择。

复用关系：搜索是现有 `YueInput + Entity<InputState>`；多选是 YueTag；浮层使用已有 PopupAnchor / PopupHost / PopupRuntime；宿主只保留一个 `Entity<YueSelectState>` 与事件订阅。state 自身实现 Render，内部 notify 能重绘控件，消费者不需要为每个状态变化另加 observe。

```rust
use yue_component::{SelectMode, SelectOption, YueSelect, YueSelectState};
use yue_component::gpui_kit::AppContext as _;

// 在宿主构造函数内创建并保留，不在 render 中重新创建：
let language = cx.new(|cx| {
    let mut state = YueSelectState::new(SelectMode::Single, window, cx);
    state.set_options([
        SelectOption::new("rust", "Rust"),
        SelectOption::new("typescript", "TypeScript").keywords(["ts", "前端"]),
    ], cx).unwrap();
    state
});

// render 中；YueField.select 可代替 accessibility_label 转发字段名：
YueSelect::new("language", &language)
    .accessibility_label("开发语言")
    .clearable(true)
```

事件通过宿主保留的 `cx.subscribe` / `subscribe_in` 订阅 `SelectEvent`。搜索模式、权限、上限和选择值用 state setters 更新；builder 只设置呈现。远程宿主接收请求中的 ticket，再调用 apply_page / reject_request；创建调用 resolve_create。票据不能自行构造，不要用旧请求的结果覆盖当前 state。

`set_value` 同值 echo 不清搜索、不关闭面板、不发用户事件。选中项暂未出现在候选中时保留 key / label 缓存，可用 set_selected_items 补齐服务端展示。跨页重复 key、无进展/循环游标、无效创建结果均可见失败并允许重试；拒绝结果不会部分写入候选。

### 19.2 具体机制修补

- Base Select / Combobox 的 on_open_request 回调返回实际 open bool；on_open_change 保留上游的无返回值通知合同，以便宿主同时使用 Kit styled 组件。拒绝打开时不聚焦隐藏 content；打开态确认不在回调关闭后夺回焦点；首次方向键开启后被消费。
- Base Select 增加 readonly、required / invalid、description、tab policy，并复用同一原生节点适配。完整变更见 vendor/gpui-base/YUE-PATCH.md。
- Base Input unmark_text 结束组合时补 notify；Select 观察真实 InputState，在预编辑时失效旧票据、保持候选，在提交或取消边界再处理 query。查询区键绑定使用 `YueSelectQuery > Input` 的祖先关系，保留一般 Input 的 Tab 缩进规则。
- Content 宽度和虚拟行几何按模型/字形/行高/宽度缓存；hover 不重测一万条文本。PageUp/Down 使用实际列表视口行数。卸载字段会结束会话，即使宿主仍持有 state，迟到回写也失效。
- 过滤或替换候选导致 active 身份 / 模型位置变化时，将该行滚入可见区域；分页仅追加且 active 位置未变时保留用户滚动位置。回归先证明滚到末尾后过滤会隐藏首个活动项，再验证修复。
- 同步工具严格支持当前菜单圆角的 `max(px, calc(px ± px))` 表达式，生成长度类型；遇到其它无法解析的样式仍显式报错。

### 19.3 验证记录

实现前已跑 workspace 普通与无头基线。先新增失败回归，再修补：Base 确认后焦点被抢回、首次 Down 传播，以及 Select 关闭残留输入焦点、非法远程数据卡住加载、IME unmark 后查询未更新。测试均经过修补前失败与修补后通过。

2026-11-02 首版实施验收（本次视觉修复前的记录；后续见 19.4。计数按测试二进制分别记录，未将 integration / doctest 混为组件单元测试）：

| 命令 / 范围 | 实际结果 |
| --- | --- |
| `node tools/sync-yue-theme.mjs --check` | 快照一致 |
| `node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check` | 与外部设计系统源一致 |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check -p windows-showcase --locked` | 通过 |
| `cargo test --workspace --locked` | 全部通过；yue-component lib 147 项，组件 doctest 27 项 |
| `cargo test --workspace --locked --features headless-tests` | 全部通过；yue-component lib 298 项，Showcase 18 项，各 integration / doctest 同时通过 |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests` | 系统字体路径全部通过；组件 lib 298 项，各 integration / doctest 同时通过 |
| Select 专项（上述 lib 的子集） | 35 项：28 项真实事件无头测试，7 项模型 / 导航测试 |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | 通过，无警告 |
| 同上加入 `--features headless-tests` | 通过，无警告 |
| `tools/test-gpui-base.ps1 -Offline -Filter select::tests` | 14 项通过 |
| 同脚本 `-Filter combobox::tests` | 2 项通过 |
| 同脚本 `-Filter a11y_disabled::tests` | 3 项原生节点标记测试通过 |
| 同脚本 `-Filter composition` / `-Filter test_ime` | 分别 9 / 4 项通过；筛选集合可能交叠，不相加统计 |

真实 Windows 窗口、微软拼音候选 UI、Narrator 实际播报、DPI 和 GPU 中间帧均 **未验收**。无头 IME 测试仅覆盖 EntityInputHandler 的预编辑/提交协议，不能替代系统输入法实测。未启动真实窗口，未提交 Git。

### 19.4 视觉修复：焦点环与开合动画

- 右侧孤立蓝线是焦点环被 Dialog Body 的滚动裁切遮罩裁掉上、下、左侧后的残留。Body 在滚动区域内按通用 focus-ring width + offset 预留空间，用等量负 margin 抵消，保留正文位置、分区间距及独立滚动。Select 同时修正绝对定位的边框基准，焦点环距外框的真实间距与 token 一致，外圈圆角相应扩大。
- Select 与 Dropdown 的开合曲线、时长和 4px 位移合并到一个私有策略，避免两处配置再次漂移。不修改行 hover、箭头时长或设计系统源变量。
- 新增真实场景回归：三尺寸 / 不同宽度的外圈几何；Dialog 内已选项焦点环四边的绘制裁切；入场 / 退出中间帧的位置和实际背景 alpha、退出禁用选项节点、reduced motion。测试先验证修复前失败，再验证修复后通过。
- 本次验证：Select 37 项全部通过；普通 workspace、headless workspace（组件 lib 300 项）和系统字体模式全量测试通过，Showcase check、两种 token 同步检查与本次修改文件的 rustfmt 检查通过。全仓两组 Clippy 暂被正在开发的 `tabs/element.rs:352` 的 `clone_on_copy` 阻断；全仓 fmt check 也报告 Tabs 相关文件及导出入口的格式差异。本次未修改这些并行开发内容，不能宣称全仓静态检查干净。

### 19.5 自定义选项内容垂直居中

示例里的 Rust / TypeScript 等白色块是 `render_option` 返回的 YueTag。Tag 的 `self_start` 用来保留自身宽度，直接放在横向选项行中时却覆盖了父级的 `items_center`，使其贴到灰色背景顶部。

Select 现在为自定义内容提供独立的全高纵向槽位，由主轴 `justify_center` 负责垂直居中；Tag 的自身对齐只影响水平方向。槽位同时占用剩余宽度，将选中勾选保留在行尾。不修改 Tag 的全局布局，不使用固定像素补偿。

新增回归先验证原实现偏移，再检查三尺寸、单行 / 说明行、普通 / 虚拟列表这 12 种组合；通过真实点击确认禁用项仍拒绝选择、可用项仍可选中并关闭。

本次 Select 38 项专项测试通过（内置字体与系统字体两种路径）；workspace 无头全量测试通过（组件 lib 302 项），两组 workspace Clippy `-D warnings`、Showcase check 与全仓格式检查通过。Showcase 模块声明仅按 rustfmt 调整 Tabs / Tag 的顺序；未修改这两个组件的实现。Windows 实窗仍由用户复测，未提交 Git。
