# YueTable 设计自评后的五项完善

日期：2026-11-10；范围为 [主设计](../45-table.md)、[API 与接入示例](../45-table-api-examples.md)、[性能方案](../performance/2026-11-10-table-benchmark-plan.md)。不实施 Table，不修改参考网页，不运行参考测试或 Rust 构建。

本记录对应上一轮自评提出的 API、原子提交、已有控件适配、性能材料、文档收敛五项；保持 Windows GPUI／Base-only 和既有 M1–M6，不新增产品能力或插件／协议引擎。

## 1. 三份 refer/table 网页复核

本轮浏览器访问 host／native pipe 均不可用，已尝试后停止；直接以 UTF-8 解析三个已保存 HTML 的实际说明、示例代码、标题和 API 行，未声明进行了新的浏览器交互验证。首次浏览器调研与截图范围仍见[原三库记录](2026-11-10-table-reference-review.md)。三文件 SHA-256 与原记录一致。

| 本地网页 | 本轮定向核对 | 采用方式 |
| --- | --- | --- |
| [Ant Design Vue](<../../refer/table/表格 Table - Ant Design Vue (2026_11_10 18：34：39).html>) | API 的 filteredValue、sortOrder 明确为受控属性；FAQ 解释筛选后默认返回第一页以免出现空页；rowKey 说明强调稳定主键。页面保存的代码区并非全部包含完整示例 | query 仍唯一受控事实；筛选和页码一次提交，示例显式稳定 key；不复制多个列级状态 owner，也不声称离线保存了所有可执行源码 |
| [Arco Design（React）](<../../refer/table/表格 Table ｜ ArcoDesign (2026_11_10 18：34：36).html>) | 基础例明确 columns＋data；目录独立划分选择、排序筛选、多排序、自定义分页、内容、固定、编辑等；多排序说明要求各列 sortOrder 一致；renderPagination 指向分页组件 | 基础例少配置，逐项示例与综合例分开；一个 ordered sort 避免列间漂移；分页复用 YuePagination 并只做窄适配 |
| [TDesign（Vue Next）](<../../refer/table/Table 表格 ｜ TDesign (2026_11_10 18：33：14).html>) | 基础例 row-key＋data＋columns；本地分页说明会内部切 data，远程分页说明只输出分页信息；列配置说明区分确认前 onColumnChange 与确认后 onDisplayColumnsChange；可分别配置列设置入口 | Local／Remote 例分开，禁止双重分页；列草稿确认一次；示例宿主提供 Dialog／按钮，不新建列配置框架。主设计仍一个 Table 入口 |

采用网页中的 API 教学与交互组织，不照搬其 CSS 布局、index 主键示例、DOM 属性透传或运行时阈值。网页静态内容不能证明性能或组合功能已在 Yue 实现。

## 2. 五项完善的具体结果

| 自评问题 | 本轮完成 | 后续实施证明 |
| --- | --- | --- |
| 公共 API 不够具体、无 Rust 示例 | 来源窄 trait、Row 借用、typed 异构列／筛选、请求重基与默认值；共享数据／列与三个完整宿主接线草案，无省略业务 helper | M1 编译与挂载基础例；M3 编译 Local／Remote、可控 service、typed 值测试；仍未编译不能发布可运行承诺 |
| 提交失败后旧数据能否保留 | 采用不可变 TableSource 快照，准备→校验→重验→一次发布→通知；State 读取 accepted source，不读 delegate 可变最新容器 | TC02 注入重复 key／total／span／票据／卸载失败；断言旧 source 引用、view、revision 与合法会话不变 |
| 复用只写名称、未写接线 | 新增主设计第 19.2.1 节；分页仅在值变化调 setter，候选更新不重建 Entity，Input／Form／Tree／Scrollbar／Popup 各有更新范围 | TC03／TC12／TC13：跳页草稿、选区／组词、同值 Popup、拒绝／取消和真实输入断言 |
| 性能没有固定材料和门禁 | TB01–TB06、W0–W5、计时／内存／调用层、身份与原始日志、ABBA 独立重复、噪声基线与暂定诊断线 | M1 首份可信基线；后续能力开放前增加对应 fixture；本轮无采样结果，不把 5% 当已达标 |
| 回归按参考库堆积、排版缺陷 | 主设计合并为 TC01–TC18 行为矩阵；E01–E11 单项到组合示例；来源编号在本记录追溯；修复 Universal 孤立表格行 | 文档结构／链接校验；实施测试按合同编号而非各参考库复制一套，原研究证据保留 |

## 3. 对现有源码的适配核对

- [PaginationValue](../../crates/yue-component/src/pagination/model.rs)：page／page_size／total、规范页码与非零 page_size，以及 PaginationChange 的 before／value／source；当前已知 total 合同不被 Unknown 混入。
- [YuePaginationState](../../crates/yue-component/src/pagination/state.rs)：set_value 会 reset_draft，并同步 Select；因此 Table 无关更新不能无条件调用它。set_loading 单独存在，等待不必伪造分页值。
- [YueSelectState](../../crates/yue-component/src/select/state.rs)：set_options 更新同一 Entity，set_selected_items 可补已选身份；SelectValue 是稳定 SharedString key，**不是任意 Rust typed V 的容器**。Table 业务 typed 值须经列适配映射，显示标签不作为数值或 bool。
- [Form](../../crates/yue-component/src/form/mod.rs)和 [Field](../../crates/yue-component/src/field/mod.rs)：复用既有 typed 规则、submit 与错误呈现；异步业务校验／保存仍属宿主，不声称 Form 已有 Table 竞态引擎。
- [分页 Showcase](../../examples/windows-showcase/src/pages/pagination.rs)：Entity／subscribe_in／Task／spawn_in／弱引用更新形式作为接线依据；没有把该例的延时或展示状态机当作 Table 新 API。
- [Base Table](../../vendor/gpui-base/src/table.rs)、[GridTrack](../../vendor/gpui-base/src/virtualization/grid_track.rs)、[Grid](../../vendor/gpui-base/src/virtualization/grid.rs)和 [YueScrollbar](../../crates/yue-component/src/scrollbar/mod.rs)：分别为语义壳、约束求解、双轴／span／前缀冻结、overlay 呈现；尾端冻结和 Table 产品接线仍需实现验证。

不可变快照的目的在于固定“本次接受读取什么”，不是要求整份数据每次深拷贝。来源索引可在新结构时 O(N) 验证；选择、hover、列宽等使用已验证来源，不每次重验全部 key。旧快照只有合法使用者仍在时保留，任务／缓存不积累全部历史代。

## 4. 按行为追溯参考证据

下列为原吸收清单编号到主设计 TC 合同的映射；这些是设计证据，不是已经存在的测试名称／通过状态。同一编号可落多条合同；测试实现可一次证明多项，正式用例还需标注测试路径与结果。

| 专项记录 | 原编号 → 当前主要合同 |
| --- | --- |
| [Tabulator](2026-11-10-table-tabulator-review.md) | T01→TC16；T02→TC02；T03→TC08；T04→TC05；T05→TC05／TC16；T06→TC06／TC15；T07→TC11／TC13；T08→TC13；T09→TC04；T10→TC04／TC11；T11→TC07／TC08；T12→TC18／主设计范围 |
| [TanStack](2026-11-10-table-tanstack-review.md) | TS01→TC02／TC03；TS02→TC16；TS03→TC01；TS04→TC01／TC05；TS05→TC06；TS06→TC15；TS07→TC10／TC14；TS08→TC10；TS09→TC12；TS10→TC07／TC17 |
| [Glide](2026-11-10-table-glide-review.md) | GD01→TC02／TC16；GD02→TC16；GD03→TC07／TC16；GD04→TC17；GD05→TC17；GD06→TC08／TC16；GD07→TC11／TC16；GD08→TC13 |
| [RevoGrid](2026-11-10-table-revogrid-review.md) | RG01→TC07；RG02→TC07／TC09；RG03→TC06／TC07；RG04→TC08；RG05→TC07；RG06→TC13；RG07→TC04；RG08→TC18 |
| [MUI](2026-11-10-table-mui-review.md) | MX01→TC02／TC04；MX02→TC02；MX03→TC08／TC17；MX04→TC13；MX05→TC04／TC13；MX06→TC06；MX07→TC10／TC13；MX08→TC18 |
| [react-data-grid](2026-11-10-table-react-data-grid-review.md) | RD01→TC07／TC10；RD02→TC07／TC10；RD03→TC05；RD04→TC15；RD05→TC07；RD06→TC09；RD07→TC13；RD08→TC10／TC13 |
| [MRT](2026-11-10-table-material-react-table-review.md) | MRT01→TC05；MRT02→TC05；MRT03→TC11／TC12；MRT04→TC06／TC16；MRT05→TC14／TC18；MRT06→TC15；MRT07→TC13；MRT08→TC04／TC10／E11 |
| [Grid.js](2026-11-10-table-gridjs-review.md) | GJ01→TC11／TC16；GJ02→TC11；GJ03→TC11／TC18；GJ04→TC11；GJ05→TC12；GJ06→TC04；GJ07→TC01／TC05／TC16 |
| [SlickGrid](2026-11-10-table-slickgrid-review.md) | SG01→TC07；SG02→TC07；SG03→TC02／TC16；SG04→TC11／TC16；SG05→TC16／TC17；SG06→TC13；SG07→TC04；SG08→TC02／TC09 |
| [Universal](2026-11-10-table-slickgrid-universal-review.md) | SU01→TC04；SU02→TC05；SU03→TC12／TC13；SU04→TC11；SU05→TC16／性能方案；SU06→TC04／TC17 |

归并没有撤销原章节的细节要求：订阅建立窗口补偿、数据批次顺序、非连续 span、字段权限、同击键浮层路由、尺寸重入与资源字节等仍保留在正文和专项记录。Windows UIA Grid 链路仍有已知缺口，没有借本轮设计扩大 COM provider 维护范围。

## 5. 验证与交付范围

本轮检查 Markdown 相对链接／锚点、表格结构、代码围栏、尾空白、编码、引用编号覆盖，以及参考网页哈希和参考仓库未修改。Rust 草案的 7 个代码块已用 rustfmt 解析并格式化；这只检查语法形式，不解析不存在的 Table API 或进行类型检查，不能推断消费者已经编译。没有为文档修订运行无关 Cargo 测试。

五项完善均已落到具体文档与阶段出口。下一步仍是 M1 的真实实现与失败基线，用实际消费者和 GPUI 交互检验这些设计；本记录不宣布新的质量评分、运行时性能或完整组件交付。
