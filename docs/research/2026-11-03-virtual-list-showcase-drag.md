# VirtualList Showcase 扩展与滚动条拖动采样

日期：2026-11-03。对应 [设计](../28-virtual-list.md)、[基础层实施记录](2026-11-03-virtual-list-implementation.md) 与 [Showcase 源码](../../examples/windows-showcase/src/pages/virtual_list.rs)。本轮只改演示页及其测试，没有根据未采集的实窗调用栈改动虚拟器算法。

## 1. 用户观察与测量边界

用户观察：按住滚动条快速拖动时，任务管理器中的 Showcase CPU 从约 0.7% 升至最高约 4.5%。本轮通过只读进程信息确认，当时运行的路径为 `target/debug/windows-showcase.exe`。根 Cargo 的 dev profile 仅关闭 debug symbols，没有设置 opt-level；不能把这个观察直接当作 release 产品的 CPU 结论。

依此前记录的 20 个逻辑处理器简单折算，4.5% × 20 ≈ 0.9 个逻辑处理器的占用份额；这不是单线程饱和、实际卡顿、GPU 忙碌或百万项全部重建的证据。仍需要帧节奏与热点证据建立因果。本轮不启动、关闭或操纵真实 Showcase，也不对用户观察的 4.5% 宣称已复现或已降低。

## 2. 八个场景

| 标签 | 内容 | 可操作行为与边界 |
| --- | --- | --- |
| 百万记录 | Fixed，1,000,000 项，36px 行高 | 首尾定位、头插锚点、短距离平移 |
| 分组记录 | Known，10,000 项，52/36px 混合高度 | 组标题和普通记录、首尾定位；没有声称 sticky 分组头 |
| 动态卡片 | Measured，10,000 项，自然换行 | 首卡展开/收起、宽度变化后的测量、首尾定位 |
| 横向时间轴 | Known / Horizontal，500 项，144/208px 混合宽度 | 原始 Base VirtualList、横向输入与滚动条，不新建横向 Yue 状态 |
| 多列卡片 | Fixed，3,000 行，每行 3 张，共 9,000 张 | 按行虚拟化；按卡片身份保留选择；不是单元格级双轴 Grid 或瀑布流 |
| 编辑保留 | Measured，1,000 项 | 首行一个真实 Input Entity；输入值跨离屏和标签切换保留，聚焦时稀疏保留该行 |
| 追加日志 | Measured，初始 200 项，文字高度不同 | 手动追加 20 条、跟随最新、暂停跟随；查看历史时不抢走阅读位置，无背景定时器 |
| 空状态 | Fixed，0 / 1,000 项 | 加载、清空与重新加载；空列表 renderer 为 0 |

使用现有 YueTabs（默认 lazy），只挂载当前场景；八份 controller 与业务状态保留。此前三个列表同时挂载，现在不让其它演示与当前拖动同时构建。这个结构变化有确定性计数测试，但没有前后实窗 CPU 百分比对照，不能量化其对 4.5% 的影响。

行高、节点宽度、gap 和 overscan 的明确像素值用于可重复的虚拟几何样例；普通布局、控件与 Measured 内容使用相对尺度。每个场景提供显式“读取统计”，显示可见范围、工作范围、主轴总长、估算状态和累计 renderer 次数；不按帧更新整个页面统计，也不将它称为 FPS。

## 3. 自动化回归

改动前，原 Showcase 虚拟列表流程测试 1/1 通过，保留原行为后将其扩展为以下合同：

1. 三种原模式、百万项末尾、头插锚点、动态首卡展开。
2. 八场景只挂载当前列表，稳定帧不构建其它场景；切换后滚动位置保留。切换点击的 pointer-down 仍可能提交旧场景，测试不把这个正常过渡算作后台构建。
3. 横向 wheel 与首尾定位。
4. 九千张卡片的选择跨离屏与场景切换保留。
5. 真实 Input 的输入值与离屏 focus 保留，没有为每条记录创建 Input。
6. 日志追加时跟随尾部、上滚退出跟随、历史阅读位置保留、重新跟随。
7. 空数据不构建，清空和恢复后几何正确。
8. 窄窗口、深色主题下的真实标签键盘 Home/End 路径。
9. 原生滚动条实际按下、连续快速移动与释放：Fixed / Known / Measured 都能跨过列表中点；每批构建和工作项有界，其它场景计数不变。

九项均由真实生产页与组件组成的无头交互测试覆盖。另有一个默认 ignored 的 release CPU 探针，不用不稳定 wall-clock 阈值作为普通 CI 的通过条件。测试先通过真实 wheel 活动显示 Scrolling 模式滚动条，再按实际 thumb 内部位置拖动，不以直接改 offset 冒充原生拖动。

验证命令：

```powershell
cargo test -p windows-showcase --locked --features headless-tests
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
cargo fmt --all -- --check
cargo test -p windows-showcase --locked --release --features headless-tests showcase_scrollbar_drag_cpu_probe -- --ignored --nocapture --test-threads=1
cargo build -p windows-showcase --locked --release
```

最终结果：虚拟滚动页 9 passed + 1 ignored；完整 Showcase 27 passed + 1 ignored；Yue 虚拟滚动基础合同 28 passed + 3 ignored；工作区 all-targets / headless Clippy、格式检查与生产 release 构建均通过。release 拖动探针连续三轮各 1/1 通过，debug 对照一轮 1/1 通过。Showcase 扩展后的结果与基础层实施时的全量工作区结果分开记录；本轮没有重跑整个工作区全部 headless 测试，也没有通过修改断言掩盖原 ImagePreview 的独立失败。

release 产物为 `target/release/windows-showcase.exe`，仅构建、未启动。用户原来打开的 debug 进程没有被替换或关闭。

## 4. 采样与结果

[原始 CSV](virtual-list-showcase-drag-2026-11-03.csv) 共 3,600 行：release 连续三轮各 900 条，debug 一轮 900 条。每轮五个场景，每场景 180 次快速往返 pointer-move；前 30 次保留为 warm-up，统计使用后 150 次。release 每组 450 样本，debug 每组 150 样本。

环境沿用基础层记录的 Windows x64、i5-13600KF、rustc 1.98.1；真实 Showcase 页面置于 Base Root，无头窗口 1400×1000，列表外框 1352×352 逻辑像素，overscan 72px，系统 reduced motion 打开。包含 Tabs、按钮、页内说明和实际行内容；不是只有几何查询的微基准。Measured 使用默认收起的真实换行卡片。横向、多列和编辑等新类型走功能测试，未把这五组时序外推给所有类型。

计时从发出一次真实 thumb pointer-move 开始，到显式 draw/scene clear、执行到 parked 结束；不包含随后读取快照和写 CSV 的工作。这是输入/CPU 提交批次，可能包含事件产生的自动提交与显式提交，不是物理显示帧，也没有隔离每个框架阶段。renderer 是条目根构建数，不能代表子元素数量或完整布局次数。

### Release：三轮合并

| 模式 | 项数 | P50 / P95 / P99 批次 (ms) | 根构建中位数 / 最大值 | 最大工作项 |
| --- | ---: | --- | --- | ---: |
| Fixed | 1,000 | 0.7178 / 0.9083 / 1.1054 | 15 / 15 | 15 |
| Fixed | 100,000 | 0.7158 / 0.9391 / 1.0826 | 15 / 15 | 15 |
| Fixed | 1,000,000 | 0.7129 / 0.8843 / 1.2229 | 15 / 15 | 15 |
| Known | 10,000 | 0.6558 / 0.8486 / 1.0509 | 14 / 14 | 14 |
| Measured | 10,000 | 0.7000 / 0.8858 / 1.0605 | 9 / 10 | 10 |

在当前固定视口、行复杂度和输入脚本下，Fixed 从 1K 到 1M 没有出现随 N 增长的根构建或明显随 N 增长的批次耗时。Measured 每批根数较少，因为每张卡片更高；不能据此宣布它比 Fixed 更快。相比基础层 240×100 简单 fixture 的约 0.08ms，这里包含更大视口、真实页面和输入，不是同等条件下的性能倒退对照。

### Debug：一轮对照

| 模式 | 项数 | P50 / P95 / P99 批次 (ms) | 根构建中位数 / 最大值 | 最大工作项 |
| --- | ---: | --- | --- | ---: |
| Fixed | 1,000 | 11.8684 / 13.0349 / 14.0260 | 30 / 30 | 15 |
| Fixed | 100,000 | 11.8671 / 13.3523 / 15.8922 | 30 / 30 | 15 |
| Fixed | 1,000,000 | 11.6904 / 14.0915 / 18.5072 | 30 / 30 | 15 |
| Known | 10,000 | 10.8345 / 13.1404 / 16.3981 | 28 / 28 | 14 |
| Measured | 10,000 | 11.2812 / 15.6228 / 22.5614 | 16 / 16 | 10 |

Debug 构建的批次更慢，且根构建计数显示额外提交周期。原生滚动条根据真实 elapsed time 合并通知，测试又显式提交一轮；本对照因此同时包含构建配置、框架检查与提交节奏差异，不能把两表相除当作单帧编译加速比。Measured 的真实尺寸逐步收敛、条目高度不同，也会影响每批工作集。CSV 原样保存这些差异，没有人为除以二“校正”。

## 5. 可以得出什么

- 快速 thumb 拖动已通过真实输入验证，没有发现遍历或构建全部逻辑条目的迹象；当前热点不是由“百万项就逐项渲染”解释。
- Showcase 改为仅挂载当前场景，是本轮明确减少同时工作量的结构变化；实际 CPU 收益尚未在实窗测量。
- 当前可量化的是 CPU 提交批次与工作项，而非 Task Manager 的真实进程占用率、DirectWrite、D3D11、GPU 时间、VRAM 或显示帧节奏。不能由这些无头数值宣布已解决或精确解释 4.5%。
- 本轮没有实窗 CPU 调用栈，不将字符串格式化、Taffy、字体或 GPU 中的任一项武断指定为热点。继续优化应在相同 release 构建、窗口尺寸和拖动轨迹下采集热点，再决定缓存、通知或绘制修改；实窗 UI 检查仍由用户负责。
