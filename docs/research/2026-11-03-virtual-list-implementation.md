# 虚拟滚动基础层实现、回归与性能记录

后续状态：本文保留初次实施时的结果与限制；横向动态、增量 Known、工作集扩展、缓存恢复及 Grid/Masonry 已在 [1–8 项升级记录](2026-11-03-virtual-list-upgrades.md) 补齐，最新全工作区回归也在该记录中。不要将本文的历史能力矩阵或 ImagePreview 失败记录当作当前结论。

日期：2026-11-03。对应 [设计文档](../28-virtual-list.md) 与 [改造前源码调研](2026-11-03-virtual-list-source-review.md)。当前状态：基础层、Yue 适配、Select 迁移与 Showcase 已实现；虚拟滚动专项合同通过；共享工作区的最新完整回归有一项并行 ImagePreview 新测试失败，详见第 3 节。实窗观感、GPU 与 Windows 屏幕阅读器体验由用户检查。

## 1. 已交付结构

| 所有者 | 实现 | 职责 |
| --- | --- | --- |
| GPUI core | `vendor/gpui-pre/src/elements/uniform_list.rs` | 原生 UniformList 与 Base 共用 `uniform_list_range`，已知几何无需代表项测量 |
| GPUI core | `vendor/gpui-pre/src/elements/list.rs` | 唯一 Measured SumTree、持久不可变几何、可借用宿主的真实布局批次、hint/锚点/焦点/预算/滚动条修复 |
| Base | `vendor/gpui-base/src/virtualization/{mod,geometry,state,element}.rs` | source/sizing、局部 binding、版本化 snapshot/request、锚点、命令、事件及 Owned 一维 Element |
| Yue | `crates/yue-component/src/virtual_list.rs` | 纵向 YueVirtualList、主题与可覆盖的 List/ListItem 语义；不镜像状态 |
| Select | `crates/yue-component/src/select/{model,state,panel}.rs` | 复用同一公共状态；保留查询、active/selection、Popup 与异步票据等业务职责 |
| Showcase | `examples/windows-showcase/src/pages/virtual_list.rs` | 八场景：Fixed、Known、Measured、横向时间轴、多列卡片、编辑保留、追加日志和空状态；Tabs 仅挂载当前场景 |

旧 `vendor/gpui-base/src/virtual_list.rs` 及 v/h 构造器已删除，不保留另一套兼容引擎。源码与调用方没有旧 v/h/VirtualListScrollHandle 入口。Base 根和 `virtualization` 导出机制，Yue 根和 prelude 导出消费者入口。

### 1.1 支持矩阵

| 模式 | 主轴 | Owned | Attached |
| --- | --- | --- | --- |
| Fixed | Vertical | 支持 | 支持 |
| Known | Vertical / Horizontal | 支持 | 支持 |
| Measured | Vertical | 支持 | 支持 |
| Fixed / Measured | Horizontal | 构造前明确拒绝 | 构造前明确拒绝 |

Attached 不分配 ScrollHandle、不注册 wheel、不写宿主 offset。两个主轴通过同一个局部 binding 借用真实宿主：一个输入 owner，一次合并/clamp/write，独立轴分量不会互相覆盖。SharedHost 测试直接消费两轴快照与 Measured 原生批次，没有嵌套两个滚动列表或复制测试引擎。

Fixed/Known 可纯查询、不需要 renderer/Window/主题/role。Measured 未测与异常回退均保留 estimated；目标已收敛可以结束命令，而离屏总尺寸仍保持 estimated。Follow-tail 只支持 Measured。

### 1.2 已落实的边界

- 身份：业务稳定 key、正反索引互逆检查，原生元素命名空间包含条目 key；替换数据后 pointer-up 不会提交到同 index 的新行。
- 锚点：key + 绝对行内偏移；prepend、删除、替换和重排后恢复；focus 保留独立于连续窗口。
- 版本：source generation、geometry、viewport/input、binding revision 与 state 身份同时校验。过期输入不能重新签发请求；请求至多接受一次；旧 snapshot 仍可读取其冻结几何。
- 生命周期：挂载由 WindowId + GlobalElementId 校验；相同 ElementId 切换 Entity 或 binding 后更新观察与释放责任。旧命令终态仍发出，卸载取消 timer/frame/request。
- 焦点：保留真实 YueInput 的编辑值与 focus；删除时检查实际当前 focus，不依赖上一帧缓存。备用焦点或 blur 在正确阶段处理。
- 命令：start/center/end/nearest；短距离 smooth 使用明确 duration、ease-out cubic 和一倍视口上限。Measured、长距离、reduced motion、隐藏/非活动窗口即时定位；真实 wheel/scrollbar 会打断。
- 有界工作：Measured 每 pass 默认 1,024 次布局，Owned 最多两次 pass；预算可显式调整。无进展、超过 32 次 prepare 或 2 秒活动采样的 Measured 命令有 NoProgress 终态。
- 通知：range 按 generation/范围合并；用户活动合并为单个可取消弱 Entity 的 120ms idle task。非法尺寸和预算耗尽按连续失败/generation 去重，不自激重测。
- 间距：gap 只计 N-1 次。Measured 初始/新增估算、末项边界变化和实际布局共用原生树；条目查询排除 gap。scrollbar 内容范围包含首尾 padding，拖拽期间使用原生冻结映射。

## 2. 改造前基线与失败回归

基线提交：`4846e89`。调研文件保存原始关键源码 SHA-256；下面的旧行为是改造时实际执行到的结果，并非把新 API 在旧版本无法编译当作失败。

| 缺口 | 改造前 | 改造后保留的回归 |
| --- | --- | --- |
| Known gap 缓存 | 同一 sizes backing，gap 由 0 改 10，第 8 项仍位于 160px，预期 240px | `known_gap_revision_and_frozen_snapshots_have_one_consistent_geometry`；旧 Base 的 8 项原生规则测试在迁移前通过修复后再删除旧实现 |
| Measured 首帧 hint | 1,000 × 20px、60px 视口，原生最大 offset 只有 20px，预期 19,940px | 原生 `native_hints_survive_first_layout_resize_splice_and_snapshot_reads` 及 Yue 原生对照；首帧、splice、宽度变化保留 hint |
| 异常测量状态 | 零高 renderer 的 64 个回退槽位被 counted 为 measured，预期 0 | `invalid_zero_height_cannot_pull_all_items_into_one_frame` 先失败：left 64 / right 0；修复后 measured_count=0，item.is_measured=false |
| Measured gap 估算 | 加入 gap/padding 的 follow-tail fixture 得到总高 2,035px，预期 2,520px | `owned_measured_padding_gap_tail_shrink_and_zero_viewport`，当前总高/offset/scrollbar/空白间隙一致 |
| 既有 Select | 迁移前 headless 40/40 | 迁移后 40/40；搜索、分组、离屏导航、disabled、Popup、异步、主题/rem 等既有断言保留 |

定位到末尾、图像内容晚到失效的版本检查、局部展开、焦点保留和两个轴共享一次原生输入均走生产接口，不使用 mock 范围引擎。

## 3. 自动化验证

基础层实施阶段的检查结果如下。后续 Showcase 扩展、拖动输入回归与 debug/release 采样另见 [扩展与拖动记录](2026-11-03-virtual-list-showcase-drag.md)，不要将不同时点的完整命令总数混为当前全量结果。

| 命令 / 范围 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过；被修改的 vendor 源文件另用 rustfmt 检查 |
| `cargo test --workspace --locked` | 233 passed |
| `cargo test --workspace --locked --features headless-tests --no-fail-fast` | 最新 732 passed / 1 failed / 3 ignored；失败是下述并行 ImagePreview 新测试 |
| `cargo test -p yue-component --locked --no-default-features --features headless-tests --no-fail-fast` | 最新 711 passed / 1 failed / 3 ignored；同一 ImagePreview 新测试 |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | 通过 |
| 同上增加 `--features headless-tests` | 通过，包含新增集成测试与 Showcase 测试 |
| Yue `--test virtual_list` | 28 passed + 3 ignored；三个探针另在 release 连续三轮运行，各 3/3 passed |
| Showcase virtual-list 页（基础层初次实施） | 1 passed；实际列表、统计、百万项尾部、头插与卡片展开；八场景扩展另见新记录 |
| Base `tools/test-gpui-base.ps1 -Offline -Filter virtualization` | 2 passed |
| core `--test virtualization --features test-support` | 4 passed |

732/711/233 是对应完整命令内各 test binary 与 doctest 的 passed 总和，彼此包含重叠用例，不相加声称独立测试数。专项包括所有 Owned 模式、Known 横轴、Measured Attached、共享宿主对角 wheel/同时定位、真实 Input/focus、scrollbar、smooth 中间帧/重定向/用户打断/隐藏恢复、源替换后的 pointer-up、相同 ElementId 换 Entity、过期版本及非法配置。

为运行这些验证，`tools/test-gpui-base.ps1` 的临时清单补齐了本仓库 gpui-pre-windows patch，使其与工作区同一 core/backend，避免误连注册表旧版而出现 PolychromeFloat 等不匹配。临时清单与锁文件均位于 target。全量 lint 暴露的已有 ImagePreview 测试临时 Vec 指针警告，仅改为保留 owned Vec 后比较地址，断言语义不变，未改图像功能。

首轮完整回归曾得到 headless 731/731、no-fonts 710/710；随后共享工作区新增 ImagePreview paint-only 接口/测试。2026-11-03 21:50 的 no-fail-fast 全量复查中，其 `paint_only_preview_shares_encoding_geometry_and_sampling_without_chrome` 在 `crates/yue-component/tests/image_preview.rs:1660` 断言失败：sprite width 为 481，预期 482。完整命令返回失败，不能据专项通过宣称整个最新工作区通过；该独立功能和测试的并行改动保留，未通过改断言或跳过测试掩盖。所有测试 target 都执行完毕，虚拟滚动 28 项、Select 40 项和 Showcase 均通过。

公开归档的 core 全量 lib-test 另有缺失字体文件与测试宏问题；本轮没有顺带修改这些无关基础设施。显式 `virtualization` integration target 测试的是同一生产 library，不声称已跑通 core 所有上游内部单测。

## 4. Release 性能与分配证据

### 4.1 环境、采样和统计口径

- Windows x86_64，Intel Core i5-13600KF，20 个逻辑处理器；rustc 1.98.1 / LLVM 22.1.8；工作区 Cargo.lock，release optimized。
- Headless GPUI TestAppContext，240 × 100 逻辑像素视口；固定模式槽高 20px；零 overscan；50 个 warm 样本加 1 个 cold 样本/场景，共 18 个场景，连续三轮。
- [CPU 原始样本](virtual-list-cpu-2026-11-03.csv)：2,754 行，含 run、mode、N、子元素数、长卡片标识、sample、批次耗时和四阶段调用/耗时。下面排除 sample=0，使用 P50、nearest-rank P95/P99；重复的 Fixed 100K 简单场景合并为 300 个 warm 样本，其余每组 150 个。
- `batch_ns` 包括命令提交、通知及显式 draw/scene clear；fixture 的一个采样批次通常有两个 GPUI 描述/提交周期，所以五个工作项会记录 10 次 build。按实际批次数记录，不人为除以二或称为真实显示帧。
- request 指 Element::request_layout 的描述/分配阶段，不等同于完整 Taffy 布局耗时；prepaint/paint 计入被包装内容。主循环还包含 native layout、版本校验及文本/框架工作，四阶段时间不能相加冒充完整剖析。
- 原生对照使用相同文字/子元素、稳定 key 根、同一外层 Div 与 test_support 包装。mode 3 为原生 List，mode 4 为原生 UniformList；mode 0/1/2 分别是 Base Fixed/Known/Measured。原生 UniformList 有额外代表项/工作项构建，原始计数保留该差异。
- 时序数据是本机 CPU 提交基线；文本走测试平台，未测实际 DirectWrite、DPI/rem 排版、图片解码、纹理、GPU、合成、显示刷新或输入到显示延迟。机器不是专用基准环境，计数/分配更适合断言，时序用来定位量级和复测趋势。

### 4.2 稳定工作集与复杂度

| mode | N | 子元素/项 | 长卡片 | 样本 | P50 / P95 / P99 批次 (ms) | build/request/prepaint/paint 中位数 |
| --- | ---: | ---: | --- | ---: | --- | --- |
| 0 | 1000 | 1 | false | 150 | 0.0919 / 0.1304 / 0.1641 | 10/10/10/10 |
| 0 | 10000 | 1 | false | 150 | 0.0829 / 0.1307 / 0.1863 | 10/10/10/10 |
| 0 | 100000 | 1 | false | 300 | 0.0814 / 0.1288 / 0.1441 | 10/10/10/10 |
| 0 | 1000000 | 1 | false | 150 | 0.0808 / 0.1118 / 0.1775 | 10/10/10/10 |
| 0 | 100000 | 20 | false | 150 | 0.6214 / 0.7365 / 0.8570 | 10/10/10/10 |
| 1 | 100000 | 1 | false | 150 | 0.0791 / 0.1454 / 0.2296 | 10/10/10/10 |
| 1 | 100000 | 20 | false | 150 | 0.6222 / 0.7513 / 0.8249 | 10/10/10/10 |
| 2 | 100000 | 1 | false | 150 | 0.1020 / 0.1162 / 0.1862 | 10/10/10/10 |
| 2 | 100000 | 20 | false | 150 | 0.6429 / 0.7690 / 0.8740 | 10/10/10/10 |
| 3 | 100000 | 1 | false | 150 | 0.0824 / 0.0961 / 0.2331 | 10/10/10/10 |
| 3 | 100000 | 20 | false | 150 | 0.6211 / 0.7731 / 0.8845 | 10/10/10/10 |
| 4 | 100000 | 1 | false | 150 | 0.0922 / 0.1287 / 0.2035 | 14/14/10/10 |
| 4 | 100000 | 20 | false | 150 | 0.7929 / 1.0097 / 1.4248 | 14/14/10/10 |
| 2 | 100 | 1000 | true | 150 | 5.0074 / 5.8235 / 6.1535 | 2/2/2/2 |
| 2 | 100000 | 1 | true | 150 | 0.1003 / 0.1607 / 0.2061 | 10/10/10/10 |
| 3 | 100 | 1000 | true | 150 | 5.0678 / 5.7876 / 6.6032 | 2/2/2/2 |
| 3 | 100000 | 1 | true | 150 | 0.0793 / 0.1415 / 0.2422 | 10/10/10/10 |

结论与可证伪边界：

1. Fixed 1K→1M 的 build/request/prepaint/paint 中位数均为 10/10/10/10，没有随 N 增长。另有 [纯查询原始记录](virtual-list-query-2026-11-03.csv)：每种 N 每轮 20,000 次快照查询，含 1B 项压力输入；最大工作项始终 5。
2. 固定 N 和几何，子元素由 1 增至 20，根构建数不变，但 Base Fixed 批次 P50 约 0.081→0.621ms。虚拟化没有消除卡片内部工作；增加缓冲不能解决这类热点。
3. 同样 100,000 条逻辑文字，100 张各含 1,000 个子项的超高卡片，与展平的 100,000 个独立根对照：Measured P50 约 5.007ms 对 0.100ms。减少工作项数不一定减少工作量，拆分粒度属于业务语义层；这个结果不支持自动递归拆卡片。
4. 新 Measured 比原生 List 简单组多约 0.020ms 批次 P50，20 子元素组多约 0.022ms。版本化 source/绑定/提交接缝有可测固定成本，不能宣称“零开销”。在当前 fixture 中未引入随 N 增长的热路径或重复全量内容工作。
5. 据这三轮数据固化当前复测预算：同 fixture 的新增 Measured 辅助成本，P50 差值不超过 0.05ms，复杂组不超过原生 10%；Fixed 固定工作集计数不随 N 线性增长。此预算不外推至任意分辨率、行复杂度或物理 GPU。

真实换行、YueButton/YueInput、局部高度改变和 key 保留由功能集成测试及 Showcase 验证；这些合成时序不代替真实长文、图像/模糊资源和物理帧率场景。

### 4.3 真实 heap payload 与冷启动优化

[改造前分配记录](virtual-list-alloc-before-2026-11-03.csv) 与 [改造后连续三轮分配记录](virtual-list-alloc-2026-11-03.csv) 使用测试程序的 thread-local System allocator 计量。scope 覆盖 sizing 与 plain VirtualizerState 的构造和释放；source 在外部创建并继续保留，不计业务数据。Known 输入尺寸数组与 prefix 计入，Measured 只计唯一 native tree 的实际分配，不复制第二个 Map。

| 模式 / N=1M | 构造后保留 payload | 构造期 peak payload | 分配调用 |
| --- | ---: | ---: | ---: |
| Fixed | 288 B | 288 B | 7 |
| Known | 12,000,320 B | 12,000,320 B | 10 |
| Measured 改造前 | 93,095,208 B | 269,094,128 B | 181,910 |
| Measured 改造后（三轮相同） | 93,095,208 B | 94,211,312 B | 90,950 |

Measured 的实际热点是 `ListState::new(...).with_uniform_item_height(...)` 先建未知树、再分配估算树。新增原生 `new_with_uniform_height` 从同一构造路径直接带 hint 创建一次树；Base 使用它，普通 List 与旧 builder 仍有效，没有为冷启动新建缓存。等价几何与后续 splice 有原生回归，释放后计量 scope 的 live payload 返回 0。

这使百万项构造峰值减少约 65%，分配调用近乎减半，保留的原生 O(N) 元数据并未减少。单次旧样本约 102ms；改造后三轮约 20.7–21.1ms。时序没有独占机器和多轮旧基线，不能把这一组记录当成严格置信区间或所有设备的加速比；分配与几何等价是更直接的证据。

这里的 payload 不包含 allocator 分桶/页、栈内 state、Entity、源数据、窗口、文本缓存、Element 树、GPU 或纹理；不是 RSS/Private Bytes/VRAM。Fixed 的常量数值仅证明虚拟器自身没有逐项尺寸分配，不能宣称整个应用恒定内存。Known/Measured 的百万项元数据成本应纳入使用者预算。

### 4.4 复现

```powershell
cargo test -p yue-component --locked --release --features headless-tests --test virtual_list -- --ignored --nocapture --test-threads=1
pwsh -NoProfile -File tools/test-gpui-base.ps1 -Offline -Filter virtualization
cargo test --manifest-path vendor/gpui-pre/Cargo.toml --offline --features test-support --test virtualization --config profile.dev.debug=0 --target-dir target
```

性能测试默认 ignored，避免普通 CI 以不稳定 wall-clock 阈值失败；工作集、身份、预算和几何等确定性断言仍在常规集成测试中。原始 CSV 完整保留冷样本和阶段计数，不只保存摘要。首轮临时日志位于 target，CSV 是仓库内持久证据。

## 5. 明确限制与维护条件

- 首版是一维主轴机制，不是完整 Table/Grid/Tree/瀑布流；双轴验证使用独立、已知的行列尺寸，没有假设未呈现列能决定最终动态行高。
- 当前验证与补丁面向项目的 gpui-pre 生产后端；未将 Fast/wasm 或其他物理平台列为已验证交付。
- 显式注册 F 个交互 focus scope 时，焦点检查为 O(F)。宿主应随交互工作集注册/注销，不为所有逻辑项预建 Entity/FocusHandle；虚拟器不会自动为 N 项分配焦点对象。
- Fixed/Known 纯几何使用 f64；原生 ScrollHandle/Pixels、Measured SumTree 为 f32。验证包含 Fixed 1M×20px 的实际首尾定位，1B 项仅为 headless 查询；未实现任意巨大坐标的分段原生滚动，不能外推亚像素精度。
- 极密/异常视口的 Measured 预算耗尽会诊断并保留估算。默认不自动扩大工作集或不停 request_frame；消费者必须调整预算、有限视口或条目粒度。零预算与 estimate+gap 非法先拒绝。
- 图片、纹理、背景模糊、网络分页、昂贵文本预处理均由资源/业务 owner 拥有；当前证据不宣称这些成本已虚拟化。
- 实窗观感、硬件 GPU/刷新率、Windows Narrator/UIA、实际 DPI 字体与远程桌面不在上述自动化结果范围；不主动启动 Showcase 或接管用户 UI。
- Base 来源为本地 gpui-base 0.8.0，core 来源为 gpui-pre 0.3.9，具体 upstream snapshot 见各自 YUE-PATCH.md。当前必须保留根 Cargo patch，并让临时测试清单使用同一 core/Windows backend。
- 只有上游具有等价的共享 uniform 算术、冻结原生测量几何、外部宿主批次、hint/绝对锚点/稀疏焦点合同，以及来源/绑定/单次请求协议后，才逐项移除对应补丁；移除时重跑本文的 source/几何、Owned/Attached、Select、Showcase 和完整工作区回归。不得通过恢复旧引擎绕开新合同。
