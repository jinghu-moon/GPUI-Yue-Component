# 虚拟滚动第二轮实施与测量

后续状态：[第三轮对标](2026-11-04-virtual-list-followup-review.md) 在本轮升级后的实现中复现六项组合／扩展布局问题。本文保留本轮已完成的改造与测量结果，不代表后续发现已经修复；当前待修门槛见主设计文档第 16.1 节。

日期：2026-11-04。依据 [跨库对标](2026-11-04-virtual-list-cross-library-review.md) 的优先级，先修正确性，再处理内容准备、结构更新和几何内存；不新增另一套滚动／测量 owner。本文取代前轮记录中的当前限制描述，历史数据不改写。

## 1. 实施范围与合同变化

| 项 | 当前实现 | 主要证明 |
| --- | --- | --- |
| 三模式内容失效 | Fixed/Known/Measured 均记录显式内容 epoch；准备缓存读取相应 index 的局部 epoch；旧 snapshot 的迟到写入拒绝 | 旧实现 3 项失败回归，新实现通过；同高度内容也更新；无关行保留原 view |
| 定位策略失效 | scroll padding 改变撤销未确认的请求身份，下一次 prepare 计算新目标 | 200→185；旧请求拒绝，实际尺寸与只读几何仍有效；overscan-only 不撤销请求 |
| 内容／几何准备分离 | `new_with_item_views` factory 改收 `VirtualizerItemIdentity`，只有 index/key | 上方行从 20→40px，visible 仍 50..55，factory 从 5→0；layout revision 不重做内容准备 |
| 原生增量 splice | `ListState::splice_with_size_hints` 先验证整批，再保留前缀／后缀的原生树和焦点状态 | 10000 项 append 1 项，估算调用 10001→1；非法新增 hint 无部分提交；末项无多余 gap |
| Masonry 完整替换 | 顺序读取旧尺寸，按 key/content 迁移真实测量；尺寸和 measured 标记一次打包到树 | 移除临时 state、二次 pack、每个未测项单独树更新 |
| Masonry 增量追加 | `append_source(next,sizing)` 沿当前列末端追加，按列合批扩展几何 | 原分栏／测量／旧快照保留，只评估新增 hints；失败不修改状态 |
| 紧凑持久索引 | B=128 的紧凑叶块＋已有 SumTree；批量更新每个触及块只复制／累计一次 | 百万项 Known retained payload 31,274,376→12,433,856 B；冻结快照下点更新仍为 KB 级 |
| 顺序工作集 | `snapshot.items(range)`、`working_items()`；Base 算术路径的呈现使用后者 | 连续区间一次 seek；跨叶块正确；远端 retained 不遍历中间行 |
| 验证机制 | 固定 seed 的混合操作 oracle；64 周期释放 gate；故意保留 owner 的 gate 自测 | 覆盖两轴、Known/Measured、插入/删除/换序/内容/尺寸/gap/定位；自测确认能发现保留 |

主要源码：`vendor/gpui-base/src/virtualization/{geometry,state,updates,element,masonry}.rs`、原生 `vendor/gpui-pre/src/elements/list.rs`，以及 Yue 构造器和公共导出。

### 1.1 内容版本的所有权

内容失效使用 BTreeMap 记录分段 epoch。对百万项的一个连续范围失效只增加范围边界，不逐项分配 key/version；只有显式失效才支付这份稀疏元数据。重叠区间覆盖本区间的 epoch，保留两侧原值；整源事务清理旧 index 域的边界。它不是不断增长的失效日志，也不是第二份业务数据。

准备身份为 key、index、source revision、局部失效 epoch、外部 content_revision。start/extent/measured/layout revision 不再隐式参与。index 改变仍会重新准备，因为 factory 可按 index 访问 source；不能宣称任意 reorder 都零准备。

**破坏性变更：**factory 参数由 `VirtualizerItem` 改为 `VirtualizerItemIdentity`。已迁移仓库内 Base/Yue 导出、构造器和测试调用方；Showcase 既有 index/key 用法无需改动。普通 renderer 仍接收完整 `VirtualizerItem`。外部宽度等条件确实参与预处理时显式更新 content_revision；view 内外部状态依赖使用正常 observe/notify，正常文本布局继续跟随可用空间。缓存仍只持工作集内 AnyView，不缓存跨帧 AnyElement。

### 1.2 Splice 的不变区间承诺

`VirtualizerChange::Splice` 明确保证替换区间之外的 key/content/hint 未变；仅评估新增项的估算并保留原生前后子树。既有 hint 也变更时，调用方应显式更新 hints，或使用 Reorder/Replace 完整验证。不能因为换了一个 callback 就在 Splice 中要求旧项隐式重估。

新增 hints 在变更前完整检查。新增末项本身已排除末尾 gap，旧末项转为中间项或旧中间项变为末项时另行校正；混合 trace 曾在改造过程中发现新增末项被重复减 gap，已修正并纳入正常回归。

### 1.3 几何结构与实际复杂度

Known 的每块保存 f32 实际尺寸（符号位表示估算／已测）与 f64 局部累计，上层 SumTree 汇总 count/extent。小尺寸不通过两个巨大 offset 的差值恢复，避免精度丢失；累计属于同一几何 owner 的索引，不是独立测量真值。

- 构造 O(N)，gap 仍是查询参数；gap-only 不复制 N 项。
- 单项查询 O(log(N/B))，范围含块内二分；连续区间顺序读取 O(log(N/B)+W)。多个稀疏连续段分别 seek。
- 单项修改复制一个固定容量块与树路径，O(B+log(N/B))。批量先验证、排序，再合并触及块；包含 k 次验证查询、O(k log k) 排序、q 个块的局部累计与树路径，不能写成整个事务 O(log N)。
- Measured 仍使用原生 ListState/SumTree，其约 93 MB／百万项元数据没有被本次 Known 压缩替代。
- Masonry 泛化替换仍访问 N 项，成本含 source 反查、迁移和 O(N log L) 的最短列分配。新的 append 只估算新增项，沿 L 个当前列末端建堆，再追加；索引向量使用 Rc<Vec> COW，存在旧 snapshot 时仍可能复制 N 个索引，无旧 snapshot 时 Vec 扩容也可能出现一次线性复制。不能称为任意一次 append 都与 N 无关。
- Masonry 追加、尺寸更新按 lane 合批；相同 set_layout 输入直接返回，不增加 revision 或恢复锚点。

## 2. 改造前失败与改造后验证

首先把独立观察转换为正式正确行为断言，再运行旧实现：

| 测试 | 改造前结果 |
| --- | --- |
| `refinements_content_invalidation_refreshes_each_backend` | Fixed 期望版本 1，实际仍为 0 |
| `refinements_content_invalidates_late_callbacks_without_changing_fixed_geometry` | 旧 snapshot 期望 StaleSnapshot，实际 Ok |
| `refinements_alignment_policy_rejects_old_request_without_remeasurement` | 旧目标期望 StaleRequest，实际 Ok |
| `refinements_geometry_changes_reuse_prepared_content` | 纯移动期望 0 次 factory，实际 5 次 |

四项现已通过。新增及扩展测试位置：

- [refinements.rs](../../crates/yue-component/tests/virtual_list/refinements.rs)：局部与重叠内容失效、请求策略、追加原子性、分栏与快照、叶块边界、巨大坐标旁的小 extent、顺序／稀疏迭代。
- [traces.rs](../../crates/yue-component/tests/virtual_list/traces.rs)：8 seeds × 2 axes × 2 sizing modes × 200 steps，共 6400 步。与线性 oracle 对照全部尺寸／位置／可见范围／标记／总长、旧快照及定位目标；不通过更新 golden hash 掩盖差异。
- [lifecycle.rs](../../crates/yue-component/tests/virtual_list/lifecycle.rs)：64 次挂载、滚动、换源和卸载，排空 headless 任务后检查 state/row WeakEntity。故意保留一个真实 row owner 时断言必须失败，释放后必须通过。
- [原生 virtualization.rs](../../vendor/gpui-pre/tests/virtualization.rs)：新增 per-item hint splice 测试，覆盖实际测量、旧 geometry、默认 hint 路径、非法批次及后续 splice。

当前结果：完整工作区 headless＋doctest **774 passed / 8 ignored / 0 failed**；虚拟滚动专项 **60 passed / 7 ignored**（ignored 为手动性能探针，相关项本轮已显式运行）；原生专项 **6 passed**，Base 几何 **2 passed**。完整 Clippy、格式检查及 Showcase Release 构建通过。

这些释放断言证明所覆盖的 Entity／准备缓存／相关任务不再持有 owner，不等于进程 RSS 或 GPU 资源在所有场景均无泄漏。真实 Showcase 未启动、关闭或截图，实际 UI 由用户检查。

## 3. Release 测量

Windows x64 开发机，沿用前轮硬件环境，rustc 1.98.1。最终测量串行执行，未同时运行其他测试／构建任务。记录 CPU 几何操作／提交时间和测试分配器追踪的 payload，不是物理显示帧、GPU 完成或任务管理器进程 CPU。

分配排除宿主业务数据、Element、图片／纹理、分配器内部碎片及栈上 state 本体。压缩前索引基线为叶块改造前同一探针的一轮（此前已修内容／请求与准备缓存）；压缩后三轮。时间中位数均保留首样本，未专门预热，不以小样本推导实时上限。

### 3.1 Known 内存与点更新

[索引分配 CSV](virtual-list-index-refinements-2026-11-04.csv)、[更新 CSV](virtual-list-updates-refinements-2026-11-04.csv)。点更新保留一份冻结 snapshot；前每规模 10 次，后三轮合计 30 次。

| N | 旧 retained payload | 新 retained payload | 旧／新点更新 P50 | 新点更新最大 peak |
| --- | --- | --- | --- | --- |
| 1,000 | 31,952 B | 13,128 B | 0.00265 / 0.00110 ms | 2,676 B |
| 100,000 | 3,128,296 B | 1,245,336 B | 0.00555 / 0.00260 ms | 5,428 B |
| 1,000,000 | 31,274,376 B | 12,433,856 B | 0.00800 / 0.00420 ms | 6,804 B |

百万项几何 retained payload 减少约 60.2%；Fixed 仍为 304 B 的探针堆 payload，Measured 百万项仍为 93,095,248 B。这是几何层分账，不是整个进程内存下降 60.2%。

### 3.2 工作集读取

[读取 CSV](virtual-list-reads-refinements-2026-11-04.csv)。Known 全部 20px、100px 视口，每个样本进行 10000 次分散 offset 查询并读取 5 个工作项。压缩前每规模 3 样本，压缩后各路径 9 样本；逐项与顺序路径均执行 source key 反向校验。

| N | 旧逐项读取 | 新逐项读取 | 新 working_items 顺序读取 |
| --- | --- | --- | --- |
| 1,000 | 4.7989 ms | 3.2429 ms | 3.3442 ms |
| 100,000 | 10.0128 ms | 7.6329 ms | 6.6969 ms |
| 1,000,000 | 12.2380 ms | 9.9338 ms | 8.2220 ms |

表中是整批 10000 次查询的中位数，不是单帧耗时。顺序路径对小集合仍有工作索引迭代的固定成本，不能宣称任何规模必然更快。叶块和顺序读取在此负载同时降低大集合内存与读取成本，故保留该改造；没有移植可变前缀数组或破坏旧 snapshot。

### 3.3 Masonry 冷替换与追加

[Masonry CSV](virtual-list-masonry-refinements-2026-11-04.csv)。三列、统一 40px 未测估算，源 key/index O(1)，末尾增加一项。完整 replace 前后各 3 样本，显式 append 每条件三轮合计 9 样本。

| N | 旧完整 replace | 新完整 replace | 新 append（无旧快照） | 新 append（持旧快照） |
| --- | --- | --- | --- | --- |
| 1,000 | 1.5941 ms | 0.0337 ms | 0.0092 ms | 0.0073 ms |
| 10,000 | 25.0499 ms | 0.2736 ms | 0.0448 ms | 0.0912 ms |
| 100,000 | 341.0327 ms | 2.6431 ms | 0.2786 ms | 0.5602 ms |

泛化 replace 下降主要来自避免二次打包与逐项树更新；显式 append 利用未变前缀合同。这不是完全等价 API 的“无条件倍速”：只有满足不变前缀才能选择 append。小规模样本受分配／调度噪声影响，不能据 1000 项的列顺序认定持快照更快。

同一 [独立观察探针](virtual-list-review-2026-11-04.rs) 的最终结果还确认：两种缓存路径准备内容版本均为 1，局部失效 factory=1；纯位置移动 factory=0；旧对齐请求拒绝；一万项 splice 新增一项估算 calls=1。

## 4. 复现与维护

```powershell
cargo test --workspace --locked --features yue-component/headless-tests,windows-showcase/headless-tests
cargo clippy --workspace --locked --all-targets --features yue-component/headless-tests,windows-showcase/headless-tests -- -D warnings
cargo fmt --all -- --check
rustfmt --edition 2024 --check vendor/gpui-base/src/virtualization/mod.rs vendor/gpui-pre/src/elements/list.rs vendor/gpui-pre/tests/virtualization.rs
tools/test-gpui-base.ps1 -Offline -Filter virtualization
cargo build -p windows-showcase --release --locked

cargo test -p yue-component --release --locked --features headless-tests --test virtual_list geometry_allocation_evidence -- --ignored --nocapture --test-threads=1
cargo test -p yue-component --release --locked --features headless-tests --test virtual_list upgrades_known_update_evidence -- --ignored --nocapture --test-threads=1
cargo test -p yue-component --release --locked --features headless-tests --test virtual_list refinements_ -- --ignored --nocapture --test-threads=1
```

原生测试继续使用前轮一次性 `target/gpui-virtual-upgrades-contract/Cargo.toml`，仅依赖本地 gpui-pre/test-support，test path 指向上述原生 virtualization.rs；未修改 registry 或生产锁文件。独立 review fixture 的生成方式见跨库对标第 7 节。

## 5. 本轮没有据此扩大基础层的部分

- 保持当前 overscan 默认与对称策略。已确认热点在错误失效、重复准备和冷事务；没有实窗空白／CPU 的新测量依据改变缓冲默认，也不引入速度预测。
- 内容、几何、定位请求分别失效，已缩小真实工作范围；不为模仿 bitmask 订阅引入新的状态运行时。
- Masonry 的命令终态、idle/follow-tail/cache 公共协议仍弱于一维 state，属于下一项消费合同扩展；本轮没有借重构虚构其已支持。
- 稀疏 I/O、请求取消与资源字节预算属于消费者／适配层；未向几何核心加入网络请求。自动估计校准同样等待明确消费者与有界样本方案。
- 本轮没有证明此前任务管理器 0.7%→4.5% 的实窗 CPU 峰值已降低。上述微基准只证明其覆盖的工作减少，不能替代用户实窗观察。
