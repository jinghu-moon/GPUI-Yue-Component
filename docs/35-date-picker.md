# YueCalendar / YueDatePicker 日期组件设计

- 状态：核心实现已落地；第 17 节记录实际 API、自动验证及仍需真机确认的项目。
- 日期：2026-11-08。
- 目标：为 Windows GPUI 应用提供可直接输入、键盘操作和组合的日期选择能力，尽可能复用已有基础组件。
- 参考：[TDesign 参考图](assets/date-picker-tdesign-reference.png)、[Arco 参考图](assets/date-picker-arco-reference.png)、[本地 GPUI DatePicker demo](../refer/gpui-datepicker-demo.html)。
- 约束：[组件架构](24-component-architecture.md)、[Input / Field / Form](03-input.md)、[Popup](12-popup.md)、[Select](20-select.md)、[Tag](19-tag.md)。

## 1. 定位与组件划分

主要任务是填写一个日期值或日期集合。宿主负责业务有效性、持久化和时区转换；组件负责编辑会话、可选性校验、日历导航、提交、焦点与外观。酒店价格、库存、预订计费是示例宿主的业务，不放进日期控制器。

| 单元 | 公开职责 | 复用关系 |
| --- | --- | --- |
| `YueCalendar` | 可嵌入页面的日期／年月面板，包含标题、星期行、日期格与页脚 | 包装 Base Calendar 的结构、状态和行为，Yue 提供外观及显式插槽 |
| `YueCalendarState` | 独立面板的会话所有者：选择、活动日期、浏览月份、预览 | 内部保留唯一 Base 日历状态；不能再维护一份同义选择数组 |
| `YueDatePicker` | 字段＋日历弹层，支持单值、范围、多选、不同粒度及日期时间 | 组合 Calendar、既有 Input、Popup、Button、Tag 等 |
| `YueDatePickerState` | 唯一已提交值、输入草稿、打开会话、确认／取消和事件 | 复用日历与输入状态；不复制编辑器、Popup runtime 或业务模型 |

不为 YearPicker、MonthPicker、WeekPicker、QuarterPicker 再各写一套组件和状态机；它们是粒度配置。范围是选择模式，日期时间是时间精度配置，浏览年／月是面板视图，三者不能混成一个 `mode` 枚举。

无时间的日期使用 `chrono::NaiveDate`；带时间使用 `NaiveDateTime`。项目锁文件已含 `chrono 0.4.45`，实现如需直接使用则显式声明与 Base 相容的依赖，不另引日期库。日期不是毫秒时间戳，不能通过“午夜转 UTC”编码普通生日或账期。

## 2. 三份参考的采用与取舍

| 来源 | 吸收的能力 | Yue 决策 |
| --- | --- | --- |
| TDesign | 单日／范围、年月季周粒度、多选、日期时间、快捷项、禁用、可输入与确认配置 | 采用场景覆盖；配置分为粒度、选择模式、时间精度和提交策略，避免无效组合 |
| Arco | 状态、前缀、格式、三尺寸、禁用、额外页脚、自定义格、受控更新 | 复用现有字段样式和槽位；公开类型值与静默赋值入口，后端格式化由宿主完成 |
| Arco 的 UTC 示例 | 区分显示时间与实际时刻 | 首版只输出无时区日期／当地墙上时间；UTC 偏移、时区数据库和夏令时歧义由宿主显式转换，不伪装已经支持 |
| 本地 demo | 八部结构、固定 6×7、今天／选中／焦点、范围预览、键盘、时间列与整体场景 | 作为结构和交互主参考，外观使用 Yue token；浏览器 JS 仅是行为原型 |

参考截图未展开的选项只证明展示了对应入口，不据此声称已核实该库内部语义。以下合同由 Yue 明确决定，不推测截图看不到的行为。

本地 demo 中需要修正的差距：合法输入未完整失焦提交；今天／快捷项和部分粒度入口未统一校验；只读清除缺少完整拦截；日期时间的“确定”主要是关闭，未形成独立提交；JS 动画未统一响应减少动态效果。正式实现不继承这些缺口。

## 3. 基础能力核对与复用硬约束

核对版本为当前仓库实际使用的 GPUI Kit 0.8.0、受维护的 Base 0.8.0 和 GPUI 0.3.9。

| 能力 | 当前源码事实 | 实施方式 |
| --- | --- | --- |
| Base `CalendarState` / `Date` | 已有单日／范围、浏览年月、禁用 Matcher、月／年视图、多月和选择事件 | 复用日期计算和选择规则；不在 Yue 复制第二个日历引擎 |
| Base `Calendar` | 有 `item`、`label`、`first_day_of_week`、`number_of_months` 和可装饰的 `CalendarItem` | Yue 使用这些缝隙注入尺寸、色带和内容；保留已接线的事件及语义 |
| Base 日历差距 | 周数随月份变化；反向点击范围起点会重新开始；没有完整日期格导航、范围悬停预览和多选值 | 在确需时最小扩展无样式行为，并补回归与 [YUE-PATCH](../vendor/gpui-base/YUE-PATCH.md)；新增行为不能放在 paint 或靠两份状态互相同步 |
| Base `DatePicker` | 控制式开关、ComboBox 语义、Confirm／Cancel；不提供字段和定位 | 按输入焦点合同组合，防止父级 Confirm 再次响应编辑器的 Enter；不误当作完整日期选择器 |
| Base Input / YueInput | 已有真实文本编辑、IME、撤销、占位、状态、前后缀、禁用与只读 | 单输入优先直接组合 YueInput；范围共享外框若需要复用 frame，只抽最小具名内部接口，不复制边框和编辑逻辑 |
| Base `TimeField` | 分段时分秒编辑、24／12 小时制、精度与键盘行为 | 日期时间先复用 TimeField；如实现列式面板，两种呈现共用一个时间值，不造另一套时间解析器 |
| 现有 Popup | 锚点、碰撞、可用高度、注册、关闭路由、退出生命周期 | 与 Select 一样组合；不另写 absolute 定位、全局点击监听、应用 Root 或焦点陷阱 |
| Button / Tag / Tooltip / Field / Form | 命令、可关闭标签、图标说明及校验呈现已经存在 | 导航、今天、清除、确定复用按钮；多选复用 Tag；表单验证复用现有合同 |
| Icon / typography / motion / theme | Tabler、字体、数字字体、token 与减少动效策略已有 | 只补缺少的语义映射和组件 token，不复制 SVG、字体或计时器 |
| GPUI Component DatePicker | 已有单值／范围日期时间、预设、时间精度等，但使用另一套主题和呈现 | 阅读其组合经验；不启用当前关闭的 Component feature，也不包装后再覆盖全部样式 |

**复用验收规则**：每项新代码须说明现有能力为什么不能直接满足。不得因自己重写更快而绕开现有 Input、Popup 或 Calendar；不得为每个日期格创建 Entity、任务或订阅。日期网格最多两个 42 格面板，不使用虚拟列表；长时间列可复用既有滚动基础。

## 4. 能力矩阵与有效组合

| 粒度 | 单值 | 范围 | 多选 | 时间 |
| --- | --- | --- | --- | --- |
| Date（日） | 支持 | 支持 | 支持 | 单值／范围可选分钟或秒；多选不组合时间 |
| Week（ISO 周） | 支持 | 支持 | 暂不支持 | 不支持 |
| Month（月） | 支持 | 支持 | 暂不支持 | 不支持 |
| Quarter（季度） | 支持 | 支持 | 暂不支持 | 不支持 |
| Year（年） | 支持 | 支持 | 暂不支持 | 不支持 |

不支持的组合在配置／状态构造时返回类型明确的错误，不静默忽略时间或切换粒度。配置创建后粒度、选择模式与时间精度不可通过下一帧 builder 偷换；需要变更时显式重建状态或调用将旧会话取消的重配置入口。

公共已提交值按粒度使用真实类型：单日／日期范围为 NaiveDate，日期时间为 NaiveDateTime；月、季度、年和 ISO 周使用有合法构造与 getter 的值对象。其内部可转换为起始日期供 Base 排版，但不能在公共事件中把“2026 年第 1 周”伪装成普通某一天。

范围结束值为**包含的最后一个所选单位**；转半开时间区间由宿主负责。周数和周年按 ISO 8601，跨年须同时保留 ISO week-year；Week 模式固定周一开始。`week_start` 影响 Date 模式星期排列，不能改变 ISO 周定义。

月／季度／年规范化到该单位首日；默认禁用判断以单位首日作为代表日期。业务若要求整个单位满足条件，应提供单位可选性 predicate，而不是在格子 renderer 内偷偷禁用。范围端点以粒度排序，不通过格式化字符串比较。

## 5. 状态、草稿与事件

### 5.1 唯一所有者

| 状态 | 所有者 |
| --- | --- |
| 已提交类型值 | `YueDatePickerState`；宿主通过事件读取并持久化，通过静默 `set_value` 外部更新 |
| 输入字符、选区、IME composing、撤销历史 | Base InputState；日历不能保存另一份可编辑字符串 |
| 当前候选日期／范围、多选集合、浏览视图、活动格 | 内部 Base Calendar 状态；作为会话草稿，不冒充已提交值 |
| 日期时间草稿的时分秒 | Base TimeField 状态；与候选日期组合产生 candidate，不重复保留字符串时间 |
| 范围悬停预览 | 日历短期状态，不触发已提交值 Change |
| 打开、活动输入端、会话代次和取消快照 | Picker controller；Popup runtime 只管理存在性和几何 |

独立 Calendar 和 Picker 内部 Calendar 不同时争当同一个已提交值的所有者。嵌入 Picker 时使用其内部状态与提交协调；不能新建另一个 YueCalendarState，再跨帧双向订阅同步。

采用项目现有 retained state 模式：宿主创建并保留 Entity，组件只接收引用。`set_value` 用于外部同步，不触发用户 Change；外部更新使正在编辑的旧会话失效，重新同步输入和候选值。相同值不重复发布变更；不在 render 中 set_value、notify 或创建状态。

### 5.2 事件合同

事件快照使用私有字段及 getter；Change、OpenChange、InvalidInput 携带只读快照，DraftChange 携带可选类型值（None 表示不完整候选），PanelChange 携带视图与浏览年月。

| 事件 | 触发时机 |
| --- | --- |
| `Change(value, source)` | 真实已提交值变化，包含 clear；每次逻辑提交最多一次 |
| `DraftChange(candidate, phase)` | 可选订阅；范围首端、多选草稿、时间草稿变化，不触发网络或保存 |
| `OpenChange(open, reason)` | 实际打开／关闭一次；点击已关闭容器不重复事件 |
| `PanelChange(view)` | 浏览年月／视图变化，与值变化分离 |
| `InvalidInput(error, endpoint)` | 解析或可选性失败；保留可纠正文本与说明，不改变提交值 |

用户路径先完成内部一致状态再通知，回调发出后不继续依赖旧实体借用。`Change` 先于该提交引起的关闭通知；控制器在回调中被关闭或外部更新时，通过代次判断停止旧路径。取消只关闭／回退草稿，不发 Change。

## 6. 提交和取消规则

提供 `DatePickerCommitMode::{Auto, Immediate, Confirm}`。Auto 默认：普通单值与范围完整后立即提交；日期时间和多选使用 Confirm。

| 操作 | Immediate | Confirm |
| --- | --- | --- |
| 单值点击／按 Enter 选中 | 校验后提交并关闭 | 只更新草稿，等确定 |
| 范围第一端 | 记录草稿，进入第二端，不发布 Change | 同左 |
| 范围第二端 | 校验、必要时交换两端、提交并关闭 | 完整草稿，等确定 |
| 时间编辑 | 完整有效时提交；保持面板打开 | 只更新草稿；确定提交并关闭 |
| 多选切换 | 提交新集合；保持打开 | 只改草稿；完成提交并关闭 |
| 清空按钮 | 独立明确命令，立即提交 Empty 并清理草稿 | 同左；不要求再次按确定 |
| Escape / 点击外部 | 关闭并丢弃未提交部分，不撤回已发出的即时提交 | 丢弃整个未确认会话 |
| 确定／完成 | 有效候选才允许提交；同值只关闭 | 同左 |

候选不完整、格式非法、端点不可选、区间违规时禁用确定并提供相邻说明。打开时以已提交值初始化草稿；取消后再打开不能恢复上次未确认的残留。

手动输入成功时，Immediate 在 Enter 或整个复合控件失焦时提交完整值；Confirm 的 Enter 只接受当前编辑端进入候选，确定按钮才发布 Change。失焦进入日历、快捷项或另一个范围输入不算离开组件，不能抢先提交。

反向范围默认排序并完成；如第二端设置为锁定，不能为排序而改动锁定端，应拒绝该候选。一个完整范围上的再次选择从新第一端开始。键盘活动格同样驱动预览，不依赖鼠标 hover 才能理解将要选中的范围。

## 7. 输入与格式

- 默认允许输入；`.editable(false)` 只禁止手动编辑，仍可通过面板选择。它与 readonly 不同。
- `.readonly(true)` 保留读取、聚焦和复制，禁止打开可修改面板、清除、删 Tag、快捷项及键盘修改。disabled 不进入 Tab 顺序，不能打开或修改。
- 复用 InputState 编辑／粘贴／撤销／IME。组合输入期间不得解析、回退或使用 Enter 确认日期。
- 默认格式：日 `%Y-%m-%d`，月 `%Y-%m`，年 `%Y`，分钟日期时间 `%Y-%m-%d %H:%M`，秒追加 `:%S`。格式采用 chrono 语法，文档与 JS 的 `YYYY-MM-DD` 明确区分。
- `.format(...)` 在构造时验证受支持字段及与精度的相容性；不承诺任意格式都可逆。季度和周采用明确的值对象格式化／解析规则；不使用宽松的 JS Date 自动进位。
- 范围由两个真实编辑器组成，同一外框；不通过按 `-` 分割整段字符串识别端点。
- 非法值保留用户文本及相邻错误说明，已提交值不动；Enter 不关闭、不清空，不用抖动作为唯一反馈。Escape 取消编辑并恢复已提交显示。
- 空文本接受为空值；范围仅空一端属于不完整草稿，默认不提交开放区间。整组清空通过清除按钮或两端为空提交，行为一致。
- 同一编辑发生 Enter 后又触发 blur，必须去重。禁用／只读在编辑中变化时，取消旧草稿并保留已提交值。
- `format` 只控制编辑／显示，公共值始终类型化；后端字符串／时间戳转换由宿主负责，不再提供含混的 `value_format`。

## 8. 日期限制与统一校验

所有入口统一经过 `candidate → normalize → validate → commit`：鼠标、键盘、输入、今天／此刻、快捷项、多选移除、确定和程序赋值均不得绕过可选性判断。

配置包括包含式 min／max、日期／单位 predicate、时间 predicate、可选多选上限，以及范围长度／整个区间 validator。默认范围只验证端点，不暗示区间中每一天可选；需要“不能跨禁用日”时显式提供区间规则。跨度长度采用日期差和单位索引计算，不以毫秒除以 86400000 计算自然日。

日期支持年 1..=9999；所有加减月／年、补齐相邻日期与边界翻页使用 checked 运算。极值旁无法表示的补位格保持网格但不可选，不 unwrap 越界。配置 `min > max`、步长为零、与粒度不符的范围限额等直接返回错误。

当禁用规则改变：已有提交值不静默被清空，也不发伪用户 Change；标记不可满足当前规则的值并展示校验信息，下一次新提交必须满足规则。外部 set_value 不合法则返回错误、原值和会话保持不变；需要只展示历史非法值的业务应使用只读文本而非绕过校验。

快捷项是惰性 resolver，在点击时从注入的 Clock 读取“今天／此刻”，不能在创建控件时固定一天。统一校验失败时不关闭、不替换旧值。Clock 可在测试中固定；生产按交互／重新打开刷新，不给每个控件创建跨午夜定时器。

范围的一端禁用是独立能力：锁定端可阅读但不可编辑／替换。整组清除按钮在不能同时清空两端时隐藏；另一端也不能通过排序或快捷项改变锁定端。该能力在范围阶段实施，不照搬截图里的布尔数组到所有单值 API。

## 9. 结构与布局

### 9.1 八部结构

1. 字段：共用 Input 的尺寸、状态、焦点环和内容槽。
2. 值／占位：一个或两个 Base 编辑器；多选用 Tag 及输入区域。
3. 后缀：日历／清除，命中大小与图标绘制尺寸分开；清除可通过键盘到达，不能仅 hover 出现。
4. 弹层：既有 Popup，默认下方起始侧、4px token 间距，空间不足翻转并约束可用尺寸。
5. 面板头：上一年／上一月、可点年／月标题、下一月／下一年；各为语义按钮。
6. 星期行：七列；Date 默认周一，可切周日；Week 固定 ISO 周一。
7. 日期格：日视图恒定 6×7；年视图 12 格（当前十年＋相邻两年），月视图 12 格，季视图 4 格。
8. 页脚：可选今天／此刻、快捷项、额外说明与确定／完成，不把不同业务按钮固定塞进组件。

双面板展示连续月份；共享导航月份、活动日期和候选值，不能是两个独立 Picker。相同日期在相邻月补位中重复出现时，只允许一个活动 Tab 目标，ID 必须含面板身份；完整日期仍用于选择身份。

相邻月日期可选并导航到对应月份，弱化样式不等于禁用。切换日历月份高度固定；切换日／月／年视图保留日期主体区域的最小高度，避免弹层跳动。隐藏页脚不留下虚假空行，出现错误说明可以正常增加高度。

### 9.2 容器、窄窗口与时间面板

主体宽度由七列尺寸及内边距决定，不从截图设置独立固定宽度。三档参考：Small 28、Medium 32、Large 40 的字段高度和日期格步长，均转换为 rem 并通过 token 调整。自定义价格内容可以增加统一 cell 高度，不能只有某一天撑高一行。

Popup 采用现有实测位置，优先翻转／平移，不在退出动画后保留可命中的旧日期。单面板受容器约束；范围双月宽度放不下时降为单月视图，两个输入端和选择能力保留。日期时间面板横向放不下时将时间区放在日历下方，不缩小日期格、裁掉确定按钮或引入横向页面滚动。

字段、值区、弹层文本列使用 `min_w_0`，多选 Tag 可换行／折叠；默认最多显示两个 Tag，其余 `+N` 通过现有浮层／说明入口可查阅和移除，不能只显示计数而无恢复路径。

只有实际需要滚动的 Popup 内容区或时间列拥有滚动；不让 Calendar 根、Popup 根、页面三层抢滚轮。时间列至多 24／60／60 个项，可直接复用既有滚动容器与 ScrollHandle，选中项滚入视口；没有证据不添加虚拟化。

## 10. 外观、状态与动效

默认状态区分如下：今天为细描边；单值／端点为强调色实心；范围中间为连续浅色带；键盘焦点有独立可见环；disabled 为不可选且可解释；非本月仅弱化。选中＋今天仍能辨识；选中＋焦点保留焦点环，不能只靠颜色表达。

普通周／月切换可复用短淡入／平移，视图切换可短缩放；动效使用 Yue motion，不复制网页常量 180ms 或 WAAPI。首阶段以正确静态状态优先，动效阶段补连续中断、减少动态效果和静止停止请求帧验收。范围 hover 预览立即更新，不使 42 格一起重复播放缩放。

不复制网页 cursor:pointer 到全部日期格／按钮，遵循桌面箭头约定。没有独立 DatePicker 阴影引擎或持续闪烁反馈。

### 10.1 token 计划

当前没有日期组件专用 token。实现时在设计系统定义 `calendar.css`／`date-picker.css` 并注册，再同步快照；不手写 generated.rs 或 inventory.json。

| 拟新增来源 | 引用已有 token／机制 |
| --- | --- |
| `--calendar-cell-size-sm/md/lg` | `--size-28`／`--size-32`／`--size-40` |
| `--calendar-cell-height-sm/md/lg` | 默认对应 cell size；统一增加高度承接自定义内容 |
| `--calendar-font-size-sm/md/lg` | `--input-font-size-sm`／`--input-font-size-md`／`--input-font-size-lg` |
| `--calendar-color`／`muted-color`／`disabled-color` | `--text-primary`／`--text-secondary`／`--text-disabled` |
| `--calendar-selected-background`／`selected-color` | `--accent-solid`／`--on-accent` |
| `--calendar-range-background`／`today-color` | `--accent-subtle`／`--accent-solid` |
| `--calendar-cell-radius`／`content-inset` | `--radius-md`／`--space-2` |
| `--calendar-focus-color` | `--focus-ring-color` |
| `--date-picker-popup-gap` | `--space-4` |
| 面板颜色、边框、圆角、阴影、动效 | 直接复用 `--popover-*` 与 Popup 适配，不新建同义 token |
| 字段高度、padding、圆角、前后缀、状态 | 直接复用 `--input-*`，不在日期 token 再抄一遍 |

文本使用现有 UI 字体与数字等宽能力；所有尺寸随 rem 变化。日期 cell 显示数字且可访问名称包含完整年月日；价格／标记使用现有颜色角色，自定义渲染不得硬编码一套浅色背景。

## 11. 键盘、焦点与无障碍

| 上下文／按键 | 行为 |
| --- | --- |
| 字段 Down / Alt+Down | 打开，聚焦当前端已选日期；无值时到今天或范围内最近可展示日期 |
| 日期 Left / Right / Up / Down | 一天／一周；跨月同步浏览，禁止以键盘移动偷偷提交 |
| Home / End | 当前星期行首／尾 |
| PageUp / PageDown | 上／下月，保留日号并按目标月合法日数收敛 |
| Shift+PageUp / Shift+PageDown | 上／下年，闰日收敛 |
| Enter / Space | 激活当前日期／单位，统一校验，disabled 不提交 |
| 月／年／季网格方向键 | 按实际列数移动，边界翻页，Home／End 到行边界 |
| Escape | 仅关闭最上层日期弹层并取消未提交草稿；返回活动输入端 |
| Tab / Shift+Tab | 日期网格只一个活动 Tab 点；继续经过时间区、页脚等到下／上一个控件，不形成隐藏陷阱 |
| 清除／Tag 移除 | 真实按钮可键盘激活，恢复合理字段焦点，不把 Backspace 从编辑器全局截走 |

日期禁用格仍允许网格内方向键聚焦并说明禁用原因，但不可激活；这与整个控件 disabled 的 Tab 排除不同。年份等导航按钮的可达边界必须同步可用状态。

弹层是非模态日期选择对话区域，嵌套在 Dialog 内沿用同一 Popup 路由；关闭时不能同时关闭外层 Dialog。失焦到外部按提交策略处理，然后允许焦点自然到达目标；点击外部关闭不把焦点抢回输入框，Escape／确认则按关闭原因恢复。控件卸载后不得恢复到悬空句柄。

输入具有姓名、格式提示、invalid／readonly／disabled 和 expanded 语义；日历有 Grid、Row、ColumnHeader、GridCell，日期包含完整名称、selected、disabled 与 today。日历根不能设置一个字符串 label 而吞掉单元格；自定义内容只替换内部呈现，不移除日期名称和激活行为。

跨月提示只在实际导航时更新适度公告；hover 不播报整张日历，不每次 render 宣告。Windows Narrator 与原生 UIA 桥接实际能力分开验收；不以新增 Windows COM provider 为本组件前置工作。

## 12. 公共 API 草案

以下为 Yue 公开接口形状；它们不是 GPUI Kit 的现成方法。字段私有，类型从 crate 根导出；没有 `pub mod calendar::internal` 消费路径。

```rust
pub enum DatePickerGranularity { Date, Week, Month, Quarter, Year }
pub enum DatePickerSelection { Single, Range, Multiple }
pub enum DatePickerCommitMode { Auto, Immediate, Confirm }
pub enum DatePickerEndpoint { Start, End }
// DatePickerValue / DatePickerUnit 为枚举；DateYear / DateMonth /
// DateQuarter / DateWeek 使用验证构造器及 getter。

// 创建时验证粒度／选择／精度组合；state 由宿主持有 Entity。
YueDatePickerState::new(options, window, cx) -> Result<Self, DatePickerError>;
YueCalendarState::new(options, window, cx) -> Result<Self, DatePickerError>;

// 命令式外部同步；静默、不回声用户事件；失败不改变旧值。
state.set_value(value, window, cx) -> Result<(), DatePickerError>;
state.value() -> &DatePickerValue;
state.set_constraints(constraints, window, cx) -> Result<(), DatePickerError>;

YueDatePicker::new(&state)
    .size(InputSize::Medium)
    .status(InputStatus::Normal)
    .editable(true)
    .clearable(true)
    .readonly(false)
    .disabled(false)
    .bordered(true)
    .placeholder("请选择日期")
    .leading_icon(outline::Calendar)
    .presets(presets)
    .footer(|interactive, window, cx| extra_information(interactive, window, cx))
    .render_day(render);

YueCalendar::new(&calendar)
    .size(InputSize::Medium)
    .render_day(render);
```

options 包含稳定 ID、粒度、选择模式、提交模式、week_start、时间精度、format 与约束，不把影响行为的设置散落到短期 render builder 后与 state 冲突。支持的时间配置直接复用 Base TimePrecision／HourCycle 或窄 re-export，不重定义语义相同的类型。

`render_day` 接受 Base 已接线日期项及只读 cell 信息（date、today、selected、in_range、disabled、preview、focused），返回装饰后的元素。追加价格优先保留默认日期数字；完全自定义时仍保留根语义、事件、命中和焦点。renderer 不承担校验，不捕获业务 Entity 后在绘制过程中 notify。

footer 是额外内容槽，不替换关键确定／取消语义；预设传稳定 ID、label 和惰性 resolver。callback／event 接口复用项目既有事件风格，不同时公开两套等价 on_change 与 subscribe 以制造双回调。

## 13. 目录、分层与 Base 扩展管理

```text
src/date_picker/
  mod.rs          # 组件与公开类型导出
  behavior.rs     # 复用 Root 遍历，承接输入与弹层的 Tab action
  model.rs        # 合法单位／值、配置、约束、错误和事件快照
  state.rs        # 编辑会话、Base 状态组合、订阅、提交与取消
  element.rs     # 字段、日历、Tag、操作组合
  panel.rs       # Popup、时间区、预设与页脚
  tests.rs        # 纯日期／输入、窗口交互与组合回归
```

不为了凑目录预建空文件。Calendar 与 Picker 可共享具名私有适配，但 Popup／Input 等基础设施不能反向依赖日期业务。表单标签、错误绑定继续通过 YueField／YueForm 对外合同接入。

Base 扩展建议按依赖顺序拆分：固定六周及日期边界；活动项和网格导航；可配置反向范围完成／预览；不同单位和多选。每项保留旧默认行为，使用显式选项启用新合同；上游兼容 API 与 Yue 专属配置分清。每项有独立回归、补丁来源和移除条件，不修改 Windows COM 层，不将 Yue 主题 token 放入 Base。

## 14. Showcase 场景

1. 八部结构与日期格状态矩阵：默认、hover、今天、选中、焦点、禁用、补位、范围起中止。
2. 独立 Calendar：周起始切换、年月标题导航、今天、键盘路径。
3. 日期输入：默认、三尺寸、前缀、bordered(false)、格式、不可输入、readonly、disabled、error／warning。
4. 日期范围：两个编辑端、双月／窄宽单月、反向、跨月／跨年、单端锁定、合法／非法输入。
5. 日期时间：分钟／秒精度、时间编辑、此刻、确认与取消、同一天时间范围。
6. 周／月／季／年：单值及范围，跨 ISO 周年、单位规范化。
7. 多选：选中／取消、Tag 删除、+N 查看、上限、Confirm 和 Immediate 对照。
8. 限制与快捷项：过去、周末、min／max、最大跨度、越界预设拒绝，所有入口一致。
9. 受控更新与表单：宿主外部 set_value、不回声、保存、重置、草稿中外部更新。
10. 酒店预订组合：自定义价格、入住／离店、晚数和总价由宿主按半开区间计算；不是 DatePicker 内置业务。

每个示例操作必须改变真实示例状态，不能只弹一条“已点击”。不把截图中的页面框、配置面板或网页装饰变成组件固有外观。

## 15. 测试与验收

优先借鉴已有 Base Calendar、TimeField、GPUI Kit DatePicker 和项目 Select／Popup 的测试结构；测试事件／几何／焦点真实结果，不只调用内部 helper 证明 setter 工作。

| 层级 | 必验合同 |
| --- | --- |
| 日期纯规则 | 闰年／闰日、大小月、周起始、年 1／9999 边界、ISO 周年、单位规范化、固定 42 格 |
| 格式与限制 | 合法／非法日期、格式不匹配、完整输入消耗、空端、min/max、反向范围、跨度、单位／时间禁用、所有入口一致 |
| 会话与事件 | Immediate／Confirm、半范围取消、清除、相同值、Enter＋blur 去重、外部同步不回声、回调重入和卸载 |
| 鼠标与键盘 | 每次激活只提交一次、disabled 可聚焦不提交、完整方向导航、翻月／年、IME Enter、只读不能清除／快捷修改 |
| Popup 与焦点 | 碰撞、窄窗口降级、Tab 离开、点击外部不抢焦点、Escape 只关一层、确认恢复活动端、关闭后无旧命中 |
| 实际布局 | 6×7 高度固定、范围色带无断缝、格位稳定、自定义高 cell、三尺寸、rem 16／20／24、浅深主题、焦点环不裁切 |
| 时间／多选／粒度 | 分段与列共用值、精度截断、禁用时间、Tag 移除、+N 可达、上限、粒度无效组合拒绝 |
| 性能与资源 | 静态停止帧、hover 不重建状态、订阅卸载、不重复分配每格 Entity；有异常先测量，不预加缓存 |
| Windows 实窗 | 用户验收字体、像素、DPI、真实 IME、Narrator 名称与阅读顺序；与无头通过分开报告 |

复测使用注入 Clock 与本地确定数据，不等待真实午夜或依赖网络。整区间验证需有明确复杂度界限，不扫描数千年每一天来判断一个页头按钮是否可用。

## 16. 实施阶段与完成标准

1. **共用基础与单日**：最小 Base 扩展、独立 Calendar、Picker 输入／Popup、固定六行、三尺寸、校验、键盘和无障碍；先完成一条真实可用链路。
2. **范围与限制**：双端输入、共享双月、反向及预览、单端锁定、跨度规则、预设、表单与外部同步回归。
3. **时间与其他选择模式**：组合 TimeField、确认事务、单值／范围日期时间、年月季周及日多选、Tag／折叠与相应测试。
4. **展示与交付**：补齐上述 Showcase、按已有 motion 加必要过渡、资源与布局复测、更新文档状态；格式、Clippy、相关测试、token 同步通过后分别列出自动与真机证据。

每阶段独立可审查，当前已有未提交改动必须保留；没有本轮明确请求不自动提交或推送。修改设计系统来源和 Base 时同时保留来源／补丁记录，不能只交生成快照。

## 17. 当前状态

实现入口：[组件公开 API](../crates/yue-component/src/date_picker/mod.rs)、[类型与规则](../crates/yue-component/src/date_picker/model.rs)、[会话状态](../crates/yue-component/src/date_picker/state.rs)、[字段](../crates/yue-component/src/date_picker/element.rs)、[面板](../crates/yue-component/src/date_picker/panel.rs)、[Showcase](../examples/windows-showcase/src/pages/date_picker.rs)。

已落地的共用链路：独立日历／日期字段、固定 42 格、完整日期导航、月／季／年网格、ISO 周、范围反向完成和预览、双端真实输入、双月及窄窗口退单月、单端锁定、分钟／秒 TimeField、多选与可展开 Tag、Auto／Immediate／Confirm、今天／此刻、惰性预设、静默外部赋值、统一限制、三尺寸、前缀图标、自定义日格与等高 cell、额外页脚。Popup 直接使用既有定位、关闭路由、可中断进出动效与减少动态效果策略。

复用情况：没有启用 GPUI Component feature；Base Calendar 持有唯一候选集合，InputState 持有文本，TimeFieldState 持有时间；YueInput 和范围外框共用 `input::surface_style`。新增 `YueField::date_picker`、`YueForm::require_date_picker` / `rule_date_picker`，表单校验读取已提交类型值。Form 保存由宿主显式调用 `submit`，日期字段内的 Enter 先完成日期输入，不自动保存整个表单。

实际 API 与原草案的收敛：

- `YueCalendarState` 是 `YueDatePickerState` 的公开别名，复用同一会话实现；独立日历与 Picker 仍分别创建 Entity，禁止同一个 Entity 同时挂载两个外观。
- `DatePickerOptions::new(id)` 固定结构配置；`set_value` 静默并使旧续接失效，`set_constraints` 返回配置错误并保留历史已提交值，同时取消旧候选。用户候选和程序赋值始终校验；取消时恢复先前已接受的快照，不能因规则刚改变而丢失历史值。
- 值为 `Empty`、`Single(DatePickerUnit)`、`Range`、`DateTime`、`DateTimeRange`、`Multiple(Vec<NaiveDate>)`。月／季／年／ISO 周均用合法值对象；范围排序，多选去重排序，时间按精度截断。
- 首版格式有明确可逆白名单：日支持 `%Y-%m-%d`、`%Y/%m/%d`、`%d/%m/%Y`；月支持 `%Y-%m`、`%Y/%m`；年 `%Y`；周 `%G-W%V`；季 `%Y-Q%q`；日期时间支持横线或斜杠日期＋24 小时时分／时分秒。TimeField 可配置 H12，编辑字符串仍使用明确的 24 小时格式。
- `footer` 是接收 `(interactive, window, cx)` 的工厂；退出时应禁用自定义交互。`render_day` 装饰已接线的 Base 根；`.cell_height(rems(...))` 给所有日期格设置统一高度，不能只拉高一个价格格。
- 时间首版使用可键盘编辑的 Base 分段字段。没有复制网页的三列时间滚动器，也没有新增日历翻页动画引擎；现有 Popup 过渡承接必要的开关动效。

Showcase 导航新增“日期选择 DatePicker”，含独立日历、三尺寸与输入状态、范围／锁定、日期时间、四种单位及范围、多选两种提交方式、限制和越界预设、外部赋值／重置与表单保存、酒店价格组合。示例的日期值、保存、晚数及事件计数均连接真实状态。

自动验收见 [专项测试](../crates/yue-component/src/date_picker/tests.rs)：23 项覆盖合法日期／格式／ISO 周、固定六周及年 1／9999、键盘与指针、反向范围和禁用、Enter＋blur 去重、IME、确认取消、时间编辑、多选上限／清空、锁定、外部赋值及回调重入、Tab／外部关闭、Form、rem／尺寸及窄窗口。Base 改动及上游移除条件记录在 [YUE-PATCH](../vendor/gpui-base/YUE-PATCH.md)。

2026-11-08 自动复测结果：

| 命令 | 结果 |
| --- | --- |
| `cargo test -p yue-component --locked --features headless-tests --lib --quiet` | 417 项通过，包含全部日期专项 |
| `cargo test -p yue-component --locked --features headless-tests --test input --test field --quiet` | Input 10 项、Field 3 项通过 |
| `cargo test -p windows-showcase --locked --features headless-tests --quiet` | 49 项通过、9 项已有忽略；包含日期页导航 |
| `cargo clippy -p yue-component -p windows-showcase --locked --all-targets --features headless-tests -- -D warnings` | 通过 |
| `node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check` | 来源与生成映射一致 |
| 文档本地链接、代码围栏与 `git diff --check` | 通过 |

Base Calendar 独立测试 9 项通过。该 crate 不属于根 workspace，独立复测必须显式使用根项目的本地 GPUI 补丁，避免错误编译 registry GPUI：

```powershell
cargo test --manifest-path vendor/gpui-base/Cargo.toml --offline --features test-support `
  --config 'patch.crates-io.gpui-pre.path="D:/100_Projects/110_Daily/Yue-GPUI-Component/vendor/gpui-pre"' `
  --config 'patch.crates-io.gpui-pre-windows.path="D:/100_Projects/110_Daily/Yue-GPUI-Component/vendor/gpui-pre-windows"' `
  --config profile.dev.debug=0 calendar --lib --quiet
```

迁移目录时调整上述两个绝对路径。独立构建使用自己的锁文件；常规项目测试仍使用根锁文件和 `--locked`。

示例程序已成功编译为 `target/debug/windows-showcase-date-picker.exe`。常规 `cargo build -p windows-showcase --locked` 在替换现有 exe 时遇到 Windows 文件占用（拒绝访问），因此保留现有程序，改用以下命令输出独立可执行文件；编译器提示输出文件名适配及忽略 `--out-dir`，属于显式 `-o` 的预期诊断：

```powershell
cargo rustc -p windows-showcase --locked --bin windows-showcase -- -o D:/100_Projects/110_Daily/Yue-GPUI-Component/target/debug/windows-showcase-date-picker.exe
```

真机待用户验收：参考外观的细节、文字与价格格密度、真实输入法、Windows Narrator、显示器缩放及弹层方向。无头结果不代替这些检查；未实现时区、UTC 转换或 Windows COM provider。
