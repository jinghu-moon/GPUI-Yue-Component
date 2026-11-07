# YueKbd 与 YueShortcut 快捷键展示组件设计

状态：V1 已实现（2026-11-07），已吸收参考页的组合场景、局部按键反馈、逐键状态、过渡、分隔符与规格对照。真实 Windows 视觉与 Narrator 验收由用户负责。

本文定义两个用于展示快捷键的轻量组件：`YueKbd` 展示单个键帽，`YueShortcut` 将键帽组成组合、序列或备选快捷键。它们服务于菜单、按钮、Tooltip、帮助文案和设置页等提示场景，不拥有快捷键命令，也不参与事件监听。

## 1. 结论与目标

采用 **Windows-only 的展示层 + GPUI/Base 的行为层**：

```text
功能 owner / GPUI Action 与 KeyBinding
                │ 真实绑定、事件处理、焦点和命令
                ▼
YueShortcut ── YueKbd ── 文本 / 方向字符
                │ 仅呈现调用方提供的快捷键事实
                ▼
菜单、按钮、Tooltip、正文或设置行
```

目标是让同一套键位标签在不同场景保持一致的尺寸、间距、顺序、状态和无障碍名称，同时不复制 GPUI 的 Action、KeyContext 或输入路由。

项目当前只面向 Windows，因此首版不设计 macOS/Linux 映射、`mod` 平台别名、平台自动检测、物理键盘布局识别、输入法层字符映射或键盘图例。文档中的“快捷键”指 Windows 用户界面中展示的逻辑组合，例如 `Ctrl + Shift + P`。

## 2. 参考与取舍

设计参考 `refer/Shortcut 快捷键 · GPUI 组件 Demo.html`，并复用本仓库已有的 Tabler SVG 和 Yue 组件约定。

| 参考能力 | Yue 设计 | 首版边界 |
| --- | --- | --- |
| 单键键帽、组合、序列、备选 | `YueKbd` 与 `YueShortcut` 两层组合 | 不把所有输入解析和命令注册塞进一个组件 |
| 小/中/大三档 | 20 / 24 / 28px 键帽高度 | 高度、字体和间距由 token/组件统一拥有 |
| raised、flat、ghost、inverse | `KbdVariant` | inverse 只是表面变体，不改变语义或行为 |
| default、pressed、active、disabled | `KbdState` | 状态由宿主显式传入；组件不监听键盘 |
| `+` 组合、空格序列、备选分隔符 | 采用明确的 Windows 语法 | 不引入跨平台 `mod` 语义 |
| 特殊键标签 | Windows 文字与方向字符 | 按用户决定不使用图标，包括 Win |
| `aria-label`、`aria-keyshortcuts` | 视觉容器提供可读名称；触发元素由 owner 设置 ARIA shortcut | 不把可访问属性错误地挂到装饰键帽上 |

平台切换、命令注册和完整设置工作流不是这两个展示组件的职责。录制能力已独立实现为 [YueShortcutRecorder](31-shortcut-recorder.md)，复用本文的键帽与解析，不改变展示层的边界。

## 3. 职责边界

### 3.1 `YueKbd` 负责

- 展示一个逻辑键位的文本标签和键帽表面。
- 统一三档高度、字体、内边距、圆角、边框和底边厚度。
- 呈现 `Default`、`Pressed`、`Active`、`Disabled` 四种明确状态。
- 由组合容器或真实触发元素提供完整可访问名称。
- 保持内容在键帽内居中；长标签按组件规则收缩或由调用方拆分，不改变键帽语义。

### 3.2 `YueShortcut` 负责

- 将 `YueKbd` 按 chord、then、or 三种关系排列。
- 在组合内部显示 `+`，在先后序列之间显示 `然后` 或紧凑连接符，在备选项之间显示 `或`。
- 保持整个快捷键不换行、不压缩键帽；可由外层容器决定裁剪、滚动或省略。
- 从结构化输入生成稳定的可访问名称，并向调用方提供用于 `aria-keyshortcuts` 的规范化值（若实现该 accessor）。
- 支持菜单行、按钮内、Tooltip 和正文行内的不同表面变体。

### 3.3 两者都不负责

- 不注册或取消注册 GPUI Action、KeyBinding、KeyContext 或窗口快捷键。
- 不监听 `keydown`/`keyup`，不判断冲突，不执行命令，不修改业务状态。
- 不录制快捷键、不提供设置页、不做冲突替换或恢复默认。
- 不推断调用方的真实绑定；展示内容必须由功能 owner 与绑定表保持一致。
- 不维护平台枚举、自动检测或物理键盘布局映射。

行为层的推荐关系是：GPUI 的绑定触发功能 owner，owner 在需要时把同一个规范化快捷键传给 `YueShortcut`；展示组件不反向查询绑定表。

## 4. 视觉与布局

### 4.1 键帽 anatomy

```text
raised / flat / inverse
┌─────────────────┐
│      Ctrl       │  ← 文本或方向字符
└─────────────────┘
        └─ 立体变体额外保留底边
```

- 键帽是一个不可交互的 inline-flex 视觉单元，内容水平、垂直居中。
- 最小宽度等于对应档位高度，单字符和方向键呈正方形；文字键通过最小宽度加水平内边距自然伸展。
- 严格遵循参考 HTML 第 99–106 行：四周 1px 边框、底边 2px、圆角 4px；底边加厚是厚度的唯一视觉线索，不添加投影。Pressed 下沉 1px，底边减为 1px；长度随 rem 缩放，外部布局高度保持不变。
- `YueShortcut` 按 Alternative → Sequence → Chord → Kbd 分层排列；组合内间距 Small 为 2px、Medium/Large 为 4px，Ghost 为 1px；序列与备选之间为 8px。
- `+`、`然后`、`或` 是次级文本连接符，不渲染成键帽；shortcut 本身不额外加边框。`ShortcutSeparator::None` 仅隐藏组合内的 `+`，不移除序列或备选边界。
- Raised、Flat、Inverse 保留相同的逻辑边框位置；Ghost 按参考移除最小宽度，水平内边距缩为 1px，因此会更紧凑。

参考页浅色键帽为白色、正文为 `#1f1f1f`、控制边框为 `#a3a3a3`；深色分别为 `#1f1f1f`、`#f5f5f5`、`#737373`。`kbd/appearance.rs` 集中定义这套参考色板，避免用通用分割线色替代控制边框导致立体感丢失。Flat 与按下状态同样使用参考页的浅色/深色配对。

`+` 的字号跟随档位；`然后` 和 `或` 固定为 12px。方向键显示箭头，标点键显示实际字符，`Delete/PageUp/PageDown` 显示 `Del/PgUp/PgDn`；规范化值和可访问名称仍保留逻辑键名。

### 4.2 尺寸规格

数值是设计验收基线，实际实现通过 Yue token 和 rem 适配，不把物理像素写死到消费方。

| `KbdSize` | 键帽高度 | 基础字号 | 水平内边距 | 适用场景 |
| --- | ---: | ---: | ---: | --- |
| `Small` | 20px | 11px | 4px | 菜单、工具栏、紧凑 Tooltip |
| `Medium`（默认） | 24px | 12px | 6px | 正文提示、普通按钮和设置行 |
| `Large` | 28px | 14px | 8px | 帮助页、快捷键速查和醒目演示 |

字体使用项目既有 UI 字体和字重，不另引 Keycap 字体。键帽内部行高为字号的 1 倍并垂直居中；英文缩写、数字和特殊键名不得因字体 fallback 造成单独的垂直偏移。

`Win` / `Windows` 键帽统一显示文本 `Win`，不使用 Windows 图标。完整键盘中的左右 Win 键与 `Win+E` 组合均使用相同渲染；组合的可访问名称保留 `Win`，`aria-keyshortcuts` 仍输出 `Meta`。

### 4.3 变体与状态

| 类型 | 语义 | 默认表现 |
| --- | --- | --- |
| `Raised` | 标准键帽 | 表面、边框和加厚底边 |
| `Flat` | 低强调键帽 | 中性底色、透明边框，保留边框布局空间 |
| `Ghost` | 正文行内提示 | 无边框、次要文字色、最小额外空间 |
| `Inverse` | 反色表面上的提示 | 透明底色、继承宿主文字色，边框为该颜色 40% 不透明度，连接符为 70% |

| `KbdState` | 视觉变化 | 来源 |
| --- | --- | --- |
| `Default` | 正常表面 | 默认值 |
| `Pressed` | 下沉 1px、1px 底边、蓝色表面与边框 | owner 的即时反馈 |
| `Active` | 深色填充与反色文字，不下沉 | owner 的受控状态 |
| `Disabled` | 保留原变体，键帽整体透明度 0.4 | owner 的可用性状态 |

`KbdState` 是互斥状态；owner 选择当前状态，组件不自行从窗口输入推导。`YueShortcut::key_state(key, state)` 可覆盖同名逻辑键的每次出现，接受与解析器相同的键名和别名；重复设置以最后一次为准。整组 `Disabled` 优先于逐键状态。禁用只表达展示状态；因为组件本身不可交互，不产生新的禁用事件。

### 4.4 过渡与身份

`YueShortcut` 用自身 ID 与结构路径为每个键帽生成稳定的动效身份。独立 `YueKbd` 可用 `.id(...)` 开启相同过渡；无 ID 的静态键帽直接呈现目标状态。背景、边框色和下沉位置使用参考 CSS 的 100ms / ease 过渡；底边宽度、文字色与禁用透明度直接切换。复用 Base `transition`，响应减少动态效果设置，不注册按键监听或自建计时器。ID 改变代表新的视觉实例，首次出现直接呈现当前值。

## 5. Windows 快捷键语法

### 5.1 展示语法

静态字符串入口可采用下列候选语法；结构化 builder 应生成相同结果。

| 形式 | 示例 | 含义 |
| --- | --- | --- |
| 单键 | `F5`、`Esc`、`Enter` | 一个 `YueKbd` |
| 组合 chord | `Ctrl+Shift+P` | 同时按下；修饰键先于主键 |
| 先后序列 | `G I` | 先按 `G`，再按 `I` |
| 备选 | `Ctrl+Y` 或 `Ctrl+Shift+Z` | 同一动作的两个可选组合 |

输入可接受大小写差异，但渲染应使用统一标签：`Ctrl`、`Alt`、`Shift`、`Win`、`Esc`、`Enter`、`Tab`、`Backspace`、`Delete`、`Home`、`End`、`PageUp`、`PageDown`、`Left`、`Right`、`Up`、`Down` 以及 `A`–`Z`、`0`–`9`、`F1`–`F24` 等普通键。

备选分隔符在规范化字符串中使用竖线：

```text
Ctrl+Y | Ctrl+Shift+Z
```

`mod`、`cmd`、`option`、`meta` 等跨平台别名不在 Windows-only 首版语法中。需要显示加号键时，结构化输入应使用明确的 `Plus` 键值，避免把 `Ctrl++` 解析成空 token。未知或空 token 不应静默渲染为一个空键帽；应返回类型化错误或显示调用方提供的错误占位。

修饰键顺序固定为 `Ctrl`、`Alt`、`Shift`、`Win`，再接主键。调用方输入顺序不影响渲染顺序；重复修饰键、同一 chord 中缺少主键的情况应在构造/解析阶段拒绝。

### 5.2 公共 API

公共类型从 `yue_component` crate 根导出。静态快捷键可从字符串解析，也可复用已校验的 `ShortcutSpec`：

```rust
YueKbd::new("Ctrl")
    .size(KbdSize::Medium)
    .variant(KbdVariant::Raised)
    .state(KbdState::Default);

let spec = ShortcutSpec::parse("Ctrl+Shift+P")?;
YueShortcut::new("open-command-palette-shortcut", spec)
    .variant(KbdVariant::Ghost);

YueShortcut::parse("history-shortcut", "G I | Ctrl+Y")?;
```

`YueShortcut::parse(id, value)` 是静态快捷键的便利入口；需要复用或检查规范值时用 `ShortcutSpec::parse`、`display_label()` 和 `aria_keyshortcuts_value()`。Shortcut 需要调用方提供稳定 `ElementId`，用于组级 `Image` 语义与可访问名称。键帽文本本身可读，不额外加入焦点或事件行为。`aria_keyshortcuts_value()` 只对全部由单个 chord 组成的 alternatives 返回字符串；含 then 序列时返回 `None`，避免生成错误的 ARIA 语义。

解析后的值区分 alternatives、sequences、chords 与 key：

```text
Shortcut
├── Alternative*
│   └── Chord*
│       └── Key
```

字符串解析适合静态内容；宿主持有规范化值时可保留并复用 `ShortcutSpec`，避免反复解析。设计不包含录入器。

## 6. 特殊键标签

按用户决定，当前统一使用文本与字符：`Ctrl`、`Alt`、`Shift`、`Win`、`Enter`、`Backspace`、`Tab` 等保留文字，方向键使用 `↑↓←→`，标点显示实际字符。未使用 Tabler 或外部键盘图标，不维护自绘路径及图标缩放规则。逻辑键名、可访问名称和可见短标签分别处理，避免将 `PgUp`、`Del` 等缩写误写为 ARIA 键值。

## 7. 无障碍与绑定提示

- `YueKbd` 默认是装饰性子元素；若只由 `YueShortcut` 负责播报，单个键帽不重复设置可访问名称。
- `YueShortcut` 应提供由规范化标签拼出的可读名称，例如 `Ctrl + Shift + P`、`G then I`、`Ctrl + Y or Ctrl + Shift + Z`。当前连接词使用“然后”和“或”，键位标签保持 Windows 约定。
- Shortcut 本身不是按钮、菜单项或输入控件，不主动取得焦点，也不接管 Tab 顺序。
- `aria-keyshortcuts` 属于真正执行动作的 Button/MenuItem 等元素。调用方应把与 GPUI binding 相同的规范化值挂在触发元素上；不能只因为旁边渲染了 `YueShortcut` 就认为行为已经可访问。
- 如果实现提供 `aria_keyshortcuts_value()`，它返回 ARIA 规范使用的名称（如 `Control+Shift+P`、`Control+/`、`Control+ArrowLeft`），与展示文本分开；它不注册监听器，也不验证真实 binding 是否存在。
- 图标键必须有文本等价物；仅显示箭头 SVG 不能把方向键的意义留给形状猜测。

## 8. 组合场景

### 菜单行

菜单项的标题、描述和 shortcut 是兄弟区域。菜单 owner 负责 Action、KeyContext、禁用状态和 `aria-keyshortcuts`；`YueShortcut` 只占用菜单行的末端空间。

### 按钮内

按钮文本与 `YueShortcut` 可以放在同一横向布局中，例如“命令面板  Ctrl + Shift + P”。快捷键不应被按钮内部的文本省略规则压到不可读；空间不足时由按钮或外层采用明确的隐藏/Tooltip 策略。

### Tooltip 与正文

Tooltip 中使用 `Small + Inverse` 继承浮层文字色；正文行内使用 `Medium + Ghost`。这两种场景都不产生额外焦点节点。帮助页或速查表可使用 `Large + Raised`，但不应让展示组件承担全局快捷键列表的状态管理。

### 设置行

设置页可以把“功能名称、当前快捷键、修改入口”组合成普通 List 行。修改入口和录入器属于设置功能；YueKbd/YueShortcut 仍只呈现已确认值。

### 现有组件入口

```rust
let save = ShortcutSpec::parse("Ctrl+S")?;
YueButton::new("save", "保存").shortcut(save.clone());
YueMenuItem::new("保存").shortcut(save.clone());
YueTooltip::new("save-help", theme, "保存当前文件").shortcut(save);

YueShortcut::parse("live", "Ctrl+Shift+K")?
    .key_state("Ctrl", KbdState::Pressed)?
    .key_state("K", KbdState::Default)?
    .separator(ShortcutSeparator::None);
```

- Button 保留原命令名称、焦点与执行逻辑，在文字后附加紧凑的反色键帽；单组合绑定同时写入原 Button 的 ARIA shortcut。
- MenuItem 在末端附加紧凑 Ghost 键帽；`.shortcut(...)` 与原文字 `.hotkey(...)` 互斥，最后调用者生效。菜单行与键帽的禁用状态联动；不会自动创建命令绑定。
- Tooltip 将说明和键帽一起测量，使用自己的一个完整可访问名称。空间不足时将说明和快捷键一起降级为可换行文字，避免长组合越出浮层。
- 三种宿主都关闭内部 Shortcut 的独立 Image 语义，避免重复播报；独立 `YueShortcut` 保留组级语义。`.semantics(false)` 仅供已有完整语义的宿主使用。

## 9. 错误、布局与生命周期合同

- 空字符串、只有连接符、未知修饰键、重复修饰键或非法序列应在构造/解析时返回可诊断错误；不生成空键帽或吞掉输入。
- 一个 chord 至少包含一个主键；只有修饰键的展示必须显式使用结构化 `ModifierOnly` 语义，不能由字符串歧义推导。
- `YueShortcut` 默认 `white-space: nowrap` 等价的 GPUI 布局合同；键帽、连接符和 alternative 均不得被 flex 压缩到改变字形。
- 外层宽度不足时，调用方选择裁剪、横向滚动、Tooltip 或替代文案；Shortcut 不在连接符处自动换行。
- 组件是值式展示；除 owner 显式传入的状态外不持有跨帧业务状态，不创建输入任务或订阅。动效采样保存在 GPUI 的 keyed state 中，Showcase 的输入反馈状态由页面 Entity 持有。
- 组件销毁不影响任何 GPUI binding、Action、焦点或命令状态。

## 10. 验收计划

实现前后的验收应覆盖：

1. **解析与规范化**：单键、四种修饰键、F1–F24、方向键、组合、序列、备选、空值和非法 token；确认修饰键排序稳定。
2. **尺寸与变体**：20/24/28px 高度、字体、底边、ghost/inverse 对比度、pressed 下沉和 disabled 对比度；确认按下不改变组合逻辑宽度，Ghost 按规格紧凑布局。
3. **布局**：菜单末端、按钮内、Tooltip、正文和设置行；确认不换行、不重叠，窄宽度下由外层处理溢出。
4. **反馈与动效**：逐键状态独立更新，100ms 中间帧与结束帧正确，减少动态效果时立即切换；失焦、窗口失活后无残留按下状态，Tab 正常离开局部演示区。
5. **绑定边界**：GPUI Action/KeyBinding 仍由 owner 处理，展示组件不响应按键；owner 提供 `aria-keyshortcuts` 时与显示值一致。
6. **可访问性**：快捷键容器只播报一次，装饰图标不重复播报，真实触发元素拥有正确 role、名称和 shortcut 属性。

Showcase 的 **组件 → 快捷键 Shortcut** 页面提供与参考相同的 `Ctrl + K 然后 C 或 Ctrl + /` 结构示例、ANSI Windows 全尺寸键盘图、尺寸与状态、组合/序列/备选，以及真实 Dropdown / Button / Tooltip / List 组合示例、可切换规格对照和局部实时按键区。键盘图由 `YueKbd` 实际绘制，窄窗口下横向滚动查看；它展示常见 ANSI 键位布局，不是物理键盘检测面板。局部实时反馈由独立 Showcase 页面 owner 处理；只有演示区持有焦点且出现在当前输入上下文时拦截按键，失焦与窗口失活均清空状态。页面另绑定 Ctrl+S 与 Ctrl+Shift+P 演示命令，用于验证局部监听不会误触发真实绑定。录制器、冲突检测或完整设置工作流不属于 `YueKbd`/`YueShortcut` 的实现范围。


### 本轮自动化验收

- `cargo test -p yue-component --features headless-tests --lib --quiet`：364 项通过。快捷键相关检查覆盖解析、真实键帽尺寸/边框、逐键状态、分隔符、100ms 动效与减少动态效果、Button/Menu 命令及禁用语义、Tooltip 尺寸与长组合降级。
- `cargo test -p windows-showcase --features headless-tests pages::shortcut::tests --bin windows-showcase --quiet`：2 项通过，覆盖四组规格控件的真实点击、组合/序列/备选语义，以及局部输入、释放、Tab、失焦和窗口失活。
- `cargo check -p windows-showcase --quiet` 通过；组件库与 Showcase 的 Clippy 均以 `-D warnings` 通过。
- 本轮未启动真实窗口。Windows 实机视觉效果和 Narrator 体验由使用者验收，以上无头测试不替代该项。

## 11. 后续边界

`YueShortcutRecorder` 已作为独立组件实现，详见 [录制器文档](31-shortcut-recorder.md)；组件提供校验与显式覆盖入口，具体命令冲突及解绑由宿主处理。快捷键设置持久化、完整命令面板、键盘布局/输入法适配、跨平台显示映射、物理键盘图例，以及 Windows UIA 中额外的键盘模式语义仍属于独立能力。
