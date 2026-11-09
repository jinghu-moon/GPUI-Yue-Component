# YueTimePicker / YueTimePanel 时间选择器设计

- 状态：首版已实现，已接入 Showcase、Field／Form 和自动测试；Windows 真机视觉与真实输入法由用户验收。
- 日期：2026-11-08。
- 目标：为 Windows GPUI 应用提供时间点与同日时间区间输入，复用现有编辑、浮层、焦点、滚动及表单能力。
- 参考：[本地 TimePicker demo](../refer/TimePicker%20时间选择器%20·%20GPUI%20组件%20Demo.html)、[Arco TimePicker 参考图（用户图 1）](assets/time-picker-arco-reference.png)。用户图 2 是设备现场照片，与时间选择器无关，不作为 UI 或功能依据；如后续补充正确图片，再补差异核对。
- 约束：[组件架构](24-component-architecture.md)、[Input / Field / Form](03-input.md)、[Popup](12-popup.md)、[Select](20-select.md)、[日期组件](35-date-picker.md)。

## 1. 定位与范围

主要任务是填写“每天几点”或“一天内的起止时间”，例如营业时间、提醒时间、会议时段。值使用 `chrono::NaiveTime`，不附带隐含日期、时区或 Unix 时间戳；预约日期、持久化、时区转换、重复规则及冲突检测由宿主负责。

| 单元 | 职责 | 状态关系 |
| --- | --- | --- |
| `YueTimePicker` | 可编辑时间字段、时钟按钮、清除、滚动列 Popup、确认事务 | 宿主持有 `Entity<YueTimePickerState>`，组件接收引用 |
| `YueTimePanel` | 不带字段和 Popup 的嵌入式滚动列；可带此刻、确认和附加页脚 | 与 Picker 使用同一种控制器，但独立挂载分别创建 Entity |
| `YueTimePickerState` | 唯一已提交值、候选协调、输入验证、提交／取消、打开和焦点路由 | 组合 Base InputState、TimeFieldState 和既有 Popup |
| 时间项／时间列 | 每列一个单位，显示候选、活动项和禁用态，提供键盘及滚动 | 内部零件，不另公开一套 List 或 Select 状态 |

范围由 `TimePickerSelection::Range` 配置，不再建一套 `YueTimeRangePickerState`。不新增独立时间 crate，不启用 GPUI Component feature，不复制文本编辑器或 Popup runtime。

首版完整交付分钟／秒精度、H23／H12、单值／范围、严格输入、步长、联动禁用、此刻、确认／即时、三尺寸、表单和独立面板。毫秒、滚轮停下自动选中、跨午夜区间及 UTC／时区不列为首版完成条件，理由见第 13 节。

## 2. 参考的采用与取舍

| 来源 | 已核实内容 | Yue 决策 |
| --- | --- | --- |
| 本地 demo 的结构 | 输入框、后缀、Popup、列标题、单位列、选中／禁用项、页脚；支持嵌入式面板 | 采用结构与交互层次，外观使用现有 Yue token |
| 本地 demo 的事务 | 默认确认，修改先回显草稿；外部点击／Escape 还原；可切即时 | 采用；提交前统一复验，外部赋值使旧会话失效 |
| 本地 demo 的联动 | 小时改变后修复禁用分钟，范围开始越过结束时联动结束 | 采用可预测、有限的候选修复，明确失败、平局及来源规则 |
| 本地 demo 的格式／步长 | 时分秒、时分、H12、毫秒；分／秒可指定步长 | 首版采用分钟／秒、H12／H23和步长；毫秒需先补齐 Base 能力 |
| 图 1 的使用场景 | 单值／范围、前缀、受控赋值、状态、尺寸、默认值、附加内容、禁用、H12、免确认 | 采用场景覆盖；沿用 Yue 的静默 `set_value`、三尺寸和事件风格，不搬入 React 属性 |
| 图 1 的 API | `scrollSticky`、自动排序 `order`、四尺寸、UTC／timezone、DOM 容器等 | 明确取舍，不因参考库公开了属性就全部加入 Yue |
| 图 2 | 设备现场与缺陷标注照片 | 不推导时间组件需求，也不纳入组件资源 |

本地 demo 是行为原型，不直接照搬其 JS。源码核对发现的差距：

1. 清除入口没有完整的 readonly／disabled 拦截，不能只靠鼠标样式保护。
2. `commit()` 未统一检查非空、完整范围、步长和禁用规则；列内 Enter、页脚确定等入口可能绕过输入验证。
3. 合法输入在关闭状态失焦时只校验、不提交；非法输入回滚且抖动，不利于修正。Yue 保留原输入与明确错误。
4. 步长主要控制列项目，输入可能接受不在刻度上的值；`fixT`／此刻未形成覆盖全部边界的统一求解。
5. DOM pointerdown、焦点、碰撞与 CSS 过渡是浏览器实现；Yue 应使用既有 Popup、Root 和 motion。
6. 原型的程序赋值和用户确认、草稿预览在会议场景中容易混用；宿主执行预约必须读取已提交值。

## 3. 已有能力与复用决策

核对当前仓库的 GPUI Kit／Base 0.8.0 与本地 GPUI 补丁源码，具体签名以实现时版本为准。

| 已有能力 | 源码事实 | 本组件复用方式 |
| --- | --- | --- |
| [Base TimeField](../vendor/gpui-base/src/time_field.rs) | `TimeFieldState` 持有分段编辑值，`set_time` 静默；有分钟／秒、H23／H12、数字输入、AM/PM、方向键和边界 Tab 传播 | 保留其候选时间和分段编辑引擎，滚动列操作同一候选；不复制 SegmentEditor |
| TimeField 的实际缺口 | 没有滚动列、空值、步长、联动禁用／候选验证、readonly；当前步进是单位内回绕且不进位 | 缺口用显式 opt-in Base 行为扩展补齐；旧默认行为保持不变 |
| Base InputState／YueInput | 完整字符编辑、选区、粘贴、撤销、IME 和状态外观 | 字段继续用真实 Input；不以 TimeField 的数字缓冲冒充文本编辑器 |
| [Input surface](../crates/yue-component/src/input/frame.rs) | `surface_style` 已由 YueInput 与日期复合字段共用 | 范围字段用同一外框，双端输入内部不叠两层边框 |
| [日期状态](../crates/yue-component/src/date_picker/state.rs) | 已有草稿确认、TimeField 组合、输入去重、会话代次、外部静默赋值和关闭焦点路由 | 借鉴已测试合同；共享确实相同的时间规则，不复制整个 DatePicker controller |
| Popup | 已有锚点、碰撞、可用高度、进出生命周期、关闭路由和减少动态效果 | 直接组合，不新建窗口级 owner 或全局点击监听 |
| Root 焦点 | `focus_next`／`focus_prev` 及 modal-aware 遍历已经存在 | Tab 离开组件时委托 Root；不另建 focus trap |
| Select／虚拟滚动 | 项目已有活动项可见性、单滚动 owner 和大列表虚拟化合同 | 时间列至多 60 项，默认用原生 ScrollHandle；不为短列接入虚拟化或创建第二个滚动 owner |
| YueList | 内容布局组件，宿主负责状态和滚动；不是 listbox 行为原语 | 不强行套用其内容列表语义；时间列使用 Base 选择语义零件 |
| Button／Tooltip／Icon | 标准命令、图标说明、Tabler 图标齐备 | 此刻／确定／取消／清除使用 Button；图标直接用 `outline::Clock`／`X` |
| Field／Form | 已支持类型化 DatePicker 校验与错误项聚焦 | 同模式新增 `.time_picker(...)`、`.require_time_picker(...)`、`.rule_time_picker(...)` |
| motion／typography／tokens | 已有动画策略、数字字体、语义 token 及同步工具 | 数字等宽，Popup 动效直接复用；补缺少的时间列 token |

复用不等于跨兄弟模块访问私有字段。公共类型保持从 crate 根导出；时间基础规则通过 Base 时间原语复用，不把 TimePicker 变成 DatePicker 的依赖。

## 4. 值、精度与格式

### 4.1 公共值

`TimePickerValue` 为 `Empty`、`Single(NaiveTime)`、`Range(start, end)`；已提交范围必须完整且 `start <= end`。半范围仅作为候选，不向 Form 暴露为完整值。多选时间点、时长及重复规则不是本组件的选择模式。

- H12 只改变显示；内部始终使用 00..23 小时。12 AM 为 00 点，12 PM 为 12 点。
- `24:00`、闰秒与隐藏的次秒值不作为合法 UI 值。先拒绝 chrono 可表示的闰秒，再按分钟精度归零秒和次秒、秒精度归零次秒；随后执行步长和限制校验。
- `min/max` 是同一日的包含边界，并在构造时按相同精度规范化；`min > max` 为配置错误。
- 相同起止默认合法。需要会议时段严格正时长时，由范围 predicate／Form 规则禁止相等。
- 结束早于开始不解释为次日，也不自动交换端点；日期时间跨日使用现有 DatePicker。

### 4.2 格式合同

格式沿用项目的 chrono 指令，参考中的 `HH:mm:ss` 不直接用作 Rust format。首版提供可逆白名单，`format` 缺省时根据 precision 与 hour_cycle 推导；显式格式与它们冲突则返回配置错误。

| precision | hour_cycle | 格式 | 示例 |
| --- | --- | --- | --- |
| Minute | H23 | `%H:%M` | `09:30` |
| Second | H23 | `%H:%M:%S` | `09:30:15` |
| Minute | H12 | `%I:%M %p` | `09:30 AM` |
| Second | H12 | `%I:%M:%S %p` | `09:30:15 PM` |

默认 Second／H23／`%H:%M:%S`，与时间选择器参考一致；不改变 Base TimeField 和现有 DatePicker 的默认精度。

解析允许外侧空白、AM/PM 大小写，数值位数及分隔符必须符合格式；消耗完整文本，拒绝尾随字符、缺失单位、负数、越界值及小数。H12 必须明确 AM/PM。首版不解析“晚上九点半”或自动猜测中文日段，不新增第二套自然语言解析器。

`editable(false)` 只禁止手动输入，仍能打开面板；`readonly(true)` 保留焦点、复制与查看已提交文本，所有值修改、清除和面板打开入口均禁用；`disabled(true)` 不接受交互且不进入 Tab 顺序。

## 5. 唯一状态所有者与基础扩展

| 事实 | 唯一 owner |
| --- | --- |
| 已提交类型值 | YueTimePickerState |
| 每端当前候选的时分秒 | Base TimeFieldState；列与分段字段共用该 Entity |
| 候选为空／完整性 | Yue controller 的每端存在性标记；空时 Base 的暂存数值不是有效候选 |
| 输入字符串、选区、IME、撤销 | 对应 Base InputState |
| 列活动项与滚动位置 | 每个 endpoint＋segment 的保留焦点／ScrollHandle；不保存另一份候选时间 |
| 会话代次、错误、活动端、打开 | Yue controller |
| Popup 几何、退出生命周期 | 已有 Popup runtime |

输入文本与候选值是不同编辑阶段：文本仅在明确接受时解析到候选；列表修改候选后按格式回显输入。同一时间事实不得同时保存在 `panel.value`、`selected_items` 和 `TimeFieldState.time` 中。取消恢复已接受快照，不依赖重新验证历史快照才能回滚。从空值首次选择暂存数值相同的项目，也必须把该端标记为存在；不能只依赖 Base 的“数值变化”事件判断用户是否已经选择。

Base 扩展方向：保留 TimeFieldState，引入可装饰的无样式时间列零件，以及显式的刻度／约束选择策略。数字分段、列点击、键盘步进先计算提案，由同一策略验证或修复后原子写入候选，最终只发一次用户 Change；不要先写非法值、发事件，再由 Yue 订阅者纠回。

已实现的 Base 扩展为 `TimeRules`／`TimeSteps`／`TimeChoices` 和 `TimeOption`。TimeField 暴露 `input_proposal`／`accept_time_proposal`，支持上层原子接受；原生分段编辑可通过 `set_selection_rules` 显式启用约束。TimeField 旧客户端未启用策略时，仍保持现有分段回绕、默认步长、事件和 `set_time` 合同。共享基础层不能引用 Yue theme、TimePickerValue、Popup 或业务服务。

`set_value` 静默、校验失败保持旧值／编辑不变；成功时使旧 continuation 失效，即使值相等也要清理过期会话。`set_constraints` 接受合法配置、使旧候选失效，保留历史已提交值；若历史值现已不可用，标出错误并让 Form 拒绝提交，不自动替用户选择其他时间。

## 6. 编辑、提交与事件

### 6.1 提交策略

`TimePickerCommitMode::{Confirm, Immediate}`。默认 Confirm，不使用与日期组件含义可能不同的 Auto。独立面板由宿主显式配置 Immediate；若沿用 Confirm，则保留确定／取消，不因某帧换外观偷偷改行为。

| 操作 | Confirm | Immediate |
| --- | --- | --- |
| 点击列／列内方向键／分段步进 | 更新候选、回显及 DraftChange | 完整合法候选更新已提交值；范围只有首端时仍为草稿 |
| 在已打开 Popup 的字段按 Enter | 接受文本、复验整个候选、提交并关闭 | 同左；不重复发布相同值 |
| 列内 Enter／确定按钮 | 完整合法才提交并关闭 | 列内 Enter 结束会话；不重复提交 |
| 在关闭字段中 Enter／离开整个控件 | 合法完整文本一次性提交，非法保留并显示错误 | 同左 |
| 点击外部／Tab 离开已打开 Popup | 取消未确认候选，不抢外部目标焦点 | 保留已提交值，丢弃不完整／非法文本 |
| Escape／取消 | 恢复已提交快照，关闭并恢复活动端焦点 | 同左，但不会撤销已发布的即时变更 |
| 清除 | 立即提交 Empty 并关闭；无值则不发 Change | 同左 |
| 此刻 | 设置合法候选，仍需确认 | 设置并即时提交；保持面板可继续编辑 |

字段或面板内部焦点转移不是离开整个控件，不提前提交或取消。输入非法时保留用户文本；错误通过 Field 消息／辅助描述呈现，不以摇晃动效代替解释。

在空值上首次打开只展示可操作的默认活动项，不自动提交午夜或此刻。首次选择一个单位时，以最早可用时间作为完整候选种子，再应用该单位选择；没有任何合法时间时保留空、禁用确定并显示“没有可选时间”。

### 6.2 事件

`TimePickerEvent` 采用 `subscribe`，不再公开同义 `on_change` 回调。事件快照私有字段配 getter。

| 事件 | 合同 |
| --- | --- |
| `Change` | 仅真实已提交值变化；携带 value 和 source（Input／Panel／Now／Clear）；一次逻辑操作最多一次 |
| `DraftChange` | 候选变化，携带含空端的只读候选快照；不得被宿主当作保存成功 |
| `OpenChange` | 实际打开／关闭；携带原因；单纯重复 open／close 不发事件 |
| `InvalidInput` | 一次明确接受失败，携带端点与错误；不每次按键刷屏 |

先更新候选并发布 DraftChange，再在需要时更新已提交值、发布 Change，最后安排关闭。Enter＋blur 同次接受去重；IME composing 期间 Enter 不提交。所有 deferred close／焦点续接检查会话代次与存活状态，允许宿主在 Change 中同步 `set_value`、重开或卸载。

Form 只读取已提交值及当前错误。日期／时间字段内部 Enter 先完成自己的输入，不直接触发表单保存。

## 7. 刻度、禁用与候选修复

### 7.1 刻度规则

`TimePickerSteps` 提供 hour／minute／second；默认均为 1。小时步长为 1..=24，分钟／秒为 1..=60，等于基数时该列只保留 00；零或超界返回配置错误。分钟精度下不能配置非默认秒步长。

刻度集合从 0 开始：例如 minute=15 是 00／15／30／45，minute=7 是 00／07／…／56，不要求步长整除 60。H12 的小时刻度按真实 H23 小时生成后分组显示；例如 hour=2 在上午显示 12、02、04、06、08、10，下午对应 12、02、04、06、08、10，不能把“1 开始每隔两项”误当作偶数小时。

步长是值约束，不只是显示过滤。文本、程序赋值、此刻、预设和列点击都必须满足刻度；手动输入与 `set_value` 不静默吸附。精度截断不等于步长取整。

### 7.2 限制

`TimePickerConstraints`：包含式 min／max、`disabled_hours(hour)`、`disabled_minutes(hour, minute)`、`disabled_seconds(hour, minute, second)` 和完整范围的 `range_allowed(start, end)`。回调返回 true 表示禁用，与名称一致；均须纯、同步、廉价，不在 paint 期间通知宿主或进行 I/O。

单值合法性由边界、刻度和逐层单位规则共同决定。范围 predicate 仅在完整候选提交时评估，不宣称它能逐格精确预测任意业务冲突。宿主若要阻止覆盖已有预约，使用 range_allowed 或 Form；时间轴和网络请求留在示例宿主。

`hide_disabled(true)` 只改变显示，不能改变校验；保留当前历史选中项供解释时，必须标明已不可用、不可再次提交。全禁用时提供明确空提示，不留下一个可提交的默认值。

### 7.3 选择可达性和修复顺序

小时项可选，当且仅当该小时有至少一个符合边界、刻度及单位规则的分秒组合；分钟项同理。不能因为“把小时替换后保留旧分钟恰好非法”就将整个小时错误禁用。例如 min=09:30，09 时仍可选，00..29 分禁用。

用户选择小时后保留原分钟／秒；若下级组合非法，在**所选小时前缀内**选与原低位时间距离最近的合法组合；选择分钟时只在所选时分前缀内修复秒。距离相同时取较早值。没有合法组合则拒绝整次选择，不跳到另一个小时、不发布半修复状态。切换 AM／PM 同样复验，不绕过限制。

“此刻”在点击时读取注入时钟，按 precision 截断，在全部可选刻度中找距离当前墙上时间最近的合法值，平局取较早；此刻不可精确选择时提示实际采用的值，没有可选值则保持原候选。距离按同一天线性计算，不绕过午夜。Clock 只在命令时求值，不挂每秒刷新任务。

实现按小时／分钟逐层剪枝，单次完整秒精度搜索上界为 86,400 个组合，候选修复固定小时前缀最多 3,600 个组合。不得在每个项的 paint 中分别执行完整搜索；一次交互准备所需可选性快照，render 只消费结果。只有测量发现必要时才引入带配置版本失效的缓存，不预建大日期／时间表。

## 8. 时间区间与窄布局

范围字段是共用外框内的双输入、分隔符及后缀；弹层通常并排开始／结束两组列，标题及字段活动标记对应当前端点。可选颜色之外仍有“开始时间／结束时间”文本，不能只靠蓝色区分。

时间范围保留端点身份，与日期组件的反向范围排序有意不同：

1. 用户在开始面板选到晚于现有结束的时间，尝试把结束调整到不早于新开始的最早合法值；两端原子接受，候选只通知一次。
2. 若找不到合法结束，或完整范围 predicate 拒绝联动结果，则拒绝整次面板操作并解释原因。不能靠自动跟随绕过禁用。
3. 结束面板不允许早于当前开始；相同时间是否允许由业务规则决定。
4. 输入与程序赋值要求显式合法起止，不自动跟随、不排序。`18:00 – 09:00` 提示“结束时间不能早于开始时间”。
5. 没有结束时不自动提交。输入首端后可以继续编辑另一端；点击确定前必须完整。

范围跨午夜由宿主使用日期时间／带日期的模型表达。不要把第二端偷偷加一天，也不把 `00:00 – 00:00` 解释为 24 小时。

并排宽度按可见列数、列宽、padding 和 gap 实测。空间不足时切为单个活动端面板，上方用现有 YueSegmented 切换“开始／结束”；候选、焦点和已提交值保持不变，不在窄屏简单缩小命中尺寸。单面板在最小支持窗口及 rem=24 下仍需可用；Popup 约束不足时减少可见行数，列内滚动，页脚保持可达。

## 9. 键盘、滚动与无障碍

### 9.1 键盘路径

| 位置 | 按键 | 行为 |
| --- | --- | --- |
| 字段 | Alt+Down／时钟按钮 | 打开面板；键盘打开定位到活动端当前单位，不修改值 |
| 字段 | Up／Down | 文本可解析时沿选区所在单位增减一个合法刻度；遵守 Base 分段单位内回绕、不进位；非法文本先报错，不丢弃文本 |
| 字段 | Left／Right、Home／End | 保留 Input 原生光标与选区行为，不拿来切时间列 |
| 字段 | Enter | 非 IME 合成时接受完整输入；打开时同时请求确认 |
| 列 | Up／Down | 移动并选择相邻可选项，跳过禁用，不在首尾回绕 |
| 列 | Home／End | 选择该列首／末可用项 |
| 列 | Left／Right | 切换单位列，定位候选对应项；边界不跳到另一端 |
| 列 | 数字、A／P | 复用 Base 数字缓冲和 H12 语义，按相同候选策略验证 |
| 列 | Enter／Space | Enter 确认整个合法候选；Space 选择活动项；阻止父级重复确认 |
| 组件内 | Tab／Shift+Tab | 沿字段、按钮、各列及页脚遍历；从组件边界离开时遵循第 6 节关闭合同 |
| Popup | Escape | 只关闭顶层本组件，回到活动输入端；不同时关闭外层 Dialog |

每列一个 roving 活动焦点，不将 60 个选项全部加入 Tab 顺序。Base 分段字段保留内部 Tab 行为，到边界后再传播给共享 Root；keybinding 必须接入具名上下文，不仅依赖 raw key_down 捕获已被 Input 消费的按键。

字段单位映射由已支持的格式和实际 UTF-16 选区生成；不在任意鼠标 x 坐标上猜时分秒。跨多段选区时使用起始段；位于分隔符时归右侧段，尾部归末段。IME composing 时不执行时间步进，不覆盖 marked text。

### 9.2 滚动与动效

每列一个原生 ScrollHandle，列是唯一垂直滚动 owner。鼠标滚轮默认只浏览，不悄悄修改候选；点击／键盘选择后将选中项滚入稳定可视位置，优先约第三行，边界夹紧。尾部留白用于最后一项可见，不生成虚假选项。

初次打开、外部赋值、主题／rem／窗口调整用立即定位；用户连续选择使用现有 motion 的 Interaction／Standard 策略驱动有限滚动过渡，中断从当前位置衔接。用户主动滚动时停止程序定位，不能每帧把列表拉回选中项。不要同时叠加原生滚动惯性和第二个持续动画 owner。

默认可见约六行，滚动条采用项目已有约定，至少保留滚轮、拖动及键盘可达性。Popup 直接使用既有进出动效和 reduced-motion；错误不抖动，不复制 demo 的独立 CSS 时序。

### 9.3 无障碍

字段保留真实编辑语义；关联必填、错误、说明和展开状态。范围两端分别命名。列为 ListBox，项为 ListBoxOption，提供 selected／disabled、稳定值名称及真实活动焦点。选中、焦点、hover 分开呈现；小时制与上午／下午名称不可只剩空白标题。

外部点击／Tab 关闭不恢复焦点，避免抢走用户目标；确定／Escape 恢复活动端。退出期间面板停止接受交互，卸载后不留旧命中。Windows Narrator 的实际输出与无头语义树分开验收，不把新增 Windows COM provider 当作本组件前提。

## 10. 外观、尺寸与 token

沿用 InputSize 的 Small／Medium／Large，默认 Medium；不因截图含 mini 再增加一档公共尺寸。字段高度、边框、readonly／disabled／error／warning 由已有 Input token 决定。时钟、前缀、清除按钮使用现有 Icon 与 Button 尺寸。

时间列的默认结构：列标题、约六行选项、hairline 分隔、页脚。数字使用现有等宽数字排版；H12 的 AM／PM 列有明确标题。选中项使用选择色及字重，焦点用独立焦点环，禁用无 hover 激活。背景、圆角、阴影沿用 Popup，不从网页截图取色。

已新增 `time-picker.css` 来源 token，几何仍需在实窗核对：

| 实际 token | 用途 |
| --- | --- |
| `--time-picker-item-height-sm/md/lg` | 28／32／40 px，经 rem 随字号缩放 |
| `--time-picker-column-width` | 默认 56 px，同样按 rem 缩放；窄窗口按可用宽度收敛，H12 不裁掉日段列 |
| `--time-picker-panel-gap` | 分组及页脚间距，引用已有 space-12 |
| `--time-picker-item-radius` | 引用已有 radius-md |
| `--time-picker-hover-background` | 引用已有 Ghost Button hover |
| `--time-picker-selected-background/color` | 引用 accent-subtle／accent-solid |
| `--time-picker-disabled-color/focus-color` | 引用 text-disabled／focus-ring-color |
| `--time-picker-popup-gap/window-margin` | 引用 space-4／space-8 |

来源已注册到邻仓 `Yue-Design-System/packages/tokens/src/component-tokens/_index.css`，由同步工具生成 Yue 映射；不能只提交生成文件而遗漏邻仓来源。可见行数根据可用高度在 2–6 行间调整，时间列复用 Base Scrollbar。数字字体只用于时间项，中文标题与错误使用正文排版。

## 11. 公共 API

以下为当前公开 Yue API；完整可编译用法见 [Showcase](../examples/windows-showcase/src/pages/time_picker.rs)。结构配置在构造时校验，不能下一帧通过 presentation builder 偷换时制／精度／选择模式。

```rust
pub enum TimePickerSelection { Single, Range }
pub enum TimePickerCommitMode { Confirm, Immediate }
pub enum TimePickerValue {
    Empty,
    Single(NaiveTime),
    Range(NaiveTime, NaiveTime),
}
// TimePrecision、HourCycle 复用现有 Base 类型，不另定义等价枚举。

let options = TimePickerOptions::new("opening-hours")
    .selection(TimePickerSelection::Range)
    .precision(TimePrecision::Minute)
    .hour_cycle(HourCycle::H23)
    .commit_mode(TimePickerCommitMode::Confirm)
    .steps(TimePickerSteps::new().minute(15))
    .constraints(TimePickerConstraints::new()
        .min(open_time)
        .max(close_time)
        .disabled_hours(|hour| hour == 12)
        .range_allowed(|start, end| start < end));

let state = cx.new(|cx| {
    YueTimePickerState::new(options, window, cx)
        .expect("valid static time picker options")
});
state.update(cx, |s, cx| s.set_value(value, window, cx))?;

YueTimePicker::new(&state)
    .size(InputSize::Medium)
    .status(InputStatus::Normal)
    .editable(true)
    .readonly(false)
    .disabled(false)
    .clearable(true)
    .bordered(true)
    .leading_icon(outline::Clock)
    .placeholder("请选择时间")
    .range_placeholders("开始时间", "结束时间")
    .hide_disabled(false)
    .show_now(true)
    .footer(|interactive, window, cx| extra_information(interactive, window, cx));

// 使用独立 Entity；需要直接响应时，在该 state 的 options 中设为 Immediate。
YueTimePanel::new(&panel_state).size(InputSize::Medium);
```

示例的静态配置在 Entity 构造闭包内处理错误；动态配置应把配置错误返回宿主，不能保留一个 `Entity<Result<...>>` 假充控制器。

控制器提供 `value()`、`error()`、`is_open()`、静默 `set_value(...)`、`set_constraints(...)`、`open(...)`、`close(...)` 和 `focus(...)`。外部 open 同样受 disabled／readonly 限制。初值通过宿主创建后 `set_value` 注入，不并存两套受控 value／defaultValue 语义。

`footer` 只添加内容，不替换必要的提交／取消合同；退出期间收到 `interactive=false`，宿主自定义按钮应禁用。Popup 首版固定请求 Bottom／Start，再由已有 runtime 碰撞处理；没有新增 placement、DOM 容器、CSS class 或任意 HTML trigger API。

## 12. 与 DatePicker 的共享及迁移

首版把刻度、限制、可达性和候选修复放在无样式 Base `time_field/rules.rs`，Yue 直接组合现有 TimeFieldState 和 InputState。DatePicker 继续复用同一个 Base 时间编辑器。日期输入仍使用自己的日期时间格式／范围语义，尚未迁移到时间列，也未为了统一解析器改动其既有合同；后续出现第二个真实消费方时再提取共同解析片段，无日期规则不得依赖 DatePickerUnit 或日期 controller。

DatePicker 继续持有自己的完整日期时间事务，TimePicker 继续持有时间事务；不能把一个完整 TimePicker controller 塞进另一个日期 controller，造成两份提交值、Popup 和确认按钮。将来 DatePicker 使用时间列时，直接给列 adapter 传现有 `times[index]` 与日期相关约束。

首版迁移保留 DatePicker 已有 TimeField 呈现、默认格式、精度和事件顺序，不强制换成更宽的多列面板。可先共享纯规则及 Base 校验入口；任何迁移必须重跑现有日期 23 项专项与 Input／Field 回归。日期时间的 `time_allowed(NaiveDateTime)` 仍由日期控制器组合真实日期后判断，不能降级成只比较时分秒。

Base 修改同时补充 [YUE-PATCH](../vendor/gpui-base/YUE-PATCH.md)：显式 opt-in、旧合同测试、来源及上游移除条件。不得为了新组件改 TimePrecision 默认值，或让原 TimeField 的静默 setter 开始发送用户事件。

## 13. 分期与未纳入能力

| 能力 | 决定与理由 |
| --- | --- |
| 毫秒 | 后续独立项。当前 TimePrecision／TimeSegment、截断、数字缓冲均只支持分钟／秒；不能只加一列而丢失精度。确需实施时再扩 Base、格式、规则及完整测试，默认毫秒步长 100；步长 1 的 1000 项列优先复用现有虚拟化 |
| 滚动结束吸附并选中 | 后续可选。首版滚轮只浏览，点击／键盘才改变值；不把截图 `scrollSticky` 与“选中项滚入可视区”混为同一能力。需要时须定义停滚、拖动、取消和重复事件合同 |
| 小时精度／任意重排列 | 首版仅四种可逆格式；Base 当前不支持 hour-only，不伪装忽略分钟就是支持 |
| UTC／timezone | 时间点需要日期才能决定偏移、夏令时和跨日；由宿主用带日期模型转换。无日期 TimePicker 不接收 timestamp，不造假的 UTC 开关 |
| 跨午夜区间 | 使用日期时间；未来确需每日循环时段时引入明确的次日标记／领域值，不滥用反向时间范围 |
| 范围自动排序 | 不采用参考 `order=true`；保留“开始／结束”身份，面板联动规则见第 8 节 |
| 四档尺寸／任意自绘时间项 | 保留已有三尺寸，首版没有实际需求就不增加；额外信息用 footer |
| 会议时间轴／预约数据库 | 仅作为 Showcase 宿主组合，非 TimePicker 内置能力 |

## 14. Showcase 与实施顺序

Showcase 新增“时间选择 TimePicker”，覆盖以下真实交互场景，不以静态截图或“已点击”提示代替值更新：

1. 结构和五态时间项；独立面板，分钟／秒、H12／H23及可选刻度。
2. 单值字段三尺寸、前缀、warning／error、disabled／readonly／editable(false)、无边框、清空。
3. 默认确认与即时提交对照；显示候选与已提交值、真实事件次数，验证 Escape／取消。
4. 严格文本、粘贴和非法输入修正；不在渲染时自动改正用户字符串。
5. 范围双端、开始联动结束、结束禁用、同日相等／严格正时长、窄窗口切换。
6. 分钟 15、秒 10、自定义非整除步长；输入／此刻／外部赋值同规则。
7. 营业时间、午休、17:30 后禁用、隐藏禁用项、全部不可用、规则变更后的旧值。
8. 静默外部同步、打开中重置、Field／Form 保存与聚焦、附加页脚。
9. 会议室预约：时间区间＋15 分钟步长＋本地已占用区间，预览用 DraftChange，提交预约读 value；冲突、正时长与半开区间 `[start, end)` 由宿主处理。

按以下阶段实施，各阶段保留可审查差异，不自动提交或推送：

1. **规则与 Base**：精度／格式、刻度／限制／修复、可复用时间列行为、旧 TimeField 兼容测试；先打通一个真实单值面板。
2. **单值字段**：Input＋Popup＋列、确认／即时、此刻、清除、错误、IME、焦点、尺寸和状态。
3. **范围及组合**：双端与原子联动、窄布局、Field／Form、外部同步、DatePicker 时间编辑兼容及既有回归。
4. **交付**：完整 Showcase、主题／rem／资源与帧停止验证、文档实际 API／测试结果，用户负责 Windows 真机视觉和真实输入法验收。

以上四阶段首版均已落地；具体自动覆盖和真机待验收范围见第 16 节。

## 15. 验收清单

| 层级 | 必验合同 |
| --- | --- |
| 纯规则 | 00:00／23:59:59、12 AM／PM、精度截断、拒绝 24:00／闰秒、严格格式／完整消耗、步长边界及非整除步长 |
| 候选求解 | min=09:30 不误禁 09 时、依赖分钟／秒、AM／PM 不绕禁用、最近值平局、修复不可达、所有单位全禁用、同日此刻不跨午夜 |
| 输入 | 真实键入／粘贴、选区步进与单位回绕、非法文本保留、Enter＋blur 去重、IME Enter／marked text 不丢失 |
| 单值事务 | Confirm／Immediate、开空不提交、此刻、清除、取消、相同值、程序赋值不回声、Change 内重入／卸载／旧关闭无效 |
| 范围 | 半范围不可提交、开始联动结束原子性、失败不改变任何一端、结束早于开始拒绝、不排序、不暗含跨日、range predicate |
| 鼠标与滚动 | 真实项点击一次、滚轮只浏览、选中项可见、不抢主动滚动、隐藏禁用后稳定 ID、退出后无旧命中 |
| 焦点／可访问性 | 每列一个活动项、Tab 与 ShiftTab 双向、Home／End、顶层 Escape、外部目标焦点、ListBox／Option 和 disabled／selected 语义 |
| 状态／表单 | readonly／disabled 所有入口拦截、editable(false) 仍可选、必填／错误命名、保存仅读已提交值、规则变化旧值不能悄悄保存 |
| 几何 | 三尺寸、浅深主题、rem 16／20／24、窄窗口单端、列间边界、滚动可达末项、页脚与焦点环不裁切 |
| 兼容／资源 | 现有日期专项和 Input／Field 不退化、Base 旧 TimeField 分段规则不变、静态不持续刷新、无逐项 Entity／计时器、关闭卸载订阅 |
| Windows 真机 | 字体、像素、滚轮／拖动感受、真实 IME、Narrator 名称／顺序、显示缩放与 Popup 方向；无头结果不能替代 |

测试优先复用 Base `time_field::tests`、日期的输入／焦点／重入测试和 Select／Popup 的原生事件 helper，断言实际值、事件、焦点与几何，不只调用内部 setter。

复测命令：

```powershell
cargo test -p yue-component --locked --features headless-tests --lib
cargo test -p yue-component --locked --features headless-tests --test input --test field
cargo test -p windows-showcase --locked --features headless-tests
cargo clippy -p yue-component -p windows-showcase --locked --all-targets --features headless-tests -- -D warnings
node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check
pwsh -NoProfile -File tools/test-gpui-base.ps1 -Offline -Filter time_field::tests
git diff --check
```

新增 Base 时间列应再运行其专项目标。格式检查只覆盖相关改动，避免递归格式化其他未提交文件。测试使用注入 Clock 与固定 NaiveTime，不等待真实午夜或接入网络。

## 16. 当前交付状态

首版已完成：[组件代码](../crates/yue-component/src/time_picker/mod.rs)、[控制器](../crates/yue-component/src/time_picker/state.rs)、[Showcase](../examples/windows-showcase/src/pages/time_picker.rs)。不启用 Kit Component 层；复用 YueInput、Button、Segmented、Field／Form、Popup，以及 Base TimeField、Root、ScrollHandle、Scrollbar 和 motion。Base 修改与上游移除条件见 [YUE-PATCH](../vendor/gpui-base/YUE-PATCH.md)。

实际合同：

- 默认 Second／H23／Confirm；Minute、H12、Immediate、同日 Range 在构造 options 中固定。
- 每端 TimeFieldState 是唯一候选时间，InputState 持有实际文本；控制器只另持已提交值及候选存在标记。
- 四种严格格式、刻度及禁用规则覆盖文本、列选择、此刻和静默 set_value。范围面板原子跟随，文本反向拒绝；非法文本保留供修正。
- 外部赋值清除半输入缓冲并使旧关闭续接失效；约束变化保留历史值、取消旧草稿并显示不可用错误。
- 原生滚轮只浏览；选择定位约第三行，末项夹紧可见。连续选择用现有 motion，滚轮／拖动中断，尺寸变化和 reduced motion 立即定位；没有永久计时器。
- 范围窄窗口用 Segmented 切换活动端，H12 四列在 rem=24 时按可用宽度适应。独立面板为 Group，Popup 为 Dialog，时间列及选项为 ListBox／ListBoxOption。
- Form 仅读已提交类型值与当前错误；候选不能当作保存成功。完整公开 builder 及控制器方法以源码为准。

Showcase 新增“时间选择 TimePicker”，配置 23 个独立 state，覆盖双独立面板、三尺寸、输入及状态、范围／即时、H12、分钟／秒、15 分／10 秒／7 分步长、限制／隐藏禁用／全禁用、静默赋值／重置、表单，以及带冲突检查的本地会议预约和时间轴。

[21 项时间专项](../crates/yue-component/src/time_picker/tests.rs)覆盖格式／步长／候选修复、指针和键盘、确认／取消／即时、Enter＋blur 去重、IME、合法文本接续选择、范围联动与失败原子性、外部静默赋值及重入、数字缓冲重置、readonly、约束后的历史值、注入时钟、全禁用、Tab 退出、Form、滚轮中断、reduced motion、三尺寸及 rem 16／20／24、360 px 窄窗口 H12 范围与末项可达。

2026-11-08 自动验证：

| 检查 | 结果 |
| --- | --- |
| Yue 库全量无头测试 | 438 项通过，包含 21 项时间专项及原日期 23 项 |
| Input／Field 集成回归 | 10＋3 项通过 |
| Showcase 无头测试 | 49 项通过，9 项已有忽略，包含新页导航 |
| Base `time_field::` 独立测试 | 15 项通过，含原 10 项分段合同、4 项规则和 1 项 opt-in／旧默认兼容 |
| Clippy，两个目标、all-targets、`-D warnings` | 通过 |
| 设计系统 token `--check` | 来源与生成映射一致 |
| 本地文档链接、代码围栏、相关格式与 `git diff --check` | 通过 |

独立可执行文件：`target/debug/windows-showcase-time-picker.exe`，使用以下命令输出，不替换可能运行中的同名主程序：

```powershell
cargo rustc -p windows-showcase --locked --bin windows-showcase -- -o D:/100_Projects/110_Daily/Yue-GPUI-Component/target/debug/windows-showcase-time-picker.exe
```

`-o` 的输出文件名适配与忽略 `--out-dir` 提示为预期编译器诊断。浏览“组件 → 时间选择 TimePicker”即可验收。

Windows 真机仍由用户验收：浅深主题与字体像素、真实滚轮／滚动条拖动感受、真实 IME、Narrator 朗读顺序、DPI 与弹层碰撞。无头几何测试不等同于真实多显示器验收。毫秒、停滚自动选中、时区／UTC、跨午夜和日期面板迁移仍按第 13 节取舍，未自动提交或推送。
