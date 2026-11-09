# YueResizable 实施与验收记录

日期：2026-11-09。设计合同：[YueResizable 可调整分栏](../40-resizable.md)。

## 交付与边界

组件属于声明式、原位、持久、复合布局。横向为左右分栏，纵向为上下分栏，可组合嵌套。每组由宿主保留一个 `Entity<YueResizableState>`；Base 为唯一尺寸 owner，Yue 负责主题与相对长度，内容保留原有 Input／Tree／ScrollHandle，不创建第二套拖动或滚动状态。

参考实现为本仓固定版本的 Base ResizablePanelGroup／ResizableState／ResizeHandle。吸收其状态、布局测量、拖动负载和外观注入点；补充显式启用的稳定面板身份、相邻双向约束、键盘操作及复位。没有借用 Vue DOM／Reka 运行时，也不为此引入依赖。Dock 重排、浮动窗口、面板折叠及 Windows COM Provider 均非本次分栏合同。

## API 与实现

- `YueResizable::horizontal/vertical(id, &state)`，`child/children`、`disabled`、`theme`、`on_resize`、`Styled`。
- `YueResizablePanel::new(id)`，`label`、`initial_size/min_size/max_size`、`fixed`、`ParentElement` 内容组合。尺寸接受 AbsoluteLength，建议 rem；fixed 要求 initial_size，保留用户当前尺寸。
- `YueResizableState`／`ResizableEvent` 有意重导出 Base 类型。`sizes()` 用于读取快照，`set_sizes` 校验后静默恢复；`resize_panel`／`reset_layout` 对实际变化发出状态事件。用户回调不回送程序同步或测量。
- 指针只调整相邻两块，维持其总尺寸；边界范围同时考虑双方 min／max。固定侧栏保持宽度，其余面板按比例分享余量并逐步钳制。最小值总和过大时溢出，最大值耗尽时留白，不篡改约束。
- Tab／Shift+Tab，正确轴的方向键，Shift 大步长，Home／End，Enter／双击复位。无变化不提交，拖动结束只提交一次，保留抓取偏移并消费最终 mouse-up 坐标。
- 禁用只影响调整器，内容继续可操作；卸载、失活和结构替换取消会话，不伪造提交。生命周期订阅由 keyed state 持有，弱引用尺寸 owner，不安装空闲计时器或持续帧循环。
- AccessKit Splitter 角色、名称、线方向、当前值及有效范围；注册 Increment／Decrement。headless 检查了语义快照，原生 UIA／Narrator 未验收。

Base 的新约束与键盘行为通过 `with_interaction` 显式启用，旧分栏默认保留原分配方式；横向组的交叉轴伸展修正适用于两种模式。维护记录见 `vendor/gpui-base/YUE-PATCH.md`。本组件没有新增 GPUI core 补丁。

源文件：`crates/yue-component/src/resizable/{mod.rs,tests.rs}`，Base 的 `resizable/{mod.rs,panel.rs,resize_handle.rs,bounded.rs}`。根出口和 prelude 均已接入。README 与设计文档覆盖用法、状态、回调、尺寸异常与限制；组件 Rust 文档含实际编译示例。

## 外观与示例

11 个专属 token 来源为相邻 Yue-Design-System 的 `packages/tokens/src/component-tokens/resizable.css`，已登记聚合入口、冻结词汇、外部消费者和对比度配对。本仓通过 sync 工具生成主题及 inventory，没有手改生成值。可见线 1px、透明命中带 9px、焦点／活动宽度 2px、默认最小面板 100px、键盘步长 8／32px，均按当前窗口 rem 换算。焦点令牌使用 `outline-color/width`，与现有命名语法一致。

自定义线处于普通绘制顺序，弹层可以覆盖它。绘制回归确认线位于命中带中心，避免早期实现在整个热区内偏移的问题；断言按测试窗口 scale_factor 将绘制像素换回布局像素，不声称因此完成了真实跨 DPI 验收。

Showcase 入口：**组件 → 可调整分栏 Resizable**。

1. 左右集合／详情，含可编辑输入和提交计数。
2. 上下预览／输出，演示方向键与复位。
3. 文件树／工作区／属性，工作区再嵌套上下日志区，复用 YueTree、YueInput、YueScrollbar 和 YueButton。

三个示例共用禁用、恢复默认和局部主题预览控件；各分栏拥有独立保留状态。自动测试操作真实控件与键盘，检查独立布局、默认尺寸和输入内容保留。

## 验证证据

实际命令与结果（完整日志保留在本仓 `target/resizable-*.log`）：

```text
cargo test -p yue-component --features headless-tests --lib --offline
test result: ok. 502 passed; 0 failed; 0 ignored

cargo test -p windows-showcase --features headless-tests --offline
test result: ok. 51 passed; 0 failed; 9 ignored

cargo test -p yue-component --doc --offline
test result: ok. 38 passed; 0 failed; 0 ignored

Base resizable 专项（命令见下）
test result: ok. 20 passed; 0 failed; 0 ignored

cargo clippy -p yue-component -p windows-showcase --features headless-tests --all-targets --offline -- -D warnings
Finished dev profile / exit 0

node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check
Yue GPUI theme mapping is current

git diff --check
exit 0
```

Base 独立测试显式使用本仓补丁，避免独立 manifest 解析到注册表版本：

```powershell
cargo test --manifest-path vendor/gpui-base/Cargo.toml `
  --config 'patch.crates-io.gpui-pre.path="D:/100_Projects/110_Daily/Yue-GPUI-Component/vendor/gpui-pre"' `
  --config 'patch.crates-io.gpui-pre-windows.path="D:/100_Projects/110_Daily/Yue-GPUI-Component/vendor/gpui-pre-windows"' `
  --features test-support --lib resizable --offline
```

12 项新增 Yue 测试涵盖实际几何、双边尺寸限制、键盘、抓取点／最终释放坐标、双击复位、纵向语义、固定栏／容器变化、恢复／重排、禁用、rem、会话取消、空／单面板、稳定重绘与局部主题焦点绘制。Showcase 新增 1 项集成测试。Base 原有 19 项测试完整保留并通过，另增 1 项尺寸反馈回归。

尺寸反馈回归先失败后修复：原逻辑把 100px 当作尚未测量标记，显式约束分栏中的合法 100px 也会被判断为需更新。`resizable-minimum-base-before.log` 记录 `bounded_minimum_size_is_not_an_unmeasured_sentinel` 失败；修复将该哨兵限定于旧模式，最终 Base 20 项通过。没有删除测试、跳过新测试或放宽原有约束。Showcase 的 9 项 ignored 是既有手工窗口／性能测试，没有计入已执行验收。

## 主题来源与外部门禁

实际运行相邻设计源 `corepack pnpm audit:tokens`，exit 1：

```text
architecture: 1522 declared tokens across 62 reachable files,
39 catalogue groups in 39 files
unreachable 0, undeclared 0, duplicates 0, cross-file slots 0, layer violations 0
Resizable grammar problems: 0
Other grammar problems: 358
```

新增 6 组分栏配对在浅／深主题共 12 项全部 PASS，最低 4.16:1，满足设定的 3:1。静止线是结构提示，禁用线为非活动控件，不将它们描述为活动状态对比度通过。

按设计源 review 流程执行 `corepack pnpm verify:all`，exit 1。类型检查及构建完成；测试结果 **36 文件通过／7 失败，806 测试通过／14 失败**。失败涉及既有 API 文档、架构统计／库存快照、改名表、原型一致性、token 语法、动效与未使用令牌清单。新 token 会影响清单统计；没有为使总门禁通过而重写其他组件快照或弱化冻结检查。失败类别与此前 Scrollbar 交付记录一致。串联的后续 docs／dist／treeshaking／visual／tarball 等阶段未执行，不声称整个设计源全绿。日志：`resizable-token-audit.log`、`resizable-design-system-verify.log`。

Vue SFC／DOM／SSR、浏览器强制色 CSS 和 npm tarball 不是本 Windows Rust 组件的产物；其验证不能替代 GPUI 的输入／布局与原生构建。相邻仓库的 token 来源和本仓生成产物需要一起保留，不能只提交 Rust 快照。

## 原生构建与实机交接

```powershell
cargo rustc -p windows-showcase --bin windows-showcase --offline -- --emit=link=D:/100_Projects/110_Daily/Yue-GPUI-Component/target/debug/windows-showcase-resizable.exe
```

实际结果：`Finished dev profile`，exit 0。独立程序 `target/debug/windows-showcase-resizable.exe` 已生成，未启动窗口或结束任何用户进程。

用户负责 Windows 实窗验收：打开 Resizable 页，检查边界命中、拖动／松手、双击、Tab／方向键、固定侧栏和嵌套内容滚动；切换禁用与主题，恢复默认并确认输入保留。额外关注窗口最小化恢复、系统缩放和高对比度。真实拖动观感、掉帧、跨显示器 DPI、UIA 操作及 Narrator 尚未设备验收，headless 结果不构成这些能力的证明。

改动保留在工作区，未自动提交或推送；其他组件既有改动保持原样。
