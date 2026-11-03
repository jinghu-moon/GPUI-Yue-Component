# 07 Yue Dropdown Menu

## 2026-10-30：菜单行高亮的源码核对与视觉参数调整

当前实现的高亮绘制链路为 `MenuPanel::bind_row` → `PanelMotion` 的 active/hovered → `row_tint` → Base `transition` → `mix_rgba` → 行背景 `.bg(...)`。本项目现在自绘菜单行，没有使用 Kit 内置 `MenuItemElement` 的 hover 背景。

对照锁定依赖的源码：

- `gpui-component 0.8.0/src/menu/menu_item.rs`：内置行通过 `.hover(|style| style.bg(...))` 和 selected 条件设置背景，没有安装颜色过渡。
- `gpui-pre 0.3.9/src/elements/div.rs`：`.hover(...)` 保存 `StyleRefinement`；解析交互样式时命中 hover 就 `style.refine(hover_style)`。这是状态样式合并，没有 CSS transition 引擎。
- `gpui-base 0.8.0/src/motion.rs`：`transition_with_status` 通过 `Window::use_keyed_state` 保存 from/target/start time。新通道首次直接采用目标；后续目标变化先采样当前值，再从该值向新目标插值。进行中调用 `request_animation_frame()`；反向切换使用当前值并缩短返回时长。
- `gpui-pre 0.3.9/src/window.rs`：keyed state 按全局元素身份跨连续帧保留，`request_animation_frame` 在下一帧通知当前 view 重绘。因此每行通道必须稳定且每帧继续采样，只有改颜色或监听 hover 并不会自动产生动画。
- Base 的 `reduce_motion.rs` 初始化时读取 Windows `SPI_GETCLIENTAREAANIMATION`；`App::reduce_motion()` 为 true 时 `transition` 直接返回终值。尊重此设置，不为了视觉演示强制开启动画。Windows 运行期间修改系统设置，需通过 Base 提供的同步入口重新读取或重启应用。

本次实际绘制测试证明旧实现并非完全没有动画：它借用了 `--popover-duration-exit` 的 100ms，低强度底色很快达到终态，125ms 与 250ms 的背景采样相同。当前行 hover、键盘高亮与按下 tint 统一使用新增组件 token `--menu-item-duration`（引用 `--duration-300`，300ms）与 `--menu-item-ease`（引用新增 `--ease-in-out`，`cubic-bezier(.33, 0, .67, 1)`），保留原有颜色强度；面板开关时长独立保留。进入/离开高亮均从当前色连续变化，快速反向不跳回旧端点。

保留并更新 3 条 UI 集成回归测试，从真实菜单行提交给 GPUI 的 `painted_quads()` 读取背景颜色，使用虚拟时钟和 `simulate_next_frame` 验证：

1. 指针进入时首帧连续，30ms 完成约 2.8%、75ms 约 15.7%、150ms 为 50%、225ms 约 84.3%，300ms 达到终态；旧 400ms 加速版本不能通过新比例断言。
2. 60ms 时移到另一行，旧行从当时颜色反向淡出；禁用行不产生高亮。
3. 方向键切换同样具有中间帧；开启 reduced motion 后直接到达终态。

这些测试证明绘制命令中的颜色过渡和帧调度，不代替 Windows GPU 输出、实屏帧率或读屏验证。

参数试验经过 100ms 标准缓动、250ms 标准缓动、250ms 线性三个版本后，曾改为 400ms 先慢后快；用户反馈偏慢后，当前改为 300ms 的平衡加速/减速曲线。它是针对感知节奏的设计调整，不是动画机制修复；用户提供的真实窗口插值和屏幕像素采样也确认了此前版本的渐变正常执行。背景幅度与菜单行身份保持原样，未增加淡入延迟。当前策略同时用于移入和移出；中途反向仍由 Base 按已完成进度缩短返回时长，因此短暂停留后的淡出可能少于 300ms。

### 300ms 的 token 与曲线来源

原有 `--duration-300`、`--duration-400`、`--ease-exit` 均由设计系统 `packages/tokens/src/primitives/motion.css` 定义，并已生成到 GPUI catalog。此前没有菜单专用的动画 token。本次新增 `--ease-in-out`，以及 `component-tokens/menu.css` 中的 `--menu-item-duration → --duration-300`、`--menu-item-ease → --ease-in-out`，再通过同步脚本生成；Rust 不内嵌曲线或时长。

参考 [Fluent UI curveEasyEase](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/curves.ts)，选用 `cubic-bezier(.33, 0, .67, 1)`。它提供温和的双端缓动；[Ant Design](https://github.com/ant-design/ant-design/blob/master/components/theme/themes/seed.ts) 也有双端缓动，但其具体曲线不同。300ms 是本项目按体验反馈选择的时长，不声称这些设计系统默认用 300ms 做 hover。百分比指颜色从起点到终点的变化进度，不是最终背景叠加比例。

| 时间 | 颜色变化完成度 |
| --- | --- |
| 0ms | 0% |
| 30ms | 约 2.8% |
| 75ms | 约 15.7% |
| 150ms | 50% |
| 225ms | 约 84.3% |
| 300ms | 100% |

本次只新增 token 并调整 GPUI 菜单高亮；Vue 组件的 API、样式接线、键盘与文案不涉及。无延迟、无额外位移动画，不修改全局既有标准曲线。

## 目标与参考图提炼

为 Windows 桌面应用提供基于命令菜单的下拉入口。三张参考图分别说明：

1. 示例展示多种触发控件、分隔线、多级菜单、禁用项和自定义弹层内容。
2. API 展示方向、禁用、点击后关闭、最大高度、分隔线、前置图标、主题、稳定 value、点击回调、选项、面板上下插槽、放置位置、Popup 属性和触发方式。
3. 指南划分下拉菜单与选择器的职责，展示触发器附近不同角落的弹出位置，并建议在重要操作分组时使用菜单、选项过多时使用级联或选择器。

本组件库服务 Windows GPUI 应用，因此保留参考图背后的交互意图，不照搬浏览器 DOM、hover-trigger 或任意 Popup 属性透传。主路径使用 GPUI Kit 的 `DropdownMenu` + `PopupMenu`；需要选择一个持续值时使用 Select/Combobox，不使用命令菜单伪装选择器。

## GPUI Kit 0.8.0 能力核对

GPUI Kit 0.8.0 通过 `gpui_kit::component::menu` 和 `button` 提供下列能力：

| 需求 | GPUI Kit 能力 | Yue 决策 |
| --- | --- | --- |
| 文本或图标按钮打开菜单 | `Button::dropdown_menu`，可设置 `Anchor` | 直接复用；触发器仍由 Yue Button 统一外观与无障碍名称 |
| 按钮主操作 + 独立菜单箭头 | `DropdownButton` | 作为 split-button 入口，不把两个动作合并到同一个点击目标 |
| 菜单项、禁用、图标、checked、分隔线 | `PopupMenuItem::{new,disabled,icon,checked,on_click,separator}` | 复用，不复制菜单项状态机 |
| 标题/非交互标签、子菜单 | `PopupMenu::label`、`submenu`、`submenu_with_icon` | 仅用于短分组；较复杂内容应换 Dialog/Popover/专用面板 |
| 行为 Action 与快捷键提示 | `PopupMenu::menu` / `menu_with_disabled`，GPUI Action | 命令型项目优先使用 Action，快捷键提示来自应用绑定 |
| 自定义菜单行 | `PopupMenuItem::element` | 仅用于可保持 MenuItem 键盘和语义的行；不提供任意嵌套表单内容契约 |
| 尺寸和长菜单滚动 | `min_w`、`max_w`、`max_h`、`scrollable` | 限制在窗口内，长列表使用滚动；不按每个子菜单随意配置列宽 |
| 菜单开关状态与焦点 | `DropdownMenuPopover::on_open_change`、菜单自带 focus handle 和 dismiss 管理 | 使用框架焦点恢复与关闭路径，不另建 Yue 弹层状态机 |
| 键盘导航 | PopupMenu 绑定 Enter、Escape、上下左右方向键，并支持子菜单 | 采用 Base/Kit 行为，Showcase 验证 Tab 进入、选择、Escape 返回触发器 |
| 右键上下文菜单 | `ContextMenuExt` | 右键对象操作用 ContextMenu，不伪装成普通按钮 dropdown |
| 自定义任意 Popover 内容 | `Popover` | 单独使用 Popover/专用组件；不是本菜单 API 的“插槽”扩展 |

### 平台与 API 边界

- GPUI Kit 的 `DropdownMenu` 是按钮触发的 Popover 菜单，不是截图中可任意设置 `hover/click/focus/context-menu` 的 Web Dropdown。默认点击触发；不把 hover 设为开菜单方式，避免鼠标掠过即展开和桌面键盘路径不一致。
- `Anchor` 是触发器的锚点角，不等同截图中完整 placement 字符串集合。Kit 的 popover 根据空间处理布局；Yue 首版只暴露经过验证的 `Anchor`/默认锚点，不承诺任意四边定位和 viewport 算法。
- `PopupMenuItem::checked` 表示菜单项目的勾选/当前项提示，不是由菜单持有的选择值。命令执行与应用模型更新仍由消费者控制。
- `PopupMenu` 默认有最小/最大宽度和最大高度边界；Yue 不额外照搬 `maxColumnWidth`、多列等网页属性。Kit 的 PopupMenu 是单列命令菜单。
- `PopupMenuItem::element` 可绘制自定义行，但不能据此承诺面板 top/bottom 插槽、复杂任意控件、文本输入、多选筛选或滚动联动都符合 Menu 键盘语义。
- Kit 使用 `PopupMenuItem` 的 icon API 接受 Kit `Icon`。若 Yue Tabler icon 无法直接转换为 Kit Icon，应在实现时验证适配：支持时直接桥接，否则菜单项以文字为基础，不引入文件路径或另一个 icon loader。
- `ContextMenuExt`、`DropdownMenu`、`Popover` 和 `Select/Combobox` 是不同语义入口。API 不设计成一个 `trigger` 字符串切换所有弹层行为。

## 组件边界

### 第一阶段：复用 Kit，不新增菜单引擎

Yue 提供 `YueDropdownMenu` 作为轻量 facade/扩展 trait，或者在确认没有需要统一的视觉差异后直接 re-export/文档化 GPUI Kit API。开始实现前用编译探针确定消费者最需要哪一种 seam。不得复制 `PopupMenu` 的焦点、键盘、子菜单和关闭逻辑。

候选调用形态（设计草案，非现有 API）：

```rust
YueDropdownMenu::new(
    YueButton::new("file-menu")
        .label("更多")
        .secondary(),
    |menu, window, cx| {
        menu.item(PopupMenuItem::new("复制链接").on_click(|_, _, _| {}))
            .separator()
            .item(PopupMenuItem::new("删除").disabled(true))
    },
)
```

更理想的复用路径可能是给 YueButton 暴露可转换/可包装为 Kit Button 的 seam，再调用 `.dropdown_menu(...)`。最终 API 必须从实际 crate 类型、trait bounds 和编译结果确定，避免公共 API 绑定私有实现或让消费者手动管理 `Entity<PopupMenu>`。

### 触发器类型

1. **文本/图标按钮**：一个明确的动作入口，例如“更多”或省略号按钮，主按钮本身只负责打开菜单。
2. **Split button**：左侧执行主动作，右侧箭头打开次级命令，使用 Kit `DropdownButton` 语义。两侧有独立可访问名称、点击区域和禁用逻辑。
3. **上下文菜单**：对象上的右键入口使用 `ContextMenuExt`；若同一命令也需触屏/键盘可发现，应另提供可见的更多按钮。

不把当前选择值显示在普通 dropdown trigger 里；要让用户选取并持续显示单个值，使用 YueSelect。多个对象/批量操作使用 CheckboxGroup 或专门筛选控件。

### 菜单项目

- `YueMenuItem` 若被引入，只包裹 Kit 的 PopupMenuItem 配置，不持有应用状态、不自己关菜单、不实现快捷键。
- 项目类型限于命令、外链、勾选命令、分隔线、短标签和子菜单。分隔线用于切分语义不同的命令组，不在每两项之间添加。
- 禁用项目应可见说明但不响应鼠标和键盘；重要原因通过邻近说明或 tooltip 表达，不能只以低对比度隐藏原因。
- destructive 项使用危险语义样式并以明确动词命名。验证 GPUI Kit PopupMenu 当前版本是否支持 Yue danger token；不支持时不通过未知 API 仿造。
- 图标用于快速识别少数熟悉动作；同一菜单中对齐一致，不能让每个项目各自选择颜色。
- 快捷键提示应反映实际绑定。使用 `Action` 菜单项以便 Kit 从 action context 查询，不手写可能失真的快捷键文本。
- 子菜单最多一层作为默认建议；命令树更深、选项很多或需要分类浏览时改用 Dialog/侧栏/Select，不建立多层级联迷宫。

## 交互与视觉规范

- 默认点击打开；再次点击、Escape、选择命令或点击外部按 Kit 的关闭规则收起。完成命令后菜单收起，除非该项是明确的 toggle/check 状态操作且框架语义要求保持打开；行为要按 Kit 实际实现测试。
- 打开后焦点进入菜单，方向键浏览可用项目，Enter 激活，Escape 关闭并将焦点返回触发器，左右键在父菜单/子菜单间移动。禁用项不可执行。Tab 的退出行为需在 Windows Showcase 实测，不自行承诺网页 ARIA 菜单行为。
- 菜单靠近触发器展开并根据窗口边界避让；锚点默认与触发按钮的 leading edge 对齐。Split button 选择适合箭头半区的锚点。
- 普通命令菜单宽度由最宽项目、快捷键提示和 Kit 的合理上下限决定；长菜单设置 max height 并滚动。菜单不跟随容器填满宽度，不作为表格/表单容器。
- Hover/keyboard highlight 与 checked 状态都可读。Focus 是菜单当前活动项，不要把 checked 和 hover 混成同一种视觉反馈。
- 色彩、圆角、阴影、间距和字号使用 Yue Theme/token。保留 Kit 的默认菜单视觉作为起点；只有发现与 Yue token 有稳定差异时才扩展 tokens，不做一份平行菜单调色板。
- 尊重 GPUI Kit 的自然桌面指针语义，不因网页截图用 hover 就要求光标手型。

## API 草案

若验证后 Yue facade 有实际价值，首版接口保持最小：

```rust
YueDropdownMenu::new(trigger, menu_builder)
    .anchor(anchor)
    .disabled(false)
    .on_open_change(|open, _, _| {});

menu.item(YueMenuItem::new("复制").action(CopyAction))
    .separator()
    .submenu("更多操作", |submenu| { /* items */ });
```

只保留 Kit 已能稳定支持且 Yue 需要统一的能力。不承诺截图中的完整 Web Props 清单（direction string、trigger mode、maxColumnWidth、minColumnWidth、panel slots、任意 popupProps、独立 onClick/value 模型）。Yue 类型不公开字段，稳定身份来自调用方 ElementId/Action，而不是可变标签或列表 index。

## 与其他组件选择

| 用户任务 | 组件 |
| --- | --- |
| 打开一组短命令 | DropdownMenu |
| 在一组命令里选一个持续值 | Select |
| 搜索/过滤大量选项后选择 | Combobox |
| 查看鼠标所在对象的上下文命令 | ContextMenu |
| 需要任意布局、表单或持续打开状态 | Popover 或 Dialog |
| 一个主动作加关联次动作 | Split button (`DropdownButton`) |

谨慎使用菜单承载“更多”。如果主任务需要多个常用动作，应将主要动作直接展示，次要和低频操作才收进菜单；不要把核心工作流藏起来。

## Showcase

在组件导航中增加 Dropdown Menu 页面，使用左侧目录、右侧可滚动示例文档布局。展示：

1. 文本按钮触发的基础命令菜单，带真实可观察的操作结果。
2. 图标触发器并提供可访问 label。
3. 带分隔线、禁用项、checked 项、快捷键提示的菜单。
4. 一层子菜单和键盘开启/退出。
5. Split button：主动作和箭头菜单各自只触发自己的操作。
6. 长菜单的高度限制、滚动和窗口边缘附近的定位。
7. ContextMenu 与 DropdownMenu 的使用对照，以及 Select/Combobox 的替代边界。

每个示例 ID 唯一。不得展示 hover-trigger、任意面板插槽或无法在 GPUI Kit 中实现的 placement 控件。

## 测试与验收

- API/编译：验证 Yue Button 到 Kit Button 的组合方式、Anchor 类型传递、Kit Icon 适配和 `PopupMenuItem` callback 生命周期。
- 交互：点击打开、再次关闭、点击外部关闭、命令只触发一次、disabled 不触发、separator 不可选择、子菜单打开/返回、Escape 关闭并恢复触发器焦点。
- 键盘：上下选择、Enter 激活、左右进入/退出子菜单、Tab/Shift+Tab 行为和焦点可见性；在 Windows 实际 Showcase 验证。
- 视觉：light/dark Yue profiles、长文本、checked/disabled、菜单靠近窗口四边、窄窗口和滚动。
- 无障碍：触发器可访问名称、Menu/MenuItem 角色、disabled 与 checked 状态。使用 Windows UI Automation/屏幕阅读器验证实际播报；GPUI element tree 单测不能替代平台验证。
- 明确边界：无头测试不能证明 Windows UIA 的菜单读屏、DPI 排版或系统焦点呈现；没有实现的 hover trigger 和任意 panel content 不列为验收项。

## 实现落地（gpui-kit 0.8.0 + gpui-pre 0.3.9）

实现位于 `crates/yue-component/src/dropdown/`（`mod.rs` 公开模型与 host，`behavior.rs` 导航/typeahead/dwell，`placement.rs` 定位与几何，`panel.rs` 绘制），从 `lib.rs` 导出 `YueDropdownMenu`、`YueContextMenu`、`YueDropdownSplitButton`、`YueMenuItem`、`DropdownPlacement`、`DropdownAlign`、`DropdownIndicatorSide`。

### 为什么没有复用 Kit 的 PopupMenu

- `gpui_kit::component` 是 gpui-kit 的 `component` feature 门控模块。workspace 根 pin 是 `gpui-kit = { version = "0.8.0", default-features = false }`，只有 `examples/windows-showcase` 额外开了 `features = ["component"]`；`crates/yue-component` 因此看不见 `DropdownMenu`、`DropdownButton`、`PopupMenu`、`PopupMenuItem`。
- 给组件库补上这个 feature 也换不来复用：库的无头测试需要 `gpui-base` 的 `test-support`，而 gpui-component 0.8.0 在同一张依赖图里编译失败（它的 `Observed` 包装不带 `ScrollableElement`，理由写在 `crates/yue-component/Cargo.toml` 的注释里）。
- 于是菜单坐在 `gpui_kit::base` 的弹层与动效原语（`PopoverState`、`Positioner`、`Presence`、`Transition`/`Timing`/`Keyframes`/`Stagger`/`animate_keyframes`）加 gpui 的 `div`/`canvas` 上。开合、焦点登记与焦点归还仍由 Base 的 `PopoverState` 负责——草案的"不另建 Yue 弹层状态机"守住了；"复用 PopupMenu 的行与键盘"没有做到，行模型、导航算术、绘制与动效都是 Yue 自己的代码。

### 公共 API

```rust
// 触发器是 Yue 按钮：host 负责 expanded 状态与开合，调用方的按钮不再挂 handler
YueDropdownMenu::new("file-menu", theme, YueButton::text("file-menu", theme, "更多"))
    .items(items)
    .label("选择操作")            // 面板的可访问名字
    .placement(DropdownPlacement::Auto)
    .align(DropdownAlign::Start)
    .indicator_side(DropdownIndicatorSide::Auto)
    .columns(1)                   // 只有可比的小集合（色板、图标格）才大于 1
    .max_height(px(320.))
    .panel_width(px(240.))
    .default_open(false)
    .on_open_change(|open, _, _| {});

// 触发器是任意元素：open 标志交给它自己表达箭头
YueDropdownMenu::with_trigger(id, theme, |open, window, cx| /* -> AnyElement */);

// 右键区域：整块区域就是触发面，面板的角落落在指针上
YueContextMenu::new("row-menu", theme).label("文件").items(items).child(region);

// 左半运行默认命令，右半（只有箭头的按钮，靠 arrow_label 命名）打开其余命令
YueDropdownSplitButton::new("save-menu", theme, save_button, "更多保存方式").items(items);
```

行由调用方每帧构造，自己不持状态：`YueMenuItem::{new, heading, separator, loading, submenu}` 决定这一行的种类，`.id(...)` 给出稳定身份（缺省用 label，分隔线用 `row-{index}`），`.icon(impl IconDefinition)` 直接吃 Yue 的 Tabler 图标（不需要 Kit 的 Icon，也没有第二个 loader），`.avatar(initials)`、`.hotkey(text)`、`.checked(bool)`、`.radio(bool)`、`.danger(bool)`、`.disabled(bool)`、`.keep_open(bool)`、`.rule(bool)` 与 `.on_click(|&ClickEvent, &mut Window, &mut App|)`。`ClickEvent` 会带 `Keyboard(Enter|Space)` 或鼠标点击，所以处理函数能分辨行是怎么被激活的。

`YueContextMenu` 刻意没有 `placement`/`align`/`indicator_side`/`columns`/`default_open`：一块没有触发器可对齐的面板，这些参数说不出差别。枚举取值是 `DropdownPlacement::{Auto, Above, Below, Leading, Trailing}`、`DropdownAlign::{Start, Center, End}`、`DropdownIndicatorSide::{Auto, Leading, Trailing}`。

### 面板几何、翻转与滚动

- `Positioner::side(anchor).placement(...).align(...).offset(gap).margin(8px)` 负责一级面板，`Positioner::corner(TopLeft, 指针点)` 负责右键面板；子面板固定请求 `Right + Align::Start`，右侧放不下由 Base 翻到左侧。
- 请求的边由 `placement` 给出：`Auto` 显式要 `Bottom`（Base 把"没有偏好"当成"先试上方"，这里要说出口才和参考件的下展顺序一致）。固定 placement 是请求，不是保证——碰撞规则可以拒。
- 间隙：触发器面板 12px（`--space-12`）、子面板 8px、右键面板 0。视口四边各避让 8px。
- 指向箭头只有触发器面板画：8px 方块转 45°，中心落在面板自己的边线上，只描朝外的两条边（`--popover-background` 填、`--popover-border-color` 描，线宽 `--popover-border-width`），尖端对准触发器中心，离圆角太近时夹进 16px 安全区；面板翻到上方后箭头跟着换边。子面板和右键面板没有箭头。
- 宽度两条边都挂在面板自己的盒子上（taffy 的 `min_w`/`max_w`/`w` 含 padding 与 border，与 CSS 的 border-box 同义）：地板是 `--menu-width`（192px；参考件 `.dd .menu` 写的是字面量 200px，这里以 token 为准），天花板是 `min(320px, 90% 视口宽)`，行盒在两者之间按内容生长——参考件那条 `min-width:200px;width:max-content;max-width:min(320px,90vw)` 就是这个形状。`.panel_width(px)` 或 `columns(n)` 把地板换成死宽（`width:min(请求, 天花板)`），长标签在这种面板里以省略号收尾而不是把它推宽。
- 指示位只在菜单真用它的时候占位。`menu_column` 把参考件的 `ind = o.side==='r' ? 'r' : has ? 'l' : null` 原样读成三态：`Auto` 有任何可标记行时列在标签之前，一行都标不了时整列不占位（`Absent`），只有显式 `Leading`/`Trailing` 才无视内容强行占位。列存在时它排在快捷键提示之前——参考件的行模板是 `${ind==='r'?slot(k):''}${n.h?<em class="hn">${n.h}</em>:''}`，最右的那个名字始终是快捷键本身。槽挂在提示之后的话，连一行都标不了的普通菜单也会在快捷键右边留下 24px（16px 空槽 + 8px 间隙）谁也贴不上边的空白。
- `max_height` 缺省为 `min(视口高 / 2, 450px)`，超过就在面板内部 `overflow_y_scroll`；键盘步进落到的行用 `ScrollHandle::scroll_to_item` 滚进视野。
- 面板是 deferred 出来的独立 surface，挂在宿主节点下而不是挂在行下面，所以列表滚动不会裁掉它；`columns > 1` 时行按 grid 排布，一列时是竖列。
- 锚点由宿主自己的 canvas 在 prepaint 记录，面板用的是这一帧开始时的位置：跟着一个正在滚走的触发器，会有一帧的滞后（已知取舍，见"与草案的差异"）。首次测到锚点时主动 `request_animation_frame`，所以同一帧用键盘打开的面板不会停在窗口原点；`popup(self.open && self.placed)` 保证没测过锚点的那一帧不挂面板。
- 关掉的面板从不挂载。`Presence` 在"缺席"的第一帧仍会给出 `phase = Exiting`、`progress = 0`，`popup` 因此额外拦下 `!present && progress <= 0.0`：那个隐形幽灵会正好压在触发器上，把 MouseDown 的焦点抢到自己身上，于是 Escape 归还的是键盘给一个已经不存在的 surface。
- 只有打开中的面板 `occlude()` 指针；退场中的面板照常画完淡出，但不绑任何 handler（`interactive = present`），等价于参考件的 `pointer-events: none`。

### 动效

全部数值取自 Yue token，参考件那份 ramp 表：

- 面板：`Presence` + `--popover-duration-enter`（200ms，与参考件的 `--duration-normal:200ms` 同值）配 `--ease-enter`（`cubic-bezier(0, 0, .38, .9)`）入场，`--popover-duration-exit`（100ms，即参考件的 `--duration-fast:100ms`）配 `--ease-exit`（`cubic-bezier(.2, 0, 1, .9)`）退场；进度只用来做两件事——整块透明度，以及沿"离开锚点"那条轴的 4px 位移。参考件闭合态的 `.menu.p-bottom{transform:translateY(-4px)}` 是"贴着触发器起步、张开时向外走"，所以这里读成 `offset = gap - 4px·(1-progress)`：起步时面板离触发器近 4px，开到位时退回正常间隙。写成 `+` 就是反的——面板往上收、行往下落（行的 `translateY(-4px)` 与参考件一致），两下互相抵消，观感退化成一个纯淡入。参考件的 `scale(.985)` 与随之而来的 `transform-origin` 没做：gpui 0.3.9 的 `Style` 没有 scale/rotate，deferred 子树整体不受变换，靠重排面板来伪造会让每行在两帧里重新布局。右键面板按指针落点挂，没有可离开的那条轴，所以只淡出不位移。
- 行入场：`Stagger::new(20ms, StaggerOrigin::First)` 逐行延迟（参考件 `animation-delay:calc(var(--i,0)*20ms)`），跑 `--popover-duration-enter` 的时长（它的 `animation:in var(--duration-normal)`），透明度 0→1 并跟着 4px 下落（`@keyframes in{from{opacity:0;transform:translateY(-4px)}}`）；通道名里带 generation，所以再开一次会重播，而一次什么都没改的 repaint 不会重跑。面板开了 520ms 之后停止入场（`ROW_ENTER_WINDOW`，即参考件的 `setTimeout(()=>p.classList.remove('enter'),520)`），后来的行直接画成终态。
- 高亮：hover 与键盘高亮共用一条 tint 通道，混色强度是 `--opacity-hover`（light .08 / dark .12），按下是 `--opacity-pressed`（.10 / .18），过渡使用 `--menu-item-duration`（引用 `--duration-300`，300ms）与 `--menu-item-ease`（引用新增 `--ease-in-out`，`cubic-bezier(.33, 0, .67, 1)`），让背景前段平缓、中段加速、末段减速。参考件的 `open()` 走 `focusFirst`，面板打开那帧首行就已经是高亮；Yue 也在打开时把标记放在首行（`MenuMotion::note_open` 收到 `first`），但 Base 给一条**新**通道的第一次采样直接返回目标值——于是这一行没有淡入，看起来像底色瞬间出现。所以每次开的入场首帧先给各行 tint 通道播一次 `0.0`（`Duration::ZERO`），同帧再把高亮目标交给它，让那一行也爬同一条 300ms；种子按 generation 只播一次（`PanelMotion::tint_seeded`）。
- 勾选与圆点：tick 复用 `checkbox` 的 `CHECK_LINE` 路径与 `paint_mark`，填墨走 200ms（延 100ms、`--ease-enter`）、擦墨走 100ms（`--ease-exit`）；radio 的环色与点半径骑同一条 200ms ramp，点不会先于环到色。
- loading 行：`outline::Loader` 以 `--spinner-duration`（900ms）循环旋转。
- Base 的 `Presence` 在 `cx.reduce_motion()` 或时长为 0 时直接返回端点值——这条由 Base 保证，Yue 侧没有为它写测试。

### 键盘与指针

- 触发器是菜单唯一的 Tab 停靠点。`YueButton` 触发器上，Enter/Space 与点击都开合（toggle 绑在按钮本身：gpui 把 Enter/Space 派发给持有焦点的那个元素自己的 click listener，绑在外层 wrapper 上会让键盘用户 Tab 到按钮却按不动）。`with_trigger` 的自定义触发器只回答指针，开合状态由 `open` 标志交给它自己表达。
- 打开时 `PopoverState` 记录 previous focus 并把键盘交给面板，面板自己 `track_focus`；方向键读在根面板上，再路由到当前显示的那块面板（子面板是它的后代），命中的键 `stop_propagation()`，没命中的字母留给窗口。
- 高亮是面板自己标的一行（`aria_selected` + `aria_active_descendant`），焦点不会逐行移动；指针停在哪一行也把标记放在哪一行，两个来源共用一个高亮。
- 键位：↑↓ 在可运行行之间步进（单列绕回），Home/End 到首尾，Grid 里 ↑↓ 整行步进、←→ 同行列步进并在边缘夹住，→/Enter 打开子菜单，←/Escape 先关子菜单并把标记放回它父行，Esc 在没有子菜单时关整块菜单并把键盘还给触发器，Enter/Space 运行高亮行，Tab 走的是同一扇关门的分支（它不自己搬焦点，之后焦点去哪由宿主的 tab 绑定决定）。字母键做 typeahead：以前缀匹配，从标记的下一行开始找，600ms 静音后重来，匹配不上则原地不动且前缀退到最后那个字母。
- 禁用行不绑 handler，也不给手型指针，而是换成参考件那条 `.dd .mi:disabled{cursor:not-allowed}`（`cursor_not_allowed()`）——指针停在上面要说得出"这行不能点"。它没有监听器也照样换得到光标：gpui 见到 `mouse_cursor` 就为那个盒子注册 hitbox，而注册过 hitbox 才谈得上设光标。换光标只在面板还能接指针时发生（退场中的行照常是普通箭头，与 `pointer-events: none` 同义）；标题、分隔线与 loading 行不换——它们不是候选项，参考件也只对 `:disabled` 那一条改。键盘与 typeahead 一律跳过禁用行；标题、分隔线、loading 行同样不在漫游集合里。
- 子菜单靠停留打开：指针在同一行上保持 80ms（`SUBMENU_DWELL`）才挂出子面板，未到点的行会主动请求下一帧；移开指针不等于关闭（参考件也只在换行时收起），所以指针能穿过 8px 间隙走进子面板。子面板只有一层：子项里再带 submenu 会被降级成普通命令行。
- 外部按下关菜单，但两类例外不算外部：本宿主自己的 surface（含子面板，逐帧重建的 bounds 表），以及触发器本身——那一下属于触发器的 click，它会自己完成关合，两处都关会让菜单合上又立刻被点开。
- 触发器上的点击会 `stop_propagation()`，所以坐在可点击卡片、列表行或工具栏按钮里的菜单不会顺带激活底下的东西。
- 激活一行的顺序：先跑调用方的 handler，再按行种类决定去向——submenu 行 arm 并保持打开，`.keep_open(true)` 保持打开，其余 dismiss（也就是走归还焦点的那条路）。

### 无障碍

- 面板：`Role::Menu` + `aria_label`。名字来自 `.label(...)`（右键面板也只有这里能给名字，因为它没有触发器可借）；子面板用打开它的那一行的 label。gpui 没有 `aria-labelledby`，所以不做"面板指向触发器"的连线，改由调用方给同名文字。
- 行：命令 `Role::MenuItem`、`.checked(...)` `Role::MenuItemCheckBox` + `aria_toggled`、`.radio(...)` `Role::MenuItemRadio` + `aria_selected`、submenu 行 `aria_expanded`、`.hotkey(...)` `aria_keyshortcuts`、高亮行 `aria_selected(true)` + `aria_active_descendant()`。radio 行是个例外：它先被写上高亮、再被自己的选中值覆盖，所以它的 `selected` 说的是值而不是"键盘现在在这"。
- 标题行与 loading 行是 `Role::Caption`；分隔线没有 role（gpui 0.3.9 没有 separator 角色），只是一条画出来的线。
- 触发器与右键区域的可访问名字由调用方负责：按钮自带 label，`with_trigger` 与区域元素要么有可见文字，要么自己 `.aria_label(...)`——树里能读到面板名与行名，这一点由无头测试钉住。
- `aria_disabled`、`aria_controls`、`aria_haspopup`、`aria_busy`、`aria_readonly` 在 gpui 0.3.9 的观察层里都不存在，因此禁用与"正在加载"只由行为与可见文字表达，不能声称平台已经把它们播报出去。没有任何一条断言证明 Windows UIA/屏幕阅读器会读成什么。

## Showcase 与验收

Windows Showcase 的 Dropdown Menu 页面按这份契约提供 17 段可交互样例（`examples/windows-showcase/src/pages/dropdown.rs`，`dropdown_content`）：

1. **Command menu**：点一下展开，选中一项后关闭并把焦点还给触发器；尾部箭头随展开转半圈，面板的箭头指着触发器中心。
2. **Alignment**：`align` 决定面板在交叉轴上的落点；指向箭头始终对准触发器中心，滑到离圆角太近时夹进 16px 安全区。
3. **Edge collision and flip**：默认先试下方，整块放不下就翻到上方；两侧都放不下取露出最多的一侧并夹回视口，四边各避让 8px。
4. **Submenus**：指针在一行上停留 80ms 才展开二级面板，移开即取消计时；二级面板的名字来自打开它的那一行。
5. **Checks and radios**：勾选与圆点由调用方状态决定，点击后面板不关闭；`indicator_side` 决定它们落在标签哪一侧。
6. **Long list**：面板到达 `max_height` 才在内部滚动；键盘漫游走到边界会把该行滚进视野。
7. **Truncation**：定宽面板让长标签以省略号收尾，而不是把面板推得更宽。
8. **Keyboard and accessibility**：↑↓ 漫游、Home/End 到首尾、Enter/Space 运行、Esc 关闭并归还焦点；字母键按前缀跳，重复同一字母在命中项之间轮转。
9. **Async loading**：打开时先给一行 spinner，数据到达后同一块面板换成结果（这里的等待是 spinner 转三圈）。
10. **Icons and avatars**：行首图标与 initials 头像占同一条基线；头像只说明这是哪个人，命令名仍由标签给出。
11. **Trigger seam**：`with_trigger` 接任意元素，开合状态交给它自己表达；面板的名字来自 `.label(...)` 而不是不可见的 id。
12. **Groups and rules**：分组标题不占键盘停靠点；`rule` 给标题加一条下划线，`separator` 是两组命令之间的一条线。
13. **Marks**：勾选（多选）与圆点（单选）共用一套指示位，两者都在面板里保持打开。
14. **Attached to an input**：挂在输入框尾部，`align = End` 与触发按钮右边缘对齐，输入框宽度不受影响。
15. **Context menu**：区域里点右键，面板角落落在指针上，没有指向箭头也没有间隙；落点仍由视口碰撞决定。
16. **Colour grid**：`columns` 只给可比的小集合用，方向键按列步进、到边界夹住而不是绕回；一行仍然只有一个停靠点。
17. **Split button**：左半运行默认命令，右半打开其余命令；两个目标两个名字，面板从右半浮出、箭头指向右半。

页面状态留在 Showcase 视图上（`dropdown_log`、`dropdown_sets`、`dropdown_open`、`dropdown_loading`、`dropdown_remote`、`dropdown_suffix`），每段样例都能反复操作并把结果写回可见文字。

`cargo test -p yue-component`（73 项，其中 dropdown 24 项）钉住的是这些纯派生值：

- `move_for`：down/up→`Line(±1)`、home/end→`First`/`Last`；left、right、enter、space、escape、tab、字母一律 `None`（左右属于子菜单或列步进，不在这一层）。
- `typeable`：只有裸的可打印键能进 typeahead——`"a"`、`"-"` 收到，`escape`/`space`/`enter`/`tab`/`home`/`f1` 这类具名键被拒（具名键的字符串长度就是判据，等价参考件的 `e.key.length === 1`）；Shift 照样打字，Ctrl/Alt/平台键打字无效。
- `move_index`：无标记时 `Line(1)` 从 0 进、`Line(-1)` 从末行进；单列在两端绕回；`First`/`Last` 落在漫游集首尾；三列网格 `Line` 整行步进（9 行里 0→3、8→5）并在边缘夹住（8→8、0→0），`Column(1)` 0→1 且 8 不动；只有一列时横箭头不产生任何移动。
- `typeahead`：无标记从第一行开始找，有标记则从标记的下一行开始，所以同一字母连按会在命中项之间轮转；唯一命中项绕一圈也能找到；大小写按拼写匹配。更长前缀收窄命中（`"co"`+`l`→color、`"co"`+`p`→copy）；前缀匹配不上时只剩最后那个字母（`"a"`+`x`→x，buffer 变 `"x"`；`"a"`+`q`→不动，buffer 变 `"q"`）；距上次击键超过 600ms（或从未击键）时前缀从头开始。
- `menu_column`：三态各钉一次——普通菜单（无可标记行）的 `Auto` 是 `Absent`，带勾选的 `Auto` 是 `Leading`，`Leading`/`Trailing` 无论内容都拿到自己那一侧（`Trailing` 连"有标记"的那种菜单也照样是 `Trailing`）。
- 箭头几何：`caret_along` 夹在 `[16, w-16]`，面板窄到放不下两个边距时塌到中心（20px 宽的两次都得到 10）；`caret_centre` 把中心放在朝向锚点那条边上（Bottom→y=0、Top→y=h、Right→x=0、Left→x=w）；`CARET_RADIUS` 等于 8px 方块的半对角线，`caret_points` 三点里中间那点是朝外的尖端（Bottom 时 `y = -R`），`caret_diamond` 的第四角是尖端的中心对称点——填得多边、描得只有朝外两边。
- `dwell_elapsed`：79ms 不算、80ms 算；`PanelMotion::dwell` 从 `hovered_at` 起算，40ms 时不成立、90ms 时成立。
- `MenuKind::wants_caret`：只有 Trigger 面板有指向箭头。
- `gap_for`：触发器 12px、子面板 8px、右键面板 0。`offset_for` 在这条间隙上再走那 4px：ramp 0 时触发器面板是 8px（贴着锚点）、0.5 是 10px、1 是 12px；子面板 4px→8px。方向是"张开时离开锚点"，写成 `+` 会让面板与行各走 4px 互相抵消。
- `PanelPath::channel`：根面板 `yue-dropdown-presence`，`Submenu(3)` 是 `yue-dropdown-submenu-3-presence`；同一行名在两块面板里的通道不同。
- `DropdownPlacement::side`/`DropdownAlign::align`：Auto→Bottom、Below→Bottom、Above→Top、Leading→Left、Trailing→Right；Start/Center/End 原样传给 Base。
- `YueMenuItem::key`：有 `.id(...)` 用它，否则用 label；分隔线与空 label 标题退化成 `row-{index}`。
- `is_roving`：命令与子菜单为 true，标题/分隔线/loading/禁用命令/禁用子菜单全 false；`is_checkable` 对那七行都是 false，`.checked(false)` 与 `.radio(true)` 各为 true，且 mark 分别是 `Check(false)`、`Radio(true)`；`.loading(...)` 的 kind 是 `Loading`。
- `PanelMotion::note_open`：清 active/hovered/prefix、`dwell` 归零、进入入场态、generation+1、`tint_seeded` 归零；开了 520ms 之后 `is_entering` 为 false，第二次打开拿到新 generation、重新进入入场态，并且重新欠一帧 tint 种子。
- `MenuMotion::arm`：`note_open` 会把 `armed` 清空、把标记放在根面板的首行、且两块面板都还没播过种子；arm 某行时子面板拿到唯一标记、根面板让出标记、generation 前进一格；重复 arm 同一行是 no-op（既不改标记也不重启入场）；换一行 arm 得到又一个 generation；`arm(None, None)` 只清 `armed`，根面板上调用方写回的标记保持不动。
- `MenuShape::bounds`：1000×800 视口里是 `(192, 320)`，即地板与天花板两条边；300px 宽视口里天花板收成 270（`min(320, 90%)`）；`.panel_width(px(400.))` 只换请求的那一条，天花板仍是 320——把它夹成死宽是面板布局的事，不是这个函数的。
- `curve`：`--ease-enter` 读成 `cubic-bezier(0, 0, 0.38, 0.9)`、`--ease-exit` 读成 `cubic-bezier(0.2, 0, 1, 0.9)`，不是这两个形状就 panic。

`cargo test -p yue-component --features headless-tests`（dropdown 那一份独立测试文件 23 项）在无头元素树上驱动真实交互，钉住的是这些行为：

- 从未打开过的菜单在树上一个面板都没有；触发器报 `expanded == Some(false)`。点开后面板以 `Role::Menu` 出现、名字是 `.label("选择操作")`、触发器改报 `expanded == Some(true)`；再点一次收起，且面板从树上消失（卸载，不是隐藏）；开着时点"重命名"上报一次，而这条命令会关掉自己来的那块面板。
- 点空白区域收起面板，面板里的 `Role::MenuItem` 一起归零。
- 三行都带 `Role::MenuItem` 与自己的名字；普通命令行的 `checked` 与 `expanded` 都是 `None`（没有状态可报，也没得展开）。
- Tab 到达触发器（菜单唯一的停靠点）；Enter 打开同一块面板；打开后面板 `focused == Some(true)`、触发器 `focused == Some(false)`；紧接的 down 被面板读到，高亮走到"复制"；Escape 收面板并把键盘还给触发器。
- 指针打开的面板同样把键盘交出去，Escape 与"点外面"两种关闭都归还给触发器。
- `with_trigger` 的自定义 div 点一下照样开出面板，面板里的行照常运行。
- 打开时标记落在第一行；down 跳过禁用行到达"复制"，再 down 绕回"重命名"，up 再到"复制"；Home/End 落在首尾，且 End 停在最后一个可用行而不是它后面的禁用行。
- 标题+分隔线+loading 那种菜单打开时落在第一条命令"移动"；树里有 2 个 `Role::Caption`（分组标题与 loading 行各一）；一步跨过 hairline 与 loading 到"删除"；再一步绕回命令，而不是落到家具上。
- Enter 运行键盘所在的那一行（不是指针最后看过的那行）并关闭面板。
- 点禁用行不上报、高亮不落在它上面、面板也不因此关闭；End 之后 Enter 只会运行键盘被允许到达的"复制"。
- 字母 typeahead：`a` 标到"Archive 归档"，`c` 标到"Copy 复制"，再 `c` 走到"Copy path 复制路径"，`z` 匹配不上时高亮原地不动。
- 标记行读得出主人的值："已完成"是 `MenuItemCheckBox` 且 `checked == Some(true)`、"归档"`checked == Some(false)`；"即时"是 `MenuItemRadio` 且 `selected == Some(true)`、"每天"`Some(false)`；点"归档"后它的 `checked` 跟着主人变 true 且面板保持打开；点"每天"后 `selected` 换到该行、原行变 false，主人持有的 `pace` 值成了"每天"。
- 子菜单：两次 down 停在"导出为"（`expanded == Some(false)`）；right 开出第二块面板，名字是这一行的 label，行的 `expanded` 变 true，子面板里的"PDF"同样是 `MenuItem` 且是唯一高亮行；down 在子面板内走到"Word"；left 折回子面板并把标记放回"导出为"；right 之后第一次 Escape 只被子菜单吃掉；Enter 在带子菜单的行上不运行任何命令，但它是打开的另一种方式；再按两次 Escape 才关到树里没有面板。
- 3 列网格：down 整行步进（甲→丁），right 同行步进（丁→戊→己）并在边缘夹住不再绕到下一行，up 沿同一列回到丙。
- 右键区域在右键之前树上没有面板；右键之后面板叫"文件"（这个名字只能由调用方给，因为没有触发器可借），且 `focused == Some(true)`；它的行照常运行、运行即关闭；同一区域再右键会再次打开。
- 一次什么都没改的连续两帧重画之后，高亮仍在那一行、`Role::MenuItem` 仍是同样三行——repaint 不会重新 stagger，也不会丢标记。
- 面板的盒子包住每一行：短菜单的面板正好是 192（`--menu-width` 地板），三行的左右边都落在面板左右边之内。这两条读的是 `ElementSnapshot::bounds()`，也就是 taffy 真实布局完的像素，不是样式声明——行溢出面板背景那个缺陷只有这一类断言看得见。
- 一行长到地板装不下时面板跟着长（>192）但停在天花板（≤320），把面板撑宽的那一行同样在面板里面。
- 快捷键贴右：`.hotkey(...)` 的那个 div 也挂了 `.id(...).test_support()`，所以它的盒子能从行节点的子代里读到。断言是"行的右边缘减去提示的右边缘 == 8px"（行自己那圈 `--space-8` 内缩），两条各测一次：全是普通命令的菜单，和 `.indicator_side(Trailing)` 的带标记菜单。三条变异实测过：把槽挪回提示之后，只有带标记那条读到 32px 并失败（普通菜单已经没有槽可挪）；把 `Absent` 换成 `Trailing` 而顺序不动，两条都照样通过——列在不在，提示都在同一条右边缘上，那 24px 只体现在标签多出来的省略号余量里，由 `menu_column` 那条派生断言去钉。两处一起退回旧实现（普通菜单也占一列、槽还挂在提示之后）才是用户看到的那 24px 空白。

不在测试范围内的：面板与行的透明度/位移中间帧、tick 与圆点的描边进度、spinner 真的转起来、指向箭头画出的像素、禁用行指针下换成的 not-allowed 光标（`ElementSnapshot` 不报 cursor，它只有 role/label/value/bounds/focus/disabled）、80ms dwell 与 600ms 前缀静音的墙钟计时（只测了纯函数那一半）、`max_height` 之后滚动条与 `scroll_to_item` 的落点、`columns` 的视觉排布、Tab 之后焦点的去向、除行高亮外的 `reduce_motion` 表现、Windows DPI 下的实际像素，都只在 Showcase 与实机检查。没有一条断言证明 Windows UIA/屏幕阅读器把这块菜单读成什么。

其余合约保持：

- 行的 handler 由调用方持有，组件不保存选择值；`checked`/`radio` 是调用方写回来的读数。
- 稳定身份来自 `.id(...)` 或 label（再退化成位置），不是每帧重算的下标；motion 通道 keyed 在宿主 ElementId 上，所以同页多份菜单互不串台。
- 面板不跟随容器填满宽度，也不当表格/表单容器；要持续选一个值仍用 YueSelect，要搜索大量选项用 Combobox。
- 运行 `cargo check --workspace`、`cargo test --workspace`、`cargo test -p yue-component --features headless-tests`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`，以及 `node tools/sync-yue-theme.mjs --design-system "D:\100_Projects\110_Daily\Yue-Design-System" --check`。

## 与草案的差异

草案的 API 与验收清单是契约草稿，实现时按依赖可达性、gpui 0.3.9 的观察层能力与编译结果做了这些收敛，语义意图保持不变：

- **不复用 Kit 的 `DropdownMenu`/`PopupMenu`**：`gpui_kit::component` 在本 crate 不可达（feature 门控），且与无头测试要的 `gpui-base/test-support` 同图编译失败。第一阶段的"轻量 facade"因此变成"复用 Base 的 `PopoverState`/`Positioner`/动效原语，自建行模型与面板绘制"。草案里"不得复制 `PopupMenu` 的焦点、键盘、子菜单和关闭逻辑"这条守住了——那四件事里没有一件是从 PopupMenu 抄的：焦点与开合是 Base 的，键盘阶梯和子菜单调度是 Yue 自己的。
- **`split button` 用 `YueDropdownSplitButton` 而不是 Kit 的 `DropdownButton`**：左半是调用方给的 `YueButton`，右半是只有箭头的 `YueButton::icon`（靠 `arrow_label` 命名），两块各绑各的 handler；面板从右半浮出，所以箭头指向右半。
- **`Action` 型菜单项与"快捷键提示来自 action context"没做**：gpui 0.3.9 没有从 action context 反查绑定文本的公开路径，`.hotkey(text)` 是调用方给的提示。因此提示可能与真实绑定失真，这条由调用方负责，组件不做校验。
- **`disabled` 不在面板层，而在触发器层**：`YueDropdownMenu` 没有 `.disabled(...)`（`YueDropdownSplitButton` 有，因为它同时压住左右两半）。按钮触发器一旦禁用，gpui 的 button 就同时吞掉指针与键盘激活，不存在第二个可能说反话的 flag。
- **`trigger` 只有 click（右键另有一个组件）**：草案的 hover/focus 触发没实现。hover 只用于子菜单的 80ms 停留预备，根面板不受指针掠过影响。`YueContextMenu` 的区域只绑 `on_mouse_down(Right)`，草案提到的键盘 Menu/Application 键没有接——需要键盘可发现时按草案的说法另配一个可见的"更多"按钮。
- **没有 panel 上下插槽、任意自定义行、面板内搜索/输入**：行的种类固定为命令、标题、分隔线、loading、子菜单五种，勾选与圆点是行上的 mark 而非行种类。要做任意内容的面板请改用 Popover 或专用组件——这与草案"不能据此承诺 top/bottom 插槽与复杂控件的键盘语义"是同一条立场。
- **`columns()` 实现了，尽管草案不鼓励**：它只服务参考件那种可比的小集合（色板、图标格），默认 1；一列时左右箭头留给子菜单。它不是 `maxColumnWidth` 那类网页排版属性，方向键语义也随之改变（列步进 + 夹边），所以放在 Showcase 的 Colour grid 一段而不是默认路径。
- **`Anchor` 被 `DropdownPlacement`/`DropdownAlign` 取代**：Base 的 `Positioner::side` 需要"哪条边 + 交叉轴怎么对齐"，`Anchor` 的四角表达不了 flip 之后的对齐轴。`Auto` 显式请求下边，是为了绕开 Base "无偏好即先试上方"的默认。
- **`max_height`/`panel_width` 是像素参数，不是 PopupProps 透传**：宽度上限固定 `min(320px, 90% 视口宽)`，高度上限默认 `min(视口高/2, 450px)`，都不开放任意值组合。
- **无障碍只做到观察层能表达的那些**：`aria-labelledby`/`aria-controls`/`aria-haspopup`/`aria-disabled`/`aria-busy`/`aria-readonly` 在 gpui 0.3.9 都不存在，所以面板名走 `.label(...)`、子面板名走行 label，禁用与加载只由行为与可见文字表达。草案"高亮即焦点"这条也换了实现：焦点停在面板容器上，行用 `aria_selected` + `aria_active_descendant` 报告，避免每移动一行就搬一次真实焦点。
- **子菜单固定一层**：子项里再带 submenu 会被降级成普通命令行。草案说"子菜单最多一层作为默认建议"，实现把它写成了硬边界，因为第三层需要的是 Dialog/侧栏而不是更深的级联。
- **`YueMenuItem` 不持状态也不自己关菜单**：与草案一致，但补了一条草案没有的规则——submenu 行的激活必然保持打开（它的"运行"就是展开），`.keep_open(true)` 只给命令行用。
- **面板跟着滚动中的触发器会滞后一帧**：锚点在这一帧开始时读取，参考件是 DOM 同步测量，gpui 里 deferred surface 拿不到同帧的布局结果。实测表现为长页面临边展开时的一次轻微错位，只在 Showcase 可见。
- **关掉的面板完全不在树上**：草案假设 Base 的 popover 会自己处理"看不见但还在"的状态；实测 `Presence` 会在缺席的第一帧交出一个 progress 0 的幽灵 surface，它压在触发器上抢走 MouseDown 的焦点，导致 Escape 之后键盘归还给一个已卸载的 handle。`popup()` 因此在 `!present && progress <= 0.0` 时直接返回 `None`，并由"指针打开的面板也把键盘还回去"那条测试钉住。
- **hover 高亮与键盘高亮写进同一个标记**：草案要求两者都"可读"且不与 checked 混同，实现更进一步——它们落在同一行的 `active` 上（参考件的 `mouseover` 也是这么做的），所以任何时刻只有一行拿到 `aria_selected`；带勾选的行用 `aria_toggled`、带圆点的行用 `aria_selected` 报自己的值，与高亮走的是不同的属性（radio 行因此不额外报出它的高亮态），指示位的视觉也和 hover 不一样。

本阶段验证：`cargo test --workspace`、`cargo test -p yue-component --features headless-tests`（其中 Dropdown 23 项）、常规 workspace Clippy、fmt 和 token 同步检查通过。当时带 `headless-tests` 的 workspace Clippy 因上游 styled component 的观测别名编译失败。后续 [08 接入与架构完善](08-integration-hardening.md) 已将 Showcase 改为 Base-only，本工作区现在支持整体 `--features headless-tests`；未修改上游依赖源码。Dropdown 实现也已拆至 `src/dropdown/` 的行为、定位、绘制模块。

### 300ms 调整交付记录

- 来源：Fluent UI `curveEasyEase`；只采用曲线，不照搬其余命令菜单 API。调整属于 GPUI 菜单的视觉参数，无公开 Rust API 变更，无新增延迟，系统 reduced motion 仍有效。
- 源文件：设计系统 `primitives/motion.css` 与 `component-tokens/menu.css`；三个新名称同步登记至 token vocabulary 和 package-only register。README 说明 GPUI 消费者，库存与使用报告及文档计数按源码重新生成；未改 Checkbox/Radio/Switch token。
- 验证：旧 400ms 测试先通过，新 300ms 比例断言在旧实现上失败，切换实现后 Dropdown `23 passed; 0 failed`。`cargo test --workspace`、`cargo test -p yue-component --features headless-tests`、常规 workspace Clippy、fmt 和 token 同步检查通过。
- 设计系统检查：`corepack pnpm verify:visual` 输出 `RESULT: PASS`（该命令刷新设计系统测试截图）；三个新增名称的 grammar 检查通过。`corepack pnpm audit:tokens` 仍为 FAIL：其余 Checkbox/Radio/Switch 等既有词汇登记不齐。`corepack pnpm verify:all` 在 test 阶段停止；登记新增名称并刷新库存后，重跑 test 为 `7 failed | 689 passed (696)`，剩余涉及既有 token 的 prototype parity、grammar 和 124 个未消费/未文档化 token，不声称设计系统全量通过。未修改或弱化失败断言，后续应单独对齐这些 token 登记和消费文档。
- 预览：运行中的 `target/debug/windows-showcase.exe` 被 Windows 锁定，常规 build 无法替换；使用 `cargo rustc -p windows-showcase --bin windows-showcase -- -o .../windows-showcase-hover300.exe` 成功生成独立预览。当前运行进程仍是旧程序，需要打开新预览才能比较本轮观感。
- 平台边界：本轮没有重新做屏幕 GetPixel 或 Windows GPU 帧率测量；比例证据来自实际菜单提交的绘制命令。

## Popup 基础层迁移（2026-11-01）

已按 [Popup 基础浮层设计](12-popup.md) 完成内部迁移。通用锚点、定位、箭头、Presence / deferred 挂载、退出卸载和嵌套区域登记由 `src/popup/` 承接，保留 MenuHost、MenuMotion、菜单行、导航、命令和 Base PopoverState 的焦点生命周期。

沿用现有淡入淡出、4px 位移和菜单 token；行 stagger、300ms 高亮曲线仍在 Dropdown。使用 Owner 关闭路由，Root runtime 不重复处理菜单的 Escape / outside press，content 继续保留在原 dispatch 子树。

`MenuAnchor` 的前帧 bounds 与 surfaces Vec 已删除。side 和 submenu 的 live anchor 在 prepaint 读取本帧测量，通过仓库内维护的 gpui-base Positioner 补丁复用同一个求解器；来源与验证见 Popup 第 3.1、14 节。公共 Dropdown API 保持，完整视觉包络的边缘约束比旧矩形 clamp 更保守。

迁移前 Dropdown 23 项基线通过；迁移后验证以 Popup 第 14 节为准。Tooltip 是下一阶段，尚未迁移。

## 小窗口与父级快捷键修复（2026-11-03）

SnapClip 贴图窗口接入中发现并修复两项基础问题：

- 菜单行滚动高度除了窗口比例上限，还要扣除同一 PopupHost 位置求解使用的阴影/箭头/边缘预留，以及 panel padding/border。否则在 240px 或 144px 高的小窗口内，即使菜单行本身不高，完整视觉包络仍可能 NoSpace 而不绘制。`PopupHost::content_size()` 统一提供该尺寸上限。
- GPUI 先解析 Action keybinding，再调用 raw keydown。菜单 Root 现在使用 `YueDropdownMenu` key context，并屏蔽 Escape、Tab/Shift+Tab、Enter/Space、方向键、Home/End 的父级动作，让现有菜单键盘逻辑先消费它们。Escape 先 dismiss 菜单并恢复焦点，不能穿透成窗口的关闭命令。

初始化接在现有 `yue_component::init`，调用方不需要增加额外初始化。没有改变菜单 token、时长、行密度或公开 builder。

回归：新增生产小窗口测试覆盖 240/144 高、12 行滚动菜单和父级 Escape 隔离，1 项通过；原 Dropdown 23 项与 Popup 相关 34 项通过。无头几何及 action 断言不能替代真实桌面字体、DPI 和动画视觉验收。
