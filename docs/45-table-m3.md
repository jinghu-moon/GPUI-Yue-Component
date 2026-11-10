# YueTable M3 查询与数据流程

本阶段建立 typed 查询、Local 投影和 Remote 返回页合同。实际可编译入口见 [API 与三个消费者](45-table-api-examples.md)，阶段状态和证据见 [实施进度](research/2026-11-11-table-delivery.md)。

数据仍由宿主持有。`TableLocalQuery::execute` 在更新路径做一次过滤、稳定多排序和分页，只共享 Row、不克隆整条业务记录。`TableProjection` 的位置是当前结果位置，`row(key)`／`contains_key(key)` 仍可解析完整本地域；不能把显示位置当稳定身份。`execute_with_summary` 在分页前给宿主一次完整结果 key 切片，用同一次查询计算筛选域汇总，不再为每个汇总重复排序。汇总值由宿主保存，render 不运行查询。

`TableColumn::value` 先声明 typed accessor，再选择 `display_with`／`render_with`，构造同一异构列集合。默认排序要求 Ord，自定义比较器负责完整顺序；筛选携带 `TableFilterKey<V>`，支持 Scalar、Set、闭区间 Range 和显式 Missing。数字 0、布尔 false、空文本、空集合和缺失分开处理。非法区间、重复排序列、不支持的列和过期规则在发布前拒绝。clone 列保留语义身份；重新声明同名规则会使旧 filter key 和等待中的查询失效。

排序循环为无→升→降→无；Shift 加入多排序，方向切换保留优先级。表头显示已接受方向和序号。自定义 header 优先，宿主在自定义控件中调用同一 request_sort。导航模式 Alt+S 排当前活动列，内部 Input／Popup 保留自己的键盘合同。

`request_filter`／`request_sort`／`request_page`／`request_refresh` 共享完整查询票据。新查询替换旧查询，同一更新中的连续 typed 意图累积在最新待处理目标上；接受前仍显示旧查询与旧行。响应发布前先检查 `is_current_request`，再转换 total／summary／行；commit 和 reject 再重验。其他领域请求不因新 query 自动丢弃，同 scope 列宽调整保留合法等待，来源替换或查询规则变更使旧完成失效。

Remote 不运行 Local 管线。宿主用 `TableRemotePage::new(page, known_keys)` 显式保留已知 key 域，`row(key)` 只返回已加载记录。批量命令消费 selected keys，不能将当前页可解析 records 冒充全部选择。页内全选／取消只作用于本页，表头三态根据本页可选和已选数判断；计数在接受时计算，滚动不扫描所有选择。

total 是查询结果总数，source.len 是当前页记录数。Known(0) 与 Unknown 分开；已知总数的页码／页长／实际记录数不一致会原子拒绝，Unknown 页也不能超过 page_size。Local 自动夹紧末页。已有 `YuePagination` 只处理 Known，Unknown 可组合宿主的上一页／下一页按钮，不能伪装成零页。`sync_table_pagination` 静默同步且仅改变值时调用 set_value，跳页 Input 的未提交草稿不会被选择、主题或无变化通知重置。

Showcase 按需挂载独立 Local／Remote 查询卡片，复用 DropdownMenu、TreeSelect、Input、Pagination、Button 和 Table。名称／数值范围先形成草稿，取消零请求，应用一次完整查询。Remote 例由宿主保有完整模拟服务端数据，提供“完成最新／旧请求”和失败按钮；该例的完整域汇总由模拟服务端计算，不能推导真实 Remote 当前页能计算完整结果。

自动化覆盖完整 250,000×8 fixture oracle、零／false／日期／嵌套字段、字面特殊字符、清除与零匹配、分页末页、同名规则失效、迟到成功失败、跨页 key 和 records 分离、本页三态、批量操作、真实普通／树筛选、分页草稿与失败重试。查询谓词计数探针覆盖滚动、选择、rem 和主题更新，预算为零重算。

TB04 是 CPU 查询管线基线，第一执行与重复执行分开，验证／过滤／排序／投影／最终接受分段记录，fixture 和 oracle 不进计时。不把该结果称为真实窗口 FPS、GPU 或原生输入延迟。真实 IME／Narrator／跨显示器验收仍按设备能力独立记录。
