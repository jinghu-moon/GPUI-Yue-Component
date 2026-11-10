# YueTable M1 实施记录

日期：2026-11-10。对应[主设计](../45-table.md)、[实际使用指南](../45-table-m1.md)及[性能方案](../performance/2026-11-10-table-benchmark-plan.md)。

## 1. 交付与复用

基础组件位于 [table/mod.rs](../../crates/yue-component/src/table/mod.rs)、[model.rs](../../crates/yue-component/src/table/model.rs)、[state.rs](../../crates/yue-component/src/table/state.rs)、[element.rs](../../crates/yue-component/src/table/element.rs)。

- 不可变 Source／View、非 Clone Row、稳定身份、列定义、静默原子接受、可重基的选择请求。
- Static 与 Rows＋Fixed；共享整行轨道和一个 ScrollHandle，固定表头、有限视口、横向溢出、自然多行与格内对齐。
- 单／多选、禁选行、全选／半选、范围选择、键盘导航与真实控件焦点；空／加载／刷新／错误、三档密度与边框。
- [最小外部消费者](../../examples/minimal/src/bin/table.rs)和 [Showcase](../../examples/windows-showcase/src/pages/table.rs)：属性切换、Card、Avatar、Tag、真实 Button、状态与重排、大数据示例按需创建／卸载。

复用 Base VirtualGrid／VirtualizerState 的冻结前缀、精确定位、工作集、焦点保留和单一滚动 owner；Rows 以一个横向轨道承载一整行，不建立第二个虚拟化引擎，也不将整行模式表述为 Cells。YueScrollbar、Empty、Spinner、OverflowTooltip、Checkbox、Radio、Card、Avatar、Tag、Button 均使用现有组件。唯一的近邻 API 补充是 `YueCheckbox::tab_stop`，转交 Base 已有同名能力，默认 true 保持原行为。

本次没有修改 GPUI 或 Base 源码、升级依赖或引入组件层。开发前 Git HEAD 为 `5767372`；本阶段按用户授权完成实现、测试与基线归档后提交、推送。此前设计与参考调研文档一并保留。开始时的 tracked diff 与状态备份在 `target/table-m1/preexisting.patch`／`preexisting-status.txt`。

## 2. 失败基线与实际修正

开始前 `cargo check -p yue-component -p windows-showcase` 通过；先新增 Table 公共合同测试，构建因公共 API 不存在而失败，再实现 API。

后续测试固定了这些问题：失败候选不能换 source；请求按最新 accepted view 重基；两个同帧动作不能因较新票据丢失第一个动作；旧拒绝不能清新等待；无变化不重跑几何；旧来源／空结果不能保活历史 source；内嵌 Button 的完整 Space 按下／松开由 Button 消费；Enter／F2 直接进入明确挂接的真实控制焦点，不从旧一帧的 Tab 顺序猜目标；AccessKit 元数据使用零基索引；Table 入口遵守侧栏既有字母排序。

## 3. 自动化验证

| 层次 | 结果与内容 |
| --- | --- |
| Table 合同与 headless UI | 18 项通过，1 项 CPU 探针默认 ignored；包含自定义坏 Source、重复／空 key、无 Clone 共享、原子拒绝、过期／跨实例票据、64 等待上限、同帧选择、真实键鼠、四主题、三密度、12／16／24px rem、空加载、header/footer、横向对齐、100k 行定位、焦点行稀疏保留与释放 |
| 近邻回归 | Checkbox 11、Radio 10、List 10、ListGrid 10、VirtualList 191 项通过；合计 232 项。既有 12 个 opt-in 性能探针未混入常规回归 |
| Showcase 页面 | `pages::table::tests::table_page_controls_update_real_tables` 通过，操作真实属性控件、来源重排、状态和 100k 示例创建／卸载 |
| Showcase 导航 | `sidebar::tests::categories_are_sorted_and_pointer_collapse_keeps_the_current_page` 通过，包含 Table 实际页面入口 |
| 消费者编译 | `cargo check -p windows-showcase`、`cargo check -p yue-minimal --bin table` 通过 |

复现命令：

```powershell
cargo test -p yue-component --features headless-tests --test table --test checkbox --test radio --test list --test list_grid --test virtual_list
cargo test -p windows-showcase --features headless-tests pages::table
cargo test -p windows-showcase --features headless-tests sidebar::tests::categories_are_sorted_and_pointer_collapse_keeps_the_current_page
cargo check -p yue-minimal --bin table
```

这些是生产组件上的 headless 布局／事件／语义和生命周期验证。没有运行三个网页或十个参考项目的测试；没有据此声明真实 Windows DPI、Narrator、IME、GPU 或显示帧率已验收。

提交前重新运行上述 250 项组件回归与 2 项 Showcase 测试，全部通过；`cargo clippy -p yue-component -p windows-showcase -p yue-minimal --all-targets --features headless-tests -- -D warnings` 通过。主题生成检查、Python 语法、Table 新增文件的 rustfmt 与 `git diff --check` 通过。全仓 `cargo fmt --all --check` 未通过，旧 HEAD 的 Checkbox／Field／lib 导出／Select／Showcase 等文件已存在同类格式差异；本阶段未批量格式化旧代码，不宣称该项全仓检查通过。完整差异输出随基线归档。

## 4. TB01 首份证据与采样状态

**完整记录见 [M1 headless 规模基线](../performance/2026-11-10-table-m1-baseline.md)**，包含可下载的原始证据归档、逐组结果、构建与计时边界。

工作集正确性对照：同一个 64rem×32rem 宿主、28rem Rows 视口、12 个文本列，1,000 行与 100,000 行在一次稳态绘制中均执行 **156 次 cell renderer**。大跳到 `row-99990` 后目标存在、`row-0` 卸载，正文 Cell 语义节点少于 600。该结果验证本场景工作集有界，不代表复杂单元格成本、全部内存或帧耗时恒定。

[TB01 探针](../../crates/yue-component/tests/table/benchmark.rs)与 [sample-table.py](../../tools/sample-table.py)在第四次采样 `target/table-m1/tb01-abba-04` 完成三组独立 ABBA：12 轮、每轮 120 预热＋1,000 有效帧，共 **12,000 帧**。1 轮因编译／Showcase 干扰被丢弃并重采；源／lock／二进制身份不变；每个接受轮次指定测试确实运行且通过，帧无缺失，大小表逐帧 renderer 数量完全相同，范围 156–192。A／B 表示同一实现的 1k／100k 行规模，不是代码优化前后。

100k／1k 的每轮分位数中位数比为 P50 **0.984**、P95 **0.995**、P99 **0.988**。但 1k 的 P50 轮间极差／中位数达到 25.56%，100k 为 5.31%，因此 P50 规模差异仍不确定，不能据此吸收优化或冻结 5% 正式阈值。**正确性、工作集和采样完整性通过；计时只建立初始基线，不宣称完整性能出口通过。**

前 3 次尝试分别被已有 Showcase 或其他项目并行编译阻止／打断，均未形成完整三组记录；未结束用户进程、放松门禁或将部分日志拼入第四次结果。旧日志保留在 `target/table-m1/tb01-abba-01` 至 `03`，交付归档只包含第四次完整实验及其废弃轮次。

当前简化文本 fixture 不包含完整 TB01 混合列与 W2 的 200 组首中尾／活动格回窗轨迹。CPU draw 不等于真实窗口 FPS；独立分配、GPU、输入到呈现、真实空闲和原生显示验收仍未覆盖。采样后的 Clippy 类型整理仅将表头字段的闭包类型写为已有等价 `Slot` 别名，归档保留采样时 model 文件与确切差异；相关测试重新执行，不把旧源码哈希冒充交付哈希。

## 5. 阶段边界

M1 基础功能、工作集与完整 headless 基线已交付，计时噪声与真实窗口验收如上单列。M2–M6 仍按主设计推进，不声称已有左右固定列／尾行、分组表头、伸缩、查询分页、展开树形、Cells／Measured、编辑或拖拽。

M1 的实际 API 精简了 Row 重复关联类型、无查询功能时的 TableDataMode 参数和不需要的 Window 参数。第 3 节最小消费者已同步；API 附录其余高级示例继续明确标记为草案，避免用未实现 API 写当前教程。
