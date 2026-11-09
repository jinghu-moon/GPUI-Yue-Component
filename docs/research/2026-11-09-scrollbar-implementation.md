# YueScrollbar 实施与验收记录

日期：2026-11-09。对应 [设计与实际 API](../39-scrollbar.md)。

## 身份与合同

Windows / GPUI 原地 overlay；RenderOnce，复用宿主唯一 ScrollbarHandle，不引入第二个 offset／Entity／smooth 引擎。参考已确认 HTML 原型，吸收悬浮圆头滑块、固定外侧边、透明命中、按页点击；排除可见轨道、装饰槽、原型独立键盘焦点以及新增 COM provider。

`YueScrollbar::{vertical,horizontal,both}(id,&handle)`、`visibility(Auto|Hover|Always)`、可选 `theme`／viewport bounds 已导出。默认 Auto；仅真实溢出轴显示。Hover 面向整个所属视口，Auto 支持隐藏后的边缘唤醒。双轴一个实例，只有两轴真实溢出才保留角区。

没有改业务列表选择、键盘事件合同或虚拟化算法。Base 行为为显式启用；原默认几何／点击跳转保持兼容。新组件无内置可见文字、无新 locale key；示例文案由 Showcase 宿主所有。滚动条不增加 Tab 停靠点，鼠标操作保持内容焦点，键盘仍由 Tree／List／编辑器等视口承担。

## 源码与迁移

- `crates/yue-component/src/scrollbar/{mod,tests}.rs`：token 投影和真实布局／输入回归。
- `application.rs`：一次安装按 Window 解析的全局 Base resolver，支持内部编辑器／Text 滚动条；局部 rem 不写进全局 Pixels。
- `virtual_list.rs`：YueVirtualList 统一入口，覆盖其 List／Tree／TreeSelect／Select 消费者。
- `time_picker/panel.rs`：各时间列沿用原 handle 和唯一 id。
- Showcase `scroll.rs`：页面、侧栏等现有包装统一呈现。
- Showcase 虚拟 List、Grid、Masonry、横向示例：替换旧 Base 调用；同一 Grid 两轴合为一个实例。
- Showcase 新「滚动条 Scrollbar」页：复用 YueSegmented／YueButton，策略、短内容、局部深色、两轴与回到起点都操作实际滚动容器。
- Base 和 GPUI 补丁均登记 YUE-PATCH，写明迁移／移除条件。

普通 `.overflow_*_scroll()` 不自动绘制滚动条；没有给原本隐藏滚动条的短条、选项容器擅自增加实例。统一的是现有可见滚动条入口，保留所有原滚动 owner。

## 几何、动效和清理

rem=16 时 4→8px，14px 透明命中，3px 留白，最短可见长度32px。rem=20 对应5→10、17.5、3.75、40px；paint bounds 会对齐物理像素，因此几何验收允许半个物理像素的绘制取整，不能据此声称 DPI 设备验收完成。

所有 track normal／hover／active 背景和边框显式透明。点击可见边缘空白按90% viewport翻页；拖动立即写原 owner，保留抓取点和另一轴。右键不触发翻页或拖动。页面外侧内容 Button 点击、焦点和滑块区域 wheel 均有回归。

透明度和宽度沿用100ms Interaction／Exit，以及 Fade／Standard token曲线。改为 executor clock，支持真实受控时间测试和中途反向。reduced motion、失活／隐藏窗口直接完成动效。idle等待800ms，不在鼠标每次移动时创建新一批任务。

Base DragLease 保证卸载调用 `end_drag`；idle／throttle任务只持有轻量 state，不保活拖动租约。浮动拖动显式使用 `capture_pointer_for_frame`，跨帧续接、卸载释放；失活、隐藏、溢出消失、显式捕获取消和 mouseup 清理。GPUI 原 `capture_pointer` 未改生命周期。

## 已运行证据

```text
Base 修改前：45 scrollbar/editor tests passed
Base 修改后：46 passed; 0 failed
Yue library：487 passed; 0 failed
最后清理增补后 scrollbar 专项：11 passed; 0 failed
Showcase：50 passed; 0 failed; 9 ignored
Input integration：10 passed; 0 failed
List integration：10 passed; 0 failed
ListGrid integration：10 passed; 0 failed; 1 ignored
VirtualList integration：191 passed; 0 failed; 11 ignored
Showcase Clippy --features headless-tests --all-targets -- -D warnings：exit 0
Yue Clippy --features headless-tests --lib --tests -- -D warnings：exit 0
Theme source sync --check：Yue GPUI theme mapping is current
```

Base 独立命令需显式 patch 本仓的 gpui-pre 与 gpui-pre-windows：

```powershell
cargo test --manifest-path vendor/gpui-base/Cargo.toml `
  --config 'patch.crates-io.gpui-pre.path="D:/100_Projects/110_Daily/Yue-GPUI-Component/vendor/gpui-pre"' `
  --config 'patch.crates-io.gpui-pre-windows.path="D:/100_Projects/110_Daily/Yue-GPUI-Component/vendor/gpui-pre-windows"' `
  --features test-support --lib scrollbar --offline
```

最终库包含11项 scrollbar测试；覆盖布局、rem、多窗口、两轴动态溢出、无可见轨道、策略、翻页／拖动、取消／卸载、右键、内容焦点／wheel、受控时钟以及对比度。实际输入和布局是主要断言，不以复述实现公式替代交互验证。

Showcase原快速拖动测试最初失败：测试窗口没有激活，新生命周期正确终止其拖动。修正测试的激活步骤后保持原150次单批构建／100行挂载预算，实际通过；没有放宽预算或移除断言。原有9项 Showcase ignored 是手工原生窗口／探针，12项集成ignored是既有手工性能检查，未作为已执行结果。

稳定颜色测量：Light／Dark × Azure／Neutral × 四种 NeutralTone × page／surface／popover × 三状态全部达到3:1，最低3.822:1。淡入淡出中间透明度不计作静态对比保证。

原 `cargo build -p windows-showcase --offline` 被占用的 `target/debug/windows-showcase.exe` 阻止，报 `failed to remove file ... os error 5`。保留正在运行的用户进程，使用以下命令成功链接独立验证程序：

```powershell
cargo rustc -p windows-showcase --bin windows-showcase --offline -- --emit=link=D:/100_Projects/110_Daily/Yue-GPUI-Component/target/debug/windows-showcase-scrollbar.exe
```

构建 `Finished dev profile` / exit0；程序未自动启动。源/测试更新及最后一次构建证据保留在本仓target下 `scrollbar-*.log`，日志不作为版本化交付。

## Token 来源与完整门禁

邻仓 Yue-Design-System 新增 `packages/tokens/src/component-tokens/scrollbar.css` 共15 token，登记 `_index.css` 和 `tools/lib/token-vocabulary.mjs`。本仓通过既有 sync 工具生成主题与 inventory，没有手改生成快照。

```text
Scrollbar frozen vocabulary problems：0
Architecture：1510 declared tokens / 61 reachable files / 38 catalogue groups
unreachable 0 / undeclared 0 / duplicates 0 / cross-file slots 0 / layer violations 0
```

**邻仓完整门禁失败，未声称全绿。** 实际运行 `corepack pnpm audit:tokens` exit1：358项既有 token grammar问题，Scrollbar为0。实际运行 `corepack pnpm verify:all` exit1：36文件通过／7失败，806测试通过／14失败；包括历史清单／文档计数、prototype对齐和Collapse等失败，后续串联阶段未执行。日志为 `target/scrollbar-token-audit.log`、`target/scrollbar-design-system-verify.log`。这与本轮前Tree实施记录中的基线相同，不扩展本任务去重写其他组件快照。

Vue 的SFC、SSR、DOM、打包入口／tarball不是本Rust组件交付物；完整邻仓命令已执行并如实记录失败。token来源改动与本仓生成产物需一并保留／提交，不能只提交生成文件。

## 实机交接与限制

用户负责当前Windows设备实窗验收：重启或运行独立验证程序，在Scrollbar页切换策略／长短内容／局部主题；再查看侧栏、List／Tree／Select弹层、TimePicker、虚拟Grid与多行编辑器。检查4→8px观感、抓取后拖出窗口、滚轮／触控板、窗口最小化恢复及高对比度。

本轮没有启动用户窗口、结束既有进程或声称实机帧率提升。多显示器／跨DPI、触控板、Narrator及系统高对比度未完成设备验收。全局Base颜色仍是应用级；支持不同窗口rem，不承诺内部Base消费者各窗口独立调色板。独立UIA ScrollBar／COM Provider明确不在首版合同中。

改动留在工作区，未自动提交或推送；保留其他组件既有修改。


## 2026-11-09：按实窗反馈补齐 HTML 动效

用户指出实际滚动条没有 demo 中的动效。本轮对照 HTML 的 `.rail`／`.thumb::before`：opacity、width／height、background 都使用140ms、`cubic-bezier(.2,0,.2,1)`。前版保留了宽度／透明度100ms通道，遗漏颜色 tween，淡出还复用了位移播放器，与原型不一致。

先写实际布局／指针回归，两项在修复前都失败：hover首帧从Hsla亮度0.45098瞬跳0.32157；淡出中右边缘从237px移到242px。日志 `target/scrollbar-demo-before.log`。修复后新增两个case均通过，不以修改预期掩盖缺陷。

实现补充：

- Base `ScrollbarMotion::with_color` 提供默认关闭的纯色背景插值，颜色和厚度共享传入曲线；颜色独自变化也请求下一帧。渐变背景维持原行为，不引入通用背景播放器。
- 浮动 Fade 保持 position=1，进入／退出均只改变透明度，Legacy Base 的显隐位移合同保持原样。
- scrollbar专用 token改为HTML140ms／相同曲线，新增duration-color别名，共16项；通过设计源同步生成，不改变全局Interaction／Exit token。
- 进入、按下、松开／移出均从当时已采样值向新目标过渡。受控时钟在70ms检查HTML曲线的实际sRGB颜色／透明度；通过 `simulate_next_frame` 驱动自身回调，不手工refresh来伪装续帧。检查稳定后回调数量归零，reduced motion同时直接完成厚度和颜色。

旧文中的100ms／15 token是前版实施证据，当前规范以更新后的设计文档§6／§8为准。原有宽度回归的最终等待从100ms改为140ms，保留中间厚度、反向连续和终态断言。

本轮实际命令与日志：`scrollbar-demo-after.log`（13项滚动条专项通过）、`scrollbar-motion-base-tests.log`（46通过）、`scrollbar-motion-yue-tests.log`、`scrollbar-motion-showcase-tests.log`、`scrollbar-motion-clippy.log`、`scrollbar-motion-build.log`。Windows实窗观感继续由用户验收。

来源门禁已重新运行：audit:tokens exit1，架构1511 token／61文件／38组、未声明／不可达／重复／跨层均0；Scrollbar词汇问题0，其他既有grammar问题仍358。verify:all继续在既有Vue测试阶段失败，未把它当成GPUI动效通过的证据。详见 `target/scrollbar-motion-token-audit.log` 与 `target/scrollbar-motion-design-verify.log`。


本轮最终验证：Yue库489通过／0失败，Showcase50通过／0失败（原有9项手工测试未运行），Base46通过／0失败；两包Clippy `--features headless-tests --all-targets -- -D warnings` exit0。专用动效验收程序输出为 `target/debug/windows-showcase-scrollbar-motion.exe`，沿用前述 `cargo rustc --emit=link=绝对路径` 方式链接；未自动启动或关闭任何用户窗口。先前两个带 scrollbar 后缀的程序属于前版构建，验收这次动效应使用 motion 后缀新程序或重新构建标准程序。

## 2026-11-09：拖动刷新与显示帧同步

用户反馈多个页面拖动均有卡顿感。共用 Base 路径虽然逐次写入最新 offset，但宿主刷新仍使用最高120Hz的独立定时器。先增加真实指针／绘制回归，修复前失败在“下一显示帧必须收到刷新请求”，见 `target/scrollbar-drag-before.log`。这确认了调度间隙，不将其直接等同于已测得的实机掉帧原因。

新增默认关闭的 `ScrollbarBehavior::with_frame_driven_drag`，Yue 投影统一开启，覆盖直接入口及内部 Base 消费者。鼠标／触摸仍立即更新已有 owner；只在首次尚未呈现的输入注册一个 `on_next_frame`，由该帧通知宿主，后续输入合并，不创建拖动定时任务。松手／取消的 generation 使旧回调失效；回调不保活拖动租约。Base 默认及隐藏的 `max_fps` 保持原定时器合同，Yue 选择显示帧调度后不受该上限限制。不增加位置缓动或第二份 offset。

回归将可交互内容预先滚出指针路径，隔离按钮的 hover／focus 通知；保留实际滚动内容和双轴滑块。连续32次指针输入逐次更新原 handle，下一显示帧仅执行一次请求、通知一次宿主，绘制滑块对齐最新指针及原抓取点；按住不动不持续续帧，松手使尚未执行的回调失效。真实窗口观感和帧率仍由用户验收，不以 headless 调度结果宣称实机性能提升。

最终验证：Yue库490通过，Showcase50通过／9项原有手工测试未运行（快速拖动仍满足原构建预算），Base滚动条／输入专项46通过；两包 `--features headless-tests --all-targets` Clippy零警告，`git diff --check`通过。日志为 `target/scrollbar-drag-{yue-tests,showcase-tests,base-tests,clippy,build}.log`。原生验收程序为 `target/debug/windows-showcase-scrollbar-drag.exe`；上一版 `windows-showcase-scrollbar-motion.exe` 保留用于对比，未启动或关闭用户窗口。未自动提交／推送。
