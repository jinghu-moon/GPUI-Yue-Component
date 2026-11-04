# 虚拟滚动第三轮修复、回归与测量

日期：2026-11-04。对应[第三轮对标](2026-11-04-virtual-list-followup-review.md)的六项确认问题。本文记录生产修复及代价；原调研、28 条历史观察和此前改造记录不被覆盖。

## 1. 六项逐项修复

| 问题 | 生产修改 | 当前合同 |
| --- | --- | --- |
| P0 内容失效后同批 Splice 命中旧准备 | [state.rs](../../vendor/gpui-base/src/virtualization/state.rs) | 清理非空旧 index epoch 表时提升单调 base epoch；旧内容 stamp 不会复活 |
| P0 Masonry 预算截断可见项 | [masonry_element.rs](../../vendor/gpui-base/src/virtualization/masonry_element.rs) | 完整呈现工作集，动态额外校正才受预算约束；未完成项按估算槽位呈现 |
| P1 Masonry 未测项失效仍接受旧反馈 | [masonry.rs](../../vendor/gpui-base/src/virtualization/masonry.rs) | 成功非空失效总是推进 revision；空输入／失败不变，旧 snapshot 只读有效、回写过期 |
| P1 缓存误标实际测量约束 | [cache.rs](../../vendor/gpui-base/src/virtualization/cache.rs)、[updates.rs](../../vendor/gpui-base/src/virtualization/updates.rs)、[native list](../../vendor/gpui-pre/src/elements/list.rs) | 导出检查每条原生实际 cross，混合约束拒绝；恢复先验证，再降级旧约束尺寸；等主轴值的新 cross 更新仍受理 |
| P1 无位移 Smooth 误报 NoProgress | [state.rs](../../vendor/gpui-base/src/virtualization/state.rs) | 最终目标已确定且到达时直接提议终点，第一次正常确认成功；未测目标／拒绝宿主不伪完成 |
| P2 Grid 重复列元数据 | [grid.rs](../../vendor/gpui-base/src/virtualization/grid.rs) | 行顺序游标＋一份列元数据，source／几何按每轴准备一次；交点仍逐 cell 生成 |

### 1.1 内容准备身份与结构位置分别处理

局部失效仍使用区间边界，不为 N 项建立 key→epoch 表。清理非空边界表只触发一次保守的工作集刷新；随后没有新失效的连续追加继续复用。局部不相关行失效不会刷新第 0 行。结构位置变化仍由原有 index/key/source revision stamp 校验，不把全局 generation 永远加入每项准备身份。

代价明确：发生“已有局部内容失效＋结构事务”时，当前 W 项可能全部重新准备，而不是精确迁移每个区间。先保留简单、不碰撞的屏障；尚无性能证据要求加入复杂坐标迁移。

### 1.2 Masonry 预算的具体含义

一维 Measured 的原生 pass 预算不变。Masonry 改为：

1. 初始工作集 W0 的所有根正常布局，Fixed/Known 不消耗动态预算。
2. 合批提交真实尺寸，重新查询范围；额外暴露根的 intrinsic 测量最多 budget 项。
3. 最终工作集 Wf 中剩余项以固定估算高度、局部裁切的卡片呈现；不把这个槽位高度作为真实测量回报。
4. 只有真实尺寸让几何推进且仍有待测根，才请求下一帧继续；不跨帧缓存 AnyElement，也不在无效零高前缀上无进展自激。

因此每帧根布局上界是 `W0 + budget + Wf`，不是 `budget`。构建还包括可能已离开最终范围的校正项；几何重查询与单卡布局另计。密集 0.125px 有效项的测试实际呈现 2400 根，大于默认 1024，不能把预算当作静默缺项的容量限制。此时真实呈现成本本来就高，应由消费者决定呈现粒度。

估算 40px、真实 1px、低预算 1 的场景允许多帧逐步校正，不宣称固定一两帧完成。测试逐帧核对最终工作集与节点集合，验证能推进至当前工作集已测且不再调度。无效尺寸另验“不因反复错误而产生动画帧循环”，并保留去重诊断与删除焦点修复。

### 1.3 测量身份来自尺寸 owner

原生树已存储 `Size<Pixels>`。新增 `ListItemGeometry::cross_extent()` 只读取现有事实，横向布局投影后仍是规范化 cross；不加第二套尺寸缓存或全局 owner。`prepare(new viewport)` 不代表新约束下布局已发生，因此导出遇到旧 cross／混合 cross 时返回 `InvalidGeometry`。

恢复是冷路径：全批输入先验证；若已有测量含其他 cross，先保守地将已有真实尺寸整体降为 hints，再导入有效记录。导出／恢复的扫描继续属于冷路径 O(N)，没有搬到滚动热路径。同主轴尺寸在新 cross 下显式回报时，不能按“高度没变”提前跳过；相同约束下的重复更新仍去重。

Smooth 的完成判定只跳过已确定最终目标的无意义插值，使用 0.01px 到达容差；不将本帧 easing 的采样位置误认为最终目标，也不让动态未测项提前成功。

## 2. 升级前后测试

先把六个独立观察转换为生产回归，再运行旧实现；结果为 **0 passed / 6 failed**。原始失败记录：[before](virtual-list-followup-fixes-before-2026-11-04.txt)。失败分别是：

- 内容版本仍为 0，期望 1。
- 400 cells 的列 source 读取为 800，期望 40。
- 预算 2 的 Fixed Masonry 只有 2 个节点，期望完整 15 个。
- 未测内容失效后 revision 未推进。
- 旧 cross 的测量仍可导出为新 cross。
- 无位移 Smooth 经过 9 次确认后报告 NoProgress，期望一次 Settled。

最终复核还补出同类 owner 边界：Masonry 卸载但 state 仍存活时，旧挂载的 snapshot 曾能继续写入。新增用例先在修补前失败（实际 Ok，期望 StaleSnapshot），见[卸载失败记录](virtual-list-followup-unmount-before-2026-11-04.txt)。`release_mount` 现在仅在实际释放挂载时推进 revision；卸载和重挂均拒绝旧写入，只读冻结几何仍有效。纯 headless 未挂载用法不因此失效，也未新增生命周期 owner。

修复后新增正式断言位于：

- [followup.rs](../../crates/yue-component/tests/virtual_list/followup.rs)：六个复现门槛、三模式／操作顺序、删除／真实尾项换序／替换与后续追加复用、Masonry 可见集合与连续校正、超过默认预算的密集工作集。
- [followup_edges.rs](../../crates/yue-component/tests/virtual_list/followup_edges.rs)：两轴 cache 身份、新约束同尺寸、原生实际 resize 前后、原子非法导入、layout revision／零 cross、空 Grid、稀疏 retained／冻结几何、失效 owner guard、保留 state 卸载／重挂后拒绝旧测量、Nearest／clamp／微小差与真正宿主拒绝。
- [layouts.rs](../../crates/yue-component/tests/virtual_list/layouts.rs)：将旧“renderer ≤ budget”断言改为实际完整工作集，继续检查无效尺寸诊断和焦点删除；不保留错误合同。
- [lifecycle.rs](../../crates/yue-component/tests/virtual_list/lifecycle.rs)：保留一维 64 周期及故意保留 owner 的失败自测，新增交替 Grid/Masonry 共 64 周期（每类 32 次）挂载、滚动、换源、卸载、排空续帧，检查 state 与子 Entity 的 WeakEntity 均释放。
- [原生 virtualization.rs](../../vendor/gpui-pre/tests/virtualization.rs)：实际 cross 读取、等主轴尺寸跨约束更新、冻结几何及降级测量，共七项原生合同。

WeakEntity 断言证明这些路径不再持有相关 Entity，不等于所有进程 RSS／GPU 资源的通用泄漏证明。旧预算测试的合法测量续帧先排空，再独立检查无效尺寸 episode，避免把上个 episode 的帧回调误判为自激。

最终检查结果见下方执行记录；不把 ignored 的性能探针算作自动正确性通过。新增 Grid 探针已单独以 Release 显式运行。

## 3. Grid Release 时间与分配

测量 Windows x64 本机生产公共 snapshot API；Fixed／Known 各 100000 项轴源，20×20 与 50×50 个实际 cells，每样本枚举 1000 次，共五样本。交替先后顺序；同一 source、工作集、输入约束与坐标 checksum。旧路径仅作为测试内的原枚举策略保留，使用逐行克隆列索引、逐 cell 读取列项；不把旧算法留在生产中。两条路径均消费完整行列描述，避免优化器消除无关字段准备。

探针：[performance.rs](../../crates/yue-component/tests/virtual_list/performance.rs) 中 `followup_grid_metadata_evidence`。原始数据：[40 条 CSV](virtual-list-grid-followup-fixes-2026-11-04.csv)。时间是 1000 次纯元数据枚举的整批 P50，不是 UI 帧时间或 GPU 完成；分配器只追踪本测试线程的 payload，峰值不含业务源、持久轴树或框架渲染。

| 轴模式／工作集 | 旧枚举 P50 | 新枚举 P50 | 每轮分配次数（旧 → 新） | 临时 peak payload（旧 → 新） |
| --- | --- | --- | --- | --- |
| Fixed，20×20 | 14.7104 ms | 7.2874 ms | 22 → 6 | 480 → 2368 B |
| Fixed，50×50 | 90.1975 ms | 41.1180 ms | 52 → 7 | 1200 → 4896 B |
| Known，20×20 | 21.2020 ms | 7.1622 ms | 22 → 6 | 480 → 2368 B |
| Known，50×50 | 130.8677 ms | 40.6059 ms | 52 → 7 | 1200 → 4896 B |

这组负载的整批时间减少约 50%–69%，并非整个 UI 提速相同比例。取舍是多物化一份列项元数据，临时峰值增加约 1.8／3.6 KiB；它只活到枚举器释放，不随 N 保存。20×20 的列源调用从每轮 800 降为 40；50×50 从 5000 降为 100。rows 保持每轮 40／100 次，不牺牲 stable-key 反查验证。

旧策略对照保留原来的 flat_map／map 迭代器组合，而非改成不同优化机会的手写循环；新策略使用顺序行游标和单列向量，去掉了最初方案中每行的 Rc 包装。样本交替顺序，未丢弃首样本。它们是本机微基准，不是跨框架／跨语言跑分或任意业务的上界。

## 4. 执行与维护

| 检查 | 结果 |
| --- | --- |
| 改前六项生产回归 | 6 failed，均命中确认的问题 |
| 补充的卸载／重挂旧回写 | 修补前 1 failed，修补后通过 |
| 最终 virtual_list 专项 | 78 passed / 8 ignored / 0 failed |
| 完整工作区 headless＋doctest | 792 passed / 9 ignored / 0 failed |
| 原生 virtualization 合同 | 7 passed |
| Base virtualization 几何 | 2 passed |
| 工作区全 target Clippy，`-D warnings` | 通过 |
| windows-showcase Release 构建 | 通过；未启动真实 UI |
| workspace／vendor 格式、`git diff --check` | 通过 |
| 文档本地链接、40 条 CSV 与原始日志核对 | 通过 |

8 个专项 ignored 是显式手动性能探针；本轮相关的 Grid Release 探针已单独通过，其他探针沿用前轮证据，未冒充重新采样。完整工作区还包含其他模块已有的一个 ignored。

```powershell
cargo test -p yue-component --locked --features headless-tests --test virtual_list -- --test-threads=1
cargo test --workspace --locked --features yue-component/headless-tests,windows-showcase/headless-tests
cargo clippy --workspace --locked --all-targets --features yue-component/headless-tests,windows-showcase/headless-tests -- -D warnings
cargo fmt --all -- --check
rustfmt --edition 2024 --check vendor/gpui-base/src/virtualization/mod.rs vendor/gpui-pre/src/elements/list.rs vendor/gpui-pre/tests/virtualization.rs
tools/test-gpui-base.ps1 -Offline -Filter virtualization
cargo test --manifest-path target/gpui-virtual-upgrades-contract/Cargo.toml --offline --locked --target-dir target --test virtualization -- --test-threads=1
cargo build -p windows-showcase --release --locked

cargo test -p yue-component --release --locked --features headless-tests --test virtual_list followup_grid_metadata_evidence -- --ignored --nocapture --test-threads=1
```

原生 fixture 沿用前轮一次性清单；不修改 registry 或生产 Cargo.lock。完整执行日志位于 `target/virtual-followup-fixes-20261104/`，应长期保留的失败证据和性能数据已保存到本目录。补丁维护同步到 [Base](../../vendor/gpui-base/YUE-PATCH.md) 与 [native](../../vendor/gpui-pre/YUE-PATCH.md)。

本轮不启动、关闭或截图真实 Showcase，实际 UI 由用户检查。没有新实窗数据证明之前任务管理器 0.7%→4.5% 的 CPU 峰值已降低。Grid auto 轨道／冻结区域／精确 cell 保留、Masonry 更完整公共命令生命周期和原生大 offset 精度仍按具体消费者继续扩展；不把这六项修复说成所有参考库能力均已覆盖。
