# YueShortcutRecorder 快捷键录入器

状态：V1 已实现（2026-11-07）。参考 `refer/Shortcut 快捷键 · GPUI 组件 Demo.html` 的 Recorder 与设置列表，面向 Windows；真机视觉和 Narrator 验收由用户负责。

## 1. 职责与复用

`YueShortcutRecorder` 编辑一个快捷键组合，`YueShortcutRecorderState` 是已确认值与录制草稿的唯一所有者。宿主保留 Entity 及事件订阅，负责持久化、命令注册和其他命令的解绑。

- 复用 `ShortcutSpec` 的解析、别名规范化与修饰键排序。
- 复用 `YueKbd` / `YueShortcut` 展示文本键帽，不使用图标或跨平台映射。
- 复用 Base Button 的语义、指针与键盘激活、焦点及禁用行为，Base A11yLive 提供状态消息语义。
- 外观使用 Input 的尺寸、边框、背景、字体与焦点 token；覆盖和取消复用 YueButton，设置示例复用 YueList。

展示组件仍不监听输入；录制器的局部输入订阅独立保存在录制状态中，不安装 GPUI Action 或 KeyBinding。

## 2. 交互合同

| 状态 / 输入 | 结果 |
| --- | --- |
| 点击字段，或聚焦后按 Enter / Space | 开始录制，保留已确认值 |
| 仅按修饰键 | 实时预览 Ctrl / Alt / Shift / Win，不提交 |
| 按下主键 | 通过校验则立即提交；不等待松键 |
| 无修饰键 Esc | 取消草稿，保留原值 |
| 无修饰键 Backspace | 清除已确认值 |
| Tab / Shift+Tab | 正常移动焦点；离开整个录制区域时取消草稿 |
| Ctrl / Alt / Win + Tab | 作为组合录入 |
| 缺少修饰键、保留键、宿主拒绝 | 显示错误，保留原值，继续录制 |
| 宿主返回 Conflict | 显示候选键帽与冲突消息，提供“覆盖 / 取消”；原值保持不变 |
| 覆盖 | 再次运行校验；仍冲突时提交并发送 `overwrite: true` |
| 取消冲突，或再次点击候选字段 | 丢弃候选，保留原值 |
| 离开区域、窗口失活、禁用、外部同步值 | 取消草稿与候选，不产生用户变更事件 |

覆盖、取消按钮属于同一焦点区域，Tab 到这些按钮不会丢失候选。Esc 在整个候选区域内可取消。禁用字段不能激活，也不参与 Tab 顺序。

默认要求 Ctrl / Alt / Win 中至少一个；F1–F12 可单独使用，也可与 Shift 组合。`set_require_modifier(false, cx)` 可允许普通单键或 Shift+单键；无修饰键的 Esc、Backspace 和 Tab 保留编辑操作含义。

字段有录制指示点与焦点光晕、错误/冲突边框及文字提示。状态消息保留一行高度，长消息自然换行。字段内部只对过长键帽组合横向滚动；宽度由 `.width(...)` 调整，默认 14rem。三档尺寸沿用 Input 的 28 / 32 / 40px，并随 rem 缩放。未复刻参考页的摇晃、闪烁和成功闪色，状态通过稳定的文字和颜色反馈。

## 3. 接入

```rust
use gpui_kit::{AppContext as _, Context, Entity, Subscription, Window};
use yue_component::{
    ShortcutSpec, ShortcutRecorderEvent, YueShortcutRecorder,
    YueShortcutRecorderState,
};

struct Settings {
    shortcut: Entity<YueShortcutRecorderState>,
    subscription: Subscription,
}

impl Settings {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let shortcut = cx.new(|cx| {
            let mut state = YueShortcutRecorderState::new(window, cx);
            state.set_value(Some(ShortcutSpec::parse("Ctrl+Shift+K").unwrap()), cx).unwrap();
            state
        });
        let subscription = cx.subscribe(&shortcut, |_, _, event, _| {
            let ShortcutRecorderEvent::Change { value, overwrite } = event;
            // 保存 value，并更新真实绑定。
            // overwrite 为 true 时，由此 owner 一并解除冲突命令的绑定。
        });
        Self { shortcut, subscription }
    }
    fn field(&self) -> YueShortcutRecorder {
        YueShortcutRecorder::new("open-panel-shortcut", &self.shortcut)
            .accessibility_label("打开面板快捷键")
    }
}
```

用户确认或清除后，先更新状态，再发送 `ShortcutRecorderEvent::Change { value, overwrite }`。确认相同值和清除空值不重复发事件；显式覆盖即使值相同仍发送，方便宿主处理其他冲突命令。`set_value(...)` 是静默外部同步，不重新调用用户校验或回送 Change。

`set_value` 只接受单个组合且恰好一个主键；序列、备选和多个主键会返回错误，原有值与草稿不变。展示层仍支持序列和备选，不改变其原有合同。

## 4. 保留键与宿主校验

内置参考页的 `Alt+F4`、`Win+L`、`Win+D`、`Ctrl+Alt+Delete` 四项保留键。`set_reserved(Vec<ShortcutSpec>, cx)` 替换附加保留键，内置四项继续生效。

```rust
use yue_component::ShortcutRecorderValidation;

// 在 state.update(cx, |state, cx| ...) 中设置；闭包可读取宿主的当前数据。
state.set_validator(move |candidate, cx| {
    if registry_has_conflict(candidate, cx) {
        ShortcutRecorderValidation::Conflict("与「保存」冲突".into())
    } else {
        ShortcutRecorderValidation::Accept
    }
}, cx);
```

`registry_has_conflict` 代表宿主自己的查询函数。校验顺序为保留键 → 修饰键要求 → 宿主校验。`Reject(message)` 阻止提交；`Conflict(message)` 要求用户显式确认。闭包应无副作用；捕获所属视图时使用 WeakEntity，避免循环持有。组件不会搜查全局绑定或自行修改其他组件的值。

保留键列表是应用校验，并非 Windows 热键注册或系统拦截。系统可能在 GPUI 收到输入前处理 Win+L 等组合；此时组件无法显示该组合的错误，也不能阻止系统动作。

## 5. 输入、生命周期与可访问性

录制时使用 GPUI `intercept_keystrokes`，在 Action 解析前阻止录制中的命令执行。只有本状态的焦点区域实际包含焦点，且当前渲染的输入上下文包含 `YueShortcutRecorder` 时才处理输入。闲置状态放行普通命令；已提交或取消的主键在松开前继续抑制自动连发。

采用 GPUI `Keystroke.key` 的逻辑键名，不把 `key_char` 或 IME 输出当作命令键。Windows 会将部分 Shift 标点折入符号，所以支持 `Plus`、`Pipe`、`?` 等逻辑表示。规范化字符串相等可作为简单冲突比较；不同布局或 `Shift+Equal` / `Plus` 等等价写法的统一由宿主结合自身绑定系统处理。本组件不宣称识别物理键位，也不增加扫描码或全局键盘钩子。

字段是一个有名称的 Button，名称包含当前值或录制预览，说明包含操作提示/错误；键帽关闭独立 Image 语义。状态消息用 Polite live region，避免重复朗读完整键帽子树。该字段不是已绑定的命令入口，因此不把录制值写成其自身的 `aria-keyshortcuts`。

每个挂载字段对应一个状态；不要将同一状态同时用于两个字段。订阅随状态释放；移出页面后输入上下文及焦点检查阻止隐藏字段捕获输入。

## 6. 示例与验收

Showcase → 快捷键 Shortcut → 快捷键录制与设置：演示字段、保存文件和命令面板三行共享一个由页面管理的冲突策略。覆盖会解除另一行的值，恢复默认也由页面处理冲突。这些是演示设置，不修改页面已有的 Ctrl+S / Ctrl+Shift+P 示例绑定。

自动化测试通过真实无头输入验证录制、修饰键预览、Action 拦截、自动连发、取消清除、保留键、功能键豁免、冲突覆盖前重验、键盘确认、焦点移出、窗口失活、禁用、外部同步、多实例隔离及原生标点。Showcase 测试覆盖冲突行解绑与恢复默认。

真实 Windows 视觉、系统实际投递、输入法和 Narrator 由用户验收。设置持久化、命令注册、序列录制、全局热键和物理布局映射属于其他能力。
