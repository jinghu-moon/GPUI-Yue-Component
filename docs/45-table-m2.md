# YueTable M2 布局与固定

本阶段扩展 [M1](45-table-m1.md)。阶段任务与验收持续记录在 [实施进度](research/2026-11-11-table-delivery.md)，性能采样完成后才提交、推送本阶段。

列的 `width` 是首选宽度，`min_width`／`max_width` 限定有效范围，`flex` 按权重分配容器余量，达到上限后的余量保留。不可能满足最小值时允许横向滚动，不把业务偏好覆盖为临时夹紧值。`responsive_priority` 越大越先隐藏，容器恢复时自动恢复；`visible(false)` 是宿主明确隐藏，两者互不覆盖。

`pin(TablePin::Start/End)` 固定左右列；`YueTable::frozen_rows(start,end)` 只计数据行，表头与列 footer 独立。`footer_fixed(true)` 固定列 footer；整表 footer 仍是内容槽。窄容器保留中间可读区，先解除 End、再解除 Start，扩大后恢复偏好。固定单元格只构建一次，复用原有 ScrollHandle 和 Base 冻结工作集，没有三表复制。

`header_path(vec![TableHeaderGroup::new(key,title), ...])` 声明外到内的祖先。组共享叶列宽，浅层叶表头纵向跨带。空组 key、重复祖先、非连续同组、同 key 不同语义及跨 pin 区域的组，在 `commit_view` 中原子拒绝。隐藏全部列显示无列状态；导航只访问实际显示列。

启用 `resizable(true)` 后，拖动分隔符即时预览；释放使用最后绝对坐标，只发一次 `TableEvent::ColumnResizeRequested`。宿主接受方式：

```rust,ignore
if let TableEvent::ColumnResizeRequested(request) = event {
    let accepted = state.read(cx).view().unwrap().clone();
    let next = request.apply_to(&accepted)?;
    state.update(cx, |s, cx| s.commit_view(next, Some(request.id()), cx))?;
}
```

拒绝时调用 `reject_request`，预览恢复；未结算请求不会伪造已接受列宽。分隔符可聚焦，左右键调整，Shift 加大步长，keyup 或正常 blur 结算一次，Escape、窗口失活、失去拖动或卸载取消。新来源／布局替换取消旧会话。批量配置由 `accepted.clone().with_columns(columns)` 一次接受，保存／持久化仍由宿主负责；完整列配置草稿面板在 M6。

实际宽度通过 `column_width` 读取，临时有效固定状态用 `effective_pin` 读取。以上设置为真实公共 API；跨区分组拆分、Cells 双轴与 span 属 M5。
