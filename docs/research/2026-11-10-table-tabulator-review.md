# YueTable 源码调研 01：Tabulator

日期：2026-11-10。状态：本地源码和测试阅读完成，设计吸收完成；未实现 YueTable，未运行 Tabulator 测试或性能采样。按用户要求逐个项目推进，本记录只评价 Tabulator，不代表其余九个仓库已经审阅。

关联：[YueTable 设计](../45-table.md)；此前的[三库网页调研](2026-11-10-table-reference-review.md)。

## 1. 来源与证据边界

| 项目 | 本次核对结果 |
| --- | --- |
| 本地目录 | `refer/DataTable-Libs/tabulator`，独立 Git 仓库，阅读时工作区干净 |
| HEAD | `b3adf5c07e640c9b303c4739020a1bbeb83aff77` |
| package.json | `tabulator-tables`，版本 **6.7.1** |
| 许可 | MIT；[LICENSE](../../refer/DataTable-Libs/tabulator/LICENSE)，版权行 2015–2026 Oli Folkerd |
| 主要入口 | [Tabulator](../../refer/DataTable-Libs/tabulator/src/js/core/Tabulator.js)、[核心模块](../../refer/DataTable-Libs/tabulator/src/js/core/modules/core.js)、[可选模块](../../refer/DataTable-Libs/tabulator/src/js/core/modules/optional.js) |
| 阅读方法 | 从 RowManager、两个虚拟 renderer、数据加载器进入，追踪相关列／行／编辑／响应式模块，再阅读对应单测和真实浏览器回归测试代码 |

用户提供的介绍记为 6.5.0；本记录以本地 6.7.1 快照为准，不将它称为经过联网核实的“当前最新发布版”。星数、接管维护时间、百万数据性能及所有包装器支持不在本次源码核对结论内。没有修改介绍文件或参考仓库。

通过 Context7 补充检索[官方站点](https://www.tabulator.info)的 responsive／virtual DOM 说明；返回的是功能介绍和组合示例，未给出精确的 redraw／横向兼容合同。因此下面的具体限制以本地实现为证据，不用检索摘要替代源码。

## 2. 数据投影：分阶段恢复比“缓存整张表”更有价值

证据：[RowManager.js](../../refer/DataTable-Libs/tabulator/src/js/core/RowManager.js) 的 `registerDataPipelineHandler`、`registerDisplayPipelineHandler`、`refreshActiveData`、`refreshPipelines`、`blockRedraw`、`restoreRedraw`。

实现将原始 rows、activeRows 与 displayRows 区分：data pipeline 的处理结果逐级保留，display pipeline 再生成最终呈现序列。`refreshActiveData` 可以找到某个 handler 的位置，从指定阶段开始恢复；redraw 被阻止时记住最早需要恢复的阶段，而不是每次输入都立即整表重算。

实际注册顺序可在各模块核对：Filter 的 data priority 为 10，Sort 为 20；display 阶段 FrozenRows 为 10、GroupRows 为 20、DataTree 为 30、Page 重置／切片为 40／50。这说明其默认具体顺序并不是本设计的“根分页再展开”。

吸收：

- Yue 固定自己的处理语义，按数据／query／页／展开／几何分别推进 revision，从最早失效步骤重算。列伸缩、hover、选择与稳定滚动不重跑过滤排序。
- 同次数据／列配置提交合并失效与 summary 更新，向后调度一次；不逐格重算。
- Rust 复用 key／index 投影和不可变输入，不复制每个业务 Row。参考实现各阶段 `.slice(0)` 并保留中间数组，会有随阶段数增长的 O(KN) 引用存储与复制成本，不能简单称其“零成本缓存”。

不吸收：面向任意 JS 模块的数字优先级注册平台、公开成对 redraw 锁。Yue 沿用 App 更新事务和已有批次机制，不让使用者承担漏解锁风险，也不增加第二份可写业务状态。

设计落点：第 5.4 节；M1 建立失效接线，M3／M4 接入查询／展开，并用阶段调用计数验证。

## 3. 虚拟渲染：读取／计算／反馈分批，保留本项目已有预算

证据：[VirtualDomVertical.js](../../refer/DataTable-Libs/tabulator/src/js/core/rendering/renderers/VirtualDomVertical.js) 的 `_virtualRenderFill`、`_quickNormalizeRowHeight`、`rerenderRows`；[Row.js](../../refer/DataTable-Libs/tabulator/src/js/core/row/Row.js) 的 `calcHeight`、`calcMaxHeight`、`setCellHeight`。

纵向 renderer 维护窗口与上下 padding，常规滚动按方向增删窗口边缘，跨度很大时重新填充。重绘保留一个靠近当前视口的行与相对偏移，数据缩短后处理原窗口超出新末尾的情况。

在填充时先批量创建／附着行，再分开调用 rendered、计算需要更新的行高、统一写回 cell 高度，源码明确保留分开的循环以减少 DOM 读写交错。`Row.calcMaxHeight` 遍历当前 row.cells 取最大值，避免同排短格各自决定行高。缓存高度未初始化或为零时重新计算。

可借鉴的是阶段分离、无效尺寸处理、批次反馈与锚点恢复。DOM 的 DocumentFragment、offsetHeight、padding 和同步回流修补不是 GPUI 的实现方案。本项目已有 Base measured sizing、测量 revision、measurement budget、方向 overscan 和锚点，Table 应接入它们。

不得据 `calcMaxHeight` 推导它已经解决“横向离屏列自然行高”。Yue 的 Rows 测全部显示列，Cells 仍限定 Fixed／Known；不因此开放 Cells＋Measured。参考的最少窗口行数、单次链长或像素 buffer 常量也不直接搬入 Base。

设计落点：第 9、21 节；M4 验收合批测量、迟到测量拒绝、预算推进和尺寸收敛。

## 4. 横向虚拟化：组合限制必须进入合同

证据：[VirtualDomHorizontal.js](../../refer/DataTable-Libs/tabulator/src/js/core/rendering/renderers/VirtualDomHorizontal.js) 的 `compatibilityCheck`、`calcWindowBuffer`、`rerenderColumns`、`dataChange`；[默认选项](../../refer/DataTable-Libs/tabulator/src/js/core/defaults/options.js)。

默认 `renderVertical` 是 virtual，`renderHorizontal` 是 basic；具备纵向虚拟化不等于默认已启用双轴。横向 renderer 单独建立列左右位置与可见窗口，非冻结列参与中央窗口计算，固定列另处理。

`compatibilityCheck` 对 fitDataTable、responsiveLayout、RTL 发出不兼容警告，属于源码声明的边界，不能把“各自存在”当成支持任意组合。`dataChange` 在 fitData 模式可能取一条样本行生成 cell 再测列宽；这是样本策略，不是全数据最大宽度证明。`calcWindowBuffer` 按视口与最大列宽调整 buffer，也有扩大工作集的成本。

吸收：显式区分 Rows／Cells；声明能力矩阵；样本测宽必须说明样本范围、版本和成本；表头、正文、固定区、summary 共用一次轨道结果。Yue 使用类型化错误或不开放入口，不将 warning 后继续运行当作正确性保障。

设计落点：第 7.1、8、17、21 节；M2／M5。

## 5. 固定、列宽与响应式：共享几何，分清用户意图

证据：[FrozenColumns.js](../../refer/DataTable-Libs/tabulator/src/js/modules/FrozenColumns/FrozenColumns.js)、[fitColumns.js](../../refer/DataTable-Libs/tabulator/src/js/modules/Layout/defaults/modes/fitColumns.js)、[ResponsiveLayout.js](../../refer/DataTable-Libs/tabulator/src/js/modules/ResponsiveLayout/ResponsiveLayout.js)。

固定模块集中计算左右列累计偏移，并将同一列配置应用到 header、数据行和 calc rows；列宽、隐藏、增删与重排都关联几何失效。分组固定由父分组决定，单独冻结分组内子列会警告。这与 Yue 拟定的叶列显式 pin／跨区表头合同不同，不应照搬其 inferred left/right 规则。

fitColumns 处理 min／max、宽度增长／收缩、滚动条占用和取整余量。Yue 已有 `VirtualGridTrack::resolve`，继续使用既有约束求解；只吸收所有区域共用结果、余量仅分配一次和尺寸边界的测试思路，不重写另一个列宽分配器。YueScrollbar 为 overlay，不能照搬浏览器 offsetWidth−clientWidth 的滚动条扣宽。

响应式模块保留列原有 visible 状态和 responsive 优先级，按优先级及列顺序决定临时隐藏；`updateColumnVisibility` 特意区分是否由响应式触发。collapse 模式将被隐藏字段生成详情，并对行高重新计算。

吸收：主动隐藏、权限隐藏、响应式暂时隐藏分开；扩宽仅恢复后者；列显示集合的求解避免布局振荡。需要窄面板详情时复用 detail＋YueDescriptions，按许可字段读取值，不复制编辑器与操作控件。

不照搬 `generateCollapsedContent` 遍历全部 display rows 的内容重建。Yue 只构建当前必要工作集，Cells＋full-width detail 仍不支持；响应式只是几何投影，不能篡改宿主保存的列配置。

设计落点：第 7、8、11、17 节；M2，详情示例在 M4。

## 6. 值、格式与编辑：分离职责，不建立万能注册表

证据：[Accessor.js](../../refer/DataTable-Libs/tabulator/src/js/modules/Accessor/Accessor.js)、[Mutator.js](../../refer/DataTable-Libs/tabulator/src/js/modules/Mutator/Mutator.js)、[Edit.js](../../refer/DataTable-Libs/tabulator/src/js/modules/Edit/Edit.js)。

Accessor 区分 data／download／clipboard／print／htmlOutput 等读取用途；Mutator 区分 data／edit／clipboard／import 写入用途。它们表明“存储值、读取载荷、formatter 和编辑输入”不必相同。Accessor 还深拷贝返回对象以隔离内部数据；这是 JS 对象边界，本项目用宿主持有的 Rust 类型和窄读取合同表达，不逐次深拷贝整条记录。

Yue 原始数值 1234 与显示“¥1,234.00”、比例 0.8 与显示“80%”分开。排序读取真实类型；编辑器先解析／验证，再由宿主保存；完整提示／复制文本显式提供，不从复杂渲染树反向取文本。仅在列有需求时增加窄的 typed 读取／解析能力，不复制 accessor／mutator 的字符串插件系统。

Edit 有当前 cell 与 editFinished 校验，防止重复完成；校验失败保留编辑，已经在编辑的同格不会重开。只有确认可以编辑后才消费对应 pointer 事件，避免不可编辑格吞掉原有 click。

该版本为了避免 resize／重绘拆掉 DOM editor，在创建编辑器后 `blockRedraw`，在 clearEditor 中 restore。Yue 学习其需要保护的会话和焦点，继续用真实 Entity、精确保留与会话 revision：编辑时仍要允许正常滚动、布局和主题更新，不能整段编辑期间冻结 Table。

设计落点：第 9、14 节；M1 renderer 的值边界，M4 编辑与事件交接。

## 7. 异步加载：不仅拦截过期成功结果

证据：[DataLoader.js](../../refer/DataTable-Libs/tabulator/src/js/core/tools/DataLoader.js) 的 `requestOrder`、`load`、`blockActiveLoad`；[Page.js](../../refer/DataTable-Libs/tabulator/src/js/modules/Page/Page.js) 的 progressive 初始化、scrollVertical 和响应解析。

DataLoader 每次 load 递增 requestOrder；成功结果在写入 rows 前检查它是否仍是最新请求，并检查 table.destroyed。Page 将 local／remote 和 progressive 模式区分，pagination 与 progressive 同时设置会报错；滚动加载按剩余距离和 loading 门禁决定是否继续，不按当前 cell renderer 的调用次数触发。

值得特别注意的阅读结果：DataLoader 的 `data-loaded` 转换链在 requestNo 检查之前执行，而 catch 和 finally 中也没有对应的 requestNo 条件。这里只描述控制流，没有实际复现并认定某个上游缺陷；它提醒 Yue 不能只检查最终 setData，就宣称竞态完整隔离。

Yue 宿主适配器须在调用任何可能更新分页／总数／summary 的响应转换前校验实例、结果域、query 与请求代次；成功、失败、finally、定时重试和卸载都遵守相同规则。旧请求结束不能清除新请求的 busy，也不能发布旧错误。

渐进加载复用同一 owner 的实际可见距离；合并重叠范围、相同游标去重和并发上限由宿主适配器负责，失败／末尾停止。一次视图不隐含混合页切片和尾部追加；未知总数仍不能伪造 YuePagination.total。业务数据及缓存可能继续增长，虚拟工作集有界并不代表总内存有界。

设计落点：第 15、17、22 节；M3 竞态与分页，M4 尾部／节点，M5 范围加载。

## 8. 汇总与持久化：保留边界，不捆绑服务

证据：[ColumnCalcs.js](../../refer/DataTable-Libs/tabulator/src/js/modules/ColumnCalcs/ColumnCalcs.js)、[Persistence.js](../../refer/DataTable-Libs/tabulator/src/js/modules/Persistence/Persistence.js)。

ColumnCalcs 监听值／行／数据变化，redraw block 内先记录需要重算、恢复后统一处理；计算结果和 formatter 分开。Yue 采用明确聚合范围、按输入版本重算、批次不逐格重算，并与固定列共享轨道；不把可见工作集当全量合计。

Persistence 可配置读取／写入函数，并保存列／query／页等设置；浏览器默认存储选择不是桌面组件职责。Yue 只交付按 key 的配置快照与校验／恢复规则，宿主控制持久化和版本迁移。不增加本地文件写入、账户同步或通用序列化服务。

## 9. 测试逻辑：采用真实失败场景与有效样本

以下均是已阅读的测试代码，未在本轮运行；存在回归测试不等于本轮证明参考通过，更不等于证明 Yue 通过。

| 参考测试 | 断言的具体问题 | Yue 转化与阶段 |
| --- | --- | --- |
| [VirtualDomVertical.spec.js](../../refer/DataTable-Libs/tabulator/test/unit/core/rendering/VirtualDomVertical.spec.js) | maxHeight 填充不嵌套启动 render pass；限制小于 header 时正文为 0，不意外构建全部行 | M1／M4：真实 GPUI 布局深度、收敛、零高预算；不是只断言“不 panic” |
| 同文件的无锚点重绘 | 空数据变有数据从顶部恢复；原窗口超出缩短后的数据时回到合法末尾 | M1／M3：按 stable key 和产品查询策略修复，保留横向位置；不照抄数组 index 锚点 |
| [scroll-jump.spec.ts](../../refer/DataTable-Libs/tabulator/test/e2e/scroll-jump.spec.ts) | 快速下滚、横滚、反向上滚；追踪同一条记录的屏幕位移与实际 scroll 位移；平缓滚动作为对照 | M4／M5：用 key、owner offset 与 scene bounds 量残差，单独统计固定／树／多行场景 |
| [editor-percentage-height.spec.js](../../refer/DataTable-Libs/tabulator/test/e2e/editor-percentage-height.spec.js) | 百分比高度下 textarea 引发尺寸反馈，编辑器不能刚打开就被重绘拆掉 | M4：在有限／max-height／Resizable 宿主中实际输入、resize 后检查同 Entity／焦点／选区，而非检查曾经发过 EditRequested |
| [edit.spec.js](../../refer/DataTable-Libs/tabulator/test/e2e/edit.spec.js) 与 [EditCellActivation.spec.js](../../refer/DataTable-Libs/tabulator/test/unit/modules/EditCellActivation.spec.js) | 连续 Tab 每个格都激活、旧格关闭、跨行继续、验证失败阻止交接、同格 focus 不重开 | M4：输入／Select／Textarea 的真实导航与 IME，拒绝保存时留在当前格且不发两次请求 |
| [ResizeColumns.spec.js](../../refer/DataTable-Libs/tabulator/test/unit/modules/ResizeColumns.spec.js) | 先给行设高度，再改变 cell 值触发重绘，新句柄仍保留应有高度 | M2／M4：以当前 header／轨道 bounds 命中；编辑／密度／分组变高后仍可拖动，避免无限创建逐行句柄 |
| [ColumnCalcsIntegration.spec.js](../../refer/DataTable-Libs/tabulator/test/unit/modules/ColumnCalcsIntegration.spec.js) | 切换分组时 calc 行对应的结构分隔不能不断累积 | M2／M3：反复显示隐藏 summary、冻结与分组后，scene 中边框和装饰数量不累积 |
| [ResponsiveLayout.spec.js](../../refer/DataTable-Libs/tabulator/test/unit/modules/ResponsiveLayout.spec.js) | collapse／hide 状态与列集合更新 | M2：补真实宽窄往返和主动隐藏保持；该参考部分断言只检查方法存在或 mock 状态，不足以证明视觉正确 |

滚动测试不能直接搬阈值：参考快速路径容许 20px、对照路径 5px；Yue 应以 GPUI 舍入基线定义容差。固定布局、尺寸收敛后的 `Δ屏幕y + Δ实际scroll` 与无输入的测量锚点保持分别度量，避免把合法补偿当跳动。没有足够有效样本、追踪 key 丢失或没有实际发生滚动必须单列结果，不能算零误差。

浏览器测试中的 DOM 类名、500ms／20ms 等等待和 jsdom mock 几何只是原测试实现，不变成 GPUI 验收合同；Yue 应等待可验证的布局／状态完成，检查真实 bounds、scene、Entity 与事件次数。

## 10. 吸收决策与实施检查表

| 编号 | 决定 | 主设计落点 | 实施验证 |
| --- | --- | --- | --- |
| T01 | 分阶段失效；从最早受影响阶段重算 | 5.4、21 | 列宽／选择／滚动时查询计数不变；改页复用过滤排序 |
| T02 | 合并 accepted view 更新和聚合，不提供公共 redraw 锁 | 5.4、21 | 多格更新不逐格全表重算；拒绝／卸载不遗留 pending |
| T03 | 行高合批、版本隔离、反馈防重入 | 9、22 | 零高不污染缓存；布局收敛且最大重入深度受控 |
| T04 | 响应式只恢复临时隐藏列 | 7.2、22 | 收窄／扩宽不泄露权限列、不恢复主动隐藏列；无振荡 |
| T05 | 样本测宽显式标注与版本化 | 7.1 | 有界样本、统一轨道、无离屏全量误宣传 |
| T06 | resize 命中与固定／summary 同几何 | 7.3、8、22 | 编辑／分组／密度变化后命中有效、无重复边框与控件 |
| T07 | typed 值／显示／编辑／完整载荷分离 | 9、14 | 货币／比例／0／false 正确排序、验证和保存 |
| T08 | 编辑交接幂等，尺寸变化保留真实 Entity | 14、22 | 连续 Tab、失败留焦点、重复 focus、输入事件互不串扰 |
| T09 | 所有异步完成路径代次隔离 | 15、22 | A/B 成功、失败、finally、转换、延时与卸载竞态 |
| T10 | 渐进加载去重、边界与背压 | 15 | 重叠范围、相同游标、失败停止、末尾不重复、查询切换 |
| T11 | 位移残差＋锚点稳定两类度量 | 21、22 | 快滚／反向／横滚、有效样本下限、几何与数据变更分组 |
| T12 | 保留功能与平台范围 | 2.2、17 | 无新增 Web 依赖、Canvas 分支、动态插件、导入导出或电子表格范围扩张 |

本轮没有新增组件 API 功能项或扩大 M1–M6 的产品范围；typed 值边界、失效与请求合同均是在现有设计职责内补全。新增验收应落到对应阶段，尚无实现证据，不能先标为通过。

## 11. 下一项

接下来按顺序调研 `refer/DataTable-Libs/table`（TanStack Table）。重点核对其本地版本、受控状态、行模型依赖／缓存、列身份和 pin／size 模型，先验证源码，再判断哪些能改进现有 Yue 设计。此处仅登记顺序，尚未评价其实现。

后续进展：第二项已完成，见 [TanStack Table 9.3.1 源码与测试调研](2026-11-10-table-tanstack-review.md)。上文保留第一轮形成结论时的范围，进度以主设计第 2.2 节为准。
