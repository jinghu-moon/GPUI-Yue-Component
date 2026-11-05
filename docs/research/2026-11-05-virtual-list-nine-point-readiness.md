# 虚拟滚动工程成熟度提升（2026-11-05）

起点为 `1233387`。目标是通过可复现的改进接近自评 9 分，不把测试数量、头文件属性或一次 CPU 样本等同于完整产品验收。继续使用独立提交和前后证据；参考项目只读，生产 manifests／locks 不变。

## 验收顺序

1. **Grid 结构化语义**：逻辑总行列数、零基原生坐标、显式跨度和唯一 origin；在滚动／冻结／RTL 后仍正确。先补失败回归，读回实际 AccessKit 节点，消费者决定 role 和可读说明。
2. **span 配置复用**：只尝试显式 opt-in 的有界复用；源身份／generation、描述符身份与冻结边界失效必须正确。用同一完整 Grid workload、独立几何 oracle、两轮 Release ABBA 决定是否采用；默认路径不能隐式保留 N 级表。
3. **持续守卫**：CI 保持确定性的正确性／工作量合同，单独提供带数据完整性校验的性能任务和原始 artifact；共享 runner 的毫秒数不冒充固定硬件预算。缺失测试、空样本和故意退化的负控制必须被拒绝。
4. **原生复验与证据归档**：最终源码运行普通回归、vendor 合同、Clippy 与 Windows 原生探针；语义、CPU 工作、真实输入和无法覆盖的硬件条件分别记录。

## 1. Grid 结构化语义

新增生产页回归在旧接线下失败：Grid 的 `row_count()` 为 None，预期为 1,000,000。修复后逻辑行列总数、零基 cell 坐标、2×3 span、覆盖项省略、RTL 后相同逻辑身份及冻结角／最后 cell 均通过。普通 cell 的隐式跨度为 1。

GPUI 新增正跨度的 `aria_row_span`／`aria_column_span` 接线；普通 Div 只增加一个可空指针，只有显式 span 分配小 payload。Base 的观察器从同一个原生 Node 读取可选表格属性，不伪造测试属性，不为逻辑总数创建离屏节点。Showcase 复用源计数常量，补可读的位置／跨度说明。首次修复误调用了不存在的 state.len()，已改为消费者持有的源计数，不新增只为示例服务的 state API。

当前锁定的 `accesskit_windows 0.34.0` 未完整实现 UIA Grid／GridItem provider；结构化 AccessKit 数据不能直接宣称已经成为 Windows 原生表格导航能力。本轮不修改 Cargo registry、不扩充生产依赖，也不以文字说明冒充结构化跨度。真实读屏／桥接缺口在最终结论中保留。

定向 Showcase 回归 1 项通过，Showcase all-targets Clippy 通过；原生 vendor 合同 17 项通过，源码 manifests／locks 未变。合同首次编译发现 Div 必须先设置 id 才能调用 StatefulInteractiveElement 的表格方法，修正后通过。日志散列见 `virtual-list-nine-a11y-2026-11-05.json`。

## 2. span 配置复用

采用显式 `VirtualGridSpanSet`／`.span_set()`，保留 `.spans()` 默认路径。消费者在 render 外持有不可变描述符；克隆共享描述符和最多一份缓存解析。缓存键包含两个完整 EntityId、两个 source generation 与冻结前缀；每次验证轴合同，未命中完整验证后才发布。失败不会破坏最后一份良好缓存。仅存索引和范围，不持有源／controller／窗口／滚动 owner，不缓存 track 尺寸；被旧 frame 持有的解析快照可自然释放。

三项定向回归通过：32 次命中零源反查；冻结变化重验／非法配置原子拒绝；新实体同 generation 不复用；源删除拒绝缺失 origin；轴不支持的模式拒绝；源 reorder／Known 尺寸反馈／离屏 origin 的完整 cell 几何及点击目标与旧路径一致；释放 controller 后保留 set 不保活源。首版测试试图通过 replace_source 改 sizing 模式，被已有合同拒绝，测试已改为重建轴，未放宽生产限制。Showcase 的单个正文 span 在 view 初始化时构造 set，原生测试也将覆盖这一接线。

### 同一完整 Grid 的 Release ABBA

基线为 `e7b09cf`，依赖记录散列一致，工作负载分别针对前／后源码构建。每轮 ABBA 四个独立进程；16 个场景 × 5 个样本 × 4，共 320 行；暖帧每样本 64 次 notify＋draw，冷启动每样本 16 次等价描述符复制＋首次配置＋释放。暖帧有独立 BTreeSet oracle 并逐格核对尺寸，保持实际 workset 相同。预热首帧不计，计时期间没有修改源或运行其他 Cargo 构建。

初次 `span-reuse-a` 的冷启动释放时点不一致，320 行原样保留并排除采用判断；初版 workload 按原始 hash 复原保存。修正后 `span-reuse-b/c` 两轮数据完整且均通过检查。下表是各轮 10 个样本的完整暖帧 CPU 总时长中位数变化，不是配置单项收益或 GPU FPS。

| 分布／跨度数 | B 轮变化 | C 轮变化 |
| --- | ---: | ---: |
| 分散／0 | −2.0% | +0.8% |
| 分散／100 | −3.5% | −4.6% |
| 分散／1,000 | −23.6% | −21.9% |
| 分散／10,000 | −75.9% | −74.3% |
| 同排／0 | −2.2% | +1.7% |
| 同排／100 | −11.2% | −7.1% |
| 同排／1,000 | −63.2% | −62.6% |
| 同排／10,000 | −95.5% | −95.5% |

冷启动不是免费：非空集合变化为 −2.3% 至 +9.9%；10,000 分散为 −0.1%／+1.8%，同排为 −1.1%／+0.3%。空集合首次配置 +9.4% 至 +22.5%，但每 16 次配置只增加约 0.0008–0.0019ms。额外持有描述符与解析索引均为 O(S)，分配次数／峰值本轮未测，不能宣称零内存开销。

另跑 `span-default-a`，两边均调用旧 `.spans()`：暖帧 −1.5% 至 +3.3%，冷启动 −4.3% 至 +3.6%，未观察到同规模实质退化；并不据此宣称严格零开销。默认路径没有新增 retained cache／Rc 分配，非空 GridOptions 的 enum 包装仍有布局成本。

原始 CSV、独立 identity 与完整汇总见 `virtual-list-series-span-{reuse-b,reuse-c,default-a}-2026-11-05.csv`、`virtual-list-nine-span-evidence-2026-11-05.json`。普通切片适用于少量／频繁改变的 descriptors；大量稳定 span 才显式持有 set，不能每次 render 重建来期待缓存收益。

## 3. 持续守卫

待实现并运行负控制。

## 4. 最终验证

收尾时记录实际运行结果；不沿用上一轮测试计数作为本轮通过证明。
