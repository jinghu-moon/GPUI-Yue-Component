# 08 接入与架构完善

本轮执行 log.txt 评审中 P0–P2 的八项任务。基于现有未提交工作完成改造，不引入 gpui-component 主题依赖；宿主仍拥有应用、窗口和业务状态。

## 升级前基线

2026-11-01：`cargo test --workspace` 通过（组件库 96 项单元测试、图标库 1 项及 1 项 doctest）；`cargo test -p yue-component --features headless-tests` 通过（Checkbox 11、Dropdown 23、Input 6、Radio 10）。

## 执行范围

- [x] 统一 token 读取；不支持的格式明确诊断。
- [x] Button / Switch 鼠标、键盘、受控值、禁用、loading、焦点及按压回归测试。
- [x] Base-only 最小示例和 showcase；验证依赖特性与测试组合。
- [x] 初始化、默认主题、显式覆盖和主题更新入口，移除 render 全局主题写入。
- [x] token 快照、来源哈希、类型化生成和离线校验。
- [x] 布局、焦点、键盘、无障碍扩展入口与可编译接入示例。
- [x] 菜单按行为/定位/绘制拆分，showcase 按组件页拆分。
- [x] release 图标和字体资源测量，依据结果提供资源选择。

## P0：读取、测试与真实接入

Button/Input 不再维护独立的颜色、数值、时长、曲线解析器，改用 `src/tokens.rs`。生产组件使用生成的 `theme::keys::*`；字符串入口保留给动态使用。曲线参数必须全部合法；shadow 中遇到不支持的 inset/em 等格式会拒绝整个表达式，不静默丢弃其中一层。

新增 `tests/controls.rs` 的 11 项 UI 集成测试，实际渲染 Button/Switch 并发送输入，覆盖鼠标与 Enter/Space 单次回调、受控状态、外部更新不触发回调、disabled/loading（先在可用时取得焦点再禁用后发键）、外部松开取消且按压面恢复、持续按压与中心缩放、按压期间禁用后恢复、Button/Switch 默认主题切换、调用方焦点与 action 冒泡（含 Switch 的 `track_focus`/`key_context`/`tab_stop`）、滑块中间帧和 reduced motion。

测试暴露并修复了三个布局问题：

- Button 的键盘焦点样式把边框从 1px 加到 2px，导致测试中的按钮宽度从 43px 变为 45px。
- Switch 的焦点边框同样把宽度从 40px 变为 42px。
- Switch 在高 flex 行中，语义根节点会被拉到整行高度，命中区域与 24px 高的轨道不一致。

前两项改为绝对定位的焦点环绘制层，焦点前后外框保持一致。Switch 的焦点环挂在轨道上，按 `--focus-ring-offset` + `--focus-ring-width` 向外让出，与轨道同色（Neutral 选中态）时仍可见，且不随调用方加宽的根节点拉伸；后一项在语义根节点设置 token 宽高。回归断言检查真实提交的绘制四边形和命中节点，而非只检查样式字段。按压仍以中心缩小、保持到释放，不加入按压变色。

新增 `examples/minimal`：只依赖 `yue-component`，通过其 Kit 重导出创建应用，演示受控 Switch 和 Button 回调。Showcase 改用 Base Slider 和 Base Scrollbar，移除 `gpui-kit/component`；依赖树没有 `gpui-component`、`gpui-kit-assets`。新增 showcase 的滚动与 Slider 点击测试，防止迁移只做到编译通过。

无头观测直接 opt-in `gpui-base/test-support`，不启用会带来 Fast 后端依赖的 `gpui-kit/test-support`。目前本工作区可整体运行 `--features headless-tests`，不再需要隔离组件测试。外部宿主如果启用上游 styled component 和其测试观测，仍需单独验证该依赖组合；本次没有修改上游库。

## P1：主题、生成与扩展

新增 `init`、`current_theme`、`set_theme`：初始化 Kit/字体后同步 Yue 默认主题和 Base 全局主题；主题切换从事件/update 触发窗口刷新。Showcase 的主题选择、页面切换与系统外观观察都走这个入口，render 不再写全局主题。

`YueButton::new(id, label)` 使用应用默认主题。已有显式 theme 构造器保持局部覆盖，组件可用 `.use_default_theme()` 选择跟随默认值，`.theme(theme)` 选择显式覆盖。Dropdown 的 panel 和传入的 trigger 分别拥有自己的配置，需要整体跟随时使用默认主题 Button。Base 只有一个应用级主题，局部 Yue palette 无法让不同窗口的 Base 滚动条等获得独立的全局 palette。

组件公开最小布局入口 `width/min_width/max_width/margin/flex_grow`；Button/Switch 提供 `track_focus/key_context/tab_index/tab_stop`、无障碍名称和描述，`tab_index`/`tab_stop` 同样作用于调用方传入的焦点句柄；Radio/Checkbox 保留调用方焦点入口。布局入口作用于控件语义根节点，内部 padding、轨道及交互色仍由 token 控制。测试验证控件获得外部焦点后仍可激活，父级 Ctrl+S action 仍可执行。输入框的 Tab 缩进语义保持 Base 原有行为，不声称支持表单 Tab 遍历。

生成器现在从仓库内 `tokens/inventory.json` 离线重建，外部设计系统仅在刷新/对照来源时需要。当前完整库存为 1003 个 token（含 [Segmented](09-segmented.md) 新增的 44 项）、4 个 profile 和 neutral tone overrides；`TokenKey` 直接索引，动态字符串用二分查找；`TokenValue` 预计算 Color/Pixels/Number/Milliseconds/CubicBezier，字体、shadow、currentColor 等保留 Contextual 原文，由明确的适配器处理。

快照记录 `@yue-ui/design-tokens@0.1.0`、Git revision、dirty 标志、每份源文件及整体 SHA-256。当前来源有未提交修改（dirty=true），以内容哈希识别，不能仅凭包版本判断一致：

```text
payloadSha256: 4df7ea62cb350f4fe8b2c078502a3d6e30b9b047f6536c2120c1a19e49c11956
source.contentSha256: f40008cf348d1bf5975dadb47728af9470d25aadb37a12fbc4c07a01e0b64b29
```

生成时检查名称合法性、有序唯一（按字节序，不依赖区域设置）、Rust 标识符不冲突及各 profile 值完整；离线检查验证 payload 哈希和生成产物一致。对照设计系统时只比较 payload 与源文件内容哈希，revision/dirty 变化只提示不报错。新增测试遍历全部 profile × neutral tone × token，比较静态与动态查找及类型化颜色与原始字符串解析结果。P0–P2 阶段公开组件构造示例为 14 项通过编译的 doctest；新增 Segmented 后为 15 项，最新验收见 [09](09-segmented.md)。

## P2：模块和资源

Dropdown 拆为 `dropdown/mod.rs`（公开模型和 host）、`behavior.rs`（导航/typeahead/dwell）、`placement.rs`（定位与几何）、`panel.rs`（绘制）及 `tests.rs`。行为模块不读取视觉 token。Showcase 的 Button/Input/Form/Switch/Radio/Checkbox/Dropdown 移入 `pages/`，入口保留应用状态和页面路由。

资源测试使用 `rustc 1.98.1 (48a229cea 2026-09-01)`、MSVC x86_64 默认 release profile，逐份检查 PE 中是否包含完整 SVG/TTF 字节，独立于总文件大小；不把 Cargo feature 称作树摇。结果见 [resource-measurements.json](resource-measurements.json)：

| 产物 | 字节数 | 完整 SVG | 完整字体 |
| --- | ---: | ---: | ---: |
| 空资源探针 | 123,904 | 0 | 0 |
| 只引用 Search 的探针 | 316,928 | 1（Search） | 0 |
| 引用全目录的探针 | 4,143,616 | 6,238 | 0 |
| 引用字体注册函数的探针 | 34,356,736 | 0 | 6 |
| 最小应用，系统字体 | 12,622,336 | 未统计 | 0 |
| 最小应用，内置字体 | 46,642,176 | 未统计 | 6 |

目录共有 6,238 个 SVG，原始总量 3,683,485 字节；6 份 TTF 总量 34,018,828 字节。仅引用 Search 的实际 release 产物未保留其他图标，因此暂不增加 outline/filled feature。提供更有收益的 `bundled-fonts`：库默认开启，宿主可用 `default-features = false` 改用 Windows 系统字体，最小应用减少约 32.4 MiB。Cargo feature 可加性意味着同次构建中任一依赖开启该 feature 都会启用字体；两个最小应用必须分别构建，不能用 workspace 统一构建测此差值。

复现命令（输出大小可能随工具链变化）：

```powershell
cargo build -p yue-resource-probe --release --features bundled-fonts
cargo build -p yue-minimal --release
New-Item -ItemType Directory -Force target/resource-measurements
Copy-Item target/release/yue-minimal.exe target/resource-measurements/minimal-system.exe
cargo build -p yue-minimal --release --features yue-component/bundled-fonts
Copy-Item target/release/yue-minimal.exe target/resource-measurements/minimal-bundled.exe
python tools/measure-resources.py
```

`measure-resources.py` 对四个探针的资源数量和 Search 唯一性有断言；若找到最小应用对照文件，也验证字体份数。该实验不测 SVG GPU 性能，也不承诺所有 profile/链接器均有相同裁剪结果。

## 升级后验证

以下命令构成本轮验收，Windows CI 同步运行离线生成检查、fmt、两组 workspace 测试、独立系统字体测试和两组 Clippy：

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

2026-11-01 评审收尾后：上述 GPUI 检查全部通过；此前的资源探针 release 构建与完整资源字节断言通过，`yue-minimal` debug/release 均构建成功。

| 测试入口 | 通过数量 | 构成 |
| --- | ---: | --- |
| 普通 workspace | 116 | 组件单元 100 + 图标单元 1 + 组件 doctest 14 + 图标 doctest 1 |
| workspace + headless | 181 | 上述 116 + Checkbox 11 / Button·Switch 11 / Dropdown 23 / Input 6 / Radio 10 + Showcase 4 |
| 独立系统字体组件测试 | 175 | 组件单元 100 + 组件 UI 集成 61 + 组件 doctest 14 |

两组 workspace Clippy（含 headless / 不含 headless）以 `-D warnings` 通过，fmt 与 `git diff --check` 通过。升级前先运行既有测试，再补充交互与类型化读取测试；新焦点测试在修复前暴露尺寸改变，修复后通过。字体测试分别断言原始设计 token 与当前资源模式的有效字体，避免关闭字体时继续要求内置字体族。

当前运行中的旧 `windows-showcase.exe` 曾被 Windows 锁定，未停止它；评审收尾后重新生成 `target/debug/windows-showcase-review-fixes.exe`，包含新版 Switch 焦点环、loading 语义及分组侧边栏，使用这一份审阅。该 `cargo rustc -o` 命令会产生输出路径相关的编译器提示，不是源码 Clippy 警告。此前两个示例进程的短时存活检查不等同于窗口显示或视觉验收。

## 评审遗留项与侧边栏

- 6 份 TTF 已通过明确文件路径加入 Git 索引，待最终项目提交时一并提交；未创建 commit。`.gitattributes` 增加 `*.ttf binary`。`git ls-files --stage -- crates/yue-component/assets` 可确认全部资源，避免默认字体 CI 缺文件。
- 根目录 `log.txt` 保留在磁盘，加入根锚定忽略规则 `/log.txt`，未加入暂存区。
- `form.rs` 已恢复 LF，原有文档改动保留。Dropdown 内部 crate 级类型/函数降为家族内部；panel 的辅助结构、字段和函数改为 private，只保留 host 构造所需的 `MenuPanel` 字段和 `popup` 接口。
- Switch 根节点将 `disabled || loading` 交给 Base，loading 期间无障碍节点报告不可用，鼠标和键盘不能激活，子项不参与 Tab，禁用按压伸展；颜色仍按 loading 呈现。先给旧实现增加无障碍断言，复现 disabled=None，再修复至 Some(true)，控件 11 项回归通过。GPUI/Base 没有公开 busy setter，不声称播报“加载中”。
- 浅色 `--error` 改为 red-700 `#a14138`，`--warning` 改为 amber-700 `#905a1d`，源文件在 Yue-Design-System 的 `packages/tokens/src/semantics/feedback.css`；同步本库快照与生成代码。白底对比度分别为 6.31:1 和 5.73:1，页面底色为 5.79:1 和 5.25:1，均超过正文 4.5:1。暗色值保持原契约，通过全部模式/强调色/中性色调测试。原始 prototype 作为历史参考保留，差异显式登记。
- Web 新增 16 项 status text 对比度测试，旧值 8 失败 / 8 通过，修复后 16 通过；Rust 新增全 profile/tone 的状态文字测试，旧值以 4.473954 失败，同步后通过。设计系统对比度审计也新增四组 feedback 文字配对，全部通过。
- 侧边栏实现移至 `examples/windows-showcase/src/sidebar.rs`，分为“组件”“设计”，两组默认展开，组内按英文名称升序。组件为 Button / Checkbox / Dropdown / Form / Icon / Input / Radio / Switch；设计为 Color / Dark Mode / Font / Icon / Layout / Motion。组件 Icon（`pages/icon.rs`）展示构造、尺寸、颜色、旋转及按钮组合 API；设计 Icon（`icon.rs`）展示语义表、尺寸配对、线宽与绘制规范，两个入口具有独立页面身份。折叠保留当前页面、独立保留各组状态；Base Button 处理鼠标和键盘，`aria-expanded` 发布状态，Collapsible/MotionReveal 处理高度动画，时长/曲线读取 Yue motion token，尊重 reduced motion；侧栏本身可滚动。
- 标题及目录项背景铺满侧栏可用宽度，留白放在行内；标题文字和子项文字分别保留一级、二级缩进。展开内容显式 `w_full()`，避免测量时收缩为文字宽度。先补全宽几何断言复现旧实现的 16px 外侧留白，再修复布局。
- 两项实际 Showcase 无头测试覆盖视觉顺序、整行宽度、两个 Icon 入口各自的页面内容、动画中间帧、独立收起/展开、当前页面保留、Space 操作、隐藏项退出 Tab、reduced motion 立即收起和页面切换。

外部设计系统的全量 gate 单独记录：

```text
corepack pnpm exec vitest run tests/token-feedback-contrast.test.mjs
  Test Files 1 passed; Tests 16 passed (16)
corepack pnpm verify:visual
  RESULT: PASS（使用已构建页面，输出 Button/Input 浅色/暗色/窄屏截图）
corepack pnpm audit:tokens
  四组新增 feedback 配对均 PASS；RESULT: FAIL（既有词汇登记等问题）
corepack pnpm verify:all
  Test Files 5 failed | 30 passed (35)
  Tests 11 failed | 708 passed (719); exit 1
```

为确认没有引入新的全量失败，临时只还原本轮两色修改、四组配对/两项差异登记和相应 prototype 对比度测试变更，排除本轮新增的测试文件后，运行 `corepack pnpm exec vitest run --exclude tests/token-feedback-contrast.test.mjs`：`11 failed | 692 passed (703)`。11 个失败名称与改动后相同：API 文档 1、架构文档/库存 3、prototype parity 4、grammar 1、usage 2；之后按字节恢复本轮修改。四项既有 parity 断言未改动，prototype 测试扩展为精确断言历史配色的四个实际对比度失败。没有删除、跳过或放宽这些既有失败测试，也不将外部仓库描述为全量通过。

## 验收边界

### Tooltip 接入补充（2026-11-01）

`yue_component::init` 在开窗前注册 PopupRuntime 和每窗 Tooltip controller；与原有字体、主题初始化共用同一个 Base Root。已存在的 Root 不会补装插件。绕过 Kit 开窗辅助函数的宿主须显式使用 Base Root；没有 Root 时保留原控件，开发构建诊断一次并省略 Tooltip。

Button `.tooltip(text)` 和 Segmented 的图标选项已经迁入统一 Tooltip，勿在同一 trigger 上再安装原生 `.tooltip()`。默认 hover 等待 500ms、离开宽限 300ms，实际请求读取 Base TooltipDefaults；焦点立即显示且没有新增 Tab stop。Button 未提供显式 description 时使用提示文本补充描述，仍保留原 accessible name。通用 wrapper 使用共享 handle；任意 child 的 description 由宿主提供。

Popup 仍是 `pub(crate)` 基础层，不公开可配置成菜单/Tooltip 并集的 `YuePopup`。Tooltip 的特殊尺寸退化、显式 Group 轨道、24/160px rem 距离边界及 hover 时序都在上层实现；尺寸或 variant 改变时安全淡出/淡入，不把位置插值冒称尺寸 morph。实现、最新自动检查与实窗边界分别见 [Tooltip 第 12 节](10-tooltip.md#12-2026-11-01-实施记录) 和 [Popup 第 15 节](12-popup.md#15-tooltip-作为第二个消费者接入)。

无头测试验证逻辑、输入、焦点、布局和提交给 GPUI 的绘制数据。不能据此声称通过了 Windows 读屏播报、DPI 实际像素、字体栅格化或文字抖动的真实窗口验收。最终验证和资源测量记录在本文件。

本轮尝试通过 Computer Use 原生窗口 API 验收，`sky.list_windows()` 返回 `Computer Use native pipe is unavailable: failed to connect native pipe: 系统找不到指定的文件。 (os error 2)`；重试及重置会话后相同，无法取得可操作窗口。以下真实 Windows 检查仍待人工完成：打开最新 review-fixes 预览，使用 Tab 检查 Neutral 开启态的外侧焦点环；在 100%/150%/200% 显示缩放下检查字体、裁切和滚动；用读屏检查名称/开关值/loading 不可用播报；检查 Input 的中文 IME 组合。未修改系统显示或辅助功能设置。
