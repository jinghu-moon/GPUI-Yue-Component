# YueCard 卡片组件设计

- 状态：V1 已实现，已接入 Windows Showcase；自动化结果与真机待验项见[实施记录](research/2026-11-09-card-implementation.md)。
- 日期：2026-11-09。
- 范围：Windows GPUI 应用。基于优化后的 Card HTML DEMO、用户提供的 Arco Card 与 TDesign Card 两张参考图。
- 原则：Card 负责相关内容的表面和区域布局，优先组合已有 Yue 组件与 Base 行为；宿主拥有数据、选择、命令、异步任务与滚动。

## 1. 用途与核心决策

卡片将一个对象的标题、摘要、媒体与操作组织成独立的信息单元，适用于文件、产品、工单、成员和统计摘要。默认是静态内容容器，无 Tab 停靠、无点击命令、无固定宽高。

1. 提供 `YueCard`、`YueCardHeader`、`YueCardMeta`、`YueCardCover`、`YueCardFooter` 五个值式部件。按区域职责拆分，不为统计、人物、图片和页签各建一种 Card。
2. 头部、封面、正文、页脚均可省略；缺省区域不占空间。支持只有正文、只有头部和封面＋页脚等组合。
3. 四种表面使用一个 `CardVariant`；三档尺寸同步调整间距和排版。点击性与悬停动效分开配置。
4. 卡片不内建选择组、加载状态机、Tabs、Collapse、网格或滚动条；这些能力通过现有组件组合。
5. 整卡主操作采用显式、真实的 Button 和有限的指针命中扩展。卡片容器不成为包含其他按钮的“大按钮”。
6. 先完成静态布局与组合，再验证整卡操作。覆盖式加载先作为基础能力缺口记录，不把 HTML 的 DOM 能力假定为 GPUI 现有能力。

适用边界：设置条目优先使用 [YueList](29-list.md)，窗口和工作区分栏使用 [YueResizable](40-resizable.md)，同一页面的大区块优先用背景与分隔线。无需给每个区块套卡片，也不以卡片嵌套卡片制造层次。

## 2. 三份参考的吸收方式

参考快照随文档保留，避免依赖桌面截图和被 Git 忽略的 `refer/`：

- [Card HTML DEMO](prototypes/card.html)：本次优化后的原型快照，原文件为 `refer/gpui-card-demo.html`。
- [Arco Card 参考图](assets/card-arco-reference.png)：用户 Image #1。
- [TDesign Card 参考图](assets/card-tdesign-reference.png)：用户 Image #2。

| 来源／能力 | Yue 采用方式 |
| --- | --- |
| Arco 基础、无边框与简洁卡片 | 描边为默认；无边框与填充表面用于有背景区分的区域；允许仅正文或仅头部。 |
| Arco 自定义悬停、尺寸 | 统一 `hover_effect` 与三档 `size`，不照搬 CSS selector／style 字符串。 |
| Arco `Card.Meta`、封面和底部 actions | 用 Meta 组合头像、标题和描述；封面单独约束；Footer 接收真实 Button 或其他内容。 |
| Arco 带 Tabs 的卡片 | 将 `YueTabs` 放入正文，不增加 Card 的 `activeTab`、`tabList` 或页签状态。 |
| Arco `Card.Grid` | 集合布局复用 `ListGrid`，不新建 CardGrid／CardGroup。 |
| TDesign header / footer 插槽 | 标准头部提供便捷字段，也允许整块自定义；页脚支持操作、头像组、描述和混合布局。 |
| TDesign 封面在头部前后、头尾同时组合 | 提供明确的封面位置；省略任一区域后重新按真实顺序安排间距与圆角。 |
| TDesign 不同内容与长文本 | 默认自然换行；截断显式开启；不用固定高度假设标题、描述或正文只有一行。 |
| DEMO 四种表面、三档密度 | 纳入基础合同；保留中性配色与一致的边界，不复制原型的临时色值。 |
| DEMO 顶部／左右／背景媒体 | 顶部、头部后、左侧、右侧纳入 Cover；背景图＋遮罩作为应用组合，不内建图片上的文字对比度算法。 |
| DEMO 整卡点击、单选／多选 | 主操作复用 Button；选择复用 Checkbox／RadioGroup。文字拖选与独立动作不能触发主操作。 |
| DEMO 保留节点的骨架覆盖 | 作为增强目标；现有 GPUI LoadingRegion 是分支替换，必须区别说明和验收。 |
| DEMO 生产看板 | 作为组合验收场景；工单、产线、统计、图表和排产不进入 Card 公共 API。 |

参考站点外部的代码复制、在线编辑、下载、贡献者和配置面板属于文档工具，不属于组件能力。

## 3. 现有能力核对与复用

核对基于本仓库锁定的 `gpui-kit 0.8.0`、受维护的 Base 与 Yue 源码；网上同名 API 不能替代本地签名。当前 Base 没有通用 Card 原语；`HoverCard` 是悬停浮层，与本组件不同。

| 已有能力 | Card 的组合方式／限制 |
| --- | --- |
| GPUI `div`、flex、原生 grid、图片与绘制 | 负责表面、区域排列、换行、同行拉伸和局部裁剪。普通卡片不需要自定义布局引擎或 canvas。 |
| `YueListItemMeta` | 沿用“标题＋描述”的语义经验；Card Header 与 Meta 共用文本布局，截断直接复用 YueOverflowTooltip，不复制文本测量。 |
| `YueAvatar`／`YueAvatarGroup` | 放入 Header／Meta／Footer；图片回退、重叠和数量上限继续由 Avatar 家族负责。 |
| `YueButton`、Base `Button` | 负责命令、键盘激活、焦点、禁用和语义；整卡静态区域共享真实 Button 的回调与焦点，独立按钮保留自身行为。 |
| `YueDropdownMenu`、`YueTooltip` | 更多操作与图标按钮说明。菜单关闭及焦点返回复用现有浮层机制。 |
| `YueCheckbox`、`YueRadio`／`YueRadioGroup` | 宿主持有唯一选择值；Card 只读取同一个值呈现 `selected`，不添加选择状态 Entity。 |
| `YueTag`／`YueBadge` | 标签、状态与计数通过插槽组合；不加 Card 状态色枚举和通知计数 API。 |
| `YueSkeleton`、`YueLoadingRegion` | 首次加载的显式占位与分支替换。后者只构建当前分支，不能保证内容元素树一直挂载。 |
| `YueSpinner`、Button loading | 已有内容的后台刷新优先在操作处反馈，避免卸载输入和页签。 |
| `YueTabs`、`YueCollapse` | 放入正文，沿用其状态、键盘与动效。当前 YueTabs 切出面板会离开输入树，业务 Entity 应由宿主保留。 |
| `YueEmpty` | 在已确认无数据时组合到正文；加载、失败与空数据分别处理。 |
| `ListGrid`、`YueList`、`YueVirtualListGrid` | 小集合使用原生网格；大量卡片复用现有虚拟网格和测量失效机制。 |
| `YueScrollbar` | 卡片通常自然增高；确需局部滚动时由明确的内容视口拥有，复用已有滚动条。 |
| `YueTheme`、`tokens`、`typography`、`motion`、`ControlLayout` | 复用主题解析、相对尺寸、字体、动效与外部布局，不新建 Card 主题。 |

关键源码：[List](../crates/yue-component/src/list/mod.rs)、[网格几何](../crates/yue-component/src/list/grid.rs)、[Loading](../crates/yue-component/src/loading/mod.rs)、[Base LoadingRegion](../vendor/gpui-base/src/loading.rs)、[Button](../crates/yue-component/src/button/mod.rs)、[Tabs](../crates/yue-component/src/tabs/mod.rs)。

不为展示卡片新增 Base 补丁，不启用 Kit 的另一套 styled component／图标资源，不增加 Cargo 依赖。若整卡行为或覆盖加载确有通用缺口，另以最小基础层扩展和回归证据评审，不能将补丁隐含在 Card 外观实现中。

## 4. 部件、布局与省略规则

### 4.1 结构合同

| 部件 | 内容 | 布局责任 |
| --- | --- | --- |
| Card | 区域容器 | 表面、边界、圆角、局部密度、外部尺寸。 |
| Header | leading、标题、描述、extra | 标题区域可缩，额外操作独立，不挤压操作命中框。 |
| Cover | 图片或任意媒体元素 | 媒体框尺寸、位置及独立圆角裁剪；不拥有网络请求与图片预览。 |
| Body | 任意子元素 | 默认内边距；内容自然决定高度，可显式取消 inset。 |
| Meta | leading、标题、描述 | 轻量信息行；可位于正文或页脚，不限定为用户身份。 |
| Footer | 任意内容／操作 | 共享横向 inset，独立分隔线；父布局拉伸时贴底。 |

默认顺序为 `Cover(BeforeHeader) → Header → Body → Footer`；`AfterHeader` 则为 `Header → Cover → Body → Footer`。左右封面位于内容列两侧，内容列仍为 Header／Body／Footer。

没有 Header 时正文从卡片顶部开始，没有 Footer 时正文底部使用完整 inset。只有 Header 时同时补齐底部 inset；只有 Cover 时四个角都跟随表面。不因缺少正文生成空白 Body。调用方传入的空白自定义元素无法由 Card 猜测是否“无内容”，应省略对应 builder。

分隔线只由一个边界拥有。Header 分隔线默认关闭，Footer 分隔线默认开启；缺少相邻区域时不画多余线。无边框表面不等于强制删除内部区域分隔线。

### 4.2 标题、描述与额外操作

- Header 使用顶部对齐的横向结构；leading 和 extra 保留自身尺寸，标题／描述列使用 `flex_1 + min_w_0`。
- 默认标题和描述自然换行，标题与正文处理长中文、URL 和连续标识符；不能撑出卡片。
- `truncate_title(true)` 仅对标准字符串标题启用单行省略，并复用真实溢出判断与 Tooltip；描述继续自然换行。完整标题保留为可访问名称。
- extra 放入按钮、标签或菜单；窄宽度下可转到下一行，不能遮住标题或把按钮压成零宽。超多命令由宿主组合更多菜单。
- 自定义 Header 替换标准 Header 的内容布局，仍保留区域 inset 与边界策略；不遍历后代猜测标题、按钮或头像。

### 4.3 封面

`YueCardCover::new(content)` 接受实现 `IntoElement + Styled + 'static` 的 GPUI 图片、SVG 或应用媒体根元素。组件在类型擦除前给媒体根设置四角；自定义图表可以用可设置样式的 div 包装。图片加载与错误回退复用 GPUI `ImageSource`／Img；调用方提供替代内容。`YueImagePreview` 只用于点击后打开的查看器。

- 垂直封面默认 16:9；可设正且有限的 `aspect_ratio` 或显式 `height`，两者后设者生效，避免矛盾约束。
- 左右封面使用 `side_width`，默认由 token 指定为卡片宽度的 3/8。侧封面的自身宽高比不强制拉高整列，至少保留可读的媒体高度。
- 在受限宽度下不会悄悄裁掉正文或换掉阅读顺序；宿主根据实际容器宽度改为顶部封面。组件不假定窗口宽度等于卡片宽度。
- 图片的 fit 由图片元素负责；Cover 不强行覆盖任意自定义媒体的绘制方式。
- 媒体框独立进行矩形溢出裁剪。GPUI 的 `overflow_hidden` 不提供通用后代圆角蒙版，因此可见圆角直接设置在媒体根；嵌套背景、Img fallback 或自定义绘制须由调用方按相同形状处理。根容器不裁剪，保留按钮、Checkbox 和外侧 focus ring。
- 真正贴住外框的 Cover 角使用 `max(0, 外圆角 − 边框厚度)`；中间封面不带外角。自定义 Header／Footer 内的背景由调用方负责圆角。

背景图片＋文字遮罩由宿主组合为 Cover 或正文内容，必须自行提供对比度与图片失败回退。首版不提供 `Background` 封面变体。

### 4.4 正文与页脚

Body 默认 inset；`body_inset(false)` 用于自身已经拥有 inset 的 Tabs、List 或媒体，避免双重内边距。此配置只作用 Body，不改变 Header／Footer。

Footer 默认自然换行，操作独立成组；允许头像组在左、操作在右，或上方描述＋下方按钮。额外空间由正文侧的弹性区域吸收，Footer 使用布局贴底，不测量后再写入固定偏移。

等分操作是 Footer 的布局策略，不是新的按钮：`actions(...)` 让各操作单元等分空间，单元内使用现有 Button；窄时换行，操作单元有 token 指定的最小宽度。没有 Footer 或 `actions([])` 不创建空操作栏。

## 5. 表面、密度与动效

### 5.1 表面

| `CardVariant` | 背景／边界 | 场景 |
| --- | --- | --- |
| `Outlined`（默认） | 常规表面＋细边界，无静止阴影 | 大多数信息卡片。 |
| `Elevated` | 常规表面＋轻量阴影，边界弱化 | 确有浮层关系的局部表面，不能取代 Dialog／Popup。 |
| `Filled` | 次级填充背景，弱边界 | 汇总、分组或低强调信息。 |
| `Borderless` | 常规表面、无可见外边界与阴影 | 已有父背景承托的内容。 |

不再额外提供 `bordered`、`shadow`、`filled` 等互相冲突的布尔值。四种表面使用同样的几何边框占位，选中和悬停变化不改变尺寸。

### 5.2 三档密度

以下是 16px rem 基准下的设计目标，不是在 Rust 调用点写死的像素值。最终数值从设计系统 Card tokens 同步；原型的 13px 辅助文案不直接成为新的字号阶梯。

| `CardSize` | 区域 inset | 相关区域间距 | 标题／正文／辅助文本 |
| --- | --- | --- | --- |
| `Small` | 12px | 8px | 14／12／12px |
| `Medium`（默认） | 16px | 12px | 16／14／12px |
| `Large` | 24px | 16px | 18／16／14px |

标题权重、正文行高与辅助色来自既有排版角色。Header、Body、Footer 共享横向对齐线。Card size 影响卡片本身的排版与间距，不擅自改变宿主传入 Button／Avatar／Input 的 size；示例应显式组合相配的尺寸。

### 5.3 悬停与选择呈现

`CardHoverEffect::{None, Outline, Lift}` 默认 `None`。`Outline` 加强边界；`Lift` 在边界反馈基础上轻微上浮并加强阴影，位移目标为约 2px 的相对尺寸。三者均不自动增加点击、手形光标或 Tab stop。

整卡操作开启且未显式设置 effect 时，使用 `Outline`。普通静态卡片没有按下反馈；主操作的按下反馈跟随 Button。禁用主操作时停止该操作的 hover／pressed 反馈，不改变其他可用按钮的状态。

`selected(true)` 只呈现受控选择：全周边界／填充及已有选择控件的状态共同表达，不用左侧色条。Card 不额外设置重复的 checkbox／radio 角色。选择控件是语义与值的唯一来源。

颜色、阴影和位移复用共享 Motion；不为常驻卡片添加循环动画。减少动效时取消上浮位移，边界／阴影即时切换。加载动画沿用 Skeleton／Spinner 自己的策略。

## 6. 公共 API

以下为已实现的签名摘要；公共类型字段私有，从 crate 根导出。普通外部布局复用 `ControlLayout` 的 `width / min_width / max_width / margin / flex_grow`；可编译示例见 YueCard 的 Rustdoc。

```rust,ignore
pub enum CardVariant { Outlined, Elevated, Filled, Borderless }
pub enum CardSize { Small, Medium, Large }
pub enum CardHoverEffect { None, Outline, Lift }
pub enum CardCoverPlacement { BeforeHeader, AfterHeader, Left, Right }

impl YueCard {
    pub fn new(id: impl Into<ElementId>) -> Self;
    pub fn theme(self, theme: YueTheme) -> Self;
    pub fn size(self, size: CardSize) -> Self;
    pub fn variant(self, variant: CardVariant) -> Self;
    pub fn hover_effect(self, effect: CardHoverEffect) -> Self;
    pub fn selected(self, selected: bool) -> Self; // 仅呈现
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self;
    pub fn header(self, header: YueCardHeader) -> Self;
    pub fn cover(self, cover: YueCardCover) -> Self;
    pub fn body_inset(self, inset: bool) -> Self;
    pub fn child(self, content: impl IntoElement) -> Self;
    pub fn children(self, content: impl IntoIterator<Item = impl IntoElement>) -> Self;
    pub fn footer(self, footer: YueCardFooter) -> Self;
    pub fn primary_action(self, action: YueButton) -> Self;
}

impl YueCardHeader {
    pub fn new(title: impl Into<SharedString>) -> Self;
    pub fn custom(content: impl IntoElement) -> Self;
    pub fn description(self, text: impl Into<SharedString>) -> Self;
    pub fn leading(self, content: impl IntoElement) -> Self;
    pub fn extra(self, content: impl IntoElement) -> Self;
    pub fn truncate_title(self, truncate: bool) -> Self;
    pub fn divided(self, divided: bool) -> Self;
}

impl YueCardCover {
    pub fn new(content: impl IntoElement + Styled + 'static) -> Self;
    pub fn placement(self, placement: CardCoverPlacement) -> Self;
    pub fn aspect_ratio(self, ratio: f32) -> Self;
    pub fn height(self, height: impl Into<AbsoluteLength>) -> Self;
    pub fn side_width(self, width: impl Into<DefiniteLength>) -> Self;
}

impl YueCardMeta {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self;
    pub fn description(self, text: impl Into<SharedString>) -> Self;
    pub fn leading(self, content: impl IntoElement) -> Self;
    pub fn truncate_title(self, truncate: bool) -> Self;
    pub fn theme(self, theme: YueTheme) -> Self;
    pub fn size(self, size: CardSize) -> Self;
}

impl YueCardFooter {
    pub fn new(content: impl IntoElement) -> Self;
    pub fn actions(actions: impl IntoIterator<Item = YueButton>) -> Self;
    pub fn divided(self, divided: bool) -> Self;
    pub fn inset(self, inset: bool) -> Self;
}
```

构造默认：继承应用主题、Medium、Outlined、无悬停动效、未选中、Body inset 开启、无主操作、无可访问名称。Header 默认不分隔；Footer 默认分隔并 inset。Cover 默认为 BeforeHeader／16:9。Header／Cover／Footer 非独立业务视图，不需要单独的公开 ID；其子节点由 Card ID 派生，插槽内的控件仍使用宿主提供的稳定 ID。

Header／Footer 可接收普通自定义区域，但标准 builder 与 `custom` 不混合；自定义头部不接受标准字段追加，避免静默丢弃字段。Header／Cover／Footer 的区域样式由 Card 解析；`YueCardMeta` 是独立值式元素，默认继承应用主题和 Medium，放进任意内容槽后仍由调用方显式配置与 Card 相配的 size／theme。不通过遍历类型擦除后的子元素自动注入配置，也不通过临时全局主题污染后代。

Meta 需要独立且稳定的 ID，以隔离标题溢出 Tooltip 的 keyed state；因此实现签名是 `new(id, title)`，不能只用标题充当身份。

`primary_action` 是可见、带名称的主 Button，默认位于 Footer 操作组；没有 Footer 时生成仅包含该按钮的页脚。Button 的事件、disabled、loading 和焦点保持原合同。自定义 Footer 必须为主按钮预留由 Card 安排的独立操作位置，不能同时把同一个按钮对象放进两个槽。Card 不再提供第二个 `on_click` 回调。

不提供 `YueCardState`、`on_selected_change`、`loading(bool)`、`active_tab`、`columns`、`scrollable` 或业务 `status`。它们由原有组件与宿主分别拥有。首版不开放任意 `className`、CSS 字符串、内边距像素覆盖或选择状态专属主题。

## 7. 输入、焦点与状态所有权

### 7.1 整卡主操作

普通卡片不拦截鼠标、键盘或应用 Action。显式 `primary_action` 在保留可见 Button 的前提下，将卡片内安全的静态区域点击作为同一命令的指针便捷入口；键盘始终由真实 Button 接收 Enter／Space，不额外建立卡片 Tab stop。

安全区域限定为：标准标题／描述列、非交互正文、非交互封面及这些区域内部的空白。Header 外侧 padding、leading／extra、自定义 Header、Footer 的其他内容不接线。任意正文或封面含 Input、Link、Tabs、Collapse、Scrollbar、拖动区域等交互时，不启用整卡扩展，只保留可见主按钮；组件不遍历任意后代猜测控件类型。

实现需要把一份命令回调连接到真实 Button 和安全区域命中，不能通过模拟点击、另写键盘处理或全局吞事件实现。共享接线若需调整 Button，仅增加 crate 内明确的组合入口，不暴露其私有状态。

行为要求：

- 同一次用户输入只触发一次命令；Button 禁用／loading 时指针扩展同步关闭。
- 拖选文本、已选文本、超过拖动阈值、拖拽媒体或右键均不执行主操作；不覆盖文本选区，不让透明点击层挡住正文。
- 子按钮、菜单、Checkbox 与其他独立控件保留自己的事件和焦点；只在确有冲突的边界局部隔离，不全局 `stop_propagation`。
- 打开 Dialog／Popup 后沿用其焦点规则；关闭返回同一个有效主 Button，Card 不抢回焦点。
- Root 保留内容分组语义，Button 独立暴露；不把整个含后代控件的子树改成 Button，也不重复声明激活操作。

原生无头测试已验证共享命令、键盘激活、禁用／加载、独立子动作及真实文字选区。GPUI 事件边界只在显式安全区域接线，根节点没有父级点击转发；Windows 真机验收仍需执行。

### 7.2 单选、多选与禁用

首版选择示例把实际 Checkbox 或 Radio 放入 Header leading／Body，Card 的 `selected` 读取宿主的同一值。多选用 Space，单选组的方向键与互斥使用既有 RadioGroup；Card 不写另一套方向键或二维选择算法。

Checkbox 的可访问名称应包含对象名。禁用由实际选择控件和相关业务 Button 分别控制；卡片文字仍可阅读。Card 不提供看似能递归禁用任意子元素的 `disabled(bool)`。

首版不扩大选择控件的命中到整张卡片，不增加选择卡片专用公共类型。DEMO 的整卡选择仅作为后续命中扩展参考；应先确认真实设置／选择场景确有需要，再复用选择控件的 label／受控行为接线。整卡主命令与整卡选择不得同时竞争同一片命中区域。

### 7.3 身份与状态

所有 Card、主 Button、Avatar、Tabs 和集合条目使用稳定领域 ID；排序／过滤后不得以数组下标重建身份。值式 Card 只保留必要的 keyed hover／动效几何，不创建每卡 Entity。

宿主持有输入 Entity、页签选择、勾选值、数据和异步任务。外部状态更新不回送用户回调；一次交互由原 owner 更新后 notify。切主题、加载或调整尺寸不能触发业务命令。

## 8. 加载、空态与失败

### 8.1 首版复用策略

| 状态 | 推荐组合 | 必须保证 |
| --- | --- | --- |
| 首次加载，无内容 | `YueLoadingRegion`＋按预期区域显式构建的 `YueSkeleton` | 预留封面比例与近似文本／操作高度；加载状态可被读屏识别。 |
| 已有内容，后台刷新 | 保留现有 Card，刷新 Button loading 或附近 Spinner | 不卸载用户输入和已选内容；是否允许继续操作由宿主决定。 |
| 必须暂时替换内容 | LoadingRegion 的 placeholder／content 两分支共享外部尺寸约束 | 输入 Entity 保留在宿主；切出前处理焦点，切回不自动抢焦点。 |
| 无数据 | Body 组合 `YueEmpty` | 仅请求已完成且确实为空时展示。 |
| 失败或无权限 | Body 显示明确说明和实际恢复 Button | 不伪装成空态，不把重试任务或权限判断放进 Card。 |

不承诺“任意自定义内容切换 loading 都绝对等高”。有限数据可以共享同一 Cover／Header／Footer 布局定义；未知高度内容由宿主提供合适的占位约束。不能读取 HTML DOM 的结果后在 GPUI 写死同样高度。

### 8.2 覆盖式加载的增强边界

原型通过 `inert`、`aria-hidden` 和保留 DOM 实现覆盖，当前 `YueLoadingRegion` 没有这一模式。若后续要达到同等效果，应由 Loading 基础层提供统一能力，Card 只消费它：

1. 原内容保持布局／业务 Entity 身份；可见骨架按已声明语义区域或当前有效布局定位，不递归推断任意子元素。
2. 加载期间原内容必须同时退出指针命中、键盘焦点、快捷键作用域及可访问操作树；一层透明遮罩不能满足此要求。
3. 从正在编辑的内容进入加载，需先关闭依赖触发器的子浮层并把焦点移到宿主指定的稳定位置；完成后仅在合适条件下恢复原有效目标。
4. 宽度、内容版本、rem、主题排版变化重新测量；缓存不能只按 Card ID 固定。动效期间仍不得接受原内容输入。
5. 快速开关、取消、删除卡片和窗口关闭不留下任务、焦点或持续帧；用户勾选、输入值和宿主页签状态不被重置。

这一增强不作为基础 Card 的发布前置，本次实现不修改 Base。需要独立设计、测试和补丁维护说明。

## 9. 卡片集合、等高与滚动

Card 默认宽度服从父布局，高度随内容。首版不增加固定高度参数；宿主确需有限高度时，应组合一个有明确约束的外层或正文视口，并说明超出内容如何处理。

小集合使用现有 `YueList::grid(ListGrid::min_width(...))` 或固定列数配置。`ListGrid::resolve` 基于实际容器宽度、gap 与当前 rem 计算列数，少于一列时按现有规则保留一列；Card 不读取屏幕分辨率，不新建断点系统。

- 父 grid 负责同行 stretch；Card 及中间的 ListItem 包装层必须把行高传递到表面，不能只有外层空白等高。
- Header／Cover／Footer 不被压扁，Body 接受额外空间；同行 Footer 底边对齐，不同行保留自然高度。
- ListItem 是集合语义外壳，使用 `YueListItem::inset(false).content(card)` 去掉壳的 padding／gap，并通过中间层传递同行高度。外壳默认 inset 保持开启；Card 集合不要同时打开 ListItem 的选中背景、条纹或 hover。
- `ListGrid` 自适应和 Footer 对齐须经过真实 GPUI 布局断言；HTML grid 通过不代表这条原生组合链路已通过。
- 大集合使用 `YueVirtualListGrid`／既有虚拟滚动基础；宽度、字号、图片框和文本变化按现有合同使行测量失效。不为 Card 新增 overscan、masonry 或滚动状态。
- 默认不建立卡片内部滚动。确需长正文滚动时，有限高度、`min_h_0`、ScrollHandle 和 YueScrollbar 属于同一个正文视口；滚动条贴住该视口边缘，正文 inset 放在视口内部。

## 10. Token、主题与可访问性

设计系统已新增并同步 34 个 `--card-*` tokens。以下为主要家族，完整列表以生成 inventory 为准。

| Card token 家族 | 值的来源／用途 |
| --- | --- |
| `--card-background{,-filled}` | 现有常规／次级 surface 角色。 |
| `--card-border-color{,-hover,-selected}` | border 与 accent 语义角色；不采用业务状态色。 |
| `--card-border-width / --card-radius` | 现有线宽、radius 阶梯；改变颜色不改变占位。 |
| `--card-padding-{sm,md,lg} / --card-gap-{sm,md,lg}` | 既有 12／16／24 和 8／12／16 间距阶梯。 |
| `--card-title-font-size-* / --card-body-font-size-* / --card-description-font-size-*` | 既有字号与行高角色，见密度表。 |
| `--card-title-color / --card-description-color` | text primary／secondary。 |
| `--card-shadow / --card-shadow-hover` | 既有轻量 box shadow 角色，不重写解析器。 |
| `--card-offset-hover / --card-cover-side-ratio` | 有限的上浮距离与侧封面默认比例。 |
| focus 与 motion | 直接复用全局 focus ring、MotionDuration／MotionEase，除非有明确调参需求，不再复制别名。 |

设计系统为唯一源，通过 [同步工具](../tools/sync-yue-theme.mjs) 生成 inventory 与 Rust 映射。此次修改相邻 `Yue-Design-System` 仓库的 Card token、入口、命名注册和专项测试；当前仓库保留生成快照，构建不依赖相邻仓库。不要手改生成代码或在渲染中添加临时色值 fallback。

Card 主题覆盖只影响 Card 自己绘制的表面与标准部件；不自动重写任意用户子元素的主题。混用局部 Card theme 和显式 Button theme 是宿主的组合责任。

可访问性合同：静态 Card 无额外 Tab stop；有名称时按内容分组暴露，不假装是交互 widget。主要内容保持阅读顺序，标题、描述、按钮名称来自真实对象。选择由真实 Checkbox／Radio 表达，图标操作提供名称与 Tooltip。焦点可见且不被 Cover 裁剪；selected、disabled、失败不能仅靠颜色区分。

Windows Narrator、系统高对比度、GPU 像素及 DPI 需要真实窗口验收；AccessKit 树断言只证明桥接前语义。不新增 COM provider，不以 Card 需求扩展跨平台适配。当前设备可验收当前屏幕与应用 rem 缩放，多显示器／跨 DPI 移动明确列为环境受限项。

## 11. 示例页面

Showcase 增加“卡片 Card”，按现有导航规则排序。使用真实可操作的基础组件，避免只有外观、按钮无结果。

| 示例 | 内容与验收目的 |
| --- | --- |
| 基础与简洁 | 标题＋正文＋更多；仅正文；仅 Header；省略区域无空白。 |
| 表面与密度 | 四种 variant、三档 size；灰底展示 Borderless，浅深／Azure／Neutral 主题一致。 |
| Header 与 Meta | 头像、长中文标题、描述、状态与更多；比较自然换行和真实溢出 Tooltip。 |
| 封面 | 头部前／后、左／右；封面在最外角与中间；图片失败 fallback。 |
| Footer | 纯操作、等分操作、头像组＋说明、独立按钮；操作结果可见。 |
| 内容组合 | YueTabs、YueCollapse、YueList、统计文本；不创建 Card 内部业务状态。 |
| 选择 | 实际 Checkbox 多选、RadioGroup 单选；禁用控件；Card selected 与宿主值同步。 |
| 加载与异常 | 首次骨架、已有数据刷新、空数据与失败重试；输入 Entity 不在 render 中重建。 |
| 响应式集合 | 调整真实容器宽度，观察 1／2／3 列、同行等高与 Footer 对齐；提供虚拟卡片集合组合。 |
| 整卡操作（行为阶段） | 静态内容扩展点击、文字拖选、独立收藏与更多；复杂交互正文只用明确 Button。 |
| 生产看板 | 用少量统计、产线 Tabs、异常 List 和工单 Card 检查组合，保留一个主任务与明确阅读顺序。 |

## 12. 测试与验收

测试验证用户可观察合同，不写只检查 builder 字段是否存入的镜像测试。HTML 原型通过的测试只作为案例来源，不能抵充 Rust 组件验收。

| 层／场景 | 必须证明的结果 |
| --- | --- |
| 原生布局 | 全部区域省略组合、Header-only、无 Body Footer、封面前后左右；区域不留下幽灵间距或重复线。 |
| 缩放与长内容 | 16／20／24px rem、受限容器、长中文／URL；文字不撑出表面、操作可达、Tooltip 只在溢出显示。 |
| 封面与焦点 | 真正外角按层次裁剪；中间封面不误圆角；按钮外侧焦点完整；失败媒体框稳定。 |
| 集合布局 | 同行每卡表面高度与 Footer 底边相同，其他行不受全局等高影响；列数随容器宽度变化。 |
| 内容隔离 | Tabs、Collapse、Input、独立 Button、菜单、滚轮与虚拟列表继续沿用自身输入合同。 |
| 选择 | Checkbox Space、RadioGroup 方向键、禁用、宿主拒绝更新；没有 Card 第二份状态或重复通知。 |
| 主操作 | Button 的鼠标／Enter／Space 各一次；安全区域一次；子动作不连带触发；拖选／拖拽／右键零次。 |
| 焦点与浮层 | 更多／详情打开与 Escape 关闭后返回有效按钮；Card 不吞应用 Action，不额外进入 Tab。 |
| 加载组合 | placeholder 与 content 的实际切换、隐藏分支不可操作、业务 Entity 保留、无重复请求；首次占位与刷新策略符合文案。 |
| 主题与动效 | 四个主题组合与局部覆盖、状态不改布局；减少动效即时完成；静止无持续帧。 |
| 虚拟集合 | 宽度／rem／内容变更重测、滚动正常、稳定 ID 不错配输入与选择；不重建整个数据集。 |
| Windows 真机 | 当前屏幕的可读性、阴影、圆角、滚动、Narrator 和高对比度；未执行项目单独列出。 |

上表为验收矩阵；自动化已覆盖项、未执行的真机检查及具体命令见[实施记录](research/2026-11-09-card-implementation.md)。自动化布局检查不能抵充 GPU 像素、Narrator 或跨 DPI 验收。

## 13. 实施顺序与完成标准

1. **基础部件与 tokens**：确认设计系统映射，完成表面、尺寸、Header／Meta／Cover／Footer、圆角与省略规则；补公共导出和核心布局测试。
2. **已有能力组合**：完成示例、选择／加载／Tabs／Collapse／Empty、ListGrid 与虚拟卡片集合；必要的 List 外壳布局改动单独验证，保持单一滚动所有者。
3. **整卡主操作**：先用原生窗口验证 Button 与安全区域共享命令、拖选保护和独立子操作，再发布 `primary_action`。无法满足合同则保留明确按钮，不宣称已实现整卡命中。
4. **收尾验收**：主题／rem／减少动效／焦点／性能静止检查，用户执行 Windows 视觉验收；同步实际签名、默认值、测试证据和已知限制。

保留内容的覆盖加载、整卡选择命中、背景图可读性自动处理不在基础版必做清单。后续只因真实业务需求推进，不为增加属性数量扩展 Card。基础版完成的标准是结构组合可靠、能复用已有控件、受限尺寸下可读可操作，且文档中的能力与已验证实现一致。
