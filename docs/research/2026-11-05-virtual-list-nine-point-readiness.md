# 虚拟滚动工程成熟度提升（2026-11-05）

起点为 `1233387`。目标是通过可复现的改进接近自评 9 分，不把测试数量、头文件属性或一次 CPU 样本等同于完整产品验收。继续使用独立提交和前后证据；参考项目只读，生产 manifests／locks 不变。

## 验收顺序

参考项目保持只读。virtua 的 `src/react/VGrid.spec.tsx` 关于逻辑总量、绝对坐标、span 与 covered cell 省略的断言用于设计本轮 Grid 回归；原生 AccessKit 的零基坐标单独适配。TanStack 的 seeded scroll trace 与 vlist 的 Masonry／键盘测试用于复核既有测试分层，未把浏览器 ARIA 或业务键盘导航直接搬进 Base 引擎。

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

普通 Windows CI 新增证据解析器测试；独立手动／每周 performance workflow 使用相同隔离 Release ABBA，上传 CSV、identity 和逐进程日志。默认基线为语义修复后的 `e7b09cf`，可手动选择其他兼容提交。云端任务的日期按实际运行日生成，未把本地通过写成云端运行通过。

解析器要求全部 16 个场景、每场景 5 样本、4 个 ABBA 进程块共 320 行；拒绝缺失／重复／未知场景／错轮次、NaN、非法分位数、配置与 draw 总数不一致、workset 变化、依赖身份不一致和工作负载内容漂移。允许 Git checkout 的 LF／CRLF 差异，不允许其他内容变化。保留 raw source／workload hash 和独立依赖记录。

粗粒度性能报警同时要求总 CPU 中位数回退超过 35% **且**绝对增加超过 50µs／样本；空集合冷启动只有约 10µs／16 次配置，相对数不能单独作为失败依据。稳定 10,000-span 的配置成本还要求至少下降 50%，避免功能完全失效却因对照旧基线总耗时相同而漏报。此宽松报警用于共享 runner 排查，不是产品帧预算，也不替代固定机器的两轮采用测量。

原生 PowerShell runner 每项单独运行，保存独立日志，再检查准确的 test 身份和“1 passed／0 failed／0 ignored”。Cargo 返回 0 但 filter 漂移成 0 tests 也会失败。七组 Python 测试通过，含 18 个负控制（10 种数据缺陷、2 种总耗时回退、4 种原生错误、1 种收益丢失及 1 种错进程顺序）；两轮真实样本通过同一 CLI 检查。最终原生执行结果在下一节记录。

## 4. 最终验证

### 正确性与执行证据

- 本轮 workspace（headless-tests,virtual-profile）：**902 passed／17 ignored**；其中 virtual_list **185 passed／10 ignored**，Showcase 的新语义回归另行纳入 workspace。ignored 探针不计入普通通过数。
- 无默认特性＋headless-tests：**869 passed／10 ignored**。
- workspace all-targets Clippy（headless-tests,virtual-profile，`-D warnings`）通过。
- 生产 pin 的隔离 vendor 合同：**17 个原生＋4 个 Base**，源 manifests／locks 未改。
- 最终 Release 原生探针 **5 个独立进程全部通过**；每项准确执行 1 个测试，新增 runner 校验实际通过。另以真实 libtest 的错误 exact filter 产生 0 tests／退出 0，checker 退出 1；真实 CSV 删一行和 workload hash 漂移也均被拒绝。
- `cargo fmt --all -- --check`、已改 vendor 文件的 rustfmt check、`git diff --check` 通过；两份文档的 73 个本地链接／锚点及 23 个归档文件的 LF 散列核对通过。

### 原生交互与平台边界

CUA 重新绑定本轮构建的生产窗口，读取 AX 和截图并注入真实按钮／滚轮。初始 88 个命名 cell；合并后 83 个，`2 × 2 · 合并 2×3` 唯一，覆盖位置省略；切换 RTL 后同一逻辑 origin，冻结列移至右侧。滚轮请求 3,000px 后可见行标签 31–40 和冻结行；这不是实际逻辑位移／输入延迟。末端定位后 88 个名称唯一的 cell，`1000000 × 1000` 与冻结角 `1 × 1` 各一个。关闭后窗口从 fresh inventory 消失，helper 成功退出。

当前 AX 工具只展示这些节点的 role／name，没有展示数值 Grid 模式或 description，不能据此宣称 UIA 的行列数／跨度／HelpText 已通过。结构化数据仍以同一个 AccessKit Node 的回归为证。未启动／试听 Narrator，浏览与 ReadAll 未验收。单显示器 scale=1.5，rem 16／20 不等于真实跨 DPI；跨显示器与不同物理 DPI 未覆盖。截图当场检查，未归档图片文件。摘要和 stdout 见[交互记录](virtual-list-nine-native-transcript-2026-11-05.txt)。

### 原生 CPU 尾部成本：不利轮次与同期对照

首次最终探针的 768 个 draw 样本全部保留：Grid rem16 P50／P95／P99 为 5.078／7.488／8.376ms，27/96 次超过当时 6.25ms 刷新间隔；rem20 为 5.874／8.209／10.565ms，39/96 次超过。其余场景也比历史单轮慢。没有用历史非同期样本把这种差异归因给源码。

因此追加同一份生产 native_profile workload 的原生 ABBA。基线 `1233387` 完整工作区与当前 `6d21622` 的源码分别构建；生产 manifests／locks 不改，依赖记录相同。工具先遇到共享 target 的 stale 导出：当前源码被标为 fresh 但缺失 VirtualGridSpanSet。两次失败均发生在编译阶段、没有任何计时样本；[诊断摘录](virtual-list-nine-build-diagnostic-2026-11-05.txt)保留。改为原生前后独立 target、完整基线消费者和固定可执行文件后成功。当前可执行文件 SHA256 与先前 5 项正向探针完全相同。

补充 ABBA 共 **3,072 行**，每个版本／场景 192 个样本，8 场景的 step 身份完整且唯一，没有编译干扰。下表保留 Grid 的成本变化；不能把追加轮次替换掉较慢的首次 768 个样本。

| 场景 | before P50 | after P50 | before P95 | after P95 | before／after 超过 6.25ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Grid rem16 | 2.847ms | 2.980ms | 4.762ms | 5.154ms | 2／0（各 192 次） |
| Grid rem20 | 3.260ms | 3.288ms | 5.470ms | 5.728ms | 3／5（各 192 次） |

Grid P50 +4.7%／+0.8%，P95 +8.2%／+4.7%。修复提供更完整的语义，接受这一可见工作负载下的成本，**不宣称零开销或原生整体加速**。其他场景 P50 −8.4% 至 +12.3%、P95 −21.6% 至 +5.2%，原始数据与所有分位数均保留。一次 ABBA 不足以把细小变化精确拆分为 metadata、cache 或环境的单项贡献。

这里是应用侧 draw／CPU present 时间，不是 GPU 完成、真实 FPS 或硬件输入到显示延迟；仍不保证稳定 160 FPS。补充核验了普通工具路径：旧基线启用 span-cache 故意编译失败，确认旧 Base 没有新 API；新工具再次运行完整普通 ABBA（span-tool-guard，320 行）并通过同一数据检查。最初两轮暖／冷 span 采用证据没有被替换。

### 复现与回退

```powershell
cargo test --workspace --features headless-tests,virtual-profile --offline --locked -- --test-threads=1
cargo clippy --workspace --all-targets --features headless-tests,virtual-profile --offline --locked -- -D warnings
cargo test -p yue-component --no-default-features --features headless-tests --offline --locked -- --test-threads=1
python tools/test-virtualization-contracts.py --offline
python tools/test-virtualization-evidence.py
& tools/test-virtualization-native.ps1 -Release -Offline -IncludeFrameProfile
python tools/compare-virtualization.py --before e7b09cf --label span-reuse-new --date 2026-11-05 --filter series_span --workload docs/research/virtual-list-span-reuse-2026-11-05.rs --after-feature span-cache
python tools/compare-virtualization.py --native --before 1233387 --label native-new --date 2026-11-05
```

label 必须全新，不覆盖旧样本。原生命令会打开真实窗口，只在已授权的桌面验收环境运行。云端 workflow 已配置，本会话未声称 GitHub runner 已执行通过。

| 改动 | Git 点 |
| --- | --- |
| Grid 原生元数据、失败／通过回归 | `e7b09cf` |
| 显式 span set、消费者、前后样本 | `6d21622` |
| 证据检查、原生准确执行、独立产物、CI | `019135f` |
| 最终原生证据、评分、索引 | 本报告同批提交 |

提交间有消费者／工具依赖；按上述顺序的逆序使用 `git revert` 回退，再跑相应合同。没有 reset 或 force push。参考目录只读；生产 manifests／locks 不变。

## 5. 再次自评与 9 分门槛

**综合自评 8.8／10，尚未把 9 分目标记为完成。** 这是六项等权的主观工程评估，不是三个参考项目之间的统一跑分。

| 维度 | 本轮后 | 依据／保留缺口 |
| --- | ---: | --- |
| 几何、身份与事务正确性 | 9.2 | 独立 oracle、组合轨迹、reorder／反馈／冻结／离屏 origin，非法配置原子拒绝 |
| 布局与能力组合 | 9.0 | 两轴 Grid、Masonry、一维 Measured；冻结／RTL／span／有界 smooth，明确模式限制 |
| 工作量与 CPU 成本 | 9.0 | 两轮稳定 span 采用测量、默认控制、冷成本、原生同期对照和不利尾样本均保留 |
| 回归与证据链 | 9.2 | 实际执行计数、正负控制、原始样本／身份、完整 workspace 与原生回归；云端启用待实际观察 |
| API 与生命周期 | 9.0 | opt-in、单一缓存项、完整轴身份失效、无 owner 保活，未引入默认全量缓存 |
| 平台与可访问性验收 | 7.5 | Windows 原生交互可复验，但 Grid/GridItem 桥接、Narrator、真实跨 DPI 未闭环 |

到 9 分优先补两个验收闭环：

1. 在专门分支评估锁定 AccessKit Windows 桥的升级／局部修复，提供实际 UIA Grid/GridItem provider 合同，再做 Narrator 的位置、总量、跨度、滚动回收、浏览和 ReadAll 验收。依赖调整需要单独评审，本轮维持生产 pin。
2. 在实际多显示器／不同 DPI 环境验证跨屏、尺寸反馈、阅读锚点、焦点、最小化／恢复，保留同一轨迹下的 tail 样本。当前单屏环境无法提供这一证明。

核心机制和工程回归已显著接近 9 分；下一步按这些缺口补证据与平台接线。

## 证据索引

- [Grid 语义失败／通过日志](virtual-list-nine-a11y-transcript-2026-11-05.txt)
- [span 采用与默认对照汇总](virtual-list-nine-span-evidence-2026-11-05.json)
- [最终验证／原生同期分位数／正负控制](virtual-list-nine-validation-2026-11-05.json)
- [首次原生 768 样本](virtual-list-nine-native-2026-11-05.csv)
- [同期原生 ABBA 3,072 样本](virtual-list-series-nine-native-isolated-2026-11-05.csv)
- [原生构建／依赖／可执行文件身份](virtual-list-nine-native-identity-2026-11-05.json)
- [工具重验 ABBA](virtual-list-series-span-tool-guard-2026-11-05.csv)
- [归档文件 raw／LF 散列](virtual-list-nine-artifacts-2026-11-05.json)
