# Tree / TreeSelect 首版实施记录

日期：2026-11-09。设计入口：[YueTree](../37-tree.md)、[YueTreeSelect](../38-tree-select.md)。平台为 Windows / GPUI。

## 1. 实现结果

| 层 | 交付与复用 |
| --- | --- |
| Base | 新增 opt-in `keyed_tree::KeyedTreeModel`，统一管理节点、active、selected、checked、expanded、过滤和加载票据；旧 Tree / TreeState 保留原模型及默认行为 |
| YueTree | 三尺寸、固定行高虚拟视口、箭头／节点图标／连线、单选／修饰键多选、Strict／Cascade、半选与投影、搜索、reveal、懒加载及失败重试、明确的行内容与操作槽位 |
| YueTreeSelect | 组合一个 YueTreeState，支持 Single／Multiple／Checked；本地／远程搜索、过期响应隔离、未知值快照、标签折叠／换行／移除、只读／禁用、LeavesOnly、数量上限与面板槽位 |
| 共享字段 | 从 Select 抽出 crate-private `choice_field`，共享尺寸换算、文本测量、折叠预算、字段几何及 Input surface；不共享第二份值状态 |
| 基础组件 | 直接复用 YueInput、Checkbox mark、YueTag、YueEmpty、Spinner、Icon、OverflowTooltip、Popup、Scrollbar 和既有 virtualization；字段接入 YueField / YueForm |
| 语义 | Base TreeItem 元数据与 ComboBox `HasPopup::Tree`；树行提供选择／勾选、层级、同级位置与数量，Focus、Click 和分支 Expand／Collapse 进入同一状态 owner |
| 示例 | 独立 **组件 → Tree** 和 **组件 → TreeSelect** 页面，前者含 100×100 子节点虚拟树，后者含 12 个独立字段状态与 Form |

源码入口：[Base model](../../vendor/gpui-base/src/tree/keyed.rs)、[Tree](../../crates/yue-component/src/tree/mod.rs)、[TreeSelect](../../crates/yue-component/src/tree_select/mod.rs)、[共享字段](../../crates/yue-component/src/choice_field.rs)。示例：[Tree 页面](../../examples/windows-showcase/src/pages/tree.rs)、[TreeSelect 页面](../../examples/windows-showcase/src/pages/tree_select.rs)。

Base 只新增通用行为及语义接口；Yue token、图标、业务对象、网络任务与 Popup 生命周期均不进入 Base。补丁升级／移除条件见 [YUE-PATCH](../../vendor/gpui-base/YUE-PATCH.md)。没有增加 Windows COM provider。

## 2. 重要行为边界

- key 全局唯一且非空；非法数据原子拒绝。导入后节点只保留元数据，真实 children 通过 keyed 索引读取，不在每个虚拟行克隆子树。
- active、行选择、复选框勾选互相独立。独立树单击选择、双击／Enter 激活；TreeSelect 单击即提交／切换。箭头和 Checkbox 不冒泡为第二次行操作。
- Strict 保持选入顺序；Cascade 保存规范叶子 key，Parents／All／Leaves 只改变派生展示。disabled／check_disabled 作为传播边界，透明的 checkable(false) 中间层允许祖先传播。
- 未知子树不能整支级联勾选；加载新叶子不会隐式选中。根加载也会清理不再可选／可勾及由叶子变分支的旧值，并保留仍有效的展开 key。
- 完整 `set_nodes` 可确认删除；远程结果快照缺席不代表删除。搜索仅保留当前结果与已选快照，避免无限累计历次结果。
- 搜索票据区分 owner、会话与 generation；加载票据另有请求序号。关闭／重开／重查／数据替换后，旧响应无法提交。不同分支的并发加载互不失效。
- query 保留真实 InputState。专属 GPUI actions 在 Base Input 处理前读取 marked range，避免 Escape 先结束组合后又误关面板；自动测试不等于已验证真实 Windows IME 候选窗口。
- active 按 key 保留挂载；行操作 F2 进入、Tab 遍历、Escape 返回，过滤／删除前修复焦点。折叠标签 `+N` 可用键盘查看和删除；Tag 删除后迁移焦点到相邻项或字段。
- 默认整行命中、固定行高；后续动效增补已加入箭头旋转与有预算的子树高度过渡，详见第 7 节。没有内容区命中开关、fill 高度或任意 renderer 的自然宽度测量。Content 面板宽度使用已知可见标签与层级槽位估算。

## 3. 自动验证

以下为当前实现已执行的专项检查。子串过滤命令会包含名称重叠的测试，数量不重复累加。

| 检查 | 结果 |
| --- | --- |
| `cargo check -p windows-showcase --offline` | 通过，两个新页面及其路由可编译 |
| Yue `--lib tree` | 行操作连线修复后 38 通过：Tree 15、TreeSelect 23 |
| Yue `--lib select::tests` | 动效增补后 56 通过，其中旧 Select 33、TreeSelect 23 |
| Yue `--test checkbox --test field` | Checkbox 11、Field 3 通过 |
| Base fixture `--lib tree::` | 28 通过：keyed 16、旧 Tree 10、名称匹配的 Dock 2 |
| Base fixture `--lib select::tests` | 15 通过 |
| Base fixture `--lib a11y_disabled::tests` | 4 通过，含 HasPopup::Tree 与原 Select 默认值兼容 |
| Yue library Clippy `-D warnings` | 通过 |
| 新增 Rust 文件格式与 Git whitespace 检查 | 通过；没有运行全仓自动格式化 |
| token 同步 `--check` | 通过；生成内容与邻仓来源一致 |

组件检查命令：

```powershell
cargo test -p yue-component --features headless-tests --lib tree --offline
cargo test -p yue-component --features headless-tests --lib select::tests --offline
cargo test -p yue-component --features headless-tests --test checkbox --test field --offline
cargo clippy -p yue-component --features headless-tests --lib --offline -- -D warnings
cargo check -p windows-showcase --offline
```

Base 在现有隔离 manifest 中验证，指向当前 vendor 源码，避免把被 workspace 排除的 vendor 当作普通成员：

```powershell
cargo test --manifest-path target/gpui-base-contract/Cargo.toml --target-dir target --features test-support --lib tree:: --offline
cargo test --manifest-path target/gpui-base-contract/Cargo.toml --target-dir target --features test-support --lib select::tests --offline
cargo test --manifest-path target/gpui-base-contract/Cargo.toml --target-dir target --features test-support --lib a11y_disabled::tests --offline
```

覆盖的可观察合同包括：真实指针／GPUI 按键路由、一次操作一次值事件、Shift 锚点、IME marked range、字段 Tab 退出、+N 标签焦点、禁用／只读切换、非法赋值不破坏会话、未知快照锁定、权威删除、远程乱序／关后返回／卸载后返回、加载重试，以及 Tree／TreeItem 语义不退化为 ListBox／Checkbox。

原生语义属性及 HasPopup 在自动测试中检查；新增行 Focus／Expand／Collapse 的真实辅助技术分发仍属于 Narrator 人工验收范围，不宣称已做 UIA 端到端验证。

## 4. Token 来源与全量门禁状态

新增来源位于邻仓 `../Yue-Design-System/packages/tokens/src/component-tokens/tree.css`，共 18 个 Tree token；同时登记 import、词汇表、package-only 清单、对比度配对和 GPUI 外部消费者说明。本仓通过现有同步工具更新 `generated.rs` 与 `inventory.json`，没有手改生成值。

```powershell
node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --write
node tools/sync-yue-theme.mjs --design-system ../Yue-Design-System --check
```

Tree 命名语法专项为 0 问题。4 对语义配色在 light / dark 下全部通过：

| 配色 | Light | Dark |
| --- | --- | --- |
| 普通文字／背景 | 16.48 | 16.29 |
| hover 文字／背景 | 14.09 | 11.67 |
| selected 文字／背景 | 7.32 | 10.53 |
| 焦点轮廓／背景 | 4.54 | 9.57 |

**邻仓完整门禁未通过。** `corepack pnpm audit:tokens` 报告总计 358 项非 Tree 的命名规则问题；architecture 为 1495 token、60 个可达文件、37 个 catalogue group，未声明／重复／不可达／跨层错误均为 0。`corepack pnpm verify:all` 的测试阶段为 36 个文件通过、7 个文件失败，806 项通过、14 项失败，未继续证明后续阶段成功。

失败涉及现有各组件 token 词汇、历史清单／文档计数／prototype 对齐、Collapse 动效 token 和消费登记。新增 Tree 后全量计数同样需统一更新；本轮只完成 Tree 注册，不将整体门禁描述为绿色，也没有顺带重写其他组件的来源或历史快照。日志保留在本仓 `target/tree-token-audit.log`、`target/tree-design-system-verify.log`，target 为本地检查产物，不作为版本化交付文件。

## 5. 性能证据及限制

一万节点交互测试检查只挂载视口加 overscan／retained active 行、End 定位末节点，以及导航与选择不重建相同投影。一万层深树测试检查导入和展开不依赖递归调用栈，所有可见行共享 O(N) 连线索引，避免为每层复制祖先数组造成 O(depth²) 存储。

这些是复杂度与挂载合同，**没有完成相同构建条件的前后耗时／分配基准，也没有实机帧率结论**。首版实现不能由这些测试推导出所有规模和所有展开／过滤操作都具有固定耗时。

## 6. 人工验收与范围

用户负责 Windows 实窗视觉、明暗主题、实际 IME、Narrator、当前设备 DPI、滚轮／触控板。建议从 Showcase 依次检查：

1. 三尺寸行高、图标列、长标题、深层连线、焦点轮廓及窄面板。
2. 方向键、Ctrl／Shift、F2 行操作、失败分支重试及虚拟树末端定位。
3. TreeSelect 单／多／勾选、Parents 标签移除、+N、Wrap、清空、数量上限、只读与必填。
4. 搜索候选状态、快速交替远程查询、`error` 失败重试、Escape／Tab、Popup 翻转和窗口边界。
5. Narrator 对 TreeItem 层级／位置／勾选／展开的朗读与动作，以及 Dialog 内的焦点路径。

未承诺多显示器或当前设备无法执行的 DPI 切换验收。节点拖放、Tag 拖排、TreeGrid、远程分页合并与可变行高留在明确的后续范围。

改动当前保留在工作区，未提交或推送。工作区还有其他组件的既有修改；后续提交时需同时管理本仓补丁与邻仓 token 来源，不能只提交生成文件而丢失来源。

## 7. 开合动效增补（2026-11-09）

用户验收指出原版没有展开／折叠动画。本次在 Yue 呈现层增加默认箭头旋转与子树高度过渡，TreeSelect 直接继承。复用现有 motion 语义时长与曲线，没有修改邻仓 token 来源、generated、Base 模型或公共 API。

虚拟视口内部改用现有 Known 几何，使每帧通过 keyed size batch 更新变化行，而不是重建整棵树／整个尺寸数组。静止业务行仍为固定高度。动画结束后清除退出占位与快照；快照缓存以视口 paint 为边界，只保留上一帧实际挂载行。折叠后不保留后代 element、焦点控件或可访问节点。方向导航进入子项前完成当前高度过渡，搜索／数据替换／主题／尺寸变化亦直接完成，避免过时几何参与定位。

完整时长采用当前 Interaction 100ms、Enter 200ms、Exit 100ms，反向时按剩余距离缩短。每次最多过渡 128 个变化行，超预算或同一帧多支不相关的插入／移除直接完成行布局，箭头仍可旋转。初始化、reduced motion、隐藏／失活窗口直接完成；离屏 retained 箭头不持续请求帧。

新增回归使用真实 GPUI 指针／键盘与可控时钟，检查中间高度、组件自行请求动画帧、反向连续、立即卸载后代语义、值／事件不变、方向目标可见、数据替换取消旧几何、窗口失活、rem 变化及一万子节点挂载预算。TreeSelect 的专用动效用例另检查字段值、打开会话与 Escape。

增补后实际验证：`--lib tree` 为 `36 passed; 0 failed`，`--lib select::tests` 为 `56 passed; 0 failed`；Showcase `cargo check` 与 library Clippy `-D warnings` 均 exit 0。新增 Rust 文件 rustfmt、Git whitespace、文档链接及 token 同步检查通过。本轮未修改 token 来源或 Base，未重跑邻仓已记录失败的全量门禁。

## 8. 连线对齐修复（2026-11-09）

竖线位置补入缩进槽之后的 flex gap，并以半线宽修正左边坐标，使线中心与对应祖先展开箭头的向下尖端对齐。修复前，真实布局回归在 Small / rem=16 下测得线中心 16.5px、箭头尖端 20px；修复后两层祖先连线在 Small / Medium / Large、rem=16 / 20 的全部组合通过。`--lib tree` 为 `37 passed; 0 failed`，Showcase `cargo check` 通过。

随后按用户要求将来源 `--tree-line-width` 从 1px 改为 2px，并重新同步主题与 inventory。随 rem 缩放，rem=20 时宽度为 2.5px；原绝对边缘布局在该组合出现 0.25px 中心偏差，因此改为按祖先箭头同宽槽位布局，再在绘制阶段从槽位中心向两侧各扩半线宽。既有三尺寸／两种 rem 对齐回归及全部 37 项 Tree / TreeSelect 测试通过，token 同步检查通过。未重跑邻仓先前记录失败的全量门禁；真实像素观感仍由用户验收。

## 9. 行操作撑高导致连线断点（2026-11-09）

Small 树行高 28px，但示例“复制路径”使用默认 32px 按钮。内部内容容器未显式限制行高，随按钮变高后将居中的缩进连线下移 2px；虚拟行边界裁剪后出现上沿断点。修复为内容容器显式使用树行高，确保自定义 actions 不改变连线纵向几何；示例按钮同时改用 Small。回归保留默认尺寸的较大按钮，检查三档树尺寸、rem=16 / 20 下连线上沿与本行对齐、下沿衔接下一行。修复前该测试实测 62px 对 60px 并失败；修复后 `--lib tree` 为 `38 passed; 0 failed`。

真实窗口动画观感仍由用户验收；未据此声称实机帧率提升。原有全量 token 门禁的状态不因本轮没有来源改动而变为通过。
