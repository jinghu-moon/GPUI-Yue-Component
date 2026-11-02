# 24 组件架构与目录规范

状态：已采纳的目标架构，存量组件渐进迁移。日期：2026-11-02。

本文约束 `crates/yue-component` 的组件设计、内部组织、公共接口与改造验收，同时明确与图标 crate、设计系统、Base 及宿主应用的边界。本次新增规范，不代表下面列出的目录迁移已经完成。

“必须”表示不可缺少的合同，“建议”表示通常应采用的做法，“按需”表示存在真实职责后才增加。用户明确要求优先；具体可用 API 以本仓库锁定的源码和测试为准。历史设计文档与本文冲突时，结构约定以本文为准，组件特有的行为仍以对应设计文档和经过验证的实现为准；发现冲突时须同步修正文档。

## 1. 核心决策

1. **按组件或紧密组件家族组织目录，统一以 `组件名/mod.rs` 为入口。** 不新增一层 `components/`，不为每个组件新建 crate。
2. **按职责拆分，不按文件数、类型数或行数套模板。** 小组件可以只有 `mod.rs`；不要求每个组件都有 `model.rs`、`state.rs`、`element.rs`、`paint.rs`。
3. **一个事实只有一个状态所有者。** 宿主、Yue controller、Base state 各自持有其负责的事实；Element 接收配置并组合，不复制业务值。
4. **复用 Base 的行为和 GPUI 的布局、输入与绘制。** Yue 负责设计语言与组合；通用能力缺口通过受维护的 Base 扩展解决。
5. **公共类型从 crate 根导出，内部文件不是公共接口。** 消费者使用 `yue_component::YueSelect`，不依赖 `select::panel` 等实现路径。
6. **设计变量只有一个上游来源。** 组件使用同步后的 token、排版和动效适配，不新建另一份主题或重复抄写数值。
7. **结构迁移与行为变更分批执行。** 开发期允许破坏性改动，但须标出变化、补充改造前后的测试并更新调用方。

## 2. 为什么调整现有结构

当前源码同时存在单文件、目录模块以及 `name.rs + name/` 三种形式；后者是合法 Rust 结构，问题在于项目约定不统一，并非编译错误。

| 当前情况 | 实际问题 | 目标 |
| --- | --- | --- |
| `input.rs` 同时包含 Input、Textarea、NumberInput、Field | 编辑器外观、数值控件、表单字段组合混在一起；Field 还已支持 Select | Input 家族保留关联；Field 成为独立模块 |
| `button.rs`、`radio.rs`、`switch.rs` 等包含 API、组合和测试 | 查找某项职责需要跨越大量无关代码 | 先统一目录，再按真实职责拆分 |
| `tag.rs + tag/`、`divider.rs + divider/` | 入口规则与已有目录组件不同 | 改为 `tag/mod.rs`、`divider/mod.rs` |
| Select 已拆模块，但 `panel.rs` 仍混合字段主体、浮层与选项行 | 文件名不能准确表达内容，修改边界不清晰 | 分开 element、panel、option |
| `dialog/mod.rs` 仍包含较多渲染实现 | 公共合同与视图构造交织 | 将独立的视图职责移入 element/layer |

文件较长是检查职责的信号，不是独立的重构理由。评价标准是：一项需求是否能够定位到明确的所有者，修改后是否能用对应层级的测试证明。

## 3. 项目分层与依赖方向

当前 workspace 使用 `gpui-kit 0.8.0`，关闭其默认 features，通过 `gpui_kit::base` 复用基础行为。`vendor/gpui-base` 是仓库维护的 0.8.0 补丁源码，由 workspace 根的 `[patch.crates-io]` 接入；精确解析结果由 `Cargo.lock` 固定。

```text
宿主应用 / examples
  └─ yue-component：公共组件、产品交互合同、Yue 外观
       ├─ 组件内组合：Input、Button、Tag 等公共能力
       ├─ 内部基础设施：popup、motion、control_layout
       ├─ 设计适配：theme、tokens、typography、layout
       ├─ yue-component-icons：SVG 资源与图标渲染
       └─ gpui_kit::base + GPUI：行为、焦点、编辑器、布局与绘制

Yue-Design-System/packages/tokens
  └─ 同步工具 → token 快照 + Rust 生成代码 → 设计适配 → 组件
```

| 层 | 负责 | 不拥有 |
| --- | --- | --- |
| 宿主应用 | 窗口和生命周期、业务值、导航、网络/文件等业务异步任务 | 组件内部 hover、过渡采样、Popup 几何 |
| Yue 语义组件 | 公共配置、组件交互合同、视觉、插槽、对 Base 的组合 | 第二套文本编辑器、业务服务和页面状态 |
| Yue 内部基础设施 | 多组件共享且已有实际需求的布局、浮层和动效机制 | 菜单命令、选择值、业务校验规则 |
| Base | 无样式行为、可访问性零件、焦点、编辑、通用定位等 | Yue 的主题选择与业务文案 |
| GPUI | Element 生命周期、事件分发、布局、窗口和绘制 | 产品组件的业务语义 |
| 图标 crate | 图标定义、SVG 资源、同步与渲染入口 | Button/Select 的尺寸和交互规则 |

组件可以组合其他组件。例如 Select 使用 YueInput、YueTag 与 Popup；ColorPicker 可以使用 Input、Button 和 Slider。禁止基础设施反向导入具体语义组件，禁止通过兄弟模块的私有字段形成隐式耦合。

项目当前只面向 Windows，不预建跨平台抽象。接入其他 GPUI Kit 框架项目时，必须统一 GPUI/Base 的解析版本与后端 feature，复用宿主的窗口和 Base Root，避免出现同名但来源不同的类型或重复的窗口级所有者。`application.rs` 是初始化组合入口，可以调用组件注册函数；这不意味着 popup、motion 等底层机制可以反向依赖组件。

不要为了外观方便启用 Kit 的另一套 component 层，也不要另外建立 Root、Popup host 或输入编辑器。应用初始化统一进入 `yue_component::init`，它负责启用当前所需层和注册；宿主继续拥有自己的窗口和视图。默认主题切换使用 `set_theme`，不从 Render 修改全局主题。

默认 `bundled-fonts` 与可选的系统字体路径都属于接入合同；新增依赖时注意 Cargo feature 的合并效果，不能在内部依赖中无意替宿主重新开启资源包。`headless-tests` 只服务测试，不应成为生产构建前置条件。

Base 补丁须保留来源、许可证、回归测试及移除条件，见 [YUE-PATCH.md](../vendor/gpui-base/YUE-PATCH.md)。外部 workspace 接入时还须在自己的根配置 patch，Cargo 不继承依赖库中的 patch；不能仅搬动 Yue 目录就宣称消除了这一接入要求。

## 4. crate 级目录

以下是目标结构的局部示例。`field/`、`input/` 等属于待迁移目标，不是当前路径清单。

```text
crates/yue-component/
  src/
    lib.rs                 # 模块声明、公共导出、prelude
    application.rs         # 初始化、默认主题和应用级注册入口
    button/                # Button / ButtonGroup
    input/                 # Input / Textarea / NumberInput
    field/                 # label、描述、错误和控件组合
    form/                  # 校验、提交和首个无效项聚焦
    select/                # Select 的公共合同与实现
    loading/               # Progress / Spinner / Skeleton 家族
    ...                    # 其他独立组件或紧密家族
    popup/                 # 内部基础浮层机制
    control_layout.rs      # 控件外部布局适配
    layout.rs              # 宿主布局数值与断点适配
    motion.rs              # 共享动效 token 适配与策略
    tokens.rs              # token 解析和类型转换
    typography.rs          # 排版角色与字体注册
    theme/
      mod.rs               # 主题公开合同和查询
      generated.rs         # 生成代码，禁止手改
    icon.rs                # 到独立图标 crate 的桥接与 Yue 适配
  tests/                   # 公共接口和跨组件集成测试
  tokens/inventory.json    # 设计系统 token 快照，禁止手改
  assets/                  # 实际需要分发的资源
```

组件目录采用 `snake_case`。一个目录可容纳紧密家族，例如 ButtonGroup 属于 Button、RadioGroup 属于 Radio；Input/Textarea/NumberInput 共享编辑器与外框，Loading 家族共享进度语义和绘制。不能因为几个组件都用圆角背景就归为一个家族。

`icon.rs` 是桥接入口，不在 Yue 中重新复制 SVG 数据或实现另一套图标加载。`popup/` 等基础设施按机制命名，与组件目录并列；不新增含糊的 `common/`、`utils/` 或按技术类型横切所有组件的 `states/`、`elements/`。

## 5. 单个组件需要哪些文件

### 5.1 职责与拆分条件

| 文件 | 职责 | 何时需要 |
| --- | --- | --- |
| `mod.rs` | 模块说明、公共类型入口、导出、简短构造与 builder | 每个组件目录必需；简单组件可只用这一个文件 |
| `model.rs` | 值、选项、变体、事件、错误及纯数据规则 | 公共合同或规则已能独立阅读和测试时 |
| `element.rs` | RenderOnce/Render 组合、布局、样式和事件接线 | 视图组合已妨碍公共合同阅读时 |
| `state.rs` | retained controller、跨帧状态、订阅、任务和会话 | 确实需要长期所有者，且 Base 不已持有同一事实时 |
| `behavior.rs` | Actions、键盘、焦点流、确认/关闭等交互规则 | 规则复杂到需要独立检查与测试时 |
| `paint.rs` | Canvas、路径、自定义 Element 绘制与纯几何辅助 | 普通元素组合无法表达，确需自定义绘制时 |
| `panel.rs` | 浮层面板、搜索区、列表视口、头尾插槽组合 | 面板是实际独立部件时 |
| `option.rs` / `card.rs` / `layer.rs` 等 | 选项行、卡片或窗口层等具名部件 | 存在可说明的独立职责时，使用具体名字 |
| `tests.rs` 或 `tests/` | 模块内规则与实现合同测试 | 随测试复杂度选择一种入口，不重复注册同一测试 |

补充规则：

- `mod.rs` 可以保留简单枚举、builder 和少量清晰实现，不强制变成空壳转发文件。
- `model.rs` 不能成为杂物箱；纯规则不依赖窗口，不保存 Entity、Subscription 或 Task。事件载荷确需资源句柄时，应明确合同，不能把 controller 本身当作数据模型。
- 只涉及单个渲染闭包的事件接线留在 element；不能把所有 `.on_click` 都机械搬到 behavior。
- `paint.rs` 不发请求、不决定业务值、不持有另一份可变状态。交互式 Canvas 的事件可转发给明确的 controller。
- 不为每个 enum、trait 或 struct 单独建文件，也不为未来可能用到的能力建空文件。
- 一个职责可以分布在多个 `impl` 块中；不得仅因移动 `impl` 就放大字段可见性。

### 5.2 简单组件示例

```text
button/
  mod.rs       # YueButton、ButtonGroup、简单选项和 builder
  element.rs   # 表面、图标/文本、loading、交互样式组合
  tests.rs     # 内部纯规则；公共事件测试仍可在 crate tests 中
```

这是 Button 的建议目标，不要求另建 ButtonState、ButtonPainter 或一套 ButtonTheme。若某个简单组件整体仍易于阅读，全部实现可放在 `mod.rs`，测试再按需拆出。

### 5.3 复杂组件示例

```text
select/
  mod.rs       # 公共 builder、导出与必要的配置入口
  model.rs     # 单选/多选值、选项、事件、请求票据合同
  state.rs     # 唯一选择状态、搜索会话、请求代次、生命周期
  behavior.rs  # 键盘导航、确认、关闭、焦点规则
  element.rs   # 字段主体、已选值、清除按钮、箭头
  panel.rs     # Popup、搜索、头尾插槽、列表视口
  option.rs    # 普通行、分组、自定义内容槽
  tests/
    mod.rs
    model.rs
    keyboard.rs
    pointer.rs
    requests.rs
    layout.rs
```

此例根据现有 Select 的职责给出，不是新组件模板。拆分后 `YueSelectState` 仍可作为 retained 渲染所有者，调用 element 中的组合函数；不能为了文件名把状态迁到新 Entity，或让通知失效。

## 6. Input、Field 与 Form 的专门边界

`YueField` 目前位于 `input.rs`，已能包裹 Input、Textarea、NumberInput 和 Select，因此它是字段组合层，不属于文本编辑器内部。

```text
Field ──> Form ──> InputState / YueSelectState 的公开校验与聚焦合同
  └─────> Input / Textarea / NumberInput / Select 的公开组件合同
Select ──> Input / Tag / Popup
Input ──> Base 编辑器与 SpinButton 行为
```

| 模块 | 拥有 | 不负责 |
| --- | --- | --- |
| `input/` | 文本控件外观、前后缀、清除/密码操作、数值步进的组合 | 字段标签、表单提交、复制 Base 的文本/选择区/IME 状态 |
| `field/` | label、description、required、错误文案及状态向子控件传递 | 另一份控件值、业务校验算法 |
| `form/` | 校验规则、何时显错、提交、聚焦首个无效项 | 渲染字段外框、读取 Select 搜索词冒充选择值 |

Input 家族可按需拆 `element.rs`、`textarea.rs`、`number_input.rs`、`frame.rs`、`model.rs`；共同外框只处理真实共享的几何与视觉。禁止复制三份几乎相同的外框，也禁止预先创建万能输入引擎。

迁出 Field 时先通过现有公开或窄范围内部接口传递名称、描述和状态；不要求立即新增 `FieldControl` trait。只有重复适配已经出现、且多种控件可服从同一合同，才提取最小接口。

目标依赖中 Input/Select 不反向依赖 Field/Form。Form 可以使用它们的公开状态接口；Field 可以依赖 Form。若一个状态枚举真正成为多组件公共语义，再单独评估归属，不借本次目录迁移批量改名或引入“通用状态”层。

## 7. 公共接口与可见性

- 组件模块默认私有，`lib.rs` 显式 `pub use` 稳定入口；内部再由 `mod.rs` 导出。已有 `theme`、`layout`、`motion`、`typography`、`icon` 等公共基础模块按既有合同保留，是否收窄另行评审。
- 公共名称沿用 `YueButton`、`YueSelectState`、`SelectOption` 等清晰前缀；模块内已有上下文时使用 `Row`、`Presentation` 等简名，不增加无意义的重复前缀。
- 字段默认私有。Rust 子模块可访问祖先模块的私有项，兄弟模块使用必要的 `pub(super)` 或窄访问器；仅真实跨组件共享才考虑 `pub(crate)`。
- 不因拆文件把所有字段改成 `pub(crate)`；不让消费者获得可任意修改的不变量字段。
- 外部配置通常使用 builder；长期状态变更使用明确的状态方法；一次性用户行为通过回调或类型化事件传递。不要为每个组件同时复制两套等价通知渠道。
- 仅将高频入口加入 prelude；罕见配置从 crate 根显式导入，避免 prelude 随文件搬迁无边界增长。
- 公共接口说明默认值、尺寸/单位、非法输入、事件是否触发、回调重入限制和生命周期；可恢复的调用错误返回类型化错误，编程前置条件导致 panic 时须写明。
- 目录调整原则上保留根导入路径。若主动更改构造方式、事件语义、类型或默认行为，列出升级前后示例并更新所有仓库消费者，不留下未经说明的兼容壳。

## 8. 状态与生命周期

### 8.1 选择最小所有者

| 状态种类 | 通常的所有者 | 规则 |
| --- | --- | --- |
| 业务选中值、业务加载结果 | 宿主，或公开约定的组件 controller | 明确唯一真值，不能两边各保留一份并双向监听 |
| 编辑文本、光标、选区、组合输入 | Base InputState/TextareaState | Yue 持句柄并适配，不另存一份“同步文本” |
| 搜索会话、异步票据、列表活动项 | 需要它们的 retained controller | 与业务值区分，派生缓存有失效规则 |
| hover、按压、当前过渡采样 | Base/GPUI 元素状态或窄内部状态 | 不为每片视觉创建 Entity |
| 布局结果、anchor、测量缓存 | 所属 Element 或基础设施 | 标明有效阶段、坐标空间与失效条件 |
| 变体、尺寸、renderer 配置 | 本帧 builder / presentation | 不能悄悄变成新的业务状态 |

能用 `RenderOnce` 表达的值式组件优先用它。只有跨帧交互、订阅、任务或通知所有权需要保留时才使用 `Entity<T>`；不要求所有组件公开 State，也不要求所有组件受控。

### 8.2 受控与 retained 合同

- **受控组件：** 宿主传入当前值，组件发出变更请求，宿主决定是否接受并写回。宿主拒绝时，组件不能仍停留在内部已选状态。
- **retained controller：** 宿主创建并持有 Entity，组件通过它读取和变更值；通知须能驱动必要的重绘，不能要求消费者猜测还需订阅什么。
- 同一事实不要同时提供未定义优先级的 `value + default_value + state.value`。确需两种模式时必须明确模式、切换和事件合同。
- 明确程序化 setter 是否静默、用户事件何时发出、同值是否发出、值变更与关闭的顺序。不能在结构重构中顺手改变既有语义。

### 8.3 身份、任务与 render

- ElementId、选项 key 和动画身份跨帧稳定；重排后仍指向同一个逻辑项。可重排列表不能用当帧 index 或随机值代替逻辑身份。
- Subscription、Task 与注册令牌由明确的 retained 所有者持有；卸载、关闭、禁用、窗口销毁时按合同取消或失效，避免每帧重复注册。
- 网络、文件和业务保存任务由宿主执行。组件可以维护去抖、展示期限和动画任务，但不能把业务客户端塞入通用 State。
- 异步写回验证 owner、会话/代次和必要的值 revision；晚到结果不能覆盖新查询、已关闭会话或其他实例。票据维度按真实并发风险选择，不为简单按钮增加请求系统。
- Render 用于派生视图，不发业务请求、不无条件 notify、不修改业务真值。合法的本帧 presentation/config 同步与测量缓存更新可以保留，但须幂等、限定范围，不制造观察反馈循环。
- `Rc<RefCell<_>>` 等内部共享容器只在必要的阶段桥接中使用；不以共享可变对象绕开 GPUI 的状态所有权和重绘通知。

## 9. 设计变量、布局与绘制

### 9.1 唯一设计源

设计变量上游为 `D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens`。本库使用 `tokens/inventory.json` 快照和 `theme/generated.rs` 生成代码，经 `tokens.rs`、YueTheme、typography、motion 等入口消费。

颜色、间距、控件尺寸、圆角、边框、阴影、排版和动画都应追溯到设计源；不能只同步颜色。新增组件 token 先在设计系统定义，再同步并补解析/映射测试。禁止手改生成代码或在组件旁复制一份 `theme.rs`。

并非所有数字都是 token：算法常量、归一化范围和由测量得到的几何不应伪装成设计变量。视觉选择若已有 token 就复用；缺少且确需独立调节时才新增语义 token，并记录用途。

### 9.2 每个布局责任只有一个所有者

- 页面骨架和可用宽度由宿主决定；控件自身处理内容、尺寸合同与最小可用几何，不感知它在哪一个 Showcase 卡片内。
- 默认宽度、显式 width、填充父级及 min/max 的优先级必须写清。不能为修复一个示例把所有输入框改为父级全宽。
- 外部宽度、margin、grow 由现有 ControlLayout 等适配；内部 padding、文字行高、图标占位和最小点击区由组件与 token 决定。
- 修复垂直对齐应找到内容槽所有者。例如 Select 自定义选项由全高内容槽完成垂直居中，不在每个 Tag 上追加不同的位移补偿。
- 阴影、focus ring、圆角、clip 和滚动边界要区分内外包络。例如 Dialog 滚动区域需要的外圈留白由对应遮罩/容器负责，不让每个 Select 自行增加补丁 margin。
- 自定义绘制须使用同一套测量结果组织视觉、hitbox 与交互；明确逻辑像素/物理像素以及窗口/局部坐标。不能让看见的滑块和可拖动区域使用不同数据。
- 长列表按真实规模选择虚拟化，避免在每帧为全部数据构建重元素；缓存须有内容与布局失效条件。

## 10. 动效、交互与可访问性

- 颜色、透明度、位移、缩放等过渡优先复用 Base 的采样，Yue motion 提供共享 token 和策略；不因一个效果不可感知就重写动画引擎。
- 动效时长和曲线选择由语义组件/设计 token 决定，Popup 基础层接收参数，不选择组件主题。菜单与选择面板等需要一致的开合应复用同一策略。
- 目标中途改变时从当前采样值续接；区分逻辑关闭、退出动画和最终卸载。退场保留的画面不能继续接收激活。
- 按压缩放优先影响绘制而非兄弟布局，明确中心与 hitbox；释放在外、取消捕获、禁用及 loading 都须有确定的恢复路径。
- 使用 GPUI Actions 和正确 key_context；调用方提供 FocusHandle 时，tab_index/tab_stop 等合同仍须生效，避免重复注册 track_focus。
- role、名称、值、禁用、只读、busy、校验说明由真正可交互节点表达。图标装饰不重复播报；icon-only 操作必须有可访问名称。
- 支持 reduced motion、键盘和焦点可见性；不能仅靠颜色表示错误。loading 与 disabled 是否阻止交互、如何暴露语义，按组件合同分别说明。
- 不以常见网页习惯替代 Base 真实能力。例如当前输入上下文的 Tab/Shift-Tab 是缩进/反缩进，不能宣称 Input 已支持字段遍历；详见 [Input 能力边界](03-input.md)。
- Base 暂不暴露的语义要记录缺口与验证范围；不能添加一个方法名就声称 Narrator 已能正确播报。

## 11. 测试如何组织

### 11.1 按可证明的合同分层

| 层级 | 放置位置 | 主要证明 |
| --- | --- | --- |
| 纯规则/数据测试 | 组件内 `#[cfg(test)]` 或其 `tests` 子模块 | 值规范化、边界、状态转换、几何、token 解析 |
| 模块内无头交互测试 | 组件内按 `headless-tests` 条件编译的子模块 | 私有会话、真实事件路径、布局和提交绘制数据 |
| 公共接口/集成测试 | `crates/yue-component/tests/` | 外部导入、公开事件、跨组件组合、默认主题联动 |
| Base 补丁测试 | `vendor/gpui-base` 的对应测试模块 | 通用行为和补丁合同，不重复证明 Yue 皮肤 |
| Showcase/Windows 实窗 | 对应示例和人工验收记录 | GPU 像素、DPI、字体、真实 IME、Narrator、动效观感 |

无头测试是显式 opt-in；当前 feature 直接启用 Base test-support，不启用 Kit test-support，以免引入另一后端。普通纯测试必须在不打开该 feature 时仍能运行。

小组件使用 `tests.rs`；复杂组件使用 `tests/mod.rs` 加具名子模块。混合测试可以采用下面的组织方式，纯测试不能跟着整目录一起被关掉：

```rust
// component/mod.rs
#[cfg(test)]
mod tests;

// component/tests/mod.rs
mod model;
#[cfg(feature = "headless-tests")]
mod keyboard;
#[cfg(feature = "headless-tests")]
mod layout;
```

若公开集成测试只依赖无头环境，在 Cargo test target 的 `required-features` 中声明；不要为了沿用已有写法把本来可独立运行的纯测试也放进该 target。已有内联纯测试迁移后删除旧注册，避免重复。

### 11.2 重构前后都要有证据

1. 先跑相关基线，记录环境、命令、失败项和覆盖边界；未通过的既有问题不写成“通过”。
2. 修 bug 时先补会在旧行为下失败的回归测试；结构迁移先保留现有测试，不重写断言以迁就实现。
3. 至少覆盖正常路径、非法边界，以及本次改动涉及的状态/生命周期组合；复杂异步组件覆盖晚到、乱序和卸载。
4. 禁用/loading 键盘测试先让可用控件真正获得焦点，再改变状态并发键；不能以“未获得焦点所以没响应”证明禁用实现正确。
5. 动画不能只测起终点：按需采样中间帧、中途反向、外部释放与 reduced motion。布局断言使用测得 bounds，不能只比较配置值。
6. 测试名称只描述实际测到的内容。无头节点信息不能代替真实读屏，无头逻辑像素不能代替 Windows DPI/GPU 验收。

## 12. 存量组件改造计划

以下是改造队列，不表示已有功能存在缺陷，也不授权一次性批量改写。每次选择一个边界清晰的批次，完成后更新该组件文档。

| 对象 | 目标改造 | 优先级/边界 |
| --- | --- | --- |
| Button、Switch、Checkbox、Radio、Segmented、Link、Swatch | 单文件迁入同名目录；仅将清晰的组合/模型/测试职责拆出 | 第一批：建立规则；简单组件不强制拆 State |
| Tag、Divider | 将 `name.rs` 改为 `name/mod.rs`，接续已有子模块 | 第一批：只统一入口，保持根导出与行为 |
| Input 家族、Field、Form | 迁出 Field；Input 按文本/多行/数值/共享外框分工；Form 独立 | 第二批：先测 Field+Input/Select 与提交聚焦，避免依赖环 |
| Select | 从 panel 中分出字段 element 和选项 option；保持现有 controller 与请求合同 | 第三批：分别验证字段布局、键盘、选择与异步 |
| Dialog | 分离公共 builder 与视图组合，保留 model/state/layer 职责 | 第三批：不能改变模态、焦点恢复和退出生命周期 |
| Slider、Tabs、Collapse | 已有目录保留；仅在出现独立职责时继续拆分 | 按改动需要，不为了与 Select 文件数一致而扩张 |
| ColorPicker、Dropdown、Toast、Tooltip、Breadcrumb、Loading | 保留现有合理分工，检查越界依赖与测试分层 | 按改动需要；Loading 继续作为紧密家族 |
| Popup、theme、tokens、motion、typography、layout、icon bridge | 维持基础设施边界，清理须有实际消费者和证据 | 最后审视公共抽取，不借此制造通用引擎 |

建议顺序为：**低耦合目录统一 → Input/Field/Form 边界 → Select/Dialog 职责 → 其他按需 → 评估真实重复的公共机制**。每批都允许小范围调整顺序，不应把所有组件同时搬动以追求表面一致。

### 12.1 每个迁移批次的步骤

1. 写清本批范围、当前依赖、目标目录、状态所有者及是否修改公共合同。
2. 补必要的基线和回归测试，记录改造前结果。
3. 机械迁移：移动文件、调整 `mod/use`、保持根 re-export；不同时调 token、交互或默认尺寸。
4. 检查字段可见性、测试注册、资源路径（如 `include_bytes!`）、doc 链接和 Showcase 引用。
5. 再做必要的职责分离；若有行为/API 变更，作为单独的可审查批次列出前后差异并同步测试。
6. 按下一节完成相应验收，读回文档与公开入口，记录未覆盖的实窗项。

### 12.2 验收命令与范围

以下为结构/API 改造的完整检查集；执行时保存真实结果，不在规范中固定易过期的测试数量。只改文档时检查链接、路径和描述即可，不必为此重跑全部 Rust 测试。

```powershell
node tools/sync-yue-theme.mjs --check
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
cargo check -p windows-showcase --locked
```

第二条 token 命令验证与上游设计系统的一致性，需要该本地路径可用；普通消费者可仅执行快照一致性检查。若改了 Base，须另跑对应补丁测试；Base 不在 workspace 内，不能以 workspace 全绿替代：

```powershell
# 示例过滤器，按本次改动替换；跨模块改动需覆盖所有受影响模块。
pwsh -NoProfile -File tools/test-gpui-base.ps1 -Offline -Filter select::tests
```

Windows 实窗验收按改动范围选择 DPI、字体、键盘、IME、读屏、拖动或开合动效；需要观察性能与动效时使用 release 构建。记录“未验证”比用无头测试推断实窗结论更准确。

## 13. 后续组件设计文档必须回答的问题

新组件继续使用递增编号的 `docs/NN-组件名.md`，引用本规范，不复制一整份架构模板。设计至少回答：

| 问题 | 最低要求 |
| --- | --- |
| 组件是什么 | 使用场景、非目标、与相邻组件的边界 |
| Base 已提供什么 | 锁定版本的实际源码位置、公开能力、缺口及补丁必要性 |
| 谁拥有状态 | 业务真值、编辑/会话状态、受控或 retained 模式、生命周期 |
| 消费者如何使用 | 根导入示例、配置默认值、事件顺序、错误与重入合同 |
| 如何组合 | 依赖现有组件与基础设施的方向、插槽拥有的语义和布局 |
| 如何表现 | token 来源、尺寸/宽度合同、交互状态、动效和 reduced motion |
| 如何操作与访问 | 鼠标、键盘、焦点、无障碍节点与能力限制 |
| 文件如何划分 | 最小目录与各文件真实职责；没有职责的文件不列入 |
| 如何证明完成 | 纯规则、无头事件、公共组合、实窗验收及覆盖边界 |
| 是否改变已有合同 | 受影响调用方、升级前后示例与回归计划 |

文档须区分“拟议”“已实现”“已测试”。不得把设计稿中的 API、token 或验收表自动当作已经可用的事实。

## 14. 完成标准

- 目录入口一致，文件职责能用一句话说清，没有占位文件或仅为套模板引入的抽象。
- 公共根导出、可见性与初始化保持可解释，内部路径不泄漏给消费者。
- 状态唯一，订阅/任务/注册有所有者与失效路径，没有业务反馈循环。
- 复用已有组件和 Base 机制，基础设施不反向依赖语义组件，未复制编辑器、定位器或主题系统。
- 视觉可追溯到 token；外部布局、内部几何、绘制、hitbox 和焦点边界各有所有者。
- 改造前后测试证据齐全，相关检查通过；实窗未覆盖项明确列出。
- 对应设计文档、README、示例和升级说明同步；未把“目标结构”误写成“已完成改造”。

## 15. 关联资料与代码入口

- [接入加固、token 同步与资源验证](08-integration-hardening.md)
- [Input、Field 与 Form](03-input.md)、[Select](20-select.md)、[Dialog](17-dialog.md)
- [Popup 基础层](12-popup.md)、[Loading 家族](22-loading-feedback.md)
- [crate 公共入口](../crates/yue-component/src/lib.rs)、[初始化入口](../crates/yue-component/src/application.rs)
- [控件外部布局适配](../crates/yue-component/src/control_layout.rs)、[动效适配](../crates/yue-component/src/motion.rs)
- [token 同步工具](../tools/sync-yue-theme.mjs)、[Base 补丁说明](../vendor/gpui-base/YUE-PATCH.md)

本文依据本仓库现状及已加载的 gpui-kit Coding Guides / Design Guides 编写；上游指南用于约束所有权、组合和交互，具体行为以锁定源码验证，不以类比其他框架推定。
