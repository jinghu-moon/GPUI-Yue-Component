# Virtualization 五项后续实施

基线：`7fb2c29fcd2cdc08f50b86271111114cc260327d`。
用户授权：按下列五项逐项完成；先测试和比较，再决定吸收，独立提交并推送，保持可回退。
状态：第一至四项完成，第五项进行中（auto 轨道已验证，RTL 与内容测量试验继续）。证据见 docs/research/2026-11-05-virtual-list-five-followups.md。

## 固定约束

- Base 拥有行为、几何、焦点生命周期；应用拥有展示。一个原生滚动 owner，不缓存 AnyElement。
- `refer/virtual` 只读；生产 Cargo manifests/locks 不改。隔离基准 fixture 在 `C:/Temp`。
- 缺陷先建立失败测试；优化同一正确画面/工作量对比，两轮 Release ABBA，保留不利样本。
- 原生布局/交互证据与 headless CPU 分开，不声称 GPU FPS 或物理输入延迟。
- 新增能力按真实支持范围声明；不能用测全量数据或无界沿途范围实现虚拟化能力。
- 每项提交可单独审阅/回退；最后验证整轮逆序补丁，正常推送，不改写历史。

## 1. 冻结 Grid 组合回归与成本分析（完成）

覆盖连续尺寸、插入/删除/重排、冻结切换、视口缩放、程序命令与输入交错。
先定位必要工作量和可避免开销。保留既有正确阅读位置，任何性能方案须比较 cell/build/分配与 CPU，未获证据不采用。

## 2. 精确 cell 保留（完成）

由 row key + column key 表达稀疏保留，普通工作集仍由两轴决定；不额外制造整行整列乘积。
覆盖焦点编辑、重复 key 对、缺失/删除、重排、准备 View 复用与卸载，默认路径有守卫。

## 3. 显式跨格（完成）

先 Fixed/Known column span，再 row span。定义稳定 origin、跨度范围、重叠拒绝、离屏 origin 查询、冻结边界裁剪/拒绝规则和唯一命中。
不改变行高/列宽为自动推断，不扫描所有 cells；更新接口必须原子验证。

## 4. Measured 平滑定位（完成）

先短距离可测区域的动态尺寸校正与用户打断；长距离以有界跳转加末段 smooth 做独立试验。
目标 key/revision 可重求，采样从实际 owner 位置开始；未知/不稳定尺寸安全即时定位，reduced motion 和超时保持确定。

## 5. auto 轨道、内容自适应与 RTL（进行中）

逐项定义消费者和尺寸失效合同，完成实现/验证或记录实验不采用结论。
auto 不假装知道离屏内容最大值；RTL 使用逻辑 start/end，保证冻结、定位、wheel 和命中一致。
通用二维画布/三维空间不在本轮范围。

## 最终验证

专项和 workspace tests、fmt、严格 Clippy、vendor contracts、Windows 原生用例；新增展示按需挂载，必要时实窗视觉核对。
保存基准原始 CSV、依赖/源码身份、采用/拒绝理由及日志哈希，更新 docs/28-virtual-list.md 和 vendor 补丁说明。
逐提交回退 tree 验证，推送并核对远端 SHA、干净工作区。
