# 27 · Windows HDR 图片与显示管线实施方案

日期：2026-11-03。状态：**P1 高精度资源、P2 Windows 输出及 P3 的像素／行带底座、冻结帧来源代次、压缩母版发布、WGC 格式合同和 D2D 内部 FP16 链路已实现；P0 AVIF 门禁、P2 真实 HDR 显示、P3 生产捕获／滚动／编码和 P4～P5 尚未完成，SnapClip App／Capture 严格 Clippy 仍有失败，不代表 SnapClip 已支持 HDR。** 适用：GPUI Kit 0.8.0、本仓库维护的 GPUI core／Windows D3D11 补丁，以及 SnapClip 的捕获、保存面板和贴图。实施证据和剩余项见 [本轮记录](performance/2026-11-03-hdr-resources.md)。

本次核对本地源码、Rust 依赖和微软／W3C／ITU 官方资料。本文不把库的格式支持列表当作端到端支持，不把数值测试当作显示器验收，也不沿用“先转为 SDR 才能显示”的假设。

## 1. 目标与基本事实

用户要完成的任务是：截取或打开 HDR 内容，在允许 HDR 输出的屏幕上看到保留高光的图片，并能明确选择保留 HDR 或导出 SDR。原图、原位贴图、保存预览与最终文件必须遵守同一色彩合同。

| 不可省略的事实 | 推导出的设计要求 |
| --- | --- |
| 数字像素只有结合色域、传递函数、亮度尺度才表示颜色 | `Vec<u8>`、16 位和“HDR”布尔值都不足以定义图像 |
| 先量化到 RGBA8 会失去源精度及高光信息 | 在解码、捕获、代理、存储、上传、输出间保留高精度 |
| 屏幕支持 HDR、系统启用 HDR、窗口成功使用 HDR 输出是不同条件 | 分别检测并报告，不以设置页“受支持”代替实际输出状态 |
| SDR 屏幕无法直接呈现 HDR 的完整亮度范围 | 仅显示派生结果做映射，HDR 母版不被覆盖 |
| HDR 比 SDR 存储更多有效信息，但资源成本也取决于尺寸、次数和生命周期 | 有界处理和实测优先，不先引入零拷贝框架或全局缓存 |

实施决策：

1. 继续使用现有 D3D11，不另接 wgpu 或第二套图片渲染器。
2. HDR 工作／显示资源使用 **线性 scRGB、RGBA16F**；CPU 计算按需使用 F32，文件原始编码独立保留。
3. 采用微软 FP16／scRGB 输出路线，不首发并行的 RGB10A2／HDR10／PQ swapchain 路线。[W1]
4. 通用组件只负责图像资源、相机和展示；捕获、文件 I/O、保存、剪贴板与任务取消继续由宿主负责。
5. 删除错误的“所有图像都是四字节 BGRA”合同；保留有效的 SDR 能力。SDR 输出是产品功能，不是旧接口兼容层。
6. 复用既有模块和生命周期。首轮不新增 HDR runtime、loader、cache 或 GPU bridge crate。

### 1.1 交付边界

首轮完整范围包括：WGC scRGB 捕获、带有效色彩定义的 PQ PNG16、SDR／ICC 静态图、HDR 预览、保存面板、F3／F6 图片贴图、滚动截图母版、PNG HDR 导出和明确的 SDR 派生输出。AVIF 必须先确认当前消费链路和解码精度，不能因为其他入口能处理 SDR AVIF 就直接宣称 HDR AVIF 可用。

EXR、Radiance `.hdr`、视频、Ultra HDR／gain map、任意 HLG 场景解释不属于本轮交付。遇到不支持的传递函数明确报错，不按 SDR 显示来制造“支持”。不要因扩展名或位深判定 HDR。

## 2. 官方能力与系统范围

| 官方依据 | 对项目的约束 |
| --- | --- |
| WGC创建frame pool时指定像素格式；微软以FP16／scRGB作为通用Advanced Color路线 [W1][W2] | 本方案选择HDR捕获全链路FP16；frame pool、冻结纹理、拷贝、覆盖层和导出不能中途降成BGRA8 |
| Windows HDR输出中scRGB的1.0对应80nit，可表达大于1和负RGB；SDR输出中1.0是显示白 [W1] | 资源工作尺度与最终输出尺度分开；禁止对HDR RGB通用`saturate` |
| Win32 `DisplayInformation.GetForWindow` 最低 Windows 11 22H2，build 22621 [W3] | 新 API 需运行时探测；不把它当作所有 Windows 的必有能力 |
| Advanced Color 区分当前 SDR／WCG／HDR 类型 [W4] | Wide Color 不等同于 HDR；系统状态和渲染成功状态分开存储 |
| `CheckColorSpaceSupport` 与 `SetColorSpace1` 控制 swapchain 色彩空间 [W5][W6] | 检查 PRESENT 支持并检查设置结果；创建 FP16 纹理本身不能证明 HDR 输出成功 |
| `GetDesc1` 提供输出色彩及亮度能力；桌面配置变化后应刷新 factory／输出 [W7] | 不把显示器峰值当作图像 MaxCLL，不缓存永远不变的显示器信息 |

设计默认：**完整 HDR 实机验收基线为 Windows 11 22H2+**，不提高整个组件库的最低系统要求。较早系统仍可使用现有 SDR 窗口和明确的 SDR 显示派生结果、保留／导出 HDR 文件；首轮不宣称旧系统已通过 HDR 实机认证。若后续必须覆盖 Windows 10 原生 HDR，再依据实际用户系统增加完整检测和通知测试，不能仅靠一个 enabled 位猜测。

新 API 不需要引入 Windows App SDK 大运行时；使用现有 `windows` 绑定接入系统 WinRT。`GetForWindow`要求当前线程已有`DispatcherQueue`，应在Windows平台初始化中检查并按需建立，随线程关闭，不为每张图启动线程。初始化、事件回调线程和注销必须与窗口生命周期绑定；不得在 GPUI `render` 中创建订阅。[W3]

## 3. 实施前源码审计：基线断点

下列记录改造前的基线断点；不再表示当前工作区的实现状态。已实现和未通过的验收项分别见 §11 与本轮记录，不能把整张审计表理解为全部已经交付。

| 位置 | 当前事实 | 必须改造的合同 |
| --- | --- | --- |
| `crates/yue-component/src/image_preview/resource.rs` | 解码最终调用 `into_rgba8()`；ICC／非 sRGB 仅标为 `Unmanaged` | 保留位深、解析色彩、明确转换状态 |
| `vendor/gpui-pre/src/assets.rs` | `RenderImage` 由 `image::Frame`／u8 像素承载 | 真实的像素存储格式与色彩编码，不能只加 HDR 标志 |
| `vendor/gpui-pre/src/platform.rs`、`scene.rs` | atlas／Polychrome 数据按既有格式组织 | 增加格式可区分的纹理资源及绘制元数据 |
| `vendor/gpui-pre-windows/src/directx_atlas.rs` | Polychrome texture 为 BGRA8，上传按四字节计算 | FP16 独立格式桶、stride、上传和预算 |
| `vendor/gpui-pre-windows/src/directx_renderer.rs` | HWND／DirectComposition 均以固定 BGRA8 创建；读回按 `width * 4` | 输出模式、色彩空间、按格式读回 |
| `vendor/gpui-pre-windows/src/shaders.hlsl` | 现有 sRGB 近似、文字校正及编码通道内混合 | HDR 模式下完整的线性颜色、Alpha、文字与滤波处理 |
| `directx_renderer/backdrop.rs` | 中间工作纹理已 FP16，但原始 ROI 来自 BGRA8 目标 | ROI 格式、合成、容量计算跟随实际目标 |
| `window.rs`、`window/paint_snapshot.rs` | 已有精确采样、Weak 上传回收与快照回放 | 保留新格式、资源所有权、色彩参数和失效机制 |

SnapClip 还存在捕获、Direct2D 覆盖层、行带、母版、剪贴板及原生 F6 的八位假设。具体改造见相邻仓库 [53 · HDR 捕获与消费接入](../../SnapClip/docs/53-hdr-capture.md)。不能仅修改 Yue 后对整个应用标注支持。

## 4. 数据合同：原始内容与显示资源分离

### 4.1 最小必要信息

| 对象 | 必须携带的信息 | 所有者 |
| --- | --- | --- |
| 原始内容 | 尺寸、原始文件或无损像素母版、编码格式、位深、ICC／CICP、方向、来源及 revision | 应用／持久化层 |
| 解码像素 | 存储格式、尺寸、row stride、色彩定义、Alpha 方式、原始尺寸／代理尺寸 | CPU 准备任务 |
| 显示资源 | sRGB8、相对线性 SDR16F 或绝对线性 scRGB16F、资源 ID／revision、纹理尺寸、源尺寸、描述 | GPUI `RenderImage`＋Yue 包装 |
| 窗口输出 | 当前显示类型、SDR white、峰值等能力、实际选用输出、失败原因、generation | Windows Window／renderer |
| 派生结果 | 原始 revision、转换用途、tone-map 参数版本、目标尺寸／编码 | 生成该结果的宿主任务 |

不允许把 `Rgba16F` 与任意编码排列组合。显示资源接受三种受约束的语义：**sRGB 编码、相对 SDR 白的 BGRA8**；**相对 SDR 白的线性 Rec.709 RGBA16F**；**绝对线性 scRGB、1.0=80 nit 的 RGBA16F**。增加第二种是为了保留 16 位 SDR／ICC 转换的精度和扩展色域，不能把它误标为绝对 HDR。PQ16 是源编码，必须经 EOTF 和色域变换后才能作为 scRGB 上传。文件来源保留原编码，捕获来源保留FP16像素母版；导出的PQ PNG不冒充逐bit保留浮点值的母版。

缓冲长度使用 checked 运算；验证 stride 至少为一行有效像素、总长度覆盖最后一行。FP16 构造校验 NaN／Inf、Alpha 范围和乘法溢出；不把合法负 RGB 或大于 1 判成损坏。源元数据不完整、不支持或冲突应有不同错误，不统一变成 `Srgb`。

GPUI core 拥有底层资源格式和显示编码；Yue 拥有图像准备及描述；SnapClip model 拥有可序列化的图像合同，不引入 GPUI／D3D 句柄。边界处一次显式转换类型，避免两个来源可互相覆盖的 `is_hdr`。

### 4.2 Alpha 与采样

- 公共 CPU 输入明确为 straight Alpha；scRGB 在 GPU 合成前进入 premultiplied linear 表示。透明源 RGB 不得污染可见边缘。
- SDR 资源进入 HDR 场景：先按真实 sRGB 分段函数线性化，再乘参考白比例、再预乘 Alpha。不能对已经预乘的编码 RGB 直接 EOTF。
- HDR 纹理不按 sRGB 再解码；读取 FP16 线性值后仅按合同处理 Alpha。WGC 桌面图像入口明确建立 opaque Alpha，不从任意内容猜测。
- 双线性、面积缩图与模糊在关联 Alpha 的线性颜色中进行；最近邻保留原样。不要把现有“编码 sRGB 中的 Alpha 加权”称为完整线性滤波。
- 覆盖率和 Alpha 可限制到 `[0,1]`，RGB 不套用同一个限制。圆角、opacity、Compare 分隔线和叠加工具条都应纳入测试。

## 5. 色彩解析与转换

### 5.1 解析策略

PNG 按第三版规范处理：cICP 优先于其他颜色信息；没有 cICP 时 iCCP 优先于 cHRM／gAMA。cICP 的 matrix／range 必须适用于 PNG RGB，不符合规范的数据不能靠猜测修复。无色彩标签的普通输入采用明确的 sRGB 默认。[P1]

AVIF 按容器解释 ICC、nclx 和 YUV 的矩阵／range，**不照搬 PNG 优先级**：YUV→RGB 所需矩阵与 RGB 的 ICC 描述职责不同。nclx 中 unspecified 的 primaries／transfer 不能压过有效 ICC；也不能因为 `ffprobe` 没打印 PQ 就认定文件没有 HDR。解码器必须输出原有精度并返回容器色彩定义。[P2]

ICC 用结构和转换能力识别，不能只匹配 profile 名称。已知 PQ 的矩阵／TRC 与 LUT profile 都需验证绝对亮度尺度；ICC PCS 的相对颜色结果不能直接冒充“1=80 nit”。不支持的 profile 显示明确错误或由宿主提供经过验证的像素，不静默丢弃 profile。

### 5.2 三条转换路径

| 来源 | 到工作空间的过程 | 是否 tone mapping |
| --- | --- | --- |
| WGC RGBA16F／scRGB | 保留线性值和绝对亮度定义 | 否 |
| Rec.2020／PQ 整数像素 | PQ EOTF 得到绝对亮度 → Rec.2020 到线性 Rec.709 原色 → 除以80得到 scRGB | 否 |
| sRGB／其他 SDR ICC | 保持源精度，ICC／传递函数及原色变换 → 相对线性 SDR；呈现时应用 SDR white | 否 |

PQ 使用 BT.2100 的 ST 2084 定义，整数归一化和 full／limited range 在 EOTF 前处理。转换到 Rec.709 原色不等于缩为 Rec.709 色域；负分量表达扩展色域，不能提前 clamp。[C1]

`moxcms` 是 ICC 转换工具，不是整个 HDR 系统。以锁定版本验证 ICC→线性 F32 的实际行为、intent 和绝对尺度；一旦 PQ 已由 CICP 解释，就不再重复经过 ICC 的 PQ 曲线。色彩转换、统计与缩图尽可能合并在行／块处理过程中。[L1]

### 5.3 SDR 回退的确定策略

采用一套固定、可复现的显示映射：以 **BT.2390 EETF** 为亮度压缩依据，配套明确的色域压缩与 sRGB 编码；不使用简单截断或临时 Reinhard 公式代替质量方案。BT.2390 是技术报告，不能将某个实现的观感称为显示器校准结果。[C2]

参数合同包括：源参考白、源峰值、目标白／峰值、亮度映射、色域压缩和算法版本。元数据中可靠的内容亮度可使用；缺失时在导入／转换过程中统计一次有限像素的 MaxRGB，不能直接拿显示器 `MaxLuminance` 当内容峰值。截图 scRGB 的绝对值不随目标屏幕重写。

静态导出SDR默认以100nit白／峰值计算映射，再归一化成sRGB相对值；SDR显示回退同样输出相对白，1.0表示目标显示白。固定同一目标参数时，CPU导出与GPU预览使用同一数学定义及测试向量，避免“两套相似算法”。`gainforge`只作为可比较的CPU实现候选，不预设其实现了本方案指定的每一步。[L4]

P0 必须完成 BT.2390／候选实现对照，冻结参数、色域压缩和容差，才能进入对比预览验收。若候选库不能满足，就在已有图像模块实现所需的窄函数并写参考向量；不为一个公式新增框架。显示器 HDR 输出时不执行这条 SDR 映射，也不主动把所有 HDR 亮度压到 SDR 白。

### 5.4 HDR屏幕之间的亮度适配

直接HDR显示不等于忽略显示器可表达范围。内容在当前HDR屏幕范围内时保留绝对scRGB；内容高光超出目标能力时，只在显示派生路径做HDR→HDR高光压缩及必要色域处理，保留高于SDR白的范围，不把整张图转SDR。沿用同一参数化映射定义，目标峰值取有效显示能力；获取不到可靠值时报告能力未知，不虚构1000nit。[W11]

WGC母版保留捕获后的绝对亮度，回到同一屏幕且未超范围时不再次套用SDR白或全局曝光补偿。屏幕峰值是显示约束，文件导出仍从母版出发；显示映射不能写回原图或成为HDR保存的默认内容。

## 6. Windows 输出与生命周期

### 6.1 一个渲染器，两种必要的输出行为

新增窗口色彩策略，命名以源码评审为准：普通窗口默认 `Sdr`，图片宿主明确请求 `AutomaticHdr`。不在每个按钮、主题或图片上维护窗口输出开关。

对`AutomaticHdr`窗口优先创建FP16／scRGB场景和swapchain；在HDR显示状态下呈现线性HDR，必要时按§5.4适配目标范围。SDR显示状态下，仅HDR图片执行SDR映射，UI及SDR图片按相对白合成，不能沿用HDR的绝对80nit缩放。FP16／scRGB是否可PRESENT由实际接口检查，不从屏幕开关推定。[W5][W6]

若系统／设备无法建立该输出，走现有 BGRA8 输出并明确使用 SDR 派生图，窗口状态报告回退原因。不要维护一份 SDR 场景加一份 HDR 场景：正常 scRGB 路径尽量直接绘制到可呈现目标。普通 `Sdr` 窗口不因引入 HDR 功能被强制增加整窗 FP16 中间目标。

这两种输出共用 scene、资源和 renderer，区别是目标格式、颜色常量和必要的输出处理。所有资源创建、resize、设备恢复和 readback 都必须使用实际格式。HDR 路线不首发另一套 PQ swapchain，也不机械调用 HDR10 metadata 接口。[W1]

### 6.2 SDR UI 白与 HDR 内容不能混为一谈

HDR显示状态下，Win32查询的SDR white原始值换算为`nits = SDRWhiteLevel × 80 / 1000`；此时scRGB中SDR UI白为`nits / 80`。SDR显示状态下则使用相对白1.0，不将这条HDR输出规则照搬过去。[W1][W8]

- 主题仍提供 sRGB 设计颜色，不把 nit 或显示器 profile 塞进 token。
- UI／SDR图像在HDR输出时只在线性化后乘一次参考白，在SDR输出时保持相对尺度；文字、emoji、SVG、gradient和阴影都需覆盖。
- 捕获所得绝对 scRGB 以及已转换的 PQ 图像不再乘 SDR white，避免重复提亮。
- source／result 比较使用同一窗口状态；改变 white level 不重设缩放、平移或 Compare 位置。

### 6.3 状态变化

Windows backend 为每窗口保留一个输出快照：显示器标识、当前 color kind、SDR white、能力值、实际输出与 generation；由窗口事件更新，不设常驻轮询。

1. 创建 HWND 后，能用时通过 `GetForWindow` 订阅 `AdvancedColorInfoChanged`；回调投递到拥有窗口／renderer 的线程并合并更新。[W3][W4]
2. 初始化、跨屏、`WM_DISPLAYCHANGE`、相关设置变化、恢复和设备丢失时刷新；窗口关闭时取消订阅。
3. 优先以`GetForWindow`关联的显示器为事实；DXGI补充查询枚举输出并匹配同一显示器，不能把`GetContainingOutput`当多GPU场景的唯一事实。窗口跨多屏维持一份窗口输出策略，不承诺一窗两种亮度映射。[W1][W3]
4. 同一次重配置内验证支持、重建必要资源、设置色彩空间、更新常量后重绘。失败进入有原因的 SDR 状态；新 generation 不能复用旧目标格式或旧映射结果。
5. 显示变化使窗口输出参数、目标和显示派生缓存失效；原始文件／绝对 scRGB 不失效。PaintSnapshot 回放使用当前窗口参数，不冻结旧显示器的亮度常量。

### 6.4 不能漏改的 renderer 部分

| 子路径 | 设计要求 |
| --- | --- |
| atlas | SDR8 与 RGBA16F 分桶；纹理 ID／key 可区分；按真实字节记账；设备丢失清理全部桶 |
| UI shader | HDR 模式采用真实分段 sRGB 转换；shader输出与 premultiplied blend state一致 |
| 文字 | coverage／gamma校正与颜色空间分别处理；先用灰阶抗锯齿完成 HDR 正确性，再证明 subpixel 的线性混合正确性，不能沿用旧常数碰运气 |
| emoji／SVG／渐变 | 识别 SDR 编码，转换一次；保留 gradient 的插值空间语义，dither幅度按输出单位定义 |
| backdrop | 在线性 FP16 中滤波；ROI捕获不能降为8位；按8字节重算预算，保留64MiB容量约束及正常底色回退 |
| 精确采样 | 完整UV、tile隔离、透明边缘、nearest、物理像素对齐都保留 |
| snapshot／回收 | 记录编码、格式和资源持有；Weak登记及最后持有者释放后回收继续有效 |
| readback | 新增带类型的高精度结果；原有 `RgbaImage` 接口若保留，明确定义为SDR结果并先转换，不能按四字节读取FP16 |

## 7. Yue 接口与展示状态

复用 `image_preview/` 模块，建议在 `resource.rs` 下按实际复杂度拆出 `color.rs`、色彩测试与fixture；不因本文表格数量创建同等数量的模块。

保留 `from_rgba8`／`from_bgra8` 的 SDR 意义，增加明确的线性 scRGB16F 输入；encoded 解码返回色彩已解析的资源或具体错误。删除含混的 `Unmanaged` 成功状态，不让未转换图像混入正常比较。

`ImagePreviewImage` 继续区分原始尺寸与纹理尺寸，HDR代理也必须保留HDR值。资源准备在宿主后台任务，结果通过 revision检查后一次更新；颜色变化不创建新相机，陈旧任务不能覆盖新结果。取消／错误保留上一份可用显示结果，但不能让旧结果状态冒充新结果。

显示状态分为内容和输出两个维度：

| 情况 | 显示说明 | 图片行为 |
| --- | --- | --- |
| HDR内容＋成功HDR输出 | `HDR`，详情提供当前输出信息 | 保留高光与扩展色域 |
| HDR内容＋SDR显示 | `SDR预览 · HDR原图保留` | 只呈现经过映射的派生结果 |
| SDR内容＋HDR窗口 | 普通SDR内容，无需增加醒目提示 | 按系统SDR白显示 |
| profile不支持／损坏 | 邻近预览的具体错误，可重新加载 | 不静默当作sRGB |
| 输出初始化失败 | `SDR预览`并提供原因 | 不丢原图，可继续明确导出 |

状态不只靠颜色，详细诊断放既有信息区／菜单；不堆 Alert，不加入“模拟 HDR”亮度滑块。原图／结果若编码策略不同，要标出 SDR结果，保证比较含义清楚。键盘、焦点、工具条、缩放和平移沿用现有合同；本轮不增加用于掩饰亮度切换的动画。

## 8. 导出、复制与格式能力

| 操作 | 第一轮合同 |
| --- | --- |
| HDR PNG保存 | RGB／RGBA 16位、Rec.2020／PQ、有效cICP；Alpha仍是线性覆盖率；不能表达的颜色明确拒绝，不宣称数学无损 |
| SDR PNG／JPEG／WebP／BMP | 显式SDR策略后编码，写正确色彩标签；已有格式能力和Alpha限制继续校验 |
| 原文件保存 | 不编辑且用户选择保留原文件时可直接复制，保留原有ICC／CICP，不经代理重编码 |
| HDR AVIF | 在消费方验证高位深解码、ICC解释、编码、许可和性能后启用；没有证明前显示不可用原因 |
| 普通复制 | 明确定义为SDR兼容复制；PNG与CF_DIBV5表达同一SDR结果，不把HDR PNG伪装成默认兼容粘贴 |
| HDR复制 | 单独命令／明确策略，发布带色彩定义的HDR PNG；是否提供SDR备用格式由接收端验证决定；提示目标应用可能按SDR解释 |

PNG第三版定义cICP、mDCV和cLLI；仅写16位像素不等于正确HDR。元数据须由真实内容或可靠来源产生，不能复制显示器峰值或凭空填写MaxCLL／MaxFALL。FP16→PNG16/PQ是量化／编码，不能承诺负scRGB和所有浮点值无损往返。[P1]

HDR PNG首轮默认只编码变换后处于Rec.2020／PQ有效范围的颜色；允许经数值测试确定的舍入容差。真正超出范围或10000nit时，返回具体的编码范围错误，原始母版仍保留；不默默clamp。以后若需要“映射到HDR编码范围”，应增加可见的独立策略及测试，不能混入“保留HDR”默认行为。

Windows支持发布多个剪贴板格式，但接收应用会自行选格式；注册名为PNG并不保证接收方会保留HDR。系统复制测试与浏览器／Office／图像软件的实际粘贴测试必须分开。不引入私有格式来宣称解决跨应用保真。[W9]

## 9. Rust 依赖决策

使用锁文件版本，不以“搜索到最新版”作为升级理由。任何新增直接依赖必须记录features、许可证、引入的重复版本及Windows MSVC构建结果。

| 工具 | 采用方式 | 明确不承担的职责 |
| --- | --- | --- |
| `windows`＋D3D11／DXGI／系统WinRT | 继续现有技术栈，按需补绑定features | 不是自动色彩管理方案 |
| `image 0.25.10`／`png 0.18.1` | 保留16位解码，使用PNG行接口及色彩chunk能力 | `into_rgba8`不能保留HDR；DynamicImage色彩标签不代表已经转换 [L2][L3] |
| `moxcms 0.8.1` | 优先验证现有锁定版本的ICC、U16／F32转换，减少重复依赖 | 不自动完成窗口HDR、tone mapping及绝对尺度解释 [L1] |
| `half` | FP16存储／F32转换，显式依赖及版本跟随锁文件核对 | 不是ICC／PQ／HDR显示库 [L5] |
| `gainforge` | 仅对比SDR映射质量、耗时、依赖图，过门禁才加入 | 不是直接HDR显示的前置步骤 [L4] |
| `pic-scale` | 现有面积滤波不能满足性能后，再比较U16／F32缩图 | 不因其F16接口升级nightly工具链 [L6] |
| `zenavif` | 只在SnapClip消费层评估现有AVIF入口；Yue不默认引入 | 不未经许可审查加入MIT组件默认依赖 [L7] |

优先在一次分块处理中完成所需转换，不能因用了SIMD库就自动承诺低CPU／高帧率。CPU与GPU实现通过同一数值向量约束，避免为了“复用代码”引入新的全局服务或让capture依赖UI。

## 10. 大图与资源预算

本地样本来自SnapClip的 [HDR-IMAGES清单](../../SnapClip/HDR-IMAGES/manifest.json)。四张均为8256×5504，45,441,024像素；HDR PNG为196.21MiB、SDR PNG为211.69MiB。AVIF文件较小不代表解码后图像较小。

| 单份像素缓冲，不含pitch／容量／其他副本 | RGBA8 | RGBA16F |
| --- | ---: | ---: |
| 3840×2160 | 31.64MiB | 63.28MiB |
| 8256×5504 | 173.34MiB | 346.69MiB |
| 4K三缓冲，仅swapchain像素 | 94.92MiB | 189.84MiB |

工程要求：

- 文件字节上限、解码峰值、代理纹理字节、存活源／旧版、atlas容量、backdrop和swapchain分别核算。现有64MiB纹理限制须按真实格式计算，FP16可容纳像素数为8位的一半。
- 本地两张PNG超过现有64MiB encoded上限；导入大文件应提供**文件流＋有界解码**入口及宿主明确预算，不能先读成200MiB `Vec`再声称流式，也不全局调大默认限制。
- 优先PNG逐行／块转换、统计和代理生成，避免同时常驻U16、F32、FP16整图及其克隆。AVIF是否支持流式／代理解码由解码器实际能力决定；不支持时先估峰值并拒绝超预算，不能假造行解码。
- 原始文件／磁盘母版仍是导出真源；显示代理不替代原图。当前8192边与64MiB代理不能承诺45MP原始细节逐像素显示；tiles等原细节加载需单独证据后设计。
- Weak资源回收、快照持有、取消任务、连续结果替换和多个窗口共同计入峰值。空闲不持续重绘；色彩统计不每帧重做。
- 捕获与GPUI未必共用device，`Arc<Texture>`不等于零拷贝。首轮允许一次受控readback／upload，记录拷贝次数及字节后再决定是否需要共享纹理和跨GPU同步。

## 11. 分批实施与测试顺序

每批遵循：**现状基线 → 新测试暴露缺口 → 实现 → 新增测试及既有回归 → 记录证据**。下表保留完整交付要求；各阶段实际状态如下，不能以“代码存在”替代门禁。

| 阶段 | 2026-11-03 实际状态 |
| --- | --- |
| P0 | 原创小样本、ICC 尺度探针、显示探针、固定 BT.2390 数学与独立向量已实现；HDR AVIF 的消费解码／许可／性能门禁未通过，继续不启用 |
| P1 | 三种真实编码、FP16 atlas、typed readback、逐行 PNG／受限 matrix-TRC ICC、线性代理和源峰值传递已实现；不支持的 ICC／HLG 明确拒绝 |
| P2 | opt-in `AutomaticHdr`、FP16/scRGB swapchain、能力／系统／输出分离、事件刷新、UI／文字／path／backdrop 和 SDR 读回已实现；WARP 与隐藏硬件 HWND 的切换／resize／读回／资源重建通过，真实 HDR 显示／跨屏／HDR 切换／驱动恢复仍未验收 |
| P3 | 独立源像素合同、typed ROI／上传／目标、FP16 行带底座、冻结帧来源 generation、typed 选区与圆角、压缩母版发布、WGC 格式合同及 D2D 内部 FP16 已实现；真实源能力／输出白快照、生产滚动／倒计时／取色、HDR 编码仍未完成，不能称 HDR 捕获已完成 |
| P4 | 待实施；SnapClip 仅适配新资源 API，当前 Pin 模型仍拒绝 HDR，不存在静默转 SDR 的“成功”入口 |
| P5 | 待实施；只做了单次 CPU 大图准备测量，未验证真实 HDR 屏、RSS／显存峰值或端到端性能 |

| 阶段 | 具体交付 | 升级前必须看到的缺口／升级后门禁 |
| --- | --- | --- |
| P0 证据与算法 | 小型PQ／scRGB／ICC fixture、纯数值参考、显示能力探针、tone-map和AVIF验证、资源账本 | 旧路径丢失>1／位深；确认ICC绝对尺度及候选库行为，冻结SDR映射参数 |
| P1 数据与资源 | RenderImage格式、atlas桶、typed readback、Yue高精度准备、代理合同 | FP16字节／stride测试先失败；新增格式往返及所有SDR资源回归通过 |
| P2 Windows输出 | scRGB目标、色彩空间、SDR白、事件／设备生命周期、完整UI／backdrop | WARP验证>1／负值、混合、采样；Debug／Release一致；普通组件与快照不回退 |
| P3 捕获和母版 | WGC＋D2D覆盖层、行带／滚动、无损FP16母版、HDR PNG导出、分析派生图 | 不能只验证枚举；浮点母版逐bit往返，PNG导出按量化容差验证 |
| P4 应用消费 | 保存面板、F3、F6统一图片展示、历史缩略图、SDR／HDR保存复制 | 新老工作流、状态说明、原位无激活与复制策略回归 |
| P5 性能与实机 | 同场景SDR／HDR对照、跨屏／HDR切换、DPI与驱动验证 | 无证据不标完整HDR支持；列明机器、设置、包版本和未覆盖项 |

P1／P2未完成时可合入内部类型改造，但不能把用户入口标成HDR已就绪。P3～P5不可用仅P1的库测试替代。

### 11.1 数值与GPU测试

| 测试层 | 必测内容 |
| --- | --- |
| 纯函数 | PQ正反转换覆盖0／100／203／1000／10000nit；scRGB白1.0／2.5375／12.5；原色矩阵、分段sRGB、黑、Alpha、无效元数据 |
| 资源 | U16不经u8、FP16格式／字节数、stride padding、溢出／NaN／Inf、代理保高光、取消与revision |
| 真实shader＋WARP | FP16目标>1与合法负数；HDR白按nit、SDR白为1；白系数仅乘一次；HDR→HDR适配；透明有色边、UI／emoji／渐变、backdrop、Compare、typed readback |
| 生命周期 | HDR↔SDR、white变化、跨屏、设备丢失、resize、snapshot回放、atlas驱逐、窗口关闭和重复资源替换 |
| 宿主行为 | 不因输出变化重设相机；旧结果不冒充新结果；格式与复制策略改变时发事件和编码各一次 |

GPU数值用F32 CPU参考和FP16的绝对／相对混合误差，按运算累计给出容差；不能要求所有中间值逐bit相等，也不能用“看起来差不多”代替容差。fixture包括透明HDR像素及超亮小区域，不能只测试整幅中灰。

### 11.2 升级前后的既有回归

实施前记录了工作区版本与本轮基线日志；保留原有未提交改动，没有通过 reset 制造干净基线。下列是回归命令，实际执行结果见证据记录，不能把命令列表本身当成已经通过。

```powershell
cargo test --workspace --locked --offline
cargo test --workspace --features headless-tests --locked --offline
cargo test -p yue-component --no-default-features --features headless-tests --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo clippy --workspace --all-targets --features headless-tests --locked --offline -- -D warnings
cargo fmt --all -- --check
pwsh -NoProfile -File tools/test-gpui-snapshot.ps1 -Offline
pwsh -NoProfile -File tools/test-gpui-backdrop.ps1
pwsh -NoProfile -File tools/test-gpui-backdrop.ps1 -Release
pwsh -NoProfile -File tools/test-gpui-hdr.ps1
pwsh -NoProfile -File tools/test-gpui-hdr.ps1 -Release
# opt-in 隐藏 HWND，不显示窗口或切换系统 HDR
pwsh -NoProfile -File tools/test-gpui-hdr.ps1 -Native
pwsh -NoProfile -File tools/test-gpui-hdr.ps1 -Native -Release
node tools/sync-yue-theme.mjs --check
```

`tools/test-gpui-hdr.ps1` 已实现，明确选择维护的 core patch、同一根 lock、生产 shader 和 Debug／Release，并执行 DirectX 回归及输出策略测试。八位采样、透明边缘、网格、相机、工具条、所有既有组件交互和SDR输出都需回归，不能删弱断言制造升级成功。

### 11.3 实机与性能门禁

实机矩阵至少覆盖HDR关／开、仅WCG、同窗切换、SDR与HDR双屏、不同SDR白、100／125／150／200%DPI、透明窗口与普通窗口、设备恢复；记录Windows build、GPU／驱动、显示器、HDR状态和文件hash。机器没有某条件就写未测，不推定通过。

使用Release同图同场景比较：首张可见延迟、解码／转换／代理／readback／upload分项、交互帧时间P50／P95、CPU／GPU时间、进程私有字节和GPU预算／峰值、20次替换后残留、空闲重绘次数。小图、4K和45MP大图分开记录。

暂定回归警戒：普通SDR窗口时间／占用相对基线上升超过10%需分析；HDR交互在固定60Hz验收机P95总帧时间目标≤16.7ms；相同viewport缩放／平移不重复上传，空闲重绘为0；清空资源并完成后续回收帧后不保留每一历史revision。以上是待测目标，不能据此承诺所有硬件表现；重复测量排除噪声后才能判定性能回退。

WARP能证明shader数值与生命周期，不能证明显示器实际峰值、DWM／驱动管理、真实显存或肉眼观感。作者提供的SDR对照可用于视觉比较，不是任意tone-map的逐字节ground truth。

## 12. 完成定义与下一步

完成定义：高精度从源到HDR屏、从源到HDR文件均不断；SDR回退明确且不覆盖母版；SDR UI不过亮；F3／F6／保存面板／覆盖层与滚动母版均有证据；新增数值／GPU测试和既有工作流回归通过；资源预算及实机测量有记录。一个环节只通过代码检查，不足以宣布“完整支持HDR”。

接续顺序：P2 真实 HDR 显示门禁仍保留；P3-B 已将色彩定义及来源 generation 传播到冻结帧、选区、行带接口和压缩母版发布（SnapClip docs/53 §9）。P3-C 已接通 WGC 三处格式合同、D3D typed 上传／目标和 D2D 内部 FP16／scRGB 链路（§10），离屏数值不替代真实 WGC 捕获与显示验收。接下来先接源能力／输出白快照，再改生产滚动分析／原始行带及导出，最后接 P4 消费入口。禁止仅切换 WGC 枚举、仅增加 `is_hdr` 或在不完整链路开放 HDR 保存。

手动查看现有输出代码：`cargo run -p yue-component --example hdr_preview --release --offline`；传入 PNG 路径可检查该文件。此诊断显式允许 256 MiB encoded 文件，组件与应用默认上限保持不变。默认原创测试色带、输出说明和真实显示器测试是不同层的证据；程序不会切换系统 HDR。大文件代理仅显示 Source，不从代理重新估计峰值来生成 SDR 对比。当前不修改系统 HDR 设置、不安装新渲染运行时。当前检查点：本仓库 `67ea607` 已交付共用 paint 入口；SnapClip `de39fa9` 已交付显式 SDR 数值取色，`e541c80` 已交付 HDR composition 倒计时。应用实现与实际验证层次以 SnapClip `docs/53-hdr-capture.md` §11、§14、§15 为准；F6 宿主替换、历史保真引用与回放、AVIF 门禁、资源性能和真实 HDR／跨屏／DPI 仍未完成，不能把底座与离屏测试等同于完整产品支持。

## 13. 无附加控件的共用绘制入口（2026-11-03）

`ImagePreviewImage::paint(viewport, image_bounds, radius, sampling, window)` 是现有图片预览组件的底层绘制入口。它只在 GPUI paint 阶段使用；宿主负责布局、宽高比和交互。没有新建媒体引擎、图片缓存或第二套相机。现有 `YueImagePreview` 也调用这个入口，继续保留原有的工具栏、比较裁切、背景、错误反馈及手势。

调用方可在 `canvas` 中直接绘制同一份不可变资源，用于原位贴图等没有页脚／工具栏的窗口。调用本身不解码、不重采样 CPU 缓冲、不复制原图；资源的 sRGB／linear SDR／scRGB 定义由原生 renderer 解释。Windows 平滑／最近邻路径与完整预览一致，代理纹理仍回退到平滑；错误通过 `ImagePreviewError::Render` 返回，非法几何明确拒绝。调用方必须处理错误，不能把成功创建空窗口等同于图像呈现成功。

验证使用[前置合同记录](performance/2026-11-03-pin-window-contracts.md)中的隔离依赖，叠加本节的 `paint.rs`／`model.rs`，不混入并行虚拟列表改造：纯 lib **354** 项、图片预览交互 **38** 项通过（原有 37 项和新增 1 项），相同范围 strict Clippy `--no-deps -D warnings` 通过。新增测试检查 100／125／150／200% 下 GPUI 实际分配的物理几何、圆角、HDR 标记、平滑／最近邻以及同一 atlas 资源复用。完整预览的比较裁切、缩放、焦点和卸载回归仍通过。修改前编译明确缺少此接口，证据为 `target/hdr-p3f-surface-before.log`；最终结果为 `target/hdr-p3f-{preview-final,preview-clippy}.log`。此项只交付共用呈现 API，**F6 窗口宿主迁移仍由 SnapClip `docs/53-hdr-capture.md` 跟踪**。

## 14. 原位窗口物理几何合同（2026-11-03）

Windows core 新增 `WindowOptions::physical_bounds`（外框虚拟桌面物理像素坐标）、`keep_physical_size_on_dpi_change`（默认 false）和 `Window::set_physical_bounds`。后者异步应用、不激活、不改 Z 序；普通窗口保持默认逻辑布局策略。hidden 创建也先应用所请求的几何，不能用尚未显示的默认大尺寸准备图片。`PopUp` 使用真正的 `WS_POPUP`；Windows `start_window_move` 在输入回调退出后进入系统移动循环，避免同步重入。

`WM_DPICHANGED` 在保留物理 client 时仍更新 logical viewport 与 scale；普通 suggested-rectangle 分支在没有 `WM_SIZE` 时也更新 viewport。SnapClip F6 在此基础上只调整 DIP 边框，源图片保持物理尺寸，不重新解码或 CPU 缩放。该 API 不提供跨平台保证，也不是对最终 DWM present 的成功回执。

验证继续隔离并行虚拟列表改造：普通 activation 测试 **1 passed / 2 ignored**，两个 native filter 分别在独立进程中各 **1 passed**。DPI 探针用自己的隐藏 HWND 合成 96/120/144/192/96 DPI 消息，比较保物理窗口与普通窗口负对照；临时禁用 preserve 分支时实际失败（变成 suggested 320×200），恢复后通过。它不是实屏切换或跨屏测试。日志：本仓 `target/hdr-f6-window-{pure,final}.log`、`target/hdr-f6-dpi-regression-red.log`。SnapClip 的 F6 生产画布原生 probe 另验 FP16、alpha、无激活、位置、滚轮资源复用和释放，详见消费者 `docs/53-hdr-capture.md` §16。

`tools/test-gpui-window.ps1 -Native` 为每个 native Application 启动独立测试进程，避免进程静态 Win32 类/dispatcher 的复用。不要将多个 native Application fixture 合并到一个 test runner。真实 F6 热键、系统鼠标移动、实际跨屏/DPI、HDR 最终屏幕亮度和设备恢复仍须单独验收。

## 15. 官方资料与版本

核对日期2026-11-03。规范／API结论与本项目工程选择分开；Rust引用固定到本文核对的版本或注明候选。

- [W1] Microsoft：[Use DirectX with Advanced Color on high/standard dynamic range displays](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/high-dynamic-range)。scRGB、SDR白、Win32输出与多屏。
- [W2] Microsoft：[Screen capture](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture)；[Direct3D11CaptureFramePool.CreateFreeThreaded](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.createfreethreaded)。捕获和frame pool像素格式；FP16全链路为本项目据此采用的工程合同。
- [W3] Microsoft：[IDisplayInformationStaticsInterop::GetForWindow](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.display.interop/nf-windows-graphics-display-interop-idisplayinformationstaticsinterop-getforwindow)。Win32窗口与最低系统版本。
- [W4] Microsoft：[DisplayInformation.GetAdvancedColorInfo](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.display.displayinformation.getadvancedcolorinfo)；[AdvancedColorInfoChanged](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.display.displayinformation.advancedcolorinfochanged)。颜色类型／白及变化通知。
- [W5] Microsoft：[CheckColorSpaceSupport](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_4/nf-dxgi1_4-idxgiswapchain3-checkcolorspacesupport)。
- [W6] Microsoft：[SetColorSpace1](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_4/nf-dxgi1_4-idxgiswapchain3-setcolorspace1)。
- [W7] Microsoft：[IDXGIOutput6::GetDesc1](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_6/nf-dxgi1_6-idxgioutput6-getdesc1)。
- [W8] Microsoft：[DISPLAYCONFIG_SDR_WHITE_LEVEL](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_sdr_white_level)。
- [W9] Microsoft：[Clipboard formats](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats)。多格式及接收方选择。
- [W10] Microsoft：[Using color management](https://learn.microsoft.com/en-us/windows/win32/direct2d/color-management)。Direct2D的颜色空间、context和高精度处理。
- [W11] Microsoft：[HDR Tone Mapping effect](https://learn.microsoft.com/en-us/windows/win32/direct2d/hdr-tone-map-effect)。内容／显示范围与HDR→HDR、HDR→SDR映射；该效果自Windows 10 1809可用，是否复用D2D效果由捕获覆盖层决定，GPUI HLSL仍须验证相同输出合同。
- [P1] W3C：[PNG, Third Edition](https://www.w3.org/TR/png-3/)。cICP优先级及mDCV／cLLI／ICC。
- [P2] AOMedia：[AV1 Image File Format](https://aomediacodec.github.io/av1-avif/)。AVIF容器和色彩属性。
- [C1] ITU：[BT.2100-3（02/2025）](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.2100-3-202502-I%21%21PDF-E.pdf)。PQ／HLG与HDR参数。
- [C2] ITU：[BT.2390-11（2023）](https://www.itu.int/dms_pub/itu-r/opb/rep/R-REP-BT.2390-11-2023-PDF-E.pdf)。本方案冻结参考该版本的技术报告，不宣称它是当前或未来最新版。
- [L1] 项目文档：[moxcms 0.8.1](https://docs.rs/moxcms/0.8.1/moxcms/)。
- [L2] 项目文档：[image 0.25.10 DynamicImage](https://docs.rs/image/0.25.10/image/enum.DynamicImage.html)。
- [L3] 项目文档：[png 0.18.1](https://docs.rs/png/0.18.1/png/)。
- [L4] 项目源码：[gainforge](https://github.com/awxkee/gainforge)。候选，尚未选定版本／引入。
- [L5] 项目文档：[half](https://docs.rs/half/)。以Cargo.lock实际版本为准。
- [L6] 项目文档：[pic-scale 0.7.12](https://docs.rs/pic-scale/0.7.12/pic_scale/)。候选，非新增依赖承诺。
- [L7] 项目文档及许可：[zenavif](https://docs.rs/crate/zenavif/latest)。AGPL／商业双许可，需按实际版本核对。
