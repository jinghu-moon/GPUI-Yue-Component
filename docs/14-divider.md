# 14 YueDivider 分隔线

日期：2026-11-01。依据用户提供的 TDesign Divider 截图实现；沿用 GPUI Kit Base-only 与 Yue token 体系。

## 合同

`YueDivider` 为无状态、非交互的 `RenderOnce` 元素：水平 / 垂直、实线 / 虚线 / 点线，水平文字支持 Start / Center / End。默认水平实线、无文字、占满父容器宽度；有文字时默认居中。配置由宿主保存，组件不新增焦点、事件、动画或业务 Entity。

```rust
YueDivider::new("section-rule", theme);
YueDivider::new("details-rule", theme)
    .label("详细信息")
    .align(DividerAlign::Start)
    .dashed(true);
YueDivider::new("dotted-rule", theme)
    .line_style(DividerLineStyle::Dotted);
YueDivider::new("toolbar-rule", theme)
    .layout(DividerLayout::Vertical)
    .height(gpui_kit::rems(1.5));
```

- `DividerLayout::{Horizontal, Vertical}`；`DividerAlign::{Start, Center, End}`。
- `DividerLineStyle::{Solid, Dashed, Dotted}`，通过 `.line_style(...)` 选择。保留 `.dashed(true / false)` 作为 Dashed / Solid 的兼容写法；多个线型 setter 按最后一次调用生效。
- `.label(text)` 只用于水平分隔线；空白文字按无文字处理。垂直线与非空 label 组合在渲染时明确拒绝，调用方将文字作为相邻元素组合。
- `.theme(theme)` / `.use_default_theme()`；外部 `width / min_width / max_width / margin / flex_grow` 及独立 `height`，不开放任意交互子元素。
- 实线使用静态 Canvas 绘制填充细条；虚线与点线使用同色的独立填充段。虚线为四倍线宽实段、三倍线宽空隙；点线为直径等于线宽的圆点、三倍线宽空隙。末段裁到实际长度。文字用 Flex 分割左右线段，不用背景色覆盖线条，不增加 Entity、定时器或逐帧任务。

## 规格与语义

| 部分 | 来源 |
| --- | --- |
| 颜色 | `--divider-border-color`；桌面主题层映射浅色 `--neutral-400` / 深色 `--neutral-700`，保留 neutral tone 色调 |
| 线宽 | `--divider-border-width`，默认 1px 基准，按 rem 缩放 |
| 虚线 | 默认 4px 实段 / 3px 空隙，均为线宽倍数，随 rem 缩放 |
| 点线 | 默认 1px 圆点 / 3px 空隙，均为线宽倍数，随 rem 缩放 |
| 外部间距 | `--divider-margin-block`，默认 12px；水平为上下间距，垂直为左右间距 |
| 文字间隔 / 短边线段 | `--space-16` / `--space-24` |
| 文字 | `--font-size-md` / `--line-height-md` / `--text-primary` |
| 垂直默认高度 | `--line-height-md`；父级工具条需要更高时显式 height |

参考图借鉴线条轻量、水平 / 垂直和带字三种对齐，不复制网页的配置侧栏、代码操作、变更日志和截图中的内容文案。长标签在窄容器内换行，两侧线段按可用空间收缩，文字保持可读。

当前 GPUI 没有 Separator role：纯线条不伪造角色；带字根节点为 Caption，并提供与可见文字一致的名称。均不占 Tab 停靠点，不吞鼠标事件，不替代可交互 Splitter；真实文档分节仍由宿主提供标题结构。

## 演示与验证

运行 `cargo run -p windows-showcase`，进入 **组件 → Divider**。页面演示基础水平 / 垂直、三种线型、三种文字对齐和四种主题，并提供线型分段选择、方向、文字显示与对齐的实时配置。切换垂直后文字相关配置禁用，线型仍可切换；切回水平后恢复文字设置和当前线型。

2026-11-01 自动化验证：

- Divider 的 7 项定向测试通过：默认值与空白标签、全主题 token、水平满宽、三种对齐、窄容器中文换行、垂直尺寸、Tab 顺序；布局在 16px / 24px rem 下验证线宽与间距。
- Showcase 新增真实点击配置测试，验证预览、禁用状态和水平设置恢复；Showcase 全部 11 项 headless 测试通过，包含侧栏导航到 Divider。
- `cargo test --workspace --locked --features headless-tests --no-fail-fast` 通过（组件库 178 项单元测试，另含集成测试与 doctest）。
- 默认 feature 的 workspace 测试通过；`cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings`、`cargo fmt --all -- --check`、token 快照检查通过。
- `--no-default-features --features headless-tests` 下 Divider 的 7 项定向测试同样通过，兼容宿主自行配置字体。

### 2026-11-01 线条不可见修复

- 原始源快照颜色定义完整：浅色 `#eee`、深色 `#2b2b2b`；Slate / Stone / Haze 也有独立覆盖。逐一验证两种模式、两种 accent、四种 neutral tone 的精确 RGBA 值与不透明度，并验证静态 key 与 CSS 名称一致，排除颜色缺失或解析成透明色。当前显示颜色见下方可见性调整。
- 原绘制将元素高度 / 宽度设为线宽，仅设置 `border_t` / `border_l`。在 1 个物理像素时，唯一像素的采样中心恰好位于矩形中线；当前 GPUI Windows border shader 在中线选择 bottom / right，而它们宽度为零，导致横线 / 竖线丢失。改用填充矩形，避开单边边框的退化几何；保留原颜色、线宽、文字、间距和公共 API。
- Divider 的 9 项定向测试通过。新增绘制场景回归检查浅 / 深色、16px / 24px rem、水平无字 / 带字 / 垂直、实线 / 虚线，验证填充颜色、线段尺寸、连续实线和虚线间隙；已有对齐、换行与焦点测试继续通过。
- 本次复验：默认及 `--no-default-features` 的 Divider headless 测试各 9 项通过；Showcase 配置交互测试通过；组件库与 Showcase 的全 target headless Clippy（`-D warnings`）、`cargo fmt --all -- --check`、Showcase 原生构建通过。

### 2026-11-01 可见性与线型调整

- 加强对比：默认浅色改为 `#c7c7c7`，深色改为 `#525252`；Slate / Stone / Haze 沿用各自相同级别的灰阶。映射集中在 `YueTheme::resolve`，组件仍读取 `--divider-border-color`；生成文件、上游源快照和其他组件颜色保持不变。
- 虚线由 2px 实段 / 1px 空隙调整为 4px 实段 / 3px 空隙；新增圆点线型，默认 1px 圆点 / 3px 空隙。水平、垂直和带文字布局共用相同绘制路径。
- 演示页以实线 / 虚线 / 点线分段选择替代原虚线开关，并增加水平、垂直点线示例。
- 自动化覆盖三种线型的浅 / 深色、16px / 24px rem 绘制；检查精确颜色、实段长度、间隙和圆点圆角，以及旧 `.dashed(...)` API 和配置切换。
- 本次验证：组件库 223 项 headless 单元测试、10 项无默认 feature 的 Divider 定向测试、Showcase 配置交互测试通过；Clippy（`-D warnings`）、格式和 token 同步检查通过。常规 Showcase 输出文件拒绝覆盖，改用 `cargo rustc -p windows-showcase --locked --bin windows-showcase -- --emit=link=target/debug/windows-showcase-divider.exe` 成功生成独立预览程序。

验收分工：自动化测试覆盖布局、语义和实际提交的填充图元；真实 UI 与 Windows GPU 显示效果由用户检查，开发流程不执行实窗自动化验收。
