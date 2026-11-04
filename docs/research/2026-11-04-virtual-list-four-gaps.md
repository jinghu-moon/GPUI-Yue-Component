# 虚拟滚动四项逐项优化

起点 `c04735a`，分支 `quality/virtualization-four-gaps-20261104`。按实窗验证、API 合同、性能与内存、补丁回归依次测试。只采用有证据的改动；每项独立 Git 提交，不改三个参考项目、不推送。既有性能结论与未支持能力仍有效。

## 1. 实际 Windows 窗口验证

此前已有 headless 行为覆盖，但没有固定的虚拟布局原生窗口探针。本轮新增 `tools/test-virtualization-native.ps1`：直接用 `gpui_kit::application` 和生产 `VirtualLayoutsPage`，在真实 Windows 平台、事件循环、字体与 renderer 中验证 Grid 百万行／千列右下角定位、动态 Masonry 三列改五列后重新绘制，以及横向 Measured 第 2000 节点定位与命令收敛。每阶段最多 100 次 20ms 轮询，失败清理窗口并退出；每个原生探针分进程运行。renderer 数量只有宽松的有界性断言，不当成 FPS。

复现：

```powershell
./tools/test-virtualization-native.ps1 -Offline -TargetDir C:/Temp/gpui-yue-owner-perf-20261104
```

首轮通过，实际累计 renderer 调用 Grid 352、Masonry 49、横向 Measured 28；数量受实际帧调度影响，不要求逐次一致。此探针用控制器驱动生产画面，不冒充真实鼠标／键盘输入测试。

另用 computer-use 的 Sky 对起点 Release Showcase 实窗进行输入与截图验收（本机当前显示环境）：

| 操作 | 实际观察 |
| --- | --- |
| 百万记录定位末尾 | 最后一条 domain key 为 1000999，末尾完整显示 |
| 尾部头插一条 | 可见旧 key 1000990～1000999 保持，未跳回起点 |
| 动态卡片展开 | 首卡自然增高，后续卡片随布局移动 |
| Grid 定位右下角 | 最后单元格 1000000 × 1000 可见 |
| Grid 在尾部两轴反向滚轮 | 行列均向前移动，无整片空白 |
| Masonry 切换三列／五列 | 重新换行，卡片按五列显示 |
| 横向 Measured 定位最后节点 | 节点 2000 完整进入视口，前面的异宽节点正常呈现 |

覆盖边界：当前一台 Windows 机器与 GPU；没有物理多屏 DPI 切换、不同显卡／刷新率、触控板惯性或图片网络时序证据。不据此宣称 GPU 帧率提升。自动探针可在这些环境重跑，手动输入与 headless 测试各自提供独立证据。

## 2. API 一致性与边界

起点存在两个公开接缝缺口：Masonry 只接受 align，缺少能力查询；Grid 的配置错误要到布局时 assert。新增合同测试在实现前因缺失 API 编译失败；实现后 3 项全部通过，并继续保留原有 align 调用测试。

- Masonry 的 `scroll_to_key` 接受 `impl Into<VirtualizerScrollOptions>`，原 align 经 `From` 转为默认 instant 选项。新旧入口在真实布局中的 Center 落点一致。`capabilities()` 返回独立 `VirtualMasonryCapabilities`，沿用 axis／supports_attached／supports_follow_tail／supports_smooth／supports_scroll_to_offset 查询名；明确后四项尚不支持。一维能力查询补充 offset reader。
- 新请求的 key、motion 边界和 unsupported 在观察输入、取消旧命令和分配 command ID 之前校验。非法／不支持请求保留旧命令与事件队列，测试检查 ID 未消耗。Masonry 显式 smooth 返回 Unsupported；合法的 reduced-motion 请求即时定位，仍验证 duration／distance。原有一维动态／长距离安全即时定位规则不改变。
- `VirtualGrid::validate_axes` 可在构造前返回轴错误、Owned／Measured 不支持、宿主绑定冲突；布局入口复用该校验。错误测试验证 source 未增加查询，原有效绑定仍可用。旧 `new` 保留其编程错误断言合同，未改成静默降级；预检不声称检查随后的挂载竞争。

没有加入 RTL／反向布局、Masonry Attached／offset／follow-tail／smooth，或从可见单元格推测全局 Grid 动态行高。它们仍是明确能力边界，不能用接口形状一致冒充算法支持。

## 3. 性能与内存

行为／workload 起点 `73961a8`；最终采用源码 `7b9148d`。复用隔离 Git 源码归档和 Release ABBA，before/after/after/before；每个微批次 case 每版本 10 个样本，表中是每样本 2000 次调用的中位数。`ns` 来自 `Instant::elapsed` 的主线程工作段墙钟耗时，不是进程 CPU 使用率；heap 只计测量窗口内的线程局部分配 payload，不含预存模型，也不是 RSS／GPU 数据。source／构建次数提供独立保护，不通过少呈现内容换速度。

### 拒绝的方案

1. 手写 32 项栈缓冲 `df672d0`：小批次可消除临时 Vec，但大批次 heap scratch 活到几何写入之后，33／256 项实际变高反馈峰值分别增加 528／4096 B。`699736e` 修正释放时点并对空批次提前返回，峰值恢复，但 33 项无变化耗时 +11.53%、256 项实际变化 +4.98%。不采用。
2. 复用既有依赖 SmallVec `221a453`：峰值恢复且无需新增依赖，但 30／32／33 项无变化耗时分别 +3.60%／+6.61%／+13.56%，没有稳定净收益。不采用。
3. 在主函数内加入单项专用路径 `7b7e437`：单项获益，但 30／32 项无变化批次分别 +11.34%／+7.91%。移到独立私有非内联 helper 后，这组多项退化在两轮新测量中消失；不把观测强称为某个已证明的编译器阈值原因。

试验与撤销都保留在 Git：SmallVec → `b50d5f2`、释放时点版本 → `080b27a`、最初栈缓冲 → `40478b5`；主函数单项版本 → `172f171`，独立 helper 试验 `ce686a3` 先撤为 `8793215`，再从干净算法起点收入最终 `7b9148d`。因此最终采用项可单独 revert，不会退回某个未采用试验。

### 采用：仅优化单项测量反馈

单项无需排序、去重和按 lane 分组，直接在原尺寸 owner 中校验、判断 no-op、捕获／恢复锚点和推进一次 revision；全部多项批次继续原算法。无持久 scratch／尺寸镜像／新依赖。两个独立 ABBA：

| 每样本 2000 次调用 | 首轮 ms 前→后 | 独立复测 ms 前→后 | 分配次数前→后 | 临时峰值 B 前→后 |
| --- | --- | --- | --- | --- |
| 1 项，无变化 | 0.10185→0.03460 | 0.10345→0.03480 | 2000→0 | 16→0 |
| 1 项，40／41px 交替变化 | 3.79305→3.56420 | 3.88295→3.58245 | 31985→25987 | 8992→8856 |

单项无变化耗时 −66.03%／−66.36%；实际变化 −6.03%／−7.74%、分配约 −18.75%。单项实变的 source 访问不变。31／32／33／256 项、倒序、重复拒绝、非法 key／NaN 整批拒绝、空输入、no-op revision、旧冻结几何和真实测量标记都有回归；该行为测试也在优化前通过。

多项批次分配／峰值不变，不能把单项结果推广为所有反馈优化。首轮 32 项实变耗时 +4.10%，独立复测 −2.34%；其余多项 case 在复测 −3.76%～−0.41%。原有 10000 次 30 项无变化保护耗时 5.34170→5.37155ms（+0.56%），分配仍是 10000 次，明确没有消除此热点的整批临时 Vec。

17 类帧轨迹的 source／构建／分配／峰值计数全部相同。首轮 Masonry 稳定／快速／复杂／改宽／变高轨迹耗时变化 +0.64%／−0.28%／+1.66%／+1.20%／−4.50%；其他模式也有正负波动，不能宣称全面帧率或整机 CPU 收益。Fixed O(1) metadata、Known／Masonry O(N) 初始化、Masonry 追加复制 O(N/256) 目录和既有原生小稀疏批次峰值代价都没有被这次局部优化消除。

9 组原始数据与源码／workload 哈希统一见 [来源记录](virtual-list-four-gaps-2026-11-04.json)，CSV 哈希按 UTF-8／LF 规范化，避免 Windows checkout 的 CRLF 改变校验。采样工具另外修正了长期 `cargo dev` watcher 导致永久等待的问题：只忽略其监听器本身，普通 cargo／rustc／clippy 及实际子构建仍阻止采样，要求连续两次安静观察。未终止其他项目进程。所有实验采用相同工作量，不宣称完全排除了操作系统调度影响。最终另外尝试一次完整保护复测，但遇到同机其他 Release 构建而取消，没有输出 CSV，也未计入已完成证据；以上采用结论来自两轮独立微批次 ABBA 与一轮完整保护矩阵。

复现采用项：

```powershell
python tools/compare-virtualization.py --before 73961a8 --label four-masonry-recheck --filter four_masonry_feedback --workload docs/research/virtual-list-adoption-2026-11-04.rs
python tools/compare-virtualization.py --before 73961a8 --label four-masonry-guards-recheck --filter series_
```

## 4. 本地补丁升级回归

原 workspace 把三个 vendor crate 排除在外，常规 workspace green 并不能代表本地 GPUI 补丁的直接测试已执行。本轮把手工／临时 fixture 固化为 `tools/test-virtualization-contracts.py` 和受版本控制的 17 项合同清单，并接入 Windows CI：

- 从生产 lock 起点生成一次性 fixture，只去掉 Base 冲突的备用后端 dev-dependencies，指向实际维护源码，不改生产 Cargo.toml／Cargo.lock。构建后找到唯一测试 executable，每项按 exact 名运行，要求恰好 1 passed／0 failed／0 ignored；测试缺失、全过滤或被 ignore 都失败。
- 直接运行 15 项原生 ListState／ScrollHandle 合同与 2 项 Base 几何合同，覆盖焦点生命周期、批量反馈原子性、hint／真实标记、持久快照、宽度失效、横向布局、巨大坐标输入与尾项可达。Yue／Showcase 集成仍由 workspace 测试提供，真实窗口探针单独选择运行，不把 headless CI 冒充 GPU 验收。
- `--core-path`／`--base-path`／`--windows-path` 可指向升级候选源码；原生 integration tests 始终取自当前维护仓库，避免候选恰好缺少补丁测试却显示 green。失败保留日志与 failed 报告，成功记录版本、源码／清单／测试哈希和实际通过项；检查前后生产 manifest／lock 未被修改。

正向：**15 native + 2 Base passed**。负向不是只检查脚本字符串，而是运行实际构建／测试：

1. 在临时清单增加不存在的测试，门禁拒绝 `required contracts missing`，不会把 0 tests 当成功。
2. 复制 core 到 target 下隔离目录，把 precise_offset 错误地从 f32 投影读回。候选仍可编译，维护中的精度测试实际失败：期望 `(-16777216.25,-16777216.75)`，错误返回 `(-16777216,-16777216)`。生产源码未修改。还原到生产来源后 17 项再次全通过。

这两个负控已固化到 `tools/test-virtualization-contracts-negative.py`，只修改经路径校验的临时 source copy，完成后清理该副本。复现：

```powershell
python tools/test-virtualization-contracts-negative.py --offline --target-dir C:/Temp/gpui-yue-owner-perf-20261104
python tools/test-virtualization-contracts.py --offline --target-dir C:/Temp/gpui-yue-owner-perf-20261104
# 升级候选审查，例如：
python tools/test-virtualization-contracts.py --core-path D:/Candidate/gpui-pre --layer native
```

没有在本轮替换 GPUI 版本或宣称维护成本归零。升级仍须审查 YUE-PATCH.md、API／ABI 兼容、完整 workspace 行为与性能基线，以及目标机器实窗体验；自动门禁负责把已知合同丢失变成可复现失败。

实际正向与两次预期失败的版本／源码身份见[合同验收记录](virtual-list-four-gaps-contracts-2026-11-04.json)。负控中的 failed 是门禁成功抓住故障的预期结果；最后一次生产来源报告为 passed。

## 最终验证与回退

针对最终采用源码完成：

- `cargo test --workspace --features windows-showcase/headless-tests --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104 -- --test-threads=1`：全部 workspace 测试及 doctests，合计 **825 passed / 10 ignored / 0 failed**。其中 virtual_list 为 **111 passed / 8 ignored**；原生窗口探针在普通批次中按预期 ignored，另显式运行。
- `python tools/test-virtualization-contracts.py --offline --target-dir C:/Temp/gpui-yue-owner-perf-20261104`：**15 项原生底层 + 2 项 Base 合同通过**，源 manifest／lock 未变化。`test-virtualization-contracts-negative.py` 的两个负控通过，随后生产来源重跑通过。
- Workspace all-targets／headless-tests Clippy `-D warnings` 通过；底层原生 integration fixture 的 Clippy `-D warnings` 也通过。fixture 的未使用 patch 提示来自该独立依赖图不包含 Base／Windows 平台，不代表代码警告或缺失测试。
- `cargo fmt --all -- --check`、vendor 虚拟化与 workload 的 `rustfmt --check`、三个 Python 工具语法编译、`git diff --check` 均通过。
- `cargo build --release -p windows-showcase --offline --locked --target-dir C:/Temp/gpui-yue-owner-perf-20261104` 通过。`./tools/test-virtualization-native.ps1 -Offline -Release -TargetDir C:/Temp/gpui-yue-owner-perf-20261104` **1 passed**，真实 renderer 累计 Grid 264／Masonry 49／横向动态 28 次；不把该调用次数解释成帧率。
- 9 组 CSV 的 before／after、行数、workload SHA256 和 after 源码提交核对通过；`ce686a3` 与最终 `7b9148d` 的 Masonry 源码逐字一致。单项性能与 CI 门禁提交的 `git apply --reverse --check` 通过，只检查反向可应用性，未改工作区。

| 内容 | 提交／检查点 |
| --- | --- |
| 本轮起点 | `c04735a` |
| 实窗探针与初始验收 | `2019c63` |
| 公共 API 合同 | `09a2cab` |
| 优化前就通过的批次行为／workload | `73961a8` |
| 最终采用的单项反馈优化 | `7b9148d` |
| 补丁合同门禁、负控与 CI | `ef83b33` |

回退单项性能优化可执行 `git revert 7b9148d`，会回到原多项／单项统一算法，不会进入被拒绝的缓冲试验。CI 门禁可单独 `git revert ef83b33`。API 的后续测试依赖 `09a2cab`，需要撤销这组功能时应连同相应后续测试按依赖逆序处理。若要完整对照本轮起点，工作区干净时可用 `git switch -c review/virtualization-before c04735a` 新建审查分支，不覆盖当前成果；不要用 hard reset 删除试验与证据历史。

所有改动保留在 `quality/virtualization-four-gaps-20261104`，未推送，没有修改参考项目或生产依赖版本。
