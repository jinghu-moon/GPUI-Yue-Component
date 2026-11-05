# 虚拟 Grid：键盘、Windows 桥与当前设备验收

日期：2026-11-05。承接[前轮 9 分门槛](2026-11-05-virtual-list-nine-point-readiness.md)。用户确认当前只有一台设备；**真实跨 DPI／多显示器验收调整为设备受限待验**，不作为已通过项目，也不阻塞现有设备可做的修复。

## 1. 本轮采用的改动

行为位于 `windows-showcase` 的算术记录消费者，复用既有 `VirtualGrid`、`retained_cells` 和 axis 定位接口。没有把示例的业务导航规则塞入通用 Base，也没有更改生产依赖、锁文件或 `refer/virtual`。

| 项目 | 修改前 | 修改后与验收 |
| --- | --- | --- |
| 输入与焦点 | Grid 仅展示数据，点击正文后没有可导航的逻辑当前单元格 | 单一 Grid Tab stop；点击任意已呈现单元格进入，方向键移动；仅根容器实际持有键盘焦点时处理 |
| 行列定位 | 只有“定位右下角”按钮 | Home/End 到当前逻辑行首尾，Ctrl+Home/End 到全表首尾；Ctrl+Home 明确回到 offset 0，断言实际 offset 而不只看选中名称 |
| 翻页 | 无 | PageUp/PageDown 以实际正文视口高度计算行数，扣除当前冻结首行，至少一步并钳制边界 |
| RTL 与合并 | 只有布局呈现 | 物理左右键随 RTL 镜像，逻辑索引不变；进入覆盖坐标归一到合并 origin，离开时跳过整个跨度；切换合并时修正当前坐标 |
| 回收与重入 | 无单元格级键盘合同 | 仅额外保留一个稳定 key pair；滚轮不修改当前坐标，Tab 可离开，重入时找回当前单元格；工具栏和其他控件不被抢焦点 |
| 原生恢复 | 已有 offset 与布局断言，未覆盖当前单元格焦点 | 正文 `(1,1)` 持有焦点时完成 resize／rem／RTL 轨迹及真实最小化／恢复；阅读偏移与逻辑焦点同时保持 |

当前坐标在此不可重排的 `Records` 中同时是稳定 key。这个示例没有承诺为任意可重排数据使用 index 作身份。单元格的当前标记使用主题色与内置绝对边框，不改变原行高／列宽；真正的键盘焦点保留在 Grid 根，AccessKit active-descendant 指向当前单元格。

只有支持的导航键会停止冒泡；Shift、Alt、平台修饰键以及非 Home/End 的 Ctrl 组合交给原有处理路径。没有截获 Tab，也没有给所有单元格创建 FocusHandle。窗口停用保留原逻辑焦点句柄；恢复时不将阅读位置强行拉回离屏坐标，只有实际离开后重新进入 Grid 才找回当前单元格。

保留无窗口的非交互 draw 工厂，让既有 `native_profile.rs` 与历史基线保持逐字兼容；生产窗口和交互测试使用带实际 Window 的焦点工厂。CPU workload 不用于证明键盘行为。

## 2. 前后测试与真实输入

同一生产视图新增首批 4 项输入测试，修改前全部失败：点击不进入 Grid 焦点，当前单元格语义缺失。最终扩展为 **5 项通过**，覆盖边界、修饰键负例、合并／RTL、真实正文翻页、工具栏滚动后重入、resize／rem、切换合并和卸载／重挂。

测试挂载真实 Base Root 并激活 headless 窗口，经过正常 pointer／keyboard 路径；没有直接调用导航函数代替输入。开发时的私有 helper／fixture 编译适配失败，以及未激活测试窗口导致 focus listener 不执行的失败，均不是产品功能失败基线；只把成功编译后、改动前实际 4 项失败用于前后比较。

最终完整 workspace：**907 passed／17 ignored**。最终 Showcase 独立复验 **36 passed／7 ignored**；5 个键盘测试包含在该数量内。workspace all-targets Clippy（`headless-tests,virtual-profile`，`-D warnings`）通过。格式、主题同步与证据解析器 7 组测试通过。ignored 探针随后各在独立原生进程执行，准确执行数守卫全部通过：**5 项原生 Release 探针**。

真实交互通过 CUA 绑定本轮 Release 窗口，fresh AX 与截图核对：

1. 点击 `2 × 2` 后，AX 焦点为该单元格；Right、Down 后变为 `3 × 3`。
2. Ctrl+End 到 `1000000 × 1000`，焦点边框可见；Ctrl+Home 返回 `1 × 1`，正文同时回到起点。
3. 启用合并后，`2 × 2 · 合并 2×3` 唯一；按 Right 到 `2 × 5`。
4. Shift+Tab 离开 Grid 到末个工具栏按钮；通过键盘切换 RTL 后 Tab 重入，仍为 `2 × 5`；Right 回到合并 origin。截图确认冻结列在右侧、边框包住完整合并区域。
5. 真实滚轮请求 3,000px 后，可见行 31–40，当前逻辑坐标与 AX 焦点仍是合并 origin；退出再进入恢复到该区域。原生 owner trace 的实际纵向偏移为 1,050，不将滚轮请求量当作实际位移。
6. 关闭临时窗口后 fresh inventory 确认消失，交互 helper 正常退出。截图当场检查，未归档截图文件；未启动或试听 Narrator。

原生生命周期探针保留 24 步 resize／rem／RTL 与偏移断言，现同时断言正文当前坐标 `(1,1)` 和焦点。当前环境 `displays=1`、`scale=1.5`；rem 16／20 是内容尺寸测试，不能替代 OS DPI 切换。

## 3. Windows 桥：实际 COM 能力边界

新增 `python tools/test-windows-grid-bridge.py --offline`。从生产 Cargo metadata 定位锁定 `accesskit_windows 0.34.0`，将源码复制到 target 下的独立 fixture，注入直接 COM provider 测试；不编辑 registry 或生产 manifests／locks。fixture 的 registry 依赖身份必须是生产 lock 中的原身份，禁止通过临时解析出新版本“修好”探针。

**3 项精确合同通过**，每项各执行 1 个测试，拒绝 0-test／ignored 成功：

- Grid／Cell 的 DataGrid／DataItem control type、名称和描述确实到达实际 COM 属性。`description` 映射到 **FullDescription**；HelpText 来自 placeholder，这里为空。
- 实际 `GetPatternProvider` 对 Grid、GridItem、Table、TableItem、ItemContainer、VirtualizedItem 均没有可用 provider；同一节点的 Invoke 是正控制，不能靠一个整体失效的 provider 通过。
- Invoke 与 SetFocus 分别转发真实 Click／Focus ActionRequest，目标 ID 与数量正确。

这些合同是锁定依赖的能力清单，不是完整表格读屏通过。测试故意在支持能力变化时要求重新评审，不能直接改预期来宣布验收完成。它们不向真实桌面查询 UIA，也不能代替 Narrator 的播报、浏览和 ReadAll。Windows CI 已接入此清单，本会话未宣称 GitHub runner 已完成。

尚缺的是可调用的 Grid/GridItem 及离屏 GetItem／查找／Realize 到消费者的完整协议。不能只补几个 getter 就暴露一百万行、一千列的完整 Grid 合同，也不能把总量改成可见数量来规避。当前生产 pin 不动；后续在专门分支评估协议和桥接变更，独立复验 sparse workset、跨度、失效、焦点及真实 Narrator。

## 4. CPU 尾部与证据限制

本轮接受的是交互正确性修复；没有同期源码前后 ABBA，**不宣称提速或零开销**。两个开发／最终原生回归轮次各保留 768 个 draw 样本，均非性能采用对照。首轮较慢样本没有删除；最终轮来自已提交的生产代码。分位数使用 nearest-rank。

| Grid | 首轮 P50／P95／P99 | 最终轮 P50／P95／P99 | 首轮／最终超过 6.25ms |
| --- | --- | --- | --- |
| rem16 | 3.989／8.249／10.146ms | 3.021／5.001／5.645ms | 24／0（每轮 96 次） |
| rem20 | 3.545／9.096／12.799ms | 3.549／5.490／6.791ms | 23／1（每轮 96 次） |

两轮存在时段与开发状态差异，不能将其差值归因为最终工厂适配的性能收益。首次测量的二进制 SHA256 与阶段说明保存在验证记录中；前轮正式 ABBA 与不利尾样本仍保留，不被这两个单独回归轮次替换。以上是 CPU draw／present 证据，不是 GPU 完成、FPS 或输入到显示延迟，仍不保证稳定 160 FPS。

## 5. 当前自评与后续任务

沿用六维等权的主观工程评估：几何与事务 9.2、布局组合 9.0、工作量与 CPU 9.0、回归证据 9.2、API／生命周期 9.0、平台可访问性 7.8，四舍五入 **8.9／10**。平台项的提升来自实际键盘与焦点、COM 正负能力边界；未把硬件延期折算成通过，**9 分目标仍未完成**。这不是三个参考项目的统一跑分。

| 后续项 | 当前状态与完成条件 |
| --- | --- |
| Windows 完整 Grid 读屏链路 | 当前主任务；先完成可复验的 provider 与离屏实现协议，再做 Narrator 的位置／总量／跨度／回收／浏览／ReadAll |
| 真实跨 DPI／多显示器 | **设备受限待验**；有相应硬件后，在同一轨迹中验证跨屏、尺寸反馈、阅读锚点、焦点、最小化／恢复及 tail；不重复把单屏 rem 测试当替代 |
| 持续证据 | 观察 Windows CI 与 performance workflow 的真实运行，保留不利样本；必要时按同机同期 ABBA 排查尾部成本 |

## 6. 复现、回退与归档

```powershell
cargo test --workspace --features headless-tests,virtual-profile --offline --locked -- --test-threads=1
cargo clippy --workspace --all-targets --features headless-tests,virtual-profile --offline --locked -- -D warnings
python tools/test-windows-grid-bridge.py --offline
python tools/test-virtualization-evidence.py
& tools/test-virtualization-native.ps1 -Release -Offline -IncludeFrameProfile
```

实际 Windows 探针需要已解锁的桌面。键盘与焦点改动为 `10a77a3`，桥能力检查／CI 为 `70fe510`；文档与证据另行提交。每项可用 `git revert` 回退；无 reset 或 force push。新工作分支为 `quality/virtual-grid-keyboard-20261105`。

历史 [nine artifacts](virtual-list-nine-artifacts-2026-11-05.json) 增补 `snapshot_ref=1c8eb93…`，原有散列不改，应对该历史提交的内容核验。当前文档的新增链接与本轮产物散列另行归档：

本轮核对两份文档的 75 个本地链接／锚点、11 个当前产物的 LF 散列，以及历史提交中 23 个产物的 LF 散列，均通过。原始字节散列与可跨 checkout 的 LF 散列分别记录。

- [实际失败基线](virtual-grid-keyboard-before-2026-11-05.txt)
- [最终检查记录](virtual-grid-keyboard-checks-2026-11-05.txt)
- [首轮原生完整记录](virtual-grid-keyboard-native-first-2026-11-05.txt)
- [最终原生完整记录](virtual-grid-keyboard-native-final-2026-11-05.txt)
- [两轮原始 draw 样本](virtual-grid-keyboard-frames-2026-11-05.csv)
- [真实交互轨迹与 AX 摘要](virtual-grid-keyboard-interactive-2026-11-05.txt)
- [COM 桥合同与源码身份](virtual-grid-keyboard-bridge-2026-11-05.json)
- [验证汇总及阶段身份](virtual-grid-keyboard-validation-2026-11-05.json)
- [本轮产物散列](virtual-grid-keyboard-artifacts-2026-11-05.json)
