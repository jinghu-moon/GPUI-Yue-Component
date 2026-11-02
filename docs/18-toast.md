# 18 YueToast 消息提示组件设计

状态：V1 已实现，自动化验证及实现对应关系见第 20 节；真实 UI 由用户验收。创建：2026-11-01；审核修订与实施：2026-11-02。已结合 `TEMP/1.txt`、`TEMP/2.txt` 审核并复核本地源码；取舍记录见第 19 节。第 2、12.1 节描述实施前的 Base 行为及扩展依据，实际公共 API 以第 20 节和可编译源码为准。

依据本地 `refer/sonner-main` 的 Sonner 2.0.8 源码、用户提供的 React / Vue Sonner 截图、当前 GPUI Kit 0.8.0、受维护的 `gpui-base` 0.8.0 与锁定的 GPUI pre 0.3.9 编写。以本地源码为研究基线，不以社区热度、打包体积或网页宣传替代行为证据。

## 1. 定位与目标

Toast 是当前窗口内、非模态、短时的操作反馈。它在不打断当前工作的前提下说明发生了什么，并在适合时提供撤销、重试或查看详情。它不代替字段错误、长任务面板、必须确认的 Dialog 或操作系统通知。

核心组成：**每窗状态宿主 + 八方位通知区域 + 可展开的卡片栈 + 语义化卡片 + 可选操作按钮 + 独立播报适配**。

本设计覆盖单卡片布局、多个卡片的堆叠与展开、入场与退出、消失顺序、暂停计时、异步状态更新、键盘和屏幕阅读器，以及主题与窗口尺寸变化。

## 2. 源码研究结论与复用边界

### 2.1 Sonner 的真实行为

| 研究入口 | 已核实事实 | Yue 处理 |
| --- | --- | --- |
| `refer/sonner-main/package.json` | 本地版本为 2.0.8，React / ReactDOM 是 peer dependencies | 借鉴机制，不引入 React、DOM 或 CSS 运行时 |
| `src/types.ts` 的 `Position` | 原生只有上 / 下的左、中、右共六位置 | 本项目另增左中、右中，形成八方位；不能称为直接支持 |
| `src/index.tsx:21–43` | 默认可见 3 张、宽 356px、窗口边距 24px、间隔 14px、存活 4000ms、退出保留 200ms | 数值仅作参考，Yue 规格由后文和 token 决定 |
| `src/index.tsx:667–695` | 新消息插到队首；同 ID 更新原卡片，不改其列表位置 | 新建与更新必须分开，Promise 状态更新不重新排队 |
| `src/index.tsx:192–241` | 先标记退出，保存退出前偏移，再延迟卸载；各卡片独立计时，展开、交互和文档隐藏时暂停 | 复用退出保留与剩余时长模型，不规定全栈严格先进先出 |
| `src/index.tsx:784–795` | 容器使用 polite live region，通知出现不主动抢焦点 | 原生语义需另做适配，不能把 DOM aria 属性当作 GPUI 已有能力 |

另须区分以下源码行为与 Yue 选择：

- Sonner 的 hover 暂停经区域 `expanded` 生效，条件是 `expanded || interacting || isDocumentHidden`；单独的键盘 focus 不暂停。Yue 增加 focus 暂停。
- 纯文案更新虽然不执行 `[duration]` 的剩余时长重设 effect，但会因整个 `toast` / `deleteToast` 依赖变化重建 timer effect；cleanup 只清 timeout，不扣除这段已运行时间。因此不能据此断言 Sonner 保证更新后保留准确剩余时间。Yue 明确保留，见第 9、11 节。
- `styles.css:297–301` 的 `--scale` 是未加括号的 token 序列：代入后为 `calc(-1 * rank * 0.05 + 1)`，即 `1 - 0.05 × rank`；不能把自定义属性当作自动带括号的数学变量，误读为整卡负缩放。
- Sonner 超出可见数的卡片仍挂载并计时；`pointer-events:none` 不等于移出 Tab 或可访问树。Yue 的隐藏交互与语义隔离是明确的增强要求。
- Sonner 的 `MAX_HISTORY_SIZE=100` 只裁剪已关闭历史；不限制同时 active 的条数，与 Yue 的每窗 32 条挂载上限没有继承关系。

后续章节分别定义本项目的规格与和原版的差异。Vue 截图里的配置项不自动视为 React 源码已有 API。

### 2.2 GPUI Base 的真实能力

| 已存在能力 | 入口 | 可直接复用 / 必须补齐 |
| --- | --- | --- |
| 无样式卡片根 | `vendor/gpui-base/src/toast.rs` 的 `Toast` | 可组合内容；目前强制 `Role::Alert`，需补可配置语义 |
| 卡片入退场 | `Toast::render` 只组合根和 children | 不自带入场 / 退出动画；Yue 选择位移、淡变，Base 提供可复用通道和生命周期机制 |
| 生命周期与独立剩余时间 | `ToastManager`、`ToastOptions`、`ToastTransitionStatus` | 复用 Starting / Present / Ending 与退出后移除 |
| 栈的测量、不同高度、折叠、展开、弹簧重排 | `ToastStack`、`ToastStackState`、`ToastMotion` | 优先复用；不能将同名现成能力视为已经满足所有 Yue 合同 |
| 同 ID `push` | `ToastManager::push` | 现行为删除旧项后作为最新项重新入场，不能用于 Promise 原位更新；需最小 Base 扩展 |
| 显示数量 | `ToastManager::visible(limit)` | 是呈现过滤，不是待展示队列；Ending 仍返回。需要另定义容量和隐藏项计时政策 |
| 方位 | `ToastStack::placement(Anchor)` | 几何只区分 bottom 与非 bottom；不等于完整八方位窗口定位和侧边中点的稳定锚定 |
| 每窗协调 | `Root::register_plugin` / `RootPlugin` | 注册一次、每窗一个状态宿主，不给每张消息创建 Root |

### 2.3 决策来源与有意差异

下表是本项目的设计取舍，不是对 Sonner 的逐值移植；数值均以当前设计稿为准。

| 决策 | Sonner 2.0.8 / 当前 Base | Yue 选择与理由 |
| --- | --- | --- |
| 方位 | Sonner 六方位；Base 栈只区分上下生长 | 保留用户要求的八方位；V1 每窗仅一个通知区域 |
| 卡片宽 / 露边 / 展开间距 | Sonner 356 / 14 / 14px | 360 / 12 / 12px，与桌面布局及现有 spacing 梯度对齐 |
| 入场 / 重排 | Sonner CSS transform / opacity / height 为 400ms；Base 布局 spring response 为 400ms | Yue 入场 300ms，布局采用 response 400ms 的临界阻尼 spring；必须拆开生命周期与布局配置 |
| 正文显隐 / 收起宽限 | Sonner 正文 opacity 400ms、鼠标离开立即收起 | Yue 正文 150ms、收起宽限 100ms，减少后景文字干扰并容忍短暂越过边界 |
| 退出 | Sonner 前景向外移动整卡高度，部分后景另有 CSS；200ms 后卸载 | Yue V1 统一 200ms，前景向外 24px、其他可见项 8px；不增加第二套卸载期限 |
| 阅读时长 | Sonner 默认统一 4000ms | Yue 普通 / 成功 / 信息 4s，警告 / 错误 8s，Loading / 带操作默认常驻；是可覆盖的行为默认值 |
| 更新 / 暂停 | Sonner timer effect 受整个消息对象影响；focus 不在暂停条件中 | Yue 保留已消费时间、增加焦点暂停；不把内容更新当作重新入场 |
| 隐藏与播报 | Sonner 的隐藏项仍有 `tabIndex=0`，保留在 live region 中 | Yue 隐藏卡片操作，并把播报与视觉栈解耦，避免展开、重排、模态恢复重复播报 |
| 倒计时 / 容量 | 本地 Sonner 无倒计时条、无 active 总量硬上限 | 按用户要求保留可选倒计时条；32 条为可配置资源防护默认值，不由可见三张或八方位推导 |

拖动、多区域碰撞仲裁不属于 V1 必交付能力。八方位、连续重排、单操作按钮和倒计时条不因审核中的简化建议而删减。

## 3. 范围与基础层归属

V1 实施目标：普通、成功、信息、警告、错误、加载消息；标题、描述、状态图标、最多一个操作按钮、关闭、可选倒计时进度条；实测高度的堆叠与展开；八方位；独立计时与明确关闭原因；同 ID 更新；Rust Future 状态适配；键盘与原生可访问支持。

业务请求、持久消息中心、日志历史、系统通知、任意 HTML / CSS 注入和表单编辑不归 Toast 所有。截图展示的是文档网站，不复刻安装区、GitHub 按钮、代码复制操作和网页布局。

Base 拥有无样式生命周期、稳定 ID、计时和布局机制；Yue 拥有 token、卡片呈现、类型映射及每窗的薄协调层；宿主拥有业务数据和异步任务。Toast 以窗口可用区域定位，不以触发器定位，不包装进 Popup 的十二方位定位器，也不为 Popup 添加 Toast 分支。

## 4. 与列表组件的关系

**Toast 集合是列表数据，但呈现和交互属于通知栈。优先把 ToastStack 的合同设计完整，不以完整通用 List 为前置。**

需要区分“卡片内部像列表项”和“多张卡片组成列表”两件事。前者可以复用无状态的内容排版：前置图标、标题 / 描述、尾部操作；后者才涉及有序集合和容器布局。若后续先设计普通列表，建议先约定这种列表项的槽位与尺寸规则，使 ListItem 和 ToastCard 可以组合相同的排版能力；不要求 ToastCard 继承带选择、行导航或整行点击行为的 ListItem。该共享排版仅在真实消费者出现时提取，本文不新增独立列表 API。

| 层次 | 可抽象的部分 | 不应混入的内容 |
| --- | --- | --- |
| 卡片内部 | leading 图标、content 标题 / 描述、trailing 关闭 / 动作，与一般列表项具有相似组成 | 不能因此默认整卡可点击、选中或带 ListBox / Option 语义 |
| 有序集合 | 稳定 ID、顺序、可变高度测量、可用区域约束 | 不自动引入选择、筛选、分页、拖动排序、行导航或虚拟化 |
| 通知栈 | 覆盖、露边、展开、插入 / 删除重排、退出保留 | 计时、异步结果和播报也不是普通 List 必有的功能 |

建议依赖顺序：**现有 Base Toast 机制核对与最小补齐 → ToastCard → ToastStack 的布局与动效 → 每窗 Toaster 与异步适配 → Showcase**。普通 `YueList` 可以独立设计；当第二个真实消费者需要相同的有序测量 / 动态布局时，再提取窄的无样式机制，不先做 `List(mode = Toast)` 或带计时属性的万能 List。

```text
YueToasterState：每窗命令、数据与生命周期
└─ ToastArea：一个位置的几何、焦点入口、暂停原因
   └─ Base ToastStack：测量、露边、展开与连续重排
      ├─ ToastCard A：图标 / 文案 / 操作 / 关闭
      ├─ ToastCard B
      └─ ToastCard C
```

`ToastCard` 是内部无状态呈现单元，实际由 `card.rs` 的呈现函数组合 Base Toast 和 Yue 控件。`ToastStack` 是无样式机制和少量保留的布局状态；Toaster 是窗口级保留状态。三者不各存一份相互同步的消息数组。

## 5. 单张卡片：布局与组成

### 5.1 标准结构

```text
┌──────────────────────────────────────────────┐
│ [状态图标]  标题                         [×] │
│             描述，可自然换行                  │
│             [操作按钮，最多一个]              │
│ ━━━━━━━━━━━━━ 可选倒计时进度条 ───────────── │
└──────────────────────────────────────────────┘
```

- 标题必填，描述可选；空白标题明确拒绝。文案说明对象和结果，例如“已保存「项目设置」”，不只写“操作成功”。
- 状态图标为装饰，含义由可访问名称中的状态和可见文案表达。普通消息无图标时释放其槽位；Loading / Success / Error 原位更新时保留同宽图标槽，避免文字横跳。
- 单行内容可以居中对齐；一旦带描述或多行文字，图标与标题首行对齐，不跟着整张卡片的高度上下漂移。
- 关闭按钮固定在卡片内的右上角，预留独立列，既不覆盖文字，也不越过卡片边界。Sonner 的外悬圆形关闭按钮不作为 Yue 默认；Vue 截图中的四角关闭位置不纳入 V1 API。
- 每张卡片最多一个操作按钮，通常是“撤销”，也可按场景使用“重试”或“查看详情”；不提供次操作按钮。复用既有 YueButton，操作不是整卡点击；右上角的可选关闭按钮独立于业务操作。
- 操作按钮优先后置，短文案和短动作可同行；文案余宽不足时，按钮移到内容下方并与文字起点对齐。按钮不能 absolute 覆盖内容，也不能靠负 margin 挤入。
- 所有文本容器 `min_w_0`，长路径 / URL 允许断行。按钮保持自身最小宽度和命中区，空间不足时先换动作行，再让标题换行。
- 卡片仅保留六类组成：标题、可选内容、可选前置图标、可选操作按钮、可选关闭按钮、可选倒计时进度条。进度条是卡片底部的细条，遵守圆角裁剪并预留空间，不覆盖文字或按钮。

### 5.2 默认几何与约束

以下是本项目的设计默认值，不是对 Sonner 默认值的复述。尺寸按 16px rem 基准列出，实施时统一由 token / rem 解析。

| 项目 | 默认规格 | 约束 |
| --- | --- | --- |
| 卡片宽度 | 360px | `min(preferred_width, safe_rect.width)`；窄窗不强守固定最小宽度 |
| 水平 / 垂直内边距 | 16px / 12px | 内容、关闭和操作共享同一内边界 |
| 图标与正文间距 | 8px | 图标默认 20px，不能随文字压缩 |
| 标题与描述间距 | 4px | 标题用 body 的中等字重；描述用次级文字层级 |
| 操作行与正文间距 | 8px | 优先复用 Button 的既有尺寸和命中合同 |
| 关闭图标 / 命中区 | 14px / 28px | 视觉与命中区分离，禁止侵入文字 |
| 圆角 | 8px | 阴影和背景服从同一边界，内部背景不得漏出方角 |
| 高度 | 自然高度，无统一固定高度 | 不因为标题多一行而裁掉操作按钮 |
| 窗口安全内边距 | 24px，窄窗降到 16px | 另叠加宿主声明的标题栏 / 工具条 / 状态栏保留区域 |

可用文字宽度必须真实扣除内边距、图标、关闭列、间隙和同行操作宽度。Auto 布局在剩余正文宽度低于 `--toast-content-min-width`（160px）时将动作移到下一行；160px 是换行决策阈值，不是窄窗中强制溢出的 min-width。

极端长内容不无限撑出视口：展开区域是唯一滚动所有者，卡片内部不再嵌套第二个滚动容器；完整文本和关闭 / 动作仍能通过键盘到达。需要长篇阅读、表单或复杂操作的内容应转为详情面板，不把 Toast 做成小型 Dialog。

### 5.3 类型与视觉

| 类型 | 状态图标 | 默认行为 |
| --- | --- | --- |
| Normal | 可省略 | 中性表面，普通结果反馈 |
| Success | 勾选状态图标 | 确认已完成的结果，不整张使用高饱和背景 |
| Info | 信息图标 | 补充信息，不冒充需要确认的警告 |
| Warning | 警告图标 | 可恢复的问题，默认较长停留 |
| Error | 错误图标 | 失败结果，说明对象与下一步；有重试操作时默认常驻 |
| Loading | 进度指示 | 不自动消失；结束后在同一张卡片切换结果 |

默认 `Standard` 使用随主题变化的中性表面，接近参考图的卡片层次。可选 `Rich` 使用对应状态的低强度背景、边界和前景 token；可选 `Inverse` 复用现有 Yue toast 反色 token。Rich 只对有意义的状态生效，不用颜色代替标题、图标或播报。

## 6. 八方位定位与安全区域

### 6.1 方位定义

```text
┌─────────────────────────────────────────────┐
│ TopLeft          Top           TopRight     │
│                                             │
│ Left                             Right      │
│                                             │
│ BottomLeft      Bottom        BottomRight   │
└─────────────────────────────────────────────┘
```

`ToastPosition::{TopLeft, Top, TopRight, Right, BottomRight, Bottom, BottomLeft, Left}`。Top / Bottom 指水平居中；Left / Right 指垂直居中。没有窗口正中心的位置。默认 BottomRight；RTL 不偷偷翻转这些物理位置，文字和动作的 leading / trailing 则按阅读方向镜像。

安全矩形 `R = (x₀, y₀, W, H)` 是当前窗口内容可用区域，扣除默认边距和宿主四边保留量（top / right / bottom / left）后的结果，不使用显示器绝对坐标，也不重复扣除已不在内容区内的系统标题栏。四边可分别表达工具条、侧栏和状态栏避让。`w` 为已约束卡片宽度，`h₀` 为前景卡片实测高度：

| 方位 | 前景卡片最终左上角 | 旧卡片的生长方向 | 新卡片入场方向 |
| --- | --- | --- | --- |
| TopLeft | `(x₀, y₀)` | 向下 | 从上向下 |
| Top | `(x₀ + (W-w)/2, y₀)` | 向下 | 从上向下 |
| TopRight | `(x₀ + W-w, y₀)` | 向下 | 从上向下 |
| Right | `(x₀ + W-w, y₀ + (H-h₀)/2)` | 默认向下 | 从右向左 |
| BottomRight | `(x₀ + W-w, y₀ + H-h₀)` | 向上 | 从下向上 |
| Bottom | `(x₀ + (W-w)/2, y₀ + H-h₀)` | 向上 | 从下向上 |
| BottomLeft | `(x₀, y₀ + H-h₀)` | 向上 | 从下向上 |
| Left | `(x₀, y₀ + (H-h₀)/2)` | 默认向下 | 从左向右 |

四个边角默认仍采用上下的单轴入场，不叠加斜飞或旋转。侧边居中不是把整个展开列表重新居中：**前景卡片的中心是锚点**，展开不能让前景随总高度上下移动。

Left / Right 区域在一次非空会话开始时选择向下或向上生长：下方空间能容纳默认露边时优先向下，否则向上；该方向在本次会话内保持稳定。展开后空间不足采用区域约束和滚动，不在 hover 时突然翻转。窗口明显缩小后可以在安全状态重新求解；过渡合同见第 8 节。

### 6.2 V1 单区域与位置切换

八方位是一个窗口通知区域的八种可选位置。V1 每窗只挂载一个通知栈，位置由 Toaster 配置，单条消息不再独立覆盖位置；不增加多区域注册表、碰撞优先级或 `WaitingForSpace` 队列。

窗口状态为空（含退出项和模态待展示项均已释放）时可更改位置。有消息时改为其他位置返回明确的 Occupied；不静默搬移现有卡片、不让下一条消息在另一区域另起一个栈。Showcase 切换方位时可由演示页显式清空，并在退出结束后应用新位置。

单区域仍须处理窄窗、长文字、侧中点生长和展开滚动。多区域同时展示与碰撞仲裁留待真实使用场景出现后单独设计，不属于八方位 V1 的隐含承诺。

## 7. 多张卡片：顺序、堆叠、展开

### 7.1 三种顺序分开定义

- **存储 / 创建顺序**：保留每条消息稳定的创建序号；新建追加，同 ID 更新不改序号。
- **前后层次**：最新创建的 active 卡片在前景，旧卡片向生长方向露出。Base 当前顺序为旧到新，最后一项是前景，适配时不能再反转一次。
- **消失顺序**：按每条消息自己的剩余时间 / 关闭请求决定，不强制所有消息严格 FIFO 或 LIFO。展示顺序不等于关闭顺序。

### 7.2 折叠态

默认最多呈现三层：前景完整显示；后两层只显示卡片壳和露边，不显示压缩文字、动作、关闭按钮或倒计时进度条。后景可用来表达“还有消息”，不能变成文字与图标叠在一起的缩略卡片。

设 `r=0` 为前景，`r=1,2` 为后景，默认露边 `p=12px`，宽度缩减比例 `s=0.05`：

```text
可见宽度 wᵣ = w × (1 - s × r)
左右内缩 iᵣ = (w - wᵣ) / 2
露边偏移 dᵣ = r × p
```

宽度缩减采用布局内缩，**不缩放正在阅读的文字**。当前 GPUI 没有适用于普通 Div 子树的通用 CSS transform，布局内缩也更适合保持文字排版与命中一致。折叠壳高度以当前前景高度为基准；各卡片自身的自然高度另行保留。旧卡片很高也不能从前景后面漏出正文或形成一块长尾。Base 当前保留每张完整高度，不能直接等同于这个合同，需补“自然高度 / 折叠可视壳高度”分离与裁剪。

自然高度在约束宽度下单独测量，不能拿裁剪后的壳高覆盖缓存。裁剪施加于每个折叠后景的内部内容，卡片壳与阴影保留在裁剪层外；不在栈容器上统一 `overflow_hidden` 切掉前景阴影和焦点环。

```text
Top / TopRight                         Bottom / BottomRight
┌── 最新 C，完整内容 ──┐                  ┌── 最旧 A 的露边 ──┐
└─────────────────────┘                 ┌── 较旧 B 的露边 ───┐
  └── 较旧 B 的露边 ──┘                ┌── 最新 C，完整内容 ──┐
    └── 最旧 A 的露边 ──┘              └─────────────────────┘
```

露边只是卡片壳示意；实际重叠的 z-order 始终是新卡覆盖旧卡，不能由 DOM / 迭代先后碰巧决定。阴影、边界与背景必须共同表达层次，不能只靠卡片色差。

### 7.3 展开态

指针进入通知栈的卡片 / 露边 / 卡片间连接区域，或键盘进入通知区域时展开。离开后若焦点仍在栈内、存在按压或可访问操作尚未结束，继续保持展开；所有交互原因解除后，经过 100ms 收起宽限再收起，避免经过边缘时抖动。后续启用拖动时，拖动也保持展开。

展开只呈现该区域当前允许的最新 `visible_toasts` 张 active 卡片，默认仍为三张；其余不是无限长滚动历史。所有可见卡片恢复全宽、自然高度和正文 / 动作，相邻间距为 `g=12px`。

若 `hⱼ` 是第 j 张卡片的自然高度，按前景到后景的方向，卡片 r 的锚边偏移为：

```text
dᵣ(expanded) = Σ(j < r) hⱼ + r × g
dᵣ(collapsed) = r × p
```

Top 以 top 为锚边，Bottom 以 bottom 为锚边；Left / Right 使用第 6 节锁定的生长方向和前景中心锚点。不把展开写成 `r × 固定行高`。前景的高度变化只能引起相关卡片重排，不能让别的位置或整个页面一起动。

展开区域受安全矩形裁剪，并在必要时承担唯一滚动；间隙的 hover 连通依靠观察指针位置，不铺一张阻挡底层操作的全窗口透明命中层。后景露边点击只展开，不触发其隐藏动作。

### 7.4 插入、更新与移除

| 事件 | 顺序与布局处理 |
| --- | --- |
| 新建 C | C 成为前景；A / B 从各自当前视觉位置朝新目标移动，不能先跳到旧端点 |
| 更新 B 的文案 / 类型 | B 保留 ID、位置、排序；自然高度重新测量，邻项连续重排；不重新播放整卡入场 |
| 删除前景 C | 固定 C 的退出快照；B 升为前景，A 前移；保留各自未消费的时间 |
| 删除展开中间项 B | B 原地退出，其他项填补其真实高度和间距形成的缺口，不整栈重新挂载 |
| 删除折叠后景项 B | 隐藏内容不重新闪现；壳体淡出，剩余层次收拢 |
| 连续加入 / 连续关闭 | 每个稳定 ID 重定向其独立的布局动画；不能以数组下标作为动画 ID |
| 同帧同时到期 | 在同一批次标记全部 Ending，再算一次目标布局；不强制制造连环的逐张关闭延迟 |

## 8. 动效合同：用户可见的核心验收项

动效不是“增加一个 transition”即可完成。**必须分别验收新消息的出现、旧消息的让位、折叠与展开、退出时的补位、内容更新，以及动画被下一次操作打断时的连续性。** 本节参数是设计目标，不能以 Base 的默认 `sonner()` 值或“看起来平滑”替代。

### 8.1 通道分工

每张卡片最终可见姿态由彼此独立的部分合成：

```text
最终位置 = 区域锚点 + 连续的栈布局偏移 + 入退场偏移 + 手势偏移
最终宽度 = 栈布局宽度（正文始终以真实宽度排版）
卡片壳透明度 = 存在性透明度
正文透明度 = 存在性透明度 × 后景正文显隐
```

不能让入场和重排同时写同一个 y 状态，否则新卡片到来会重置旧卡片的入场。几何布局、存在性和手势均以 `(window, area, toast_id, generation, channel)` 作为稳定身份；隐藏 / 重用 ID 时不会继承旧消息的弹簧或卸载任务。

V1 的手势偏移恒为零。通道需要落到实际布局节点，不能假设存在普通 Div 的 `.transform()`：

```text
窗口定位与内容视口裁剪层
└─ Base active-item 布局层：offset、左右 inset；拥有自然测量
   └─ Yue 存在性偏移层：相对布局偏移、入退场 opacity，不改变自然高度
      └─ 卡片壳 / 后景内部裁剪 / 正文显隐
```

Ending 使用关闭瞬间的最终 bounds、宽度、正文可见性和存在性采样，转入独立退出层；不再从 active-item 的新 rank 取位置。未来拖动偏移也应有独立包装层。所有包装由真实布局移动，让命中与语义 bounds 随之变化；图标 V1 只交叉淡变，不依赖 Svg 专属变换实现通用缩放。

### 8.2 默认参数

| 动作 | 默认节奏 | 位移与透明度 | 完成条件 |
| --- | --- | --- | --- |
| 新卡片入场 | 300ms，`MotionEase::Enter` | 从所属窗口边缘外进入最终位置；前 150ms 完成 0→1 淡入 | 存在性入场完成后才进入 Present 和开始消费超时 |
| 栈的插入 / 删除重排 | 临界阻尼 spring，response 400ms，damping=1 | 从当前采样位置和速度到新目标，不额外淡出旧卡 | 几何误差 ≤0.1 逻辑像素且速度达到稳定阈值；response 不是强制卸载期限 |
| 展开 / 收起的几何 | 同一 400ms response 的 spring | 露边偏移 ↔ 实测高度累加偏移；宽度恢复 / 内缩 | 与上项相同；中途 hover 反向保留速度 |
| 后景正文显隐 | 150ms，`MotionEase::Fade` | 展开时 0→1，收起时 1→0 | 不以 scale 改变字号；收起先禁用隐藏操作 |
| 前景自动 / 手动关闭 | 200ms，`MotionEase::Exit` | 向所属外边移动 24px，透明度 1→0 | 退出完成再卸载；剩余项可同时补位 |
| 展开中间项 / 折叠后景退出 | 200ms，`MotionEase::Exit` | 冻结当前视觉快照，向外偏移 8px 并淡出 | 与前景共用退出期限；不穿过前景正文，Ending 不再吃鼠标或 Tab |
| Loading→结果图标 | 150ms | 同槽淡出后淡入，图标槽尺寸不变；与文案共用 fade-through | 只更新卡片内容，不让整张卡片退场重入 |
| 文案 / 色彩更新 | 150ms 淡变；高度使用重排 spring | 位置不重置，旧文案不可透过新文案重叠阅读 | 最后一次 revision 为真值 |
| 离开后收起宽限 | 100ms | 只延迟决策，不执行位移 | 再次进入 / 焦点仍在时取消 |

这里选择的是 duration 梯度：入场 `motion::rung(theme, 300)`、退出 `rung(theme, 200)`、显隐 `rung(theme, 150)`、宽限 `rung(theme, 100)`；曲线独立使用对应 `MotionEase`。不能把 300ms 入场替换成当前实际为 200ms 的 `MotionDuration::Enter`，也不能把 200ms 退出替换成当前为 100ms 的 `MotionDuration::Exit`。

布局明确采用 `Spring::new(motion::rung(theme, 400)).with_epsilon(0.1)`，默认 damping=1。400ms 是无阻尼固有周期，不是 400ms 必须静止；不使用 `settle_spring(theme)`，后者按当前 Settle=300ms 换算的 response 约为 204ms。固定 400ms 的 response 与 Sonner CSS 400ms transition 不等价，稳定时刻取决于位移、速度和容差。当前仓库不存在额外的主题级 time-scale token；示例慢放另由诊断时钟承担。24px / 8px / 12px 取 spacing token。帧采样用框架单调时钟，不依赖系统日期变化。

### 8.3 入场路径的精确定义

先以最终约束宽度排版并测得高度，在未测量时不得先绘制一张零高或错误高度的卡片。若最终 top 为 `y`、卡片高为 h、内容视口上边为 `v_top`、阴影保留为 q，则从上进入时：

```text
起始 top = v_top - h - q
终点 top = y
位移由 Enter 曲线插值；最终稳态边距等于安全边距
```

下、左、右对称处理，侧边用宽度而不是高度计算。q 从解析后的 `--box-shadow-toast` 同一绘制数据求保守外延；当前默认是 `0 12px 36px rgb(0 0 0/.16)`，不能把 blur=36px 机械当作四边外延。视口裁剪所有者是覆盖窗口内容区的每窗容器，不是栈自身；窗口边缘可裁剪超出内容区的阴影，静止卡片保留安全边距。出发点和整条轨迹均受该内容视口限制，不能从标题栏或另一个窗口上方穿过。若安全区域不是窗口边缘，入场可穿过本窗口未保留的内容，但不能盖住宿主标记为保留的区域；存在实际保留障碍时改为最终位置 24px 的短距平移淡入。

角落的入场轴由位置固定，不根据哪条边“这一次更近”改变方向。前景最终位置、阴影、命中区和可访问几何必须一致；不能只是把绘制平移了，按钮仍在旧位置响应点击。

### 8.4 四个必须可复现的时间线

**新卡片到来**：已有 A；t=0 接受 B、测量最终尺寸并采样 A 当前姿态；下一可见帧 B 从边缘进入，A 同时让位；t≈150ms B 已可读；t=300ms B 入场完成；A 的 spring 继续收敛到露边位置。不能等 B 全部进入后再让 A 突然退后。

**hover 展开后立即离开**：以一帧采样值为转折点，位置、宽度都保持 C0 连续，重排 spring 保留速度；不能重建卡片从全折叠端点再播放一次。再次进入发生在 100ms 宽限内时取消收起。

**退出时又来一条**：C 已进入 Ending；D 到来只重算 active 项的布局，C 使用关闭那一帧冻结的位姿完成自己的退出；C 卸载任务仅能操作其 generation。D 不能被 C 的旧卸载任务关闭。

**异步结果导致内容增高**：Loading 和 Success / Error 使用同一稳定 ID；图标同槽切换，文案在原位置更新；重新测得的自然高度成为新的布局目标，邻项从当前姿态移动，不出现先清空高度、栈坍塌一帧、再撑开的过程。

### 8.5 边界与减少动态效果

- 同一张卡片在 Starting 中被关闭，从当前姿态退出，不先完成入场再走一次退出。
- Starting / Present 的普通 `update` 保留存在性进度。Ending 后 `update` 返回 AlreadyClosing；显式重新 `push` 同业务 key 创建新 generation，并从当前视觉样本衔接，旧回调不得影响它。`Entering / Exiting / Absent` 是 Base Presence 的另一套枚举，不能与 Toast 状态混用。
- 调整 rem、主题、窗口宽度或标题字体时重新测量。宽度 / 字体变化先恢复正确排版，再进行有界的布局重排；不得把整个通知区当图片拉伸。
- 窗口缩小导致当前轨迹会越界时，安全约束优先：立即把动画目标钳在新视口。若避让需要很大的位置修正，使用一次原位淡变和新几何定位，不长距离追赶；V1 不支持非空通知区主动跨方位迁移。
- Reduced motion：取消平移、缩放、弹簧旅行和循环 spinner；显示静态加载图标和“正在…”文本，布局直接采用终态。生命周期的 Starting / Ending 以零时长结算，不仍等待 300 / 200ms。自动关闭时长与键盘合同不变。
- 帧预算仅用于未稳定动画、可见加载指示、正在运行的可见倒计时条或实际交互。倒计时条作为明确的可见动画例外，按窗口刷新节奏连续更新，直接由同一单调时间快照计算比例，不另设每卡 timer，也不每帧推进全体 Manager。未显示进度条、被遮挡、暂停或 reduced motion 时停止其逐帧调度；reduced motion 的静态剩余比例只在其他必要刷新时更新，最终超时仍由 deadline 准时结算。V1 不采用 100ms 一跳的进度动画。所有通道稳定后停止请求帧，生命周期超时由最近 deadline 的一次性任务驱动。

## 9. 计时、消失顺序与容量

### 9.1 独立计时

| 消息 | 默认主动展示时间 |
| --- | --- |
| Normal / Success / Info | 4 秒 |
| Warning / Error，无动作 | 8 秒 |
| Loading、有可用操作按钮、明确常驻消息 | 不自动关闭，由更新或关闭请求结束 |

自动时长可显式覆盖，None 表示常驻；不使用 Infinity、0 或负数作为秘密哨兵。生命周期采用 `Waiting → Starting → Present → Ending → Removed`。V1 的 Waiting 指模态存在、窗口不可呈现或首个有效布局尚未准备好时的消息，不是 Base 当前已存在的枚举值，也不是多区域碰撞队列。首个有效可见帧开始 Starting 的入场计时，不把接受请求到完成初始测量的等待算进入场时长。

单卡片进入 Present 后才开始消费时长；退出的 200ms 不从阅读时间中扣除。通知区域 hovered / focused / pressed，或窗口不可见 / 不活跃、存在模态时，暂停剩余时间；多种暂停原因采用集合或引用计数，不用一个 hover=false 覆盖 focus=true。拖动若在后续加入，也作为一个独立原因。

离开后继续剩余时间，不从头给完整 4 秒。新消息到来不会重置旧消息；纯文案更新默认保留剩余时间；显式更新 timeout 重设该项的剩余时间；Loading→结果按结果类型开启一次新的结果计时。

倒计时进度条可选，仅对启用自动关闭的消息显示；Loading 和常驻消息不显示。进度等于本轮剩余阅读时长 / 本轮总阅读时长，从满到零；与关闭共用同一计时源，暂停时冻结，恢复后继续，不另设一个关闭计时器。显式重设 timeout 时同步重设进度；到零后进入 Ending 并播放正常退出动效。含操作按钮的消息也可显式指定自动关闭时长，此时遵守同一进度规则。

### 9.2 消失顺序示例

```text
t=0s  A 开始 Present，4s
t=1s  B 开始 Present，4s        前景顺序 B / A
t=2s  C 开始 Present，8s        前景顺序 C / B / A
t=4s  A 自动关闭               C / B 连续补位，C 不重新入场
t=5s  B 自动关闭               C 成为唯一卡片
t=10s C 自动关闭
```

这里恰好 A→B→C，是因为截止时间依次到达，不是隐藏的 FIFO 规则。若 B 是短时反馈、A 是 Loading，B 可以先消失而 A 继续保留；手动关闭哪一张就退出哪一张。

若 t=3s 开始 hover 到 t=5s，相关区域所有正在运行的阅读计时统一暂停 2 秒，对应截止时刻顺延；入场、退出和重排动画仍正常完成，不能把 pause_timer 实现成冻结整个 UI。

### 9.3 可见数量不等于可靠待展示队列

沿用 Sonner 的关键区分：`visible_toasts` 决定可见卡片层数，默认三张，不承诺每条消息都有完整的阅读窗口。已经进入 active 的后景项，即使暂不在三张可见项内，仍按自身时钟到期；它们不是等前面消失才开始计时的 FIFO 队列。

Yue 另作交互增强：隐去的视觉卡片不保留透明命中区或 Tab 停靠，不重复创建播报节点。独立播报通道可以交付该消息的文字一次，但不能把隐藏卡片的按钮暴露为可操作控件。这个语义约束不归为 Sonner 已实现行为。

这适合短时操作反馈；必须保证阅读、必须响应的消息应改用常驻消息、就地状态或持久通知中心。连续同类业务事件由宿主用稳定业务 key 合并，例如“已导入 12 个文件”，不要产生 12 张一闪而过的卡片。

V1 每窗默认最多接受 32 条尚未 Removed 的消息，包含 Waiting / Starting / Present / Ending；限额统一可配且必须大于零。满额时返回明确的 Capacity 错误，不静默删除 Loading、含动作或聚焦消息；同 ID 的原位更新不占新名额。无需自动保存历史，Removed 后释放文案、回调、任务引用和测量记录。连续 dismiss_all 也不能绕过挂载上限。

32 是限制突发请求和回调 / 测量内存的产品默认值，可由宿主调整；不是 Sonner 的历史上限换算。模态待展示项共用这个总额，不另建无限等待列表；不设隐式等待超时，宿主可显式取消，窗口释放必清理。

## 10. 操作、关闭与拖动

### 10.1 关闭语义

精确关闭原因：`Timeout / CloseButton / Keyboard / Action / Programmatic / ScopeDisposed / WindowClosed`。Keyboard 对应通知区内显式 Delete 命令；ScopeDisposed 供宿主显式结束业务作用域时传入，不绑定页面切换；后续加入拖动时再增加 Swipe。需要区分**接受关闭请求**与**退出完成**；每次 generation 的结束最多发生一次：

1. 检查本次请求资格，进入 Ending，停止自动计时并关闭事件命中。
2. 固定退出快照；通知已接受的 reason，回调在状态更新后调用，允许宿主发起新的 Toast 请求。
3. 退出动效完成后 Removed，发出完成事件并清理资源。

默认关闭按钮可见，允许鼠标 / 键盘关闭；`.dismissible(false)` 关闭用户关闭入口（未来也包括拖动），但不能阻止宿主程序化撤销。Loading 默认常驻且不显示普通关闭；如果业务确实支持取消，应提供明确的“取消导出”动作并由宿主取消任务，不把关闭消息误当作取消业务。

操作回调返回 `Close / KeepOpen` 决策，避免自动关闭后才发现操作失败。对启动重试等异步行为，回调先更新同一张卡片到 Loading 并 KeepOpen；一次按压只能触发一次业务动作，后续结果仍使用 generation ticket。

### 10.2 后续可选扩展：拖动关闭（不属 V1）

V1 用关闭按钮、明确的键盘命令和程序化关闭完成桌面主路径，不注册卡片拖动。下列为后续扩展的研究约束，不预留一个默认开启但不完整的实现。

| 区域 | 允许的外向方向 |
| --- | --- |
| Top | 上 |
| Bottom | 下 |
| Left / Right | 左 / 右 |
| TopLeft / TopRight | 上、左 / 上、右 |
| BottomLeft / BottomRight | 下、左 / 下、右 |

将来启用时，仅从卡片非按钮内容区开始，超过 4px 才锁轴。右键、滚动、文字选择、按钮点击不启动拖动。候选阈值是距离达到 48px，或距离至少 8px 且释放速度达到 0.25px/ms，并且方向被允许；禁止方向只能阻尼移动，不能仅靠很高速度通过关闭判定。这些是尚待手感验收的 Yue 候选值，不沿用 React 源码的 45px / 0.11px/ms 常量。

跟手期间采用直接位移，不让 spring 落后鼠标；未达阈值时从释放位置带速度回弹到零。达到阈值时沿释放方向离开窗口并在 200ms 内淡出，不改成默认区域退出方向。必须先核实指针捕获与全局释放能力，再决定复用的 Base 机制；现有 `constrained_drag` 的手柄 / 视口约束语义不等于整卡滑动关闭。释放、取消和窗口失活均须清理。

## 11. 原位更新与 Rust 异步结果

Sonner 的 Promise API 值得借鉴的是“同一通知从加载切到结果”，不是 JavaScript Promise、ReactNode 或 `.unwrap()` 的表面命名。

`ToastTicket` 持有弱状态引用、窗口身份、ToastId 和 generation / operation revision：

```text
push Loading → 得到 ticket
宿主执行 Future<Result<T, E>>
  Ok(T)  → ticket.update(ToastSpec::new(...).kind(Success), ...)
  Err(E) → ticket.update(ToastSpec::new(...).kind(Error).action(...), ...)
关闭 / 窗口释放 / 新一次重试 → 旧 ticket 结果被拒绝，不复活卡片
```

- `push` 是新建，`update` 是已存在项的原位替换；不把 Base 的同 ID push 当作 update。
- 纯文案 / 外观 / 图标更新保留准确的剩余阅读时间：先在原时钟上结算到 update 时刻，再替换内容；除显式重设 timeout、Loading→结果外不重置。本要求修正 Sonner timer effect 重建的边界，不以保留一个 remainingTime 引用作为充分证明。测试例：进入 Present 后 4s 到期，t=2s 更新标题，仍在 t=4s 进入 Ending。
- 更新以完整描述符替换展示字段，不隐式继承上一轮 action、description、loading 或 close 设置。需要局部 patch 时使用有明确 Keep / Set / Clear 的独立变更类型，禁止用 Option 同时表示“未修改”和“清空”。
- 异步任务和错误类型由宿主保留。Toast 只提供安全的状态更新票据 / 薄适配，不吞掉业务 Result，不默认把 HTTP 响应规则塞进通用组件。
- 同 key 重试增加 operation revision；较早请求即使后完成，也不能覆盖新请求的结果。
- 用户关闭消息并不自动取消业务 Future。反过来，任务取消需要明确的取消结果，不能永远留下 Loading。
- V1 提供 `update` / `begin_operation` 票据而不增加 `track_future` 或 `success/error` 别名；宿主保留原始 Result、任务与取消权，Showcase 给出两秒异步更新示例。不同时创建一条 success 消息和一条 error 消息竞争替换 Loading。

## 12. 状态宿主、基础能力和公共接口

本节记录公共接口的职责；完整签名见源码与第 20 节示例。`YueToasterState` 由 RootPlugin 构造，使用前须完成 Yue 初始化并挂载 Base Root。

| 类型 / 命令 | 合同 |
| --- | --- |
| `ToastSpec` | 不可变消息描述符：标题、类型、描述、图标、最多一个动作、超时、可关闭、是否显示倒计时进度条、外观和播报策略；自身不启动计时或业务任务 |
| `YueToaster` | 窗口通知呈现入口，引用唯一 `YueToasterState`；由 RootPlugin 安装，宿主不重复挂载第二份 |
| `YueToasterState` | 每窗保留的唯一消息所有者；由 RootPlugin 构造，不能每次 render 新建 |
| `ToastId` | 窗口内稳定身份；支持显式业务 key，返回的完整句柄还带 generation |
| `ToastTicket` | 弱引用和 revision 保护的异步更新票据，不拥有业务任务 |
| `ToastPosition` | 第 6 节八个物理方位，属于窗口 Toaster 配置，不属于逐条消息更新 |
| `ToastAppearance` | Standard / Rich / Inverse |
| `ToastDismissReason` | 第 10 节精确原因 |
| `for_window(window, cx)` | 获取该窗口的状态 Entity，初始化 / Root 缺失时返回明确错误，不创建第二个 Root |
| `push(descriptor, ...)` | 接受新消息并返回 ticket / id；容量或无效数据返回错误 |
| `update(ticket, descriptor, ...)` | 保持顺序、位置和存在性；验证窗口、generation、revision 和是否 Ending |
| `dismiss(ticket, ...)` / `dismiss_all(...)` | 接受一次退出；重复请求幂等，仍保留退出动效 |

V1 内部保留一个 `ToastManager<u64, Record>` 和一个 `ToastStackState`：Manager 的 u64 是窗内单调 generation，业务 ToastId 在 Record 中，因此同业务 key 的旧退出项与新项可以短暂共存。尚未呈现的等待请求由同一宿主有界保存，模态 / 窗口资格与初始测量准备完毕后再移入 Base 的 Starting，不靠一张 opacity=0 的交互卡片假装“还没展示”，也不将同一记录同时存入两份消息数组。对外配置使用私有字段、构造器 / builders 与只读 readers；卡片由内部 `card.rs` 呈现，不把 `YueToast` 同时用作元素名和描述符名。

### 12.1 实施前的 Base 缺口与扩展依据

| 当前实际行为 | 所需最小扩展 |
| --- | --- |
| `ToastManager::push` 同 ID 重新排到队尾并变为 Starting | 明确原位 update，保留顺序与阶段，并支持独立的超时重设政策 |
| `advance(now, paused)` 的 paused 是本次推进参数，Manager 不保存全局暂停字段；剩余时间私有 | 暂停原因集合由窗口宿主维护；增加剩余时间快照与下次有效截止时间读取，不复制一套计时器 |
| 生命周期 duration 同时充当 Base 默认 spring response | 将存在性 enter / exit deadline 与重排 spring response 分开，不能因 400ms spring 默认值而强制所有卡片入场等 400ms |
| Toast root 固定 Alert | 支持通知语义 / 播报策略和必要的 AccessKit 信息写入；不能假设覆盖外层 role 就生效 |
| ToastStack 折叠按 `max(自然高度 + peek × rank)` 撑大，且有 `tall_behind` 回归固定此行为 | 增加可选的以前景壳高折叠策略，保留默认策略以免破坏其他消费者；自然测量与可视壳裁剪分离。新增策略测试，勿静默改写原有断言 |
| ToastStack 高度 / 宽度测量私有，只识别上下锚点 | 暴露只读测量快照；增加侧中点锚定和首帧测量合同，不能在外层读取零高度后下一帧跳回 |
| Stack 对所有 children 计算 rank，不知道谁是 Ending；Manager `visible(limit)` 已包含全部 Ending | 在 Stack 输入中显式区分 active 与 Ending 快照。Ending 不占 active 名额、rank 或高度，也不被折叠层数隐藏；保留 Manager 已正确的过滤行为 |
| `is_expanded()` 反映 hover / focus，不直接等于所有计时暂停原因 | 暴露必要 hover / focus 状态；Yue 合并 pressed / inactive / modal 等原因，未来再纳入 dragging，不在 render 中猜测 |

补齐时还须满足以下跨层合同：

- 生命周期配置须能分别表达 enter deadline、统一 exit deadline 与 layout spring response；现有一个 `duration` 无法同时满足 300ms 入场和 400ms spring。语义名称与具体 API 在实现时确定，不伪装成现成字段。
- `next_deadline` 覆盖 Starting→Present、Present→Ending、Ending→Removed 三种期限；paused 只排除 Present 的自动关闭期限，不能让入场完成或退出卸载失去调度。
- 暂停原因切换前先以旧暂停状态推进到该时刻，再切换状态并重排 deadline；否则一次性调度会把整段此前已读时间错误地当作暂停，或把暂停时间错误扣掉。迟到的回调须结算跨阶段的时间余量，而不是从回调真正执行时才另起一整段寿命。
- 测量快照至少能读当前约束宽、各项自然高度、前景高及折叠 / 展开 extent；缺项标记为未测量。新项先参加无绘制、无命中、无可访问节点的测量，必要时多等一帧；提交新几何前，旧卡片保留上一组有效布局，不把缺测高度当作零发布。中点锚定由 Base 内部对同一份有效快照求解，不用两个不同帧的高度相减。
- 收起宽限需要受控 expanded 目标或等价交互策略接口；目前只有 `is_expanded()` 无法实现 100ms 宽限。拖动状态接口待相应功能加入时再扩展。
- 生产 transition / spring 使用 `cx.background_executor().now()`，已有 headless executor 的 `advance_clock()` 可直接复用，并非没有可控测试时钟。实时 Showcase 慢放 / 单步还缺局部诊断时间输入；为这一用途补可选采样时间与帧调度入口，默认保持现有行为，详见第 16 节，不为普通测试再造一套全局时钟。
- Base 的现有 `.invisible()` 已有隐藏绘制的用途，不重写相同能力；仍要验证整棵子树的命中、焦点与语义，以及 Ending 被错误传入普通 rank 的组合情况。
- 不能把 `.invisible()` 的 paint 早退当作可访问隐藏的充分证明：GPUI `element.rs::prepaint` 在子元素 prepaint 之前建立语义节点，`Div::should_insert_hitbox` 也在 prepaint 路径。视觉显隐、语义 `set_hidden`、焦点停靠和事件资格须分别处理。
- `active_modal_scope` 当前可读，但 `set_active_modal_scope` 没有供 Toast 使用的变更通道。需要窗口级模态存在性通知，携带变更时的单调时刻；只在有效状态变化时发布，不能因每次 prepare 重复通知。Toast 订阅后及时结算暂停边界，具体窄接口在实现时确定。

每项扩展先增加能证明合同的 Base 测试，再给 Yue 使用。修改受维护的 `vendor/gpui-base` 时同步 `YUE-PATCH.md` 和消费者接入说明；禁止修改 Cargo registry 或复制整个 ToastManager / ToastStack 到 Yue 来规避缺口。动画的样式、数值和策略留在 Yue，Base 只提供通用机制。

## 13. 窗口、模态和命中边界

- 在 `yue_component::init` 中为未来窗口注册 Toast RootPlugin，在创建 Root 前完成；接在现有 popup → tooltip → dialog 注册之后，使 prepare 能读取本轮模态状态。每个窗口独立状态、计时、位置和焦点恢复，不能使用跨窗单例数组。
- Toast 是窗口内容覆盖层，不是独立系统窗口。页面导航不会自动销毁仍有效的窗口消息；宿主可用业务 scope 主动撤销某页相关消息。
- 层序为应用内容 < Toast < Popup / Tooltip；Dialog / 模态边界优先。具体采用 Root overlay 中的普通子树，不使用 deferred，也不新建 Toast priority 常量。GPUI 先绘制普通树，再绘制 deferred：当前 Popup 基础 priority=100、Tooltip=200，Dialog 根据 `--dialog-z-index`（当前 1000）、父模态及最高 Popup 动态取更高值。Toast 的普通树路径自然位于这些 deferred 表面之下，不能把注册在最后误解为自动盖过 deferred Dialog。
- `--layer-toast=50` 属于设计系统另一套层角色，目前没有运行时消费者，且其与 popover / tooltip 的数字关系不对应上述实现层序。实现 Toast 时不读取它当 priority，不改动该 token；未来若要统一层级 token，另作全组件映射，不能在本组件中偷偷混用。
- V1 任一窗口模态 Dialog 存在时（包括非交互的退出屏障），窗口 Toast 暂停计时并隐藏视觉、交互和播报；新请求按第 9.3 节总容量等待。Dialog 内校验和失败反馈使用就地消息。模态完全关闭后恢复原有消息剩余时间；成功关闭后再发结果 Toast 是推荐流程。模态内独立 Toast 区域不纳入 V1，不绕过 focus trap 和背景语义隐藏。
- 模态协调以 `active_modal_scope(window, cx).is_some()` 为准，不能仅检查 `scope.is_interactive()`。Toast 接收第 12.1 节拟补的窗口级变更通知，以事件时刻结算暂停；新命令和 deadline 回调也复核当前 scope。不得只靠下一次 Root prepare 才发现模态已打开，否则通知静止时计时器可能先触发。
- 与 `17-dialog.md` 的合同分别解释：后台 Popup / Tooltip 属于依赖触发器的瞬态浮层，模态打开时失效；Toast 是窗口消息，按本节暂停与恢复。该文“不排隐形队列”约束第二个独立顶层 Dialog 的 open，不等于禁止有界保存待呈现通知。本设计不修改 Dialog 的串行打开合同。
- RootPlugin 的全窗口定位容器不加 `.occlude()`、鼠标监听、cursor、tracked focus 或滚动行为，以免插入覆盖整个窗口的 hitbox；稳定 ElementId 本身不必禁止。hover 连通用 `window.on_mouse_event` 加实测 bounds 观察，参考 Base ToastStack。仅卡片 / 露边的必要区域接受输入；展开滚动时，仅真实通知区域作为滚动命中面，间隙不吞掉背景按钮点击。
- 窗口最小化、失活、视口不可用时暂停展示计时和无意义帧。窗口关闭释放所有消息、测量、截止任务和票据引用；旧 Future 完成后安全忽略。

## 14. 键盘与原生可访问性

### 14.1 焦点

- 新消息和异步更新绝不自动抢焦点。普通 Toast 内容不是 ListBox 的 Option，没有 selected 状态。
- 提供 `FocusToasts` Action，不默认注册全局 `Alt+T`，以免占用宿主菜单快捷键；Showcase 提供显式“聚焦通知”按钮。该 Action 进入当前非空通知区并展开，先保存原焦点；活动模态或 Popup 存在时不穿透抢焦点。使用项目现有 `actions!`、初始化 once-guard 和通知区专属 `key_context` 注册局部命令，不为每张卡片注册窗口拦截器。
- 只有主动进入通知区后，卡片和其真实按钮进入该区域的键盘访问顺序；普通页面 Tab 不因新消息不断增加停靠点。通知区域本身不成为焦点陷阱。
- 区域内 Tab / Shift+Tab 按可见顺序访问动作和关闭；越过首尾回到合理的应用顺序。Up / Down 可以移动到相邻可见卡片，仅用于焦点导航，不建立选择模型。
- Escape 仅在通知区持有焦点且没有所属子 Popup 时收起并恢复原焦点，不默认清空所有通知；背景输入焦点下的 Escape 不归 Toast 拦截。关闭卡片使用其关闭按钮，或卡片焦点下明确绑定的 Delete。
- Escape 仲裁遵循现有 Popup Runtime / Owner 路由：Popup / 模态先处理，已经消费的物理事件不再交给 Toast；否则仅通知区焦点路径的 Action 可以消费。Toast 不新增第二个 `intercept_keystrokes`，也不在收起后继续关闭父浮层。未处理的 Action 正常传播。
- 含焦点消息暂停自动关闭。手动关闭聚焦卡片后，优先移到邻近可见卡片；没有其他卡片时恢复先前仍可用的焦点；原目标已移除则交给宿主的合理后备焦点，不能保留悬空句柄。

### 14.2 播报与语义

Sonner 的 section 使用 `aria-live="polite"`、`aria-relevant="additions text"`、`aria-atomic="false"`。本地平台核实结果如下，不能直接复制 DOM 属性：

| 平台链路 | 已核实事实与实现约束 |
| --- | --- |
| GPUI pre 0.3.9 `elements/div.rs` | `AriaProperties` 没有 live builder；通用 `Element::write_a11y_info` 可以写原生节点。Base `Toast::render` 内部强设 Alert，会覆盖此前 `.role()`，须先补可配置角色 |
| AccessKit 0.24.1 | 有 `Live::{Off, Polite, Assertive}`、`Node::set_live`、无参数的 `set_live_atomic()`、`set_label`、`set_value`；没有 DOM 的 Relevant 对应物，也没有 `set_name` 方法。可访问 name 是适配器根据 label 等信息计算的结果 |
| Windows adapter 0.34.0 | 新增的具名 live 节点可触发 `UIA_LiveRegionChangedEventId`；更新时，name / live 变化或从被过滤状态重新纳入树也可触发。因此恢复一个仍带旧文案的 live 节点可能重复播报 |
| macOS adapter 0.26.3 | live 节点的新增或 value / live / 过滤状态变化可产生 announcement；只设置可访问名称不能视为所有平台都已覆盖 |
| AccessKit consumer 0.38.0 | live 未显式设置时沿祖先继承，根部默认为 Off；Role::Alert 本身不自动赋予 Assertive。只加 Alert 或只包一个空的 polite 容器不足以保证消息播报 |

Yue 采用**视觉卡片语义与独立窗口播报通道分离**：

1. 可见卡片保留标题、描述和真实按钮的可访问名称 / 关系，显式 live=Off；装饰图标、spinner、倒计时逐帧值不播报。折叠后景、完全隐藏与 Ending 的视觉子树不暴露动作、Tab 停靠或旧焦点。
2. 每窗一个轻量播报宿主。待交付事件以 `(window, toast_id, generation, semantic_revision)` 为键；每条逻辑消息最多保留当前版本，数量受同一个窗口容量约束。事件节点不跟随卡片 rank、宽度或折叠显隐变换，不能以视觉节点恢复作为再次播报的触发器。
3. 每个新事件节点直接承载可本地化的状态与正文：Role::Status + Live::Polite；宿主明确声明急迫时用 Alert + Assertive；同时用 `set_label` 和 `set_value` 写同一份播报文本，并设置 atomic。Off 不产生事件节点。一个事件一经交付，其节点身份和文本在后续布局帧保持不变。
4. 新消息的首个有效可见帧交付一次；Loading→结果及有意义的正文 revision 各交付一次。同一帧同一消息的多次更新合并为最终版本，不播放已经过时的中间文案。不同消息即使文字相同，也有不同事件键与节点，避免单个全局字符串槽因 name 未变化而漏报；同一批多个消息不能被最后一次字符串覆盖。提交的平台 TreeUpdate 与内部“待播报”标记分别记录，不能在未输出语义树时先标作已交付。
5. 模态等待、窗口不可用时不交付。进入静默前清除旧的 live 事件载荷 / 节点，并保留已交付 revision 记录；恢复时不把旧文案重新插回树，只交付尚有效、尚未交付的最新事件。消息进入 Ending / Removed 时清理对应事件；展开、重排、恢复窗口和 spinner 不新增语义 revision。这样既遵守背景隐藏边界，也规避“隐藏再恢复”自动触发旧 live 的问题，不把播报节点偷偷放到模态隔离之外。
6. 在 Base 增加窄语义 Element 适配，参考现有 `a11y_disabled.rs` 的转发模式及 `ModalBoundary::write_a11y_info`；不额外建立焦点 / hitbox，不发明 `.aria_live()`，不使用嵌套 Alert 补救。
7. headless 验证节点角色、label / value / live、稳定身份、静默清理和 TreeUpdate 去重；Windows / macOS 适配器事件条件另外核对。发送事件不等于读屏一定逐条读完，实际朗读节奏由用户用 Narrator / NVDA 等真实工具验收，不把本设计的源码核实写成已完成的读屏测试。

带动作消息默认不自动关闭，以免用户无法及时到达；状态不只用颜色表达；放大字号、长中文、RTL 和高对比环境仍须保持文案与按钮可读。真实窗口和读屏效果由用户检查，开发流程不执行实窗自动化验收。

关闭按钮的名称、通知区名称、默认加载文案及状态前缀提供可本地化文本入口；操作按钮文本始终由宿主传入，不在组件内写死“撤销 / 重试”。默认键盘 Action 也不依赖这些可变文案作为 ID。

## 15. Token 与既有组件复用

| 部分 | 复用来源 / 新增项 |
| --- | --- |
| Standard 表面 / 文字 / 边界 | 现有 `--surface-level-2`、`--text-primary`、`--text-secondary`、`--surface-level-2-border`；对比不足只能在主题层显式调整 |
| Inverse | 已有 `--box-background-toast`、`--box-color-toast`；它们在浅色默认是深底浅字，不能误称为 Sonner 白卡默认 |
| 圆角 / 阴影 | 已有 `--box-border-radius-toast` / `--box-shadow-toast`，实窗层次由用户验收 |
| 标题 / 描述 | 现有 typography 的 body / secondary 层级及字体资源，不复制 Sonner 的网页字体栈 |
| 间距、位移、展开间距 | 现有 `--space-4 / 8 / 12 / 16 / 24`；露边 12px、展开间距 12px、退出位移 24px |
| 操作 / 关闭 | 既有 YueButton / Tabler Icon，保留正常焦点环和命中区 |
| 倒计时条 | `--toast-progress-height`，沿用卡片语义前景 / 状态色，底部独立布局槽；不增加另一套时钟 |
| duration / easing / spring | 现有 Yue motion 梯度和 Base transition / spring / presence 机制 |
| 状态色 | Yue 既有 success / info / warning / error 语义角色，Rich 背景与前景成对选择 |
| 专属布局 token | `--toast-width`、`--toast-content-min-width`、`--toast-padding-block / inline`、`--toast-viewport-offset`、`--toast-collapsed-scale-step` |

现有 box toast 内边距为 8px / 12px，而本设计新卡片为 12px / 16px，差异是明确的桌面卡片密度选择。Standard / Rich / Inverse 共用本组件新的布局 token，Inverse 只复用既有反色颜色，不继承旧 box 的内边距；不静默改写原有 box token 合同。

新增 token 已在源设计系统的 `packages/tokens/src/component-tokens/toast.css` 登记，按现有结构接入 `_index.css`、词表和 `tests/token-toast.test.mjs`。测试覆盖四个 theme/accent profile × 四个 neutral tone 的解析、文字 / 焦点对比、几何和动效引用。随后使用现有同步链：

```powershell
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --write
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check
node tools/sync-yue-theme.mjs --check
```

`--write` 正常重建 `crates/yue-component/tokens/inventory.json`、内容哈希与 `theme/generated.rs`；禁止的是手改生成物 / 哈希，而不是禁止工具生成新的哈希。本轮只修订设计文档，不修改源系统、不执行同步、不把未登记名称写入运行时查询。

窗口时间策略（4s / 8s）、32 条容量、可见三张和交互阈值是配置 / 行为默认值，不混成色彩 token。所有可见动画参数集中在一个 Toast motion 投影中，不能在卡片、栈、页面各有一套数字。

## 16. Showcase 与验收矩阵

演示页按“单卡片 → 动态通知栈 → 八方位 → 状态更新”组织，不复刻截图中的网页安装说明。

| 演示区 | 用户可直接观察的内容 |
| --- | --- |
| 卡片组成 | 标题、描述、图标、可选关闭、最多一个操作按钮、可选倒计时进度条；短文案、长中文、长路径和窄窗口 |
| 类型 / 外观 | Normal、Success、Info、Warning、Error、Loading；Standard / Rich / Inverse；浅 / 深主题 |
| 八方位 | 按窗口周边的八个位置排列触发按钮；清楚标出最终锚点和入场方向，不能用八个普通列表按钮代替位置示意 |
| 堆叠 | 连续加入 A / B / C / D，显示创建时间、剩余时间和当前 rank；hover / 键盘展开，再关闭前景或中间项 |
| 时间策略 | 相同时长、混合时长、常驻、有动作、hover / focus 暂停；可以明确看到消失顺序 |
| 动效实验区 | 正常速度和 0.25 倍回放、暂停 / 单步；插入中再插入、展开一半反向、退出中更新 / 重建、增高重排 |
| 异步 | 成功、失败、重试、乱序完成、关闭后才完成；同一卡片 Loading→结果，不弹出第二张 |
| 约束 | 减少动态效果、窗口缩小、rem 放大、非空时切换位置的明确反馈、模态期间等待 |

正常演示和诊断时间分开：现有 headless 测试直接用 executor 虚拟时钟；实时 Showcase 的 0.25 倍 / 单步使用局部诊断时钟，同时驱动同一组生命周期、deadline、transition 与 spring。暂停时停止自动帧，只在用户单步或改变输入后采样；单步按固定的逻辑帧间隔推进。不能仅将 duration 乘四，或只慢放存在性而让布局 spring、阅读计时继续走真实时间。

该诊断入口是第 12.1 节显式缺口，在接入前不显示假装可用的慢放 / 单步控件；可先展示正常速度的真实 Toast。诊断时钟的默认生产路径没有全局缩放，不改变宿主其他组件。每个可见事件均可复现，不靠随机成功 / 失败作为唯一入口。

### 16.1 自动化重点

- 纯状态：创建 / 原位更新、同 ID generation、关闭幂等、混合时长、同帧到期、暂停原因并集、容量、Waiting 和窗口释放。
- 几何：八方位锚点、变高卡片、自然高度 / 可视壳高度分离、三层露边、展开真实间距、侧中点不漂移、滚动单一所有者、非空时位置切换被拒绝。
- 动画：固定时长 transition 在 0 / 25 / 50 / 75 / 100% 采样；spring 按具体单调时刻采样并等待静止，不能把 response 当作 100% 完成时间。重定向前后比较同一时刻的位置 / 速度，而非限制正常动画两帧之间的位移。
- 生命周期：Starting 不消费阅读时长；Ending 不可交互、退出前仍绘制、退出后释放；Reduced motion 即时结算；静止状态不持续请求帧。
- 原生元素 / 事件：真实按钮的点击、Enter、Tab、Escape、Delete；隐藏后景不误触，透明区域背景按钮仍能点击；一个物理 Escape 最多被一个当前有效层消费。拖动待后续单独加入验收。
- 异步：旧票据不能复活新 generation；重试的乱序完成不覆盖最新结果；清空动作 / 描述不会继承旧字段。
- 可访问：稳定节点、名称、关系、隐藏、聚焦恢复和播报事件去重；所有主题 / neutral tone 的文字对比和控件几何。

定量判据与必须固定的边界：

| 项目 | 判据 / 用例 |
| --- | --- |
| 重定向连续性 | 在同一逻辑时刻、改目标前后比较：位置 / 尺寸差 ≤0.1 逻辑像素，opacity 差 ≤0.001；spring 重定向保留采样速度。不是要求每帧只能移动 0.1px |
| Spring 静止 | 沿用 GPUI `SpringConfig::is_settled`：位移误差 ≤epsilon，速度绝对值 ≤epsilon × natural_frequency；几何 epsilon=0.1。单纯 response=400ms 不证明此时已经静止 |
| 透明度 | 始终在 [0,1]；无反向输入的一段入场 / 退出按目标单调变化、终态正确。帧间变化随采样间隔而定，不设不合理的固定 Δopacity≤0.02 |
| Ending 与可见数 | 3 张 active + 1 张 Ending 时仍绘制退出快照；active 的 rank、宽度、偏移和名额仅由 active 决定，不能让 `visible(3)` 的第 4 项挤掉正在退场或仍有效的卡片 |
| 未测量首帧 | 没有零高卡片、负 gap 偏移或侧中点先跳半张再跳回；约束宽度、rem、正文变更后用同一有效快照提交布局 |
| 展开中更新 | hover / focus 展开时更新同 ID 的高度，不重建展开状态、不清空高度一帧；t=2s 更新标题不推迟原定 t=4s 自动关闭 |
| 窄窗与穿透 | 宽小于 360px 时正文、唯一操作和关闭列不重叠；卡片间隙的后台按钮仍接收点击，滚动仅作用于通知滚动区域 |
| 进度与调度 | 进度与同一阅读时钟相符；暂停、重设、Loading→结果和 reduced motion 都无独立 timer 漂移；无进度 / 加载 / 未完成动效时没有常驻帧循环 |
| 模态与播报 | 模态打开的同一逻辑时刻停止阅读计时；退出屏障仍暂停；恢复不重新发布已交付 revision。不同 ID 的相同文本、同帧多条消息均不被单个文本槽覆盖 |

测试只覆盖已实现的合同；本文未运行或宣称上述新 Toast 测试已经通过。实现后执行相关 Base、Yue 和 Showcase 的定向回归，再运行格式、Clippy、token 同步和完整相关测试。

### 16.2 用户负责的真实 UI 检查

重点观察入场是否轻快、旧卡片让位是否连贯、后景是否只露边、展开是否让前景移动、倒放是否突跳、退出是否出现空洞 / 闪回、同 ID 更新是否变成新卡片、八方位方向是否一眼可理解，以及文字 / 关闭 / 操作是否重叠。屏幕阅读器朗读效果也属于真实平台检查，不由代码截图推断。

## 17. 实施顺序与完成标准

1. 先锁定本文第 4、6–9 节的 ToastStack 几何和动效合同；不先扩建通用列表组件。
2. 在受维护的 Base 中补齐第 12.1 节最小缺口和回归测试，保持单一计时 / 布局机制。
3. 实现 Token 投影和 ToastCard，先保证长文案、按钮分列、关闭命中、主题和可访问名称。
4. 实现可独立测试的 ToastStack 动效：插入、退出、展开反转、变高和三层露边；让用户优先审阅这些可见行为。
5. 接入每窗单区域 Toaster、八方位配置、安全边距、模态等待、异步票据和精确生命周期事件。
6. 完成第 16 节的可复现 Showcase 和自动化覆盖；用户进行真实 UI 检查后再记录视觉验收结果。

完成标准是“用户能看清卡片内容、理解卡片之间的关系、预测出现与消失、顺利操作并且变化连续”，不以存在一个队列或能调用 `push` 作为完成。

### 17.1 交付文件与验证入口

| 位置 | 实施内容 |
| --- | --- |
| `vendor/gpui-base/src/toast.rs` 及相关 motion / modal / 语义适配模块 | 第 12.1 节机制扩展与定向测试；保持现有默认折叠策略，Yue 显式选用新壳策略 |
| `vendor/gpui-base/YUE-PATCH.md` | 实际修改后新增 Toast extensions 记录：来源、行为差异、兼容默认、测试与消费者 patch 接入说明 |
| `crates/yue-component/src/toast/` | `mod.rs`、`model.rs`、`state.rs`、`card.rs`、`layer.rs`、`tests.rs`；语义转发机制在 Base `a11y_live.rs`，不复制整套 Base Manager / Stack |
| `crates/yue-component/src/lib.rs` / `application.rs` | 模块导出、prelude、公共文档与 doctest，以及 RootPlugin 的一次性注册 |
| `crates/yue-component/src/toast/tests.rs` | 优先使用已有 in-crate `#[gpui_kit::test]` 与 executor 虚拟时钟；若另增 `tests/toast.rs`，须在 Cargo.toml 登记 `required-features=["headless-tests"]` |
| `examples/windows-showcase/src/pages/toast.rs` | 演示页；同时接入 `pages/mod.rs`、`main.rs` 的枚举 / 持有状态 / 构造 / render，以及 `sidebar.rs` 的排序项 |
| 设计源 Toast token 文件、离线 inventory、generated | 按第 15 节先登记后工具同步，不直接手改生成物 |
| `README.md` | 文档入口、初始化 / 消费示例与验证命令 |

新增模块后，运行 Base 与 Yue 定向测试，再按仓库现有验证链执行；以下是实施阶段的验证入口，实际执行结果记录在第 20 节：

```powershell
node tools/sync-yue-theme.mjs --check
pwsh -File tools/test-gpui-base.ps1 -Offline -Filter toast::tests
cargo test -p yue-component --locked --features headless-tests --lib toast::tests
cargo fmt --all -- --check
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
```

Base 定向测试以实际新模块的测试名和当前 vendored crate 接入方式执行，不把独立 crate 的构建结果冒充 workspace 回归。真实 UI 检查仍由用户负责。

## 18. 源码索引与参考资料

所有 Sonner 行号对应本地 2.0.8 快照，后续源文件变更时以符号为准。本设计不宣称已测量该项目 gzip 体积、社区热度或最新发布版本。

| 文件 / 片段 | 研究用途 |
| --- | --- |
| [`package.json`](../refer/sonner-main/package.json) / [`LICENSE.md`](../refer/sonner-main/LICENSE.md) | 版本、peer dependencies 与 MIT 许可；原样移用代码时保留许可义务 |
| [`src/state.ts`](../refer/sonner-main/src/state.ts)，Observer / create / dismiss / promise | 订阅补发、稳定 ID、dismiss 后重建竞态、原位合并、Promise 结果、历史有界清理 |
| [`src/types.ts`](../refer/sonner-main/src/types.ts)，ToastT / Position / ToasterProps | 类型、六方位、操作、时间、关闭和外观配置边界 |
| [`src/index.tsx`](../refer/sonner-main/src/index.tsx)，21–43 / 101–241 | 默认值、实测高度、偏移、退出快照、独立计时与暂停 |
| 同文件，267–513 | 单卡片结构、指针捕获、轴锁定、阻尼、速度阈值和按钮关闭 |
| 同文件，604–897 | Toaster 状态、同 ID 更新不重排、多区域、焦点、polite live region |
| [`src/styles.css`](../refer/sonner-main/src/styles.css)，81–242 / 292–363 | 卡片排版、图标 / 按钮 / 外悬关闭、后景内容隐藏、折叠 / 展开与退出 |
| 同文件，467–534 / 536–582 | 浅深主题表面与 rich 状态色；不归入 81–242 的排版区 |
| 同文件，159–164 / 365–428 / 584–729 | Promise 图标淡入选择器、方向化滑出、loader / 图标关键帧、reduced motion |
| [`src/hooks.tsx`](../refer/sonner-main/src/hooks.tsx) / [`src/assets.tsx`](../refer/sonner-main/src/assets.tsx) | 文档隐藏暂停、状态图标和 loader 的具体实现 |
| [`test/tests/basic.spec.ts`](../refer/sonner-main/test/tests/basic.spec.ts) | 类型、Promise、关闭、焦点、主题、定向手势、ID 复用、历史上限和图标语义回归；没有把其通过视为 Yue 验证 |
| [`Base toast.rs`](../vendor/gpui-base/src/toast.rs) | Manager / Stack / Motion / semantic root 和已存在的行为测试 |
| [`Base root.rs`](../vendor/gpui-base/src/root.rs) / [`modal.rs`](../vendor/gpui-base/src/modal.rs) | 每窗插件生命周期、注册顺序与模态语义边界 |
| [`a11y_disabled.rs`](../vendor/gpui-base/src/a11y_disabled.rs) / [`Base motion.rs`](../vendor/gpui-base/src/motion.rs) | 窄语义包装的转发先例、spring 采样时间与默认参数 |
| [`Yue motion.rs`](../crates/yue-component/src/motion.rs) / [`12-popup.md`](12-popup.md) / [`17-dialog.md`](17-dialog.md) | 主题动效、非锚定通知与 Popup 的边界、模态协调 |

平台源码位于本机 Cargo registry，按锁定版本和符号定位，不把用户目录固化为项目依赖：

- `gpui-pre-0.3.9/src/element.rs::prepaint`、`elements/div.rs::{AriaProperties, should_insert_hitbox, paint, write_a11y_info}`：语义创建阶段、命中与视觉显隐的不同路径。
- `gpui-pre-0.3.9/src/elements/svg.rs::with_transformation`、`src/styled.rs`：Svg 专属变换与普通元素布局能力的边界。
- `gpui-pre-0.3.9/src/spring.rs::SpringConfig::is_settled`：位置与速度的共同静止条件。
- `accesskit-0.24.1/src/lib.rs`：Live / LiveAtomic、label / value 写入；`accesskit_consumer-0.38.0/src/node.rs::live`：祖先继承与 Off 默认。
- `accesskit_windows-0.34.0/src/adapter.rs::{node_added, node_updated}`（257–263 / 314–324）：具名 live 节点的 UIA 事件条件。
- `accesskit_macos-0.26.3/src/event.rs::{node_added, node_updated}`（228–239 / 298–305）：value、live 与过滤状态变化的 announcement 条件。

用户提供的 [Vue Sonner 页面截图](assets/toast-sonner-vue-reference.png) 和 [React Sonner 页面截图](assets/toast-sonner-react-reference.png) 用于功能与视觉参照。两张截图都不是八方位动画、关闭顺序或原生可访问行为的证明；具体机制以本地源码和本文明确的 Yue 决策区分记录。

## 19. 审核取舍（2026-11-02）

输入按顺序研读：[TEMP/1.txt](../TEMP/1.txt)、[TEMP/2.txt](../TEMP/2.txt)。两份都是审核意见，不作为高于用户要求或实际源码的规范；下表区分采纳、调整采纳与不采纳。重叠的事实核对归并记录，不重复粘贴报告。

### 19.1 第一份报告的处理

| 审核意见 | 处理与依据 |
| --- | --- |
| §二 1–3、§七：动效归属、duration 梯度和 spring 的解释 | 采纳。新增第 2.3 节，明确 Yue 自定规格；第 8 节固定 300ms rung 入场、400ms response 布局，不误用 Enter / Exit 语义时长，也不声称 response 是静止期限 |
| §二 4：文案更新影响 timer | 调整采纳。准确说是 effect 重启且未结算这段已运行时间，不是每次都把 remainingTime 引用重设为原始 duration；第 2、9、11 节给出 Yue 保留剩余时间及 t=2s 更新 / t=4s 到期用例 |
| §二 5–6：隐藏项语义、CSS 范围与露边 | 采纳来源区分和行号修正；12px 是 Yue 值。负缩放解释不采纳：自定义属性按 token 展开后为 `1 - 0.05 × rank`，第 2.1 节已列算式 |
| §三：paused 误诊、Ending 过滤与缺失接口 | 采纳。暂停是 `advance` 参数，`visible()` 已保留 Ending；真正缺口是剩余快照 / 三类 deadline、独立 motion 参数、测量、受控展开、active / Ending 布局分离，见第 12.1 节 |
| §四：平台 live 能力与必须使用具名节点 | 采纳平台事实和窄语义适配；不采纳“每张视觉卡片必须自带 live”。卡片折叠 / 模态恢复会影响过滤状态，故第 14.2 节采用独立事件节点，并同时覆盖 Windows name 与 macOS value |
| §五：普通 Root overlay、模态协调、Escape 与背景穿透 | 采纳实现路径与事件仲裁。调整“禁止容器 id”为禁止全窗口命中行为，因为 id 本身不在 `should_insert_hitbox` 条件中。不采纳让 Toast 位于模态遮罩之上：窗口消息遵守背景模态隔离 |
| §五：与 Dialog 不排队合同冲突 | 不按冲突处理。Dialog 的原文约束第二个独立顶层 Dialog；后台 Popup / Tooltip 失效与窗口 Toast 暂停是不同合同。第 13 节显式说明，补拟新增模态状态通知 |
| §六：八方位改六、统一 duration、推迟多区域 / 拖动、统一退出 | 保留八方位与独立入场 / 重排，这是已确定的可见体验；采纳每窗单区域、拖动延后、退出统一 200ms，减少非核心机制 |
| §六、§八：慢放 / 单步与动画采样 | 调整采纳。现有 executor 已能做 headless 采样；实时局部慢放才需额外时钟入口，见第 12、16 节，不把已有能力误写成缺失 |
| §七：命名、文件清单、token 生成、i18n | 采纳。描述符改为 ToastSpec，呈现入口 / 状态为 YueToaster / YueToasterState；补文件与验证链，允许同步工具更新哈希，保留本地化入口 |
| §八：每帧位移 ≤0.1px、Δopacity≤0.02 | 不采纳固定帧间阈值。正常平移动画本就会一帧移动多个像素，150ms 淡变也常超过 0.02；改为同一采样时刻的重定向连续性、范围 / 单调性与真实静止判据 |
| §九：8s / 32 条 / Alt+T / 四边保留区 | 8s 与 32 条保留为可覆盖的 Yue 行为默认值并标注来源；采纳不默认占用 Alt+T，以及四边分别避让 |

### 19.2 第二份报告的处理

该报告前半部分是逐项 fact-check，后半部分重新使用 A–D 编号；下表按标题区分，避免把同名编号视为同一意见。

| 审核意见 | 处理与依据 |
| --- | --- |
| Fact-check 的版本 / 六方位 / 默认值 / 同 ID / 退出保留 / 历史上限等确认项 | 核实后保留研究结论；补 Source 索引中的 CSS 精确范围、focus 非 Sonner 暂停原因、32 active 与 100 历史的区分 |
| Extras 2：负 scale | 不采纳，理由同第 19.1 节；不能给 `var(--scale)` 隐式添加原本没有的括号 |
| Extras 5：“只有 duration 变化重设 ref，所以纯文案更新保持准确剩余时间” | 不采纳该推论。`toast` 对象和 `deleteToast` 依赖也会重启 timer effect，cleanup 不结算 elapsed；应同时分析 ref 与 effect，而不是只看 `[duration]` |
| MOTION A4 / A5、精度 D1–D3：time-scale、梯度与状态词汇 | 采纳，移除不存在的主题时间尺度，明确 response 与 rung，统一使用 Toast 的 Starting / Present / Ending |
| 漏写硬约束 A1：普通元素缺通用 transform | 采纳。第 8.1 节写实际节点分层，图标改用同槽交叉淡变，布局内缩不等于整卡缩放 |
| 漏写硬约束 A2：普通树与 deferred 的层序 | 采纳。Toast 不用 deferred；修正“注册在最后必盖住 Dialog”的推断，deferred 仍按自身阶段与 priority 绘制，不能只靠注册顺序判断 |
| 漏写硬约束 A3：独立播报、过滤恢复会重报 | 采纳风险与解耦；不采纳“必须是一个永久具名字符串槽且永不被模态过滤”。相同文本和同帧多条会漏报，绕过 ModalBoundary 也违反隔离。用独立事件键、实际交付记录和静默清空解决，见第 14.2 节 |
| Base 冲突 B1 / B2 / B4 / B5：测量、中点首帧、Ending rank、deadline | 采纳。第 12.1、16.1 节补到可实现 / 可验收程度；deadline 中只有 Present 自动关闭受 paused 抑制，Starting / Ending 仍推进 |
| Base 冲突 B3：前景壳高推翻 tall_behind 测试 | 调整采纳。确认行为不同，但以显式可选策略加入 Base，保留原默认与原测试，再为新策略加回归和 patch 记录；不直接删改上游断言掩盖差异 |
| 范围 C1 / C2：多区域 / 拖动延后 | 采纳，并进一步明确 V1 一窗一栈、位置只在空状态更改，避免用“碰撞时如实重叠”作为最终体验 |
| 范围 C3：进度条每 100ms 一跳或移出 V1 | 不采纳。进度条来自用户明确列出的组成，可见动效优先；第 8.5 节明确它是有界的逐帧动画例外，暂停 / 隐藏停止采样，关闭仍用单一 deadline |
| 范围 C4：Inverse 延后 | 暂不采纳。复用已有反色颜色、共用新卡片布局即可，不必改旧 box 的 8 / 12px 合同；不增加新的按钮或交互档，见第 15 节 |
| 精度 D4–D8：CSS、focus、容量、等待与阴影 | 采纳。统一窗口容量、无隐式等待超时，修正裁剪所有者；阴影外延由解析绘制数据求解，不把 36px blur 当四边固定 q |
| “已核实没问题”：全局 paused、invisible 已保证 a11y 隐藏 | 不采纳这两项结论。Manager 没有全局 paused 字段；GPUI prepaint 先创建语义节点和命中，paint 早退不能证明前面未发生，见第 12.1、18 节 |
| “已核实没问题”：已有 advance_clock / debug_bounds 与 token 同步链 | 采纳并保留。区分现有 headless 采样与未有的实时诊断控制；哈希不能手改，但工具正常再生成属于必要流程 |

### 19.3 设计审核轮的验证边界

前述设计审核轮仅修改本文档：逐份研读审核报告，重新核对 Sonner / Base / GPUI / AccessKit 的相关实现，修订合同并检查 Markdown 本地链接、章节、代码围栏和旧约定残留。该轮未实现 Toast、未修改 Rust / token / 审核报告，未运行 Cargo 构建或真实窗口检查。后续实施及其验证单独记录在第 20 节；真实 UI 由用户负责。

## 20. 实施记录

### 20.1 实现范围（2026-11-02）

- `toast/model.rs`：ToastSpec、单个 ToastAction、六种类型、三种外观、八方位、超时与播报策略、容量 / 配置错误及精确关闭原因；字段私有，通过 builders 设置。
- `toast/state.rs`：每窗唯一状态、Waiting 到 Removed 的生命周期、暂停原因并集、最近 deadline 的一次性调度、弱票据、generation / operation 防陈旧结果、窗口关闭清理与事件。
- `toast/card.rs`：自然高度、同行 / 次行动作、独立关闭列、底部进度槽、同槽图标、正文淡变和颜色过渡。关闭使用既有 YueButton 的 Small 28px 命中区，内部图形覆写为 14px，不新增公共按钮尺寸。
- `toast/layer.rs`：预测量、八方位安全区域、前景壳策略、展开 / 收起、独立退出快照、区域滚动、键盘导航及独立原生 live 事件。Toast 属于普通 Root 子树，不使用 deferred，不盖过 Popup / Tooltip / Dialog。
- `lib.rs` / `application.rs`：公共导出、prelude、可编译示例与 RootPlugin 的一次性注册。Showcase 的 ToastPage 在 Root 挂载后才获取窗口 Toaster，避免应用 view 构造阶段错误访问尚未安装的插件。
- 设计系统源新增 24 个 Toast token，再由同步工具更新本仓库 inventory 和 generated；没有手改生成哈希。Base 的具体扩展与消费者 patch 要求见 [YUE-PATCH.md](../vendor/gpui-base/YUE-PATCH.md)。

### 20.2 实施中的具体取舍

1. **测量与进场**：新消息先以无绘制 `layout_as_root` 预测量，提交有效自然高度后才进入 Base Starting；更新先保留旧展示 / 几何，直到新 revision 的测量有效。Base 默认自然高度折叠不变，Yue 显式启用 front-shell / center-front。
2. **正文与图标切换**：150ms 采用前 75ms 旧内容淡出、后 75ms 新内容淡入的 fade-through，不将两段文本同时叠在一起；图标与正文同槽切换。高度由 400ms response 的弹簧连续衔接；打断时从当前段落和 alpha 接续。描述符替换立即失效旧操作，淡变期间只保留不可交互的绘制占位。
3. **退出与重入**：Ending 从 active rank / extent 中排除，冻结视觉 bounds、壳高、内容、颜色和 spinner 采样。后景退出先画、原前景退出后画，避免后景壳盖住前景正文。同 key 再 push 使用新 generation，从仍在退出的样本衔接；旧项保留生命周期容量直到 Removed。
4. **侧边会话**：按首张有效自然高度和默认露边所需空间选择向下 / 向上，非空期间不翻转；展开空间不足使用区域滚动。清空后的下一次会话重新选择。
5. **焦点与遍历**：通知区和卡片使用局部 tab_group，Tab 按屏幕上从上到下的卡片、动作、关闭顺序访问，正常页面不增加消息停靠点；Up / Down 移动卡片，Escape 回到原焦点，Delete 关闭当前可关闭消息。新消息挤出被聚焦的旧可见项时，将焦点交给新前景，避免保留失效 dispatch 路径。FocusToasts 遵守模态和活动 Popup 的仲裁。
6. **原生播报**：视觉卡片 live=Off；独立具名 Status / Alert 事件同时写 label / value / atomic。语义 revision 与样式 revision 分开，已交付事件保留稳定身份；模态或失活隐藏后抑制旧事件，恢复不重新插入。不同通知即使文案相同，也有不同事件键。本地化可覆写 announcement_text、region_label、close_label。
7. **时钟与资源**：Normal / Slow（0.25 倍）/ Paused 以及 step 只影响本窗口 Toast，存在性、布局、正文和阅读时间同源。无动画、无可见运行进度 / spinner 时不持续请求帧。业务 Future 和取消权归宿主，V1 不引入通用列表、全局任务管理器、多区域、拖动关闭或额外业务按钮。

### 20.3 公共 API 示例

先 `yue_component::init`，再创建 Base Root；下面在挂载完成后的事件 / update 中调用，不能在 Render 中 push：

```rust
use gpui_kit::{App, Window};
use yue_component::{ToastError, ToastKind, ToastSpec, ToastTicket, YueToaster};

fn saved(window: &mut Window, cx: &mut App) -> Result<ToastTicket, ToastError> {
    YueToaster::push(
        ToastSpec::new("设置已保存")
            .kind(ToastKind::Success)
            .description("更改已写入当前项目。")
            .progress(true),
        window,
        cx,
    )
}
```

窗口为空时，通过 `YueToaster::for_window(window, cx)` 得到 Entity，再在 `update` 内调用 `configure(ToasterOptions::new().position(...).insets(...), window, cx)`。非空返回 Occupied，包括 Waiting 和 Ending，不隐式移动现有卡片。

异步流程用 `push(ToastSpec::new(...).kind(ToastKind::Loading), ...)` 返回的 ticket。宿主持有 Task，完成后调用 `ticket.update(ToastSpec::new(...).kind(ToastKind::Success), window, cx)`；结果原位切换并启动默认结果时长。重试前 `ticket.begin_operation` 返回新票据，使旧结果失效；关闭不会自动取消业务任务。普通 update 保留精确剩余时间，显式 `.timeout(Some(duration))` 才重开，`.timeout(None)` 表示常驻。

### 20.4 自动化验证

以下均为本地实际运行结果，不含真实 UI / GPU 或读屏验收：

| 命令 / 范围 | 结果 |
| --- | --- |
| `cargo test -p yue-component --locked --features headless-tests --lib toast::tests` | 26 项通过：初始测量、独立计时、票据、容量、壳高、八方位、退出冻结、原位更新、暂停并集、模态屏障、焦点 / Tab / 原生按钮、长卡滚动、播报键、同 key 重入、无持续帧、侧边会话、精确事件与窗口释放 |
| `cargo test --workspace --locked --features headless-tests` | 全 workspace 通过，含 Toast Showcase 两项交互回归及组件 doctest |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests` | 系统字体路径通过 |
| `cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings` | 通过 |
| `cargo fmt --all -- --check` | 通过 |
| `node tools/sync-yue-theme.mjs --check` | 投影一致 |
| 设计系统内 `node node_modules/vitest/vitest.mjs run tests/token-toast.test.mjs` | 17 项通过 |
| `pwsh -File tools/test-gpui-base.ps1 -Offline -Filter toast::tests` | Base Toast 14 项通过，包括原默认策略及新 front-shell 回归 |
| 同一 Base 定向测试入口，Filter 为 `a11y_live::tests` / `modal::tests` / `motion::` | 分别 1 / 1 / 50 项通过 |

Base 在 workspace exclude 中，独立验证工具仅在 ignored target 下生成临时 manifest / lock，库指向真实 vendor 源码；移除独立开发 manifest 中无条件 Fast 测试后端依赖，生产 pins 来自根 lock。没有修改 Cargo registry，不把这组测试混称为 workspace 回归。

### 20.5 用户检查入口

Showcase **组件 → Toast**：九宫格八方位、描述 / 图标 / 单个撤销 / 关闭 / 进度条开关、六类型与三外观；连续三张、长内容、原位更新、中间关闭、全部关闭、两秒异步、模态等待；四分之一慢放、暂停和 16ms 单步。位置切换显式清空，等退出完成再应用新位置。

本轮不启动真实窗口或进行截图检查。动效手感、主题 / DPI 下的实际呈现和系统读屏输出由用户在真实 UI 中确认；自动化通过不等于这些项目已验收。

### 20.6 圆角进度条与隐藏卡片退出闪烁修复（2026-11-02）

用户在真实窗口中报告倒计时条未融入卡片底部圆角，以及快速连发后的中层 / 底层卡片闪烁，并指出后者可能来自不可见通知的消失。两项问题均已在修改前通过回归测试复现，分别记录于 `target/toast-corners-before.log` 和 `target/toast-hidden-exit-before.log`。

**进度条的绘制边界。** GPUI 的普通 `overflow_hidden()` 提供矩形 content mask；父节点有圆角并不意味着后代会沿该圆角裁剪。原来的底部矩形 div 因而侵入圆角外部。现在 Canvas 绘制与卡片内表面同尺寸的圆角 quad，再用“底部条高 × 剩余比例宽度”的矩形 mask 显露倒计时。内圆角由卡片圆角减现有 1px 边框推导，沿用 `--toast-border-radius`、`--toast-progress-height` 与既有语义色；不把整条进度画成另一种胶囊，不改外阴影，也不增加时钟或 token。进度为完整宽度时左右两端都遵循卡片轮廓，缩短时移动端仍代表真实剩余比例。

**不可见生命周期与已挂载表面分离。** 超过 visible budget 的记录仍然必须独立计时，保持容量、票据和事件语义；但之前退出可见集合后，`Record.visual` 仍保留最后绘制的 bounds / opacity / foreground。它随后超时进入 Ending，退出层便把这份过期画面重新插入堆叠，造成闪烁。修改前的测试实际观察到了本应隐藏的 Rich Error 背景重新提交到绘制场景。

现在离开可见集合、或整个通知层因屏障 / 失活而隐藏时，明确使挂载快照失效：bounds=None、opacity/content=0、foreground=false。隐藏记录退出时没有退出表面，不会复活旧卡；真正仍可见的卡片继续冻结自身位置并执行退出动画。未修改 ToastManager 生命周期、阅读预算、事件发出或 Base 弹簧机制，也没有用延长动画来掩盖问题。

本轮 Toast 专项为 30 项，其中新增四项：底部条实际圆角 quad / mask、Light / Dark × 三种外观 × 1 / 1.5 / 2 缩放 × 两档 rem 下完整 / 部分 / 末段进度几何、不可见记录的超时绘制与移除、八方位下 16ms 间隔快速连发后关闭隐藏项的同时间可见几何连续性。原有可见后景退出快照、原位更新、同 key 重入、暂停、焦点和模态测试仍保留。

几何及场景断言不是 GPU 像素截图或肉眼验收。本轮不自行启动真实窗口、不提交 Git；用户复核入口仍为 `target/release/windows-showcase.exe` → **组件 → Toast**，重点观察连续点击后等待隐藏旧消息到期的过程。

| 本轮回归 | 实际结果与日志 |
| --- | --- |
| Toast 专项 | PASS：30 项；`target/toast-visual-targeted.log` |
| workspace 普通 / headless | PASS：156 / 470 项（含各自 doctest）；`target/toast-visual-{workspace,headless}.log` |
| 组件系统字体模式 headless | PASS：454 项（含 doctest）；`target/toast-visual-system-fonts.log` |
| 普通 / headless Clippy `--all-targets -- -D warnings` | PASS；`target/toast-visual-clippy.log`、`target/toast-visual-clippy-headless.log` |
| Showcase check / release | PASS；`target/toast-visual-showcase-check.log`、`target/toast-visual-release.log`；预览 exe 更新于 2026-11-02 08:21:22，56,028,160 bytes |
| workspace fmt、双模式 token 投影、diff / 本轮未跟踪文件尾空白 | PASS；未修改设计系统源 token 或 vendored Base |
