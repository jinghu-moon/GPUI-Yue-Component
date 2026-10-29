# 04 YueSwitch

## 目标

提供可控、键盘可操作且具备 Switch 无障碍语义的二值开关。GPUI Base 0.8.0 管理交互、焦点和 `Role::Switch`；Yue Component 呈现尺寸、轨道形状、滑块、状态色，并还原参考 demo 的图标动效。

## 公共 API

```rust
YueSwitch::new("wifi", theme, enabled, "Wi-Fi")
    .size(SwitchSize::Medium)
    .shape(SwitchShape::Capsule)
    .indicator(SwitchIndicator::CheckX)
    .loading(pending)
    .disabled(false)
    .on_change(|next, _, _, _| {
        // 将 next 写回调用方持有的状态
    })
```

`new` 接受调用方当前的 `checked` 值和必需的可访问名称。`on_change` 仅报告下一状态，不在组件内部保存第二份业务状态；调用方更新自己的状态并重新渲染。Base Switch 提供点击、Space 键、Tab 焦点、disabled 拦截和 `aria-checked`。

`loading(true)` 表示一次异步提交正在进行：滑块停在原位转加载环，× 与 ✓ 淡出到 0，`on_change` 在此期间不上报，因此慢提交不会被连点两次；按下仍然拉宽滑块。调用方在延迟之后写回新的 `checked` 并清掉 `loading`，目标图标的描边会重新播放一次。

## 变体

- 尺寸：`Small`、`Medium`、`Large`。三档都满足 `轨道高 = 滑块 + 2×inset`，滑块靠 `top: inset` 落在轨道正中；轨道不吃边框（见下）。
- 形状：`Capsule`、`RoundedRectangle`；轨道和滑块均使用对应轮廓 token。
- 轨道无边框（同 demo 的 `border: 0`）。GPUI 的绝对定位子元素从内容盒起算，轨道一旦留 1px 边框，`top: 2px` 实际从边框内侧起算，滑块就会偏下——投影与居中问题都来自这里。`--switch-border-width` 现在只供焦点环使用，`--switch-border-color` 不再被消费。
- 滑块投影：`0 1px 2px` 黑 20%，深色 profile 提到 50%（`switch.rs` 的 `thumb_shadow`）；`disabled(true)` 时不投影，与 demo 的 `box-shadow: none` 一致。Yue 的 `--shadow-*` token 是两层 CSS 字符串、没有单层等价物，所以这两组数仍留在代码里。
- 指示器：`None`、`Check`、`X`、`CheckX`。四个图标层始终按同一套进度值淡入淡出，"看不见"是透明度为 0 的同一个图标，而不是挂载或卸载元素：`CheckX` 让 × 与 ✓ 反向交叉，`Check` 与 `X` 只挂对应那一层，随 `checked` 淡入或淡出。
- 颜色按图标绑定，不随 `checked` 换色：× 恒用次级文字色（`--switch-indicator-color-unchecked`），✓ 恒用 accent（`--switch-indicator-color-checked`）。轨道在开启时切到 accent、关闭时切回中性灰，滑块两态都保持前景表面色。
- `disabled(true)` 阻断鼠标与键盘激活，并应用禁用透明度。

## 两层动效

图标分成两层驱动：容器层照 demo 的 `<svg class="ico">`，描边层是 Yue 在 demo 之外加的一层。

- **容器层**：轨道变色、滑块位移与宽度、图标的透明度/旋转/缩放，全部走 `--switch-duration`（200ms）+ `--switch-ease`。
- **描边层**：× 的两条对角线共用一条通道、✓ 单独一条，只驱动"墨走了多远"。demo 里这两个图标只有透明度与 transform，没有逐笔书写——`pathLength="1"` + `stroke-dashoffset` 那套是 demo 用在 `.check path`（多选框）上的写法，这里把它借给开关，时钟也一并搬过去：起笔延迟取 `--switch-duration-fast`（100ms）、书写取 `--switch-duration`（200ms）、曲线 `--ease-enter`；擦除只需 `--switch-duration-fast`（100ms），曲线 `--ease-exit`，因此在淡出结束之前就已完成收笔。

| 阶段 | 变化的量 | 时长 · 曲线 · 延迟 | 取值来源 |
| --- | --- | --- | --- |
| 轨道变色 | 两 token 的 RGB 线性混合 | 200ms · `--switch-ease` | token |
| 滑块位移 / 宽度 | `left`、`width` | 200ms · `--switch-ease` | token |
| 按压拉宽 | 宽度 +2×`--switch-inset-*`（4px），checked 端把增量还给位移 | 200ms · `--switch-ease` | token |
| 图标透明度 / 旋转 / 缩放 | 1↔0、0↔±45°、1↔0.5 | 200ms · `--switch-ease` | token（45° 摆角与 0.5 缩比为代码常量） |
| × 两条对角线 | 描边进度 0→1 | 200ms · `--ease-enter` · 延迟 100ms | token |
| ✓ 一笔 | 描边进度 0→1 | 200ms · `--ease-enter` · 延迟 100ms | token |
| × / ✓ 擦除 | 描边进度 →0 | 100ms · `--ease-exit` · 无延迟 | token |
| 加载环 | 一整圈循环 | `--spinner-duration` = `calc(300ms*3)` = 900ms · linear | token |

关→开：轨道与滑块起步，× 在 100ms 内擦干并旋出、缩到 0.5；✓ 同时旋入、淡入，100ms 后从左端起笔，先补到下方拐角、再沿长臂向上收完那一笔。开→关：× 的两条对角线同时写出，✓ 先擦墨再缩回、反向旋出。

设计系统没有"图标摆角"这类 token，所以 45° 摆角与 0.5 缩比写成 `switch.rs` 顶部的 `INDICATOR_SWING_TURNS`、`INDICATOR_MIN_SCALE` 两个常量。笔画节奏不再需要常量：它读的就是多选框那两档 `--switch-duration` / `--switch-duration-fast`。两边数值同今天都是 200/100ms，但来源不同一支 ladder——`--checkbox-duration*` 引用 semantic motion 角色，`--switch-duration*` 引用 raw duration；reduced-motion 媒体查询目前还落不到 Rust（见 docs/06-checkbox.md 的 Token 一节），所以暂不影响观感。

## 为什么图标自己描边

GPUI 的 `svg()` 元素把整张 SVG 一次性栅格化，没有按 `<path>` 控制 dashoffset 的入口，所以 × 与 ✓ 用 `canvas` + `PathBuilder` 现画（`switch.rs` 的 `glyph_layer`、`paint_glyph`）：× 的两条对角线端点直接沿用 demo 的 Tabler 路径数据（24×24 viewBox）；✓ 是 demo 的 `M5 12l5 5l10 -10` 按 1.1 倍重画、外接框重新落回 24 单位方框正中（`3.75,12 → 9.25,17.5 → 20.25,6.5`），因为 demo 里 × 占满 12×12 而 ✓ 只有 15×10，两者共用同一图标框交叉淡入时 ✓ 明显偏小；放大后 ✓ 竖向 11、横向 16.5 单位，仍比 × 矮 1 单位——要完全齐平得 1.2 倍，但那已在 20px 滑块里显得过大。旋转与缩放在建路径之前作用到坐标上，测出的路径长度因此和实际画出的长度一致；`dash_array(&[长度 × 进度, 长度])` 等价于 demo 在多选框那两条 `<path pathLength="1">` 上用的 `stroke-dashoffset = 1 - 进度`，墨从路径起点开始走。描边宽度随缩放（3/24 的视口比例，Medium 的 12px 图标即 1.5px），两个图标共用这一笔宽，✓ 的放大只改几何不改粗细，端点与拐点用圆头。透明度写进画笔颜色而不是元素样式，因为路径绘制不读元素自身的 opacity。

加载环不描边：demo 的 `stroke-dasharray:28 100` 就是圆周长 56 里的一段 28，等价于把一段 `arc_to` 圆弧的起点往后推，所以它由 `Animation::new(--spinner-duration).repeat()` 的 delta 直接决定起弧角度，永远匀速。

其余组件仍然使用 `yue-component-icons` 的内置 Tabler 子集；只有开关和复选框为了拿到逐条笔画的进度而自带路径数据。

## Token

尺寸几何、两态轨道/滑块/图标颜色、两种轮廓、图标尺寸、禁用透明度、动画时长与 easing、加载环颜色与周期均来自设计系统 `packages/tokens/src/component-tokens/switch.css`（`--switch-duration` / `--switch-duration-fast` 各引用 primitives 的一档时长，曲线仍直接读 `--ease-enter`、`--ease-exit`）；`--switch-border-width` 只用于焦点环。修改后运行：

```powershell
node tools/sync-yue-theme.mjs --design-system D:\100_Projects\110_Daily\Yue-Design-System --write
```

尺寸几何满足 `轨道高 = 滑块 + 2×inset`，inset 为 2px：三档轨道 32×20 / 40×24 / 48×28，滑块 16 / 20 / 24，行程 12 / 16 / 20，图标 8 / 12 / 16。按住时滑块加宽 2×inset（4px），checked 端把这段增量从位移里扣掉，前缘不动。

消费项目只依赖 `yue-component` 与 `gpui-kit`，无需直接传入 SVG 或图标资产。

## Showcase 与验收

Windows Showcase 的 Switch 页面提供受控可交互、禁用、异步提交（点击后转约 0.9 秒再落到目标位置，期间重复点击无效）三类样例，并展示三种尺寸 × 两种形状 × 四种指示器的完整组合。

`cargo test -p yue-component`（45 项，其中 switch 8 项）钉住的是这些纯派生值：

- builder 保留 size / shape / indicator / loading / disabled 选项。
- 三档尺寸的轨道宽/轨道高/滑块/inset 四组 token 值，并逐档校验 `轨道高 = 滑块 + 2×inset`；未开启轨道、两态滑块、checked 指示器颜色的具体 token 值。
- 字形笔宽（`3/24` × 本档图标 = 1 / 1.5 / 2px）与 `--checkbox-mark-stroke-width-sm/md/lg` 逐档相等——复选框的 ✔ 与半选横线和开关的 ✔、× 用的是同一支笔。
- 按压缩宽取 2×inset（4px）、禁用时不压；checked 端"位移 + 宽度"守恒，拉宽不会把滑块顶出轨道。
- 滑块投影只有一层、`0 1px 2px`、不 inset、不 spread，浅色 alpha 0.2、深色 0.5。
- × 与 ✓ 在进度 0、0.5、1 上的旋转量与缩放量精确取值，两者互为镜像。
- ✓ 路径是 demo 路径的 1.1 倍：竖向跨度 11 单位（demo 的 10 与 × 的 12 之间）、两腿仍保持 45°、外接框仍以图标框为中心，且算上圆头端点也不超出图标框。
- `--switch-duration-fast` 解析为 100ms、`--spinner-duration` 的 `calc(300ms*3)` 解析为 900ms，且 `--switch-duration` / `--switch-duration-fast` 与 `--checkbox-duration` / `--checkbox-duration-fast` 逐档相等（书写时长与起笔延迟都同一条时钟）；`--switch-ease`、`--ease-enter`、`--ease-exit` 三条曲线在 0 与 1 端点落在 0 与 1，20 步采样不回绕、不越界。
- 轨道色在进度 0、0.5、1 处沿 RGB 三轴直线插值，不绕道色相，透明度保持不变。

不在测试范围内的：`transition` 通道逐帧推进出来的实际时序手感、`canvas` 真正画出的像素、描边进度与旋转缩放合成后的观感、加载环是否在转、按压缩宽与焦点环的视觉、滑块是否真在轨道上下各留 2px、投影在两层轮廓上的观感、✓ 放大后与 × 交叉淡入时大小是否已经对齐、以及 reduce-motion 下由 GPUI Base 直接落定的行为——这些只在 Windows Showcase 里人工确认。开关暂无无头渲染断言。

其余合约保持：

- enabled 使用点击或 Space 更新调用方状态；disabled 不触发变更。
- accessibility label 必须非空，Base Switch 负责开关语义与键盘焦点。
- capsule 与 rounded rectangle 的轨道/滑块轮廓各自读取 Switch token。
- 运行 `cargo test --workspace`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 和 `cargo fmt --all -- --check`。
