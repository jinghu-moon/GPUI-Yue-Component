# 虚拟化五项后续实施（2026-11-05）

本轮起点 `7fb2c29`；按冻结组合、稀疏 cell、span、Measured smooth、auto/content/RTL 顺序实施。参考代码只读，生产依赖清单和 lock 不变；每项独立提交。headless draw/CPU 不是 GPU FPS 或输入延迟。

## 1. 冻结组合与成本

新增 24 轮连续双轴 Known 反馈、视口 resize、冻结切换、命令与真实 wheel 接管回归，并检查四区内部 row-major 绘制顺序。冻结专项旧实现与候选均 10 项通过。已有 source 变更、正文消失、焦点和准备生命周期测试继续保留。

原冻结路径收集全部 R×C cells 后稳定排序。改为只暂存 R+C 轨道，分割冻结前缀，以四个矩形按原顺序流式遍历。普通未冻结路径不增加轨道 buffer。

用固定 [workload](virtual-list-frozen-anchor-2026-11-05.rs)、基线 `7fb2c29` 两次 Release ABBA。每版每场景 6 samples，sample 为 160 个计时 draw 周期，中位数 CPU 变化如下，负数为改善。

| 场景 | 第一轮 | 复测 |
| --- | ---: | ---: |
| 未冻结 steady | +0.92% | +0.77% |
| 未冻结 scroll | +1.80% | -2.24% |
| 未冻结 prefix | -3.71% | +0.45% |
| 未冻结 hidden | -0.68% | +0.77% |
| 冻结 steady | +1.72% | -1.72% |
| 冻结 scroll | -1.28% | -3.21% |
| 冻结 prefix | +1.06% | -1.14% |
| 冻结 hidden | -2.62% | +0.10% |

冻结四场景分配均减少 960，窗口内增量 peak 均减少 3328 bytes。所有场景 reads/builds、reading_error 完全一致（误差均 0）；非冻结分配及 peak 完全相同。按有确定证据的空间收益吸收，不声称 CPU 普遍加速或零回归。

完整不利样本见 [第一轮](virtual-list-series-frozen-stream-2026-11-05.csv) 和 [复测](virtual-list-series-frozen-stream-repeat-2026-11-05.csv)。重现：`python tools/compare-virtualization.py --before 7fb2c29 --label frozen-stream --date 2026-11-05 --workload docs/research/virtual-list-frozen-anchor-2026-11-05.rs --filter frozen_anchor_workload`；复测替换 label。隔离构建 identity 和编译静默检查由比较器记录在 `C:/Temp/gpui-yue-series-20261104/` 对应目录。
