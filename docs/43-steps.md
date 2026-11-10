# YueSteps 步骤条设计

- 日期：2026-11-09；状态：已实现；验证与真机边界见[实施记录](research/2026-11-09-steps-implementation.md)。
- 平台：Windows GPUI；遵循[组件架构](24-component-architecture.md)。
- 参考：[Ant Design](assets/steps-ant-reference.png)、[Arco Design](assets/steps-arco-reference.png)、[TDesign](assets/steps-tdesign-reference.png)。

## 用途和参考吸收

步骤条说明一个有先后关系的任务处于何处，并可由宿主允许返回或跳转。业务校验、前进／后退按钮、网络、数据和页面内容由宿主持有。静态进度和可切换流程使用同一视觉合同；不创建另一个 Form 或 Tabs 系统。

| 参考 | 吸收 |
| --- | --- |
| 三份共同的编号、标题、说明、状态 | 三档尺寸；Waiting／Process／Finish／Error；当前节点与单项显式状态。完成使用勾，错误使用叉，当前保留编号；颜色之外保留形状和可访问状态。 |
| Ant 水平／垂直、小尺寸、自定义图标 | 标准布局和 Tabler 图标；辅助文字 subtitle 与 description 分开；不依赖系统 Unicode 特殊符号。 |
| Ant 点状、Arco 纵向点状 | Dot 类型保留完整标题／说明，标记缩小且连接线仍对准中心。 |
| Arco 文字在下方、导航式、箭头式 | label placement Right／Below；Default／Dot／Navigation／Arrow 外观。Navigation 和 Arrow 只支持水平、右侧文字，非法组合明确 panic。 |
| TDesign 逆序和额外内容 | reverse 仅反转视觉顺序，逻辑 key、步骤编号和状态计算不变；extra 为独立内容槽，不嵌套到步骤 Button，复用现有 Button／Input／Progress 等。 |
| Arco 步骤切换与错误、TDesign 显式状态 | 用户请求 key，宿主接受或拒绝；单项 status 优先于当前步骤的 status，再按逻辑位置派生。支持全等待与全部完成。 |
| 倒计时、任意点 renderer、百分比圆环 | 不增加常驻计时器或任意 renderer；倒计时、局部进度可组合 subtitle／extra 和 YueProgress。 |

## 现有能力和职责

锁定的 GPUI-KIT 0.8.0 有 `gpui_kit::component::stepper::{Stepper, StepperItem}`，源码在 registry 的 `gpui-component-0.8.0/src/stepper/`。它是带样式组件，不能作为本项目 Base-only 架构的依赖。当前 vendor `gpui-base/src/lib.rs` 没有 Stepper 导出，也没有 Steps 行为模型。

复用 Base Button 的鼠标／Enter／Space／禁用／焦点与语义，现有 Icon 和 Tabler 标记、Yue motion 的状态反馈、ControlLayout 的外部尺寸、YueScrollbar 的窄容器横向浏览；Showcase 复用 YueCard、YueField、YueForm、YueInput、YueSelect、YueButton、YueEmpty。没有引入新 Cargo 依赖或新增 GPUI 补丁的计划。

YueSteps 是 RenderOnce 受控组件；宿主传入 `StepsValue`，收到 `StepsChange` 后决定是否更新。内部 keyed runtime 只保留焦点、滚动、最新请求合同及配置身份，不保留第二份业务选中值。没有公开 StepsState，没有业务异步 controller。

## API 和状态合同

```rust,ignore
YueSteps::new("import-steps", "configure")
    .items([
        YueStepItem::new("source", "选择文件").description("选择导入来源"),
        YueStepItem::new("configure", "配置导入").subtitle("必填").description("检查名称与选项"),
        YueStepItem::new("result", "导入结果"),
    ])
    .size(StepsSize::Medium)
    .layout(StepsLayout::Horizontal)
    .variant(StepsVariant::Default)
    .label_placement(StepsLabelPlacement::Right)
    .status(StepsStatus::Process)
    .reverse(false)
    .readonly(false)
    .disabled(false)
    .accessibility_label("导入步骤")
    .on_change(|request, window, cx| { /* host accepts request.key() */ });
```

- `StepsValue::{Pending, Current(SharedString), Complete}`，字符串可转换为 Current。Pending 为全等待、Complete 为全完成；不使用越界 index 表示完成。
- `YueStepItem::new(key, title)`；key 和 title 不能空白，key 必须唯一。支持 description、subtitle、icon、status、disabled、extra。空列表合法且无步骤／连接线。Current key 不存在时不静默改成第一个，而是明确 panic，宿主原子更新 items 和 value。
- 状态优先级：item.status → 当前项的 root.status → 当前之前 Finish／之后 Waiting。Pending／Complete 参与派生，但显式 item.status 仍可表达异常。业务 current 身份和显式视觉状态分开，错误不会擅自前进。
- 有 on_change 且非 readonly 时可操作；disabled 阻止整组请求，item.disabled 阻止对应步骤。readonly 不把静态展示伪装成禁用按钮。
- disabled／readonly 约束步骤切换，extra 中控件的禁用、读写与命令仍由宿主显式配置。
- 同 current key、无 handler、禁用或只读不请求。有效请求一次，携带 key、逻辑 index 和 before 值。宿主拒绝时展示不变；外部 value 更新不触发回调。回调可同步移除组件，不在持有 runtime 借用时调用宿主。
- 当前页之前是否允许返回、未来步骤是否解锁由宿主设置 item.disabled 或拒绝请求。组件不推测业务权限。
- extra 总在步骤标题操作区域之外，内部按钮只执行自身命令；不会自动点击步骤。输入 Entity 保留在宿主，步骤条重新 render 不创建编辑器。

## 布局和连接线

默认水平等分可用宽度，单项保留 token 最小宽度；长标题自然换行，说明位于标题下方。窄容器保持步骤顺序并提供已有 YueScrollbar 横向浏览，不静默折行使步骤关系断裂，不根据全屏宽度自动改布局。宿主可显式切换 Vertical。Below 仅影响水平布局，垂直布局始终把文字放在标记右侧。

Right 标题与标记在同一行，水平连接线从标题后方延伸，始终对准标记中心高度；长标题换行不会下移标记。Below 的节点居中，左右各半段连接线连接相邻标记，文字在标记下方。垂直布局左侧使用固定宽度的标记列，连接线位于标记中心，下方内容和 extra 的自然高度决定线长；末项没有尾线。Dot 的连接线仍按相同的标记列中心定位，不借标题长度补位。

连接线宽度采用 `--steps-line-width`。已完成源节点连至下一节点用完成色，目标错误段用错误色，其余用中性线色；reverse 后仍按逻辑相邻关系计算，不把视觉右侧误当未来。标题长度、subtitle 和 extra 不能改变标记中心及目标锚点。Right 的可见线段起点随标题自然宽度变化，不用假定父表面颜色遮线。

Navigation 使用下划线强调当前步骤，并用现有 ChevronRight 分隔；Arrow 使用连续箭头表面，凹口／尖角按 token 几何绘制，文字和命中由真实 Button 拥有，装饰不接管鼠标。箭头背景仅为外观，不绘制额外伪交互元素。

Arrow 整块拥有对称的上下内边距（`--steps-gap`），description／extra 同样位于内边距之内。Below 标题宽度受所属步骤约束，在步骤内部居中换行，不跨入相邻步骤；横向溢出的其余步骤可通过滚动查看。

三档 marker、icon、字体和点击高度来自设计系统 `--steps-*`，外部 width／min／max／margin／grow 复用 ControlLayout。局部 theme 传给组合部件。视觉色彩不能硬编码，禁用之外的普通说明及状态文字对比度至少 4.5。

## 键盘、焦点和可访问性

根使用 List（带名称），步骤用 ListItem 表达顺序。可操作标题使用 Base Button；静态步骤由 ListItem 提供标题、位置与状态名称，没有额外 Tab stop。不会声明 TabList／RadioGroup，也不虚构 UIA Current。

Tab／Shift+Tab 遍历可操作步骤及 extra 的真实控件；Enter／Space 走 Base Button 的同一请求路径。当前状态包含在名称 `第 N/M 步，标题，进行中，当前步骤` 中，description 和 subtitle 在说明中。禁用步骤被跳过，readonly 不拦截 extra 中的独立控件。

焦点按 key 保留；重排不改变身份。聚焦步骤被移除、禁用或转为 readonly 时，只在该组实际拥有焦点时尝试移到可用当前步骤或最近可用步骤；没有目标则释放焦点，不抢占外部输入。水平滚动在当前值或焦点变更时揭示目标；组拥有焦点时优先揭示聚焦项。容器改宽与 rem 改变在布局后按实际尺寸重新揭示，稳定后不再请求帧。

颜色／背景／连接线切换沿用共享 Interaction 时长和标准曲线；几何稳定，不增加整行跳动、标题飞入或常驻动画。减少动效即时落地。静态和稳定状态不持续请求帧。

## 实施和验收

文件为 `steps/{mod,model,element,tests}.rs`。model 只放派生规则／请求数据；element 组合、焦点与几何；mod 定义公共 builder／导出；tests 验证真实交互和布局。设计系统新增 Steps token 源、名称／注册与四主题对比度测试，再由同步工具更新生成映射。

Showcase 增加“步骤条 Steps”：基础／三尺寸、水平／垂直、Right／Below、图标、Dot、Navigation、Arrow、错误／显式状态、逆序、readonly／disabled、长文本和窄容器、extra 独立命令。主示例为可校验和返回的导入流程，数据 Entity 留在宿主，失败可重试，完成可重新开始。

自动化覆盖纯派生与非法配置；鼠标／Enter／Space 一次请求、同值与宿主拒绝、禁用／只读、真实 extra、重排／移除焦点、空／单项／全部完成、横向滚动、连接线中心和末项边界、四主题 × 三尺寸 × rem 缩放、三种非默认外观及真实向导的校验／返回／失败／重试。运行受影响回归、Clippy、工作区检查、Rustdoc、Showcase 构建、token 同步与文档链接检查。Windows GPU 视觉／Narrator／系统 DPI／原生 IME 仍由用户真机验收。
