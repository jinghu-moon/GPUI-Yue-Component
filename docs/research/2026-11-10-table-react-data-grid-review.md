# YueTable 源码调研 06：react-data-grid

日期：2026-11-10。状态：本地源码、类型及定向浏览器测试代码阅读完成，结论已补入 [YueTable 设计](../45-table.md)。只调研与补充设计，未实现 Table、安装依赖、启动应用、运行参考测试或采集性能。

上一项：[MUI X Data Grid](2026-11-10-table-mui-review.md)。按用户要求一个项目一个项目推进，本轮不研究后续仓库。

## 1. 快照、许可和范围

| 项目 | 核对结果 |
| --- | --- |
| 目录 | `refer/DataTable-Libs/react-data-grid`，独立 Git 仓库，阅读时工作区干净 |
| HEAD | `6f6469d88dac7d7263862ef2d449b7b06d98e1f7` |
| 版本 | [package.json](../../refer/DataTable-Libs/react-data-grid/package.json)：`react-data-grid` **7.0.0-beta.64**；保留 beta 标识，不将其写成稳定版 7.0 |
| 许可 | [LICENSE](../../refer/DataTable-Libs/react-data-grid/LICENSE)：MIT，原作 2014 Prometheus Research、修改 2015 Comcast |
| 实现 | React、CSS Grid、JS 行列窗口；package 没有 dependencies，React／React DOM 为 peerDependencies，构建／测试有 devDependencies；“零运行依赖”不等于无框架依赖或小内存证明 |
| 仓库规则 | 已阅读 [AGENTS.md](../../refer/DataTable-Libs/react-data-grid/AGENTS.md)；视觉回归仅允许 CI，本轮也没有运行其他测试。源码修改所需构建命令不适用于本轮只读研究 |
| 测试组织 | [vite.config.ts](../../refer/DataTable-Libs/react-data-grid/vite.config.ts) 配置 Chromium／Firefox／WebKit browser 项目，Node SSR 单独组织，visual 仅 CI 纳入；本轮不据配置宣称三浏览器已通过 |

主线：列归一化／几何 → 行列取窗 → 离屏活动格 → span → 定位与焦点 → 编辑完成 → 宽度与键盘伸缩；另读 TreeDataGrid 的结构投影与映射边界。未穷尽复制粘贴、填充、导出、全部 Demo 或每份测试的所有断言。

Context7 查询 `/comcast/react-data-grid`，用于交叉理解 key、onRowsChange、受控宽度和两端冻结。返回示例仍称 `rowKeyGetter`，本地 [DataGridProps](../../refer/DataTable-Libs/react-data-grid/src/DataGrid.tsx) 与 README 主示例已经使用 **getRowKey**；本轮按该 HEAD 的真实签名解释，不将在线片段直接当本地 API。

## 2. 双轴工作集之外，保留活动格而不是整行列

证据：[DataGrid.tsx](../../refer/DataTable-Libs/react-data-grid/src/DataGrid.tsx) 的 getViewportRows／renderViewportRow、[useViewportColumns.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useViewportColumns.ts)、[viewportUtils.ts](../../refer/DataTable-Libs/react-data-grid/src/utils/viewportUtils.ts)。

普通行共享冻结列＋横向窗口的列集合。活动行在行窗口之外时额外插入该行，但用 mapViewportColumnsOutsideOfViewport 只生成它的活动列；行在窗口内、活动列在横向窗口外时，才将活动列插入有序列集合。此逻辑保留的是导航活动格，不只编辑器。回到窗口后不再重复额外插入，减少控件消失和焦点丢失的机会。

Yue 直接复用 Base [VirtualGrid::retained_cells](../../vendor/gpui-base/src/virtualization/grid.rs) 的稳定二元 key 和稀疏并集，不再实现一份 React 行池。Cells 下按真正需要的 active／editor cell 保留，不扩为全行或保留行列的乘积；Rows 路径仍如实报告整行成本。保留实例不允许突破裁剪、抢回滚动位置或永久增加所有经过的 Entity。

参考所有 start／end frozen 列都会加入工作列；这不代表冻结成本恒定。Yue 保持已有固定区预算与超宽受限策略，不照搬“全部 frozen 就全部构建”的默认。测试应检查双轴屏外额外项、回到窗口去重及下一次导航，而不只是控件仍存在。

## 3. 已知行高不等于自然测量，范围边界要核对身份

证据：[useViewportRows.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useViewportRows.ts)、[useCalculatedColumns.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useCalculatedColumns.ts)。

固定行高以算式计算位置。函数行高则预先遍历所有行，建立 top／height 数组并生成压缩重复段的 CSS grid-template-rows，之后二分找行；源码注释明确提示全量预处理成本。它是宿主已知高度，不是屏内排版反馈的自然 Measured。列几何按配置与宽度缓存生成；滚动时从非冻结起点线性查找可见列，不宜宣称两轴所有查找均 O(log N)。

Yue 继续使用 Base Fixed／Known／Measured 尺寸索引，不为 CSS 字符串或函数回调再建全量尺寸源；Cells＋Measured 仍不支持。稳定滚动和活动格保留不能重新遍历所有已知行高，非均匀高度的 PageUp／PageDown 按有效正文距离和现有索引查找，不按固定条数猜测。

参考 overscan 为上下各 4 行、横向各 1 列；只借鉴“恰好在边界及两侧”的用例，不复制常量或 1920×1080 下的元素数量。断言预期 key 集合、顺序、范围和合法空白：总数相同仍可能漏一个 key、重复另一个。包含零行／零列、summary 和两端固定区。

## 4. 宽度偏好、有效宽度和测量缓存分开

证据：[useColumnWidths.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useColumnWidths.ts)、[DataGrid.tsx](../../refer/DataTable-Libs/react-data-grid/src/DataGrid.tsx) 的 getColumnWidth／受控适配、[renderMeasuringCells.tsx](../../refer/DataTable-Libs/react-data-grid/src/utils/renderMeasuringCells.tsx)。

参考 columnWidths 按 key 区分 measured 与 resized。用户已调整的列不随容器改变重新测量；可伸展的字符串宽度列在条件满足时重新测量。未知宽度先以 minWidth 维持几何，再读取 CSS Grid 实际尺寸。隐藏 measuring cell 是尺寸探针，并不是渲染所有业务内容来找全局最大文本。

有价值的合同是保留宽度来源及未夹紧偏好：暂时降低 maxWidth 只影响有效宽度，恢复约束后可回到先前偏好；重置覆盖重新使用当前列声明，隐藏与永久删除分开。Yue 的唯一列配置仍属 accepted TableView，布局缓存不得自立一份可接受配置；只有明确接受的样本测宽才成为用户偏好，临时测量结果不直接持久化。不为 JSX／CSS 的 auto／max-content 添加无限隐式测宽。

参考尚有明确的 `test.fails`：过滤掉的 measured 条目未按期望丢弃、旧环境导入的 measured 宽度未重算，以及百分比列在伸缩中超 max 后造成余量错误。它们可用于设计失败基线，不能写成参考已经解决。Yue 用既有 Flex 求解器，并验证临时夹紧恢复、无变化不改宽度策略、过期测量失效及有效 Flex 余量。

参考对 maxWidth＜minWidth 的处理是忽略该 max；Yue 保持类型化原子拒绝。也不照搬它以是否同时提供回调来切换受控／非受控的双模式；本项目已有唯一请求／接受通道。

## 5. 键盘伸缩也要有明确完成和取消

证据：[HeaderCell.tsx](../../refer/DataTable-Libs/react-data-grid/src/HeaderCell.tsx)、[useColumnWidths.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useColumnWidths.ts)、[resizable.test.tsx](../../refer/DataTable-Libs/react-data-grid/test/browser/column/resizable.test.tsx)。

参考在表头 Ctrl＋左右键时预览宽度，阻止同次导航／滚动，keyup 或 blur 结束会话；测试区分按住时不回发受控宽度、释放后一次回发，随后释放其他键不重复完成。其逐次 onColumnResize 回调与最终宽度回调不是 Yue 的单请求合同，不能直接照搬。

Yue 保留已有“句柄聚焦＋方向键／Shift 步长”入口，补 repeat 预览、释放伸缩键或正常转移控件焦点后单次提交；Escape、窗口失活、卸载或列失效撤销，迟到 keyup／blur 去重。达到边界且无有效变化不产生冗余修改，伸缩不同时排序／导航／滚动。参考失去 pointer capture 会结束提交，Yue 指针路径仍按已有合同撤销，不因本轮调研改变。

Base [ResizeHandle](../../vendor/gpui-base/src/resizable/resize_handle.rs) 已有命中、状态和拖动组合，不能宣称它已经包办列宽业务事务；键盘会话由 Table 接线，必要的通用行为再按失败基线提取。无需引入另一组全局快捷键或 Web pointer 监听。

## 6. 定位按轴避让，焦点与滚动各司其职

证据：[DataGrid.tsx](../../refer/DataTable-Libs/react-data-grid/src/DataGrid.tsx) 的 scrollToCell／scroll-padding、[useScrollToPosition.tsx](../../refer/DataTable-Libs/react-data-grid/src/hooks/useScrollToPosition.tsx)、[domUtils.ts](../../refer/DataTable-Libs/react-data-grid/src/utils/domUtils.ts)。

参考只把非冻结列作为横向定位目标；有效 rowIdx 独立处理，因此定位固定列中的屏外行仍能纵向滚动。header／summary 不作为正文纵向目标。普通格的 scroll-padding 避开 sticky 区，sticky 自身在该轴不重复避让。真实焦点使用 preventScroll，并由显式 scrollIntoView 负责定位。

Yue 用已存在的 [VirtualizerLayout::scroll_padding](../../vendor/gpui-base/src/virtualization/mod.rs)、[alignment_offset](../../vendor/gpui-base/src/virtualization/state.rs) 与命令结算；默认 Nearest，固定轴不乱动、目标已可见不动，定位不默认改选择或抢焦点。已经从视口扣掉的固定区域不能再次作为 inset 扣除。超大目标按 Base 规则稳定结算，不要求“不可能的完整可见”导致两端往返；End／尾冻结仍须 M2 最小扩展与失败测试。

不复制用于 CSS 自动宽度定位的临时 grid 占位 div，也不静默吞掉 Yue 的缺失 key／非法宿主结果；不跨页请求数据。参考测试的 visible 有时只指元素在文档中且可显示，不能替代 GPUI 下检查目标 bounds、冻结遮挡、命中和 owner 两轴实际变化。

## 7. 稀疏列中的跨度按原索引，不按工作数组位置跳过

证据：[viewportUtils.ts](../../refer/DataTable-Libs/react-data-grid/src/utils/viewportUtils.ts) 的 mapColumnsWithColSpan、[useViewportColumns.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useViewportColumns.ts)、[activePositionUtils.ts](../../refer/DataTable-Libs/react-data-grid/src/utils/activePositionUtils.ts)。

冻结列、稀疏活动列和窗口列形成不连续但有序的集合。参考用 `column.idx + colSpan` 判断覆盖后的下一原始索引，不用工作数组中“跳过 N 项”，因此离屏活动格的跨度不误吞屏内普通格或尾冻结列。取窗还向前查找覆盖窗口的 origin；导航按当前 row kind 的跨度跳到合法 origin，多层表头处理可到达的父级行。

Yue 借鉴断言，沿用 Base span 索引，不按每次滚动扫描所有候选回调。参考发现屏外 origin 后还会将共享窗口起点向前移，可能让其他行多构建中间列；Yue 仅补必要 origin，不用该方式扩大所有行的工作集。正文／表头／summary 各自在合法范围内使用共享轨道；editor 与 origin 同一矩形。测试叠加“左侧离屏活动 span＋中间窗口”“右侧活动 span＋尾冻结列”“窗口尾部跨度＋尾冻结列”，并核对语义索引和可操作格唯一。

参考会将跨度夹紧到冻结分区／最后列；Yue 继续显式拒绝非法跨区、越界或重叠，不能悄悄截短后保存另一份结构。表头跨冻结分片若未实现一致语义也仍按原组合规则拒绝，不根据参考名称放宽能力矩阵。

## 8. 外部数据修订、保存入口和同次事件路由

证据：[useActivePosition.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useActivePosition.ts)、[EditCell.tsx](../../refer/DataTable-Libs/react-data-grid/src/EditCell.tsx)、[DataGrid.tsx](../../refer/DataTable-Libs/react-data-grid/src/DataGrid.tsx)、[useRovingTabIndex.ts](../../refer/DataTable-Libs/react-data-grid/src/hooks/useRovingTabIndex.ts)。

参考编辑位置保存 row 与 originalRow，默认当前位置的 Row 引用变化便关闭编辑；选项可以关闭该规则，但位置变成 summary 时仍必须结束。Yue 不按引用或显示索引决定是否丢草稿：使用 stable key、数据／草稿 revision，未影响编辑的修订保留会话，真实冲突交宿主处理。被删除格的状态和导航路由都须退出，不能只有 Input 消失、Table 却仍吞导航键。

参考通过 flushSync 避免外点完成与选另一格引发两次更新；还在 window capture／bubble 与 postTask／rAF 间协调默认外点保存。测试中“外部 Save 读取新值”显式给 click 增加 10ms 延时。Yue 默认失焦不保存，工具栏 Save 明确走同一编辑完成流程，读取当前草稿、经过校验／接受，不依赖 mouse-down 先提交后 click 读取的时序，不向产品暴露额外延时配置。

参考在 keydown capture 识别子 modal，记住同次 native event；即使 modal 在处理 Enter 时关闭，后续 cell handler 仍跳过。它也区分“Table 在 modal 内”和“编辑器另开 modal”。Yue 复用 Root／Dialog owner 与 GPUI 事件处理，补同次按键关闭后仍不穿透；不造公共 CellEvent 或 Web 调度框架。外部焦点已转移，旧完成不得把焦点带回屏外旧格。

roving tabIndex 让活动格中的 Checkbox 等真实控件接过焦点，格壳不再重复停靠。Yue 吸收交互模式下不双重聚焦、合法 active key 再进入恢复；renderer 显式接入控件焦点，不查后代树。参考默认 Tab 逐格导航与本设计单导航入口不同，不改变 Yue 的默认键盘合同。

## 9. TreeDataGrid 是分组包装，不等同于任意父子树

证据：[TreeDataGrid.tsx](../../refer/DataTable-Libs/react-data-grid/src/TreeDataGrid.tsx)。

该组件按 groupBy 字段构造分组树，结构构建独立于展开状态；扁平化只遍历展开分支，保存 group row 的索引辅助查父级。每条业务记录会加入各层 group 的 childRows，因此有随分组深度增加的引用成本。分组选择按全部后代数据行派生，分组列被自动提前并冻结；不是接受任意 `children` 的树表。

Yue 继续复用 KeyedTreeModel 和已有树选择／投影，不增加新的 groupBy 引擎、强制分组列冻结或改变默认父子独立选择。参考对修改位置回找原数据使用 indexOf，内部 ARIA 还保留全展开序号；本项目用既有 key 反查与当前逻辑投影语义，不以这些实现替代身份合同，也不据 role=treegrid 宣称 Windows UIA 链路齐全。

## 10. 已阅读的测试断言及其边界

下表是实际阅读的定向片段，未执行；没有将测试标题、预期失败或浏览器结果替代本项目验证。

| 参考 | 实际断言 | Yue 的验收落点 |
| --- | --- | --- |
| [virtualization.test.ts](../../refer/DataTable-Libs/react-data-grid/test/browser/virtualization.test.ts) | 244／245 等相邻位置的首末索引和数量；start／end 冻结索引集合；summary、零行列、关闭虚拟化；屏外未知宽列保留 minWidth 总长 | GPUI 边界两侧 key 集合与有界成本，不复制固定像素／数量或未知列默认宽度 |
| [keyboardNavigation.test.tsx](../../refer/DataTable-Libs/react-data-grid/test/browser/keyboardNavigation.test.tsx) 的活动格片段 | 滚离活动行后只余 1 个活动格，下一次导航恢复；两轴离屏继续导航；Checkbox 接管焦点；外部再进入及删行／列 | Cells 精确保留、回来去重、焦点／边界联合验收；不采用逐格 Tab，也不以一次 React Profiler render 断言 GPUI 一次 paint |
| [scrollToCell.test.tsx](../../refer/DataTable-Libs/react-data-grid/test/browser/scrollToCell.test.tsx) | 行／列分别定位，start／end frozen 横轴 no-op，summary／header 不纵向定位；无效位置不移动 | 按轴核对真实 owner 与未遮挡 bounds；Yue 非法目标仍返回其约定结果，不改成静默忽略 |
| [endFrozen.test.ts](../../refer/DataTable-Libs/react-data-grid/test/browser/column/endFrozen.test.ts) 的开头片段 | 固定列稳定分区，滚动前后右侧两列实际矩形贴边且相邻 | 两端固定和边框实际几何；class 断言本身不算布局通过 |
| [rowHeight.test.ts](../../refer/DataTable-Libs/react-data-grid/test/browser/rowHeight.test.ts) | 函数行高 40／60／80、重复段和首尾例外，工作行数与 Ctrl＋End 到实际底部 | Known 的范围／末端及不同密度基线，不声称自然 Measured 或全量高度预处理恒定成本 |
| [width.test.tsx](../../refer/DataTable-Libs/react-data-grid/test/browser/column/width.test.tsx) | 声明宽度变化、number→auto、min 临时扩大后恢复、Dialog 打开时不产生 observer 循环 | 当前声明／缓存版本、临时约束恢复、真实容器中尺寸收敛；不导入 CSS auto 策略 |
| [resizable.test.tsx](../../refer/DataTable-Libs/react-data-grid/test/browser/column/resizable.test.tsx) 的定向片段 | 键按住时无最终受控回调，释放／失焦只回一次；重置覆盖、用户宽度随容器变宽保留、max 临时降低后恢复；无变化适配不标 resized | 单请求、撤销／迟到事件、偏好与有效宽度、Flex 预算；本文件的 3 个宽度 `test.fails` 仅作为待防范案例 |
| [colSpan.test.ts](../../refer/DataTable-Libs/react-data-grid/test/browser/column/colSpan.test.ts) 的定向片段 | ROW／HEADER／SUMMARY 中离屏活动跨度后仍显示窗口／尾冻结列，editor 同跨度；参考非法跨度会夹紧 | 不连续工作索引不误跳、origin 单实例、编辑 bounds 同源；Yue 非法 span 明确拒绝 |
| [grouping.test.ts](../../refer/DataTable-Libs/react-data-grid/test/browser/column/grouping.test.ts) 的键盘片段 | 不同深度表头在上下／左右／Home／End 时到可达父级与叶列 | 表头共享几何和可达语义测试；不新增另一套表头逐格 Tab 模式 |
| [renderEditCell.test.tsx](../../refer/DataTable-Libs/react-data-grid/test/browser/column/renderEditCell.test.tsx) 的定向片段 | 外部修订的关闭选项、删行后 summary 替位仍能继续导航；portal 内点不算外点；外点关闭不抢外部 Input 焦点；子 modal 关闭同击键不进 cell handler，而 Table 在 modal 内仍正常处理 | 草稿／来源冲突、无幽灵编辑状态、显式保存、事件 owner／焦点；其延时 click 和 `it.fails` 的未激活直接输入不作通过证据 |

## 11. 吸收清单与下一项

| 编号 | 吸收决定 | 阶段 |
| --- | --- | --- |
| RD01 | Cells 的离屏活动／编辑格精确保留，回到窗口去重，不构建整条离屏行；Rows 成本另计 | M1／M4／M5 |
| RD02 | 行列边界两侧核对 key 集合；非均匀 Known 高度和空正文导航，不重写尺寸索引 | M1／M5 |
| RD03 | 宽度声明、接受偏好、有效值与测量缓存明确；临时夹紧／隐藏恢复、显式重置与 Flex 余量 | M2 |
| RD04 | 键盘 repeat 预览、释放／正常 blur 单次完成，取消／失活及迟到事件结算，不改变快捷键入口 | M2 |
| RD05 | 定位逐轴避让固定区，默认不抢焦点；已可见／超大目标稳定结算 | M1／M2／M5 |
| RD06 | 不连续工作列的 span 用原索引；header／summary／editor 与 origin 一致，非法跨度仍拒绝 | M2／M4／M5 |
| RD07 | 外部修订按 key／revision 协调草稿，删除后退出真实编辑状态；工具栏保存不依赖外点时序 | M4 |
| RD08 | 同次关闭子 modal 的输入不穿透，Table 在 Dialog 内仍可操作；格内控件无重复停靠 | M1／M3／M4 |

保持已有 M1–M6 范围。没有新增 React／CSS 层、groupBy／填充／复制粘贴引擎、打印字符启编、通用事件包装器或多平台支持；只将参考优点和未解决用例转为 GPUI 下需要实现、验证的合同。

下一项为 `refer/DataTable-Libs/material-react-table`。先核对其 TanStack Table／MUI 依赖版本，再区分自身提供的组合能力与上游行为，重点看列配置、编辑／工具区与状态接线；本轮尚未开展该项调研。

后续进展：第 7 项已完成，见 [Material React Table 3.2.1 专项记录](2026-11-10-table-material-react-table-review.md)。上段“尚未开展”保留为本轮结束时的历史范围，不代表当前总进度。
