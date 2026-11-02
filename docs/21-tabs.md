# 21 YueTabs 标签页组件设计

状态：V1 已实现。创建 / 实施：2026-11-02。公开 API、Base 扩展、Yue token 与 Showcase 均已接入；原生节点关联的后端限制见第 12 节，实施与验证记录见第 16 节。真实 UI 由用户检查。

依据用户提供的 [Tabs 参考图](assets/tabs-reference.png)、仓库内实际使用的 `gpui-base 0.8.0` 源码、现有 Yue 组件与设计规范编写。采用参考图的基础、图标、增删、尺寸、外观、四位置、滚动、禁用、内容切换及懒渲染能力；不复制网页的配置侧栏、覆盖率标记或贡献者区域，不按截图缩放比例推算尺寸。

关联：[Button](02-button.md)、[Segmented](09-segmented.md)、[Tooltip](10-tooltip.md)、[Popup](12-popup.md)、[Tag](19-tag.md)、[Select](20-select.md)。

## 1. 定位与首版范围

Tabs 在同一个容器内切换一组同级内容面板，同一时刻只呈现一个活动面板。适合设置分类、详情分区和轻量多文档容器。标签头是带选中语义的 Tab，不是把普通文本按钮改色；新增、关闭和滚动才是 Button 命令。

| 参考图场景 | Yue 设计 |
| --- | --- |
| 基础标签页 | `Normal`，活动文字和独立指示线，面板位于标签栏的内容侧 |
| 带图标 | 可选 Tabler 前置图标；图标和文字组成一个激活目标 |
| 新增 / 删除 | 显式开启，独立新增按钮与逐项关闭按钮；只发送请求 |
| 三种尺寸 | Small / Medium / Large，字号、图标、内边距和命令热区成套变化 |
| 不同外观 | `Normal` / `Card`；不增加与 Segmented 重叠的胶囊外观 |
| 不同位置 | Top / Bottom / Left / Right；左、右布局的文字保持横排 |
| 可滚动 | 单行 / 单列标签轨道，固定滚动按钮，选中项和焦点项可见性保证 |
| 禁用 | 整组禁用和单项禁用；禁用项可以保留已显示的面板 |
| 内容切换 | 默认即时切换；可选 Fade / Slide，指示线始终提供明确反馈 |
| 懒渲染 | 默认只构建当前活动面板；与业务数据加载、Entity 保留分开 |

不包含拖动排序、拆分窗口、跨窗口拖放、停靠布局、多级路由、自动持久化和文档保存决策。这些能力由宿主或 Dock 家族提供，不能因为支持关闭标签就把 Tabs 扩展成工作区管理器。

与相邻组件的边界：Tag 表示分类或筛选值；Segmented 改变一个选项；Tabs 同时声明标签栏和对应面板；Dropdown 执行命令或提供额外入口。不能用 Radio role 或 Tag 的 checked 状态代替 Tab / TabList / TabPanel 语义。

## 2. 已核实的基础能力与缺口

| 当前源码 | V1 已实现 | 职责与边界 |
| --- | --- | --- |
| [Base Tab / Tabs](../vendor/gpui-base/src/tabs.rs)、[Controller](../vendor/gpui-base/src/tabs/controller.rs)、[Panel](../vendor/gpui-base/src/tabs/panel.rs) | `Tab::in_list`、受控快照与请求、roving focus、按轴导航、Home / End、确认与关闭；TabList 方向、TabPanel 活动边界 | Controller 保存交互状态，不自行提交选择；standalone Tab 保留旧 API |
| [Base motion](../vendor/gpui-base/src/motion.rs) | keyed transition、Presence、当前呈现值反向、reduced motion | 不提供 Tabs 外观、指示线时长或面板切换视觉策略 |
| [YueSegmented](../crates/yue-component/src/segmented.rs) | 实际边界测量、局部坐标指示块、几何修订失效和稳定动画通道的现成经验 | 可借鉴测量与失效方式，不能直接复用 Radio 语义或复制成第二套 Tabs 行为 |
| [YueButton](../crates/yue-component/src/button.rs)、[YueOverflowTooltip](../crates/yue-component/src/tooltip/) | 命令、禁用、焦点、实际文本溢出提示 | 组合成新增、关闭、滚动命令和标签全文提示 |
| Yue Design System / [token 快照](../crates/yue-component/tokens/inventory.json) | `component-tokens/tabs.css` 已新增并登记；同步生成 `--tabs-*` 对应 Rust keys | 外部设计系统是唯一来源，不能直接手改生成代码 |

`Tab::on_click()` 保留 standalone 点击合同；完整复合行为显式接入 `Tab::in_list(&controller, key)`。`Tabs::children()` 不扫描任意子元素推测模型。Yue 已采用显式 Controller 路径。

现有 Base Tab 的 disabled 优先级、原生可访问操作与几何居中已有测试，扩展必须保留。保持根级 Cargo patch、Kit 的 Base-only 依赖和单一 GPUI 后端，不引入 styled Component 层。

## 3. 分层与状态所有权

```text
宿主：业务 key / 顺序 / 当前 key / 添加删除审批 / 各面板 Entity 与异步任务
                     ↓ 受控快照
YueTabs：RenderOnce 外观、token、图标、命令、Tooltip、动效策略
                     ↓ 显式标签模型
Base Tabs：身份、请求协议、复合键盘、焦点、滚动几何、面板激活边界
                     ↓
GPUI：布局 / ScrollHandle / FocusHandle / Actions / 原生可访问树
```

- **宿主是真值所有者**：当前 key、标签集合与顺序由宿主持有；`on_change` / `on_close` / `on_add` 只报告意图。宿主可以拒绝，组件不能提前移动选中线、替换内容或移除标签。
- **Base 保留交互状态**：按 Tabs id 保存 focused key、逐项 FocusHandle、滚动句柄、已访问 key 和测量修订；不保存第二个业务 selected key。render 只同步必要的快照，不执行命令或无条件 notify。
- **Yue 保留呈现状态**：指示线 / 面板动效通道、hover / pressed 和主题签名；不管理业务集合，不用逐帧任务替代 Base motion。
- **宿主保留面板状态**：输入、编辑器、选中行、请求和缓存属于各面板 Entity；面板暂时隐藏不等于业务 Entity 被销毁。

反复渲染、插入标签或修改 label 不得改变同一 key 的身份。标签 key 用非空、唯一的 `SharedString`，元素和动画 id 由 `(tabs id, key, part)` 派生；不能用索引、标题或每帧生成的随机值。

Base 扩展采用显式模型和 controller，不遍历任意子元素猜测 label、disabled 或关闭按钮。现有 `Tab::new` / `Tabs::new` 兼容路径保持可用；完整复合行为通过新增的明确入口接入。

## 4. 组成与布局

```text
YueTabs
├─ Header
│  ├─ PreviousScrollButton（仅溢出时）
│  ├─ TabViewport / Base Tabs（唯一标签轨道滚动所有者）
│  │  ├─ TabCell(key)
│  │  │  ├─ Base Tab：可选图标 + 单行标题
│  │  │  └─ CloseButton（可选，与 Tab 是独立命令）
│  │  └─ SelectionIndicator（Normal；不接收输入）
│  ├─ NextScrollButton（仅溢出时）
│  └─ AddButton（显式开启时，固定在轨道外）
└─ PanelViewport
   └─ Base TabPanel：当前内容，或显式空内容
```

Header 与 PanelViewport 相邻的那条结构边界只绘制一次；不要同时给 Header、Tab、面板各加一条同色边。内容默认不额外包 Card、不加阴影、不擅自增加宿主已有的面板内边距。

### 4.1 四个位置

| placement | 标签排列 | 标签栏位置 | Normal 指示线所在侧 |
| --- | --- | --- | --- |
| Top（默认） | 水平 | 面板上方 | 标签底部，朝向面板 |
| Bottom | 水平 | 面板下方 | 标签顶部，朝向面板 |
| Left | 垂直 | 面板左侧 | 标签右侧，朝向面板 |
| Right | 垂直 | 面板右侧 | 标签左侧，朝向面板 |

四位置采用同一份顺序和选择模型，换位置不重建业务 Entity。横向、纵向的布局与导航轴一起改变；文字和图标不旋转。纵向头部采用固定的一列宽度，标题左对齐，关闭命令在尾侧对齐，不让各行宽度随文字不同。

采用参考图的线形 Tabs 作为 `Normal` 的明确视觉选择；选中还必须有更强前景色 / 字重，不能只靠一条颜色线。Card 通过选中表面与连接边界表达选择，不再叠加第二条强调线。

### 4.2 尺寸建议

以下是已登记 token 在 16px rem 基线下的尺寸，按 rem / 字号共同缩放。

| 项目 | Small | Medium（默认） | Large |
| --- | --- | --- | --- |
| 单项最小高度 | 28px | 32px | 40px |
| 字号 | 12px | 14px | 16px |
| 前置图标 | 14px | 16px | 18px |
| 水平内边距 | 8px | 12px | 16px |
| 图标与文字间隔 | 4px | 4px | 8px |
| 关闭 glyph | 12px | 14px | 16px |
| 关闭 / 新增 / 滚动命令热区 | 24px | 28px | 32px |

Normal 单项横向宽度随内容，默认上限 200px；最小宽度必须容下内边距、可选图标、至少一个文字 em 和关闭热区。三种尺寸的关闭按钮独立横排，不压缩为细小叠放的图标。

Left / Right 的标签栏宽度默认 160px，允许通过显式 `header_width` 改变，并按实测结构最小值约束；宿主负责提供足够容器宽度。横向标签不换行；纵向标签不换列。超长标题单行省略，实际发生截断才显示完整文字 Tooltip，完整 accessible name 不截断。

关闭区域在 closable 项中始终占真实宽度并可见，不因 hover 突然挤动标题；non-closable 项不预留空白。不设置与图标对称的虚假空槽。无前置图标的标签同样不占图标槽。

### 4.3 两种外观

| variant | 静止 | 选中 | hover / pressed |
| --- | --- | --- | --- |
| Normal | 透明表面、常规文字、连续基线 | 强前景色，独立指示线位于内容侧 | 轻量中性表面反馈；不移动指示线 |
| Card | 单项卡片轮廓，未选中表面略弱 | 与面板同层的表面，连接侧去掉重复边界 | 仅改变当前卡片的表面 / 边框 |

Card 靠近面板的一侧不画双边；远侧外角跟随同一圆角 token，不能用白色矩形遮住边框，因为暗色主题会露馅。disabled 覆盖 hover / pressed；focus visible 独立于 selected。默认箭头光标，不使用超链接手形光标。

## 5. 受控选择与变更顺序

`selected: Option<SharedString>`：`Some(key)` 指向当前存在的标签，`None` 表示当前没有活动面板。初始化默认项由宿主明确选择，组件不会每次 render 自动选第一项。

1. 点击、确认键或自动激活导航提出目标 key。
2. Base 检查目标存在、可交互且不同于已提交 key；相同标签不重复请求。
3. 调用一次 `on_change(request, window, cx)`；request 包含目标 key 和 Pointer / Keyboard / Accessibility 来源，不伪造鼠标事件。
4. 宿主检查权限、未保存内容等条件，接受后更新自己的 selected 并 notify。
5. 下一次已提交快照决定 Tab selected、指示线和面板；动效从当前呈现值到新目标。

回调可以同步修改、删除或重建组件；Base 不在回调之后继续使用旧索引和借用。外部 selected / items 同步不回调 `on_change`，不在 render 中发“纠正事件”。焦点允许先移动到请求目标，但宿主拒绝时选中线和面板保持原样。

宿主删除时应在同一次更新中协调 items 和 selected。若传入不存在的 selected key，展示无活动面板并把入组焦点落到第一个可用标签，不回调纠正事件；不能显示一个被静默选中的不同面板。重复 key、空 key / 空标题触发明确的配置断言。

## 6. 键盘与焦点

Base 使用 `BaseTabs` key context，并在 Tab 的实际焦点路径处理按轴导航；Tab / Shift+Tab 复用 Base Root 的遍历 Actions。只在 Tab 本身持焦点时处理导航、确认和 Delete，内容输入框、菜单和子 Tabs 的按键由其自身处理。四位置保持 Header → Panel 的逻辑遍历顺序，Bottom / Right 通过 flex reverse 改变视觉顺序。

| 输入 | 默认行为 |
| --- | --- |
| Tab / Shift+Tab | 标签集合只有一个常规入组停靠点；进入已选中且可用的 Tab，否则进入记忆的可用焦点项 / 第一项；退出后继续正常表单顺序 |
| Left / Right | Top / Bottom 沿显示顺序移动焦点，跳过禁用项；不劫持垂直方向键 |
| Up / Down | Left / Right 沿显示顺序移动焦点，跳过禁用项；不劫持水平方向键 |
| Home / End | 到第一个 / 最后一个可用标签 |
| Enter / Space | 手动激活模式下请求切换；自动模式不重复发事件 |
| Delete | 焦点位于 closable Tab 时请求关闭该项；其他情况不消费，不从面板输入框冒泡误删 |
| Escape | 交给当前 Tooltip / Popup 等层；Tabs 本身不是需要关闭的浮层 |

默认 `TabsActivation::Automatic`，方向键移动焦点时请求切换，适用于已就绪且开销小的内容；`Manual` 只移动焦点，确认后再切换，适用于昂贵面板或审批流程。自动模式也受宿主接受规则约束。

方向键首尾默认循环；若只剩一个可用项，保持原处且不重复回调。全禁用时没有可激活 Tab；`None` 或空集合不制造空白的伪 Tab。暂不全局绑定 Ctrl+Tab、Ctrl+W、Ctrl+PageUp / PageDown，避免抢占宿主工作区命令。

关闭按钮作为独立 Button 暴露原生命令，不嵌套为 Tab 的点击区域；默认不为每个关闭按钮增加 Tab 停靠，键盘通过聚焦 Tab 后 Delete 操作，辅助技术仍可执行按钮的原生操作。新增和可用滚动按钮有自己的停靠点，跟随 Header 的视觉顺序。提供明确名称，如“关闭 外观”“新增标签页”“向后滚动标签”。

切换不自动把焦点抢进内容；用户按 Tab 进入活动面板。Panel 默认提供带焦点环的入口；有正常交互控件时，宿主使用 `panel_tab_stop(false)` 关闭这个额外停靠，组件不自动检查任意子树。

若外部切换将要隐藏当前持焦点面板，先把焦点移到新活动 Tab，再隐藏旧子树。若焦点本来在 Tabs 外，不抢焦点。标签被删除或禁用时按稳定 key 协调焦点，不能留下指向已消失节点的 FocusHandle。

## 7. 新增、关闭与动态集合

默认 `addable = false`，逐项 `closable = false`。开启新增时，“+”在标签轨道外的尾侧，不随轨道滚走，也不冒充一个 Tab。点击只发 `on_add` 请求；key、标题、内容和是否选中新项由宿主决定。

关闭按钮点击只发 `on_close`，不能先触发切换。命令激活时先把焦点交给同 key 的 Tab，宿主接受删除后再由 Controller 协调邻项，避免焦点留在被卸载的关闭按钮上；拒绝时保留该 Tab 焦点而不改变 selected。已禁用项不允许用户切换或关闭；宿主仍可通过数据更新移除。未保存标记、是否弹 Dialog、是否允许关闭最后一项是宿主策略。

| 宿主接受的操作 | 推荐 selected / focus 协调 |
| --- | --- |
| 删除未选中项 | 当前 key 不变；如果被删项持焦点，焦点移向邻近可用项 |
| 删除已选中项 | 优先选右邻可用项，其次左邻；没有可用项则 None |
| 删除最后一项 | None；焦点转移到新增按钮或宿主明确的恢复目标 |
| 新增并选中 | 同一次数据更新插入并设置新 key；使新项可见，不排队经过旧项 |
| 外部重排 | selected / focused key 不变；清理失效几何，不把旧索引当身份 |

Base 提供基于操作前顺序的候选 key 计算帮助，但不替宿主提交业务 selected。`CloseRequest` 的邻项候选只是建议；若审批期间集合已变，宿主必须按最新集合重新确认，不能提交陈旧索引。

V1 增删直接更新集合，不保留一个仍可交互的“正在消失的标签”。其余标签正常重排；不承诺宽度弹跳或逐项入场。指示线按第 9 节处理删除 / 插入后的几何失效。

## 8. 标签轨道滚动

### 8.1 布局与命令

- Header 的滚动与 Panel 的滚动完全独立；面板的大表格不能把标签栏一起滚走。
- 测量实际标题、图标和关闭命令的边界决定是否溢出，不用字符数或固定标签数量判断。
- 溢出时为前后两个滚动命令预留固定布局位置；按钮不得覆盖标签、关闭图标或选中线。新增按钮始终在独立尾侧命令区。
- 滚到首 / 尾时对应按钮禁用而非消失，避免视口反复变宽；两个按钮只在整个轨道不再溢出时一起移除。
- 单次点击按一屏减去一个完整标签的重叠量推进，并向可辨识的标签边界对齐；超宽单项按视口长度分页，至少推进一个有效步长。
- 支持触控板原生轴向滚动与 Shift+滚轮的横向滚动；纵向只消费轨道实际使用的轴，不把面板普通纵向滚轮全部劫持。

Header / Viewport 的弹性子项释放 `min_w` / `min_h`，由一个 ScrollHandle 管理轨道。命令热区不压缩，横向轨道保留能容纳标签结构的最小宽度；纵向 Header 同样有结构下限。极窄容器需要宿主提供足够宽度，V1 不提供额外的最小尺寸报告事件。

### 8.2 可见性与对齐

键盘 focused key、用户接受的 selected key、外部设置 selected 以及新增选中，都要确保相应标签可见。默认 `Auto` 使用最小必要位移；可选 Start / Center / End 是显式定位命令的对齐策略，不在用户每次滚轮时强制居中。

标签大于视口时，优先露出标题起始侧；关闭按钮保留键盘 Delete 和原生操作路径。用户主动滚离选中项后不马上拉回；只在新的导航 / 选择 / 显式定位请求时执行 ensure-visible。

滚动位置和标签边界存储于**轨道内容坐标**。屏幕坐标只在绘制 / hit-test 时由 `轨道原点 + 内容坐标 - 当前滚动偏移` 得出。滚动不能被误判成指示线在不同标签之间切换。

鼠标、键盘和外部更新提出新定位请求时，以最新目标替换旧目标，不排队播放滚动任务。rem、窗口尺寸、标题、图标、closable、顺序、尺寸和 placement 变化时使测量失效，再按同一 key 重新计算。

## 9. 动效与连贯性

主要连续动效是**选中指示线的平移与长度变化**。面板里的内容优先保持立即可用，不为已经就绪的设置页制造加载感。

| 变化 | 表现 | 已实施 token / 当前值 |
| --- | --- | --- |
| Tab hover / pressed / 选中文字色 | 当前项表面与文字轻量过渡；无整体缩放 | `--tabs-duration` → interaction，100ms；standard |
| Normal 切换选中 | 共享指示线从当前边界平移到目标，同时改变长度 | `--tabs-indicator-duration` → duration-150；standard，150ms |
| Card 切换选中 | 原卡与新卡的表面 / 文字交接；不让一个大色块跨过中间卡片 | interaction，100ms |
| ensure-visible / 滚动按钮 | 轨道朝新目标平移；直接手势跟随系统输入 | `--tabs-scroll-duration` → duration-150，150ms |
| 内容 None（默认） | 立即切换，不做空白等待 | 无 |
| 内容 Fade（显式开启） | 旧子树立即移除，只有新活动内容淡入 | enter 150ms，enter 曲线 |
| 内容 Slide（显式开启） | 旧子树立即移除，新活动内容沿导航轴轻移并淡入 | enter 150ms；距离 8px（rem） |
| 首次显示、主题 / 尺寸 / placement 改变 | 直接落到测量后的正确位置 | 不从 (0,0) 或旧布局飞入 |
| reduced motion | 指示线、滚动和内容立即到位 | 保留语义与输入响应 |

### 9.1 指示线

Normal 的指示线沿活动标签的“图标 + 文字”可见槽长度绘制，不覆盖关闭命令；无关闭项和可关闭项使用同一文字起点。纵向时沿整行的内容高度对齐，厚度使用同一个 token。位置由真实 bounds 计算，不拿平均标签宽度乘索引。

共享通道以 Tabs id 为身份，变化目标是轨道局部坐标的 indicator rect；连续点击 A → B → C 时，从当前呈现位置朝 C 继续，不先完成 A → B，不淡出重建一根线。hover 和手动模式的焦点移动不改变 selected 指示线，焦点环另行表达。

平移仅用于同一个轨道、同一几何修订且前后项都在同一可见视口的情况；跳转距离超过视口主轴长度的一半、目标原来在视口外或需要跨段滚动时，直接在目标处定位，由轨道滚动带入可见区域，不另外播放指示线淡入。阈值为归一化几何规则，不依赖某个 DPI 下的固定像素距离。

滚动期间指示线随轨道同速运动，禁止再叠加一层“滚动引发的指示线追赶”。尺寸、顺序、关闭区或文字导致轨道几何变化时，重新测量后直接定位；不能把 resize / 字体变化当成一次选中切换。失去合法 selected 时立即移除指示线。

### 9.2 内容 Fade / Slide

Slide 的轴与标签导航轴一致。在当前显示顺序中前进时，新内容从主轴尾侧轻移进入；后退反向。Top / Bottom 共用水平逻辑，Left / Right 共用垂直逻辑，不因为 Header 位于另一侧就把前进方向颠倒。8px 是局部提示位移，不把整屏内容横穿视口。

内容切换不等待指示线结束；新面板立即取得唯一活动语义和输入权限。**V1 只动画新面板入场，旧面板立即从布局、输入和可访问树移除，不保留退出呈现。** 这收紧了原设计的双面板交接，避免任意编辑子树在退出期仍能响应输入。若旧内容持焦点，先执行第 6 节的焦点协调。

复用 Base Presence 和面板激活边界，在每帧重新组合宿主持有的 Entity；不缓存上一帧的 `AnyElement`，不为动画克隆业务表单或重放异步任务。快速切换始终只有一个活动子树，新面板以最新切换世代入场，不排队播放历史切换。

面板卸载时，其 Dropdown / Popup anchor 同步退出，不能把浮层留在窗口上；已用真实 Dropdown 的外部切换 / 返回测试验证，不全局关闭其他面板或窗口的 Popup。面板高度默认由外部容器或当前内容确定，不自动播放高度补间；需要稳定工作区时由宿主给定高度。启用内容动效时仅裁切内容视口，宿主内容应为自身焦点环提供内边距。

V1 不实现旧面板淡出；未来只有在可证明退出子树彻底失活时才增加该呈现路径。当前 Fade / Slide 的旧子树移除与唯一活动语义均有 headless 测试。

## 10. 面板懒渲染与生命周期

`.lazy(true)` 为默认：每次只调用**宿主已提交的活动 key**对应的 content renderer。`.lazy(false)` 每次调用所有 renderer，但仍只有当前活动内容接入布局、输入和语义树；其余构建结果当帧丢弃。Controller 记录已访问 key 供 Base 消费者读取，不据此缓存 RenderOnce 子树。这个配置控制 UI 构建，不等于自动发网络请求。

面板离开后，其业务 Entity 由宿主继续保留。Base 的 TabPanel 激活边界保证只有当前面板参与正常布局、命中、键盘和可访问遍历；停止构建不销毁宿主持有的 Entity。V1 不保留退出子树。

不承诺仅凭 `lazy(false)` 就保留任意 RenderOnce 子节点的隐式内部状态。要跨切换保留输入值、光标、列表滚动等，宿主必须保留对应 Entity / ScrollHandle，并在 content renderer 中引用它们。组件不创建一套复制表单数据的缓存。

请求在宿主处理已接受的切换时启动，或由面板自己的显式生命周期启动；render closure 中不创建每次刷新重复的 Entity、不启动任务、不订阅。异步结果按 panel key 和请求世代匹配；离开是否继续请求由宿主决定，切回不能让旧结果写入另一个面板。

隐藏面板的无限帧动画、计时器和重型绘制应暂停；业务后台任务可以继续。若宿主需要销毁离开的编辑器，由宿主释放其 Entity；不要给 Tabs 首版再增加多个 keep-alive / destroy-on-hide 布尔开关。

## 11. 禁用、空内容与异常状态

| 状态 | 合同 |
| --- | --- |
| 单项 disabled | 可见且标示禁用；不能由用户切换、关闭或成为 roving 目标 |
| 已选中项变 disabled | 保留 selected 与可阅读内容；不自动切到另一页。标签操作禁用不意味着业务面板里的全部控件被禁用 |
| 整组 disabled | 停止 Tab 切换、新增、关闭及滚动命令；现有内容仍可阅读，面板业务 readonly 由宿主单独决定 |
| None / 空集合 | 不画选中线，不制造假面板；可显示宿主提供的 empty 内容，新增按钮按配置保留 |
| 面板 loading / error / empty | 由面板内容呈现；保持标签名称、Header 高度和选中身份，不用整条标签栏闪烁代替内容状态 |
| 关闭被拒绝 | 标签与内容不变；宿主负责必要说明，不先播放退出 |
| 切换被拒绝 | selected 与内容不变；焦点与 selected 可分离，便于继续键盘操作 |

disabled、主题或外观切换要清理旧 hover / pressed，并使受影响动效直接同步到目标；不遗留“禁用后还会发亮”的状态。

## 12. 原生可访问性

- 集合根为 TabList，逐项为 Tab，活动内容为 TabPanel；名称、selected、disabled、方向和集合位置与实际内容一致。
- Tab 与 Panel 使用同一稳定业务 key，Panel 使用对应标签命名。只有当前面板对辅助技术可见；原生节点 ID 关系的限制如下。
- 关闭、新增、滚动是独立 Button，各有名称和真实 enabled 状态；关闭不会同时触发 Tab 的无障碍激活。
- 不用 radio / toggle 角色替代 Tab，不把所有焦点都挂在没有名称的 Header 外壳。
- 选中、焦点、禁用不只靠颜色区分；指示线 / 选中表面与 focus ring 分别可辨认。

**当前能力边界**：已验证 Tab / TabList / TabPanel role、名称、selected、disabled、方向、集合位置与 Tab 原生 Click。GPUI 当前没有公开 `aria_controls` / `aria_labelled_by` builder，`GlobalElementId::accesskit_node_id()` 是 `pub(crate)`，Base 无法通过受支持 API 建立 Tab ↔ Panel 的原生 ID 关系。V1 公开稳定业务 key 与 Panel 名称，但不声称已经导出该双向原生关联；不复制私有 ID hash，不修改 Cargo registry。

若需要后端补丁才能导出稳定的双向关系，应将改动限制在原生语义桥接层并记录版本依赖，不能修改 Cargo registry 缓存。真实 Windows UIA / 读屏效果由用户验收，headless node 测试不能代替该结论。

## 13. 公共 API

以下 API 已导出；公开构造示例见 `tabs/mod.rs` doctest，可运行演示见 Showcase 的 Tabs 页面。

| 类型 | 责任 |
| --- | --- |
| `YueTabs` | 受控复合组件的 RenderOnce 外观 |
| `YueTabItem` | 稳定 key、标题、图标、disabled、closable、content renderer |
| `TabsSize` / `TabsVariant` / `TabsPlacement` | 三尺寸、Normal / Card、四位置 |
| `TabsActivation` | Automatic / Manual |
| `TabsContentMotion` | None / Fade / Slide |
| `TabsChangeRequest` / `TabsCloseRequest` | 只读 key、来源、建议邻项等请求数据；私有字段与 reader |
| `TabsHandle` / `TabsScrollAlign` | 按稳定 key 聚焦 / 定位；不改变宿主 selected |

```rust,ignore
// self.selected_tab: Option<SharedString>；面板 Entity 在宿主中保留。
YueTabs::new("settings-tabs", self.selected_tab.clone())
    .accessibility_label("设置分类")
    .size(TabsSize::Medium)
    .variant(TabsVariant::Normal)
    .placement(TabsPlacement::Top)
    .activation(TabsActivation::Automatic)
    .lazy(true)
    .content_motion(TabsContentMotion::None)
    .items([
        YueTabItem::new("general", "常规")
            .icon(outline::Settings)
            .content({
                let panel = self.general_panel.clone();
                move |_, _| panel.clone().into_any_element()
            }),
        YueTabItem::new("appearance", "外观")
            .icon(outline::Palette)
            .content({
                let panel = self.appearance_panel.clone();
                move |_, _| panel.clone().into_any_element()
            }),
    ])
    .on_change(cx.listener(|view, request: &TabsChangeRequest, _, cx| {
        view.selected_tab = Some(request.key().clone());
        cx.notify();
    }))
```

`.on_close(...)` / `.on_add(...)` 不默认出现按钮；分别需要逐项 `.closable(true)` 与根 `.addable(true)`。content renderer 为可重复调用的 `Fn(&mut Window, &mut App) -> AnyElement`，不强迫调用方把任意业务视图换成 Tabs 专属 trait。

关闭回调签名为 `Fn(&TabsCloseRequest, &mut Window, &mut App)`。新增回调为 `Fn(&mut Window, &mut App)`，使用捕获的宿主 Entity 更新集合；不能直接套用要求事件参数的 `cx.listener`。`YueTabItem::panel_tab_stop(true)` 默认给面板一个可遍历入口；已有输入框 / 按钮等自然入口时，宿主显式设为 `false`，V1 不猜测任意子树是否可聚焦。

主题默认继承 `current_theme`，允许 `.theme(theme)` 局部覆盖。布局遵循现有 ControlLayout 风格，提供 width / min_width / max_width / height / flex_grow 等明确外部约束；内部字号、间距和热区仍由 token 决定。`header_width` 仅影响纵向头部。显式滚动定位以稳定 key 和 Auto / Start / Center / End 对齐表达，不暴露内部像素偏移作为持久化标识。

## 14. Token 来源与新增清单

外部唯一来源：`D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens\src\component-tokens\tabs.css`。已在 `_index.css` 登记，并通过 `tools/sync-yue-theme.mjs --write` 同步 inventory 和 generated.rs；不直接手写生成代码或在 renderer 里散落配色。

| 已登记 token 族 | 语义 |
| --- | --- |
| `--tabs-height-{sm,md,lg}`、`--tabs-font-size-*`、`--tabs-icon-size-*` | 成套密度与图标尺寸 |
| `--tabs-padding-inline-*`、`--tabs-gap-*`、`--tabs-max-item-width`、`--tabs-header-width` | 标签内部布局、横向标题上限、纵向宽度 |
| `--tabs-action-size-*`、`--tabs-close-icon-size-*` | 命令热区与关闭 glyph，不能混成一个尺寸 |
| `--tabs-color`、`--tabs-color-selected`、`--tabs-color-disabled` | 常规 / 选中 / 禁用前景 |
| `--tabs-background-hover`、`--tabs-background-pressed`、`--tabs-card-background`、`--tabs-card-background-selected` | 交互及 Card 表面 |
| `--tabs-border-color`、`--tabs-border-radius`、`--tabs-border-width` | 结构边界 |
| `--tabs-indicator-color`、`--tabs-indicator-size`、`--tabs-indicator-radius` | 强调线，厚度 2px，不增加第二条边框 |
| `--tabs-focus-ring-{color,width,offset}` | 明确的键盘焦点，不能被轨道裁掉 |
| `--tabs-duration`、`--tabs-ease`、`--tabs-indicator-duration`、`--tabs-scroll-duration` | 交互 100ms、线与滚动 150ms，别名到已有通用阶梯 |
| `--tabs-content-duration-{enter,exit}`、`--tabs-content-offset` | 已使用 enter 150ms、8px 位移及通用 enter 曲线；exit 100ms 为预留 token，V1 不绘制旧面板退出 |

颜色别名指向现有 text / border / surface / accent / focus 语义，覆盖明暗、accent 和 neutral 配置，不从参考图取固定蓝色。视觉长度随 rem 缩放，测量结果和裁切边界才转换为实际 Pixels。

## 15. 实施顺序与验收

1. **Base 复合行为**：保留现有 API，补稳定 key 模型、请求、roving focus、按轴导航、确认、禁用与删除后的焦点协调；先以无皮肤真实事件测试证明行为。
2. **面板边界**：实现 TabPanel role / 关系、唯一活动子树、隐藏焦点迁移、懒渲染及浮层拥有者清理；明确当前后端可验证范围。
3. **Token 与 Yue 外观**：先新增设计系统 token，再同步 Rust；接入 Normal / Card、四位置、三尺寸、图标与独立命令。
4. **滚动与动效**：实际几何、ensure-visible、固定命令区、指示线与当前进度反向；最后加入经失活测试验证的 Fade / Slide。
5. **Showcase 与回归**：新增“组件 → Tabs”页，公开 API doctest、消费例和完整回归；真实 UI 交由用户检查。

实际文件：`vendor/gpui-base/src/tabs.rs`、`tabs/{controller,panel}.rs`；`crates/yue-component/src/tabs/{mod,element}.rs`；`crates/yue-component/tests/tabs.rs`；`examples/windows-showcase/src/pages/tabs.rs`。Base 补丁记录、公共 re-export 和 README 随实现同步。

| 测试面 | 必须覆盖 |
| --- | --- |
| 受控选择 | 接受 / 拒绝、重复点击、外部 silent 更新、回调重入、selected 失效、空集合与 None |
| 键盘 | 两轴、Automatic / Manual、Home / End、跳禁用、首尾循环、只有一项 / 全禁用、Tab 顺序和子控件按键不被截获 |
| 动态集合 | 关当前 / 非当前 / 最后一项、取消关闭、增加后选中、重排后 key / focus 不变、陈旧邻项建议 |
| 命令隔离 | close 不切换，add 不成为 Tab，图标和文字同一次激活，禁用无任何用户请求 |
| 实际布局 | 四位置 × 两外观 × 三尺寸，长中英文、图标、closable、命令不重叠；关闭项与普通项无假空槽 |
| 滚动 | 两端禁用按钮、阈值不抖动、视口外 selected、超宽标签、用户滚轮不被拉回、缩放 / 改标题后重新测量 |
| 连贯动效 | A → B → C、中途反向、远跳降级、滚动不追赶、布局修订直接定位、reduced motion、主题 / disabled 清理 |
| 面板生命周期 | 未访问不构建、Entity 内容保留、退出子树不能命中或收键、唯一活动语义、焦点转移、子 Popup 清理、异步旧结果隔离 |
| 原生语义 | TabList / Tab / TabPanel、稳定关系、名称、位置、方向、selected / disabled、命令原生操作；记录缺失的后端关联能力 |

Showcase 已提供基础 / 卡片、图标、增删、三尺寸、四位置、25 项滚动及定位、禁用项、受控拒绝切换、动效配置、懒渲染与输入保持。交互预览使用 Segmented / Switch 切换配置；全局动效设置可检查 reduced motion。关闭拒绝、空集合、全禁用、快速切换、原生节点和缩放由 headless 测试覆盖。

验证记录见第 16 节。原生节点的双向关联仍受 GPUI 公共 API 限制；真实 UI 及 Windows UIA / 读屏效果由用户检查。

## 16. V1 实施记录（2026-11-02）

- 已实现 Normal / Card、四位置、三尺寸、可选图标、独立增删命令、受控选择及两种键盘激活模式。
- Normal 共享指示线近距离平移与变长 150ms；远跳、首次、几何修订直接定位。Card 保留不透明基础表面，在其上按准确 hover / pressed token 叠加反馈；相邻边与面板连接边只绘制一次。
- 已实现单轴轨道、固定滚动命令、按 key 定位、当前项可见性与用户滚轮后的滚动保持。命令不因 Tab 的字重 / 长度变化而重叠。
- 唯一活动面板；None 即时切换，Fade / Slide 只动画新面板入场。宿主持有的输入 Entity 保留，旧子树及其 Dropdown 随面板卸载退出。
- Panel 自身 tab stop 是显式配置；Tab ↔ Panel 原生节点 ID 关系未实现，限制见第 12 节。
- Tabs headless 测试 **23 项通过**：含真实输入事件、唯一活动树、四位置 × 两外观 × 三尺寸、Tab / Shift+Tab、显式 Panel 停靠、鼠标关闭后的焦点回收、指示线中间值 / 反向、快速切换、Dropdown 卸载、窗口缩小后重定位、1 / 1.5 / 2 倍 DPI 与 16 / 20 / 24px rem。
- Base Tabs 测试 **8 项通过**：保留 standalone 行为，验证受控语义、禁用 / 原生 Click、纵向 orientation、邻项建议、滚动边界对齐及超宽项分页。
- 工作区测试与 doctest、Clippy（`-D warnings`）、token 同步和限定文件 rustfmt 检查通过。独立预览为 `target/debug/windows-showcase-tabs-preview.exe`，未自动启动；真实 UI 和读屏效果未代用户验收。

构建与回归命令：

```powershell
node tools/sync-yue-theme.mjs --check
cargo test -p yue-component --locked --features headless-tests --test tabs
pwsh -NoProfile -File tools/test-gpui-base.ps1 -Offline -Filter 'tabs::'
cargo test --workspace --locked --features headless-tests
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
```
