use super::super::*;
use gpui_kit::{Entity, Subscription, rems};
use yue_component::{
    SliderControls, SliderDomain, SliderLayout, SliderMark, SliderSize, SliderTooltip, SliderValue,
    YueSegmented, YueSegmentedOption, YueSlider, YueSliderEvent, YueSliderState,
};

pub(crate) struct SliderPage {
    states: Vec<Entity<YueSliderState>>,
    changes: Vec<usize>,
    commits: Vec<usize>,
    range: bool,
    vertical: bool,
    size: SliderSize,
    disabled: bool,
    controls: SliderControls,
    tooltip: bool,
    marks: bool,
    _subscriptions: Vec<Subscription>,
}
impl SliderPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let standard = SliderDomain::default();
        let mut configs = vec![(standard, SliderValue::Single(40.)); 31];
        for ix in [1, 3, 5, 7, 13, 28] {
            configs[ix].1 = (20., 80.).into();
        }
        configs[11] = (SliderDomain::new(-20., 100.).step(5.), (0., 60.).into());
        configs[12] = (SliderDomain::new(-5., 5.).step(0.1), 0.5.into());
        configs[14] = (SliderDomain::new(3., 13.).step(2.), 7.0.into());
        configs[15] = (SliderDomain::new(0., 10.).step(3.), 9.0.into());
        configs[16] = (SliderDomain::new(0., 10.).step(20.), 0.0.into());
        configs[17] = (standard, (50., 50.).into());
        let states = configs
            .into_iter()
            .map(|(d, v)| cx.new(|cx| YueSliderState::new(d, v, window, cx)))
            .collect::<Vec<_>>();
        for ix in [6, 7, 30] {
            states[ix].update(cx, |s, cx| s.set_disabled(true, window, cx));
        }
        let subscriptions = states
            .iter()
            .enumerate()
            .map(|(ix, state)| {
                cx.subscribe(state, move |this, _, e, cx| {
                    match e {
                        YueSliderEvent::Change(_) => this.changes[ix] += 1,
                        YueSliderEvent::Commit(_) => this.commits[ix] += 1,
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            changes: vec![0; states.len()],
            commits: vec![0; states.len()],
            states,
            range: false,
            vertical: false,
            size: SliderSize::Medium,
            disabled: false,
            controls: SliderControls::Input,
            tooltip: true,
            marks: false,
            _subscriptions: subscriptions,
        }
    }
    fn control(&self, ix: usize, theme: YueTheme, label: &'static str) -> YueSlider {
        YueSlider::new(format!("slider-demo-{ix}"), theme, &self.states[ix])
            .accessibility_label(label)
            .lower_label(format!("{label}下限"))
            .upper_label(format!("{label}上限"))
            .input_error_message("请输入有效数字")
            .value_format(|v| format!("{v}").into())
    }
    fn status(&self, ix: usize, theme: YueTheme, cx: &Context<Self>) -> impl IntoElement {
        readout(
            theme,
            format!(
                "当前值 {} · Change {} · Commit {}",
                self.states[ix].read(cx).value(cx),
                self.changes[ix],
                self.commits[ix]
            ),
        )
    }
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().items_start().gap_4().w_full()
}
fn marks() -> [SliderMark; 3] {
    [
        SliderMark::new(0., "0"),
        SliderMark::new(50., "50"),
        SliderMark::new(100., "100"),
    ]
}

impl Render for SliderPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let row = || div().flex().flex_wrap().items_center().gap_4();
        let mut options = row()
            .child(
                YueSegmented::new(
                    "slider-mode",
                    theme,
                    if self.range { "range" } else { "single" },
                )
                .label("滑动条模式")
                .options([
                    YueSegmentedOption::new("single", "单值"),
                    YueSegmentedOption::new("range", "范围"),
                ])
                .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                    this.range = value.as_ref() == "range";
                    cx.notify();
                })),
            )
            .child(
                YueSegmented::new(
                    "slider-direction",
                    theme,
                    if self.vertical {
                        "vertical"
                    } else {
                        "horizontal"
                    },
                )
                .label("方向")
                .options([
                    YueSegmentedOption::new("horizontal", "水平"),
                    YueSegmentedOption::new("vertical", "垂直"),
                ])
                .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                    this.vertical = value.as_ref() == "vertical";
                    cx.notify();
                })),
            )
            .child(
                YueSegmented::new(
                    "slider-size",
                    theme,
                    match self.size {
                        SliderSize::Small => "small",
                        SliderSize::Medium => "medium",
                        SliderSize::Large => "large",
                    },
                )
                .label("尺寸")
                .options([
                    YueSegmentedOption::new("small", "小"),
                    YueSegmentedOption::new("medium", "中"),
                    YueSegmentedOption::new("large", "大"),
                ])
                .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                    this.size = match value.as_ref() {
                        "small" => SliderSize::Small,
                        "large" => SliderSize::Large,
                        _ => SliderSize::Medium,
                    };
                    cx.notify();
                })),
            );
        options = options.child(
            YueSegmented::new(
                "slider-controls",
                theme,
                match self.controls {
                    SliderControls::None => "none",
                    SliderControls::Input => "input",
                    SliderControls::Buttons => "buttons",
                },
            )
            .label("辅助控件")
            .options([
                YueSegmentedOption::new("none", "无"),
                YueSegmentedOption::new("input", "数字输入框"),
                YueSegmentedOption::new("buttons", "两端 −/+"),
            ])
            .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                this.controls = match value.as_ref() {
                    "input" => SliderControls::Input,
                    "buttons" => SliderControls::Buttons,
                    _ => SliderControls::None,
                };
                cx.notify();
            })),
        );
        for (key, label, checked) in [
            ("disabled", "禁用", self.disabled),
            ("tooltip", "数值提示", self.tooltip),
            ("marks", "刻度", self.marks),
        ] {
            let page = cx.entity();
            options = options.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        YueSwitch::new(format!("slider-preview-{key}"), theme, checked, label)
                            .size(SwitchSize::Small)
                            .on_change(move |checked, _, window, cx| {
                                page.update(cx, |this, cx| {
                                    match key {
                                        "disabled" => {
                                            this.disabled = checked;
                                            for state in &this.states[..2] {
                                                state.update(cx, |s, cx| {
                                                    s.set_disabled(checked, window, cx)
                                                });
                                            }
                                        }
                                        "tooltip" => this.tooltip = checked,
                                        _ => this.marks = checked,
                                    }
                                    cx.notify();
                                })
                            }),
                    )
                    .child(div().text_sm().child(label)),
            );
        }
        let slot = usize::from(self.range);
        let mut preview = self
            .control(slot, theme, "交互预览")
            .size(self.size)
            .controls(self.controls)
            .layout(if self.vertical {
                SliderLayout::Vertical
            } else {
                SliderLayout::Horizontal
            })
            .tooltip(if self.tooltip {
                SliderTooltip::Auto
            } else {
                SliderTooltip::None
            });
        if self.marks {
            preview = preview.marks(marks());
        }
        let mut page=column().debug_selector(||"slider-content".into()).p_8().gap_6()
            .child(div().text_2xl().child("Slider 滑动条"))
            .child(readout(theme,"按住轨道或滑块后移出边界，仍可继续拖动；松手结束一次编辑。方向键、Home/End、PageUp/PageDown 可调节焦点滑块。".into()))
            .child(section(theme,"交互预览","拖动位置直接跟随指针与步长；动画只用于边框、光晕和按压外观",column().child(options).child(preview).child(self.status(slot,theme,cx))))
            .child(section(theme,"基础与禁用","单值与范围；禁用保留选中段和滑块轮廓",column()
                .child(self.control(2,theme,"基础音量").value_format(|v|format!("{v}%").into()))
                .child(self.control(3,theme,"基础范围"))
                .child(self.control(6,theme,"禁用音量").controls(SliderControls::Input))
                .child(self.control(7,theme,"禁用范围").controls(SliderControls::Input))))
            .child(section(theme,"垂直方向","底部为最小值，顶部为最大值；垂直布局的数字框位于下方",row()
                .child(self.control(4,theme,"垂直音量").layout(SliderLayout::Vertical).controls(SliderControls::Input))
                .child(self.control(5,theme,"垂直范围").layout(SliderLayout::Vertical).height(rems(14.)).marks(marks()).controls(SliderControls::Input))));
        let mut sizes = column();
        for (ix, size, label) in [
            (8, SliderSize::Small, "Small"),
            (9, SliderSize::Medium, "Medium"),
            (10, SliderSize::Large, "Large"),
        ] {
            sizes = sizes.child(div().text_sm().child(label)).child(
                self.control(ix, theme, label)
                    .size(size)
                    .controls(SliderControls::Input),
            );
        }
        let mut button_examples = column();
        for (ix, size, label) in [
            (25, SliderSize::Small, "Small"),
            (26, SliderSize::Medium, "Medium"),
            (27, SliderSize::Large, "Large"),
        ] {
            button_examples = button_examples
                .child(div().text_sm().child(label))
                .child(
                    self.control(ix, theme, label)
                        .size(size)
                        .controls(SliderControls::Buttons),
                )
                .child(self.status(ix, theme, cx));
        }
        button_examples = button_examples
            .child(readout(
                theme,
                "范围：默认调节上限，操作下限滑块后改为调节下限；到达边界后对应按钮禁用。".into(),
            ))
            .child(
                self.control(28, theme, "按钮范围")
                    .controls(SliderControls::Buttons)
                    .marks(marks()),
            )
            .child(self.status(28, theme, cx))
            .child(
                self.control(29, theme, "垂直按钮")
                    .controls(SliderControls::Buttons)
                    .layout(SliderLayout::Vertical),
            )
            .child(
                self.control(30, theme, "禁用按钮")
                    .controls(SliderControls::Buttons),
            );
        page=page.child(section(theme,"尺寸","数值居中，数字框宽度与编辑区内边距独立控制；按压不改变排版",sizes))
            .child(section(theme, "两端步进按钮", "− / 轨道 / + / 无箭头数字框；输入即时更新，越界文字与边框标红", button_examples))
            .child(section(theme,"刻度与温度","靠近刻度才吸附，移开继续自由拖动；键盘与输入保留 step，重叠标签优先保留端点",column()
                .child(self.control(11,theme,"温度范围").controls(SliderControls::Input).value_format(|v|format!("{v}°C").into())
                    .marks([SliderMark::new(-20.,"−20°C"),SliderMark::new(0.,"0°C"),SliderMark::new(37.,"体温 37°C"),SliderMark::new(60.,"60°C"),SliderMark::new(100.,"100°C")]))
                .child(self.status(11,theme,cx))))
            .child(section(theme,"精确输入","合法输入即时更新；越界标红，Enter 结束编辑，Escape 或失焦恢复最近合法值。Tab 遵循 Base 的缩进合同",column()
                .child(self.control(12,theme,"小数增益").controls(SliderControls::Input)).child(self.status(12,theme,cx))
                .child(self.control(13,theme,"输入范围").controls(SliderControls::Input)).child(self.status(13,theme,cx))))
            .child(section(theme,"数值边界","min 对齐；max 总是可达；范围端点不交叉、不交换身份",column()
                .child(readout(theme,"3..13 / step 2；0..10 / step 3；0..10 / step 20".into()))
                .child(self.control(14,theme,"非零起点").controls(SliderControls::Input)).child(self.control(15,theme,"非整除步长").controls(SliderControls::Input))
                .child(self.control(16,theme,"超跨度步长").controls(SliderControls::Input))
                .child(readout(theme,"重合范围：首次拖动选择上限，此后优先最近活动端点。Tab 可独立聚焦两个滑块。".into()))
                .child(self.control(17,theme,"重合范围").controls(SliderControls::Input))))
            .child(section(theme,"宽度与窄布局","默认 320px；block 显式占满父级；数字框在窄宽下换行",column()
                .child(div().w(rems(12.)).child(self.control(18,theme,"窄容器").controls(SliderControls::Input).marks(marks()).block()))
                .child(self.control(19,theme,"全宽滑动条").block())));
        let mut profiles = row();
        for (ix, (mode, accent, label)) in [
            (ThemeMode::Light, Accent::Azure, "Light / Azure"),
            (ThemeMode::Dark, Accent::Azure, "Dark / Azure"),
            (ThemeMode::Light, Accent::Neutral, "Light / Neutral"),
            (ThemeMode::Dark, Accent::Neutral, "Dark / Neutral"),
        ]
        .into_iter()
        .enumerate()
        {
            let t = YueTheme::new(mode, accent);
            profiles = profiles.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded_md()
                    .bg(gpui_kit::rgb(t.page()))
                    .text_color(gpui_kit::rgb(t.text_primary()))
                    .child(label)
                    .child(
                        self.control(20 + ix, t, label)
                            .width(rems(16.))
                            .marks(marks()),
                    ),
            );
        }
        page.child(section(
            theme,
            "主题",
            "四个主题使用同一组 Slider tokens",
            profiles,
        ))
        .child(section(
            theme,
            "宿主同步",
            "外部赋值和 domain 更新静默执行，取消旧事务；同值回写保留正在进行的拖动",
            column()
                .child(
                    self.control(24, theme, "宿主设置")
                        .controls(SliderControls::Input),
                )
                .child(self.status(24, theme, cx))
                .child(
                    row()
                        .child(
                            YueButton::text("slider-reset", theme, "重置到 40").on_click(
                                cx.listener(|this, _, w, cx| {
                                    this.states[24].update(cx, |s, cx| {
                                        s.set_domain(SliderDomain::default(), w, cx);
                                        s.set_value(40., w, cx);
                                    });
                                }),
                            ),
                        )
                        .child(
                            YueButton::text("slider-domain", theme, "改为 −5..5 / step 0.5")
                                .on_click(cx.listener(|this, _, w, cx| {
                                    this.states[24].update(cx, |s, cx| {
                                        s.set_domain(SliderDomain::new(-5., 5.).step(0.5), w, cx)
                                    });
                                })),
                        )
                        .child(
                            YueButton::text("slider-max", theme, "设置为当前最大值").on_click(
                                cx.listener(|this, _, w, cx| {
                                    this.states[24]
                                        .update(cx, |s, cx| s.set_value(s.domain(cx).max(), w, cx));
                                }),
                            ),
                        ),
                ),
        ))
    }
}
