# YueAvatar 头像组件设计

- 状态：V1 已实现（2026-11-07）。Windows 真机图像、DPI 与 Narrator 验收待完成。
- 日期：2026-11-07。
- 目标：定义单头像与紧凑头像组的视觉、内容回退和组合边界，复用 GPUI Base 的头像插槽、GPUI 图片 fallback 机制及 Yue Avatar tokens。

`YueAvatar` 用图片、字符或默认图标表示用户/对象身份。它是值式展示组件，不拥有用户数据、图片加载请求、点击命令或人员选择状态。`YueAvatarGroup` 仅负责一组头像的重叠、尺寸和超量计数，不成为成员管理组件。

## 1. 参考与取舍

参考用户提供的两张 Avatar 示例图，以及锁定版本 `gpui-kit 0.8.0` 的 GPUI Base / GPUI Component 实现。参考图用于提取常见能力，不要求复刻网页文档站的配置台或 Arco 的全部属性。

| 参考能力 | Yue 取舍 |
| --- | --- |
| 圆形与圆角方形 | 提供 `Circle` / `Rounded` 两种形状；默认 Circle。Rounded 需要补充非 full 的形状 token。 |
| 图片、名字缩写、默认人像 | 依次优先显示成功加载的图片、fallback 文本、默认用户图标。 |
| 小、中、大和自定义大小 | 首版采用 Yue 已有 `SM / MD / LG / XL` 尺寸 tokens；先不提供任意像素尺寸覆盖。 |
| 长字符自动适配 | 提供默认开启的字号向下适配，保留头像外框尺寸；内容不能撑大头像。 |
| 头像重叠组和超量计数 | 首版纳入轻量 `YueAvatarGroup`，使用最大可见数与 `+N` 回退头像。 |
| 头像作为操作触发器、状态角标 | 不放入头像本体；由 Button、Badge、Tag 或宿主组合。 |
| 图片加载失败隐藏 | 失败时按 fallback 顺序呈现；不保留一个空白或破图表面。 |

GPUI Component 自带主题版 `Avatar` / `AvatarGroup`，但依赖其 `ActiveTheme`、`Size` 和视觉系统。Yue 不直接包装其主题呈现，避免在一套控件内混用两个主题；只复用其已验证的初始化颜色对比思路作为参考，不复制其实现或调色策略。

## 2. GPUI-KIT 能力与缺口

锁定依赖实际提供：

- `gpui_kit::base::Avatar`：无样式根元素，可放置图片和 fallback 插槽，并支持普通交互能力。
- `AvatarImage`：使用 `ImageSource` 绘制图片。
- `AvatarFallback`：无样式 fallback 容器，可容纳字符或调用方图标。
- GPUI `Img` 的 `StyledImage::with_fallback(...)`：图片加载错误时可渲染替代元素。
- `gpui-component 0.8.0` 的 `Avatar`：图片、名字缩写、默认 User 图标、尺寸与姓名派生颜色；`AvatarGroup` 提供重叠、数量上限和省略项。

初始缺口已处理：Base `AvatarImage` 增加 `with_fallback` 和 `object_fit` 透传；AvatarGroup 由 Yue 用值式布局组合实现。Base 未新增头像组状态或图片加载状态机。

Base 扩展只连接 GPUI 已有的 `StyledImage` 回调，不下沉 Yue 颜色、字体、圆角或尺寸，也不引入新依赖。无头测试通过缺失资源验证错误回调能显示头像 fallback。

## 3. 组件职责

### 3.1 YueAvatar

- 以固定外框展示图片或 fallback。
- 提供圆形/圆角方形、四个语义尺寸、姓名/自定义 fallback 文本和默认图标。
- 图片加载失败时交由 GPUI `Img` fallback 显示同一个字符/图标 fallback。
- 使用 Yue Avatar tokens 呈现背景、文本、尺寸、字体和圆角。
- 提供稳定的可访问名称；纯装饰头像允许调用方显式关闭独立语义。

### 3.2 YueAvatarGroup

- 以稳定调用顺序排列头像，按统一尺寸与有界重叠比例计算位置和组宽度。
- 通过 `max` 限制显示数量；剩余项显示为 `+N`，总数可访问名称同步表达。
- 为每个子头像保留固定尺寸，不根据容器剩余宽度压缩。
- 左侧头像绘制在右侧头像之上，重叠边缘保留随主题变化的细轮廓；顺序由绘制测试固定。

### 3.3 非目标

- 不管理头像 URL、缓存、网络请求、文件权限、图片裁剪工作流或上传。
- 不从全局账户、用户模型或系统身份自动读取名称/图片。
- 不内置选中、在线状态、通知数字、删除按钮、点击/右键菜单或人员弹窗。
- 不实现可编辑头像、头像选择器、可拖拽排序或独立焦点顺序。
- 不为单个头像创建 retained `Entity`；图片缓存和加载任务仍由 GPUI 图片系统拥有。
- 不复刻网页 Demo 的配置面板或无边界 API（CSS style/className 等）。

需要点击头像时，宿主将头像放入具有明确名称的 YueButton；状态圆点或角标由宿主在外层组合。Avatar 自身默认不成为 tab stop。

## 4. 建议 API

以下为 V1 已发布 API。`id` 是稳定的 `ElementId`，用于 Image/Group 无障碍节点和 fallback 的布局身份；同一页面的每个头像及头像组必须使用唯一 ID。

```rust
pub enum AvatarSize {
    Small,  // 24px
    Medium, // 32px，默认
    Large,  // 40px
    XLarge, // 56px
}

pub enum AvatarShape {
    Circle,  // 默认
    Rounded, // Yue Avatar 圆角方形 token
}

pub struct YueAvatar { /* RenderOnce */ }

impl YueAvatar {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme) -> Self;
    pub fn use_default_theme(self) -> Self;
    pub fn theme(self, theme: YueTheme) -> Self;
    pub fn src(self, source: impl Into<ImageSource>) -> Self;
    pub fn name(self, name: impl Into<SharedString>) -> Self;
    pub fn fallback_text(self, text: impl Into<SharedString>) -> Self;
    pub fn placeholder(self, icon: impl IconDefinition) -> Self;
    pub fn size(self, size: AvatarSize) -> Self;
    pub fn shape(self, shape: AvatarShape) -> Self;
    pub fn auto_fit_font_size(self, enabled: bool) -> Self;
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self;
    pub fn semantics(self, enabled: bool) -> Self;
}

pub struct YueAvatarGroup { /* RenderOnce */ }

impl YueAvatarGroup {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme) -> Self;
    pub fn children(self, avatars: impl IntoIterator<Item = YueAvatar>) -> Self;
    pub fn size(self, size: AvatarSize) -> Self;
    pub fn shape(self, shape: AvatarShape) -> Self;
    pub fn max(self, visible: usize) -> Self;
    pub fn overlap(self, ratio: f32) -> Self; // 非有限值回到 0.28，其余夹在 0.2..=0.4
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self;
}
```

构造参数只保留必要的 YueTheme，与现有 Yue 值式组件一致；`.use_default_theme()` 在 render 时解析当前应用主题。没有跨帧状态的组件使用 `RenderOnce`。如果未来有异步业务图片 provider，应通过 `ImageSource`/GPUI image cache 接入，不改变 Avatar 的状态所有权。

`name` 派生 fallback 字符：按 Rust Unicode whitespace 分词，拉丁名称取最多两个词的首字符；单词名称取前两个 Unicode scalar；CJK 名称默认取第一个字符。此规则不识别全部 Unicode grapheme/书写系统；用户有本地化规则或特殊名称时用 `fallback_text` 明确覆盖。默认图标只在图片与文字均缺省时出现。

`auto_fit_font_size(true)` 按头像可用宽度和字符数估算字号，只缩小、不放大默认字号，比例下限为 0.55；头像外框和行布局尺寸不受字符长度影响。超过可读范围的自定义 fallback 在固定裁剪框内被裁切，调用方应优先提供短文本或用 `name` 派生 initials。字符宽度是估算值，中文字体和 emoji 的最终效果需在 Windows 实窗验收。

## 5. 图片与 fallback 合同

渲染优先级：

```text
提供 src 且图片成功      -> 图片
提供 src 且图片加载失败  -> fallback_text / name 派生字符 / placeholder
无 src                   -> fallback_text / name 派生字符 / placeholder
```

- 图片使用 `ObjectFit::Cover`，裁剪到头像圆形或圆角形状；图片本身不改变固有布局尺寸。
- 加载中的表现维持固定容器与中性背景，不显示会改变尺寸的 loading 文字或 Spinner。
- 图片错误 fallback 必须由 GPUI `Img` 的 `with_fallback` 路径产生，不能从请求 URI 推断成功/失败，也不轮询 cache。
- 解码失败、资源不存在和加载错误均由 GPUI `Img` 的 error fallback 统一处理。头像不提供加载错误事件；业务层若需要诊断，应在图片资源 owner 处理。
- 如果 Base fallback callback 无法覆盖 Yue 文字/图标回退，优先扩展 `AvatarImage` 的 fallback builder；不在 Yue 中复制图像加载生命周期。

调用方明确需要保留错误图像状态时，V1 不提供此开关；这会让失败占位与正常头像行为分叉，当前参考图的 `hideOnLoadFailed` 控件不构成必须照搬的产品 API。

## 6. 尺寸、形状与主题

Yue 已生成 Avatar tokens：`--avatar-size-sm/md/lg/xl`、`--avatar-font-size`、`--avatar-background`、`--avatar-color`、`--avatar-border-radius` 和 Rounded 方形专用 `--avatar-shape-radius`。圆形读取 full radius，Rounded 读取独立形状 token。实现直接读取 token；不手写第二份颜色/尺寸表，也不照搬 GPUI Component 的 OKLCH 姓名配色。

| `AvatarSize` | 默认外框尺寸 | 默认用途 |
| --- | ---: | --- |
| `Small` | 24px | 紧凑列表、菜单、辅助信息 |
| `Medium` | 32px | 普通列表和设置项，默认 |
| `Large` | 40px | 详情标题、用户信息块 |
| `XLarge` | 56px | 资料摘要或独立身份展示 |

上述像素是当前 Yue token 值，GPUI 中按 rem/token 换算，主题缩放改变时头像随之缩放。外框具有固定宽高、`flex-shrink: 0`、`overflow: hidden`；禁止字符内容、图片固有尺寸或状态装饰改变参与布局的包围盒。

| Shape | 表现 |
| --- | --- |
| `Circle` | `border-radius: 9999px`；默认头像外形 |
| `Rounded` | 使用新增的 `--avatar-shape-radius` Yue token 圆角方形 |

图片、字形和默认图标共用同一个裁剪形状。容器/字符 fallback 使用 Avatar background/color token，浅色/深色、Azure/Neutral 均由主题映射提供。V1 不添加自定义颜色入口，避免不同调用方造成身份颜色不可读或不稳定。

## 7. AvatarGroup 布局

```text
最多可见 4 项时，展示顺序与输入一致：
[头像 A][头像 B][头像 C][头像 D][+3]
```

- 视觉顺序与输入顺序保持一致；从右到左绘制，让左侧头像覆盖右侧头像。默认从左到右按输入顺序阅读，末尾 `+N` 表示未展示成员数量。
- 子头像尺寸使用 group 的统一 `AvatarSize` / `AvatarShape` 覆盖，避免组中几何不齐；如果需要一组不同尺寸，不使用 AvatarGroup。
- 重叠比例默认 0.28，限制有效区间，保证每张头像至少留出可辨身份的可见部分。位置步长为 `尺寸 × (1 - overlap)`；组宽度为 `尺寸 + (显示头像及 +N 的个数 - 1) × 步长`，高度等于头像尺寸。
- 每个头像和 `+N` 的边缘覆盖一圈 `--border-2` 宽的分隔轮廓，颜色使用 `--surface`；轮廓与头像形状一致，不改变头像外框尺寸。浅色呈现浅色分隔，深色随主题变暗，避免同色头像连成一片。圆角方形与组尺寸统一覆盖子头像设置。
- `max(0)` 表示不截断，不渲染 `+N`；空组不产生占位节点。
- `+N` 使用 Avatar 中性色背景和文字色；它不伪装成可点击按钮。查看隐藏成员由宿主另设 Button 或 Tooltip。
- accessible name 默认为调用方提供的组名和总数；各头像在组语义下不得重复制造独立 tab stop。是否使用 group role 需按 AccessKit 可用语义验证，不能只设置 role 字符串就声称 Narrator 行为完成。

## 8. 可访问性与交互

- 静态图片若表达用户身份，应有名称，如“王伟”；只为装饰时用 `.semantics(false)`，不应朗读文件名或 URL。
- fallback 首字符不是可访问名称的替代，仍使用完整用户名称；自定义 fallback text 也不自动覆盖 accessibility label。
- Avatar 默认无 Button/Image 等可聚焦交互角色，不进入 Tab 顺序。GPUI Base 根节点是否生成可访问容器需测试确认；装饰性使用不得留下空名称节点。
- 交互场景由 YueButton 包裹，Button 提供命令名称、焦点、按压、禁用和键盘激活；Avatar 继续负责图像呈现。
- Group 的 `+N` 和数量语义不能只靠颜色或视觉省略号表达；组名称作为 label，描述同时包含总成员数与显示数。
- 图片对比度不做修饰；字符 fallback 与背景需满足普通文字对比度要求，轮廓在需要区分边界的主题下达到非文本对比度要求。

## 9. 文件与依赖

目标最小结构：

```text
crates/yue-component/src/avatar/
├── mod.rs       # 公共值类型、枚举、builder、导出
├── element.rs   # Yue token 呈现、Base Avatar 插槽适配和 Group 布局
└── tests.rs     # initials、固定几何、组截断与公开语义回归
```

不拆 State、loader、manager 或通用图片服务。Base 透传留在 `vendor/gpui-base/src/avatar.rs`，按 Base 补丁维护流程记录。根导出和 `prelude` 已同步。圆角 token 已在 Yue Design System 添加并同步到生成快照；组件不直接手写颜色或几何值。

## 10. 示例与验收

Showcase 的 Avatar 页面覆盖：

1. 默认图标、1 字/2 字 fallback、中英文姓名和明确自定义 fallback 文本。
2. 图片成功、图片资源错误后回退到字符、回退到默认图标。
3. 圆形与圆角形，Small/Medium/Large/XLarge；长拉丁名、长 CJK 字符、emoji 与主题缩放下不撑开容器。
4. AvatarGroup 空组、单项、恰好到达 max、超出 max、max=0、数量 badge 和两种主题下重叠边线。
5. 装饰性无语义头像、具名静态头像，以及 YueButton 包裹时只有 Button 是焦点节点。

自动化验收：

- 纯测试：缩写规则、Unicode 空白分词、空字符串和长文本字号边界。
- Base 图片路径测试：成功图像优先；图片错误走 callback fallback；无图时使用 AvatarFallback。
- 组件布局测试：尺寸/形状矩阵、文本不改变外框、头像组显示数与总数计算、顺序与 overlap 几何、空组。
- 无障碍测试：名称、装饰节点、group 总数描述和 Button 组合的 role/focus 不重复。
- token 测试：四种主题 profile 的 Avatar token 能解析，文字对比度合格。
- Windows 实窗：检查 raster 裁剪、图像加载反馈、中文字体/emoji、主题切换、DPI 缩放与 Narrator。headless 通过不代表真实 Narrator 已验收。

## 11. 实现状态与剩余验收

- Base `AvatarImage` 已透传图片加载失败 fallback 与 `ObjectFit`，错误回退复用 GPUI 图片组件回调，不复制加载状态机。
- Rounded 形状已使用 Yue Design System 的 `--avatar-shape-radius` token，并同步到本仓库生成快照。
- 自动化覆盖 Unicode initials、fallback、尺寸、头像组溢出描述与基础无障碍语义。
- 仍需在 Windows 实窗检查不同字体下中文/emoji 字号适配、图片裁剪、主题缩放和 Narrator 播报；这些依赖真实平台，headless 结果不能替代。
- AvatarGroup 使用现有 `--border-2` 与 `--surface` tokens 绘制重叠边缘分隔；单头像不增加轮廓。

自动化已覆盖 Unicode initials、无障碍名称/角色、头像组溢出描述、尺寸差异及缺失图片 fallback。真机上仍需检查不同 Windows 字体下中文/emoji 字号适配、图片裁剪、DPI 缩放和 Narrator 的 Image/Group 播报；无头结果不替代这些验收。
