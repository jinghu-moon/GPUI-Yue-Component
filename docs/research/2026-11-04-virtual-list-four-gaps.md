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

覆盖边界：当前一台 Windows 机器与 GPU；没有物理多屏 DPI 切换、不同显卡／刷新率、触控板惯性或图片网络时序证据。不据此宣称 GPU 帧率提升。自动探针可在这些环境重跑，手动输入与 headless 测试各自提供独立证据。

## 2. API 一致性与边界

待完成。

## 3. 性能与内存

待完成。

## 4. 本地补丁升级回归

待完成。
