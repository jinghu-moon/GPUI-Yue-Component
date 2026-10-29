# 01 GPUI 组件库探索与方案

- 状态：探索提案
- 日期：2026-10-26
- 项目：GPUI Yue Component

## 目标

基于 Rust 与 GPUI 构建一套可供多个 GPUI-KIT 应用复用的组件 crate，复用 GPUI-BASE 的交互和基础设施，并通过 gpui-kit 对齐 GPUI 版本与依赖。目标平台限定为 Windows。组件库消费现有 Yue Design System 的设计变量与规范，将其映射到 GPUI 主题和组件 API；不另起一套互不关联的视觉 Token。

当前工作区只有 `.spec-workflow` 模板和本方案文档，没有 Cargo workspace、源代码、README 或 Git 元数据。因此本文是面向新项目的架构提案；crate 名称与首批组件需在初始化及 API 验证阶段落定。gpui-kit 版本由项目约束为 `0.8.0`。

## 探索结论

### 上游分层

根据 GPUI Kit 当前项目文档，其主要层次为：

```text
GPUI runtime
    ↑
gpui-kit               统一 GPUI 版本并重导出基础层、组件层和资源
    ├── gpui-base       无样式交互行为、状态与基础设施
    └── gpui-component  完整的带样式通用组件系统
    ↑
GPUI Yue Component     Yue 自有主题、样式和组件 API
```

- `gpui-base` 的职责是可复用行为与实现这些行为所需的几何/状态，包括焦点、键盘交互、受控状态、无障碍语义和虚拟列表等。它不负责品牌颜色、间距、尺寸、圆角或应用视觉风格。
- `gpui-component` 已经是一套完整的通用带样式组件体系。若直接采用它，应把它视为组件方案本身，而不只是无样式基础层。
- `gpui-kit` 是版本一致性与依赖入口。官方 Base 安装说明建议关闭默认特性以跳过带样式层：

```toml
[dependencies]
gpui-kit = { version = "0.8.0", default-features = false }
```

版本下限由项目指定为 `0.8.0`，Cargo 的兼容更新允许同一兼容版本范围内的修订版升级；交付 workspace 的 `Cargo.lock` 固定实际解析结果。避免另外直接引入不匹配的 `gpui`/GPUI snapshot，否则会出现不同 GPUI 类型无法互通的问题。

### 设计与编码约束

- 组件库拥有呈现，Base 拥有可复用行为；主题和样式不向 Base 下沉。
- 视觉规范以 `D:\100_Projects\110_Daily\Yue-Design-System` 为准：规范文档见 `apps/docs/design/`，Token 契约见 `packages/tokens/src/`。Yue Design System 的 CSS Token 是设计值来源，GPUI Theme/Rust 类型是消费适配层。
- 保持 Token 单向分层：primitives → semantics → component tokens。组件优先读取语义角色，组件实现只消费自己的 Component Token，不跨组件读取其他组件 Token；不在组件实现里散落裸色值、间距和时长。
- Yue 现有规范要求正文对比度至少 `4.5:1`，非文本边界和焦点至少 `3:1`；颜色不能成为状态的唯一表达。浅色、深色、Azure Accent 与 Neutral Accent 配置纳入验证。
- 深色主题保留页面、内容表面、辅助表面、控件表面的层级；使用近黑而非纯黑大面积底色。字体、排版与动效分别遵循 `typography.md` 和 `motion.md`，支持系统字体回退、`prefers-reduced-motion` 与 `forced-colors`。
- Yue 设计系统的规则以现有文档与 Token 包为准；GPUI Kit 设计指南用于补足 GPUI 原生窗口、键盘、焦点及弹层生命周期，不覆盖 Yue 已定的品牌变量。
- 先使用 Base 暴露的语义组件、状态与行为能力，不用通用 `div` 重写已有的菜单、弹层、焦点或选择逻辑。
- 无状态组件优先使用值语义；跨帧状态、焦点或异步行为才采用 GPUI Entity。具体选择以当前上游源码/API 为准。
- 公开数据结构用构建器与只读方法表达，不暴露 `pub` 字段；可重复元素使用稳定 ID。
- 主题使用语义 token，并覆盖亮/暗主题、焦点、禁用、选中、危险、加载等状态。
- 组件文档与 Story/Gallery 示例应能作为可运行的交互验收入口，不把静态截图当作行为验证。

## 需求完整性评估

总分：9/10。目标、交付物、平台（Windows）、消费方式（多个 GPUI-KIT 应用）及技术方向均已明确。首批组件和发布渠道仍待实施前确定，不阻碍 workspace 初始化与依赖验证。

## 方案比较

### 方案 A：在 gpui-base 上构建 Yue 自有组件系统（推荐）

通过 `gpui-kit` 统一依赖与 GPUI 类型，关闭默认特性，并在 `gpui_kit::base` 之上实现 Yue 自有主题、样式、组件 API 与展示 Gallery。

优点：视觉和 API 所有权清晰；直接复用 Base 的焦点、状态、键盘和基础行为；不绑定 gpui-component 的完整视觉体系。风险：需要维护 Yue Token 到 GPUI Theme 的映射，并补齐组件文档和桌面端交互测试。

### 方案 B：以 gpui-component 为主，封装 Yue 品牌层

默认采用 `gpui-kit` 带入的 `gpui-component`，通过主题、组合组件与少量包装匹配 Yue 的产品风格。

优点：交付速度快，通用控件和复杂行为覆盖较广。风险：组件 API、设计取舍及版本节奏更多受上游约束；若需要大幅改变视觉或公开 API，包装层可能逐渐膨胀。

### 方案 C：Fork gpui-kit / gpui-component 并长期维护分支

在上游实现基础上直接修改和发布 Yue 分支。

优点：能改动底层组件且可最大化复用现有代码。风险：需要持续同步上游、解决冲突并承担更大兼容面；在产品差异与组件清单尚未验证前，维护成本过高。

### 建议

以方案 A 为主。若后续某类组件经验证能显著节省成本，可通过明确的适配边界少量借鉴或依赖方案 B 的实现；在用户体验和 API 稳定前，不建议 fork。任何直接复用带样式组件的决定，都应先验证它与 Yue 视觉及公开 API 的一致性。

## 建议的项目结构

第一阶段优先保持一个可运行的 Cargo workspace，不为每个控件提前拆 crate；当出现独立版本、独立依赖或多个实际消费者时再拆分。

```text
.
├── Cargo.toml                 # Workspace 与统一依赖版本
├── crates/
│   └── yue-component/         # 初始公开组件库 crate
│       └── src/
│           ├── lib.rs         # 稳定公开 API 与模块导出
│           ├── theme/         # 语义 token、主题与状态样式
│           ├── primitives/    # Yue 样式原语和组合基础
│           ├── components/    # 按组件族组织实现
│           └── testing/       # 公用测试辅助（确有复用后再抽取）
├── examples/
│   └── windows-showcase/      # Windows 上验证消费方集成的可运行示例
├── story/                     # 组件目录、状态和交互示例
├── docs/                      # 架构决策、组件规范和迁移记录
├── assets/                    # 组件库自有静态资源
└── tools/                     # Yue Token 主题映射生成/校验
```

组件库作为被多个 GPUI-KIT 应用依赖的 library crate，不接管窗口、应用初始化或事件循环。所有消费方通过同一 `gpui-kit` 版本使用 GPUI 类型，组件公开 API 直接接受/返回该版本的 GPUI 类型，避免类型边界重复。示例应用只用于 Windows 上验证，不构成组件库的运行时依赖。

## 实施路线

### 1. 初始化与上游验证

- 建立 Cargo workspace、库 crate 和可运行的最小 GPUI 窗口示例。
- 使用已指定的 `gpui-kit = 0.8.0`，核实该版本 feature flags、最低 Rust 版本与 Windows 构建前置条件。
- 用 `default-features = false` 验证 `gpui_kit::base` 及相关类型可正常编译、测试。
- 记录上游来源、版本策略及不可直接依赖第二份 GPUI runtime 的约束。

预期结果：干净环境可编译的最小 workspace，以及经编译验证的依赖边界。

### 2. 产品与设计基础

- 盘点 Yue Design System 的 primitives、semantics、component tokens 和组件规范，明确首批 GPUI 控件对应的现有 Token namespace。
- 定义 CSS Token 到 GPUI Theme/组件样式的映射方式。由于 GPUI 渲染不直接消费 CSS custom properties，评估机器可读 Token 导出、生成 Rust 常量或手工映射加 parity test；选定单一设计事实来源，禁止无校验的双份手填值。
- 映射 Yue 既有主题 profile（light/dark、Azure/Neutral Accent）及颜色、字体、间距/尺寸、形状、边界、交互、层级、动效角色；检查平台字体回退和 GPUI `rem`/尺寸尺度如何表达既有值。
- 定义组件状态规范、尺寸档位、命名、构建器约定、事件/回调语义及稳定性策略，并分别标注哪些来自 Yue 规范、哪些是桌面端补充决策。
- 目标平台、默认语言与主题切换入口在适配层确定；视觉基线不再从零选择，直接遵循 Yue 设计规范。
- 先实现主题切换和状态展示的 Story 页面，用真实窗口验证视觉尺度。

预期结果：可追溯到 Yue Token 的 GPUI 主题适配契约，以及覆盖主题和交互状态的可运行 Story。Rust 侧生成完整 Token catalog，覆盖 primitives、semantics、component tokens 的变量名和所有支持 profile 的解析值；组件可通过语义/组件 Token 访问器消费，Token 审计或 parity test 能发现适配值漂移。

### 3. 首批核心组件

推荐按依赖关系推进：Button、Label/Badge、Checkbox/Switch、TextInput、Select/Popover、Dialog/Sheet、Tabs、Tooltip。每个控件先做 API 与 Base 能力映射，再实现主题呈现、键盘/焦点状态、文档和 Story；不适用的组件可调整顺序。

预期结果：形成一组能覆盖常见桌面表单与窗口交互的组件，而非只完成静态外观。

### 4. 质量与发布准备

- 对受控状态、事件触发、焦点移动、弹层关闭/焦点恢复和禁用状态做 GPUI UI 集成测试。
- 在 Gallery 中覆盖默认、悬停、聚焦、按下、禁用、错误、加载及长文案等状态。
- 在 Windows 验证编译、示例运行与关键交互；macOS/Linux 不在本项目支持范围内。
- 配置格式化、Clippy、测试、文档示例编译和 CI；定义 semver、变更日志和发布流程。

预期结果：每个公开组件均有 API 文档、可查看示例及与行为承诺匹配的测试。

## 首轮验收标准

- 示例应用只依赖组件库公开 API 即可启动，且没有重复引入不兼容 GPUI 类型。
- 主题 token 与组件视觉分离于 Base 行为，切换主题时控件状态仍正确。
- 首批组件的键盘交互、焦点可见性、禁用行为和弹层生命周期有可重复的集成测试。
- 组件 API 与展示状态在 Gallery 中可检查，公开 API 有 Rust 文档。
- CI 至少覆盖格式检查、静态检查、测试与示例构建；具体系统矩阵在平台决策后确定。

## 主要风险与应对

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| 上游仍在演进，Base API 或 gpui-kit feature 有变化 | 构建/API 兼容成本 | 锁定兼容版本，升级时集中验证 workspace 与交互测试 |
| 同时引入 gpui-kit 与独立 GPUI 依赖 | GPUI 类型不兼容、编译失败 | 全 workspace 统一经 gpui-kit 导入 runtime 与 Base |
| Base 的“无样式”边界被误认为没有可复用行为 | 重复实现焦点、键盘与弹层逻辑 | 实现前做 Base 能力映射，优先复用受支持的行为 API |
| 过早扩展组件数量或拆分 crate | 维护与 API 设计成本增加 | 以首批任务流验证组件，再按真实所有权和消费者拆分 |
| Yue CSS Token 与 GPUI Theme 表达不同 | 视觉值重复维护并逐渐漂移 | 将 CSS Token 作为设计事实来源，明确生成/映射方案，并用主题快照或 parity test 守住一致性 |
| 桌面平台与主题切换方式未定 | 系统色彩、字体与窗口行为难以稳定 | 在主题适配阶段验证 Windows/macOS/Linux 差异，遵循 Yue 规范并记录必要的平台例外 |

## 待决策问题

1. 首批组件是否以上述表单、弹层、导航控件为主，还是有明确的业务场景优先级？
2. Token 同步已使用生成 Rust 映射并提供漂移检查；后续是否将生成器迁入 Yue Design System 的正式发布流程？

## 参考资料

- GPUI Kit 仓库与分层：<https://github.com/longbridge/gpui-kit>
- GPUI Base README：<https://github.com/longbridge/gpui-kit/blob/main/crates/base/README.md>
- GPUI Kit 文档：<https://gpui-kit.com/>
- GPUI Kit Coding Guides：<https://gpui-kit.com/docs/coding-guides.md>
- GPUI Kit Design Guides：<https://gpui-kit.com/docs/design-guides.md>
- Yue Design System 设计规范：`D:\100_Projects\110_Daily\Yue-Design-System\apps\docs\design\`（色彩、深色模式、字体、图标、布局、动效）。
- Yue Design System Token source：`D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens\src\`（primitives、semantics、component-tokens）。Token 架构与门禁说明另见 `docs/05-yue-token-refactor-plan.md`、`tools/audit-tokens.mjs`。
- 本地编码/设计约束：`gpui-kit` 与 `gpui-kit-design-guides` skills（2026-10-26 读取）。

