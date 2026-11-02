# YueLink 链接

状态：已实现，验收记录见第 7 节。参考用户提供的 Link 截图；复用 GPUI Kit 0.8.0 Base Link。仅面向 Windows，开发期不增加兼容层。

## 1. 范围与依据

采用参考图中的五种配色（Default、Primary、Danger、Warning、Success）、三种尺寸、常显下划线、hover 下划线或变色、禁用、前后图标和独立示例页。基础构造器默认 Primary，沿用设计系统既有 `--link-color = --text-link`；Default 是可显式选用的中性文字色。参考图的页面大面积留白和覆盖率徽章不属于组件，不复制。

Link 表达资源目标，必须同时提供非空文字与 href。默认交给 `App::open_url`，由 Windows 的默认应用处理；宿主可通过 `open_with` 接管，例如使用嵌入式浏览器。应用内部命令与页面切换继续使用 Button / 导航组件，不给普通命令伪造 href。没有 HTML 的 target/new-tab 概念，不承诺浏览器创建新标签。`on_activate` 是打开策略执行后的观察通知，不替代默认打开；需要接管必须使用 `open_with`。

分类：声明式 RenderOnce、in-place、persistent、atomic。Base 负责 Link 语义、指针/键盘激活、disabled 和 Tab；Yue 负责 token 视觉、微动效、内容与外部打开默认策略。仅 hover 和 focus 保留视图局部状态，不建立业务 Entity。

## 2. 公共合同

```rust
YueLink::new("guide", "使用指南", "https://example.com/guide")
    .variant(LinkVariant::Primary)
    .size(LinkSize::Medium)
    .underline(false)
    .hover(LinkHover::Underline)
    .leading_icon(outline::Link)
    .trailing_icon(outline::ExternalLink);
```

- `LinkVariant::{Default, Primary, Danger, Warning, Success}`；默认 Primary。
- `LinkSize::{Small, Medium, Large}`；默认 Medium。
- `LinkHover::{Underline, Color, None}`；默认 Underline。常显 underline 与 hover 独立：开启常显后，Color 仍只变色，None 仍保留下划线。
- `disabled`、`accessibility_label/description`、caller-owned `track_focus`、`tab_index/tab_stop`、`key_context`、`theme/use_default_theme`，以及现有 ControlLayout 的外部布局接口。
- `open_with(Fn(&str, &ClickEvent, &mut Window, &mut App))`、`on_activate(Fn(&ClickEvent, &mut Window, &mut App))`；指针、键盘共用同一条激活路径，各只调用一次，先 open 后 observe。
- 图标复用 `IconDefinition`/Tabler SVG，尺寸随文字档位；只装饰，不重复暴露 Link 角色，不隐式添加外链图标。需要图标时调用方指定。

不增加 loading、纯图标 Link、HTML target、visited 状态、下载管理器、内置路由或多套 URL 状态。URL 作为宿主资源标识原样传递，不在组件内发网络请求或探测目标是否可达。

## 3. 视觉与交互

文字为内容宽度，无按钮背景、边框或上下 padding；字号、行高、图标间距、下划线厚度、焦点环、各语义色/hover 色/disabled 色和 motion 均来自源 `packages/tokens`，同步到本仓库后离线使用。警告/危险/成功使用正文可读的 foreground 角色，不照搬图片中对比度偏低的浅色。禁用保持语义配色的减弱关系，不整体对整个布局降低 opacity。

| 状态 | 合同 |
| --- | --- |
| 默认 | 文本及可选图标，无背景；Primary 沿用 text-link |
| Hover Underline | 文本下划线淡入/淡出，图标不被整条线贯穿 |
| Hover Color | 同语义色轻微加深/变亮；既有下划线不丢失 |
| Focus-visible | 外侧焦点环，不改变文字排版；鼠标聚焦不伪造键盘焦点环 |
| Disabled | 不打开、不通知，Tab 不停留，没有手形或 hover 动效；禁用色不承诺正文对比度 |
| Reduced motion | 直接到目标样式；不缩放/移动文本 |

Base 当前通过同一个 on_click 通道支持 Enter 与 Space；Yue 沿用此 GPUI 行为，不声称与浏览器只用 Enter 的原生 anchor 完全一致。Tab/Shift+Tab 在宿主焦点树中流转。调用方传入的焦点句柄也服从 tab_index/tab_stop；禁用时不保留可激活的原生节点。

## 4. 验收与交付

实施前 `cargo test -p yue-component --locked --lib`：114 项通过。新增测试覆盖真实 Base Root 下的 Role::Link、目标/回调顺序、默认系统打开与自定义策略、Enter/Space、禁用前先聚焦、Tab/caller focus、主题继承、三尺寸/图标/宽度、hover 动效/reduced motion 和边界配置。示例页在组件组 Input → Link → Radio，提供图片所示配置面板及完整矩阵。

无头测试只能证明语义、事件、状态、几何及提交给渲染器的颜色/下划线数据；Windows 实际浏览器启动、读屏、DPI 和 GPU 文本/下划线像素需真实窗口验证，不把 mocked open_url 当成系统已打开。

## 5. 已实现 API 与文件

| 入口 | 用途 / 默认值 |
| --- | --- |
| `new(id, label, href)` | Primary、Medium、Hover Underline、无常显下划线；空白 label/href 明确 panic |
| `variant` / `size` | 五种配色 / 三种尺寸 |
| `underline(bool)` | 独立覆盖源 `--link-decoration`；不传时读 token，目前为 none |
| `hover(LinkHover)` | Underline / Color / None；连续淡入淡出，无文字缩放 |
| `leading_icon` / `trailing_icon` | 接收任意 IconDefinition，仅装饰，无额外 Tab stop |
| `disabled(bool)` | 保留配色并降低文字/图标透明度，不打开或回调，跳过 Tab |
| `open_with` | 接管目标；签名 `Fn(&str, &ClickEvent, &mut Window, &mut App)` |
| `on_activate` | 观察通知；签名 `Fn(&ClickEvent, &mut Window, &mut App)`；open → observe |
| `theme` / `use_default_theme` | 显式局部主题 / 恢复应用主题继承 |
| `track_focus` / `tab_index` / `tab_stop` / `key_context` | 单一真实 Link 节点上的焦点与上下文 |
| `accessibility_label` / `accessibility_description` | 默认 name=label，可覆盖；不把外链图标充当名称 |
| `width` / `min_width` / `max_width` / `margin` / `flex_grow` | 外部布局，默认内容宽；不添加 Button 的 padding 或背景 |

源代码 `crates/yue-component/src/link/mod.rs`，根和 prelude 导出 YueLink；选项类型在根显式导入。集成测试 `crates/yue-component/tests/link.rs`。实际 Showcase 页面 `examples/windows-showcase/src/pages/link.rs`，侧栏按英文升序排在 Input 与 Radio 之间；保留原有组件 / 设计分组与折叠。

Base 0.8.0 原本 render 始终选用私有 keyed FocusHandle，普通 InteractiveElement::track_focus 会被它覆盖。本轮在 `vendor/gpui-base/src/link.rs` 增加显式 caller-focus builder，并让原生节点接入现有可选 test_support，和 Base Button 一致。没有复制 Base 的键盘、点击或 disabled 行为。Yue 默认焦点在组件身份下稳定保存，焦点环与该原生节点共用同一 handle。消费工作区仍需 README 中现有 `[patch.crates-io]` 配置。

## 6. Token 来源与视觉合同

所有新增参数来自 `Yue-Design-System/packages/tokens/src/component-tokens/link.css`；同步到本仓库完整 token snapshot 与 `theme/generated.rs`。源词表和 grammar 同步登记，`tests/token-link.test.mjs` 检查全部 16 个 theme/accent/neutral 组合。

| Token 族 | 当前来源 / 用途 |
| --- | --- |
| `--link-color` | 保留既有 `--text-link` 合同，Primary 使用 |
| `--link-default/danger/warning/success-color` | `--text-primary` 与各状态 foreground |
| `--link-*-color-hover` / `--link-color-hover` | 同语义色向正文色混合 20%；中性 hover 指向 text-link |
| `--link-opacity-disabled` | 全局 disabled-content 透明度，仅减弱文字与图标 |
| `--link-font-size-{sm,md,lg}` | 12 / 14 / 16px |
| `--link-line-height-{sm,md,lg}` | 16 / 20 / 24px |
| `--link-icon-size-{sm,md,lg}` | 16 / 16 / 20px |
| `--link-font-weight` / `--link-gap` | regular / space-4 |
| `--link-decoration` / `--link-underline-width` | none / border-1；真实文本下划线，图标不连线 |
| `--link-focus-ring-{color,width,offset,radius}` | 通用焦点语义 + radius-sm；外扩 offset+width，无重排 |
| `--link-duration` / `--link-ease` | interaction / standard，目前 100ms；Base reduce_motion 直接收敛 |

启用状态的 resting/hover 文字在 page、surface、surface-component 上按 4.5:1 检查，焦点色按 3:1 检查；disabled 为诊断状态，不宣称满足正文对比度。只对展示颜色做插值，不动画字号、行高或布局尺寸。任意祖先强制裁剪仍可能裁掉外侧焦点环，宿主须留出空间。

## 7. 验收记录与边界

实施前普通 lib 测试 114 项通过。新增边界测试验证空白 label/href/name 拒绝；集成测试通过真实 Base Root，覆盖系统打开请求、宿主替代策略、open/observe 顺序、指针/Enter/Space 各一次、先聚焦再禁用、父容器不误激活、caller/default 焦点、Tab stop/index/key context、三尺寸与内容宽、前后图标、显式宽、下划线不跨图标、hover 中间值/反向/减弱动画、主题继承与禁用颜色，以及外侧焦点环不重排。

Showcase 两项测试用真实控件改变配置、验证预览尺寸和禁用/恢复，再验证默认打开与宿主接管。源 token 专项 17 项检查命名和 16 个 profile/tone 组合。

Windows GPU 像素、实际读屏播报、系统默认浏览器/邮件客户端启动，以及系统高对比度自动适配没有被这些无头测试证明。组件没有内置文案，label/name/description 均由宿主传入；Showcase 中文说明不属于组件本地化机制。Vue/DOM/SSR/Teleport 检查不适用于此 native in-place 组件。

2026-11-01 本轮实际结果：

| 命令 | 结果 |
| --- | --- |
| `cargo test --workspace --locked` | PASS；普通组件 lib 119 项，包含本轮 3 项 Link 边界测试；Yue doctest 20 项通过 |
| `cargo test --workspace --locked --features yue-component/headless-tests,windows-showcase/headless-tests` | PASS；组件 lib 178 项，所有集成、示例与 doctest 通过；未修改既有断言来制造通过 |
| `cargo test -p yue-component --locked --features headless-tests --test link` | PASS；10 passed; 0 failed，最终代码再次复跑通过 |
| `cargo test -p windows-showcase --locked --features headless-tests pages::link::tests` | PASS；2 passed; 0 failed |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests` | PASS；系统字体模式，包含 Link 在内的组件和集成测试通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS；无警告 |
| `cargo clippy --workspace --all-targets --locked --features yue-component/headless-tests,windows-showcase/headless-tests -- -D warnings` | PASS；无警告 |
| `cargo fmt --all -- --check` / `git diff --check` | PASS |
| `node tools/sync-yue-theme.mjs --check` | PASS；Yue GPUI theme mapping is current |
| `node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check` | PASS；源 payload 与文件哈希一致 |
| `cargo check -p windows-showcase --locked` | PASS |
| `cargo build -p windows-showcase --locked --release` | PASS；`target/release/windows-showcase.exe` 已重建 |
| 源库 `corepack pnpm exec vitest run tests/token-link.test.mjs` | PASS；17 passed; 0 failed |
| 源库 `corepack pnpm verify:visual` | PASS；现有 Web 组件回归，不代表 GPUI Link 像素验收 |
| 源库 `corepack pnpm audit:tokens` | FAIL；原有跨目标 parity / Button、Input、Switch、Radio、Checkbox 命名治理尚未通过；Link 专项无命名错误 |
| 源库 `corepack pnpm verify:all` | FAIL；760 passed / 11 failed，5 个测试文件失败，见下文；typecheck/build 已执行，后续聚合链在测试失败处停止 |

源库仍失败的测试族：api-docs、docs-architecture-consistency（计数/审计摘要/已提交 inventory）、token-audit（prototype/package 差异与 package-only 登记）、token-grammar、token-usage（文档/读者与旧报告）。失败项数量与本轮前记录的 11 项相同，但新增 Link tokens 也增加了 inventory/package-only 待治理差异，不能把新增差异称为“完全无影响”。本轮补充源 `packages/tokens/README.md` 的 GPUI Link 参数及实际读者说明，没有调整旧断言、伪造已有 Vue 读者或重写全库治理快照。后续应统一刷新源库 token inventory/parity/usage 文档与登记，并修复其其他组件命名；本轮没有将源库聚合门禁描述为全绿。

实现文件、根/prelude 出口、真实示例、源 tokens 和验收记录均已落地；没有新增第三方依赖或兼容分支，没有提交。启动 `target/release/windows-showcase.exe`，在 **组件 → Link** 查看。Base FocusHandle / ControlLayout / Tabler 共用既有机制；没有第二个 Link 行为所有者。
