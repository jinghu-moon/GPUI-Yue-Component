# YuePagination 分页组件设计

- 日期：2026-11-09；状态：V1 已实现；自动化验证见[实施记录](research/2026-11-09-pagination-implementation.md)，Windows 真机视觉待用户验收。
- 平台：Windows GPUI；遵循[组件架构](24-component-architecture.md)。
- 参考：[Arco Pagination](assets/pagination-arco-reference.png)、[TDesign Pagination](assets/pagination-tdesign-reference.png)。参考图作为能力与布局依据，不照搬 Web 属性、四档尺寸或 CSS 接口。

## 1. 用途与边界

分页用于已知总条数的列表、卡片集合和服务端查询结果。核心任务是查看当前位置、切换目标页、修改每页数量。它不持有业务数据、不发网络请求、不自动滚动列表，也不取代虚拟滚动。分页后的单页仍可使用虚拟列表。

提供 `YuePagination` 和 `YuePaginationState`。State 保留已提交分页值及输入／Select 会话；使用者订阅变更请求后显式提交或拒绝。外部 setter 不发变更事件。禁止同时出现一份组件内部乐观页码和另一份宿主页码。

普通页码、简洁模式、总数／范围文案、每页数量、输入跳页、首末页、快速省略号、三档尺寸、描边／无框、禁用与加载纳入 V1。未知总数、游标分页、独立竖向 Mini 子组件、无限加载、任意 item renderer 与全局快捷键不纳入本次。

## 2. 两份参考的吸收

| 参考能力 | Yue 决策 |
| --- | --- |
| 基本、更多页码、首末页与上一页／下一页 | 基本导航常驻；首末独立按钮可选，端点状态准确。页码窗口复用 Base。 |
| 省略号附近页码数量 | `visible_pages(5..=15)` 配置有界窗口；采用 Base 合同，端点与省略号不冒充完整数字页。 |
| 可点击省略号 | 默认真实 Button，标明向前／后跳转目标，键盘可达；可改为静态省略号。 |
| 每页条数 Select | 复用 YueSelect；原子请求同时带 page 与 page_size，不分别发两次事件。 |
| 跳转与简洁分页 | 复用 YueInput，Enter 确认；不在每个字符或失焦时翻页。简洁模式默认显示当前／总页数，可启用编辑。 |
| 总条数与范围 | 默认 `共 N 条`，允许格式化 `第 A–B 条，共 N 条`；零数据范围为 0–0。 |
| 四档尺寸 | 统一项目 Small／Medium／Large，Button、Input、Select 同步。 |
| 默认／描边、受控、隐藏单页 | 类型化选项；不复制 defaultCurrent 与 current 双状态。 |
| 任意 CSS／图标／页码 renderer | 保留语义一致的组件；前后页支持文字标签，其余通过现有主题与布局合同组合。 |
| PaginationMini 的上下布局 | 不单列组件；与数据视图不同的记录逐条切换可后续按真实需求设计。 |

## 3. 已有能力与职责

本地 `gpui-kit 0.8.0` 的 Base 已提供 `pagination::PaginationState`（值式模型），包含页码校正、端点、页码／省略范围和受控请求 guards，以及 `Pagination` 导航 landmark。直接复用，不复制一套可见页码算法。

| 所有者 | 职责 |
| --- | --- |
| Base PaginationState／Pagination | 当前页边界、可见页码计划、同页／越界／禁用保护、Navigation 语义。 |
| YuePaginationState | 有效分页快照、输入草稿、Select 与订阅生命周期、请求事件、显示控件的禁用同步。 |
| YuePagination | token 外观、区域布局、Button／Input／Select 组合、稳定身份与焦点衔接。 |
| YueButton／Base Button | 鼠标、Enter／Space、禁用、焦点、hover／pressed 与焦点环。 |
| YueInput／InputState | 原生编辑、IME、光标、剪贴板与选区。整数校验在提交边界。 |
| YueSelect | 单选每页数量，键盘、弹层、关闭与焦点返回。 |
| 宿主 | 查询参数、请求代次、加载／失败／重试、业务数据、是否接受请求及滚动复位。 |

跳页不使用 NumberInput 的 f64 数值通道：分页使用 `usize` 精确整数，Windows 64 位超过 2^53 的页码不能经浮点中转。复用 YueInput 已有编辑器，不自绘数字框，也不新增数值输入基础层。无新的 Cargo 依赖。键盘验收发现 GPUI 对同一 FocusId 的重复登记会导致反向 Tab 原地停留，已添加最小去重补丁，见 [GPUI 补丁记录](../vendor/gpui-pre/YUE-PATCH.md)。

## 4. 分页数学与数据变化

`PaginationValue::new(total, page, page_size)` 校验 page_size > 0，当前页归一化至 `[1, max(1, ceil(total/page_size))]`；total=0 时保留当前页 1、总页数 1，所有翻页不可执行，显示范围 0–0。page=0 归一化为 1。页码使用一基索引；`item_range()` 返回零基半开区间用于切片。

总页数用除法与余数计算，禁止 `total + page_size - 1` 溢出。范围上界使用饱和加法后与 total 取小值，覆盖 `usize::MAX`。

每页数量变化默认回第一页 `Reset`。可选 `KeepFirstItem`：保持原页首条数据位置，`new_page = old_start / new_size + 1` 后归一化。事件载荷包含完整目标快照，不允许宿主先处理页码再处理 page_size。

总数、过滤条件、page_size 和页码均通过 `set_value(...)` 原子更新。收缩总数会静默校正当前页，并同步编辑草稿与 Select，不通过 render 触发请求。宿主希望重新查询校正页时应在数据流程中显式执行，不能依赖组件制造循环事件。

每页选项默认 10／20／50／100，必须为非空、正、无重复的有界集合（最多 32 项）。当前 page_size 不在选项时，Select 临时补入实际值；不伪造当前数量。外部设置与宿主拒绝请求均保证 Select 回到提交值。

## 5. 输入、请求和键盘

`PaginationChange` 带 `before()`、`value()`、`source()`；source 区分页码、前后页、首末页、省略跳转、页数输入与每页数量。一次有效输入只产生一个请求；同页、边界、禁用或加载为零次。用户选择后不自行改写 committed value，宿主通过 `set_value` 接受。

跳页只接收去首尾空白后的 ASCII 十进制正整数；小数、负数、指数、空白串、中文数字及 usize 溢出不提交。范围外正整数在确认时钳制至有效页。未确认草稿不影响真实页码。

- Enter：确认一次；非法值保留草稿并显示邻近错误，不翻页。
- Escape／失焦：撤销草稿回到提交页，不提交；Escape 仅在跳页框实际聚焦时局部处理。
- IME：复用 Base 编辑器提交顺序，正在组词时不抢确认键；不在容器全局拦截 Enter。
- Tab／Shift+Tab：沿视觉顺序访问实际按钮、Select 和输入；页码没有额外根 Tab stop。跳页框局部将 Tab／Shift+Tab 接至 Base Root 的焦点遍历，避免 Base Input 默认缩进；组合输入期间不抢 Tab／Escape。不增加与文本编辑冲突的左右方向键全局翻页。
- 页码窗口变化导致已聚焦数字消失时，将焦点移到新的当前数字；不把焦点抢出分页或正文。省略按钮、前后按钮身份保持稳定。
- 加载期间禁用整个分页及 Select／输入；加载状态在邻近文案表示。数据错误及重试属于宿主，不能把失败显示成 0 条。

当前页用选中表面与可访问名称 `第 N 页，当前页` 表达，避免把 Navigation 伪装成 TabList 或 RadioGroup。没有额外伪造的 UIA Current 属性。

## 6. 布局、主题和动效

区域阅读顺序为总数、每页数量、页码导航、跳转。总数与每页数量都是可选，不留空槽；前后页不可因缺省数字窗口被省略。简洁模式显示 `上一页 · 当前 / 总页数 · 下一页`，跳页开启时将当前数字替换为编辑器。

分页按实际父容器宽度自然换行，各组保持内部关系；数字按钮允许换行，辅助组转到下一行，不在窄窗静默丢失操作或切换模式。特别窄的容器应由宿主显式改为 Simple；不存在基于全屏尺寸的伪响应式。

三档尺寸沿用 Button／Input／Select 的既有高度与字体，不新增第四档；页码 min-width 随按钮高度，长数字保持自然宽度。当前页、hover 和 loading 不改变按钮几何；焦点环不裁剪。根不拥有列表滚动，宿主调整数据后决定保留还是复位滚动。

`PaginationVariant::{Ghost, Outline}` 控制非当前页按钮的边界。当前页使用现有选中 Button 表面；不添加一侧标记线。色彩和 hover／pressed／focus 动效复用 YueButton；不增加页码滑动、数值翻转或常驻循环动画。减少动效沿用现有组件合同。

新增 `--pagination-*` 只表达组合间距、字号、辅助文案色及跳页／Select 宽度；Button、Input、Select 的视觉仍由各自 token 控制。设计系统为唯一来源，同步 inventory 与 Rust；不手改生成代码。局部 theme 显式传给内部组合控件。

## 7. 公共 API 合同

以下签名与实际导出一致，完整可编译用法见 `YuePagination` Rustdoc。

```rust,ignore
PaginationValue::new(total: usize, page: usize, page_size: usize)
    -> Result<PaginationValue, PaginationError>;
// readers: total(), page(), page_size(), page_count(), item_range()

YuePaginationState::new(value: PaginationValue, window, cx) -> Self;
// value(), is_disabled(), is_loading()
// set_value(value, window, cx) -- silent
// set_disabled(bool, window, cx), set_loading(bool, window, cx)
// set_page_sizes(iter, window, cx) -> Result<(), PaginationError>
// set_size_change_policy(PaginationSizeChangePolicy::{Reset, KeepFirstItem})
// EventEmitter<PaginationChange>; only requests, host commits explicitly

YuePagination::new(id, &state)
    .theme(theme)
    .size(PaginationSize::{Small, Medium, Large})
    .variant(PaginationVariant::{Ghost, Outline})
    .mode(PaginationMode::{Pages, Simple})
    .show_total(false)
    .total_format(|value| SharedString)
    .page_size_selector(false)
    .show_jumper(false)
    .first_last(false)
    .ellipsis_jump(true)
    .visible_pages(7) // 5..=15, not a total-item allocation
    .hide_on_single_page(false)
    .navigation_labels("上一页", "下一页") // optional visible labels
    .accessibility_label("工单分页");
// shared ControlLayout external width/min/max/margin/grow
```

同一个 state 只挂载一次。多组分页若要同步，宿主持有一份业务查询并将接受值同步到各 state，不能把同一个 Input／Select Entity 同时装到两处。外部更新与配置 setter 都在事件／update 中执行，不在宿主 render 中执行。

错误类型覆盖零 page_size、空／重复／过多选项；fields 私有，公共类型从 crate 根导出。数值计划以纯模型测试验证，组件家族按 `pagination/{mod,model,state,element}.rs` 拆分；无独立 controller 泛化层。

## 8. Showcase 与验收

导航按现有英文排序添加“分页 Pagination”。示例使用真实数据切片，不让按钮只改一个演示数字：

1. 基础、长页码、省略跳转与首末边界。
2. 每页数量、Reset／KeepFirstItem 对照、当前数量不在候选集。
3. 跳页、非法输入、简洁只读／可编辑、描边与三档尺寸。
4. 总数／范围、自定义前后文字、隐藏单页、零数据。
5. 禁用与异步加载、宿主拒绝、数据收缩、窄容器。
6. 工单 List 与 Card 网格的真实分页；取消旧请求和过期响应由宿主实现。

自动化检查：纯整数边界／巨量数据、Base 窗口复用、鼠标／Enter／Space 一次请求、拒绝后输入／Select 回退、实际编辑／Escape／Blur／Enter、禁用／加载、每页数量原子事件、稳定焦点、四主题与三 rem、窄窗换行及真实 List 切片。不得用直接调用内部方法代替所有交互测试。

交付前运行专项与受影响回归、严格 Clippy、Showcase 构建、生成映射检查和文档链接检查。保留工作区既有格式／全量 token 审计失败的事实，不扩散无关格式化。Windows 视觉、Narrator 与系统 DPI 由真机验收补充，应用 rem 布局测试不能替代它们。
