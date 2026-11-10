# Table 源码调研 08：Grid.js

- 日期：2026-11-10；对应 [Table 设计](../45-table.md)，接续 [Material React Table 调研](2026-11-10-table-material-react-table-review.md)。
- 本地目录：`refer/DataTable-Libs/gridjs`；HEAD：`a8e41b91cb42a96dcd96da421ff578fa572ef07f`；包版本 **6.2.0**，根 LICENSE 为 MIT。
- 依据：[package.json](../../refer/DataTable-Libs/gridjs/package.json)、定向核心源码和下述测试；另通过 Context7 核对官方远程 search／sort／pagination 与 updateConfig／forceRender 说明。在线说明不替代本地快照。
- 本轮只做调研与设计补充，未安装依赖、运行参考测试／应用、测量性能或实现 YueTable。

## 1. 定位：轻量数据表，不是虚拟化引擎

Grid.js 依赖 Preact，按数据获取、转换、过滤、排序、分页与表格视图组织代码。[TBody](../../refer/DataTable-Libs/gridjs/src/view/table/tbody.tsx) 对当前结果的 `data.rows` 逐行创建 TR；本轮在 `src` 和 selection 插件中检索、阅读未见行／列虚拟化取窗路径。分页可以减少单页渲染量，不能据此称其为有界虚拟工作集。

值得学习的是小而明确的数据阶段、远程参数适配、独立规则测试与轻量入口。Yue 继续使用已有 Base 虚拟化，不移植 Preact、DOM、HTML 输入、Web 插件注册或第二套表格引擎；也不以其 README 的轻量／快速描述推导本项目性能。

[Tabular.fromArray](../../refer/DataTable-Libs/gridjs/src/tabular.ts) 全量创建 Row／Cell；[Base](../../refer/DataTable-Libs/gridjs/src/base.ts) 默认生成 ID，重新转换会重建嵌套身份。Yue 仍以宿主 stable key 读取记录，仅为实际工作集创建视图，不预先为所有 N×C 格建立实体，也不因刷新重新生成业务身份。

## 2. 阶段缓存：借鉴增量重算，补结构变化与执行中失效

证据：[pipeline.ts](../../refer/DataTable-Libs/gridjs/src/pipeline/pipeline.ts)、[processor.ts](../../refer/DataTable-Libs/gridjs/src/pipeline/processor.ts)、[pipelineUtils.ts](../../refer/DataTable-Libs/gridjs/src/pipeline/pipelineUtils.ts)。

ProcessorType 规定 Initiator → ServerFilter／Sort／Limit → Extractor → Transformer → Filter → Sort → Limit。处理器按 ID 缓存结果，以最早发生 props 更新的阶段索引为失效点；setProps 内容未变时不发更新。顺序执行可等待 Promise。这个机制把“翻页不重新获取／转换本地数据”的成本边界直接体现在代码中。

Yue 第 5.4 节已有固定流水线与版本依赖，采用同一思路而不建立任意处理器平台：纯页码变化复用过滤／排序，过滤变化使全部下游失效，等值接受不重复调度。阶段结果共享记录／key，不复制完整 Row；更换输入必须推进版本，不能凭处理器 ID 命中旧输入结果。

源码还提示两类需要单独防范的情况，**本轮未执行复现，不将其写成已确认运行故障**：

- unregister 移除步骤并发 updated，未见同步维护最早失效位置／移除对应缓存的处理。缓存已热后移除中间筛选／排序，不能继续消费仍受旧步骤影响的下游结果。
- process 在开始时保存步骤及失效位置，await 期间允许外部改条件；末尾又将失效位置标成 steps.length。需要防止旧执行把期间到来的新失效标成“已处理”，或污染新代缓存。Processor 的 afterProcess 在返回 Promise 后立即发出，也不等于异步结果已经完成。

Yue 因此补验收：移除已接受的筛选／排序条目、切换合法处理配置都失效正确下游；若宿主把本地查询放到异步任务，完成须核对来源／query／列语义版本，旧任务不能发布结果、填入当前代缓存或吞掉下一轮失效。沿现有代次与合批规则实现，不新造异步 pipeline API。

## 3. 远程请求参数可以组合，网络所有权仍归宿主

证据：[ServerStorage](../../refer/DataTable-Libs/gridjs/src/storage/server.ts)、[ServerInitiator](../../refer/DataTable-Libs/gridjs/src/pipeline/initiator/server.ts)、[ServerGlobalSearch](../../refer/DataTable-Libs/gridjs/src/pipeline/filter/serverGlobalSearch.ts)、[ServerSort](../../refer/DataTable-Libs/gridjs/src/pipeline/sort/server.ts)、[ServerPagination](../../refer/DataTable-Libs/gridjs/src/pipeline/limit/serverPagination.ts)。

MRT 主要提供 manual 状态接线；Grid.js 则让 search／sort／pagination 依次接收上一步 URL／body，合成一次取数参数。ServerStorage 分开处理响应、行数据和 total，并允许 data 回调替换 fetch、接入宿主 SDK。其可取之处是传输与表格查询解耦，并区分服务端总数与当前返回数据。

Yue 沿现有完整 `TableQuery` 请求，在宿主适配器中一次映射到 HTTP／SDK／数据库参数，不要求 Table 知道 URL、headers 或响应 JSON。列用业务 key 映射到宿主允许的查询字段，不能按当前显示 index 取字段名；隐藏／插入操作列／重排后仍命中正确字段。过滤、排序和分页不得分别发出三个网络请求，也不在 Remote 下重复执行本地处理。

本轮已读 Container／Pipeline／ServerStorage 路径未见内建的请求代次发布隔离；throttle 仅控制启动频率，不保证旧请求不覆盖新请求。Yue 延续全部完成路径核对代次、同批接受记录／总数和失败保留旧视图的规则，不照搬远程加载框架。

## 4. 总数、空结果与失败不能用 truthiness 混合

证据：[tabular.ts](../../refer/DataTable-Libs/gridjs/src/tabular.ts)、[ArrayToTabular](../../refer/DataTable-Libs/gridjs/src/pipeline/transformer/arrayToTabular.ts)、[Pagination](../../refer/DataTable-Libs/gridjs/src/view/plugin/pagination.tsx)、[view/actions.ts](../../refer/DataTable-Libs/gridjs/src/view/actions.ts)。

Tabular 用 length 同时承载本地行数与远程 total，getter 为 `_length || rows.length`；因此代码未显式区分“总数为 0”与“未提供总数”。Pagination 在本地 Limit 前取总数、远程在 pipeline 完成后取总数，这个阶段区别值得借鉴；但其 error 处理将页码／total 归零，SetDataErrored 将 data 清空，不符合 Yue 默认刷新失败保留旧内容的合同。

Yue 用不同语义承载结果域总数、已加载数、投影数和工作集数。Known(0) 与 Unknown 分开；未知总数不退成当前页长度，不塞入只支持已知 total 的 YuePagination。宿主响应里 total 与当前页相矛盾时先校验，不静默用 truthiness 替换；加载／失败也不能把上一份有效 total 擅自清成 0。这里只补既有计数与接受规则的边界，不增加新的 Pagination 模式。

## 5. 文本匹配按值与字段范围，不搜索控件树

证据：[operator/search.ts](../../refer/DataTable-Libs/gridjs/src/operator/search.ts)、[GlobalSearchFilter](../../refer/DataTable-Libs/gridjs/src/pipeline/filter/globalSearch.ts)、[TD](../../refer/DataTable-Libs/gridjs/src/view/table/td.tsx)。

Grid.js 会转义正则特殊字符再匹配，支持自定义 selector 和是否忽略 hidden 列；formatter 在 TD 展示时执行，普通匹配从 cell.data 读取。这说明查询值应与呈现分开，搜索是否覆盖隐藏字段也必须有明确口径。其 HTMLContent 分支仍会读整个 HTML 字符串，源码 TODO 也说明尚未只抽取可见内容。

Yue 内置文本筛选默认按字面匹配，不把用户的 `[]`、`.`、`*`、反斜杠当作正则；大小写／空白处理随列声明，0／false 不当空值。使用 typed 原始／查询值，不遍历 renderer、图标名称、Tooltip 或控件文本反推。宿主组合跨字段搜索时，明确授权字段 key 范围；响应式暂时隐藏列不自动重写已接受 query，权限变化则按新规则重验。此处是已有文本筛选和宿主组合的合同，不另立全局搜索引擎。

[NativeSort](../../refer/DataTable-Libs/gridjs/src/pipeline/sort/native.ts) 复制行引用数组后排序，多字段依次比较，避免改变上游数组；内部还使用位运算累计数值 comparator 结果。Yue 保持稳定排序、typed 比较与显式空值规则，不移植 JS 数值／位运算语义或索引字段映射。

## 6. 去抖、卸载与配置更新：调度只是临时工作

证据：[Search](../../refer/DataTable-Libs/gridjs/src/view/plugin/search/search.tsx)、[debounce](../../refer/DataTable-Libs/gridjs/src/util/debounce.ts)、[throttle](../../refer/DataTable-Libs/gridjs/src/util/throttle.ts)、[Container](../../refer/DataTable-Libs/gridjs/src/view/container.tsx)。

远程搜索默认去抖，本地输入则无同样等待。Search 的延后回调保留 event 并读取 target.value；debounce 只返回调用包装器，没有取消／完成接口，被覆盖的 Promise 也未见结算；throttle 同样未暴露取消。Container cleanup 解除 pipeline updated 监听，不能据此推导全部延后任务与进行中的请求都已结束。

Yue 不默认给筛选添加固定延时。宿主如为文本／候选请求使用去抖，应在输入事件时保存 typed 值及 owner／草稿代次，禁止延后读取可变事件或旧 Input。确认采用当前合法草稿并使旧调度失效；清除／重置产生的最新意图不能被旧词再次覆盖。取消、失权、换来源、卸载只撤销所属未提交工作；已经接受的 query 请求仍由宿主按 query／请求代次结算，面板关闭不冒充服务器已取消。沿现有 Task 与会话，不引入公共 debounce 框架。

证据：[grid.ts](../../refer/DataTable-Libs/gridjs/src/grid.ts)、[config.ts](../../refer/DataTable-Libs/gridjs/src/config.ts)。updateConfig 重建配置相关部件，forceRender 会 destroy、清缓存并卸载后重挂载。Yue 接受数据／列／主题更新仍保留合法 state、editor 和滚动锚点，不把常规更新做成重建整表；显式卸载再挂载才走 mount 代次与资源清理。

## 7. 尺寸、选择与扩展能力的取舍

[Header.adjustWidth](../../refer/DataTable-Libs/gridjs/src/header.ts) 的注释提及少量样本，但实际 [ShadowTable](../../refer/DataTable-Libs/gridjs/src/view/table/shadow.tsx) 会 clone 当前整张 table，再让浏览器测量；它没有在该路径中截取“前 10 行”。Yue 以实际源码为准，显式样本测宽记录样本 key／预算和内容版本，不克隆整张可操作树、不自动扫描全部数据，也不把样本最宽称为全局最宽。

[Resize](../../refer/DataTable-Libs/gridjs/src/view/plugin/resize/resize.tsx) 使用文档级 mouse／touch 监听、节流后直接写表头 style；end 解除监听，但未见取消已排队 move 的接口。Yue 延续 Base ResizeHandle、共享几何、最终坐标结算及 Escape／失捕获／卸载撤销，不采用 DOM 写宽或固定 10ms 拖动节流。

[selection 插件](../../refer/DataTable-Libs/gridjs/plugins/selection/src/rowSelection/rowSelection.tsx) 将 rowIds 放 store，同时把 checked 写入 cell.data，并改祖先 TR class。Yue 保持唯一已接受选择事实与现有 Checkbox，不将选择状态塞入业务值、依赖祖先树结构或临时生成的 Row ID。插件机制可说明独立职责，但不成为建立公共动态插件系统的依据。

## 8. 实际阅读的测试断言及限制

下表均为源码阅读，**未执行**。区分纯规则、JS DOM／Preact 与浏览器测试，不能将快照或 axe 输出视为 GPUI 实际几何、性能或 Windows UIA 通过。

| 参考测试 | 已读断言 | Yue 的验收落点与不足 |
| --- | --- | --- |
| [pipeline.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/pipeline/pipeline.test.ts) | 注册／移除、优先级、props 通知、连续三次命中缓存、clearCache 后重算、末尾新增阶段只执行新阶段 | 记录阶段次数及结果身份；另补热缓存移除中间条件、执行中再失效，不能仅检查注册数量 |
| [storage/server.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/storage/server.test.ts) | fetch 次数／参数、then 行转换、total 转换、自定义 data 替换 fetch | 宿主映射后的完整 query／结果／总数；补拒绝、倒序响应及取消，不把此组成功路径当竞态证明 |
| [storageResponseToArray.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/pipeline/transformer/storageResponseToArray.test.ts) | 数组／对象转换、静态展示数据、自定义字段读取与 total 保留 | 按 key 的字段适配与显示列变化不串值；Yue 不全量复制 Cell，也不依赖标题猜字段 |
| [operator/search.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/operator/search.test.ts) | 精确／部分词、空格、特殊符号、selector、隐藏字段包含／排除 | 字面文本匹配与字段范围；再补正则元字符、中文及 0／false；不复制 HTML 字符串搜索 |
| [sort/native.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/pipeline/sort/native.test.ts) | 字符串、数值含 0 的升降序、多字段优先级、非法列位置抛错 | 排序结果、行字段不拆散、稳定性与上游不变；非法 key 用 Yue 既有类型化拒绝，不等运行时数组越界 |
| [limit/pagination.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/pipeline/limit/pagination.test.ts) | 五行以两行为一页，首／中页两条、末页一条及具体内容 | 过滤后总数与页内 key 边界；另补 0／Unknown／失败保留和 Remote 不重复分页 |
| [search/search.test.tsx](../../refer/DataTable-Libs/gridjs/tests/jest/view/plugin/search/search.test.tsx) | 初始词、无词不发 action、模拟输入后发词、class／快照；输入断言用了 100ms setTimeout | 复用输入接线思路，使用可控任务调度覆盖快速替换／清除／取消／卸载，不用任意延时掩盖状态问题 |
| [container.test.tsx](../../refer/DataTable-Libs/gridjs/tests/jest/view/container.test.tsx) | 正常／空／错误快照、axe；卸载后 instance eventEmitter 和 store 监听为空；注册／注销调用数相等 | 生命周期联合检查 pipeline／任务／业务订阅等真实来源；原断言未等同检查 pipeline 全部监听或迟到任务 |
| [grid.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/grid.test.ts) | Loading → Data → Rendered action 次序，各一次；配置／存储初始化 | 已接受结果与 busy 结算的一致性；不照搬 setTimeout(0) 的 rendered 状态或推导真实绘制完成 |
| [util/throttle.test.ts](../../refer/DataTable-Libs/gridjs/tests/jest/util/throttle.test.ts) | 尾缘调用、最新参数及多次间隔调用；部分 sleep 调用未 await | 仅作调度思路，不能证明异步串行、卸载取消或最后 move 结算 |
| [selection actions](../../refer/DataTable-Libs/gridjs/plugins/selection/tests/rowSelection/actions.test.ts)、[Cypress table.spec.ts](../../refer/DataTable-Libs/gridjs/tests/cypress/integration/table.spec.ts) | 选择 action 增删 rowIds；浏览器只断言存在一张 table | 不当作跨页选择、键盘、虚拟化或真实布局覆盖 |

本轮未逐条阅读全部测试；分页组件测试仅检索断言范围，不作为已完整审阅的用例列入上表。

## 9. 吸收清单与下一项

| 编号 | 补充的合同／回归 | 阶段 |
| --- | --- | --- |
| GJ01 | 热缓存移除筛选／排序、异步执行中改条件，结果与缓存按最新版本接受，不吞下一轮失效 | M1／M3 |
| GJ02 | 宿主将完整 query 一次映射到业务查询字段，重排／隐藏／操作列变化不串字段 | M3 |
| GJ03 | Known(0)、Unknown、当前页数和投影数分开；响应矛盾校验、失败不清旧 total | M3 |
| GJ04 | 文本按字面／typed 值与明确字段范围匹配；不搜索控件树，不由临时隐藏列暗改 query | M3 |
| GJ05 | 去抖的最新值、确认、清除、取消和卸载；只撤销所属工作，旧调度不复活查询 | M3 |
| GJ06 | 配置更新保合法 state／焦点／锚点；卸载检查所有实际监听／任务，不止注销次数 | M1–M4 |
| GJ07 | 显式样本测宽守预算、稳定排序不改上游、小型纯规则用例与真实 GPUI 接线分层 | M1／M2／M3 |

保持 M1–M6 既有范围。本轮重点补查询、缓存与生命周期的边界测试，没有增加 Web 插件系统、数据获取器、全局搜索引擎或新的尺寸测量平台。

下一项先调研 `refer/DataTable-Libs/SlickGrid`，再单独调研 `slickgrid-universal`，不将两个仓库合成一份能力／版本结论。本轮尚未开展这两项。

后续进展：SlickGrid 6.0.0 已完成，见[第 9 项专项记录](2026-11-10-table-slickgrid-review.md)；Slickgrid-Universal 仍待单独调研。上段保留该轮结束时的范围，当前进度见主设计第 2.2 节。
