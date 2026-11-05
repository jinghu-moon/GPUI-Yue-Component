# 虚拟滚动下一项工作准入（2026-11-05）

本记录用于防止为了提高自评分数而增加没有实际消费者的能力。判断依据是当前仓库调用方、三个参考项目的共同机制、现有回归证据和设备条件；`refer/virtual` 只读。

| 候选 | 当前决定 | 重新打开的触发条件 |
|---|---|---|
| 一维横向 RTL | 暂不实现 | 出现真实横向 RTL 调用方，并能提供显式 direction 配置；同时覆盖物理／逻辑 offset、scrollbar、wheel、命中、焦点和语义顺序 |
| 完整 reverse layout | 暂不实现 | 出现聊天／时间线等需要“底部起步、持续 prepend”的真实页面；现有 `Splice` stable-key 锚点已覆盖位置补偿，但不等同于 reverse layout |
| Masonry 横向主轴 | 暂不实现 | 出现横向瀑布流页面；需要独立主轴几何、滚动 owner、命中和 reflow 合同，不能把纵向实现旋转复用 |
| Grid 观察尺寸公共 collector | 不吸收 | 出现多个消费者共享同一 epoch／预算合同；当前消费者可直接按轴提交 Known 尺寸，通用 collector 仍会增加状态和失效复杂度 |
| Windows UIA Grid/GridItem/Realize | 不并入本分支 | 完成独立 COM provider 协议和真实 Narrator 验收；仅升级 AccessKit 依赖不足以关闭缺口 |
| 跨 DPI／多显示器 | 等待设备 | 获得第二显示器或不同 DPI 环境后，复用同一轨迹验收锚点、焦点、尺寸反馈和恢复 |

当前继续有价值的工作是维护已有合同：稳定 key 的 prepend 锚点、Grid 双轴原子尺寸、冻结／span／RTL 命中、Masonry 活动和尾部跟随，以及固定 workload 的性能尾部复测。没有满足触发条件的候选时，保持 API 和默认路径不变比增加一个“看起来完整”的开关更可靠。
