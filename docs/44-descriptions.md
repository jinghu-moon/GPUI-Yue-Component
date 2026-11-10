# YueDescriptions 描述列表设计

- 日期：2026-11-10；状态：已实现，真机视觉与读屏待用户验收。
- 平台：Windows GPUI；遵循[组件架构](24-component-architecture.md)。
- 参考：用户提供的 `refer/gpui-descriptions-demo.html`，已保存[可独立打开的参考副本](prototypes/descriptions.html)；[结构截图](assets/descriptions-anatomy-reference.png)、[换行后同排对齐截图](assets/descriptions-alignment-reference.png)。
- 公共 API 已按本文落地；实际验证范围、底层修正与真机待验收项见[实施记录](research/2026-11-10-descriptions-implementation.md)。

## 1. 定位与范围

YueDescriptions 以“标签—内容”对展示一条记录的属性，适用于工单详情、设备参数、账户资料、订单摘要。外观可以有边框并形成网格，但组件属于详情展示层，不是基础层，也不依赖未来的 YueDataTable。

DataTable 负责多记录的行列交互；Descriptions 负责属性配对、标签对齐、自适应列数、跨列与自然内容高度。二者只在存在真实公共需求时共享底层布局或单元格机制，不提前建设新的通用表格引擎。

首版包含：

- 可选标题和 extra；稳定 key 的描述项；文字与任意内容插槽。
- 水平标签／上方标签，最多列数与容器自适应降列，固定列跨度和 Filled。
- 有边框／无边框、三档密度、标签宽度、标签／内容水平对齐、单元格垂直对齐、可选冒号。
- 默认自然换行，单行及 N 行截断、实际溢出时的完整提示；文本复制和明确的复制载荷。
- 与 List、Card、Avatar、Badge、Link、Progress、Button 及已有表单控件组合。

排序、筛选、行选择、分页、虚拟化、列拖动、业务编辑状态、数据加载和保存服务不进入组件。组件不自动将空数据变成 Empty，不因加入输入框就成为 Form。

## 2. 参考吸收与修正

| demo 能力 | 本项目合同 |
| --- | --- |
| `column` + `minItemWidth`，ResizeObserver 降列 | 以实际内容容器宽度计算，不能使用窗口宽度猜测；rem 改变后重新解析尺寸。 |
| 数字 span／filled；有边框的每行末项补齐 | 保留顺序装箱规则，明确超长跨度、换行和无边框不自动补齐的差别。 |
| 左／上布局、三密度、标签对齐、冒号 | 保留；另明确内容水平对齐与整格垂直对齐，避免把标签文字对齐和网格列位置混淆。 |
| `.ds-v { align-items:flex-start }` | 修正：同排各项铺满最高行高，再在标签／内容格内部对齐；短文本默认垂直居中。 |
| 文本、头像、状态、链接、进度、复制 | 内容组合现有组件；复制用真实 YueButton 和 GPUI 剪贴板，不从任意子树提取文本。 |
| 单行／N 行截断与完整提示 | 已在共享 OverflowTooltip 增加 N 行与条件键盘访问，按真实布局判断溢出。 |
| 列表选择后淡入详情、编辑／保存／取消 | 作为 Showcase 宿主流程；数据、草稿、表单、焦点恢复及内容切换动效由页面持有。 |

参考截图中，长工单编号换为两行后，同排状态、负责人及标签仍贴在顶部。这是要修正的对齐缺陷，不作为复刻目标。

## 3. 复用清单与依赖边界

| 已有能力 | 使用方式与限制 |
| --- | --- |
| GPUI grid／flex、文本布局 | 构造平铺的稳定 item 树；按计划设置列位置／跨度。行高交给布局取最大自然高度，避免手工测量每格高度。 |
| `ListGrid` 的列数／宽度几何 | 复用已有公式或在确有两个消费者时提取最小纯几何函数；当前 ListGrid 是固定列数或最小宽度二选一，不支持最大列数和 span，不能直接当成完整 Descriptions 布局。 |
| ControlLayout | 外部 width／min_width／max_width／margin／grow；默认填满可用宽度并允许收缩。 |
| YueOverflowTooltip／Tooltip controller | 共用溢出提示、窗口浮层、定位、延迟、焦点触发及生命周期；按第 8 节补齐多行与条件 Tab stop。 |
| YueButton、Tabler Icon、GPUI Clipboard | 复制操作、Copy／Check 反馈；复用按钮焦点、Enter／Space 和禁用行为，不另绘伪按钮。 |
| Typography、YueTheme、tokens、motion | 排版、视觉和交互反馈；新增 token 先进入设计系统，再同步生成。 |
| YueCard／Header、YueList、YueResizable | Showcase 的详情表面、主从选择与窄容器；Descriptions 本身不自动套 Card、不带独立滚动所有者。 |
| Avatar／Badge／Tag／Link／Progress | 直接作为内容插槽。图标和控件用自身公开尺寸，不给各个使用方添加像素偏移。 |
| Input／Select／DatePicker／NumberInput／Textarea、Field／Form | 宿主编辑流程中的值槽与校验，Entity 保留在宿主。Descriptions 不重复实现编辑器、校验或提交。 |

GPUI-KIT 0.8.0 提供的是 `gpui_kit::component::description_list` 带样式组件；当前仓库 Base 没有对应 DescriptionList 导出。遵守 Base-only 约束，不启用 Kit styled component／默认 assets。未新增 Cargo 依赖。实施测试发现 GPUI 的多行布局缓存缺少行数上限身份，已作最小修正，见[补丁记录](../vendor/gpui-pre/YUE-PATCH.md)。

`YueCardHeader` 是 Card 接收的配置类型，并不是可直接嵌入的独立元素。Descriptions 自有轻量标题区使用既有排版与 extra 插槽；外层 Card 已有标题时不重复设置 Descriptions 标题。禁止为“复用”虚构 Header 渲染 API。

## 4. 公共 API 与状态归属

组件采用 RenderOnce；不公开 DescriptionsState，不复制业务数据。内部只保留按 key 标识的测量失效信息、溢出／复制焦点和短暂复制反馈。所有公共结构体字段私有，通过 builder 和必要 reader 暴露。

```rust,ignore
YueDescriptions::new("work-order-details")
    .title("工单信息")
    .extra(YueButton::new("edit-order", "编辑"))
    .columns(3)
    .min_item_width(rems(13.75))
    .layout(DescriptionsLayout::Horizontal)
    .bordered(true)
    .size(DescriptionsSize::Medium)
    .label_width(rems(7.0))
    .label_align(DescriptionsAlign::Left)
    .value_align(DescriptionsAlign::Left)
    .vertical_align(DescriptionsVerticalAlign::Center)
    .colon(false)
    .accessibility_label("工单详细信息")
    .items([
        YueDescriptionItem::new("number", "工单编号")
            .text("WO-20261110").copyable(true),
        YueDescriptionItem::new("status", "状态")
            .content(status_badge),
        YueDescriptionItem::new("owner", "负责人")
            .content(owner_avatar_and_name),
        YueDescriptionItem::new("note", "备注")
            .text(note)
            .span(DescriptionsSpan::Filled)
            .text_overflow(DescriptionsTextOverflow::Lines(2)),
    ]);
```

以上类型已从 crate 根导出；crate 内的最小用法由 Rustdoc 编译验证。

| API | 默认／合同 |
| --- | --- |
| `YueDescriptions::new(id)` | 稳定根身份；默认无标题、无 extra、无边框、水平标签、中密度。 |
| `items(iter)` | 替换描述项；空集合合法。不设置 hide 状态，宿主过滤后传入，装箱只处理可见项。 |
| `columns(u16)` | 最多列数，默认 3；不能为 0，不表示强制固定列数。 |
| `min_item_width(AbsoluteLength)` | 默认使用 token，参考基准 220px；有限且大于 0。是“一组标签＋值”的最小目标宽度，不是值区宽度。 |
| `layout(Horizontal / Vertical)` | Horizontal：标签在左；Vertical：标签在上。不表示整个集合排列方向。 |
| `label_width(AbsoluteLength)` | 参考默认 112px，水平布局生效，包含标签内部 padding；不得为负、NaN 或无穷。Vertical 保留配置但不使用固定宽度。 |
| `label_align / value_align(Left / Center / Right)` | 默认 Left；分别控制格内内容，不移动整组属性的列位置。 |
| `vertical_align(Top / Center / Bottom)` | 默认 Center；对齐合同见第 7 节。 |
| `size(Small / Medium / Large)` | 字号、行高、边框内 padding 和无边框间距均随密度变化。 |
| `title(text) / extra(element)` | 均可单独存在；都未设置时不保留头部空白。长标题换行；空间不足时 extra 整体换到下一行，保持可操作。 |
| `theme / accessibility_label` | 跟随默认主题，可局部覆盖；名称默认“描述列表”，显式名称不能空白。 |
| ControlLayout 外部布局 builder | 内部视觉仍由 token 拥有；不新增任意 Styled 穿透 API。 |

`YueDescriptionItem`：

- `new(key, label)`：非空白 key 和文本标签；key 在组内唯一，不能用可变 index 或翻译后的标签作为身份。
- `text(value)`／`content(element)`：互斥的值表示，后设置者生效；默认空文本，空字符串合法且不自动替换为破折号。
- `label_content(element)`：可选的自定义标签外观；new 的文本标签仍为可访问名称，不从子树推测名称。
- `span(DescriptionsSpan::{Columns(u16), Filled})`：默认 Columns(1)，0 非法；超过当前列数按当前列数收缩，不修改原始配置。
- `text_overflow(DescriptionsTextOverflow::{Wrap, Ellipsis, Lines(usize)})`：默认 Wrap；Lines(0) 非法。非 Wrap 只用于 text 值，若最终值为 content 则在渲染验证时明确报错，避免配置悄然无效。插槽自己组合 OverflowTooltip。
- `copyable(bool)`：默认 false。text 值默认复制完整原文；content 没有隐含可复制字符串。
- `copy_text(text)`：设置明确复制载荷并开启复制；可以为自定义插槽提供文本，不读取其绘制文本。content 开启复制但没有 copy_text 是配置错误。
- `vertical_align(...)`：可覆盖整组设置，让编辑器／复杂插槽需要时顶部对齐。
- `key()`／`label()` 提供必要的只读身份信息；不向外暴露 AnyElement 内部字段。

重复 key、空白标签、非法数字、无效溢出配置采用与现有 Steps／Card 一致的明确断言。尺寸单位允许 rem／px；默认 token 用 rem 适配应用缩放。

## 5. 自适应列数与装箱

令 W 为排除外围边框后实际可用的网格宽度，M 为解析后的最小项宽，G 为水平间隔，Cmax 为最多列数：

```text
C = max(1, min(Cmax, floor((W + G) / (M + G))))
T = max(0, (W - (C - 1) × G) / C)
跨度为 s 的项宽 = s × T + (s - 1) × G
```

无边框的 G 来自列间距 token；有边框的 G 为唯一内部边框厚度。计算使用有限浮点中间值，先夹紧再转换为列数，避免超宽容器转为 u16 时溢出。W 为 0 的首次不可见布局仍合法，不能用临时宽度写坏业务状态。

按传入顺序装箱，不为了填空重排项目：

1. 数字跨度先 clamp 到 1..C；当前行放不下时，从下一行开始。
2. Filled 消费当前行剩余列；新行时占整行。
3. 每次结束一行，有边框时扩展该行最后一项到余下所有列；无边框保留空白。
4. 重新计算列数时始终使用原始跨度，不能把上次自动补齐的跨度写回 item。

例：3 列下 A(1)、B(1)、C(2)。C 放不下而换行；有边框时第一行 B 扩至 2 列，第二行 C 扩至 3 列；无边框时第一行尾留 1 列，第二行尾留 1 列。

采用一个网格根下平铺的 keyed item 树，装箱计划只更新有效 span，native grid 按顺序确定 row／column。不要把项重新挂到按行 index 命名的父节点下；否则容器降列时，稳定 key 仍会因祖先路径变化而重建插槽、丢失输入或焦点。

布局计算必须依据本次实际有限可用宽度，优先使用 GPUI 正常布局阶段；确需元素测量时使用受控的布局失效机制。不能靠每次 render／prepaint notify 循环逼近宽度，也不能永久使用上一帧宽度。当前 GPUI 没有内容可反向决定容器自然高度的 container query；实施采用受控的离散失效：捕获布局后的网格内容宽度，仅在列数／极窄模式改变时 defer 一次通知。跨阈值最多下一帧更新离散布局；原生 grid 的列宽始终使用本帧可用宽度。稳定时不通知，不缓存上一帧像素列宽。布局稳定后不申请新帧。

## 6. 标签、边框与极窄容器

Horizontal 的标签列固定目标宽度，值列 min-width 为 0 并消费剩余空间；标签和值都允许自然增高。标签在同一设置下对齐，跨列只拓宽值区域，不把标签按跨度放大。Vertical 中标签在上，标签宽度随格宽；标签和值的分隔变为水平线。

标签格、值格及网格根不铺背景色，直接透出宿主表面；Avatar、Badge 等组合组件保留自己的背景。外围边框由网格拥有；项的右侧／底部分隔线独立绘制在预留的单线间隙中，标签格拥有标签／值分隔，避免双倍线宽。外层圆角不应以全组 overflow_hidden 裁掉 Input、Button 的焦点环和浮层。

当降至 1 列后仍不足以容纳目标标签宽度、值区最低可用宽度及 padding，Horizontal 在该极窄布局中退化为标签在上的堆叠单元。这是确定的防溢出规则，不修改宿主 layout 配置；宽度恢复后复原。最低可用值宽来自 token，不能继续挤压到 0 或让长标签覆盖值。

根本身不创建横向／纵向滚动区域，跟随内容自然增高；页面已有滚动所有者继续负责滚动。任意插槽若自行设置不可收缩的固定宽度，须由该插槽提供自身换行／滚动策略，Descriptions 不缩放其绘制结果。

## 7. 重点：换行后同排水平与垂直对齐

这是用户截图明确提出的必验收项。只给内容行加 `items_center` 或给文字加 `text_center` 都不能完整解决。

布局分成两层：

1. 网格行高由本行最高项的自然高度确定。每个属性项 stretch 到整行高度；其中标签格、值格及其边界同样 stretch，不能让它们各自只保留一行文字高。
2. 每格内另设内容容器，按 vertical_align 对齐实际内容；水平按 label_align／value_align 独立对齐。文字使用 text alignment，Avatar＋姓名、Badge、复制等组合使用内部 flex 对齐，不能把整列挪动来冒充文字对齐。

| 场景 | 必须满足 |
| --- | --- |
| 工单编号变为 2／3 行，其余格为单行 | 同排标签、状态、负责人等短内容默认垂直居中，不能仍贴在原行顶。 |
| label_align Left／Right／Center | 各自维持水平设置，行变高不改变水平位置；不能为垂直居中强制把左对齐标签改为水平居中。 |
| label_align Center + value_align Center + vertical_align Center | 标签和内容各自在本格内水平、垂直同时居中。 |
| 同排既有长标签，又有长值 | 取各项最大自然高度，不裁行、不覆盖相邻项，所有单元格与分隔线等高。 |
| Avatar＋姓名、Badge、进度、链接＋图标 | 组合体整体在格内居中，内部沿用对应组件的行内对齐，不为每种插槽手填不同偏移。 |
| 文本＋复制按钮 | 两者属于同一内容组合；按钮默认对齐文字块中心，长值换行后不单独贴顶。先预留按钮及 gap 宽度，剩余宽度用于文本换行，不允许按钮覆盖文字。 |
| item 覆盖为 Top／Bottom | 只改变该项标签和值的内容对齐，不破坏其单元格 stretch 和整排高度。 |
| Vertical 标签在上 | 标签带与内容带保持上下顺序，标签带留在上方；剩余行高由值格消费，vertical_align 控制值格中的内容，不能把标签和值叠放或一起挤到中部。 |

不要通过固定行高、添加特定 px 的 top／translate、逐项高度测量或图片内容特判解决此问题。单行和多行的行高、上下 padding 来自同一组 token，行增高由正常布局决定。

## 8. 长文本、提示与复制

默认 Wrap：正文、标签与连续编号均不得撑破所属格；中文、英文、URL、连续无空格编号都要测。普通文本不得直接作为未受约束 flex 匿名子项；使用可收缩、宽度受限的文本容器，避免重演 [Steps 窄标题问题](research/2026-11-09-steps-implementation.md)。

Ellipsis／Lines(n) 显示完整文本提示的条件是实际截断，不是字符串长度或配置本身：

- YueOverflowTooltip 在 tooltip 模块统一支持可选行数：单行比较 shape_line 自然宽度，多行使用未截断的 shape_text 与实际可用宽度计算换行数；字体／rem／文本／宽度改变后重新判定。
- GPUI 已有 line_clamp，但“画出省略号”不等于“已正确判断需要提示”；不得把多行测量逻辑私藏在 Descriptions，再生成第二套 tooltip controller。
- 只有实际截断的纯文本入口进入 Tab 顺序并显示焦点环；文本完整时移除这个额外 Tab stop。通过 focusable_when_overflow(true) 开启这一能力；track_focus 仍可接收外部控件焦点，默认不新增 Tab stop，保持其他消费者行为。
- 变宽后不再截断：关闭旧提示，移除新增 Tab stop；若焦点正在该文本上，仅在自己拥有焦点时有序转移至邻近真实操作或退出该组，不抢外部焦点。
- 自定义插槽不应用根的 line_clamp，以免裁掉按钮／输入框；由宿主在插槽内部显式组合文本截断。

复制按钮位于值内容组合内但不嵌套到另一个按钮。点击／Enter／Space 复制完整载荷，不能复制视觉上带省略号的文本；装饰图标不重复播报。复制后显示 Check／“已复制”短暂反馈，再恢复 Copy；反馈时长复用既有设计系统语义变量，不引入常驻计时器或全局 Toast 洪泛。

回调／计时任务以根 id＋item key＋内容版本为身份；快速重复复制、值改变、换记录、删除项或卸载后，旧任务不得覆盖新反馈、重开提示或拿走焦点。普通文本和插槽中的链接／按钮保留自身命令，不让点击整项触发复制。

## 9. 语义、键盘与焦点

当前 AccessKit 0.24.1 已有 `Role::DescriptionList`、`Role::Term`、`Role::Definition`，GPUI 直接导出 Role。设计使用根 DescriptionList、标签 Term、值 Definition 表达属性配对，不用 Table／Grid 或 ListBox 假装同样的语义。Windows Narrator 实际映射在实施后验收，不能把角色存在当作读屏已通过。

标题使用已有标题排版；根有明确可访问名称。标签名由显式文本提供，冒号属于外观，不加入重复的语义名称。值按实际文本或子控件语义表达。GPUI 当前没有公开的通用 aria-labelledby builder，不声称任意插槽自动关联其内部输入名称；宿主使用 Field、Input、Select 的公开命名 API。

静态项没有 Tab stop，不实现整组方向键／选中模型。Tab／Shift+Tab 按传入顺序遍历 extra、实际截断文本、复制和插槽中的真实控件；Enter／Space 复用控件自身行为。Input 的 Tab／IME 行为仍遵守[已有输入合同](03-input.md)，不宣称 Descriptions 会拦截并改成表单遍历。

改宽、降列、调整 span 不改变 key 或父身份，也不重新创建宿主 Entity。Text／content 切换由宿主负责编辑焦点；宿主示例必须演示开始编辑时聚焦、保存／取消后回到编辑按钮。组件内部新增的复制／溢出焦点，仅在自身目标移除或变为不可操作且确实拥有焦点时修复。

## 10. 视觉、尺寸与动效

demo 的数值只作视觉参考：默认最小项宽 220px、标签宽 112px；三档字号 12／14／16px、行高 20／22／24px；有边框的纵向 padding 6／10／14px、横向 padding 10／14／18px。实现不在 Rust 旁复制这些裸数，先映射到现有语义尺度；必要时在设计系统定义可推导的组件 token，并在源中说明与参考的差异。

已新增并登记 30 个 `--descriptions-*` token：

| 变量组 | 职责 |
| --- | --- |
| `font-size-{sm,md,lg}`／`line-height-{sm,md,lg}` | 默认文本与行内组合的排版，不用固定行高裁多行。 |
| `padding-x/y-{sm,md,lg}`／`row-gap-*`／`column-gap-*` | 有边框的格内间距和无边框的行列间距；两种外观均响应密度。 |
| `label-width`／`item-min-width`／`value-min-width` | 默认列几何和极窄退化阈值。 |
| `color`／`label-color` | 正文与次要标签文字；单元格透明，不为每个值自动分配状态色。 |
| `border-width`／`border-color`／`radius` | 分隔与外围圆角，单线绘制。 |
| `label-value-gap`／`content-gap`／`header-gap`／`header-margin-bottom` | 标签／内容、图标与按钮组合、标题区的间距。 |

标题优先复用既有 Typography，复制按钮及焦点环沿用 Button，Tooltip 沿用既有 token；不再为这些零件重复定义外观系统。颜色在 light／dark × azure／neutral 四主题验证，普通标签及正文对比度目标至少 4.5。

描述表格自身不在改宽／降列时做布局动画，以保证阅读、命中和编辑焦点稳定；复制反馈沿用共享 Interaction 过渡及 reduced motion。示例中选择新记录后的详情淡入由宿主控制，不能让每次输入引发整个详情重复入场。

## 11. Showcase 与文件组织

已增加 **组件 → 描述列表 Descriptions**，导航按既有英文名称排序。页面包含：

1. 结构示例：标题／extra、两列工单属性、头像、状态、编号复制与 Filled 备注。
2. 布局控制：1–4 最大列数、水平／上方标签、可拖动容器；演示同窗口中的宽 Card 与窄面板分别降列。
3. 跨列矩阵：数字跨度、Filled、越界跨度、提前换行、每行末项补齐；有／无边框对照。
4. 外观控制：三密度、标签宽度、标签／内容水平对齐、Top／Center／Bottom、冒号。
5. 专门的“多行与同排对齐”示例：长工单号＋单行标签／Badge／头像，切换一至三行、密度、居中与顶部对齐；与用户反馈截图直接对应。
6. 长内容：中文段落、连续编号、单行／两行截断、无溢出短文、宽度变化后提示与 Tab stop 更新。
7. 内容插槽：Link、Badge、Avatar、TagGroup、Progress、复制；自定义复杂插槽自行管理内部布局。
8. 工单详情：左侧 YueList 选择，右侧 YueCard 中展示；编辑组合既有输入与 Select／DatePicker 等，保存有校验、取消保留原值，草稿与 Entity 属于宿主。

首版文件按真实职责组织：

```text
crates/yue-component/src/descriptions/
  mod.rs       公共 builder、配置与导出
  model.rs     列数、装箱与跨度纯计划
  element.rs   语义、网格、对齐、插槽组合
  tests.rs     状态边界、真实输入、焦点与几何验收
```

若复制反馈／元素测量形成独立生命周期，再增加有明确职责的私有文件；不预建公开 controller 或虚拟数据源。多行 OverflowTooltip 改动留在 tooltip 模块，并覆盖其他使用方回归。最终从 crate 根导出 Descriptions 类型，prelude 只增加高频入口。

## 12. 分阶段验收

| 阶段 | 必须完成的证据 |
| --- | --- |
| A：纯布局与基本表面 | 列数阈值前／后、0 宽与隐藏恢复、最大列数、跨度重算、Filled、提前换行、无边框留空和有边框补齐；空／单项／重复 key／非法尺寸。 |
| B：真实几何与对齐 | 三尺寸 × 四主题 × 12／16／24px rem；标签／值一到三行；透明单元格和边框等高；Left／Center／Right 与 Top／Center／Bottom 组合；宽高随内容变化。 |
| C：交互与组合 | 复制一次、完整载荷、键盘、反馈过期／卸载、独立链接／按钮；降列／跨度变化保留 Input Entity、内容和焦点；外部主题更新。 |
| D：长内容与提示 | 中英／连续编号不越界；单行／N 行仅溢出触发；变宽取消提示与条件 Tab stop；tooltip 生命周期和所有既有消费者回归。 |
| E：宿主场景与交付 | 主从详情、编辑／保存校验／取消／焦点恢复、窄面板；Rustdoc 用例、token 专项与同步、Clippy、工作区检查、Showcase 构建、文档链接。 |

对齐测试必须观察实际内容容器，不能只检查 item 外框：Center 时短文本／组合体中心与所属格内容区中心误差控制在布局舍入容差内；Top／Bottom 分别比较 padding 后的边。多行另一项使整行增高后再次检查，不能只测等高单行。

增加防回归断言：实际文本框不得越出所属值格或与下一项重叠；复制按钮仍在格内；单元格不铺色且分隔线覆盖到同行底部；相邻边框不出现 2 倍线宽；降列后的旧跨度和旧测量不继续生效。稳定界面不重复通知或持续申请动画帧。

已交付 Rust 组件、Showcase 页面、30 个源 token、同步产物和自动化验收，详见[实施记录](research/2026-11-10-descriptions-implementation.md)。Windows GPU 视觉、Narrator、系统 DPI／高对比度仍需用户真机确认。
