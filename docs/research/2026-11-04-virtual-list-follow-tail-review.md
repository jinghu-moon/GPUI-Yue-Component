# Masonry 尾部跟随：继续对标与验证（2026-11-04）

本轮基线为 `dc4fc69`，实现检查点为 `486b8b5`，沿用 `quality/virtualization-four-priorities-20261104`。实验于 2026-11-04 开始，最终验证于 2026-11-05（Asia/Shanghai）完成，文件名保留开始日期。决定吸收的是 Masonry 的 opt-in 尾部跟随能力；默认关闭。参考目录没有修改。

## 参考机制与采用范围

| 参考项目 | 本地源码依据 | 本项目采用的合同 |
| --- | --- | --- |
| TanStack Virtual | [`followOnAppend` 与 `isAtEnd`](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts)，`cancelScroll` 将程序意图交还用户输入 | 已在尾部才随 append；用户输入覆盖程序定位；尾部按当前布局重新解析 |
| vlist | [`appendItems`](../../refer/virtual/vlist-main/src/core/create.ts)，[reverse append 回归](../../refer/virtual/vlist-main/test/core/data-ops.test.ts) | 位于末端时追加继续贴底；回看历史时追加保留视口。其 reverse 布局没有直接移植 |
| virtua | [driver 的 offset 归一化、写入与取消路径](../../refer/virtual/virtua-main/src/core/driver.ts) | 继续只向唯一平台 owner 提交位置；测量后重新解析几何，不增加第二个 offset owner |

这里只阅读本地参考导出，没有运行三个 JavaScript 项目的 benchmark，也不比较它们与 GPUI 的绝对 FPS。TanStack/vlist 提供尾部跟随的直接行为依据；virtua 提供 driver/owner 边界依据，不能把它们表述为相同的公共 API。

基线 Masonry 的 `supports_follow_tail()` 为 false，无法表达持续跟尾；instant `scroll_to_offset` 只完成一次定位。当前 `set_follow_tail(true)` 请求进入尾部，`is_following_tail()` 读取活动状态，capability 改为 true。内部只增加常数规模的会话状态和本帧输入确认标记，没有逐项位置表、测量缓存或新滚动驱动。

## 行为合同与回归

- append 和真实尺寸修正使用同一个原生 `ScrollHandle` 重新解析当前几何尾部，已有 lanes 与测量所有权不变。
- 用户 native offset／wheel 离开尾部后暂停；追加不抢回历史视口。输入覆盖旧阅读锚点，尾部判定在本帧尺寸反馈和 clamp 之后再次确认。
- Measured 的尾部包含尚未测量的 hints；不为了跟尾测量全量内容，不承诺离屏项已经取得实际尺寸。
- 显式 key／offset 命令暂停已有跟随会话；即使命令落在尾部也不自行重启。没有会话时，命令不创建会话。原有命令校验与恰好一次终态保持不变。
- `set_follow_tail(false)` 和 source replace 结束会话；replace／显式命令清除待确认的旧输入。消费者仍在成功变更后通知其所属 Entity。
- Attached、reverse/RTL 和 smooth 仍未扩展；这次 capability 变化只声明 follow-tail。

新增[四项生产接缝回归](../../crates/yue-component/tests/virtual_list/masonry_follow_tail.rs)：

| 场景 | 可复核断言 |
| --- | --- |
| Fixed append 与暂停 | 30 项、3 lanes、20px item、100px viewport：开启时 offset 为 100，追加到 33 项后为 120；用户 native offset 改为 20 后再追加仍为 20 |
| source replace 与命令优先 | 替换源结束跟随；旧输入确认不恢复会话；显式 offset 到 20 覆盖跟随 |
| Measured append、真实 wheel 离尾/返回 | hints 40px、实际根 20px；append/校正后位于当前尾部；断言两个 wheel 回调确实发生且离尾输入确实减小 offset；暂停中追加保留 offset；返回新尾部恢复跟随；稳定后无持续续帧 |
| key／offset 与无会话追加 | key 和 offset 命令落在尾部也不重新开启；关闭会话后完成尾部命令，再追加不会形成隐式跟随 |

测试过程中进一步区分了两个事实：20px 输入后总高也可能因实际测量缩小 20px，因此“仍在尾部”不能单独证明输入被回滚；最终测试同时检查 native offset 的确变化和 wheel 回调计数。大幅 wheel 输入还可能先超出旧尾部再被 clamp，且同帧测量会继续改变总高，故活动跟随的确认使用最终几何，而不是只用输入前的尾部。明确命令的确认路径不会借此重新开启跟随。

这些是 GPUI headless UI integration tests，实际分发生产 wheel 事件链；不是物理滚轮、触控板或 OS scrollbar 的硬件录制。

## 前后性能守卫

使用既有[相同 Release workload](virtual-list-optimization-series-2026-11-04.rs)，将 Git 归档的 `dc4fc69` 与 `486b8b5` 的 vendor 源码分别构建到隔离 fixture。生产源码和 workload 在两轮采样期间冻结。每轮按 before/after/after/before 执行，编译守卫在每段前等待两次安静观察，并在采样期间轮询；两轮均没有被检测到编译干扰而丢弃的样本。

每轮 27 个 case、404 行原始记录，覆盖 Fixed/Known/Measured 一维、Masonry、稳定/快速/复杂/resize/尺寸轨迹、追加冻结快照、no-op、查询与 idle。Masonry 轨迹为 1 万项，逐样本去除 20 次预热，保留 120 次工作段；表中为每版本六个样本的工作段合计耗时中位数。单位为毫秒。

| Masonry 轨迹 | 首轮前 → 后 | 变化 | 独立复测前 → 后 | 变化 |
| --- | ---: | ---: | ---: | ---: |
| 稳定 | 9.1424 → 9.2813 | +1.52% | 9.6160 → 9.3431 | −2.84% |
| 快速跳转 | 8.8838 → 8.8931 | +0.11% | 8.6776 → 8.7131 | +0.41% |
| 复杂内容 | 21.2273 → 21.1199 | −0.51% | 20.3938 → 20.7729 | +1.86% |
| resize | 10.3234 → 10.3084 | −0.15% | 10.3391 → 10.2530 | −0.83% |
| 尺寸改变 | 8.2236 → 8.5658 | +4.16% | 8.3500 → 8.1805 | −2.03% |

两轮每个 case 的分配次数、临时峰值、source 查询数和构建数的逐样本取值集合均相同。追加 10 万项模型的单次微样本首轮 +29.02%、复测 −13.09%，反映该量级短工作段的计时不稳定，不能挑一轮宣称提升或退化。Masonry 五条轨迹也没有一致的耗时方向。保留功能的依据是行为缺口被填补、完整合同通过、默认路径没有新增确定性工作量；不宣称 CPU 加速。

原始数据：[首轮 CSV](virtual-list-series-follow-tail-guards-2026-11-04.csv)、[独立复测 CSV](virtual-list-series-follow-tail-guards-repeat-2026-11-04.csv)。源码/workload/数据哈希、采样身份和验证汇总见[机器可读记录](virtual-list-follow-tail-2026-11-04.json)。复现命令：

```powershell
python tools/compare-virtualization.py --before dc4fc69 --label follow-tail-guards --filter series_
python tools/compare-virtualization.py --before dc4fc69 --label follow-tail-guards-repeat --filter series_
```

该矩阵在默认未开启 Masonry 跟随的路径上比较新增状态/分支的回归成本。基线没有跟随接口，所以不把它当作“开启跟随后相同功能”的速度对照；开启会话的正确性由上面的追加、测量和输入用例证明，未声称开启状态比旧版更快。分配数据是测量线程窗口内 allocator payload，不是 RSS；墙钟工作段不是 CPU 使用率。后台非编译任务、热态和调度仍可能影响计时。

## 当前结论与边界

采用 `486b8b5`，补齐 Masonry 的主动尾部会话，默认值保持关闭。它继续使用 Base 行为与几何、Yue 呈现的层次，没有新依赖或第二滚动真值。`docs/28-virtual-list.md` 的能力矩阵、命令优先级与 `vendor/gpui-base/YUE-PATCH.md` 已同步。

本轮没有新增 GPU 完成时间、输入到显示延迟、实际屏幕 FPS 或跨 GPU/DPI 证据。后续优先考虑 Masonry 与一维 state 的 idle/direction 事件、纯值测量缓存协议的一致性；Attached、reverse/RTL、smooth 仍需要独立消费者和所有权合同，不能因参考项目具备而无条件加入。

## 最终验证与回退

验证源码始终为 `486b8b5`；机器记录核对当前六个改动源码/测试文件均与该提交一致，两个 ABBA 使用相同 workload SHA-256。

- 完整 workspace + doctest：842 passed / 10 ignored / 0 failed，其中 virtual_list 为 128 passed / 8 ignored。
- all-targets Clippy 同时启用 headless-tests 与 virtual-profile，`-D warnings` 通过；workspace fmt 和改动 vendor 文件 rustfmt 通过。
- vendor 原生层 15 项、Base 层 2 项合同通过；runner 核对生产 manifests/locks 未改动。
- Windows Showcase Release 构建通过；现有实窗布局 smoke 为 1 passed，验证 Grid、Masonry 与横向 Measured 的原生布局/定位，renderer 调用分别为 264/49/28。它没有执行新增的 Masonry append-follow 场景，不能代替上面的 headless 功能证据，也不是帧率测量。
- `git apply --reverse --check` 已验证实现提交可在当前源码逆向应用；只检查，没有实际回退。

主要验证命令：

```powershell
cargo test --workspace --features headless-tests --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1
cargo clippy --workspace --all-targets --features headless-tests,virtual-profile --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- -D warnings
python tools/test-virtualization-contracts.py --offline --target-dir C:/Temp/gpui-yue-owner-perf-20261104
cargo fmt --all --check
cargo build -p windows-showcase --release --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104
./tools/test-virtualization-native.ps1 -TargetDir C:/Temp/gpui-yue-owner-perf-20261104 -Release -Offline
```

实现与四项回归集中在 `486b8b5`，文档/CSV/身份记录另作同批提交。需要完整撤销时，先 revert 同批文档数据提交，再 `git revert 486b8b5`；这会同时恢复原 capability 与测试入口。基线 `dc4fc69` 保留在 Git 历史中，不使用 hard reset 覆盖工作区。
