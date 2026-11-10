# YueTable 源码调研 03：Glide Data Grid

日期：2026-11-10。状态：本地源码、API 文档和定向测试代码阅读完成，结论已补入 [YueTable 设计](../45-table.md)。本轮仅调研与设计，没有实现 Table、安装 JS 依赖、运行参考测试或测量性能。

上一项：[TanStack Table](2026-11-10-table-tanstack-review.md)。按用户要求一次一个项目，本轮不评价其他参考库。

## 1. 快照与实际阅读范围

| 项目 | 核对结果 |
| --- | --- |
| 目录 | `refer/DataTable-Libs/glide-data-grid`，独立 Git 仓库，阅读时工作区干净 |
| HEAD | `0875d78cc41535a39f8b417ff88ce1ebe129c363` |
| 包版本 | [core/package.json](../../refer/DataTable-Libs/glide-data-grid/packages/core/package.json)：`@glideapps/glide-data-grid` **6.0.4-alpha25**；不是介绍文档所写稳定版 6.0.3 |
| 许可 | [LICENSE](../../refer/DataTable-Libs/glide-data-grid/LICENSE)：MIT，2021 typeguard, Inc. |
| 架构 | React 管状态、输入和编辑浮层，Canvas 绘制表格；core 依赖 Linaria、canvas-hypertxt、react-number-format 等 |
| 测试环境 | [vitest.config.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/vitest.config.ts) 使用 jsdom；[vitest.setup.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/vitest.setup.ts) 导入 vitest-canvas-mock，模拟 ResizeObserver 和图片 decode |

阅读主线：DataEditor → DataGrid → drawGrid／drawCells；damage、动画队列、窗口化资源、文本测量、批量取值、编辑结束与定位；以及相应测试的具体断言。没有穷尽所有 cell renderer、浏览器兼容分支、搜索／复制／填充和 Storybook。

通过 Context7 查询 `/glideapps/glide-data-grid` 的 `getCellContent`、`updateCells` 和 `getCellsForSelection`，交叉核对按需取值、局部更新和离屏批量读取的区别。在线返回的 main API 不当作本地 alpha 快照的发布证明，源码与本地测试是本轮证据。

## 2. 按需读取，不等于读取回调只能被绘制调用

证据：[data-editor.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/data-editor/data-editor.tsx) 的 `getCellContent`／`getMangledCellContent`、[data-grid-render.cells.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/render/data-grid-render.cells.ts) 的 `drawCells`、[use-cells-for-selection.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/data-editor/use-cells-for-selection.ts)。

`drawCells` 遍历有效列和视口行，先判断 damage／绘制区域，再读取 cell 内容；不先构造整张表的单元格对象。输入、导航、编辑以及其他功能也会调用取值，调用次数与绘制次数不是同一概念。

`getCellsForSelection=true` 会逐格读取请求矩形，可覆盖屏外数据。自定义回调还可返回异步 thunk；它是显式批量操作，不能把“当前仅渲染窗口”误写成“任何数据读取都不会访问屏外”。行标记列会经过坐标转换。

Yue 吸收：按 key 读取已接受的数据，无业务副作用、无网络发起，不假设一次调用。renderer、查询 typed accessor、复制文本载荷各有职责；Rows 路径仍可能访问工作行的全部显示列，Cells 才按两轴工作集构建。按字段／key 统计业务取值、格式化、Entity factory、Render、布局和 paint，避免只靠一个 renderer 计数证明优化。

远程数据预取沿用范围事件与宿主任务。将来宿主组合复制或内容宽度估算时，显式声明数据范围、数量预算、权限列与完整性；异步取消和版本检查独立处理。没有启用的能力不注册隐式全表读取；本轮不新增复制／导出或自动遍历全表的列宽自适应功能。

## 3. damage 的价值：把变化定位到格子，而非重建取值入口

证据：[data-editor.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/data-editor/data-editor.tsx) 的 `updateCells`、[data-grid.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/data-grid.tsx) 的 `damageInternal`／`damage`、[cell-set.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/cell-set.ts)、[data-grid-render.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/render/data-grid-render.ts) 的 damage 分支。

`updateCells` 把公开列位置转换成包含行标记的内部位置，再交给 damage；`CellSet` 去重。绘制路径对损伤位置取交集，屏外更新可不读取 cell。直接 imperative damage 会触发绘制，不是所有更新入口都自动进入动画合帧队列。

Yue 不复制坐标数组公共入口，也不允许仅通知“重画一下”却不推进来源版本。接受事务同时发布新数据 revision 和稳定 `TableCellKey` 变化集合；不知道精确范围时默认完整内容失效。按最新投影反查位置，旧的行号／列号不得命中新对象。局部提示不能推翻数据查询依赖：涉及筛选／排序／summary 或无法证明不相关时，仍从正确阶段重算。

同一帧内按 key 合并失效；与视口、冻结区、精确保留 cell 的去重并集取交。删除或当前未显示的 key 不构建屏外元素；以后进入工作集从最新来源读取，不能因为没有绘制而漏掉更新。队列有容量上限，超量退为完整内容失效，不能永久积攒整个数据集的 dirty key。

内容变化还要覆盖相关装饰、合并 origin／区域和受影响的 Measured 行；列轨道、主题、字体、密度或范围变化走各自的几何／视觉失效，不能只重画格内文字留下旧分隔线或冻结底板。

参考的提前判断区域包含常量 `height: 300`；文本测量、资源清理也有各自的固定阈值。Yue 依据实际 Base snapshot、裁剪区和配置预算判断，不复制这些数字或 JavaScript 数字打包坐标方案。

## 4. GPUI 中能复用什么，哪些尚不存在

| 本项目源码 | 当前事实 | 设计落实 |
| --- | --- | --- |
| [grid.rs](../../vendor/gpui-base/src/virtualization/grid.rs) 的 `new_with_cell_views` | 按稳定行列 key 和内容 stamp 复用 factory 结果；帧末清理未触及项；child Entity 自身 Render／布局／paint 另有生命周期 | 复用现有工作集与视图缓存，不能把 factory 未调用当成 child 没有重绘 |
| 同文件 `retained_cells`／snapshot | 精确保留 cell 与工作集去重，不扩大为两轴保留的乘积 | 编辑等少量保留项继续走原路径 |
| [state.rs](../../vendor/gpui-base/src/virtualization/state.rs) 的 `invalidate_items` | 有轴范围内容失效；Grid 有整体 content revision；未发现直接等价 Glide 的逐 cell damage API | Table 内部可按 key 更新内容模型／已有 Entity；没有独立 cell 内容 Entity 时可退为受影响行更新。先测试，再判断是否需要最小 Base 扩展 |
| [text_system.rs](../../vendor/gpui-pre/src/text_system.rs) | 已有 line layout、shape 与帧间复用 | 复用真实 GPUI 文本测量，不新建 Canvas 式文本引擎 |
| [image_cache.rs](../../vendor/gpui-pre/src/elements/image_cache.rs) | 有 ImageCache、ImageCacheItem、view 完成订阅与 drop_image；RetainAllImageCache 一直保留至显式移除／清空或释放 | 复用接口，按具体大量图片场景补有界策略；不能因注释提到 LRU 就称 retain-all 已有容量淘汰 |

整体 `content_revision` 或轴内容失效会使相应 factory stamp 改变，不能一边每次推进全局内容代次，一边宣称所有未改格都命中缓存。业务数据版本和视图构造失效范围须正确桥接；复用中的 Entity 通过正常更新读到新值，或在没有细粒度路径时诚实采用较宽失效。editor Entity 仍由宿主保留，wrapper 重新构建不能重置草稿。

这里约定的是按需取值、缓存失效和任务成本；不承诺 GPUI 只提交对应矩形或只绘制一个 cell。要证明实际绘制收益，须另测 GPUI scene、布局／paint 和帧耗时。本轮不增加第二套 renderer、不改 GPUI 合成器。

## 5. 动画按需续帧，结束就停

证据：[use-animation-queue.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/use-animation-queue.ts)、[data-grid-render.cells.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/render/data-grid-render.cells.ts) 的 `drawCell`。

队列在从空变为非空时申请帧，同一位置去重；执行前取出并清空，绘制中只有仍需动画的 cell 才重新入队。renderer 的请求与更新高亮可驱动下一帧；队列空时没有永久循环。其数组 includes 去重、连续 600 帧后隔帧等实现不作为 Yue 默认值，也不据此认定卸载清理已完备。

Yue 沿用 GPUI 帧调度和 motion 策略，不增加默认“内容更新闪烁”效果。只有存在真实过渡、活动加载控件或 Base 滚动命令时才续帧；重入的新任务不能在清空旧批次时丢失。动画完成、取消、失去使用者或卸载后停止；reduced motion 下直接落到目标状态。静态空闲、有限过渡和持续 loading 分开测，不能把合法 Spinner 当空闲失败。

## 6. 图片共享与窗口化要同时考虑使用者、字节和晚到结果

证据：[image-window-loader.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/common/image-window-loader.ts)、[render-state-provider.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/common/render-state-provider.ts)、`drawGrid` 尾部 `imageLoader.setWindow(...)`。

参考按 URL 共享一次加载，记录使用该图片的 cell；窗口变化保留可见区、冻结列和尾部冻结行中的使用者，清理无使用者项目。成功 load／decode 后仅通知关联位置，通知经 throttle 合并；cancel 标记隔离已经淘汰的请求，图片对象池最多 12 个。

值得吸收的是“共享资源 → 当前使用者 → 完成后通知”与退出窗口清理。12 个对象池不等于限制解码字节或 GPU 内存；URL 本身也不代表内容永远不变。错误路径的 undefined 不等于完整错误／重试产品合同，不能原样照搬为原生资源管理器。

Yue 的约定：

1. 普通文字表不创建图片资源管理器。大量 Avatar／缩略图场景优先使用已有 GPUI ImageCache 接口与异步设施，缺少有界策略时在资源层提供最小适配，不另写网络／解码管线。
2. 使用者按当前实际 cell 工作集、冻结区域和必要编辑保留项的并集计，不用首尾两个行号包住所有位置；固定区存在不代表保留两行之间的全部图片。
3. 来源身份包含资源自身版本，不能只用 row index 或可变 URL。共享给其他格／视图的资源仍有有效使用者时不被其中一个格的卸载误取消；LRU 热缓存须有明确容量和解码字节预算。
4. 来源替换、淘汰、失败和卸载均结算任务／订阅。晚到成功与失败检查资源代次和 owner；丢失使用者后不回写错误格，不无界重试、不维持空转帧。加载状态使用占位和已有 Avatar 回退能力。
5. 释放遵守 ImageCache 的窗口纹理移除合同；不能只删 HashMap 却留下窗口资源。CPU 解码字节、缓存条目和 GPU 占用分别报告，不将 `drop_image` 调用等同于已测得显存立即下降。

[ImagePreviewLimits](../../crates/yue-component/src/image_preview/resource.rs) 是已有单图片处理预算，不是 Table 的全部图片总预算；也不把 Showcase 的磁盘缩略图示例直接提升为公共 Table 依赖。

## 7. 文本缓存和热路径：复用已有能力，先量成本

证据：[data-grid-lib.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid/render/data-grid-lib.ts) 的 `measureTextCached`／`clearCacheOnLoad`、`drawCells` 的区域过滤与可复用参数。

参考测量 key 含文本、字体和 baseline；字体 ready 后清理旧值，超过 10,000 项时整批清空。绘制热循环避免逐格临时区域对象与闭包，以降低 JS GC 压力。这些是具体实现机制，不能凭注释认定 Rust 也有同等瓶颈。

Yue 使用 GPUI shaped text 与 OverflowTooltip；若额外缓存格式化结果或格布局，依赖至少覆盖内容／列语义、字体／字号／rem、实际列宽、padding、换行策略及密度，缓存有界。颜色变更不必重做 shaping，但仍应更新绘制。不同 DPI 下先遵循 GPUI 逻辑／物理像素合同，不用一项全局 DPI 数值粗暴清空所有业务缓存。

先用长中文、数值、URL、自定义复杂 cell 的采样确定分配来源，再考虑复用临时容器、保留不可变快照或减少格式化。保留可读性，不复制 Canvas 像素 blit、双缓冲、字体度量常量和数组手工打包。

## 8. 编辑覆层借鉴协调，不复制默认交互

证据：[data-grid-overlay-editor.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid-overlay-editor/data-grid-overlay-editor.tsx)、[use-stay-on-screen.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/src/internal/data-grid-overlay-editor/use-stay-on-screen.ts)。

Canvas 显示格与真实 DOM editor 分离；editor 保持临时值、校验和结束标记，协调 Enter／Escape／Tab 与自定义 editor 的完成回调。定位 hook 监测屏幕边界并做横向偏移，离屏时可通过 rAF 持续校正；这不等于完整的多方向遮挡和多显示器策略。

Yue 已有真实 Input／Select Entity、Popup 和窗口 Root，直接复用。对同一会话的按键、失焦和子 editor 完成回调只结算一次，旧回调携带会话身份不得结束新会话。定位跟随共享轨道、当前 scroll owner 和 anchor revision；几何未变不为了 clamp 每帧发状态更新。

Yue 保持原设计：默认失焦不自动保存，校验失败保留草稿且不前进，IME 优先。参考中的 click-outside 尝试提交、无效结果转 undefined 再结束与延迟 setTimeout 不是需要照搬的交互；单元格编辑只保留一个真实 editor，行编辑按明确的有限控件数处理。这里不新增另一个 Table 私有弹层系统。

## 9. 实际测试证据与 Yue 验收映射

以下均已阅读具体断言，**没有执行**。jsdom、fake timers、mock Canvas 和 mock decode 的结果范围不能扩展为浏览器真实帧率、GPUI 原生焦点、IME 或 GPU 回收证明。

| 参考测试 | 读到的断言 | Yue 应采用／补足的验收 |
| --- | --- | --- |
| [data-grid.test.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/test/data-grid.test.tsx) 的 Simple damage／Out of bounds damage | 可见 damage 后取值有调用，远处 `[1,900]` 后无调用；没有断言只绘制一格 | 测精确 key 内容更新、离屏不构建、再次进入取到新值；另数实际 Entity、布局和 paint |
| [data-editor.test.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/test/data-editor.test.tsx) 的 Imperative damage gets right cell | 开行标记后 updateCells 仍读公开 `[1,0]` | 操作列、hide／pin／reorder／排序后映射正确；变更按 stable key，不依赖旧坐标 |
| 同文件 Emits finished editing／Does not edit when validation fails | Enter 完成回调带新值和移动方向；校验失败不调用 onCellEdited | 额外断言草稿／焦点仍保留、不前进；IME、重复完成和旧会话回调均走真实控件 |
| 同文件 Copy/paste with simple getCellsForSelection | 范围选择后复制读取矩形并写预期制表符文本 | 仅借鉴显式读取范围；不为本轮新增电子表格复制／粘贴；未来宿主组合须测试预算与取消 |
| [use-animation-queue.test.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/test/use-animation-queue.test.ts) | 三个 enqueue 合到 draw 的 CellSet 参数 | 补测重复 key 去重、执行中再入队不丢、完成后零续帧与卸载取消；该测试没覆盖这些出口 |
| [render-state-provider.test.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/test/render-state-provider.test.ts) | 坐标 pack／unpack，冻结列内可见行保留、窗口外项目移除 | 使用 Rust 结构化 key；测不连续工作集、尾冻结、稀疏编辑保留，无大矩形扩张 |
| [image-window-loader.test.ts](../../refer/DataTable-Libs/glide-data-grid/packages/core/test/image-window-loader.test.ts) | 窗口变更触发清理、相同窗口不重复清理、缓存／加载中命中、模拟 decode 成功／失败 | 补真实使用者共享、旧请求晚到、失败重试边界、字节预算、纹理释放；现有断言没有证明全部竞态 |
| [data-grid-overlay.test.tsx](../../refer/DataTable-Libs/glide-data-grid/packages/core/test/data-grid-overlay.test.tsx) | 多个 editor 的 smoke test 只挂载，没有交互断言 | 不能拿 smoke 当编辑验收；验证真实 bounds、遮挡、焦点、Escape、选择与会话 |

建议针对同一数据快照建立三组更新 workload：一个可见格、一个离屏格、密集一批格。再叠加排序依赖变化、重复 key 通知、Measured 行和冻结／合并。分别比较取值／factory／布局／paint／分配，不把“通知变少”当作性能最终结果。

## 10. 吸收清单与下一项

| 编号 | 决定 | 阶段 |
| --- | --- | --- |
| GD01 | 同一接受事务发布内容版本与稳定 key 变化范围，缺失精确信息保守全量失效 | M1；M5 验证双轴路径 |
| GD02 | 合并稀疏失效、最新投影映射、工作集交集和有界退化；查询／测量依赖不能跳过 | M1／M3／M4／M5 |
| GD03 | 复用 Base cell views；区别 factory、Entity Render 与实际绘制；不足先测再补 Base | M1／M5 |
| GD04 | 空队列不续帧、再入队不丢、卸载结算；不默认新增更新动画 | 各阶段 |
| GD05 | 图片共享使用者、来源代次、容量／字节预算和窗口纹理释放 | M1 有大量图片的场景；M4／M5 扩展组合验收 |
| GD06 | 复用 GPUI 文本缓存；完整布局依赖与有界策略；热路径优化以采样为据 | M1／M2／M4／M5 |
| GD07 | 普通读取无业务副作用；屏外批量读取显式范围和预算 | M1／M3／M5；复制等仍属宿主组合 |
| GD08 | 编辑多来源完成只结算一次、旧会话隔离、浮层随合法几何变化定位 | M4 |

不吸收 Canvas 第二 renderer、像素搬运／双缓冲、Web 编辑器、JS 数字坐标压缩、固定缓存常量和为功能数量增加的填充／电子表格能力。Canvas 的高吞吐宣传不能直接转换成 GPUI 性能承诺。

下一项为 `refer/DataTable-Libs/revogrid`（RevoGrid，具体本地目录和版本下一轮核对），重点应看分区／列行虚拟化、尺寸索引、数据与视口协调及编辑生命周期。本轮尚未开展该项源码调研。

后续进展：第四项已完成，见 [RevoGrid 专项记录](2026-11-10-table-revogrid-review.md)；上段保留为本轮结束时的调研边界。
