# 虚拟滚动第四轮对标：阅读位置、宿主边界与输入所有权

后续状态：本记录保留修复前事实和原始数字。阅读锚点、Attached 边界、Masonry 输入接管及 Fixed/Known 精确输入链已完成[修复与回归](2026-11-04-virtual-list-owner-fixes.md)；Measured 累计树的 f32 边界仍在，不覆盖下文当时的观察结论。

日期：2026-11-04。基线是[第三轮修复](2026-11-04-virtual-list-followup-fixes.md)后的生产实现，HEAD `2f7d43bc78be188655fe6a9488a163a9e42c769e` 加现有未提交改动。本轮新增观察探针和设计约束，**未修复下述生产问题**，不覆盖此前的失败证据和实施记录。

结论：目前最需要补的不是更多布局类型，而是让阅读位置、程序定位与用户输入始终服从同一个滚动 owner。确认三类正确性缺口：空白区域锚点失真、离屏 Attached 列表产生越权校正、Masonry 测量／待校正定位覆盖新输入；另量化了此前已知的大坐标输入精度边界。持久缓存与局部失效的关系归为合同澄清，不计为第四个新缺陷。

## 1. 范围与证据

| 对象 | 本地版本 | 本轮阅读重点 |
| --- | --- | --- |
| TanStack Virtual | virtual-core 3.19.10 | 有符号阅读偏移、测量批次中的当前位置、命令写入与回读分类 |
| virtua | 0.54.1 | 宿主位置／起始占位／相对位置分离、相对补偿、可保留的停止状态 |
| vlist | 3.3.4 | 逻辑位置与局部原生坐标、输入驱动的取消、snapshot 恢复边界 |
| 本项目 | 第三轮修复后的工作区 | 三模式 × 两轴、Attached 离屏区、生产 Masonry 原生输入、生产 Fixed/Known 小增量输入 |

版本来自各本地 `package.json`，不表示上游最新。本轮没有运行参考库的 JavaScript 测试，也没有跨语言性能排名。

- [独立观察源码](virtual-list-owner-review-2026-11-04.rs)：调用生产公共接口；Masonry 通过 GPUI 原生事件分发进入真实 wheel listener，另有显式 native-owner 位移对照。
- [68 条原始记录](virtual-list-owner-observations-2026-11-04.txt)：6 个 Release/headless 观察测试。`passed` 只表示观察完成，不能解释为被观察行为正确。
- `mode=0/1/2` 分别表示 Fixed/Known/Measured；精度测试只覆盖 Fixed/Known。所有位移均为逻辑像素。

## 2. P1：padding 与 gap 被压缩成项内位置

### 2.1 复现

100 项，每项 20px、gap 10px、首尾 padding 各 10px。先 commit 阅读位置，再仅在尾部追加一项并 prepare；视口前方几何没有改变，应该保留原位置。

| 原位置 | 所在区域 | 追加后提议位置 | 问题 |
| --- | --- | --- | --- |
| 5 | 起始 padding 内 | 10 | 向后跳 5px |
| 25 | 第 0 项内容内 | 25 | 正常对照 |
| 35 | 第 0 项与第 1 项之间 | 30 | 向前跳 5px |

两轴、三种 sizing 的 18 条观察均得到上述结果，其中 12 条发生位移。

### 2.2 根因与参考

[state.rs](../../vendor/gpui-base/src/virtualization/state.rs) 的 `commit` 选择条目后，把相对位置限制为 `(viewport.offset - item.start).clamp(0, item.extent)`。顶部留白中的负偏移与 gap 中超过内容 extent 的偏移都丢失；`restore_anchor` 又用 `offset.min(extent)` 恢复。几何范围查询正确不能证明锚点表达完整。

TanStack 的 [setOptions](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 在 `anchorTo: 'end'` 的结构变化路径保存 `scrollOffset - item.start`，恢复时限制最终可达 offset 的下界，不先压到项内 extent。这证明“阅读位置相对某项”可以带符号；该分支有自己的启用条件，不能宣称它覆盖本项目全部模式。

vlist 的 [snapshot plugin](../../refer/virtual/vlist-main/src/plugins/snapshots/plugin.ts) 保存绝对 scrollTop、项内偏移和比例，在总数匹配时优先用绝对位置；其项内偏移同样有 clamp。不能把任何参考库的 snapshot 直接当作通用 gap/padding 方案。

### 2.3 最小修复方向与门槛

- 让锚点能表达内容区域及相邻空白的真实相对距离；可优先评估选择相邻边缘并保存有符号偏移，只有不能表达必要事实时再增加区域状态。
- 项内阅读、gap、padding 各自明确缩短／变化规则。不能简单删除全部 clamp，导致真实内容缩短后保留无效项内位置。
- 最后由实际滚动 owner 按可达范围规范化；不在 renderer 添加像素补丁。
- 同步检查 portable cache 的 anchor 校验：当前 [cache.rs](../../vendor/gpui-base/src/virtualization/cache.rs) 拒绝负 offset。若新合同引入有符号锚点，缓存语义、版本和恢复测试须一起调整。

改前测试须复现上述 5→10、35→30；改后同样的尾部追加应分别保留 5、25、35。再覆盖跨轴、三模式、gap/padding 自身改变、项缩短、锚点删除、首尾边界与 cache 往返。稳定滚动不增加 N 级扫描。

## 3. P1：离屏 Attached 列表把宿主拉回列表内部

### 3.1 复现

列表 10 项 × 20px，在宿主内容中的 origin=500；宿主总长 1500px，视口 100px。尾部追加一项后宿主总长改为 1520px。没有主动定位或尾部跟随。

| 宿主原 offset | 列表原 visible range | 追加后列表请求 | 后果 |
| --- | --- | --- | --- |
| 0 | `0..0` | 500 | 用户正在列表之前的宿主区域，列表请求把视口拉到自身起点 |
| 900 | `10..10` | 700 | 用户正在列表之后的宿主区域，列表请求把视口拉到自身旧尾端 |

两轴 × 三模式共 12 条观察全部发生上述请求。这是 headless 公共协议产出的错误提议；本轮没有使用真实 Showcase 观察宿主应用该提议的画面。

### 3.2 根因与参考

`commit` 在列表没有可见内容时仍通过 `index_at` 选出首／尾项，保存一个本不属于该列表的阅读锚点。随后数据更新恢复这个锚点。已有 `with_content_origin`、`with_scroll_extent` 和空范围计算仍不足以保护宿主：**范围、锚点和请求所有权必须一致。**

virtua 的 [store](../../refer/virtual/virtua-main/src/core/store.ts) 分别保存 `scrollOffset` 与 `startSpacerSize`，用相对 offset 求范围，视口在列表外时显式返回空范围；[relayout](../../refer/virtual/virtua-main/src/core/store.ts) 使用布局前后锚点位置差。可以借鉴坐标职责和增量补偿，但这不等于源码已提供任意 footer 的业务锚点政策。

### 3.3 最小修复方向与门槛

- 区分“视口完全在列表负责的几何区间之外”和“视口在列表 padding／大 gap 中”。不能只根据 `visible_range.is_empty()` 一律禁止锚点。
- 完全离屏时清理或停用该列表的阅读锚点；普通结构／测量变化不产生把宿主拉回列表的请求。显式定位、显式开启跟随另走已有命令合同。
- footer 应保持绝对位置还是跟随前方内容平移，由宿主阅读锚点政策决定；本轮不擅自规定 900 或 920 必须是哪一个。列表不能代替宿主选择自身尾项。

升级前复现 0→500、900→700；升级后无主动命令时不得产生上述抢占请求。补齐两轴 × 三模式、进入／离开列表、超大 gap、padding、两个附着区域共享一个宿主、明确定位／尾随及 stale request 的回归。

## 4. P1：Masonry 用旧锚点／旧定位覆盖用户新输入

### 4.1 普通滚轮也会丢位移

10000 项、3 列，每项估算与实际都为 40px，视口 300×100px。没有主动命令，连续四次向后滚动 20px：

| sizing | 每次事件分发后的 offset | 排空输入引发的任务／绘制后 offset | 四次净位移 |
| --- | --- | --- | --- |
| Fixed | 20、40、60、80 | 20、40、60、80 | 80 |
| Known | 20、40、60、80 | 20、40、60、80 | 80 |
| Measured | 20、40、40、60 | 20、20、40、60 | 60 |

12 条记录逐次检查 wheel listener 的调用数。Measured 第二次事件确实把原生 offset 从 20 写到 40，随后回到 20；不是测试没有命中组件。即使实际尺寸恰好等于估算，未测项首次变为 measured 仍可能触发该路径。

### 4.2 待校正定位未被输入打断

同样 10000 项／3 列，估算 40px、实际 1px、额外校正 budget=1，先定位 key=500：

```text
首帧后                  6523
wheel -20 分发后        6543
排空输入引发的任务后     6523
推进下一帧后             6523
```

对照为实际 40px，首帧 6640，wheel 后保持 6660，未请求续帧。另有直接写原生 owner 的 +20px 对照：实际 1px 时 6543→6523，实际 40px 时 6680→6660；后者说明旧阅读锚点问题也可独立于待校正命令发生。4 条补充记录不被解释为所有外部写入都必然回退。

`after_dispatch` 是 native listener 完成后、事件调用栈内读取的值；`after_input` 在排空后台任务后读取，可能已绘制；`after_frame` 再显式推进下一帧。这样区分输入应用和后续覆盖，不把所有步骤统称“立即”。

### 4.3 根因与参考

- [masonry.rs](../../vendor/gpui-base/src/virtualization/masonry.rs) 的 `update_sizes` 在尺寸或 measured 身份改变后调用 `restore_anchor`，采用上一次 `commit_anchor` 保存的位置，没有先纳入最新实际输入。
- [masonry_element.rs](../../vendor/gpui-base/src/virtualization/masonry_element.rs) 保存 pending，校正 pass 重放，在真实进展但仍有 deferred 时保留至下一帧；构造器没有一维 VirtualList 那样的输入取消处理。

参考中可吸收的事实：

1. TanStack [resizeItem](../../refer/virtual/virtual-main/packages/virtual-core/src/index.ts) 区分首次测量／重测、是否影响视口上缘以及命令回读阶段；同一测量批次若前一个补偿已移动位置，先用当前几何判断下一项。学习的是“补偿所依据的位置必须是本次有效位置”，不机械搬运其浏览器特殊分支。
2. virtua [driver](../../refer/virtual/virtua-main/src/core/driver.ts) 在非边界用相对 `scrollBy` 应用 jump，以免覆盖并发滚动；`_scroll` 将 `stopped` 保留为状态，避免与测量交错时丢失停止。**这份源码的 wheel handler 本身主要推断滚动状态，不能据此宣称全部 wheel 都取消定位。**
3. vlist 的 [synthetic handler](../../refer/virtual/vlist-main/src/synthetic/handler.ts) 在接纳 wheel 后先 `cancelScroll()`；普通 [native scroll handler](../../refer/virtual/vlist-main/src/core/scroll.ts) 的 wheel/commit 路径没有相同取消调用。应借鉴明确的输入接管点，不把某个 driver 的保证泛化至整个库。

### 4.4 最小修复方向与门槛

先采样最新原生位置，按旧几何捕获当前阅读锚点，再应用测量并补偿真实几何差。用户输入推进活动身份并取消旧 pending；自身几何补偿不伪装成用户输入。只保存一个有效定位与必要的终止状态，不复制整套一维命令 runtime 或引入第二个滚动 owner。

改前保留“4×20 仅到 60”和“6543 被写回 6523”的失败基线；改后普通等高 Measured 应到 80。补充向前／向后、未知高度、估算大幅偏差、跨帧预算、滚轮与 scrollbar／键盘导航接管、测量先后顺序、新命令替换、无进展及卸载。动态几何确实变化时按阅读对象验证，不能统一要求绝对 offset 永远不变。原有完整工作集呈现和预算上界仍须通过。

## 5. P2：已知 f32 边界在真实输入链上的量化

生产 Fixed/Known VirtualList，100 万项 × 20px，两轴各执行 5 种输入，共 20 条记录：

| 初始 offset | 单次向后 delta | 次数 | 实际净位移 |
| --- | --- | --- | --- |
| 1000 | 0.25 | 1 | 0.25 |
| 16777216（2²⁴） | 0.25 | 1 | 0 |
| 16777216 | 1 | 1 | 0 |
| 16777216 | 2 | 1 | 2 |
| 16777216 | 0.25 | 8 | 0 |

不是新增的“超大数量支持”发现，而是[第三轮已记录的输入链待测边界](2026-11-04-virtual-list-followup-review.md)获得了实测。一次 2px 可移动、连续八次 0.25px 仍为 0，说明缺失的是可持续累积的小增量，而非范围查询不存在目标。

[Div 原生 wheel](../../vendor/gpui-pre/src/elements/div.rs) 直接对 `Pixels`/f32 offset 加 delta；本项目 Fixed/Known 的 f64 prefix 和局部绘制没有改变这个输入 owner。vlist 的 [logical handler](../../refer/virtual/vlist-main/src/core/logical.ts) 分离逻辑位置、baseOffset 和局部放置；[runway](../../refer/virtual/vlist-main/src/core/runway.ts) 用同量反向拆分保持逻辑位置不变。runway 属于轮播路径，synthetic 另有输入 driver，不能称普通 native list 默认拥有此能力。

后续应在原生 owner 评估更高精度位置／增量累计与有界局部坐标的整体方案：仅保留 delta 残差可避免增量永久丢失，但不能单独保证亚像素绘制；只把 prefix 改成 f64 也不够。验收要覆盖两轴、正负连续输入、方向反转、边界、跳转后残差重置、拖动映射、Measured 累计和焦点命中。不能照搬浏览器元素高度上限作为 GPUI 阈值。

## 6. 合同澄清：局部失效不等于持久缓存身份改变

Measured 第 0 项已测 60px，导出 cache；`invalidate_items(0..1)` 后为未测，再恢复旧 cache：

| 同 key 的 source revision | 恢复条数 | 第 0 项结果 |
| --- | --- | --- |
| 保持 0 | 1 | 再次成为已测 60px |
| 更新为 1 | 0 | 保持未测，采用 20px 估算 |

[VirtualizerSource::revision_at](../../vendor/gpui-base/src/virtualization/mod.rs) 明确要求同 key 的内容准备或尺寸相关内容改变时递增。portable cache 目前保存的是这个 source revision，不保存控制器局部 content epoch，也不知道 [item view factory](../../vendor/gpui-base/src/virtualization/element.rs) 的外部依赖。因此：

- 在同一控制器中，显式失效可以使旧实时反馈／准备缓存过期；不能据此假定所有历史导出值的持久身份自动改变。
- 需要跨控制器恢复时，尺寸相关内容变化必须进入稳定的 `source.revision_at`；字体、密度等共享排版依赖进入 layout revision。只影响准备结果、不影响尺寸的外部依赖不应无故清空全部测量。
- 若产品合同要求“任意一次局部失效后，旧 portable cache 永不可恢复”，需要额外定义可跨控制器的身份或恢复政策。控制器私有 epoch 原样序列化不能自然解决跨会话含义。

本轮归为 API 文档／测试需明确的条件性风险，不断言现有代码违反 `revision_at` 合同。virtua 的 [List snapshot](../../refer/virtual/virtua-main/src/core/layouts/list.ts) 主要保存 sizes/defaultSize，vlist 的恢复侧重位置和选择，均不能充当本项目完整内容／约束身份的替代方案。

## 7. 实施优先级与性能边界

| 顺序 | 工作 | 最小交付 |
| --- | --- | --- |
| 1 | Attached 离屏阅读所有权＋padding/gap 锚点 | 先补上述失败测试，再统一锚点恢复与请求资格；同步 cache 锚点语义 |
| 2 | Masonry 最新阅读位置和输入接管 | 普通输入与程序定位都不被旧意图覆盖；维持第三轮完整呈现和有界校正 |
| 3 | 持久 cache 身份合同 | 明确 live invalidation 与 portable identity，补合法恢复／过期跳过的测试 |
| 4 | 原生大坐标输入 | 原生 owner 内的精度方案与局部坐标、scrollbar、Measured 路径一并验证 |

这些是现有合同的正确性门槛，优先于 Grid auto／冻结区域、更多 layout 或插件。锚点状态应保持常量规模；输入处理不扫描 N 项；Masonry 工作依然按工作集及预算计量；native residual 如引入只按轴保存有界状态。新方案需要改前失败、改后回归以及涉及热路径时的 Release 时间／分配 A/B，不能仅因多了一个判断就声称性能提升。

本轮没有 P95/FPS/真实进程 CPU 的新采样，也不能据这些值推断用户曾观察到的 0.7%→4.5% 峰值已改善。不启动或操控真实 Showcase，实际 UI 由用户检查。

## 8. 检查与复现

| 检查 | 本轮结果 |
| --- | --- |
| 独立 Release/headless 观察 | 6 passed；68 条观察，包含上述已确认问题 |
| 当前生产 virtual_list 专项 | 78 passed / 8 ignored / 0 failed |
| 独立 fixture Clippy，`-D warnings` | 通过 |
| 探针 rustfmt、文档链接、原始数据与日志核对 | 通过 |
| `git diff --check` | 通过 |

8 项 ignored 为原有手动性能探针，本轮没有重新执行。未修改生产实现，不重复上一轮完整工作区与 Showcase 构建来冒充新问题已通过；后续修复仍需执行相应完整回归。

从仓库根目录复现；独立 fixture 的生成物和锁文件只放 target：

```powershell
$fixture = 'target/virtual-owner-review-20261104'
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
@'
[package]
name = "yue-virtual-owner-review-20261104"
version = "0.1.0"
edition = "2024"
[workspace]
[[test]]
name = "review"
path = "../../docs/research/virtual-list-owner-review-2026-11-04.rs"
[dependencies]
yue-component = { path = "../../crates/yue-component", default-features = false, features = ["headless-tests"] }
gpui-kit = { version = "0.8.0", default-features = false }
[patch.crates-io]
gpui-base = { path = "../../vendor/gpui-base" }
gpui-pre = { path = "../../vendor/gpui-pre" }
gpui-pre-windows = { path = "../../vendor/gpui-pre-windows" }
'@ | Set-Content -LiteralPath "$fixture/Cargo.toml" -Encoding UTF8
if (-not (Test-Path -LiteralPath "$fixture/Cargo.lock")) {
    Copy-Item -LiteralPath Cargo.lock -Destination "$fixture/Cargo.lock"
}

# 首次执行允许 fixture 添加自己的 package 记录，之后使用 --locked。
cargo test --manifest-path "$fixture/Cargo.toml" --offline --release --target-dir target --test review -- --nocapture --test-threads=1
cargo test --manifest-path "$fixture/Cargo.toml" --offline --release --locked --target-dir target --test review -- --nocapture --test-threads=1
cargo clippy --manifest-path "$fixture/Cargo.toml" --offline --release --locked --target-dir target --all-targets -- -D warnings
cargo test -p yue-component --locked --features headless-tests --test virtual_list -- --test-threads=1
rustfmt --edition 2024 --check docs/research/virtual-list-owner-review-2026-11-04.rs
git diff --check
```

本轮完整执行日志位于 `target/virtual-owner-review-20261104/`。本目录保存观察源码和原始输出以便在生产修复后比较，不通过重写旧证据掩盖行为变化。
