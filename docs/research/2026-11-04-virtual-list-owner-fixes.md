# 虚拟滚动第四轮实施：阅读锚点、宿主边界与输入所有权

日期：2026-11-04。承接[第四轮对标](2026-11-04-virtual-list-owner-review.md)，在开发期接口上直接修复，不增加兼容层。真实 UI、GPU 帧节奏及任务管理器 CPU 由用户检查；本文只报告生产接口的 headless 回归与 Release CPU 采样。

## 1. 实施范围与唯一所有者

| 问题 | 修改所在所有者 | 结果 |
| --- | --- | --- |
| padding/gap 被当作项内位置 clamp | Base VirtualizerState | 内容和相邻留白分开表达，三模式、两轴使用相同规则 |
| 离屏 Attached 结构更新拉动宿主 | Base commit／锚点恢复 | 列表外的阅读起点归宿主；清除旧列表阅读校正 |
| Masonry 旧测量／定位覆盖新输入 | Base Masonry + 原生 ScrollHandle | 按当前真实位置补偿几何差；用户输入取消 pending |
| 大 offset 丢失微小输入 | 原生 ScrollOffset／ScrollHandle | 同一个位置对象内维护精确值与布局投影；Fixed/Known、Grid、Masonry 接入精确读写 |
| live invalidation 与 portable cache 混淆 | Base cache／source API 文档 | 持久身份由 source/layout revision 决定；schema 2 保存区域锚点 |

没有新增 viewport Entity、逐项镜像位置、通用 driver／插件总线或第二套测量树。Masonry 的 last_offset 仅用于识别外部输入，不负责滚动位置；锚点和待定位命令也不成为另一份当前位置真值。

## 2. 内容与留白的锚点

`VirtualizerAnchorPosition` 取代旧的无符号项内偏移：

```rust
pub enum VirtualizerAnchorPosition {
    Before(f64),
    Within(f64),
    After(f64),
}
```

距离单位为逻辑像素，须有限且非负。

| 当前阅读区域 | 保存 | 恢复到新几何 |
| --- | --- | --- |
| 首项前 padding | 首项 key + Before(distance) | 内容起点减 distance，最多当前 leading padding |
| 项内内容 | 当前 key + Within(distance) | 内容起点加 distance，最多当前内容 extent |
| 项间 gap | 后继 key + Before(distance) | 后继内容起点减 distance，最多当前 gap |
| 最后一项后的 padding | 末项 key + After(distance) | 内容末端加 distance，最多当前 trailing padding |

最终由唯一 owner 按实际可达范围 clamp。Measured 原生槽位包含 gap，恢复时先去除 gap，以真实内容 extent 解析位置。内容缩短不会把原来的 Within 越界位置自动改成 After，仍在内容末端收敛。

20px 内容、10px gap、10px leading padding 下，位置 5 位于首 padding，35 位于第一段 gap。仅尾部追加不影响它们前方几何，修复后分别保持 5、35。若 gap 缩为 3，原来距后继起点 5px 的 Before 只能保留 3px；若 leading padding 增为 20，距内容起点 5px 的位置变为 15。

Attached 以视口**起始边缘**是否在 `[origin, origin + total_extent)` 判定阅读归属，不以是否至少有一项可见判定。header 起点即使部分覆盖列表也不归列表；列表内部大 gap 即使没有可见项仍是合法阅读位置。区域外 commit 清除 anchor 与 pending anchor；显式定位与显式尾随仍按各自合同处理。

## 3. portable cache 的破坏性变更

`VirtualizerCache::new()` 产生 schema 2，恢复只接受 schema 2。除了既有 axis、layout revision、cross extent、gap、各项 key／content revision／extent，调用方还须保存锚点的 Before／Within／After 变体与有限非负 distance。

`.anchor(key, position)` 接收枚举，`scroll_anchor()` 返回 `Option<(&ElementId, VirtualizerAnchorPosition)>`。Yue 模块及 crate 根已显式 re-export。schema 1 不做猜测迁移；应用若在外部持久化旧格式，应丢弃旧版本或以已知业务语义转换，不能把任意旧浮点数自动当作留白距离。

restore 先验证整批，非法锚点、尺寸、重复记录及不兼容布局不会部分修改几何。两轴 cache 往返覆盖 Before 与 Within，尾 padding 覆盖 After。

`invalidate_items` 的 epoch 属于当前控制器反馈身份，不是跨控制器持久内容版本。相同 source revision 下旧 cache 可恢复是合同边界；尺寸相关内容改变须推进 `VirtualizerSource::revision_at`，字体等共享排版改变须推进 layout revision。测试验证同 revision 可恢复，以及推进 revision 后跳过旧测量，没有把条件性风险伪报为已修复缺陷。

## 4. Masonry 的最新位置与用户接管

原问题是尺寸变化时恢复上一帧锚点，以及跨帧 pending 在用户继续滚动后仍写回旧目标。修复分开处理两个原因：

1. 原生 wheel 自动 `cancel_scroll()`，包括边界没有位移的输入。外部 scrollbar／offset 更新在下一次消费前被识别并取消旧 pending；消费者也可显式取消。
2. `observe_input()` 比较原生精确位置与本组件上次应用值，确认所有权变化；它不扫描锚点。
3. `update_sizes()` 先完整校验输入，确定尺寸或 measured 身份确实变化后，在**旧几何**上按当前原生位置捕获锚点，再更新树、补偿几何差。相同尺寸的稳定批次不为锚点另扫工作集。
4. layout／reflow／source replace／append 在修改前执行同样的输入观察和位置捕获。卸载清 pending 与上次位置，不接受旧挂载反馈。

`has_scroll_command()` 仅报告 pending，`cancel_scroll()` 不写 offset。定位继续使用既有预算与 deferred 校正路径；本轮没有复制一维完整的命令结果事件和止损状态机。既有完整可见覆盖、动态预算和资源释放测试继续执行。

## 5. 原生精确位置与局部呈现

原生共享 Rc 内部从 `Point<Pixels>` 改为 `ScrollOffset`：包含 f64 精确位置、Pixels 投影和旧接口可变写入标记。没有再分配第二个 Rc，也没有随 N 增长的位置数组。

- wheel 在同一 owner 上以 f64 累加和 clamp；即使 f32 投影相同，只要精确位置变化就通知。
- native prepaint 使用精确值 clamp，不在稳定帧抹去小数；超出起点的输入不留下反向滚动余量。
- `precise_offset()` 返回 `Point<f64>`；`set_precise_offset()` 接收有限值并同步投影。
- 既有 offset()／set_offset() 保留 Pixels API。显式 jump 或原生旧路径可变写入会重置旧小数，即使写入相同 f32 投影也生效。
- 原生 max offset 的两位小数 round 使用 f64 中间运算，避免大值先乘 100 再丢精度。

Fixed/Known Owned、Grid 两轴和 Masonry 从同一 handle 精确读写。Fixed/Known／Grid 已有局部绘制，先在 f64 中计算 item.start − offset，再转换 Pixels；点击区域和呈现 bounds 一起验证。设备像素 snap 是正常合同，例如测试 scale=2 时逻辑 3.75px 的边缘落到 3.5px，不能用未量化坐标替代实际绘制断言。

**仍有范围限制：**Measured ListState 的累计 SumTree 和绝对位置仍是 f32，普通 Div／UniformList 仍以 Pixels 投影绘制；scrollbar 的 f32 映射不承诺所有亚像素位置均可寻址。本轮修复的是输入保留与已接入路径的精确位置／局部呈现，不是任意规模下全部模式的精度保证。

## 6. 改前失败与改后证据

同一组六项主回归在隔离的修复前 Base／GPUI 源码运行，**5 failed / 1 passed**。cache identity 一项本来符合合同，其余五项行为断言失败。没有临时回滚生产工作区，没有把仅打印观察的测试当作正确性验收。

| 行为 | 修复前 | 修复后 |
| --- | --- | --- |
| 尾部追加，leading padding 位置 5 | 10 | 5 |
| 尾部追加，gap 位置 35 | 30 | 35 |
| Attached 区域外宿主位置 0／900 | 提议 500／700 | 无抢占请求 |
| Measured Masonry 四次 20px 输入 | 60 | 80 |
| Masonry 待校正期间用户滚至 6543 | 被写回 6523 | 保持 6543，pending 取消 |
| Fixed/Known 在 offset=2²⁴，单次 0.25px | 0 | 0.25 |
| 同处单次 1px／连续 8×0.25px | 0／0 | 1／2 |

精度测试覆盖纵横轴及 Fixed/Known，不据此推断 Measured 累计树已转换。新增回归共 13 项，额外覆盖 cache 原子拒绝、留白变化、部分 Attached 覆盖、Masonry scrollbar／外部位置接管与显式取消、原生反向／jump／边界、实际 bounds／点击及 Grid 两轴精确合并。

- [旧源码真实失败输出](virtual-list-owner-before-2026-11-04.txt)。
- [新源码 68 条主观察输出](virtual-list-owner-after-2026-11-04.txt)；历史第四轮原始记录另行保留。
- [六项主回归](../../crates/yue-component/tests/virtual_list/owner.rs)、[七项边界回归](../../crates/yue-component/tests/virtual_list/owner_edges.rs)。
- [原生集成测试](../../vendor/gpui-pre/tests/virtualization.rs)。

最终验证结果：

| 范围 | 结果 |
| --- | --- |
| virtual_list 全专项 | 91 passed / 8 ignored，包含本轮新增 13 项 |
| workspace headless + doctest | 777 passed / 8 ignored |
| Showcase headless（单独启用其 feature） | 28 passed / 1 ignored |
| 上述 workspace 与 Showcase 合计 | 805 passed / 9 ignored，无重复计算 owner 专项 |
| 原生 virtualization 集成 | 8 passed |
| Base virtualization 内部几何 | 2 passed |
| workspace 全 targets Clippy | `-D warnings` 通过，包含 Showcase headless feature |
| 格式与构建 | workspace fmt、所改 vendor rustfmt、Showcase Release build 通过 |

再次筛选 owner 时还匹配了 5 个既有测试，共 18 passed；其中 `owner::` 模块恰为新增 13 项。没有把这 18 项再次计入总数。9 项 ignored 为手动性能探针，不视为正确性通过；本轮 A/B 是下节独立执行的测量。机器摘要、命令范围和结果见[验证记录](virtual-list-owner-validation-2026-11-04.json)，隔离改前／当前七个主要实现文件的身份见[源码 SHA-256](virtual-list-owner-source-hashes-2026-11-04.json)。

## 7. Release A/B 的测量合同

[共享 workload 源码](virtual-list-owner-perf-2026-11-04.rs) 使用生产原生事件分发及 headless draw。Fixed／Known 各百万项，Measured Masonry 万项、三列；窗口分别为 240×100、300×100 逻辑像素，内容为简单 Div。每样本预热 40 帧，计时 3,000 帧的 ±1px 交替输入，最终位置为 0；排除 source/state 初始化，两版本使用相同源码和 callback 计数。

这里测量已测稳定小窗口，避免旧版 Masonry 输入错误使新旧落在不同工作集。它检验精确 owner 与稳定测量处理的固定 CPU 成本，不代表冷启动、大范围拖动、复杂内容、持续尺寸变化或真实 GPU 工作负载。先构建再交替运行，保存逐样本微秒及 source 读取／renderer 构建计数；不使用任务管理器总 CPU 推导单帧成本。

环境：Intel Core i5-13600KF（14 核／20 线程）、Windows x86_64、rustc 1.98.1；同一 Release 构建配置。所有基准构建完成后，按 before/0 → after/0 → after/1 → before/1 运行。每轮每模式 5 样本，共每版本每模式 10 样本；采样开始和结束均无 cargo/rustc 进程。不固定 CPU 核心或锁频，桌面后台噪声仍可能存在。

| 模式 | 改前中位数／范围（ms / 3,000 帧） | 改后中位数／范围 | 中位数变化 | 每样本 source 读取／构建 |
| --- | --- | --- | --- | --- |
| Fixed，百万项 | 66.156 / 64.526–66.856 | 66.213 / 64.938–67.592 | +0.09%，+0.019µs/帧 | 39,000 / 16,500，两版本一致 |
| Known，百万项 | 67.991 / 66.837–69.205 | 68.515 / 66.977–69.332 | +0.77%，+0.175µs/帧 | 39,000 / 16,500，两版本一致 |
| Measured Masonry，万项 | 101.296 / 99.939–104.270 | 103.022 / 100.838–105.081 | +1.70%，+0.575µs/帧 | 189,000 / 27,000，两版本一致 |

原始 60 个样本分别保存在[改前 CSV](virtual-list-owner-perf-before-2026-11-04.csv)和[改后 CSV](virtual-list-owner-perf-after-2026-11-04.csv)。范围重叠，未做统计显著性检验，不能宣称稳定帧速度提升，也不能据这组简单窗口样本认定所有场景零代价。本轮直接收益是修复位置／输入正确性；稳定帧没有因额外捕获锚点增加 source 访问或卡片构建，时间中位数略增，按原始结果公开。

这些数据不能证明原来任务管理器 0.7%→4.5% 的实窗 CPU 峰值已下降，不能换算成 GPU 帧率；没有采集 allocator 或峰值内存，也不作其改善声明。持续尺寸变化时新增的锚点捕获仍有工作集成本；本轮只通过消除相同测量批次的额外捕获来避免稳定路径支付该成本。

## 8. 复现与维护

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list owner --locked -- --nocapture --test-threads=1
cargo test --workspace --features windows-showcase/headless-tests --locked -- --test-threads=1
cargo clippy --workspace --all-targets --features windows-showcase/headless-tests --locked -- -D warnings
cargo fmt --all -- --check
tools/test-gpui-base.ps1 -Offline -Filter virtualization
cargo build -p windows-showcase --release --locked
```

原生 fixture 的实际验证命令：

```powershell
cargo test --manifest-path target/gpui-virtual-upgrades-contract/Cargo.toml --offline --locked --target-dir target --test virtualization -- --test-threads=1
cargo test --manifest-path target/gpui-base-contract/Cargo.toml --offline --locked --release --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --features test-support --lib virtualization
cargo test --manifest-path target/virtual-owner-fixes-20261104/perf-before/Cargo.toml --offline --locked --release --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --test regressions -- --nocapture --test-threads=1
cargo test --manifest-path target/virtual-owner-fixes-20261104/perf-before/Cargo.toml --offline --locked --release --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --test perf --no-run
cargo test --manifest-path target/virtual-owner-fixes-20261104/perf-after/Cargo.toml --offline --locked --release --target-dir C:/Temp/gpui-yue-owner-perf-20261104 --test perf --no-run
```

上述 regressions 的预期结果为 5 failed / 1 passed。两次 perf 构建完成后，直接运行各自输出的 perf 测试二进制 `--nocapture --test-threads=1`，不在计时期间构建；按前述顺序各运行两次并提取 OWNER_PERF 行。fixture 是本机可丢弃构建配置，长期保留的共享源码、前后原始输出、逐样本 CSV 和 SHA-256 身份由本文链接。

原生 virtualization 集成 fixture 只加载当前 gpui-pre 与 test-support，避免发布包无关测试资产问题；Base fixture 由现有脚本生成。改前／改后性能 fixture 分别 patch 指向隔离旧源码／当前 vendor，运行同一份 `[[test]]` workload，使用单后端 gpui-kit + Base test-support，不引入 Fast 后端。

本机 D 盘不足时，构建产物保留迁往 C 盘；Base 内测、最终 Showcase 构建与基准使用 `--target-dir C:/Temp/gpui-yue-owner-perf-20261104`，不改变生产源码。完成编译后直接交替运行对应测试二进制。隔离旧源码来自本轮修改前文件备份，不将仓库 HEAD 误当作本轮基线。

上游移除条件：区域锚点、Attached 阅读归属、Masonry 用户接管、原生精确值／旧接口写入重置及局部 bounds／点击合同均有等价实现，并通过同样回归与性能对照。按已覆盖部分移除补丁，不保留两套 owner 并行。相关记录见 [Base 补丁](../../vendor/gpui-base/YUE-PATCH.md)与 [GPUI 补丁](../../vendor/gpui-pre/YUE-PATCH.md)。
