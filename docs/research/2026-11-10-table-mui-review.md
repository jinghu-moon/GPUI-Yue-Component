# YueTable 源码调研 05：MUI X Data Grid

日期：2026-11-10。状态：本地源码、类型和定向测试代码阅读完成，结论已补入 [YueTable 设计](../45-table.md)。仅调研与设计；未安装依赖、运行参考测试、启动应用或测量性能，没有实现 Table。

上一项：[RevoGrid](2026-11-10-table-revogrid-review.md)。按用户要求一次一个项目，本轮不研究后续仓库。

## 1. 快照、许可和阅读范围

| 项目 | 核对结果 |
| --- | --- |
| 目录 | `refer/DataTable-Libs/mui-x`，独立 Git 仓库，阅读时工作区干净 |
| HEAD | `4126d61d4004efef21b432c029b4acd6b3a199fe` |
| Community | [package.json](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/package.json)：`@mui/x-data-grid` **9.17.0**；[LICENSE](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/LICENSE) 为 MIT |
| Virtualizer | [package.json](../../refer/DataTable-Libs/mui-x/packages/x-virtualizer/package.json)：`@mui/x-virtualizer` **0.7.5**；[LICENSE](../../refer/DataTable-Libs/mui-x/packages/x-virtualizer/LICENSE) 为 MIT |
| Pro／Premium | 两包均为 **9.17.0**；[Pro LICENSE](../../refer/DataTable-Libs/mui-x/packages/x-data-grid-pro/LICENSE)、[Premium LICENSE](../../refer/DataTable-Libs/mui-x/packages/x-data-grid-premium/LICENSE) 为商业许可，不能把整个 monorepo 当 MIT 代码来源 |
| 功能归属 | [许可说明](../../refer/DataTable-Libs/mui-x/docs/data/introduction/licensing/licensing.md)将多列排序／多条件过滤、列伸缩／固定列列为 Pro，分组／Excel 等列为 Premium；公开文件可见不等于可按 MIT 移植 |
| 仓库规则 | 已阅读根 [AGENTS.md](../../refer/DataTable-Libs/mui-x/AGENTS.md)；未发现其提及的 AGENTS.local.md。本轮不修改参考源码 |

主读 MIT Community 和 virtualizer：选择器／订阅、数据更新、尺寸缓存、编辑流程、布局与交互回归。Pro／Premium 仅核对包许可、功能归属和目录边界，没有深入商业功能算法或将其测试作为已读机制的证明；Yue 既定固定列等能力仍基于已有 Base 合同自主设计，本轮无代码移植。

Context7 查询 `/mui/mui-x`，交叉理解 `updateRows` 的 Community 单行限制、`processRowUpdate` 返回最终行及编辑去抖；在线材料不替代该 HEAD，也不把介绍文档中的旧版本当当前快照。

## 2. 订阅要补挂载窗口，派生值要来自一致快照

证据：[useGridSelector.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/utils/useGridSelector.ts)、[createSelector.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/utils/createSelector.ts)。

`useGridSelector` 记住计算时的 store state；快照身份没变便跳过重复选择器调用，派生结果相等则不 setState。参数有浅比较，结果默认 Object.is。订阅在挂载阶段建立，并先调用 updateState，补上首次 render 到订阅之间的 store 更新；释放时解除订阅。不能因此声称所有不相关 listener 都不会执行，或这是一张按字段依赖精确调度的图。

当前 createSelector 将 apiRef 解到 state 后委托 `@base-ui/utils/store`；不能沿用旧版本印象，写成此快照仍以 reselect 或自制 instanceId 缓存为核心。

Yue 吸收三点：首次读取至绑定之间核对 revision；同次接受的内容／计数／选择／语义不混用新旧切片；多个表格互相隔离。复用 Entity、Subscription 和 GPUI 更新事务，仅给真实需要独立更新的子视图建立窄读取，不为百万逻辑 cell 建订阅，不新增 React store。参考测试的“一次 React commit”不是“一次 GPUI paint”的证明。

## 3. 合批可以减少重算，但不能损坏业务更新顺序

证据：[useGridRows.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/rows/useGridRows.ts)、[gridRowsUtils.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/rows/gridRowsUtils.ts) 的 `updateCacheWithNewRows`。

参考先更新内部行缓存，再按 throttleRowsMs 合并后续行派生与发布；全量 setRows 和局部 updateRows 是不同路径。局部处理中先按 row id 合并重复更新，再协调 insert／modify／remove：新插入再修改仍为插入，删除再插入需撤掉旧删除动作，删除不存在的项是 no-op。还区分 merge 与显式 replace。Community 拒绝一次多行 updateRows 是产品分层，不是合批算法必需。

Yue 宿主处理 typed 业务更新；同 key 两次修改不同字段不能“只保留最后通知”，删除／重建及完整来源替换必须有顺序与代次。Table 只接受一致来源 revision、投影和变化范围，查询／行高依赖仍按原阶段失效。完整换源使旧待执行批次失效，不能让其恢复旧记录。不复制 JS 对象 prototype／私有字段补丁，不新增字段字符串 patch DSL，不设每次单行限制或照抄节流常量。

## 4. 有界缓存还要处理真正删除与闭包保活

证据：[useGridRowsMeta.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/rows/useGridRowsMeta.ts)、[useGridVirtualizer.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/core/useGridVirtualizer.tsx)。

参考明确指出：virtualizer 只知道当前页，不能判断记录已删除；表格依据完整 rowTree 清理不再存在的高度 key，避免留下所有见过的 id。Yue 按已接受结构域／删除／来源代次清理，不以“当前页没有”推断删除；过滤、折叠和离屏仍按有界 key＋约束缓存恢复。Remote 不为未加载的 total 构造缓存。

另一个直接值得借鉴的注释：每次 render 的 virtualizer wrapper 持有当次回调，长期闭包若捕获它，可能让历次 render 连同行数据一直可达；参考改为捕获稳定 virtualizerStore。GPUI 的 Entity、Rc、Task／Subscription 同样需要检查持有链，但这不表示本项目已经查到泄漏。长期绑定读当前 owner，任务结束释放必要快照；验收用 drop／弱引用探针和容量计数，再另测资源字节。

该文件仍把 virtualizer 的 dimensions／rowsMeta／virtualization 同步回 grid，并标注 HACK／TODO。Yue 不复制这层过渡镜像，继续复用 Base 的尺寸与滚动事实。高度条目消失只能证明缓存清理，不证明堆内存或 GPU 资源已下降。

## 5. 编辑保存必须处理待执行输入，校验必须识别草稿代次

证据：[useGridEditing.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/editing/useGridEditing.ts)、[useGridCellEditing.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/editing/useGridCellEditing.ts)。

参考按 id／field 保存去抖任务，新值替换旧定时器；停止编辑前通过 runPendingEditCellValueMutation 执行待处理变更，可按格或整行处理。随后检查错误／isProcessingProps，有问题则保持编辑，不移动焦点。processRowUpdate 可以同步或异步返回最终行；失败留在编辑，成功写回前还检查记录是否存在。Yue 不复制其全部 Promise 分支或默认失焦保存／Enter 换行导航策略。

异步预处理完成时，参考重验是否仍处于编辑，并取当前草稿值，避免旧校验返回值直接覆盖新文本。不过“仍在编辑”不足以证明同一会话 A／B 校验倒序、取消后同格重启都安全；本项目在已有 session＋数据 revision 上补 draft revision，成功、错误和 busy 清理都核对。所有取消／删除／卸载路径须明确结算等待方。

Yue 保留真实 Input Entity 为草稿来源。解析／业务校验或预览若合批，保存／Tab 前读取当前控件并处理所需待执行更新，行编辑覆盖所有参与字段；不能保存滞后镜像，不能在组词未完成时强行 flush。Field／Form 复用既有同步规则与错误呈现，宿主处理异步，不虚构现有 Form 已有通用竞态引擎，也不为了模仿参考新增固定去抖。

测试启发是“输入后立即保存／Tab”“校验拒绝仍为同一 Input”“重排不换编辑实体”；不是为每格预挂载隐藏输入框。Yue 不吸收 experimental 隐藏 editor、打印字符启编入口，也不手工拼接 Windows 字符。合成 composition 测试不能代替真机中文／韩文 IME。

## 6. 先识别滚动条模型，再借鉴尺寸收敛

证据：[dimensions.ts](../../refer/DataTable-Libs/mui-x/packages/x-virtualizer/src/features/dimensions.ts)、[useGridVirtualization.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/virtualization/useGridVirtualization.tsx)、[虚拟化说明](../../refer/DataTable-Libs/mui-x/docs/data/data-grid/virtualization/virtualization.md)。

参考计算容器／可见尺寸、固定区及原生滚动条占位，横纵溢出会相互影响；还记录短时间内滚动条反复切换，在达到条件后抑制纵向滚动并重算横向。Yue 借鉴阈值附近的回归用例，不移植“振荡后强制隐藏”的处理。

[YueScrollbar](../39-scrollbar.md) 是 overlay，不占布局空间。Table 的内容区应正确扣 root 边框／padding 和具名区域；防遮挡留白稳定且只算一次，不能从 MUI 直接移植 scrollbarSize 扣宽高。滑块显隐、悬停变宽与拖动不能触发列宽／文字换行／行高变化。阈值两侧伸缩需有界收敛，复用 Base 反馈预算。

参考 autoHeight 关闭行虚拟，jsdom 路径也不具备真实布局。不能将这两类测试当大数据布局性能或 GPUI GPU 验收；不照抄浏览器最大像素限制、Community 行数限制、buffer 常量或实验性的受控帧策略。现有 Cells＋Measured 拒绝规则保持不变。

## 7. 浮层属于原交互所有者，空状态需区分无列

证据：[GridRow.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/components/GridRow.tsx)、[useGridOverlays.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/features/overlays/useGridOverlays.tsx)。

GridRow 发布事件前排除 portal 目标，并确认当前行仍存在；相关测试检查 portal 输入不触发 rowClick、Dialog 内输入保留焦点。Yue 用已有 Root／Popup／Dialog owner 达成对应边界，不让浮层点击再次变成外层行选择／展开，不在模态对话框输入时抢焦点。嵌套表格按实例隔离；行内 Button／Input 沿用交互排除合同，不照搬其 inline input 可冒泡 rowClick 的细节，不要求每个槽手写 stopPropagation。

参考分别处理 loading、noRows、noResults 和 noColumns。Yue 已有加载／刷新／失败／筛选空合同，本轮补“没有显示的列”：业务数据仍存在，使用 YueEmpty＋已有列配置恢复入口，不清 total／选择，不新增另一种查询事件。关闭表头不是隐藏全部列；加载和失败按 accepted view 策略优先，不能闪现错误空状态。

## 8. 已阅读的测试断言与可移植边界

以下为实际阅读过的定向片段，均未执行，不代表这些文件的全部分支都已审计。测试文件位于 MIT Community 包；浏览器输入／布局与纯状态测试分开解释。

| 参考 | 实际断言 | Yue 的验收落点 |
| --- | --- | --- |
| [useGridSelector.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/hooks/utils/useGridSelector.test.tsx) | StrictMode 开／关：ref 挂载回调在订阅前改值，最终显示 1；同一次 store 更新 a／b 后只观察到 `1:1` 的 commit | 挂载窗口不漏更新，内容／计数／语义同一 revision；不照搬 React commit 计数为 paint 断言 |
| [createSelector.test.ts](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/utils/createSelector.test.ts) | 同 state 结果引用复用；换 state 重算；交错访问另一实例不清掉第一实例结果 | 派生缓存有效性和多实例隔离；不臆测 wrapper 以 instanceId 为缓存 key |
| [rows.DataGrid.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/tests/rows.DataGrid.test.tsx) 的更新与缓存片段 | 连续更新／新增／删除得到预期行；Community 批量更新抛错；高度 key 从 1／2／3 换到 3／4，再删 4 后只剩 3 | 不吸收单行限制；删行清理但换页不误判；删除／插入排列和换源旧批次为基于源码新增的待测用例，不声称原测试全覆盖 |
| 同文件的 portal 片段 | portal input 点击不调 rowClick，inline input 点击会调用；Dialog 输入 a／b 后焦点仍在输入框 | 浮层与嵌套 owner 隔离、无重复行事件；Yue 行内控件不沿用该冒泡策略 |
| [editComponents.DataGrid.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/tests/editComponents.DataGrid.test.tsx) 的长文本片段 | 检查 setEditCellValue 收到 value 和 debounceMs:200 | 只证明调用参数；不能证明立即保存不丢字，Yue 另补当前输入→保存／Tab 与多字段用例 |
| [imeFastEdit.data.DataGrid.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/tests/imeFastEdit.data.DataGrid.test.tsx) | 组词文本同步／异步处理一次，返回变换后的行；拒绝保留同一 Input／值并报错，Escape 恢复旧值；排序换位置仍同一编辑实体且有焦点 | 实体、值与完成去重联合断言；补 draft A／B 倒序、取消重启和任务结算；合成组词不代替真机 IME |
| [editing.DataGrid.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/tests/editing.DataGrid.test.tsx) 的 browser timing 片段 | 显式绕开 act 合批的浏览器按键：受控／非受控 cell／row 首字一次，Enter 启动多行编辑不插入意外换行 | 验证实际输入时序，不只等待后检查最终字符串；不扩大 Yue 的启编入口 |
| [imeFastEdit.layout.DataGrid.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/tests/imeFastEdit.layout.DataGrid.test.tsx) | 实验隐藏 editor 与基线 bounds／scroll 尺寸比较；布局用例跳过 jsdom | 编辑结构不能撑开滚动范围或造成跳动；不复制预挂载隐藏 Input 方案 |
| [layout.DataGrid.test.tsx](../../refer/DataTable-Libs/mui-x/packages/x-data-grid/src/tests/layout.DataGrid.test.tsx) 的定向片段 | 真实布局组跳过 jsdom；无多余横滚、空态不盖原生横条、loading 时不呈现 noRows、结束后更新；2px 根边框下宽度稳定 | GPUI 实际 bounds／scene 下验根边框和 overlay 合同；不搬原生横条占位公式或固定等待时长 |

参考的测试组织值得吸收：状态规则、真实布局、浏览器事件时序、IME 合成与数据保存分别覆盖。Yue 必须落到自己的 GPUI 事件、bounds、state 和资源释放；不能因为参考已有用例就写“本项目已通过”。

## 9. 吸收清单与下一项

| 编号 | 决定 | 阶段 |
| --- | --- | --- |
| MX01 | 窄派生读取、挂载至订阅窗口补偿、一致快照和多实例隔离；复用 GPUI，不新建 store | M1，后续阶段复测 |
| MX02 | 宿主 typed 更新保留字段／删除重建顺序，全量换源使旧批次失效；不增加单行限制或 patch DSL | M3／M4 |
| MX03 | 真删除清理高度，换页／离屏不误删；长期闭包不保活历次来源，验证释放而非只看条目数 | M1／M3／M5 |
| MX04 | 保存／Tab 前处理所需待执行输入，行多字段不丢值；真实 Input 和 IME 优先 | M4 |
| MX05 | 同会话 draft revision 隔离倒序校验与取消重启，所有完成路径及等待方结算 | M4 |
| MX06 | root 边框／padding 正确参与几何，overlay 滑块不占位，溢出阈值有界收敛 | M1／M2／M5 |
| MX07 | Popup／Dialog 与嵌套表实例隔离，外层不重复处理事件、不抢浮层焦点 | M1／M3／M4 |
| MX08 | 无可见列与无数据／筛选空分开，用已有 Empty／Button／列设置恢复 | M2／M3 |

本轮不新增商业组件依赖、React／DOM 适配、通用状态／异步框架、隐藏输入预热、打印字符启编、导出或电子表格功能。所有补充属于既定 M1–M6 合同与验收，没有据参考代码给出 GPUI 性能结论。

下一项为 `refer/DataTable-Libs/react-data-grid`，继续核对其本地版本、许可、行／列窗口、编辑／焦点和回归测试；本轮尚未开展该项源码调研。

后续进展：第 6 项 [react-data-grid 调研](2026-11-10-table-react-data-grid-review.md)已完成，以上仍保留本轮的阅读范围。
