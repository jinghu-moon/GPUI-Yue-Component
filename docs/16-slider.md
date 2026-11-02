# 16 YueSlider 滑动条组件设计

状态：**首版已实现，Windows 实窗基础拖动已检查；多 DPI / 读屏验收待完成**。日期：2026-11-01。依据[用户提供的 Slider 截图](assets/slider-reference.png)、[本地 HTML demo](../refer/GPUI%20Kit%20组件文档.html)、vendored GPUI Base 0.8.0 源码及 Yue 现有组件。公共 API 位于 `crates/yue-component/src/slider/`，示例位于 `examples/windows-showcase/src/pages/slider.rs`。自动验收记录见第 18 节。

## 1. 目标与首版范围

滑动条将一个有界数值或一个有界区间映射到轨道上的位置。用户可以拖动快速调整，通过键盘或配套数字输入框精确调整。它是数值输入控件，不是进度条、滚动条或 Segmented。

采用截图中的功能和基本外形：**胶囊轨道、圆形滑块、强调色选中段**，提供单值 / 范围、水平 / 垂直、禁用、数值提示、刻度标签、数字输入框、自定义步长与上下限。默认跟随 Yue 的主题和控件密度，不复制截图网站的蓝色常量。

| 截图内容 | 本项目方案 |
| --- | --- |
| 一个滑块 | `SliderValue::Single(value)`；填充从 min 到当前值 |
| 两个滑块 | `SliderValue::Range(lower, upper)`；只填充两者之间 |
| 水平 / 垂直 | 水平从左到右增大，垂直从下到上增大 |
| 禁用 | 值仍可读；滑块、轨道、输入框整体不可编辑 |
| 悬浮数值 | 复用 Yue Tooltip；拖动 / 键盘焦点即时展示，hover 使用公共延迟 |
| 刻度与温度等标签 | 有值定位的 marks，支持自定义文字及单位 |
| 数字输入框 | 单值一个、范围两个；与轨道使用同一数值源 |
| 自定义 step / min / max | 一个原子 domain 配置，所有输入路径使用同一规则 |
| 页面交互预览 | Showcase 的组件组增加 Slider 页，按英文首字母排序 |

首版同时完成上述功能与 Small / Medium / Large 三档尺寸。尺寸扩展来自本库的控件一致性要求，不是截图明确规定的功能。

不纳入首版：任意数量滑块、拖动整段范围、滑块交叉换身份、非线性 / 对数刻度公开配置、播放缓冲段、多色分区、自定义滑块子树、常驻双气泡、滚轮改值、异步加载状态。有刻度时默认仅在指针靠近有效 marks 时局部磁吸，其余位置继续按 step 自由拖动；`.snap_to_marks(false)` 可将刻度恢复为纯展示。不增加第二份 stops 数据。

## 2. 能力基础与必须先修的缺口

### 2.1 实施前源码与消费端基线

行为基础是 [`vendor/gpui-base/src/slider.rs`](../vendor/gpui-base/src/slider.rs)，不启用 gpui-kit styled Component，也不引入第二个 GPUI 后端。

| 已存在能力 | 当前合同 / 边界 |
| --- | --- |
| `SliderState` | 持有 min / max / step、数值、轨道 bounds、拖动状态 |
| `SliderValue` | `Single(f32)` / `Range(f32, f32)`；已有单数、元组及 Rust Range 转换 |
| `SliderScale` | 已有 Linear / Logarithmic；Yue 首版只开放 Linear |
| `SliderEvent` | 已有 `Change` / `Release`；Release 表示交互结束，不保证数值变化 |
| `Slider` | 根节点、方向、disabled、可访问名称、调用方焦点句柄 |
| `SliderTrack` | 轨道点击、部分轨道拖动与触摸输入行为 |
| `SliderThumb` | 单值或范围端点的指针拖动、触摸拖动 |
| `SliderIndicator` | 通过 prepaint 保存完整映射轨道的 bounds；不是自动绘制选中段的部件 |
| `set_value` | 静默赋值并通知观察者；当前不会完整规范化实际数值 |
| `is_dragging` / `cancel_interaction` | 本地已有 ColorPicker 所需扩展；取消旧交互不能误停其他控件的全局拖动 |

Base 不自动提供 Yue 外观、尺寸、刻度标签、Tooltip 或数字输入组合。这些由 Yue 呈现与组合层负责。

### 2.2 当前缺口及处理层

| 当前源码事实 | 风险 | 实施要求 |
| --- | --- | --- |
| 指针量化使用 `round(value / step) * step` | 非零 min 时步进网格错位 | Base 改为从 min 起算的量化 |
| 量化后没有统一的最终归一化 | 越界、端点不可达 | Base 补最终 clamp 和明确端点规则 |
| 保存的是未吸附的 pointer percentage | 数值已吸附，滑块仍停在另一位置 | 百分比从最终数值重新计算 |
| 同值也可能发出 Change | 重复业务更新 | Base 在规范化后去重 |
| 外部赋值只夹紧绘制位置，原值可以越界 / 倒序 | 显示、事件和读取值不一致 | 补赋值不变量；Yue setter 另执行本组件的 step 策略 |
| 单值轨道和范围轨道拖动分支不对称 | 范围模式按住轨道后行为不一致 | Base 补范围轨道选定端点后的连续拖动 |
| 没有 Arrow / Home / End / Page 键策略 | 鼠标可用，键盘不可用 | 可复用键盘行为下沉 Base |
| 根只有一个 Slider 角色，读的是 end 值 | 双滑块无法分别操作 / 识别 | 每个 thumb 有独立焦点和 Slider 语义 |
| 无障碍增减以 f32 调用 `set_value` | Range 会被变成 Single，且无用户变更事件 | 改为按目标 thumb 执行同一用户数值操作 |
| 线性 domain 缺少完整校验；零长度轨道可能参与除法 | NaN / Inf 或无效状态 | Base 增加 domain 与有效几何校验 |

表中缺口已在 vendored Base 修复，由新的数值与 UI 集成测试锁定。Base 增加 `SliderDomain`、`Start` / `Cancel` 事件和逐 thumb 语义；旧的 `Change` / `Release` 仍保留。具体补丁见 `vendor/gpui-base/YUE-PATCH.md`。ColorPicker 的程序赋值继续保留非 step 精度，Layout 的 Base 使用方式保持可用。

## 3. 分层与状态所有权

```text
宿主 Entity
  └─ Entity<YueSliderState>：用户编辑事务、字段草稿、公开事件、可用性
       ├─ 私有 Entity<BaseSliderState>：唯一的规范数值与拖动机制
       ├─ 稳定的单值 / lower / upper 焦点句柄
       └─ 可选的数字输入 Entity<InputState> 与订阅

YueSlider（RenderOnce）
  ├─ Base Slider / Track / Thumb：输入、焦点、语义、映射
  ├─ Yue 轨道、填充、滑块、刻度、标签：token 呈现
  ├─ YueNumberInput：可选精确编辑
  └─ YueTooltip → 既有 Popup：数值提示
```

- `YueSliderState` 不再保存第二份 canonical value；读值委托私有 Base state。起始值快照只用于判断一个编辑事务是否产生 Commit，不是渲染数据源。
- Base 保存可复用行为；Yue 保存数字字段草稿、用户完成编辑的策略和 token 外观。宿主决定是否即时预览、保存设置或发请求。
- state 在宿主初始化阶段通过 `cx.new` 创建，订阅由宿主保留；不在 render 中反复创建实体、焦点或订阅。
- 单值 / 范围模式在 state 创建时确定。不能靠 render 中切换 `.range(true)` 重建模式。Showcase 的模式开关切换两个预先持有的 state。
- disabled 由 state 统一拥有，经显式 setter 更新，同时中断交互、禁用数字字段并发布禁用语义，避免轨道与输入框出现两个 disabled 真值。禁用节点仍可被辅助技术读取。
- 不对外暴露可变 Base Entity，避免调用方绕过 domain、事件和模式约束。Base 的单独消费端仍然可以直接使用 Base。

引入 `YueSliderState` 的依据是本控件需要协调滑块事务、两个输入草稿与事件，不能由一个无状态外观 builder 可靠承担。`SliderDomain` 只解决三项互相关联配置的原子校验，不引入额外业务模型。

## 4. 公共 API

以下接口已导出，也可从 prelude 引入。字段私有，宿主保留 state 与事件订阅。Rustdoc 中的最小示例通过编译；下方片段说明宿主初始化与 render 的职责。

```rust
// 在宿主初始化中创建，保存 Entity 与订阅。
let volume = cx.new(|cx| {
    YueSliderState::new(
        SliderDomain::new(0.0, 100.0).step(1.0),
        40.0,
        window,
        cx,
    )
});
let interval = cx.new(|cx| {
    YueSliderState::new(
        SliderDomain::new(-20.0, 100.0).step(5.0),
        (20.0, 80.0),
        window,
        cx,
    )
});

let subscription = cx.subscribe_in(&volume, window, |this, _, event, _, cx| {
    match event {
        YueSliderEvent::Change(value) => { /* 当前规范值已经更新；即时预览 */ }
        YueSliderEvent::Commit(value) => { /* 用户完成一次有效编辑；保存 */ }
    }
    cx.notify();
});

// render：传现有 Entity，不创建新状态。
YueSlider::new("volume", theme, &self.volume)
    .accessibility_label("音量")
    .size(SliderSize::Medium)
    .width(px(320.0))
    .controls(SliderControls::Input)
    .value_format(|value| format!("{value:.0}%").into());

YueSlider::new("temperature", theme, &self.interval)
    .accessibility_label("温度范围")
    .lower_label("最低温度")
    .upper_label("最高温度")
    .marks([
        SliderMark::new(-20.0, "−20°C"),
        SliderMark::new(0.0, "0°C"),
        SliderMark::new(50.0, "50°C"),
        SliderMark::new(100.0, "100°C"),
    ]);
```

### 4.1 数值与 domain

复用并有意 re-export Base 的 `SliderValue`，不再定义等价的 Yue 数值 enum。

| 接口 | 合同 |
| --- | --- |
| `SliderDomain::new(min, max)` | min、max 有限且 min < max；默认 step = 1 |
| `domain.step(step)` | 有限且 > 0；允许大于跨度，此时可选端点只剩 min / max |
| `domain.min()` / `max()` / `step_value()` | 只读配置 |
| `YueSliderState::new(domain, value, window, cx)` | 根据 Single / Range 固定模式；规范化初值，不发事件 |
| `state.value(&App)` / `domain(&App)` | 从私有 Base state 读取；不假装无需 App 就能读子实体 |
| `state.set_value(value, window, cx)` | 静默规范化；模式必须一致；同值回写是真正 no-op |
| `state.set_domain(domain, window, cx)` | 原子更新上下限及步长，再规范化当前值；静默，不经过无效中间配置；相同 domain 为 no-op |
| `state.set_disabled(bool, window, cx)` | 默认 false；同值 no-op；变化后同步全部交互部件 |
| `state.is_disabled()` / `is_range(&App)` | 查询可用性 / 模式 |

构造、setter 的非法配置 / NaN / Inf / 模式不匹配属于编程错误：同步拒绝并给出带参数名的明确 panic 信息，不偷偷改成 0。输入框里的非法文字属于用户草稿错误，不 panic。所有参数先验证后写入，失败不留下部分更新。

### 4.2 外观与组合

| builder | 默认 / 合同 |
| --- | --- |
| `YueSlider::new(id, theme, &Entity<YueSliderState>)` | 稳定 ID、显式主题；水平、Medium；每个挂载实例独立 state |
| `.use_default_theme()` | 与本库其他控件一致，响应默认主题变化 |
| `.size(SliderSize::{Small, Medium, Large})` | 同步轨道、滑块、交互厚度、字体及配套输入框 |
| `.layout(SliderLayout::{Horizontal, Vertical})` | 仅改呈现方向，不改值或 lower / upper 身份 |
| `.width(length)` / `.height(length)` | 外部布局尺寸；垂直且有辅助控件时 height 为最小高度，必要时随内容增长；不是数值 domain |
| `.block()` | 显式占据父级可用宽度；默认不 block |
| `.controls(SliderControls::{None, Input, Buttons})` | 默认 None；Input 为带箭头数字框，Buttons 为两端按钮加无箭头数字框；范围模式中两种形式都显示两个字段 |
| `.marks(iter<SliderMark>)` | 默认无刻度，按值排序；有效 marks 默认参与吸附 |
| `.snap_to_marks(bool)` | 默认 true；指针接近 mark 才磁吸，其他位置与键盘 / 数字输入保持 step；false 关闭磁吸 |
| `.tooltip(SliderTooltip::{Auto, None})` | 默认 Auto；不提供常驻双 Tooltip |
| `.value_format(formatter)` | `Fn(f32) -> SharedString`；提示与可访问值文字使用，不能改变原始值 |
| `.accessibility_label(text)` / `.accessibility_description(text)` | 控件名称必须由宿主提供；描述可选 |
| `.lower_label(text)` / `.upper_label(text)` | 范围端点名称；默认根名加英文 `lower` / `upper`，宿主提供本地化名称 |
| `.input_error_message(text)` | 默认 `Enter a valid number.`；宿主提供本地化错误说明 |
| `.tab_index(isize)` / `.tab_stop(bool)` | 作用于实际 thumb 焦点句柄；范围内 lower 在 upper 之前 |

本库目前没有统一 locale catalog，不声称这些默认名称已自动本地化。单值数字框名称为根名加 `value`，范围数字框为端点名加 `value`，与滑块角色区分。Showcase 显式传入中文根名称、端点名称及错误说明。

不提供同时存在的 `.value(...)` render builder 和 state setter，以免出现两套受控来源。用户事件统一经 state 订阅；首版不再增加内容相同的 `.on_change` / `.on_commit` builder。

`SliderMark::new(value, label)` 以数值定位，`.label_visible(false)` 可以仅显示刻度点；同值重复项或非有限值是配置错误。越界 marks 不参与呈现，domain 变化后按新范围重新筛选。按数值排序不改变业务身份。

## 5. 数值不变量与步进规则

### 5.1 唯一规范化规则

记最小值为 a，最大值为 b，步长为 s。用户与 Yue 外部 setter 统一选择以下有限有序候选集合中最近的点：

```text
G = { a + k·s | k 为非负整数，a + k·s <= b } ∪ { b }
```

先 clamp 到 [a, b]，再找相邻候选；距离相等时选数值较大者，最后再次 clamp。实现按相邻网格序号计算，**不枚举整个集合**。包含 b 的目的，是跨度不能被 step 整除时，两个端点仍然可达。

| domain / 输入 | 规范值示例 |
| --- | --- |
| min=3, max=13, step=2；输入 6 | 7；候选为 3 / 5 / 7 / 9 / 11 / 13 |
| min=0, max=10, step=3；输入 9.8 | 10；最后一段为 9 → 10 |
| min=−5, max=5, step=2；输入 −2 | −1；不按零点量化 |
| min=0, max=1, step=0.1；输入 0.26 | 0.3；展示按步长消除浮点尾数 |
| min=0, max=10, step=20 | 只允许 0、10；键盘仍能到两端 |

- Single 始终满足 a ≤ value ≤ b。
- Range 始终满足 a ≤ lower ≤ upper ≤ b；外部范围赋值先分别规范化再排序。用户操作只夹紧活动端点，不交换端点身份。
- lower == upper 合法，不强制最小区间宽度。
- 百分比必须从最终规范值算出：`p=(value-a)/(b-a)`。垂直绘制为 `y=bottom-p*length`。
- 中间计算使用 f64，公共值仍与 Base 一致为 f32；格式化不能通过字符串反向更改数值。
- 浮点精度校验须覆盖范围和步长是否能表达有效的相邻值；拒绝会让相邻网格在 f32 中退化为同值的配置，不能靠任意 epsilon 吞掉合法小数变化。无需物化 G 即可检查边界相邻点。
- 无有效轨道长度时不处理位置输入，不生成 NaN；键盘和输入框仍按 domain 操作。

### 5.2 与现有 ColorPicker 的精度边界

不能把所有 Base `set_value` 一律改成 step 吸附：ColorPicker 的 hue / alpha state 还用于精确的颜色同步，现有调用可能包含非整数 hue。

实施时区分两个机制：**Base 程序赋值保持有限、夹紧、有序且保留合法精度；Base 用户操作以 min 对齐的 step 为基础**。指针落在 mark 的小范围磁吸区时采用该 mark 的精确值，范围外使用 step；键盘和数字步进始终沿 step 移动。YueSlider 的公开 setter 主动调用 Base 同一规范化函数：精确等于有效 mark 时保留其值，其余按 step；同值回写提前返回。ColorPicker 的程序赋值继续保留精度。数值与磁吸算法只在 Base 实现一次。

Base 程序赋值仍须从实际最终值重算百分比。此次通用修复必须在修改前添加 ColorPicker 精确同步及现有拖动中替换值的回归用例。

## 6. 鼠标、触摸与重合端点

| 操作 | 单值 | 范围 |
| --- | --- | --- |
| 点击轨道 | 将单值移至点击位置并取得焦点 | 选择较近端点并移动；随后可继续拖动该端点 |
| 拖动滑块 | 在 min / max 内吸附 | lower 不超过 upper，upper 不低于 lower |
| 拖动越过轨道 | 夹紧到端点，继续保持拖动 | 同左，不换活动端点 |
| 在控件外松开 | 完成一次交互 | 同左；不能保持永久 pressed |
| 点击刻度 / 标签 | 装饰，不独立激活；命中轨道时遵循轨道规则 | 同左 |
| 滚轮 | 不改值，交给父滚动区 | 同左 |

端点距离相同：优先最近活动的 thumb；首次交互没有历史时选 upper。重合时最近活动 thumb 的被动视觉体最后绘制，两个真实焦点节点始终 lower → upper。重合 hitbox 的鼠标按下同样按该规则选端点，不依赖绘制顺序决定业务身份。用户可用键盘分别展开重合端点；不在拖动中交换 lower / upper。

轨道只在主键 / 合法触摸接触时启动交互。触摸复用 Base 的 TouchDragEvent 与命中判定；开始拖动后声明本次手势归属，避免同时滚动父级。不要对全窗口触摸一律 prevent_default。

视觉滑块与命中区分离。hover、按压或焦点描边不能改变 hitbox、测量轨道、兄弟布局或文本排版；按压仅作中心对称的轻微视觉缩放，端点的数值锚点不变。禁止拖动时把手指映射到经过缩放的 bounds。

## 7. 键盘、焦点与无障碍

### 7.1 键盘合同

| 按键 | 行为 |
| --- | --- |
| Tab / Shift+Tab | 进入 / 离开 thumb；范围固定 lower 后 upper，两个都是正常 Tab stop |
| Right / Up | 下一个较大的候选值；两个方向布局均适用 |
| Left / Down | 下一个较小的候选值 |
| Home | 活动 thumb 可用下界：单值 / lower 为 min，upper 为当前 lower |
| End | 活动 thumb 可用上界：单值 / upper 为 max，lower 为当前 upper |
| PageUp / PageDown | 以 domain 跨度的 10% 为大步进目标，再规范化；至少前进 / 后退一个候选 |
| Escape | 关闭当前数值提示；不回滚已变更的滑块值 |
| Enter / Space | 不模拟 Button 点击，不切换数值 |

键盘自动重复产生连续 Change，相关调节键全部释放后完成一次 Commit。修饰键组合不劫持宿主已有命令；Slider key context 只在 thumb 焦点时生效，不能拦截数字编辑框内的方向键 / 光标操作。

轨道点击把焦点交给所选 thumb。焦点环跟随该 thumb，使用 focus-visible 策略，不给整条轨道画成超长椭圆。禁用后移除 Tab stop，已有焦点不再接受数值操作；是否将焦点转给其他业务对象由宿主决定。

### 7.2 语义结构

| 模式 | 需要的节点 |
| --- | --- |
| Single | 一个真实 `Role::Slider`：名称、当前值、min、max、step、orientation、可用性 |
| Range | 非额外可操作 Slider 的容器；lower / upper 各一个独立 Slider 节点与焦点 |
| Range lower | accessible max = 当前 upper，min = domain min |
| Range upper | accessible min = 当前 lower，max = domain max |
| 数字输入 | Single 一个、Range 两个 SpinButton；名称明确区分“滑块”与“数值输入” |
| 刻度 / 填充 / Tooltip | 装饰，不产生重复的可操作节点；Tooltip 不成为读值的唯一途径 |

可访问值文字与可见提示共用 formatter，例如“50%”“20°C”；实际数值属性仍是数字。需要核实 GPUI 到 Windows UIA 桥是否能传递值文字：如果当前缺槽，补最小 Base / GPUI 适配并测试；若原生桥无法表达，文档明确降级为数值加单位描述，不把屏幕文字等同于读屏播报。

现有 Base root 的单一 Slider role 必须调整，不能在外面再套两个有角色的 Div，留下第三个错误 Slider。无障碍 Increment / Decrement 必须针对被操作的 thumb，保留 Range，并经过与键盘相同的 Change / Commit 路径。

数字输入沿用 Base 编辑器的键盘合同：**目前 Tab 在输入上下文是缩进，不是字段遍历**。不能把本设计的 thumb Tab 规则写成整组数字字段已支持顺序跳转；该问题若另行修改，应作为独立 Base 输入能力变更并补旧组件回归。

## 8. 用户事件与中断

对外提供 `YueSliderEvent::Change(SliderValue)` 和 `YueSliderEvent::Commit(SliderValue)`。事件发出时规范值已经生效，回调读 state 能得到相同值。

| 来源 | Change | Commit |
| --- | --- | --- |
| 指针 / 触摸拖动 | 每次规范值实际变化时一次 | 正常松开，且最终值不同于本次起始值时一次 |
| 轨道按下后松开 | 同上 | 同上；不是 mouse-down 和 mouse-up 各提交一次 |
| 键盘调节 / repeat | 每次有效改变一次 | 调节键全部松开时一次；焦点转移时先完成已发生的键盘编辑 |
| 无障碍增 / 减 | 一次有效改变 | 同一离散操作后一次 |
| 输入框 Enter / 失焦提交 | 有效且不同值时一次 | 同一操作后一次；Enter 后的 blur 不重复提交 |
| 构造、外部 set_value / set_domain | 不发 | 不发 |
| 按下未移动、边界继续加减、返回起始值 | 不发同值 Change | 不发无变化 Commit |

不把 Base 的 Release 原样改名为 Commit。Yue 记录事务起点并验证是否有有效变化；单次事件顺序始终 Change → Commit。

- 用户回调把相同值 echo 给 setter：不打断拖动，不清草稿，不重置焦点。
- 真正的外部值替换 / domain 变化：取消旧事务，清除旧拖动及草稿，再静默应用；稍后到达的旧 Release 不产生 Commit。
- 禁用、卸载、窗口失活、触摸 Cancelled：取消未完成事务、关闭提示，保留最后已应用值；不再发送迟到的 Commit。触摸 Cancelled 不能继续等同正常 Release。
- 中断不自动回滚预览；需要“确认 / 取消”事务的业务在宿主保存独立业务快照，不让 Slider 承担业务撤销。
- 回调中同步禁用或替换值必须安全；使用事务代次或等价机制拒绝陈旧事件，不用动画完成时机决定提交。

## 9. 数字输入框组合

`SliderControls::Input` 组合 `YueNumberInput` 与 Base `InputState`，不另写文本编辑器。InputState 的数值约束由 Slider state 动态同步；Slider 使用减小 / 居中编辑区 / 增大的横排布局，glyph 保持 Tabler `chevron-down` / `chevron-up`。字段宽度按当前字体及三档字号实测至少三位数字，同时容纳 domain 的符号、小数与较长端点；再叠加 4px 数字专用横向 padding、边框、光标和存在时的两个图标，不再固定 96px。非法草稿不会撑宽字段，超长内容在编辑器内部滚动。`YueNumberInput` 新增 `.controls(false)`，只隐藏箭头，保留 Up / Down、SpinButton 与原生增减动作。

### 9.1 互斥的两端按钮形式

```rust
use yue_component::SliderControls;
// 两端 −/+，后接无箭头数字框。
let slider = slider.controls(SliderControls::Buttons);
// 选择数字框形式；同一个 builder 再次调用时替换选择，不叠加。
let slider = slider.controls(SliderControls::Input);
```

- None 为单纯轨道；Input 为带 Chevron 的数字编辑；Buttons 为两端步进加无箭头数字编辑。字段内箭头与轨道两端按钮互斥。删除旧的 `show_input(bool)`，调用方以枚举明确选择，无两个独立布尔开关或兼容分支。
- Buttons 复用 `YueButton::icon` 和 Tabler `Minus` / `Plus`，尺寸、hover、按压、焦点、禁用均沿用既有 Button token。水平“− / 轨道 / + / 数字框”；垂直上增下减，字段在组合底部。独立 `--slider-stepper-gap` 默认 0，轨道安全区与按钮命中区相邻，无命中重叠；数字框间距仍为 `--slider-input-gap`。
- 每次有效点击 / Enter / Space 沿 domain step 调整，复用数字框的 `apply_step` → Base `set_user_value` / Release，一次 Change 与一次 Commit；按住不连续自增，边界和整体 disabled 会禁用相应按钮。按钮保留自己的焦点，不聚焦隐藏编辑器。
- 范围模式调节最近实际操作的端点，初始为 upper；不以单纯获得焦点作为端点操作。upper 不能小于 lower，lower 不能大于 upper。默认可访问名称由端点名称加 `increase` / `decrease` 组成，沿用本库英文默认副本边界，不声称自动本地化。
- 水平单值进入顺序为减小按钮 → thumb → 增大按钮 → 数字框；`.tab_stop(false)` 跳过整组。进入数字框后，Tab 仍遵循 Base 的缩进合同，不能据此承诺 Tab 可离开编辑器。
- 模式变化取消旧拖动 / 提交事务、清除尚未提交的输入草稿，隐藏的控件若持有焦点则交回对应 thumb，不伪造 Change / Commit。宿主先触发普通输入失焦的情况仍遵循输入原有失焦提交合同。

实施时增加最小内部 bridge：初始化 InputState 的约束、domain 变化时更新约束；范围字段的动态上下界同步另一端点。步进、输入提交都回到同一 Slider 规范化函数，不能分别依赖两种有差异的 rounding 算法。共享价值明确时再补 `YueNumberInput` 公共能力，不为了 Slider 发布一个泛化表单框架。

Base NumberInput 已补描述、文本值、有限数值、min / max / step 和原生增减动作；InputBase 的 disabled 现写入同一个无障碍节点。字段错误说明传到 SpinButton 的真实 description，新增测试直接查询这些原生属性；这不等同于 Narrator 的实际播报验收。

| 阶段 | 规则 |
| --- | --- |
| 正在输入 | 合法数字立即按 step 更新轨道并发 Change，不发 Commit；保留原始编辑文字与光标。非法、空串、未完成或越界草稿标红，不写入 Base |
| Enter | 合法值结束实时编辑事务，展示规范值，至多一次 Commit；不重复发布 Change |
| Enter 输入非法 | 不改变滑块；保留草稿、显示字段错误及可访问错误描述 |
| 失焦 | 结束事务；非法草稿恢复最近合法值并清除错误，不应用 Base 的自动越界夹紧结果。已有合法变更至多一次 Commit，否则无事件 |
| Escape | 恢复最近合法值、清临时错误；结束已有实时编辑，不回滚已生效的合法预览 |
| 点击增减 / 数字步进键 | 先解析当前草稿；无效时以当前规范值起步，再执行相邻候选操作 |
| 拖动 / 键盘滑块改值 | 该操作取得编辑所有权；放弃旧字段草稿，同步规范值，不让失焦事件再提交旧草稿 |
| 真正外部赋值 / 禁用 | 取消草稿事务，显示最新规范值；同值 echo 除外 |

范围输入不交换字段或悄悄夹紧越界草稿：lower 超过 upper 或 upper 低于 lower 时标红，保留最近合法值。逐键输入 101 时，1 和 10 都是合法的实时更新，最终 101 越界，因此轨道停在 10；一次粘贴越界文本则保留粘贴前的合法值。数字文本与边框使用错误 token，可访问描述提供 `input_error_message`，避免在紧凑字段下强塞多行错误文案。

数字字段只编辑数字，百分号、温度符号等作为旁边的单位文本或提示；`value_format` 不是 parser，不把任意 formatter 的输出喂回数值解析器。首版用明确的十进制数字语法，不接受单位、千分位和 NaN / Infinity；小数展示应覆盖步长与非整除 max 的精度，不能固定只保留两位。

## 10. 刻度、标签与碰撞

- marks 的位置直接由其数值映射，不要求与 step 对齐；例如 step=3 时的 mark 50 在磁吸区内精确取 50，不被二次量化为 51。指针距离按实际轨道主轴的逻辑像素计算，在 step 量化之前判断。
- 默认磁吸半径来自 `--slider-snap-distance: var(--space-4)`，随 rem 缩放。每个 mark 的半径还限制为最近相邻 mark 像素间距的四分之一，确保密集刻度之间仍有自由拖动空间；不按整个数值范围选最近刻度。
- 指针离开半径后立即恢复 step，无全程锁定、延迟释放或额外迟滞状态。隐藏标签仍保留磁吸；越界 mark 不参与。范围端点不能交叉，另一端点作为动态边界。min / max 继续由 domain 保证可达。
- 键盘、可访问增减动作、数字框步进继续按 domain step；数字提交和 Yue setter 仅在精确等于有效 mark 时保留其非 step 值，其余按 step 规范化。Slider / SpinButton 继续报告实际的 numeric step。
- marks / 磁吸策略变化会取消旧拖动，但不移动当前值，不伪造 Change / Commit；移除 marks 或 `.snap_to_marks(false)` 只关闭局部磁吸。已吸附的非 step 当前值保留到下一次实际编辑，同值回写仍是 no-op。
- Single 的 active mark 区间为 [min, value]；Range 为 [lower, upper]；边界包含端点。
- 未选中刻度用弱化前景，选中区间内使用强调色。既显示刻度位置也显示选中段，不只靠文字颜色传递状态。
- 水平标签在轨道下方，垂直标签在轨道右侧。行距与标记到轨道的距离由 token 决定。
- 首尾标签向容器内对齐，中间标签中心对齐；长文本单行省略并可使用现有 OverflowTooltip 查看完整内容。
- 标签碰撞按实际文字测量处理，不能以字数估算：优先保留可同时容纳的首尾标签，然后按数值顺序放入不碰撞的中间标签；无法容纳的标签隐藏，但保留刻度点。
- 首尾标签自身仍碰撞时各分配一半可用标签槽并省略，记录真实溢出；不撑大根宽度，不把标签覆盖到数字输入框上。
- 标签显示策略不因当前值变化而重新排序，避免拖动时文字抖动。主题、字体、尺寸、方向、domain、marks 或可用空间变化后重测。
- marks 是调用方提供的有界展示数据，不自动按每个 step 生成标签。小 step 不应创建几十万个元素。

## 11. 几何与布局

### 11.1 尺寸建议

下表是**待进入源 token 的默认档位设计**，不是在 Rust 中硬编码的许可。长度单位均为逻辑像素；高 DPI 以最终物理像素对称吸附验证。

| 档位 | 轨道厚度 | 圆滑块直径 | 最小交互厚度 | 配套输入 |
| --- | --- | --- | --- | --- |
| Small | 4 | 12 | 28 | Small |
| Medium（默认） | 4 | 16 | 32 | Medium |
| Large | 6 | 20 | 40 | Large |

轨道和填充均为胶囊，滑块为圆，焦点环外绘且不改变布局。刻度点置于轨道中心；滑块必须覆盖自身附近刻度，避免刻度穿过圆心。

### 11.2 外部尺寸与内部映射

- 默认水平组合宽度 320，垂直组合高度 180；由组件 token 提供，可显式覆盖。没有 `.block()` 时不无条件拉满父卡片。
- `.width` / `.height` 定义整个组合的外框，包括存在时的数字输入与标签；轨道从余下空间求解，不把一个默认固定宽轨道塞进更小外框。
- 水平：轨道区弹性占剩余宽度，数字输入在右；Range 两个等宽字段。字段由 `--slider-input-min-digits` 与当前字体实测确定宽度，间距由 `--slider-input-gap` 提供；Buttons 不计入 Chevron 占位，真实编辑器数值居中。
- 当右侧输入会使轨道的可用主轴短于 96 时，整组数字输入移到下行；不压缩到无法命中、不溢出卡片。再小的可用宽度按宿主最小布局合同处理。
- 垂直：轨道与刻度标签构成主区；数字输入在下方。Range 字段在空间足够时横排，不足时换行；数字区计入组合高度，默认 / 显式 height 在该模式下作为最小高度，必要时随字段换行增长。标签在独立 lane，不挤入拖动路径。
- 数字字段保留完整实测宽度，不通过压缩编辑区来容纳 Range；字段组最多占根宽度，放不下时换行。默认最小根宽度至少容纳一个字段；宿主仍需遵循最小宽度合同。
- Buttons 模式轨道组的最小主轴空间包含最短轨道、thumb hitbox inset、两个按钮和两处间距；Small / Medium / Large 默认分别为 180 / 192 / 216px，右侧数字字段放不下时移到下行。刻度标签的 inset 同步计入端部按钮，不使标签映射到另一段长度。垂直模式必要时增长，不挤压实际轨道。
- 交互区主轴两端保留至少“滑块半径 + 焦点环宽度 + offset”的空间；**映射轨道从 min 的圆心到 max 的圆心**。视觉端帽、标签边缘、外框都不是数值坐标边界。
- `SliderIndicator` 测量这个固定映射区；选中填充是其上的独立绘制层，不能把会随值改变宽度的填充当测量器。
- 稳定坐标计算一次供轨道、thumb、marks、提示 anchor 使用；用同一 scale factor 对圆心对称吸附，避免端点与刻度产生一像素偏差。
- 先解除正确 flex 子项的自动最小尺寸，再分配空间；不以 `.w_full()` 修复所有宽度问题，不剪裁外侧焦点环。

## 12. 状态外观、动效与 Tooltip

| 状态 | 反馈 |
| --- | --- |
| Default | 弱轨道、强调填充、表面色滑块与明确边框 |
| Hover | 活动滑块边框 / 轻量外晕增强；不移动数值、不改变主轴宽度 |
| Pressed / Dragging | 中心轻微缩放滑块视觉体，提示当前值；保持锚点和 hitbox |
| Focus-visible | 活动滑块外侧完整焦点环，不能与强调填充融为一体 |
| Disabled | 分别使用禁用轨道 / 填充 / thumb / 文本 token，保留选中区间可辨识性 |
| 输入错误 | 错误只标记该数字字段及说明；不把合法的滑块当前值染成错误值 |

禁用不只是整棵树降低 opacity；不得因在白底上双重混色而让滑块消失。浅色、深色、Neutral / 其他 accent、背景不同的卡片都要验证。

**跟手位置与数值无延迟。** pointer、键盘和外部赋值后，thumb 与填充立即绘制最新规范位置，不给位置套 300ms tween。否则会制造跟手迟滞，并让显示值与位置暂时不一致。

hover / pressed 的颜色、外晕和视觉缩放使用公共 interaction / exit duration 与 easing，经 Slider token 引用；不复制早期菜单为感知实验设置的长入场时长。按压缩放为 0.90，回弹无 overshoot。原设计 0.94 在 100% 显示缩放下会被偶数物理像素取整吞掉，小 / 中尺寸不产生可见缩小；实施时调整源 token 为 0.90，三档均围绕中心缩小，命中区不变。reduced motion 时停用装饰插值，保持立即的状态差异、数值更新和焦点可见性。

Tooltip 复用 `YueTooltip` 与现有 Popup 层：

- 默认只显示活动 thumb 的一个数值提示，不为两个端点争抢窗口的共享显示槽。
- 水平优先上方，垂直优先侧方；由既有 Positioner 避让窗口、翻转和限制边界。
- hover 按公共 show delay；拖动 / thumb 键盘焦点立刻显示；disabled、失活、卸载时关闭。
- 浮层锚在 thumb 圆心对应的稳定 bounds，随最终位置更新；数字变化只更新同一 owner 的内容，不重新开始 show delay / enter 动画，不抢焦点。
- 当前 Tooltip 已有 Manual / controlled open 和动态配置同步；实施须增加连续更新文字及移动 anchor 的回归测试，确认不会重置动画。若缺口存在，最小扩展既有 Tooltip controller，不创建 Slider 私有定位层。
- Escape 后保持关闭直到离开后重新 hover、再次聚焦或开启下一次交互；不能因为下一帧数值 / render 同步立即重开。

## 13. Token 设计与来源

源头仍为 `D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens`。本次调研未找到已登记的 `--slider-*`；以下是**拟新增的组件覆盖点**。

实施时在源库 `src/component-tokens/slider.css` 登记，并纳入索引、catalogue / contract、测试与同步。值引用现有 semantic / primitive；确实缺少尺寸 primitive 时先补源 primitive，不在组件 token 中散写无法追踪的常量。

| 拟新增 token | 角色 / 建议来源 |
| --- | --- |
| `--slider-track-height-{sm,md,lg}` | 轨道厚度；源 size scale |
| `--slider-thumb-size-{sm,md,lg}` | 圆形滑块直径；源 size scale |
| `--slider-hit-size-{sm,md,lg}` | 交互厚度；与控件高度档位一致 |
| `--slider-width` / `--slider-height` / `--slider-min-track-length` | 默认组合尺寸与换行阈值，见第 11 节 |
| `--slider-radius` / `--slider-thumb-radius` | `--radius-full` |
| `--slider-track-background` | 中性、可辨识的组件轨道语义 |
| `--slider-fill-background` | `--control-accent` |
| `--slider-thumb-background` / `--slider-thumb-border-color` | 表面与控件边框语义 |
| `--slider-thumb-border-color-hover` / `--slider-thumb-border-color-active` | 强调边框语义，不能依赖 raw hex |
| `--slider-thumb-border-width` / `--slider-thumb-shadow` | border / shadow scale；阴影可为 none |
| `--slider-tooltip-gap` | `var(--space-4)`；相对可见滑块定位后的箭头 tip 间距，另外保留焦点环 clearance |
| `--slider-thumb-halo-color` / `--slider-thumb-halo-size` | 活动反馈；颜色由强调色混合得到 |
| `--slider-disabled-track-background` / `--slider-disabled-fill-background` | 分离的禁用轨道与选中段 |
| `--slider-disabled-thumb-background` / `--slider-disabled-thumb-border-color` | 禁用滑块仍有清晰轮廓 |
| `--slider-mark-size` / `--slider-mark-color` / `--slider-mark-color-active` | 刻度尺寸、未选与已选颜色 |
| `--slider-snap-distance` | `var(--space-4)`；局部指针磁吸半径，按 rem 缩放，密集刻度另限至最近间距的四分之一 |
| `--slider-label-color` / `--slider-label-color-active` / `--slider-label-color-disabled` | 文本前景语义 |
| `--slider-label-font-size-{sm,md,lg}` / `--slider-label-line-height-{sm,md,lg}` | 源 control typography |
| `--slider-label-gap` / `--slider-label-min-gap` | 标签到轨道及相邻标签的间距 |
| `--slider-input-gap` / `--slider-input-min-digits` / `--slider-stepper-gap` | 输入组间距、实测最少位数、两端按钮与轨道命中区间距 |
| `--slider-focus-ring-color` / `--slider-focus-ring-width` / `--slider-focus-ring-offset` | 公共 focus ring 语义 |
| `--slider-state-duration` / `--slider-state-ease` / `--slider-release-duration` | 公共 interaction / standard easing / exit role |
| `--slider-thumb-scale-pressed` | 单位比例 0.90；只缩放视觉体 |

Slider 不复制 Tooltip 的背景、阴影或进退场 token，也不复制 NumberInput 的边框 / 内部 padding。复用组件的视觉合同由各组件自己维护。

源库 `Yue-Design-System/packages/tokens/src/component-tokens/slider.css` 已登记并由 `_index.css` 引入，词汇表及四 profile 测试同步增加。经 `tools/sync-yue-theme.mjs` 同步到本项目 token 快照及 `theme/generated.rs`，消费者通过 `theme::keys` / token adapter 读取；没有手工维护第二套色值。新增 token 的相关测试通过，源库整体治理失败另见第 18 节。

## 14. Showcase 规划

侧边栏组件组顺序为 `… Radio → Segmented → Slider → Switch → Tooltip`，设计组保持独立。已新增 `pages/slider.rs`，继续使用已有左右布局和页面滚动结构。

页面以能直接操作和比较为目标：

1. **交互预览**：单值 / 范围、方向、尺寸、disabled、Tooltip、数字输入、marks；展示当前值与 Change / Commit 计数。
2. **基础矩阵**：Single / Range × Horizontal / Vertical，包含默认与禁用；不把垂直条横向拉满。
3. **刻度与标签**：数字、温度、自定义短语、与 step 不对齐的标记；窄容器展示碰撞处理。
4. **精确输入**：一个 / 两个数字字段，负数、小数，展示无效草稿、恢复及提交行为。
5. **数值边界**：非零 min、非整除 step、step 大于跨度、重合端点、端点键盘操作。
6. **主题与布局**：浅 / 深、Neutral 与 accent、固定宽 / block、父级窄宽 / 高 DPI；焦点环不可裁剪。
7. **宿主同步**：外部静默重置、修改 domain、拖动时外部替换，验证不会收到旧事务的 Commit。

不复制截图网站的 coverage 徽章、源码复制按钮、贡献者区或大面积固定高度空白。交互提示明写数字输入的 Tab 边界，不能承诺当前 Base 不提供的行为。

## 15. 测试、回归与验收

### 15.1 修改前先建立基线

当前没有 `YueSlider`，不能先写“现有滑动条全部通过”。记录 Base 已有线性 / 对数映射和 retained rendering 测试结果，再补能复现第 2.2 节缺口的失败用例，随后修复。

Base 的共享消费端至少有 ColorPicker hue / alpha 和 Showcase Layout 模拟宽度滑块。实施前记录其既有行为；通用修复后重跑，不能只证明新组件可用。

### 15.2 测试矩阵

| 层级 | 必须覆盖 |
| --- | --- |
| 纯数值 / 几何 | min 对齐、负数、小数、非整除 max、s 大于跨度、越界、倒序范围、finite 校验、浮点可表达性、零长度、水平 / 垂直映射 |
| Base 行为回归 | 指针吸附位置与数值一致；同值不通知；范围轨道连续拖动；外部值保留合法精度；取消后旧 Release 无效 |
| Yue state / 事件 | Single / Range 模式固定，silent setter，同值 echo 不取消，Change / Commit 次数及顺序，回调重入，外部替换与 domain 原子变更 |
| 指针 / 触摸 | 真正 dispatch 按下、移动、外松；未移动、重合、越过另一端、最小 / 最大、取消、禁用中断、其他控件拖动不被误清 |
| 键盘 / 焦点 | 从已获焦状态发送按键；所有调节键、repeat / keyup、边界、两个 Tab stop、disabled 后不改值、输入编辑上下文不被 Slider 劫持 |
| 无障碍结构 | 每 thumb 角色、名称、数值、动态 min / max、方向、disabled；无重复 root Slider；动作保留 Range 并发正确事件 |
| 数字输入 | 合法 / 非法草稿、Enter 后 blur 去重、Escape、域更新、另一端点约束、拖动取得编辑权、单位不被解析为数字 |
| 布局 / 呈现 | 首 / 中 / 末位置、长标签与碰撞、窄宽换行、垂直输入、hitbox 不随缩放改变、焦点环外绘、不同档位及主题 |
| Tooltip | 活动端点唯一、hover 延迟 / focus 即时、拖动动态值不重入场、移动锚点、边界避让、Escape 不自动重开、失活关闭 |
| 现有消费端 | ColorPicker 非整数 hue 精确同步、alpha、拖动中替换 / 禁用；Layout 的宽度调整；已有 Button / Input 等不受 keybinding 影响 |

纯测试与 headless `VisualTestContext` 证据分开报告。语义节点断言不是读屏实测；逻辑像素断言不是高 DPI 像素验收；直接调用内部 setter 不能代替鼠标 / 键盘分发用例。

### 15.3 实施后的自动门禁

按实施时实际 token 工具参数执行双模式同步检查；保留源设计系统聚合门禁原有失败项的独立记录，不把本项目通过当成源库全绿。Rust 门禁沿用：

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo test -p yue-component --features headless-tests
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --features headless-tests -- -D warnings
cargo check -p windows-showcase
```

Base vendored crate 自身的新增单测应按该 crate 的 manifest / features 单独执行并记录；不能假定 workspace 命令一定覆盖所有 vendored 测试。使用本项目已有工具链与 CI 配置，不在此文档臆造测试数量。

### 15.4 Windows 实窗验收

用包含本次实现的 release Showcase，验证 100% / 125% / 150% / 200% 显示缩放、不同字体 / 主题、轨道端点和焦点环、连续拖动帧率、触摸与父滚动区竞争。经 Windows UIA / Narrator 检查单值和双端点名称、可用性、值及增减。无硬件或原生能力时标记未验证，不以无头通过替代。

已完成当前 Windows 显示配置下的 release 实窗基础冒烟：单值轨道外拖动 / 松开、范围不交叉、键盘调值与 Change / Commit 计数，以及 UIA 可读取的名称和值。不同 DPI 矩阵、像素级比对、Narrator 和硬件触摸验收尚未执行；无头测试不替代这些结论。完整记录与边界见第 18 节。

## 16. 实施顺序

1. **补失败测试与锁定精度合同**：Base 缺口、ColorPicker / Layout 基线，特别是外部赋值保精度与用户 step 吸附的区别。
2. **修 Base 机制**：domain / 数值不变量、去重、范围轨道拖动、取消、逐 thumb 焦点 / 键盘 / 无障碍；更新 `vendor/gpui-base/YUE-PATCH.md`。
3. **登记并同步 token**：源库合同与测试、快照、generated keys；不先在 Rust 堆常量。
4. **实现 Yue state 与呈现**：一个数值源、Change / Commit 事务、布局、刻度、状态反馈；先完成可操作的单值与范围主路径。
5. **接输入与 Tooltip**：草稿 bridge、动态约束、复用浮层；补两个依赖的必要回归测试。
6. **补 Showcase 与最终验收**：自动门禁、消费端回归、Windows 实窗；再将本文从设计稿更新为实际 API 与明确验证边界。

## 17. 依据

以下以当前工作区文件为准；仓库有未提交改动，不能把本地行为都归为上游 0.8.0 原版。

- [用户参考图](assets/slider-reference.png)：本组件的功能范围与基础视觉。
- [Base Slider 源码](../vendor/gpui-base/src/slider.rs)：状态、映射、指针、触摸、语义与已有限制。
- [本地 Base 扩展记录](../vendor/gpui-base/YUE-PATCH.md)：ColorPicker 所需的 focus / cancellation 等补丁。
- [Base NumberInput](../vendor/gpui-base/src/number_input.rs)、[InputState](../vendor/gpui-base/src/input/base/state.rs)：数字编辑、步进、约束。
- [Yue Input](../crates/yue-component/src/input.rs)、[Input 文档](03-input.md)：呈现复用与输入能力边界。
- [ColorPicker state](../crates/yue-component/src/color_picker/state.rs)、[panel](../crates/yue-component/src/color_picker/panel.rs)：现有 Base Slider 消费端与独立 hitbox / 视觉体。
- [Layout 示例](../examples/windows-showcase/src/layout.rs)：共享 Base Slider 的另一消费端。
- [Yue Tooltip](../crates/yue-component/src/tooltip/mod.rs)、[controller](../crates/yue-component/src/tooltip/controller.rs)、[Tooltip 文档](10-tooltip.md)、[Popup 文档](12-popup.md)：复用浮层与显示槽的依据。
- [Token 同步工具](../tools/sync-yue-theme.mjs)、[主题 adapter](../crates/yue-component/src/theme/mod.rs)：源 token 到本项目的同步与消费路径。

外部一手参考（2026-11-01 核对）：[WAI-ARIA APG Slider Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/slider/)、[Slider Multi-Thumb Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/slider-multithumb/)。其方向键、Home / End、稳定 Tab 顺序、每端点角色及动态上下界用于交互核对；10% 大步进及事件完成时机是本项目策略，不是 APG 规定。

原生 GPUI / Windows 能否表达这些语义由实际桥接与测试证明；不把网页 ARIA API 名直接当作 GPUI 现有方法。

## 18. 实施与交接（2026-11-01）

### 18.1 已交付与使用入口

- 公共类型、builder、保留状态与事件分别位于 `crates/yue-component/src/slider/{mod,element,state}.rs`，由根模块和 prelude 导出；原有消费端没有增加第二份数值状态。
- `cargo run -p windows-showcase --release` → **组件 → Slider**。示例当前保留 31 个独立 state，覆盖单值 / 范围、水平 / 垂直、三尺寸、禁用、数字输入、两端步进按钮、刻度、Tooltip、四主题、窄布局及外部赋值 / domain 更新；实时展示 Change / Commit 次数。
- 主体为原地、持久、声明式呈现与 retained state 的数值控件，Tooltip 为既有浮层组合。参考图和 HTML 只提供外形及演示交互；不引入网页 DOM、CSS 拖动、全局鼠标状态或第二套弹层。
- Slider token 在设计系统源库登记后同步，涵盖尺寸、色彩、几何、文字、焦点、禁用与动效。按压比例从草案 0.94 调至 0.90：100% 缩放下小尺寸的真实提交绘制几何先复现失败，调整后检查三档均缩小、中心和命中区不变、松开恢复。

### 18.2 共享机制变更与迁移

| 接口 / 行为 | 实施前 | 实施后与原因 | 消费端 / 回归 |
| --- | --- | --- | --- |
| Base `SliderEvent` | Change / Release | 新增 Start / Cancel，完整区分开始、正常完成和取消 | 使用穷尽 match 的消费端需处理新事件；ColorPicker 已适配 |
| Base 数值赋值 | 原值可越界、范围可倒序，位置单独夹紧 | 实际值统一 clamp / order；程序赋值保留非 step 精度 | ColorPicker 非整数 hue 回归；Slider Base 赋值测试 |
| 用户步进 | 相对零量化，末端不可达、位置可能未吸附 | 相对 min，包含非整除 max，数值和位置一致 | 非零 min / 小数 / 超跨度 / 倒序及精度测试 |
| 无效 domain | 可能进入除法或浮点无效状态 | finite、min < max、positive step 与 f32 可表达性校验；无效配置 panic | 构造、set_domain 与 implicit-step 回归；原子配置用 `SliderDomain` |
| Base 范围交互 / 焦点 | 轨道拖动不完整；单根语义 | 最近端点捕获，稳定逐 thumb 焦点与动态语义；Yue 显式启用 thumb semantics | 真指针、键盘、Tab、取消和语义节点测试 |
| YueNumberInput | 未挂载实际编辑器 | 通过 `.input(Base Input)` 挂载真实编辑器，补右侧图标控件与 step bridge | Input 8 项测试、Slider 数字输入测试；旧空白外观不保留 |
| Tooltip | 按压帮助提示默认关闭 | 新增 `dismiss_on_press(false)` 用于 Slider 实时数值；默认仍为 true | Slider 按住 / Escape 后继续拖动回归；Tooltip 原有回归 |

本轮没有新增第三方运行时依赖。更新 vendored Base 时须携带 [补丁记录](../vendor/gpui-base/YUE-PATCH.md)，下游 workspace 仍须自行配置 `[patch.crates-io]`。兼容策略为直接修正机制并更新消费端，不维护错误行为分支。

### 18.3 回归依据

实施前普通组件基线为 125 项单测与 21 项 doctest。新用例覆盖完整用户事件分发，而非直接调用事件回调：按住 thumb 边缘不跳值、轨道按下继续拖、轨道外移动 / 松开、范围重合 / 不交叉、同值去重、往返起点不提交、按键 repeat 完成、外部替换 / 禁用 / 失活 / 触摸取消、输入草稿移交、Tooltip 抑制及窄 / 垂直几何。

特别增加最终 MouseUp 才发生移动、宿主在 Change 回调替换值的失败回归；修复为事务代次校验并推迟 Commit，防止同一 Base update 的 Change + Release 产生陈旧提交。ColorPicker 的 35 项回归覆盖共享 Slider 保精度及 Commit 去重。Base 独立 manifest 的单测单独执行，不将 workspace 测试误称覆盖 vendored 单测。

另以 `step = f32::MAX` 复现超跨度步长将 min 错吸附到 max 的失败。容差改为以实际 domain 跨度计算，不按任意大的 step 放大，同时保留小数中点向大值的规则；新增断言验证 min / max、区间中点与两侧均遵守端点规则。

### 18.4 自动验证与限制

构建 / 测试日志保留在本地 `target/slider-*.log`，不作为产品文件提交。本轮还修正了整仓验证中的两个编译阻塞：Dialog 用真实存在的条件滚动 API 代替不存在的 `overflow_visible`，Base Modal 的单测补上所调用方法需要的 trait import；没有更改 Dialog 的业务状态机或放宽失败断言。

| 本项目命令 | 结果 |
| --- | --- |
| `node tools/sync-yue-theme.mjs --check` | PASS；`Yue GPUI theme mapping is current` |
| `node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check` | PASS；源文件和 payload 与快照一致 |
| `cargo fmt --all -- --check`、定向 vendored Rust 格式检查、`git diff --check` | PASS；另检查未跟踪的新增 Slider 文件，无行尾空白 |
| `cargo test --workspace --locked` | PASS；普通组件与 doctest 通过 |
| Slider integration / workspace headless 中的全部 integration targets | PASS；Slider 29、Input 8、ColorPicker 35，全部 9 个 integration target 共 149 项通过 |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | PASS，exit 0 |
| `cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings` | PASS，exit 0 |
| `cargo check -p windows-showcase --locked` | PASS，exit 0 |
| `cargo build -p windows-showcase --release --locked` | PASS，exit 0 |
| Base 独立 `--lib slider:: --features test-support` | PASS，12 passed；含旧 linear / logarithmic、精度、domain 和刻度中点回归 |
| `cargo test --workspace --locked --features headless-tests` | PASS，exit 0；最新工作区组件单测 222、9 个 integration target 共 149 项及组件 doctest 23 项均通过，workspace 其余目标也通过 |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests` | PASS，exit 0；系统字体模式下组件单测 222、integration 149、doctest 23 项均通过 |

工作区的 Dialog 在并行开发期间，早一轮整仓验证曾被拖动边界和 Enter 策略两项断言阻塞。最新工作区重新执行上述两种全量命令均通过，这两项断言也通过；本次未删除、跳过或削弱断言。Slider 的集成回归同时独立执行，确保不会因 lib test 提前失败而漏跑。

设计系统源库当前登记 53 个 Slider token（首轮 52 个，视觉调整增加 `--slider-tooltip-gap`）、词汇、逐项 package-only 登记、README 消费合同及相关测试，不额外创建 Vue Slider、locale catalog、SSR 或 npm 发布入口；这些 Vue 组件项不属于本次 GPUI 组件实现。首轮的 `audit:tokens` 与 `verify:all` 已实际执行：

| 源库命令 | 实际结果 |
| --- | --- |
| `corepack pnpm exec vitest run tests/token-slider.test.mjs tests/token-color-picker.test.mjs tests/token-segmented.test.mjs` | PASS：3 files / 40 tests |
| `corepack pnpm audit:tokens` | FAIL，exit 1；1190 个 token / 48 个可达文件，unreachable / undeclared / duplicates / cross-file slots / layer violations 均为 0；冻结原型 parity 和 Switch / Radio / Checkbox 等既有词汇问题未清零 |
| `corepack pnpm verify:all` | FAIL，exit 1；dark-opacity、interviews、typecheck、build 先通过，测试为 782 passed / 11 failed（5 files）；因聚合 test 失败，后续门禁未执行 |

这 11 个失败在本轮前保留的源库基线中也存在（`target/color-picker-slider-source-verify.log`），集中于 API 文档、架构文档 / inventory、prototype parity、词汇及 token usage 报告。新 Slider 名称已逐项登记且具外部消费说明；冻结 inventory / 报告仍滞后于当前源 token 集，也包含本轮新增项。这里只报告现状，不把失败断言改成通过，不将本项目自动验证通过等同源库全绿。源库全量治理对齐需另行处理。

**未验证边界**：像素级渲染比对、真实 100% / 125% / 150% / 200% DPI 矩阵与字体排版、Narrator 播报 / UIA 增减动作、硬件触摸与滚动竞争、release 实窗帧率。无头测试查询原生属性并分发鼠标、键盘、raw TouchEvent，未注入 Windows UIA 增减动作；提交绘制几何不是 GPU 截图。首版不自动跟随 Windows 强制对比度，也不声明新增了一套自动本地化系统；应用可传可访问名称、端点名称、格式化和错误文案。

### 18.5 Windows 实窗冒烟

通过 computer-use 在本轮 release 产物的临时副本运行真实窗口，鼠标 / 键盘输入经过 Windows，观察画面及 UIA 名称 / Value，而非只读无头快照：

| 操作 | 实际观察 |
| --- | --- |
| 单值从 40 抓住 thumb 后拖至轨道和卡片右下方，再松开 | 数字框与 thumb 到 100；Change 1 / Commit 1，其他示例保持原值 |
| 范围 20..80 抓 lower，向 upper 右侧并移出轨道拖动 | 结果 80..80，端点未交换；Change 1 / Commit 1 |
| lower 保持焦点，按一次 Left | 结果 79..80；Change 2 / Commit 2，输入框同步 |

以上只证明所列路径在当前显示配置中可操作。没有将一次 drag helper 视为逐帧动画或持续帧率测量。实窗检查另发现示例四个配置开关缺少可见文字，现补上“禁用 / 数字输入 / 数值提示 / 刻度”，保留原有可访问名称。

### 18.6 视觉与刻度吸附调整（2026-11-01）

本节记录早先的离散吸附实现；交互合同由 18.7 的局部磁吸修正取代。

- 滑块主题色边框从 1px 加粗为 2px，由源 token `--slider-thumb-border-width: var(--border-2)` 控制；三档直径和 hitbox 保持原有定义。
- Tooltip 的定位 anchor 改为可见滑块的正常直径，不再把整个 28 / 32 / 40px 命中区当作视觉外缘；独立 `--slider-tooltip-gap` 为 4px，并保留 4px 焦点环 clearance。默认箭头深度 4px，因此提示主体到滑块表面的逻辑间距约为 12px；窗口边缘翻转后同样适用。按压只缩放绘制，不缩放 hitbox 或使提示跟着抖动。
- YueNumberInput 的增减控件改用 Tabler `chevron-up` / `chevron-down`。Slider 的右侧控件保持上增下减，既有 `on_step`、焦点和禁用行为不变。
- Base 统一拥有离散刻度选择、相邻刻度、范围夹紧与策略切换取消；Yue 仅把已有 marks 交给 Base，不另存一份数值。提供 `.snap_to_marks(false)`，供只需装饰性刻度的调用方使用。
- 回归先复现带 marks 的拖动仍落在 step 网格的问题，再验证修复；新增覆盖跨 step 的精确刻度、键盘和输入、上下按钮顺序、隐藏刻度、垂直范围、切换策略取消、三尺寸双方向的实际绘制边框和 Tooltip 几何。另复现并修正配置切换后数字框仍显示旧值的问题：在组合视图读取数值和构造输入框之前，通过 Base 原子配置吸附策略，再同步字段。
- 最终执行：Slider `29 passed`；workspace 普通 / headless、系统字体模式及两组 `clippy -D warnings` 均 exit 0；Base 独立 Slider `12 passed`；源库 focused tokens `3 files / 40 tests passed`；双模式 token 同步检查、格式和空白检查通过。release Showcase 已重新生成。工作区全量数量见 18.4，不将其他同时开发切片新增的单测算作本次 Slider 新增用例。
- 重新执行源库 `audit:tokens`：exit 1；1191 tokens / 48 files，unreachable / undeclared / duplicates / cross-file slots / layer violations 均为 0，冻结 parity / 词汇仍未通过。重新执行 `verify:all`：exit 1，`5 files failed / 35 passed`、`11 tests failed / 782 passed`，仍为上述治理与文档断言；报告滞后也包含本轮新增 token，不将其全部归为无关差异。
- 本轮源库只调整 token 消费合同，没有新增 Vue Slider 或浏览器交互。未运行源库的 Vue 截图门禁；GPUI 自动化检查只证明逻辑像素与提交绘制几何。按照用户分工，本轮真实 UI 与交互由用户验收，未自行启动实窗；上一轮 18.5 的实测不能代替本轮视觉验收。

### 18.7 局部磁吸与数字按钮反馈（2026-11-01）

**缺陷与合同修正。** 原实现在整个轨道上选择最近 mark，导致开始拖动就跳到刻度，显示 marks 时还会静默移动当前值。新增回归先得到失败：初值 40 在启用 0 / 20 / 100 marks 后错误变为 20。用户明确要求保留自由拖动，因此替换原离散选择合同，旧测试也按新合同逐项更新，并保留越界、精度、事务和焦点覆盖。

- Base 以 raw pointer 到 mark 的实际主轴距离判断，默认半径由新 token `--slider-snap-distance: var(--space-4)` 提供，随 rem 缩放；密集 marks 的半径不超过最近间距的四分之一。范围外立即恢复 min 对齐 step，不增加锁定 / 延迟释放状态。
- marks / 策略切换只取消旧交互，不重定位当前值；同值 owner echo 在规范化前识别，保留正在拖动和移除刻度后留下的合法当前值。有效的非 step mark 仍精确保留，范围端点仍不交叉。
- 键盘、数字步进及无障碍增减回到 domain step，固定 numeric step 继续发布。Base 程序 setter 仍保留非 step 精度，ColorPicker 同步路径不变。Base 配置方法 / builder 改为同时接受 points 和逻辑像素 distance，属于本项目开发期的合同修正。
- 数字按钮需求经用户明确为“保留 hover 背景，增加点击闪烁”。Yue 增加中性 hover 与按下加深反馈，松开恢复 hover；持续按压保持稳定，禁用不反馈。三个独立 `--input-stepper-*` token 同步源库与本项目快照，详见 [Input 文档](03-input.md)。没有替换 Base click / focus 机制。

**本轮验收。** Slider `33 passed`，包含局部进入 / 退出、不同宽度、密集刻度、双方向、非 step 精度、同值回写、策略取消与数字按钮真实绘制状态；Base 独立 Slider `13 passed`。workspace 普通 / headless、组件系统字体模式均 exit 0；两组 workspace Clippy `-D warnings`、格式与双模式 token 同步检查通过。源库 focused tokens `4 files / 44 tests passed`。日志为 `target/slider-local-*.log`。

源库全量检查也重新执行：`audit:tokens` exit 1，1195 tokens / 48 reachable files，unreachable / undeclared / duplicates / cross-file slots / layer violations 均为 0，但冻结 parity / 词汇未通过；`verify:all` exit 1，`5 files failed / 36 passed`、`11 tests failed / 786 passed`，为治理 / 文档 / report 断言，包含本轮新增 token 带来的快照差异，不能声称全部无关或全量验收通过。

release Showcase 已重新生成，入口仍为 `target/release/windows-showcase.exe` → **组件 → Slider**。本轮未自行启动真实窗口，视觉强度和真实鼠标手感由用户验收；没有将逐事件无头绘制断言等同于实窗逐帧测量。未提交 Git。

### 18.8 数字框横排与可读宽度修正（2026-11-01）

本节保留上一轮实现记录；其中 128px 宽度及编辑器对齐由 18.9 的紧凑居中合同取代。

用户截图显示右侧数字框过窄，大尺寸数值与 Chevron 重叠。根因是三档均消费 64px 的 `--slider-input-width`，并由 Slider 强制开启右侧竖排；大尺寸的左右 padding 和图标留给真实编辑器的空间不足。修改前新增几何回归，首先在 `Small/16` 得到“两个控件必须同一行”的失败，日志为 `target/slider-input-layout-before.log`。

修正直接使用 Base 已有的默认横排结构：左 ChevronDown 减小、中间真实编辑器、右 ChevronUp 增大；不重新实现编辑器或改换 glyph。源 token 宽度改为 `calc(var(--space-64) * 2)`，即 128px，并重新生成 GPUI 快照，不在 Rust 中散落尺寸常量。hover / pressed、焦点、禁用、步进及 Commit 仍走原有路径。

同步处理加宽后的 Range 窄布局：修改前的补充回归在 240px 根宽度上发现第二个字段溢出，日志为 `target/slider-input-range-wrap-before.log`。字段组现允许换行、约束最大宽度，单个字段不收缩；垂直组合至少容纳一个字段，按最小高度加内容自然增长，避免换行字段落到根 bounds 之外。

新增覆盖 Small / Medium / Large × 16 / 20 / 24px rem 的控件同排与可读宽度；独立 Input 测试通过真实 `InputState::input_bounds()` 检查编辑器处于两按钮之间、没有交叠、负小数文本保留且编辑区足够宽；240px 的 Single / Range、水平 / 垂直及原有拖动、反馈回归继续检查。对应本轮日志前缀为 `target/slider-input-layout-*`，真实编辑器几何日志为 `target/slider-input-editor-geometry.log`。

实际自动验收：Slider `35 passed`、Input `9 passed`；workspace 普通 / headless、组件系统字体模式、两组 workspace Clippy `-D warnings` 均 exit 0，格式与双模式 token 同步检查通过。源库 focused tokens `4 files / 44 tests passed`；重新执行源库 `audit:tokens` / `verify:all` 仍 exit 1，后者为 `11 failed / 786 passed`，治理 / 文档 / report 问题与 18.7 同类，不声称全量源库门禁通过。未运行真实窗口或 Vue 截图验收，本轮截图问题的实际观感由用户在更新后的 release Showcase 中复核。未提交 Git。

### 18.9 紧凑居中数字框与互斥两端按钮（2026-11-01）

本节保留上一轮记录；固定宽度、Buttons 无字段和输入提交时机已由 18.10 修正。

**缺陷与修正。** 上一轮只将数字框加宽到 128px，仍沿用普通 Input 随尺寸增长的横向 padding，真实 `InputState` 也仍是左对齐。修改前分别补充失败回归：`target/slider-controls-width-before.log` 记录 128px 不符合紧凑 96px 合同，`target/slider-controls-centering-before.log` 记录实际 `TextAlign::Left` 不符合居中要求。

- 源库 `--slider-input-width` 改为 `calc(var(--space-48) * 2)`，默认 96px；新增 `--input-number-padding-inline: var(--space-4)`，默认 4px。经同步工具生成本项目快照与 Rust keys，不手改 generated 文件。`YueNumberInput` 将真实编辑器设置为 `TextAlign::Center`，保留左减、中间数值、右增的横排结构。
- 公共 API 改为 `.controls(SliderControls::{None, Input, Buttons})`，默认 None；删除 `.show_input(bool)` 并迁移所有调用方。数字框与两端按钮由同一个枚举选择，不能共存，不保留独立开关或兼容分支。
- Buttons 使用 Tabler Minus / Plus 与既有 Ghost Button：水平左 − 右 +，垂直上 + 下 −，尺寸跟随 Slider。每次有效点击或 Enter / Space 步进一次，复用数字框的 domain step 和 Change / Commit 路径；到达边界时禁用相应按钮，按住不连续步进。Range 默认调节 upper，随后调节最近实际操作的端点，端点不交叉。
- 模式切换取消旧拖动与未提交草稿；隐藏控件持有焦点时，将焦点交回对应 thumb，不产生伪造的 Change / Commit。按钮保留独立焦点，`tab_stop(false)` 同时覆盖按钮和 thumbs。默认可访问名称为端点名称加英文 increase / decrease，不声称自动本地化或 Narrator 实测通过。
- Showcase 配置区用单个 Segmented 切换“无 / 数字输入框 / 两端 −/+”，新增“两端步进按钮”示例，覆盖三尺寸、Range、Vertical、Disabled 和 Change / Commit 计数；当前共 31 个独立 state。

**几何与交互回归。** Input 三尺寸 × 16 / 20 / 24px rem 在实际 96px 默认宽度下检查编辑区、按钮不交叠；通过真实 `input_bounds()` 和 `range_to_bounds(0..5)` 验证负小数 `-99.9` 的文字中点与编辑区中点一致。Range 窄容器收紧到 180px，检查字段换行且不溢出。新增两端按钮测试覆盖鼠标 / Enter / Space、边界禁用、模式互斥、隐藏草稿、焦点恢复、最近端点、双方向刻度映射、外部松手、Tab 顺序和切换模式取消按住拖动。

| 本轮命令 / 检查 | 实际结果 |
| --- | --- |
| Input / Slider 专项无头测试 | PASS：Input 9、Slider 41；日志 `target/slider-controls-final-targeted.log` |
| `cargo test --workspace --locked` | PASS；日志 `target/slider-controls-workspace.log` |
| `cargo test --workspace --locked --features headless-tests` | PASS；日志 `target/slider-controls-headless.log` |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests` | PASS；日志 `target/slider-controls-system-fonts.log` |
| workspace 普通 / headless 两组 `cargo clippy --all-targets -- -D warnings` | PASS；日志 `target/slider-controls-clippy*.log` |
| `cargo check -p windows-showcase --locked` | PASS；日志 `target/slider-controls-showcase-check.log` |
| `cargo build -p windows-showcase --release --locked` | PASS；日志 `target/slider-controls-release.log` |
| 最终 `cargo fmt --all -- --check`、双模式 token 同步检查、`git diff --check` | PASS；加强几何断言后再次执行 headless workspace Clippy `-D warnings` 也通过，日志 `target/slider-controls-final-clippy.log` |
| 源库 Slider / NumberInput / ColorPicker / Segmented focused token tests | PASS：4 files / 44 tests；日志 `target/slider-controls-source-focused.log` |
| 源库 `audit:tokens` | FAIL，exit 1；1196 tokens / 48 reachable files；unreachable / undeclared / duplicates / cross-file slots / layer violations 均为 0，冻结 parity / 词汇未通过 |
| 源库 `verify:all` | FAIL，exit 1；5 files failed / 36 passed，11 tests failed / 786 passed，为治理 / 文档 / report 断言 |

源库失败也包含本轮新增 padding token 对冻结快照的影响，不能归为全部无关或声称全量门禁通过。源库日志为 `target/slider-controls-source-{audit,verify}.log`。本轮未修改 vendored Base，也未创建其独立 Cargo.lock。

release 产物为 `target/release/windows-showcase.exe`，关闭旧窗口后启动，进入 **组件 → Slider → 两端步进按钮**。本轮未自行启动真实窗口，也未运行源库 Vue 截图门禁；高 DPI、真实字体渲染、Narrator 和鼠标手感仍由实窗验收确认。未提交 Git。

### 18.10 内容宽度、无箭头字段与实时输入（2026-11-01）

用户明确修正 18.9 的合同：Buttons 采用“− / 轨道 / + / 无箭头数字框”，只有字段内部的 Chevron 与两端按钮互斥，数字编辑能力保留。合法输入立即更新 Base 数值与轨道，Enter / 失焦只结束编辑事务；非法、非有限、超出 domain 或另一端点的草稿保留，边框与数字使用错误色，轨道保留最近合法值。Enter 不接受非法值；Escape / 失焦恢复最近合法值，已有合法实时变更最多提交一次。模式切换取消旧事务但保留已生效的预览值。

不再以固定 96px 决定字段宽度：使用当前字体和尺寸实际测量至少三位数字，并计入 domain 的符号、小数及端点宽度、数字专用 padding、边框、光标与可选 Chevron。非法草稿不撑宽控件。删除源 `--slider-input-width`，替换为 `--slider-input-min-digits: 3`；新增独立 `--slider-stepper-gap: var(--space-0)`，与轨道安全区共同控制相邻步进按钮，输入组仍消费 `--slider-input-gap`。

这是用户授权的开发期行为与 token 合同修正，不保留旧固定宽度和 Enter 后才变更的分支。修改前已复现两项失败：`target/slider-live-before.log`（输入 75 后仍为 40），`target/slider-buttons-field-before.log`（Buttons 没有数值字段）。另在 `target/slider-measurement-before.log` 临时恢复固定 96px 进行负向探针，`numeric_width_fits_all_three_digit_values_and_adds_only_the_visible_controls` 在 Small / 16px rem 明确失败：实际 96px、内容所需 57px；随后逐字节恢复源文件，未保留探针。

#### 合同变化与回归

| 变化面 | 旧合同 | 新合同 / 原因 | 回归证据 |
| --- | --- | --- | --- |
| Buttons | 无输入框 | 端部 −/+ 后仍可直接输入，只隐藏内部 Chevron | `button_controls_include_a_plain_numeric_editor_after_the_plus`、Tab 进入与跳过测试 |
| 字段宽度 token | `--slider-input-width` 固定 96px | `--slider-input-min-digits: 3`；内容决定宽度，符号 / 小数 / 较长 domain 自动扩容 | 三尺寸 × 16 / 20 / 24px rem 遍测 `000..999`，符号小数、窄布局回归 |
| 端部间距 | 复用 8px 输入间距 | 独立 `--slider-stepper-gap: 0px`；按钮与端点 thumb 命中区相邻，避免叠加留白 | 双方向、三尺寸在 min / max 的实际 bounds 断言；刻度与拖动映射回归 |
| 数字 Change | Enter / blur 才生效 | 每个合法文本变更立即经 Base `set_user_value` 更新数值；不覆盖正在编辑的原始文字 | `numeric_edits_update_the_track_immediately_and_reject_out_of_range_drafts` |
| 越界草稿 | 提交时夹紧到边界 | 文字与边框标红；Enter 保留错误，Escape / blur 恢复最近合法值 | `invalid_live_drafts_restore_the_last_valid_prefix_on_blur_and_commit_once`；真实绘制边框与可访问描述断言 |
| 数字 Commit | 一次提交对应一次赋值 | Enter / blur 结束实时事务；同值 echo 保留草稿，外部替换 / 禁用 / 模式变化取消旧事务 | `live_edits_keep_the_draft_on_echo_but_external_replacement_cancels_the_transaction`，原事务、取消与事件去重回归 |

原有固定宽度、Buttons 无 SpinButton、越界夹紧和“合法草稿尚未写回”的断言已按用户新要求迁移，未删除测试或削弱其他交互约束。源码新增 `YueNumberInput::controls(bool)` / `tab_stop(bool)` 与对应 Base 呈现开关；默认仍显示数字框箭头，不影响独立 NumberInput。没有新增第三方依赖、第二个 canonical 数值或全局鼠标处理器。

输入框向 thumb 转移指针操作时，旧 Blur 不能调用 Base Release 而取消刚取得的 pointer capture：字段放弃原始文字，连续实时事务由拖动松开完成一次 Commit。该路径由 `dragging_discards_old_numeric_draft_and_range_inputs_do_not_cross` 覆盖。逐键输入的合法前缀与一次性粘贴分开测试：`invalid_paste_is_atomic_and_a_legal_multidigit_minimum_can_still_be_typed` 覆盖多位 min 和真正的剪贴板 Paste 路径。

#### 验收与边界

`000..999` 的自动测试遍历三档尺寸与三档 rem，验证字段只计入所需字形、光标、padding、边框及可见箭头；另覆盖负小数和非法长文字不撑宽。测试中的默认逻辑字体在 16px rem 下得到：

| 尺寸 | 三位数字字形宽度 | 带 Chevron 的字段 | 无箭头字段 |
| --- | --- | --- | --- |
| Small | 21.6px | 57px | 33px |
| Medium | 25.2px | 65px | 37px |
| Large | 28.8px | 72px | 40px |

以上是 headless 字体与逻辑像素证据，不是 Windows GPU / 字体实测尺寸，也不是新硬编码值。release 根据实际继承字体和 Input 字号重测。本轮只设置 `InputEditorStyle.foreground`，尚不足以改变 Base 普通 TextRun 的实际数字颜色；此缺陷由 18.11 修复。该轮自动化检查实际绘制的错误边框、值、描述、事件和几何，没有截取 GPU 字形像素或宣称已完成 Narrator 验收。

| 本轮验证 | 结果与日志 |
| --- | --- |
| 最终 Slider / Input 专项 | PASS：Slider 47、Input 9；`target/slider-live-final-targeted.log`，含真实剪贴板粘贴及全部三位数字组合 |
| workspace 普通测试 | PASS；`target/slider-live-workspace.log` |
| workspace headless / 组件系统字体模式 | PASS；`target/slider-live-headless.log`、`target/slider-live-system-fonts.log` |
| 两组 workspace Clippy `--all-targets -- -D warnings` | PASS；`target/slider-live-clippy.log`、`target/slider-live-clippy-headless.log` |
| Showcase check / release build | PASS；`target/slider-live-showcase-check.log`、`target/slider-live-release.log` |
| 最终一致性复查 | PASS：workspace fmt、定向 vendored NumberInput rustfmt、双模式 token 同步检查与 `git diff --check`；新增 Paste 用例后再跑 headless workspace Clippy 与 release build 均 exit 0，日志 `target/slider-live-final-{clippy,release}.log` |
| 源库 focused tokens | PASS：4 files / 44 tests；`target/slider-live-source-focused.log` |
| 源库 `audit:tokens` | FAIL，exit 1：1197 tokens / 48 reachable files；unreachable / undeclared / duplicates / cross-file slots / layer violations 均为 0；冻结 parity / 词汇仍未通过 |
| 源库 `verify:all` | FAIL，exit 1：5 files failed / 36 passed，11 failed / 786 passed；`target/slider-live-source-verify.log` |

源库失败包含本轮 token 替换与新增带来的冻结快照差异，不能声称全部无关；治理 / 文档 / report 需后续单独对齐，没有放宽失败断言。其聚合命令因测试失败未进入后续门禁；本轮未运行 Vue 截图门禁，亦未新增 Vue Slider，Vue/DOM/SSR/locale catalog 不属于本次 GPUI token 消费切片。

本轮真实 UI 仍由用户测试，未自行启动窗口。未覆盖 Windows DPI 矩阵、GPU 像素比对、真实字体回退、Narrator / UIA 原生动作和硬件触摸。本轮不创建独立 vendor lock，也未提交 Git；复核入口为更新后的 `target/release/windows-showcase.exe` → **组件 → Slider**。

### 18.11 错误文字色、轨道对齐与数值动画（2026-11-02）

本节修正用户在真实窗口中发现的三个问题，不改变公开 API 或新增 token。

- **错误文字色。** 越界草稿继续保留，边框使用 `--input-border-color-invalid`；数字通过 NumberInput 根节点继承 `--error-foreground`。原先只改 editor foreground，Base 的普通 TextRun 仍使用父级颜色，导致边框红而数字不红。修复也适用于独立 YueNumberInput，合法输入恢复正常色。
- **垂直对齐。** 水平 frame 的交叉轴改为起点对齐。数字框高度与同尺寸 Slider hit 区一致，二者中心直接对齐；刻度标签位于轨道下方，不再拉低旁边字段。保留窄容器换行和垂直 Slider 的布局，不增加补偿像素。
- **数值动画。** 合法输入立即更新 Base 数值、原生节点数值和 Change；可见 fill、双端点及其命中 / Tooltip anchor 共用一个百分比插值。复用 `--slider-state-duration`（当前四种主题均为 100ms）与 `--slider-state-ease`（`cubic-bezier(.2, 0, .38, .9)`）。连续输入从当前采样位置转向，动画帧不生成 Change / Commit。数字框与两端按钮的步进使用相同呈现路径。
- **直接操作与取消。** 拖动不插值，立即跟手；抓住尚在运动的 thumb 时，Base 以实际已布局中心保留抓点偏移，并将该端点接管到可见位置，按 domain step 量化，避免跳到旧目标。触摸也使用实际几何。外部替换、禁用、domain / 控件模式变化取消旧动效；reduced motion 直接落到目标。

Base 仅增加对实际 thumb 几何的抓取支持和 `test-support` 下的 TextRun 颜色观察，不决定动画曲线、时长、配色或布局。Yue 不保存第二份规范数值，仅保留呈现插值。

修改前已复现：带刻度字段对齐失败（`target/slider-visual-align-before.log`）、数值更新没有中间帧（`target/slider-visual-motion-before.log`）、抓取运动中的 thumb 跳到旧目标（`target/slider-visual-grab-before.log`）；错误色通过移除修复的负向探针再次验证失败（`target/slider-visual-text-color-before.log`）。

专项回归目前为 Slider 55、Input 10，覆盖三尺寸的 Input / Buttons 轨道对齐、真实 TextRun 错误色、数值即时生效与几何中间帧、反向连续输入、水平 / 垂直 Range 的 fill / thumb 一致、鼠标与触摸接管、reduced motion、取消与事件去重。几何证据来自真实组件的 headless 布局与提交绘制场景；不声称已验收 Windows GPU、DPI、真实字体或硬件触摸。本轮真实窗口继续由用户测试，不自行启动程序，不提交 Git。源 token 未变，本轮不重跑源库全量验证；18.10 中记录的源库治理失败仍未宣称解决。

| 最终验证 | 结果 / 日志 |
| --- | --- |
| Input / Slider 专项 | PASS：10 / 55；`target/slider-visual-targeted.log` |
| workspace 普通 / headless | PASS：155 / 437 项（含各自 doctest）；`target/slider-visual-{workspace,headless}.log` |
| 组件系统字体模式 headless | PASS：423 项（含 doctest）；`target/slider-visual-system-fonts.log` |
| workspace 普通 / headless Clippy `--all-targets -- -D warnings` | PASS；`target/slider-visual-clippy.log`、`target/slider-visual-clippy-headless.log` |
| Showcase check / release | PASS；`target/slider-visual-showcase-check.log`、`target/slider-visual-release.log` |
| workspace fmt、定向 Base rustfmt、双模式 token 同步、diff 与本轮未跟踪文件尾空白检查 | PASS |

上述 workspace 不含被 exclude 的 Base 独立单测；直接 `cargo test -p gpui-base --locked --lib slider::tests` 被 Cargo 拒绝，原因是它不是 workspace member 且需要 dev-dependencies（`target/slider-visual-base.log`）。未为此修改工作区边界或生成独立 vendor lock；Base 本轮行为通过真实 Yue Slider / Input 及 workspace 内其他消费者的集成测试验证。
