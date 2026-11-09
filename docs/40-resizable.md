# YueResizable 可调整分栏

状态：已实现并完成自动化回归，Windows 实窗效果待用户验收。面向 Windows GPUI 应用，复用 Base ResizablePanelGroup、ResizableState 和 ResizeHandle。验证证据与限制见[实施报告](research/2026-11-09-resizable-implementation.md)。

## 1. 用途与边界

用户拖动面板边界，为导航、正文、属性或日志分配空间。组件为原位、持久、复合布局；宿主持有尺寸状态，面板内容由调用方组合。横向表示左右分栏，纵向表示上下分栏，支持嵌套。

首版提供最小／最大尺寸、初始尺寸、固定侧栏、键盘调整、双击／Enter 复位、禁用与程序尺寸恢复。Dock 的拖放重排、浮动窗口、面板折叠不属于分栏合同。

## 2. 复用及分层

| 层 | 职责 |
| --- | --- |
| Base ResizableState | 唯一尺寸 owner、相邻边界约束、容器变化协调、键盘／拖动／复位共用计算 |
| Base ResizablePanelGroup／ResizeHandle | 命中、拖动、焦点、分隔器语义、面板测量及稳定身份 |
| YueResizable／YueResizablePanel | rem 参数、主题 token、分隔线呈现、内容组合 |
| 宿主 | 保留 Entity、持久化尺寸、给分栏有限宽高；每个面板自有滚动视口 |

Base 新行为显式启用，默认旧分栏行为保留。Yue 不维护另一份 sizes，不另写拖动引擎。

## 3. API 合同

```rust,ignore
let state = cx.new(|_| YueResizableState::default()); // 构造宿主时执行一次
YueResizable::horizontal("workspace", &state)
    .child(YueResizablePanel::new("navigation").label("导航")
        .initial_size(rems(15.)).min_size(rems(10.)).max_size(rems(24.))
        .fixed(true).child(navigation))
    .child(YueResizablePanel::new("document").label("正文")
        .min_size(rems(16.)).child(document))
```

`horizontal / vertical(id, &state)` 要求稳定组 id 和保留的状态 Entity。面板 `new(id)` 要求组内唯一稳定 id；`child`／`children` 接收内容。`initial_size` 仅用于初次建立及复位；未指定时参与分配剩余空间。`min_size` 默认读取组件 token，`max_size` 默认无上限。`fixed(true)` 须配合 `initial_size`，在容器改变时保留该面板当前尺寸，自己的分隔器仍可调整；未指定初始尺寸时按普通弹性面板处理。

YueResizable 支持 `child`／`children`、`disabled(bool)`、`theme(theme)`、`on_resize` 和 Styled 宿主布局。`on_resize` 在用户拖动结束、键盘调整或复位后调用，参数是当时的 Base 状态 Entity；容器测量与普通重绘不回调。程序调用状态的 `resize_panel`／`reset_layout` 使用状态事件订阅；`set_sizes` 用于恢复快照，校验数量与有限数值并按当前约束协调，不回送用户回调。状态及提交事件从 Base 有意重导出为 `YueResizableState`／`ResizableEvent`。

面板顺序改变时按 id 保留尺寸；删除正在调整的边界结束会话。状态不可同时绑定多个组或窗口。空组／单面板合法，单面板没有交互分隔器。

## 4. 尺寸与滚动

长度参数采用 GPUI `AbsoluteLength`，普通尺寸建议 rem；已测量／持久化尺寸可传 Pixels。初始值与恢复值服从 min／max；非有限或负长度属于配置错误，不能进入几何运算。

拖动只调整相邻两块，保持它们总尺寸及其他面板尺寸。可行范围同时考虑两边 min／max，不能因扩大左侧而突破右侧最大值。分栏位置不加缓动。固定面板保留用户尺寸，其他面板按当前比例分配容器剩余空间并钳制约束。

容器小于所有面板最小尺寸之和时保留最小尺寸，允许布局超出容器；宿主应提供合适最小窗口尺寸或外部滚动。容器大于所有最大尺寸之和时保留最大尺寸，剩余空间留空；不违反显式约束以强行填满。

每块内容包裹 `min_w_0 / min_h_0` 并限制在其面板内；边界命中带独立于内容，不被内容自身 clip 裁断。布局不自动创建 ScrollHandle；示例复用 YueScrollbar，滚动与分栏尺寸分别由原 owner 负责。

## 5. 交互、焦点及语义

| 输入 | 结果 |
| --- | --- |
| 左键拖动边界 | 即时调整相邻面板，结束后一次提交 |
| 双击边界／Enter | 将前一面板恢复为其默认尺寸；无默认时恢复相邻两块均分，仍受约束 |
| Tab／Shift+Tab | 在内容及可用分隔器间正常移动 |
| 左／右（左右分栏），上／下（上下分栏） | 前一面板减少／增加一步 |
| Shift＋对应方向键 | 调整较大一步 |
| Home／End | 移至当前相邻约束允许的最小／最大位置 |
| 禁用 | 不接受拖动／键盘／双击，不进入 Tab；内容本身仍可使用 |

分隔器使用 AccessKit Splitter 角色、调用方提供的面板名称、边界方向及当前尺寸／有效范围。水平调整的分隔线方向为 Vertical，纵向调整的分隔线方向为 Horizontal。支持 Increment／Decrement 可访问操作；不新增 Windows COM Provider。原生 UIA 暴露程度沿用当前 GPUI 桥接能力，不将快照断言当成 Narrator 验收。

## 6. Token 与状态矩阵

| 状态／属性 | token | 默认意图 |
| --- | --- | --- |
| 线宽 | `--resizable-line-width` | 1px，按 rem 缩放 |
| 命中带 | `--resizable-hit-width` | 9px，透明命中范围 |
| 默认最小尺寸 | `--resizable-panel-min-size` | 100px，调用方可覆盖 |
| 键盘步长／大步长 | `--resizable-keyboard-step / -large` | 8／32px |
| 静止／悬停／活动／禁用线色 | `--resizable-line-color{,-hover,-active,-disabled}` | 中性色边界；活动与焦点可辨认 |
| 焦点 | `--resizable-outline-color / --resizable-outline-width` | 位于命中带内部的可见边界 |

浅／深、Azure／Neutral、不同 NeutralTone 均解析组件 token；局部 theme 仅影响本组。无装饰位移动画，无持续空转帧，减少动效设置下保持即时调整。loading、selected、invalid 不适用于布局分隔器。Windows 系统高对比度与实机 DPI 需独立验收。

## 7. Showcase 与验收

示例包含：左右基础分栏、上下分栏、Tree 导航＋工作区＋属性的嵌套布局、固定侧栏、禁用／启用及恢复默认。每个面板有真实可操作内容；大内容复用 YueScrollbar。

验收以实际布局和输入验证：双向 min／max、另一边界不动、容器缩放及固定宽度、复位／程序恢复、方向键及 Home／End、焦点可见、双击只提交一次、禁用内容仍可用、嵌套互不干扰、稳定身份、rem／主题、静止无持续帧。真实窗口视觉由用户验收，自动化结果与限制记录在实施报告。
