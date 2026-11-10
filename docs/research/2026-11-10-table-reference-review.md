# Table 三库参考网页调研

日期：2026-11-10。范围：用户指定的三份本地保存网页，以及本仓库当前 Base／Yue 源码。产出为 [YueTable 设计](../45-table.md)，本轮不实现组件、不改依赖或 token。

## 1. 文件与阅读方式

| 文件 | 页面识别 | 本次重点 |
| --- | --- | --- |
| `refer/table/表格 Table - Ant Design Vue (2026_11_10 18：34：39).html` | 浏览器标题“表格 Table - Ant Design Vue” | 30 个实际 demo 区块、API／FAQ、多级表头的 colSpan／rowSpan 与横向滚动、固定宽度说明 |
| `refer/table/表格 Table ｜ ArcoDesign (2026_11_10 18：34：36).html` | 浏览器标题“表格 Table \| ArcoDesign”，React 页签 | 选择／展开操作列、固定列、虚拟化限制、排序优先级、API、外部交互依赖 |
| `refer/table/Table 表格 ｜ TDesign (2026_11_10 18：33：14).html` | 浏览器标题“Table 表格 \| TDesign”，Vue Next，页面显示 1.20.11 | 表格分层、首尾冻结、列配置、编辑、树表、本地／远程处理、懒加载／虚拟化 |

通过 Playwright 浏览器加载网页，查看可访问快照、渲染后的 heading／说明／API 文本，并截图检查固定列、多级表头和冻结区的实际布局。浏览器禁止直接访问 file 协议，故临时仅将 `refer/table` 目录以 `127.0.0.1` 本地 HTTP 提供；未修改原始网页，也未上传至外部服务。

浏览器抽取与截图保留于本地 `target/table-*-browser.*`、`target/table-ant-groups.png`、`target/table-arco-fixed.png`、`target/table-tdesign-fixed.png`，作为本次阅读辅助，不作为运行时资源或永久测试基线。

保存页可能不含原站完整脚本；本次没有声称实际执行了三库的全部排序、拖拽、数据请求和虚拟化交互。TDesign 保存的是示例页，未将导航栏 API 标签当成已读取另一页的完整 API；参数结论只取自保存页说明。Ant／Arco 则读取了文件中实际存在的 API 表。

原文件 SHA-256，便于确认调研版本：

```text
Ant Design Vue
249CE8FC207F3EBA5AE4D74B0A560140F53E0F27BD934F7BEE65D9C4182B23A1
Arco Design
2F68675EC575B593B0BA3D357CA174AFF6B85DC88124E628CECA8E999717B5F7
TDesign
4E33DF0973DF20ABF017ED9A99423780511EC4C9C4ECEDF488581C54C2131143
```

## 2. Ant Design Vue

实际 demo 包含基本、远程加载、边框、省略、合并、单元格／行编辑、树形、展开、固定头列、多级表头、多列排序、嵌套、筛选及自定义菜单、受控排序筛选、搜索筛选项、树型筛选、选择和操作、自定义选择项、页面 sticky、伸缩、密度、斑马纹、summary、响应式、template。

具体依据与设计影响：

- `components-table-demo-fixed-header`／`fixed-columns`／`fixed-columns-header` 说明指定 width、防止列头错位／列重复、预留弹性列以及长连续文本破坏布局。Yue 因而必须共享同一列轨道，而非分别测 header 和 body。
- `components-table-demo-grouping-columns` 在保存 DOM 中有四层表头，Name／Gender 的 rowSpan 为 4，Other 的 colSpan 为 4，Address 再包含 Street／Block。多级表头是矩形布局，不是额外标题行简单平铺。
- `components-table-demo-colspan-rowspan` 用 0 表示不渲染覆盖 cell。Yue 采用明确 origin／覆盖计划，不复用 0 的多重语义。
- `components-table-demo-ajax` 将排序筛选交服务端处理，特别提醒 preserveSelectedRowKeys。Yue 需区分暂时离页和真删除。
- `components-table-demo-summary` 要 summary cell 同步列固定状态；聚合必须同轨道而非独立 footer 文本。
- ellipsis 说明／API 对表头排序筛选存在限制。Yue 用标题文字槽和独立操作按钮解决空间分配，不继承这项外观限制。
- template 示例明确“不推荐使用，会有一定的性能损耗”，说明只是描述 columns 的语法糖。Rust 无须额外模板系统。
- “大数据渲染”“行拖拽排序”“列拖拽排序”是 Surely Vue 外链，不能记成此页面 Table 已内置的实测能力。
- FAQ 说明过滤常回第一页、远程可受控保留；固定列层次可能穿透遮罩。Yue 将 query 原子更新与窗口 Popup 层次列为验收点。

API 文本涉及 columns、rowKey、rowSelection、scroll、sticky、summary、filterDropdown、sortOrder、响应式及 resize 事件。其 Web 样式／DOM 透传机制不转成 GPUI 公共接口。

## 3. Arco Design

保存页为 React 文档，不能误认作 Arco Vue API。主要示例覆盖基础、选择、展开参数、嵌套、树、属性、排序筛选、定制分页、内容、固定列、样式、分组、合并、编辑、伸缩、sticky、操作列、拖拽、虚拟化、空状态和 summary。

特别记录：

- API `virtualized` 明写会自动关闭树形数据支持。因此“各自有示例”不代表“可同时启用”。Yue 采用显式能力矩阵和投影合同。
- `Column.sorter.multiple` 数值越大优先级越高，受控多排序要同时保持各列状态一致。Yue 用有序 sort 列表表达，不在多列各存独立状态。
- 自定义分页通过 total／pageSize／onChange 与 renderPagination，沿用独立 Pagination 文档。Yue 直接复用已经实现的 YuePagination。
- 固定列说明要求 fixed＋width＋scroll.x，保留一列不设宽度。Yue 使用显式 Flex 约束，不将“恰好留一列无宽”变成隐藏前置条件。
- components 的 operations 能定义 selection／expand 操作列名称、宽度和顺序。Yue 设计具名列种类，避免插槽暗中增删轨道。
- 可伸缩列、表头吸顶和拖拽分别借助 react-resizable 3.0.0、react-sticky 6.0.3、react-sortable-hoc 2.0.0。它们是借鉴对象，不是 GPUI 可以直接调用的机制。
- 虚拟列表参数包含 height、itemHeight、threshold、isStaticItemHeight 等；不能据此推定支持双轴自然高度。

浏览器固定列截图可见左侧选择／展开区域、中央滚动区与右侧 Salary，以及滚动边界阴影。Yue 只在内容实际被遮挡时保留必要分界，不照搬浏览器厚轨道。

## 4. TDesign

页面将表格分为 BaseTable、PrimaryTable／Table、EnhancedTable，分别说明基础布局、高级交互、树结构。Yue 借鉴内部职责分离，但不复制三个对外根类型。

重点依据：

- 基础表格默认 fixed 宽度；多级表头只需给叶列宽度。auto＋固定列另要求 tableContentWidth。Yue 使用一份列轨道，不继承 CSS table-layout 分支。
- `fixedRows` 示例表达首尾冻结数量 `[2, 2]`；固定列有 left／right。这超过本项目当前 Grid 的前缀冻结，必须登记 Base 缺口。
- 页面分别有 headerAffixedTop、footAffixedBottom、horizontalScrollAffixedBottom、paginationAffixedBottom。表内固定和页面吸顶是不同宿主合同。
- 列配置区分 displayColumns 的确认结果和 onColumnChange 的草稿变化，也能由外部触发打开面板。Yue 需完整的确认／取消路径，不只是即时 Checkbox。
- 本地排序说明只处理传入 data；远程分页不内部切片，本地分页自动处理 data。Yue 显式 Local／Remote，避免重复处理。
- 跨页选择默认保留，可以关闭；rowKey 错误会导致点击一行选中全部。Yue 要求稳定唯一业务 key 并验证。
- row highlight 有上下键、Escape、Space、Shift 路径。Yue 学习完整键盘工作流，但不照搬 Escape 全局清空选择，先处理 IME／最上层会话。
- 虚拟化 `scroll.type='virtual'`，说明 threshold 默认 100；懒加载 `scroll.type='lazy'` 使用 bufferSize。二者不是同一种成本合同，本项目不无依据照搬阈值。
- virtual 示例声明可组合固定头尾、吸顶和多级表头；保存页无法证明所有组合的运行时正确性，Yue 分组合验收。
- cell／row 两类编辑，列编辑能用 Input、Select、DatePicker 并包含校验和结束事件。Yue 复用现有控件及 Form，不新增编辑器。
- 树功能使用 EnhancedTable，包含加载节点、结构变更、父子独立／级联选择和滚动定位。Yue 复用已存在的 Base keyed tree 结构投影。

## 5. 本项目源码能力核对

| 源码 | 已有事实 | 对设计的约束 |
| --- | --- | --- |
| `vendor/gpui-base/src/table.rs` | 无样式 Table／Row／Head／Cell，索引输入有一基说明 | 语义可以复用，不宣称现成完整表格引擎 |
| `vendor/gpui-base/src/virtualization/mod.rs` | Source key／反查、Fixed／Known／Measured、Attached | 业务数据和虚拟化元数据分开，复用已有成本合同 |
| `virtualization/grid.rs` | 同宿主双轴、前缀冻结、精确 cell 保留、span、双轴 update_sizes 先验证后应用 | 尾端冻结尚缺；双轴动态自然测量不在当前合同内；使用已有事务而非重复实现 |
| `virtualization/grid_track.rs` | 有限 min／max／weight 的 O(C log C) 约束求解 | 约束 auto 不等于 intrinsic 全量测量 |
| `virtualization/grid_span.rs` | 显式跨度、重叠／冻结边界校验、origin 查询 | 合并不能静默跨冻结、跨页或依赖可见 origin |
| `vendor/gpui-base/src/tree/keyed.rs` | key、结构索引、展开、过滤、加载票据、选择规则 | 树表共享模型，业务 Row 保留在 delegate |
| `crates/yue-component/src/scrollbar/mod.rs` | 单 owner overlay，可调整 viewport_bounds | 固定／页面横条只换呈现位置，不创建第二滚动范围 |
| `crates/yue-component/src/pagination` | 已知 total、整数边界、受控请求、真实输入与 Select | 复用，不伪造未知 total 或在 Table 内重做分页 |
| `crates/yue-component/src/tooltip/overflow.rs` | 一／N 行真实溢出、条件焦点、完整载荷 | 与 active cell 共用焦点，避免给每段文本加 Tab stop |
| `crates/yue-component/src/card`／`descriptions` | 自然高度与等高卡片回归；透明格子、独立分隔和头像背景回归 | Table 同样需要真实绘制与包含关系测试 |

现有 Windows UIA provider 缺口沿用[既有调查](2026-11-05-virtual-grid-uia-provider-investigation.md)，不能用语义元数据通过推导完整 Narrator 表格导航。本文不以设计工作扩大 Windows COM 维护范围。

## 6. 本轮交付边界

本轮交付设计、参考记录和 README 索引；没有实现 YueTable、没有增加 Cargo 依赖、没有生成 Table token，也没有重复运行与文档无关的构建性能测试。实施按设计 M1–M6 逐项提交、复测并记录实际证据。

后续按用户要求增加 `refer/DataTable-Libs` 的逐项源码调研，第一项见 [Tabulator 6.7.1 源码与测试记录](2026-11-10-table-tabulator-review.md)。这与本记录的三库浏览器阅读分开留证；其余源码项目的完成状态见主设计第 2.2 节。
