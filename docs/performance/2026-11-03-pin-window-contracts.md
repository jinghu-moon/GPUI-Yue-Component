# F6 迁移前置合同：两阶段交接与 GPUI 无激活窗口

日期：2026-11-03。本文记录实际执行的代码／窗口检查，不代表 F6 已完成迁移或 HDR 已完成产品验收。应用侧以 SnapClip `docs/53-hdr-capture.md` 为准。

## 1. 根因与修改

1. SnapClip 原先先 `take_selection` 清空截图槽位、结束捕获，再提交窗口请求。同步拒绝或异步 `Failed` 后无法重试。新流程为 `PinSource::claim` → 创建 → `Created` → `SelectionClaim::commit`；失败／取消只释放 claim，截图仍在原槽位。源身份比较使用同一 Arc，不复制像素；旧请求不能消费新选区。捕获线程只结束与确认匹配的旧会话；新会话或新手势仍保留。
2. GPUI 原有 `focus: false` 只控制首次显示。Windows `WM_MOUSEACTIVATE` 固定返回 `MA_ACTIVATE`；后续 `activate_window` 还会进入设置焦点／模拟 Alt 的普通窗口路径。它不能表达 F6 的持续无激活合同。
3. core `WindowOptions`／`WindowParams` 新增 `is_activatable`，默认 true；仅 Windows 实现。false 在 `CreateWindowExW` 前设置 `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` 并移除 `WS_EX_APPWINDOW`，覆盖初始 `focus`；首次／延迟显示使用无激活 show command，鼠标消息返回 `MA_NOACTIVATE`。显式 activation 请求仍可显示窗口，但不进入焦点／模拟输入路径。普通窗口仍按原分支处理。

该选项不授予无焦点窗口键盘输入，不替代置顶、图片几何、拖动或缩放策略。F6 仍使用已有 native 宿主并拒绝 HDR；尚未因本次前置能力验收而删除旧实现。

## 2. 修改前后证据

| 检查 | 修改前 | 修改后 |
| --- | --- | --- |
| 截图创建失败后的源 | `hdr-p3e-source-before.log` 实际断言失败，源已被消费 | App claim／重试／新旧选区和 Capture ack／post 失败／会话隔离测试通过 |
| 无激活公共合同 | 用旧 core 构建时缺少 `is_activatable`，`hdr-p3e-activation-before.log` 编译失败；这是接口缺口证据，不是原生焦点实测 | `activation_tests` 普通测试 1 通过，native 1 ignored |
| 真实 GPUI 窗口 | 原参数不能表达持续无激活 | 精确选择 ignored native probe，**1 通过**：隐藏创建、首次显示、显式延后显示、实际 HWND 样式和生产鼠标消息处理均检查；前台 HWND 保持不变 |
| 默认窗口负对照 | 原默认窗口可激活 | 同一个 native probe 中创建隐藏普通窗口，生产 handler 仍返回 `MA_ACTIVATE`；没有实际点击或主动激活普通窗口 |
| Windows 后端既有回归 | 现有测试套件 | **51 通过／3 ignored**，ignored 未计入通过；与独立 native probe 分开记录 |

真实 probe 创建短暂的 8×8 GPUI popup，约 1 秒内关闭；没有修改系统 HDR、移动鼠标、实际截屏、写剪贴板或调用普通窗口的模拟输入路径。它证明当前桌面环境下的窗口激活合同，**不证明 F6 迁移、HDR 高光、跨屏 DPI、窗口圆角和缩放交互**。

## 3. 复现和隔离范围

```powershell
pwsh -NoProfile -File tools/test-gpui-window.ps1
# 明确 opt-in，短暂显示真实 GPUI popup：
pwsh -NoProfile -File tools/test-gpui-window.ps1 -Native
```

`-Release` 选择优化构建，`-CorePath` 可显式选择待验证的维护 core。上述普通／native 检查本轮使用 SnapClip `target/hdr-vendor-baseline/vendor/gpui-pre`：以 Yue 已提交检查点为底，叠加本次 `platform.rs`／`window.rs` 激活选项。Windows 后端使用本仓库真实修改后的代码。此举隔离另一任务中持续变化的虚拟列表代码；不能宣称那个工作树通过了本轮验证。

主要日志位于 Yue `target/hdr-p3e-{activation-before,activation-after,activation-native,windows-regression}.log`；应用交接证据在 SnapClip `target/hdr-p3e-{source-before,source-after,handoff-headless}.log`。target 快照和日志不提交。Release、真实 F6、HDR 开关／双屏／DPI 的组合矩阵仍待单独执行。
