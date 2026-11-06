# List 示例页滚动修复（2026-11-06）

用户反馈 List 页滚动明显卡顿。此前的 ListGrid 探针只测单独的普通/虚拟网格，没有覆盖整页嵌套布局和外层滚动，不能用它排除示例页的问题。

## 原因与修复

1. `YueList` 集合外壳、集合体和分组，以及示例 section，使用了多层纵向 Flex。它们仅需按内容自然上下排列，却在整页重新布局时触发重复的自然尺寸计算。将这些容器改为原生 block 流布局；横向集合、条目槽位和带 gap 的内容继续使用 Flex，卡片分列继续使用原生 Grid。section 的原间距通过明确的相邻间距保留。
2. 示例页的两个内嵌虚拟视口没有隔离滚轮冒泡，同一个事件会同时滚动虚拟列表和外层页面。在有限高度的示例边界中，先让子级处理滚轮，再阻止它继续传给外层页。未改变 Base/GPUI 的滚动引擎，也没有创建额外滚动状态。

内外层滚动回归测试先失败：向内层发送 `-120px` 滚轮后，外层内容 top 从 `-2400px` 变成 `-2520px`。修复后外层 bounds 保持不变，内层已有 virtualizer 的 offset 增长。

## 前后测试

对比基线为 `249236f`，仅向基线移植同一个 ignored 性能探针。自动脚本使用 `try/finally` 恢复最终源码，分别构建并运行 Debug/Release 基线和修复版本。窗口为 1200×800px，通过生产 `overflow_y_scrollbar()` 包装完整 `ListPage`，启用 reduced motion，预热两次，再采集 24 个滚动位置。每次在右侧页内留白发送 `-120px` 滚轮，确保测量外层页面滚动。计时只包含 `Window::draw().clear()`，快照读取在计时结束后进行；强制 refresh 覆盖完整布局重绘成本。

| 构建 | 修复前中位数 | 修复后中位数 | 耗时降低 | 修复前范围 | 修复后范围 |
|---|---:|---:|---:|---:|---:|
| Debug | 112.602 ms | 34.008 ms | 69.8% | 106.973–124.439 ms | 32.179–43.183 ms |
| Release | 11.511 ms | 3.953 ms | 65.7% | 11.196–14.413 ms | 3.838–6.027 ms |

初始两处虚拟视口仍分别为 352px / 288px 高，普通数据和一万项源均未减少。原始 96 条记录包含每个位置的耗时、观测节点数和两处虚拟视口 bounds，见 [CSV](2026-11-06-list-page-scroll.csv)。

这证明本机无头 CPU 绘制成本下降；不代表 Windows GPU、合成器帧率或实窗最终验收。Debug 构建仍明显慢于 Release。

复测命令：

```text
cargo test -p windows-showcase --locked --features headless-tests list_page_scroll_cpu_evidence -- --ignored --nocapture --test-threads=1
cargo test -p windows-showcase --release --locked --features headless-tests list_page_scroll_cpu_evidence -- --ignored --nocapture --test-threads=1
```

## 功能验收

- List 与 ListGrid 的 20 项现有布局/交互测试保留，覆盖自然换行、头尾/分组、横排、条目槽位、主题/rem、原生普通/虚拟网格、部分末行、阅读锚点与焦点。
- Showcase 增加真实 1200×800px 外层滚动测试，验证内容总尺寸不随滚动变化、虚拟示例有限高度保持正确、内层滚轮仅更新自己的 owner。
- 本轮没有修改 GPUI/Base、依赖或编译优化等级；没有继续先前被用户 Esc 中止的实窗自动化。
