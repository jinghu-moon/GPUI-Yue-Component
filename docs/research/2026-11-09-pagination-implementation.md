# YuePagination V1 实施与验证记录

日期：2026-11-09。范围：[Pagination 设计合同](../42-pagination.md)、Windows Showcase、设计系统 token 与实际交互验收。

## 交付与使用

- 新增 `YuePagination`、`YuePaginationState`、`PaginationValue`、请求载荷及类型化配置，从 crate 根导出；高频入口加入 prelude。模块按 model／state／element 拆分。
- 普通／简洁分页、总数／范围文案、每页选择、精确整数跳页、首末页、省略号快速跳转、三档尺寸、无框／描边、单页隐藏、禁用与加载均已实现。
- 复用 Base Pagination 的有界页码计划和请求 guards，以及 YueButton、YueSelect、YueInput、ControlLayout、现有主题和动效。没有新增 Cargo 依赖。
- 宿主持有唯一提交值；`PaginationChange` 只请求目标快照，宿主通过 `set_value` 接受，拒绝不改变当前页或每页条数。外部 setter 不反发事件。每页数量变化以一个载荷同时请求 page 与 page_size，支持 Reset／KeepFirstItem。
- `PaginationValue` 使用精确 usize 整数与防溢出分页数学；`item_range()` 返回真实数据切片。跳页不用 f64 通道，覆盖 Windows 64 位超过 2^53 的页码及 usize::MAX。
- 页码按钮、输入和 Select 各自保留稳定焦点；数字页消失时移到新当前页。Enter 确认、Escape／Blur 撤销；跳页框局部 Tab／Shift+Tab 使用 Base Root 焦点遍历，正在组词时不处理 Tab／Escape。

Showcase 新增 **组件 → 分页 Pagination**。257 条真实工单在 List／Card 网格间切换；含尺寸／宽度／表面／模式控制、普通与极大页数、每页数量策略对照、可编辑简洁分页、空数据、禁用、宿主拒绝、900ms 异步失败／重试／取消、查询收缩和单页隐藏。数据、任务及代次由页面持有，不进入通用分页组件。

## 验收发现的两个基础问题

1. Tooltip 已关闭但保留退场画面时，禁用触发器会在每次 render 的同步中继续调用 close 并通知。这使异步失败后重试的真实点击无法结束。`close_owner` 现在仅在 present／moving 实际改变时通知；专门回归测试在非 reduced-motion 状态观察通知次数并确认重复绘制不再通知。失败／重试／取消的页面测试也完整通过。
2. GPUI 的语义外框与内层 Input 合法共享同一 FocusId，但 TabStopMap 重复登记后，Shift+Tab 会回到同一编辑器。最小补丁只保留当前帧第一次登记；原始操作仍进入 replay history。真实键盘测试验证跳页框 → 外部命令 → 跳页框 → 前一导航按钮，输入草稿取消且没有请求。补丁位置和上游移除条件见 [YUE-PATCH.md](../../vendor/gpui-pre/YUE-PATCH.md)。

Showcase 导航测试还改为先将接近窗口上下边缘的条目滚到内部再点击；添加新导航项后，目标中心仍在窗口内不等于整个按钮可达。

## 自动化证据

以下为 GPUI 无头窗口中的实际输入、状态、焦点和几何检查，不等于 Windows GPU 像素、系统读屏或真实 DPI 验收。

| 命令／范围 | 结果与证据 |
| --- | --- |
| `cargo test -p yue-component --features headless-tests --lib pagination:: --quiet` | **13 通过**：3 项纯整数／解析测试与 10 项实际组件测试，覆盖鼠标／Enter／Space、宿主拒绝、Select 原子请求／回退、真实编辑／Enter／Escape／Blur／双向 Tab、非法／溢出草稿、禁用／加载、首末边界、省略跳转、焦点消失、隐藏输入／关闭 Select、极大总数、四主题 × 三 rem × 三尺寸及窄容器。 |
| `cargo test -p yue-component --features headless-tests --lib --test list --test list_grid --test controls --test input --quiet` | **567 通过、1 原有忽略、0 失败**：526 项单元测试（含 Pagination 13 项、Tooltip 新增回归和已有 DatePicker／TimePicker／Select／TreeSelect 等）与 41 项相关集成测试。 |
| `cargo test -p windows-showcase --features headless-tests --quiet` | **56 通过、9 原有忽略、0 失败**。含新增页面导航和两项 Pagination 页面测试：真实数据切片／List 与 Card 布局切换；宿主拒绝后接受；异步失败保留旧页、重试成功、取消后旧响应不覆盖新查询。 |
| `cargo test -p yue-component --doc pagination:: --quiet` | **1 通过**，完整受控用法 Rustdoc 可编译。 |
| `cargo check --workspace --quiet` | 通过。 |
| `cargo clippy -p yue-component -p windows-showcase --features headless-tests --all-targets -- -D warnings` | 通过。 |
| `cargo rustc -p windows-showcase --bin windows-showcase -- --emit=link=target/pagination-preview/windows-showcase.exe` | 通过，独立预览 EXE 供真机验收，本任务未启动窗口。 |
| `rustfmt --edition 2024 --config skip_children=true --check`，参数为 Pagination 五个文件及 Showcase Pagination 页面 | 通过。 |
| `git diff --check`、新增文档本地链接检查 | 通过。 |
| `node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check` | 通过，生成映射与设计源一致。 |
| 设计系统内 `corepack pnpm exec vitest run tests/token-pagination.test.mjs` | **5 通过**：8 个组合 token 的名称／注册／解析及四主题辅助文字对比度至少 4.5。 |

## 构建与验证边界

常规 `cargo build -p windows-showcase --quiet` 完成链接后，无法替换正在运行的 `target/debug/windows-showcase.exe`（Windows 拒绝访问）。未结束用户启动的窗口；改用上述 rustc 显式输出路径，成功生成 `target/pagination-preview/windows-showcase.exe`。关闭现有窗口后可正常使用常规构建路径。

工作区全量 rustfmt 与相邻设计系统全量 token 审计存在既有失败，本轮没有扩散无关格式化，也没有把专项通过称为全量通过；基线说明见 [Card 实施记录](2026-11-09-card-implementation.md)。

Windows 真机视觉、Narrator 实际播报、系统 DPI 与原生 IME 候选窗口仍由用户验收。当前页以选中表面和可访问名称表达，未声称提供 UIA Current 属性。未知总数／游标分页、无限加载不属于此次已实现合同。

## 设计系统与回退边界

Pagination 增量为相邻 `Yue-Design-System` 中新增 `packages/tokens/src/component-tokens/pagination.css` 与 `tests/token-pagination.test.mjs`、入口 import、pagination 词表及 8 项 package-only 登记。本仓库由同步工具更新 inventory／generated.rs；不手改生成文件。相邻仓库和当前仓库原先均有未提交工作，包含上一轮 Card；本轮未提交／推送，回退时应按 Pagination 增量处理，不能整文件覆盖共享 token／README／导航／导出，也不能丢弃原有 Card 改动。
