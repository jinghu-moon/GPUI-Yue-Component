# 28 虚拟滚动基础层与 VirtualList 设计

状态：阅读锚点、Attached 宿主边界与 Masonry 输入所有权已修复；Measured 原生累计树与偏移已接通 f64，portable cache 为 schema 2。新增 Masonry 可选 View 准备缓存、原生稀疏测量导出／宽度失效与分块快照追加；逐项收益、代价及拒绝方案见[本轮实测取舍](research/2026-11-04-virtual-list-adoption.md)。单项 Pixels 等剩余精度边界见第 13.3 节，实窗仍交由用户验收。更新：2026-11-04。

继续对标已采用 Masonry 测量代次失效与原生有界尺寸批次；source-slot 批量冷构造因重复热反馈退化被撤销，整批树反馈改为每八项提交以控制峰值。前后数据、剩余代价和 Git 回退点见[失效／冷构造／批量反馈报告](research/2026-11-04-virtual-list-continuation.md)。

实现入口与初次验证见 [实现、回归与性能记录](research/2026-11-03-virtual-list-implementation.md)，1–8 项能力补齐与性能取舍见 [升级记录](research/2026-11-03-virtual-list-upgrades.md)。本文的 API、默认值和支持矩阵已同步到当前源码；物理 GPU 帧节奏、实际视觉及 Windows 屏幕阅读器体验仍由用户检查。

升级后的 [第二轮跨库对标](research/2026-11-04-virtual-list-cross-library-review.md) 补充 virtua、vlist 的可借鉴机制。随后完成三模式内容失效、定位策略请求校验、内容与几何准备分离、增量 splice／Masonry append、紧凑持久索引、顺序工作集迭代及混合轨迹／多周期释放验证，见 [第二轮实施与测量](research/2026-11-04-virtual-list-refinements.md)。跨库对标中的问题与数值是修复前快照，不代表当前状态。

[第三轮对标](research/2026-11-04-virtual-list-followup-review.md) 复现的六项问题已完成[修复与回归](research/2026-11-04-virtual-list-followup-fixes.md)：内容失效跨结构事务保留有效身份、Masonry 呈现与校正预算分离、非空内容失效拒绝旧反馈、缓存验证实际跨轴约束、无位移 Smooth 正确收尾、Grid 单轴元数据只准备一次。原始调研保留修复前事实，当前状态以实施记录为准。

[第四轮对标](research/2026-11-04-virtual-list-owner-review.md) 保留修复前的 68 条观察。随后完成[阅读位置与输入所有权修复](research/2026-11-04-virtual-list-owner-fixes.md)：三种区域锚点、离屏宿主保护、Masonry 最新位置采样／用户接管、单一原生 ScrollHandle 的 f64 偏移和 Fixed/Known 局部绘制验证；portable cache 的 live／持久身份合同同步到源码。性能结论与尚未消除的精度边界以实施记录为准。

Showcase 已增加八个按需挂载的场景：百万记录、分组记录、动态卡片、横向时间轴、多列卡片、编辑保留、追加日志和空状态。快速滚动条拖动的原生输入回归、debug/release 采样与实窗 CPU 观察的边界见 [Showcase 扩展与拖动采样](research/2026-11-03-virtual-list-showcase-drag.md)。

另设「虚拟布局」页面，按需挂载百万行 × 千列的单元格 Grid、万张动态卡片的稳定分栏 Masonry、两千节点的横向 Measured 时间轴；不与原八场景同时构建。原动态卡片使用工作集内的 item view 复用昂贵准备。

本文定义本项目虚拟滚动基础层及其首个一维呈现 VirtualList 的职责、headless 接口、原生复用路径、滚动与测量合同、性能边界和实施验收。源码依据与逐库分析见 [虚拟列表源码调研记录](research/2026-11-03-virtual-list-source-review.md)，目录及依赖遵循 [组件架构与目录规范](24-component-architecture.md)。

**核心决定：在 GPUI 已有三种列表路径上建立按主轴表达、可借用宿主滚动容器的 Base 虚拟化接缝；VirtualList 是一维便利适配，Yue 提供外观与默认列表语义。复用原生几何和测量，不再实现一套滚动引擎，也不把选择、搜索、分页或行样式作为前置条件。**

项目尚未正式发布。必要时直接调整 Base 接口、重构 Select 消费路径和补丁源码，更新所有仓库调用方并删除替代实现，不保留仅为兼容旧设计的壳。每个行为变更都须有改动前基线、能证明新合同的测试和改动后回归证据。当前实现迁移了 Select，删除旧的 v/h 构造器与尺寸数组入口；性能结论以实现记录中的原始样本和测量范围为准。

## 1. 从基本事实推导需求

### 1.1 要解决的成本

设逻辑条目数为 N，真正与视口相交的条目数为 V，预先测量／呈现的缓冲条目数为 B，因焦点、编辑或自定义策略额外保留的条目数为 P（重复项只计一次）。

1. 窗口每帧只能展示有限区域，构建 N 个复杂 Element、排版 N 行文本和解码 N 张图不会增加当前可见信息。
2. 要知道哪些行可见，必须知道或估计行的位置。固定高度可直接计算；可变高度需要尺寸信息；动态内容必须由实际布局反馈校正。
3. index 是当前顺序，业务身份不会因前面插入一行而改变。保留状态与位置不能只依赖 index。
4. 上方内容增删或改变高度会改变累计位置，用户正在看的对象不应因此意外移动。
5. 滚轮、滚动条、键盘和程序定位最终都改变同一个视口，必须有一个滚动所有者。
6. 一帧的测量、偏移和绘制必须对应同一份数据与几何；优化通知或分配不能破坏这个一致性。
7. 虚拟化减少参与工作的条目，不自动减少单个条目的内部元素、文本排版、解码和绘制成本。条目数量与条目复杂度是两个独立的性能维度。

由此得到最小职责：**按源坐标寻址、按需构建、尺寸管理、范围计算、滚动命令、锚点恢复及与原生焦点/可访问性的连接。** 不从“几个组件看起来都是列表”推导万能 List。

### 1.2 成本目标

- 稳定滚动只构建工作窗口 `W = V + B + P`，不得每帧遍历／复制 N 个业务对象。
- 固定高度不为每项分配 Size、offset 或 FocusHandle。
- 已知可变高度允许 `O(N)` 紧凑尺寸／前缀数据；动态列表允许原生 `O(N)` 元数据。二者均须报告真实内存，不能称为恒定内存。
- 宿主数据、虚拟化元数据、Element/文本/图片、GPU 资源分别分账。虚拟化渲染不等于虚拟化数据加载。
- 冷启动、数据更新、尺寸失效和稳定滚动是不同负载，不能用其中一个证明其余全部高效。
- 工作集有界不等于每帧成本恒定；还须计量工作集内各条目的实际工作量，以及估算、预处理和资源准备是否提前扫描了全量数据。

## 2. 使用范围与边界

### 2.1 当前交付范围

基础层交付可组合的主轴虚拟化机制。Fixed、Known、Measured 均支持纵／横轴的 Owned 与 Attached；YueVirtualList 提供纵向外观。VirtualGrid 组合两根已知尺寸轴，VirtualMasonry 提供独立的稳定分栏呈现；不让一维列表承担所有布局策略。

| 尺寸模式 | 适用内容 | 原生路径 |
| --- | --- | --- |
| Fixed | 主轴尺寸一致的历史记录、命令项、缩略图行 | 共享 GPUI `uniform_list_range` 算术，两轴均为常量元数据 |
| Known | 已知主轴尺寸的混合条目、分组或列 | 紧凑叶块与持久 SumTree；f64 累计，支持纵／横轴与局部更新 |
| Measured | 换行文本、图片加载、展开详情等动态条目 | GPUI `ListState`，同一测量／锚点／预算算法投影至两轴 |

三条路径对应三种不同的已知事实，不是三个可互换插件。Fixed 不支付逐项尺寸元数据成本；Known 不强迫 renderer 测量全量；Measured 不要求调用方先算出未知高度。基础层统一使用合同，内部保留各自最便宜的原生路径。

共同合同包括稳定 key、有限视口、空集合、变更事务、主轴定位、内容与布局失效、范围快照、事件和焦点安全。每项支持都以第 2.5 节矩阵与测试为准，不能只因具有同名方法就宣称行为一致。

### 2.2 不进入虚拟化核心

- 业务筛选、排序、网络请求、页缓存、请求重试、数据库查询。
- 单选/多选规则、树展开、网格列模型、表头、拖拽排序。
- 通用 ListItem 视觉模板、Toast 堆叠与退出顺序。
- 任意二维空间索引、轮播无限 lap、反向索引布局；瀑布流分配属于独立的 VirtualMasonry 适配，不进入一维 state。

这些能力可复用身份、失效、锚点和滚动所有权等机制；一维连续 range 只表达一根轴上的有序区间。二维单元格工作集由组合者求两轴交集，瀑布流另外定义分配和范围策略，不借一维接口假装已经覆盖。

### 2.3 与其他组件的关系

| 消费者 | 关系 |
| --- | --- |
| Select / 命令面板 | 消费主轴范围与定位；拥有搜索、active/selection、选项语义和提交合同 |
| 普通 List / 卡片流 | 使用一维便利适配与任意 renderer；无需先依赖统一 ListItem 模型 |
| Toast | 数量有界且生命周期、堆叠动效特殊，不迁入虚拟列表 |
| ImagePreview / 图片集合 | 长集合可消费范围；单图预览、解码与纹理预算仍属资源所有者 |
| Tree / Table / Grid | 独立定义模型到主轴/单元格的映射；可直接消费 headless 结果，无需嵌套两个 VirtualList |

### 2.4 基础层、视口、控制器与呈现布局

“虚拟化视口”描述的是使用场景。基础层的职责是可组合的主轴虚拟化；viewport 是既有滚动容器的几何输入，不新增一个拥有镜像滚动状态的 Viewport Entity。

| 概念 | 本项目的职责边界 |
| --- | --- |
| Viewport | 可见区域、裁剪后的主/跨轴可用空间和已应用滚动位置；由唯一滚动所有者提供 |
| VirtualizerState | 一根主轴上的 source、几何/测量、工作集、失效、锚点和定位；不附带默认 role、滚动条或容器 Element |
| VirtualList | Base 一维便利适配，将 state、原生布局和 renderer 接到一个滚动区域；可选列表语义由适配层设置 |
| YueVirtualList | 提供纵向单列、Yue 外观和默认 list/list-item 语义；允许由 Select 等语义所有者覆盖 |
| 自定义布局呈现 | 直接消费主轴快照与测量接缝，拥有跨轴位置、单元格组合和语义映射 |

TanStack 的两个轴共享滚动容器、多个 lanes 沿主轴分配条目的做法，说明应复用共同事实，不应强迫所有布局继承 VirtualList。动态网格还须定义行高如何受未呈现列影响；当前可见单元格最大高度不能无条件当作最终行高。

第 15.6 节的共享宿主合同已落地为 VirtualGrid 与生产回归；另有真实动态布局的 VirtualMasonry。它们是布局原语，未提供 Table 的列编辑、排序、树模型或业务选择；不预建插件注册表或通用布局运行时。

### 2.5 能力矩阵与失败合同

下表对应当前实现和生产接缝测试。Owned 表示一维适配委托对应原生路径拥有滚动；Attached 表示只借用既有宿主视口，状态只计算几何和提出定位/校正意图，最终由宿主应用。

| 尺寸模式 / 主轴 | Owned | Attached | 当前实现 |
| --- | --- | --- | --- |
| Fixed / Vertical | 支持 | 支持 | 共用原生半开区间算术，无需 renderer 查询 |
| Known / Vertical | 支持 | 支持 | 共享持久树与版本化定位合同 |
| Known / Horizontal | 支持 | 支持 | 物理 x 主轴，独立命令合并时保留 y |
| Measured / Vertical | 支持 | 支持 | 原生 ListState 测量与同帧校正 |
| Fixed / Horizontal | 支持 | 支持 | 相同 fixed 算术投影，无 N 项尺寸数组 |
| Measured / Horizontal | 支持 | 支持 | definite height + intrinsic width；真实布局、命中及绘制坐标不旋转 |

| 能力 | 当前合同 |
| --- | --- |
| 精确已知的主轴范围、总尺寸和单项位置 | Fixed/Known 可无 renderer、无主题、无可访问节点查询；不调用代表项测量来取得已知事实 |
| 动态尺寸 | Measured 在实际测量前明确标为 estimated；读取快照本身不构建内容或测量全量 |
| stable key、splice/reorder/replace、局部失效、显式对齐、焦点安全 | 上表所有必须支持的组合均须通过对应测试；不以 backend 差异静默降级 |
| 尾部跟随 | 三种模式、两轴均支持；Measured 复用原生 FollowMode，已知尺寸由同一 state 产生尾部请求；用户离开尾部暂停，实际返回后恢复 |
| smooth | 仅在路径尺寸已知、距离有界时允许；不满足时按第 9 节记录的规则即时定位，不冻结全部沿途项 |
| 借用同一宿主的双轴组合 | VirtualGrid 使用两根 Fixed/Known 轴；同 host、同 viewport revision、配对的主／跨轴空间；一个 ScrollHandle 一次合并提交 |
| 动态网格行高 | 不依据当前可见列猜测全局行高；调用方显式提供独立的行／列尺寸 |
| 瀑布流 | 独立 VirtualMasonry，支持估算与真实测量；resize／append 保留已有分栏，完整结构替换或显式 reflow 可重排 |
| 方向 | 首版索引沿物理主轴正方向增长；反向布局/RTL 镜像定位另行设计，不由语言环境隐式启用 |

构造、绑定或启用能力时校验 `(axis, sizing, attachment, requested_capability)`，通过只读 `VirtualizerCapabilities` 查询支持情况；unsupported 返回明确类型化错误。校验先于 N 级分配、挂载和滚动写入，失败保留旧有效状态，不悄悄换 sizing/backend、忽略选项或制造兼容壳。增加支持必须同时更新矩阵、原生补丁、前后基线与组合测试。

## 3. 调研结论如何进入设计

| 参考对象 | 采用 | 不直接移植 |
| --- | --- | --- |
| TanStack Virtual | headless 接缝、stable key、测量批次、几何/内容更新分离、明确条目粒度、可重求的定位目标、写回/用户输入区分 | DOM observer、Proxy、iOS gesture hack；不复制所有选项或另造通用 Viewport |
| virtua | layout/store/driver 的职责分离、像素缓冲、事件按用途去重、锚点校正 | 第二套 action/store runtime、长距离冻结完整沿途范围、滚动时全局禁用 pointer |
| vlist | fixed fast path、紧凑几何、两阶段一致性、工作集预算、身份重绑定校验、内存测量纪律 | DOM 池、插件总线、接管 Wheel、浏览器高度上限与 synthetic scroll |
| 本地 GPUI / Base | 现有 fixed/known/measured 引擎、ListOffset、SumTree、ScrollbarHandle、FocusHandle、AccessKit 节点 | 不再叠加第四种引擎、不另建 Root、不启用 Kit Component |

源码调研纠正了两个容易误判的点：

1. Base 已知尺寸列表要求外部提供尺寸，**不代表 GPUI 缺动态列表**；本地 `ListState` 已有重测、hint、尾部跟随与离屏焦点项能力。
2. headless、低分配和恒定内存是不同命题。vlist 管理 DOM，variable cache 为 `O(N)`，普通静态入口复制数据引用数组；其宣传数字不能直接成为 Rust 组件承诺。

完整的文件、符号和证据限制见 [调研记录](research/2026-11-03-virtual-list-source-review.md)。

## 4. 分层与唯一所有者

```text
宿主 / Select / 自定义布局
  ├─ source 索引快照、业务状态、语义映射、renderer
  ├─ 一维便利路径：YueVirtualList → Base VirtualList
  │    └─ Owned 绑定：委托一个原生滚动所有者
  └─ 自定义组合路径：既有滚动容器 + 主轴快照/测量接缝
       └─ Attached 绑定：同一宿主可组合 x/y 两个主轴状态
            └─ Entity<VirtualizerState>，每根主轴一个
                 ├─ source、事务、锚点、命令、失效、语义无关事件
                 └─ 私有 backend：uniform 几何 / known-size 几何 / ListState
                      └─ GPUI layout、焦点、prepaint、paint
```

| 事实 | 唯一所有者 | 禁止的镜像或越权 |
| --- | --- | --- |
| 数据、顺序、业务选中值、请求 | 宿主 | Base 不 clone 全量数据或自动提交选择 |
| index↔key 映射 | source 索引快照 | controller 不全量建立同义 key Map |
| 尺寸、前缀/树、估算状态 | 对应 backend | Yue 不维护第二份高度/宽度表 |
| 实际滚动、输入、惯性、拖拽与实际 clamp 结果 | 当前绑定指定的原生/宿主滚动所有者 | Attached state 不注册 wheel，不自行 set_offset；Owned 不再外套另一个 owner |
| 待定位意图、generation、跨快照锚点 | 每轴 Base state | 不把缓存的 viewport snapshot 当成第二份可任意写的 offset |
| 业务编辑值和展开值 | 宿主按 key 保存 | 不存入可卸载的临时 Element |
| role、active/selection、逻辑语义计数 | 具体语义组件或一维适配 | 原始 state 不强加 list/listbox/grid |
| 外观、滚动条、动效策略 | Yue / 消费者 | Base 不决定品牌颜色、密度、默认显示时机或动画时长 |

`VirtualizerState` 是 retained 适配所有者，不是新布局引擎。一个实例的 backend 互斥；source/state 与无样式 Element 可以分别被消费，外部容器路径无需先挂载 VirtualList 才能获得几何，也不额外套一个同义列表 state。

Owned 绑定让现有原生状态继续拥有其逻辑位置；例如 Measured 仍用 ListOffset，不复制 scroll_y。Attached 绑定让宿主拥有已应用位置，backend 由本帧输入推导逻辑位置并提出校正；缓存的 index/行内偏移只是同 revision 的派生结果，不能再独立接收滚轮或写回。大坐标仍按第 13.3 节保留逻辑/局部坐标精度，不用低精度绝对 Pixels 来回覆盖它。

两个主轴状态共享容器，不等于两个 Element 同时 track_scroll 同一 handle。宿主一次提交内容宽高、一次处理输入与偏移；每轴只提供自己的范围和定位意图。现有公开类型在同一迁移批次直接更新，不保留新旧状态互相同步的兼容层。

## 5. 公共接口设计

**本节对应已落地的 API。** Base 从 `virtualization` 模块与根路径导出；Yue 从 crate 根与 prelude 导出 `Virtualizer*` 和 `YueVirtualList`。原始 `VirtualList` 可从 `gpui_kit::base::virtualization::VirtualList` 导入。

### 5.1 最小公开类型

| 类型 | 合同 |
| --- | --- |
| `VirtualizerSource` | 条目数、index→key、key→index；不要求业务对象实现组件 trait |
| `VirtualizerSizing` | Fixed/Known/Measured 三种互斥尺寸输入，统一使用主轴 extent |
| `VirtualizerState` | 每主轴 retained state；持有 backend 与变更/定位合同，不内置 UI 容器 |
| `VirtualizerViewport` | 不可变的本帧视口输入值：binding/viewport/input revision、主轴位置与可用空间；不是 Entity 或新滚动宿主 |
| `VirtualizerSnapshot` | 带版本的只读范围、主轴几何与估算状态；不生成全量 VirtualItem 对象 |
| `VirtualizerCapabilities` | axis/sizing/binding 对应的支持结果；用于显式校验，非可任意组合的插件开关 |
| `VirtualizerSizeUpdate` | 稳定 key + 主轴 extent，批量实际尺寸或估算更新；整批校验、过期快照拒绝 |
| `VirtualizerRange` / `VirtualizerRangeExtractor` | 连续范围以外的工作集策略；呈现方式由消费者拥有 |
| `VirtualizerCache` / `VirtualizerMeasurement` | 可导出恢复的纯值尺寸、内容/布局身份及 key 锚点；不持有旧控制器 |
| `VirtualGrid` / `VirtualGridSnapshot` | 两轴单元格组合，独立尺寸输入，一份宿主 offset |
| `VirtualMasonryState` / `VirtualMasonry` | 稳定分栏与动态测量，显式 reflow |
| `VirtualizerChange` | source 变更事务：splice/reorder/replace |
| `VirtualizerItemIdentity` | 昂贵内容准备所需的 index/key；不暴露帧几何 |
| `VirtualizerScrollOptions` | key 定位、主轴 start/center/end/nearest、motion 意图 |
| `VirtualizerScrollRequest` | 本批同步返回的主轴定位/校正值；含请求身份与有效版本，只由唯一滚动 owner 应用 |
| `VirtualizerEvent` | 提交后的 range、scroll-end、命令终态与诊断，不充当本帧几何校正的异步通道 |
| `VirtualizerError` | unsupported、非法 source/几何、绑定冲突、过期快照/请求与已解绑等类型化错误；失败无部分提交 |
| `VirtualList` | Base 一维 Owned 便利适配；Attached 消费快照与 `layout_measured`，不挂载另一个滚动 Element |
| `YueVirtualList` | Yue 的纵向单列 RenderOnce 组合；不新增镜像 state |

公开 struct 字段默认私有，提供构造、builder 和 reader；配置/绑定错误返回类型化错误，内部 backend 和缓存不成为消费方依赖。复用 GPUI `Axis`、`ElementId`、Bounds/Pixels 等现有类型，不增加同义 VirtualAxis/ItemId。新增加的 viewport/snapshot 是表达现有事实的值，不各自再拥有缓存、输入监听或生命周期。

行为与几何类型统一使用 Virtualizer*，不专属于列表；VirtualList 保留给一维呈现适配。当前 API 尚未正式发布，职责与命名直接在唯一入口演进，不增加同义别名。

### 5.2 source 合同

已实现的只读能力为：

```rust
pub trait VirtualizerSource: 'static {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn key_at(&self, ix: usize) -> ElementId;
    fn revision_at(&self, _ix: usize) -> u64 { 0 }
    fn index_of(&self, key: &ElementId) -> Option<usize>;
}
```

- `revision_at` 在准备内容或尺寸相关内容变化时递增；默认 0 适用于不变内容。调用方按 key 更新内容却不更新版本时，应显式 invalidate_items 或修改外部 content_revision。
- source 是同一 generation 内不变的**索引快照**，可以共享宿主数据与既有索引；不要求转换为 `Vec<ListItem>`。
- `key_at` 的前置条件为 `ix < len()`；有效项必须有唯一、稳定 key。`index_of(key_at(ix)) == Some(ix)`。
- 两种查询由宿主声明并保证成本，目标为 `O(1)` 或 `O(log N)`；不接受滚动路径每次线性扫描整个数据集的默认实现。固定行高可以按规则生成 key，不必枚举百万个字符串。
- source 不向 controller 回调可变宿主，不执行 I/O，不因查询启动加载。renderer 另由宿主提供，按 key 从当前数据模型取内容。
- 全量 key 唯一性在 fixture/数据构建测试中验证；生产热路径只检查所访问窗口及锚点的一致性，不每帧扫 N 个 key。
- 未加载槽位若计入 len，必须有独立的 `(query_generation, slot)` 占位身份；加载后更换为真实业务 key。不得把占位 index 当成所有查询都稳定的业务身份。

source 只保留长度、双向身份查询和可选的逐项内容版本，不引入 RemoteDataSource、SelectionDelegate、LayoutPlugin 等平行协议。

### 5.3 构造与状态更新

状态在宿主初始化时创建，render 只组合适配或消费本帧结果：

```text
初始化：source + sizing + Axis + 滚动绑定 → 校验 capability → cx.new(VirtualizerState)
Owned：VirtualList / YueVirtualList(&state) + render_item(key, ix, window, cx)
Attached：读取宿主 viewport → 求 snapshot → 消费/测量工作集 → 宿主统一提交几何与 offset
数据改变：replace_source(next_source, next_sizing, change)
内容改变：invalidate_items(range) / invalidate_items_from_snapshot(&snapshot, range)
布局改变：set_layout(VirtualizerLayout::default().revision(layout_revision)...)
尺寸：update_sizes / update_size_hints / update_sizes_from_snapshot → 校验整个批次后提交
定位：scroll_to_key / scroll_to_offset / scroll_by → 对应绑定的唯一滚动 owner 应用
```

以上方法同步更新状态；Entity owner 在影响呈现的更新后调用 `cx.notify()`。Owned 的适配自动观察 state；Attached 由宿主固定本批版本并完成提交。三个消费者的可编译实现分别见 Showcase、Select 和 `tests/virtual_list.rs` 的 SharedHost。支持性校验不推迟到首个滚轮事件。

`replace_source` 把 next source、sizing 与变更说明作为一个事务验证并提交，不能先改 count、下一帧再改几何。一个 state 的轴、尺寸模式与滚动绑定固定；Fixed extent、Known 数组或 Measured 估算可更新。切换配置显式验证并重建，按需要移交锚点，解绑旧输入/命令，不做隐式模式猜测。

`invalidate_items(range)` 只解释当前 generation；异步反馈先核对 binding、source generation 和内容/布局版本，再按 key 求当前 index。`scroll_to_key` 也由 source 判断目标是否存在。

显式内容失效对 Fixed/Known/Measured 均生效；以区间边界记录局部 epoch，不逐项枚举整个失效范围。非空失效使先前 snapshot 的迟到写入过期；空区间和非法输入不改变版本。source 事务清理非空旧坐标区间时，提升单调的 base epoch 作为一次准备缓存屏障，不能回到旧 stamp。该屏障保守刷新当前工作集；后续没有新内容失效的 append 不再全工作集刷新。缓存仍按 key/source revision/index 验证，避免把旧 index 的 epoch 移给另一项。宿主在 coherent mutation 后 notify 一次。

更新先校验，释放内部可变借用后再通知；回调不得在 RefCell/Entity 可变借用内重入。失败保留上个有效状态。owner 替换 Attached state/绑定或卸载后，旧批次、校正和命令结果不能继续写入原宿主；不是让原 state 隐式改绑另一个视口。

### 5.4 视口、呈现与测量接缝

- 有限主轴空间由 owner 提供；无可用视口时不请求全量条目求布局。跨轴可用 extent 必须明确，不能在热路径扫描最宽/最高条目。
- Fixed/Known 的几何查询只需已知输入，不依赖 renderer、Window 文本测量、主题、role 或 VirtualList Element。读取同版本 snapshot 不触发业务 notify。
- Measured 快照可先返回估算；实际测量经 GPUI 正常布局阶段执行。renderer 提供工作集中条目根，原生测量结果写回同一尺寸 owner；不复制整棵行树来测一份、再画一份。
- 自定义呈现可直接消费 snapshot 和测量接缝；Base VirtualList 只是常用适配。Attached 路径不注册第二份 wheel、惯性、Scrollbar 或默认 role，也不借创建一个隐藏滚动列表获取测量。
- renderer 输入当前 source key/index；可以因测量、焦点或原生布局多次调用，不执行 I/O、提交业务或重复订阅。
- 一维适配负责条目根命名空间与定位；自定义组合按 key 或单元格 `(row_key, column_key)` 派生身份。renderer 的内部布局不反过来修改虚拟槽位的主轴 origin。
- 快照与请求标记 binding、source generation、geometry 和 viewport/input revision；迟到测量和指针提交核对有效身份，不能只因 index 相同就复用旧结果。

首版 Shared host 测试必须直接消费两轴结果；禁止用两个各自拥有 overflow 的 VirtualList 嵌套来通过测试。原生接缝在原有所有者提取相同算法，Owned 与 Attached 共用，不能把另一套前缀/测量/焦点树放到 Yue。

### 5.5 默认值

| 项目 | 基础层 / 适配层默认 |
| --- | --- |
| 轴与模式 | 初始化时显式指定；不猜测 |
| 方向 | 索引沿物理主轴正方向增长 |
| 容器绑定 | 明确 Owned 或 Attached；一份实际滚动真值 |
| 外观与语义 | 原始 state/snapshot 无 role、颜色、滚动条和行树；YueVirtualList 默认纵向单列与 list/list-item，可由语义组件覆盖 |
| 选择／active item | 无，由语义层拥有 |
| 定位 | nearest、instant；目标已满足可见条件则 no-op |
| 尾部跟随 | 关闭；开启前校验 capability |
| 滚动条 | 原始机制不创建；一维适配复用 Base Scrollbar，显示策略由 Yue/消费者设置 |
| inset / gap | 基础几何默认零，Yue 不暗加卡片 padding 或行间距 |
| 空状态 | 空结果；业务文案由消费者 slot 提供 |
| 条目进入／退出动效 | 无；回收不视为新业务项 |
| 范围 | 一根轴上的半开区间 `[start, end)`，附加焦点／retained／extractor 项由 working_indices 排序去重 |

### 5.5.1 当前可编译入口与提交责任

宿主提前创建 `Rc<dyn VirtualizerSource>`，在初始化而非 render 中保留 Entity：

```rust
use gpui_kit::{Axis, Entity, px};
use yue_component::{VirtualizerSizing, VirtualizerState, YueVirtualList};

let state: Entity<VirtualizerState> = cx.new(|_| {
    VirtualizerState::new(Axis::Vertical, source, VirtualizerSizing::fixed(px(36.)))
        .expect("supported source and geometry")
});
```

render 组合 `YueVirtualList::new("records", &state, |item, _, _| render_record(item))`；父容器提供有限高度。`render_record` 返回任意 GPUI Element，条目根的稳定 key 由适配层建立。真实可编译示例在 `examples/windows-showcase/src/pages/virtual_list.rs`。

外部宿主可以不创建列表 Element：

```rust
let binding = VirtualizerBinding::new();
let mut state = VirtualizerState::new_attached(
    Axis::Vertical, source, VirtualizerSizing::fixed(px(36.)), &binding,
)?;
let viewport = VirtualizerViewport::new(&binding, revision, offset, height, width)
    .input_revision(input_revision);
let (snapshot, request) = state.prepare(&viewport, now)?;
```

宿主消费 snapshot；有 request 时先 `validate_request(&request, &viewport)`，合并其他轴、clamp 并写入唯一原生 owner，再以实际应用值 `confirm(&request, applied_offset, &viewport)`；无 request 时 `commit(&final_viewport)`。Measured 调用 `layout_measured` 后重取快照/请求，完整的同帧两次测量和双轴合并范例见无头测试的 `SharedElement`。Attached 的用户输入、结束会话和帧调度由宿主拥有，宿主在真实输入后调用 `user_scroll_activity(cx)`，并消费/转发 `take_events()`。

### 5.6 虚拟化粒度与复杂条目

source 中的一个 key 对应一个 renderer 根、一个主轴尺寸及一个定位槽位。Base 不递归解析其子树，不自动拆分卡片，也不把内部子元素裁剪或不 paint 等同于避免构建、布局和资源准备。

- 一张卡片包含多个按钮、文本块或图片是合法用法；它进入工作集后，这些内容的真实成本仍由行内容所有者承担。
- 单张超高卡片包含数千条记录时，应评估由业务语义层将标题、分组和记录展平为独立可寻址条目。各自使用稳定 key，并明确组关系、逻辑计数、焦点和阅读锚点；Base 不为此要求通用 Card/ListItem trait。
- 内部只有在确实拥有独立有限视口和滚动语义时才另建虚拟区域；不能仅为省事套同轴滚动容器，造成两个滚动所有者争抢同一次操作。
- 不把内部控件数阈值写成基础层自动拆分规则。若实测巨型条目仍昂贵，先调整粒度、排版和资源策略，而非无限增大 overscan 或保留全部历史 Element。

### 5.7 主轴坐标与锚点统一表达

基础合同复用 GPUI Axis，以 `item_extent`、`main_offset`、`viewport_extent`、`cross_extent`、`padding_start/end` 表达几何；纵向适配才映射为 height/y/top/bottom，横向适配映射为 width/x/left/right。

- 索引递增对应内容坐标递增；内容 origin、主轴 start/end 与对齐定义不因轴改变。
- 公共 main_offset 是从内容逻辑起点向后的非负距离。标准 Div 的绘制 offset 方向相反；符号、padding 与局部原点换算只在 binding 边界处理一次。不得在 renderer、锚点和 scrollbar 各做一次取反。
- 先扣除实际不属于内容视口的表头/固定区域、inset、gutter，再提交可用几何；host 不让每个 state 猜测其父级布局。
- 锚点为 `(key, VirtualizerAnchorPosition)`；`Before(distance)`、`Within(distance)`、`After(distance)` 分别表示内容起点前、内容内、内容末端后的逻辑像素距离。逻辑坐标跨变更保存，绝对值与视口附近局部绘制坐标分开。
- 仅非活动轴滚动时，不失效本轴尺寸/范围；跨轴可用 extent 改变会使依赖换行的 Measured 失效。DPI/rem 与字体失效继续遵循第 7 节。
- start/center/end/nearest、边界 clamp、短距离 motion 与尾部均按主轴解释；能力受第 2.5 节限制，方向中立不等于已实现所有轴/模式组合。

### 5.8 快照与外部容器提交

`VirtualizerViewport` 是由当前滚动 owner 提供的不可变输入：binding 身份、viewport/input revision、主轴 offset、主轴和跨轴可用 extent。它不保存第二份可写 scroll state。读取实际原生 offset 或逻辑位置的转换属于对应 binding。

`VirtualizerSnapshot` 提供只读 axis、source generation、layout/geometry/viewport revision、visible/rendered range、独立 retained keys、total extent、`is_estimated()` 状态，以及按 index/key 获取工作项主轴 start/extent 的能力。`items(range)` 在遍历前拒绝非法范围，逐项返回 key 校验结果；`working_items()` 顺序遍历工作集，Known 在连续区间只 seek 一次，远端 retained 项直接跳转而不经过中间行。Fixed 按需算术求值，Known 共享持久树，Measured 查询原生树；不为了快照枚举 N 个对象，也不要求复制每个业务对象。

同一 snapshot 的范围、位置和总尺寸必须来自同一版本；几何读取要么共享不可变版本，要么校验版本后拒绝过期查询，不能从旧快照悄悄读到新树。Measured 未测项带估算标识。snapshot 不自动创建控件、测量全量或启动请求；过期快照不能用于可变提交，测量回调另核对 content/layout/binding 身份。

总尺寸的 estimated/settled、单项是否已有当前版本测量，以及定位命令是否收敛是三个不同结论。目标的实际对齐已稳定即可完成该命令，不等待所有离屏项测量；也不能据一次定位成功把整个 total extent 标为精确。

外部容器的一次提交顺序为：

1. owner 固定本批输入与可用区域，对需要组合的轴读取同一份实际 viewport；每轴 source 可独立更新，但本批版本组合必须确定。
2. 每轴取得范围/几何；消费者构建工作项。二维组合用行列结果创建单元格，单元格身份包括两个 key。
3. 必要测量写入原生尺寸 owner，重新求受影响目标。各轴同步返回含绑定、请求身份、source/geometry/viewport revision 的 VirtualizerScrollRequest，不直接 set_offset；没有需要校正的目标则不产生请求。
4. owner 合并两轴总尺寸与偏移意图，保留未改变的轴分量，按新内容范围统一 clamp，再通过原生接缝提交一次并读取实际应用结果。
5. 如 offset 或几何改变，重新取得同一最终 viewport 下的轴结果，再 prepaint/paint 并合并事件。原生允许的再布局次数和测量工作量有界；未收敛时只提交完整一致的估算几何并继续下一帧，不混合新尺寸与旧坐标。

输入 revision 已变、数据已替换或 binding 已解绑时拒绝旧意图。两个命令同时更新不同轴可以合并；同一绑定同一轴仅一个主动控制者，冲突显式报错，不按最后注册者静默覆盖。一个 state 也不能同时绑定两个视口；需要时使用共享 source 的两个独立 state。绑定冲突只在该宿主的轴槽位检查，不新增全局注册表。

校正请求是本批计算结果，必须在 owner 应用前校验并在应用后确认实际结果；同一请求至多接受一次，不能重复累加 delta。它不经 RangeChanged 或帧后语义事件绕回下一帧。过期/拒绝的请求不改 offset；后续计算可以产生新版本请求，但仍受命令收敛与工作预算限制。

Owned 路径由原生 owner 完成等价提交，不能为了接口一致强制复制额外 offset。以上值类型、事件和提交接缝须有清晰 reader/结果合同，不引入自定义 driver trait 注册表、插件总线或全局协调器。

### 5.9 扩展布局与恢复合同

- `VirtualizerViewport::with_content_origin` 是列表起点在宿主内容坐标中的位置；`with_scroll_extent` 是整个宿主的可滚动内容总长。两者分别处理列表前的 header 和列表后的 footer。native Measured 只测量列表覆盖的可见区域，不把宿主 footer 的 offset 钳回列表底部。Attached 的视口起始边缘在列表覆盖区间之外时清除该列表阅读锚点与旧锚点校正；无显式定位／尾随时，结构更新不抢占宿主。列表内部 padding/gap 仍是合法阅读位置，不能以 visible_range 是否为空代替区域判定。
- `VirtualizerLayout::scroll_padding(start,end)` 是定位避让区域，与真实内容 padding 分开；Start/Center/End/Nearest 共用可用区域。Base 在轴边界把 logical padding 转成 physical edges，native `layout_for_viewport_on_axis` 接收物理 Edges。
- scroll padding 改变会使未确认的定位请求过期，下一次 prepare 计算新目标；不使实际测量或只读几何 snapshot 失效。单纯调整 overscan 不撤销定位请求。
- Grid 的 rows/columns 必须为 Attached、共享 binding、使用 Fixed/Known；同一帧尺寸和 revision 必须配对。两个定位请求合并后只写一次 Point，单元格身份为 `(row_key,column_key)`，不构建所有行列交点。
- Masonry 初次按最短列分配，后续测量仅更新该 lane 的持久树，不在滚动中重分栏。`reflow(lanes)` 是显式冷事务；跨轴宽度变化使动态测量失效。零视口或列间距占满宽度时不构建卡片。未知动态行高不等同二维 Grid。
- Grid 的每轴工作集顺序读取一次，列元数据只物化一份 O(C) 临时向量；单元格枚举仍为 O(R×C)，不逐行克隆列索引、不逐 cell 重查列 source／几何。稀疏 retained 项仅直接查询对应位置；没有行时不物化列。
- Masonry 成功的非空显式内容失效总是推进 snapshot revision；未测／重复失效也拒绝旧反馈。实际卸载同样推进写入身份，保留 state 并重挂不能接受旧挂载的反馈。空输入和失败批次不改变版本，冻结 snapshot 仍可读取旧几何。
- `VirtualMasonryState::append_source(next,sizing)` 要求既有前缀的 key/content/hint 未变，仅估算追加项，沿当前列末端继续分配，保留原分栏与实际测量。普通 `replace_source` 允许重排，迁移真实尺寸后仅打包一次；二者都在修改前校验尺寸。lane/source 索引使用 256 项共享叶块，冻结 snapshot 后追加复制块目录与尾块，读取 O(1)，复制仍含 O(N/256) 目录成本。实测追加峰值显著降低，但冷构造和无变化反馈有代价，见本轮报告。
- `measurement_cache()` 导出 Measured 的已测项与阅读锚点。`restore_measurement_cache()` 校验 schema 2、axis、layout revision、gap、cross extent、重复 key、非法尺寸及锚点距离后原子恢复；缺失或内容版本变化的 key 跳过。锚点使用 `VirtualizerAnchorPosition` 的三个变体，距离须有限且非负；不将 schema 1 的纯项内偏移猜测成留白语义。缓存不持有 Entity、ScrollHandle、source 或旧绑定；实际跨轴不符仍须失效。
- 导出时逐条核对原生测量中实际存储的规范化 cross 与已解析视口 cross；尚未在新约束下完成布局、混合约束或 cross 未解析时返回 `InvalidGeometry`，不重新贴标签。恢复前先完整验证输入，再把已有不兼容约束的真实测量降为 hints。相同主轴尺寸在新 cross 下回报仍是新测量；横纵轴遵守同一合同。

## 6. 三种尺寸路径

### 6.1 Fixed：不为确定事实付出逐项成本

输入为有限且正的逻辑主轴尺寸 item_extent，来自 token/rem 解析或宿主几何；同一布局 revision 内所有项一致。统一槽位步长为 `stride = item_extent + gap`，总主轴尺寸为 `N × item_extent + max(N−1, 0) × gap + padding_start + padding_end`。首版 Fixed 的支持轴见第 2.5 节。

复用/提取原生 uniform_list 的范围数学，Owned 与 Attached 使用同一算术定义。headless 查询已有显式 extent 时不测代表项；原生便利呈现若仍需代表布局，使用有确定尺寸的槽位并单独计量。不能据几何查询零 renderer 调用，就声称完整呈现 callback 恰好只执行 V 次。

禁止为 fixed 构建 N 个 `Size` 或转为 known-size 数组。若内容实际不等高，应改用 Measured，而不是裁掉文字后仍宣称自动高度。

### 6.2 Known：尺寸已经是输入

输入为长度等于 source.len 的主轴 extent 序列；横向是 width、纵向是 height。构造时建立一份紧凑叶块与持久 SumTree，输入数组不永久双持有；Owned/Attached、Grid 和 Masonry 的已知尺寸几何共用此实现。

- 每项高度不含 gap，gap 只由视口算一次，最后一项后不加 gap。
- 每块最多 B=128 项，保存精确 f32 extent 与 f64 局部累计；extent 不通过两个大 offset 相减恢复。私有尺寸符号位标记估算／已测，公开尺寸始终为正。快照共享不可变叶块与树节点；point update 复制一个叶块及树路径，冻结快照仍读取旧几何。累计是同一几何 owner 的索引，不是另一套测量真值。
- 几何初始化与全量换序允许 `O(N)`；稳定范围搜索为二分，呈现为工作窗口成本。
- `update_sizes` 按 key 验证并原子合批；排序后按触及叶块构造，每块只复制／重算一次。单项成本为 O(B + log(N/B))；k 项还包含验证查询、O(k log k) 排序、触及块的局部累计及路径成本。gap-only 不复制 N 项。完整 source/尺寸替换仍为冷事务。百万项当前几何 retained payload 约 12.4 MB，具体读／写／内存取舍见第二轮实施记录。

### 6.3 Measured：估算用于未测，实际布局才是真值

支持纵／横向，复用 `ListState` 与 SumTree，默认 visible measuring，不调用 `measure_all()`。需要显式正估算高度，避免大批未测项以零高度参与总高和定位。Owned 使用原生输入与逻辑位置；Attached 复用相同布局/测量算法，由宿主提供当前视口并统一应用校正，不注册 List 自身的 wheel 或复制 SumTree。

1. 初始化、宽度变化、splice 新项后的 hint 都应有效；本地 uniform hint 与 prepaint 失效路径须先补测试，再作最小原生修正。
2. 可见项按真实宽度布局；实际高度替换估算。内容是换行、图片或展开状态时，由宿主报告内容失效，或沿原生可见布局路径取得新值。
3. 新插入项继承估算；原有未受影响项保留测量。不能一次 append 把全部旧测量丢掉。
4. Measured 的尺寸缓存以原生状态为真值。跨快照更新优先用 splice/重测和 stable-key 锚点，不先额外建立所有行的第二份高度 Map。
5. 重排按稳定 key 和 `revision_at` 转移实际测量；内容版本变化的项改用估算。原生 remap 同时迁移焦点和处理最后一项 gap。此为冷事务，不把全量映射藏到稳定滚动。
6. 估算误差意味着 total size 与 scrollbar 暂时近似，API 必须区分 estimated/settled，不能把未知尾部报告为精确。

滚动条拖拽复用 ListState 的 start/end drag，固定本次拖拽的高度映射；测量继续进行，但不能让 thumb 比例每次测量都从指针下逃走。松手后以当前逻辑位置恢复最新几何。

### 6.4 尺寸合法性与止损

- Fixed/Known 输入拒绝零、负数、NaN、Infinity；Known 长度必须匹配。
- 需要隐藏的项先从 source 移除，不用连续零高度项表达过滤。
- Measured 零视口不缓存为“所有行高为零”；隐藏窗口恢复后必须重新测量。
- 实际 renderer 返回非正／非有限高度是合同错误：本帧用有效估算维持推进并发去重诊断，标为未精确测量；不主动调度重测。后续真实呈现或显式失效可恢复，不产生 notify/测量自激循环。
- 一维 Measured 每次同步 pass 默认最多布局 1,024 项，Owned 最多两次 pass。预算耗尽发去重诊断并保留其余估算，不自动加帧重复同一昂贵工作；极密或超大视口可显式提高 `measurement_budget(count)`。零预算拒绝，不能把预算耗尽解释为数据已到尾部。
- Masonry 的预算限制尺寸校正额外暴露的 intrinsic roots，不截断必需工作集呈现；Fixed/Known 不消耗动态校正额度。先正常布局初始 W0 项，再允许至多 budget 个额外测量根，最终 Wf 中未构建项按估算高度固定并裁切呈现，不反馈伪测量。根布局总量上界是 W0 + budget + Wf，**不是** budget。只有本帧真实测量推进了几何且仍有待测槽位才续帧；无效尺寸不靠无进展 redraw 重试。不跨帧持有 AnyElement。密集有效微小项仍有真实 W 成本，不设置静默丢项上限。
- 单行高于视口是有效情况：只构建该行及必要缓冲，不强制缩小内容。

### 6.5 尺寸推导、预处理与唯一真值

参考项目的 Pretext 示例说明：尺寸可由文本、准确的字体/换行输入和可用宽度推导时，可以减少渲染后的测量校正。这是可选的尺寸信息来源，不新增第四种 sizing，也不把第三方 JavaScript 文本库带入 GPUI。

1. 精确尺寸用 Known；近似值用 `measured_with_estimates(fallback, callback)` 或 `update_size_hints`。实际测量优先；单纯读取快照不调用估算器。逐项初始估算及带回调的完整 source replacement 会访问全量输入，应只做廉价计算，不在回调排版全量文本。
2. 推导尺寸必须与实际文本系统、字体 fallback/加载状态、字重、字号、line-height、字距、换行规则、内容宽度和 padding 一致。宽度或排版身份变化后，旧高度不能继续作为精确值。
3. 图片、嵌入内容或任意子组件决定高度时，除非已有可靠元数据，否则走真实测量与失效路径；不能用纯文本算法冒充通用卡片测量。
4. 估算、文本预处理、布局缓存与测量读取都要计入成本。不得因为 renderer 只访问 W 项，就声称预先 prepare/estimate N 项也只有 W 的成本。
5. 若增加预处理缓存，须定义唯一 owner、内容/排版身份、容量、删除与销毁时机，并优先复用 GPUI 已有文本能力。近似估算与实际测量必须有确定优先级，不能由两个独立写入者来回覆盖高度。

## 7. 缓存失效与一致性

### 7.1 失效维度

| 变化 | 必须处理 |
| --- | --- |
| 内容更新、图片完成、展开/折叠 | 按 key 找当前 index，局部重测；丢弃旧 generation 测量 |
| 跨轴可用 extent、scrollbar gutter | 使受约束排版影响的测量失效；Fixed 不错误保留变化前的解析尺寸 |
| 字体、字重、字号、line-height、字体 fallback/加载 | 递增 layout revision，重新测量 |
| window.rem_size、密度、影响几何的 token | 重解算高度/gap/padding/估算并失效 |
| 仅颜色/选中底色 | 重绘，不全量清空几何 |
| 数据 prepend/delete/reorder/filter | 显式变更事务，先保存必要锚点再更新 index 空间 |
| DPI/scale | 重新验证逻辑布局和像素量化；不对所有纯逻辑几何盲目乘倍率 |

布局身份至少包含 Axis、实际跨轴可用 extent、rem、字体/排版 revision、density、gap、padding；内容身份包含 key 与内容失效版本。viewport/input revision 与内容/布局 revision 分开；单纯改变 offset 不清空尺寸。不能用 `Rc` 地址或 callback 地址替代语义 revision，也不能每滚动一像素递增全局 layout revision。

实时失效与可移植身份分开：`invalidate_items` 使本控制器的旧反馈／准备结果过期，portable cache 则以 source 的 `revision_at` 和 layout revision 判断跨控制器有效性。尺寸相关内容改变必须推进稳定 source revision，字体等共享排版依赖推进 layout revision；不能假设局部 epoch 自动使所有旧导出 cache 作废。第四轮的同 revision 恢复观察是此合同边界，不直接计为已确认缺陷。

### 7.2 已修复的原生缺口

以下缺口已通过失败基线与当前回归修复，历史描述保留以说明补丁原因：

1. **Known gap 缓存键。** 改造前 Base `request_layout` 用 item_sizes 变化触发前缀重建，解析 gap 不在条件中；只改 gap/rem 可能沿用旧 origin。应在该所有者修正，不在 Yue 每帧 clone 数组强迫失效。
2. **Measured hint 生命周期。** 首次/宽度变化不能在不知道新尺寸时把所有 hint 清成零；新增项也须有估算。
3. **统一的失效锚点。** 原生 `remeasure()` 采用比例锚点，本组件默认绝对行内偏移；须调用合适路径或暴露明确策略，不借另一个名字悄悄更改语义。
4. **离屏保留与定位。** 固定和已知尺寸路径需要与 Measured 相同的焦点保留及显式对齐合同；只添加必要原生接缝。

5. **几何与 Element 解耦。** uniform 的算术、Known 的前缀/二分、Measured 的 SumTree 继续各有一个 owner，提取可由 Owned/Attached 共用的原生接缝。Fixed/Known 不挂载列表即可查询；Measured 不用隐藏的列表 Element 获取尺寸。
6. **外部容器唯一写入。** 改造前 Known 的 with_scroll_handle 仍会更新 handle 中的 axis/count 并直接写完整 offset；List 的 paint 仍注册 wheel。Attached 必须分离这些职责，只提交主轴意图；不能简单把一个旧 handle 同时交给两个列表。

前四项是已识别的行为缺口，后两项是普适化必须新增的接缝。各项先用原生对照和失败回归确认；不为结构整齐复制已有代码。修改 core/Base 时分别更新 YUE-PATCH.md，记录来源、测试、维护和移除条件。

### 7.3 同帧提交

一次批次：读取当前 binding/viewport → 收集有效测量 → 更新尺寸与可用滚动范围 → 求主轴锚点/定位意图 → 唯一 owner 合并、clamp 并应用原生偏移 → 读取实际结果 → prepaint/paint 同一几何 → 帧后发合并事件。Attached 的完整跨轴提交见第 5.8 节。

不能先画新高度和旧位置，再下一帧补偿；也不能先写一个超出旧范围的 offset 后忘记 clamp。Base 应委托 GPUI 的布局阶段，必要时使用原生支持的再布局/延后机制，不在 `Render` 循环调用 notify 来逼迫收敛。

事件携带提交后的 generation 和范围。回调期间发生新事务，后续旧批次不能继续写入。

## 8. 数据变更、锚点与尾部跟随

### 8.1 默认阅读锚点

锚点为 `(key, VirtualizerAnchorPosition)`，距离使用逻辑像素。`Within(d)` 保存主轴起始边缘进入内容的距离；起始 padding 归首项 `Before(d)`，项间 gap 归后继项 `Before(d)`，尾部 padding 归末项 `After(d)`。恢复时 Before／After 的距离不超过当前相邻留白，Within 不超过当前内容 extent，再由唯一 owner 按可达视口范围 clamp。Measured 槽位内部包含 gap，但锚点解析使用去除 gap 后的真实内容 extent。

纵向对应顶部，横向对应左侧；保留对象及其内容／留白关系，不保存旧 index。Attached 的视口起始边缘在 `[content_origin, content_origin + total_extent)` 之外时，由宿主拥有阅读位置，清除该列表锚点与旧校正。即使视口部分覆盖列表，也不把宿主 header 的阅读起点归列表；列表内部的大 gap 则仍建立合法留白锚点。

| 变化 | 默认规则 |
| --- | --- |
| 前面插入/删除、局部高度改变 | 锚点 key 仍在则按 Before／Within／After 恢复；无关尾部追加不压缩 padding/gap 中的位置 |
| 当前跨主轴起始边缘条目的末端增长 | 不额外把该项 extent delta 加到 offset |
| 锚点行缩短，行内 offset 越界 | Within clamp 到内容末端；不把原项内位置误解释为尾部留白，再规范化到可达视口 |
| gap／padding 改变 | Before／After 保持相对相邻内容的距离，最多当前留白；不沿用不存在的空白距离 |
| 锚点被删除 | 选旧顺序的后继；无后继取前驱；空源回起点 |
| 同集合 Reorder | 经 source.index_of 找回同 key；内容版本一致时迁移实际测量，其余重新估算 |
| 新查询/筛选结果 Replace | 默认回起点；调用方显式请求且同一数据身份域时可保留 key |
| 用户正在拖滚动条 | 使用原生冻结映射，释放后协调；不后台不断重设 pointer 对应位置 |

只保留恢复所需的 key、offset、后继/前驱候选等少量信息，不为每次 prepend 复制整个旧 source。一次连续大删除若必须更广泛寻找存活邻项，其成本属于冷更新，不藏入稳定滚动路径。

### 8.2 变更类型

- `Splice`：旧半开区间被新项替换，验证 `next_len = old_len - removed + inserted`；调用方保证区间外 key/content/hint 未变。Measured 仅调用新增项的估算，保留区间外实际测量与 hint。既有 hint 同时变化时用显式尺寸／hint 更新或 Reorder/Replace，不能隐式夹带在 Splice 中。
- `Reorder`：同一身份域换序，按 key 恢复锚点，不能因 count 相同而不更新。
- `Replace`：新集合，默认重置阅读位置和旧命令；source generation 必须改变。
- `invalidate_items`：同一顺序和身份的内容改变，只失效指定范围；不伪装成 delete+insert 来丢掉焦点。

固定行高的 source 更新不能扫描 N 项恢复锚点；通过 source 的反向查询定位。Measured 的简单插入/删除交给原生 splice；任意换序冷重建元数据时按 key/content 转移实际测量。

### 8.3 尾部跟随

默认关闭，三种模式和两轴均可显式开启。Measured 复用原生 FollowMode；Fixed/Known 由 state 依据总长和尾部距离产生版本化请求。Attached 目标经唯一宿主应用；Yue 不每帧调用 scroll_to_end。

- 开启时定义为请求进入尾部；正常模式下 append 不抢走正在阅读旧数据的视口。
- 已跟随且发生 append/尾行增长，继续贴合尾部；跟随校正不播放位移动画。
- 用户向上滚动或拖离尾部立即暂停，实际返回尾部后恢复；是否“已到底”必须考虑未测项和当前 hint，不能把未知高度的零总和当作到底。
- 新查询 Replace 清除旧跟随会话，消费方决定是否重新开启。
- 明确的目标定位与尾部跟随不能同时争抢 offset；非尾部目标暂停跟随，返回尾部命令可重新开启。

## 9. 滚动定位与动效

### 9.1 命令语义

公开优先按 key 定位；按 index 是对当前 generation 的临时定位，不能跨数据变更长期保存。

支持 start、center、end、nearest 四种对齐。nearest 在目标完全可见时 no-op；目标高度超过视口时，已覆盖视口则保持，否则优先让需要进入的边缘可见。start/center/end 是强制对齐意图，不能误用原生“可见就不滚”的非 strict 方法。

空源或 key 不存在返回明确结果，不创建永不完成的等待任务。首帧尚无 viewport 时可保留**一个最新**待执行命令；数据就绪等待是宿主职责，不在队列里无限保留未知业务 key。

命令受理前校验 axis/sizing/binding 与请求能力，unsupported 返回类型化错误，不产生伪造的 settled 事件。已受理命令区分 settled、cancelled、target_missing、invalid_layout；未测定位允许 estimated 阶段，不能把第一次近似落点报告成 settled。Attached 目标由宿主应用并读回实际位置后才能判定完成，绑定变化或用户输入使旧意图失效；settled 表示该目标在当前布局下收敛，不要求总集合全部测量。

### 9.2 生命周期

```text
Idle → WaitingForLayout → Seeking → Correcting → Settled
                 ↘ Cancelled / TargetMissing / InvalidLayout
```

同一时刻只有一个主动命令。新命令替换旧命令；真实 wheel/拖动/导航输入中断程序动效。自身测量校正和 offset 写回不视为新的用户输入。

一维 state 实现上述完整定位接口；Masonry 仍是独立的 key/align pending 接缝，现共用有界 `CommandProgress` 并发布恰好一次 `CommandFinished`，结果包括 Settled、Cancelled、Missing 与 NoProgress。其 wheel 自动取消旧 pending（包括边界无位移输入），native scrollbar／外部 offset 变化也使旧意图失效；`cancel_scroll()` 允许消费者显式取消，`has_scroll_command()` 只读查询。测量确实改变几何前，从唯一原生位置与旧几何捕获最新阅读锚点，只补偿几何差；相同尺寸的稳定批次不额外扫描工作集。Masonry 未提供一维的全部 offset／smooth 参数，不能宣称两种公开 API 完全等价。

Measured 定位先用估算与逻辑 item 坐标到达目标附近，实际测量后按同一 key 重求目标。最多 32 次 prepare 或 2 秒活动采样；连续 8 次确认位置无进展也结束为 `NoProgress`。隐藏、非活动、零面积或 reduced motion 的 Owned 视口直接完成安全定位，不忙等动画帧。只保存最新命令和必要校正状态，不复制浏览器完整 scroll classifier。

### 9.3 动效选择

| 场景 | 位置与动效 |
| --- | --- |
| 滚轮/触控板 | 使用 GPUI 的输入位移和平台节奏；不再叠加一层 ease 或 spring |
| 拖动 scrollbar | 指针直接映射，无缓动、无追赶动画 |
| Arrow / Page / Home / End 导航 | 默认 instant，确保 active 项立即可见 |
| 显式“定位到此项”且距离不超过一个视口、路径尺寸已知 | 允许消费者请求短距离 smooth |
| 距离超过一个视口，或途中包含大量未测项 | instant 到目标附近，再作即时测量校正；不冻结并构建所有沿途项 |
| prepend/resize/删除后的锚点恢复 | 同帧即时校正，不让内容滑过用户眼前 |
| 虚拟回收后重新进入视口 | 普通呈现，不淡入、不从边缘滑入、不触发批量 stagger |
| 明确的新业务条目插入 | 上层可请求有界动画，但不能改变核心尺寸真值或把全部历史行当新增 |

smooth 从当前采样位置续接目标，由 `VirtualizerMotion::Smooth { duration, max_distance }` 提供时长与距离上限，基础层再限制在一倍视口内并使用 ease-out cubic。计时在首次有效 prepare 开始，Owned 使用 GPUI executor 时钟；Base 不写死动画时长。消费者可把项目 Interaction duration 解析为 Duration。若目标更新导致不再满足短距离/已知尺寸条件，取消 smooth 并立即完成安全定位。reduced motion 下直接到终点，仍完成正常结果通知。

最终几何目标确定、clamp 后与当前实际 offset 相差不超过 0.01px 时，不创建无意义的平移动效，第一次正常确认即发一次 Settled；Nearest 已可见和边界 clamp 同理。动态未测目标继续等待测量，真正拒绝移动的宿主仍会得到 NoProgress；不能用动画时长替代是否实际到达的判定。

像 vlist 的“节点移动”或 Toast 的堆叠平移动画不是所有列表的默认行为。连贯性首先来自**阅读对象不跳、输入不被抢、定位结果不闪回**。

## 10. 缓冲、范围与事件

### 10.1 两种范围

- `visible_range`：在本主轴上真正与视口相交的逻辑项，半开区间。
- `rendered_range`：包含缓冲的连续窗口；`working_indices()` 合并原生焦点、`set_retained_keys` 与 `set_range_extractor` 的额外项，排序去重，不将远处保留项与窗口中间所有行一并挂载。额外策略只增加工作集，不改变可见范围。

Measured 原生 overdraw 有些项只测量而不 paint，诊断必须区分 build/measure/prepaint/paint 数量。不能从“paint 行数小”推断总布局成本小。

### 10.2 默认缓冲

公共预算使用视口比例／逻辑像素，避免极高/极矮行下固定几条 overscan 意义悬殊。Base 默认零额外缓冲，由 `VirtualizerLayout::overscan(Pixels)` 显式配置；Showcase 使用 72 逻辑像素。两侧约四分之一视口仅是消费者可测的调参起点，不作为基础层隐藏默认或性能结论。Fixed/Known 可直接求范围；Measured 把预算映射至原生 overdraw。

首版不做速度自适应。只有测出快滚空洞且固定缓冲成本不合适，才评估方向偏置或速度预测，并设置最大工作集。不能把速度越快扩窗越大当作无代价优化。

实现范围统一到 Base 接缝；缺少 explicit overscan 的原生路径只扩展其 range 输入，不在外层再生成一份行树。

### 10.3 通知合同

| 事件 | 频率／时序 |
| --- | --- |
| `RangeChanged` | 提交帧后，visible/rendered 范围或 generation 确实变化才发；同帧合并 |
| `ScrollStateChanged` | 实际 offset 推进或输入活动开始／方向变化，及 idle 结束；`is_scrolling` 与 `scroll_direction` 表达活动，`has_scroll_command` 单独表达程序命令 |
| `ScrollEnd` | 实际滚动最后活动后 120ms 一次；Entity 宿主使用一个可取消 idle task，纯 headless/Attached 可 poll_scroll_idle；命令结束不伪造滚动结束 |
| `CommandFinished { id, outcome }` | 每个有效定位命令最多一个终态，结果为 Settled / Cancelled / Missing / NoProgress |
| `Diagnostic` | 相同 generation 与连续失败原因去重；恢复或新 generation 后允许报告新错误 |

像素滚动不默认转换为向宿主每帧 `cx.notify()`。原生仍按滚动绘制；宿主只有需要范围预取、状态展示时订阅。回调收到的是不可变结果，不拿可变 controller 继续改 offset。

加载更多由宿主消费 range，在帧后启动；并使用请求 generation 去重、取消与丢弃晚到结果。render callback 不触发加载，`RangeChanged` 也不等于“这个范围已加载”。

### 10.4 几何更新与内容更新分开计量

条目位置或滚动 offset 变化，不自动表示条目业务内容已变化。Base 不把逐像素宿主通知作为正常滚动的前提；尺寸、范围和位置继续通过 GPUI 原生阶段提交，内容按实际身份与失效更新。

vlist pipeline 分开比较内容、位置和尺寸，借鉴的是减少无效内容工作。GPUI 路径仍可能为原生布局、测量或焦点多次调用 renderer；不能据此承诺范围不变就零构建，也不能缓存旧 Element 树来模拟 DOM 保留。

测试与诊断需区分：滚动几何推进、同 key 的 renderer 调用、昂贵内容预处理、宿主通知。业务内容变化即使 range 不变，也必须正常更新。`new_with_item_views` 的 factory 改为接收 `VirtualizerItemIdentity`，只含 index/key；按 key、index、source revision、显式失效 epoch、外部 content_revision 复用工作集内 AnyView，不再将 start/extent/measured/layout revision 当作内容准备依赖。普通 GPUI Render/layout/paint/事件仍遵守框架自身生命周期与缓存；读外部状态的 view 应正常 observe/notify。若确实预处理依赖宽度等外部条件，显式 bump content_revision。工作集退出即释放缓存 handle，业务编辑状态仍由宿主保留；不跨帧缓存 AnyElement。

上述 opt-in 构造器现同时适用于 `VirtualList`／`YueVirtualList` 和 `VirtualMasonry`。Masonry 的局部内容失效在 Fixed/Known 下只过期内容准备，Measured 还使尺寸降为 hint；纯宽度／分栏／布局几何变化不重建准备好的 View。默认 `new` 保持普通 renderer 生命周期；远距离来回没有缓存命中时，不承诺加速。

## 11. 键盘、焦点与可访问性

### 11.1 语义由消费者拥有

VirtualizerState、viewport 和 snapshot 不创建可访问节点，不预设 list/list-item、listbox、option 或 grid，不拥有 active/selected 值，也不截获业务方向键。Base VirtualList 只提供显式语义接线；YueVirtualList 默认非选择列表语义，允许被真正的语义所有者覆盖，不能额外包一层冲突的 role。

| 消费者 | 语义与交互所有者 |
| --- | --- |
| 普通列表/卡片流 | 一维适配选择 list/list-item；卡片内部按钮和 Input 保留各自语义 |
| Select / 命令项 | Select 拥有活动项、选项、选择及提交；先在完整模型导航再 reveal，不从已挂载项推导下一项 |
| Grid/Table | 组合者拥有 row/cell/header、二维坐标、焦点移动及两轴 key；基础层不自动把每轴都标为 list |
| Tree | 语义组件拥有层级、展开状态和树导航；可把可见模型展平后消费主轴机制 |

语义位置/数量的映射独立于 source 索引：分组标题、装饰项或占位行不一定是可选择语义项。虚拟化只帮助呈现目标，不修改选择，也不因滚动而夺走 Input 焦点。

### 11.2 两条焦点路径

1. **容器焦点 + active descendant**：适合 Select/命令项。真实焦点在容器；目标按模型导航并进入视口后，当前行报告 active descendant。离窗行不保留一个指向不存在节点的活动声明。
2. **行内交互控件焦点**：适合编辑、按钮等。业务状态按 key 保存；持焦点行离窗时只保留该行/焦点分支，继续参与必要的事件与焦点树，不把整个中间区间保留。

Measured 复用原生 splice 的 focus 保留路径；新增项可通过 `splice_with_size_hints` 一次提交估算，再绑定真实交互分支的焦点。Fixed/Known 同样通过 focused-item 接缝保留，不通过“滚动就 blur”掩盖缺口，也不一律换成有 N 个尺寸节点的引擎。FocusHandle 按真实交互分支惰性创建，禁止为百万行预建。

编辑行被业务删除前，所有者先提交／取消编辑并将焦点移到有效后继或容器。Popup/Dialog 打开期间的焦点与恢复遵循各自合同，不能因列表回收把焦点交给同 index 的另一条记录。

### 11.3 原生语义

当前 GPUI 已提供 role、aria_label、aria_selected、aria_active_descendant、aria_position_in_set、aria_size_of_set，由第 11.1 节确定的语义所有者映射到真实节点。活动后代声明在子节点，与网页容器上的 ARIA 属性方式不同。

- 行的 set position 使用逻辑位置，不使用工作窗口中的 0..W；size 使用逻辑总数。
- 分组标题如不参与选择，不能冒充 selectable option。source 索引与语义项计数有差异时，由拥有分组语义的组件明确映射。
- 未知总数的无限流不伪造精确 set size；首版可以通过当前已知 source 长度逐段增长，说明“已加载项”含义。
- 行内图标只作装饰时不重复播报；交互控件使用自己的可访问名称和禁用状态。
- 不为全部 N 项生成 synthetic accessibility nodes 破坏虚拟化。逻辑定位与必要的节点暴露需要结合原生平台能力逐项验证。

无头测试证明节点属性、动作和焦点逻辑；Windows Narrator 的实际播报、浏览模式能否访问离屏项由用户实窗验证，不能仅凭 API 存在宣称完成。

### 11.4 滚动时的 overlay 与命中

Tooltip/Dropdown 的锚点是行 key 与本帧 bounds。滚动后重算有效锚点；行已回收、删除或变更身份则关闭对应浮层，不能继续附着到旧 index。对正在独立 Dialog 中编辑的对象，Dialog 可继续存在，恢复焦点时重新按 key 寻找合法目标。

滚动期间不全局禁用 pointer。防止误激活依赖 GPUI 输入合同和 binding 校验；按下 A、数据重排后释放时，不得把点击提交给同位置的 B。

## 12. 外观与状态组合

- YueVirtualList 默认作为平面内容区域，不给每行加卡片、阴影或独立外框；真实卡片由 renderer 决定。
- 内容与滚动条分清边界：滚动条贴实际 viewport 边缘，内容 inset 在内部，gutter 的宽度计入测量。
- 行首、文本、辅助信息与尾部操作的列对齐由行组件/消费者负责。不能因某行缺 icon 而随机改变整列排版。
- 选中、活动项、hover、键盘焦点为不同状态，由拥有相应语义的行呈现；不能只有 hover 代表选择。
- loading/empty/error 用外部 slot 或可寻址的占位行表达。整页未加载无有效条目时不创建一万行 Skeleton；保留查询结果刷新时也不先清空再闪回。
- 所有视觉值来自现有 Yue token/typography/motion；不新建 VirtualList 私有颜色体系。算法预算、测得高度和误差阈值不是视觉 token。

## 13. 性能与资源边界

### 13.1 分路径成本合同

| 场景 | 允许成本 | 禁止的隐藏工作 |
| --- | --- | --- |
| Fixed 稳定滚动 | 范围数学 `O(1)` + 工作窗口布局；元数据不随 N 线性增长 | N 个 Size、全量 key 克隆、N 个 Entity/FocusHandle |
| Known 稳定滚动 | 范围搜索 O(log(N/B)+log B)；连续工作集读取 O(log(N/B)+W)，稀疏区间分别 seek；几何 O(N) | 每帧重建全量前缀／逐项业务内容比较 |
| Known 几何更新 | 单项 O(B+log(N/B))；批量含校验、排序、触及块与路径；冻结快照共享未变块；gap-only O(1) | 全数组复制或宣称任意 resize 为 O(1) |
| Measured 稳定滚动 | 原生树查询 + W 附近的实际布局/更新 | 每次 scroll reset/measure_all、全量业务扫描 |
| 源 Replace/Reorder | 必要索引构建／元数据重建可 `O(N)` | 把这项成本挪到下一次用户滚轮后才同步执行 |
| 内容变化 | 局部失效，测量与通知合批 | 每个测量单独广播整个页面 |

SumTree 的树操作不等于全部组件只花 `O(log N)`：行 renderer、文本 shaping、图片和 source 查询必须一起计量。用户提供的慢 renderer 也不能靠虚拟化自动变快。

### 13.2 内存与生命周期

不引入 DOM 式 Element 池。GPUI 元素树是当帧描述，retained 状态由原生/Entity/稳定 key 保存；不要缓存一整棵旧 Element 树替代框架生命周期。

若实测需要额外尺寸缓存，必须说明：唯一 owner、key、layout/content revision、上限、何时移除、对锚点的影响。Measured 没有第二份全量 key→height 缓存；可导出的 VirtualizerCache 是显式冷路径的纯值，应用决定保存多久。删除项和旧 generation 的请求／测量／动画必须解除持有。

不要在每次滚动克隆所有业务字符串、创建所有 row Entity、解析全部 SVG、解码全部图片。图片集合另设解码与纹理缓存预算；虚拟列表最多报告范围和生命周期，不拥有全局图像缓存。

### 13.3 极大数量与精度

“支持百万条”必须同时证明：模型寻址、初始化内存、总高表示、局部绘制精度、scrollbar 映射、首尾可达和输入响应。不能只证明传入 `count = 1_000_000` 没崩溃。

Fixed/Known 查询与 prefix 使用 f64；原生 ScrollHandle 在同一个共享位置对象内保存 f64 精确值和 Pixels 布局投影。`precise_offset()`／`set_precise_offset(Point<f64>)` 与原来的 `offset()`／`set_offset(Point<Pixels>)` 共享一个 owner；wheel 以 f64 累加、clamp 并在精确位置改变时通知。显式旧接口 jump 重置旧小数，即使新的 f32 投影与原投影相同，也不能留下隐藏余量。Fixed/Known、Grid、Masonry 接入精确位置；局部绘制先作 f64 的 item.start − offset，再转换 Pixels 并遵守设备像素对齐。

第四轮修复前 Fixed/Known 两轴在 offset=16777216 时，0.25px／1px／8×0.25px 的原生 wheel 净位移均为 0；修复后分别为 0.25／1／2px。回归同时验证方向反转、同投影显式 jump、起点 clamp、局部实际 bounds、点击命中和 Grid 两轴合并。已验证 Fixed 1M × 20px 的实际 Owned 首尾定位及 1B 项 headless 查询；后者不等于 1B 项完整原生绘制验收。

Measured 原生 ListState 现已在同一 SumTree 使用 f64 累计与 seek；Base 直接使用 precise geometry、viewport batch、offset 和 prepaint 接口。原生 wheel/scroll_by、双轴局部 bounds/命中、尾项可达与滚动条端点已增加升级回归。测量计数由总数减未测数推导，使 summary 保持原大小；具体正确性收益和 CPU 代价见[逐项比较](research/2026-11-04-virtual-list-optimization-series.md#5-measured-累计精度采用修正后的紧凑方案)。

稀疏测量导出按原生摘要跳过未测子树；宽度变化只将已测变体转为 hint 并保留未变化子树，稠密输入保留线性路径。二者都保留冻结快照、测量身份和唯一尺寸 owner；实测成本及回退条件见[本轮报告](research/2026-11-04-virtual-list-adoption.md)。Measured 焦点句柄 Rc 压缩因稠密焦点退化已撤销，方向缓冲／自动中位数估算也未改为核心默认。

Masonry 宽度／布局失效通过 KnownGeometry 与尺寸块代次呈现旧值为 hint，首次写回或尾块追加才归一化该块；冻结快照保留原代次。原生 `set_item_sizes` 校验整个批次，保留重复索引的最后输入并每八个唯一索引拼接一次持久树，仍只提交一次 revision；32 项稀疏批次仍有约 97 KiB 临时峰值增长，具体成本见[继续对标实测](research/2026-11-04-virtual-list-continuation.md)。

**仍有边界：**单项尺寸、项内偏移、padding/Attached 内容原点、普通 Div／UniformList 绘制及滚动条输入仍使用 Pixels 投影；f64 本身也有有限精度。旧 geometry/batch 的 Pixels 接口仍投影，大累计坐标宿主须使用 precise 接口。不能扩展成“所有模式无限尺度精确”或滑块可逐一寻址任意亚像素的声明。

浏览器 16M/33M 等元素高度上限不适用于 GPUI，也不能作为阈值。发布说明只写真实验证过的规模与尺寸范围；超出已验证范围须可诊断，不能静默漏行或把尾部当作已到达。百万条是必测压力场景，测试通过前不是能力宣称。

### 13.4 先测量再优化

| 测量段 | 记录 |
| --- | --- |
| source 准备 | 数据生成、排序/过滤、索引时间；不混入组件首帧数字 |
| state 初始化 | N 相关分配、构建树/前缀、保留字节 |
| range query | 范围查询次数与耗时、source 查询次数 |
| build/layout | renderer 调用数、代表行测量、重测数、layout 时间 |
| 条目内容 | 按 key 记录重复构建、文本预处理/排版、子树布局、图片资源准备；不得靠生产热路径遍历全部子树来统计 |
| 尺寸推导 | estimate/prepare 调用数、缓存命中与字节、宽度/字体失效成本；区分 W 范围工作与 N 级预处理 |
| prepaint/paint | 行数、提交时间、GPUI 整帧 CPU/GPU 时间 |
| 通知 | range/scroll-end/命令结果次数，是否引发宿主重建 |
| 更新与销毁 | splice/replace/remeasure 延迟、缓存大小、pending task、释放后增长趋势 |

固定随机种子、窗口、rem、DPI、字体、数据形态、release 构建、硬件、刷新率和采样区间。区分冷/热运行，报告 P50/P95/P99、最大停顿、掉帧与分配；不只报平均 FPS。测量工具加入故意的全量构建／保留实例对照，验证能检测出错误。

初始性能验收目标为：相同 W 下 Fixed 的每帧查询/构建数不随 N 线性增长；Known/Measured 无额外全量业务扫描；新增接缝相对同等内容原生 baseline 的固定成本必须实测公开。当前 240×100px fixture 的 Measured 辅助成本 P50 差值预算为 0.05ms、复杂组不超过原生 10%，来自三轮 release 基线；这不是零开销或任意场景帧率承诺。其余毫秒和内存预算继续按真实 workload 固化，不能在未测前承诺“120 FPS／0.1 MB”。

### 13.5 工作集数量与单条目成本分别验收

每帧 CPU 成本应按“source/range + 各参与条目的构建/布局 + 测量校正 + 通知所引发的工作”分账；GPU 的提交、纹理和效果成本另记，不把 CPU/GPU 并行阶段机械相加。W 只是条目数量，不包含每个条目的成本上界。复杂度记为 `O(W)` 的呈现路径，也可能因单项成本很大而卡顿。

必须采用两组独立对照：

| 对照 | 固定条件 | 改变条件 | 要回答的问题 |
| --- | --- | --- | --- |
| 总量增长 | 实际工作集、内容复杂度、尺寸和视口 | N 从小到压力规模 | 基础层是否隐藏全量查询、构建或通知 |
| 条目复杂度增长 | N、外部槽位几何、实际工作集和操作 | 文本块/子控件数量、布局深度、资源与效果 | 时间耗在哪个条目阶段，基础层是否重复触发该成本 |

另测高度随内容增长的真实卡片、超高单条目与展平后的等价内容，不能只用固定槽位的合成测试代替真实场景。子元素计数使用可控 fixture；对照须保留相同的逻辑内容与状态，避免拿删减功能的行宣称优化。

优化按热点归属落地：大量无效宿主更新先修通知边界；重复排版先核对文本/布局身份与缓存；批量 resize 先修合批及测量时序；巨型单条目先评估虚拟化粒度；冷启动估算过重先移除全量昂贵预处理；图片解码、纹理和模糊效果由其资源所有者优化。增加缓冲、节点池、第二套尺寸表或通用插件不是默认答案。

## 14. 源码组织与 Base 补丁范围

按职责而不是按三个库的目录照搬：

```text
vendor/gpui-base/src/virtualization/
  mod.rs                             # Virtualizer* 的公共行为/几何接缝
  state.rs / geometry.rs / element.rs # 唯一 state、三种几何与 Owned 一维适配
  cache.rs                           # 纯值测量导入／导出
  grid.rs                            # 两轴组合、一个滚动 owner
  masonry.rs / masonry_element.rs    # 稳定 lanes 几何与实际动态呈现
vendor/gpui-base/src/lib.rs           # 明确导出行为与呈现入口
vendor/gpui-pre/src/elements/
  uniform_list.rs                    # 共用算术几何、buffer/focus 接缝
  list.rs                            # hint、锚点及可借用宿主的布局/测量接缝

crates/yue-component/src/virtual_list.rs # Yue 公共组合、re-export、简短 builder
crates/yue-component/tests/
  virtual_list.rs                    # 公共入口与组合回归
examples/windows-showcase/src/
  ...                                # 对照场景与计数信息，用户做真实 UI 验收
```

模块按实际职责拆分，公共 Virtualizer* 与 VirtualList 适配有明确出口，不要求示意树里的每个内部文件都存在。Base 调整目录前确认 use/re-export/test 注册；已知尺寸算法随其唯一 owner 迁移/提取，Owned 和 Attached 共用。修复 hint/几何缓存落在原生 owner，Yue 不复制规则。

公共变更方向：旧 v/h/virtual_list 的外部尺寸数组签名迁为 source + sizing + Axis + binding + retained state；行为类型使用 Virtualizer*，一维呈现使用 VirtualList。迁移全部仓库调用方并删除冗余旧构造器，不保留同义别名。Select 可提供 Known 分组尺寸，等高无分组时可显式选择 Fixed；不按内容自动反复切模式。

迁移后删除 Select 的重复范围／滚动协调；保留它真正拥有的 active/selection/query/请求票据、Popup 生命周期和行外观。若某项原生能力不适合统一，先写明语义差异并调整最小公共合同，不用 `legacy_*` 或两个 controller 互相同步。

## 15. 测试、前后基线与验收

### 15.1 改造前必须补的基线

实现开始先记录当前三条原生路径和 Select 的行为/性能。基线有失败照实记录；不得将“旧代码没有测试”当作无需验证的理由。

| 基线 | 方法与证据 |
| --- | --- |
| Fixed / Known / Measured 工作集 | 同一视口、数据与 renderer，统计调用、测量、范围、首尾可达 |
| 原生滚动/定位/焦点 | 使用真正的 GPUI 元素和输入事件，保存断言，不以内部字段比较替代交互 |
| gap/hint 等缺口 | 先新增在旧实现下失败的测试，再补丁；保留旧版本失败与修复后通过记录 |
| 既有 Select | 分组、搜索、键盘、选中值、Popup、异步晚到、主题与 rem 的现有测试 |
| 性能 baseline | 同一 workload 的原生路径与 Select；保留环境、原始样本和测量范围 |
| 复杂卡片与尺寸推导 | 固定工作集下的简单/复杂内容、超高卡片、冷/热文本准备与宽度失效；保留各阶段调用数和资源占用 |
| 主轴与外部宿主 | 现有 Known 横纵轴、with_scroll_handle 的几何、两轴偏移写入与原生输入次数；新增 headless 路径对照同等原生内容 |

新增接缝在旧版本中不存在时，先通过现有生产 Element 记录等价几何和行为，作为新路径的对照；不能把新 API 在旧版本无法编译当作行为缺陷的失败证据。

### 15.2 纯规则与数据测试

- 0/1 项、超大 N、视口边缘、半开范围、gap 只加 N−1 次。
- Fixed 范围对照朴素实现；Known 随机高度与前缀对照；测量更新顺序不同结果一致。
- 非有限／非正尺寸、越界区间、计数不一致，验证失败不改变有效状态。
- 主轴投影、符号转换、padding/局部 origin、start/end/nearest 与 clamp；Known 横纵轴用等价输入对照，不能只测改一个 enum。
- capability 矩阵逐项校验；不支持的配置在 N 级分配、挂载、渲染和滚动写入之前失败。
- key/index 互逆、重复 key fixture、相同 count 的 reorder、新查询占位身份。
- prepend、锚点删除、跨顶边增长、锚点缩短、Replace reset 与显式 preserve。
- 命令取代/取消/缺失目标/无进展，终态事件只一次。
- 使用实际算法边界数据，例如跨多个块/树节点；测试名称写“跨块”时数据必须真的跨块。

### 15.3 无头集成测试

使用当前 `headless-tests` 接入，不启用 Kit 另一后端。测试生产视口和真实宿主组合，而不是复制一份测试专用引擎。

| 场景 | 必须断言 |
| --- | --- |
| 慢滚、快滚、跳至首尾、缩小数据集 | 目标行 bounds 与 viewport 相符，无越界调用、空洞和旧身份 |
| 固定 100K/1M 逻辑项 | 同一 W 的 renderer/source 查询不变为 N；没有 N 个行状态 |
| gap、width、font、rem 改变 | 原生真实布局更新，前缀不陈旧，锚点符合规则 |
| 图像晚到、动态展开、批次 resize | 旧 generation 结果被丢弃；viewport 上方与跨顶边区别正确 |
| range 不变时滚动与内容修改 | 位置正确推进；几何变化不广播全量业务更新；内容 revision 变化仍正常呈现，不因范围去重而陈旧 |
| 复杂卡片与超高单条目 | 按条目根测量；内部控件和状态正确；工作范围不因内部元素增多扩成 N 项；不假设内部自动虚拟化 |
| 缓存/推导尺寸的失效 | 字体、宽度、正文和图片变化后重新取得有效尺寸；估算不会反复覆盖真实测量；无双写振荡或 notify 自激 |
| 初帧 hint / splice 新项 / resize | 未测项有合理估算，不能假尾部跟随；不 measure_all |
| nearest 与 strict 对齐、高于视口的单行 | 实测 bounds 验证语义，不只比较请求 options |
| 行内 Input 获焦后滚离视口 | 同 key 保留焦点和编辑值；删除后转到有效目标 |
| pointer down 后重排/替换 | 不把释放提交给另一 key |
| Select 导航至离屏选项 | active、selected、确认值、disabled 跳过仍正确 |
| 滚动条拖拽中测量变化 | 指针映射稳定，结束后新几何正确，follow 不误恢复 |
| Tooltip/Dropdown 锚点回收 | 不残留在同 index 新项上，焦点恢复有效 |
| 卸载、换页、隐藏恢复 | 无遗留 frame/timer/request 回写，无 notify 自激循环 |
| 可访问性 | role/name、逻辑 position/size、active-descendant 与 focus 可验证 |

短距离 smooth 测中间帧、重定向、用户打断、reduced motion；不能只断言起终点。每次使用同一组件 state 走多个动作，覆盖保留状态问题。

### 15.4 基准矩阵

| 维度 | 至少覆盖 |
| --- | --- |
| 数量 | 0、1、1K、10K、100K；1M 压力场景 |
| 高度 | 固定；已知混合；动态长尾；单行超过视口；错误尺寸保护 |
| 行内容 | 简单文字、CJK 换行、图标/按钮、受控 Input、异步图片 |
| 条目复杂度 | 同一槽位的少/多子元素、浅/深布局；真实长卡片及展平对照；包含有界纹理/效果的资源场景 |
| 尺寸信息来源 | 低成本估算、已知精确尺寸、实际测量；若启用文本预处理则覆盖冷/热缓存和全量失效 |
| 操作 | 首帧、连续滚动、远距离定位、头插、尾加、删除、重排、筛选替换、局部 resize |
| 布局 | 窄/宽视口、rem 放大、DPI、隐藏/恢复、滚动条拖拽 |
| 生命周期 | 多次创建销毁、不同 query generation、数据规模显著缩小 |

同时比较“同一业务语义的旧路径”和“新统一接缝”，不能拿无内容的 fixed 行与复杂动态行横比后宣称更快。真实 GPU/Windows 帧节奏、动效观感与 Narrator 由用户检查；自动化提供计数、时序与无头几何证据，**不主动启动 Showcase、截图或接管真实 UI**。

第 13.5 节的两个对照须分别记录；复杂度增加时不能要求帧耗时恒定，而应验证成本可归因、没有非必要重复工作。参考项目的简单文本 benchmark 不能代替复杂卡片基线。首版必须完成第 15.6 节双轴最小组合；动态 Grid 行高仍需明确未呈现列的依赖；已交付的 Masonry 单独覆盖稳定 lanes、显式重排和真实测量，不把一次组合验证当作全部布局能力。

### 15.5 改造后检查集

当前改造使用以下检查集；具体运行结果和未通过项见实现记录：

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets -- -D warnings
```

开发中先运行新增虚拟列表、Select 与所改补丁相关的定向测试，再执行上述回归集。修改 vendor 后还需运行对应 crate 的补丁测试，确认解析到同一组本地 GPUI/Base；workspace 测试不自动执行依赖 crate 的全部内部单测。token 如有必要新增，另跑设计系统同步与映射检查。

保存改造前后命令、结果、已知失败及性能样本，不为了兼容错误行为改断言。只改本文时检查文件路径、链接、语义和 diff 即可，不重跑无关 Rust 构建。

### 15.6 基础层普适化验收门槛

以下生产接缝由当前 `virtual_list` 无头集成目标验证，不自动接管真实 UI；对应测试与证据见实现记录。

| 消费者/场景 | 必须证明 |
| --- | --- |
| 普通复杂卡片流 | 不依赖 Select/ListItem 模型；动态内容、焦点和业务状态正确；Yue 默认列表语义不污染原始 state |
| Select 面板 | 使用同一公共行为入口；选择、active、搜索、disabled 跳过和 Popup 回归；没有重复 list/listbox 或另一份滚动状态 |
| 单个宿主中的双轴最小组合 | 纵向 Fixed/Known + 横向 Known 直接消费 snapshots；仅一份 overflow/输入/offset owner；渲染工作单元为两轴工作集交集；单元格双 key 正确 |
| 对角滚动与同时定位 | 横向更新保留 y，纵向更新保留 x；同批合并总尺寸与 clamp；事件不重复、bounds 不错位，不因共享旧 handle 的 axis 字段被覆盖 |
| Measured / Vertical Attached | 无 List 自身 wheel；测量、锚点和焦点与 Owned 等价；resize 意图经宿主一次应用，无高度/位置双写 |
| 无呈现的 Fixed/Known 查询 | 不创建 VirtualList、renderer、可访问节点或主题依赖即可获得范围/位置/总尺寸；查询不分配 N 个项对象或触发 notify |
| 语义归属 | 原始机制零默认 role；列表适配的语义可覆盖；Select、Grid 分别拥有逻辑计数、active/selection 和导航 |
| 快照过期、解绑与能力失败 | 旧 source/geometry/viewport/binding 意图被拒绝；不支持的轴/模式、重复绑定同轴、一个 state 同挂两个宿主均明确失败且无副作用 |
| 同批校正与完成判定 | 校正请求同步计算、一次接受并读回实际结果；不经帧后通知延迟一帧；目标已稳定时可完成命令，未知离屏项仍使总尺寸保持 estimated |

双轴 fixture 使用已知且独立的行/列尺寸，不夹带未知动态行高。测试通过公共入口和真实生产接缝验证，不复制一份测试专用引擎；保存改造前后原生对照和原始计数。未通过时不能将公共 API 标记为稳定，也不能以补一层包装声明已完成 headless。

## 16. 实施顺序与完成定义

1. **建立基线与失败回归。** 三条原生路径、Known 横向/外部接缝、Select；分别记录总量增长、复杂度和尺寸预处理成本。
2. **验证主轴与唯一写入者。** 提取原有几何/测量接缝，补 gap/hint/锚点/focus 缺口；Known 外部句柄和 List 输入不能继续在 Attached 模式直接写滚动。每项有原生对照与补丁记录。
3. **实现 Base headless 合同。** VirtualizerSource/Sizing/State、viewport/snapshot、capability、变更、锚点、定位意图与事件；Owned/Attached 共用 backend，无主题和默认 role。
4. **通过三个消费者的接缝验收。** 卡片流、Select 消费与共享宿主双轴最小组合，包含 Measured Attached 及 unsupported 无副作用测试；据证据固定公共 API。
5. **完善 Yue 并迁移旧入口。** 三种模式、一维外观/默认语义与计数案例；迁移 Select 和全部仓库调用方，删除重复状态与冗余构造器，同步导出、文档和测试。
6. **测量后优化并完整回归。** 原生/新接缝同等功能对照，按热点决定增量前缀、缓冲或缓存；记录真实规模和待用户验收的 UI 项目。

完成标准：第 2.5 节必须支持的组合、第 15.6 节三类消费者与公共导入均有测试；实际滚动只有一个 owner；原始机制不强加语义；Owned/Attached 不复制引擎；既有 Select 回归通过；稳定滚动无隐藏 N 级业务工作；复杂内容和尺寸预处理可归因；锚点、焦点、事件与命令有确定结果。明确不支持的动态 Grid 行高、反向索引与 RTL 镜像仍不作隐式推断；性能与实窗能力按真实证据陈述。

### 16.1 第三轮对标修复结果

修复前复现见[第三轮对标记录](research/2026-11-04-virtual-list-followup-review.md)，修复策略、改前六项失败、改后回归及 Release 时间／分配 A/B 见[第三轮实施记录](research/2026-11-04-virtual-list-followup-fixes.md)。下表六项已落实到生产接缝。

| 优先级 | 已修复问题 | 回归覆盖 |
| --- | --- | --- |
| P0 | 内容失效后同批 Splice 清理 epoch，旧准备缓存重新匹配 | 三模式、不同操作顺序均显示新内容；不只验证版本增加 |
| P0 | Masonry measurement budget 截断实际可见根节点，Fixed 也受影响 | 呈现覆盖与动态测量／额外校正预算分开；不反复只构建同一前缀 |
| P1 | Masonry 未测项内容失效未推进身份 | 任意成功非空失效使旧可写 snapshot 过期；空输入和失败无副作用 |
| P1 | cache 用最新 viewport.cross 标记旧测量 | 导出／恢复依据实际测量约束，不把新视口误当成新测量 |
| P1 | 已到达的 Smooth 命令仍等待计时并报告 NoProgress | 确定且已到达的目标正确完成，动态／拒绝目标继续受止损约束 |
| P2 | Grid 每行重复读取／校验相同列项 | 每轴工作集顺序准备一次，cell 枚举仍按交点数量计量 |

Grid 的 auto 轨道、冻结区域、精确 cell 保留和 Masonry 公共生命周期继续按实际消费者扩展；不因参考库具备某项功能便无条件加入基础层。新增验收同时覆盖同批组合、实际呈现集合与 Grid/Masonry 多周期资源释放。

### 16.2 第四轮对标修复结果与边界

修复前依据见[第四轮调研](research/2026-11-04-virtual-list-owner-review.md)，修复方案、失败基线、13 项新增回归与 Release A/B 见[实施记录](research/2026-11-04-virtual-list-owner-fixes.md)。不覆盖历史观察数字，也不撤销第三轮六项已完成状态。

| 优先级／分类 | 已修复／澄清内容 | 当前证据与边界 |
| --- | --- | --- |
| P1 | 三种区域锚点；schema 2 保留留白语义 | 两轴三模式的 padding 5→5、gap 35→35；内容缩短、留白变化、cache 往返与非法输入原子拒绝 |
| P1 | Attached 区域外不建立列表阅读锚点 | header／footer 与部分覆盖不产生无关追加校正；列表内部大 gap 仍合法 |
| P1 | Masonry 在旧几何上读取最新原生位置；用户接管取消 pending | 四次 20px 保留 80px；旧命令不再把 6543 写回 6523；wheel、外部 offset／scrollbar、显式取消覆盖 |
| P2／精度 | 单一原生位置增加 f64 读写并接入局部绘制 | 第四轮验证 Fixed/Known 两轴小增量、反向、jump、bounds／命中与 Grid；其后 Measured 原生累计也升级为 f64，当前边界见 13.3 |
| 合同澄清 | live invalidate epoch 与 portable source/layout revision 分开 | 同 source revision 的旧 cache 可恢复；尺寸相关内容变更推进 revision 后跳过旧测量，源码与设计一致 |

隔离修复前生产源码运行带真实断言的六项主回归，5 failed / 1 passed；当前新增 13 项回归通过，virtual_list 专项 91 passed / 8 ignored，workspace 加 Showcase headless 合计 805 passed / 9 ignored。新增状态只含每控制器的区域锚点、Masonry 上次自写位置，以及原生共享位置对象内的精确值／旧接口写入标记；没有逐项镜像 offset 或第二滚动引擎。相同工作量的稳定帧 Release A/B，中位数变化为 Fixed +0.09%、Known +0.77%、Masonry +1.70%，样本区间重叠，不宣称速度提升或实窗 CPU 下降。逐样本与复现命令见实施记录。

## 17. 依据与关联文档

- [继续对标与 Git 回退](research/2026-11-04-virtual-list-continuation.md)：测量代次、有界批次、冷构造拒绝项，含持续尺寸变化和峰值保护证据。
- [本轮候选优化取舍与 Git 回退](research/2026-11-04-virtual-list-adoption.md)：六类候选的 ABBA、内存与覆盖证据，采用项和拒绝项分别记录。
- [原生 Measured 精度与前序优化](research/2026-11-04-virtual-list-optimization-series.md)：工作集迭代、反馈、计时器、f64 精度的前后对照与代价。
- [第四轮修复与测量](research/2026-11-04-virtual-list-owner-fixes.md)：区域锚点、宿主边界、用户接管、schema 2、原生精确偏移与性能 A/B。
- [第四轮阅读位置与输入所有权对标](research/2026-11-04-virtual-list-owner-review.md)：三类确认问题、已知精度边界的原生输入证据、持久缓存合同、68 条观察与后续验收。
- [第三轮对标与修复前证据](research/2026-11-04-virtual-list-followup-review.md)：组合内容失效、预算与可见覆盖、测量身份、命令收尾、Grid 热路径，以及进一步借鉴的边界。
- [第三轮修复与测量](research/2026-11-04-virtual-list-followup-fixes.md)：六项逐项修复、失败基线、覆盖／收敛／约束身份回归、多布局释放与 Grid Release A/B。

- [第二轮实施与测量](research/2026-11-04-virtual-list-refinements.md)：正确性修复、内容准备合同、批量／增量路径、紧凑索引与混合轨迹、资源释放证据。
- [第二轮跨库对标](research/2026-11-04-virtual-list-cross-library-review.md)：virtua、vlist、TanStack 的可借鉴机制及修复前探针。

- [1–8 项能力升级与性能记录](research/2026-11-03-virtual-list-upgrades.md)：当前增量树、横向动态、工作集、缓存恢复、Grid/Masonry 的实现与代价。

- [源码调研与快照](research/2026-11-03-virtual-list-source-review.md)：三库架构、算法真实成本、测试证据及可移植边界。
- [组件架构规范](24-component-architecture.md)：模块、状态与 Base/Yue 分层、破坏性变更和前后测试。
- [Select 设计](20-select.md) 与 [当前 panel](../crates/yue-component/src/select/panel.rs)：真实长列表消费者及已知尺寸路径。
- [Base virtualization](../vendor/gpui-base/src/virtualization/mod.rs)、[GPUI uniform_list](../vendor/gpui-pre/src/elements/uniform_list.rs)、[GPUI list](../vendor/gpui-pre/src/elements/list.rs)：首选复用对象。
- [Base Scrollbar](../vendor/gpui-base/src/scrollbar.rs)、[GPUI div](../vendor/gpui-pre/src/elements/div.rs)：滚动 handle、焦点和可访问语义实际接口。
- [Base 补丁记录](../vendor/gpui-base/YUE-PATCH.md)、[GPUI core 补丁记录](../vendor/gpui-pre/YUE-PATCH.md)：后续基础层修改的维护位置。
