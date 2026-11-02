# 23 YueCollapse 折叠面板组件设计

状态：已实现。日期：2026-11-02。Yue / Base API、自然高度动效、源 token 与 Showcase 已接入；自动验证记录见第 14 节。真实 UI、Windows UIA / Narrator 与 IME 实机体验由用户检查。

依据用户提供的 [Collapse 参考图](assets/collapse-reference.png)、仓库中实际使用的 `gpui-base 0.8.0` 源码、gpui-kit Coding Guides / Design Guides，以及本文列出的官方交互资料编写。

关联：[Button](02-button.md)、[Checkbox](06-checkbox.md)、[Dropdown](07-dropdown.md)、[Tooltip](10-tooltip.md)、[Popup](12-popup.md)、[Divider](14-divider.md)、[Tabs](21-tabs.md)、[Loading feedback](22-loading-feedback.md)。

## 1. 定位与参考图取舍

Collapse 将同一页面的内容按标题分组，允许原地展开和收起。默认可以同时展开多项；手风琴模式最多展开一项，允许全部收起。它改变文档流中的内容可见范围，不建立浮层，也不承担路由、异步请求或业务数据缓存。

| 参考图内容 | Yue 首版设计 |
| --- | --- |
| 基础折叠面板、默认展开项 | 受控 `expanded_keys`；初始值由宿主设置，不再维护一份内部 default 值 |
| 手风琴模式 | `CollapseMode::Accordion`，一次请求同时表达关闭旧项和展开新项 |
| 自定义内容、嵌套面板 | 以内容 renderer 组合文本、图片、输入框、列表或下一层 YueCollapse |
| 可配置展开图标及左右位置 | 默认前置 ChevronRight，展开时向下；支持尾侧、隐藏及自定义静态图标 |
| 仅图标响应点击 | `CollapseExpandTrigger::Icon`；标题与右侧命令保持各自职责 |
| 右侧操作区 | 独立命令槽，可放 Button / Checkbox / Dropdown；与展开按钮是兄弟节点 |
| 整组 / 单项禁用 | 禁用折叠触发，不自动关闭内容，不递归禁用业务内容或操作槽 |
| 有边框 / 无边框、显示箭头开关 | `CollapseVariant::Bordered / Borderless`；隐藏箭头不留下占位 |
| 三个尺寸 | Small / Medium / Large，文字、行高、图标与内边距成套变化 |

不复制参考网页的覆盖率徽章、示例代码工具栏、贡献者列表、网页页脚和文档导航。图中像素仅用于理解布局关系，不按截图缩放比例直接推导 token。

与相邻组件的边界：

- Tabs 在同一位置切换同级面板，Collapse 则保留各标题并让展开内容参与纵向排版。
- `Collapsible` 是底层单区域显隐 / 展开机制；`Accordion` 提供折叠面板语义零件；YueCollapse 提供成组合同、外观与动效。
- 普通设置分组若不需要隐藏内容，应使用标题和 Divider，不为了展示能力默认做成折叠面板。
- 折叠面板不内置加载、取消、重试、保存或删除。宿主可以组合现有反馈和命令组件。

## 2. 资料与已核实的 Base 能力

官方资料：

- [TDesign Collapse](https://tdesign.tencent.com/vue-next/components/collapse)：参考图对应的基础、手风琴、图标、右侧操作与外观场景。
- [WAI-ARIA Accordion Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/accordion/)：标题中的独立按钮、expanded 状态、键盘激活和可选的标题间导航。具体原生投影仍以 GPUI / AccessKit 能力为准。
- [Radix Accordion](https://www.radix-ui.com/primitives/docs/components/accordion)：单项 / 多项受控值、可折叠、键盘路径及基于内容尺寸的动效思路；不照搬 DOM / CSS API。

以下保留设计前的源码基线，便于解释为何扩展 Base；实现后的能力列在表后。

| 设计前源码 | 原有能力 | 当时不能直接假定具备的能力 |
| --- | --- | --- |
| [Base Accordion](../vendor/gpui-base/src/accordion.rs) | Group 根、Item、Heading / Trigger / Panel 零件；受控 open、指针请求、expanded、关闭时默认不附加面板 | 没有组级单开 / 多开模型、完整键盘激活与导航、统一焦点修复、完整 disabled 原生投影 |
| `AccordionHeader` | 传入 id 后投影 Heading 和 level | 不传 id 时不投影 Heading；不能漏掉稳定 id 后声称有完整结构语义 |
| `AccordionPanel::keep_mounted(true)` | 保留并渲染内容 | 现实现没有据此隐藏、隔离输入或移除可访问子树；不能直接当安全的关闭保留策略 |
| [Base Collapsible](../vendor/gpui-base/src/collapsible.rs) | `.open(bool)` 控制内容附加；`.reveal(id, progress)` 接入 MotionReveal | reveal 路径始终保留内容；不是通用 inert 边界 |
| [MotionReveal](../vendor/gpui-base/src/motion/reveal.rs) | 按实际宽度测量自然高度、纵向裁切与归一化展开 | 裁切不等于阻止键盘 / action；首次未测量且 progress 大于零时允许 auto 高度，不能直接保证无首帧闪现 |
| [Base motion](../vendor/gpui-base/src/motion.rs) | 稳定通道、可中断过渡、减少动效、过渡帧调度 | 不决定折叠面板的时长、默认样式和退出内容政策 |
| [Base Button](../vendor/gpui-base/src/button.rs) | 指针 / 键盘命令、FocusHandle、禁用、Tab 顺序与原生属性 | 应复用这些机制补齐 AccordionTrigger，不能在 Yue 再复制一套按钮输入分发 |
| [Showcase sidebar](../examples/windows-showcase/src/sidebar.rs) | Button + Collapsible 的已存在组合；每个导航按钮显式处理关闭时的 tab_stop / disabled | 只适用于已知按钮集合，不是任意输入、编辑器、菜单内容的通用安全方案 |

实现已补齐：

- [AccordionController](../vendor/gpui-base/src/accordion/controller.rs)：显式快照与只读请求、按 key 保留焦点身份、单开 / 多开投影、键盘标题导航与焦点修复。
- `AccordionTrigger` 组合 Base Button，保留 standalone bool / ClickEvent 回调；`.in_group(...)` 显式选择组合同。Enter / Space 只使用 Button 的原生按键释放 click，不在 keydown 再提交一次；前置物理 keydown 用于区分 Keyboard 与 Accessibility 来源。
- [MotionDisclosure](../vendor/gpui-base/src/motion/disclosure.rs)：按实际宽度在正常布局中测量不收缩的子项，以像素高度处理首次揭示、反转和动态高度；收起传入 `None`，仅保留几何背景壳。既有 MotionReveal / Collapsible 调用不变。

所有扩展位于仓库的 `vendor/gpui-base`，未修改 Cargo registry；沿用根级 patch。原生 Accessibility 的测试使用 GPUI 的合成 click seam，不等同于已验证真实 Narrator。

## 3. 分层与状态所有权

```text
宿主：展开 key、业务内容 Entity、嵌套展开值、异步任务、右侧命令状态
                       ↓ 受控快照
YueCollapse：RenderOnce，布局、token、图标、命令槽与动效政策
                       ↓
Base Accordion：组模型、展开请求、Trigger 输入、标题导航、焦点与活动面板边界
Base MotionDisclosure / motion：自然尺寸量测、活动内容裁切、几何过渡
                       ↓
GPUI：布局、焦点、输入分发、绘制、AccessKit
```

- 宿主是展开值唯一所有者。组件只发请求；宿主可接受、延迟或拒绝。未接受前不能提前旋转图标、隐藏旧项或展开新项。
- Base 保留按 key 索引的 FocusHandle、最新快照和必要的尺寸 / 交互状态，不保存第二份可自行提交的 expanded_keys。
- Yue 保存极少的呈现状态；不把展开值、输入文本和请求任务藏进动画 Entity。
- 每项 key 使用非空、唯一的 `SharedString`，与标题分开；重命名、插入和重排不改变身份。元素 id 由 `(collapse id, item key, part)` 派生。
- 内容 Entity 由宿主持有。关闭时退出渲染树不等于清空输入数据；重新展开使用同一个 Entity。
- 不新增全局 CollapseManager，不为每个标题创建业务 Entity，不跨组猜测状态。

## 4. 结构与布局

```text
YueCollapse / Base Accordion（Group）
├─ Item(key)
│  ├─ HeaderRow
│  │  ├─ Heading（稳定 id / heading_level）
│  │  │  └─ AccordionTrigger（Header 模式占据剩余可触发区域）
│  │  │     ├─ Indicator（可选，Leading 或 Trailing）
│  │  │     └─ Title（纯文本，可换行）
│  │  └─ Actions（可选，与 Trigger 为兄弟节点）
│  ├─ ContentViewport
│  │  └─ 活动 Panel + 宿主内容；或只用于退出布局的 ClosingShell
│  └─ ItemSeparator（需要时只绘制一次）
└─ 后续 Item
```

### 4.1 标题行

默认 Header 模式下，标题、箭头和 Trigger 内留白属于同一个激活目标；右侧 Actions 不属于这个目标。HeaderRow 本身不注册展开回调，避免在嵌套按钮外再包一个大按钮。

- 主标题使用纯文本 `SharedString`，不接受把 Input / Checkbox / Button 随意塞入 Trigger 的富交互标题。
- 标题默认自然换行，行高随内容增长；不能用固定高度裁掉长标题。宿主需要复杂编辑头时，在 Actions 放命令，在内容区放表单。
- Trigger 占据剩余宽度并设置 `min_w_0`；Actions 保持自然宽度，优先完整显示。窄到无法同时容纳时，宿主把低频命令组合进已有 Dropdown，不由 Collapse 私自隐藏命令。
- 有 Actions 时，Trailing 箭头仍位于 Trigger 的尾侧、Actions 之前；右侧命令不随箭头位置换边。
- Header / Icon 两种触发方式使用同样宽度的图标槽，槽宽取命中区 token、图形在槽内居中；切换方式不让标题横向跳动。Icon 按钮在行内垂直居中，不再额外叠加一份标题纵向内边距。
- 隐藏箭头时移除整个槽及其 gap，不在尾侧补空位，不保留不对称缩进。
- 标题使用默认箭头光标。键盘焦点框围绕真实 Trigger，外层不能通过 overflow 裁掉标题或命令的焦点框。

### 4.2 仅图标触发

Icon 模式下，Heading 内只放独立图标触发按钮，普通标题文本作为 HeaderRow 的相邻节点；可访问标题仍由按钮的完整名称提供。只有图标按钮响应展开，标题、留白和操作槽均不代为触发，不把普通文本节点塞进 Heading 冒充第二个命令。

图标可视尺寸与按钮命中区分别取 token，不能把 16px 图形本身当作整个点击区域。图标按钮的可访问名称来自完整标题，expanded 状态位于该按钮。

`expand_trigger(Icon)` 与 `show_indicator(false)` 是无可用触发目标的非法组合；构造时按现有组件参数校验方式明确报错，不静默退回整行触发。

### 4.3 内容、嵌套与滚动

- 内容只由选中项 renderer 构建；自然高度参与外层文档流，不为每个面板默认建立滚动容器。
- 面板提供一层规范内边距；内部复杂表单、列表可自行布局。表格等需要贴边时用 `.content_padding(false)` 关闭这一层，避免双重内边距。
- 内容前沿默认对齐标题文字前沿；Leading 图标时补偿图标槽，Trailing / 隐藏图标时不保留补偿槽。关闭 content_padding 后由宿主完全负责内容内边距。
- 嵌套 Collapse 只处理最近一组的标题输入；父组不消费内层标题和内容键盘事件。
- 父项关闭时，嵌套子树一并退出输入 / 可访问树；宿主持有的子组展开值和业务 Entity 保留。重开时按实际受控值直接呈现，不能让所有子组重播初次展开动画。
- 文档流足够长时由宿主的既有滚动容器滚动。首版不做自动滚动锁定、虚拟化或任意深度的全展开 / 全收起管理器。

### 4.4 边框与圆角

`Bordered` 为默认：一条外框、项间一条分隔线、组容器使用统一圆角；标题与内容背景轻微区分，不添加阴影。

`Borderless` 去掉外框，保留必要的项间分隔线和清晰的标题层级，不改点击、焦点或展开规则。外观使用独立 enum，不用多个可能矛盾的 bordered / ghost / transparent 开关。

- 相邻项之间只画一条边界，内容的顶部不能与标题底线叠成双线。
- 第一项与最后一项的背景遵守组圆角；最后一项展开时，底部圆角属于内容，收起时属于标题 / 背景壳。
- 普通 rounded 父容器不等于对子背景完成圆角裁切。实现时处理首尾背景的实际轮廓，并单独保留向外的 focus ring。

## 5. 受控值与展开请求

公共类型：`CollapseMode::{Multiple, Accordion}`，默认 Multiple。

`YueCollapse::new(id, expanded_keys)` 接受 key 集合，`.items(...)` 提供显示顺序。没有内部默认展开值；默认展开某项就是宿主初始化时把该 key 放进集合。

值投影规则：

1. 标准集合按当前 items 顺序去重，忽略已经不存在的 key；不通过用户回调偷偷清理宿主数据。
2. Multiple 允许任意项数，空集合表示全部收起。
3. Accordion 最多显示一个 key，空集合合法。若外部临时提供多个有效 key，按 items 顺序投影第一个，不受 `Vec` 的偶然顺序影响，也不反向触发 on_change；文档示例中的宿主应在切换模式时主动规范化值。
4. 禁用不改写外部值。禁用项可以保持展开；外部程序仍可调整它。
5. 项 key 为空或重复是模型错误，构造时明确报告，不能让多个行共享焦点、展开值或动画 id。

`CollapseChangeRequest` 为只读 seam，字段私有，提供：

- `key()`：本次激活项。
- `is_expanded()`：本次希望该项达到的展开状态。
- `expanded_keys()`：根据最新受控快照得到的完整建议集合，按 items 顺序排列。
- `source()`：`CollapseChangeSource::{Pointer, Keyboard, Accessibility}`。

回调：`on_change(Fn(&CollapseChangeRequest, &mut Window, &mut App))`。构造好请求并释放 Base Entity 可变借用后再调用，允许宿主同步删除项目、切换模式或关闭外层视图。

| 操作 | 请求结果 |
| --- | --- |
| Multiple 展开 A | 在当前集合中加入 A，保留其他项 |
| Multiple 收起 A | 仅移除 A |
| Accordion 展开 B | 完整建议集合为 `[B]`，一次请求替换当前展开项 |
| Accordion 收起当前项 | 建议集合为空 |
| 禁用项 / 禁用组直接激活 | 不发请求 |
| 宿主拒绝请求 | 展开状态、内容和图标保持原值；允许实际点击把焦点移到所点 Trigger |
| 外部同步、插入 / 重排 / 删除 | 同步新快照，不冒充用户操作、不发 on_change |

禁用是输入限制，不是“固定展开”。Accordion 中 A 已展开且被禁用时，激活可用的 B 仍可建议以 `[B]` 替换 A；若业务要锁定 A，宿主应拒绝该请求，而不是赋予 disabled 另一层隐藏含义。

一次激活最多发一次请求；手风琴不得先发一次关闭 A、再发一次展开 B，也不得为了等待退出动画延迟提交 B。

## 6. 键盘、焦点与操作区隔离

### 6.1 键盘合同

| 输入位置 / 按键 | 行为 |
| --- | --- |
| 标题 Trigger，Enter / Space | 请求切换本项，阻止 Space 顺带滚动页面；一次按键序列只激活一次 |
| Tab / Shift+Tab | 按自然顺序进入可用标题、独立操作和当前展开内容；不把整组压成 Tabs 式单一 Tab 停靠 |
| 标题 Trigger，Down / Up | 焦点到同组下 / 上一个可用标题；两端循环；不自动展开 |
| 标题 Trigger，Home / End | 同组第一个 / 最后一个可用标题 |
| 内容输入框、Actions、嵌套 Collapse | 原样交给实际控件；外组不劫持方向键、Home / End 或 Space |
| Escape | Collapse 不消费；交给当前 Popup / Dialog 或宿主 |

标题间导航使用 Base 的组模型和稳定 FocusHandle；不使用 Tabs role、selected 或 roving 规则冒充 Disclosure。所有可用标题仍保留正常 Tab 入口。

### 6.2 收起时的焦点修复

受控值已经提交、即将移除内容时才修复焦点，拒绝的请求不执行修复：

1. 当前焦点在将被隐藏的正文中，且没有其他可见命令接管焦点：移到该项可用 Trigger。
2. 点击另一个标题触发手风琴替换时，保留新标题的真实焦点，不把焦点拉回旧项。
3. 该项被删除或 Trigger 已禁用：优先同组下一个可用标题，再前一个；全部不可用时落到组的仅程序化焦点边界，再允许 Tab 自然离开。
4. 焦点本来在面板外、右侧 Actions 或外部窗口时不抢焦点。

内容拥有的菜单 / Popup 不能在锚点卸载后悬空。优先依靠现有 Popup 锚点生命周期清理；实现时若缺少此通知，应补齐明确的锚点失效合同。不能为了收起一项关闭全窗口不相关的 Dialog 或所有浮层。

### 6.3 右侧命令与禁用

- Actions 与 Trigger 为兄弟，不嵌套 Button，不给 HeaderRow 加总点击监听；点击 Button、切换 Checkbox、打开菜单均不改变展开值。
- 动态“禁用该项”复选框必须在禁用后仍能操作，便于恢复；禁用只作用于该项 Trigger。
- 组级 `.disabled(true)` 同样限制折叠操作，不递归禁用任意业务子树。Actions 和正文是否可编辑由宿主对各控件显式设置。
- 标题变灰不意味着右侧按钮也变灰。每个控件依照自己的真实 enabled 状态绘制与投影原生属性。

## 7. 动效规范

默认 `CollapseMotion::Height`；另有 `None`。动效描述空间的展开 / 收拢，不缩放文字、不过度平移内容、不使用弹簧超调，不叠加逐项错峰或每行淡入。

| 场景 | 首版可见表现 |
| --- | --- |
| 首次挂载、宿主默认展开 | 直接处于正确高度和图标角度，不从零重播 |
| 接受展开请求 | 自然内容从顶部纵向揭示；容器 0 → 自然高度，200ms；下方项目随文档流平稳下移 |
| 接受收起请求 | 正文交互子树立即移除；仅保留原高度的背景壳，100ms 收拢到零；下方项目随文档流上移 |
| 默认箭头 | ChevronRight 旋转 90° 指向下方，使用同一开合进度；不独立晚一拍 |
| 自定义非方向图标 | 默认静止，不把 Star / Message 等任意图标旋转成另一种含义 |
| 手风琴 A → B | 同一受控提交、同一时间基准启动 A 的收拢与 B 的展开，不串行等待，不额外做一次位置平移动画 |
| 连续快速开 / 合 | 从当前可见像素高度和图标角度反向，按 Base reversal factor 缩短返回路径；无异步完成回调，不能从旧端点重播 |
| 展开完成后的内容增高 | 自然文档流更新，不重播折叠动效；异步完成不是一次新的用户展开 |
| 宽度 / rem / 主题 / 尺寸改变 | 重新测量；几何失效时直接落位，不沿旧高度 / 旧字体做错误插值 |
| None / 系统减少动效 | 高度和箭头直接到位；保持完全相同的输入、焦点与语义合同 |

### 7.1 退出政策：先安全移除交互，再收拢空间

首版明确区分“正文仍存在”和“正文原来占用的空间仍在变化”。关闭提交后，正文不再参与布局子树、输入分发、Tab、IME、action 和可访问树；ClosingShell 只保留测量高度、背景及必要边界，不保留旧按钮、文字或编辑器。

因此首版关闭不是“整份旧正文继续向上裁切”：正文立即退出，剩余空间用短暂的 100ms 收拢。此取舍保证任意自定义内容的关闭安全，并保持下方布局连续。不能用 opacity=0、缩成 0 高、父级 disabled 或只挡鼠标声称正文已经 inert。

若后续确实要保留正文的退出画面，应先提供经过真实键盘 / 鼠标 / IME / 原生树验证的纯绘制快照或通用活动子树边界，再升级退出呈现；不为首版折叠面板引入一套通用截图缓存系统，也不把这个前置能力当作现有 GPUI API。

### 7.2 自然高度与首帧

- 按最终可用宽度测量正文自然高度，不能硬编码 max-height，也不能先完整显示一帧再缩回零。
- 第一次尚未量测时，先保留零可见高度，量测后请求一次有效帧再开始揭示；默认已展开项可直接完成初始量测并落位。
- 使用新增 MotionDisclosure，不改 MotionReveal 的未知高度兼容分支；动画中由正常布局中的不收缩子项提供自然高度，同一帧仍保持零可见高度后再揭示。
- 动效高度以实际像素为采样值，不能在内容自然高度改变时仅把旧的 normalized progress 乘上新高度，导致跳动。
- 动画中的正文若发生受控更新，重新量测并从当前可见高度接到新目标；几何身份变化按上表直接落位。
- 稳定展开后恢复自然高度和普通裁切边界；关闭完成背景壳高度归零，不再绘制正文。空几何载体保留下一次开合的量测状态，没有交互 / 可访问子树。没有任务时不持续请求帧，不为每项创建循环 Task。
- 视口外或窗口隐藏 / 失活时不为此动效保持连续帧；重新可见时采样当前时间，已结束的过渡直接处于终态。

## 8. 内容生命周期

首版只提供 `.content(Fn(&mut Window, &mut App) -> AnyElement)`；没有 eager children、keep_mounted 或 destroy_on_collapse 的组合开关。

- 初始关闭：不调用 content renderer，不启动宿主业务任务。
- 接受展开：才调用 renderer 并附加活动内容；框架 render 可重复调用，renderer 必须可重入且无网络 / 磁盘等业务副作用。
- 收起或组被卸载：立即不调用 renderer、不附加正文；业务 Entity 只要仍由宿主持有就不会丢失。
- 业务任务在事件 / 模型更新中启动，是否取消由宿主决定；不能把“展开内容”误当作“发起请求”的生命周期回调。
- 没有 content renderer 时允许空面板，接受展开后仍提供正确的标题 expanded 状态；不能制造假的正文或占位文案。

首版不公开“关闭但原样保留可交互内容”的模式。将来如需首访缓存，只能缓存宿主数据或已访问标记；关闭树仍须退出输入和原生树。

## 9. 公共 API

以下 API 已导出，常用的 `YueCollapse` / `YueCollapseItem` 也在 prelude 中。可编译基础示例位于类型的 doctest；完整宿主示例位于 Showcase 的 CollapsePage。

| 类型 | 成员 / 默认值 |
| --- | --- |
| `CollapseMode` | `Multiple`（默认）、`Accordion` |
| `CollapseSize` | `Small / Medium / Large`，默认 Medium |
| `CollapseVariant` | `Bordered`（默认）、`Borderless` |
| `CollapseExpandTrigger` | `Header`（默认）、`Icon` |
| `CollapseIndicatorPosition` | `Leading`（默认）、`Trailing` |
| `CollapseMotion` | `Height`（默认）、`None` |
| `CollapseChangeRequest` | 第 5 节的只读请求 |

`YueCollapse`：

- `new(id, expanded_keys)`、`items(...)`、`on_change(...)`。
- `mode(...)`、`size(...)`、`variant(...)`、`disabled(bool)`。
- `expand_trigger(...)`、`indicator_position(...)`、`show_indicator(bool)`、`motion(...)`。
- `heading_level(usize)`：默认 3，合法范围 1–6；宿主按页面结构指定，嵌套不会擅自无限加深。
- `accessibility_label(...)`：命名整组；`theme(...)` 与 current_theme 继承遵循现有组件。
- 使用 ControlLayout 的 width / min_width / max_width / margin / flex_grow；不暴露任意 Styled 覆盖去破坏语义外观。

`YueCollapseItem`：

- `new(key, title)`：稳定 key 与可读标题分开。
- `disabled(bool)`。
- `indicator(IconDefinition)`：替换为自定义静态图标；未调用时使用默认旋转 Chevron。
- `content(renderer)`、`actions(renderer)`：两者分开，Actions 在收起时仍存在。
- `content_padding(bool)`：默认 true；复杂贴边内容可显式关闭。
- `region(bool)`：默认 false，正文使用普通内容包装；复杂分区需要原生 Region 地标时显式开启，并使用该项标题命名。

不添加通用 header renderer、多个展开值来源、内部默认展开值、每项自动保存、业务加载回调和全局折叠服务。单项场景直接用只有一项的 YueCollapse；首版不再新增一套近似的 YueAccordion / YueDisclosure 别名。

### 9.1 受控多展开示例

```rust,ignore
// self.expanded: Vec<SharedString> 由宿主持有。
YueCollapse::new("settings-sections", self.expanded.clone())
    .accessibility_label("设置分区")
    .items([
        YueCollapseItem::new("basic", "基本设置")
            .content(|_, _| div().child("基本设置内容").into_any_element()),
        YueCollapseItem::new("advanced", "高级设置")
            .content({
                let input = self.path_input.clone();
                move |_, cx| {
                    YueInput::new("advanced-path", current_theme(cx), &input)
                        .into_any_element()
                }
            }),
    ])
    .on_change(cx.listener(|view, request: &CollapseChangeRequest, _, cx| {
        view.expanded = request.expanded_keys().to_vec();
        cx.notify();
    }));
```

默认展开 basic 时，在宿主构造中初始化 `expanded = vec!["basic".into()]`。需要手风琴只增加 `.mode(CollapseMode::Accordion)`；接受请求时仍使用完整建议集合。

### 9.2 独立操作示例

```rust,ignore
// 宿主含 export_section_disabled: bool 与 export_resets: usize。
let owner = cx.entity();
YueCollapseItem::new("export", "导出设置")
    .disabled(self.export_section_disabled)
    .actions(move |_, _| {
        let owner = owner.clone();
        YueButton::new("export-reset", "重置")
            .on_click(move |_, _, cx| {
                owner.update(cx, |view, cx| {
                    view.export_resets += 1;
                    cx.notify();
                });
            })
            .into_any_element()
    })
    .content(|_, _| div().child("导出设置内容").into_any_element());
```

重置命令没有修改 expanded_keys，故不展开 / 收起该项。完整宿主及事件处理已在 CollapsePage 落地，最小组装方式也有可编译 doctest；本文的宿主成员片段不作为独立程序运行。

## 10. Token 与尺寸合同

唯一来源为 `D:/100_Projects/110_Daily/Yue-Design-System/packages/tokens/src`。已新增 `component-tokens/collapse.css` 并登记 `_index.css`，由同步工具生成 Rust / inventory；没有手改 generated.rs。

| Token / 族 | 16px rem 基线下的首版值 |
| --- | --- |
| `--collapse-header-min-height-sm/md/lg` | 32 / 40 / 48px；是最小高度，多行标题可增高 |
| `--collapse-font-size-sm/md/lg` | 12 / 14 / 16px，优先引用 control-font-size 语义 |
| `--collapse-line-height-sm/md/lg` | 16 / 20 / 24px，与上下内边距共同形成 32 / 40 / 48px 的单行标题 |
| `--collapse-header-padding-x-sm/md/lg` | 12 / 16 / 20px |
| `--collapse-header-padding-y-sm/md/lg` | 8 / 10 / 12px，实际行高不小于 min-height |
| `--collapse-indicator-size-sm/md/lg` | 14 / 16 / 18px；优先沿用既有 IconSize 语义和图标 token |
| `--collapse-indicator-hit-size-sm/md/lg` | 24 / 28 / 32px，用于 Icon 模式；与可视图形尺寸分开 |
| `--collapse-header-gap` / `--collapse-actions-gap` | space-8 / space-8 |
| `--collapse-content-padding-sm/md/lg` | 12 / 16 / 20px；Leading 图标文字对齐补偿由布局计算 |
| `--collapse-radius` / `--collapse-border-width` | radius-md / border-1，单条边界 |
| `--collapse-header-background` / `--collapse-content-background` | surface / surface-component；明暗主题保持文字对比 |
| `--collapse-header-background-hover/pressed` | 引用已有中性 hover / pressed 状态语义，不硬编码透明黑 |
| `--collapse-color` / `--collapse-indicator-color` | text-primary / text-secondary |
| `--collapse-border-color` / `--collapse-opacity-disabled` | border-default / 既有禁用透明度语义；仅作用于 Trigger 及对应的纯文本标题，不影响 Actions / 正文 |
| `--collapse-focus-color` / `--collapse-focus-width` | 与现有 Button 相同的焦点语义 |
| `--collapse-expand-duration` / `--collapse-collapse-duration` | motion-duration-enter / motion-duration-exit，当前 200 / 100ms |
| `--collapse-ease` / `--collapse-interaction-duration` | motion-ease-stow / motion-duration-interaction，当前 standard / 100ms |

长度通过 rem 读取，文字、图标、命中区与内边距同步变化。Icon 模式将命中区居中在同一最小行高内，并补偿焦点边框，使切换触发模式时标题文字前沿不跳动。禁用透明度实际引用 `opacity-disabled-content`，不虚构另一份 opacity token。边框使用既有策略；设备像素对齐误差与产品尺寸区分处理。

## 11. 原生可访问性

- Group 有稳定 id 与可选名称；每个 Heading 有稳定 id、合法 level；Heading 内仅包含展开按钮和必要的非交互标题。
- Trigger 使用原生 Button、完整标题作为名称、真实 expanded 和 disabled；键盘及 Accessibility 激活走同一请求协议。
- 内容只在逻辑展开时出现在原生树。默认普通内容包装，不给每个展开项制造 Region 地标；`.region(true)` 用于少量复杂分区，投影命名 Region。原生导航表现仍按实际 GPUI / Windows 映射验证。
- Actions 是独立按钮 / Checkbox / Menu，不塞进标题的可访问名称，也不伪装为展开按钮的子控件。
- 不逐帧宣布高度、百分比或“展开中”；expanded 状态在受控提交时更新。
- 当前 GPUI 未向这些组件提供可直接复用的稳定 AccessKit 节点 ID / controls / labelled-by 关系构造合同；不能用 ElementId 字符串或私有 hash 猜出目标原生 ID。实现须沿用 Tabs 已记录的后端限制，能投影的角色、名称、层级、状态先真实验证，跨节点关系单独记录。

首版不承诺完整实现网页 ARIA 关联。Windows UIA / Narrator 的实际朗读另由用户检查；单元测试写入 Node 和 headless 快照均不能替代真实读屏验证。

## 12. Showcase 方案

已新增 **折叠面板 Collapse**，按英文 id 排在 Checkbox 之后、ColorPicker 之前。使用现有页面布局、Section 与滚动容器，不复制参考网页的装饰层。

1. **基础与默认展开**：4 项，前两项默认展开；包含长文本、图片占位和嵌套组；显示宿主实际 expanded_keys。
2. **手风琴**：4 项，演示全部收起、互斥替换、宿主拒绝和快速 A → B → A。
3. **图标与触发区域**：Leading / Trailing、显示 / 隐藏、Header / Icon、自定义静态图标；Showcase 选择 Icon 时自动保留图标，隐藏图标时切回 Header 并说明原因。组件本身仍对 Icon + 隐藏图标明确报错，不静默改合同。
4. **右侧操作**：每项一个 Checkbox 和一个 Button；Checkbox 禁用展开而仍可恢复，操作按钮统计独立点击次数；可组合 Dropdown 检查展开隔离。
5. **外观与尺寸**：Bordered / Borderless、三个尺寸、整组及单项禁用；禁用已展开项不丢内容。
6. **状态与动效**：在输入框输入后反复收起 / 展开，验证值保留；快速反转、宽度变化、异步内容增高，以及全局减少动效。

每个示例的 key、业务 Entity 和回调计数都由对应页持有；不为演示循环启动假请求。真实观感与图标 / 内容的运动验收由用户完成。

## 13. 实施顺序与验收

实施顺序：

1. 补 Base Trigger 的 Button 输入与原生合同；引入明确的组模型 / 请求协议，完成 key 和模式纯测试。
2. 补自然高度首测与活动内容 / 退出壳边界，完成焦点和输入隔离；不先把任意正文接入现有 keep_mounted / reveal 路径。
3. 在外部设计系统新增 collapse.css 并同步，随后实现 Yue 布局、槽位、三尺寸、两外观和动效。
4. 接入 Showcase、根级导出、README 与 doctest；执行限定范围格式化、目标测试、工作区回归、Clippy 和 token 检查。
5. 如需独立预览，构建新的二进制供用户检查，不覆盖或终止已有预览，不由代理启动真实 UI 验收。

| 测试层 | 必须证明的行为 |
| --- | --- |
| 纯模型 | 多开 / 单开、空值、未知 key、重复 key 校验、按 items 顺序投影、禁用项、模式切换、一次完整请求 |
| Base 原生 / 输入 | Button / Heading / Group / Region、level / expanded / disabled、Enter / Space、一次按键一次请求、Accessibility 激活 |
| headless 指针 / 焦点 | 标题与箭头热区、Actions 不展开、仅图标触发、Tab 顺序、Up / Down / Home / End、嵌套键盘隔离、禁用跳过、焦点修复 |
| 内容生命周期 | 未打开不调用 factory；关闭后按钮不能点击、编辑器不收键盘 / IME / action；宿主 Entity 的输入值可恢复；锚点浮层不悬空 |
| 动效与几何 | 首次展开无完整首帧闪现、初始默认展开不重播、自然高度、反向衔接、手风琴同批响应、关闭背景壳没有活动子树、无固定 max-height 裁切 |
| 动态变更 | 删除 / 重排当前项、内容异步增高、宽度 / rem / 主题改变、快速开合期间宿主拒绝或删除项目、过期完成回调不复活内容 |
| 主题 / 比例 | 明 / 暗、Neutral tone、三尺寸、rem 16 / 20 / 24、显示倍率 1 / 1.5 / 2；标题 / 操作不重叠，边界不叠线，圆角不漏背景 |
| 动效停止 | None、reduce motion、稳定终态、完全裁切、隐藏 / 失活窗口，不保持无意义循环帧 |

## 14. 实现与验证记录

- Yue：[公开 API](../crates/yue-component/src/collapse/mod.rs)、[布局与外观](../crates/yue-component/src/collapse/element.rs)。
- Base：[组模型与焦点](../vendor/gpui-base/src/accordion/controller.rs)、[专用自然高度动效](../vendor/gpui-base/src/motion/disclosure.rs)。
- Showcase：[六组交互示例](../examples/windows-showcase/src/pages/collapse.rs)，包括控制开关、独立 Checkbox / Button / Dropdown、输入保留、嵌套与动态高度。
- 新增 [Collapse headless 回归](../crates/yue-component/tests/collapse.rs)：25 项已通过，覆盖受控拒绝、单次请求、普通 Tab 入口、键盘隔离、焦点修复、关闭卸载、输入值、Popup 锚点、多行标题行高与文字前沿、首帧 / 中间高度 / 反转 / 动态高度、None / reduced motion、裁切 / 隐藏 / 失活停帧，以及 rem 16 / 20 / 24、显示倍率 1 / 1.5 / 2、三尺寸与明暗主题。
- Base Accordion 专项：4 项已通过，包括原生结构角色、expanded / disabled 和可同步重入提交的合成 Accessibility 请求。

最终验证（2026-11-02）：

| 检查 | 结果 |
| --- | --- |
| `cargo test --workspace --locked --features headless-tests` | 605 项通过，0 失败；含 Collapse 25 项、Showcase 导航和 doctest |
| `tools/test-gpui-base.ps1 -Offline -Filter 'accordion::'` | 4 项通过，0 失败；独立于工作区计数 |
| `cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings` | 通过 |
| 本次新增 / 修改实现文件的限定范围 `rustfmt --check` | 通过 |
| `node tools/sync-yue-theme.mjs --check` 与显式 `--design-system ... --check` | 均通过，源 CSS / inventory / generated.rs 一致 |
| 独立预览 `TEMP/collapse-preview/Cargo.toml`，`cargo build ... --target-dir target --offline` | 构建通过 |

预览文件：`target/debug/windows-showcase-collapse-preview.exe`，进入 **组件 → 折叠面板 Collapse**。只构建，未启动应用、截图或进行真实 UI 验收，也未终止既有预览进程。自动测试不代替用户的真实界面 / 读屏 / IME 检查。
