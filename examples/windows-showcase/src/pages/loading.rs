use super::super::*;
use std::time::Duration;
use yue_component::{
    FeedbackSize, ProgressShape, ProgressStatus, ProgressValue, SkeletonAnimation, SkeletonShape,
    YueLoadingRegion, YueProgress, YueSkeleton, YueSpinner,
};

#[derive(Clone, Copy)]
pub(crate) enum FeedbackPageKind {
    Progress,
    Spinner,
    Skeleton,
}

pub(crate) struct FeedbackPage {
    kind: FeedbackPageKind,
    percent: f32,
    unknown: bool,
    animated: bool,
    status: ProgressStatus,
    active: bool,
    task: Option<gpui_kit::Task<()>>,
    delay: bool,
    loading: bool,
    animation: SkeletonAnimation,
}
impl FeedbackPage {
    pub(crate) fn new(kind: FeedbackPageKind) -> Self {
        Self {
            kind,
            percent: 42.,
            unknown: false,
            animated: true,
            status: ProgressStatus::Normal,
            active: false,
            task: None,
            delay: true,
            loading: true,
            animation: SkeletonAnimation::Wave,
        }
    }
    fn start(&mut self, duration: Duration, cx: &mut Context<Self>) {
        self.task = None;
        self.active = true;
        self.task = Some(cx.spawn(async move |weak, cx| {
            cx.background_executor().timer(duration).await;
            let _ = weak.update(cx, |p, cx| {
                p.active = false;
                cx.notify();
            });
        }));
        cx.notify();
    }
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().gap_4().min_w_0()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_4()
}
fn placeholder(animation: SkeletonAnimation) -> gpui_kit::AnyElement {
    column()
        .child(
            row()
                .child(
                    YueSkeleton::new("profile-avatar")
                        .shape(SkeletonShape::Circle)
                        .animation(animation),
                )
                .child(
                    column()
                        .child(
                            YueSkeleton::new("profile-name")
                                .width(gpui_kit::rems(12.))
                                .animation(animation),
                        )
                        .child(
                            YueSkeleton::new("profile-description")
                                .width(gpui_kit::rems(18.))
                                .animation(animation),
                        ),
                ),
        )
        .child(
            YueSkeleton::new("profile-image")
                .shape(SkeletonShape::Rounded)
                .width(gpui_kit::rems(24.))
                .height(gpui_kit::rems(6.))
                .animation(animation),
        )
        .child(
            YueSkeleton::new("profile-detail")
                .width(gpui_kit::rems(22.))
                .animation(animation),
        )
        .into_any_element()
}
impl Render for FeedbackPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut body = column().p_8().gap_6();
        match self.kind {
            FeedbackPageKind::Progress => {
                let value = if self.unknown {
                    ProgressValue::indeterminate()
                } else {
                    ProgressValue::new(self.percent)
                };
                let controls = row()
                    .child(
                        YueButton::new("progress-minus", "减少 20%").on_click(cx.listener(
                            |p, _, _, cx| {
                                p.percent = (p.percent - 20.).max(0.);
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        YueButton::new("progress-plus", "增加 20%").on_click(cx.listener(
                            |p, _, _, cx| {
                                p.percent = (p.percent + 20.).min(100.);
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        YueButton::new("progress-reset", "重置到 0%").on_click(cx.listener(
                            |p, _, _, cx| {
                                p.percent = 0.;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        YueButton::new("progress-full", "设为 100%").on_click(cx.listener(
                            |p, _, _, cx| {
                                p.percent = 100.;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        YueSwitch::new("progress-unknown", theme, self.unknown, "未知进度")
                            .on_change({
                                let entity = cx.entity();
                                move |v, _, _, cx| {
                                    entity.update(cx, |p, cx| {
                                        p.unknown = v;
                                        cx.notify();
                                    })
                                }
                            }),
                    )
                    .child(
                        YueSwitch::new("progress-animation", theme, self.animated, "播放动效")
                            .on_change({
                                let entity = cx.entity();
                                move |v, _, _, cx| {
                                    entity.update(cx, |p, cx| {
                                        p.animated = v;
                                        cx.notify();
                                    })
                                }
                            }),
                    );
                let statuses = row().children(
                    [
                        ("normal", "进行中", ProgressStatus::Normal),
                        ("success", "成功", ProgressStatus::Success),
                        ("error", "失败", ProgressStatus::Error),
                        ("paused", "暂停", ProgressStatus::Paused),
                    ]
                    .map(|(id, label, status)| {
                        YueButton::new(id, label).on_click(cx.listener(move |p, _, _, cx| {
                            p.status = status;
                            cx.notify();
                        }))
                    }),
                );
                let sizes = column().children(
                    [
                        ("small", FeedbackSize::Small),
                        ("medium", FeedbackSize::Medium),
                        ("large", FeedbackSize::Large),
                    ]
                    .map(|(id, size)| {
                        row()
                            .child(
                                div().w(gpui_kit::rems(24.)).child(
                                    YueProgress::new(
                                        (gpui_kit::ElementId::from("linear"), id),
                                        value,
                                    )
                                    .label("上传附件")
                                    .size(size)
                                    .status(self.status)
                                    .animated(self.animated),
                                ),
                            )
                            .child(
                                YueProgress::new((gpui_kit::ElementId::from("circle"), id), value)
                                    .shape(ProgressShape::Circle)
                                    .accessibility_label("上传附件环形进度")
                                    .size(size)
                                    .status(self.status)
                                    .animated(self.animated),
                            )
                    }),
                );
                body=body.debug_selector(||"progress-content".into()).child(div().text_2xl().child("Progress 进度"))
                    .child(section(theme,"受控进度","比例由宿主提供；正向和反向更新均从当前绘制位置衔接。",column().child(controls).child(statuses).child(sizes)))
                    .child(section(theme,"状态与百分比独立","100% 不会自动隐藏；失败和暂停有状态文字，未知进度不伪造数值。",column()
                        .child(YueProgress::new("empty",ProgressValue::new(0.)).label("尚未开始"))
                        .child(YueProgress::new("complete",ProgressValue::new(100.)).label("附件已上传").status(ProgressStatus::Success))
                        .child(YueProgress::new("failed-hidden-value",ProgressValue::new(60.)).label("上传中断").status(ProgressStatus::Error).show_value(false))))
                    .child(readout(theme,"确定值过渡 150ms；环形旋转 900ms；线形移动 1800ms。暂停时保留静态短段，减少动效时直接到位。".into()));
            }
            FeedbackPageKind::Spinner => {
                let controls = row()
                    .child(
                        YueButton::new("spinner-short", "执行 80ms 短任务").on_click(
                            cx.listener(|p, _, _, cx| p.start(Duration::from_millis(80), cx)),
                        ),
                    )
                    .child(
                        YueButton::new("spinner-long", "执行 2s 任务 / 重新开始").on_click(
                            cx.listener(|p, _, _, cx| p.start(Duration::from_secs(2), cx)),
                        ),
                    )
                    .child(
                        YueButton::new("spinner-cancel", "结束任务").on_click(cx.listener(
                            |p, _, _, cx| {
                                p.task = None;
                                p.active = false;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        YueSwitch::new("spinner-delay", theme, self.delay, "延迟 150ms 显示")
                            .on_change({
                                let entity = cx.entity();
                                move |v, _, _, cx| {
                                    entity.update(cx, |p, cx| {
                                        p.delay = v;
                                        cx.notify();
                                    })
                                }
                            }),
                    );
                body = body
                    .debug_selector(|| "spinner-content".into())
                    .child(div().text_2xl().child("Spinner 旋转指示器"))
                    .child(section(
                        theme,
                        "短任务不闪烁",
                        "默认门限 150ms。真实完成立即退出，不强制最短显示时间。",
                        column()
                            .child(controls)
                            .child(
                                div().min_h(gpui_kit::rems(3.)).child(
                                    YueSpinner::new("task-spinner")
                                        .active(self.active)
                                        .label("正在搜索文件")
                                        .delay(if self.delay {
                                            Duration::from_millis(150)
                                        } else {
                                            Duration::ZERO
                                        }),
                                ),
                            )
                            .child(if self.active {
                                "任务执行中"
                            } else {
                                "当前没有任务"
                            }),
                    ))
                    .child(section(
                        theme,
                        "三个尺寸",
                        "局部等待，使用文案解释实际工作。",
                        row().children(
                            [
                                ("sm", FeedbackSize::Small),
                                ("md", FeedbackSize::Medium),
                                ("lg", FeedbackSize::Large),
                            ]
                            .map(|(id, size)| {
                                YueSpinner::new(id)
                                    .size(size)
                                    .delay(Duration::ZERO)
                                    .label("正在同步")
                            }),
                        ),
                    ))
                    .child(section(
                        theme,
                        "静态与装饰模式",
                        "在已有命名的 busy 容器内使用 decorative，避免重复进度语义。",
                        row()
                            .child(
                                YueSpinner::new("static-spinner")
                                    .animated(false)
                                    .delay(Duration::ZERO)
                                    .label("静态等待"),
                            )
                            .child(
                                YueSpinner::new("decorative-spinner")
                                    .decorative(true)
                                    .delay(Duration::ZERO),
                            ),
                    ))
                    .child(readout(
                        theme,
                        "旋转 900ms，共用单调时钟；失活窗口、视口外和减少动效时停止连续帧请求。"
                            .into(),
                    ));
            }
            FeedbackPageKind::Skeleton => {
                let animation = self.animation;
                let controls = row()
                    .children(
                        [
                            ("wave", "扫光", SkeletonAnimation::Wave),
                            ("pulse", "呼吸", SkeletonAnimation::Pulse),
                            ("none", "静态", SkeletonAnimation::None),
                        ]
                        .map(|(id, label, animation)| {
                            YueButton::new(id, label).on_click(cx.listener(move |p, _, _, cx| {
                                p.animation = animation;
                                cx.notify();
                            }))
                        }),
                    )
                    .child(
                        YueSwitch::new("skeleton-loading", theme, self.loading, "显示骨架")
                            .on_change({
                                let entity = cx.entity();
                                move |v, _, _, cx| {
                                    entity.update(cx, |p, cx| {
                                        p.loading = v;
                                        cx.notify();
                                    })
                                }
                            }),
                    );
                body=body.debug_selector(||"skeleton-content".into()).child(div().text_2xl().child("Skeleton 骨架屏"))
                    .child(section(theme,"内容替换","只构建当前分支。骨架退出后直接呈现真实内容，旧分支立即离开交互树。",column().child(controls)
                        .child(YueLoadingRegion::new("profile-region",self.loading).label("正在加载个人资料")
                            .placeholder(move |_,_|placeholder(animation))
                            .content(|_,_|column().child("个人资料已就绪").child("骨架与真实内容应共享布局，减少跳动。")
                                .child(YueButton::new("profile-action","查看资料")).into_any_element()))))
                    .child(section(theme,"四种形状","Text 占据完整行高，Circle 始终按短边绘制正圆。",row().children([
                        ("text",SkeletonShape::Text),("rectangle",SkeletonShape::Rectangle),("rounded",SkeletonShape::Rounded),("circle",SkeletonShape::Circle)
                    ].map(|(id,shape)|column().child(id).child(YueSkeleton::new(id).shape(shape).animation(animation))))))
                    .child(section(theme,"同组同步","所有实例共享相位；新挂载项加入当前扫光，不从头播放。",placeholder(animation)))
                    .child(readout(theme,"扫光 / 呼吸周期 1800ms。骨架不增加焦点和虚假控件语义，由 LoadingRegion 提供一次加载状态。".into()));
            }
        }
        body
    }
}
