# YueSteps 实施与验证记录

日期：2026-11-09；对应[设计文档](../43-steps.md)。参考图片已保存在 docs/assets，设计合同先于代码落地。

## 交付

新增 `YueSteps`、`YueStepItem`、`StepsValue`、`StepsChange` 及类型化尺寸、布局、状态、外观和标签位置配置；从 crate 根导出，高频入口加入 prelude。实现位于 `crates/yue-component/src/steps/{mod,model,element,tests}.rs`。

支持三档尺寸、水平／垂直、下方标签、编号／Tabler 图标／点状、标准／导航／箭头外观、等待／进行中／完成／错误、单项显式状态、逆序、只读／禁用及额外内容。Navigation／Arrow 仅支持 Horizontal + Right，非法配置明确失败。垂直布局始终把文字放在标记右侧。

业务值完全受控：回调请求稳定 key、逻辑 index 和 before，宿主通过下一帧 value 接受；拒绝和外部更新不产生隐藏状态或额外事件。Pending／Complete 显式表达首尾。单项状态优先于当前状态及位置派生；逆序只改变显示顺序。

交互复用 Base Button，图形复用现有 Icon／Tabler，动效使用 Base transition 与 Yue Interaction token，外部尺寸复用 ControlLayout，横向浏览复用 YueScrollbar。没有新增 Cargo 依赖或 GPUI／Base 补丁。现有工作区中的 Tooltip／tab_stop 等修改来自前一轮 Pagination，未归为 Steps 新增工作。

键盘 Tab／Shift+Tab 遍历真实按钮和 extra 控件；Enter／Space 走同一请求。焦点以 key 保留，删除／禁用时仅修复本组拥有的焦点。根为 List，步骤为 ListItem；静态项没有伪按钮，状态和逻辑位置包含在可访问名称中。不声称具有 UIA Current 或已经完成 Narrator 真机验收。

## Showcase

新增 **组件 → 步骤条 Steps**，加入导航排序和页面切换回归矩阵。

- 导入向导：选择内置工单／联系人示例，填写批次名，核对、模拟失败、重试、完成、重新开始；返回后保留宿主 Input Entity。复用 Select、Field、Form、Input、Button、Checkbox、Card 和 Empty。
- 可操作配置：三档尺寸、水平／垂直、Right／Below、四种外观、逆序、只读、禁用、当前错误、宿主拒绝、全等待／全完成。切换到 Navigation／Arrow 时页面原子规范化布局，避免非法组合。
- 独立示例：三尺寸、Dot、Navigation、Arrow、自定义图标、垂直审批 extra 命令、禁用归档项，以及窄容器长标题和显式错误状态。

## 验收中修正的布局问题

1. Base Button 默认居中。在步骤标题中显式使用起始对齐，让标记列位置固定；长标题只向下换行，不下移标记。Right 的连接线从标题后方自然延伸，避免假定父背景色遮线；Below 连接相邻的左右半段，垂直连接线随说明和 extra 自然高度延伸。末项没有尾线，逆序仍按逻辑相邻状态着色。
2. `ScrollHandle::scroll_to_item` 在自身 prepaint 更新 viewport 信息之前运行；同时改变容器宽度和 current 时可能读取旧宽度。增加布局后的几何观察，仅在尺寸／目标／rem 改变时按实际几何再次揭示。稳定状态不持续请求帧，也不会因普通重绘拉回用户手动滚动位置。

## 验证记录

| 检查 | 结果 |
| --- | --- |
| `cargo test -p yue-component --features headless-tests --lib` | **540 通过**，包括 Steps 新增 **14 项**。覆盖状态优先级、非法配置、鼠标／Enter／Space、受控拒绝与接受、同值／外部更新静默、禁用／只读、独立 extra、Tab 顺序、key 焦点修复、长文本连接线／末项、逆序、窄容器揭示、空／单项／首尾、四外观、四主题 × 三尺寸 × 12／16／24px rem。 |
| `cargo test -p yue-component --features headless-tests --test controls --test field --test tabs --test input --test loading` | **55 通过**，检查复用行为、输入／表单、焦点、主题、布局和动效回归。 |
| `cargo test -p windows-showcase --features headless-tests --bin windows-showcase` | **58 通过、9 原有忽略**。含两项 Steps 页面测试与导航矩阵；实际输入、必填错误、返回、失败／重试、完成／重开和独立命令均经过真实点击。 |
| `cargo clippy -p yue-component -p windows-showcase --features headless-tests --all-targets -- -D warnings` | 通过。 |
| 设计系统 `node node_modules/vitest/vitest.mjs run tests/token-steps.test.mjs tests/token-pagination.test.mjs tests/token-card.test.mjs` | **15 通过**，Steps 专项 5 项，验证 34 个 token 的登记／命名／解析和四主题文字／状态配色对比度至少 4.5。 |
| `node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check` | 通过，inventory／Rust 生成映射与源一致。 |
| 新增 Rust 文件的 `rustfmt --edition 2024 --config skip_children=true --check` | 通过。 |

最终布局调整后的 Steps 专项复测 **14 通过**。补充完成：`cargo test -p yue-component --doc steps:: --quiet` **1 通过**；`cargo check --workspace --all-targets --quiet` 通过；独立预览构建 `cargo rustc -p windows-showcase --bin windows-showcase -- --emit=link=target/steps-preview/windows-showcase.exe` 通过，生成 `target/steps-preview/windows-showcase.exe`。文档本地链接检查和 `git diff --check` 通过。

## 设计系统与回退边界

相邻 `Yue-Design-System` 中新增 `packages/tokens/src/component-tokens/steps.css` 和 `tests/token-steps.test.mjs`，更新 import、Steps 命名词表和 package-only 登记。由同步工具生成本仓库 inventory 与 generated.rs，不手改生成代码。

当前仓库和相邻仓库原有大量未提交改动，包括 Card／Pagination。本轮不提交或推送；回退只处理 Steps 文件、导航／导出／README 接线与 Steps token 增量，不能整文件恢复共享文件而覆盖原有工作。

全工作区 rustfmt 和相邻设计系统全量 token 审计有既有失败，基线说明见 [Card 记录](2026-11-09-card-implementation.md)。本轮使用新增文件格式检查及组件 token 专项，未把专项通过称为全量通过。

Windows GPU 视觉、Narrator、系统 DPI／高对比度仍由用户真机验收；应用 rem 测试不能替代系统 DPI。此次不启动原生窗口或结束用户进程。建议检查长标题／subtitle、Dot／Arrow、焦点环、窄容器拖动及普通／减少动效设置。

## 2026-11-10 真机截图反馈修正

用户截图指出 Arrow 说明贴近底边、Below 窄容器文字相互重叠。增加回归测试后，原代码分别复现为上边距 8px／下边距 0，以及单项 160px 宽度下标题宽达 219px 且不换行。此前测试覆盖了节点和连接线几何，没有断言标题文字范围及箭头底部内边距。

Arrow 的上下内边距现在统一由整块表面拥有，标题与说明之间独立保留 gap；说明和 extra 不再贴边。标题文本包裹在可约束宽度的块内；Below 标签占满所属列并留内侧间距，长文本居中换行。连接线与节点仍沿用原有中心几何。

窄容器中的第三步原本位于右侧横向滚动区域，保留该布局合同；示例补充说明，新增滚动回归验证可以到达末步且稳定重绘不会拉回当前位置。此次没有修改 tokens、公共 API、依赖或 vendor。

新增三项回归：窄容器 Below 长标题范围／换行（16／24px rem）、Arrow 对称上下留白（三尺寸 × 两档 rem）、横向滚动至末步及位置保留。Steps 专项 **17 通过**。

本次组件库单元测试 **543 通过**；Steps 页面流程 **2 通过**；严格 Clippy、改动文件 rustfmt、文档链接和 `git diff --check` 均通过。真机视觉由用户复测。

独立预览构建通过：`target/steps-layout-fix/windows-showcase.exe`，用于本次布局修复复测；未启动原生窗口。
