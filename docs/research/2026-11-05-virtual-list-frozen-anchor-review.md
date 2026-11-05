# 冻结 Grid 阅读位置对标与修复（2026-11-05）

基线为 `b934f22465ab9955455f461675d9a07bc1fdbfeb`，分支为 `quality/virtualization-four-priorities-20261104`。继续检查上一轮五项能力的组合边界，本轮确认并修复一个问题：冻结条带与正文共用普通视口起点锚点，尺寸反馈可能多补偿或漏补偿。采用依据是可复现的阅读跳动被消除，不是吞吐提速。

本文保留当轮修复与验证范围。后续布局与动画能力见[五项后续实施](2026-11-05-virtual-list-five-followups.md)，当前能力以[组件文档第 2.5–2.6 节](../28-virtual-list.md#26-布局选择与二维边界)为准。

## 三个参考项目提供了什么依据

本轮继续使用本地只读源码，不把历史候选记录中已交付的能力重新列为缺陷。

- [TanStack Virtual](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 的 `resizeItem` 区分尺寸变化是否位于阅读位置上方，以及用户方向／smooth／待确认命令；尺寸变化不应无条件等量移动滚动位置。`getOffsetForIndex` 另将 scroll padding 纳入定位区域。
- [virtua store](../../refer/virtual/virtua-main/src/core/store.ts) 使用 `getVisibleOffset`／`getAnchorIndex` 及 `ACTION_ITEM_RESIZE`，以实际阅读锚点的偏移差作 jump，方向缓冲与 idle 有独立生命周期。其中 `_frozenRange` 是手动 smooth 的测量范围冻结，**不是冻结表头／列**。
- [vlist core](../../refer/virtual/vlist-main/src/core/create.ts) 的 `remeasureItem` 在变更尺寸前取得 firstVisible，只补偿其前方变化；[Grid plugin](../../refer/virtual/vlist-main/src/plugins/grid/plugin.ts) 在更新布局前保存 anchorItem／inRow，更新后还原。[scroll-padding](../../refer/virtual/vlist-main/src/utils/scroll-padding.ts) 将避让区域传给布局插件，避免组合后丢失约束。

共同启示是“保留读者实际看到的位置，布局约束必须进入相应计算”。三个参考实现并不直接给出本项目双轴冻结合同；本轮未运行其 TypeScript 性能比较，也不据此声称本项目更快。GPUI 的修复留在 Base 原有轴状态内，继续由一个原生 ScrollHandle 应用两轴提案。

另外审阅了当前 Grid preparation、方向性 reserves、Masonry activity、cold-cache 验证与宽度失效路径。现有相关测试本轮重新通过；没有为这些路径增加新的生产缓存或重复交付上一轮能力。这是本轮审查范围，不是所有输入组合都无缺陷的证明。

## 失败基线和修复

原实现 `VirtualizerState::commit` 在 `viewport.offset` 捕获锚点。冻结后，此处可能属于被条带遮挡的普通轨道；真正的正文阅读前缘是 `offset + leading_occlusion`。原 `restore_anchor` 也没有扣除变化后的冻结条带。

两轴均使用 Known 尺寸、gap=1.5px、内容 padding=(2,3)px，冻结前三行／列；行高 20.5px、列宽 50.25px，原生 offset=(-213.25,-103.5)。修复前先添加并运行两项带断言的回归，两项均失败：

| 变化 | 修复前 | 修复后 |
| --- | --- | --- |
| 冻结轨道增大 7.25px | 两轴都多滚动 7.25px；首个可见行 key 7→8 | offset 保持；可见 key 及相对 body 的位置不变 |
| 遮挡区内普通轨道增大 7.25px | 漏补偿；阅读位置移动 7.25px | 两轴原生 offset 补偿 7.25px；阅读位置不变 |

固定 Release A/B workload 两轮都观测到上述冻结场景最大阅读误差 **7.25px→0**。这些是逻辑像素几何／原生位置观察，不是显示器物理像素或输入延迟。

修复仅调整 [state.rs](../../vendor/gpui-base/src/virtualization/state.rs) 的锚点坐标：在未遮挡 body 前缘捕获 key／项内位置，按新几何计算冻结条带并扣除；正文面积消失或原 key 进入冻结区时不套用该正文锚点。没有新增 retained 字段、逐项元数据、第二个滚动 owner 或公共 API。未冻结时遮挡为零，继续使用既有普通坐标。

## 本轮验证

[四项新回归](../../crates/yue-component/tests/virtual_list/frozen_grid.rs) 覆盖：冻结轨道反馈、被遮挡普通轨道反馈、正文消失／key 移入冻结区、gap／padding 变化与同值 no-op／旧快照失效。前两项同时验证两轴、分数尺寸、可见 key、阅读距离、原生 offset 和真实 GPUI click 命中。已有四区裁剪、焦点、准备 View 生命周期、方向、缓存及 smooth 测试随专项重新运行。

- workspace（含 doctest／Showcase headless）：**871 passed / 0 failed / 12 ignored**。
- virtual_list：**157 passed / 0 failed / 8 ignored**；冻结专项共 9 项通过。
- Clippy workspace/all-targets、`headless-tests,virtual-profile`、`-D warnings` 通过；workspace fmt 与本轮 vendor／workload rustfmt 通过。
- vendor 合同：**16 native + 2 Base** 通过，生产 manifests／locks 未改。

[Windows 原生用例](../../examples/windows-showcase/src/pages/virtual_layouts/native_tests.rs) 将既有冻结／冷恢复测试扩展为六阶段：定位 row 1000／column 100 后修改 gap=0.25px、padding=(2,3)px，要求实际原生位置为 y=36213.75、x=14280.75，且正文首 key 仍为 1000／100，再验证 Masonry 销毁重建。**3 项独立 Windows Release 原生用例全部通过**；结果与最终提交身份见下方机器证据。此用例不等同于人工截图验收或跨 GPU／DPI 测试。

## Release 成本：保留不利结果

[固定 workload](virtual-list-frozen-anchor-2026-11-05.rs) 在仓库外 fixture 用同一源码对比 Git archive baseline 与修复后 vendor。两轴 100k rows／1k columns、Known、240×100px，每场景 3 samples，每 sample 180 个显式 headless draw 周期，丢弃前 20 个；每轮 ABBA，每个版本共 6 samples。下表是 sample 总时长中位数变化，负数表示变快。分配是计数器观察，peak 是计时窗口内增量峰值，不是 RSS。

| 场景 | 第一轮 CPU | 复测 CPU | 160 周期中位数计数 |
| --- | ---: | ---: | --- |
| 未冻结 steady | −2.93% | +4.22% | 分配 65120→65120；builds 9600→9600 |
| 未冻结 scroll | −0.31% | +1.83% | 分配 65440→65440；builds 9600→9600 |
| 未冻结 prefix resize | −3.35% | +1.80% | 分配 68960→68960；builds 9600→9600 |
| 未冻结 hidden resize | −1.38% | +0.41% | 分配 66000→66000；builds 8800→8800 |
| 冻结 steady | −0.75% | +0.92% | 分配 69280→69280；builds 9600→9600 |
| 冻结 scroll | −0.78% | +3.26% | 分配 69600→69600；builds 9600→9600 |
| 冻结 prefix resize | −3.88% | +1.53% | 分配 70320→70160；builds 8800→8800；误差 7.25→0 |
| 冻结 hidden resize | **+8.82%** | **+10.44%** | 分配 70160→73120；builds **8800→9600**；误差 7.25→0 |

最后一项是明确代价：修复让正文保持正确位置，实际工作集随之变化，不能当成相同画面下的纯算法回退，也不能删掉它或包装为提速。当前选择接受稳定性修复，并完整公开更多 cell 布局／分配的成本。若以后优化，必须在相同正确几何及命中合同下比较，不能退回错误阅读位置来缩小工作集。

另外对既有默认 27 场景矩阵运行两轮 ABBA；所有场景的分配、peak、reads、builds 中位数均相同，CPU 有正负波动。保留的不利项包括 idle-2000 **+13.30%／+4.96%**、Masonry rapid **+4.17%／+4.54%**、Masonry size **+3.48%／+2.78%**。idle 原始区间相互重叠，单个 sample 总时长约 0.6–1.2ms，未调用本轮变更的 commit／restore 锚点路径；本轮没有因而宣称普遍零回归，也不把反号或无关路径的负值当成可归因收益。具体逐样本和 min/max 以 CSV／机器证据为准。

## 复现与回退

```powershell
cargo test -p yue-component --features headless-tests --test virtual_list --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1
cargo test --workspace --features headless-tests --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1
cargo clippy --workspace --all-targets --features headless-tests,virtual-profile --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- -D warnings
python tools/test-virtualization-contracts.py --offline --target-dir C:/Temp/gpui-yue-owner-perf-20261104
powershell -NoProfile -File tools/test-virtualization-native.ps1 -Offline -Release -TargetDir C:/Temp/gpui-yue-owner-perf-20261104
python tools/compare-virtualization.py --before b934f22 --label frozen-anchor --date 2026-11-05 --workload docs/research/virtual-list-frozen-anchor-2026-11-05.rs --filter frozen_anchor_workload
python tools/compare-virtualization.py --before b934f22 --label frozen-anchor-default --date 2026-11-05
```

两种性能命令均另以 `-repeat` label 独立运行一次；四组 identity 的 A/B 依赖 package 记录一致，编译干扰检查没有丢弃样本。保留四份完整 CSV，不修改生产 Cargo manifests／locks 或 `refer/virtual`。完整来源哈希、测试日志哈希、构建身份、统计与 Git 临时 index 回退核对见[机器证据](virtual-list-frozen-anchor-2026-11-05.json)。

实现／测试提交为 `d24340b194d54200d1d44050c67907996b33d77c`。已在独立临时 Git index 逆向应用该提交，得到 tree `6e2a21a3924f975ba4afc83743c74c84a31c3d5a`，与 `b934f22^{tree}` 完全一致；主 index 未变，临时 index 已删除。文档／数据在独立提交交付。撤销修复可用 `git revert d24340b`，不改写远端历史；后续如有依赖提交须先检查。测试和采样之后只改文档，不替换已测生产源码。

本轮修复没有扩展布局 API；当时尚未交付的 Grid 约束 auto、cell span、精确单 cell 保留及 RTL 已由后续实施补齐。一维 Measured 也已支持已测、有界短路径及显式末段动画；任意二维／三维空间布局、Grid Measured 轴与未知尺寸长路径的连续动画仍不在当前合同中。
