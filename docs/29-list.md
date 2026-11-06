# 29 List 列表组件设计

- 状态：阶段 1–3 已实现；基础记录见第 14 节，规则网格和按行虚拟化见第 15 节。
- 日期：2026-11-06。
- 目标：定义 Yue 通用基础 List 的布局、组合边界与现有能力复用方式。复用现有虚拟化引擎；消费测试发现的 Base 最小修复单独记录。

List 是承载一组相关内容的轻量布局组件。它应让文本行、媒体条目、操作行等常见内容共享一致的容器和对齐规则，同时允许调用方组合现有控件。List 不拥有业务数据、选择值、分页、请求或树模型。

**核心决定：List 默认是纵向连续容器，ListItem 是可组合的单行/单项布局；行内方向与集合排布方向分别表达。基础 List 不把表格、树、Masonry、选择器和数据加载并入同一个状态机。**

## 1. 参考资料与取舍

本设计参考：

| 参考 | 可借鉴内容 | 不照搬的内容 |
|---|---|---|
| [List 列表 · GPUI.html](../refer/List%20%E5%88%97%E8%A1%A8%20%C2%B7%20GPUI.html) | 容器/行 anatomy、前置/正文/附加信息/操作槽位、尺寸、分隔与斑马纹、头尾、加载/空/错误、分组、嵌套、网格及虚拟列表示例 | 网页配置面板、Inbox 整体业务、所有 API 都塞进单个 List 类型 |
| [Image #1：列表示例](assets/list-reference-1.png) | 连续文本行、主副标题、媒体行、操作行、头尾、尺寸差异、斑马纹、加载与滚动 | hover-only 关键操作与浏览器专属 CSS 细节 |
| [Image #2：列表布局与 API 示例](assets/list-reference-2.png) | 最小列表容器、尺寸档位、基础/元数据行、操作、竖排内容、固定列网格、响应式网格、虚拟滚动示例 | Web DOM observer、CSS Grid API、分页或网络加载由 List 内部自动管理 |

参考图是示例覆盖面，不代表这些能力必须共享一个公共状态或构造器。将可组合的结构抽为 List/ListItem；将特定数据行为交给对应组件或宿主。

## 2. 组件范围

### 2.1 List 负责

- 提供一个有明确布局方向的内容集合容器。
- 统一集合间距、尺寸、边框/背景、项间分隔或斑马纹。
- 提供可选 header、footer、empty/content 状态区域，以及稳定的组边界插槽。
- 将多个 ListItem 按一致的宽度、对齐和尺寸规则排列。
- 允许父级决定滚动 owner；当集合很大时，可由专门的虚拟呈现适配消费同一类 item renderer。

### 2.2 List 不负责

- 不持有或复制业务列表；项顺序、插入、删除、分页和异步请求由宿主持有。
- 不内置单选/多选模型。单项复选可组合 `YueCheckbox`；选择集合和值由宿主/选择控制器持有，候选弹出选择继续使用 `YueSelect`。
- 不内置搜索、排序、拖拽排序、加载更多、撤销删除、无限请求或错误重试策略。
- 不把树形展开、表格列语义、Masonry 分配或二维任意坐标加到普通 List。
- 不创建第二个 ScrollHandle 或镜像滚动位置；滚动由一个明确 owner 提供。

## 3. 布局词汇

必须区分三个层级，避免一个含糊的 `layout` 同时控制不同层次。下表中的 Grid 已实现：

| 层级 | 默认 | 可选方向 | 含义 |
|---|---|---|---|
| 集合 `List` | Vertical | Horizontal / Grid | 纵向连续行、横向单行或规则卡片网格。横向不自动折行；Grid 的列数与换行见第 7 节 |
| 内容 `ListItem` | Horizontal | Vertical | Horizontal 将操作放在行尾；Vertical 将元信息、正文、操作依次放在主内容区，extra 仍可位于右侧 |
| 元信息 `ListItemMeta` | Vertical | 由调用方组合 | 标题和描述纵向排布；媒体放在 Item 的 leading 槽，不重复定义媒体槽 |

行内方向和集合方向是正交配置。改变集合方向不会隐式旋转子内容或语义顺序。

**网格是独立排布模式，不等于 Horizontal List，也不等于双轴 VirtualGrid。** 普通卡片网格使用 GPUI 原生布局；大规模卡片可按行投影到一维虚拟化。`VirtualGrid` 适用于有明确行列轨道、需要双轴窗口的矩阵；Masonry 使用 `VirtualMasonry`。不另造布局或虚拟滚动引擎。

## 4. 建议结构

```text
List
├── Header?                 # 标题、计数或调用方控件
├── Group*                  # 可选分组边界与分组标题
│   └── ListItem*
└── Footer?                 # 汇总、分页控件或调用方动作

ListItem
├── Leading?                # 图标、头像、状态、复选框等
├── Content                 # 主信息；任意 GPUI 内容或 ListItemMeta
├── Body?                   # 长正文、图片、描述等扩展内容
├── Extra?                  # 时间、数值、Badge/Tag 等非主正文
└── Actions?                # Button 或其他调用方控件
```

- `List` 是容器，`ListItem` 是通用布局壳；不强迫所有项都具有标题/描述，也不为每个槽位定义业务模型。
- `Content` 可直接传 GPUI Element。需要统一双行元信息时，提供轻量 `ListItemMeta` 组合器；标题和描述仍是调用方内容，不由 List 读取或修改。
- `Leading`、`Extra`、`Actions` 为可选内容槽。默认不因槽位缺省自动填充不可见占位；需要跨行严格对齐时，调用方通过同尺寸内容或显式占位控制。
- `Actions` 默认在正常布局中保留尺寸，出现/隐藏不能改变行高或正文可用宽度。必要操作不得只在 hover 后可发现；鼠标 hover、键盘 focus-within 和触屏都须能访问。
- item 内部交互控件保持为兄弟节点，不把 Button 嵌入另一个 Button，不由 ListItem 外壳吞掉子控件的焦点/点击。

两种 Item 布局的具体区别：

```text
Horizontal（常规行）
┌────────────────────────────────────────────────────────┐
│ Leading  ┌─ Content / Meta ─┐  Extra  Actions           │
│          └─ Body? ──────────┘                          │
└────────────────────────────────────────────────────────┘

Vertical（文章/详情项；集合仍可纵向排列）
┌────────────────────────────────────────────────────────┐
│ ┌─ Leading + Content / Meta ─┐  ┌─ Extra ────────────┐ │
│ │ Body                       │  │ 封面、附加媒体等  │ │
│ │ Actions                    │  └───────────────────┘ │
│ └────────────────────────────┘                        │
└────────────────────────────────────────────────────────┘
```

Vertical 表达主内容区内的纵向关系，不把全部槽位强制依次竖排。需要上图下文的卡片时，调用方把封面组合在 Content/Body 中；Grid 只排列卡片，不改变卡片内部结构。无 Extra 时主区占满宽度。

主内容区允许收缩并具有零最小宽度；leading、extra、actions 默认不压缩。标题默认单行省略，描述和 Body 默认自然换行；调用方可显式选择多行标题或描述截断。任意自定义 Element 的文本策略仍由调用方决定。超长 actions 可在其槽位内换行，或由宿主组合 `YueDropdownMenu` 收纳，不强制压住正文。窄宽度下 Vertical 的 Extra 可按显式容器宽度阈值移到正文下、Actions 前；默认保持右侧，不暗中用窗口宽度改变结构。

## 5. 尺寸、间距与表面

沿用项目现有 `Small / Medium / Large` 命名习惯，List 自己拥有列表尺寸类型，不直接拿 Button 尺寸代替行高。默认 Medium。

| 尺寸 | 目标 | 规则 |
|---|---|---|
| Small | 命令、简短选项、密集侧栏 | 较小的行内/块间 padding、文字阶梯与 leading 图标尺寸；内容仍须完整可读 |
| Medium（默认） | 普通设置项、成员行、简短说明 | 中等密度，默认标题/描述组合可容纳 |
| Large | 触控或媒体内容较多的列表 | 增加内边距与媒体尺寸；不把纯空白当主要强调 |

- 行最小高度由尺寸与内容自然高度共同决定；不为所有 item 强制固定高度。
- 若消费者选择 Fixed 虚拟化，调用方必须保证 item 遵守固定主轴尺寸；Measured 内容由现有 Measured 路径反馈尺寸。
- padding、gap、文字层级、边框、hover、focus 和选中表面读取 Yue Design System token；不在本组件硬编码颜色或孤立间距。
- 容器表面最多采用一致的边界方案：边框/圆角由 List 拥有，项间分隔仅画在相邻项之间；不默认给每一行再套卡片。
- Selected 视觉由组合者提供受控状态；ListItem 只呈现 `selected(bool)` 与 `disabled(bool)`，不保存选择值。

### 5.1 配置默认值

| 配置 | 默认 | 合同 |
|---|---|---|
| 集合排布 | Vertical | Horizontal / Grid 必须显式选择 |
| Item 内容排布 | Horizontal | 集合改为 Grid 不隐式改 Item 布局 |
| size | Medium | Item 显式尺寸优先于 List 继承值；独立 Item 也为 Medium |
| split | 纵向线性集合为 true；Horizontal / Grid 为 false | 只支持线性集合的相邻项边界；Horizontal 显式启用时为竖向分隔，Grid 使用间距 |
| bordered | false | 开启后由 List 拥有一层外框和圆角，头尾分界不重复画线 |
| striped | false | 仅纵向线性集合启用，按数据项逻辑顺序交替，组标题/头尾不计入序号 |
| hoverable | false | 按需开启；不附带 pointer 光标、点击行为或焦点，纯展示示例保持关闭 |
| selected / disabled | false | 受控呈现；真实子控件状态由宿主显式传递 |
| 滚动 | 不创建 | 交给父级或显式虚拟视口 |
| 空状态 | 不注入默认文案 | 无数据时替换 body；header/footer 仍可显示 |

样式优先级为显式 Item 配置 → List 的继承配置 → 组件默认值。外部 GPUI 布局用于决定组件所占空间，不应用任意外部样式覆盖内部槽位几何；通用外部布局沿用项目 `control_layout` 合同。Striped 使用完整集合的逻辑 index，不能使用虚拟窗口挂载序号；分组默认连续计数，宿主重排后按新顺序计算。

当前状态表面优先级为 selected → hover → striped/基础背景；disabled 抑制 hover 并改变外壳文字颜色，保留已有 selected 背景以表达受控选择。槽内显式设置的颜色和禁用状态由子组件拥有。focus-visible 由真实控件绘制，必须同时可见。Pressed 反馈由实际动作控件呈现，List 不伪造整行 pressed 状态。

### 5.2 Token 来源

现有列表语义 token 为 `--list-row-background-hover`、`--list-row-background-selected`、`--list-row-color`；其余映射现有 `--surface`、`--surface-subtle`、`--border-default`、`--border-divider`、`--text-secondary`、`--text-disabled`、focus-ring、radius 和 space/排版 token。不得将尚未存在的完整 List 尺寸 token 表写成已实现能力。

拟议尺寸映射：Small 使用 block `space-4`、inline `space-8`、gap `space-8`；Medium 使用 `space-8`、`space-12`、`space-12`；Large 使用 `space-12`、`space-16`、`space-16`。这是现有 token 的组合提案，需实窗检查后定稿，尺寸随 rem 缩放。若需要专属 List token，先修改设计系统源头再同步快照与生成代码，不手改 `theme/generated.rs` 或 `tokens/inventory.json`。

## 6. 分组、头尾与状态区

- `header` 和 `footer` 是任意内容，不绑定特定 Header/Footer 组件。可组合标题、计数、`YueButton` 或宿主分页控件；List 不计算页数。
- 普通分组标题使用独立的 `Group`/slot 结构，并通过同一个 List 容器连续布局；首版可组合普通组，不附带 sticky 行为。Sticky group header 只有在实际消费者需要且和滚动/虚拟窗口生命周期有完整验收后才加入。
- 空、加载、错误状态是可替换的内容区域，而不是 List 内部请求状态机。初始加载可替换 body 为 `YueSkeleton`；已有内容刷新可组合 `YueLoadingRegion`，保留阅读位置；追加加载在 footer 放置 `YueSpinner` 或按钮；错误内容组合文案与 `YueButton` 重试操作。状态切换和请求去重由宿主控制，List 不添加计时器。
- 嵌套展开不是普通分组。需要层级身份、展开状态、键盘树导航与语义时，使用 Tree 组件设计，不通过 ListItem 子列表拼装出伪 Tree。

Group 的稳定 ID 与条目业务 ID 分开；组标题描述相应子集合，不伪装成可点击数据行。当前空组省略；需展示空组说明时由宿主组合有内容的条目。List 根据真实结构计算相邻边界，组标题两侧不叠加 item split 与 Divider。普通展示内容的折叠可组合现有 `YueCollapse`，这不自动获得 Tree 的导航语义。

固定头尾采用父级纵向 flex：header/footer 不收缩，中间 body 具有有限高度并成为唯一滚动区域。自然页面滚动时头尾随整个 List 一起滚动。两者由宿主结构显式选择；不通过 List 私建嵌套滚动区。

## 7. 网格、滚动与虚拟化

| 场景 | 布局原语 | 责任边界 |
|---|---|---|
| 普通纵向集合 | `YueList` + `YueListItem` | 集合布局与行样式；父级决定是否滚动 |
| 横向短集合 | `YueList` Horizontal | 项按内容宽度排列；不自动换行，不承诺多行 flow |
| 规则卡片网格 | GPUI 原生布局；大集合可按行适配一维虚拟化 | 普通 YueList.grid / 虚拟 YueVirtualListGrid，规则见下文 |
| 大型一维集合 | `YueVirtualList` | 复用同一 renderer/行组合，固定/已知/Measured 尺寸由宿主选择 |
| 大型二维表格/矩阵 | `VirtualGrid` | 双轴共享滚动 owner；独立处理冻结、列语义等合同 |
| 多列瀑布流 | `VirtualMasonry` | 分栏与测量，不属于 List 网格模式 |

虚拟呈现适配应尽量复用 ListItem 外观/槽位 renderer，但不强迫普通 List 创建 `VirtualizerState`，也不让 `YueList` 暗中在数量阈值后切换滚动/尺寸语义。由宿主显式选择普通或虚拟路径。

### 7.1 固定列与响应式卡片网格

- 支持显式固定列数，以及基于容器内容宽度的最小卡片宽度模式；二者互斥，列数和最小卡片宽度必须大于零，gap 非负，所有尺寸须为有限值。配置构造时返回明确错误，不静默修正非法值或在布局时除零。可提供容器断点列数映射，但不把 window 宽度当成嵌套 List 的实际可用宽度。
- 设可用宽度为 W、最小卡片宽度为 m、间距为 g，自动列数为 `max(1, floor((W + g) / (m + g)))`，卡片宽度为 `max(0, (W - (列数 - 1) × g) / 列数)`。固定列在过窄容器下由宿主改断点或显式选择水平溢出；不擅自修改声明的列数。
- 按数据顺序从左到右、逐行排列；末行保持与完整行相同的列宽，靠起始侧对齐，不拉宽少数项，也不补有身份的虚假项。
- 每行高度取该行最大自然内容高度，卡片外壳可拉齐；下一行从同一基线开始。单项跨列、跨行和按最短列填充不在规则网格合同内。
- Grid 默认无连续行 split/striped；需要卡片边界时由每张卡片自身表面表达，List 的 `bordered` 仍只控制集合外框。
- 普通网格优先 GPUI 原生 flex/wrap 或网格布局能力，不扩展 `layout.rs` 成第二套 tracks 引擎。容器 bounds、列数计算与数据业务状态分开，只有实际测量需要时保留最小布局信息。

### 7.2 大集合适配

现有 `YueVirtualList` 已覆盖纵向一维 Fixed/Known/Measured 和共享稳定身份。`ListGridState` / `YueVirtualListGrid` 是已实现的组合层：每个虚拟条目对应一整行卡片，行高由最大卡片高度决定，复用现有测量和滚动 owner。容器宽度改变导致列数变化时，必须按业务 item key 重新定位阅读锚点并处理焦点，不能把旧行 index 当成业务身份。

显式二维矩阵才使用 `VirtualGrid` 的双轴轨道合同。规则网格适配不能宣称当前 `VirtualGrid` 已提供任意内容高度的响应式卡片能力。基础 List 不保证横向虚拟集合适配；现有能力与新增适配须分别验收。

虚拟视口必须有有限高度；header/footer 位于视口外时不计入 item 测量。完整 item 外壳（含 padding 与分隔占用）参与尺寸合同，避免固定高度总和偏差。普通与虚拟路径复用 item 内容和 token，不要求共用同一个集合容器；由适配层负责语义、逻辑序号与组边界。

## 8. 现有组件复用清单

| 能力 | 现有实现 | List 的用法 |
|---|---|---|
| 行操作 | `YueButton`，含文字/图标构造、焦点、禁用、尺寸和变体 | 作为 `Actions` 槽子项；不重做 button 行为 |
| 多选控制 | `YueCheckbox` | 作为 leading 或 content 的真实控件；List 不管理选中集合 |
| 短状态/分类 | `YueTag` | 作为 `Extra` 内容；避免用 Tag 替代所有元信息 |
| 内容分隔 | `YueDivider` | 用于组间/区域分隔；其默认 block margin 不适合直接充当每行 split。连续行复用分隔 token 和项目细线绘制经验，不重复创建完整 Divider |
| 选择/候选列表 | `YueSelect` | 自有搜索、active 与 selection 合同；内部列表项可复用行内容原则，但不把 Select 状态搬进 List |
| 一维虚拟化 | `YueVirtualList` + Base `VirtualList` | 保持原生 owner、测量和稳定 key；List 设计不新增虚拟引擎 |
| 二维虚拟化 | `VirtualGrid` | 用于明确双轴轨道的表格/矩阵，不直接替代响应式卡片网格 |
| 瀑布流 | `VirtualMasonry` | 独立布局原语 |
| 加载反馈 | `YueLoadingRegion`、`YueSkeleton`、`YueSpinner`、`YueProgress` | 由宿主组合状态区域/进度反馈，不由 List 启动任务 |
| 更多操作 | `YueDropdownMenu`、`YueContextMenu`、`YueTooltip` | 通过公开入口组合；关键操作不能只有右键或悬停入口 |
| 头像 / 分页 | 当前无独立 Yue Avatar / Pagination | 头像使用 GPUI 图像或自定义内容；分页由宿主 footer 组合，不为 List 额外发明配套组件 |

基础容器应使用普通 GPUI `div`/flex 布局与现有 Yue token 适配。不要依赖另一套 `gpui_kit::component::List` 视觉层，不复制它的 ListState/Delegate 或 CSS API。

## 9. 基础公共 API

以下构造已实现；从 crate 根导入 `YueList`、`YueListItem`、`YueListItemMeta`、`YueListGroup`、`ListLayout`、`ListItemLayout` 与 `ListSize`。示例中的槽位变量由宿主构造。组件采用 `RenderOnce`，数据与真实控件状态由宿主持有。

```rust
YueList::new("members")
    .theme(theme) // 省略时使用应用当前主题
    .layout(ListLayout::Vertical)
    .size(ListSize::Medium)
    .split(true)
    .striped(false)
    .bordered(true)
    .hoverable(false)
    .header(header)
    .group(YueListGroup::new("design-team", "设计组").children(group_items))
    .empty(empty_state)
    .child(
        YueListItem::new("member-alex")
            .leading(avatar)
            .content(content)
            .extra(status)
            .actions(actions),
    );

YueListItem::new("notice-1")
    .layout(ListItemLayout::Vertical)
    .size(ListSize::Large)
    .selected(false)
    .disabled(false)
    .content(meta)
    .body(article)
    .extra(cover)
    .actions(actions);
```

集合 `.child/.children/.group` 按调用顺序追加；每个 item 必须有稳定业务身份，不以可见 index 或显示标题作 key。Item 仅用 `.content` 表达主内容，不再同时提供语义相同的 `.child`；leading/content/body/extra/actions 是单槽配置，重复设置按最后一次替换。List 的 header/footer/empty 同样为单槽，empty 只在没有数据项时呈现。`.empty` 不负责判断网络加载状态。

集合入口接收可识别的 Item/Group 结构；任意自定义行内容放入 Item.content，任意装饰内容放入头尾或组标题。这样 List 可在消费 Element 前计算数据项计数、继承尺寸、组边界与逻辑 index，不通过遍历任意 GPUI Element 猜测类型。仅数据项计入空判断。虚拟适配从数据源获得这些信息，不从已挂载视口反推完整集合。

公共入口遵循稳定 `ElementId`、无 public fields、`RenderOnce`、主题 token 与窄 builder 约定；不提供 `dataSource`、`pagination`、`onEndReached` 或 `virtual` 等业务/运行时配置。

`YueList.item_layout(...)` 设置集合内条目的默认内容方向；条目的显式 `.layout(...)` 优先。`YueListItemMeta::new(title).description(description)` 提供可替换的标题/描述槽，`.truncate_title(false)` 允许多行标题。主题覆盖只传递到 List 自己拥有的 Item；任意槽中嵌套的 Meta/Button/Checkbox 等组件需自行传 `.theme(theme)`，或统一使用应用主题。Leading 媒体与真实子控件尺寸由调用方决定，List 不改写任意子元素。

同一个 List 内 Item/Group ID 必须唯一，重复 ID 在 render 构造期明确 panic。分组标题不能为空。横向集合按组排列时，每个组内部仍纵向排列；不创建隐式换行或滚动。

虚拟组合使用 `YueVirtualList` 的默认 List/ListItem 语义，并对内部 `YueListItem` 设置 `.semantics(false)` 避免重复节点。需要斑马纹时显式传 `.striped(true).logical_index(item.index())`；不能传挂载窗口内的局部序号。

布局设计范围覆盖线性列表、常规行/文章行、普通分组、固定列和响应式卡片网格。实施按第 11 节分阶段，不把网格从设计目标中删除。Sticky group、拖拽排序、树导航等需独立消费者合同，暂不列入 List 公共 API。

`ListItemMeta` 只提供 title/description 的排版便利，不拥有成员、消息或文章模型。不提供会覆盖插槽内容的额外 title 字符串捷径；消费者可以只组合任意 GPUI 内容。

## 10. 交互与无障碍合同

- 基础 `List` 默认只表达列表/列表项语义，不默认可选中，不为每项创建焦点句柄。
- 单一动作且没有其他交互子控件时，可以复用 Base 真实动作元素承载整项。含独立 actions/checkbox 时，主动作与子控件是兄弟交互区域；不能用外层 Button 包住整行。具体 Base 构造须在实现时确认，不以自制 clickable `div` 代替键盘和可访问行为。
- `.selected(bool)` 只描述受控呈现；选择变更由宿主或 Select controller 发出/提交。
- `ListItem.disabled` 约束自身呈现及自身主动作；它无法自动修改任意 GPUI 槽位中的控件。宿主必须给 Checkbox/Button 等显式传递同一 disabled 真值，并决定是否保留帮助类操作；不能仅靠整体透明度或拦截鼠标实现禁用。
- 装饰性 leading 图标不重复读名；动作按钮有各自可访问名称；容器 label 由具体语义 owner 提供。
- hover、focus-visible、pressed、selected 和 disabled 的层级不互相覆盖；键盘 focus 时操作始终可见。
- 普通 List 不自行发明方向键导航规则。若使用单焦点多选/roving focus，应由独立选择/导航 controller 明确提供并测试。
- 普通集合使用 List/ListItem 语义，Grid 卡片仍是内容列表，不自动升级成 UIA Grid 或 ListBox；只有确有选择/网格导航合同的 owner 才改变角色。`selected` 外观不等于获得可访问选择模式，Checkbox 等真实控件自行暴露值。
- 纯内容项不进 Tab 顺序；Tab 访问真实控件，Enter/Space 由对应控件处理。删除、重排或虚拟卸载时，焦点恢复由宿主/既有虚拟化合同处理，不复用已换业务身份的焦点节点。
- 基础布局不加入删除限高动画、hover 抬升或默认平滑滚动；需要动效时复用已有 motion/reduced-motion 策略。

## 11. 验收与分阶段实现

### 11.1 实施顺序

| 阶段 | 交付 | 完成条件 |
|---|---|---|
| 1：基础线性列表 | 容器、Item/Meta、两种内容排布、三种尺寸、表面、头尾、空区、普通分组 | 基础 Showcase 示例与布局/组合测试通过；不引入 ListState |
| 2：规则卡片网格 | 固定列、容器响应式列数、间距、末行与变高行规则 | 同一 renderer 在窄/宽容器和实时 resize 下保持顺序、身份与边界 |
| 3：现有虚拟化组合 | 纵向固定/动态高度示例；大规模卡片按行适配与普通路径对照 | 同一内容的布局一致，锚点/焦点/测量通过；单独报告分配与帧耗时，不以肉眼顺滑代替测试 |

每阶段独立审查和保存 Git 基线，不为仅写文档改动源码。阶段 3 若现有基础层存在真实缺口，先给出失败测试和最小补丁，不把虚拟引擎复制进 List。

### 11.2 布局与交互验收

- Vertical 集合在 Small/Medium/Large 下边界、padding、分隔位置稳定；Horizontal 集合不发生意外压缩或换行。
- Horizontal/Vertical `ListItem` 下，正文收缩与长文本省略正确，leading/extra/actions 不覆盖正文，缺失槽位不产生不一致跳动。
- 单行文字、标题+描述、Leading、Extra、Actions、全部组合与长描述/长操作标签均可布局；多语言与自定义 rem 尺寸无溢出。
- split、striped、bordered、hoverable、selected、disabled 组合顺序一致；容器只有一层边界，不出现重复卡片边框。
- Button、Checkbox 等子控件可以独立聚焦和操作；pointer/keyboard focus 状态真实可见，事件不重复。
- Empty/header/footer 等内容由调用方组合时仍保持同一内容边界和对齐线。
- 浅/深主题及 neutral/accent 主题下验证正文、边界、hover、focus 与选中对比度；小/大 rem 验证尺寸随 token 缩放。
- 普通 List 和 `YueVirtualList` 使用相同 item renderer 时内容/关键布局合同一致；不同 sizing 选择不被 List 静默更改。

### 11.3 示例与测试矩阵

| 示例组 | 重点断言 |
|---|---|
| 纯文本、双行 Meta、Leading、Extra、Actions、文章+封面 | 实测 bounds 不重叠；长文本与缺省槽位自然收缩，Vertical 操作在正文下方 |
| 三种尺寸与表面组合 | token/rem 缩放；首末边界正确；组分隔无双线；selected 与 focus 同时可辨 |
| Header/Footer、空、初始加载、刷新、追加、错误 | body 替换不丢头尾；刷新保留已有内容；请求/重试仅由宿主触发 |
| 横向线性列表 | 不自动折行；有限宽容器下滚动 owner 唯一；分隔方向正确 |
| 固定列/响应式卡片 | 0/1/不足一行/恰好满行/多行；断点前后；末行不拉满、不补假节点；异高行不重叠 |
| 主动作 + Checkbox + 更多按钮 | Tab/Enter/Space 命中真实控件；一次操作只触发一次回调；disabled 真值显式传播 |
| 分组、插入、删除、重排 | ID 稳定；空组策略；逻辑斑马纹不依赖挂载顺序；数据项计数排除组标题 |
| 纵向虚拟与网格按行适配 | Fixed/Measured 内容、宽度变化重新分行、业务锚点、焦点项卸载与尺寸反馈 |

规则计算使用小型纯测试；无头 GPUI 测试读取实际 bounds、焦点和可访问节点；Showcase 实窗检查浅/深主题、长中英文、键盘、滚动与当前设备 DPI。无头节点断言不代替 Narrator 实测，也不把单显示器结果写成跨显示器已验收。已执行的范围见第 14 节。

### 后续能力准入

Sticky group、自动加载更多或拖拽排序需另建消费者工作流和失败测试；不以参考库有该 API 作为充分理由。基础加载/错误/分页组合示例在前述阶段提供，不为示例新增业务状态机。

虚拟路径测试需要稳定 domain key、重排/删除、焦点项卸载、Measured 尺寸反馈及阅读锚点合同。平台和设备相关的真实验收范围按虚拟滚动文档单独记录。

## 12. 目录与接入

已新增 `crates/yue-component/src/list/mod.rs`，承载公共 builder 和轻量模型；`element.rs` 承载集合、Item/Meta 的布局与表面。公共组合测试位于 `crates/yue-component/tests/list.rs`，Showcase 提供 List 页。普通 List 不创建业务状态；按行虚拟适配显式保留 ListGridState，不需要新 behavior 或自定义 painter。

公共类型从 crate 根导出；List 不引入新 Root、独立主题或初始化会话。普通 List 和现有 Select/VirtualList 并行接入，不要求重写它们的状态与语义。README 与 Showcase 已增加基础 List 入口。

## 13. 参考源码映射

- 本项目虚拟化范围与共享 owner：[`28-virtual-list.md`](28-virtual-list.md)。
- 组件层与 Base 复用约束：[`24-component-architecture.md`](24-component-architecture.md)。
- 既有 Yue 组件设计：[`Button`](02-button.md)、[`Divider`](14-divider.md)、[`Tag`](19-tag.md)、[`Select`](20-select.md)。
- 现有实现：[`公共导出`](../crates/yue-component/src/lib.rs)、[`YueVirtualList`](../crates/yue-component/src/virtual_list.rs)、[`Loading`](../crates/yue-component/src/loading/mod.rs)、[`Divider`](../crates/yue-component/src/divider/mod.rs)、[`布局适配`](../crates/yue-component/src/layout.rs)、[`Base 虚拟化`](../vendor/gpui-base/src/virtualization/mod.rs)。
- 三份参考均作为布局覆盖范围的输入；最终行为以本项目 GPUI 生命周期、现有组件 API 与消费者合同为准。

## 14. 基础实现记录（2026-11-06）

本轮完成第 11 节的阶段 1，并提供一维虚拟组合示例；未实现阶段 2 卡片网格或阶段 3 按行虚拟网格。这是阶段 1 的历史记录；后续实现见第 15 节。

- 已实现纵/横集合、Horizontal/Vertical 条目、任意 leading/content/body/extra/actions 槽、Meta、三种尺寸、边框/分隔/斑马纹/hover/受控 selected 和 disabled 呈现。
- 已实现 header/footer/empty 与普通分组，空组不挂载；无请求、分页、选择或滚动私有状态。
- Showcase 包含配置预览、三档尺寸、普通分组、文章与右侧封面、内容/空/加载/错误、追加内容、横向短集合，以及一万条 Measured 虚拟内容。
- 使用现有 YueCheckbox、YueButton、YueTag、YueSwitch、YueSegmented 和 YueSkeleton；虚拟组合直接使用 YueVirtualList，未修改 GPUI/Base 源码或新增依赖。
- 公共测试覆盖实测槽位边界、自然换行与尺寸覆盖、空与分组、横向排布、跨组/重排斑马纹、独立子控件事件/禁用/Tab 顺序、重排焦点、四个主题配置及 12/16/20 rem，以及一万条虚拟路径的布局一致性、语义去重和远端跳转。

| 验证 | 结果 |
|---|---|
| `cargo test -p yue-component --locked --features headless-tests --test list` | 10 通过 |
| `cargo test -p windows-showcase --locked --features headless-tests pages::list::tests::` | 1 通过，配置/选择/动作/禁用回写 |
| Showcase `sidebar::tests::` | 3 通过，包含 List 页面导航 |
| `cargo test -p yue-component --locked --doc` | 36 通过，包含新的 List 公共用法 |
| 两个相关包 all-targets/headless Clippy，`-D warnings` | 通过 |
| workspace 格式、Showcase check/release 构建、组件 no-default-features check | 通过 |
| Windows release 实窗 | 已检查默认/竖排预览、右侧附加信息、复选框与选中表面、分组斑马纹、文章正文/右侧封面/底部操作；当前设备浅色主题 |

实窗使用独立 release 验收实例；原有 debug Showcase 正在运行，未关闭或替换。验证不包含性能前后采样，不能据此声称帧耗时或分配已优化。

固定头尾的专用滚动组合、刷新期间保留内容的加载区域，以及跨 DPI/多显示器和 Narrator 均不在本轮新增验收结论中。普通 Item 不包含整行激活 API；主动作通过真实 Button 等控件组合。

## 15. 阶段 2/3 实现记录（2026-11-06）

规则卡片网格和按行虚拟化组合已实现：

- `ListGrid::columns` 与 `ListGrid::min_width` 是互斥配置，依据真实嵌套容器内容宽度和当前 rem 解析列数；非法列数、尺寸、间距、容器宽度和 Measured 估值返回 `VirtualizerError::InvalidGeometry`。
- 普通 `YueList::grid` 复用 GPUI 原生 grid tracks。数据按行优先排列，末行保留完整列宽，不生成虚假卡片；Grid 不使用连续 split/striped。
- `ListGridState`/`YueVirtualListGrid` 将一维 Base item 映射为整行卡片，复用 Fixed/Known/Measured 和唯一纵向 scroll owner。行高由本行自然内容最大值决定，业务 card key 保留在行内。
- 宽度、列数、rem、内容高度、插入、删除和重排会重新计算行映射；阅读位置按业务 key 恢复。宿主可注册真实 FocusHandle，远端焦点使用 Base retained key，删除焦点项按 fallback 恢复。
- Grid 项声明完整 `aria-posinset`/`aria-setsize`，不升级为 UIA Grid；卡片仍是内容列表。
- Base 新增的极小维护补丁只区分 Owned Measured 几何重映射与实际用户滚动，避免重新测量时误清阅读锚点。补丁及移除条件记录在 `vendor/gpui-base/YUE-PATCH.md`。

阶段测试覆盖：非法配置公式、真实 ordinary/virtual bounds、窄/宽容器、固定列、rem、末行、变高、Measured 反馈、业务锚点、插删重排、焦点/远端保留、删除 fallback 和控件事件。性能采样脚本为 ignored 测试，输出无头布局 CPU、分配和挂载卡片数量；它不冒充 GPU 帧率。原始 Release 结果见 [性能探针记录](research/2026-11-06-list-grid-performance.md)。

验证：`list_grid` 10 项（另 1 个手工性能探针）、List 10 项、VirtualList 191 项（11 ignored）及 Showcase List 2 项通过。普通网格与虚拟网格复用同一 `YueListItem` renderer；阶段 2/3 已完成，后续只保留基于真实 Release 采样的数据优化。

### 15.1 使用方式与边界

```rust
use gpui_kit::{px, rems};
use yue_component::{ListGrid, YueList, YueListItem};

let ordinary = YueList::new("projects")
    .grid(ListGrid::min_width(rems(14.), rems(1.)).unwrap())
    .children((0..7).map(|ix| YueListItem::new(format!("project-{ix}")).content("项目")));
```

虚拟集合由宿主在初始化时构造一次 `Entity<ListGridState>`：

```rust,ignore
let cards = cx.new(|cx| ListGridState::new(
    source, // Rc<dyn VirtualizerSource>，不可变、业务 key 唯一且可反查
    ListGrid::min_width(rems(14.), rems(1.)).unwrap(),
    ListGridSizing::measured(px(180.)),
    cx,
).unwrap());
// render 内只组合，不重建 controller。父级必须提供有限高度。
YueVirtualListGrid::new("project-cards", &cards, |index, key, _, _| {
    YueListItem::new(key.clone()).content(format!("项目 {}", index + 1))
});
```

`ListGridSizing::fixed` 指完整卡片外壳高度；`known` 接收按数据顺序排列的卡片高度，行内取最大值；`measured` 的参数是未测行的估值，真实行高来自原生布局。Fixed/Known 内容必须适配所声明高度，组件不裁切任意槽位。切换 sizing 类型需要重建 controller，与 Base 一致。

`ListGridState::replace_source` 同步不可变数据索引和 sizing；`set_grid` 更新列规则；`invalidate` 用于相同 key 下的主题/密度/异步尺寸变化。宽度、列数和 rem 变化由呈现自动捕获。`virtualizer()` 提供唯一 Base owner 的命令/快照入口；不能绕过适配器替换其 source。普通网格分组独占一整行，组内单独按相同规则排列；header/footer 位于网格之外。虚拟卡片源不混入组标题假数据。

真实 bounds 在布局后捕获，响应式初次挂载/resize 在下一帧收敛，不承诺同帧自动列数。只在宽度或 rem 改变时更新，不产生空转通知。列数变化需要重分行，Measured 当前会失效全部行测量；普通滚动只构建工作集，重排不是常数时间操作。

需要跨行编辑/焦点保留的调用方，必须按业务 key 保留 Input 等子实体和 FocusHandle；通过 `set_focus_handle(key, Some(handle))` 注册卡片焦点作用域。仅靠临时 keyed 控件的私有状态不能保证跨父路径保留。每张卡片可注册一个包含全部子控件的 focus scope；被删除的焦点项使用 `set_focus_fallback`，没有 fallback 时 blur。阅读项删除时按下一个存活业务项、再前一个业务项定位，靠近尾端仍遵循原生滚动边界。

无头测试验证真实 GPUI bounds、真实子控件事件和焦点；它不代表 Windows Narrator 或跨显示器验收。

### 15.2 本轮验证范围

List 10 项、ListGrid 10 项、VirtualList 191 项（11 个手工探针默认忽略）、Showcase List 2 项与导航 3 项、36 个 doctest、相关 all-targets Clippy（`-D warnings`）、组件 no-default-features 检查、Showcase Release 构建通过。ListGrid 的 Release 性能探针已显式运行。

当前设备浅色实窗检查了规则网格五列、7 项末行、普通/虚拟条目自然高度与窄容器一列重排。发现的两个示例问题已修复：两个开关增加可见文本；横向短集合默认内容按自然单行宽度排列，显式卡片宽度仍允许换行。新增 Showcase 实测文本高度回归测试。最终实窗复验被用户物理 Esc 中止，已停止 UI 自动化；不将最后修正声称为再次实窗验收。前一个独立验收实例已正常关闭。

本任务 Rust 文件格式检查通过。workspace `cargo fmt --all -- --check` 仍报告另一任务的 `image_preview/mod.rs` 模块排序；未改动该文件。跨显示器/DPI、Narrator 和真实 Windows GPU 帧采样不在新增验收范围内。

### 15.3 示例页滚动修复

整页滚动测试发现，连续集合/分组外壳及示例 section 的多层纵向 Flex 会重复计算自然尺寸。它们已改为普通流布局；横向集合、条目槽位和卡片 Grid 仍按原布局职责处理。两个内嵌虚拟示例隔离滚轮冒泡，避免同一个事件同时移动内外层。

1200×800px 完整页面的 24 个滚动位置，CPU 绘制中位数 Debug 从 112.602ms 降至 34.008ms，Release 从 11.511ms 降至 3.953ms。新增测试验证外层滚动几何和内层滚动 owner；数据及测量边界见 [修复记录](research/2026-11-06-list-page-scroll.md)。这是无头 CPU 结果，未新增真实窗口/GPU 验收结论。
