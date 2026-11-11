# Table 全流程统计与排序优化验收

日期：2026-11-11。Windows、release opt-level3、headless。统计基础提交 `a1c4fb8`，优化提交 `528c222b0798246a46237e628a5cc47823f086e8`。通过同步阶段统计定位查询排序热点，减少重复 source 查找；本地固定查询组合耗时约下降 64%。滚动路径作为回归对照，未声称滚动提速。

## 统计覆盖与初次定位

[使用指南](table-stage-profiling.md)覆盖 source／query／commit／交互／几何／虚拟化／布局／文字／绘制／依赖和清理。独立 feature，默认构建排除新 scope 与计时；没有新增依赖、全局锁或逐事件业务数据。inclusive 包含嵌套工作，self 扣除子 scope；正式 capture 均零 dropped，sum(self)+unattributed 与 elapsed 对账。

九个初诊场景为 TB02、TB05 Fixed／Measured、TB04、复杂 Measured 行、树、编辑、真实伸缩输入及 range 票据。矩阵 capture 墙钟 31765.3ms，包含预热、实际 draw、effects、分配区间和 fixture；下表是累计 self，不能当作单帧延迟或 GPU 时间。

| 矩阵阶段 | self ms | capture 占比 |
| --- | --- | --- |
| base.grid.cell.layout | 7547.7 | 23.8% |
| gpui.layout.compute | 4609.1 | 14.5% |
| base.test_support.prepaint | 5345.6 | 16.8% |
| base.test_support.paint | 5926.3 | 18.7% |
| table.element.row | 1225.5 | 3.9% |

`base.grid.cell.layout`包含逐格请求子树布局，`gpui.layout.compute`为其内部独立计算段。观测树的 prepaint／paint 合计占比很大，单独列出，不能将减少测试观测成本当作组件收益。query 初诊中排序累计约占 capture 的 64%；这是本轮可明确减少重复工作的入口。

## 优化与语义约束

过滤时为需要排序的 query 暂存 `(key 引用, row 引用)`，比较器直接借用记录，结束后生成投影 key；避免每次比较再次调用两次 `source.row`。稳定多列排序、同值原顺序、筛选／分页／汇总与 source 身份保持既有合同，不引入 Row Clone／Send 或持久缓存。无排序时不分配借用数组。

排序仍为 O(M log M)，source 准备／验证和临时空间仍有 O(N)成本。64 位下借用数组每候选槽位两个指针，预留 source.len 槽位；250k 行约4MB，查询结束即释放。本轮没有测纯 query 分配峰值，不宣称内存峰值下降。key 克隆从 filter 移到 sort 末尾，阶段时长含义因此有小幅迁移；主指标使用完整 query 区间。

新增查找复杂度测试限制4096条非 Clone 记录的完整排序／投影 source 读取不超过4N，并核对完整稳定顺序和无排序顺序。既有250k独立 oracle 检查11个条件，保留0／false、missing、范围、多列规则与分页语义。

## 同版本工作量的前后对照

先构建并保存独立 EXE，采样期间不编译；每场景三组 ABBA，原生 test 必须恰好1项通过。query 每进程11条件×6次，共66次，工作量／完整结果 oracle 相同。主指标为每进程66次 query 区间之和，再跨每版本六进程取中位；不含接受 view 和 oracle 的耗时。

| 诊断模式 | 优化前累计 s | 优化后累计 s | 降幅 |
| --- | --- | --- | --- |
| 开启活动采集 | 14.523 | 5.193 | 64.2% |
| 关闭活动采集 | 14.573 | 5.225 | 64.1% |

关闭活动采集后，mode0 无过滤全量排序的重复样本（每条件排除第一次，再跨进程取中位）为 373.578 → 117.017ms。开启采集的 sort self 中位约10.883 →1.586s，下降约85%；capture 总墙钟约16.982 →7.583s，包含验证、oracle、fixture，不能混称为一次操作延迟。关闭采集的收益与开启时一致。

查询预设接受门限为每组总 query 时间至少下降15%。滚动每组 P50／P95回退不超过10%，工作集逐轨迹点一致。矩阵为50k×256、Known尺寸、首尾行和左右列冻结、16个span；每进程120预热＋1000有效样本。下表先每轮取floor((n−1)q)分位数，再跨每版本六轮取中位；单位ms，前／后。

| 批次 | draw P50 前／后 | draw P95 前／后 |
| --- | --- | --- |
| 首批 | 11.973 / 11.959 | 20.884 / 21.193 |
| 确认批次 | 12.002 / 12.008 | 21.128 / 21.293 |

两批共24000有效 draw 样本；同轨迹点工厂调用数完全一致，范围 143–196。分配另外计 draw＋effects，不能混入单次 draw 计时。确认批次每步分配中位为 45778.5 → 45778.5次，26278652 → 26278652 bytes。

## 首次分配异常与确认

**首批精确分配门限未通过，保留失败。**120个原始分配样本中，block1／slot1／after／sample0为51840次、25832854bytes，对应其他轮同点为51839次、25790854bytes，即多1次／42000bytes。不能把原始批次说成逐点完全一致。

临时 allocator 探针记录整个运行中的42000byte分配所属同步 scope，在 `gpui.dependencies` 观察到同尺寸分配；采样10个分配点没有再出现该异常。首次异常没有调用栈，故此证据不能证明它就是同一次依赖跟踪分配，也未认定根因已闭环。临时探针源码、构建及日志单独保留，交付代码已经撤掉探针，统计版语义源码 hash 恢复一致。

预先声明只追加一批固定次数确认：原封不动的两个 EXE、fixture 和门限，关闭活动采集，完整三组 ABBA。首批失败不删除，不提高门限，也不滚动挑选通过的样本。确认批次满足原 P50／P95、逐点工作集与分配计数门限后吸收查询优化；离线 verifier 分别输出首批失败与确认通过。这个结论支持本轮优化采用，不证明绘制后调度和框架分配永远逐点恒定；原异常仍是保留观察项。

## 功能检查与边界

Table统计版59 passed／8 ignored，默认版56 passed／8 ignored；VirtualList193 passed／11 ignored；Showcase Table15 passed。严格Clippy all-targets、默认最小消费者与Showcase构建通过。临时探针恢复后，源码集字节 hash 与已测候选完全一致。正式采样没有观察到编译／Showcase干扰弃样；门禁只排除已知污染，不能保证机器绝对无其他负载。

原生 Showcase 提供 env 选择的文件输出，在正常退出后写报告；只编译验证了入口，真机体验、GPU完成、输入到屏幕延迟、真实IME／Narrator和跨屏DPI未验收。headless `test_support`成本不是原生产品必然成本；下一次布局优化应继续细分逐格请求布局的子树，保留冻结／span／焦点／语义及工作集合同。

## 证据与回退

优化前源码集 SHA-256 `4e9d70fc9049c142d4e3b2cabf458aab3ee33f532621265ff94c8c0b0222d544`；候选源码集 SHA-256 `e39e0127f023eae0f9a5a8be85ef016d5b3408735a0ff08e3d637e6d53d36b5b`。该源码集按 runner 规则，原始字节（含行尾）已随证据保存，另包含构建记录、EXE身份、57个正式进程日志、临时归因探针、阶段CSV、预设门限、首次失败、确认计划及离线复核器。未打包大体积EXE；其hash与构建身份保留。

[完整证据归档](evidence/2026-11-11-table-stage-hotspots.zip)，SHA-256 `48a32a760fad189144f009233d4f0f23b86bb92d3999b24437b44928f9d583d8`。

解压后执行 `python tools/verify-table-stage-evidence.py <解压目录>`。程序复核文件／源码身份、完整进程序列、原始日志与阶段CSV对账、query／工作集样本及各批门限；首次分配门限保持failed，确认单独输出passed。`git revert 528c222b0798246a46237e628a5cc47823f086e8` 可单独撤回排序优化，保留统计基础。
