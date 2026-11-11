# Table 全流程分阶段统计

统计由 `table-profile` feature 显式启用；默认构建不包含这些 scope／时钟读取，也不隐式开启 `test-support`。底层独立 feature 为 GPUI／Base 的 `perf-stats`，没有新增依赖。适用于同步主线程工作的定位，不将 headless 数据当作原生 FPS。

## 覆盖环节

| 层级 | 统计范围 |
| --- | --- |
| 数据与查询 | source 索引／验证、query 验证／过滤／稳定多列排序／投影／汇总、接受快照 |
| 状态与交互 | 布局同步、几何重建、选择、导航／定位、query 请求、编辑开始／保存／提交／协调、伸缩与重排、range／tail 请求与提交、树快照 |
| Table 元素 | render、行构建、delegate 和 column renderer、span 集合、Static 请求布局／预绘制／绘制、自然尺寸测量与反馈 |
| Base 虚拟化 | List／Grid 请求布局／预绘制／绘制、轴 source 替换／snapshot、Grid 轴 prepare／commit、逐格构建／布局／预绘制 |
| GPUI | 输入分发、根元素、请求布局、布局计算、预绘制、绘制、文本 shaping／layout、场景整理、可访问性整理、依赖收集、frame／arena 清理、原生 present 调用 |
| 测试观测 | Base test_support prepaint／paint，fixture draw＋effects／effects；单独归因，不能将删减观测开销称为组件优化 |

这是预先定义的同步阶段聚合，不是完整调用树。未埋点的子工作落在父层 self；异步业务、I/O 等待、系统线程、D3D／GPU 完成和输入到屏幕的延迟不包含在内。`gpui.present` 只测提交调用返回，不能证明 GPU 完成；headless 通常不会调用此阶段。资源清理记录 frame／arena；历史 source 是否释放由既有 Weak 探针独立断言。

## 输出与计时解释

`TABLE_CAPTURE` 输出 elapsed／unattributed／dropped；`TABLE_STAGE` 输出静态名称、完成调用次数、inclusive／self／max，单位 ns。`Instant` 测量经过的墙钟时间，包含抢占和诊断开销，**不是操作系统线程 CPU 时间**。inclusive 包含子阶段，不能跨层相加；self 扣除直接子 scope，可以相加；max 为单次 inclusive 最大值，不能当作 P95。

`sum(self) + unattributed == elapsed` 必须对账，`dropped == 0` 才接受诊断。unattributed 包含 scope 外的 fixture、oracle、输出和调度时间；不能当作全是业务热点。不同数据量／工作集、不同 feature 或不同诊断粒度的时间不能直接解释为优化收益。`gpui.frame.clear.0` 为 ElementIdStack 清理，`.1` 为 Frame 清理。

Recorder 同线程、固定上限 128 个静态阶段和 256 层栈。只聚合，不保留事件／业务 key／用户内容，不逐帧日志、不持有业务对象、不使用全局锁。Scope 必须按栈顺序结束，不跨 await／yield；Capture 可覆盖多个主线程事件循环回合。重叠 Capture 拒绝；未结束 scope 或乱序 drop 的 capture 返回错误；取消后的迟到 scope 不污染下一轮。容量耗尽计 dropped，不静默冒充完整结果。未启用 capture 时不读时钟，但启用 feature 的空闲 scope 仍有 TLS 检查成本。

## headless 诊断与版本对照

先构建，再采样，禁止并行编译污染时间：

```powershell
cargo test -p yue-component --test table --features headless-tests,table-profile --release --no-run --message-format=json 1> target/profile-build.jsonl
# 从 compiler-artifact 中取 target.name=table 的 executable，保存独立目录，保持 basename。
python tools/profile-table.py --before <EXE> --before-build target/profile-build.jsonl --output target/profile-run
```

默认诊断 cells（TB02）、fixed／measured 完整替换（TB05）、query（TB04）。可通过 `--cases measured-frames tree edit resize range` 单独观察复杂行、树、编辑、真实伸缩输入、range 票据。TB04 每个条件完整顺序 oracle；TB05 每次检查最新值及历史来源释放。功能场景保留原有断言，无额外任意等待。

优化前后各保存同粒度、同 fixture 的 release EXE 与构建记录；加 `--after <EXE> --after-build <JSONL>` 使用三组 ABBA，每场景十二独立进程。runner 核对完整样本序列、工作集／结果计数等价、零 dropped、自耗时对账、原生 test 成功、源码／runner／二进制采样期间不变，保留所有日志和身份。`--capture off` 时不输出 capture／stage 数据；runner 摘要中的 capture 时长 0 表示未采集，实际时间仍取原始 TABLE_QUERY／TABLE_FRAME。诊断不是性能结论的唯一依据：同时关闭 `TABLE_PROFILE`（保留同一二进制）复测，确认收益不依赖活动插桩；正式默认 feature 再编译／测试。

门禁连续两次观察无 cargo／rustc／clippy／Showcase 后采样；若期间观察到外部进程，整轮弃样重跑，不结束外部进程。这只排除已知污染，不保证机器完全无其他负载。接受时比较每轮分位数及每组前后方向，不把单次最小值当结果。

## 原生 Showcase 与自定义捕获

```powershell
$env:YUE_TABLE_PROFILE_OUTPUT = 'D:\Temp\yue-table-profile.txt' # 必须是新文件
cargo run -p windows-showcase --release --features table-profile
# 正常关闭应用后输出完整 capture；强制结束／空文件不得接受。
Remove-Item Env:YUE_TABLE_PROFILE_OUTPUT
```

仅带 feature 且设置输出路径时启用；覆盖启动、用户操作、退出，其他页面也会计入 GPUI 总量。建议每次只复现一个场景，使用相同操作轨迹。输出在原生事件循环正常返回后写入，避免退出输入作用域未闭合；不改产品界面。文件采用 create_new，已存在时拒绝覆盖。此入口已编译验证，真机结果由人工复现后另行验收。

嵌入消费者可用 `gpui_kit::perf_stats::Capture::start()`，在同线程无打开 scope 时 `finish()`，遍历 `snapshot.stages()`。业务范围用 `scope("app.table.fetch.apply")`，只用静态名字；不要把请求等待期包在同步 scope 中。后台线程需各自单独 Capture，不能把线程间数据合计冒充同一份主线程时间。
