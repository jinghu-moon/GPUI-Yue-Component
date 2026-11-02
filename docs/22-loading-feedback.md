# 22 Progress / Spinner / Skeleton：统一加载反馈

状态：首版已实现，真实 UI 由用户检查。日期：2026-11-02。本文定义共同合同、三个组件的外观与动效、实现边界和验证记录。遵循 gpui-kit Coding Guides / Design Guides、当前 Base-only 依赖和 Yue 外部 token 同步规则。

## 1. 对标与取舍

2026-11-02 检索并阅读官方资料：

| 来源 | 核实的能力 / 原则 | 本项目采用 |
| --- | --- | --- |
| [Radix Progress](https://www.radix-ui.com/primitives/docs/components/progress) | 数值受控；indeterminate 不提供当前数值；进度语义与填充外观分开 | Base 统一数值规范化、名称、范围和未知进度语义；Yue 决定外观 |
| [MUI Progress](https://mui.com/material-ui/react-progress/) | 线形 / 环形、确定 / 不确定；确定进度提供真实值；高频更新要关注过渡滞后 | 两种形状共用值模型；真实目标立即进入语义树，绘制值可短过渡 |
| [MUI Skeleton](https://mui.com/material-ui/react-skeleton/) | Text / Circular / Rectangular / Rounded；Pulse / Wave / 无动画；占位不具备焦点 | 对应四形状和三动画，不把占位做成假按钮或假输入框 |
| [Ant Design Progress](https://ant.design/components/progress/) / [Spin](https://ant.design/components/spin/) / [Skeleton](https://ant.design/components/skeleton/) | 状态、尺寸、延迟 Spin、组合骨架 | 状态少而明确；Spinner 延迟显示；骨架由少量原语组合，不做大量业务模板 |
| [Fluent Progress bar](https://fluent2.microsoft.design/components/web/react/core/progressbar/usage) / [Spinner](https://fluent2.microsoft.design/components/web/react/core/spinner/usage) / [Skeleton](https://fluent2.microsoft.design/components/web/react/core/skeleton/usage) | 百分比已知才用确定进度；显示具体等待原因；骨架匹配真实布局、组内动画同步、避免无意义重复加载 | 遵循桌面工作流；保留上下文；同一应用的循环相位共用时间基准 |

不照搬网页 DOM / CSS、全屏蒙层、任意渐变主题、仪表盘缺口环、步骤条、分段成功条、估算剩余时间或伪造的自动进度。首版对齐核心能力与可见动效，允许宿主组合扩展。

## 2. 三者的选择

- **Progress**：真实任务的完成度。已知比例使用 0–100；不知道总量使用 indeterminate，禁止缓慢爬到 99% 假装真实进度。
- **Spinner**：局部操作正在等待、无法量化，通常放在命令或状态文本旁。默认短延迟，短任务可不显示；不强制最短显示时间，不拖延已完成结果。
- **Skeleton**：首次取得内容且能够预判布局时使用；匹配最终头像、标题、段落、图片的结构与尺寸。刷新已有数据优先保留内容并在局部显示状态。
- **LoadingRegion**：可选的内容替换和 busy / status 语义边界；不是遮罩、焦点陷阱或异步任务管理器。

三者都不发起请求、估算百分比、执行取消 / 重试或自动发送 Toast。取消、重试、失败说明由宿主使用 Button / 文本组合，不能塞进 Spinner。

## 3. 统一架构

```text
宿主：真实任务状态、进度、请求世代、结果 / 错误、内容 Entity
  ├─ ProgressValue / active / loading：受控输入
  ↓
Base：ProgressValue；Progress 原生语义；LoadingDelay；共享循环时间基准
      LoadingRegion 活动子树边界；busy / live 原生适配
  ↓
Yue：FeedbackSize；线 / 环 / 弧 / 骨架绘制；语义 token；短过渡策略
  ↓
GPUI：布局、canvas、内容裁切、单调时钟、Task、reduced motion、AccessKit
```

不新增 LoadingManager、通用业务状态机或另一个全局 Root。组件均为 RenderOnce；只有微小时间状态使用 keyed Entity。循环不为每个占位生成独立 timer；绘制时检查可见内容裁切，只在确实需要连续动画时请求窗口下一帧。

### 3.1 共享值与语义

`ProgressValue` 私有存储 `Option<f32>`：

- `new(percent)`：有限值钳制到 0–100；NaN / ±Infinity 转为未知。
- `indeterminate()`：未知进度；`percentage()` 返回 None。
- 0、100 是合法真实值，不自动隐藏、不触发完成回调。
- 同一个值用于 Linear / Circle；未知值不输出 `numeric_value`，更不能输出 NaN。
- Base 旧 `Progress::value(f32)` / `indeterminate(bool)` 保留兼容，并通过同一规范化规则投影。

`FeedbackSize::{Small, Medium, Large}` 共用于三组件，语义上是密度档位，实际轨道厚度、环直径、文字骨架高度分别读取各自 token。

### 3.2 生命周期

宿主在事件 / 更新中提交状态并 notify。外部更新不回调“纠正值”，不自动认定成功。相同 id 的过渡从当前绘制值继续；id 改变代表新任务身份。

Spinner 的延迟是呈现门限，起点为 active 从 false 变为 true。关闭、卸载或切换延迟配置取消旧唤醒；timer 捕获弱 Entity，不能复活已经完成的任务。延迟阶段不逐帧等待。真实完成立即移除，不为动画强行等待 300ms。

窗口隐藏 / 失活、元素被视口完全裁切、animated=false、reduced motion 时停止循环帧请求；恢复后按共享时间基准继续，不重新播放整组入场。为非活动 Tabs / 隐藏页面应卸载反馈元素。

## 4. Progress

### 4.1 组成

```text
YueProgress（唯一 ProgressIndicator 语义）
  ├─ 可选 label + 真实状态 / 数值
  ├─ Linear：Track + 有界 Fill / 不确定移动段
  └─ Circle：完整背景环 + 从 12 点起顺时针的进度弧 + 可选中心文本
```

Linear 默认占用可用宽度，header 释放 min-width，长标签允许换行；百分比 / 状态是独立尾侧槽。Circle 使用正方形直径，不能被 flex 压成椭圆，文字不能与环重叠。装饰环和填充不生成第二份可访问节点。

轨道和填充的圆角属于同一几何：0 不画伪造最小进度，极小值不超过真实长度，100 恰好填满。环形 0 不画圆点，100 闭合，端点使用 round cap，stroke 保持在实际 bounds 内。

### 4.2 状态

`ProgressStatus::{Normal, Success, Error, Paused}`，默认 Normal。

- Success / Error / Paused 除颜色外有可见状态文案；业务 label 保留具体对象。
- 状态不改写百分比，Success 应由宿主在真实完成后传入；不会把 45% 自动变成 100%。
- Error / Paused / Success 停止不确定循环；仍允许宿主更新真实数值。
- `.show_value(false)` 可隐藏可视百分比，原生数值仍保留；`.value_text(...)` 可提供“12 / 40 项”等真实说明。
- 没有可交互 disabled 模式；暂停、失败与禁用是不同概念。

### 4.3 动效

| 场景 | 表现 |
| --- | --- |
| 初次挂载、未知 ↔ 已知、shape / size / theme 改变 | 直接呈现正确值，不从 0 重播 |
| 确定值更新 | 150ms standard；可中断、可反向，不做弹簧超调 |
| 高频更新 | 始终追最新目标；宿主可 animated(false)，不排队播放旧目标 |
| Linear 未知 | 一个长度缓慢变化的短段从左向右穿过，完整退出后无缝重入，1800ms |
| Circle 未知 | 使用与 Spinner 相同的旋转弧与时钟，900ms |
| 暂停 / 错误 / 完成、减少动效 | 确定值立即到位；未知保留静态短段 / 弧和状态说明 |

原生数值始终为真实目标，不是落后 150ms 的绘制值；百分比文本亦显示真实值。

## 5. Spinner

`YueSpinner::new(id)` 默认 active、Medium、150ms delay、900ms 匀速顺时针旋转；使用有缺口的圆弧，不旋转整个布局，不制造宽高抖动。支持 `.active(bool)`、`.delay(Duration)`、`.label(...)`、`.decorative(true)`、`.animated(false)`。

可视 label 在图形尾侧，长文本可换行。无可视 label 时仍有可访问名称；放进已有完整加载语义的 Button / LoadingRegion 时使用 decorative，避免重复朗读。

延迟时不暴露正在加载的 Spinner 节点，也不画透明点击层；宿主需要固定空间时自己保留稳定图标槽。active=false 立即返回空内容。组件不覆盖按钮、不创建全屏层、不截获鼠标、不增加 Tab 停靠。

减少动效时保留固定角度圆弧 + 文案，不能消失后只剩空白。默认延迟不影响业务操作开始、禁用或结果提交。

## 6. Skeleton

`YueSkeleton::new(id)` 为装饰原语，支持 `SkeletonShape::{Text, Rectangle, Rounded, Circle}` 和 `SkeletonAnimation::{Wave, Pulse, None}`，默认 Text + Wave。

- Text：占用真实行高，内部短一些的色块模拟字面高度；不要把多行文本压成同样粗的大色条。
- Rectangle：无圆角的图片 / 区块占位；Rounded 使用主题圆角。
- Circle：正方形占位及半径约束；不能使用矩形宽高拉成椭圆。
- 显式 width / height 决定最终结构，适配 rem、百分比和 flex；不随机生成长度。段落可由宿主用稳定 key 组合 100%、90%、60% 等长度。
- 全部纯装饰：不输出 ProgressIndicator，不逐行朗读“加载中”，无命令、hover、焦点或 Tooltip。

Wave 为 1800ms 的克制亮带，沿水平方向扫过；Pulse 为平滑的同相位明暗变化；None 固定底色。所有实例共用应用时间基准，后挂载的项目不产生独立闪烁节拍。明暗来自 token；亮带与底色均遵守同一圆 / 圆角轮廓，不能扫出直角。

不内建 AvatarCard / TableRow / Article 等业务模板；用布局原语组合。骨架只表示正在等待，不代表错误或空结果。

## 7. LoadingRegion 与内容交接

`YueLoadingRegion::new(id, loading).placeholder(Fn).content(Fn)` 只调用当前分支的 renderer。加载阶段输出 region busy=true，并以一个稳定 Status 节点提供 Polite 文案；占位本身隐藏装饰语义。完成后 busy=false，立即卸载占位并附加真实内容，不叠放两套可交互子树。

宿主持有 Input / Editor Entity，切换分支不复制或销毁宿主数据。组件不替宿主发请求，不自动将焦点跳进结果；宿主在需要移除当前持焦点编辑区域前明确协调焦点。首版不播放内容交叉淡入淡出，不留下透明输入层。

这不是权限或交互禁用包装器。刷新已有内容时，宿主可以保持内容并在旁边组合 Spinner / Progress，不必使用替换模式。

## 8. Token 与尺寸

外部唯一来源：`D:/100_Projects/110_Daily/Yue-Design-System/packages/tokens/src`。现有 progress.css / spinner.css 已有基础 token，本次扩展并新增 skeleton.css / loading.css，登记后同步 generated.rs / inventory，不手改生成文件。

| 族 | Small / Medium / Large 或默认基线 |
| --- | --- |
| Linear 高度 | 4 / 8 / 12px；保留 --progress-height 作为 Medium 兼容别名 |
| Circle 直径 / stroke | 32 / 48 / 64px；3 / 4 / 5px |
| Small Circle 中央字号 | 独立 --progress-circle-font-size-sm，10px，避免 32px 环内百分比溢出 |
| Spinner 直径 / stroke | 16 / 20 / 32px；2 / 2 / 3px；保留现有 Medium token |
| 反馈字号 | 12 / 14 / 16px，label 与图形间距读取 space-8 |
| Skeleton Text 行高 / 字面高度 | 16/8、20/12、24/16px |
| Skeleton Circle 直径 | 24 / 40 / 56px |
| Skeleton 默认区块 | 160 × 80px，宿主优先明确与真实内容一致的尺寸 |
| 时长 | value 150ms；spin 900ms；track / skeleton 1800ms；Spinner delay 150ms |

长度均通过 rem 转换，表中 px 是 16px rem 基线。几何、路径细分步数、归一化进度、相位和 device-scale 裁切不属于产品间距硬编码。

## 9. 原生可访问性

- Progress / 非 decorative Spinner：一个命名 ProgressIndicator；确定数值 0–100，未知不提供 numeric_value。
- LoadingRegion：Group + busy 原生标记；加载文案由一个 Status + Polite 节点提供，不随着每帧相位改写。
- Skeleton 与 decorative Spinner 的绘制不产生可交互语义。
- 不自动每 1% 发 live announcement；重要里程碑或结果由宿主决定如何告知。
- 所有反馈不增加键盘停靠，不抢夺鼠标。已有 Tooltip / Popup / Dialog 层级保持原所有权。
- Headless 原生节点测试与 Windows UIA / 读屏真人检查分别记录；真实 UI 由用户负责。

## 10. API 示例

```rust,ignore
YueProgress::new("upload", ProgressValue::new(self.percent))
    .label("上传附件")
    .size(FeedbackSize::Medium)
    .shape(ProgressShape::Linear)
    .status(ProgressStatus::Normal);

YueSpinner::new("query-wait")
    .active(self.query_pending)
    .label("正在搜索文件");

YueLoadingRegion::new("profile", self.loading)
    .label("正在加载个人资料")
    .placeholder(|_, _| div().flex().gap_3()
        .child(YueSkeleton::new("avatar").shape(SkeletonShape::Circle))
        .child(YueSkeleton::new("name").width(rems(10.)))
        .into_any_element())
    .content({ let profile = self.profile.clone(); move |_, _| profile.clone().into_any_element() });
```

`theme(...)` 局部覆盖和 `current_theme` 继承遵循现有组件；外部布局使用 ControlLayout。完整可编译示例以实际源码 doctest 为准。

## 11. 实施与验收

1. Base 规范化数值、未知语义、一次性显示门限、共享循环时钟、busy 与分支边界。
2. 扩展外部 token 并同步；先实现 Progress，再接入共用弧绘制的 Spinner，最后 Skeleton 与 LoadingRegion。
3. Showcase 按 Progress / Spinner / Skeleton 分别提供入口；包含尺寸、状态、未知、暂停、短任务、快速切换、延迟、波动 / 呼吸、真实内容替换和减少动效。
4. 纯测试覆盖数值、几何和时序；headless 覆盖真实语义、内容隔离、可见帧、进度反向、缩放和主题；最后工作区测试、Clippy、token 检查与独立预览构建。

不启动应用或代用户做真实 UI 验收，不修改无关的既有未提交工作，不提交 Git commit。

### 11.1 已落地代码

| 层 | 路径 / 导出 | 职责 |
| --- | --- | --- |
| Base | `vendor/gpui-base/src/loading.rs` | ProgressValue、LoadingDelay、loading_phase、LoadingRegion |
| Base | `progress.rs` / `a11y_live.rs` | 数值 / value_text 与 busy 原生投影 |
| Yue | `crates/yue-component/src/loading/mod.rs` | FeedbackSize、YueLoadingRegion；根级及 prelude 导出 |
| Yue | `loading/progress.rs` | YueProgress、ProgressShape、ProgressStatus；真实语义与短绘制过渡 |
| Yue | `loading/spinner.rs` / `paint.rs` | YueSpinner、公共弧绘制、轨道短段；延迟不依赖循环重绘 |
| Yue | `loading/skeleton.rs` | YueSkeleton、四形状与三种动画；扫光蒙版保留完整圆角轮廓 |
| 既有组件集成 | `button.rs` | loading 图形复用公共 painter / clock；保留按钮尺寸、颜色、时长与禁用行为，无显示延迟 |
| Showcase | `examples/windows-showcase/src/pages/loading.rs` | 三个独立导航入口；真实受控值、短 / 长任务演示、骨架 / 内容替换 |

Button 的加载图标已迁入共享绘制路径。Dialog 与 Toast 的既有图标及 Toast 阅读倒计时不在本次迁移范围内；尤其不能把 Toast 的剩余阅读时间解释为任务 Progress。公共时钟支持后续逐步接入，不要求每个反馈都拥有统一的业务状态机。

实现细节：

- `.show_value(false)` 只隐藏普通百分比，不隐去错误 / 暂停 / 成功状态；原生真实值不变。环中心以符号呈现状态，线形用状态文字。
- 静态未知进度使用固定相位的短段 / 圆弧，不保留一个不断唤醒的假暂停定时器；恢复时重新加入共享相位。
- Wave 先画完整圆角底色，再以左右两个渐变半段通过局部矩形 mask 重画同一圆角轮廓；不把亮带画成越出轮廓的方块。
- 完全裁切、零尺寸、失活或隐藏窗口不请求循环帧；减少动效与静态样式也不请求循环帧。相位依赖单调时钟而非系统日期。
- LoadingRegion 的分支切换保持立即提交；若宿主需要输入保留，应持有同一个 Input Entity，而非让骨架层复制数据。

### 11.2 验证记录

已通过：

- `cargo test --workspace --locked --features headless-tests`：578 项通过，包含 32 项 Yue 公开 API doctest、新增 8 项加载反馈 headless 集成测试及 Showcase 三入口回归。
- `cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings`。
- token 快照检查，以及与外部 Yue-Design-System 源的一致性检查。
- Base 原生 / 定时 / 帧调度测试：`loading::tests` 5 项、`progress::tests` 4 项、`a11y_live::tests` 1 项通过。覆盖短任务与新门限、busy 的置位 / 清除、非有限值，以及静态 / 减少动效 / 裁切 / 零面积 / 失活 / 隐藏时不请求循环帧，恢复后重新加入共享相位。
- `cargo test -p yue-component --locked --no-default-features --features headless-tests --test loading`：系统字体配置下 8 项通过。
- `cargo build --manifest-path TEMP/loading-preview/Cargo.toml --target-dir target --bin windows-showcase-loading-preview --offline`：独立预览构建通过；新增及修改的加载反馈 Rust 文件已通过限定范围 rustfmt 检查。

新增集成测试覆盖真实目标值与中间绘制值的差异、首次不从零重播、反向衔接、钳制 / NaN / Infinity、状态与形状切换、Spinner 延迟前后及短任务取消、再次激活、装饰模式、活动分支独占、旧按钮退出输入树、文本骨架字面高度、正圆与圆角、同步呼吸、扫光的真实提交数据及裁切、减少动效、明暗主题，以及 rem 16 / 20 / 24 与显示倍率 1 / 1.5 / 2 的组合。尺寸断言允许一个设备像素内的布局对齐误差，不用宽松比例掩盖漂移。

独立预览使用 `TEMP/loading-preview/Cargo.toml`，输出 `target/debug/windows-showcase-loading-preview.exe`；不覆盖旧预览二进制、不启动进程。组件导航新增 **Progress / Spinner / Skeleton** 三项，右侧保留现有页面滚动与全局减少动效开关。

上述 headless 验证只检查实际原生属性、输入分发和绘制提交数据；不声称验证了 Windows GPU 像素、UIA / Narrator 朗读或真实窗口观感。
