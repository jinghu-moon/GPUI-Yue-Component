# 19 YueTag 标签组件设计

状态：V1 已实现。创建及实施：2026-11-02。设计规格与当前公共 API 见正文，实现对应关系及验证范围见第 17 节；真实 UI 由用户验收。

依据用户提供的 [Tag 参考图](assets/tag-reference.png)、本仓库 `gpui-base` 源码，以及 Yue Design System 已有 Tag token 编写。参考图借鉴标签形态和组合方式；尺寸、颜色、焦点与交互遵循本项目规范，不按缩放截图估算像素。

## 1. 定位与范围

Tag 是承载短文本的紧凑标签，可表示分类、属性、筛选条件或轻量预设。默认不交互；需要时可执行一个动作，或切换一个受控选中值，并可附带独立的移除操作。

**Chip、可切换标签、可关闭标签归入同一个 Tag 家族**，不另做一套 Chip 外观。Tag 的形态不等于 Toggle 行为：静态标签没有按钮语义；动作标签复用 Base Button；可选标签复用 Base Toggle。

本次覆盖：五种语义色、四种外观、三档尺寸、三种形状、前置图标、可关闭、可选中、分组排列、长文本省略、自定义颜色，以及键盘、焦点和局部动效。

不包含自动补全、输入框内 token 编辑、拖拽排序、批量选择工具条、计数徽标或弹出菜单。添加标签由宿主组合现有 Input 和 Button；大量数据由列表组件承载，TagGroup 不承担虚拟化。

## 2. 参考图的采用方式

| 参考部分 | Yue 设计 |
| --- | --- |
| 基础标签、五种颜色 | `Default / Primary / Success / Warning / Danger` |
| 深色、浅色、描边、浅色描边 | `Filled / Tint / Outline / TintOutline`；Filled 指实色填充，不是暗色主题 |
| 带图标、可删除标签 | 一个可选前置图标、一个独立关闭按钮 |
| 可选择标签的 Style A / B / C | 分别使用 Filled / Tint / Outline，不引入三个含义不明的样式编号 |
| 可选择标签组 | 相同 Tag 的离散排列；单选或多选规则由宿主掌握 |
| 超长文本省略 | 只截断文字，保留图标和关闭区域；实际溢出才显示 Tooltip |
| 小、中、大；方、圆、标记 | 采用现有紧凑尺寸，支持 Rounded / Pill / Mark |
| 自定义颜色 | 显式颜色契约，保证文字和边界可辨识，不直接给任意背景配白字 |

不复制参考页的网页布局、配置侧栏、覆盖率徽章、API 页签、代码操作和贡献者列表。

## 3. 组成与布局

```text
┌─────────────────────────────────┐
│  [前置图标 / 选中标记]  标签文字  [×] │
└─────────────────────────────────┘
   └──── 主体区域 ────┘   └关闭区域┘
```

- 标签文字必填、单行，使用原始完整文本作为默认可访问名称；空白文本明确拒绝，不产生空按钮。
- 前置图标可选，复用现有 `Icon` 与 Tabler 图标资产；装饰图标不重复播报。
- 关闭按钮可选，最多一个，固定放在逻辑末端。Tag 不提供第二个尾部业务按钮。
- 外层拥有填充、边框、圆角和固定高度；内部横向排列、垂直居中，文字区域允许收缩。
- 主体和关闭区使用正常布局分配宽度，不把关闭图标绝对定位到文字上方。图标与文字之间使用 `--tag-gap`；关闭区自带正方形留白，不重复叠加一层 gap。
- 图标与关闭区域不收缩；只有文字使用 `min_w_0`、单行省略。固定宽度过小时，不压扁图标或令命中区重叠。
- 各外观始终预留同宽边框；无可见边框时使用透明边框，避免切换外观或选中时跳宽。

交互结构必须是非交互外层包裹两个**兄弟**区域：主体 Button / Toggle 与关闭 Button。静态可关闭标签的主体只是文本。禁止在主体 Button / Toggle 中嵌套关闭 Button，也禁止在外层监听整卡点击后依赖层层阻止冒泡修补。

## 4. 外观规格

### 4.1 语义色与外观

`TagTone` 为 `Default / Primary / Success / Warning / Danger`。普通元数据默认中性；颜色只表达已有含义，不为每一种文件类型或业务枚举随机分配状态色。

`TagVariant` 为 `Filled / Tint / Outline / TintOutline`，分别对应完整填充、浅色填充、描边、浅色填充加描边。静态和动作标签默认 `Default + Tint`；可选标签默认 `Primary + Tint`，未选中使用同外观的 Default 配色。

已有完整颜色矩阵，不新增重复颜色 token：

```text
--tag-{default|primary|success|warning|danger}-
      {filled|tint|outline|tint-outline}-
      {background|color|border-color}
```

实际 token 名连续书写，例如 `--tag-primary-tint-background`。颜色解析必须使用当前主题；不得只验证浅色 Azure。

### 4.2 尺寸

以下是源 token 在 16px rem 基准下的值，不是固定物理像素；实现通过现有 token 适配按应用 rem 缩放。

| `TagSize` | 高度 | 左右内边距 | 字号 |
| --- | ---: | ---: | ---: |
| Small | 20px | 6px | 11px |
| Medium，默认 | 24px | 8px | 12px |
| Large | 30px | 10px | 13px |

对应 `--tag-height-sm/md/lg`、`--tag-padding-inline-sm/md/lg`、`--tag-font-size-sm/md/lg`。Tag 已有独立的紧凑尺寸体系，不替换成 Button 的 28 / 32 / 40px 控件阶梯。

其余已有 token：`--tag-padding-block`、`--tag-gap`、`--tag-border-width`、`--tag-font-weight`、`--tag-line-height`。字体复用本项目字体配置，中文、英文、数字和混排均须在三档高度内居中且不裁字。

### 4.3 形状

| `TagShape` | 定义 | 来源 |
| --- | --- | --- |
| Rounded，默认 | 四角小圆角矩形；对应参考图的 square，不是强制直角 | `--tag-border-radius` |
| Pill | 两端圆头 | `--tag-border-radius-round`，实际半径受高度约束 |
| Mark | 起始侧圆头、末端直角；LTR 为左圆右直 | 起始侧复用 round 半径，末端为零；此规则是本设计选择 |

形状只改变外壳，不改变高度、文字基线、交互方式。Mark 不绘制突出三角，不借用 Tooltip 的箭头。关闭按钮的背景和焦点描边应尊重其所在端的轮廓。V1 遵循项目现有 LTR 布局，起始侧为左侧，不承诺自动镜像 RTL 轮廓。

### 4.4 图标与关闭区域

前置图标统一复用 `--tag-close-icon-size` 的 12px 基准，三档保持同一紧凑图标尺寸，避免增加一组未经需求验证的图标 token。图标随 rem 缩放，不能硬编码为不缩放的 12px。

关闭 glyph 同样来自 `--tag-close-icon-size`；**glyph 尺寸不等于点击尺寸**。关闭区域为与 Tag 等高的正方形，占用真实布局空间，命中范围不超出 Tag，不覆盖文字或相邻标签。复用 Base Button 行为，以 Tag 自己的几何呈现，不直接塞入高 28px 的 Small YueButton。

可关闭标签从主体末端移除外侧 inline padding，由关闭区自身提供末端留白，避免尾部双重留白。主体起始侧保持尺寸对应的 padding。不可关闭时恢复两侧对称 padding。

Small 的 20px 命中区仅用于紧凑桌面场景；以直接操作为主的标签优先 Medium 或 Large，不能以透明扩大命中区侵占邻居。

## 5. 三种行为与状态所有权

同一实例在一次构建中只有一种主体行为；使用语义构造器确定，不以 `clickable + checkable + selected` 三个布尔值拼接。

| 模式 | 主体职责 | 复用行为 | 受控状态 / 回调 |
| --- | --- | --- | --- |
| 静态 | 展示文本 | 普通文本布局 | 无主体回调，不进 Tab 顺序 |
| 动作 | 执行预设、打开路径等单次命令 | Base `Button` | `on_click` 请求动作，不隐式变为已选中 |
| 可选 | 切换筛选条件或开关选项 | Base `Toggle` | 传入 checked，`on_change(next)` 请求修改 |

Tag 用 `RenderOnce` 描述当前状态，不为每个标签新增业务 Entity。业务集合、选中值、添加输入草稿和异步操作由宿主持有。ElementId 使用稳定业务 ID，主体与关闭区分别派生子 ID；不能使用文字或可变数组下标作为身份。

`disabled(true)` 禁用主体和关闭操作，跳过键盘停靠，保留原来的选中值。禁用只影响可用性，不能顺带取消选择。静态标签可使用禁用外观，但仍不伪装成一个不可用按钮。

### 5.1 可选状态

未选中：同一 variant 的 Default token；选中：配置 tone 对应的 variant token。checkable 默认 Primary；不能仅调用 Base `Button::selected`，因为它不提供 Toggle 的按下语义。

| 场景 | 规则 |
| --- | --- |
| Filled / Tint / Outline | 分别覆盖参考图 Style A / B / C；TintOutline 同样可用 |
| 选中标记 | 未配置前置图标时，未选中不预留图标宽度和间隔；选中后展开 Check 与间隔，不在尾部补空白 |
| 自定义前置图标 | 未选中显示该图标，选中替换为 Check；沿用已有图标槽，不额外塞入第二个图标 |
| 鼠标经过 | 仅是 hover，不能修改 checked |
| 焦点 | 有独立焦点环，不能作为选中底色使用 |
| 禁用且选中 | 保留标记、配色和可访问的按下值，叠加一次禁用处理 |

未选中且没有自定义前置图标时，文字左右使用正常对称 padding，整体与普通文字 Tag 一样紧凑。选中后通过展开图标宽度和间隔、淡入勾选标记反馈状态；自然宽度随实际内容平滑变化，不以尾部占位补偿。允许文字和相邻标签随自然宽度调整，取消先前“始终保留标记槽、文字位置完全不变”的要求。有自定义前置图标时槽宽保持不变，仅切换图形。需要固定外框宽度的宿主可显式设置 `width`，文字区域相应收缩。

### 5.2 单选与多选组合

- 独立开关：每个 checked 单独受控，例如“复制路径”和“打开文件夹”。
- 多选筛选：宿主持有值集合，依据 `on_change` 增删对应 ID；返回相同值不重复触发业务提交。
- 单个生效的筛选：宿主持有 `Option<Value>`；选中新项替换旧项，再次点击当前项是否清空由宿主规则决定。回调只是请求，不保证宿主接受。
- 必选且互斥的表单选项优先现有 Radio；连续分段切换优先 Segmented。不能为了 Tag 外观伪造 RadioGroup 或 ListBox 语义。

组件不维护第二份选中集合。外部同步 checked 不反向调用 `on_change`；未收到新的受控值时不自行保持一个临时的选中结果。

## 6. 关闭与添加

`on_close` 表示请求移除此项。注册该回调才显示关闭按钮；不再设置一个独立且可能与回调矛盾的 `closable` 开关。不注册回调就没有关闭区。

1. 点击关闭或在关闭按钮上按 Enter / Space，只发出一次关闭请求。
2. 不触发主体 `on_click`，也不发出 `on_change(false)`；删除业务值时如何清理选择由宿主一起决定。
3. 宿主接受请求后更新集合并 `cx.notify()`；宿主拒绝或取消时，Tag 保持显示和原有状态。
4. 组件不修改调用方集合，不擅自追加 Toast、确认 Dialog 或执行撤销。
5. 全局 Backspace / Delete 不删除当前 Tag。没有显式的键盘关闭命令时，通过关闭按钮操作，避免误删和文本编辑冲突。

关闭按钮默认名称为“移除 {完整标签}”，可通过 `close_label` 覆盖并由宿主本地化；若实际含义是移出筛选、移除收藏，使用符合业务含义的动词。

**删除后的焦点由集合宿主处理**：仅当被删除项持有键盘焦点时，转移到下一项的可用主体或关闭按钮；没有下一项则前一项，再没有则回到“添加标签”按钮或宿主指定控件。鼠标删除未获焦点的项，不移动现有键盘焦点。宿主拒绝删除时不提前迁移焦点。

“添加标签”不是 Tag 的编辑模式：使用普通 Button 显示“添加标签”，激活后组合已有 Input；Enter 验证并提交，Escape 取消，空白或重复输入按业务规则反馈。若是在选择已有分类，组合 Dropdown，不在 Tag 内再实现一套菜单。

## 7. TagGroup 的边界

`YueTagGroup` 是小规模 Tag 的排列容器，默认按源顺序水平排列、自动换行、起始对齐，横纵间距均复用 `--space-8`。可配置不换行；需要水平滚动时由宿主提供唯一滚动容器。

- 不共享轮廓、不合并相邻边框；和 Segmented 的连续轨道明确区分。
- 不排序、不去重、不自动移除，也不推断单选 / 多选。传入子项仍是同一套受控 Tag。
- 不将普通元数据组标为 Toolbar、ListBox 或 RadioGroup；仅因外观相似就套用 Base ToggleGroup 会引入不正确语义。
- V1 各可用主体与关闭按钮按源顺序独立 Tab 停靠；组不拦截方向键、Home / End，不承诺 roving focus。
- 空组不生成占位 Tag，也不保留多余间隙。添加入口由宿主放在末尾的兄弟位置，不把任意交互控件塞进 Tag 内容。
- 换行、缩放和主题更新后保留基于业务 ID 的焦点；组负责排列，业务焦点恢复仍由宿主负责。

Base `ToggleGroup` 当前仅提供 Toolbar 角色、轴向语义及容器组合，**没有选中集合、互斥规则或方向键导航**。将来需要真正的 Tag 工具条时再补足整套行为；不能把现有容器当作这些能力已经存在。

## 8. 长文本与 Tooltip

Tag 默认自然宽度，受父容器约束；可配置 `max_width`。设定的宽度必须至少容纳边框、padding、保留图标、一个 em 的文字槽和关闭区，低于结构最小宽度时以该最小宽度为下限，由宿主解决溢出。Tag 的 `min_width` 接受绝对长度（Pixels / Rems），以便与结构最小值比较；父级相对约束使用 `width` / `max_width`。TagGroup 沿用通用布局接口。

文字在剩余宽度内单行省略，不换行为高标签，不把关闭按钮挤出卡片。完整可访问名称保留原文，不能把省略后的字符串写回业务模型。

启用溢出提示时复用现有 `YueOverflowTooltip`，根据实际字体排版结果决定是否打开，不能用字符数估算。未溢出不显示，disabled 不应因包裹 Tooltip 而重新获得激活能力。

交互标签把提示关联到真实主体的焦点；可关闭静态标签以完整关闭名称覆盖键盘读取，不额外制造一个“仅用于提示”的 Tab 停靠点。关闭区不作为展示全文的 hover 触发器，避免与“移除”含义混淆。定位、避免遮挡和入退场沿用现有 Tooltip / Popup。

## 9. 交互视觉与动效

动效服务于“这是可操作的”“选择已改变”，不让小标签表现为弹窗或通知卡片。使用 Base motion 和 Yue 现有时长 / 曲线；不创建组件自己的逐帧计时器。

| 变化 | 表现 | 时长 / 曲线 |
| --- | --- | --- |
| 静态标签 hover | 无可点击高亮，不缩放 | 无 |
| 可交互主体 hover / 离开 | 同一 tone 下背景或描边轻微变化；文字保持可读 | `--tag-duration` / `--tag-ease`，当前基准 100ms / standard |
| 按下 / 松开 | 更明确的表面反馈；不改变宽高，不整体位移 | 同一 interaction 时长，支持中途反向 |
| checked 改变 | 背景、文字、描边按 interaction 时长过渡；图标槽先展开，再沿短边、长边依次画出勾；取消时先擦除笔画，再收起图标槽 | 展开 100ms（`--tag-duration` / `--tag-ease`）→ 绘制 200ms（`--checkbox-duration` / `--checkbox-ease-enter`）；取消沿同一时间线反向，无弹跳 |
| 关闭区 hover / 按下 | 仅关闭区出现轻量底色，不点亮整个主体 | 同一 interaction 时长 |
| 键盘焦点 | 焦点环立即出现与消失，不等待动画 | 即时 |
| 禁用 / 主题切换 | 直接同步到新样式，不遗留 hover / active | 即时 |
| V1 增加 / 删除 | 宿主更新集合后正常布局；不保留不可交互的旧卡片 | 即时，不承诺退出或重排动画 |

### 9.1 状态优先级

先解析 tone + variant，再映射 checked，再处理 enabled 的 hover / active，最后绘制独立 focus ring；禁用会禁止 hover / active 分支并统一使用 `--tag-opacity-disabled`，只应用一次。关闭区与主体各自拥有 hover / pressed，不能互相触发。

颜色反馈从当前 variant 解析后的表面、文字色与已有 `--opacity-hover` / `--opacity-pressed` 派生；透明 outline 必须考虑所在父表面。`--tag-background-hover` 是已有通用默认别名，不可直接覆盖所有实色和状态色标签，否则 hover 会错误地跳回中性底色。

### 9.2 增删动画的明确取舍

V1 优先可靠的受控删除和焦点迁移，不为标签组引入额外的集合保留状态机。单个 `RenderOnce` 标签从树中移除后无法自行播放退出动画；不能在关闭请求时先淡出，再把宿主尚未同意删除的标签隐藏。

以后确需动画时，应由**持有集合的宿主**或专门的可动画集合层复用 Base `Presence` 保留退出快照：先确认数据删除、转移焦点和移除交互语义，再短暂绘制退出外观；剩余项的重排动画也由该层统一协调。单标签不自行缓存或复制业务集合。该能力不属于本次 V1 承诺。

系统 reduced motion 下，所有状态直接到达目标值，取消插值；不降低键盘、选择、关闭的响应速度。快速 hover 或连续切换从当前呈现值继续，不排队播放历史动画。

### 9.3 勾选笔画绘制

复用 Checkbox 的 `CHECK_LINE` 路径及 `paint_mark`，用圆头、圆角的矢量笔画逐步增加可见路径长度，而非淡入整张 Check 图标。绘制画布保持固定尺寸，不随图标槽展开缩放或重新居中。勾采用标签前景色；路径在原图标槽内做 `4/3` 光学放大，12px 槽中可见路径宽度由 6px 增至 8px，描边改用 Checkbox medium 的笔画 token（基准 1.5px）并随 rem 缩放。放大仅作用于笔画，不增加槽宽、左右内边距或未选中占位。

布局与绘制共享一条 Base transition 时间线：选中时先用 100ms 展开图标槽和间隔，随后用 200ms 绘制勾；取消时先用 200ms 反向擦除，再用 100ms 收起。这样擦除期间不裁掉尚未完成的笔画，快速反向也不会跳变或重播。已有自定义前置图标时，保留原有槽宽，仅在这一阶段淡出 / 恢复前置图标。初次渲染的已选中标签、禁用 / 主题 / 模式切换以及 reduced motion 直接呈现最终状态。

动画结束后，未选中且没有自定义前置图标的标签仍为零图标占位、零图标间隔，尾部不添加补偿空白。受控选择值、键盘反馈与回调不等待动画结束。

## 10. 键盘与原生可访问性

| 区域 | Tab | Enter / Space | 原生语义 |
| --- | --- | --- | --- |
| 静态主体 | 不停靠 | 无 | 文本，不伪造按钮角色 |
| 动作主体 | 可用时停靠 | 请求动作一次 | Base Button |
| 可选主体 | 可用时停靠 | 请求相反 checked 值一次 | Base Toggle：Button + toggled true / false |
| 关闭按钮 | 存在且可用时独立停靠 | 请求关闭一次 | Base Button，完整动作名称 |
| 禁用项 | 跳过 | 不响应 | 保留名称、禁用和真实 checked 状态 |

Tab 顺序为主体 → 关闭 → 下一项；静态可关闭项直接进入关闭按钮。焦点环使用 `--tag-focus-ring-color/width/offset`，向外至少预留 offset + width 的空间，不能被整卡 `overflow_hidden` 裁掉。只有文字槽裁切；根节点保留焦点环空间。

命令标签沿用桌面箭头光标，不改成外链手型。可交互性通过状态反馈、名称、焦点及原生语义表达。

主体和关闭区分别接受调用方保留的 FocusHandle。当前受维护的 Base Button / Toggle 在 `track_focus` 时也应用传入的 `tab_index` / `tab_stop` 策略，Tag 通过同一路径转发；关闭区有独立的 `close_tab_stop`。禁用时通过 Base 路径排除交互，不丢失调用方持有的 handle。

不新增 live region；普通筛选切换由 Toggle 状态反馈，异步结果由现有 Toast 或页面反馈承载。Windows 屏幕阅读器实际播报属于用户真实环境验收范围，不能以属性存在冒充已验证。

## 11. Base / Yue 分层

| 层 | 职责 |
| --- | --- |
| `gpui-base::Button` | 动作和关闭区的激活、键盘、焦点、禁用、原生按钮语义 |
| `gpui-base::Toggle` | 主体 pressed 值、一次切换请求、焦点、禁用、原生 toggled 状态 |
| Base motion / keyed state | 已有过渡与稳定行为身份，不保存业务集合 |
| Yue Tag | token 解析、形状、紧凑排版、图标、状态外观、两个独立操作区 |
| Yue TagGroup | 有间距的流式排列，不接管值和导航模型 |
| 现有 Tooltip / Popup | 实际溢出说明及浮层行为 |
| 宿主 | 数据、checked、单多选规则、关闭是否接受、添加流程、焦点恢复 |

已核实 Base `Toggle::pressed`、`disabled`、`styles`、`accessibility_label`、`on_change`、`tab_index`、`tab_stop`、`track_focus`。Yue 应直接适配这些入口，不在 div 上再监听一套鼠标和 Enter / Space。

本设计不要求先新增 Base Tag：当前 Button / Toggle 已能覆盖基本语义。若实施时发现激活、焦点或事件隔离的普遍缺口，应在 Base 修复通用行为，再由 Yue 使用，不能让 Base 依赖 Yue token。

## 12. 公共 API

下列 API 已由 crate 根导出；prelude 提供 `YueTag` / `YueTagGroup`，具体外观选项显式导入。构造器与组合示例有 doctest 覆盖。

| 接口 | 契约 |
| --- | --- |
| `YueTag::new(id, label)` | 静态标签，继承默认主题 |
| `YueTag::action(id, label)` | 动作主体，通过 `on_click` 接收激活 |
| `YueTag::checkable(id, label, checked)` | 受控 Toggle 主体，通过 `on_change` 请求新布尔值 |
| `tone / variant / size / shape` | 使用上述四个枚举，所有模式共享一套外观 |
| `leading_icon` | 装饰图标；checkable 中复用已有图标槽，否则仅在选中时展开勾选图标 |
| `on_close / close_label` | 可选、独立、受控关闭 |
| `disabled` | 整项禁用，不清空 checked |
| `theme / use_default_theme` | 与现有组件相同的局部覆盖 / 默认主题策略 |
| `width / min_width / max_width / margin / flex_grow` | 外部布局；内部高度和 padding 保持 token 所有权 |
| `overflow_tooltip` | 默认开启实际溢出提示，不凭空添加焦点点位 |
| `accessibility_label / accessibility_description` | 主体名称与说明，完整文字作为名称默认值 |
| `track_focus / track_close_focus` | 分别绑定真实主体与关闭按钮，供宿主管理删除后焦点 |
| `tab_index / tab_stop / close_tab_stop / key_context` | 显式交互策略；静态主体不能借此变为可聚焦按钮 |
| `colors(TagColors)` | 静态 / 动作标签的显式自定义三色，见第 13 节 |
| `YueTagGroup::new(id).children(tags)` | 受限为 Tag 的排列容器，支持 wrap 与外部布局 |

三个构造器固定私有主体模式，不再提供切换模式的 boolean setter。`on_click` 只适用于 action，`on_change` 只适用于 checkable；错误组合、交互模式缺少对应回调时，在构建验证阶段给出清晰错误，不静默降级。关闭与任一主体模式正交。

`TagTone`、`TagVariant`、`TagSize`、`TagShape` 使用语义枚举；配置与事件字段默认私有，通过 builder 和 reader 访问。Tag 不开放任意交互 children，不允许使用插槽绕过独立操作区约束。

基本用法；示例回调留空，实际由宿主执行动作并保存受控值：

```rust
use yue_component::{TagVariant, YueTag, outline};

YueTag::new("file-kind", "PNG")
    .variant(TagVariant::Outline);

YueTag::action("quick-path-screenshots", "截图目录")
    .leading_icon(outline::Folder)
    .on_click(|event, window, cx| { /* 打开目录 */ })
    .on_close(|event, window, cx| { /* 请求移除快捷记录 */ });

let copy_path = false; // 实际从宿主状态读取。
YueTag::checkable("copy-path", "复制路径", copy_path)
    .on_change(|next, event, window, cx| { /* 宿主保存 next 并 notify */ });
```

## 13. 自定义颜色与 token 策略

已有 Tag token 已进入仓库的 `tokens/inventory.json` 和生成目录，共 90 项。**实现前应复用现有矩阵，不重复创建 Tag 色板或手改生成 key。**

自定义颜色采用一个显式 `TagColors` 值：background、foreground、border 三项必须完整提供。它覆盖 tone 的基础三色，hover / pressed 仍使用共同状态规则。仅改变背景的便捷 setter 不进入 V1，避免任意深浅背景搭配错误文字色。

颜色优先来自语义主题；用户分类色等真实数据色可以作为明确例外输入。普通文字按至少 4.5:1 的对比度目标评审，必要的边界与状态标记按 3:1 目标评审；透明背景要相对实际父表面判断。数据色不满足要求时由宿主提供调整后的三色，不承诺自动纠正任意输入。

V1 `colors` 仅适用于静态和动作标签；可选标签保持完整语义 tone 矩阵。未来若需要自定义可选颜色，再引入同时描述未选中和选中的完整契约，不能通过一个三色值覆盖两个状态而丢失选中差异。

尺寸、圆角、状态、焦点和交互时长已有 token 可复用；Mark 的两端规则、前置图标复用 close 图标尺寸、关闭区等高、组间距复用 space-8 均是本文明确的映射，不声称已有对应独立 token。若实施证明需要独立覆写点，先在 Yue Design System 源包增加并记录依据，再运行既有同步工具，审查生成 Rust 和 inventory；不在组件中散落裸像素或颜色补丁。

## 14. SnapClip 保存面板的对应关系

| 原型用途 | Tag 模式 | 宿主职责 |
| --- | --- | --- |
| 常用保存路径 | action；若需要表示持续当前路径则用 checkable | 点击切换路径，稳定路径 ID，保存当前值 |
| 路径右侧 × | 同一 Tag 的独立关闭区 | 移除快捷记录；不得触发目录切换 |
| 100% / 75% / 50% / 25% | action | 应用一次尺寸预设；若有持续选择语义，由宿主明确改成受控单选 |
| “复制路径”“打开文件夹” | 两个独立 checkable Tag | 分别保存布尔值，不互斥 |
| 文件类型 / 属性 | 静态 Tag | 仅展示；有真正选择器时组合 Dropdown |

因此这些用途无需再设计独立 Chip 或 Toggle 皮肤；也不能把所有短按钮都改成标签。保存、取消等表单主操作继续使用 Button。

## 15. 实施与验收边界

实施对应：`crates/yue-component/src/tag.rs` 负责公共契约，`tag/element.rs` 负责 Base 组合、token 和排版，`tag/tests.rs` 与 `tests/tag.rs` 覆盖纯逻辑和交互；`lib.rs` 提供公共导出。

Showcase 的 **组件 → Tag** 包括：

1. 五色 × 四外观矩阵；三种尺寸、三种形状与图标。
2. 静态、动作、可选三种模式；disabled、关闭、选中和主题配置。
3. Style A / B / C 对应外观；独立多选与宿主控制的单个生效示例。
4. 可关闭快捷路径；主体动作与关闭的独立计数，验证关闭不触发主体。
5. 添加标签的 Input + Button 组合、窄宽省略、自动换行及 SnapClip 典型组合。
6. 交互反馈和 reduced motion；所有配置只调用公开 API，不在演示页另写替代行为。

自动化验收范围：

| 范围 | 重点 |
| --- | --- |
| token | 五色四外观在明暗主题及 neutral tone 下可解析；与源矩阵一致，禁用不重复叠透明度 |
| 布局 | 三档尺寸、三种轮廓、混排、关闭命中区、长文本和 rem 缩放；未选中无占位，选中宽度连续变化，有前置图标 / 固定宽度时外框稳定 |
| 指针 / 键盘 | 鼠标与 Enter / Space 每次只请求一次；关闭不触发主体；disabled 全部不响应 |
| 受控状态 | 宿主拒绝切换 / 关闭仍保留原值；外部同步不反向发回调 |
| 焦点 / 语义 | 静态无停靠、交互顺序、完整名称和 toggled 值、删除后的宿主焦点策略 |
| 身份 / 溢出 | 重排保留状态；实际溢出才提示；没有残留 Tooltip 或多余焦点点位 |
| 动效 | 中途反向、快速切换、禁用和 reduced motion，不重放历史动画 |

真实窗口外观、DPI 和屏幕阅读器体验由用户验收；headless 测试与真实 UI 结论分开记录。此次不启动真实 Showcase 窗口或执行真实 UI 检查。

## 16. 源码与文档索引

- 用户参考：[Tag 标签图](assets/tag-reference.png)。
- Base 行为：[Button](../vendor/gpui-base/src/button.rs)、[Toggle](../vendor/gpui-base/src/toggle.rs)、[ToggleGroup](../vendor/gpui-base/src/toggle_group.rs)、[motion](../vendor/gpui-base/src/motion.rs)。
- 现有受控组合：[Segmented](../crates/yue-component/src/segmented.rs)、[SwatchGroup](../crates/yue-component/src/swatch.rs)。
- Yue 基础设施：[motion](../crates/yue-component/src/motion.rs)、[Tooltip](10-tooltip.md)、[Popup](12-popup.md)、[Button](02-button.md)。
- token 快照：[inventory](../crates/yue-component/tokens/inventory.json)；唯一外部源为 `D:\100_Projects\110_Daily\Yue-Design-System\packages\tokens\src\component-tokens\tag.css`。
- 业务对照：`D:\100_Projects\110_Daily\SnapClip\docs\design\save-panel-demo-v3.html`。

## 17. V1 实现记录

2026-11-02 实施：

- `YueTag::new / action / checkable` 分别呈现 Caption、Base Button、Base Toggle；关闭按钮是与主体并列的独立 Base Button。缺少必需回调或错误模式调用会明确报错。
- 完整复用已有 90 项 Tag token，未新增或修改源 token、快照和生成键；20 种色彩组合按静态 key 解析，并与源名称逐一比对。
- 关闭命中区与 Tag 同高；共用外轮廓独立绘制，不挤压控制区。文字至少保留一个 em，宽度过小时以结构最小值为准；支持三档尺寸、三种轮廓和 rem 缩放。
- checked 的颜色和标记、主体与关闭区各自的 hover / pressed 均使用 Base transition；主题、禁用和主体模式变化会清除旧反馈。reduced motion 直接到达目标值。焦点环不依赖颜色过渡，也不裁切整卡。
- 溢出说明使用既有 `YueOverflowTooltip`，关联真实主体焦点；关闭区和禁用状态不成为全文提示触发器。
- `YueTagGroup` 自动换行、检查重复子 ID；组主题传给没有显式主题的子项。它不增加组级选择状态或方向键导航。
- Showcase 提供配置、颜色矩阵、选择、关闭与添加、保存面板组合、长文本与自定义颜色。示例宿主实现空白 / 重复验证、Enter 提交、Escape 取消和删除后焦点迁移。
- V1 不包含增删退出、重排动画及自定义 checkable 三色，保持第 9、13 节的既定边界。

初始实现验证：8 项单元测试、15 项组件 headless 交互测试、4 项 Showcase Tag 测试通过，另有公开 API doctest。以下工作区验证也通过：

```powershell
node tools/sync-yue-theme.mjs --check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
```

原生窗口视觉与真实读屏输出尚未检查。正常 Showcase exe 被运行中的程序占用时，不关闭用户窗口；使用 `TEMP/tag-preview/Cargo.toml` 复用同一份 Showcase 源码和构建目录，生成独立的 `target/debug/windows-showcase-tag-preview.exe` 供用户检查。

### 2026-11-02 未选中图标占位修正

用户真实 UI 检查发现：固定前置勾选槽造成未选中标签左侧空白偏大。修正为无前置图标、未选中时槽宽和 gap 均为零；不在末尾增加补偿空白。选中时沿用现有 interaction 过渡展开图标视口与间隔，glyph 自身不缩放，也不裁切主体焦点环。已有前置图标的实例保持外框宽度，显式 `width` 同样维持固定外框。

新增回归比较未选中 Tag 与普通文字 Tag 的完整尺寸及文字边界，验证两侧 padding 对称、展开有中间宽度、取消后精确回到紧凑状态，并覆盖固定宽度配置。自然宽度下文字、关闭区和相邻标签随内容布局连续调整，不新增集合重排状态机。

16 项 Tag 交互测试及工作区 headless 回归通过；本轮预览另命名为 `target/debug/windows-showcase-tag-preview-v2.exe`，供用户检查，避免覆盖正在使用的旧版预览。原生窗口外观仍由用户检查。

### 2026-11-02 勾选笔画动画

根据用户要求，勾选标记改为复用 Checkbox 的矢量路径绘制：100ms 展开图标槽和间隔，随后 200ms 从短边到长边画出勾；取消时先反向擦除，再收起。单条可反向时间线维持中断连续性，固定画布避免变形；无前置图标的未选中标签仍不占位，尾部没有补偿空白。

新增阶段采样与交互回归，覆盖先展开后绘制、先擦除后收起、连续反向和 reduced motion。9 项 Tag 单元测试、17 项 Tag headless 交互测试、Checkbox 回归、工作区 headless 测试及工作区 Clippy（`-D warnings`）通过。预览使用独立名称 `target/debug/windows-showcase-tag-preview-v3.exe`，页面为“组件 → Tag”，真实 UI 由用户检查。

### 2026-11-02 勾选标记尺寸微调

按用户反馈，在原有 12px 图标槽内将勾的路径放大 `4/3`，可见宽度从 6px 调整为 8px；描边由 1px 调整为已有 Checkbox medium token 的 1.5px。保持逐笔绘制、固定槽宽及未选中零占位。17 项 Tag headless 交互测试通过，更新预览为 `target/debug/windows-showcase-tag-preview-v4.exe`。
