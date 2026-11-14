# YueAlert 警告提示

## 1. 目标与参考

在当前页面流中展示需要注意的状态、原因和恢复操作；不遮挡窗口、不接管焦点、默认常驻。参考用户提供的 TDesign、Arco、Ant Design 三张截图，以及 `refer/Alert 警告提示 · GPUI.html`。本需求经用户确认是 YueAlert，YueDrawer 不改为另一种组件。

|参考|吸收内容|
|---|---|
|TDesign|四语义、操作、标题/描述、描述折叠、图标/文字关闭|
|Arco|图标开关、标题、公告、独立操作位、自定义关闭、关闭完成回调|
|Ant Design|四语义、描边、辅助描述、自定义图标、长文案、平滑关闭|
|本地 HTML|柔和/描边/实心、两档密度、收起高度和淡出、倒计时进度及阅读暂停|

公告完整换行，不加入自动跑马灯：Windows 桌面提示应可直接阅读与键盘操作，不能为了复刻网页把重要文字滚出视野。自动关闭显式启用且仅适用于低优先级结果反馈；警告和错误推荐常驻。无全局消息队列、遮罩、输入法或网络依赖；瞬态浮层继续由 YueToaster 提供。

## 2. 架构与复用

YueAlert 是受控 RenderOnce：业务宿主持有 visible、expanded 及业务操作，组件不创建平行业务模型。内部 keyed NoticeController 只保存当前挂载的计时、关闭完成去重、焦点边界和回调；卸载即释放，不保留业务实体。

- Base：新建无样式 NoticeController 管理可暂停阅读计时、关闭请求、代次/回调和焦点退出；不在 Base 放颜色/几何/默认动效。
- Base MotionDisclosure：复用自然高度测量、退出绘制快照、立即脱离子交互、连续反转、reduce_motion。补通用 on_settled 回调，不以固定 sleep 猜测动画结束。
- Yue：复用语义颜色、TextRole、Icon/YueIcon、YueButton、ControlLayout 和动效 token。
- 描述折叠使用现有 text shaping 检测实际行数、line_clamp；折叠按钮复用 YueButton。富文本/自定义内容通过 children，保留子组件自身语义。
- 不接入 GPUI Kit styled Component 层，不为静态提示创建窗口弹层，不修改 GPUI 平台源码。

## 3. API 和所有权

```rust,ignore
YueAlert::new("save-error", "项目未保存")
    .kind(AlertKind::Error)
    .description("连接已断开。输入已保留，恢复连接后可重试。")
    .visible(self.show_error)
    .closable(true)
    .on_close(cx.listener(|s, _: &AlertCloseReason, _, cx| {
        s.show_error = false;
        cx.notify();
    }))
    .action(YueButton::new("retry", "重试").on_click(...));
```

正式回调为 `Fn(AlertCloseReason, &mut Window, &mut App)`（值参数）；使用 cx.listener 时在闭包桥接引用。`on_closed` 为动画完成的通知，无伪造 ClickEvent。

|API|合同|
|---|---|
|new(id,message)|稳定挂载 ID；主文案必须非空；默认 Info、Tonal、Medium|
|kind|Info / Success / Warning / Error；图标和语义独立于配色|
|variant|Tonal 默认柔和 / Outlined 柔和底+边框 / Solid 实心|
|size|Small / Medium；字号、间距、图标同步变化，按钮保持现有可用命中区|
|description / children|纯文本说明可折叠；富内容由已有组件组合|
|show_icon / icon|默认显示；支持自定义同一 Tabler 字族；不改变语义|
|action|独立操作插槽；宽容器尾侧、窄容器移至正文下方；多个按钮由宿主组合|
|banner|无圆角，只保留底部分隔；不自动固定在窗口顶部|
|closable / close_text / close_icon / close_label|独立关闭按钮，Enter/Space 通过 YueButton；不支持把任意无语义 div 当关闭控件|
|visible(bool)|默认 true；请求关闭不自动修改宿主值，宿主可以不接受；须保留声明至退出结束|
|on_close|CloseButton / Timeout 请求；程序性 visible(false) 不伪造点击；超时每次显示代次只请求一次|
|on_closed|受控值变 false 且退出高度真正为零后，每代次一次；初始 false/直接卸载不发完成事件|
|max_lines / expanded / on_expanded_change|仅在描述实际超行时显示展开/收起；默认收起；宿主拥有 expanded；宽度、字体、rem 改变重测|
|expand_label / collapse_label|可自定义文案，按钮可访问状态反映 expanded|
|duration|默认 None；Some(非零 Duration) 开启可暂停自动关闭及底部剩余时间线；更改 duration 或 false→true 重置计时|
|focus_after_close|可选宿主焦点目的地；只有焦点原在提示内部时才处理；默认交给 Root 下一逻辑焦点|
|theme / use_default_theme / width 等|沿用 Yue 主题/控件布局合同|

同一 visible 代次中的超时请求若被宿主忽略，不无限重复请求。需要重新计时可隐藏/重新显示或更改 duration。点击关闭回调与操作回调互相独立；不监听 Escape，避免吞掉页面或 Drawer 的退出键。

## 4. 布局、样式和语义

外层 w_full/min_w_0，文本完整换行，允许长单词断行；图标和关闭位不缩小。单行图标在首行行高中对齐，带描述时图标对准标题首行。操作与关闭分列，窄容器操作下移；多行短内容不能把关闭控件挤出边框。父容器负责提示间距，关闭后不额外残留内边距。

Medium 复用 Body（约14/20）、Space8/12/16，基础高度随关闭按钮约40px；Small 复用 Caption（约12/16）、Space4/8，保留现有小号按钮命中区，约32px。带描述按内容自然增高。图标基础16、带描述20、小号14，均通过现有 token 的 rem 投影。边框和圆角复用现有细线与 Radius token。

Tonal/Outlined 使用 info/success/warning/error 的 bg/foreground/border；Solid 使用配对 solid/on_solid；Info 使用 accent_solid/on_accent。不将黑白当通用默认色，逐主题检查对比度。内部操作按钮保留自身 Button 状态；实心变体使用可读按钮表面，不能把默认深色 ghost 文案直接放在深底上。

信息/成功默认 Role::Status，警告/错误默认 Role::Alert；announcement 可选 Quiet/Polite/Assertive 以避免历史常驻提示反复播报。这表示原生 AccessKit 语义，不等于已验收 Windows Narrator 播报链路。关闭/操作/折叠均是独立命名按钮。静态提示不额外进入 Tab 顺序。

## 5. 生命周期、动效和性能

首次静态挂载不强制入场；visible true→false：立即脱离正文与按钮交互，MotionDisclosure 以已有绘制快照收起高度，整体 opacity 同步退出，完成后发 on_closed。若中途重开，保留当前高度反转，旧完成回调不能作用于新代次。reduce_motion、失活窗口按已有 Disclosure 规则直接收敛。

自动关闭倒计时仅在可见、窗口激活、未悬停且焦点不在提示内时前进；暂停原因分别维护，不能鼠标离开就覆盖键盘聚焦的暂停。卸载不会残留 Task。无自动关闭的常驻提示不持续请求帧；进度条只在活跃计时/动效时刷新，不对全部页面数据创建副本。折叠行数以文字/宽度/字体为签名缓存，不在每一帧无条件 shape 文本。

## 6. 全阶段任务与验收

|阶段|任务|门禁|
|---|---|---|
|A0|四参考分析、API、复用、桌面取舍与阶段设计|源码及文档核对、diff check、提交推送|
|A1|受控基础组件、语义/变体/密度、插槽、关闭动画和完成回调|编译、主题对比、几何、键鼠关闭、反转/去重、Disclosure 回归|
|A2|折叠/自动关闭/进度/阅读暂停、生命周期与焦点|超行重测、暂停交叠、重新计时、失活/离屏、卸载、关闭 focus 回归|
|A3|Showcase 独立示例、实际表单/详情消费、最小编译示例、完整文档|全量相关组件与 Showcase 测试、严格 Clippy、native build、提交推送|

Showcase 必须有：四状态、三变体、标题描述、图标开关与自定义、两密度、真实操作、图标/文字/自定义图标关闭、受控拒绝与恢复、描述折叠、顶部公告、长中文/长标识符和窄容器、可暂停倒计时、实际保存错误与重试。保留可回退 Git 阶段，不以截图或编译代替交互测试。原生观感由用户负责验收。

## 7. 交付记录

按每阶段实际结果追加，不预填通过。
