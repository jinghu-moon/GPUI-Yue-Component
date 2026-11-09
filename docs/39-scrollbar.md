# YueScrollbar 滚动条设计

- 状态：GPUI 组件与 windows-showcase 现有滚动条入口已实现统一迁移；原生实窗观感由用户验收。
- 日期：2026-11-09。
- 平台：Windows / GPUI；适配项目已有浅色、深色主题与窗口 rem。
- 目标：为普通滚动容器、虚拟列表及组合组件提供统一的悬浮滑块，复用既有滚动行为和唯一滚动 owner。
- 原型：[Scrollbar 交互 Demo](prototypes/scrollbar.html)，可离线打开。
- 关联：[组件架构](24-component-architecture.md)、[虚拟滚动](28-virtual-list.md)、[List](29-list.md)、[Tree](37-tree.md)、[TreeSelect](38-tree-select.md)。

## 1. 已确认的方向与组件边界

用户明确要求去掉可见轨道。因此默认、悬停、拖动、深色主题下均只绘制滑块，不出现轨道底色、长条边框、阴影或占位槽。早期原型“悬停时显示浅轨道”的方案已移除，不作为可选 variant 保留。

滑块采用中性色和两端圆头；静止纤细，悬停加宽并加深，拖动进一步加深。加宽只向内容侧展开，纵向滑块右边缘、横向滑块下边缘固定。鼠标使用普通箭头，不切换链接手形或抓手。

“无可见轨道”不等于没有滚动范围或命中区域。透明边缘区域仍用于发现、扩大滑块命中范围与点击翻页；它不参与正常布局，不绘制背景，不主动创建新的滚动容器。

| 负责 | 不负责 |
| --- | --- |
| 滑块位置与比例呈现、显隐、悬停／拖动状态、透明命中区 | 业务内容渲染、列表投影、行测量、筛选与选择 |
| 将用户拖动／翻页交给已有滚动 handle | 维护第二份 scroll offset、虚拟总长度或读取全量业务数据 |
| 横向、纵向、双轴及交汇避让 | 滚动容器的宽高、分页加载、无限列表的数据请求 |
| 普通容器与已有虚拟滚动适配 | 独立的平滑滚动引擎、弹性回弹或滚动同步管理器 |

不增加箭头按钮、刻度、位置气泡、进度文字、渐变或立体效果。Demo 的百分比输出是观察工具，不是正式组件的一部分。首版不提供 Small / Medium / Large：滚动条由视口、统一尺寸 token 与 rem 决定，不随 Tree 行密度变粗变细。

## 2. 已有能力核对与复用

下列事实以本仓锁定版本及本地 vendor 源码为准，并非对所有上游版本的保证。

| 来源 | 已有能力 | 本设计的使用方式 |
| --- | --- | --- |
| [Base Scrollbar](../vendor/gpui-base/src/scrollbar.rs) | 横／纵／双轴、比例与拖动映射、点击空白区定位、滚动句柄适配、显隐与宽度过渡 | 作为唯一行为实现；Yue 包装呈现及必要配置 |
| `ScrollbarHandle` | viewport_bounds、offset、set_offset、content_size、start_drag / end_drag | 读取真实 owner，输入继续经过此接口 |
| `ScrollbarStyles` | track / hover / active 与 thumb / hover / active，宽度、inset、radius、min_length | 所有 track 状态显式透明；滑块状态统一投影 |
| `ScrollbarTheme / ScrollbarMotion` | 全局默认、显隐策略、idle、enter、exit、expand 与入场方式 | 复用生命周期；参数由 Yue token 提供 |
| GPUI `ScrollHandle` | 普通 Div 的实际滚动位置与范围 | 普通容器直接复用，不转存到 Yue state |
| `UniformListScrollHandle / ListState` | 现有列表适配；ListState 含拖动开始／结束处理 | 保留已验证的测量与拖动协调 |
| `VirtualizerScrollbarHandle` | 当前虚拟 owner 的滚动条桥接 | YueVirtualList 直接使用现有 handle |
| [YueVirtualList](../crates/yue-component/src/virtual_list.rs) | 默认创建 Base 纵向 Scrollbar；已有 `.scrollbar(bool)` | 以 YueScrollbar 替换呈现，保留开关和数据合同 |
| [Showcase 滚动包装](../examples/windows-showcase/src/scroll.rs) | 内容／有限视口分离、稳定 ScrollHandle、布局边界 overlay | 保留结构，迁移滚动条调用；不复制一套布局器 |
| [TimePicker 时间列](../crates/yue-component/src/time_picker/panel.rs) | 各列现有 ScrollHandle 与 Base Scrollbar | 保留列 owner、定位取消与列表键盘语义 |
| YueTheme / tokens / motion | 主题语义、长度换算与动效策略 | 通过现有同步链路注册并消费 token |

实施前确认的三个差异，现已通过显式 Base 配置补齐，并保留旧默认行为：

1. `Scrolling` 模式在隐藏时不追踪滑块 hover，不能直接当成“靠近边缘可唤醒”的完整 Auto；`Hover` 也不能直接假定覆盖整个内容面板。
2. 双轴交汇逻辑与轴是否真实溢出有关，不能用“两根独立 Scrollbar”代替统一双轴处理，也不能在另一轴无溢出时仍缩短有效长度。
3. Base 的 `inset` 同时影响横向留白和纵向绘制长度，`min_length` 当前指逻辑长度，绘制长度还会减去两端 inset。不能将它直接设为 32px 后就声称可见滑块至少 32px。

优先用既有配置完成外观；确需补充的边缘唤醒、命中或几何机制进入 Base，使用显式配置或通用接口，保持默认调用兼容。Yue 色值、具体尺寸和时长不得写入 Base。补丁须登记到 [YUE-PATCH](../vendor/gpui-base/YUE-PATCH.md)，说明升级／移除条件与回归用例。

## 3. 状态所有权与呈现结构

```text
普通 ScrollHandle / ListState / VirtualizerScrollbarHandle
  └─ 唯一滚动位置、视口大小、内容总范围、拖动会话
                         ↓
YueScrollbar：稳定 id、轴、显隐配置、YueTheme
                         ↓
Base Scrollbar：几何、命中、输入路由、临时交互与动效状态
                         ↓
视口边缘透明命中区 + 圆头滑块
```

YueScrollbar 为 `RenderOnce` 呈现组件，不要求宿主新建 `Entity<YueScrollbarState>`。hover、drag、idle、宽度与透明度过渡继续保留在 Base 的 keyed element state。不得每次 render 创建异步任务、订阅或第二个可写滚动 owner。

每个滚动视口使用一个稳定的业务派生 id；同一视口双轴使用一个实例。多个时间列、多个列表和相同调用点的重复实例不得共享临时状态。隐藏后原位显示不重建内容或改变视口尺寸。

## 4. 布局、长度与几何合同

### 4.1 绑定实际滚动视口

滑块沿所属滚动区域边缘覆盖绘制，不以整个页面、内容总尺寸或外层卡片的尺寸作为默认边界。

| 场景 | 应绑定的边界 |
| --- | --- |
| 整个页面内容滚动 | 页面内容视口，不包含固定导航和工具栏 |
| List / Tree | 实际行视口，不包含固定搜索栏、标题和底部说明 |
| 表格 | 可滚动 body；固定表头／列通过同一 owner 的既有视口适配明确排除 |
| TimePicker | 各时间列的列表视口，不包含列标题和底部确认区 |
| Popup 内部候选 | 候选列表视口，不包含搜索框和固定面板操作 |

父级必须提供有限的可用宽高。组件不自动探测窗口或祖先中的“真正页面”，也不为解决无限高度而偷偷包一个新的 overflow 容器。优先读取 handle 的 viewport；确有复合布局时再显式使用已存在的 bounds／layout 适配。

### 4.2 透明命中区与滑块

以下为 rem=16 时的设计基准；尺寸随 rem 换算，并经窗口 DPI 映射为物理像素。

| 参数 | 基准 | 合同 |
| --- | --- | --- |
| 静止滑块厚度 | 4px | 纵向为宽度，横向为高度 |
| 悬停／拖动厚度 | 8px | 从外侧固定边向内容侧展开 |
| 边缘命中区厚度 | 14px | 状态变化时不伸缩，不绘制背景 |
| 滑块距视口外侧边缘 | 3px | 两种厚度共用；不是滚动条距整页的间距 |
| 滑块两端留白 | 3px | 放在滑块运动几何中，不绘制短轨道来表示 |
| 最短可见滑块 | 32px | 经两端留白处理后的实际绘制长度 |
| 圆角 | 当前滑块厚度的一半 | 始终圆头，随宽度过渡同步变化 |

例如纵向滑块右侧固定为 `viewport.right - 3px`：4px 和 8px 状态仅改变左边缘。命中区保持在视口内，不越过相邻面板或窗口 resize 边缘。透明区不作为整面板透明遮罩；位于它之外的内容点击必须正常到达原控件。

滑块可能覆盖底层文字或末列内容。需要避免覆盖时，由内容布局预留稳定的末端空间；该空间在滑块隐藏后仍保留，不随 hover 改变内容宽度或触发行重排。YueScrollbar 本身不自动修改内容 padding，Tree／Select／表格可按各自槽位合同处理。

### 4.3 长度与位置

计算使用真实可见范围和滚动范围：

```text
V = 对应轴的实际可见长度
C = owner 提供的内容总长度
M = max(C - V, 0)
L = 可用于滑块运动的长度（扣除两端留白、必要的交汇避让）
T = min(L, max(32px × rem比例, L × V / C))
P = clamp(逻辑滚动距离 / M, 0, 1)
滑块起点 = 运动区域起点 + P × (L - T)
```

`C <= V` 或视口不可用时不绘制该轴滑块，不注册该轴可交互命中区。所有零除、非有限数值和短到容不下最短滑块的视口都须安全处理；短视口将 T 钳到 L。无运动距离时拖动不能产生非法 offset。

公式描述外部合同；实现继续使用 Base 的统一几何函数。原生 handle 的负 offset 与虚拟化逻辑位置的转换只发生在现有适配边界，不在 Yue renderer 中再次取反。绘制、点击和拖动必须使用同一份几何。

rem=20 时，4 / 8 / 14 / 32px 分别变为 5 / 10 / 17.5 / 40px。不能逐段强转整数导致累计漂移；验证 125%／150% DPI 下的外侧基线与端点。可见像素验收与无头逻辑几何验收分别报告。

### 4.4 双轴交汇

仅在横纵两个方向都真实溢出时，在右下角各避让一个命中区厚度，保证两轴的有效命中区不重叠。交汇处透明，不画方块、槽底或分隔线。仅一轴溢出时，该轴恢复完整可用长度。

两个轴分别按自己的 viewport / content 求比例，不能共用一轴的 overflow 判断。隐藏／悬停加宽不改变另一轴可用长度；只有溢出拓扑或视口尺寸变化才重算交汇几何。

## 5. 显示策略与交互

### 5.1 三种显隐策略

| 策略 | 合同 | 建议用途 |
| --- | --- | --- |
| Auto，默认 | 实际滚动、进入边缘命中区或拖动时显示；活动结束后延迟淡出 | 页面、List、Tree |
| Hover | 鼠标进入所属视口时显示；操作／拖动期间保持，退出后淡出 | Select、紧凑弹层 |
| Always | 有溢出时持续显示，仍响应悬停／拖动状态 | 表格、持续判断位置的长文档 |

任何策略都只影响呈现，不禁用滚轮、键盘或程序定位。Auto 隐藏时仍允许边缘 hover 发现，但不得让不可见滑块抢走首次点击：先显示，已显示且可命中后再接受按下。Hover 的视口发现由同一个视口／Base 机制提供，不添加轮询或全局鼠标监听。

用户正悬停边缘或拖动时取消淡出。离开边缘／结束拖动后重新计算 idle；连续滚动刷新同一截止时间，不为每次 wheel 创建一个独立 timer。隐藏或卸载的面板不持续请求动画帧。

拖动位置不做缓动：每个指针输入立即写入原 scroll owner，宿主刷新合并到下一显示帧。同一帧前多次输入只保留一次刷新请求，绘制时使用最新位置；不再使用独立的120Hz定时器限制 Yue 拖动刷新。停止移动后不持续请求拖动帧，松手精确提交终点并使待执行的拖动回调失效。该调度同样覆盖继承 Yue 主题的 Base 内部滚动条；厚度、颜色和显隐动画继续按需续帧。

### 5.2 状态与操作

| 触发 | 外观与结果 |
| --- | --- |
| 正常显示 | 4px、中性色、圆头，无可见轨道 |
| 悬停边缘命中区 | 8px、hover 色；外侧边缘不变 |
| 按下滑块并拖动 | 8px、active 色；保留抓取点，不将滑块中心跳到指针 |
| 拖出视口后继续移动 | 持续更新同一拖动会话，offset 钳在合法范围 |
| 松手／捕获取消／窗口失活／卸载 | 结束拖动并调用对应 end_drag；不得保留卡死的 active 状态 |
| 点击已显示的边缘空白 | 向点击方向翻约一页，基准为 90% 可见长度；保持另一轴不变 |
| 鼠标滚轮 | 继续交由原滚动 owner；指针在滑块命中区内也能正常滚动 |
| 内容／窗口尺寸变化 | 更新比例、钳制已失效位置；不凭空生成用户滚动事件 |
| 单轴内容不再溢出 | 移除该轴滑块及命中，结束该轴拖动，另一轴按新几何协调 |

首版不加入轨道长按自动翻页、Shift 点击跳转或双击特殊动作。不能用“点击空白跳到指针比例位置”冒充当前原型的按页翻动；当前 Base 此处存在行为差异，须明确适配并加回归。

滚轮嵌套传播遵守既有滚动容器规则，不为滚动条单独吞掉所有方向的 wheel。列滚动、二维表格和 Shared／Attached 虚拟 owner 的其他轴不能被一次 set_offset 重置。

## 6. 动效与虚拟化约束

默认仅对可见性透明度、滑块厚度做短过渡。无横向滑入、弹簧、位置延迟、长度呼吸或不断播放的效果；drag 的目标位置立即反映真实滚动位置。

| 参数 | 设计来源 |
| --- | --- |
| idle 保留 | 800ms；是显隐策略等待时间，不冒充 motion duration |
| 显示／宽度／颜色过渡 | 140ms，与已确认 HTML demo 一致 |
| 消失过渡 | 140ms |
| 透明度曲线 | `cubic-bezier(.2, 0, .2, 1)` |
| 宽度／颜色曲线 | 同上；颜色在 sRGB 通道中连续插值 |

按用户实窗反馈，正式组件现在完整吸收 HTML 原型的140ms状态过渡。值只写在设计源的 scrollbar token 中，再经同步生成；不改其他组件的全局 motion 语义。悬停加宽与加深、按下进一步加深、松开／移出恢复、显隐均连续过渡；中断从当前采样值反向。纯淡出保持滑块基线固定，不附带侧移。

reduced motion 下直接采用目标透明度与厚度，仍保留 800ms idle 的可发现时间。窗口隐藏／失活时结束输入会话并停止过渡；重新显示后根据当前 owner 和策略恢复。

Measured 列表的内容总长可能仍为估算值，滑块比例随 owner 的最新范围更新。拖动中继续使用已有 start_drag / end_drag 冻结映射机制；不能因每帧新测量让滑块从指针下逃走。拖动属于真实用户输入，程序 smooth 定位的取消与锚点协调必须走既有路径。松手后由 owner 协调最新范围，Yue 不保存一份长期冻结的 total extent。

## 7. 公共 API 方案

以下 API 已从 `yue_component` 根导出；组件为 `RenderOnce`，不要求新增 Entity。

| 入口／配置 | 方案 |
| --- | --- |
| `YueScrollbar::vertical(id, &handle)` | 纵向，接受现有 `ScrollbarHandle + Clone` 适配 |
| `YueScrollbar::horizontal(id, &handle)` | 横向 |
| `YueScrollbar::both(id, &handle)` | 一个 owner 的统一双轴实例，内部协调交汇 |
| `.visibility(ScrollbarVisibility)` | Auto / Hover / Always，默认 Auto |
| `.theme(YueTheme)` | 可选局部主题，默认使用当前 YueTheme |
| `.viewport_bounds(Bounds<Pixels>)` | 按需转发既有复合视口边界能力，不改变 owner 滚动范围 |
| `.viewport_from_layout()` | 按需复用 overlay 自身布局界定可见边界 |

不公开 offset、content_length 或“当前百分比”作为第二份受控值，不增加 on_change 回调替代 owner 事件。不开放任意 track renderer、滑块 children 或单实例颜色／宽度覆盖；设计统一由 token 控制。没有独立 disabled 属性：容器是否允许滚动由滚动 owner 负责，是否绘制由宿主挂载和轴溢出决定。

YueVirtualList 已有 `.scrollbar(bool)` 保留，不因为替换外观而强迫消费者迁移业务 state。新显隐类型从 crate 根导出；不直接用 Base 的 Scrolling 名称掩盖 Auto 新合同。名称及签名在实施前与本项目已有公共导出检查冲突。

## 8. Token 与主题投影

新增来源位于邻仓 `Yue-Design-System` 的 component token 层，经现有同步工具生成本仓 `generated.rs / inventory.json`，禁止手改生成数据。此文档仅提出注册方案，尚未写入来源或运行同步。

| 拟定 token | 默认／来源 | 用途 |
| --- | --- | --- |
| `--scrollbar-thumb-width` | 4px | 静止厚度 |
| `--scrollbar-thumb-width-hover` | 8px | hover 厚度 |
| `--scrollbar-thumb-width-active` | 复用 hover 值 | drag 厚度 |
| `--scrollbar-hit-width` | 14px | 两轴固定命中厚度 |
| `--scrollbar-thumb-inset` | 3px | 外侧及端点留白的基准；几何上区分跨轴／主轴使用 |
| `--scrollbar-thumb-min-length` | 32px | 最短可见长度 |
| `--scrollbar-thumb-color` | `--border-control` | 静止中性色 |
| `--scrollbar-thumb-color-hover` | `--text-secondary` | hover 中性色 |
| `--scrollbar-thumb-color-active` | `--text-primary` | drag 中性色 |
| `--scrollbar-idle-delay` | 800ms | 最后一次活动后的保留时间 |
| `--scrollbar-duration-enter` | 140ms | 淡入 |
| `--scrollbar-duration-exit` | 140ms | 淡出 |
| `--scrollbar-duration-expand` | 140ms | 加宽／收窄 |
| `--scrollbar-duration-color` | `--scrollbar-duration-expand` | 滑块颜色过渡 |
| `--scrollbar-ease-fade` | `cubic-bezier(.2, 0, .2, 1)` | 显隐曲线 |
| `--scrollbar-ease-expand` | `cubic-bezier(.2, 0, .2, 1)` | 厚度／颜色曲线 |

不注册轨道背景／边框 token，避免留下与“无轨道”相冲突的公开能力。Base 的所有 track 状态显式设置透明背景与透明边框，不能依赖某个状态的默认值碰巧透明。圆头半径由实时厚度求出，不再新增任意 radius 配置。

颜色适配 Light / Dark × Azure / Neutral 及已有 neutral tones；静止滑块与实际承载 surface 的对比目标至少 3:1，hover／active 不低于静止。上述语义别名是设计起点，实施时须测量 page、surface、popover 等真实背景，不能仅凭 token 名称宣称合格。Demo 的独立 CSS 色值只是原型，不是正式主题来源。

全局 `ScrollbarTheme` 可以覆盖仍然直接使用 Base 的调用点，但其样式当前使用已解析 Pixels；rem 属于 Window。不能把某个窗口解析出的尺寸写成所有窗口共享的固定 px 值。YueScrollbar 每次按目标窗口解析局部尺寸与主题，旧入口逐个迁移后再决定全局投影范围；禁止为跨窗口 theme/rem 覆盖而在 render 中修改全局主题。

## 9. 键盘与无障碍边界

滚动能力必须能通过所属视口的既有键盘路径使用。Tree 的方向键、List 的活动项导航、TimePicker 的值步进保持各自含义；滚动条不拦截它们，也不为每个时间列增加一个额外 Tab 停靠点。

正式首版滚动条默认不单独取得键盘焦点，不把无输入能力的绘制元素伪装成可调整控件。HTML 原型为浏览器演示提供独立 scrollbar 焦点及 Home / End / PageUp / PageDown，这不等于当前 Base 已实现相同的 GPUI／UIA 语义。

当前 Base 滚动条源码没有完整独立 ScrollBar 节点、键盘 focus owner 和无障碍 SetValue 链路。首版复用滚动视口原有的可访问滚动路径，并测试滚动条鼠标操作不破坏焦点与 active item；不以新增 Windows COM provider 作为改外观的前置条件，也不声称完成了独立 UIA 滚动条支持。

reduced motion 必须生效。Windows 高对比度下应使用项目能实际取得的系统／主题信息保证滑块可见；HTML 的 forced-colors 验证不能当成 GPUI 原生支持证据。该平台验收在实现阶段单列结果。

## 10. 集成范围与迁移顺序

| 调用方 | 迁移要求 |
| --- | --- |
| YueVirtualList | 替换默认 Base 纵向呈现；保留 handle、绑定、角色及 `.scrollbar(false)` |
| YueList / Tree / TreeSelect / Select | 优先继承同一个 YueVirtualList 入口，避免各自复制 scrollbar 状态；检查 Popup 有效边界 |
| TimePicker | 每列明确 id；只覆盖列表，不含列标题；拖动继续取消进行中的程序定位 |
| Showcase 页面／侧栏 | 替换 [scroll.rs](../examples/windows-showcase/src/scroll.rs) 中现有呈现，不改变内容与视口分离布局 |
| 横向虚拟示例／二维布局 | 复用既有适配 handle；真正同一视口双轴时使用 both，不以两个实例争抢右下角 |
| GPUI 输入编辑器等自带滚动表面 | 先核对已有 owner 与可定制入口；没有迁移证据前不宣称“全项目统一完成” |

切换外观不迁移虚拟化算法、不重建 scroll handle、不重置位置、不改变 Tree 行高、列表内容宽度或旧的滚动事件次数。只迁移已核实入口，不通过修改 GPUI 绘制器强行替换所有系统滚动表面。

## 11. 测试与验收

复用当前 Base 滚动条测试和虚拟滚动已有逻辑，再对本次新增合同增加真实布局／输入回归，不编写只复述公式的测试。

| 方面 | 必须验收 |
| --- | --- |
| 无可见轨道 | Light / Dark 下 normal／hover／drag 的绘制记录无轨道底色／边框；透明角区不画块 |
| 尺寸与几何 | rem=16 / 20，4→8px 外侧基线稳定；可见最短长度；首尾可达；极短视口、零范围安全 |
| 命中 | 14px 区域可发现与拖动；区域外 Button 正常点击；隐藏滑块不吞首次点击 |
| 显隐 | Auto 隐藏后边缘唤醒；Hover 进入视口显示；Always 有溢出才显示；悬停／拖动不意外淡出 |
| 动效 | 可控时钟检查中间厚度／透明度、反向连续与按需请求帧；idle timer 不随输入无界增长 |
| 拖动 | 保留抓取点、移出视口继续、钳制端点、松手／失活／卸载清理、另一轴不被重置 |
| 点击与滚轮 | 透明边缘空白按页移动；滑块区域 wheel 正常；嵌套区域按原规则传播 |
| 双轴 | 都溢出、仅 X、仅 Y、均不溢出及动态切换；角区无重叠，单轴不被无效角区缩短 |
| 普通容器 | viewport 与内部 padding／外层卡片区分；固定标题／页脚不参与滚动长度 |
| 虚拟化 | Fixed / Known / Measured、Attached、动态测量与数据变更、smooth 被真实拖动打断；原 owner 唯一 |
| 焦点与组合 | Tree／Select／TimePicker 键盘、active、F2 行操作及 Popup 会话不因拖动改变 |
| 性能 | 万行不创建全量元素；静止无持续帧；拖动刷新预算沿用既有机制，不借新动画扩大重绘范围 |
| 主题与窗口 | 两种明暗／强调配置、局部 theme、两个不同 rem 的窗口不串色串尺寸；对比度实测 |
| 实机 | 当前设备可验证的 DPI、指针抓取、触控板和高对比度；缺少多显示器／跨 DPI 条件如实标为未测 |

HTML 原型已验证浅深主题、4→8px 固定外侧边、透明背景、横纵拖动、边缘唤醒、idle、短内容隐藏、键盘示例、减少动效与窄窗口。它不证明 GPUI 的原生输入、帧成本、UIA 或多显示器表现。

## 12. 实施阶段与完成标准

1. **基线与通用缺口。** 保留现有 Base 测试结果，补隐藏后唤醒、透明命中、双轴与可见最短长度回归；必要 Base 扩展保持旧默认兼容，记录补丁。
2. **Token 与 Yue 呈现。** 注册来源、同步生成、实现 YueScrollbar 及主题／rem／motion 投影；验证所有状态无可见轨道。
3. **调用方逐批迁移。** 先普通有限视口与 YueVirtualList，再 TimePicker、Showcase 和二维示例；每批复测原滚动／焦点／数据事件合同。
4. **示例与实机验收。** Showcase 提供与 HTML 对应的策略、主题、短内容、双轴及动态内容示例，报告测试、构建和实际设备验证结果。

完成意味着已迁移入口使用相同设计、行为回归通过、无持续空转帧、token 来源与生成一致、文档反映真实 API。GPUI 实现与自动化验证见 [实施记录](research/2026-11-09-scrollbar-implementation.md)；真实设备观感、系统高对比度与触控板不以 headless 测试替代。


## 13. 当前实现与使用

```rust
use gpui_kit::{ScrollHandle, div, prelude::*};
use yue_component::{ScrollbarVisibility, YueScrollbar};

// handle 保留在宿主状态中；外层提供有限高度。
let body = div().id("results-viewport")
    .size_full().overflow_y_scroll().track_scroll(&handle)
    .children(rows);
let viewport = div().relative().h(gpui_kit::rems(20.))
    .child(body)
    .child(YueScrollbar::vertical("results-scrollbar", &handle)
        .visibility(ScrollbarVisibility::Auto));
```

二维视口用 `both(id, &handle)`；仅横向用 `horizontal`。默认读取当前 YueTheme，局部预览通过 `.theme(theme)` 指定。YueScrollbar 不使普通 Div 自动获得额外的键盘导航；宿主仍应提供适合内容的 focus 和 actions。

`yue_component::init` 安装一个按 Window 解析的 Base ScrollbarTheme resolver。这样输入框／编辑器、Text 等内部仍使用 Base Scrollbar 的入口会继承同一呈现、rem、曲线和透明命中区。没有在 render 中写全局主题，也没有用一个窗口的 Pixels 覆盖所有窗口。显式 Base `.styles(...)` 和 `.mode(...)` 仍按原优先级覆盖相应字段；Yue 自身入口使用显式局部主题。

已迁移：Showcase 页面与侧栏的 `scroll.rs`、YueVirtualList（因此覆盖虚拟 List／Tree／TreeSelect／Select）、TimePicker 列、横向虚拟列表示例、二维 Grid、Masonry 和横向布局。普通 GPUI `.overflow_*_scroll()` 本身不绘制滚动条；本次统一已有滚动条入口，没有给所有隐藏 overflow 区域擅自增加可视控件。

Showcase 侧栏新增「滚动条 Scrollbar」。页面复用 YueSegmented／YueButton，提供 Auto／Hover／Always、纵向／横向／双轴、短内容、局部深色主题与回到起点。切换策略、颜色不会重建 scroll owner。

补丁分两层：Base 承担可选行为／几何和拖动租约；GPUI 仅新增显式启用的 `capture_pointer_for_frame`，在元素卸载后释放该捕获。原 `capture_pointer` 调用保持原生命周期，图片预览等已有使用方不受新清理策略影响。升级说明分别登记于两个 vendor 的 YUE-PATCH。

实测 Light／Dark × Azure／Neutral × 四种 NeutralTone，在 page／surface／popover 三种背景、normal／hover／active 下的对比度最低 3.822:1。此值指稳定显示的主题颜色，不将淡入淡出的中间透明度当成静态颜色对比。Windows 系统高对比度仍需实机验收，未声称自动跟随系统色。
