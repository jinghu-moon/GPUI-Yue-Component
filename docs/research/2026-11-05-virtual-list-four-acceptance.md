# 虚拟滚动四项验收（2026-11-05）

本轮按观察尺寸消费者、组合回归、大量 span、Windows 实窗依次验证。每项独立提交；参考目录只读，生产 manifests / locks 不变。当前公共能力仍以[组件文档](../28-virtual-list.md#26-布局选择与二维边界)为准。

## 1. Grid 观察尺寸的等价消费者

已完成，保留私有消费者案例，没有新增 `VirtualGridObservedSizes` 公共 API。源码为 [grid_observed.rs](../../crates/yue-component/tests/virtual_list/grid_observed.rs)，计时入口为 [performance.rs](../../crates/yue-component/tests/virtual_list/performance.rs) 的 `grid_observed_equivalent_evidence`。

两个实现使用相同 `VirtualGridCellKey`、有限 track 域、唯一 cell 预算、默认尺寸、完整输出及批量原子校验。Cells 路径保存 cell 最大值再按轴聚合；Axes 路径保存已见 key 和轴最大值。同 epoch 只增长；显式新 epoch 清空样本并恢复默认，允许收缩；过期反馈、缺失 key、非法几何、超预算整批拒绝。业务内容、源域或布局改变需要消费者开启新 epoch（域改变还需重建域索引）。这不是内容 intrinsic layout，存储为 O(R+C+budget)，不适用于无元数据的百万轨道默认路径。

借鉴 TanStack 的完整输出等价及独立 oracle 思路，普通测试用独立历史扫描核对全部行列，覆盖 1/7/64 轨道、重复反馈、三个 epoch、缩小及原子失败。实际 Known 两轴消费者测试覆盖增长、收缩、阅读 key 相对位置和迟到快照拒绝。每轴分别原子，不声称双轴联合事务。

两次 Release / offline / locked 运行，每种场景按 Cells–Axes–Axes–Cells 两轮执行，所有样本及源码身份见[机器记录](virtual-list-four-acceptance-observed-2026-11-05.json)。每个样本 40 轮、64×64 cells；warm/reset 各轮含 16 帧稀疏反馈，warm 保持同 epoch，reset 复用容量并开始新 epoch；warm 的首轮初始化也计时。两路径都校验全批、生成完整有序向量、消费所有值，计时外逐帧断言相等。

| 场景 | Cells 全部样本 | Axes 全部样本 |
| --- | ---: | ---: |
| 冷启动 | 34.28–35.79 ms | 32.86–37.24 ms |
| 跨帧复用 | 99.29–102.51 ms | 45.62–52.98 ms |
| epoch 重置并复用容量 | 105.77–117.37 ms | 51.09–54.44 ms |

峰值暂存／保留 payload 为 Cells 1,180,496 B、Axes 1,082,192 B；相同场景分配次数相同。冷启动 Axes 的 37.24ms 不利样本保留；不能宣称所有场景都更快，更不能换算成 FPS。新结果修正[旧微实验的非等价限制](2026-11-05-virtual-list-next-opportunities.md#grid-已观察尺寸-epoch-候选不吸收2026-11-05)，不覆盖或删除旧证据。

决策：吸收“消费者按轴保存最大值、显式 epoch 失效”的案例与回归，暂不提升为 Base API。这里的有限域索引、默认值、失效时机属于应用合同；没有第二个真实消费者证明通用边界。无需为该实验增加 Grid 默认路径缓存。

验证：3 个普通回归通过；等价 Release 手动探针两次各 1 passed；目标 Clippy `-D warnings` 通过。

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list observed_consumer --offline --locked -- --nocapture --test-threads=1
cargo test -p yue-component --features headless-tests --test virtual_list grid_observed_equivalent_evidence --release --offline --locked -- --ignored --nocapture --test-threads=1
```

## 2. 多能力组合回归

已完成，新增 [grid_combinations.rs](../../crates/yue-component/tests/virtual_list/grid_combinations.rs) 的生产组件组合轨迹。32 次挂载交替 LTR/RTL；同一个 Grid 同时开启冻结首行／列、2×2 span、稀疏保留及远端聚焦 editor，然后交错双轴定位、Known 冻结尺寸反馈、resize、源重排、50ms 动画采样、真实 `ScrollWheelEvent` 取消动画、撤销焦点注册、带待执行动画和 idle timer 卸载。

每轮 5 个阶段分别通过独立线性几何 oracle 检查四区共 20 次原生指针命中（合计 640 次），不是仅检查 renderer 调用数；语义 cell key 全部唯一，工作集小于 120。锚点检查使用 stable key 到冻结 body 边缘的相对位置；重排后未变 key 的编辑内容仍保留，变动索引的旧内容释放。每轮卸载后两轴及所有 cell view 的 weak 引用失效。第一轮故意保留一个强引用作负控制，确认它仍存活，再释放并验证归零。

借鉴 TanStack 的命令／尺寸／重排／frame 混合轨迹、virtua 的阅读锚点、vlist 的 scoped 生命周期和真实 hit 断言，没有运行参考项目的浏览器测试来代替 GPUI 测试。初始化类型／借用编译错误均已修正；初次运行还暴露测试自身两个错误假设（点击会正常移走焦点、未变内容身份应复用），已改成在指针输入前检查焦点，并只要求变动身份释放。最终组合测试 1 passed，目标 Clippy `-D warnings` 通过；生产代码无需修改。

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list mixed_grid_trace --offline --locked -- --nocapture --test-threads=1
```

## 3. 大量 span 成本

待本项验证完成后记录。

## 4. Windows 实窗验收

待本项验证完成后记录；真实窗口、headless CPU 和无法覆盖的硬件条件分别记录。
