# YueTable M5 双轴性能基线

日期：2026-11-11。生产 Cells／Base VirtualGrid，release opt-level3，Windows11 26100，Intel64 Family6 Model183，headless renderer。基线 HEAD 为 0fabce1，交付源码 SHA-256 `9e5047a8e80d2f4655b4577ed152a5b84cf05b1a26149d7dc5863d33533e352a`。这是新增功能的规模基线；CPU draw 不是 GPU FPS，也不是优化收益。

TB02 使用 1k／50k 行×256列，三档 Known 行高40／48／56px及列宽80／96／112px，左右各2列、首尾各1数据行固定，16个3×2正文合并。末尾冻结行两规模均40px，保证正文可用空间相同。宿主64×32rem，Table28rem，rem16。横向轨迹 `-((frame*97)%16000)`、纵向 `-((frame*37)%8000)`；未声称覆盖完整W1／W2所有轨迹。

三组ABBA、每轮120预热及1000有效帧，共12000帧。独立脚本逐轮核对日志哈希、分位数、10个独立分配步和每个轨迹点工作量。保守预算 `(ceil((448+160)/40)+3)*(ceil((1024+160)/80)+4)+16 = 377`，实际143–196格；各规模和各重复逐帧完全一致。

| 行数 | draw P50中位ms | draw P95中位ms | draw P99中位ms |
| --- | --- | --- | --- |
| 1000 | 11.809 | 16.544 | 20.088 |
| 50000 | 11.771 | 16.514 | 19.786 |

P50／P95／P99规模比分别0.9967／0.9982／0.9849。分位数取排序后floor((n−1)×fraction)，表中跨六轮取中位。通过范围为身份、正确性、工作集有界及规模解耦；没有冻结帧时回归阈值，不把噪声内差异称为性能改善。每独立分配步约5.2万次、25.8MB，包含draw和effects；不与单纯draw计时混算。索引、Known几何和完整来源仍有O(N)初始化成本。

第1批完整采样因首尾冻结尺寸不一致导致167个轨迹点工作集不同而拒绝；修正fixture重新构建。第2／3批因外部编译干扰／等待安静超时未接受。第4批完整通过，其中一轮受外部编译干扰被整轮弃用并重跑。没有结束外部进程、拼接不同批次或降低门禁。`--attempts 20`仅扩大整轮重试次数，默认仍为3。

Table dev／release 43 passed、6 opt-in ignored，VirtualList 193 passed、11 opt-in ignored，Showcase Table11 passed。组件／Showcase／最小消费者严格Clippy通过；主题生成、Python语法及diff检查通过。原生IME、Narrator、GPU与跨屏DPI仍需真机验收。

[证据归档](evidence/2026-11-11-table-m5-cells.zip) SHA-256 `c1a06ce8b4fec98a7f29b4354af6b797cb4bf96ce712dc285866ea95cdfd47c0`，包含接受样本、弃样理由与日志、源码快照／diff、构建身份、功能检查和独立验证器。归档前确认源码digest与接受identity一致。
