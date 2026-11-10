# Table 源码调研 10：Slickgrid-Universal

- 日期：2026-11-10；对应 [Table 设计](../45-table.md)，接续 [SlickGrid 调研](2026-11-10-table-slickgrid-review.md)。这是介绍顺序中的最后一个参考仓库。
- 本地目录：`refer/DataTable-Libs/slickgrid-universal`；HEAD：`f1cb220fb65b3dc51aecd91c52a373514ca85745`。
- [lerna.json](../../refer/DataTable-Libs/slickgrid-universal/lerna.json) 与已核对的 common／graphql／vanilla-bundle 包版本均为 **11.0.0**，根 package 是 private 工作区，不用根包的缺省 version 推断版本。[LICENSE](../../refer/DataTable-Libs/slickgrid-universal/LICENSE) 为 MIT，注明 Ghislain B. 与原 SlickGrid 作者；已核对三包许可字段同为 MIT。
- 已阅读该仓库 AGENTS.md；未修改参考源码或 dist。仅阅读定向源码、测试断言和基准材料，未安装依赖、运行测试／示例或进行性能实测。
- Context7 补充查询返回 Angular-Slickgrid 的 backendServiceApi／无限滚动文档，说明 service 与 process／postProcess 的分工；该在线 wrapper 文档不作为本地 11.0.0 的版本证明，默认延时等以本地源码为准。

## 1. 先分清核心、服务和封装

[README](../../refer/DataTable-Libs/slickgrid-universal/README.md) 将其描述为包含编辑器、过滤器、扩展和服务的完整装配。这个产品定位有参考价值，但不能将它与上一项 SlickGrid 6.0.0 混成同一个版本或测试对象。

| 部分 | 本地证据与职责 | Yue 的取舍 |
| --- | --- | --- |
| common | [package.json](../../refer/DataTable-Libs/slickgrid-universal/packages/common/package.json)；包含自己的 [slickGrid.ts](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/core/slickGrid.ts) 和 [slickDataView.ts](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/core/slickDataView.ts)，并有 services／editors／filters／extensions；运行依赖不靠另装 slickgrid 包来提供该核心 | 不是仅给上一仓库套一层 wrapper；本轮重点审服务接线，不重复移植核心，也不引入 Web 依赖 |
| binding／event-pub-sub／utils | 公共绑定、事件和工具包，供 common 等组合使用 | 用 GPUI Entity／Context／Subscription 与既有纯规则，避免再建响应式／事件平台 |
| graphql／odata／sql | 分别提供协议查询服务；common 的 BackendServiceApi 连接业务执行与结果处理 | 只学 typed query 与宿主映射的分工，不新增三个查询编译器 |
| pagination／empty／footer／tooltip 等 | 产品部件按包组合，另有 row detail 和 composite editor | 复用 YuePagination、YueEmpty、Tooltip、Form 和已有详情槽，不平行实现 |
| vanilla-bundle 与四框架封装 | Vanilla 装配，以及 Angular／React／Vue／Aurelia 消费面；[UniversalContainerService](../../refer/DataTable-Libs/slickgrid-universal/packages/vanilla-bundle/src/services/universalContainer.service.ts) 仅派生 common 的 MapContainerService | Windows GPUI 只需一条装配路径，不照搬框架适配矩阵或服务定位器 |
| export／rxjs-observable／web-mcp 等 | 工作区中的其他可选包 | 本轮不深入、不纳入 Yue 范围；包的存在不等于 Table 必须拥有这些功能 |

Yue 仍然是一个 Table 入口，消费 Base 虚拟化、滚动、轨道和真实 Yue 编辑控件；服务分层用于明确所有权，而不是把每个方法包装成可动态替换的插件。

## 2. 远程请求：统一发布身份比统一方法名更重要

证据：[BackendServiceApi](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/interfaces/backendServiceApi.interface.ts)、[BackendUtilityService](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/backendUtility.service.ts)、[PaginationService](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/pagination.service.ts)。

BackendServiceApi 将 query service、实际 process 和 pre／postProcess 分开；process 可接 AbortSignal。通用 executeBackendCallback 会先 abort 旧 controller，再调用 process，Promise 分支之后递增 request id；成功时只有当前 id 才进入结果处理。可选 RxJS 路径借 subject／takeUntil 取消请求。

这很好地展示了两道不同责任：取消底层工作节约资源，发布代次阻止旧结果污染新视图。Yue 保留两者，但不用“成功时比较一下 id”替代完整生命周期。

本地定向阅读还发现几处需独立验收的路径差异：

- Promise 的非 AbortError catch 直接触发错误处理，没有与成功分支相同的 id 检查。
- process 调用发生在 Promise 分支登记 id 之前，不在该方法的 try 边界中；启动失败和重入需要另行考虑。
- PaginationService.processOnPageChanged 直接调用 process(query)，未走上述 executeBackendCallback，也未传该入口的 signal；完成后直接处理结果、保存或恢复分页。
- 通用结果处理依次执行 service.postProcess、internalPostProcess、公开 postProcess，后两者可能修改 dataset／分页；检查不能只包住最后一处行赋值。

这些是源码路径观察，不是本轮运行复现的缺陷结论；不能据此声称所有 wrapper 都没有外围保护，也不能将通用工具的测试当作分页路径已经同样受保护。

**Yue 补充合同**：排序、筛选、翻页和刷新共用当前来源／query 的发布票据；先登记 pending，再调用宿主适配。相互替代的整份结果共用代次，合法并行范围／追加仍带各自范围／游标票据并受父代次限制。成功、失败、busy 清理、启动错误、拒绝、卸载和同步回调重入都验证身份；旧 A 不能清掉 B 的 busy、恢复旧页或通过转换回调覆盖新 total。Rust 业务错误使用 Result 等已有边界，不引入 catch panic 恢复。

这些规则补入主设计第 15 节，在 M3 和 M5 的既有接线中验收，不建立通用请求框架。

## 3. 列状态：保存值，重新连接当前声明

证据：[GridStateService](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/gridState.service.ts)、[GridState](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/interfaces/gridState.interface.ts)、[CurrentState](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/interfaces/currentState.interface.ts)。

getCurrentGridState 汇总 columns、filters、sorters、pinning，并按能力加入 pagination、selection、tree 等。getAssociatedGridColumns 通过 columnId 重新关联当前列声明，不要求序列化 editor／formatter；找不到的列不会直接凭旧对象复活。syncPluginColumns 再协调选择、移动、详情等展示列。

当前快照把永久 pinning 单独放在 GridState.pinning，列布局条目不再复制同一 pinning 事实；已有旧预设兼容测试。这与 Yue 的唯一事实所有者原则一致。但参考的 cssClass／headerCssClass、索引形式的行 pinning 和自动 resize 选项不成为 Yue 序列化合同。

applyColumnLayout 只有非空列表才应用，底层关联函数却能返回空列表；这提醒 Yue 必须区分缺省、显式空和重置，而不能每层用 length／truthiness 推测含义。Yue 还要保留当前必显列、权限和新增列规则，不能直接复制“旧列表外全部隐藏”的语义。

**Yue 补充合同**：宿主以带 schema／列语义版本的白名单保存列偏好；只保存允许恢复的 key、顺序、显隐、宽度偏好和固定配置。测量值、临时隐藏、Entity、renderer、pending 和编辑会话不落盘。恢复先按当前声明／权限校验，再一次静默接受并合批失效；无变化不额外测宽。若宿主另存 query，显式协调一次再取数，不在逐切片恢复过程中触发多次查询。

沿既有 ColumnsRequested／commit_view 和宿主持久化执行，不增加通用 storage service 或新的状态所有者。

## 4. 异步候选：更新数据不等于重建编辑器

证据：GridStateService.loadEditorCollectionAsync／updateEditorCollection 及[对应测试](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/__tests__/gridState.service.spec.ts)。

loadSlickGridEditors 发现 collectionAsync 后启动解析，先把 editor 标成 disabled；Promise／Fetch 或可选 Observable 完成后更新 collection。updateEditorCollection 取得当前 editor，若具有相应方法就 destroy，再 renderDomElement(newCollection)。已读测试明确断言 destroy、disable(false) 与重新 render 被调用。

有用的需求是“候选可异步提供并重新启用”，但 Yue 不复制重建流程。当前函数中未见候选列与当前 cell／编辑会话的身份核对，也不能从该测试推导草稿、选区或 IME 被保留。

**Yue 补充合同**：候选集合按来源、column key、规则和候选请求代次发布；触碰当前控件前重验草稿／编辑会话。复用 Select／TreeSelect 的 typed 值与标签、异步数据更新和真实 Entity。0／false 不丢失，旧列／旧会话的成功和失败不回写新控件；刷新不销毁编辑器、不覆盖正在输入的草稿或组词。当前值暂时不在候选中不自动清空／保存，由已有字段规则显式校验；失败结算后可重试。

它只补第 13／14 节既有筛选和编辑流程，不建设 Table 专用候选引擎。列级可共享候选结果与会话级控件更新分开校验，不为每个格子重复加载整份枚举。

## 5. 协议服务：只吸收字段和值的边界

[SQL service](../../refer/DataTable-Libs/slickgrid-universal/packages/sql/src/services/sql.service.ts) 分开 buildWhereClause／buildOrderByClause 与标识符、值处理，涉及 queryField、类型、null、IN、LIKE 和不同标识符引号。它说明 UI 的 columnId、业务字段、typed 值与最终协议表达不是同一对象。

Yue 的完整 TableQuery 继续交宿主适配器：授权字段映射、算子白名单和协议编码由宿主决定，SQL 用实际驱动参数绑定；不把用户输入或显示标签拼接为语句，也不把一个通用 escape 替代各协议的规则。这里未审计整个 SQL 服务或证明其安全性，不移植它的字符串构造／数值修正规则。

BackendServiceApi 的 useLocalFiltering／useLocalSorting 注释明确只操作已加载集合。Yue 不引入这些模糊开关：Local 的处理范围要完整明确，Remote 不对当前页重筛／重排后仍冒充全域结果或继续沿用原 total。

## 6. 扩展清理：借鉴归属，不新增插件机制

[ExtensionService.dispose](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/extension.service.ts) 遍历已登记实例调用 dispose，再清空映射和持有字段；GridState／Pagination 的 dispose 也分别解除自身事件和订阅。

可借鉴的是装配者清理自己创建的资源及清除引用。Yue 复用 Entity／Subscription／Task 与既有 owner／mount 代次，覆盖候选、去抖和进行中业务请求；释放一张表不得清掉共享 delegate 的其他订阅。dispose 函数存在、调用次数匹配，都不足以证明迟到 Promise／微任务不再触碰新 owner。主设计已有此合同，本轮只补候选和跨入口请求回归，不重复新增事件总线或插件注册表。

## 7. 基准：先完整等价，再测各阶段

证据：[基准说明](../../refer/DataTable-Libs/slickgrid-universal/test/benchmarks/README.md)、[filterService.bench.ts](../../refer/DataTable-Libs/slickgrid-universal/test/benchmarks/filterService.bench.ts)。

过滤基准使用确定数据，包含宽匹配、稀少匹配、零结果、嵌套字段、日期范围、组合过滤和清除条件。先以朴素 matches 得到完整 expectedIds，分别断言 predicate 结果和 DataView 的完整 key 顺序；不以“行数一样”代替正确性。准备与等价检查位于计时外，再分别测 predicate scan 和 DataView refresh，源码中有预热及 median／p95 样本报告。

基准说明还明确 SlickDataView 微基准隔离生产过滤／累加循环，不包含分页、分组、事件和行差异计算；相对比较、误差和同机条件须明确。这些限制与结果一样值得保留。

Yue 采用完整结果等价、计时边界和噪声判断：测试实际的数值／日期／业务嵌套取值规则与组合条件，同时核对 total／分页。候选扫描、完整投影及 GPUI 构建／布局／绘制分开报告。只在发现瓶颈后做前后采样，不移植 JS 基准工具、不照搬样本数，也不将参考 JSON 或“百万行流畅”宣传转成 GPUI 性能证据。

## 8. 实际读到的断言及其边界

| 材料 | 实际断言／观测 | Yue 应补的验收 |
| --- | --- | --- |
| [backend-utilities.spec.ts](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/__tests__/backend-utilities.spec.ts) | 两次请求后旧 signal.aborted 为 true；新结果进入处理；AbortError 不调用错误回调，普通错误调用；单次成功调用后处理和 success | 分开验证资源取消与发布身份；旧成功、旧失败、旧清理都不得改变新状态 |
| 同一文件的 stale Promise 测试 | A 用 100ms 定时，B 立即 resolve；waitFor 只等待处理次数为一次且参数是 B，未明确等待 A 也结束后再作最终断言 | 使用可控任务先后完成 A／B，确保旧任务所有分支真正执行后再断言；不能把提前返回当作竞态已覆盖 |
| [pagination.service.spec.ts](../../refer/DataTable-Libs/slickgrid-universal/packages/common/src/services/__tests__/pagination.service.spec.ts) | preProcess、before-change 拒绝，以及单次 Promise／Observable 错误回调等 | 定向已读片段未证明排序→翻页→刷新交叉竞态；Yue 必须覆盖各入口相互覆盖和挂载失效 |
| GridState 测试的 pinning／关联列／布局片段 | 隐藏固定列仍在四侧 pinning；无固定为四个空数组；旧布局 pinned 不重复覆盖当前声明；列关联与 setColumns／autosize 的调用 | 当前声明、用户偏好、测量缓存分离；旧 schema／失权／删列、空配置、批量恢复不重复请求 |
| GridState 异步 editor 测试 | Promise、content、Fetch、Observable 的集合更新，以及当前 editor destroy／重建调用；另有 Fetch body 已用的警告断言 | 测试“调用过重建”不能证明值／选区／IME 正确；Yue 检查同一 Entity 与草稿、跨列会话及迟到失败 |
| filterService 基准 | 计时外核对完整 key，计时内分别测扫描和 DataView refresh | 正确性先于收益，结果域与耗时边界明确，性能变更才比较 |

[test/vitest.config.mts](../../refer/DataTable-Libs/slickgrid-universal/test/vitest.config.mts) 使用 jsdom／mock 和可控 timer，并忽略匹配部分 EventTarget／removeEventListener 文本的未处理错误；这类环境不能独立证明真实浏览器几何和全部清理路径。根 AGENTS 还注明 Cypress testIsolation:false 的串行场景约束。Yue 的 fixture 应独立，不能依赖上一场景遗留数据或将忽略错误视为清理通过。

本轮没有执行上述测试，也未全面审计全部包或四框架；这些材料仅是可定位的设计和回归依据。GPUI bounds／scene／真实输入仍须后续验收，Windows IME、GPU、Narrator 和跨 DPI 均不因参考测试而宣称通过。

## 9. 吸收清单与调研收束

| 编号 | 补充／强化的合同与回归 | 阶段 |
| --- | --- | --- |
| SU01 | 查询入口共用发布身份；取消与发布分开；启动／成功／失败／busy／重入／卸载完整结算，合法并行不误杀；可控任务全部完成后断言 | M3／M5 |
| SU02 | 带版本白名单列偏好、当前声明重关联，缺省／空／重置明确；一次静默恢复，不重复测宽或取数 | M2／M6，query 宿主接线 M3 |
| SU03 | 异步候选核对列与会话，0／false 保真，不重建真实 editor；保留草稿／选区／IME，失败与迟到任务结算 | M3／M4 |
| SU04 | 字段映射、算子和值编码在宿主，SQL 参数绑定；不新增协议引擎，不将页内筛选冒充全域 | M3 |
| SU05 | 优化先核对完整结果，准备与断言在计时外，扫描／刷新／GPUI 成本分开，低于噪声不判收益 | M3／M5 |
| SU06 | 装配者清理自有订阅与候选／任务，旧回调不复活新 owner；独立 fixture | M1–M4；强化既有合同 |

至此，介绍顺序中的 **十个参考仓库均已完成逐项源码与验证材料调研**，状态见主设计第 2.2 节。本轮仍只交付设计与研究记录，没有实现这些合同，没有新增 Cargo／Web 依赖、运行时插件系统、协议服务、导入导出或电子表格能力。

下一步应进入主设计既定 **M1：可用的基础表格**，先建立可回退的基线与失败用例，再实现最小真实 Table、Showcase 并复测。本轮没有自动开始实现，也不把调研完成当作 Table 已经完成。
