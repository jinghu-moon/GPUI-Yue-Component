# 25 YueImagePreview 图片预览组件设计

状态：核心首版及工具条增强已实现并接入 Showcase，真实 UI 待用户检查。更新日期：2026-11-03。

本文定义可嵌入面板的图片预览组件，覆盖单图查看、原图与处理结果对比、缩放平移、透明背景、状态反馈和连续动效。依据 SnapClip 的 `D:\100_Projects\110_Daily\SnapClip\docs\design\save-panel-demo-v3.html`、本仓库锁定的 GPUI/Base 源码及 GPUI Kit Coding Guides / Design Guides 编写。

本文同时记录设计合同和当前实现。公共类型已从 crate 根导出；代码位于 `crates/yue-component/src/image_preview/`。自动化结果及尚未验收的边界见第17节，设计意图不等同于真实 UI 验收。

关联：[组件架构](24-component-architecture.md)、[Button](02-button.md)、[Segmented](09-segmented.md)、[Tooltip](10-tooltip.md)、[Popup](12-popup.md)、[Dialog](17-dialog.md)、[Loading feedback](22-loading-feedback.md)。

## 1. 定位与范围

用户的主要任务是看清图片，以及判断处理后的图片是否符合预期。图片应获得主要空间，控件服务查看，不把预览区变成编辑器工具箱。

`YueImagePreview` 是一个组件：只有原图时显示单图；宿主提供处理结果后，可启用原图、对比、处理后三种模式。三种模式复用同一个视口、焦点作用域和视角状态，不创建三份彼此独立的查看器。

| 首版包含 | 宿主负责或暂不包含 |
| --- | --- |
| 静态图片查看、适应窗口、100%、缩放、平移；后台CPU准备辅助函数 | 文件选择、I/O、网络下载、任务调度、编码与压缩 |
| 原图与处理结果的左右滑动对比 | 质量、格式、输出尺寸等编码参数及体积估算 |
| 深浅棋盘格、纯白、纯黑背景 | 将透明像素填成白色等实际输出策略 |
| 加载、更新中、错误及重试意图 | 请求执行、取消策略、保存、复制、文件拖出 |
| 适应宽度、Windows像素网格、图像坐标叠加层 | 长截图识别、分割方案与分割后的文件 |
| 键盘操作、焦点、减少动效 | 裁剪、旋转、标注、取色、相册、视频与动图播放 |

不先新增通用 Canvas 平台或仅改名的 Image 包装。需要大窗口查看时，由宿主把同一预览能力组合进现有 YueDialog；预览核心不拥有模态开关、窗口或焦点陷阱。

## 2. 原型的吸收与修正

| 原型行为 | 本组件合同 |
| --- | --- |
| 原图／对比／编码后 | 保留三模式；通用默认文案使用“处理后”，SnapClip 可改为“编码后” |
| 双图共用 transform，按视口比例裁切 | 保留；进一步明确不同像素尺寸的坐标映射 |
| 鼠标位置缩放、左键平移 | 保留；补充边界、焦点和捕获取消规则 |
| 比例按钮回到100%，双击适应窗口 | 保留；明确100%与显示器 DPI 的关系 |
| 模式切换180ms、视角过渡260ms | 保留为组件级动效合同，经设计系统 token 提供 |
| 分割线双击居中、方向键微调 | 保留；限定在分割线自己的焦点作用域 |
| 每次尺寸更新／窗口变化重新 Fit | 修正为区分适应状态与手动查看状态 |
| 全局监听预览快捷键 | 修正为局部 Actions，不抢表单与 Dialog 的按键 |
| 模糊滤镜模拟压缩结果 | 不用于正式比较；必须显示真实处理结果 |
| 400%以上绘制像素网格 | Windows使用已验证的最近邻采样；只画可见参考像素边界，其他平台不可启用 |
| 长图分割线与“拖出图片” | 通过叠加层／操作槽组合，业务仍由 SnapClip 持有 |

原型可以说明交互意图，不能证明真实编码质量、最近邻采样、色彩管理、Windows 可访问性或 GPU 大图性能。

## 3. 结构与布局

```text
YueImagePreview
  ├─ ModeBar（可选）：原图 / 对比 / 处理后
  ├─ Viewport
  │    ├─ Background：固定在视口中的透明度检查背景
  │    ├─ ImageLayers：同一几何下的原图 / 处理结果
  │    ├─ ImageOverlay（可选）：跟随图像变换的非交互标记
  │    ├─ PixelGrid（Windows可选）：≥400%时跟随参考图像的像素网格
  │    ├─ CompareHandle：固定在视口中的分割线和握把
  │    ├─ Status：加载 / 正在更新 / 错误
  │    └─ ZoomToolbar：缩小 / 比例 / 放大 / 适应窗口 / 适应宽度 / 放大预览（可选）/ 更多
  └─ Footer：像素尺寸 / 宿主操作槽（或显式放置的工具条）
```

### 3.1 外部尺寸与内部布局

- 宿主提供有限的可用宽高，组件默认填满该区域；不从图片原始尺寸反推外层窗口大小，不默认制造页面滚动条。
- 建议完整模式最小外框为 `20rem × 16rem`，舒适起点为 `35rem × 29rem`；这是宿主布局建议，不是强行撑大父级的最小尺寸。剩余空间全部交给视口。
- 顶部、底部按控件与文字的自然高度排版，视口 `flex_1` 并释放 `min_w_0 / min_h_0`。放在 Flex 面板内不能由图片固有尺寸阻止收缩。
- 控件使用现有 Small 密度，不复制原型的超小按钮。缩放工具条的百分比使用等宽数字并预留完整范围所需宽度，避免数值变化推动按钮。
- 默认工具条在视口右下角持续可见，不仅在 hover 时出现。也支持放到视口外的底栏；布局紧张时宿主优先选择底栏，不把必要按钮裁掉。
- 浮动工具条绝对定位在视口右下角，不参与布局，不预留工具条安全区。Fit、图像居中、缩放锚点与平移约束使用完整视口及正常的四边对称 inset；工具条显隐或自身尺寸变化不改变视口几何、不调整相机，也不打断已有视角过渡。工具条可以覆盖局部图片，仅在自身命中区域拦截图片拖动／缩放输入。
- 底部尺寸文字优先简写，背景选项统一收进“更多”Dropdown；关键缩放命令始终有可见或键盘入口。小于建议尺寸时不强行挤成重叠控件。

兼容说明：源主题与生成快照暂保留 `--image-preview-toolbar-clearance`，图片预览几何不再读取该token；调整它不会改变浮动工具条的图像适应区域。正常的 `--image-preview-inset` 继续作为四边对称内边距使用。

### 3.1.1 缩放状态与拖动反馈

- 常驻缩小、倍率、放大、适应窗口、适应宽度和更多；宿主配置`on_expand`时另显示放大预览。FitAll时“适应窗口”显示持续描边，FitWidth时“适应宽度”显示持续描边，Manual且目标倍率为100%时倍率按钮显示持续描边。三者互斥，自动适应恰好100%仍表示相应适应策略。
- 描边不改变按钮尺寸；当前状态同时写入按钮说明与Tooltip，命令仍可重复执行，不将它改为再次点击就取消的开关。倍率文字继续显示过渡中的实时值，当前状态反映已提交的目标策略。
- 浮动工具条仅在图片Pan跨过拖动阈值后淡化至 `--image-preview-toolbar-opacity-dragged`（默认0.5）。普通点击、滚轮、分割线拖动和Footer工具条不触发淡化；只改变绘制透明度，不隐藏或移除命中区域。
- 松手或取消手势后恢复；鼠标进入工具条时恢复，离开且仍在拖图时重新淡化。工具条内任意按钮获得焦点或“更多”菜单保持打开时恢复完整透明度，组容器本身不增加Tab停靠点。
- 透明度复用Base连续transition，采用Interaction时长与Fade曲线（默认100ms），反向从当前采样值接续；`animated(false)`、系统减少动效、失活及不可见时直接采用静态目标。稳定状态和隐藏工具条不持续请求动画帧。

状态由现有图片预览State的fit／目标相机／Pan手势驱动；hover、焦点边界和菜单打开标记由工具条自己的keyed状态持有，不写入文档快照，也不产生图片视角事件。

### 3.1.2 更多操作与放大预览

- “更多”复用YueDropdownMenu／Popup，首选从右下角工具条向上、末端对齐，空间不足由已有定位规则处理。四种背景作为单选项；`show_background_picker(false)`仅隐藏这组菜单项，不再控制底栏Segmented，也不隐藏其余更多操作。
- “像素网格（≥400%）”为勾选项；Windows且当前显示的图片均为完整分辨率、目标倍率达到400%才可操作。低倍率或代理图不画网格，但保留已开启的查看偏好；满足条件后自动恢复。
- “平滑显示／像素显示”选择采样偏好。平滑显示使用完整画面的精确线性采样；像素显示使用最近邻。代理图统一平滑显示，避免把代理像素冒充原始细节；有效像素网格优先使用最近邻。
- 仅Compare模式提供“对比归中（50%）”，提交split=0.5，保持倍率、中心、模式与图像不变；不为该次命令额外增加位移动效。
- “放大预览”只表达宿主意图；没有`on_expand`就不渲染按钮。宿主使用现有Dialog和独立预览State，共享已解码资源并继承逻辑视角，详见12.2。Showcase已接入，页面外不再重复适应宽度／100%／大图查看按钮。

### 3.2 表面与图像边缘

视口作为一个有边界的容器，使用主题背景、边框与圆角；图像、棋盘格及叠加层都遵守视口裁切。焦点框属于外层，不能一起被 clip 裁掉。

图像按真实矩形显示，不默认给图片加圆角、阴影或改变透明像素的滤镜。原图自身的透明边缘必须保留。图片以外的棋盘格不随图像平移或缩放，避免误认为它是图片数据。

控件层在图像层之上，命中区域、可见区域和辅助技术节点各自只有一个所有者。非交互标签和叠加层不截获鼠标。

浮动工具条与对比分割握把使用同窗背景的真实高斯玻璃：sigma 为 `--image-preview-glass-blur`（默认设计基准16px，随rem／DPI换算），深色76%底色、浅色图标及弱描边。模糊由 [GPUI基础层](26-backdrop-blur.md) 提供，组件只选择材质token；不得缓存一份模糊源图冒充背景采样。工具条只设置一次滤镜，按钮不单独模糊；Footer工具条仍使用普通主题表面。材质不改变焦点与命中规则，也不要求为浮动工具条预留图像空间。

材质token覆盖：`glass-blur`（长度）、`glass-background`、`glass-border`、`glass-color`、`glass-shadow`；原`grip-background`和`grip-border`别名指向同一材质。两种主题／两种accent／四种neutral使用稳定深色材质，浮动按钮用对应neutral的Dark主题保留hover／pressed／disabled／focus；最亮背景上的普通前景对比度须≥4.5。禁用保留现有语义与提示，减少动效不改变静态材质，基础层不会自行请求帧。Windows系统高对比度实窗读屏仍由用户验收。

## 4. 状态与所有权

### 4.1 唯一所有者

| 事实 | 所有者 |
| --- | --- |
| 文档身份、原始文件、编码参数、请求世代、最新结果／错误 | 宿主 |
| 已提交给预览的不可变图像快照、视角、模式、分割比例、背景选择、FocusHandle | `Entity<YueImagePreviewState>` |
| 当前过渡采样、活动手势、指针捕获、测量缓存 | 该 State 或其窄内部视口状态，不另建业务状态 |
| 布局与绘制配置、外框尺寸、槽 renderer | 本帧 `YueImagePreview` builder |
| 图像纹理缓存、事件分发、绘制与窗口尺度 | GPUI |

宿主创建并持有 State，`YueImagePreview` 是接收该 Entity 的 RenderOnce 组合。不得在 render 中反复创建 State，不能同时提供 `mode` 受控值与 `state.mode` 两个未定义优先级的来源。

图像快照由宿主在事件／异步完成时提交给 State；render 只读取。快照保存资源引用，不复制整张像素数据。组件不启动业务编码，也不向业务层回写一份竞争性的编码参数。

### 4.2 身份与版本

- `document_id` 表示当前查看对象；同一路径内容被替换时也可由宿主赋予新身份，不把路径当作唯一判断依据。
- `revision` 表示同一对象的处理请求版本；质量、格式、输出尺寸变化可以增加 revision，不改变 document_id。
- 宿主在提交结果前验证 document_id 和目标 revision；晚到的旧结果必须丢弃。快照区分目标 revision 与当前可见结果 revision，以准确表达“上一版”。
- 组件提交入口也拒绝同一文档中倒退的目标 revision，作为防御校验，不代替宿主任务所有权。
- 新文档重置视角、分割线与活动手势；同文档的新 revision 保持视角、用户模式、背景与最近一次中间分割位置。
- 背景选择属于此 State 的查看偏好，换图保留；不使用跨窗口全局状态悄悄同步所有预览。

### 4.3 显示状态

| 状态 | 展示与可用操作 |
| --- | --- |
| Empty | 稳定空区域和“暂无图片”；禁用缩放、对比，不显示虚假尺寸 |
| 原图 Loading | 保留视口尺寸；按现有反馈组件的延迟策略显示加载状态 |
| 原图 Ready | 图片可查看，首次应用初始适应策略 |
| 原图 Failed | 错误说明和一个重试按钮；按钮只发 RetryRequested，宿主须订阅并执行 |
| 处理结果首次 Loading | 原图仍可操作；结果／对比入口暂不可用，显示“正在生成预览” |
| 同图结果 Updating | 保留上一版可见结果与视角，明确显示“正在更新 · 当前为上一版” |
| 处理结果 Ready | 原子替换资源和尺寸，不重新入场、不默认 Fit |
| 结果 Failed，有上一版 | 继续显示上一版，明确显示“更新失败 · 当前为上一版”，允许重试 |
| 结果 Failed，无上一版 | 回到原图可用状态，说明结果失败；结果／对比不可用 |

失败、空状态与更新中不能仅靠颜色区分。不用全视口透明蒙层阻止本来仍可进行的缩放和平移。

新文档加载时不保留旧文档图片冒充当前内容。若上一版结果与当前文档不再具有相同的完整画面对应关系，不能继续作为有效对比，改为原图并说明原因。

## 5. 坐标、缩放与100%

### 5.1 三种单位

1. 图像像素：解码后方向已归一化的栅格尺寸。
2. GPUI 逻辑坐标：布局、输入与视口几何所用单位。
3. 屏幕物理像素：显示器实际设备像素，逻辑坐标乘窗口 `scale_factor` 得到。

UI 字号、控件与间距使用 rem；图像尺寸和采样几何不随 rem 成倍放大。不得把“图片缩放”实现为修改窗口 rem。

**100%定义：当前参考图像的一个像素，对应一个屏幕物理像素。** 150%显示缩放的屏幕上，100%图像的一个像素占 `1 / 1.5` 个逻辑单位。尺寸文字始终表达图像真实像素，不是视口逻辑宽高。

100%按方向归一化后的原始尺寸计算，不按代理纹理尺寸计算。代理图无法恢复丢失细节，必须显示“代理预览”，禁止把代理纹理网格称为原图像素网格。

Windows下静止的100%及整数倍视角将绘制原点对齐屏幕物理像素，最大校正半个物理像素；逻辑相机不变，拖动／动画中不逐帧吸附。默认平滑采样使用保留完整图像UV的`paint_image_precise`，避免先取整裁切源区域再映射；“像素显示”显式使用最近邻。比例、几何和WARP像素测试分别验收，不把无头几何当作真实窗口验收。

### 5.2 视角表示

保留一个归一化图像中心 `center=(u,v)` 和整张图片显示的物理宽度 `width`，对外倍率 `zoom=z=width/W` 根据当前参考图像推导。平移偏移由它们与实际视口推导，不再保存一份可独立变化的 pan 真值。物理宽度避免不同分辨率参考图切换时重复换算而漂移。

设当前参考尺寸 `C=(W,H)`，窗口尺度 `d`，视口安全区域中心 `V`，图像坐标 `q`，则逻辑坐标为：

```text
k = z / d
screen(q) = V + k × (q − center × C)
```

图像、网格、尺寸锚点和宿主图像标记全部使用该变换；绘制与 hit-test 使用同一份测量。分割线与工具条使用视口坐标，不跟着图片移动。

滚轮／按钮改变缩放时，先反算锚点对应的图像坐标，再求新 center，使这个图像点仍落在锚点上。倍率到达边界时不产生额外平移；执行边界约束确需移动时，以可见性约束为准。

### 5.3 比例范围与适应

- 手动缩放基准范围为5%–800%，按钮每档乘／除1.25；非有限倍率不进入状态。
- FitAll 使用安全可用区域计算完整显示比例，保留语义 inset；默认不把小图放大超过100%。
- 为长图／超大图完整适应，FitAll 可以低于5%；5%不是强迫图片溢出视口的下限。手动下限取 `min(5%, 当前 FitAll 比例)`，比例显示在1%以下保留有效小数，不能显示为0%。
- FitWidth 只适应安全区域宽度，初次定位在图像顶部；进入手动平移后不反复回顶。工具条提供常驻按钮，图片视口获得焦点时可按W执行。
- `FitAll / FitWidth / Manual` 是互斥视角策略。滚轮、按钮缩放和实际平移进入 Manual；点击适应窗口回到 FitAll。
- 没有有效宽高时延迟适应，不以0除、不写入 NaN，等待首次有效布局后应用一次。

### 5.4 平移边界

FitAll只决定初始适应与居中，不锁定平移；图片在适应比例、100%和手动缩放下均可左键按住拖动。超过2px手势阈值且位置实际改变后进入Manual，松手立即停止，双击图片或点击“适应窗口”恢复FitAll。

每轴至少保留 `min(显示图像长度, 安全区长度, max(2 × inset, 1逻辑像素))` 的可见重叠，防止图片完全拖出安全区域；很小的图片保持整张可见。留白区允许移动，所以 `center=(u,v)` 表示安全区中点映射到的归一化图像坐标，允许超出 `[0,1]`，不将其当作始终位于图片内部的点。默认不做无限画布、惯性甩动或弹性回弹。

拖拽、键盘平移与程序化命令都经过同一边界约束。只在布局、倍率或图像映射变化时重算边界，不能各自采用不同的补偿公式。

## 6. 单图、对比与尺寸变化

### 6.1 模式与参考图像

| 模式 | 内容 | 百分比参考 |
| --- | --- | --- |
| `Source` | 完整原图 | 原图像素尺寸 |
| `Compare` | 左原图、右处理结果 | 处理结果像素尺寸 |
| `Result` | 完整处理结果 | 处理结果像素尺寸 |

组件默认 Source；比较能力默认按是否提供对应配置启用。SnapClip 可在首次结果就绪时主动切到 Compare，此后不能在每次编码完成时夺走用户选择。

三模式在相同画面比例下占据同一个显示矩形。切换模式不自动 Fit；只因参考图像像素尺寸改变时，换算倍率以保持这个矩形和归一化观察中心。新的100%命令再按当前参考图像执行。换算超出合法倍率时先钳制倍率，再尽量保持观察中心；此时允许显示范围改变，不以越界倍率强行保持矩形。

例如原图1440×900，结果720×450：从原图100%切到对比，保留显示矩形后应显示200%；在对比中点击100%，才缩到720×450个物理像素。不能一边保持相同画面，一边把两种不同参考都标为100%。

### 6.2 对齐合同

- 首版比较要求两张图片表达同一幅完整画面、方向相同，允许等比例缩放及整数尺寸舍入；由宿主明确声明完整画面对应，不只靠宽高比相近推断。
- 比较时两张图映射到同一处理结果画布，原图按该范围重采样显示；这是视觉内容比较，不保证原图与输出的像素一一对应。
- 默认比较体现缩放、编码等处理的综合变化。需要单独判断压缩损失时，宿主应提供与输出同尺寸、采用同一重采样策略的基准图，并正确标注其含义，不能把GPU即时缩放当成编码器的精确基准。
- 不为两图分别使用 `contain`，不分别居中，也不独立响应缩放／拖动。
- 超出整数舍入误差的宽高比差异、裁剪、旋转或局部替换，首版返回“不支持对齐比较”，仍允许单图查看。不得静默非等比拉伸。
- 通用裁剪／仿射映射不是首版必备 API；有真实消费者后再增加明确映射合同。

仅在同一文档完整画面映射不变时，结果尺寸更新才保持观察区域。Manual 下按参考宽度变化换算倍率，先钳制到合法比例，再尽量保留归一化中心并应用平移边界。Fit 状态重算对应适应策略。

不兼容的两图仍可分别查看，但不播放滑动对比。Source与Result之间切换时直接替换并FitAll，不猜测裁剪区域的对应位置；这一降级须有可见说明。表格中的视角保持规则仅适用于已声明且通过校验的完整画面对应关系。

模式之间的尺寸换算、资源替换和视角状态在一次更新中提交，不能先闪出一帧新尺寸旧缩放。

### 6.3 分割线

- 比例 `split` 在0–1之间：0全为结果，1全为原图；0.5为左右各半。计算基于图片视口宽度，不基于图像显示宽度。
- 保存 `last_middle_split`，初值0.5。Source 向1移动，Result 向0移动，Compare 恢复最近一次非端点位置。
- 拖到端点仍保持 Compare 模式，允许从边缘重新拖回；端点不覆盖 last_middle_split，也不悄悄切换模式。
- 握把按安全区域钳制在视口内，即使分割线到边缘也有可操作目标。握把可短距离偏离线的位置，细线仍表示真实裁切位置。
- 端点吸附阈值为视口宽度的1.2%；这是归一化交互参数。拖动与键盘更新直接到位，按钮模式切换才播放过渡。
- 双击线或握把回到0.5；左右键每步2%，Shift每步10%，Home到0，End到1。
- 线条和握把有独立于图形大小的命中区；分割线手势优先于图片平移。圆形握把内绘制20px设计基准的Tabler `arrow-bar-both` 图标，随rem缩放，以主题主文字色保证可见性。两侧裁切共用同一个物理像素边界，不留下黑缝，也不以半透明叠加掩盖接缝。

## 7. 指针、键盘与焦点

### 7.1 指针

| 输入 | 结果 |
| --- | --- |
| 图片视口内滚轮 | 以指针位置为锚连续缩放；使用指数映射，归一化离散／高精度滚轮单位 |
| 左键拖动图片 | 平移；越过拖动阈值后进入活动手势 |
| 双击图片或空背景 | FitAll；不得同时触发分割线双击或工具条命令 |
| 点击缩放按钮 | 以安全视口中心为锚改变倍率 |
| 点击百分比 | 以安全视口中心为锚回到100% |
| 拖动分割线／握把 | 仅改变 split |
| 背景选择 | 立即改变检查背景，保持图片与视角 |

静止可拖区域显示 grab，拖动期间 grabbing；分割线为水平调整光标；按钮继续使用默认箭头。适应比例下同样允许拖动，不以图片完整可见为由锁定相机。

按下时获取真实 hitbox 的指针捕获。松开、取消捕获、窗口失焦、禁用、换文档或卸载都终止手势并清除抓手状态；指针离开视口不应提前丢失正在进行的拖动。

缩放工具条、模式栏、状态重试按钮与宿主操作槽先消费各自事件，不冒泡为图片操作。不用全局监听截获邻近表单的滚轮。已在合法视口上开始的滚轮缩放由视口消费，倍率到界限时也不突然滚动外层页面。

### 7.2 键盘

预览视口持有可见焦点框；点击图片可获得焦点，但加载完成不主动抢焦点。顶部、视口、分割握把、工具条、底部按视觉任务顺序可达；组合控件内部沿用现有导航合同。

| 焦点范围 | 按键 | 行为 |
| --- | --- | --- |
| 图片视口 | `+ / =`、`-` | 放大／缩小 |
| 图片视口 | `0`、`1` | FitAll／100% |
| 图片视口 | `W` | FitWidth，长图从顶部开始查看 |
| 图片视口 | 方向键 | 以统一 rem 步长平移；Shift加大步长 |
| 分割握把 | 左右、Shift+左右、Home、End | 调整 split，不平移图片 |
| 模式栏、按钮 | 对应组件的方向键／Space／Enter | 只执行自身命令 |
| 活动指针手势 | Escape | 终止手势，保留当前视角并消费本次 Escape |

没有活动手势时不吞 Escape，让现有 Popup／Dialog 的最上层关闭规则处理。焦点在模式栏或按钮上时，方向键和Space不能同时驱动视口。首版不增加依赖全局空格监听的抓手模式，左键拖动已经覆盖该操作。

禁用仅阻止预览内置交互，保留当前画面；不清空图像、不擅自禁用宿主业务槽。缺少原图时禁用无意义命令；到达比例边界时对应按钮 disabled，100%与Fit命令按实际是否可执行判断。

## 8. 动效合同

动效服务空间连续性，不掩饰图像质量变化。直接操纵必须跟手，命令式变换可以有短过渡。

| 场景 | 时长与曲线 | 变化内容 |
| --- | --- | --- |
| 首次图片就绪 | 无视角入场 | 直接呈现适应后的清晰图片，不从0缩放 |
| 滚轮、图片拖动、分割线拖动／键盘微调 | 无额外缓动 | 立即更新对应几何 |
| 按钮缩放、100%、FitAll、FitWidth | 260ms，ease-out cubic：`1−(1−t)^3` | 视角连续过渡；图片与图像标记同步 |
| 模式切换 | 180ms，`cubic-bezier(.22,.61,.36,1)` | 在固定显示矩形上平移裁切边界 |
| 背景切换 | 无图像过渡 | 立即显示准确检查背景 |
| 结果更新／更换文档 | 无图像混合 | 原子提交资源；按身份规则保持或重置视角 |
| hover／pressed／focus | 复用现有控件 motion | 不另造工具条动画体系 |

### 8.1 缩放与平移连续性

- 不分别播放缩放和位移的两个无关动画。一个视角通道统一采样，并推导整幅图像的几何。
- 锚点缩放在每个采样帧都保持锚点图像坐标不变；倍率可在对数空间插值，再推导 center，不能简单插值两份偏移导致锚点漂移。
- Fit 命令以同一进度协调倍率与中心；边界约束不得每帧来回钳制造成震动。
- 连续点击从当前显示采样继续追最新目标，不跳旧终点、不排队播放。连续同向倍率命令可基于最新目标累计，过渡起点仍是当前显示值。
- 滚轮或图片拖动立即接管视角通道。模式按钮在拖分割线时终止该手势，再从当前 split 继续；其他无关通道不必互相取消。
- 当前百分比反映正在显示的倍率，辅助技术数值按语义目标更新，不逐帧播报。

### 8.2 模式切换连续性

先按第6节换算参考倍率，确保图片显示矩形不动，再移动 split。不要同时执行一次自动 Fit。切换期间可拖分割线，手势从当前采样位置开始并进入 Compare。

Source／Result 逻辑选择立即生效；裁切仍可在短过渡中显示另一侧，这是明确的模式切换，不是处理结果冒充。分割握把只在 Compare 可交互；焦点原在握把时，程序化切出 Compare 必须先把焦点移到视口，不能留在消失节点上。

### 8.3 更新、减少动效与调度

处理结果默认不交叉淡入；混合两张图会短暂重影，妨碍判断压缩差异。更换资源不弹跳、不重播框架入场动画，更新标记与资源版本同时变化。

`animated(false)` 或系统减少动效时直接采用目标几何，保留全部功能与状态文案。图片本身没有循环动画；只有实际过渡／可见 Spinner 需要请求下一帧。隐藏或卸载后取消帧任务，恢复时不重播旧命令；隐藏期间已结束的过渡直接呈现最终状态。

## 9. 容器、DPI与视角保持

| 变化 | FitAll / FitWidth | Manual |
| --- | --- | --- |
| 容器尺寸／正常对称 inset 变化 | 重新计算适应几何，不重新入场 | 保留归一化观察中心与倍率，再做必要边界约束 |
| 同文档结果更新，像素尺寸不变 | 保持适应 | 保持全部视角 |
| 同文档等比例输出尺寸变化 | 重算适应 | 换算倍率，保持同一画面范围与中心 |
| 原图／结果模式变化 | 显示矩形不动，换算参考倍率 | 同左；不主动 Fit |
| 窗口跨屏，scale_factor变化 | 重算适应 | 保留物理像素倍率语义与图像中心 |
| rem变化 | 按实际视口和正常对称 inset 重算适应几何 | 保留图像视角，必要时钳制；不把 rem 乘进像素尺寸 |
| 浮动工具条显隐／自身尺寸变化 | 视口与适应倍率不变 | 倍率、中心及已有视角过渡不变 |
| document_id变化 | 使用初始策略 | 终止旧手势与过渡，使用初始策略 |

布局变更走几何更新，不在窗口连续 resize 时反复启动260ms过渡追着窗口跑。DPI改变后100%仍指一个图像像素对应一个物理像素，因此逻辑显示宽高会改变；不能为了保留逻辑尺寸悄悄改变100%的定义。

首次结果到达前，原图的初始适应只执行一次。结果就绪不重置用户已经放大的原图区域。

## 10. 背景、尺寸与叠加层

### 10.1 透明度检查

`ImagePreviewBackground::{CheckerDark, CheckerLight, White, Black}`，默认 CheckerDark，可配置初值。四种背景是检查选项，不受当前明暗主题自动替换；工具条和文字仍跟随主题。

棋盘格使用统一边长的正方形，以 `--image-preview-checker-size` 为首选密度，随实际视口尺寸重算整数行列和共同边长；边长与全局起点对齐物理像素，避免分数DPI下出现14×15等长方格。整片网格居中、避开外框圆角，各轴最多256格，不绘制裁短的末行／末列。

任意长宽比无法同时满足正方格、完全铺满和有意义的格子密度，因此余量作为窄实色留边，不分别拉伸两轴，也不用圆角裁出半格。网格只随视口／rem／DPI重算，不随图片平移或缩放。透明通道由图像真实数据提供，背景切换不能重新编码或修改导出内容。黑白检查色属于明确的数据检查表面token，不作为普通UI背景到处复用。

### 10.2 元数据与宿主槽

- 底栏优先显示“原始 W × H”；处理结果存在时可增加“输出 W × H”。倍率只在工具条显示一份。
- 结果更新时元数据注明当前显示的是哪一版，不把新参数尺寸贴到旧结果上。
- 尺寸锚点可选，必须避开工具条与握把；视口外不绘制不可见标签。
- Footer 操作槽可组合长图设置、拖出图片等宿主命令；组件不替这些命令决定业务可用性。
- ImageOverlay 接收只读变换与可见图像范围，绘制分割线／编号等标记；首版只支持非交互标记，不开放任意交互子树与视口争抢手势。
- 叠加层不得执行编码或在 render 时改业务状态；重要内容的文字说明和可访问语义由宿主提供，不能只画在 Canvas 上。

### 10.3 像素网格与最近邻采样

Windows已实现可选像素网格。查看偏好默认为关闭，从“更多”开启；实际显示要求过渡中当前参考倍率≥400%。缩小时网格与最近邻同步停止，偏好保留，重新放大后恢复。其他平台菜单项不可操作，普通图像显示保持现状，不宣称已提供最近邻能力。

开启时图像使用受维护基础层的`Window::paint_image_nearest`：共享已有atlas纹理，按完整图像矩形反算源texel，以Texture.Load读取；分数平移与局部裁切不先取整成子纹理。既有`paint_image`保持线性采样。无离线放大副本、重复上传或CPU读回，GPU实例结构不变。

网格表示**当前参考图像**的内部像素边界，跟随同一相机；Compare通常以结果图为参考。两图分辨率不同时，参考网格不表示原图像素一一对应，两侧虽均采用最近邻，仍各自读取各自真实源texel。棋盘格表示透明背景，固定在视口，两者不是同一坐标系统。

仅枚举图像与视口交集内的线段，每轴最多4096条，超预算不绘制网格，不按源图面积创建Element或分配列表。前景为1个物理像素的浅色线，叠2个物理像素的深色底线，通过两项专用token控制颜色；线中心对齐物理像素中心。图片分割线和握把位于网格之上，宿主ImageOverlay最后绘制。网格不改变原图数据、点击区域、缩放、Fit与平移几何，也不自行请求动画帧。

## 11. GPUI / Base复用与实现边界

本实现以仓库当前锁定源码为依据，公共API签名已通过编译和测试核对，真实绘制观感仍由用户检查。代码依据：[img.rs](../vendor/gpui-pre/src/elements/img.rs)、[assets.rs](../vendor/gpui-pre/src/assets.rs)、[window.rs](../vendor/gpui-pre/src/window.rs)与[Base导出](../vendor/gpui-base/src/lib.rs)。

| 已有能力 | 设计用途 | 限制 |
| --- | --- | --- |
| GPUI `ImageSource`、`img` | 通用资源显示可直接复用 | 不是现成的缩放／对比 controller |
| `Arc<RenderImage>`、`size(0)` | 共享已解码静态纹理，获得实际纹理尺寸 | 原始尺寸由资源合同单独记录；仅静态帧 |
| `Window::paint_image_precise / paint_image_nearest` | 双图同矩形、完整UV；平滑或最近邻 | 仅受维护Windows后端支持；普通`img`绘制路径不改 |
| `Window::with_content_mask` | 视口和两侧互补裁切 | 圆角外框、物理边界与接缝须用实测验证 |
| `capture_pointer / captured_hitbox` | 图片与分割线手势捕获 | 手势结束必须完整释放与清理 |
| Actions、FocusHandle、AccessKit | 键盘路径与原生语义 | 不将网页ARIA属性当成已有GPUI方法 |
| YueButton / YueSegmented / YueTooltip | 工具条、模式栏、快捷键说明 | 保留各自尺寸、焦点与事件合同 |
| YueSpinner / Loading反馈 | 真实加载、更新状态 | 不阻塞已有可用图像 |
| YueDialog / Popup | 宿主大图查看、背景菜单等组合 | 不再创建第二个窗口级host |

输入支持共享 RenderImage、RGBA8／BGRA8、带明确编码的 RGBA16F 和静态编码字节。CPU 辅助函数位于 `resource.rs`，宿主在后台调用，组件不拥有文件 I/O、网络或编码器。资源必须恰有一个有效帧且尺寸非空；编码入口只接受静态 PNG/JPEG/WebP/BMP，拒绝 APNG、动画 WebP 和其他格式。方向归一化、逐行 PNG 色彩解析及受限 ICC 转换已接入；不支持的 profile／HLG 返回错误，不再存在含混的 `Unmanaged` 成功状态。高精度与 Windows 输出的实现范围及待验收项见 [27](27-hdr-pipeline.md)。

Base目前没有已核实可直接复用的图片缩放／滑动对比组件。图片专属状态与几何先留在 `image_preview/` 内部，调用现有GPUI机制；不预建公共PanZoom框架。确实可复用的无样式行为若需下沉，应按组件架构规范另行评审。

图片几何与业务状态留在组件层；可复用绘制能力由受维护的GPUI补丁提供，包括已实现的背景模糊与本次新增Windows最近邻采样。最近邻补丁只增加采样选择、完整图像UV和快照回放记录，未把图片预览主题或业务写入Base。详见`vendor/gpui-pre/YUE-PATCH.md`及Windows补丁说明；新增平台支持仍须独立验证，不能把能力待验证写成“已支持”。

## 12. 已实现的公共合同

所有公共类型从 crate 根导出，字段默认私有，以构造器、builder和reader表达。可编译的最小创建示例见 `image_preview/mod.rs` 文档测试；完整宿主示例见 Showcase 页面。

| 类型 | 职责 |
| --- | --- |
| `YueImagePreview` | 接收State与视图配置，组合预览表面 |
| `YueImagePreviewState` | 由Entity保留的状态；文档快照、视角、模式、背景、焦点和命令 |
| `ImagePreviewDocument` | 文档身份、目标revision、原图／结果状态的不可变提交快照 |
| `ImagePreviewImage` | 共享资源、可访问描述、原始尺寸、实际纹理尺寸及色彩处理状态；验证代理完整画面尺寸合同 |
| `ImagePreviewPixels` | 可拥有或借用的 RGBA8/BGRA8/RGBA16F；有界解码、线性缩图、显式 SDR 派生，源像素与显示代理分离 |
| `ImagePreviewLimits` | 编码大小、解码像素／单边／字节及单纹理栅格预算 |
| `ImagePreviewColorSpace` | Srgb（编码 sRGB8）／LinearSrgb（相对 SDR 白的线性 16F）／ScRgb（绝对线性 16F，1=80 nit） |
| `ImagePreviewSampling` | Smooth默认／Nearest显式像素检查；代理图只平滑显示 |
| `ImagePreviewLoad` | Empty / Loading / Ready / Failed 的宿主提交状态 |
| `ImagePreviewMode` | Source / Compare / Result |
| `ImagePreviewFit` | FitAll / FitWidth / Manual |
| `ImagePreviewBackground` | 四种透明度检查背景 |
| `ImagePreviewToolbar` | Floating / Footer 两种工具条位置 |
| `ImagePreviewOverlay` | 视口、图片显示矩形与归一化点坐标的只读绘制合同 |
| `ImagePreviewChangeSource` | Pointer / Keyboard / Toolbar / Accessibility |
| `ImagePreviewError` | 非法图像、空身份、倒退版本、不可用模式及非法数值 |
| `ImagePreviewSnapshot` | 当前模式、参考尺寸、倍率、中心、split与资源版本的只读投影 |
| `ImagePreviewView` | 可克隆的文档与视角会话，保留上一版结果，供独立预览继承 |
| `ImagePreviewEvent` | 用户引起的ViewChanged / ModeChanged / SplitChanged / BackgroundChanged / PixelGridChanged / SamplingChanged，以及RetryRequested |

### 12.1 配置与命令

- State构造接收稳定id与必要上下文，宿主在初始化阶段创建一次。
- `YueImagePreviewState::new(id, window, cx)` 创建 Entity 的内部值，`with_initial_fit(...)` 可选 FitAll / FitWidth；同一 State 不同时挂载两个视口。
- `ImagePreviewImage::new(Arc<RenderImage>, description)` 返回 Result，验证单帧和非空尺寸；空白描述回退为“图片预览”。`ImagePreviewDocument::new(id, revision)` 配合 `with_source(...) / with_result(...) / full_frame(...)` 构建提交值。
- `set_document(...)`、`set_mode(...)`、`set_split(...)`、`set_background(...)`、`set_pixel_grid_enabled(...)`、`set_sampling(...)`、`fit_all(...)`、`fit_width(...)`、`actual_size(...)`、`zoom_at(...)`、`is_mode_available(...)`与`snapshot()`属于State合同；需要重绘的方法接收上下文并负责notify。`snapshot().is_pixel_grid_enabled()`读取查看偏好，不等同于此刻已达到倍率／平台／完整资源绘制条件。
- 程序化setter静默，不伪装成用户事件；非法输入返回明确错误且不部分应用。图片与参考几何先验证，再一次提交。
- 同值命令不重复发事件。内部派生尺寸、适应重算与每帧动画采样不发用户变更事件。
- 用户事件在逻辑状态提交后发出；同一输入造成的语义变更按固定顺序为Mode、View、Split、Background、PixelGrid、Sampling，各类型最多一次。事件携带来源与只读快照，不暴露可变controller。
- 拖动／滚轮事件可按每个输入帧合并；事件表示最新逻辑状态，动画显示采样可短暂落后。不为每个动画帧发事件，也不要求宿主回写事件才完成预览交互。
- 回调可能同步关闭或替换预览，回调后不得继续使用旧文档或手势假设。
- `RetryRequested` 只传失败对象与目标revision；宿主决定是否重试并提交新状态，组件不会自己调用业务任务。

初始适应策略由State构造配置，默认FitAll，仅在新文档首次有效布局时应用；重建Element不能再次重置视角。Element侧保留 `animated`、`disabled`、模式栏／工具条／背景选择可见性、工具条位置、文案、Footer和ImageOverlay renderer等呈现配置。默认隐藏某个控件不改变State中的事实，也不丢弃其命令能力。

Element builder 为 `new(&state)`、`theme(...)`、`animated(bool)`、`disabled(bool)`、`show_modes(bool)`、`show_toolbar(bool)`、`show_background_picker(bool)`、`toolbar(ImagePreviewToolbar)`、`on_expand(...)`、`result_label(...)`、`footer(...)` 与 `image_overlay(...)`。浮动工具条始终没有额外安全区预留，显隐不改变视口与相机；显式选择 Footer 时仍按底栏布局分配空间。

### 12.2 与Dialog组合

内嵌预览和Dialog中的预览不能同时挂载同一个State：视口尺寸、焦点和手势会相互覆盖。宿主保留独立State，使用`on_expand(Fn(&ImagePreviewView, &mut Window, &mut App))`接收会话，在目标State调用`restore_view(view, window, cx)`成功后打开现有YueDialog。也可程序化调用`capture_view()`取得会话；`ImagePreviewView::snapshot()`提供只读视角。

会话共享Arc图像资源，并保留文档身份／目标revision、原图与结果加载状态、上一版可见结果及其result_revision、full_frame对齐声明、模式、split／上次中间位置、背景和网格偏好。不能仅用snapshot重建最新Ready结果，也不能为了允许比较强制`full_frame(true)`。没有文档或有效参考图时restore返回错误且不部分应用。

Manual继承参考图物理倍率与归一化中心，按目标视口做必要可见性约束；FitAll／FitWidth使用目标视口重新适应。目标State保留自身焦点、实例id、布局、初始策略、观察订阅和禁用配置，不复制手势、捕获或动画；restore静默，不发用户事件。两份视图后续独立，宿主若要跟随异步新结果或关闭后回写，必须明确同步，默认不隐式联动。

Dialog开关、焦点陷阱及恢复仍由Dialog负责。Showcase中的大图不再次配置on_expand，避免递归弹窗；工具条无需直接依赖Dialog。

### 12.3 图片接入与准备

三条入口最终都进入同一资源和绘制路径：

```rust
// CPU工作应在宿主后台任务中执行。
let image = ImagePreviewImage::from_rgba8(width, height, rgba, "剪贴板图片")?;
let image = ImagePreviewImage::from_bgra8(width, height, bgra_slice, "原始截图")?;
let image = ImagePreviewImage::from_encoded(&bytes, &ImagePreviewLimits::default(), "本地图片")?;
```

8 位入口接受 Vec 或借用 slice，检查 `width × height × 4` 长度、非空与溢出；其方向归一化、sRGB、straight alpha 合同不变。RGBA 在拥有的缓冲中交换 R/B 后交 GPUI，BGRA 不重复转换。`from_rgba16f(width, height, row_stride, bytes, encoding)` 接受小端 RGBA16F，仅允许 LinearSrgb／ScRgb，检查完整声明 stride（包括最后一行 padding）、有限 RGB 与 `[0,1]` Alpha；合法负 RGB／高光不截断。紧密 owned 缓冲直接移动，有 padding 时紧密化一次。缩图使用关联 Alpha 的线性光面积滤波；FP16 代理保留线性值及源内容峰值，不从代理重新估计源峰值。默认纹理预算按实际 4／8 字节计算。

静态编码入口仅允许PNG、JPEG、WebP、BMP，拒绝APNG和动画WebP，不静默取动画第一帧；其他Cargo依赖启用更多image格式也不会扩大此合同。解码器创建前设置可用限制，分配像素前检查尺寸、像素数和缓冲大小。

需要自定义预算或保留解码像素时，调用 `ImagePreviewPixels::decode(&bytes, &limits)`、`from_png_reader(BufRead + Seek, &limits)` 或原始像素构造器。文件 reader 先检查实际文件长度，再逐行转换，不先读成一份 encoded Vec。`into_rgba8()` 返回 Result，仅允许无损的 8 位通道转换；FP16 必须明确调用 `into_sdr_rgba8()` 生成 SDR 派生结果，或者 `into_preview(&limits, description)` 保留高精度显示。删除了可任意重新贴标签的 `with_color_space`。`ImagePreviewImage::new(Arc<RenderImage>, description)` 读取资源真实格式，仍不复制、不重新解码。

默认预算：编码64MiB、6400万像素、原始单边32768、解码器分配限制512MiB；纹理单边8192、BGRA栅格64MiB。所有预算必须为正；纹理单边最多8192。Pin宿主另外保持2400万像素解码限制和既有总保留预算。`ImagePreviewLimits::texture_size(source_size)`可先计算目标；`texture_budget(edge, bytes)`可按宿主预算调整，例如允许8192边长、128MiB栅格以保留8K完整细节。

这些值不是进程／显存硬上限：解码器、转换临时量、原图、旧结果、新结果和GPUI atlas可能同时存活；atlas有最小尺寸，不能用栅格字节数声称实际VRAM占用。宿主仍负责并发数、总存活预算、取消与过期结果丢弃。文件路径／URL只属于宿主读取或下载阶段，核心没有全局loader或新建资源管理crate。

`pixel_size()`固定表示原始完整画面的像素尺寸，`texture_size()`表示当前可用细节，`is_proxy()`明确二者不同。`with_source_size(...)`供宿主自己准备代理图时声明原尺寸，检查非空、非放大和宽高比；不能从尺寸证明图片不是等比例裁剪，宿主必须保证完整画面。代理和完整资源同文档升级时保留Manual倍率和观察中心。

代理图仍可缩放和对比，但页脚标记其纹理和细节边界，禁用最近邻与像素网格。网格偏好保留，`snapshot().is_pixel_grid_available()`读取当前是否可实际启用（Windows／完整分辨率／≥400%）；`is_pixel_grid_enabled()`仍表示偏好。`set_sampling(...)`静默设置；菜单操作发一次SamplingChanged。开启有效网格时最近邻优先于平滑偏好；会话继承采样偏好。

EXIF 方向在报告尺寸前处理。PNG cICP 优先于 ICC；当前支持标准 sRGB 和 Rec.2020/PQ full-RGB，HLG／非法 matrix／不支持的定义明确报错。无 cICP 时支持经过验证的 RGB matrix/TRC ICC → 相对线性 Rec.709；保留负值和扩展色域，不把 ICC 的 luminance 标签当成 HDR。已知 PQ profile 走经尺度验证的 PQ 路线；未验证的 LUT／非 RGB／不完整曲线拒绝。无标签普通输入明确按 sRGB 解释。显示器最终输出另由窗口政策负责，不等同于完整任意 ICC／HDR 格式支持。

Windows精确绘制保留完整图像bounds和UV，以content mask裁切，不再先把可见部分取整成子纹理；平滑采样在tile内读取四个texel，按Alpha关联后插值再交现有混合，避免透明像素隐藏RGB染边和atlas邻图串色。静止整数倍率将绘制原点对齐物理像素，手势及动画不做逐帧吸附；逻辑相机保持不变。直接上传仅以Weak登记所有权，资源及冻结快照全部释放后，在后续draw中回收atlas项，避免连续结果版本累积；共享资源仍被任一消费者持有时不能回收。

## 13. Token与动效适配

唯一源为 `D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens`。已新增 `component-tokens/image-preview.css` 并登记公共入口、命名词表、package-only 合同、README和专项测试；`theme/generated.rs` 与inventory由同步工具生成，未手工修改。

| 语义 | 来源策略／基准 |
| --- | --- |
| 外框背景、边界、圆角、文字、focus | 优先现有语义角色与容器token |
| 工具条尺寸、图标、hover／pressed | 现有Small Button及工具表面token |
| 组内gap、安全inset、信息栏排版 | 复用spacing／typography；必要时增加image-preview语义别名 |
| 深／浅棋盘格与黑／白检查面 | 新增专用检查背景token，不混用状态色 |
| 线条、握把、热区、网格 | image-preview token；图形尺寸和命中尺寸分开 |
| 像素网格前景／底线 | `--image-preview-pixel-grid-color` / `--image-preview-pixel-grid-shadow-color`；1px／2px为物理绘图线宽，非控件布局尺寸 |
| 视角过渡 | `--image-preview-view-duration`：由250ms阶梯派生为260ms；曲线为专用ease-out cubic |
| 模式分割过渡 | `--image-preview-compare-duration`：由200ms阶梯派生为180ms；专用曲线`.22,.61,.36,1` |

保留原型180/260ms是为了延续已讨论的预览节奏，属于可追溯的组件级选择；在设计源定义派生值及曲线，不向共享 `MotionDuration` 塞入无关通用角色，不在Rust视图中写死毫秒数。

曲线值定义在设计系统 motion 原语 `--ease-out-cubic` 与 `--ease-out-smooth`，组件token引用它们。减少动效由公共时长阶梯归零，组件CSS不另写重复覆盖；GPUI侧同时服从 `cx.reduce_motion()`。

比例范围、每档1.25、吸附比例、坐标矩阵属于行为参数；图片真实尺寸、运行时测量与DPI换算属于数据／几何。它们与UI间距token分开。

## 14. 可访问性

- 预览区域有名称和图片描述，装饰图像层不重复生成两份无意义可访问图片；Source／Result／Compare的说明能辨认当前查看内容。
- 模式栏复用Segmented语义，暴露当前选择和不可用项；错误原因可见。
- 分割握把作为可聚焦的水平Slider，范围0–100，值文字为“原图占比50%”等；视觉竖线并不代表垂直调整方向。
- 缩放按钮有明确名称；比例按钮名称说明“回到100%”，当前倍率通过描述／数值信息提供；不让每帧变化触发live播报。
- 加载／更新状态使用既有busy／Status能力，错误说明和重试按钮靠近对应对象。原图可用时不能把整个区域错误标成不可用。
- 键盘焦点可见、不被视口裁切；切换模式或资源不可用导致焦点目标消失时，回到视口或下一个有效控件。
- 当前源尺寸、输出尺寸与图像质量不靠Canvas文字作为唯一传达渠道。
- AccessKit节点与键盘路径可做自动化合同测试；Windows UIA／Narrator实际表现由用户验证，不能从节点存在直接声称读屏完全支持。

## 15. 性能与内部组织

运行前设置 `YUE_IMAGE_PREVIEW_TIMING=1` 可记录目录、缓存、排队、读取、解码、颜色转换、缩放、UI交接与首次绘制提交的分阶段耗时；默认关闭。阶段定义、50张真实图优化记录和无窗口重跑命令见[分阶段耗时与加载优化](performance/2026-11-07-image-preview-timing.md)。

多图胶片条使用普通滚轮或横向滚轮移动，不显示滚动条；缩略图没有独立边框。选中框固定在胶片条中央，缩略图从框下经过，最靠近框中心的图片自动成为当前主图。点击缩略图或通过其他入口翻页时，对应缩略图对准框中心；两端留出滚动空间，第一张与最后一张同样可居中。胶片条仍随原有自动显隐动画浮动展示。

实际目录为 `crates/yue-component/src/image_preview/`：`mod.rs`维护公共入口与局部Actions；`model.rs`定义值类型；`resource.rs`负责像素／静态编码准备和有界代理；`state.rs`持有状态、版本、手势和过渡；`geometry.rs`处理纯坐标；`element.rs / paint.rs`分别承担组合与绘制；内部资源／状态／几何测试及无头集成测试验证合同。没有新增独立crate或PanZoom基础框架。

- 缩放、平移、split更新只改变几何与裁切，不重新编码、重新解码或重新创建RenderImage。
- 两侧共享资源引用与图像缓存，局部更新不复制完整像素。结果替换后释放无用旧引用；更新失败保留上一版的内存成本需明确。
- 首版不承诺无限大位图。宿主根据解码内存与GPU限制准备可显示资源；超限应返回清楚错误，不崩溃或卡死UI线程。
- 缩略代理保留完整画面映射与原始尺寸，显式报告可用纹理尺寸；100%保持原始几何，不承诺代理中不存在的细节。完整资源就绪后由宿主提交同文档新revision，保留Manual视角。当前不实现按需tiles，测量及采用条件见[资源记录](performance/2026-11-03-image-preview-resources.md)。
- 视口只绘制可见内容；棋盘格和网格使用有界绘制，不为每格创建Element。不能为一次平移生成图像副本。
- 布局缓存包含文档／资源版本、视口尺寸、正常对称 inset、rem及scale_factor等失效维度；浮动工具条显隐和自身尺寸不作为相机几何失效维度。一次测量结果同时供绘制、输入反算和约束使用。
- 指针捕获、任务、观察订阅与动画通道有明确释放所有者；多个预览实例使用独立稳定id，不共享活动手势。

## 16. 验证与实施顺序

### 16.1 自动化合同

| 层级 | 必须覆盖 |
| --- | --- |
| 纯几何 | 坐标正反变换、鼠标锚点不漂移、倍率范围、超长图Fit、边界、DPI与100% |
| 对比 | 两图相同显示矩形、分辨率换算、比例／方向不兼容、split端点与last_middle_split |
| 状态 | 初始适应只执行一次、新文档重置、同revision幂等、旧revision拒绝、更新／失败保留上一版 |
| 动效采样 | 260/180ms终点、重定向不跳变、模式参考换算、直接手势接管、减少动效 |
| 无头输入 | 滚轮锚点、按钮、拖拽捕获与释放、双击隔离、键盘作用域、禁用、回调次数与顺序 |
| 集成 | 表单内不抢按键、Dialog组合、焦点目标移除、多实例隔离、主题与rem变更 |
| 原生语义 | 图片／区域名称、模式选中、Slider范围／值、按钮名称、busy和错误说明 |

测试通过真实组件事件路径证明交互，不能只调用内部数学方法就宣称拖拽或焦点已验证。

### 16.2 Showcase与人工检查

Showcase提供单图／对比、大图／小图／长图／透明图、不同输出分辨率、加载／失败／旧版更新、工具条位置、背景、减少动效与Dialog组合。预览区域为主要展示，不堆积大量无关按钮。

顶部四个开关均展示可见标签：**动效**控制视角与模式过渡；**禁用预览交互**禁用预览内操作；**浮动工具条**切换视口内悬浮与底部放置；**长图标记（25% / 50% / 75%）**切换三条相对图像高度的标记线。Switch构造器中的无障碍名称不代替可见文本。

真实UI由用户检查：动效观感、鼠标跟手、高DPI清晰度、双图接缝、透明边缘、浮动工具条覆盖效果、窗口缩放与Narrator。实现者负责自动化、编译和可复现的检查说明，不擅自启动实窗或宣称完成用户验收。

### 16.3 实施批次与完成定义

1. 核实图像绘制／裁切／捕获能力，完成纯坐标与状态规则；若有底层限制先记录，不顺手扩展patch。
2. 实现单图视口、适应／100%、缩放平移、背景、键盘与加载失败状态。
3. 增加双图映射、split、模式切换、异步版本交接及连续动效。
4. 同步源token、接入Showcase与集成测试，更新实际API和能力边界。

完成条件是核心查看和对比合同、状态交接、原生语义及连续动效均有对应实现与自动验证，并可交由用户实窗检查。Windows像素网格已作为工具条增强实现；其他后端最近邻、通用映射与编辑功能仍属于后续范围。

## 17. 实施与验证记录（2026-11-02）

### 17.1 交付与测试资源

- 已实现静态单图、三模式对比、适应／100%、锚点缩放、捕获平移、分割线拖动及键盘、四种背景、局部快捷键、错误／重试与上一版保留。
- 视角命令260ms、模式切换180ms；滚轮和手势直接接管；同值快照不打断手势或过渡；禁用、失活、隐藏、卸载时清理捕获与动效。图片绘制错误按仍有效的资源身份反馈，避免每帧重试与无限通知。
- 已接入 **组件 → 图片预览 ImagePreview**；首次打开该页才创建页面和开始解码。宿主后台解码并以generation丢弃晚到结果，大图Dialog使用独立State、共享资源。
- 两张用户图片从仓库根移动到 [Showcase测试资源目录](../examples/windows-showcase/assets/image-preview/README.md)：`desktop.png`（3840×2088）与 `long-screenshot.png`（3840×4998）。原文件字节未修改，移动前后SHA256一致，哈希见资源说明。
- 另有真实JPEG quality=70、1920×1044的JPEG结果及640×400透明PNG。半尺寸样本同时包含缩放和编码差异，不能当作单纯压缩损失。资源仅由Showcase／测试引用，组件库不捆绑测试图片。
- 本次未新增或修改GPUI/Base补丁；现有工作区补丁属于其他组件。

### 17.2 自动化结果

图片预览专项包含15项纯几何／动效测试和16项无头交互测试。后者包含真实按钮、滚轮、拖动与捕获释放、快捷键不抢Input、握把移除后的焦点、版本拒绝、上一版保留、全帧对齐变更及不同分辨率换算。分割握把已注册AccessKit Increment / Decrement / SetValue；实际Narrator操作未验收。

| 命令 | 实际结果 |
| --- | --- |
| `cargo test -p yue-component --locked --lib image_preview` | PASS，15项几何与动效采样 |
| `cargo test --workspace --locked` | PASS |
| `cargo test --workspace --locked --features headless-tests` | PASS，包含16项图片预览真实事件路径与新增文档测试 |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests` | FAIL，Select排版断言；见下节，不标成全量通过 |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests --test image_preview` | PASS，16项 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo clippy --workspace --all-targets --locked --features headless-tests -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo build -p windows-showcase --locked` | PASS，仅构建，未启动 |
| `node tools/sync-yue-theme.mjs --check` | PASS，离线快照一致 |
| `node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check` | PASS，与设计源一致 |
| 外部 `corepack pnpm exec vitest run tests/token-image-preview.test.mjs` | PASS，6项，包含精确登记、命名、四profile几何／动效与减少动效继承 |
| 外部 `corepack pnpm audit:tokens` / `corepack pnpm verify:all` | FAIL，详见下一节与完整日志 |

### 17.3 尚未通过或需要人工检查的边界

- 系统字体全量并行回归发现Select测试 `select::tests::layout::plain_options_start_at_the_row_padding_and_mixed_icons_keep_one_text_column` 失败：28px行内测得文字高度39px；同例独立复测通过，具体字体／并行触发条件尚未定位，不能将该配置全量标成PASS。图片预览16项无头交互在系统字体配置通过，未修改Select代码或测试断言。
- 外部设计系统 `corepack pnpm verify:all` 退出1：43个测试文件中37通过、6失败，820项中808通过、12失败。失败涉及API文档、架构计数、其他token的冻结命名／原型差异／使用报告，以及Collapse的动效字面值与减少动效重复声明。ImagePreview新增族的6项专项测试全部通过；全量不能标成通过。
- 外部 `audit:tokens` 仍退出1；本族命名无诊断，整个可达图的unreachable、undeclared、duplicates及layer violations均为0。没有为通过审计放宽词表或忽略未知token：仅逐项登记ImagePreview新合同。
- 全量失败日志保存在本工作区 `target/image-preview-system-font-tests.log`、`target/image-preview-design-system-verify-final.log` 与 `target/image-preview-token-audit-final.log`。
- 未启动Showcase实窗、未截图验收。动效观感、GPU纹理限制、透明圆角、双图接缝、高DPI清晰度及Narrator由用户检查；编译和无头测试不代表这些项目已经验收。
- 像素网格、可选最近邻采样、HDR／完整色彩管理、代理大图、编辑与动图播放未实现，仍按正文边界处理。

### 17.4 截图反馈修复（2026-11-02）

- 为四个演示开关增加可见说明；对比握把替换为明确尺寸与前景色的 `arrow-bar-both`。
- 棋盘格改为整数行列、统一边长、居中留边，并对齐物理像素。真实绘制primitive测试覆盖视口尺寸变化和100%／125%／150%／200%DPI，逐格检查等宽等高及content mask未裁切。此检查读取无头绘制数据，不是实窗像素验收。
- 先以真实左键事件复现FitAll无法拖动，再修正共用相机边界；回归覆盖原图／对比／结果三模式的直接拖动、split与倍率不变、松手释放及双击复位。
- 修复通用Tooltip测量与绘制字体不一致及临界宽度取整；先复现125%DPI、rem=24时“缩小（−）”多出一行，再验证短文案完整容纳、长文案正常换行。覆盖四种DPI和三档rem，内嵌字体与系统字体配置均通过。

本轮验证：组件库headless单元测试 **320项通过**；图片预览无头交互 **19项通过**（内嵌字体、系统字体分别执行）；Showcase headless **18项通过**。workspace Clippy含／不含headless-tests均通过（all-targets、`-D warnings`），Showcase构建通过。本轮所改Rust文件独立执行 `rustfmt --edition 2024 --check` 通过；收尾时workspace fmt检查在本轮未修改的 `src/select/tests/layout.rs` 第18行报告assert格式差异，未改动该文件，不将全局fmt标成通过。本轮未重跑外部设计系统全量验证，17.3节中的历史失败记录仍保留；未修改GPUI/Base补丁，未启动真实UI。

### 17.5 真高斯玻璃与性能测量（2026-11-02）

- 在受维护的GPUI core及Windows D3D11补丁实现可复用的 `Styled::backdrop_blur` 基础能力，浮动工具条整体和圆形对比握把各使用一个滤镜；Footer保留普通表面。不是预模糊图片或窗口材质，采样当前同窗背景；接口、资源预算、降级和绘制顺序见 [背景模糊基础层](26-backdrop-blur.md)。17.1／17.4节中“未修改GPUI补丁”仅描述此前批次，不适用于本批次。
- 源token增加中性glass语义角色及5个ImagePreview token，本族共27项；颜色混合和alpha通过源token及theme同步生成，未手工写generated文件。浮动按钮显式使用深色中性主题，前景与玻璃对比专项8.37:1通过。
- 增加默认关闭的CPU／GPU阶段采样、纹理／核复用计数和无窗口硬件基准；测量首次使用与稳定帧，另外关闭分阶段诊断做envelope对照，避免把采样成本当业务成本。RTX 4070 Ti SUPER、Release、4K离屏、3轮测量中，工具条＋握把稳定GPU P50在100%等效几何为21.728–22.592µs，200%为23.360–24.384µs。只表示局部滤镜，不是应用帧率。
- 数据提示优先控制独立玻璃区域数量；大面板两遍高斯占56.3%。稳定期纹理创建及核重建均为0。当前没有增加背景结果缓存的测量依据；真实交互卡顿需进一步测完整布局、paint、图像上传和Present。详见基础层文档第5／6节及其三轮原始记录。
- 本批workspace headless全量通过（组件库321、ImagePreview21、Showcase18，其他集成／doctest也通过）；core／snapshot7项、Windows离屏8项Debug／Release、硬件基准3轮通过。workspace及backend Clippy、fmt、Showcase构建和theme双向同步通过；本批全局fmt检查已通过，17.4的格式问题为历史记录。
- 外部设计源token专项6项通过，但最新全量仍为807通过／13失败、37文件通过／6失败；API／架构和原型冻结清单等仍有差异，新增glass角色也涉及原型清单待同步，不将全部失败归为无关问题。外部Vue浏览器visual门禁通过；该脚本生成浏览器基线图，不是GPUI实窗验收。没有启动GPUI实窗，真实UI仍交由用户检查。

### 17.6 浮动工具条不占用图像布局（2026-11-02）

- 按用户确认保留右下角悬浮，取消原本为工具条额外扣除的底部空间。Fit、居中、缩放与平移统一使用完整视口和正常的四边对称 inset；工具条自身仍拦截输入，并保留真高斯玻璃。显式 Footer 布局继续可选。
- 先通过工具条显隐回归复现适应倍率由59.5%变为70.5%的问题，再删除内部 `Viewport.bottom` 与浮动工具条对几何测量的依赖。新增几何测试验证对称内边距和真实视口中心；新增无头交互测试覆盖100%／150%／200%DPI下的显隐、FitAll倍率、视口尺寸、手动缩放和平移后的中心及无多余视角事件。
- 本批组件库headless单元测试322项通过，ImagePreview无头交互23项通过；非headless图片预览专项单元测试17项通过。组件库all-targets Clippy（含headless、`-D warnings`）、workspace fmt及本批diff检查通过。没有启动实窗，真实UI由用户检查；没有重跑外部设计系统全量验证。

### 17.7 工具条状态与拖图淡化（2026-11-02）

- 按用户确认实现3.1.1节合同。复用现有四个Small Button：FitAll与手动100%分别用Outline表示当前状态，其他按钮保持Ghost；命令说明与Tooltip同步当前状态。FitAll正好100%时也不会同时标记两个按钮；按钮宽度不随状态变化。
- 透明度由工具条私有keyed交互状态与Base transition管理，默认100ms线性往返、目标0.5。真实Pan超过阈值后淡化，捕获期间进入工具条可恢复，键盘焦点立即恢复。没有为淡化复制图片、新增滤镜、改变Fit几何或向宿主发出额外视角事件。
- 源主题新增 `--image-preview-toolbar-opacity-dragged`，精确登记package-only清单及冻结词表，ImagePreview族现为28项；inventory与Rust keys由同步脚本生成。该token是独立的暂时淡化策略，不复用禁用透明度。没有修改Vue组件、公共Rust API或GPUI／Base补丁。

本批证据：

| 验证 | 实际结果 |
| --- | --- |
| 组件库headless单元测试 | PASS，322项 |
| ImagePreview无头交互 | PASS，28项；新增5项覆盖当前状态、真实描边primitive、拖图／悬停／松手、焦点与分割拖动、连续反向、减少动效、禁用及稳定后停帧 |
| workspace all-targets Clippy，含headless与`-D warnings` | PASS |
| workspace fmt／本批diff检查 | PASS |
| theme离线与设计源一致性检查 | PASS，均输出`Yue GPUI theme mapping is current` |
| 外部token专项 | PASS，`6 passed (6)` |
| 外部`audit:tokens` | FAIL，exit 1；其他族冻结命名及原型清单仍有差异，本族命名专项无诊断 |
| 外部`verify:all` | FAIL，`13 failed / 807 passed`，6文件失败／37通过；与17.5记录的失败总数一致，未放宽断言掩盖失败 |

Rust单元测试记录：`target/image-preview-toolbar-lib-tests.log`；外部完整记录：`target/image-preview-toolbar-token-audit.log`、`target/image-preview-toolbar-design-system-verify.log`。外部全量在测试阶段终止，后续门禁不作为通过。真实GPUI窗口、观感与读屏未启动或验收，继续由用户检查。

### 17.8 工具条完整操作与像素网格（2026-11-03）

- 按用户“都实现”补齐五项：适应宽度常驻按钮及局部W快捷键、宿主放大预览、更多菜单中的四种背景、Windows ≥400%像素网格、Compare限定的50%归中。常驻按钮保持Small密度，背景不再占用底栏；浮动工具条仍不参与Fit和相机几何，菜单打开时保持完整透明度。
- 新增`ImagePreviewView`和capture／restore会话接口，以独立State组合现有Dialog。手动视角继承倍率／中心；自动适应按大视口重算；目标revision与上一版可见结果revision分开保留，不把旧结果伪装成新Ready、不改变full_frame声明。没有宿主回调时不显示放大按钮，Showcase大图不递归展开。
- 最近邻采样补入受维护的GPUI core／Windows后端，复用已有atlas与GPU实例pad标志；普通线性路径不变。三项生产shader的WARP测试覆盖整倍放大、分数平移与裁切、邻图不串色、圆角／比较内边、透明度和灰度；Debug运行时编译与Release内嵌FXC路径均通过。没有新增图像副本、读回或滤镜，未做新增性能收益声明。
- 网格沿当前参考图像边界绘制，只枚举可见线段、每轴最多4096条。先用无头primitive断言复现线段越过视口圆角，再按圆弧缩短边缘线段并向内对齐物理像素；已覆盖100%／125%／150%／200%DPI、1物理像素前景线、Compare内边不圆角和双图共用完整采样几何。颜色每次绘制只解析一次，圆弧计算只发生在靠边线段；稳定网格不驱动动画帧。
- 设计源增加`--image-preview-pixel-grid-color`及`--image-preview-pixel-grid-shadow-color`，精确登记冻结词表与package-only清单；ImagePreview族由28增至30项，主题与inventory通过同步工具生成。

| 验证 | 实际结果 |
| --- | --- |
| `cargo test --workspace --features headless-tests --locked --offline` | PASS；组件库324项、ImagePreview33项、Showcase18项，其他集成及doctest均通过 |
| 最终ImagePreview专项复测 | PASS，33项；包括四档DPI网格圆角、最近邻比较几何、菜单、局部快捷键、Dialog视角／旧版保留与焦点 |
| `tools/test-gpui-snapshot.ps1 -Offline` | PASS，5项core滤镜合同＋3项paint snapshot，含最近邻缓存捕获、裁切、atlas驱逐后的回放 |
| Windows `image_sampling_tests`，Debug／Release | PASS，各3项；完整命令见Windows补丁记录 |
| Windows既有backdrop correctness，Debug／Release | PASS，各10项，硬件计时基准1项按设计忽略，未重复性能测量 |
| workspace all-targets Clippy，headless与`-D warnings` | PASS |
| Windows backend lib／tests Clippy，`-D warnings` | PASS |
| 本批Rust格式、workspace `cargo fmt --all -- --check`、`git diff --check` | PASS；未格式化其他正在修改的文件 |
| `cargo rustc -p windows-showcase --bin windows-showcase --locked --offline -- -C extra-filename=-preview-tools` | PASS，独立文件名构建，未启动程序 |
| theme离线及设计源双向一致性检查 | PASS |
| 外部ImagePreview token专项 | PASS，6项 |
| 外部`audit:tokens`／`verify:all` | FAIL；审计架构unreachable／undeclared／duplicates／layer violations均为0，全量测试仍807通过／13失败、37文件通过／6失败 |

外部失败涉及API／架构文档、原型冻结清单、token语法／使用报告及既有Collapse动效合同；本次新增token已精确登记，仍不能把整个设计源标成通过。`verify:all`在测试阶段停止，后续浏览器门禁未运行。完整记录为`target/preview-tools-workspace-tests.log`、`target/preview-tools-interaction-tests.log`、`target/preview-tools-clippy.log`、`target/preview-tools-backend-clippy.log`、`target/preview-tools-token-audit.log`和`target/preview-tools-design-system-verify.log`。

验证过程中未启动Showcase、未使用真实窗口或截图检查、未结束用户进程。动效观感、工具条实际覆盖、像素网格清晰度与Narrator继续由用户验收；WARP及无头测试不代表这些项目已经通过。

### 17.9 统一图片接入、代理语义与精确采样（2026-11-03）

本节记录本轮高精度改造之前的 SDR 交付；其中“未转换色彩标记”和“ICC/HDR 不在本批”已由后续实现取代，不能作为当前 API。当前合同以上文 §11～12 和 [27 · Windows HDR 管线实施状态](27-hdr-pipeline.md) 为准，实机及 SnapClip 端到端验收仍未完成。

本批按图片接入／渲染建议逐项实施，没有新建loader crate或分块缓存框架：

| 项目 | 升级前 | 本批结果 |
| --- | --- | --- |
| 原始像素 | 消费方重复通道转换和RenderImage构造 | 公共RGBA／BGRA入口，支持移动Vec和借用slice，保留宿主原始像素 |
| 尺寸与100% | 资源只有纹理尺寸，缩图会改变倍率含义 | 原始尺寸与实际纹理分离，显式代理；升级完整图保留Manual视角 |
| 静态解码 | 消费方分别处理 | 公共有界PNG／JPEG／WebP／BMP解码，方向归一化、拒绝动画，未转换色彩标记 |
| Windows采样 | 普通线性路径会先取整可见子纹理 | 新精确线性路径保留完整UV；按需最近邻；静止整数倍率对齐物理像素 |
| 透明边缘 | 独立通道平均／普通线性可混入隐藏RGB | CPU面积和GPU双线性均按Alpha关联插值，atlas tile内隔离 |
| 连续资源替换 | 直接绘制上传缺乏随资源释放的回收 | Weak登记、最后资源／冻结快照释放后在后续draw移除旧atlas项 |
| 消费方 | 保存面板4096／2048阈值，结果固定2048；Pin私有解码 | 两者改用公共入口，保存面板两侧统一双预算；Pin保留原始复制及总量限制 |
| 大图后续选择 | 缺少本地成本依据 | 完成可重跑CPU探针；暂保有界代理，tiles和ICC/HDR引擎不在本批实现 |

修改前基线：组件库headless lib **324**、图片预览交互 **33**；SnapClip app **283 passed / 5 ignored**、pin **70 passed / 4 ignored**。日志分别为两仓库的`target/image-preview-input-baseline.log`。本批先通过新增断言复现完整几何从640变599、透明RGB染边及最后持有者释放后atlas不回收，再修改对应实现；失败证据保留为`target/image-preview-alignment-before.log`、`image-preview-alpha-before.log`、`image-preview-gpu-alpha-before.log`、`image-preview-snapshot-before.log`。

最终自动化结果：

| 验证 | 实际结果 |
| --- | --- |
| `cargo test --workspace --locked --offline` | PASS |
| `cargo test --workspace --features headless-tests --locked --offline` | PASS；lib **336**、ImagePreview **37**、Showcase **18**，其他集成及doctest全部通过 |
| workspace all-targets Clippy，普通／headless两配置，`-D warnings` | PASS |
| workspace `cargo fmt --all -- --check` | PASS |
| `tools/test-gpui-snapshot.ps1 -Offline` | PASS；backdrop合同5项、paint snapshot5项，包含共享所有权及20次替换回收 |
| Windows生产shader WARP，Debug／Release | PASS，各6项；覆盖运行时编译与内嵌FXC路径，不是实窗检查 |
| Windows backend lib／tests Clippy，`-D warnings` | PASS |
| SnapClip app＋pin feature lib | PASS，app **285 / 5 ignored**、pin **71 / 4 ignored** |
| SnapClip pin all-targets严格Clippy、app＋pin all-targets check | PASS；不表示整个应用的全量Clippy已干净 |
| Showcase／SnapClip应用／pin_preview构建 | PASS，使用`-C extra-filename=-image-resources`；仅构建未启动 |
| theme离线快照与设计源一致性 | PASS，双配置均为`Yue GPUI theme mapping is current` |
| 本批vendor Rust定向格式检查 | PASS，未格式化无关源码 |
| 两仓库`git diff --check` | PASS；未归一化既有LF／CRLF |

组件交互新增4项覆盖四档DPI的100%完整几何与物理对齐、代理升级、Compare仅原图为代理时的网格／采样限制、采样菜单的单次事件和资源复用。lib新增11项资源合同和1项几何测试；消费方新增保存面板长图与实际编码像素回归，以及Pin长图原始复制像素回归。既有业务导出和UI断言继续执行，未以降低断言制造通过。

最新日志：`target/image-preview-final-workspace.log`、`image-preview-final-headless.log`、`image-preview-final-clippy.log`、`image-preview-final-headless-clippy.log`、`image-preview-final-fmt.log`、`image-preview-final-snapshot.log`、`image-preview-final-warp-debug.log`、`image-preview-final-warp-release.log`、`image-preview-final-backend-clippy.log`；SnapClip侧为`target/image-preview-resources-final-tests.log`、`image-preview-resources-pin-clippy.log`和`image-preview-resources-check.log`。

本批实窗检查可使用Yue的`target/debug/deps/windows_showcase-image-resources.exe`，在组件目录打开图片预览页（含代理样本）；保存面板预览设置`SNAPCLIP_YUE_PREVIEW=1`后使用SnapClip的`target/debug/deps/snapclip_app-image-resources.exe`，可选`SNAPCLIP_YUE_IMAGE`指定本地图片；贴图示例为`target/debug/examples/pin_preview-image-resources.exe image`。检查普通图／透明边缘／代理、100%／125%／150%／200%DPI、拖动缩放和连续结果更新。构建日志为两仓库`target/image-preview-*-build.log`，这些路径不代表已启动或实窗验收。

CPU准备测量：4K约**4.147ms**、8K代理约**412.233ms**、1024×32768长图代理约**95.581ms**，均为Release三次中位数，不含文件读取、解码、GPU上传、实际显存或帧率。[完整环境、命令、字节与采用tiles条件](performance/2026-11-03-image-preview-resources.md)。若必须查看超长图的原始细节，应另行实现按需原分辨率区域／tiles；当前代理不承诺这些细节。

本轮真实窗口通道返回`Computer Use native pipe is unavailable ... (os error 2)`，没有窗口截图、物理DPI切换、Narrator或实际显卡帧率证据。没有启动／结束用户进程。外部设计系统全量、SnapClip已记录的两个workspace集成失败及Win32副作用ignored用例本批未重跑；不扩大自动化通过范围。未提交或推送。
