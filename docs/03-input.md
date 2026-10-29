# 03 YueInput

## 目标

实现 Yue Design System 的单行输入框，并把 Yue 的外观映射到 GPUI Kit 0.8.0。GPUI Base 继续负责编辑行为，Yue Component 负责呈现，因此键盘、焦点、光标、选区、撤销、输入法和文本状态只有一个所有者。

## 状态与所有权

调用方创建并持有 `Entity<InputState>`，可读取或更新其中的值；`YueInput` 是 `RenderOnce` 外观包装器。组件每帧将 disabled、readonly、密码遮罩、对齐和文字颜色投影给 Base 状态。校验状态是呈现状态，不隐式触发验证。

```rust
let state = cx.new(|cx| {
    InputState::new(window, cx).placeholder("name@example.com")
});

YueInput::new("email", theme, &state)
    .size(InputSize::Medium)
    .status(InputStatus::Normal)
    .clearable(true)
```

## v1 范围

- Small、Medium、Large；Normal、Success、Warning、Error。
- Placeholder、初始值由 `InputState` 提供；左、中、右文本对齐。
- Disabled 与 Readonly；Readonly 保持可读、可聚焦，Disabled 不可编辑。
- Leading/trailing 前缀、后缀或图标；clearable；密码遮罩与显隐。
- Borderless 与 block 宽度选项。
- 所有颜色、尺寸、字体、间距、边框、圆角和焦点样式读取已生成的 Yue token。

Textarea、自动完成、数字输入、标签/帮助/错误消息编排、受控值回调、表单提交语义和异步验证不属于 v1。v1 里调用方用独立 Label 与说明文字组合字段；字段编排与表单提交语义自 V2、V3 起分别由 `YueField` 和 `YueForm` 承担，但不会在 Input 内复制 Form 的职责。

输入框宽度不由组件猜测。调用 `.width(gpui_kit::rems(20.0))` 指定长度，或用 `.block()` 填满父容器；两者同时设置时显式宽度优先。宽度参数属于布局，不伪装成 Yue 视觉 token。

## V2 已实现

### YueTextarea

`YueTextarea` 接受 `Entity<TextareaState>`，复用 Base 的多行编辑、选区、IME、撤销及自动增长。调用方在创建 state 时选择固定行数或自动增长范围，并通过 `width(...)`、`height(...)` 或 `block()` 约束布局。

```rust
let state = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(3, 6)
        .placeholder("Description")
});

YueTextarea::new("description", theme, &state)
    .width(gpui_kit::rems(32.0));
```

### YueNumberInput

`YueNumberInput` 接受 `Entity<InputState>` 并使用 GPUI Base spinbutton，提供上下键、步进按钮、数值可访问语义和边界行为。步长及范围由 state 初始化，不由外观组件另存一份数值：

```rust
let state = cx.new(|cx| {
    InputState::new(window, cx)
        .default_value("12")
        .step(1.0)
        .min(0.0)
        .max(100.0)
});

YueNumberInput::new("quantity", theme, &state)
    .width(gpui_kit::rems(12.0));
```

### YueField

`YueField` 编排 label、description、required 标记和显式校验 message。它不拥有输入值、不自动验证；业务层决定何时显示 `Normal`、`Success`、`Warning` 或 `Error` message，再渲染对应状态。

控件以 builder 交给 field：`input(...)`、`textarea(...)`、`number_input(...)`。field 因此能把 label 写成控件的可访问名称、把 required 与 message 拼进描述文案，并把 message 的状态同步给控件边框。`child(...)` 仍能放 field 无法触及的任意元素，代价是它拿不到 label、描述和状态。

## V3 已实现

### 可访问性节点

GPUI Base 把编辑器渲染成没有名字的通用容器，会被无障碍树剪掉，所以 Yue 的外层 frame 才是控件对辅助技术暴露的节点：frame 上接 `track_focus`、`role`、`aria_label`、`aria_description`、`aria_placeholder`、`aria_value`，并处理 `Focus` 与 `SetValue` 两个 a11y 动作（写入走 Base state 的 `set_value`，不产生第二份文本）。`YueNumberInput` 例外——Base 自己发布 SpinButton 节点，label 直接送到那里。

- `accessibility_label` 显式命名控件；放在 `YueField` 内时由 field 用 label 注入，不用调用方重复写。
- `accessibility_description` 写辅助技术读到的描述；field 会把 required 标记和当前 message 一起并进去。
- 密码框只发布 `Role::PasswordInput`，不发布 `aria_value`，遮罩值不会进入无障碍树。

### 长度与计数

`max_length(n)` 与 `show_counter(true)` 在单行和多行控件上都有。GPUI Base 没有 `max_length`：它的 `validate`、`pattern` 钩子只看即将插入的那一段文本，拦不住一次超长的粘贴或输入法提交。Yue 因此在渲染时把超出部分截断，截断用 `replace_all` 而不是 `set_value`，撤销历史仍归 Base；光标位置按字符数换算后保留在用户原先工作的地方。

计数按 `char` 而不是字素簇，emoji 与组合字符会按码点计数。输入过滤、掩码与正则校验继续是 Base state 构造期的 `mask_pattern`、`validate`、`pattern` 调用，Yue 不再复制一套。

### 语义类型

`input_type(InputType::{Text, Search, Email, Url})` 决定节点角色。Windows 没有原生的文本内容类型通道，所以选择只体现在角色上，不带来触屏键盘或密码管理器的差异；密码遮罩仍由 `password(true)` 控制。

### YueForm

`YueForm` 持有规则和 checked 标记，不持有文本：每个字段仍是调用方自己的 `Entity<InputState>`，编辑、IME、光标行为继续归 Base。

- `require(id, &state, message)` 是非空规则（trim 后判断）；`rule(id, &state, closure)` 接受任意 `Fn(&str, &App) -> Option<SharedString>`。
- `submit(window, cx)` 校验全部字段、揭示结果、把焦点交给第一个未通过的字段，并返回是否可以提交；`is_valid(cx)` 只判断不揭示。
- `error(id, cx)` 在首次 `submit` 之前恒为 `None`，之后每次都对当前值重新求值，所以改正后的字段无需再次提交就失去错误。
- `attach(&state, window, cx)` 订阅 Base 的 `InputEvent::PressEnter`，回车即走同一套校验；单行控件天然发出该事件，多行控件需要 state 以 `submit_on_enter(true)` 构建。
- `on_submit(callback)` 报告校验结果；回调在 form 被借用期间运行，可以更新别的 entity，但不能再提交这个 form。
- `reset(cx)` 收起已展示的错误。

`YueField::bind(&form, id)` 把字段接到规则上：规则失败时边框状态与错误文案同时出现，规则成功时两者同时消失。未 `bind` 的字段永远不显示 form 的错误。

## 公共 API

```rust
YueInput::new(id, theme, &state)
    .size(InputSize::Medium)
    .status(InputStatus::Error)
    .text_align(InputTextAlign::Left)
    .input_type(InputType::Email)
    .placeholder("Search")
    .leading_icon(outline::Search)
    .trailing("kg")
    .clearable(true)
    .password(true)
    .max_length(24)
    .show_counter(true)
    .accessibility_label("邮箱")
    .accessibility_description("用于接收构建通知")
    .disabled(false)
    .readonly(false)
    .bordered(true)
    .width(gpui_kit::rems(20.0))
```

字段与表单的组合：

```rust
let form = cx.new(|_| YueForm::new());
form.update(cx, |form, cx| {
    form.require("name", &name, "请填写项目名称");
    form.rule("email", &email, |value, _| is_complete_address(value));
    form.attach(&name, window, cx);
});

YueField::new(theme)
    .label("项目名称")
    .required(true)
    .bind(&form, "name")
    .input(YueInput::new("form-name", theme, &name).max_length(24).show_counter(true))
```

`placeholder` 會投影到 Base state，構建時和运行時均可设置；密码可切换显隐。清除操作调用 Base state 的 `clean`，不会产生第二份文本值。插槽接收 `IntoElement`，组件不限制调用方的图标库。

## Base 能力边界

GPUI 与 Base 0.8.0 不提供、Yue 也不伪造的部分：

- 没有 `aria-required`、`aria-invalid` 槽位，必填与失效状态靠可见的星号和并入描述的文案传达。
- `aria_disabled` 的 setter 在 gpui-base 内部私有，Yue 无法复用，禁用/只读同样无法向辅助技术播报。
- Windows 无原生文本内容类型通道；`InputType` 只改变节点角色。
- Base 无 `max_length`，截断发生在 Yue 的渲染路径上。
- `YueNumberInput` 的 SpinButton 节点有 label 槽位但没有描述槽位，field 的描述不会并入它。
- Base 在输入框上下文里把 `tab` 绑到 `IndentInline`、`shift-tab` 绑到 `OutdentInline`，Tab 不遍历字段；字段间移动焦点靠点击或 `InputState::focus`，回车提交靠 `YueForm::attach`。

## Token 映射

| 呈现 | Yue token |
| --- | --- |
| 背景/文字/placeholder | `--input-background`、`--input-background-disabled`、`--input-background-readonly`、`--input-color`、`--input-color-disabled`、`--input-placeholder-color` |
| 边框 | `--input-border-color`、`--input-border-color-hover`、`--input-border-color-focus`、`--input-border-color-invalid`、`--input-border-color-disabled` |
| 尺寸/排版 | `--input-height-*`、`--input-padding-inline-*`、`--input-padding-block`、`--input-font-size-*`、`--input-gap`、`--input-icon-size-*` |
| 形状/焦点 | `--input-border-width`、`--input-border-radius`、`--input-border-color-focus` |
| 附属控件 | `--input-affix-color`、`--input-icon-size-*`、`--input-clear-*` |

状态优先级：Disabled 边框最高；Success、Warning、Error 的状态边框在聚焦时保留。Normal 聚焦时使用焦点边框，Normal hover 使用 hover 边框。禁用状态不会响应 hover。

## Showcase

Input 页面使用纵向文档流展示基础、尺寸、状态、禁用/只读、密码与清除、窄槽位、长度限制、语义类型、锁定组合、前后缀/图标、对齐、多行、数字和字段编排。窄槽位把 8/11/14rem 的宽度与前后缀、清除、显隐按钮叠在一起；锁定组合把禁用/只读与同一批附属控件叠在一起。输入框示例采用显式宽度，展示宽度不依赖白色示例容器的尺寸。

Form 页面把两个绑到 `YueForm` 的字段和一个无规则的备注字段放在一起，保存与清除提示按钮走同一套校验；回车提交由 `YueForm::attach` 提供。

## 无头测试

```powershell
cargo test -p yue-component --features headless-tests
cargo clippy -p yue-component --features headless-tests --all-targets -- -D warnings
```

`crates/yue-component/tests/input.rs` 用 gpui-base 的元素观测层查询渲染后的节点树。该 feature 是可选的，因为 `gpui-kit/test-support` 会连带激活 GPUI Fast 后端，其固定依赖与本工作区锁定的 GPUI 后端冲突，且 gpui-base 的观测别名会让 `gpui-component` 0.8.0 的 `popup_menu.rs` 编译失败；默认构建与 `cargo test --workspace` 都不会开启它。

## 验收

`cargo test -p yue-component`（45 项）与 `cargo test -p yue-component --features headless-tests`（6 项无头）通过；`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check` 通过。无头测试覆盖的是这些：

- 各控件的可访问节点：单行为 `TextInput`，`InputType` 改为 `SearchInput`/`EmailInput`/`UrlInput`，密码为 `PasswordInput` 且不发布值，多行为 `MultilineTextInput`，数字控件保留 Base 的 `SpinButton` 节点；label 能被查到，即 frame 确实是被命名的那个节点。
- 超长值在渲染前截断：默认值、`set_value` 写入都被裁到 `max_length`，节点播报的值与 state 一致；逐字符输入不会突破上限，超长的整段写入保留前缀且光标停在用户原来的位置（越界的光标被夹到上限）。
- 8rem 窄槽位：frame 可见、宽度按逻辑像素测量为 128px 上下，前后缀与清除按钮不改动值。
- `YueForm`：首次提交前不显示任何错误；提交返回 false 并把焦点交给第一个未通过字段；写回合法值后无需再次提交错误即消失；第二次提交聚焦下一个未通过字段；全部合法时提交返回 true 且 `is_valid` 为真；`reset` 收起结果。
- 已 `attach` 的字段按回车走同一条校验路径，另一个字段的值不受影响。

不在测试范围内的：读屏软件的真实播报、Windows 显示缩放（DPI）下的排版（无头断言只看逻辑像素）、IME 组合过程、密码显隐与清除按钮的点击（由 Base 状态机承担）、以及 Tab 字段遍历（Base 把 Tab 用作缩进，见"Base 能力边界"）。缩放和读屏需要在 Windows Showcase 里人工确认。

其余合约保持：

- 单行文本输入、placeholder 与默认值由 Base 正确呈现。
- Disabled、Readonly、masked 与 focus 行为遵循 Base 合约。
- clearable 和密码显隐实际调用 Base state 方法。
- 各尺寸与所有状态解析完整 token；四种 Yue Theme profile 均可呈现。
- 未设置 width 或 block 时不强制铺满父级；width 与 block 均由调用方显式选择。
- YueTextarea 保留 Base 多行编辑和自动增长；YueNumberInput 保留 Base 步进、范围和 SpinButton 语义。
- YueField 只呈现调用方或绑定规则给出的说明和验证结果，不执行隐式验证。
- prefix/suffix 可与清除/密码控件组合，不挤压编辑区域或改变输入高度。
