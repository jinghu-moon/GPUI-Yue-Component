# YueEmpty 空状态组件设计

- 状态：V1 已实现（2026-11-07）；Windows 实窗像素、DPI 与 Narrator 阅读顺序由用户验收。
- 日期：2026-11-07。
- 目标：为列表、搜索结果和内容面板提供统一、克制的空状态展示，说明当前没有什么，以及用户接下来可以做什么。
- 参考：[用户提供的 Empty 示例](assets/empty-reference.png)、本项目锁定的 GPUI Kit 0.8.0 源码与[组件架构约定](24-component-architecture.md)。

## 1. 定位与边界

`YueEmpty` 是展示型组件，组合媒体、标题、说明和操作区。宿主决定何时显示、展示什么文案、有哪些操作，以及操作后的业务状态。组件不会自行判断数据是否为空、执行搜索、刷新请求或创建记录。

首版支持默认空态、自定义图标、图片、自定义媒体、可选标题、可替换说明、操作插槽、两档尺寸及纵向／横向布局。采用 `RenderOnce`，不公开 `YueEmptyState`，不需要每个空态一个 `Entity`、计时器或订阅。

| 情形 | 展示与所有者 |
| --- | --- |
| 数据已就绪，集合为空 | 宿主显示 YueEmpty，例如“暂无数据”。 |
| 搜索／筛选后没有结果 | 宿主提供“未找到匹配项目”及修改条件的建议；如有清除筛选操作，传入现有按钮。 |
| 首次使用，没有内容 | 宿主提供对象名称和创建／导入入口；不是所有空集合都应显示创建按钮。 |
| 正在加载 | 复用 `YueSpinner`／`YueSkeleton`，不提前显示“暂无数据”。 |
| 请求失败、无权限或离线 | 宿主显示明确原因与恢复路径，不能伪装成正常的空集合。可以复用本组件布局，但不新增 Empty 的业务状态枚举。 |

首版不包含自动重试、加载状态机、插画资源包、图片预览、空态切换动画、自动焦点转移或跨平台适配。项目当前面向 Windows。

## 2. 参考图的吸收方式

| 参考内容 | Yue 设计 |
| --- | --- |
| 托盘图标＋“暂无数据” | 默认中性空态，纵向居中，无标题、无操作。默认图标复用 Tabler Inbox。 |
| 自定义警告图标与英文文案 | 可替换图标和说明；有真实警告含义时才使用警告色，普通无结果保持中性。 |
| 自定义图片＋Refresh | 图片占据稳定的媒体框，说明可以隐藏，操作区组合 `YueButton`。刷新逻辑归宿主。 |
| `description` 接受内容节点 | 普通说明通过 `description` 接收字符串；`description_content` 接收富内容及外部文档链接，保留后代语义。 |
| 展示框边界和留白 | 截图的外框属于示例容器；Empty 默认透明、无边框、无阴影，不自动生成卡片。 |
| `imgSrc`、`icon`、`className`、`style` | 转为 Rust 类型明确的媒体入口、尺寸和布局配置；不复制 CSS 字符串接口。 |

不从截图推算一套独立字号、颜色或间距。外观服从 Yue 的主题、排版和缩放规则，示例中的蓝色按钮也不意味着刷新命令必须使用 Primary。

## 3. GPUI Kit 能力核对与复用

核对对象是本地 0.8.0 源码，不把其他版本的网页 API 当作现有能力。

| 已有能力 | 采用方式 |
| --- | --- |
| `gpui_kit::base` | 当前没有可直接复用的 Empty 专用组件。本需求是展示组合，无需为此扩展 Base。 |
| GPUI Component `Empty` 家族 | 现有 `Empty::header(...)`／`content(...)`，以及 `EmptyHeader` 的 media/title/description 插槽，可作为结构参考。它使用自己的主题，不能为方便而开启本项目关闭的 Component 层。 |
| Component Empty 的布局经验 | 借鉴确定的插槽顺序、文本 `min_w_0`、正文最大宽度、媒体不被拉伸，以及动作独立成组；Yue 首版不逐个复制其六个辅助类型。 |
| `Icon`／`IconDefinition`、Tabler 资源 | 默认图标使用现有 `outline::Inbox` 资源；已新增语义映射 `YueIcon::Empty`，同步映射清单及测试，不自绘另一套图标。 |
| GPUI `img`／`ImageSource`／`ObjectFit::Contain` | 复用图片加载、缓存与缩放能力，使用现有 fallback 入口，不创建 Yue 图片请求器。 |
| `YueButton`／`YueLink` | 创建、刷新、清除筛选使用 Button；外部帮助文档使用 Link，保留其键盘、焦点和禁用语义。 |
| `typography.rs`、YueTheme、token 适配 | 统一字体、行高、颜色和尺寸。Empty 不引入第二套主题。 |
| `YueList::empty(...)` | 已有任意元素空态插槽，可直接组合，空态不计入列表行。 |
| `YueSelect::empty_text(...)` | 当前只支持文案，没有自定义 Empty 插槽；不能将本文 API 示例宣称为现有 Select 能力。 |

插画不是 `YueImagePreview` 的职责。ImagePreview 是交互式图片查看器；空态图片直接使用 GPUI 图片元素即可。

## 4. 公共 API

类型从 `yue_component` 根及既有 prelude 导出，组件字段私有。以下省略实现，只列公共合同。

```rust
pub enum EmptySize {
    Small,
    Medium, // 默认
}

// 以下省略实现，所有 builder 均消费并返回 Self。
impl YueEmpty {
    pub fn new(id: impl Into<ElementId>) -> Self;
    pub fn theme(self, theme: YueTheme) -> Self;
    pub fn size(self, size: EmptySize) -> Self;
    pub fn layout(self, axis: Axis) -> Self;
    pub fn inset(self, inset: bool) -> Self;

    pub fn icon(self, icon: impl IconDefinition) -> Self;
    pub fn image(self, source: impl Into<ImageSource>) -> Self;
    pub fn media(self, content: impl IntoElement) -> Self;
    pub fn hide_media(self) -> Self;

    pub fn title(self, title: impl Into<SharedString>) -> Self;
    pub fn description(self, text: impl Into<SharedString>) -> Self;
    pub fn description_content(self, content: impl IntoElement) -> Self;
    pub fn hide_description(self) -> Self;
    pub fn actions(self, content: impl IntoElement) -> Self;
}
```

`new` 接收稳定 ID，便于重复组合、测试定位和后代命名；ID 不意味着创建 retained controller。需要改变空态时，宿主在下一次 render 提供新的配置。

### 4.1 默认值

| 属性 | 默认 |
| --- | --- |
| 尺寸 | Medium |
| 布局 | `Axis::Vertical` |
| inset | true，使用对应尺寸的内边距 |
| 媒体 | 中性 Inbox 图标 |
| 标题 | 无 |
| 说明 | “暂无数据” |
| 操作 | 无 |
| 外观 | 透明、无边框、无阴影；不响应 hover／点击 |
| 几何 | 使用可用宽度，内容高度自然增长；不自动撑满高度，不创建滚动区域 |

页面需要在剩余视口中垂直居中时，由宿主的 flex 容器提供高度与居中规则。Empty 不默认 `flex_1` 或固定最小高度，避免嵌入列表、弹层时意外抢占空间。

### 4.2 插槽与覆盖规则

视觉和阅读顺序固定为 **media → title → description → actions**，与 builder 的调用顺序无关。缺失部分不占空间，也不保留相邻的空 gap；所有部分都被隐藏且没有标题／操作时，仅保留显式配置的 inset，不生成占位行或可访问空文本。

- `icon`、`image`、`media`、`hide_media` 是同一媒体槽的互斥设置，最后一次调用生效。内部以单一枚举表达，不并存多个 `Option` 再猜优先级。
- 标题是可选短文本；空字符串不生成标题节点。只设置标题不会隐式删除默认说明；不需要“暂无数据”时显式调用 `hide_description` 或替换说明。
- `description`、`description_content` 替换整个说明槽，`hide_description` 清空它，最后一次调用生效。文本入口为空白时不占行，并为非空文本生成 Label 语义；富内容入口保留后代语义，调用方负责自身布局、文本语义和空内容判断。GPUI 普通字符串元素没有可访问 ID，不能仅把字符串擦除为 `AnyElement` 就承诺可朗读；因此实现将两个入口明确区分。
- `actions` 替换整个操作槽，不追加隐式按钮。多个操作由调用方组合一个可换行的动作容器；组件不遍历后代来猜测主操作。
- 不提供通用 `child` 混入根节点；显式插槽使间距、阅读顺序及后续演进可预测。

自定义 Icon 颜色、头像或插画布局使用 `media(...)`，无需为 Empty 扩展图标着色、头像组或一整套业务 tone。

## 5. 布局合同

### 5.1 纵向与横向

默认纵向：媒体在上，标题与说明在中，操作在下，整体居中，文字居中。内容区限制舒适阅读宽度，但受父容器可用宽度约束，不以固有文本宽度撑开面板。

横向使用 `layout(Axis::Horizontal)`：媒体在左，右侧为标题、说明、操作组成的文本列；文本和操作按起始边对齐，两列顶部对齐。适合高度受限的内容条或侧栏，不把操作强行挤到最右侧形成第三列。

组件不根据当前宽度自动切换方向，也不通过测量回写业务 Entity。宿主按区域布局选择方向；空间不足时优先改用纵向或 Small。未提供媒体时，横向退化为完整宽度的文本列，不留下空列。

### 5.2 收缩、换行与滚动

- 根、正文列和文字层允许收缩，使用 `min_w_0`；根不强制父容器变宽。
- 标题和说明默认自然换行，不固定行数，不把恢复建议裁成省略号。中英文混排及长连续文本应使用实际文本测量与可用的断行策略验证，不能用字符数估算宽度。
- 多个按钮的换行由传入的动作容器负责；使用现有按钮本身的文字与命中规则，不把按钮整体缩小以适应窄面板。
- Empty 自身不滚动，不截获滚轮。长文案或放大文字导致高度增加时，由页面／面板既有滚动区域承接；宿主不应以过小固定高度裁掉动作和焦点环。
- `inset(false)` 只移除外层内边距，媒体、文字和操作之间的内部间距保持不变，用于已有内容留白的 List 或面板。

### 5.3 媒体约束

默认图标尺寸和图片框尺寸不同，均随尺寸档位、主题及 rem 缩放。图标使用固有方形尺寸，不因为纵向 flex 的默认 stretch 被拉宽。

图片使用固定的目标媒体框及 `Contain`，保持宽高比、不裁切、不强行拉伸。媒体框最大不超过容器可用宽度；极窄容器下按比例收缩。加载中保留框尺寸且不闪现破图；失败在同一框中显示默认图标，说明和操作区不发生跳动。这里只反馈装饰图片的失败，不把它等同于业务请求失败。

自定义 `media` 保持自身固有尺寸，不强制拉满内容宽度；容器提供可用宽度上限，自定义元素负责内部缩放、图片失败回退和无障碍语义。宿主若传入固定过宽的复杂元素，应自行适配，不由 Empty 递归改写后代样式。

## 6. 主题与 token

下表已在 `Yue-Design-System/packages/tokens/src/component-tokens/empty.css` 定义、注册并同步到本仓库快照。组件统一从 token 适配层读取；禁止手写 `generated.rs` 或 `inventory.json`。

| token | 引用／默认关系 |
| --- | --- |
| `--empty-title-color` | `var(--text-primary)` |
| `--empty-description-color` | `var(--text-secondary)` |
| `--empty-icon-color` | `var(--text-secondary)`；说明仍须清晰可读，不使用禁用文字代替普通说明 |
| `--empty-font-size-sm`／`md` | `var(--font-size-sm)`／`var(--font-size-md)` |
| `--empty-line-height-sm`／`md` | `var(--line-height-sm)`／`var(--line-height-md)` |
| `--empty-padding-sm`／`md` | `var(--space-12)`／`var(--space-24)` |
| `--empty-icon-size-sm`／`md` | `var(--space-24)`／`var(--space-32)` |
| `--empty-image-size-sm`／`md` | `var(--space-48)`／`var(--space-64)` |
| `--empty-media-gap-sm`／`md` | `var(--space-8)`／`var(--space-12)` |
| `--empty-title-weight` | `var(--weight-medium)` |
| `--empty-text-gap` | `var(--space-4)` |
| `--empty-actions-gap-sm`／`md` | `var(--space-12)`／`var(--space-16)` |
| `--empty-content-max-width` | `calc(var(--space-64) * 6)`；默认转换为 24rem，约束阅读宽度而非组件固定宽度 |

文字使用现有 UI 字体与 typography 适配；标题通过现有中等字重角色建立层次，不默认放大成页面标题。按钮／链接保留自身 token，不随 Empty 私自改高度、圆角和颜色。透明背景继承宿主表面；插画不得烘焙文字或白色底板。

## 7. 交互、焦点与无障碍

Empty 容器不可点击，不加入 Tab 顺序，不拦截 Enter、Space、Escape 或应用快捷键。操作复用真实 Button／Link，按视觉和 DOM 等价的元素顺序进入键盘导航，保留 hover、禁用、loading 和 focus-visible。

默认图标与图片属于装饰，不新增重复朗读；标题与说明作为真实文本参与可访问树，不能为整个 Empty 设置一个字符串名称而吞掉富文本、按钮和链接后代。自定义媒体若携带业务信息，由宿主提供可访问名称或相应文字。

静态空态不默认声明 alert／live region，也不每次 render 播报“暂无数据”。异步搜索的结果数与必要公告由已有搜索／集合所有者负责，避免双重播报。

数据由非空变空时不自动聚焦 Empty 或第一个按钮。若当前焦点所在行被删除，宿主／集合控制器负责把焦点转移到仍存在的集合容器、搜索框或合理操作；反向由空态变为有数据时同样不能留下悬空焦点。Empty 的显示组件不拥有列表选择和焦点恢复策略。

首版没有循环、入场或切换动效。静态展示不请求下一帧；图片到达使用 GPUI 现有资源生命周期，不建立轮询任务。

## 8. 与已有组件组合

### 8.1 YueList

通过现有 `.empty(...)` 接入，不改 List 的默认值。List 未提供 empty 插槽时继续保持原合同，已有自定义空态不被替换；只有有效列表项为空时才显示空态，空分组不生成虚假行。

List 空态带已经存在尺寸相关的内容带留白，组合时使用 Small 与 `inset(false)`，避免二次面板 padding。空态属于集合的空内容区域，不增加 item count、选中项、虚拟行或方向键目标。

以下为当前调用方式：

```rust
YueList::new("search-results")
    .empty(
        YueEmpty::new("search-results-empty")
            .size(EmptySize::Small)
            .inset(false)
            .icon(outline::Search)
            .description("未找到匹配项目，试试其他关键词"),
    )
```

该片段只展示空态接线；真实列表由宿主照常提供项目。搜索过程中使用其真实加载反馈，不能仅凭本地暂时没有结果就显示空态。

### 8.2 页面与自定义图片

下列示例中 `source` 为宿主提供的 `ImageSource`，`refresh` 为符合现有 `YueButton::on_click` 签名的回调：

```rust
YueEmpty::new("archive-empty")
    .image(source)
    .hide_description()
    .actions(
        YueButton::new("refresh-archive", "刷新")
            .variant(ButtonVariant::Default)
            .on_click(refresh),
    )
```

这对应参考图的图片＋操作示例。实际业务通常应保留明确说明；只有上下文已说明状态时才隐藏。刷新过程是否禁用按钮、是否显示 loading 和完成后切换到什么内容，均由宿主掌握。

首用场景可以使用标题“还没有项目”、说明“创建项目后，会显示在这里。”与“新建项目…”按钮。若按钮打开表单，它是一个普通应用命令，不因为位于空态或是唯一按钮就自动变成 Primary；默认提交动作由表单自身定义。

### 8.3 YueSelect

当前 Select 仅支持 `.empty_text(...)`，空结果分支渲染纯文本；加载、失败重试和可创建选项各有既有分支。首版 Empty 不修改这些路径，也不在 listbox 内塞入随意可聚焦的操作区。

后续若有自定义图文空态的真实需求，再单独设计最小 `render_empty` 插槽，并验证弹层高度、可访问列表语义、方向键、焦点和创建选项优先级。它不属于本次文档要求的首版前置工作。

## 9. Showcase 示例与验收

新增独立 Empty 页面，示例使用真实已有组件：

1. 基本用法：默认图标＋“暂无数据”，展示 Small／Medium。
2. 自定义图标和文案：搜索无结果，以及中英文说明；明确其业务含义。
3. 自定义图片：保持比例，隐藏说明并提供刷新按钮；支持手动切换失败图片，检查稳定回退。
4. 首次使用：标题、说明及新建入口，按钮打开真实示例表单或修改宿主数据。
5. 布局：纵向／横向、窄侧栏、长中文／长英文和多个可换行动作。
6. List 集成：空集合、添加第一项、删除最后一项、筛选无结果与清除筛选，验证计数和焦点。

示例容器负责必要的边界、尺寸和滚动，不把展示框做成 Empty 默认外观。加载／失败／正常空态的切换由示例宿主控制，避免假定组件内置数据状态机。

| 验证层级 | 必须证明的合同 |
| --- | --- |
| 最小公共接口用例 | 根导出、默认构造、媒体替换、说明隐藏及既有 List empty 接入可编译。 |
| 定向无头布局测试 | builder 顺序不改变插槽顺序；隐藏槽无多余 gap；媒体固有尺寸；纵向／横向的窄宽度收缩与长文案换行；inset(false) 的实际几何。 |
| 图片回归 | 用确定的本地成功／失败资源验证固定框和回退，不依赖外网、任意等待或真实网络采样。 |
| 交互／语义集成测试 | 默认根不获取焦点；按钮鼠标与键盘各只触发一次；说明和后代按钮保留语义；List 空态无虚假行、无错误 item count。 |
| 宿主场景回归 | 删除最后一项及添加第一项后的焦点由集合／示例所有者正确处理，不归因于 Empty 自己管理焦点。 |
| Windows 实窗验收 | 用户检查浅色／深色、自定义主题、窄面板、界面缩放、图片清晰度、长文案、按钮焦点环和 Narrator 阅读顺序。 |

测试关注可见合同和组合回归，不为每个简单 setter 编写镜像测试，不为静态 Empty 增加独立性能基准。文档完成不等于上述实现验收已经通过。

## 10. 实施顺序与交付范围

1. **基础展示（已完成）**：新增设计系统 token 并同步；创建 `src/empty/mod.rs`、`element.rs`；实现默认值、插槽、尺寸、布局、图片回退与根导出。沿用现有依赖，不修改 Base 或 Windows provider。
2. **组件组合（已完成）**：补充 Showcase 和 List 空态示例；保留 List／Select 既有默认行为，不扩展业务状态机。
3. **验证与文档（自动回归已完成，实窗待验收）**：已执行下节记录的编译、布局、交互、语义与同步检查；完整门禁的非 Empty 限制单独记录。

实现位于 [empty/mod.rs](../crates/yue-component/src/empty/mod.rs) 与 [empty/element.rs](../crates/yue-component/src/empty/element.rs)，示例位于 [pages/empty.rs](../examples/windows-showcase/src/pages/empty.rs)。静态外观由普通 GPUI 元素组合实现，没有新增依赖或框架补丁。

自动验证与实窗验收分开记录；无头测试不代表已完成 Windows 像素、真实 DPI 或 Narrator 验收。


## 11. 本轮验证记录

- `cargo test -p yue-component --locked --features headless-tests --lib`：393 项通过，其中 Empty 定向回归 6 项，覆盖插槽顺序与隐藏、媒体固有尺寸、纵横向收缩、rem 缩放、图片加载／失败／成功的稳定框、动作键盘与鼠标、富内容语义和 List 空态切换。
- `cargo test -p windows-showcase --locked --features headless-tests --quiet`：49 项通过、8 项原有忽略；新增完整空态场景验证创建、删除最后一项、筛选恢复及工具栏焦点交接，导航测试包含 Empty 页面。
- 本次修改文件的 rustfmt 检查、`git diff --check`、文档链接检查及设计系统 token 同步检查通过。
- 常规 `-D warnings` Clippy 被当前 ImagePreview `decode.rs` 的两处 `chunks_exact_to_as_chunks` 告警阻挡；命令行仅暂时放行此项后，两个包的全部 targets 检查通过，没有修改源码 lint 配置。全仓 rustfmt 检查另有 ImagePreview 测试文件的现有格式差异；这些文件不属于 Empty 改动。
- `cargo check -p windows-showcase --locked` 通过。普通可执行构建在覆盖 `target/debug/windows-showcase.exe` 时因正在运行的 Showcase 被 Windows 拒绝；未结束该进程。关闭旧实例后重新运行 `cargo run -p windows-showcase --locked`，即可构建并验收 **组件 → 空状态 Empty**。
- 本轮未执行 Windows 真机像素、跨 DPI 或 Narrator 验收，也未提交或推送 Git。
