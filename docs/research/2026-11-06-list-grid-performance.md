# List Grid 性能探针（2026-11-06）

## 条件

Release 构建；headless GPUI `Window::draw`，600px 宽、320px 高；每个场景预热后采样 3 轮，每轮 6 次绘制。统计主线程绘制耗时、采样期间分配次数、峰值 live bytes 和本次绘制创建的卡片数。该探针没有 Windows GPU/合成器，因此不代表实际屏幕帧率。

## 结果

| path | count | ns/draw | alloc/draw | peak bytes | cards/draw |
|---|---:|---:|---:|---:|---:|
| ordinary grid | 1,000 | 76,877,316–76,963,316 | 325,175 | 9,178,232 | 1,000 |
| row virtual grid | 1,000 | 400,866–415,066 | 1,694 | 85,312 | 6 |
| row virtual grid | 10,000 | 400,166–416,450 | 1,698 | 85,472 | 6 |

命令：

```text
cargo test -p yue-component --release --locked --features headless-tests --test list_grid ordinary_and_virtual_card_cpu_evidence -- --ignored --nocapture --test-threads=1
```

普通集合的工作量随数据量增长；按行虚拟路径的挂载卡片数和绘制时间由视口工作集决定，1,000 到 10,000 项没有线性增长。这个结果支持采用按行适配，但不证明所有业务 renderer 都达到同样数据；仍需在真实窗口中关注文本、图像和子控件成本。
