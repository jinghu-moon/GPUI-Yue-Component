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

外观复用语义表面、文字、边框、阴影、间距及排版 token；尺寸为具名语义预设。边缘滑入/滑出 240ms，曲线 (.2,0,.38,.9)，遮罩同一 Presence 进度；不逐帧重新创建实体或重新布局所有背景数据。中途反转连续；尊重 reduce_motion。面板本体保持不透明。嵌套时下层保留且失去交互，最上层处理 Escape，关闭后恢复父层焦点。

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

每阶段测试通过后独立提交、推送，保留可回退点。性能验证关注关闭时零面板渲染、只有活跃弹层参与绘制、不持有外部任务、调整尺寸不复制正文数据；不将 headless CPU 结果宣传为真机帧率。原生动效观感、真实 IME、Narrator 与设备 DPI 由用户真机验收。

## 8. 交付记录

待各阶段完成后填写实际测试命令与结果，不预先声称全部通过。

D0 已完成并提交；D1 已实现四方向/尺寸/插槽/共享会话/模态与非模态/正文滚动。`cargo test -p yue-component --features headless-tests --lib drawer` 7 项通过；`--lib dialog` 25 项通过（含一项 Drawer/Dialog 交叉测试）。头尾固定、窗口钳制、关闭手势、保护、异步、焦点及卸载有实际 headless 覆盖。
