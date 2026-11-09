# VirtualGrid 稳态 scratch buffer 复用（2026-11-06）

## 候选与范围

`VirtualGrid::request_layout` 每帧都创建空的 cell 容器；prepaint 按工作集逐个追加 `AnyElement`，冻结布局另追加同长度的裁剪 mask。稳定视口每帧重复支付容量增长。候选将两份空容量留在现有 keyed Grid mount 中，下一帧取回；paint 后清空 cell，释放本帧元素，再归还容量。超过 4096 项的容量直接丢弃，避免临时超大工作集把高水位常驻到 mount 生命周期结束。

只复用 `Vec` 容量；cell、renderer、布局、prepaint、hitbox、mask 内容和 paint 仍每帧按当前快照重建。mount 卸载时 buffer 随 mount 释放。没有增加公开 API、每 cell 状态或额外几何缓存。

## 真实窗口 ABBA

基线提交 `dfe3363`，after 为候选工作树。使用固定 `native_profile` Release workload，独立构建并固定可执行文件，四轮 ABBA；每个版本／场景 192 条 draw 样本，单屏 scale 1.5。P95/P99 为 nearest rank。原始数据：[ABBA CSV](virtual-list-series-grid-scratch-reuse-20261106-2026-11-06.csv)。

| 场景 | before draw P50/P95/P99 ms | after draw P50/P95/P99 ms | before → after prepaint P95 ms |
|---|---:|---:|---:|
| Grid rem16 | 2.857 / 6.749 / 7.905 | 2.883 / 6.167 / 6.980 | 4.749 → 4.119 |
| Grid rem20 | 3.390 / 7.162 / 8.077 | 3.383 / 6.918 / 7.861 | 4.840 → 4.636 |
| Masonry rem16 | 1.226 / 3.174 / 4.045 | 1.193 / 2.795 / 4.700 | 1.377 → 1.291 |
| Masonry reflow rem16 | 1.345 / 3.172 / 3.865 | 1.282 / 2.227 / 3.403 | 1.603 → 1.145 |
| Masonry rem20 | 1.330 / 3.039 / 4.302 | 1.280 / 3.354 / 3.679 | 1.355 → 1.482 |
| Masonry reflow rem20 | 1.298 / 3.232 / 4.161 | 1.235 / 2.484 / 3.095 | 1.523 → 1.176 |
| Horizontal rem16 | 0.960 / 2.128 / 2.639 | 0.882 / 1.913 / 2.457 | 0.871 → 0.762 |
| Horizontal rem20 | 0.809 / 1.933 / 2.355 | 0.872 / 1.986 / 2.668 | 0.777 → 0.814 |

Grid draw P95 improved 8.6% at rem16 and 3.4% at rem20 in the first run; P50 was effectively flat. A second protected ABBA completed from a clean worktree at `52ded83`, using the same workload and dependency records. Its Grid draw P95 changes were smaller (1.5% at rem16 and 0.7% at rem20), while prepaint P95 was flat or slightly worse. The second run therefore does not reproduce a clear prepaint gain. Both runs support a small Grid tail improvement, but not an across-the-board or median speedup. Some attempts were discarded after the comparator observed background compilation; only complete four-round runs are counted. The unmodified workload, source hashes, dependency identity, executable hashes and logs are in `C:/Temp/gpui-yue-series-20261104/`.

The independent run's raw samples are [here](virtual-list-series-grid-scratch-clean-repeat-2026-11-06.csv). Its Grid measurements are:

| Scenario | before draw P50/P95 ms | after draw P50/P95 ms | before → after prepaint P95 ms |
|---|---:|---:|---:|
| Grid rem16 | 2.837 / 5.429 | 2.841 / 5.350 | 3.459 → 3.574 |
| Grid rem20 | 3.351 / 7.035 | 3.340 / 6.984 | 4.612 → 4.626 |

## Allocation attribution

An ignored Release probe measured the common unfrozen 20×20 Grid snapshot paths 500 times each, after snapshot construction. Across two baseline runs, `working_items()` allocated zero times on either axis. `working_indices()` allocated once per call with a 160-byte peak. `cells()` allocated four times per iterator setup with a 2,048-byte peak; consuming 500 full iterators took about 3.2 ms total. The probe isolates snapshot iteration; it does not count frozen partitioning or whole-frame GPUI allocations.

A candidate `size_hint` experiment reduced `cells()` setup from four allocations (2,048-byte peak) to one allocation (1,280-byte peak) for this 20-column case. The full iterator time was effectively unchanged. Its native ABBA was mixed: at rem16, Grid draw P95 changed 5.976 → 7.074 ms and prepaint P95 3.901 → 4.768 ms; at rem20, draw P95 changed 7.227 → 7.096 ms and prepaint P95 4.791 → 4.785 ms. Median draw was nearly flat. A second ABBA was interrupted by background compilation on all three candidate attempts and produced no valid result. This candidate is **rejected** and its production change was removed; the observed allocation reduction does not justify the rem16 tail regression risk. The raw completed candidate run is [here](virtual-list-series-grid-workingsizehint-2026-11-06.csv); its run identity and rejected attempts remain under `C:/Temp/gpui-yue-series-20261104/`.

## Decision and validation

Keep the scratch buffer reuse change with limited measured benefit: two protected native ABBA runs show small Grid draw-tail improvements, but the prepaint reduction did not reproduce and median draw stayed flat. Do not adopt the `working_items` size hint candidate. The next useful step is to profile a real heavier Grid consumer before targeting other temporary vectors; current evidence does not justify additional production changes.

Validation: complete `virtual_list` suite **190 passed / 11 ignored** in the clean worktree with the probe added; the allocation probe passed twice before the rejected candidate and once with it. The scratch-buffer candidate's prior checks are recorded above. The native repeats ran from clean worktrees because unrelated uncommitted `image_preview` edits in the user's worktree currently fail to compile (`YueImagePreviewState::gallery` is private). Those edits were preserved and not staged. `refer/virtual` and production Cargo manifests/locks were not modified.
