# YueDrawer 抽屉

## 1. 目标和参考

在 Windows GPUI 应用中，从窗口边缘打开任务、设置或记录详情，保留上下文，支持键盘、表单、异步提交和嵌套。参考用户提供的 TDesign、Arco 两张截图及 `refer/gpui-drawer-demo.html`。

- 吸收四方向、320/480/640/全尺寸、独立滚动正文、固定头尾、关闭策略、自定义插槽、异步保存、未保存保护、加载和详情切换。
- 吸收本地 demo 的边缘滑入/滑出、遮罩渐变、可调整尺寸、嵌套返回。
- 桌面优先：可调整边缘同时提供键盘操作。底部抽屉沿用四方向模型，不增加触屏专属半屏吸附、抓手下拉关闭。
- DOM attach、CSS class、全局字符串插件及销毁 React 子树不直接移植。Rust 宿主拥有实体、任务和业务数据；关闭后保留声明，退场完成后释放渲染内容。

## 2. 分层和复用

`YueDrawer` 为 RenderOnce 外观；`YueDrawerState` 明确复用 `YueDialogState`，相关 Drawer 的关闭原因、事件、异步票据也重导出共享类型。不是第二套弹层状态机。Dialog/Drawer 可以通过 `open_child` 相互嵌套。

复用 Base Dialog 的输入意图、焦点陷阱和延迟绘制；复用 ModalScope、DialogLayer、PopupRuntime、Presence/Transition、Base resize_handle。复用 YueButton、TextRole、YueScrollbar、YueLoadingRegion、YueInput、YueDescriptions 等。禁止接入 styled Sheet、复制焦点陷阱或引入新的字体/图标依赖。

增加 Base 的通用 `modal` 开关，默认行为不变。模态抽屉统一参加现有窗口模态栈；非模态面板不占用模态层，不捕获背景鼠标和滚轮，不锁定焦点。已有模态打开时不能再从背景开启非模态面板；非模态面板内部仍可打开模态决策。

## 3. API

```rust,ignore
let state = cx.new(YueDrawerState::new); // 宿主持有，声明始终挂载
YueDrawer::new("details", theme, &state)
    .title("工单详情")
    .placement(DrawerPlacement::Right)
    .size(DrawerSize::Medium)
    .resizable(true)
    .confirm_label("保存")
    .on_confirm(|request, window, cx| {
        // request.ticket() 一次性完成；业务 Task 由宿主持有
        DrawerDecision::KeepOpen
    })
    .child(content);
```

- Placement：Left / Right（默认）/ Top / Bottom。侧边填满高度，顶部/底部填满宽度。
- Size：Small=20rem / Medium=30rem / Large=40rem / Full；`extent(DefiniteLength)` 自定义主轴尺寸，支持 rem、px、百分比。始终限制在可用区域内。
- title / label / description / header / header_visible / extra / footer / footer_content / closable。隐藏标题时仍须非空语义 label。正文为 ParentElement 插槽；自定义按钮由宿主接线。
- modal(true) 默认；overlay(false) **仅隐藏遮罩，仍模态**。modal(false) 明确允许背景操作；不绘制覆盖背景的透明拦截层。窗口级模态保证背景滚轮、指针和焦点隔离。
- close_on_escape / close_on_overlay_click / confirm_disabled / confirm_on_enter / initial_focus / on_close_request / on_confirm 均复用 Dialog 合同。默认 Enter 不隐式保存详情，显式开启才提交。确认中禁止重复提交、关闭和调整尺寸。
- resizable / resize_limits(min,max) / on_resize。拖动内侧边缘；方向键调整，Shift 加大步长、Home/End 到边界；双击复原。窗口变化只钳制有效尺寸，不丢失用户首选尺寸。
- 关闭策略分别控制，遮罩关闭只接受完整背景按下/抬起手势，不因拖动从面板移出而误关。
- 异步票据绑定窗口、会话和提交次数；卸载、重开、过期回调均不得关闭新会话。

## 4. 几何、外观与动效

紧贴所属边缘，不使用居中 Dialog 的外边距。标题及额外操作在上方；正文 min_h_0 / min_w_0、flex_1，独立滚动；底部固定。极小窗口允许整个内容壳滚动，使关闭和保存始终可达。正文复用 YueScrollbar，默认不为正文创建第二个垂直滚动所有者。

外观复用语义表面、文字、边框、阴影、间距及排版 token；尺寸为具名语义预设。边缘滑入/滑出 240ms，曲线 (.2,0,.38,.9)，遮罩同一 Presence 进度；动效代码不重建业务实体、复制背景数据或遍历业务列表；宿主仍遵循 GPUI 的帧重绘策略。中途反转连续；尊重 reduce_motion。面板本体保持不透明。嵌套时下层保留且失去交互，最上层处理 Escape，关闭后恢复父层焦点。

局部挂载和内容推开采用显式容器组合：`YueDrawerHost` 在自身有界区域内渲染非模态抽屉，可选 push 内容重排；窗口模态抽屉始终使用窗口层级。这样局部详情不会把窗口其他区域伪装成“局部模态”。局部宿主必须具备明确尺寸；窗口缩放后重新测量、钳制。

## 5. 生命周期与可访问性

open → OpenChanged(true) → Opened；关闭先 guard，接受后 OpenChanged(false)，退场后 Closed、归还焦点。保持挂载期间不能改变 modal 或容器归属；需要改变时关闭并完成退场。一个 state 只挂载一份声明。

语义 Role::Dialog、非空 label、description、pending busy；只在真正模态时锁焦点/隔离背景。非模态关闭仅当焦点仍由该面板拥有时才恢复，不能夺回用户已经移到背景的焦点。嵌套不得靠任意 z-index。自定义子弹层继续参加 PopupRuntime 的所有者路由。

## 6. Showcase

独立可发现示例：基础详情、四方向、尺寸、无底部/自定义头尾、透明遮罩与非模态对比、长正文、加载、表单未保存保护与异步成功/失败、Drawer/Drawer 与 Drawer/Dialog 嵌套、可调整尺寸、局部容器与 push、列表切换详情。所有示例有可操作按钮及状态说明；不能只提供一组配置开关。

## 7. 阶段和验收

|阶段|任务|验证|
|---|---|---|
|D0|设计、参考取舍、复用和 API 合同|核对现有源码及三份参考；文档差异检查|
|D1|共享会话扩展、四方向、尺寸、模态/非模态、插槽、滚动、动效|组件编译；Dialog 回归；Drawer 几何/关闭/焦点/异步测试|
|D2|边缘调整尺寸、键盘、局部宿主/push|四方向/边界/缩窗/局部几何与交互回归|
|D3|完整 Showcase、最小消费示例、说明和最终验收|headless 测试、严格 Clippy、native build、Git 提交/推送|

每阶段测试通过后独立提交、推送，保留可回退点。性能验证关注关闭时零面板渲染、仅可见弹层链参与布局/绘制、不持有外部任务、调整尺寸不复制正文数据；不将 headless CPU 结果宣传为真机帧率。原生动效观感、真实 IME、Narrator 与设备 DPI 由用户真机验收。

## 8. 交付记录

以下按阶段记录实现与验证；最终完整结果见交付验收记录。

D0 已完成并提交；D1 已实现四方向/尺寸/插槽/共享会话/模态与非模态/正文滚动。`cargo test -p yue-component --features headless-tests --lib drawer` 7 项通过；`--lib dialog` 25 项通过（含一项 Drawer/Dialog 交叉测试）。头尾固定、窗口钳制、关闭手势、保护、异步、焦点及卸载有实际 headless 覆盖。

D2 已完成：内侧边缘鼠标调整、超出面板继续拖动、方向键/Shift/Home/End、双击复原、限值与缩窗恢复、局部宿主四方向及 push。Drawer 测试增至 12 项全通过，Dialog 相关 25 项回归通过，`cargo clippy -p yue-component --lib -- -D warnings` 通过。动效中途反转、正文滚动固定头尾均有实际覆盖。

同一窗口一次容纳一条显式弹层链。多个声明可同时挂载；已有窗口模态时其他根级打开返回 Occupied，嵌套使用 open_child。非模态面板不会隐式改变窗口其他区域的焦点策略。

## 9. 消费示例与能力边界

`YueDrawerState`、`YueDrawerEvent`、`DrawerTicket` 等是现有 Dialog 类型的公开重导出；可直接订阅、持有 Task、调用 `open_child`，不需要把 Drawer 状态转换为另一种状态。调用 `open` 前须至少渲染一次声明；不要把声明放进 `if state.is_open()`。

```rust,ignore
// 长正文或昂贵的详情：关闭且退场完成后不执行此工厂。
YueDrawer::new("record", theme, &drawer)
    .title("工单详情")
    .footer(DrawerFooter::None)
    .content(move |window, cx| build_details(&record, window, cx));

// 有明确高度的工作区，局部非模态面板可覆盖或推开正文。
YueDrawerHost::new("workspace", YueDrawer::new("inspector", theme, &inspector)
    .title("属性").extent(rems(17.)).resizable(true)
    .footer(DrawerFooter::None).child(properties))
    .push(true)
    .w_full().h(rems(22.))
    .child(document);
```

- `.content` 在正文 children 后追加惰性内容，整个进入/退出期间保持渲染；工厂不缓存业务结果，不自动执行网络请求。大数据仍交给 YueVirtualList/YueTable，不能把数万行直接塞进普通滚动正文。
- `YueDrawerHost` 始终采用非模态。其尺寸必须明确；push 是正文重排，会增加布局成本，因此默认关闭，仅在局部工作区显式开启。全窗口默认 Drawer 不重排背景。
- 用户调整后的首选尺寸按挂载身份保留；传入新的 extent 或 placement 会重新采用声明值。关闭重开保留尺寸，卸载后清除。resize_limits 只约束可调整模式；可用窗口/容器小于最小值时，以实际空间优先。
- 非模态面板中的 Popup 使用窗口非模态表面的优先级下限，保持菜单在面板上方；不伪造 active_modal_scope。弹出真正的模态决策后，背景与非模态面板的瞬态菜单一起失效。
- 头尾通常固定；当**面板自身高度**小于最小 chrome 时，整个壳使用同一个滚动句柄降级，避免矮的 Top/Bottom 面板截掉操作区。非模态只吸收面板内部滚轮，背景滚轮继续工作。
- 自定义关闭图标可用 `closable(false)` + `extra(YueButton::icon(...).on_click(...state.close...))` 组合；保持按钮有语义名称。完整自定义头部时须同时给出 label。
- 动效的 240ms 和曲线为本地参考的具名共享 Drawer 策略；颜色、字体、间距、边框、阴影和 scrollbar 继续使用已有 token，没有伪造外部 token 库的来源记录。

参考能力落点：

|参考能力|Yue 实现|
|---|---|
|基础/操作/预览、四方向、宽高、全屏|YueDrawer + DrawerPlacement/Size/extent|
|标题/额外操作/底部/关闭键定制|插槽 + YueButton；不复制按钮行为|
|modal/mask/关闭与 Escape 策略|独立 modal/overlay/close_on_*；共同 guard|
|async OK、加载、未保存保护|共享 ticket + 宿主 Task + YueLoadingRegion/YueField|
|嵌套/焦点返回/生命周期|共享 DialogLayer/ModalScope/事件与会话|
|resize|Base resize_handle + 首选尺寸记忆 + 键盘|
|本地挂载/push|有界非模态 YueDrawerHost，不隐式全窗口隔离|
|destroyOnClose/mountOnEnter|声明保持挂载；退场后不渲染正文，业务实体生命周期归宿主|
|移动端抓手/半屏吸附|不纳入 Windows 桌面组件合同|

最小可编译消费程序：[drawer.rs](../examples/minimal/src/bin/drawer.rs)。Showcase 导航：**组件 → 抽屉 Drawer**。

## 10. 最终交付验收（2026-11-12）

D0–D3 的计划任务均已实现：20 项 Drawer 组件 headless 测试、5 项真实 Showcase 流程测试、14 个独立示例及最小消费程序。不会把单纯断言 API 存在当成视觉验收。

|检查|结果|
|---|---|
|`cargo test -p yue-component --features headless-tests --quiet`|1196 项通过，20 项既有专项忽略；包括 580 项库测试和集成/文档测试|
|`cargo test -p windows-showcase --features headless-tests --quiet`|83 项通过，10 项既有专项忽略|
|`cargo clippy -p yue-component -p windows-showcase --all-targets --features headless-tests -- -D warnings`|通过|
|`cargo check -p yue-minimal --bin drawer`|通过|
|`rustfmt --check`（本次新增/修改模块）、`git diff --check`|通过|

针对性测试覆盖四方向坐标/主轴与交叉轴、Full、缩窗、字号变化、百分比上限小于最小值、正文滚动和矮面板壳滚动、透明遮罩仍模态、非模态背景输入/焦点/滚轮、退出时只冻结面板命令、完整遮罩手势、关闭拦截、pending 与过期票据、显式嵌套链、非模态 Popup 层级、鼠标拖出原面板继续调整、四方向键盘限值、局部四方向/push，以及动效中途反转。

Showcase 流程使用生产组件：14 种配方依次打开/关闭，编辑后 veto 与放弃恢复、异步失败后成功重试、加载完成、同一会话切换详情、Drawer→Drawer→Dialog 的 Escape 逐层退出。关闭状态的惰性正文工厂连续 10 次强制刷新调用数不增加；这验证关闭时不构建正文，不代表 GPU 帧率测量。

全量测试另外发现历史 icon 语义清单漏掉已存在的 DragHandle：仅补齐测试基准，独立提交 `98d343c`，未改变图标资产。

原生观感交由用户验收：重新启动 `target/debug/windows-showcase.exe`，进入 **组件 → 抽屉 Drawer**，重点体验滑入滑出、拖动边缘、局部 Push 和键盘焦点。未声称验证真实屏幕阅读器、原生 IME 或多 DPI 设备。本次未新增性能百分比宣传，也未运行会干扰用户窗口的自动真机操作。

原生构建 `cargo build -p windows-showcase` 已通过（dev，54.83s）；未自动启动或结束任何用户应用。
