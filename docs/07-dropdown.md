# 07 Yue Dropdown Menu

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
