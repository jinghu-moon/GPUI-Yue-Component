# 06 YueCheckbox

## 目标

提供独立 Checkbox 与多选组，表示可独立成立的选择；参考图覆盖单项、受控多选、组选项、全选/半选、禁用与只读示意。Checkbox 可同时选择多个项目，与 Radio 互斥选择的语义不同。

`YueCheckbox` 与 `YueCheckboxGroup` 已按这份契约落地（`crates/yue-component/src/checkbox.rs`），勾号与短横按开关那条路子用 GPUI 矢量现画。下面每一节写的都是代码里实际发生的事；与设计稿草案不一致的地方集中在最后一节。

## GPUI Kit 0.8.0 能力

实现前逐项检索过源码，结论与设计稿的判断一致：

### `gpui_kit::base::checkbox::Checkbox`

- 受控状态 `CheckboxState::{Unchecked, Checked, Indeterminate}`；激活后 Unchecked/Indeterminate 变 Checked，Checked 变 Unchecked。
- 提供 CheckBox role、toggled False/True/Mixed、label、disabled、focus handle、Tab stop/index，以及 pointer/keyboard activation。
- `on_change` 报告下一 `CheckboxState`，应用需回写，不保存第二份业务值。
- `CheckboxIndicator` 提供 checked、indeterminate、disabled 的状态样式投影；绘制和标签仍归上层。
- Base 没有 CheckboxGroup、选项集合、全选聚合或只读状态，也没有可复用的 `aria-readonly` setter。
- 核对到的两条额外事实：`disabled(true)` 的复选框不再挂 `track_focus`，因此在观察层里是"没有焦点节点"而不是"未聚焦"；Base 默认 `tab_stop(true)`，所以可用的单项天然各自是停靠点。

实现没有使用 `CheckboxIndicator`：方框、勾号、短横和焦点环全部由 Yue 画，因为勾号要按 demo 那样逐段起墨，而状态样式投影只提供"画到哪一档"的开关，拿不到笔画进度。Base 侧仍提供交互、焦点、role 与 toggled 状态。

## 公共 API

```rust
// 独立控件
YueCheckbox::new("terms", theme, CheckboxState::Unchecked)
    .label("同意条款")
    .size(CheckboxSize::Medium)
    .disabled(false)
    .on_change(|next: CheckboxState, event: &ClickEvent, window, cx| { /* 写回 next */ });

// 多选组
YueCheckboxGroup::new("permissions", theme, selected_permissions) // 任意 IntoIterator<Item: Into<SharedString>>
    .label("权限")
    .select_all("全选")
    .direction(GroupDirection::Vertical)
    .options([
        YueCheckboxOption::new("read", "读取"),
        YueCheckboxOption::new("write", "写入"),
        YueCheckboxOption::new("admin", "管理").disabled(true),
    ])
    .on_change(|values: Vec<SharedString>, event: &ClickEvent, window, cx| {
        update_permissions(values);
    });
```

- 状态用 Yue 自己的 `CheckboxState`，渲染时再映射到 Base 的等价状态，消费者不需要命名 `gpui-base` 的内部模块路径。
- `on_change` 只报告下一状态或新的完整选择，控件不保存第二份业务值；调用方写回并重绘。
- 触发 `Indeterminate` 控件请求 `Checked`，不会直接变成 `Unchecked`。
- 组的 `on_change` 报告的是整份新集合而不是 delta，因此调用方每次只写一个值；单项路径带 `&ClickEvent`（一定由指针或激活键触发），组的批量与逐项变更也各带一个真实事件。
- 选项 ID 由 `(组 id, value)` 派生；`.options(...)` 断言 value 唯一，重复时 panic 并点名重复的 value。
- 单项必须有 `.label(...)` 或 `.accessibility_label(...)`，两者皆无时 panic。
- 整组禁用与逐项禁用的 `disabled` 都阻断激活与上报，但保留并继续显示已有的选中态。

## 变体

- 尺寸：`CheckboxSize::{Small, Medium, Large}`，默认 Medium，三档同时决定方框、行高、字号与笔画粗细：

  | 档位 | 方框 | 行高 | 字号 | 笔画宽度 |
  | --- | --- | --- | --- | --- |
  | Small | 12px | 28px | 12px | 1px |
  | Medium | 16px | 32px | 14px | 1.5px |
  | Large | 20px | 40px | 16px | 2px |

  勾号的形状按档缩放，笔画不：`--checkbox-mark-stroke-width-sm/md/lg` 每档是一个绝对像素值，与开关字形的重量（24 单位框上的 3 单位落在 `--switch-indicator-size-*` = 8/12/16px 上）逐档相等，所以并排看时 ✔、半选横线、✕ 是同一支笔。参考图自己两处的比例并不相等（`.check svg` 的 2 单位摊在 14px 内容框上、`.sw .ico` 的 3 单位摊在 12px 图标框上），这里以观感为准把它对齐。
- 方向：`GroupDirection::{Vertical, Horizontal}`，默认 Vertical。
- 三态：`Unchecked`（表面底 + 控制边框）、`Checked`（accent 实心 + 勾号）、`Indeterminate`（同款实心 + 短横）。参考图把 mixed 框填成和 checked 一样，只是笔画不同，所以"是否上 accent"判断的是 `chosen()`（非 Unchecked），而不是"是否等于 Checked"。
- 组内选项只有 Checked/Unchecked 两态——它们的态由 `selected.contains(value)` 派生；`Indeterminate` 只可能出现在全选头或调用方自己设定的独立控件上。

## 全选头的检查与派生

给定 enabled 集合 `E` 和 selected 集合 `S`：

| 条件 | 全选状态 |
| --- | --- |
| `E` 为空或 `S ∩ E` 为空 | Unchecked |
| `E ⊆ S` | Checked |
| 其他情况 | Indeterminate |

批量勾选将 `S` 更新为 `S ∪ E`；批量取消更新为 `S − E`。Disabled 子项属于 `S` 时保留，但不计入"可全选集合"的完成度。

实现里对应 `select_all_state`、`eligible_values` 与 `apply_select_all` 三个纯函数，行为要点：

- 头的状态从不存储，每次渲染由 options 与当前选择现算，因此不会和具体选择值失步。
- `E` 只看选项自己的 `disabled`：整组禁用改写的是"头能不能操作"，不改写"头说什么"。所以一个禁用组的头仍是三态齐备的，只是点不动。
- `E` 为空（空组或全禁用组）时头自动禁用。
- 勾选按 `E` 的顺序追加、去重；清空只减 `S ∩ E`；调用方选中的、本组没提供的 value 原样穿过，不会被顺手丢掉。
- 半选态被点击时是"填满"而不是"清空"（Base 的 Indeterminate→Checked 规则）。
- 头的文字既是可见标签也是可访问名称（`Role::CheckBox` + 该名字），并且始终排在选项之前。

## 键盘与焦点

- 每个可操作项（含全选头）都是独立的 Tab 停靠点；组不实现 roving tabindex，Tab 会逐项走过，Shift+Tab 原路返回。这是 Checkbox 与 Radio 组在键盘上的核心区别：多选没有"一个集合只有一个入口"的必要。
- 组完全不接管方向键——Up/Down/Left/Right/Home/End 一律留给页面（外层滚动视图照常滚动）。
- Space/Enter 切换焦点项；点击整行（方框或标签）同样切换。
- 禁用项不注册焦点句柄，Tab 直接跳过。
- 焦点环只在键盘聚焦后出现（读 `last_input_was_keyboard`，与 GPUI 自己的 `focus_visible` 条件一致），鼠标点击不给环。
- 没有只读开关：见"Readonly 边界"。

## 为什么勾号自己描边

和开关同一个理由：GPUI 的 `svg()` 元素把整张 SVG 一次性栅格化，没有按 `<path>` 控制 dashoffset 的入口，拿不到 demo 那条"墨逐步走出"的动画。

- 方框与勾号都画在 `canvas` 上，路径由 `PathBuilder` 现场搭建（`mark_layer`、`paint_mark`）。
- 坐标沿用参考图的 `viewBox="0 0 16 16"`：勾号是 demo 的 `d="M4 8.5l3 3 5-6"`（两段：3,3 与 5,-6），短横是 `d="M4.5 8h7"`（7 单位长、水平居中于中线）。
- 点坐标先按 `(内容区 − 16 单位框)` 的比例换算，再画在方框内容区的正中央——参考图的 SVG 也定位在 `inset: 0` 的内容区，所以按压缩小方框时勾号跟着走。
- 笔画用 `dash_array(&[长度 × 进度, 长度])`，等价于 demo 的 `pathLength="1"` + `stroke-dashoffset = 1 − 进度`，墨从路径起点（勾号的左端、短横的左端）开始走。
- 线头与拐角都是圆头（`LineCap::Round`、`LineJoin::Round`）；宽度取本档的 `--checkbox-mark-stroke-width-*`，只随按压缩小（乘同一条 `scale`），不随方框大小换算。
- 勾号与短横是两条独立通道：切到 Checked 时前者起墨、后者擦墨，切到 Indeterminate 时反之，未选中两者都为 0。它们各自始终存在，不是挂载/卸载元素，所以过渡不会跳帧。

其余组件仍使用 `yue-component-icons` 的内置 Tabler 子集；只有开关和复选框为了逐笔进度而自带路径数据。

## 两层动效

| 阶段 | 变化的量 | 时长 · 曲线 · 延迟 | 取值来源 |
| --- | --- | --- | --- |
| 方框填充 | `--checkbox-background` → `--checkbox-background-checked` 的 RGB 线性混合 | 200ms · `--checkbox-ease` | token |
| 边框强调（hover 或选中） | `--checkbox-border-color` → `--checkbox-border-color-checked` | 200ms · `--checkbox-ease` | token |
| hover 光晕 | 扩散阴影半径 0 → `--checkbox-halo-width`(4px)，颜色按进度取 alpha | 200ms · `--checkbox-ease` | token |
| 勾号 / 短横起墨 | 描边进度 0 → 1 | 200ms · `--checkbox-ease-enter` · 延迟 100ms | token |
| 勾号 / 短横擦墨 | 描边进度 → 0 | 100ms · `--checkbox-ease-exit` · 无延迟 | token |
| 按压缩小 | 方框（含笔画、光晕、焦点环）缩放 → `--checkbox-press-scale`(0.9) | 100ms · `--checkbox-ease` | token |

- 起墨比状态切换晚一档（延迟取 `--checkbox-duration-fast`，即框填充到一半时起笔），擦墨则在淡出之前就走完，因此勾选读起来是"填实—再画勾"，取消是"先收笔—再退色"。
- 开关的 ✔/× 现在也走这同一支时钟：`--switch-duration` = 200ms 书写、`--switch-duration-fast` = 100ms 延迟与擦除，`switch.rs` 的单测逐档比对两组件的这两条时长。
- 每个通道一条独立 keyed transition（`yue-checkbox-fill`/`-border`/`-halo`/`-check`/`-dash`/`-press`），进度一律 clamp 到 0..1；一次指针移动或释放只改一份 keyed 状态（`Pointer { hovered, pressed }`），只付一次重绘。
- 禁用态不过渡而是直接重画——参考图的 disabled 规则就是整块替换填充；禁用时不挂 pointer 监听，因此既不缩也不发光。
- 焦点环不进过渡通道，但它乘在 `scale` 上，所以按下缩小时环一起收，不会脱开方框。
- hover 光是扩散阴影而不是布局边框，方框和它旁边的标签都不会因为悬停而移动。

## 布局与方向

- 组容器（Yue 自己的 `div`）始终纵向堆叠：说明文字 → 全选头 → 选项行，`items_start` 对齐，间隔 `--checkbox-group-gap`（8px）。因此水平组的说明和头仍然在行上方，不会被塞进行内。
- 选项行自己按 `direction` 排：Vertical 是列，Horizontal 是行且允许换行（`flex_wrap`），窄容器里长标签折到第二行仍属于同一个组。
- 有全选头时，Vertical 的选项列向右缩进 `--checkbox-group-indent`（24px），把子项挂在头下面；Horizontal 没有这一列可挂，因而不缩进。
- 方框装在固定宽高的槽位里（`flex_shrink_0`），按压缩放只动槽内元素，标签不抖。

## Token

几何、颜色、环、间距与动效全部来自设计系统 `packages/tokens/src/component-tokens/checkbox.css`，渲染侧只读 `YueTheme` 的 token，不另造第二份调色板：

- 尺寸：`--checkbox-size-sm/md/lg`、`--checkbox-height-sm/md/lg`、`--checkbox-label-font-size-sm/md/lg`（`--checkbox-size` 保留为 Medium 别名）。
- 方框：`--checkbox-border-width`、`--checkbox-border-radius`、`--checkbox-border-color`、`--checkbox-border-color-checked`、`--checkbox-background`、`--checkbox-background-checked`、`--checkbox-mark-color`、`--checkbox-accent-color`。
- 笔画：`--checkbox-mark-stroke-width-sm/md/lg`（绝对像素的 1 / 1.5 / 2px 三档，值来自 `--border-1`、`calc(var(--border-1)*1.5)`、`--border-2`）。
- 状态：`--checkbox-halo-width`、`--checkbox-halo-color`、`--checkbox-press-scale`、`--checkbox-focus-ring-color/width/offset`、`--checkbox-disabled-background`、`--checkbox-disabled-border-color`、`--checkbox-disabled-mark-color`。
- 文字与分组：`--checkbox-label-color`、`--checkbox-label-color-disabled`、`--checkbox-label-gap`、`--checkbox-group-gap`、`--checkbox-group-indent`。
- 动效：`--checkbox-duration`、`--checkbox-duration-fast`、`--checkbox-ease`、`--checkbox-ease-enter`、`--checkbox-ease-exit`。

所有值引用现有 semantic/primitive token（`--control-*`、`--surface`、`--border-*`、`--focus-ring-*`、`--disabled-*`、`--motion-*`、`--on-accent`），实现里没有一处原始颜色；勾号的笔画宽度也不借 Button 的交互色。禁用保留 `--disabled-content` 角色给笔画，所以"禁用但仍勾选"的方框还能读出勾选，只是不主张 accent 对比。

修改后同步 catalog：

```powershell
node tools/sync-yue-theme.mjs --design-system "D:\100_Projects\110_Daily\Yue-Design-System" --write
```

一处已知边界：`--checkbox-duration` / `--checkbox-duration-fast` 在 token 源里别名到 `--motion-duration-enter/exit`，设计系统用 `@media (prefers-reduced-motion: reduce)` 把这两个语义角色改成 0ms；`generated.rs` 存的是 `:root` 解析值（四个 profile 都是 200ms/100ms），媒体查询不会进入 Rust 快照，因此 Yue 目前不会随系统的"减少动效"设置落定。这是待补的口子，不是已实现的行为。

## Readonly 边界

截图含"只读"概念，但 GPUI Base Checkbox 0.8.0 只实现 enabled/disabled，没有 readonly 或 `aria-readonly`。禁用表示当前不可操作；只读表示值不能更改但仍可聚焦/读取，两者不能用同一语义冒充。

第一版不发布 `.readonly(true)`，也不把只读组选项伪装为 disabled。需要只读展示时由调用方呈现静态状态/文本，或在产品确认 Windows UI Automation 对应状态和行为后单独设计 Yue 适配；不可用 color/opacity 作为唯一提示。若未来提供只读模式，应写清它在无障碍树中的降级能力，并做 Windows 屏幕阅读器验证。

Showcase 里因此没有只读开关，只有明确标注的禁用组与禁用项。

## 无障碍

- Checkbox 节点需要 CheckBox role、名称和 False/True/Mixed 状态；Base 提供这些基本语义，Yue 负责把名称与状态传下去。
- 独立控件由 visible label 命名；纯图形控件必须提供 accessible label（缺失时直接 panic，不让无名控件上线）。
- 组名由 Yue 自己的容器承担：容器带 `Role::Group` 与 `aria_label`，可见说明文字与它同源，因此树里能看见"选项坐落在这个可命名的容器内"。这仍是 GPUI 观察层可见的容器 label，不等于已经证明 Windows UIA 把它读成 group。
- `aria-required`、`aria-invalid`、`aria-readonly` 不是可直接复用的 Base 行为。校验使用 YueField 可见文案；不能声称必填、错误或只读已通过 Windows UIA 暴露。
- Indeterminate 是 Mixed，不等同于 checked；全选控件的中间态来自子项聚合。

## Showcase 与验收

Windows Showcase 的 Checkbox 页面按这份契约提供可交互样例：

1. **Single options**：未选中、已选中、半选、禁用四项（禁用的那个本身是勾选态，用来说明禁用仍保留选中信息），加一个只有方框的控件（靠 `.accessibility_label("只有方框的选项")` 命名）；每项都由 Showcase 状态回写，可反复点击。
2. **Size ladder**：Small / Medium / Large 三行，方框、勾与 dash 一同缩放。
3. **Controlled multi-select**：三个选项的组 + "重置"按钮（回到初始 `["docs"]`），下面显示当前 value。
4. **Grouping A: app-drawn select-all**：应用自己画全选行，组合独立 `YueCheckbox`；派生逻辑写在 Showcase 侧。这一组的结构只是标题文字，不带 Group 语义——小节标题下明确说了这点。
5. **Grouping B: built-in select-all**：`YueCheckboxGroup` + `.select_all("全选")`，禁用项不参与三态判断也不会被全选或清空。
6. **Mixed, locked and empty**：半选头 + 个别禁用项、整组禁用（`.disabled(true)` + 头）+ 空选项组（无 option 的组，头点不动）、`.w_72()` 窄容器里的长标签水平组。
7. **Keyboard**：五条说明——Space/Enter 切换焦点项、Tab 逐项走过、每个可操作项都是独立停靠点、多选组没有漫游焦点因此方向键不属于它、没有只读开关。
8. **Theme profiles**：Light·Azure / Dark·Azure / Light·Neutral / Dark·Neutral 四格，每格一个带全选的三选项组。

`cargo test -p yue-component`（45 项，其中 checkbox 12 项）钉住的是这些纯派生值：

- `chosen()` 的判断：Unchecked 不上 accent，Checked 与 Indeterminate 都上（参考图填 mixed 与填 checked 一样）。
- 名称来源：label 与 accessibility_label 各自可用，两者皆无时 panic。
- 全选派生表逐格：空选择 / 部分 / 全集 / 空组，以及"未选中的禁用项不会把头压在半选""禁用项自己的值不算进度""全禁用组的合格集为空"。
- 头的集合运算：勾选追加 `S ∪ E` 且把禁用项的原值留在原位、清空只减 `S ∩ E`、完整组再点不产生重复、本组没提供的 value 原样穿过。
- `eligible_values` 在空组与全禁用组都返回空集。
- 单项切换幂等且保序：重复选择仍是一份、清除未选中的值不改集合。
- `.options(...)` 拒绝重复 value（panic 测试）。
- 两条笔画是参考图路径：勾号起点 `(4, 8.5)`、两段位移 `(3,3)` 与 `(5,-6)`；短横 7 单位、水平居中、落在中线；所有点都在 16 单位框内。
- 三档尺寸的具体 token 值（12/16/20、28/32/40、12/14/16、1/1.5/2px），逐档校验方框小于行高、单位长度为正、笔画就是这一档的字面量宽度。
- 动效：`--checkbox-duration` = 200ms、`--checkbox-duration-fast` = 100ms（同时也是起墨延迟与擦墨时长、按压时长）、`--checkbox-press-scale` = 0.9、`--checkbox-halo-width` = 4px；三条 easing 在 0/1 端点落在 0/1、20 步采样不回绕不越界。
- 颜色：四个 profile 下填充与笔画都不透明、禁用填充比选中更暗、禁用边框 alpha 为 0、hover 光是透明着色而非实心。

`cargo test -p yue-component --features headless-tests`（checkbox 11 项）在无头元素树上驱动真实交互，钉住的是这些行为：

- 组容器是 `Role::Group`；四个选项各是 `Role::CheckBox`、名字为可见标签、checked 反映调用方持有的选择、`indeterminate == Some(false)`，禁用项报 `disabled == Some(true)`。
- 逐项点击各上报一次，值集合依次是 `[邮件, 任务]` → `[任务]`；连点两次不产生重复值。
- 禁用项与整组禁用都不产生任何上报；禁用项仍在树上报告自己的勾选态（信息不丢）。
- 全选头只聚合它有权改动的项：从"只有禁用的日历被选中"起步，点头部选择 `[日历, 邮件, 任务, 备忘]` 且头在树上读作 Checked；点掉邮件后头变成 `indeterminate == Some(true)`；半选时再点头部是填满（`[日历, 任务, 备忘, 邮件]`）而不是清空；头回到 Checked 后再点则落回 `[日历]`——禁用项的值始终保留。
- 空选项组的头在树上报 `disabled == Some(true)`，点它没有任何上报。
- 头与每个可用项各是一个 Tab 停靠点：Tab 顺序为 头 → 邮件 → 任务 → 备忘 → 后一个控件，禁用的日历没有焦点节点（`focused == None`），Shift+Tab 原路退回。
- Space 与 Enter 各自切换焦点项；初始未选中时按 Space 报 `[邮件]`。
- 方向键（down/up/right/left）不产生任何上报。
- 独立控件处于 Indeterminate 时激活请求 Checked，再点则请求 Unchecked。
- 选项沿所在轴排布：垂直列 origin.y 递增，水平行 origin.x 递增且四项同在一行（`origin.y` 相同）。

不在测试范围内的：halo 的扩散、按压缩小的中间帧、起墨延迟与描边进度合成的观感——观察层只取每帧快照，不采样逐帧过渡；焦点环是否只在键盘后出现、`canvas` 真正画出的像素、笔画圆头与 22px/48px 缩放下的对齐、禁用组下全选头的三态配色、Windows DPI 下的实际像素，都只在 Showcase 与实机检查。没有一条断言证明屏幕阅读器在 Windows 上会播报什么。

其余合约保持：

- enabled 使用点击或 Space/Enter 更新调用方状态；disabled 不触发变更。
- 每个控件都有非空可访问名称，Base Checkbox 负责开关语义与单项焦点。
- 运行 `cargo check --workspace`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 和 `cargo fmt --all -- --check`，以及 `node tools/sync-yue-theme.mjs --design-system "D:\100_Projects\110_Daily\Yue-Design-System" --check`。

## 与草案的差异

设计稿的示例 API 是契约草案，实现时按编译与行为核对做了这些收敛，语义保持不变：

- `.select_all(true)` → `.select_all("全选")`：全选头是一个真实的 Checkbox 控件，必须有自己的名字，布尔开关给不出这个名字。 builder 的存在即表示"要头"，与草案的语义等价。
- 组的 `on_change` 是 `Fn(Vec<SharedString>, &ClickEvent, &mut Window, &mut App)`：报告整份新选择而非 delta，并带真实事件（Radio 组刻意不带，因为方向键也会改值；这里每一项改动都对应一次激活）。
- 独立控件的 `on_change` 收到的是 Yue 自己的 `CheckboxState`，不是 Base 的同名类型——`to_base`/`from_base` 在渲染边界两侧映射，避免公共 API 让消费者依赖 `gpui-base` 的模块路径。
- `YueCheckboxGroup::new` 的第三个参数接受任意 `IntoIterator<Item = impl Into<SharedString>>`（内部收进一个 `Vec<SharedString>`），所以 `&["read","write"]` 或 `Vec<String>` 都能直接传，不必先转换成 `Vec<SharedString>`。
- 方向枚举与 Radio 共用 crate 顶层 `GroupDirection`，没有再造 `CheckboxOrientation`。
- 组的容器是 Yue 自己的 `div`（带 `role(Group)` 与 `aria_label`），Base 完全没有 CheckboxGroup 这一层，因此没有"复用后被语义缺项绊住"的可能，也没有继承 orientation 投影——组的轴向只在布局里体现，容器不带 `aria_orientation`（草案里"组有可访问名称和 orientation"这一条，Checkbox 侧只做到了名称）。
- 整组禁用时全选头的三态仍然照算（只是点不动）：草案说"全禁用或没有可选项时全选控件不可操作"，实现把这条精确成"合格集为空才不可操作"，而组级禁用走的是另一条路——它让头失效，但不改写头说的话。
- 组不为选项产生 `Indeterminate`：选项态由 `selected.contains(value)` 派生，只有头和调用方自持的独立控件能显示混合态。
- 支持两种组合（应用自绘全选 + 独立 `YueCheckbox`，或组的 `options` + 内置头）这条保持不变；两者共用同一套值语义，没有隐式 uncontrolled 模式。
