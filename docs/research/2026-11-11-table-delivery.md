# YueTable 分阶段实施进度

用户授权逐项完成全部阶段；每阶段实现、功能回归和相应性能验收通过后，提交并推送，再进入下一阶段。Git 中独立保留 Base 行为补丁与组件接入，便于回退与上游升级。

| 阶段 | 状态 | 任务与证据 |
| --- | --- | --- |
| M1 基础表格 | 已提交／推送 `543fc6d` | [实施记录](2026-11-10-table-m1-implementation.md)、[规模基线](../performance/2026-11-10-table-m1-baseline.md)；计时噪声与原生验收边界保留 |
| M2 共享布局与固定 | 已提交／推送 `20bbbf2`、`1d31808` | 列约束、响应式恢复、左右固定、首尾固定行、分组表头、伸缩与键盘取消；[实际 API](../45-table-m2.md)、[性能基线与限制](../performance/2026-11-11-table-m2-layout.md) |
| M3 查询和数据流程 | 已提交／推送 `f7651dd` | typed 查询、本地／远程、排序／筛选、Pagination、三种汇总范围、批量选择；[实际 API](../45-table-m3.md)、[TB04 基线](../performance/2026-11-11-table-m3-query.md) |
| M4 复杂记录 | 实现与自动化／性能基线通过 | Measured、详情／树／节点加载、真实控件编辑与版本会话；TB03／TB06 |
| M5 双轴与高级结构 | 待实现 | Cells、Known、span、跨区表头、宿主 affix、范围加载；TB02 |
| M6 重排与交付 | 待实现 | 行列拖拽、键盘等价操作、列配置草稿、综合示例、完整合同核对 |

## M2 自动化

`cargo test -p yue-component --features headless-tests --test table --test virtual_list` 通过：Table 23 项、VirtualList 193 项，性能探针默认忽略。新增布局测试验证 TC05／TC06／TC09／TC15：共享轨道、浅层叶表头跨带及零基语义、End pin 窄容器解除与恢复、固定首尾行和单按钮命中、松手最后坐标、键盘连按到释放一次提交、宿主拒绝恢复、Escape 零提交、异步宿主接受前显示已提交宽度。

Showcase 页面交互通过，release 下 23 项 Table 测试通过。两种构建均真实执行，不能把 dev 耗时与 release 直接比较。源码格式及 Base 补丁格式检查通过；全仓历史格式差异沿用 M1 已记录边界，不批量修改无关文件。

最终 Clippy（组件／Showcase／最小消费者的 all-targets、headless feature、`-D warnings`）、Showcase 页面复测及 Checkbox／Radio／List／ListGrid 的 41 项近邻回归均通过；主题生成检查、Python 语法与 `git diff --check` 通过。

采样前复核修复了一个异步合同：伸缩结束立即清理预览，延后接受时显示旧值；新测试先断言拖动预览，再断言释放恢复，最后断言宿主接受新值。第一批三轮完整日志及第四轮未完成日志保留在 `target/table-m2-evidence-01`，停止本实验自有 runner 后重新构建与采样，旧轮次不拼入接受样本。

性能场景为 M2 Rows 布局组合，非尚未实现的完整 TB02：12 文本列、分组表头、Start／End 各一列、首尾数据行各一行、固定列 footer；48／64／80rem 宽度循环与确定 XY 位移。1k／100k 三组 ABBA，120 预热及每轮 1,000 有效帧。容器反馈收敛在 draw 计时之外，不能宣称测到了完整 resize 输入延迟。固定区域和每帧工作集必须一致，runner 保留原始日志与身份。

第二次完整采样 12,000 帧已通过，逐帧工作集在各轮完全一致，132–168 次 renderer，低于公式预算 240。P50 规模比 1.004；P95 噪声和第三组 P99 诊断线问题如性能报告单列，不宣称吸收了帧优化。

[M2 原始证据](../performance/evidence/2026-11-11-table-m2-layout.zip) SHA-256：`345ba10135ee65488f755c661aa46a2437f6f2e18c7e4c7f5ec084b726097e0a`。归档包含逐帧 CSV、12 轮日志、构建身份、源码与 diff、功能检查和独立复核脚本；归档前确认交付源码与采样源码 SHA-256 完全一致。

当前设备未进行 GPU／真实 IME／Narrator／跨屏 DPI 验收；这些不由 headless 结果替代。


## M3 自动化与证据

M3 扩展生产 Table 查询模块，复用现有 Dropdown、TreeSelect、DatePicker、Input 和 Pagination。两个独立最小消费者仅依赖 yue-component；API 附录改为实际可编译程序，不保留原草案中的伪签名。TableLocalQuery 在更新路径运行，TableRemotePage 只提供返回页和明确 known key 域。

最终 dev 与 release Table 均 29 passed、3 opt-in ignored；Showcase Table 4 passed。VirtualList 193 passed、11 opt-in ignored；严格 Clippy（组件／Showcase／最小消费者 all-targets、headless、-D warnings）通过。主题生成、Python 语法和 diff-check 通过。滚动、选择、rem／主题更新不会重跑查询谓词；真实普通／树筛选、自定义草稿取消与确认、分页跳页草稿保留、批量核验及远程迟到／失败／重试通过。

验收中修复多排序方向切换改变优先级、TreeSelect 遗漏名称、同名规则变更后旧票据、以及页外选择干扰表头三态。页内选择计数在接受时更新，绘制不扫描完整集合；汇总复用一次分页前查询，不为每个范围重复排序。源替换仍释放历史并使旧票据失效。

TB04 为 250,000×8，11 条件×6 次×3 独立进程，共 198 个有效样本。完整 key 次序／total／页范围在每次计时外逐条核对，独立验证 CSV、日志哈希和总数 oracle。第二轮第一次尝试被运行中进程门禁整轮弃用，日志保留，不拼接。通过正确性、身份与零滚动重算预算；时序是新增功能基线，没有优化前后收益结论。宽结果两列排序 query 中位数约 407 ms，排序约 317 ms，是后续性能优化的真实入口，不隐藏这项成本。

[原始证据归档](../performance/evidence/2026-11-11-table-m3-query.zip) SHA-256：`4fea9600f79f0492018acf1d9e567e150917dcf2466096647d16209839d59105`。包含完整样本、接受和弃用日志、源码／构建／可执行文件身份、功能检查与独立验证脚本；归档前交付 Rust 源码与采样 digest 一致。原生 IME、Narrator、GPU、跨屏 DPI 仍未验收，不由 headless 替代。

## M4 自动化与证据

Rows Known／Measured、TableOutline 树／详情／节点加载及真实 Input／Select 编辑会话已接入；复用 Base KeyedTreeModel、Grid Known 几何、Descriptions、Form／Field。Base 增加稳定同级排序和加载票据预检查，组件保持独立提交边界。

Table dev／release 37 passed、5 opt-in ignored；VirtualList 193 passed、11 opt-in ignored；Showcase Table 10 passed。严格 Clippy 对组件、Showcase、最小消费者全目标、headless、-D warnings 通过。真实输入覆盖立即 Enter／Tab、失败重试、行多字段、离屏草稿、外部冲突与删除焦点；树 UI 覆盖根分页、父子级联、不完整节点拒绝、失败重试、两详情独立滚动及关闭释放。

TB03／TB06 为新增能力的规模基线，非优化前后对照。采样保持三组 ABBA、120预热及每轮1000有效帧，线程内分配在计时结束后独立归因。TB03 明确持有512张64×64 RGBA本地生成资源，像素池8MiB；GPU纹理驻留尚未测量。采样期间观察到的其他 cargo 编译导致整轮弃样，原日志保留，不结束外部进程，也不以轮次拼接凑足样本。

TB03和TB06各12000有效帧，逐帧工作集在规模与重复间完全一致，P50规模比1.0007／1.0019。冷挂载树索引成本随规模增长，分配成本与原生验收边界见 [M4性能报告](../performance/2026-11-11-table-m4-complex.md)。

M4 [证据归档](../performance/evidence/2026-11-11-table-m4-complex.zip) SHA-256：`ddc6c1b26404a3a70545b3b6dfb2672e5610892230d11fb5794bd5ce521bc0e9`；交付源码与两组采样源码 digest 一致。
