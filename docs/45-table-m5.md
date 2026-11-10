# YueTable M5：双轴、合并和宿主滚动

日期：2026-11-11。此页描述可调用 API；性能证据单独记录。

## 双轴呈现

`YueTable::render_mode(TableRenderMode::Cells)` 使用 Base VirtualGrid 的真实行、列轨道。Fixed／Known 行高和各叶列有效宽度分别建立索引；只构建工作行、工作列交叉处，以及必要冻结、活动控件、编辑器与合并 origin 的去重并集。不会预建 N×C 个 cell。

Rows 继续按工作行 × 全部显示列绘制，适合常见记录与整行复杂内容；Static 适合短表和自然内容流。Measured 与全宽详情使用 Rows，Cells 的这些不支持组合会拒绝。通过 `try_render_mode(mode, cx)` 可在进入呈现前取得错误；最终 builder 配置仍须满足合同。三种模式共享 selection、query、编辑与列声明。

`YueTableState::scroll_to_cell(key, options, cx)` 分别提出行、列定位请求，由同一个 Grid／ScrollHandle 结算，避开首尾冻结区域。covered 格转换到 origin，方向键从 origin 跨过完整跨度。活动控件按单个 cell 保留，不因两个离屏格扩出两个轴的乘积；整行编辑只保留该行可编辑列。

## 稀疏合并

```rust,ignore
let plan = TableSpanPlan::new(vec![
    TableSpan::new(TableCellKey::new(row_key, column_key), 3, 2),
]);
let view = TableView::new(source, columns).with_spans(plan);
state.commit_view(view, None, cx)?;
```

完整计划在接受时验证，屏外错误同样原子拒绝：零跨度、溢出、越界、重叠、跨声明 pin 区、跨首尾冻结数据边界、响应式列和详情。Rows／Static 只接受单行 colspan，Cells＋Fixed／Known 支持 rowspan。索引按稳定 key 随新 source 重新解析；分组、列宽及 source 变化使缓存失效。Base `VirtualGridSpanSet` 保留解析缓存，渲染不逐帧扫描全部描述符建立覆盖矩阵。Rows 的 colspan 只查本行描述符。

一个合并格只有一个 renderer、命中和语义 origin，声明 row_span／column_span；屏外 origin 覆盖可见区域时由 Base 拉入。合并尺寸是已知轨道之和，不反向撑大轨道。列隐藏使计划缺失 key 时需宿主同步提交新计划，不静默猜测新跨度。

分组表头可以横跨 pin 声明：按当前有效固定区域生成显示片段，保留一致标题／父路径并要求同组连续。一个逻辑组只在首片段暴露 ColumnHeader 语义；其余片段用于显示。浅层叶表头按剩余层数跨行。窄容器解除 pin 后重新合片，偏好不丢。

## 页面／面板吸顶与吸底

```rust,ignore
YueTable::new("orders", &table)
    .scroll_host(&page_scroll, rems(3.), rems(0.))
    .footer_fixed(true)
```

宿主本身负责 `overflow_scroll().track_scroll(&page_scroll)` 和 YueScrollbar。Table 借用这个 owner，按自然内容尺寸占位，不注册第二个滚动区域。上下留白为宿主工具栏预留空间；冻结表头、表尾被约束在表格与宿主可见区的交集内。同页多表不互相夹紧偏移；离开自身范围解除。定位到完全离屏的另一张表也按宿主可用尺寸解析，再确认宿主真实夹紧结果。

Base 新增 `VirtualGrid::in_scroll_host(top_px, bottom_px)`，应在其他样式 builder 前调用。当前宿主适配支持 Rows／Cells＋Fixed／Known；Measured、Static 和 RTL 不在该适配组合内。普通 cell 继续透明，冻结底板沿用 `surface_background`。Popup 仍走既有窗口 overlay 层，没有复制局部弹出系统。宿主不应在正在滚动时更换同一表格的 ElementId。

## 范围加载

`TableRangeRows<R>` 保存查询范围内稳定 slot key、O(N) 位置索引和稀疏 `Rc<R>` records；不会为每个占位构造业务 R。`with_records(next_revision, indexed_records)` 创建新不可变快照。slot key 是查询范围内位置身份；换查询创建新的 scope，业务行编号可保留在 R 中。自定义 source 也可实现 `is_placeholder`／`has_placeholders`。占位没有业务 record／selection membership，不参与本地查询、编辑和汇总。

`YueTable::range_loading(prefetch_rows)` 从实际虚拟工作范围产生 `TableEvent::RangeRequested`；带 owner、source revision、scope、query 和区间的 `TableRangeRequest` 交给宿主启动／取消任务。预取距离这里是显式行数，尾部未知游标导航需宿主另行定义，不能和分页混用。相同／包含需求去重，每表至多一个当前范围请求，新需求使旧票据失效。

完成后调用 `commit_range(&request, candidate, cx)`，而不是无条件覆盖 view；候选必须保持查询、scope、slot 长度，并填满请求区间。旧成功、失败、另一个表的票据均拒绝。失败调用 `reject_range`，同范围不会自动无限重试；`retry_range` 生成新票据。换 source／query／列 schema 或卸载解除待处理票据，宿主任务应使用 WeakEntity。

## 示例与验证

Table 页面通过按钮按需挂载高级示例：50000×256 矩阵、3×3 合并格、同面板两张吸顶表，以及10000槽位范围加载、失败与重试。关闭示例释放 Entity，未打开时不构建大数据。

自动化验证覆盖双轴大跳／反向与定位、首尾冻结、合并 origin 和非法屏外计划、跨 pin 唯一表头、宿主留白变化、两表与卸载、完全离屏程序定位、范围去重／倒序／失败／重试／跨表及换查询、真实范围加载 Showcase 与旧 source 释放。TB02 使用三档 Known 行高／列宽、两侧各2列、首尾各1行和16个合并描述符；计时结果是新增能力的规模基线，不是优化收益或原生 FPS。

M6补齐Static自然rowspan、未知尾游标和完整builder能力校验；当前合同见[M6指南](45-table-m6.md)。本页M5性能证据保持原采样身份，不用M6结果替换。
