# YueTable M1–M6 交付核对

本文核对可调用实现、宿主组合、自动化证据与明确边界。阶段提交和性能证据见[实施进度](2026-11-11-table-delivery.md)；实际使用见[M6指南](../45-table-m6.md)。原设计中的类型名称和示意签名以[实际API附录](../45-table-api-examples.md)为准。

## 八类功能

| 用户功能组 | 交付入口 | 组合及边界 |
| --- | --- | --- |
| 基础展示与样式 | TableSize、Borders、striped、hoverable、透明cell、surface背景、text overflow、custom renderer、empty slot | 紧凑／响应式／自定义template由已有主题、列和slot组合；不增加重复模板引擎 |
| 布局与固定 | 共享列轨道、约束／Flex／responsive、resize、首尾冻结、Start／End pin、scroll_host | 宿主affix使用同一ScrollHandle；当前拒绝Static／Measured／RTL宿主组合 |
| 数据展示与结构 | typed异构列、自定义表头／footer、分组、TableSpanPlan、summary | 正文plan有完整原子校验；Static自然rowspan，Rows仅colspan，Cells需Fixed／Known；分组表头自动跨度，总结全宽内容通过footer slot |
| 展开与树 | TableOutline、TableOutlineSource、根分页、展开详情、节点加载、嵌套表 | 普通选择与父子级联分开；嵌套内容复用Descriptions和独立Table；不把平面重排用于树 |
| 交互与编辑 | 单／多选、跨页业务key、批量操作、TableEditSession、ColumnDraft | Input／Select／DatePicker／TimePicker真实草稿；业务记录详情编辑复用Dialog；权限与显示偏好分开 |
| 排序筛选拖动 | typed query、稳定多排序、普通／树／自定义筛选、行／手柄／列重排、键盘路径 | 排序下行重排拒绝；过滤／分页要求显式业务映射；列移动限定同组／pin且不能穿过受保护列 |
| 加载与性能 | Local／Remote适配、Pagination、加载状态、RangeRows、range／tail、Rows／Cells虚拟化 | 网络和取消任务由宿主持有；尾游标、已知范围、分页三选一；完整source索引和Known几何仍有O(N)接受成本 |
| API | yue-component公开导出、TableCapabilities、完整builder.validate、三个独立消费者 | 未使用能力默认关闭；错误Result保留旧accepted状态；非法builder明确显示配置错误，不隐式关闭虚拟化 |

## 测试合同对应

| 合同 | 实现与直接断言所在 |
| --- | --- |
| TC01–TC04 身份／接受／所有权／查询流水线 | `tests/table.rs`及`table/query.rs`：失败保留Rc身份、请求重基、来源释放、typed完整顺序oracle、滚动不重复执行查询 |
| TC05 列声明与偏好 | `table/layout.rs`、`table/customization.rs`：窄容器恢复、原始width、语义版本迁移、失权、protected顺序、schema、单配置请求和64项上限；Showcase配置取消／确认 |
| TC06 视觉和包含 | `table.rs`、`table/layout.rs`、`table/cells.rs`：透明格、header／body／footer轨道、自然换行及同行短格居中、固定唯一命中、span邻格伸展 |
| TC07 工作集与坐标 | `table.rs`、`table/cells.rs`：Rows／Cells计数、大跳／双轴定位、宿主完全离屏定位、实际clamp；opt-in W2完整200组／800定位验收 |
| TC08 动态尺寸 | `table/sizing.rs`、Static自然span测试：Known按key恢复、Measured宽度／rem改变、阅读锚点、有界收敛、自然跨行内容撑开 |
| TC09 合并与固定 | `table/cells.rs`、`table/layout.rs`：屏外非法plan拒绝、origin唯一、Static／Rows／Cells合法路径、跨pin唯一语义及完整逻辑跨度、首尾冻结、两表affix |
| TC10 导航与选择 | `table.rs`、`table/query.rs`、`table/outline.rs`及Showcase结构：真实Checkbox／Radio、按键、保留控件、删除修复、页外key、父子独立／级联 |
| TC11–TC12 查询与异步 | `table/query.rs`、`table/outline.rs`、Showcase查询：typed0／false／范围、Local／Remote分离、过期／失败／重试、候选草稿、真实菜单／TreeSelect／DatePicker、分页草稿保留 |
| TC13 编辑 | `table/edit.rs`、Showcase `editing.rs`／`temporal.rs`：Input／Select／Date／Time真实草稿、保存flush、失败重试、行多字段、迟到保存、外部冲突与删除；真实IME仍需用户验收 |
| TC14 树与详情 | `table/outline.rs`及Showcase `structure.rs`：根分页、路径过滤、折叠、级联、节点票据、失败／重试、两个详情的独立滚动与关闭释放 |
| TC15 拖动与伸缩 | `table/layout.rs`、`table/customization.rs`：最后释放坐标、键盘key-up单次确认、Escape／失按键／query取消、源离屏取消、同owner自动滚动、列组／pin／protected边界 |
| TC16–TC17 成本与释放 | TB01／TB02／TB03／TB04／TB06阶段证据；TB05 Fixed／Measured 100次source替换、屏内／屏外／64行变化、旧source零保活、空闲零构建。CPU draw、draw+effects、准备／接受、分配及资源字节分别报告 |
| TC18 状态与语义 | `table.rs`、`table/cells.rs`、`table/customization.rs`：初次／刷新／失败／空／无列、占位不进selection／query／编辑／span、readonly／disabled、header／span逻辑坐标、capabilities和非法组合 |

以上是断言入口，不将每项中的所有原生／系统行为笼统标为已通过。GPU纹理驻留、真实IME组词、Narrator、跨屏DPI、原生输入到呈现延迟分别保持未验收状态。

## Showcase E01–E11

| 场景 | 可操作入口 |
| --- | --- |
| E01 基础与布局 | `table/basic.rs`：10个独立小表；`table/playground.rs`：固定／伸缩／分组、长文本居中、列footer；`pages/table.rs`使用Tabs分类 |
| E02 选择与行操作 | `table/playground.rs`／`query.rs`：单多选、页内全选、disabled、跨页选择、批量操作与行内控件 |
| E03 查询与分页 | `query.rs`：普通／树／自定义筛选、日期范围、稳定多排序、分页；Local／Remote实际接线 |
| E04 状态与恢复 | 基础属性面板及`query.rs`：首次／刷新／失败／空／无列、远程完成／失败／迟到；range占位不作业务行 |
| E05 列布局 | 基础布局开关＋`configuration.rs`：固定／伸缩／分组、窄宽恢复；Dialog草稿的显隐、pin、顺序、原始宽度、重置、取消／确认 |
| E06 编辑 | `editing.rs`／`temporal.rs`：Input／Select／Date／Time单元格与行保存、失败／重试／取消、离屏草稿；`query.rs`另有Dialog业务编辑 |
| E07 树与展开 | `structure.rs`：根分页／父子级联、树节点加载失败／重试、Descriptions详情及独立嵌套表 |
| E08 合并与宿主吸顶 | `advanced.rs`：Cells合并、Static自然rowspan、单owner双表affix；跨冻结拒绝等由直接测试补验 |
| E09 拖拽 | `customization.rs`：行手柄、整行opt-in、列手柄、键盘等价、真实顺序、排序激活拒绝；末次坐标与取消由输入测试补验 |
| E10 大数据 | 基础大量行、复杂记录、`advanced.rs` 50k×256矩阵分别按需挂载；range／tail展示渐进加载，远程例明确模拟完成 |
| E11 综合工单 | `query.rs`组合查询＋分页＋跨页选择＋批量＋列配置＋Dialog详情编辑；分页、重算和编辑保留配置，不强迫业务开启span／拖拽 |

示例入口改为9个可直接访问的标签分类；基础页不再依赖一组开关展示全部样式，其他分类仅在访问时初始化、挂载当前面板。普通示例保留状态，50k×256和已加载的100k压力数据离开分类后释放；加载动画显式开始／结束。远程例子明确模拟任务完成，宿主接入真实网络仍遵守同一请求票据合同。具体变更与CPU回归边界见[示例编排记录](../performance/2026-11-11-table-gallery.md)。

## 性能及人工验收范围

阶段性能采样先做正确性、来源／可执行文件身份及工作集核对，发现外部编译／Showcase整轮弃样，不结束用户进程。M6保留TB02同一fixture复测，并补TB05；数据不是通用“性能评分”。完整W1/W2原生输入、GPU、跨显示器与纹理驻留不由CPU或factory计数替代。计时回归阈值未冻结时，只接受正确性／规模解耦和资源归属结论。

当前设备条件不阻止可自动完成的阶段，但真机滚动手感和视觉由用户验收；多显示器不可用时保持未验证。Windows UIA Grid／GridItem／离屏Realize仍不是本次交付能力，使用现有可访问语义，不扩写自有COM provider。
