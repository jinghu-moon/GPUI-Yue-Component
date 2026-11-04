# 虚拟滚动 1–8 项升级、回归与性能取舍

日期：2026-11-03。对应 [当前设计](../28-virtual-list.md)、[初次实施基线](2026-11-03-virtual-list-implementation.md) 与 [早期 Showcase 拖动采样](2026-11-03-virtual-list-showcase-drag.md)。参考对象为仓库内 `refer/virtual/virtual-main`，其中 virtual-core 版本为 3.19.10；这不是对线上最新版本的声明。

本次直接升级唯一实现，不新增兼容控制器、DOM 池、布局插件总线或另一套动态测量表。用户负责实际 UI 检查，本次没有启动、关闭、截图或操作真实 Showcase。

## 1. 逐项落地

| 项 | 原缺口 | 当前实现与入口 | 验证合同 |
| --- | --- | --- | --- |
| 1 | 滚动活动与程序命令混用，缺方向 | `is_scrolling` 表达实际活动；`has_scroll_command` 表达命令；`scroll_direction`、`ScrollStateChanged`；120ms idle，Entity 单个可取消任务，纯 headless 可轮询 | offset 前进／后退、活动结束、命令受理但未移动、用户输入取消命令、卸载取消 idle；命令终态不伪造 ScrollEnd |
| 2 | 任意重排丢失实际测量 | `VirtualizerSource::revision_at`；原生 `ListState::remap_items` 按 key/content 迁移实际尺寸和焦点；最后一项的 gap 单独校正 | 相同 key 换序保留，内容版本变化失效；旧快照冻结；新增／删除不错误继承末尾间距 |
| 3 | 统一估算、缺批量尺寸入口、Known 全量重建 | `measured_with_estimates`；`VirtualizerSizeUpdate`；`update_sizes` / `update_size_hints` / `update_sizes_from_snapshot`；Known 改为持久 f64 SumTree | 整批非法输入无部分写入、重复 key 拒绝、过期回调拒绝、真实测量／hint 标记明确；冻结快照下单项更新分配不随 N 线性增长 |
| 4 | Fixed/Measured 缺横向，跟随只在纵向动态路径 | 两轴三模式共用主轴合同；横向动态以 intrinsic width 和 definite height 测量；物理 padding、wheel、scrollbar、命中坐标正确；Fixed/Known 也支持 follow-tail | 改造前 Fixed/Horizontal 构造出现 Unsupported 的失败回归；改造后范围、动态宽度、横向点击、首尾定位和滚动条通过；尾部暂停／返回恢复 |
| 5 | 只保留焦点，无法自定义工作集 | `set_retained_keys`、`VirtualizerRangeExtractor`、`working_indices`；Measured 对接原生 retained items | 窗口外第 9,999 项能独立测量，不构建中间行；排序去重、非法 index/key 拒绝；保留项仍受预算约束 |
| 6 | 定位没有遮挡避让，外部宿主缺列表坐标原点 | `scroll_padding`、`with_content_origin`、`with_scroll_extent`；绝对 `scroll_to_offset` 与相对 `scroll_by` 共用版本化请求 | sticky 避让、Start/Center/End/Nearest、宿主 header/footer、两轴 Measured 的列表外 offset 不被钳回列表末尾 |
| 7 | 同一工作集里重复昂贵准备 | Base/Yue `new_with_item_views` 与 `content_revision`；mount 内按有效身份缓存 AnyView，继续走正常 GPUI 生命周期 | 同范围小偏移 factory 稳定为 0；内容 revision 变化重新准备；离开工作集和卸载释放 view handles；不缓存 AnyElement |
| 8 | 缺可恢复测量及正式 Grid/Masonry 呈现 | 纯值 `VirtualizerCache`；`VirtualGrid` 组合两轴 Attached；`VirtualMasonry` 稳定分栏与实际动态测量 | 跨 controller 恢复，旧布局拒绝，变更内容跳过；Grid 同宿主／帧尺寸配对、双轴合并、单轴替换不取消另一轴；Masonry 局部更新、显式重排、预算、非法尺寸与焦点删除修复 |

## 2. 所有权与复杂度

- Fixed 沿用 `uniform_list_range` 算术，百万项仍不分配逐项尺寸。横向支持没有退化成 N 项 Known 数组。
- Known 使用单份持久树，gap 保持在查询参数中。单项变更约 O(log N)，k 项约 O(k log N)；旧 snapshot 共享未变节点。当前逐项查询也是 O(log N)，工作集 W 的完整几何读取为 O(W log N)，不能把整帧写成 O(log N)。
- Measured 继续以原生 ListState/SumTree 为唯一测量 owner。横向只投影测量轴，child 的布局、焦点、hitbox 和 paint 坐标仍是物理桌面坐标。Owned 注册原生输入，Attached 不额外注册滚轮或写宿主 ScrollHandle。
- 逐项估算 callback 在初始化及带 callback 的完整 source replacement 中会访问 N 项；应只做廉价估计。简单 uniform splice 继续使用原生增量路径。重排、宽度／字体失效、source 替换均单独作为冷事务计量。
- `new_with_item_views` 只复用昂贵准备和当前工作集的 view handles。factory 身份包括 key、source 内容版本、layout revision、外部 content revision、index、start、extent 和 measured 状态；theme 等呈现依赖应在 view Render 中读取。退出缓存不保存业务编辑值，业务状态仍由宿主按 key 拥有。
- Grid 一个 ScrollHandle 拥有实际 offset；两根 state 只提供本轴请求。支持 Fixed/Known 行列，工作集为两轴交点。未呈现列可能改变行高的问题没有被隐式“测当前列最大值”掩盖。
- Masonry 首次最短列分配 O(N log L)，后续测量固定 lane，只改该列树路径。列数改变由 `reflow` 显式冷重排；宽度变化使动态尺寸重新估算。不存在按帧复制全量测量标记数组。列间距占满宽度或视口为空时不构建卡片。

## 3. Showcase

原「虚拟滚动」页保留八种按需挂载场景；动态卡片的字符串准备迁入 `PreparedCard` factory，主题仍在 Render 读取。

新增独立「虚拟布局」页面（`examples/windows-showcase/src/pages/virtual_layouts.rs`）：

1. Grid：1,000,000 行 × 1,000 列，按可见单元格构建，右下角定位和原生双轴滚动。
2. Masonry：10,000 张真实动态卡片，三／五列显式重排、末项定位。
3. 横向 Measured：2,000 个不同自然宽度节点，首／尾定位、原生横向输入。

只挂载当前 Tab。生产页面无头交互覆盖三个适配及真实按钮；未用真实窗口截图作为验收，也没有把 Fixed 的亿级交点等同亿级 Element。

## 4. 测试前后与复现

改造前 virtual_list 为 28 passed / 3 ignored；先增加 Fixed/Horizontal 回归并观察旧实现 Unsupported 失败，再升级原生接缝与所有调用方。新增合同位于：

- `crates/yue-component/tests/virtual_list/upgrades.rs`：1–6 与测量恢复。
- `crates/yue-component/tests/virtual_list/layouts.rs`：Grid/Masonry 的纯几何和真实生产 Element。
- `crates/yue-component/tests/virtual_list/preparation.rs`：准备复用、失效、释放及 A/B 采样。
- 原 `edges.rs`、`focus.rs`、SharedHost：滚动条拖动、命令取消、焦点和 Attached 组合继续回归。
- `vendor/gpui-pre/tests/virtualization.rs`：原生提示生命周期、预算、横向物理留白。

当前验证：virtual_list **48 passed / 5 ignored**；完整工作区 headless + doctest **762 passed / 6 ignored / 0 failed**，其中 Showcase 28 passed / 1 ignored，Select 单元测试 40 passed；原生 virtualization 5 passed；Base virtualization 2 passed。五个专项 ignored 均为显式性能探针，其中本轮手动执行两种升级 A/B 和几何分配探针。已有 ImagePreview 测试也通过；本次不修改其逻辑来迎合虚拟滚动测试。

常规验证命令：

```powershell
cargo test --workspace --locked --features yue-component/headless-tests,windows-showcase/headless-tests
cargo clippy --workspace --locked --all-targets --features yue-component/headless-tests,windows-showcase/headless-tests -- -D warnings
cargo fmt --all -- --check
rustfmt --edition 2024 --check vendor/gpui-base/src/virtualization/mod.rs vendor/gpui-pre/src/elements/list.rs vendor/gpui-pre/tests/virtualization.rs
tools/test-gpui-base.ps1 -Offline -Filter virtualization
cargo build -p windows-showcase --release --locked
```

原生集成测试使用 `target/gpui-virtual-upgrades-contract/Cargo.toml` 的一次性独立 fixture：只依赖本地 gpui-pre 的 test-support，测试入口指向 `vendor/gpui-pre/tests/virtualization.rs`。这避免发布 manifest 中无关 backend 的 dev-dependency 冲突；未修改 Cargo registry 或生产锁文件。

## 5. Release 性能证据

同一台 Windows x64 开发机，沿用前序记录的 i5-13600KF、rustc 1.98.1。三轮顺序运行，无并行基准；测试使用 headless 窗口。时序是 CPU 提交批或几何操作耗时，不是物理显示帧，也不包括 GPU 完成等待。

```powershell
cargo test -p yue-component --release --features headless-tests --test virtual_list upgrades_ -- --ignored --nocapture --test-threads=1
cargo test -p yue-component --release --features headless-tests --test virtual_list geometry_allocation_evidence -- --ignored --nocapture --test-threads=1
```

### 5.1 相同工作集的昂贵准备

百万项、240×100 逻辑像素视口、20px 行高。两种路径构建相同的 PreparedRow；factory 为每项准备 256 条不可变字符串。每轮 100 次 1–18px 小偏移，排除每组 sample 0，合计每路径 297 样本。

结果见下表及 [原始 CSV](virtual-list-preparation-upgrades-2026-11-03.csv)。数据只证明此负载中避免了重复准备，普通 Render、layout、prepaint 与 paint 仍然执行，不能称为“范围不变就零渲染”。

| 路径 | factory／稳定批 | P50 | P95 |
| --- | --- | --- | --- |
| 普通 renderer 每批重新准备 | 12 | 0.2715ms | 0.3227ms |
| 工作集 item views 复用 | 0 | 0.0784ms | 0.0897ms |

### 5.2 Known 单项更新

对比同一新版引擎的「完整尺寸／source replacement」与「单 key update_sizes」，两者都保留旧 snapshot，三轮每规模每路径共 30 样本。[原始 CSV](virtual-list-known-updates-2026-11-03.csv) 的 `live/peak` 为探针追踪的新增分配 payload，不是进程 RSS 或 GPU 内存。

| N | 完整替换 P50 / P95 | 单项更新 P50 / P95 | 更新最大新增 peak payload |
| --- | --- | --- | --- |
| 1,000 | 0.0085 / 0.0285ms | 0.0026 / 0.0257ms | 3,852 B |
| 100,000 | 0.7844 / 0.9379ms | 0.0057 / 0.0168ms | 6,604 B |
| 1,000,000 | 8.8034 / 9.6226ms | 0.0081 / 0.0178ms | 7,980 B |

这是同功能调用路径 A/B，完整 replacement 包含新输入数组和树构造，不是对旧前缀实现的逐指令比较；局部更新不应继续支付这项全量成本。单项 peak gate 小于 100KB，百万项实测仍为 KB 级；小样本 P95 会受调度／分配器抖动影响，不承诺实时上限。

### 5.3 常驻内存代价

[当前几何分配 CSV](virtual-list-alloc-upgrades-2026-11-03.csv) 与 [原共享数组／前缀的三轮基线](virtual-list-alloc-2026-11-03.csv) 使用相同线程局部分配探针口径，排除业务数据、Element、图片／纹理、系统分配器内部碎片。

| 百万项几何 | 升级前 retained payload | 当前 retained payload | 当前构造 peak payload |
| --- | --- | --- | --- |
| Fixed | 288 B | 304 B | 304 B |
| Known | 12,000,320 B | 31,274,376 B | 36,388,544 B |
| Measured | 93,095,208 B | 93,095,248 B | 94,211,336 B |

Known 从紧凑连续数组升级为持久树，换取冻结快照下的局部更新，必须承担树节点、摘要和引用计数的成本；不能只报告更新时间降低。尺寸恒定的数据继续使用 Fixed。Measured 仍保留原生 O(N) 元数据，没有声称百万动态项为常量内存。

## 6. 明确边界

- Measured smooth 仍保守即时降级，真实尺寸收敛优先；没有依赖未知高度播放长距离平移。
- Grid 不做未知列影响全局行高的推断；Masonry 为纵向 Owned 稳定分栏，不冒充完整 Table、Tree 或任意空间虚拟器。
- extra working set 的 sticky、拖拽／编辑展示和业务选择由消费者实现；RangeExtractor 不是样式引擎。预算不足发一次诊断，不靠无限动画帧逼迫重测。
- 反向布局、RTL 镜像、速度预测缓冲、浏览器 synthetic scrolling 和全局图片缓存不在本次交付。
- 用户曾观察快速拖动 CPU 由约 0.7% 升至最高 4.5%。本轮没有同负载的实窗性能计数，不能宣称已降低该百分比；采样结果只覆盖上述具体负载。

## 7. 维护与移除条件

原生补丁只扩展现有 ListState 的轴投影、retained、测量迁移与 headless seam。Base 拥有布局无关的源、版本、持久已知尺寸树、缓存值和适配器；Yue 拥有外观，未复制测量规则。记录分别见 [Base 补丁](../../vendor/gpui-base/YUE-PATCH.md) 与 [GPUI 补丁](../../vendor/gpui-pre/YUE-PATCH.md)。

上游具备等价原生接缝后，先用本项目 headless 合同与分配／CPU 探针对照，再删除被覆盖的本地补丁；不得仅凭相似 API 名称更换所有者或保留双实现。
