use super::super::*;
use gpui_kit::{Entity, Task, base::input::InputState, rems};
use std::time::Duration;
use yue_component::{
    SelectMode, SelectOption, SelectValue, StepsLabelPlacement, StepsLayout, StepsSize,
    StepsStatus, StepsValue, StepsVariant, YueCard, YueCardHeader, YueEmpty, YueSelect,
    YueSelectState, YueStepItem, YueSteps,
};

pub(crate) struct StepsPage {
    current: StepsValue,
    size: StepsSize,
    vertical: bool,
    below: bool,
    reverse: bool,
    readonly: bool,
    disabled: bool,
    reject: bool,
    error: bool,
    variant: StepsVariant,
    stage: usize,
    completed: bool,
    busy: bool,
    failed: bool,
    fail_next: bool,
    task: Option<Task<()>>,
    generation: u64,
    message: SharedString,
    extra: usize,
    source: Entity<YueSelectState>,
    name: Entity<InputState>,
    form: Entity<YueForm>,
}
impl StepsPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(w, cx).placeholder("例如：十一月工单"));
        let source = cx.new(|cx| {
            let mut s = YueSelectState::new(SelectMode::Single, w, cx);
            s.set_options(
                [
                    SelectOption::new("work-orders", "工单示例 · 24 条"),
                    SelectOption::new("contacts", "联系人示例 · 12 条"),
                ],
                cx,
            )
            .unwrap();
            s.set_value(SelectValue::Single(Some("work-orders".into())), w, cx)
                .unwrap();
            s
        });
        let form = cx.new(|cx| {
            let mut f = YueForm::new();
            f.require("batch", &name, "请填写批次名称");
            f.attach(&name, w, cx);
            f
        });
        Self {
            current: "configure".into(),
            size: StepsSize::Medium,
            vertical: false,
            below: false,
            reverse: false,
            readonly: false,
            disabled: false,
            reject: false,
            error: false,
            variant: StepsVariant::Default,
            stage: 0,
            completed: false,
            busy: false,
            failed: false,
            fail_next: false,
            task: None,
            generation: 0,
            message: "选择示例数据，配置批次后导入。".into(),
            extra: 0,
            source,
            name,
            form,
        }
    }
    fn go(&mut self, target: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.busy || target > 2 {
            return;
        }
        if target == 2 && !self.form.update(cx, |f, cx| f.submit(w, cx)) {
            self.message = "批次名称未填写，修正后再继续。".into();
            cx.notify();
            return;
        }
        self.stage = target;
        self.completed = false;
        self.failed = false;
        self.message = "配置保留，可以返回修改。".into();
        cx.notify();
    }
    fn import(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.generation += 1;
        let generation = self.generation;
        let fail = self.fail_next;
        self.fail_next = false;
        self.busy = true;
        self.failed = false;
        self.message = "正在导入示例数据…".into();
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(900))
                .await;
            let _ = weak.update_in(cx, |p, _, cx| {
                if p.generation != generation {
                    return;
                }
                p.busy = false;
                p.failed = fail;
                p.completed = !fail;
                p.task = None;
                p.message = if fail {
                    "导入失败，批次与数据保留，可重试。".into()
                } else {
                    format!("批次「{}」已导入完成。", p.name.read(cx).value()).into()
                };
                cx.notify();
            });
        }));
        cx.notify();
    }
}
fn row() -> gpui_kit::Div {
    div().flex().flex_wrap().items_center().gap_3()
}
fn sample_items() -> [YueStepItem; 3] {
    [
        YueStepItem::new("source", "选择数据").description("选择需要导入的示例数据"),
        YueStepItem::new("configure", "配置批次")
            .subtitle("必填")
            .description("确认名称、字段与导入选项"),
        YueStepItem::new("result", "导入结果").description("查看完成状态，失败可以重试"),
    ]
}
fn section(id: &'static str, title: &str, content: impl IntoElement) -> YueCard {
    YueCard::new(id)
        .header(YueCardHeader::new(title.to_owned()))
        .child(content)
}
impl Render for StepsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let controls = row()
            .child(
                YueButton::new(
                    "steps-variant",
                    match self.variant {
                        StepsVariant::Default => "外观：标准",
                        StepsVariant::Dot => "外观：点状",
                        StepsVariant::Navigation => "外观：导航",
                        StepsVariant::Arrow => "外观：箭头",
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.variant = match p.variant {
                        StepsVariant::Default => StepsVariant::Dot,
                        StepsVariant::Dot => StepsVariant::Navigation,
                        StepsVariant::Navigation => StepsVariant::Arrow,
                        StepsVariant::Arrow => StepsVariant::Default,
                    };
                    if matches!(p.variant, StepsVariant::Navigation | StepsVariant::Arrow) {
                        p.vertical = false;
                        p.below = false;
                    }
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-size",
                    match self.size {
                        StepsSize::Small => "尺寸：小",
                        StepsSize::Medium => "尺寸：中",
                        StepsSize::Large => "尺寸：大",
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.size = match p.size {
                        StepsSize::Small => StepsSize::Medium,
                        StepsSize::Medium => StepsSize::Large,
                        StepsSize::Large => StepsSize::Small,
                    };
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-layout",
                    if self.vertical {
                        "改为水平"
                    } else {
                        "改为垂直"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.vertical = !p.vertical;
                    p.variant = StepsVariant::Default;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-label",
                    if self.below {
                        "文字在右"
                    } else {
                        "文字在下"
                    },
                )
                .disabled(self.vertical)
                .on_click(cx.listener(|p, _, _, cx| {
                    p.below = !p.below;
                    p.variant = StepsVariant::Default;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-reverse",
                    if self.reverse {
                        "恢复顺序"
                    } else {
                        "逆序显示"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.reverse = !p.reverse;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-readonly",
                    if self.readonly {
                        "恢复交互"
                    } else {
                        "只读展示"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.readonly = !p.readonly;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-disabled",
                    if self.disabled {
                        "启用"
                    } else {
                        "禁用整组"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.disabled = !p.disabled;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-error",
                    if self.error {
                        "清除错误"
                    } else {
                        "当前错误"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.error = !p.error;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "steps-reject",
                    if self.reject {
                        "接受切换"
                    } else {
                        "拒绝切换"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.reject = !p.reject;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("steps-pending", "全等待").on_click(cx.listener(|p, _, _, cx| {
                    p.current = StepsValue::Pending;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("steps-complete", "全完成").on_click(cx.listener(|p, _, _, cx| {
                    p.current = StepsValue::Complete;
                    cx.notify();
                })),
            );
        let playground = YueSteps::new("steps-playground", self.current.clone())
            .items(sample_items())
            .size(self.size)
            .layout(if self.vertical {
                StepsLayout::Vertical
            } else {
                StepsLayout::Horizontal
            })
            .label_placement(if self.below {
                StepsLabelPlacement::Below
            } else {
                StepsLabelPlacement::Right
            })
            .variant(self.variant)
            .reverse(self.reverse)
            .readonly(self.readonly)
            .disabled(self.disabled)
            .status(if self.error {
                StepsStatus::Error
            } else {
                StepsStatus::Process
            })
            .on_change(cx.listener(|p, e: &yue_component::StepsChange, _, cx| {
                p.message = if p.reject {
                    "已拒绝切换，步骤保持原值。".into()
                } else {
                    p.current = e.key().clone().into();
                    format!("切换至第 {} 步", e.index() + 1).into()
                };
                cx.notify();
            }));
        let stage_keys = ["source", "configure", "result"];
        let workflow = YueSteps::new(
            "steps-workflow",
            if self.completed {
                StepsValue::Complete
            } else {
                stage_keys[self.stage].into()
            },
        )
        .items(sample_items())
        .disabled(self.busy)
        .status(if self.failed {
            StepsStatus::Error
        } else {
            StepsStatus::Process
        })
        .on_change(cx.listener(|p, e: &yue_component::StepsChange, w, cx| p.go(e.index(), w, cx)));
        let content = match self.stage {
            0 => div()
                .child(
                    YueField::new(theme)
                        .label("数据来源")
                        .select(YueSelect::new("steps-source", &self.source)),
                )
                .child(div().mt_3().child("使用内置示例数据，导入操作不写入磁盘。")),
            1 => div().child(
                YueField::new(theme)
                    .label("批次名称")
                    .required(true)
                    .bind(&self.form, "batch")
                    .input(YueInput::new("steps-batch", theme, &self.name)),
            ),
            _ => div().child(if self.completed {
                YueEmpty::new("steps-success")
                    .icon(outline::CircleCheck)
                    .title("导入完成")
                    .description(self.message.clone())
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(format!("批次：{}", self.name.read(cx).value()))
                    .child(format!(
                        "数据源：{}",
                        self.source
                            .read(cx)
                            .selected_items()
                            .first()
                            .map(|i| i.label().as_str())
                            .unwrap_or("未选择")
                    ))
                    .child(
                        YueCheckbox::new(
                            "steps-fail",
                            theme,
                            if self.fail_next {
                                CheckboxState::Checked
                            } else {
                                CheckboxState::Unchecked
                            },
                        )
                        .label("模拟下一次导入失败")
                        .disabled(self.busy)
                        .on_change({
                            let owner = cx.entity();
                            move |v, _, _, cx| {
                                owner.update(cx, |p, cx| {
                                    p.fail_next = v == CheckboxState::Checked;
                                    cx.notify();
                                })
                            }
                        }),
                    )
                    .child(
                        YueButton::new(
                            "steps-import",
                            if self.failed {
                                "重试导入"
                            } else {
                                "开始导入"
                            },
                        )
                        .variant(ButtonVariant::Primary)
                        .loading(self.busy)
                        .on_click(cx.listener(|p, _, w, cx| p.import(w, cx))),
                    )
                    .into_any_element()
            }),
        };
        let navigation = row()
            .child(
                YueButton::new("steps-back", "上一步")
                    .disabled(self.stage == 0 || self.busy || self.completed)
                    .on_click(cx.listener(|p, _, w, cx| p.go(p.stage.saturating_sub(1), w, cx))),
            )
            .child(
                YueButton::new(
                    "steps-next",
                    if self.stage == 0 {
                        "下一步"
                    } else {
                        "核对导入"
                    },
                )
                .variant(ButtonVariant::Primary)
                .disabled(self.stage >= 2 || self.busy)
                .on_click(cx.listener(|p, _, w, cx| p.go(p.stage + 1, w, cx))),
            )
            .child(
                YueButton::new("steps-reset", "重新开始")
                    .disabled(self.busy)
                    .on_click(cx.listener(|p, _, _, cx| {
                        p.stage = 0;
                        p.completed = false;
                        p.failed = false;
                        p.form.update(cx, |f, cx| f.reset(cx));
                        p.message = "已返回数据来源，原批次名称保留。".into();
                        cx.notify();
                    })),
            );
        let mut gallery = div().flex().flex_col().gap_6();
        for (id, title, variant) in [
            ("steps-dot", "点状步骤", StepsVariant::Dot),
            ("steps-navigation", "导航式步骤", StepsVariant::Navigation),
            ("steps-arrow", "箭头式步骤", StepsVariant::Arrow),
        ] {
            gallery = gallery.child(section(
                id,
                title,
                YueSteps::new(format!("{id}-example"), "configure")
                    .items(sample_items())
                    .variant(variant),
            ));
        }
        let extra = YueSteps::new("steps-extra", "review")
            .layout(StepsLayout::Vertical)
            .items([
                YueStepItem::new("create", "创建申请")
                    .icon(outline::File)
                    .description("已提交，11:20"),
                YueStepItem::new("review", "审核申请")
                    .icon(outline::Settings)
                    .description("等待负责人确认")
                    .extra(
                        row()
                            .child(YueButton::new("steps-approve", "确认审核").on_click(
                                cx.listener(|p, _, _, cx| {
                                    p.extra += 1;
                                    p.message =
                                        format!("已执行独立审核 {} 次，步骤值未改变", p.extra)
                                            .into();
                                    cx.notify();
                                }),
                            ))
                            .child(YueButton::new("steps-details", "查看申请").on_click(
                                cx.listener(|p, _, _, cx| {
                                    p.message = "申请 #2026-1109：工单批次导入。".into();
                                    cx.notify();
                                }),
                            )),
                    ),
                YueStepItem::new("archive", "归档")
                    .disabled(true)
                    .description("审核后解锁"),
            ])
            .on_change(cx.listener(|p, _: &yue_component::StepsChange, _, cx| {
                p.message = "审批流程由业务控制，不允许手工跳步。".into();
                cx.notify();
            }));
        let mut sizes = div().flex().flex_col().gap_5();
        for (ix, size) in [StepsSize::Small, StepsSize::Medium, StepsSize::Large]
            .into_iter()
            .enumerate()
        {
            sizes = sizes.child(
                YueSteps::new(format!("steps-size-{ix}"), "configure")
                    .items(sample_items())
                    .size(size),
            );
        }
        div()
            .id("steps-content")
            .debug_selector(|| "steps-content".into())
            .w_full()
            .h_full()
            .min_w_0()
            .min_h_0()
            .flex_1()
            .child(
                div()
                    .p_6()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(div().text_2xl().child("步骤条 Steps"))
                    .child(
                        div()
                            .text_color(gpui_kit::rgb(theme.text_secondary()))
                            .child("展示任务进度，允许返回修改；校验与业务结果由页面负责。"),
                    )
                    .child(section(
                        "steps-import-card",
                        "数据导入流程",
                        div()
                            .flex()
                            .flex_col()
                            .gap_5()
                            .child(workflow)
                            .child(content)
                            .child(navigation),
                    ))
                    .child(div().id("steps-message").child(self.message.clone()))
                    .child(section(
                        "steps-playground-card",
                        "布局与受控切换",
                        div()
                            .flex()
                            .flex_col()
                            .gap_5()
                            .child(controls)
                            .child(playground),
                    ))
                    .child(section("steps-sizes-card", "三档尺寸", sizes))
                    .child(gallery)
                    .child(section("steps-extra-card", "自定义图标与独立内容", extra))
                    .child(section(
                        "steps-narrow-card",
                        "窄容器与长文本",
                        div()
                            .w(rems(20.))
                            .max_w_full()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui_kit::rgb(theme.text_secondary()))
                                    .child("横向滚动可查看全部 3 个步骤，长标题在各自步骤内换行。"),
                            )
                            .child(
                                YueSteps::new("steps-narrow", "configure")
                                    .label_placement(StepsLabelPlacement::Below)
                                    .items([
                                        YueStepItem::new("source", "选择需要导入的数据来源与文件"),
                                        YueStepItem::new("configure", "配置字段映射和批次信息"),
                                        YueStepItem::new("result", "确认结果与错误明细")
                                            .status(StepsStatus::Error),
                                    ]),
                            ),
                    )),
            )
            .overflow_y_scrollbar()
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{
        Root,
        test_support::{ElementSnapshot, snapshots},
    };
    use gpui_kit::{ElementId, Modifiers, TestAppContext, VisualTestContext, point, px, size};
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn node(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
                .unwrap_or_else(|| panic!("missing {id}"))
        })
    }
    fn click(cx: &mut VisualTestContext, id: &str) {
        let p = node(cx, id).bounds().center();
        let height = cx.update(|w, _| w.viewport_size().height);
        if p.y < px(60.) || p.y > height - px(60.) {
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                position: point(p.x, height / 2.),
                delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), height / 2. - p.y)),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });
            draw(cx);
        }
        let p = node(cx, id).bounds().center();
        cx.simulate_click(p, Modifiers::default());
        draw(cx);
    }
    fn open(cx: &mut TestAppContext) -> (Entity<StepsPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            w.activate_window();
            let p = cx.new(|cx| StepsPage::new(w, cx));
            saved = Some(p.clone());
            Root::new(p, w, cx)
        });
        cx.simulate_resize(size(px(1280.), px(1000.)));
        draw(cx);
        (saved.unwrap(), cx)
    }
    #[gpui_kit::test]
    fn import_validates_preserves_editor_and_supports_failure_retry_completion(
        cx: &mut TestAppContext,
    ) {
        let (p, cx) = open(cx);
        click(cx, "steps-next");
        assert_eq!(p.read_with(cx, |p, _| p.stage), 1);
        click(cx, "steps-next");
        assert_eq!(p.read_with(cx, |p, _| p.stage), 1);
        let input = p.read_with(cx, |p, _| p.name.clone());
        assert!(p.read_with(cx, |p, cx| p.form.read(cx).error("batch", cx).is_some()));
        click(cx, "steps-batch");
        cx.simulate_input("November import");
        draw(cx);
        click(cx, "steps-back");
        click(cx, "steps-next");
        assert_eq!(
            input.read_with(cx, |s, _| s.value()).as_ref(),
            "November import"
        );
        click(cx, "steps-next");
        assert_eq!(p.read_with(cx, |p, _| p.stage), 2);
        click(cx, "steps-fail");
        assert!(p.read_with(cx, |p, _| p.fail_next));
        click(cx, "steps-import");
        assert!(p.read_with(cx, |p, _| p.busy));
        cx.executor().advance_clock(Duration::from_millis(1000));
        draw(cx);
        assert!(p.read_with(cx, |p, _| p.failed));
        assert!(!p.read_with(cx, |p, _| p.completed));
        click(cx, "steps-import");
        cx.executor().advance_clock(Duration::from_millis(1000));
        draw(cx);
        assert!(p.read_with(cx, |p, _| p.completed));
        assert!(!p.read_with(cx, |p, _| p.failed));
        click(cx, "steps-reset");
        assert_eq!(p.read_with(cx, |p, _| p.stage), 0);
        assert_eq!(
            input.read_with(cx, |s, _| s.value()).as_ref(),
            "November import"
        );
    }
    #[gpui_kit::test]
    fn playground_normalizes_layout_and_extra_command_does_not_change_position(
        cx: &mut TestAppContext,
    ) {
        let (p, cx) = open(cx);
        click(cx, "steps-layout");
        assert!(p.read_with(cx, |p, _| p.vertical));
        click(cx, "steps-variant");
        click(cx, "steps-variant");
        assert_eq!(p.read_with(cx, |p, _| p.variant), StepsVariant::Navigation);
        assert!(!p.read_with(cx, |p, _| p.vertical));
        click(cx, "steps-label");
        assert_eq!(p.read_with(cx, |p, _| p.variant), StepsVariant::Default);
        assert!(p.read_with(cx, |p, _| p.below));
        click(cx, "steps-readonly");
        click(cx, "steps-complete");
        assert!(p.read_with(cx, |p, _| p.readonly));
        assert_eq!(
            p.read_with(cx, |p, _| p.current.clone()),
            StepsValue::Complete
        );
        click(cx, "steps-approve");
        assert_eq!(p.read_with(cx, |p, _| p.extra), 1);
        assert_eq!(
            p.read_with(cx, |p, _| p.current.clone()),
            StepsValue::Complete
        );
    }
}
