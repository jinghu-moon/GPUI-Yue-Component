# 09 YueSegmented 设计与实现

状态：V1 已实现（2026-11-01）。基于 GPUI Kit / Base 0.8.0，保持 Base-only 和 Windows 使用范围；组件、44 个源 token、生成快照、Showcase 和自动化测试已落地。真实 Windows GPU、DPI 与读屏验收仍待人工完成，详见第 13 节。

## 1. 目标与参考

在少量、互斥、立即生效的选项之间切换，例如时间粒度、视图模式和定价周期。调用方持有选中值；组件负责选项呈现、鼠标与键盘操作、焦点，以及一个连续移动的选中块。

参考资料：

- 用户提供的本地 HTML demo（原文件名 `GPUI Kit 组件文档 (1).html`，参考资料不随仓库分发）：第 256 行起的 `.seg` 样式、第 653 行的示例和第 926 行起的交互脚本。
- [用户补充的截图](assets/segmented-reference.png)：中性圆角矩形、强调色胶囊、等宽/不等宽、单项禁用与当前值回显。
- [现有 Radio 契约](05-radio.md)、[Button 与图标](02-button.md)、[token 同步与验证约定](08-integration-hardening.md)。

本次设计覆盖：

| 维度 | 选项 | 默认 |
| --- | --- | --- |
| 外观 | Neutral 中性、Accent 强调色 | Neutral |
| 形状 | Rounded 圆角矩形、Pill 胶囊 | Rounded |
| 宽度分配 | Content 按内容宽度、Equal 各项等宽 | Content |
| 尺寸 | Small、Medium、Large | Medium |
| 内容 | 文本、图标加文本、纯图标 | 文本 |
| 禁用 | 整组禁用、单项禁用 | 可用 |

截图中的两种主要搭配为 `Neutral + Rounded` 和 `Accent + Pill`。外观决定颜色，形状决定轮廓，宽度分配决定布局，三个维度分别设置；切换颜色不会偷偷改变尺寸或圆角。

推荐 2–5 项，至少提供两项才有切换意义；数量建议不是运行时硬限制。选项过多或标签过长时由调用方改用 Radio / Select。V1 仅做水平单选，不增加多选、可取消选中、下拉折叠、拖拽选择、loading、readonly、垂直排布或任意自定义内容槽位。

## 2. 与现有组件的关系

| 组件 | 使用场景 | 视觉与语义 |
| --- | --- | --- |
| YueRadioGroup / Indicator | 表单中比较互斥选项 | 单选圆点与标签 |
| YueRadioGroup / Button | 表单中的紧凑单选 | 连续描边、分隔线、各项独立选中填充 |
| YueSegmented | 同一上下文中的模式、粒度或筛选切换 | 一条底轨，一个滑动选中块；RadioGroup / RadioButton 语义 |
| Tabs | 切换有对应面板的内容导航 | TabList / Tab / TabPanel 关系，应由独立 Tabs 组件负责 |
| ButtonGroup | 多个独立命令 | 每个按钮执行自己的命令，无互斥选中值 |

`YueSegmented` 是独立公共组件，不给 `RadioAppearance` 再追加“滑块”外观。保留现有 Radio Button 的边框和分隔线契约。只复用经测试的导航规则；实际实现时如需提取共享逻辑，只在两个控件确实共用的私有边界内提取，不先创建公共选择框架。

## 3. GPUI Kit 0.8.0 源码能力与边界

已核对本机 Cargo registry 中的 `gpui-base-0.8.0` 和 `gpui-component-0.8.0`，以下是源码事实，不是拟新增 API。

| 能力 | 源码位置（相对各 crate 的 src） | 实际提供 | 本方案的使用方式 |
| --- | --- | --- | --- |
| Base Radio | `gpui-base/radio.rs` | 受控 `checked`、RadioButton role、toggled/selected、disabled、名称、位置/总数、FocusHandle、Tab stop/index、原生点击/键盘激活 | 每一项使用 `gpui_kit::base::Radio` |
| Base RadioGroup | `gpui-base/radio_group.rs` | RadioGroup role、orientation、子项与样式；不管理值或箭头导航 | 可作为语义根，组导航由 Yue 补齐 |
| Base ToggleGroup | `gpui-base/toggle_group.rs` | Toolbar 语义容器；没有互斥选择和 roving focus | 不用于本组件 |
| Component ToggleGroup | `gpui-component/button/toggle.rs` | `.segmented()` 连接边框；回调返回 `Vec<bool>`，仍是独立多选 Toggle | 不满足单选滑块契约 |
| Base Tab / Tabs | `gpui-base/tabs.rs` | Tab/TabList 语义；源码明确说明尚未提供完整的 compound keyboard navigation | 不把 Tabs 当成已完成的单选导航内核 |
| Component TabBar | `gpui-component/tab/tab_bar.rs` | `.segmented()` 外观、选项 bounds 测量与指示器动画等实现 | 参考测量思路，不启用 Component 层或复用其 Tab 语义 |
| Base transition | `gpui-base/motion.rs` | 支持 `Bounds<Pixels>` 插值；首次目标立即采用，改目标从当前采样位置续接；运行时请求下一帧；尊重 reduced motion | 选中块使用组级稳定通道 |
| Base spring | 同上 | 支持弹簧及中途改目标；Yue 已有 `motion::settle_spring` | V1 不必引入第二套运动策略 |

Base Radio 不替组件保存业务值。点击未选项才请求 `true`，点击已选项不取消。Base RadioGroup 也没有现成的“选中值 + 跳过禁用 + 方向键循环”组合，需要 Yue 管理。

继续使用当前 Base-only 依赖结构，不为 Segmented 开启 `gpui-kit/component`。图标复用 `yue-component-icons` 的静态 SVG 与原生 GPUI SVG 渲染，无独立资源加载器。

## 4. 从参考中采纳与调整的规则

| 参考表现 | Yue 方案 |
| --- | --- |
| 默认灰底、白色浮块、轻阴影 | Neutral：底轨使用主题容器色，选中块使用主题表面色与阴影，适配暗色 |
| Accent 用主色选中块 | 使用 `--accent-solid` / `--on-accent`，反色文字由 token 决定，暗色不强制白字 |
| 截图 Accent 的浅蓝底轨、蓝色未选文字 | 底轨使用 `--accent-subtle`，未选文字使用 `--accent-subtle-text`；Neutral 强调色配置也可自然工作 |
| 截图胶囊轮廓 | 增加 Pill，底轨与选中块均为胶囊 |
| 截图时间粒度、视图模式各项等宽 | Equal；按布局所得最大内容宽度计算自然等宽，约束宽度下均分可用空间 |
| 截图定价周期两项宽度不同 | Content；每项由实际文本/图标宽度和内边距决定 |
| HTML 整组 disabled | 增加单项 disabled，覆盖截图中的“看板” |
| HTML 存储 `usize` 索引 | 对外使用稳定字符串 value，重排不会选错对象 |
| HTML 左右键、循环、roving tabindex | 保留，并补 Home/End、跳过禁用和动态选项的焦点修复 |
| HTML 首次挂载后才开启 CSS transition | 首次有效测量直接就位，不从第一个选项或零宽度飞入 |
| HTML 依靠 `offsetLeft/offsetWidth` | 读取 GPUI 实际布局，不按字符数或下标估算 |
| HTML 选中标签变粗 | 测量时预留 regular/medium 两种字重的最大宽度，选中切换不改行布局 |

参考 HTML 的 `--hover`、`--text` 和内嵌阴影是 demo 自带的别名和值。本库使用当前设计系统的语义 token，不把 HTML 的整份旧 token 表复制进 Rust。

## 5. 公共 API

所有类型在 `yue_component` 根部重导出；字段保持私有。以下为宿主 `render` 内使用片段，完整可运行用法见 Showcase；构造示例另有编译通过的 doctest。

```rust
use yue_component::{
    SegmentedDistribution, SegmentedShape, SegmentedSize, SegmentedVariant,
    YueSegmented, YueSegmentedOption, outline,
};

// 截图一：中性圆角、等宽。
YueSegmented::new("time-granularity", theme, self.granularity.clone())
    .label("时间粒度")
    .distribution(SegmentedDistribution::Equal)
    .options([
        YueSegmentedOption::new("day", "日"),
        YueSegmentedOption::new("week", "周"),
        YueSegmentedOption::new("month", "月"),
        YueSegmentedOption::new("year", "年"),
    ])
    .on_change(cx.listener(|this, value, _, cx| {
        this.granularity = value.clone();
        cx.notify();
    }));

// 截图二：单项禁用；可附 Tabler 图标。
YueSegmented::new("view-mode", theme, self.view_mode.clone())
    .label("视图模式")
    .distribution(SegmentedDistribution::Equal)
    .options([
        YueSegmentedOption::new("list", "列表").icon(outline::List),
        YueSegmentedOption::new("grid", "网格").icon(outline::LayoutGrid),
        YueSegmentedOption::new("board", "看板").disabled(true),
        YueSegmentedOption::new("chart", "图表"),
    ]);

// 截图三：强调色胶囊，按内容分配宽度。
YueSegmented::new("billing-period", theme, self.billing_period.clone())
    .label("定价周期")
    .variant(SegmentedVariant::Accent)
    .shape(SegmentedShape::Pill)
    .distribution(SegmentedDistribution::Content)
    .options([
        YueSegmentedOption::new("monthly", "按月"),
        YueSegmentedOption::new("yearly", "按年 · 省 20%"),
    ]);
```

以上后两个片段省略了与第一个相同的写回回调；真正示例必须写回 value。当前值回显属于宿主页面，不放入组件内部。

### 5.1 类型与方法

| 类型/方法 | 契约 |
| --- | --- |
| `YueSegmented::new(id, theme, value)` | value 为 `impl Into<SharedString>`，由调用方控制 |
| `.use_default_theme()` | 与现有 Yue 控件一致，渲染时解析应用默认主题 |
| `.theme(theme)` | 覆盖实例主题 |
| `.options(IntoIterator<Item = YueSegmentedOption>)` | 按提供顺序布局；检查重复 value |
| `.label(text)` | 可见组说明，同时作为无障碍名称；说明在控件上方，不参与底轨宽度计算 |
| `.accessibility_label(text)` | 仅提供无障碍名称，适用于已有外部标题的工具栏 |
| `.size(SegmentedSize)` | Small / Medium / Large |
| `.variant(SegmentedVariant)` | Neutral / Accent |
| `.shape(SegmentedShape)` | Rounded / Pill |
| `.distribution(SegmentedDistribution)` | Content / Equal |
| `.disabled(bool)` | 整组不可用，保留当前选择 |
| `.width(...)` / `.min_width(...)` / `.max_width(...)` | 遵守现有 `ControlLayout`；作用于组的外层布局，底轨填满外层并独立保留内容最小宽度；组说明跟随同一外部宽度约束 |
| `.margin(...)` / `.flex_grow(...)` | 复用现有布局接口，不开放覆盖内部选中颜色的任意 Styled 接口 |
| `.on_change(handler)` | `Fn(&SharedString, &mut Window, &mut App)`，便于直接接 `cx.listener`；只报告请求选择的 value |
| `YueSegmentedOption::new(value, label)` | 文本选项；label 是可见文字及默认无障碍名称 |
| `option.icon(impl IconDefinition)` | 文本前附一个图标，复用当前 SVG 定义 |
| `YueSegmentedOption::icon_only(value, icon, name)` | 纯图标选项，必须提供可访问名称；名称也用于简短提示 |
| `option.disabled(bool)` | 该项不可交互，不参与箭头导航 |

V1 不公开 `SegmentedState` Entity，不把选中值、绘制进度和业务状态再包装一层。FocusHandle、布局缓存和动画状态保存在控件 id 对应的私有 keyed state 内。

### 5.2 值与事件

- 选择 A → B，只回调 B 一次；点击/激活已经选中的 B 不回调、不取消选择。
- 不伪造 `ClickEvent`：左右键/Home/End 同样能请求选择，所以公共事件是语义值变更。
- 调用方不写回时，选中值与滑块停在旧项；焦点仍可移动到请求项。滑块只跟随渲染传入的已提交 value。
- 外部修改 value 会更新显示及滑块，不反向触发回调，也不抢走当前焦点。
- **选项** value 必须非空且组内唯一；重复或空选项 value、选项缺少可访问名称时，`.options(...)` 明确 panic。组缺少有效名称则在渲染时 panic，错误包含组件 id；选项错误同时包含相关 value。传入的**当前受控值**可以为空或未知，此时按无匹配值处理。
- 空 options 合法：不显示底轨与滑块、不提供 Tab 停靠；如提供可见组说明则保留说明。一个选项时正常展示，重复激活仍无事件。
- value 不在当前 options 中：显示选项但无选中项/滑块；Tab 落在第一个可用项。不擅自选第一项，不替调用方改值或发事件，适配异步更新选项的中间状态。
- 已选项变为 disabled：保留选择与滑块，Tab 入口改为首个可用项；用户下一次激活可用项才改变值。

## 6. 几何与宽度规则

### 6.1 尺寸

数值是当前 token 在基准缩放下的逻辑像素，实际通过 token/rem 解析，随界面缩放同步变化。

| 档位 | 底轨总高度 | 轨道内边距 | 选项/选中块高度 | 左右内边距 | 字号/行高 | 图标 |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| Small | 28 | 4 | 20 | 8 | 12 / 16 | 16 |
| Medium | 32 | 4 | 24 | 12 | 14 / 20 | 16 |
| Large | 40 | 4 | 32 | 16 | 16 / 24 | 20 |

选项垂直居中，padding-block 为 0；选项之间无间隔、无分隔线。底轨只绘制一次。选中块是绝对绘制层，不参与 flex 宽高分配。

Rounded 默认外圆角 8、内圆角 4，遵循现有 radius-md/sm。Pill 的底轨半径为底轨高度的一半，选中块半径为其高度的一半，自然保持均匀胶囊边带。焦点环单独计算，不被内层裁切吞掉。

### 6.2 按内容宽度 Content

设第 i 项实际内容宽度为 `cᵢ`，左右内边距为 `p`，选项高度为 `h`，则其自然宽度为 `wᵢ = max(cᵢ + 2p, h)`；底轨自然宽度为 `2 × track_padding + Σwᵢ`。

文本按当前字体、字号和两种字重实际 shaping 后取最大宽度；图标加文字再计入图标槽位与间距。纯图标项同样遵守最小方形命中区域，不靠 SVG 自身墨迹宽度决定点击范围。

Content 下传入更宽的底轨宽度时，各项保留自然宽度并从起始侧排列，不暗中均分或缩放文字。要铺满并占满每一段，应选择 Equal。

### 6.3 等宽 Equal

- 自然宽度：取所有项 `wᵢ` 的最大值作为每段宽度，底轨为 `2 × track_padding + N × max(wᵢ)`。
- 有明确可用宽度 W：每段为 `(W − 2 × track_padding) / N`，同时遵守上述内容最小宽度约束；图标、文字居中。
- V1 不压缩到内容最小宽度以下，不折行、不静默裁切标签，也不自动缩小字号。过窄时底轨按最小宽度溢出，由宿主提供横向滚动或换控件；示例必须演示这一边界。
- `max_width` 小于最小内容宽度时同样由最小内容宽度优先，文档不得声称一定适配任意父容器。
- 父级只负责控件外部间距。组件默认自然宽度，既不默认 `w_full()`，也不默认继承卡片宽度。全宽用 `.width(gpui_kit::relative(1.0))` 并设置 Equal。

两种分配都用实际布局 bounds 定位滑块。Content 尤其不能用 `index × 固定宽度`；Equal 也不累加四舍五入后的宽度，避免末段出现一像素缝隙。

## 7. 视觉状态

| 状态 | Neutral | Accent |
| --- | --- | --- |
| 底轨 | `--surface-component` | `--accent-subtle` |
| 未选文字/图标 | `--text-secondary` | `--accent-subtle-text` |
| 未选 hover | 文字提升到 `--text-primary`，无新增滑块 | 同一强调文字角色；轻微 hover 覆层 |
| 未选 pressed | 叠主题 pressed 覆层，不整体缩放 | 同规则，不移动或伸长滑块 |
| 已选 | `--surface` + `--shadow-1`，`--text-primary` | `--accent-solid`、`--on-accent`，无阴影 |
| 已选 hover / pressed | 保留选中块，不再改变背景或尺寸 | 同规则 |
| 单项 disabled | 该项文字/图标 `--text-disabled`，无 hover/press；若为选中项，使用无阴影的中性表面填充和 `--text-placeholder` 文字 | 同规则 |
| 整组 disabled | 底轨 `--surface-component`，未选文字 `--text-disabled`，选中块 `--surface`、文字 `--text-placeholder`，无描边和阴影 | 使用相同的禁用配色，不残留明亮主色 |
| 键盘 focus-visible | 当前焦点项具有清晰焦点环 | 同规则，不能与强调滑块混成一块 |

选中与焦点分开：焦点可在回调被拒绝时停在未选项；选中块仍在旧项。纯鼠标点击不额外展示键盘焦点环。可用项使用本项目约定的手指光标，禁用项使用不可用光标。

### 7.1 文字稳定性与 Accent 的过渡对比度

文字、图标与各段命中区域固定在各自位置，仅背景块移动；不缩放整组、不变更行高、不让选中块挤压布局。未选 regular、选中 medium 的字重变化只在 value 改变时发生，测量宽度提前预留，不逐帧切换字重。

Accent 的白字不能在滑块还未到达时立刻画在浅色底轨上；也不能在滑块经过其他标签时让深色字直接压在主色上。采用同一份布局的两次绘制：先画底轨文字/图标，再在滑块覆盖区域内绘制 `--on-accent` 版本。后一层仅绘制、无命中区域和无障碍节点，两层使用同一份字形布局和坐标。

裁切跟随滑块绘制区域，不能通过复制整排可交互 Radio 来实现；最后始终只有一套 Radio 语义与事件。V1 先验证文本与 Tabler SVG 的同坐标着色/裁切，再做 Accent 完整样式；这是需要真实窗口检查的实现重点。

## 8. 滑块动画与布局生命周期

### 8.1 运动策略

- 选中块的位置与宽度共同过渡，默认 **200ms**，读取 `--segmented-duration` → `--duration-200`；曲线读取 `--segmented-ease` → `--motion-ease-standard`，当前为 `cubic-bezier(.2, 0, .38, .9)`。
- 这是位置/大小变化，不复用菜单 hover 的慢起步曲线、延迟或手写分段百分比；无需拉伸、回弹、过冲或点击收缩。
- hover/press/文字状态用独立的 `--motion-duration-interaction`，当前 100ms，不为背景滑动而把所有反馈拖到 200ms。
- A → C 动画中改为 B：沿同一个组级动画通道，从此刻的绘制 bounds 接着向 B 运动；不从 A 重启，不为每次选择生成新通道。
- Base transition 保持采样位置连续，不承诺速度连续；V1 不把它描述为物理弹簧。若以后有明确的手势跟随需求，再采用现有 settle spring。
- reduced motion：滑块和颜色直接落到目标，不改变选择、焦点或事件时序；不手改应用全局设置来绕过系统偏好。

### 8.2 测量与状态归属

渲染结构：RadioGroup 语义根 → 组说明（可选）+ 底轨。底轨内每项由一个 Base Radio 提供语义与命中区域，子 canvas 只收集几何；末尾绝对定位 canvas 绘制选中块、反馈、文字/图标和焦点环。Accent 反色内容是同源绘制覆盖层。

私有 keyed state 仅保存：各 value 的 FocusHandle、上一帧有效几何/布局版本、选中块的展示生命周期、必要的指针瞬时状态。业务 value 始终来自调用方。选项身份用 `(group_id, value)`，滑块通道用 `(group_id, "indicator-bounds")`，不按选项索引或当前 value 重建。

布局流程：

1. 常规 layout 决定底轨和每项 bounds；每个 Radio 的无绘制 canvas 在 prepaint 收集当前项 bounds。
2. 末尾绘制 canvas 的 prepaint 在所有项之后运行，读取本帧底轨及选项 bounds，换算到**底轨局部坐标**；不把窗口绝对位置作为动画目标。
3. 在同一次 prepaint 用本帧几何采样 Base `transition(Bounds<Pixels>)`，结果直接交给本帧 paint；不经过“保存测量 → notify → 下一帧重画”的延迟循环。GPUI 允许在 prepaint 使用 keyed element state；动画后续帧由 Base 驱动。
4. 首次有效几何直接显示目标；后续仅 value 改变时执行选择动画。
5. 只发生窗口缩放、rem、字体、标签、分配方式、形状或选项顺序变化时，更新目标几何并立即对齐，不把重排误当一次用户切换。缓存失效必须包含 rem 与字体/标签/选项顺序和宽度约束。
6. 纯粹滚动或移动窗口不会改变局部目标，滑块不追逐屏幕坐标。

实现位于 `segmented/`，公共 builder、element 和 behavior 按职责分离，通过标准 Radio、flex 与 canvas 完成，无额外公共 Element 或布局引擎。几何变化优先于选择动画，选择与 resize 同帧时使用新布局，不携带旧尺寸。

初次 value 从无匹配变为有效、或已选项被删除：前者直接定位，后者移除滑块，均不从其他选项编造起点。整组禁用或选中项变为禁用时，停止选择动画并直接显示实际受控值的禁用填充。

## 9. 键盘、焦点与无障碍

组用 `Role::RadioGroup`、Horizontal orientation 和组名；每项用 Base Radio 的 `Role::RadioButton`、toggled/selected、label、disabled 与一基位置/总数。滑块、阴影和重复着色层均为装饰，不产生第二个可聚焦节点。

焦点环绘制在当前焦点项外围，采用主题 width/offset；focus 绘制层位于底轨内容裁切之外，宿主需留出环宽与间距。它跟随焦点项而不是动画中的滑块，避免快速切换时焦点环滞后，也避免 Neutral 主题下与选中填充同色而消失。

| 输入 | 行为 |
| --- | --- |
| 鼠标点击可用未选项 | 将焦点交给该项，请求选择一次 |
| 鼠标点击已选项 | 保留选择，无变更事件 |
| Tab / Shift+Tab | 整组一个停靠点；从组外进入当前可用选中项，无匹配时进入第一个可用项；组内 Tab 离开组 |
| ← / → | 移动焦点并请求选择上/下一个可用项，首尾循环 |
| Home / End | 移动到首/末可用项并请求选择 |
| Space / Enter | 复用 Base 原生激活；已选时不重复请求 |
| ↑ / ↓ | 不处理，保留给外层上下文 |
| 带 Ctrl/Alt/系统键的方向键 | 不拦截宿主快捷键 |
| 全部禁用、空组 | 无 Tab 停靠点；方向键与激活不产生事件 |

一个可用项的循环仍落在自身，不重复发事件。整组禁用需要传递给每个 Base Radio，不能只降低 opacity；loading 不在本组件状态模型中。

选项重排后，FocusHandle 仍跟随 value。当前焦点项被移除或禁用时，仅当焦点原先在本组内才修复：优先聚焦可用的选中项，否则最近的可用邻项；无可用项则按窗口现有焦点顺序离开组。不得在每次 render 自动聚焦，也不得因外部 value 变化抢焦点。

组只要求有有效无障碍名称；`.label` 和 `.accessibility_label` 至少一个。存在外部可见标题时，传同义名称。纯图标项的 tooltip 不替代可访问名称。

Base 私有的 disabled 无障碍投影通过 Base Radio 获得，不虚构 Yue 能调用的 `aria_disabled` setter；Windows 读屏实际播报仍需真人或原生窗口工具验证。

## 10. Token 方案

已在设计系统正式 component-tokens 新增独立 `--segmented-*` 家族，共 44 个 token；未修改 Vue 组件或 prototype 的渲染选择器。

来源仍是 `D:/100_Projects/110_Daily/Yue-Design-System/packages/tokens`。已新增源文件 `src/component-tokens/segmented.css` 并加入该层入口、README、名称词表与语法审计；本项目同步工具已生成 inventory 和 Rust keys，组件直接使用类型化 keys。构建只用本地快照，不依赖外部路径。

| 已实现 token | 默认引用/规则 |
| --- | --- |
| `--segmented-height-sm/md/lg` | `--control-height-sm/md/lg`：28 / 32 / 40 |
| `--segmented-padding-inline-sm/md/lg` | `--control-padding-inline-sm/md/lg`：8 / 12 / 16 |
| `--segmented-font-size-sm/md/lg` | `--control-font-size-sm/md/lg`：12 / 14 / 16 |
| `--segmented-line-height-sm/md/lg` | `--line-height-sm/md/lg`：16 / 20 / 24 |
| `--segmented-icon-size-sm/md/lg` | `--control-icon-size-sm/md/lg`：16 / 16 / 20 |
| `--segmented-track-padding` | `--space-4` |
| `--segmented-content-gap` | `--space-4` |
| `--segmented-radius` / `--segmented-thumb-radius` | `--radius-md` / `--radius-sm`；Pill 使用几何圆角规则 |
| `--segmented-weight` / `--segmented-weight-selected` | `--weight-regular` / `--weight-medium` |
| `--segmented-background` | `--surface-component` |
| `--segmented-color` / `--segmented-color-hover` | `--text-secondary` / `--text-primary` |
| `--segmented-thumb-background` / `--segmented-thumb-color` | `--surface` / `--text-primary` |
| `--segmented-thumb-shadow` | `--shadow-1`，使用现有完整阴影解析，不截断多层阴影 |
| `--segmented-accent-background` / `--segmented-accent-color` | `--accent-subtle` / `--accent-subtle-text` |
| `--segmented-accent-thumb-background` / `--segmented-accent-thumb-color` | `--accent-solid` / `--on-accent` |
| `--segmented-hover-background` | 按 `--text-primary` 与 `--opacity-hover` 混合的透明覆层；Accent 未选 hover 使用 |
| `--segmented-pressed-background` | 按 `--text-primary` 与 `--opacity-pressed` 混合的透明覆层 |
| `--segmented-disabled-background` / `--segmented-disabled-color` | `--surface-component` / `--text-disabled` |
| `--segmented-disabled-thumb-background` / `--segmented-disabled-thumb-color` | `--surface` / `--text-placeholder`，平面填充，无描边或阴影 |
| `--segmented-focus-ring-color/width/offset` | 对应 `--focus-ring-color/width/offset` |
| `--segmented-duration` / `--segmented-ease` | `--duration-200` / `--motion-ease-standard` |
| `--segmented-state-duration` / `--segmented-state-ease` | `--motion-duration-interaction` / `--motion-ease-standard` |

不增加“day 宽度”“year 宽度”“第几项偏移”或每一百分比的动画 token：这些来自内容、布局和曲线。也不把 theme RGB 硬编码到组件。若源设计系统未来改变 token 引用，组件随同步更新。

底轨及阴影与 HTML 的具体颜色不必逐像素一致；以当前 Yue 的 Light/Dark、Azure/Neutral 和中性色调体系为准。可用文字要求 ≥4.5:1，焦点/必要控件边界目标 ≥3:1；禁用态单独记录，不据此宣称所有状态都达到正文对比度。

## 11. Showcase 方案

增加 `examples/windows-showcase/src/pages/segmented.rs`，侧栏归“组件”，英文排序放在 Radio 与 Switch 之间。保持侧栏折叠、全宽背景及内容区域独立滚动。

页面按以下顺序展示：

1. 时间粒度：Neutral + Rounded + Equal，日/周/月/年，下面回显真实 value。
2. 视图模式：Neutral + Rounded + Equal，列表/网格/看板/图表，单独禁用看板。
3. 定价周期：Accent + Pill + Content，按月 / 按年 · 省 20%，长标签导致更宽的真实选项。
4. 宽度对照：同一组长短标签并排展示 Content 与 Equal，另展示显式宽度 Equal。
5. Small/Medium/Large；中性/强调色与两种轮廓的组合。
6. 文本、图标加文本、纯图标；展示可访问名称与提示。
7. 全禁用、已选项禁用、单项可用、空选项、外部更新 value；显示实际事件次数。
8. 动画验证：长短标签连续快速切换、运行中反向、reduced motion；resize/rem 改变时滑块立即重对齐。
9. Light/Dark × Azure/Neutral 预览，检查反色、禁用填充及焦点环。

预览里的值、事件计数和外部更新按钮只属于示例，不扩展为组件属性。正式示例不借默认选中假象掩盖缺失的回调写回。

## 12. 实施顺序与验收

本节保留实现顺序与验收范围；已执行结果及未覆盖边界见第 13 节，不把验收计划当成已通过证据。

### 12.1 实施顺序

1. 运行现有 workspace 与 headless 基线，保留 Button、Radio、Switch、Input、Dropdown 和 Showcase 的测试结果。
2. 先补本组件的值契约、键盘与几何测试，再添加源 token、同步生成物和 `segmented.rs`。
3. 先做 Neutral 的内容测量、等宽/不等宽与单选交互，再验证指示器连续插值和动态布局。
4. 完成 Accent 的反色绘制、Pill、图标、禁用与 focus，接入 Showcase；真实 Windows 验收作为待完成项明确记录。

当前组件位于 `crates/yue-component/src/segmented/`，按 [组件架构规范](24-component-architecture.md) 拆分为 `mod.rs`、`element.rs`、`behavior.rs` 与 `tests.rs`；公共 UI 集成测试仍使用 `tests/segmented.rs`。新增测试沿用 opt-in `headless-tests`，不改变当前依赖 feature 边界。

### 12.2 必须证明的行为

| 层次 | 验收内容 |
| --- | --- |
| 单元 | value 唯一/非空校验，首末/循环/跳过禁用，缺失选中值、空组、全禁用组，Content/Equal 最小宽度规则 |
| 受控 UI | 点击及 Space/Enter 各只请求一次，选中项重复激活无事件；拒绝写回不改变滑块；外部更新不发事件 |
| 焦点 UI | 单一 Tab 停靠、反向 Tab、左右/Home/End、焦点与选中分离、选项删除/禁用修复，不抢走组外焦点 |
| 禁用 UI | 先取得焦点再禁用后发键、鼠标点击、动画中禁用均不可请求变更；保留受控选择和禁用语义 |
| 布局 UI | Content 长短项实际不等宽、Equal 相等、显式宽度均分、默认不占满卡片、纯图标命中尺寸，选中变粗不改变各项 bounds |
| 绘制 UI | 滑块命中实际目标 bounds；中间帧的位置和宽度都处于合理插值；多实例通道互不影响；reduced motion 立即落位 |
| 动态 UI | 快速 A→C→B 不跳回旧起点；resize/rem/字体/标签/重排后无旧几何残留；父级滚动不引发滑块追赶 |
| 语义 | RadioGroup/RadioButton、名称、toggled/selected、disabled、位置/总数、装饰层无重复节点；只断言测试工具实际可观察到的层次 |
| 主题 | 默认主题切换及显式主题覆盖；全部 profile/tone 的可用文字/选中反色和焦点对比度 |
| 真实窗口 | 100%/150%/200% DPI、中文字体、长标签、Pill 边缘与阴影、Accent 途中反色、键盘 focus、Windows 读屏 |

Accent 裁切/着色在无头里最多证明提交的绘制数据，不能替代 GPU 实际像素验证；滑块是否流畅也要在 Windows release 窗口中观察。验收不能只写“设置了 200ms”，要证明存在中间帧，并确认没有误改文字和命中区域。

实现后的检查命令：

```powershell
node tools/sync-yue-theme.mjs --check
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
```

设计系统源 token 的测试按该仓库规范单独执行并记录；既有失败与本组件新增失败分开说明。实际实现、测试结果和仍未验证的 Windows 行为记录如下。


## 13. 实现交接与验证证据

### 13.1 落地文件

- 组件：[segmented.rs](../crates/yue-component/src/segmented/mod.rs)；根部与 prelude 导出在 [lib.rs](../crates/yue-component/src/lib.rs)。未新增第三方依赖，业务 value 由调用方持有。
- UI 集成测试：[tests/segmented.rs](../crates/yue-component/tests/segmented.rs)，保持 opt-in `headless-tests`。5 项单元测试、12 项组件 UI 集成测试、1 项构造 doctest；Showcase 另增 1 项真实页面选择写回测试。
- 示例：[pages/segmented.rs](../examples/windows-showcase/src/pages/segmented.rs)，侧栏“组件 → Segmented”，位于 Radio 与 Switch 之间。两个 Icon 页与现有折叠、滚动行为保持测试覆盖。
- Button 原有私有 tooltip 绘制提取到 [tooltip.rs](../crates/yue-component/src/tooltip/mod.rs)，供 Button 与纯图标 Segmented 共用；没有新增公共 Tooltip API。Button 的交互回归随 workspace 测试执行，tooltip 的实际弹出外观未做原生窗口验收。
- 源 token：`Yue-Design-System/packages/tokens/src/component-tokens/segmented.css`，入口、README、精确名称词表和语法登记同步更新；`tests/token-segmented.test.mjs` 有 17 项测试。Vue/SFC/DOM/SSR、npm 出口和本地化资源未变，相关实现清单不适用于本次纯 token 维护。

实现相较提案的一处调整是：使用有序 canvas 的**本帧 prepaint** 直接测量并采样过渡，省掉额外一帧刷新。Accent 双重绘制复用相同文本坐标及 SVG 字节；GPUI 内容掩码只有矩形，私有 `rounded_masks` 按实际设备像素扫描圆角端部，中间用一块矩形覆盖。无重复语义节点，不将绘制层变为另一套 Radio。

布局入口统一作用于语义外层。底轨 `w_full` 并独立保留内容下限，因此父级百分比宽度可正常解析，调用方较小的 `min_width/max_width` 也不能压扁标签。空组选项不再套用底轨自然宽度，保留组说明的自然布局。重排后焦点跟随 value，必要修复按真实选项位置选择最近的可用项。

### 13.2 自动化结果

实现前先运行基线；新增 API 和 token 测试在实现前分别因缺失组件和 token 失败，落地后通过。此次是新增能力，没有移除旧 API、删除测试或修改既有断言以制造通过。

```text
Windows / Rust MSVC，2026-11-01
cargo test --workspace --locked
  before: 116 passed; after: 122 passed; 0 failed
cargo test --workspace --locked --features headless-tests
  before: 181 passed; after: 199 passed; 0 failed
cargo test -p yue-component --locked --no-default-features --features headless-tests
  after: 192 passed; 0 failed
cargo clippy --workspace --locked --all-targets -- -D warnings
  exit 0
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
  exit 0
cargo fmt --all -- --check
  exit 0
node tools/sync-yue-theme.mjs --check
  Yue GPUI theme mapping is current; exit 0
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check
  Yue GPUI theme mapping is current; exit 0
git diff --check
  exit 0
```

以上计数合计各测试二进制与 doctest。UI 测试实际证明：受控点击/Space/Enter、重复激活无事件、拒绝写回、独立焦点、单一 Tab 停靠、左右循环/Home/End/跳过禁用、修饰键不拦截、整组禁用后离开、删除焦点项修复、空/未知 value 不抢焦点、Content/Equal 和最小内容宽度、父级百分比宽度、选中不重排、图标尺寸、标签/rem/重排变化、独立实例与默认/显式主题。动画测试读取提交的选中块 quad，验证位置与宽度的中间帧、改目标连续性和 reduced motion 立即落位。

语义测试检查可观察的 RadioGroup/RadioButton 名称、selected、disabled 和 focus；Base 原生提供的 orientation/position/set size 在当前快照工具中无读取接口，未将它们写成已验证断言。

设计系统源仓库（纯 token 维护）的实际结果：

```text
corepack pnpm exec vitest run tests/token-segmented.test.mjs
  17 passed; 0 failed; exit 0
corepack pnpm audit:tokens
  exit 1: 既有 checkbox/radio/switch 等命名与原型契约问题仍在
  architecture: 1003 declared; unreachable 0; undeclared 0;
                duplicates 0; cross-file slots 0; layer violations 0
corepack pnpm verify:all
  before: 11 failed | 708 passed (5 failed files)
  after:  11 failed | 725 passed (5 failed files)
  逐项比较 11 个失败名称，前后一致；新增 Segmented 测试全部通过
```

17 项源测试包含命名审计登记、16 个 Light/Dark × Azure/Neutral × neutral tone 组合；每个组合验证三档高度、200ms、padding、四组可用文字对比度 ≥4.5:1，以及焦点环对两种轨道/基础表面的对比度 ≥3:1。源审计与 `verify:all` 的既有失败未扩大为本次修复范围，也没有把源仓库总门禁报告为通过；`verify:all` 在测试阶段失败，后续门禁未继续执行。外部 CSS 本次仅新增声明，没有改变浏览器渲染选择器，未运行浏览器视觉门禁。

本地 token 库存从 959 增加为 **1003**，新增 44 项均从源生成：

```text
payloadSha256: 0a4746238f1696ba8e683afdf5b73f4c248c17c62ac9d0b4d2e3945e1d3b9694
source.contentSha256: d5a74987f5a9a5e99f9bb351cd472d21b682a71513532e9f46f58bd35d9ade33
```

### 13.3 预览与未验证项

运行 `cargo run -p windows-showcase --release`，选择“组件 → Segmented”。本轮另生成独立预览 `target/release/windows-showcase-segmented-200ms.exe`，避免覆盖其他正在使用的预览路径。release 编译通过；用 `rustc -o` 指定文件名产生的两条输出路径提示不属于 Clippy 代码警告。

当前环境未提供可用的 Windows 原生窗口观察/操作通道，未验证 GPU 实际像素、Pill 圆角裁切/文字边缘、SVG 反色、tooltip 外观、100%/150%/200% DPI、真实字体栅格化、读屏播报及动画主观流畅度。无头测试证明布局与绘制数据，不替代这些检查。后续需在该 release 窗口中逐项检查第 12.2 节的“真实窗口”项；尤其注意 Accent 滑块经过文本与图标时的边缘。Windows 高对比度/forced colors 未适配，不声称具备该能力。

没有提交 Git，也未对已有未提交改动进行清理。


## 14. 间距与动效调整（2026-11-01）

根据实际预览反馈调整两个源 token，公共 API 不变：

| 项目 | 调整前 | 当前 |
| --- | --- | --- |
| `--segmented-duration` | `--duration-300`（300ms） | `--duration-200`（200ms） |
| `--segmented-track-padding` | `--space-2`（2px） | `--space-4`（4px） |
| Small / Medium / Large 滑块高度 | 24 / 28 / 36px | 20 / 24 / 32px |

总高度仍由原三档高度 token 决定；四周间距加倍，滑块及命中区域依新内边距布局。Equal 显式 600 / 700 / 800px、四项时，各段分别为 148 / 173 / 198px，测试期望随该几何契约更新。仍使用原标准缓动与独立 hover/press 时长。

调整前组件 11 项、源 token 17 项测试通过。先修改间距、尺寸与 200ms 到达目标的断言，原值下 4 项组件测试失败；更新源 token 并重新生成后，11 项组件测试与 17 项源测试全部通过。原有中间帧、改目标连续性和 reduced motion 检查仍保留。

本轮 `cargo test --workspace --locked --features headless-tests` 合计 199 项通过，带 feature 的 workspace Clippy（`-D warnings`）、格式、两种 token 同步检查及 `git diff --check` 通过，release 编译通过。源仓库 `audit:tokens` 与 `verify:all` 仍失败，后者仍为 11 failed / 725 passed，失败名称与调整前一致。

本轮验证日志保存在本地忽略目录 `target/segmented-tuning-*.log`；原生 GPU/DPI/读屏的验证边界与第 13 节相同。最新 release 预览为 `target/release/windows-showcase-segmented-200ms.exe`。


## 15. 禁用视觉调整（2026-11-01）

截图中的细线框让已选项看起来像独立输入框，因此将禁用选中块调整为保留选择的平面填充：中性底轨、表面色滑块、灰色文字，无描边或阴影。禁用已选文字使用 `--text-placeholder`，比未选的 `--text-disabled` 稍深；禁用提示仍由 Base disabled 语义和不可用光标表达。两种外观的禁用滑块均不使用强调色或反色文字。

整组 disabled 和“所有 option 均 disabled”统一使用中性底轨，修复后者在 Accent 下仍保留强调底色的问题。仅单项禁用时保留其他项的正常主题与交互。200ms、4px、尺寸、受控 value 与事件规则沿用上一轮。

源 token 破坏性调整：移除 `--segmented-disabled-thumb-border-color/width`，新增 `--segmented-disabled-thumb-background/color`；对应生成的 `theme::keys::*` 也同步替换，引用旧键的调用方需改用新键。库存仍为 1003、Segmented 仍为 44 项，无兼容别名。disabled-background 改为引用 `--surface-component`，避免叠加透明灰造成整组发脏。

调整前 11 项组件测试通过；新增平面填充回归测试先在旧实现上失败，修复后通过。组件 UI 集成测试现有 12 项，新测试覆盖 Light/Dark × Azure/Neutral × Neutral/Accent × 整组/已选项/全部选项禁用的 24 组情况，验证实际提交的实色滑块、零描边、禁用中性底轨、保留选择及点击不发事件。源 token 17 项测试通过，并补充对应主题角色引用断言。Showcase 的四个主题卡片增加禁用预览。

本轮 workspace 无头测试合计 200 项通过，带 feature 的 workspace Clippy（`-D warnings`）、格式、两种 token 同步检查及 `git diff --check` 通过；release 编译通过。源仓库 `audit:tokens` 与 `verify:all` 仍有既有失败，后者为 11 failed / 725 passed，失败名称与调整前一致。

新版 release：`target/release/windows-showcase-segmented-disabled.exe`。本轮证据位于本地忽略目录 `target/segmented-disabled-*.log`；原生 GPU/DPI/读屏与截图比对仍未自动验证。
