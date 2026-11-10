# YueCard V1 实施与验证记录

日期：2026-11-09。范围：[Card 设计合同](../41-card.md)中的 V1、Windows Showcase 与设计系统 token 同步。

## 交付

- `YueCard / YueCardHeader / YueCardMeta / YueCardCover / YueCardFooter` 从 crate 根导出。四种表面、三档尺寸、区域省略、标准／自定义头部、标题溢出提示、四种封面位置、独立正文 inset、自定义／等分页脚、受控选中外观与悬停反馈已实现。
- 可见主 Button 与静态区域共享一份回调及焦点；键盘、disabled、loading 仍由真实 Button／Base 拥有。Card 根没有点击转发，也没有新增 Tab stop。leading、extra、自定义 Header 及其他 Footer 命令独立。正文／封面含交互元素时使用普通 Footer Button。
- 指针拖动超过 5 个 GPUI 逻辑像素后，即使返回起点也取消激活；右键、已有文本选择及新建的真实文本选区均不触发主操作。
- `YueListItem::inset(false)` 是唯一新增的跨组件公共布局开关：移除壳的 padding／gap，并把网格行高传给 Card。默认开启 inset，保留普通列表布局。
- 封面根在类型擦除前应用实际四角。GPUI 矩形 overflow 不等于后代圆角蒙版；嵌套背景、图表绘制及 Img fallback 仍由宿主遵守相同形状。根 Card 不裁剪后代焦点环。
- 省略区域不生成空栏；空 `Footer::actions([])` 不占位；Header 紧邻 Footer 时只画一条分隔线；减少动效取消 Lift 位移并即时完成视觉切换。

Showcase 新增 **组件 → 卡片 Card**，包括基础／简洁、四种表面、四种封面、图片失败回退、头像／头像组、更多菜单、真实选择控件、详情 Dialog、可选文本、Tabs／Input／Collapse、首次加载、刷新、取消、空态、错误重试、普通网格与 10,000 张工单的虚拟网格。首次加载使用现有 LoadingRegion 分支替换；刷新保留内容。业务输入 Entity、选择值与异步任务由页面持有，两处同时展示的输入使用各自 Entity。

## API 与设计稿的落地差异

1. `YueCardMeta::new(id, title)` 要求稳定 ID，隔离独立溢出 Tooltip 的 keyed state。
2. `YueCardCover::new` 接受 `IntoElement + Styled + 'static`，以便在实际媒体根应用圆角；自定义元素可使用可设置样式的包装根。
3. 主操作命中只扩展到明确的标准文字列、静态正文／封面及其内部空白；Header 外侧 padding 等未接线区域不假称“整张卡片均可点击”。
4. Card 不创建选择组、加载状态机、网格或滚动所有者；覆盖式保留挂载加载、整卡选择命中和背景图对比度算法不在 V1。

## 自动化证据

以下测试使用 GPUI 无头窗口、实际输入事件、布局快照及绘制 quad；不等于 Windows GPU 截图或系统 UIA 验收。

| 命令／范围 | 结果与证据 |
| --- | --- |
| `cargo test -p yue-component --features headless-tests --lib card::tests --quiet` | **10 通过**。全部区域省略组合、四种封面顺序、真实媒体 quad 的外角、四主题 × 三尺寸 × 三 rem、selected 不改尺寸、ListGrid 同行表面与页脚对齐、主命令鼠标／Enter／Space、禁用／loading、独立动作、往返拖动／右键、真实文本选区、非法尺寸、减少动效和分隔边界。最后一次核心整理后复跑通过。 |
| `cargo test -p windows-showcase --features headless-tests --quiet` | **54 通过、9 原有忽略、0 失败**，含 Card 导航。Card 三项页面交互测试另行复跑通过，新增了真实输入编辑后的值保留断言。 |
| Card 三项页面测试 | 真实按钮切换尺寸／宽度／截断、Checkbox 角色与受控值、独立收藏、Dialog 关闭焦点返回；加载取消不会覆盖手动空态、失败重试、刷新禁用及实际备注值保留；10,000 卡片只挂载有界行数（大于 0 且小于 100），收窄容器后列数正常。 |
| `cargo test -p yue-component --features headless-tests --lib --test list --test list_grid --test loading --test controls --test checkbox --test radio --test tabs --test dropdown --test input --quiet` | **629 通过、1 原有忽略、0 失败**：512 单元测试及 117 项相关集成测试。含已有 Button、List、Loading 与输入／选择回归；Card 10 项包含在 512 项中，不重复累计。 |
| `cargo test -p yue-component --doc card:: --quiet` | **1 通过**，Card Rustdoc 可编译。 |
| `cargo check --workspace --quiet` | 通过。 |
| `cargo build -p windows-showcase --quiet` | 通过，生成 `target/debug/windows-showcase.exe`，供用户真机验收；本次未启动窗口。 |
| `cargo clippy -p yue-component -p windows-showcase --features headless-tests --all-targets -- -D warnings` | 通过，无警告。 |
| `rustfmt --edition 2024 --config skip_children=true --check`，参数为 Card 的三个 Rust 文件和 Showcase Card 页面 | 通过。 |
| `git diff --check` | 通过；Git 提示两个既有 List 文件后续写入索引时规范 CRLF 为 LF。 |
| `node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check` | 通过，生成映射与源一致。 |
| 设计系统内 `corepack pnpm exec vitest run tests/token-card.test.mjs` | **5 通过**：Card 名称／注册／可达性、四主题所有文字与普通／填充／选中表面的对比度至少 4.5、密度顺序及完整解析。 |

## 全量门禁的既有失败

`cargo fmt --all -- --check` 仍失败：既有 Checkbox、Field、Tree、TimePicker、主模块导出排序等文件未符合当前 rustfmt。未格式化无关组件；新增 Card 文件通过单独检查。完整输出保留在本地 `target/card-fmt-final.txt`（构建产物，不纳入 Git）。

相邻设计系统的 `corepack pnpm audit:tokens` 仍失败：已有 Badge／Calendar 等命名词表、目录登记及对比度问题。通过只暂时移除 Card import 并在 finally 恢复入口的方式建立无 Card 基线，确认失败早已存在；最终 Card 专项测试通过，架构审计为 1556 tokens、63 可达文件、40 目录组，unreachable／undeclared／duplicates／cross-file slots／layer violations 全为 0。日志保留在本地 `target/card-token-audit-{baseline,after,final}.txt`，不能将专项通过称为全量审计通过。

## 设计系统改动与回退边界

相邻 `Yue-Design-System` 仓库原先有大量未提交修改。本任务仅新增 `packages/tokens/src/component-tokens/card.css`、入口 import、`tools/lib/token-vocabulary.mjs` 的 Card 词表、`tools/token-audit.pairs.mjs` 的 Card package-only 登记及 `tests/token-card.test.mjs`。

当前仓库同步快照新增 **34 个 Card tokens**；逐项比较同步前后 `resolved`，既有 tokens 的解析值没有变化。Rust TokenKey 按名称排序，增加 Card 后续编号移动，导致生成文件 diff 较大。不能手工还原其中一半编号；应通过源和同步工具整体重新生成。下游构建只消费本仓库快照，不要求存在相邻目录。

没有 Cargo 依赖、Lockfile 或 vendor 补丁变更。代码与文档保留为工作区改动，未提交或推送。若回退，按 Card 文件及明确接线改动回退；相邻仓库共享文件只移除本次 Card 增量，不能整文件恢复从而覆盖已有修改。

## 真机验收交接

由用户执行 Windows 视觉验收，重点检查四种表面的阴影／边界、媒体与失败回退圆角、标题溢出 Tooltip、同行 Footer、外侧焦点环、页面／局部虚拟网格滚动，以及普通／减少动效下的悬停。检查详情 Dialog 和更多菜单关闭后的焦点、真实选择与备注编辑、首次加载／刷新／重试。

Narrator、系统高对比度、实际 GPU 像素与系统 DPI 尚未执行；16／20／24px rem 布局测试只证明应用缩放布局，不能替代系统 DPI。当前单设备环境无法完成多显示器／跨 DPI 移动验收。本次未启动用户真机窗口或结束用户进程。
