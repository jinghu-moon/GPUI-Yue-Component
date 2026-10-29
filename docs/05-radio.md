# 05 YueRadio

## 目标

提供单选控件和单选组，选择互斥、状态由调用方控制。实现参考图中基础圆点、按钮分段、三档尺寸、水平/垂直分组和禁用状态，同时遵循 Windows 桌面键盘与无障碍语义。

Radio 用于少量、互斥且需要同时比较的选项。选项多、需要搜索或占用空间明显时，使用 Select/Combobox，不把 Radio 扩展成下拉菜单。

`YueRadio` 与 `YueRadioGroup` 已按这份契约落地（`crates/yue-component/src/radio.rs`）。下面每一节写的都是代码里实际发生的事；与设计稿草案不一致的地方集中在最后一节。

## GPUI Kit 0.8.0 能力

实现前逐项检索过源码，结论与设计稿的判断一致：

### `gpui_kit::base::Radio`

- 受控 `checked(bool)`；激活未选项时 `on_change(true, ClickEvent, ...)`，激活已选项不触发取消选择。
- 负责 RadioButton role、checked/selected 状态、disabled、焦点、Tab stop、accessible label，以及可选的一基 `position_in_set` / `size_of_set`。
- 支持调用方提供 FocusHandle、tab index 和 tab stop；外观、标签、布局由上层组合。
- Base 不提供选中值容器、互斥更新、箭头键组导航、组标签或视觉选项样式。
- 额外核对到的一条：`disabled(true)` 的选项不会注册焦点绑定，因此它在观察层里读起来是"没有焦点节点"，而不是"焦点为 false"。

### `gpui_kit::base::RadioGroup`

它只是带 `Role::RadioGroup` 和 orientation 的语义容器；不持有值、不更新选中项、不实现 roving tabindex 或方向键导航。子 Radio 默认各自是 tab stop。没有专用组 label builder，但继承 GPUI 的交互样式能力可供 Yue 尝试投影 `aria_label`，实现时需以编译和无障碍树测试确认。GPUI Component 的 `RadioGroup` 以 index 管理选中项并补 position/set 元数据，但同样没有方向键组导航。Yue 不直接把它描述成已经完整实现的原生单选组。

实际实现没有使用 Base 的 `RadioGroup`：Yue 的组容器是自己的 `div`， role、orientation、`aria_label` 和按键处理都挂在那一层，理由见"键盘和焦点"。

## 公共 API

```rust
// 单个选项：调用方自己比较出 checked
YueRadio::new("png", theme, selected_format.as_deref() == Some("png"))
    .label("PNG")
    .size(RadioSize::Medium)
    .appearance(RadioAppearance::Indicator)
    .disabled(false)
    .on_change(|checked: bool, event: &ClickEvent, window, cx| { /* 写回自己的值 */ });

// 单选组：值由调用方持有，组只报告新值
YueRadioGroup::new("export-format", theme, selected_format) // Option<SharedString>
    .label("导出格式")
    .direction(GroupDirection::Horizontal)
    .appearance(RadioAppearance::Button)
    .options([
        YueRadioOption::new("png", "PNG"),
        YueRadioOption::new("svg", "SVG"),
        YueRadioOption::new("pdf", "PDF").disabled(true),
    ])
    .on_change(|value: SharedString, window, cx| save_format(value));
```

- 组的当前值是 `Option<SharedString>`；无初始选择是合法状态。
- 选项 ID 由 `(组 id, value)` 派生，不用数组下标，排序变化后选中项和焦点不串位。
- `.options(...)` 会断言 value 唯一，重复时 panic 并点名重复的 value——两份同名选项在互斥写入时无法区分。
- `.label(...)` 既是可见说明文字，也是组在无障碍树上的名字（容器同时投影 `aria_label`）。
- 单项必须有 `.label(...)` 或 `.accessibility_label(...)`，两者皆无时 panic；纯圆点选项靠后者命名。
- `YueRadio::on_change` 收到的 `bool` 只会是 `true`：Base 只在选项可用且尚未选中时请求变更，因此 Radio 一经选择，点击自身不取消选择。若业务需要撤销选择，应提供明确的"无"选项或独立清除命令；Yue 不提供 `allow_uncheck`。
- 组的 `on_change` 报告被选中的 value 本身，而不是 delta；调用方更新自己的值并重绘。
- `.disabled(bool)` 整组禁用时，组内没有任何可 Tab 到的选项，激活也不上报；选项可单独禁用。

## 变体

- 尺寸：`RadioSize::{Small, Medium, Large}`，默认 Medium。三档同时决定圆点直径、行高、字号与分段行的几何：

  | 档位 | 圆点直径 | 中心点 | 行高 | 字号 | 分段行高 | 分段内边距 |
  | --- | --- | --- | --- | --- | --- | --- |
  | Small | 12px | 4.5px | 28px | 12px | 28px | 8px |
  | Medium | 16px | 6px | 32px | 14px | 32px | 12px |
  | Large | 20px | 7.5px | 40px | 16px | 40px | 16px |

  中心点在任何档位都是外接框的 0.375（参考图 Medium 的 6-in-16），Medium 扣除 1px 边框后正好落在参考图的 4px inset 上。
- 外观：`RadioAppearance::{Indicator, Button}`，默认 Indicator。两种外观共用同一套 role、键盘与互斥契约，只有画法不同（见下节）。
- 方向：`GroupDirection::{Vertical, Horizontal}`，默认 Vertical。组的容器始终纵向堆叠（说明文字在上、选项在下），方向只决定选项那一排怎么排；Vertical 是列，Horizontal 是行。
- Indicator 外观的 Horizontal 行允许换行（`flex_wrap`），窄窗下长标签折到第二行仍属于同一个组、同一套键盘方向。分段外观不换行：一行折成两段就不再读作一个选择集，长标签、很多选项应退回 Indicator 或 Select。
- 圆点和方框都装在固定宽高的槽位里，按压缩放只动槽内元素，不会让旁边的标签抖动。

## 键盘和焦点

Base Radio 提供单项焦点与激活，组级导航由 `YueRadioGroup` 自己实现：

- **一个 tab stop**：组内只有一个选项进入 Tab 顺序——已选中且可操作的那一项；无选中（或调用方持有的 value 已不在选项里）时取第一个可操作项；整组禁用或全部禁用时组内没有任何停靠点。Tab 一次进入组、再一次离开整组，Shift+Tab 原路返回。这个停靠点跟着值走：调用方改写 value 后，Tab 落在新的那一项上。
- **方向键沿本组轴向**：垂直列读 Up/Down，水平行读 Left/Right；垂直列不消费 Left/Right，水平行不消费 Up/Down，另一种方向的键留给外层（嵌套异向组因此各读各的）。Home/End 定位首/末可操作项。
- **移动即选中**：方向键落到的选项会被选中并上报，和原生单选组一致；越过末尾向前循环，越过开头向后循环；禁用项不进入这条序列，因此天然被跳过；只有一个可操作项时原地不动。
- **键权归组**：处理过的按键调用 `cx.stop_propagation()`，外层滚动视图不会同时把 Up/Down 当作滚动。
- Space/Enter 激活焦点项；点击标签也选择它（整行都是点击目标）。
- 组的按键处理挂在 `on_change` 上：没有回调就没有能写入 value 的调用方，此时组不接管方向键，把它还给页面。
- 焦点句柄由组为每个可操作选项创建（键为 `(选项 id, "yue-radio-focus")`），选项自己绘制焦点环时也读同一个句柄；禁用项不创建句柄。

## 分段外观（Button）

整组是一个连续轮廓，而不是几个互不相关的按钮：

- 边框只画一次——由组的行容器带 `--radio-button-border-width` 的整圈边框 + `overflow_hidden` 裁两端，圆角只出现在首尾；段与段之间不再留 gap，也不再各自描边。
- 每段只补它和前一段之间的那条分界线：水平行加左边框、垂直列加上边框，第一段不加。分界线与外框同宽同色，因此交界处不会双线加深。
- 选中段读系统的"已选中"角色，不是按下态：填充取 `--selected-background`、文字取 `--selected-color`，再加 `--radio-button-weight-selected`（Medium 500）的字重，单色或高对比 profile 下选中仍可读。
- 未选中段才有 hover / press 覆盖层，且以叠加的方式染在自己的表面色上（`tint()`：保留底色不透明度、只把颜色通道推向覆盖层），所以悬空的分段不会变成半透明洞；已选中段的填充与文字不再被 hover 抬升。
- 分段本身不做按压缩放，按下读作更深的覆盖层——连续行里一段单独缩一下会把分界线拉歪。
- 焦点环画在段内侧：整行裁了端点，画在外面会被切成半个环。环的位置是 `--radio-focus-ring-offset - --radio-button-border-width`，即从内容盒起算、先让开自己那条分界线。

## 两层动效

两种外观都按参考 demo 拆成"状态换色"与"按压"两档，且都读同一对语义时长，因此不会感觉走在两个时钟上。

| 阶段 | 变化的量 | 时长 · 曲线 | 取值来源 |
| --- | --- | --- | --- |
| 圆环填充（Indicator） | `--radio-background` → `--radio-background-checked` 的 RGB 线性混合 | 200ms · `--radio-ease` | token |
| 边框强调（hover 或选中） | `--radio-border-color` → `--radio-border-color-checked` | 200ms · `--radio-ease` | token |
| hover 光晕 | 扩散阴影半径 0 → `--radio-halo-width`(4px)，颜色按进度取 alpha | 200ms · `--radio-ease` | token |
| 中心点生长 / 收拢 | 缩放 0 → 1 | 200ms · 生长用 `--radio-ease-enter`，收拢用 `--radio-ease-exit` | token |
| 按压缩小 | 圆点整体（含中心点、边框、焦点环）缩放 → `--radio-press-scale`(0.9) | 100ms · `--radio-ease` | token |
| 分段选中换色 | 填充与文字混向 `--radio-button-background-selected` / `--radio-button-color-selected` | 200ms · `--radio-button-ease` | token |
| 分段 hover / 文字抬升 | 叠加层进度、文字混向 `--radio-button-color-hover` | 200ms · `--radio-button-ease` | token |
| 分段按下叠加 | `--radio-button-pressed-background` 进度 → 0 | 100ms · `--radio-button-ease` | token |

- 每个通道是一条独立的 keyed transition（`yue-radio-fill`、`-border`、`-halo`、`-dot`、`-press`，分段的 `-segment-fill/-hover/-press/-text`），进度一律 clamp 到 0..1；一次指针移动或释放只改一份 keyed 状态（`Pointer { hovered, pressed }`），只付一次重绘。
- 禁用态不过渡而是直接重画——参考图的 disabled 规则就是整块替换填充。
- 焦点环不进过渡通道：它随 `focus_visible` 出现或消失，没有中间值。
- pointer 事件（hover、按下、抬起、抬起在框外）只在非禁用时挂上；禁用项既不缩放也不起光晕。
- hover 光是扩散阴影而不是布局边框，因此悬停不会挪动圆点或它旁边的标签；按压缩放发生在固定槽位里，同理不会 reflow。

## Token

几何、颜色、环、间距与动效全部来自设计系统 `packages/tokens/src/component-tokens/radio.css`，渲染侧只读 `YueTheme` 的 token，不另造第二份调色板：

- 尺寸：`--radio-size-sm/md/lg`、`--radio-dot-size-sm/md/lg`（`--radio-dot-size` 保留为 Medium 别名）、`--radio-height-sm/md/lg`、`--radio-label-font-size-sm/md/lg`。
- 圆点：`--radio-border-width`、`--radio-border-radius`、`--radio-border-color`、`--radio-border-color-checked`、`--radio-background`、`--radio-background-checked`、`--radio-dot-color`、`--radio-accent-color`。
- 状态：`--radio-halo-width`、`--radio-halo-color`、`--radio-press-scale`、`--radio-focus-ring-color/width/offset`、`--radio-disabled-background`、`--radio-disabled-border-color`、`--radio-disabled-dot-color`。
- 文字与分组：`--radio-label-color`、`--radio-label-color-disabled`、`--radio-label-gap`、`--radio-group-gap`。
- 动效：`--radio-duration`、`--radio-duration-fast`、`--radio-ease`、`--radio-ease-enter`、`--radio-ease-exit`。
- 分段外观：`--radio-button-height/padding-inline/font-size-sm/md/lg`、`--radio-button-border-radius/width/color`、`--radio-button-background`、`--radio-button-color`、`--radio-button-color-hover`、`--radio-button-background-selected`、`--radio-button-color-selected`、`--radio-button-weight-selected`、`--radio-button-hover-background`、`--radio-button-pressed-background`、`--radio-button-disabled-background`、`--radio-button-disabled-color`、`--radio-button-duration`、`--radio-button-duration-fast`、`--radio-button-ease`。

所有值引用现有 control/text/border/focus/surface/radius/spacing/motion 语义或 primitive token，不内嵌颜色；分段的选中态借用 `--selected-background` / `--selected-color`，与 Button 的按下态不是同一个状态。分段行也没有直接复用 Button 的 token 集：三档行高与 Button 恰好同源（都取 `--control-height-*`，28/32/40），但选中读的是一组角色、按下读的是另一组，分开命名才能各自调而不会互相带歪。

修改后同步 catalog：

```powershell
node tools/sync-yue-theme.mjs --design-system "D:\100_Projects\110_Daily\Yue-Design-System" --write
```

一处已知边界：`--radio-duration` / `--radio-duration-fast` 在 token 源里别名到 `--motion-duration-enter/exit`，设计系统用 `@media (prefers-reduced-motion: reduce)` 把这两个语义角色改成 0ms；`generated.rs` 存的是 `:root` 解析值（四个 profile 都是 200ms/100ms），媒体查询不会进入 Rust 快照，因此 Yue 目前不会随系统的"减少动效"设置落定。这是待补的口子，不是已实现的行为。

## 无障碍

- 每项可访问名称来自可见 label；无可见文本时要求显式 `.accessibility_label(...)`。
- 组容器带 `Role::RadioGroup` 与 `aria_orientation`（Vertical/Horizontal），并同时以 `aria_label` 携带组名——名字由容器和可见说明共同承担，屏幕阅读器在选项之前先报出组。
- 每个选项带 `Role::RadioButton`、checked 状态，以及由组投影的 `position_in_set` / `size_of_set`（1 基，"option 2 of 4"）。
- Focus visible 与 checked 可区分；disabled 项不可激活、不可成为箭头导航目标、也不进入 Tab 顺序。
- `aria-required` 与 `aria-invalid` 不是 Base Radio 的现成能力。必填组需要由 YueField/表单层用可见说明与校验信息表达；不伪称屏幕阅读器已获得原生 required/invalid 状态。

## Showcase 与验收

Windows Showcase 的 Radio 页面（侧栏左目录、右侧纵向文档流，Switch 之后）按这份契约提供可交互样例：

1. **Single options**：未选中、已选中、禁用（禁用项本身是选中态）三项，加一个只有圆点的控件（靠 `.accessibility_label("只有圆点的选项")` 命名）；每项都由 Showcase 状态回写，可反复点击，整行都是点击目标。
2. **Controlled groups**：垂直的导出格式组（含一个禁用项）与水平的主题模式组，每组下面显示当前 value，点击即回写。
3. **Button appearance**：一段分段行与一列分段列，后者初始不选任何项。
4. **Size ladder**：三行 × 两种外观，每行一个独立 group，各自持有自己的 value。
5. **Keyboard**：四条文字说明（Tab 一步进/出组、方向键沿轴移动并选中、Home/End、Space/Enter）加一个可实时操作的频率组。
6. **Empty, locked and narrow**：初始无选中的组、整组禁用（无回调，读值说明"不可操作"）、`.w_72()` 窄容器里的长标签水平组。
7. **Theme profiles**：Light·Azure / Dark·Azure / Light·Neutral / Dark·Neutral 四格，每格同时跑两种外观，共用一个 value。

每个样例都持有稳定 value 和独立回调（示例状态放在 Showcase Entity 中），没有展示用的固定 checked 值。

`cargo test -p yue-component`（45 项，其中 radio 16 项）钉住的是这些纯派生值：

- 名称来源：label 与 accessibility_label 各自可用，两者皆无时 panic。
- `tab_stop_index`：选中项优先、无匹配 value 回落到首项、禁用项即便持有 value 也不能成为停靠点、全禁用与空组无停靠点、整组禁用无停靠点。
- `move_for`：Up/Down 只属于列、Left/Right 只属于行、Home/End 两者通用、Space/Enter/Tab/字母键一律留给行。
- `move_index`：前后循环、单选项不动、Home/End 忽略当前位置、空集合返回 `None`。
- `.options(...)` 拒绝重复 value（panic 测试）。
- 三档尺寸的圆点/行高/字号具体 token 值，逐档校验圆点小于行高、中心点小于扣除边框后的外接框，并校验 0.375 比例与 Medium 的 4px inset。
- 分段档位读取 control ladder 的 28/32/40 行高、12/14/16 字号、8/12/16 内边距，且逐档只增不减。
- 分段焦点环的内缩量非负、加上两圈环宽仍不超过所在行的行高。
- 动效：`--radio-duration` = 200ms、`--radio-duration-fast` = 100ms、`--radio-press-scale` = 0.9、`--radio-halo-width` = 4px；三条 easing 在 0/1 端点落在 0/1、20 步采样不回绕不越界；分段与圆点共用同一对时长档位。
- 颜色：四个 profile 下填充与中心点均不透明、中心点与填充可辨（gap > 0.25）、禁用边框 alpha 为 0、禁用中心点比可用时更暗、hover 光是透明着色而非实心；分段侧校验选中填充/文字等于 `--selected-*` 角色、hover 文字等于 `--radio-label-color`、未选中文字比 hover 更弱、选中字重大于 400。
- `tint()`：进度 0 时底色不动、进度 1 时不透明度守恒（不会把悬停的分段变透明）、颜色沿同一轴单调行进。
- 默认外观是 Indicator，且脱离组的单项自己描完整边框（`segment` 为空）。

`cargo test -p yue-component --features headless-tests`（radio 10 项）在无头元素树上驱动真实交互，钉住的是这些行为：

- 组容器是 `Role::RadioGroup` 且名字叫"导出格式"；四个选项各是 `Role::RadioButton`、名字为可见标签、checked 反映调用方持有的值；禁用项报 `disabled == Some(true)`，其余不报禁用。
- 点击未选项上报一次该 value，前一项在树上读作未选中（互斥由组完成），调用方持有的值与报告一致。
- 重点击已选项、点击禁用项都不上报，选择保持原样。
- Tab 一步进入组、且落在已选中的那项（PDF），其余项 `focused == Some(false)`，禁用项 `focused == None`（根本不注册焦点绑定）；再一步离开整组，Shift+Tab 回到同一停靠点；改写 value 后重新 Tab，停靠点跟到新值（JPG）。
- 空选择与不匹配的 value 都仍能进入组，落在第一个可操作项（PNG）。
- 垂直组的 Down 三次依次报告 pdf → jpg → png（跳过禁用的 SVG 并在末尾循环），Up 从首项回到末项，Home/End 分别选中 PNG/JPG，Left+Right 不产生任何上报。
- 水平组相反：Right 报告 pdf，Up/Down 不产生上报。
- Space 选中焦点项并上报一次；Enter 再按已选项不上报。
- Button 外观重复同一套契约：仍是 `Role::RadioGroup` + `Role::RadioButton` + checked + 禁用标记，Tab 仍一步落在选中段，Right 越过禁用段选择并上报，Up 不响应，点击未选段照常上报。
- 分段行的选项沿所在轴首尾相接：水平行 origin.x 递增，垂直列 origin.y 递增。

不在测试范围内的：halo 的扩散、按压缩小的中间帧、中心点生长的节奏——观察层只取每帧快照，不采样逐帧过渡；`aria-orientation`、`position_in_set`/`size_of_set` 在这层没有 getter，只有代码写入这一侧可证；分段的轮廓、分界线、填充色和字重是绘制结果；焦点环是否只在键盘后出现、Windows DPI 下的实际像素、以及屏幕阅读器真正播报什么，都只在 Showcase 与实机检查。

其余合约保持：

- enabled 使用点击或 Space/Enter 更新调用方状态；disabled 不触发变更。
- 每个选项都有非空可访问名称，Base Radio 负责开关语义与单项焦点。
- 运行 `cargo check --workspace`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 和 `cargo fmt --all -- --check`，以及 `node tools/sync-yue-theme.mjs --design-system "D:\100_Projects\110_Daily\Yue-Design-System" --check`。

## 与草案的差异

设计稿的示例 API 是契约草案，实现时按编译与行为核对做了这些收敛，语义保持不变：

- `.orientation(RadioOrientation::Horizontal)` → `.direction(GroupDirection::Horizontal)`。方向和 Switch/Checkbox 组完全同构，因此抽到 crate 顶层的 `GroupDirection`，不再为每个控件造一个 orientation 枚举。
- 组的 `on_change` 签名是 `Fn(SharedString, &mut Window, &mut App)`，刻意不带 `&ClickEvent`：组的选中值也会因方向键移动而改变，为一次按键伪造一个点击事件会报告用户从未按下的按钮。单项 `YueRadio::on_change` 仍是 `Fn(bool, &ClickEvent, &mut Window, &mut App)`，那条路径确实只由指针或激活键触发。
- 组的容器是 Yue 自己的 `div`（带 `role(RadioGroup)`、`aria_orientation`、`aria_label` 和 `on_key_down`），不是 Base 的 `RadioGroup`：容器必须同时是"能被观察到的节点"和"接收方向键的元素"，Base 那层两者都提供不了，且它不持有值。
- 分段外观不换行（草案写的是"Horizontal 遇窄窗可换行"）。换行的分段行失去连续轮廓的可读性，因此 Indicator 水平组保留 `flex_wrap`、分段组保留单行并在文档里把长标签指向 Indicator/Select。
- 分段外观没有滑动指示块（thumb）、没有图标、段间没有 gap：它是描边一次的分段行，选中只改填充、文字与字重。
- 组的键盘处理需要 `on_change` 才挂载（草案未提）；没有回调的组没有可写入 value 的调用方，接管方向键只会吞掉页面的滚动。
- `set_position/set_size` 只对组内选项投影（1 基、含禁用项），组外的单项 Radio 不属于任何集合，不投影该元数据。
- 设计稿"Radio 组需实现 roving tabindex"的落点：tab stop 的判定与句柄的创建都在组的 render 里完成，选项通过 crate 内部的 `track_focus/tab_stop/set_position` 接收，不作为公共 API 暴露——调用方不需要（也不应该）自己协调这三件事。
