# YueTable M2 Rows 布局组合基线

日期：2026-11-11；实际功能见 [M2](../45-table-m2.md)。这是新增布局功能的 release 基线，不是优化前后比较，也不是尚未实现的 Cells TB02。

## 验收与边界

三组独立 ABBA（1k／100k／100k／1k），每轮 120 个预热帧及 1,000 个计时帧，共 12,000 帧。指定测试逐轮真实通过，没有丢弃轮次；源码、runner 与可执行文件在采样期间保持同一身份。独立复核逐帧 CSV、原始日志哈希及各轮工作集，全部一致。

每帧调用 132–168 次 cell renderer，与总行数无关。按 28rem 视口、Medium 2.5rem 行高、两侧各两行 overscan、一个表头轨道、两个首尾数据轨道、一个 footer 轨道，保守预算为 `(ceil(28/2.5)+4+1+2+1)*12 = 240` 格。分组表头占用更高，不能据 168 格反推全部内容成本。

**正确性、固定区域与有界工作集通过。时序是基线，不作性能改善或完整性能出口结论。** 1k／100k 的 P50 轮间极差／中位数为 5.71%／3.96%，P95 为 32.35%／26.26%，仍有显著设备波动；第三组 P99 规模比 1.051，超过暂定 5% 诊断线。没有配对旧版本样本，不能把该差异解释为代码回归，也不能用整体比值掩盖它。后续优化仍需相同构建和 fixture 的 before／after 独立区组。

## Fixture 与计时

Windows 11 26100，Intel i5-13600KF；沿用 M1 设备。release，opt-level=3、debug assertions 关闭，具体 Cargo profile、OS、lock／可执行文件／源码／构建记录 SHA-256 在归档 identity 中。headless 无物理窗口，rem=16px、light／neutral、reduced motion，不采 GPU、DPI 或呈现延迟。

12 文本叶列，每列 8rem；中间列分组表头，左右各一列固定；首尾数据各一行固定，列 footer 固定。宿主在 48／64／80rem 间每十帧切换，横轴 `-((frame*19)%400)`、纵轴 `-((frame*37)%8000)`。容器反馈与帧间任务在计时外结算；计时是 `Window::draw(cx).clear(cx)`。固定首尾格在每轮末验证存在和有效位置。测试覆盖 Rows，不能写成两轴虚拟化。

挂载包含测试环境、来源建立、布局配置及初次绘制，不是纯索引时间。宽度切换前的反馈收敛不在计时区，因此这不是 W3 的完整输入到布局成本。完整拖动末坐标、取消、键盘与异步宿主接受，由单独 UI 测试验收，不混成 CPU 帧延迟。

## 结果

单位 ms，各规模取六轮分位数的中位数。

| 行数 | 挂载 | draw P50 | draw P95 | draw P99 |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 71.994 | 8.245 | 14.578 | 17.031 |
| 100,000 | 97.126 | 8.275 | 14.341 | 17.083 |
| 100k／1k | 1.349 | 1.004 | 0.984 | 1.003 |

| 独立组 | P50 规模比 | P95 规模比 | P99 规模比 |
| ---: | ---: | ---: | ---: |
| 0 | 0.993 | 0.987 | 1.004 |
| 1 | 0.998 | 1.028 | 1.003 |
| 2 | 1.003 | 1.018 | 1.051 |

分位下标为 `floor((n-1)*q)`；组比值为组内两个 B 分位数的算术平均／两个 A 的算术平均，独立组不混池。CPU 时间含 headless renderer 和观察语义开销，不换算原生 FPS，也不与 M1 dev 数据直接比较。

## 复现与证据

```powershell
cargo test -p yue-component --features headless-tests --test table --release --no-run --message-format=json > target/table-build.jsonl
$artifact = Get-Content target/table-build.jsonl | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'table' -and $_.executable }
python tools/sample-table.py --executable $artifact.executable --output target/table-m2-new --profile release --build-record target/table-build.jsonl --workload m2-layout
```

采样输出为 `target/table-m2-evidence-02`；源码修正前的未完成实验不拼入。证据归档链接与哈希在阶段实施记录中，包含原始日志、CSV、身份、独立验证、构建记录和功能回归。独立验证脚本按 fixture 公式核对 240 格上界。采样 HEAD 为 `543fc6d`、dirty=true；该 HEAD 只表示 M1 基线，M2 实际源码由身份哈希和归档 diff 追溯。
