# YueDescriptions 实施与验证记录

日期：2026-11-10；对应[设计文档](../44-descriptions.md)与[参考 demo](../prototypes/descriptions.html)。

## 已实现

新增 `YueDescriptions`、`YueDescriptionItem` 与类型化布局、密度、跨度、水平／垂直对齐、文本溢出选项。公共字段私有，从 crate 根导出，高频组件入口进入 prelude。组件为 RenderOnce，宿主持有记录、输入 Entity、校验与保存状态。

支持可选标题／extra、最大列数与容器降列、Columns／Filled 跨列、超长跨度收缩、有边框行尾补齐、无边框留空、水平与上方标签、极窄堆叠、三密度、独立标签／内容对齐、item 垂直对齐覆盖、冒号、自定义标签及内容。空集合不生成空白行或边框。非法 key、重复 key、非法尺寸、零跨度／行数、无载荷复制自定义内容等配置明确失败。

无新增 Cargo 依赖。不使用 Kit styled DescriptionList、DataTable 或虚拟列表，不新增业务编辑器／内部滚动所有者。

## 换行后同排对齐

使用一个 native grid，所有描述项直接作为稳定 key 的子节点。装箱只计算有效跨度；原生网格决定顺序行列、自然行高和 stretch。降列不会把项移入不同的“行父节点”。

每项的标签格和值格填满整排高度；格内再用列向 flex 的 justify 控制 Top／Center／Bottom。文字水平对齐独立，任意内容插槽使用格内行向 justify。长编号换为多行后，短标签、正文和组合内容仍在自己的内容区居中。上方标签始终保留在顶部，剩余高度分配给值格。

复制按钮与正文共享格内内容组合，先预留按钮和 gap；按钮对齐整段文字块中心。连续编号、URL 与中文段落受可收缩文本盒约束。没有固定行高、特殊文字偏移或逐格手工高度缓存。

有边框时 grid 拥有外围边框、各项拥有右侧／底部分隔、标签格拥有标签／值分隔；各条线只有一个所有者。分隔独立绘制在 grid gap 中，单元格及 grid 根保持透明，不使用背景铺色模拟网格线；整组不使用 overflow_hidden，避免裁掉真实控件的焦点环。

## 自适应机制的实际边界

当前 GPUI `container_query` 的内容不能反向决定自身自然高度，直接套用会丢失详情列表的自然高度。这里使用受控的离散失效：canvas 读取实际网格内容宽度，只在列数或极窄堆叠模式改变时 defer 一次通知。

原生 grid 的像素列宽始终取本帧可用空间；组件不保存上一帧像素宽度、不在每次 prepaint 无条件通知、不插值动画。首次显示或跨阈值后最多下一帧更新离散列数。稳定状态通过“没有新动画帧请求”的测试。这一机制与设计文档同步公开，未声称是同步 container query。

## 共享 OverflowTooltip 与 GPUI 缓存修正

`YueOverflowTooltip` 新增 `.lines(n)` 和 `.focusable_when_overflow(true)`。默认仍不新增键盘入口，保留 Breadcrumb、Card、Tabs、Select、Tag 等已有使用方式。

- 单行比较自然宽度；多行以真实字体、字号和当前宽度执行完整 shape_text，比较实际换行数。
- 显示端同时设置省略和 line_clamp；单行把物理换行转换为空格，仅影响展示，提示、可访问名称与复制原文保持完整。
- 只有实际截断时注册 Tab stop。焦点触发复用共享 controller；变宽或改文后不再截断时，关闭提示、撤销入口，仅修复自己拥有的焦点。
- 自建焦点的入口卸载时释放并修复自己的焦点；不替外部控件创建第二个命令或接管业务焦点。

测试最初发现：两行文本确实被截断，却无法通过 Tab 进入完整提示。原因是 GPUI `LineLayoutCache` 的缓存身份缺少 `max_lines`；同一文本、字体、宽度的完整测量命中了先前的受限换行结果。现向普通缓存的 owned／borrowed key 补充该字段，无限制与未换行条目明确使用 None。

这是 7 行的必要 GPUI 正确性补丁，维护入口为 [YUE-PATCH.md](../../vendor/gpui-pre/YUE-PATCH.md)。回归覆盖同宽度下 2 行、无限制、1 行、再次 2 行的混合测量，以及完整 Tooltip 使用方。上游修复后按同一回归验证再移除。

## 复制生命周期与语义

复制复用真实 YueButton、Tabler Copy／Check 与 GPUI Clipboard。点击、Enter、Space 走同一控件命令；文字或自定义显式载荷复制完整原文，不复制视觉省略号。反馈保持 4 × 共享 settle 时长，即当前主题 1.2 秒；不创建常驻计时器或全局 Toast。

状态按根身份与 item key 保存；载荷／文本修订会取消旧任务，重复复制重新计时，删除项或卸载会释放任务。弱 Entity 与代次校验防止旧反馈写入新记录。自定义插槽没有可比较的文本修订，宿主应以记录身份更新 root key 或显式 copy_text 载荷，组件不会猜测任意子树的业务版本。

语义使用 `DescriptionList`、`Term`、`Definition`。静态项不添加 Tab stop，真实输入、按钮和链接保持自己的语义与按键路径；自定义输入由宿主显式命名。角色存在和 headless 语义断言不等于 Narrator 已验收。

## Showcase

入口：**组件 → 描述列表 Descriptions**，已接入导航排序及页面切换矩阵。

1. 结构：标题、extra、编号复制、状态、头像与 Filled 备注。
2. 布局：1–4 最大列数、标签方向、三密度、边框、冒号、标签宽度和独立水平／垂直对齐；可拖动分栏内的两个详情表各自降列。分栏示例的滚动由宿主面板持有。
3. 跨列：Columns、Filled、99 列收缩、提前换行、有边框补齐和无边框留空的对照。
4. 专门的同排多行例：工单编号 1–3 行，旁边为徽标与头像姓名；沿用同一套对齐控制。
5. 长内容：中英段落、连续 ID、URL、单行／两行省略、短文不提示、宽度变化后重测。
6. 插槽：Link、TagGroup、Progress、Avatar、Badge、自定义标签与显式复制载荷。
7. 主从详情：YueList 选单，YueCard 详情，保留 Input／Select Entity；Form 校验保存、取消保留原值，进入编辑时聚焦输入，保存／取消后回到编辑按钮。

没有为参考 demo 的业务故事新增公共组件状态或网络服务。详情修改也不触发整表入场动画。

## Token 与验证

源位于兄弟仓库 `Yue-Design-System/packages/tokens/src/component-tokens/descriptions.css`；注册 import、词汇表及 package-only 清单，新增专项测试。30 个 token 包括字号／行高、padding、行列间隔、三种布局宽度、正文与标签文字、边框和组合间距。padding 映射到共享间距尺度 8／12／16，而非在 Rust 重复 demo 的 6／10／14 裸值。

通过同步工具生成本仓库 inventory 与 generated.rs，没有手改生成内容。新 token 的四主题解析与正文／标签对比度专项通过；同时回归 Card、Pagination、Steps token。

自动化记录：

- Descriptions 纯布局与真实 UI 集成：容器阈值、0 宽恢复、跨度与实际边框间距、非法配置、三密度 × 四主题 × 12／16／24px rem，同排等高与格内居中、Top／Bottom、任意插槽两轴对齐、连续文本与复制按钮边界。
- 交互：完整载荷、Enter／Space、重复复制重计时、内容修订／移除、稳定页面停止帧请求；降列保留真实 Input 的 Entity、文本、路径与焦点。
- Tooltip：一／N 行真实溢出、短文不提示、条件 Tab、变宽／改文／字号／rem 后恢复，以及 GPUI 混合 clamp 缓存回归。
- Showcase：选择、编辑、必填错误、保存、取消、焦点返回与可操作配置；导航矩阵纳入新入口。

| 验证命令 | 结果 |
| --- | --- |
| `cargo test --workspace --locked --features headless-tests` | 1178 通过、0 失败、21 个既有人工／硬件用例忽略；其中组件 lib 558、Showcase 60（9 忽略）、组件 Rustdoc 42。日志 `target/descriptions-workspace-tests.log`。 |
| `cargo test -p yue-component --no-default-features --features headless-tests --lib descriptions::tests` | 系统字体路径 12 项通过，包含工作区运行之后补充的 native span／单线间距验收。日志 `target/descriptions-system-font-tests.log`。 |
| `cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings` | 通过，覆盖最新增加的测试源码。日志 `target/descriptions-clippy.log`。 |
| `corepack pnpm exec vitest run tests/token-descriptions.test.mjs tests/token-card.test.mjs tests/token-pagination.test.mjs tests/token-steps.test.mjs`（设计系统仓库） | 4 个文件、20 项通过；Descriptions 专项 5 项。 |
| `node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check` | 同步一致。 |
| 新增 Rust 文件及本轮 Tooltip 文件的 `rustfmt --check`，`git diff --check` | 通过；未为无关模块执行整库格式重写。 |
| `cargo rustc -p windows-showcase --bin windows-showcase -- --emit=link=target/descriptions-preview/windows-showcase.exe` | 正常功能配置编译和链接通过，生成独立预览。日志 `target/descriptions-preview-build.log`。 |
| 文档相对链接、参考 HTML 副本字节比较 | 通过，原参考内容保留。 |

计数为各次验证的实测记录，新增的 span 回归单独列出，未把较早工作区运行伪记为已执行更新后的全部测试。

## 真机交付

独立预览位于 `target/descriptions-preview/windows-showcase.exe`，构建不覆盖常规运行中的 EXE，不启动或结束用户的原生进程。用户负责 Windows GPU 视觉、滚动手感、Narrator、系统 DPI／高对比度验收；自动化未声称覆盖这些设备行为。


## 截图问题修复（2026-11-10）

按用户反馈移除标签、值与网格根背景色。头像此前与值格共用了 surface-component，因而圆形轮廓与格子融为一体；现在格子透出 Card 表面，Avatar 继续使用独立背景，无须修改头像全局样式。取消两项不再使用的背景 token，并同步源、登记、专项对比度测试和生成产物。

“跨度与行尾补齐”的第二组没有多渲染列：Card 的百分比高度、零最小高度与内部零基准 flex 组合，使自然高度小于实际内容。保留卡片在等高列表中的伸展能力，恢复根的内容最小高度，内部列及正文使用自然基准增长；Descriptions 根和网格不参与纵向压缩。未通过裁切隐藏溢出。

新增 Showcase 回归覆盖 1–4 列、宽窄容器、Horizontal／Vertical，断言两组及其所有内容都在 Card 内且互不重叠。新增实际绘制回归覆盖四主题、有／无边框：标签格、值格及网格不铺色，头像保留圆形背景，项间竖分隔保持单线宽度。Card 的等高、同排页脚对齐回归也纳入验收。


本轮验证：全工作区 `cargo test --workspace --locked --features headless-tests` 共 1181 通过、0 失败、21 个既有人工／硬件用例忽略；严格工作区 Clippy 通过；四组件 token 共 20 项通过；主题同步、定向 rustfmt 与 `git diff --check` 通过。日志分别为 `target/descriptions-screenshot-fix-workspace-tests.log`、`target/descriptions-screenshot-fix-clippy.log`。独立预览构建通过，位于 `target/descriptions-fix-preview/windows-showcase.exe`，未启动原生窗口；用户继续负责真机视觉验收。
