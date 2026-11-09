# YueBadge 徽标组件设计

- 状态：V1 已实现（2026-11-07）；Windows 实窗字体、裁剪、DPI 与 Narrator 由用户验收。
- 日期：2026-11-07。
- 目标：为头像、图标、按钮等提供数字、圆点和短文本提醒，同时支持独立计数与状态点展示，复用项目已有的布局、主题、排版和动效基础。
- 参考：[参考图 1](assets/badge-reference-1.png)、[参考图 2](assets/badge-reference-2.png)，以及本项目锁定的 GPUI Kit 0.8.0 源码。

## 1. 定位与边界

`YueBadge` 是非交互的信息标记。常见用途是通知数量、新内容提醒和对象状态；业务数量、是否已读、状态含义与清除提醒的时机均由宿主掌握。组件不会自己增加计数、轮询消息或在点击后清零。

与现有组件的分工：

| 组件 | 负责内容 | 与 Badge 的组合 |
| --- | --- | --- |
| `YueTag` | 分类、属性、可选择或可关闭的短标签 | 独立属性继续使用 Tag；不把 Tag 的点击、移除和选择行为搬到 Badge。 |
| `YueAvatar` / `YueAvatarGroup` | 身份图像、姓名回退、头像组 | Badge 包裹头像附加提醒；组内的 `+N` 仍是 AvatarGroup 自己的成员省略项。 |
| `YueButton` / `YueLink` | 命令或外部导航 | Badge 附着在外层，宿主控件继续拥有点击、焦点、键盘操作与命令说明。 |
| `YueListItem` | 行内容与布局 | 行尾数量使用独立 Badge；需要角标时附着在行内图标或头像上。 |
| `YueSpinner` / `YueProgress` | 正在执行的操作与进度 | 真实加载反馈继续复用这些组件，不用持续闪烁的 Badge 代替进度。 |

首版包含数字、圆点、短文本、独立展示、状态点与说明、五种语义色、自定义颜色对、两档尺寸、两种外形、右上角定位与偏移，以及可选的短过渡。

首版不包含角标点击、关闭按钮、自动已读、菜单、任意位置枚举、丝带样式、图标型角标、富文本插槽、数字翻牌或无限呼吸动画。项目当前只面向 Windows，不新增跨平台适配层。

## 2. 两份参考图的吸收方式

| 参考能力 | Yue 设计 |
| --- | --- |
| 基础角标 | 一个宿主元素，角标中心默认锚定宿主右上角。 |
| 数字与最大值 | 默认上限 99；超过上限显示 `99+`，实际数量保留在可访问说明中。 |
| 零值开关 | 数字为 0 默认隐藏；`show_zero(true)` 显示 0。 |
| 红点提醒 | 显式 `.dot()` 选择圆点；是否显示由 `.visible(...)` 控制，不要求再传一个虚假的 count。 |
| `NEW` / `HOT` 等短文本 | `.text(...)`，长度受限并可省略；完整语义仍可读取。 |
| 无 children 独立使用 | 无 `.child(...)` 时按徽标自身尺寸参与正常布局。 |
| 标准、小号尺寸 | `Medium` / `Small`，读取 Yue token，不从截图比例推算尺寸。 |
| 圆形与方形 | `Pill` / `Rounded`；Pill 的单字符数量呈圆形，多字符自然成为胶囊。 |
| Default / Processing / Success / Warning / Error 状态点 | 使用 `.dot().tone(...).label(...)` 组合；处理中使用 Primary 配合明确说明，不建立第二套业务状态机。 |
| 预设色和自定义色 | 五种 Yue 语义色，加显式背景/前景颜色对；不复制网页的彩虹色字符串表。 |
| 动画示例 | 仅在显式启用时采用短淡入淡出；数量直接更新，无滚轮数字或常驻循环动效。 |
| style / className / dotStyle 等 | 不移植 CSS 入口；用类型明确的尺寸、形状、偏移和颜色配置。 |

首版借鉴信息表达和组合关系，颜色、字体、间距与缩放遵循 Yue。两图没有展示完整的截断、命中与无障碍行为，这些由下文补齐。

## 3. GPUI Kit 能力核对与复用

已核对本地锁定版本，而非仅依据网页名称推断：

| 已有能力 | 核对结果与采用方式 |
| --- | --- |
| `gpui_kit::base` | 当前没有 Badge 专用组件；复用已有无样式基础能力。 |
| GPUI Component 0.8.0 的 `badge::Badge` | 提供 count、默认 max=99、dot、icon、自定义背景色、宿主 children 与尺寸配置。其 Dot 不依赖 count；数字为 0 隐藏。 |
| 上述 Component Badge 的限制 | 使用其 `ActiveTheme`、固定白色前景和若干像素偏移；没有本设计的短文本、show_zero、状态说明合同。当前源码无独立展示的固有尺寸处理，不能直接视为已满足独立用法。 |
| 本项目 GPUI Kit 接入 | 已关闭默认 features；不为 Badge 启用 Component 层，也不引入另一套主题。Component Badge 只作为能力与边界参考。 |
| GPUI 布局、文本测量、Element 生命周期 | 负责宿主与徽标测量、定位和绘制。采用真实排版宽度，不用字符数估计角标宽度。 |
| Yue `tokens.rs` / `typography.rs` | 复用 token 解析、UI 字体和数字字体、`tabular_nums()`。 |
| Yue `motion.rs` 与 Base Transition / Presence | 复用短过渡和减少动态效果策略；不创建独立计时器或动效引擎。 |
| Base 可访问性与 `TestSupportExt` | 保留宿主语义，注册实际徽标节点与布局事实，供无头测试检查。 |

Badge 是展示型组合，采用 `RenderOnce`。宿主保留业务值；可选过渡仅保留以稳定 ID 索引的短期呈现状态，不需要公开的 `YueBadgeState` 或每个徽标一个 `Entity`。

定位优先使用现有布局能力。如果普通 Div 无法表达随文本宽度变化的半宽偏移，可使用 Badge 私有 Element 在布局/预绘制阶段读取真实尺寸。该 Element 只负责 Badge 的局部几何，不为定位向业务 Entity 回写测量值或反复触发 `notify`。

## 4. 公共 API

```rust
pub enum BadgeSize {
    Small,
    Medium, // 默认
}

pub enum BadgeShape {
    Pill, // 默认；圆形或胶囊
    Rounded,
}

pub enum BadgeTone {
    Default,
    Primary,
    Success,
    Warning,
    Danger, // 默认提醒色
}

pub struct YueBadge { /* RenderOnce，字段私有 */ }

impl YueBadge {
    pub fn new(id: impl Into<ElementId>) -> Self;
    pub fn theme(self, theme: YueTheme) -> Self;
    pub fn use_default_theme(self) -> Self;

    pub fn count(self, count: u64) -> Self;
    pub fn dot(self) -> Self;
    pub fn text(self, text: impl Into<SharedString>) -> Self;
    pub fn max_count(self, maximum: u64) -> Self;
    pub fn show_zero(self, enabled: bool) -> Self;
    pub fn visible(self, visible: bool) -> Self;

    pub fn child(self, child: impl IntoElement) -> Self;
    pub fn label(self, label: impl Into<SharedString>) -> Self;
    pub fn size(self, size: BadgeSize) -> Self;
    pub fn shape(self, shape: BadgeShape) -> Self;
    pub fn tone(self, tone: BadgeTone) -> Self;
    pub fn colors(self, background: Rgba, foreground: Rgba) -> Self;
    pub fn separator_color(self, color: Rgba) -> Self;
    pub fn offset(self, offset: Point<Rems>) -> Self;
    pub fn animated(self, enabled: bool) -> Self;

    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self;
    pub fn semantics(self, enabled: bool) -> Self;
}
```

合同：

- 构造默认解析当前应用主题；`.theme(...)` 显式覆盖，`.use_default_theme()` 恢复默认主题来源。
- 初始模式为数字 0、max=99、show_zero=false、visible=true、Medium、Pill、Danger、零偏移、animated=false；因此空构造不显示徽标。
- `.count(...)`、`.dot()`、`.text(...)` 都替换同一个内部内容枚举，最后调用者生效，不保留多个互相竞争的可见模式。
- `.max_count(...)` 与 `.show_zero(...)` 仅影响数字模式；max 必须大于 0，0 为配置错误；数量采用非负整数，不接受负数、浮点或 NaN。使用 `u64::MAX` 可避免数量截顶，不执行可能溢出的 `max + 1` 运算。
- `.visible(false)` 隐藏徽标及其独立说明，不移除宿主；再次显示仍使用宿主传入的当前值。
- `.text(...)`、`.label(...)` 和显式可访问名称拒绝空白文本。文本只展示单行，换行/控制字符应在入口规范化为空格，不被解释为富文本。
- 只允许一个宿主；第二次 `.child(...)` 替换前者。不实现 `ParentElement` 接受任意多宿主；多个元素应先由调用方组成一个容器。
- `.label(...)` 是独立模式下圆点/徽标右侧的可见说明。附着模式不接受 label，应使用宿主文字或可访问描述；渲染前校验这一组合，避免静默丢弃文字。
- Dot 始终圆形；shape 仅影响数字与文本。offset、separator_color 只影响附着模式。
- `.colors(...)` 显式覆盖当前 tone 的颜色；之后 `.tone(...)` 清除覆盖，恢复语义色。颜色通道须有限且在 0..=1，调用方负责自定义颜色在真实背景下的对比度。
- 不开放通用 `Styled` 改写内部几何；宿主的外部布局用普通容器组合，徽标内部尺寸由 token 统一控制。

## 5. 内容与可见性

| 配置 | 视觉结果 | 语义事实 |
| --- | --- | --- |
| 初始构造 / count(0) | 隐藏 | 无徽标节点；宿主保留。 |
| count(0) + show_zero(true) | `0` | 真实数量为 0。 |
| count(8) | `8` | 真实数量为 8。 |
| count(99) + max_count(99) | `99` | 到达阈值仍显示精确值。 |
| count(100) + max_count(99) | `99+` | 真实数量为 100。 |
| count(12) + max_count(9) | `9+` | 真实数量为 12。 |
| dot() | 圆点 | 默认语义“有新内容”；可以显式覆盖。 |
| dot() + visible(false) | 隐藏 | 不保留旧提醒语义。 |
| text("NEW") | `NEW` | 默认读取完整文本，可显式提供“有新内容”。 |
| count(8).dot().text("HOT") | `HOT` | 最后一个模式设置生效。 |

数字排版采用真实文本宽度和等宽数字特性。数量上限只压缩视觉表达，不改写业务值；不可访问描述成“99 条”却丢失实际 100 条。

自定义文本默认用于 `NEW`、`HOT`、“新”等短提示。达到文本最大宽度后单行省略，完整文本继续作为默认语义；不自动创建 Tooltip 或焦点节点。需要额外解释时由宿主组合现有 YueTooltip。

## 6. 布局、命中与裁剪

### 6.1 附着模式

宿主先按原有约束完成布局，Badge 包装层报告同一布局尺寸。徽标作为不参与宿主尺寸计算的绘制内容，默认中心对齐宿主右上角：

```text
anchor = (host.right, host.top)
badge.origin = anchor + offset - badge.size / 2
```

offset 使用 rem 长度，正 x 向右、正 y 向下。无论数字从 `9` 变成 `99+`，还是切换到圆点/短文本，宿主的位置、尺寸和相邻项间距都不应变化。Badge 包装层不得继承默认 flex stretch，把一个小图标的定位框扩大到整行。

角标本体的圆形/圆角裁剪只约束其内容；包装层不添加 overflow_hidden。外侧分隔轮廓使用表面颜色与 token 宽度，不挤压文本或扩张布局尺寸。默认表面来自 `--surface`；置于侧栏或其他表面时，通过 `.separator_color(...)` 明确匹配实际背景。

角标默认在宿主之后绘制，盖在宿主上方，但不创建独立点击处理、焦点或遮挡性 hitbox。点击角标与宿主交叠的区域必须继续到达宿主；伸出宿主范围的部分不扩大宿主的点击区域。不能通过复制宿主 on_click 达成透传，也不能调用 occlude 阻断事件。

允许包裹任意一个现有组件，但不反向访问它的私有字段，不改写它的 role、FocusHandle 或事件。

### 6.2 独立模式与状态点

没有宿主时，徽标按自身宽高参与正常布局；不保留虚构的宿主宽度，也不使用附着定位。

`.label(...)` 在徽标右侧以正文辅助样式显示说明，两者垂直居中、间距由 token 控制；说明不进入徽标的实色背景。状态映射示例：

| 展示 | 组合 |
| --- | --- |
| 默认 / 未启动 | Dot + Default + 明确说明 |
| 正在处理 | Dot + Primary + “处理中” |
| 成功 | Dot + Success + “已同步” |
| 警告 | Dot + Warning + “需要注意” |
| 错误 | Dot + Danger + “同步失败” |

Primary 遵循当前主题的强调色，不保证所有主题都显示蓝色。状态必须有文字或可访问说明，不能只展示五个不同颜色让用户猜含义。

独立徽标隐藏后布局尺寸为零、不留下空占位；若父容器为兄弟项设置 gap，gap 是否保留仍由父容器的子项组合决定。大量列表默认关闭动画，避免成批隐藏时保留过渡占位。

### 6.3 父级裁剪与滚动

Badge 遵守宿主所在窗口的正常裁剪与滚动，不进入 Popup host、不绕过列表视口、不提升到窗口最上层。祖先若裁剪边缘，可能裁掉超出宿主的半个角标；应由调用方预留外部空间或使用负偏移把徽标移回可见范围。

不能为避免角标裁剪去关闭整个虚拟列表的裁剪。虚拟行高度只计算宿主盒，Badge 不改变虚拟滚动基础层的尺寸反馈。

## 7. 主题与尺寸 token

当前已有七个 Badge token，均已核对生成快照和 Yue Design System 的 `component-tokens/badge.css`：

| 已有 token | 当前值/来源 | 使用方式 |
| --- | --- | --- |
| `--badge-height` | 20px / `--size-20` | Medium 高度与最小宽度。 |
| `--badge-font-size` | 12px / `--font-size-sm` | 数字及短文本字号；Small 也保留可读字号。 |
| `--badge-padding-inline` | 8px / `--space-8` | Medium 短文本的左右内边距。 |
| `--badge-border-radius` | full radius | Pill 外形。 |
| `--badge-background` | `--surface-component` | Default 中性徽标背景。 |
| `--badge-foreground` | `--text-primary` | Default 中性徽标文字。 |
| `--badge-color` | `--text-secondary` | Default 圆点颜色；不是另一份徽标正文前景。 |

现有中性 Badge tokens 用于 Default；提醒配色使用独立组件 token。以下新增 token 已在 Yue Design System 源中补齐，并由同步工具更新本仓库快照；实现没有手改 generated.rs 或 inventory.json：

| 新增 token | 来源/用途 |
| --- | --- |
| `--badge-height-sm` | `--size-16`，Small 高度 16px。 |
| `--badge-count-padding-inline` | `--space-4`，数字模式的左右内边距。 |
| `--badge-padding-inline-sm` | `--space-4`，Small 短文本内边距。 |
| `--badge-dot-size` / `--badge-dot-size-sm` | `--space-8` / `calc(var(--space-4) + var(--border-2))`，8px / 6px。 |
| `--badge-shape-radius` | `--radius-xs`，Rounded 的紧凑圆角。 |
| `--badge-separator-width` / `--badge-separator-color` | `--border-1` / `--surface`，附着徽标的外侧分隔轮廓。 |
| `--badge-label-gap` | `--space-8`，独立说明与标记之间的间隔。 |
| `--badge-text-max-width` | `--space-64`，短文本模式的最大外框宽度 64px；不限制精确数字。 |
| `--badge-primary-background` / `--badge-primary-foreground` | `--accent-solid` / `--on-accent`。 |
| `--badge-success-background` / `--badge-success-foreground` | `--success-solid` / `--on-success-solid`。 |
| `--badge-warning-background` / `--badge-warning-foreground` | `--warning-solid` / `--on-warning-solid`。 |
| `--badge-danger-background` / `--badge-danger-foreground` | `--error-solid` / `--on-error-solid`。 |

Dot 使用对应 tone 的背景色绘制，Default Dot 使用 `--badge-color`。label 使用现有辅助文字色和 UI 字体，不使用徽标背景上的反色文字。颜色对须在 Light/Dark、Azure/Neutral 与 neutral tone 下验算，小字对比度目标不低于 4.5:1；不能因语义 token 名称正确就省略对比度验证。

数字/短文本高度固定，宽度为 `max(高度, 排版宽度 + 两侧内边距)`；短文本再受最大宽度约束。默认字号和 padding 使单个数字自然落在圆形内，多位数字自然扩展成胶囊。Rounded 使用同样的宽高计算，仅改变圆角。Dot 不带数字内边距。

上述像素仅是当前默认设计值；实际布局统一换算为 rem，随应用主题缩放变化，不额外写第二份尺寸常量表。外侧轮廓需按 GPUI 绘制规则进行像素对齐。

## 8. 可访问性与交互合同

- Badge 不提供按钮角色，不进入 Tab 顺序，也没有 hover/pressed/disabled 的假交互样式。宿主被禁用时仍可显示数量，是否隐藏由业务决定。
- 默认语义表达为非交互文本，优先沿用项目静态标签的 Caption 类语义；实际角色和名称须用 AccessKit 树验证，不把静态提醒声明成 live Status 或 Alert。
- 数字默认名称表达真实数量；带 label 时合并说明和真实数量。Dot 默认“有新内容”，自定义文本默认完整文本。业务单位与含义通过 `accessibility_label` 显式提供，组件不能把任意数字都称为“未读消息”。
- 显式可访问名称非空；`.semantics(false)` 只关闭徽标及其说明的独立语义，必须保留宿主原有语义。隐藏徽标不能残留过期数量节点。
- Badge 无法可靠地改写任意宿主控件的可访问描述。键盘聚焦按钮需要读出数量时，宿主显式设置 `accessibility_description`，同时关闭 Badge 独立语义，避免重复播报。
- 例如：Badge 显示 `99+`，按钮描述应是“128 条未读消息”。两处值来自同一宿主业务事实。不能仅在角标节点写一个名称就声称 Narrator 聚焦按钮时一定会读到。
- 独立状态点与可见说明应形成一个可读语义，圆点自身不再重复报一次“有新内容”。自定义色不自动产生成功/警告等语义。
- 数量变更不默认主动播报。需要即时播报的业务由宿主选择现有通知机制，不让大量徽标更新占用读屏语音。

## 9. 动效与性能

默认 `animated(false)`；静态标记不请求持续重绘。显式启用时：

- 首次挂载直接显示当前事实；后续从隐藏到显示采用 Enter/Fade，显示到隐藏采用 Exit/Fade，复用 `motion.rs` 与 Base Presence。
- 数量和文本在新帧直接替换，不逐字滚动、不插值业务数量，也不为了位数改变重排宿主。样式变化无需加入单独的彩色闪烁。
- 退场期间可以保留旧的纯视觉内容完成淡出，立即移除其过期语义。附着宿主始终保留；独立模式仅在退场期间保留原徽标尺寸，结束后释放。
- 快速反向更新从当前透明度继续；稳定 ID 用于区分各徽标。卸载后不得留下定时任务、绘制快照或后续帧请求。
- 系统启用减少动态效果时直接呈现目标结果，不增加计时等待。常驻状态点不添加呼吸循环。

正常状态只派生当前文字和可见性，不扫描外部数据、不分配实体、不缓存另一份 count。若确需测量缓存，必须包含文本、字体、主题字号、rem 和显示缩放的失效条件；没有测得收益前优先使用 GPUI 的文本排版缓存。

## 10. 组合示例

以下为当前 API 用法；对应组合已纳入 Showcase。代码片段中的 theme 与 has_unread_updates 由宿主提供。

```rust
// 头像角标：Badge 不改写头像自身的裁剪形状。
let avatar = YueBadge::new("profile-notifications")
    .count(8)
    .child(YueAvatar::new("profile-avatar", theme).name("王伟"));

// 数字独立展示，适合列表尾部的数量。
let quantity = YueBadge::new("inbox-count")
    .count(128)
    .tone(BadgeTone::Default)
    .accessibility_label("128 条未读消息");

// 状态点带说明。
let status = YueBadge::new("sync-status")
    .dot()
    .tone(BadgeTone::Success)
    .label("已同步");

// 键盘聚焦时由按钮读出数量，Badge 不制造第二份语义。
let notifications = YueBadge::new("notifications-badge")
    .count(128)
    .semantics(false)
    .child(
        YueButton::new("open-notifications", "通知")
            .accessibility_description("128 条未读消息")
            .on_click(|_, _, _| { /* 宿主打开通知 */ }),
    );

// 圆点模式无需虚构 count；宿主明确控制已读状态。
let updates = YueBadge::new("updates-badge")
    .dot()
    .visible(has_unread_updates)
    .accessibility_label("有未读更新")
    .child(YueAvatar::new("updates-avatar", theme).name("Yue"));
```

Showcase 使用已有 YueButton、YueSwitch、YueSlider 或数量输入组件驱动数量与可见性，不用临时可点击 Div。页面演示：

1. 基础数字、圆点与零值开关；附着 Avatar、Icon 和 Button。
2. 独立的 `0`、`2`、`16`、`99+`，以及列表行尾数量。
3. `NEW`、`HOT`、中文短文本与长文本省略。
4. 上限 9 / 99 / 999，在恰好等于和刚超过阈值时切换。
5. 五种状态点配可读说明、五种语义色与一个显式自定义颜色对。
6. Small / Medium、Pill / Rounded，文本宿主与头像宿主的定位。
7. 偏移、深色背景分隔轮廓、靠近裁剪边缘的正确用法。
8. 数量增减、显示/隐藏、可选过渡和减少动态效果。

## 11. 文件组织与实施顺序

```text
crates/yue-component/src/badge/
├── mod.rs       # 公共枚举、私有配置和 builder
├── element.rs   # 内容呈现、独立布局与宿主附着布局
├── tests.rs     # 内容规则、参数校验与主题对比度
└── tests/
    └── visual.rs # 几何、绘制、语义、输入与过渡回归

examples/windows-showcase/src/pages/badge.rs
docs/33-badge.md
docs/assets/badge-reference-1.png
docs/assets/badge-reference-2.png
```

只有在私有 Element 的生命周期职责确实独立时再拆文件，不预建 State、manager 或通用 badge service。根导出及 prelude 同步公开 `YueBadge` 和选项；复用现有 Showcase 路由与主题切换。

本轮已按以下顺序完成：

1. **Token 与内容规则**：补齐设计系统 token，重新同步快照；完成数字格式、可见性、模式替换与参数校验。
2. **独立呈现**：完成两档尺寸、两种外形、语义色、短文本和状态说明，明确独立布局尺寸。
3. **附着组合**：完成真实测量定位、分隔轮廓、偏移、命中透传与裁剪；以 Avatar、Button 和 List 验证布局不回流。
4. **过渡与完整验收**：接入可选动效和减少动态效果，补齐 Showcase、公共调用检查、无头绘制测试及文档实施记录。

本轮未升级依赖或新增 GPUI/Base 补丁。局部附着 Element 直接复用宿主布局节点，在预绘制阶段测量并定位徽标，绘制遵守当前父级裁剪；可选退场只保留呈现快照。

## 12. 验收与证据边界

| 类别 | 必须验证的事实 |
| --- | --- |
| 内容规则 | 0、1、阈值、阈值+1、u64::MAX；show_zero、visible、max 校验；模式最后设置生效；文本规范化及空白拒绝。 |
| 附着布局 | 数字位数、长文本、Dot 和隐藏切换时，宿主尺寸与相邻项位置保持一致；锚点以真实徽标半宽/半高计算；正负偏移符合定义。 |
| 独立布局 | 无宿主仍有有效宽高；隐藏无固有占位；说明不进入色块；两档尺寸、Rounded/Pill、长中文和 emoji 不撑开宿主。 |
| 绘制 | 数字和圆点背景、前景、分隔轮廓、圆角真实绘制正确；角标在宿主上方；文字最大宽度与省略可见；浅深主题都有边缘分隔。 |
| 交互组合 | 点击角标覆盖宿主的区域仍触发一次宿主命令；外溢区域不扩大宿主点击范围；按钮禁用与键盘激活不受包装影响；Tab 不多出停靠点。 |
| 语义 | 真实数量与视觉 `99+` 区分；自定义说明、semantics(false)、隐藏节点和独立状态说明正确；宿主角色/描述/焦点保持。 |
| 约束与缩放 | 四种主题 profile、neutral tone、16/24 rem、模拟显示缩放、窄容器、列表视口裁剪；虚拟行测量不被角标扩大。 |
| 动效 | 首帧不闪烁、关闭时无持续帧、开关可逆、退场语义及时移除、卸载任务释放、减少动态效果立即完成。 |
| 性能 | 大量静态徽标没有定时器/独立 Entity；仅对实际变化刷新；必要时再基于现有虚拟列表场景比较数量变化前后成本。 |

先使用纯测试证明格式与校验，再通过 headless 窗口检查真实生产元素的布局、AccessKit 事实和绘制 primitive；不只测试一份脱离组件的定位公式。涉及按钮的验收必须派发真实指针与键盘输入，不能直接调用回调冒充组合成功。

完成实现时运行适用的格式、Clippy、库测试、Showcase 编译与 token 同步检查。Windows 实窗由用户验收字体、细小角标与分隔轮廓、裁剪、DPI 和 Narrator；自动化不能声称替代实机播报或真实多显示器验收。本文不承诺当前设备无法执行的跨显示器测试。


## 13. V1 实施记录

- 公共实现位于 `src/badge/`，根导出与 prelude 已接入；覆盖本设计的内容、布局、主题、语义及可选过渡 API。
- Showcase 增加独立 Badge 页面，展示头像/图标/按钮角标、独立数字、短文本、阈值、状态点、五种语义色、自定义色对、两档尺寸、两种形状、偏移及列表行尾数量；按钮和 Switch 驱动数量、可见性与动效。
- 11 项 Badge 测试覆盖格式与参数边界、真实数量语义、布局不回流、真实指针命中与键盘激活、禁用/焦点、文本宽度、主题尺寸与圆角、父级裁剪、模拟显示缩放、外侧分隔轮廓、过渡反向及减少动态效果。
- 配色测试覆盖 Light/Dark、Azure/Neutral 与全部 neutral tones，五种徽标文字配色均达到 4.5:1。
- 无头绘制检查使用 GPUI 实际 Quad/Shadow primitive；没有把语义名称或配置值当作最终视觉证据。静态徽标已检查无持续下一帧请求。
- 验证通过：`cargo test -p yue-component --features headless-tests --lib`（385 项）、Badge 定向测试（11 项）、Showcase 侧栏导航测试、库与 Showcase 的 all-targets Clippy（`-D warnings`）、局部 rustfmt、设计系统 token 源同步检查及 `git diff --check`。
- Windows 实窗尚未由本轮自动化验收。待用户检查小字字形、省略号、分隔轮廓和真实 DPI/Narrator；模拟显示缩放不等于真实多显示器验收。
