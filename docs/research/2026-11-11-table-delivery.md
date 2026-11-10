# YueTable 分阶段实施进度

用户授权逐项完成全部阶段；每阶段实现、功能回归和相应性能验收通过后，提交并推送，再进入下一阶段。Git 中独立保留 Base 行为补丁与组件接入，便于回退与上游升级。

| 阶段 | 状态 | 任务与证据 |
| --- | --- | --- |
| M1 基础表格 | 已提交／推送 `543fc6d` | [实施记录](2026-11-10-table-m1-implementation.md)、[规模基线](../performance/2026-11-10-table-m1-baseline.md)；计时噪声与原生验收边界保留 |
| M2 共享布局与固定 | 功能与工作集验收通过 | 列约束、响应式恢复、左右固定、首尾固定行、分组表头、伸缩与键盘取消；[实际 API](../45-table-m2.md)、[性能基线与限制](../performance/2026-11-11-table-m2-layout.md) |
| M3 查询和数据流程 | 待实现 | typed 查询、本地与远程适配、排序／筛选、Pagination、summary 与批量选择；TB04 完整结果 oracle |
| M4 复杂记录 | 待实现 | Measured、详情／树／节点加载、真实控件编辑与版本会话；TB03／TB06 |
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
