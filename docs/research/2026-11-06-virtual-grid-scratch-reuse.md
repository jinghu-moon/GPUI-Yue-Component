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

Grid draw P95 improved 8.6% at rem16 and 3.4% at rem20; P50 was effectively flat. Grid prepaint P95 fell 13.3% and 4.2%. The mixed Masonry/horizontal samples prevent a claim of broad speedup, and one completed ABBA is limited evidence. The second independent run was attempted with a new label but all three candidate samples were interrupted by concurrent compiler activity; the comparator retained those logs and stopped without producing a CSV. They are not counted as a successful repeat. The unmodified Grid workload, source hashes, dependency identity, executable hashes and run logs are in the comparator's external `C:/Temp/gpui-yue-series-20261104/grid-scratch-reuse-20261106/` directory.

## Decision and validation

Keep this narrow change: the measured Grid tail improvement is in the phase that allocates the repeated containers, while median draw is flat and the implementation has a fixed per-mount capacity ceiling. Treat it as reduced repeated scratch growth, not an across-the-board rendering speedup. Keep the raw CSV and the incomplete repeat logs; rerun a second protected ABBA when the machine is free before making stronger performance claims.

Validation on the candidate source: complete `virtual_list` suite **190 passed / 10 ignored**; frozen Grid's 13 production layout/hit/focus cases passed; `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy -p gpui-base --lib --offline --locked -- -D warnings` passed. A combined Base/Yue Clippy invocation was blocked by existing uncommitted `image_preview` casts (`unnecessary_cast`), outside this change. `refer/virtual` and production Cargo manifests/locks were not modified.
