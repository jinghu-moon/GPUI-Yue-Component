# YueTable M1 headless 规模基线

日期：2026-11-10。对应 [M1 实施记录](../research/2026-11-10-table-m1-implementation.md)、[固定 workload 协议](2026-11-10-table-benchmark-plan.md)。

## 1. 验收结论与边界

**M1 简化文本场景的正确性、工作集有界与完整采样验收通过。** 三组独立 ABBA，共 12 轮、12,000 个有效计时帧；1 轮检测到编译／Showcase 干扰，已作废并重采。源、lock 与测试二进制在采样期间未变化。每个接受轮次确实执行且通过指定测试，帧序列完整，大小表的逐帧 cell renderer 数量完全相同，范围为 156–192。

本次建立首份耗时基线，**耗时回归阈值尚未冻结，也没有证明优化收益**。1k 行的 P50 轮间极差／中位数为 25.56%，100k 为 5.31%；第一组 P50 规模比为 1.104，后两组为 0.982／0.993。自然波动超过暂定 5% 诊断线，因此 P50 规模差异判为不确定，不用总体比值抵消第一组差异，也不据此放宽阈值。P95 轮间极差／中位数分别为 2.94%／2.50%，三个区组规模比为 1.002／0.990／0.993，未见本场景 P95 随总行数同比扩张。

这是新增组件的工作集验收与初始 CPU 记录，不能表述为完整 TB01、真实 Windows 性能出口或全部 M1–M6 性能门禁已通过。后续吸收性能优化时，仍须在相同 fixture 下取得 before／after 双方的稳定独立样本；P50 噪声问题解决前不能用本次数据裁决 5% 的改善或回归。

## 2. 环境与计时边界

| 项目 | 本次配置 |
| --- | --- |
| 系统 | Windows 11 IoT 企业版 LTSC，10.0.26100 |
| CPU／内存 | Intel Core i5-13600KF，14 核／20 逻辑处理器；系统可见内存 33,381,488 KiB |
| 图形设备 | RTX 4070 Ti SUPER，驱动 32.0.16.2101；另有 GameViewer 虚拟显示适配器；本探针不采集 GPU |
| 构建 | dev；Table 测试 artifact opt-level=2、debug assertions／overflow checks 开启；依赖遵循 workspace dev 配置，不能称为完整 release 构建 |
| features | `headless-tests`，生产 Table 与 Base VirtualGrid |
| 视口 | 64rem×32rem host，28rem body，rem=16px；headless，不对应物理显示器 DPI |
| 内容 | Rows＋Fixed、12 文本列、1,000／100,000 行、light／neutral、reduced motion |
| 顺序 | 三组 `[1000, 100000, 100000, 1000]`；每轮 120 预热＋1,000 有效帧 |
| 稳态计时 | `Window::draw(cx).clear(cx)` 的 CPU 时间；设置 owner 位移和帧间任务结算在计时外 |
| 挂载计时 | 包含测试框架初始化、来源准备、首次绘制；初始 light／azure，随后切 neutral 并预热；不是独立 source 索引时间 |
| 分位定义 | 每轮升序样本下标 `floor((n-1)*q)`；下表再取每种规模六轮分位数的中位数，不混池伪造独立重复 |

轨迹为 `-((frame * 37) % 8000)` 的重复纵向位移。身份记录中的 `W2 repeated vertical trace` 是早期标签，不是协议里 200 组首／中／尾、反向和活动格回窗的完整 W2。12 文本列也不是完整 TB01 的混合列形状。单独的正确性测试覆盖 100k 深处定位与稀疏焦点保留，不能算作这些场景的计时样本。

## 3. 结果

单位 ms；各行均为六轮结果的中位数。

| 总行数 | mount | draw P50 | draw P95 | draw P99 |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 121.160 | 70.541 | 101.672 | 107.597 |
| 100,000 | 143.871 | 69.433 | 101.119 | 106.275 |
| 100k／1k | 1.187 | 0.984 | 0.995 | 0.988 |

CPU draw 数值包含测试 renderer、语义与观测开销，不能换算成 Windows 窗口 FPS。mount 的来源规模成本另列；工作集相同不代表总数据内存、建索引、复杂内容或图片成本恒定。单次稳态正确性对照为 156 次 renderer；本轨迹跨行边界后最大为 192，不能把整个轨迹都写成 156。

| 区组 | 1k P50（两轮） | 100k P50（两轮） | P50 规模比 | P95 规模比 |
| ---: | --- | --- | ---: | ---: |
| 0 | 54.782／71.339 | 71.464／67.778 | 1.104 | 1.002 |
| 1 | 69.742／72.814 | 71.063／68.905 | 0.982 | 0.990 |
| 2 | 67.956／71.897 | 69.669／69.198 | 0.993 | 0.993 |

区组比值采用该组 B 两轮分位数的算术平均／A 两轮算术平均；不剔除第一组较快的小表轮次。没有独立分配字节、资源峰值、真实空闲、GPU、输入到呈现或真机读屏证据。

## 4. 原始证据与复现

[证据归档](evidence/2026-11-10-table-m1-baseline.zip)包含 `frames.csv`、`identity.json`、13 轮原始日志（12 接受＋1 作废）、Cargo JSON 构建记录和组件回归日志。`identity.json` 保存源码／lock／二进制／构建记录 SHA-256、每轮日志 SHA-256、分位数与丢弃名单。首次三次未完成尝试不拼入本次记录。

归档 SHA-256：`630eb5f58b136328898c6ea8aeb5e5d77653b39678b47723041ba3e7eea41865`。`verification/delivery.json` 记录独立复核：原始日志哈希正确、CSV 等于原始逐帧样本、工作集逐帧一致，以及用采样时 model 文件重建的源码哈希与身份相同。它也保存类型整理后的源码哈希与唯一差异，便于追溯。

采样时 HEAD 为 `5767372a4ad4cf7ffe25364286f557995fc82b86`，`dirty=true`：M1 源码尚未提交，不要把旧 HEAD 当作含 Table 的 baseline commit。采样源码 SHA-256 为 `f853aac0f9b7ec2741556691761fa0de537d6269b88a7dd3395209dc0c7574a3`。采样后 Clippy 要求整理表头字段类型：`Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>>` 改用已有的等价 `Option<super::Slot>`，不改运行逻辑。归档保留采样时 `model.rs`、此处差异及整理后的源码哈希；相关回归和 Clippy 在整理后通过，不把采样身份写成交付身份，也不为类型别名整理宣称新的计时收益。哈希按实际文件字节计算，换行配置不同需记录；没有随归档提交测试可执行文件。

```powershell
cargo test -p yue-component --features headless-tests --test table --no-run --message-format=json > target/table-build.jsonl
$artifact = Get-Content target/table-build.jsonl | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'table' -and $_.executable }
python tools/sample-table.py --executable $artifact.executable --output target/table-baseline-new --profile dev --build-record target/table-build.jsonl
```

输出目录必须不存在。先构建再采样；采样中不启动编译或 Showcase，不改源码／测试／runner。门禁的进程观察无法穷尽操作系统负载，本次仍实测到了明显噪声；安静门禁通过不等于机器完全无负载。
