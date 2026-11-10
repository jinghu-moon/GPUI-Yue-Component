use super::super::*;
use gpui_kit::{Entity, Subscription, Task, rems};
use std::time::Duration;
use yue_component::{
    ListGrid, PaginationChange, PaginationMode, PaginationSize, PaginationSizeChangePolicy,
    PaginationValue, PaginationVariant, YueCard, YueCardFooter, YueCardHeader, YueEmpty, YueList,
    YueListItem, YueListItemMeta, YuePagination, YuePaginationState,
};

pub(crate) struct PaginationPage {
    states: Vec<Entity<YuePaginationState>>,
    size: PaginationSize,
    outline: bool,
    narrow: bool,
    simple: bool,
    labels: bool,
    hide_single: bool,
    grid: bool,
    reject: bool,
    fail_next: bool,
    failed: Option<PaginationValue>,
    task: Option<Task<()>>,
    generation: u64,
    requests: usize,
    message: SharedString,
    _subscriptions: Vec<Subscription>,
}
impl PaginationPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let values = [
            (257, 1, 10),
            (200, 1, 10),
            (445, 12, 10),
            (445, 12, 10),
            (100, 1, 10),
            (50, 1, 10),
            (0, 1, 10),
            (100, 1, 10),
            (200, 1, 10),
            (usize::MAX, usize::MAX / 10, 10),
            (0, 1, 10),
        ];
        let states = values
            .into_iter()
            .map(|(t, p, s)| {
                cx.new(|cx| YuePaginationState::new(PaginationValue::new(t, p, s).unwrap(), w, cx))
            })
            .collect::<Vec<_>>();
        states[3].update(cx, |s, _| {
            s.set_size_change_policy(PaginationSizeChangePolicy::KeepFirstItem)
        });
        states[5].update(cx, |s, cx| s.set_disabled(true, w, cx));
        let subscriptions = states
            .iter()
            .enumerate()
            .map(|(ix, state)| {
                cx.subscribe_in(state, w, move |p, state, e: &PaginationChange, w, cx| {
                    p.requests += 1;
                    if ix == 7 && p.reject {
                        p.message = "宿主拒绝了请求，当前页与每页条数保持不变".into();
                    } else if ix == 8 {
                        p.start_request(e.value(), w, cx);
                    } else {
                        state.update(cx, |s, cx| s.set_value(e.value(), w, cx));
                        p.message = format!(
                            "已显示第 {} 页，每页 {} 条",
                            e.value().page(),
                            e.value().page_size()
                        )
                        .into();
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            states,
            size: PaginationSize::Medium,
            outline: false,
            narrow: false,
            simple: false,
            labels: false,
            hide_single: false,
            grid: false,
            reject: true,
            fail_next: false,
            failed: None,
            task: None,
            generation: 0,
            requests: 0,
            message: "选择页码后，下方工单列表会显示对应数据。".into(),
            _subscriptions: subscriptions,
        }
    }
    fn control(&self, ix: usize) -> YuePagination {
        YuePagination::new(format!("pagination-example-{ix}"), &self.states[ix])
            .size(self.size)
            .variant(if self.outline {
                PaginationVariant::Outline
            } else {
                PaginationVariant::Ghost
            })
            .accessibility_label(format!("示例 {} 分页", ix + 1))
    }
    fn start_request(&mut self, value: PaginationValue, w: &mut Window, cx: &mut Context<Self>) {
        self.task = None;
        self.generation += 1;
        let generation = self.generation;
        let fail = self.fail_next;
        self.fail_next = false;
        self.failed = None;

        self.states[8].update(cx, |s, cx| s.set_loading(true, w, cx));

        self.message = "正在请求工单，原有结果继续保留。".into();
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(900))
                .await;
            let _ = weak.update_in(cx, |p, w, cx| {
                if p.generation != generation {
                    return;
                }
                p.states[8].update(cx, |s, cx| {
                    s.set_loading(false, w, cx);
                    if !fail {
                        s.set_value(value, w, cx);
                    }
                });
                if fail {
                    p.failed = Some(value);
                    p.message = "工单加载失败，保留原页，可重试。".into();
                } else {
                    p.message = format!("已加载第 {} 页", value.page()).into();
                }
                p.task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}
fn row() -> gpui_kit::Div {
    div().min_w_0().flex().flex_wrap().items_center().gap_3()
}
fn section(
    title: &str,
    description: &str,
    content: impl IntoElement,
    theme: YueTheme,
) -> gpui_kit::Div {
    div()
        .min_w_0()
        .w_full()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child(title.to_owned()))
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(description.to_owned()),
        )
        .child(content)
}
impl Render for PaginationPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let controls = row()
            .child(
                YueButton::new(
                    "pagination-size",
                    match self.size {
                        PaginationSize::Small => "尺寸：小",
                        PaginationSize::Medium => "尺寸：中",
                        PaginationSize::Large => "尺寸：大",
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.size = match p.size {
                        PaginationSize::Small => PaginationSize::Medium,
                        PaginationSize::Medium => PaginationSize::Large,
                        PaginationSize::Large => PaginationSize::Small,
                    };
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "pagination-variant",
                    if self.outline {
                        "切换无框"
                    } else {
                        "切换描边"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.outline = !p.outline;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "pagination-width",
                    if self.narrow {
                        "展开容器"
                    } else {
                        "收窄容器"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.narrow = !p.narrow;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "pagination-mode",
                    if self.simple {
                        "显示页码"
                    } else {
                        "简洁模式"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.simple = !p.simple;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "pagination-labels",
                    if self.labels {
                        "箭头导航"
                    } else {
                        "文字导航"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.labels = !p.labels;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "pagination-data-layout",
                    if self.grid {
                        "列表展示"
                    } else {
                        "卡片展示"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.grid = !p.grid;
                    cx.notify();
                })),
            );
        let value = self.states[0].read(cx).value();
        let range = value.item_range();
        let data = if self.grid {
            YueList::new("pagination-orders")
                .grid(ListGrid::min_width(rems(13.), rems(1.)).unwrap())
                .children(range.clone().map(|ix| {
                    YueListItem::new(format!("order-{ix}"))
                        .inset(false)
                        .content(
                            YueCard::new(format!("order-card-{ix}"))
                                .header(YueCardHeader::new(format!("工单 WO-{:05}", ix + 1)))
                                .child(format!("计划数量 {} 套", (ix + 1) * 12))
                                .footer(YueCardFooter::new("待排产")),
                        )
                }))
                .into_any_element()
        } else {
            YueList::new("pagination-orders")
                .bordered(true)
                .children(range.clone().map(|ix| {
                    YueListItem::new(format!("order-{ix}")).content(
                        YueListItemMeta::new(format!("工单 WO-{:05}", ix + 1))
                            .description(format!("计划数量 {} 套 · 待排产", (ix + 1) * 12)),
                    )
                }))
                .empty(YueEmpty::new("pagination-orders-empty").title("没有匹配的工单"))
                .into_any_element()
        };
        let mut main = self
            .control(0)
            .show_total(true)
            .page_size_selector(true)
            .show_jumper(true)
            .first_last(true)
            .mode(if self.simple {
                PaginationMode::Simple
            } else {
                PaginationMode::Pages
            })
            .total_format(|v| {
                let r = v.item_range();
                format!(
                    "第 {}–{} 条，共 {} 条",
                    if r.is_empty() { 0 } else { r.start + 1 },
                    r.end,
                    v.total()
                )
                .into()
            });
        if self.labels {
            main = main.navigation_labels("上一页", "下一页");
        }
        let collection = div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_4()
            .child(main)
            .child(data);
        let policies = div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_4()
            .child("改变每页数量后回到第一页")
            .child(
                self.control(2)
                    .show_total(true)
                    .page_size_selector(true)
                    .show_jumper(true),
            )
            .child("改变每页数量后保留原首条所在位置")
            .child(
                self.control(3)
                    .show_total(true)
                    .page_size_selector(true)
                    .show_jumper(true),
            );
        let simple = row()
            .child(
                self.control(4)
                    .mode(PaginationMode::Simple)
                    .page_size_selector(true)
                    .show_jumper(true),
            )
            .child(
                self.control(6)
                    .mode(PaginationMode::Simple)
                    .show_total(true),
            );
        let disabled = row().child(
            self.control(5)
                .show_total(true)
                .page_size_selector(true)
                .show_jumper(true)
                .first_last(true),
        );
        let rejected = div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                YueButton::new(
                    "pagination-reject",
                    if self.reject {
                        "允许请求"
                    } else {
                        "拒绝请求"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.reject = !p.reject;
                    cx.notify();
                })),
            )
            .child(
                self.control(7)
                    .show_total(true)
                    .page_size_selector(true)
                    .show_jumper(true),
            );
        let async_controls = row()
            .child(
                YueButton::new(
                    "pagination-fail",
                    if self.fail_next {
                        "已设置下一次失败"
                    } else {
                        "下一次请求失败"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.fail_next = !p.fail_next;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("pagination-cancel", "重置查询并取消等待").on_click(cx.listener(
                    |p, _, w, cx| {
                        p.task = None;
                        p.generation += 1;
                        p.failed = None;
                        p.states[8].update(cx, |s, cx| {
                            s.set_loading(false, w, cx);
                            s.set_value(PaginationValue::new(20, 1, 10).unwrap(), w, cx);
                        });
                        p.message = "已取消旧请求，查询更新为 20 条。".into();
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new("pagination-retry", "重试失败请求")
                    .disabled(self.failed.is_none() || self.states[8].read(cx).is_loading())
                    .on_click(cx.listener(|p, _, w, cx| {
                        if let Some(value) = p.failed {
                            p.start_request(value, w, cx);
                        }
                    })),
            );
        let async_example = div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(async_controls)
            .child(
                self.control(8)
                    .show_total(true)
                    .page_size_selector(true)
                    .show_jumper(true),
            );
        let hidden = row()
            .child(
                YueButton::new(
                    "pagination-hide-single",
                    if self.hide_single {
                        "显示单页分页"
                    } else {
                        "隐藏单页分页"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.hide_single = !p.hide_single;
                    cx.notify();
                })),
            )
            .child(self.control(10).hide_on_single_page(self.hide_single));
        let basic = div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.control(1).first_last(true))
            .child(
                self.control(9)
                    .show_total(true)
                    .show_jumper(true)
                    .ellipsis_jump(false),
            );
        div()
            .id("pagination-content")
            .debug_selector(|| "pagination-content".into())
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
                    .child(div().text_2xl().child("Pagination 分页"))
                    .child("分页只请求查询变化，宿主提交页码、保留数据并决定滚动位置。")
                    .child(controls)
                    .child(div().id("pagination-message").text_sm().child(format!(
                        "{} · 已收到 {} 次请求",
                        self.message, self.requests
                    )))
                    .child(
                        div()
                            .w(if self.narrow { rems(24.) } else { rems(68.) })
                            .max_w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_6()
                            .child(section(
                                "工单集合",
                                "改变页码或每页条数，会显示对应的真实工单切片。",
                                collection,
                                theme,
                            ))
                            .child(section(
                                "页码、省略与首末页",
                                "省略按钮可跳过一组；超大总数也只构建固定数量的页码。",
                                basic,
                                theme,
                            ))
                            .child(section(
                                "每页数量策略",
                                "对照第一页重置与首条位置保留；两个示例各自持有状态。",
                                policies,
                                theme,
                            ))
                            .child(section(
                                "简洁与空数据",
                                "输入后 Enter 确认，Escape 或失焦取消；零条数据不显示第零页。",
                                simple,
                                theme,
                            ))
                            .child(section(
                                "禁用",
                                "按钮、输入与每页选择器同时禁用。",
                                disabled,
                                theme,
                            ))
                            .child(section(
                                "宿主拒绝",
                                "请求不会乐观改写当前页，选择器也会回到已提交值。",
                                rejected,
                                theme,
                            ))
                            .child(section(
                                "异步、失败重试与取消",
                                "900ms 请求期间保留原页；失败后重试，重置查询取消旧任务。",
                                async_example,
                                theme,
                            ))
                            .child(section(
                                "单页隐藏",
                                "只有一页时可省略整个导航；应用仍拥有总数和空态。",
                                hidden,
                                theme,
                            )),
                    ),
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
    use gpui_kit::{ElementId, Modifiers, TestAppContext, VisualTestContext, point, px};
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
    }
    fn node(cx: &mut VisualTestContext, id: impl Into<ElementId>) -> ElementSnapshot {
        let id = id.into();
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&id))
                .unwrap_or_else(|| panic!("missing {id:?}"))
        })
    }
    fn click(cx: &mut VisualTestContext, id: impl Into<ElementId>) {
        let id = id.into();

        let p = node(cx, id.clone()).bounds().center();
        let h = cx.update(|w, _| w.viewport_size().height);

        if p.y < px(60.) || p.y > h - px(60.) {
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                position: point(p.x, h / 2.),
                delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), h / 2. - p.y)),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });

            draw(cx);
        }
        let position = node(cx, id).bounds().center();
        cx.simulate_click(position, Modifiers::default());

        draw(cx);
    }
    fn page(cx: &mut VisualTestContext, ix: usize, part: &str) {
        click(
            cx,
            (
                ElementId::from(format!("pagination-example-{ix}")),
                part.to_owned(),
            ),
        );
    }
    fn open(cx: &mut TestAppContext) -> (Entity<PaginationPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            w.activate_window();
            let view = cx.new(|cx| PaginationPage::new(w, cx));
            saved = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1280.), px(1000.)));
        draw(cx);
        (saved.unwrap(), cx)
    }
    #[gpui_kit::test]
    fn real_collection_slices_and_layout_controls_work(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        assert!(node(cx, "order-0").visible());
        page(cx, 0, "next");
        assert_eq!(
            view.read_with(cx, |p, cx| p.states[0].read(cx).value().item_range()),
            10..20
        );
        assert!(node(cx, "order-10").visible());
        assert!(
            !cx.update(|w, _| snapshots(w))
                .iter()
                .any(|n| n.path().last() == Some(&ElementId::from("order-0")))
        );
        click(cx, "pagination-data-layout");
        assert!(view.read_with(cx, |p, _| p.grid));
        assert!(node(cx, "order-card-10").visible());
        click(cx, "pagination-width");
        assert!(view.read_with(cx, |p, _| p.narrow));
        click(cx, "pagination-size");
        assert_eq!(view.read_with(cx, |p, _| p.size), PaginationSize::Large);
        click(cx, "pagination-variant");
        assert!(view.read_with(cx, |p, _| p.outline));
        click(cx, "pagination-mode");
        assert!(view.read_with(cx, |p, _| p.simple));
    }
    #[gpui_kit::test]
    fn asynchronous_failure_retry_cancel_and_host_rejection_work(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);

        page(cx, 7, "next");
        assert_eq!(
            view.read_with(cx, |p, cx| p.states[7].read(cx).value().page()),
            1
        );
        click(cx, "pagination-reject");

        page(cx, 7, "next");
        assert_eq!(
            view.read_with(cx, |p, cx| p.states[7].read(cx).value().page()),
            2
        );
        click(cx, "pagination-fail");

        page(cx, 8, "next");

        assert!(view.read_with(cx, |p, cx| p.states[8].read(cx).is_loading()));
        assert_eq!(
            view.read_with(cx, |p, cx| p.states[8].read(cx).value().page()),
            1
        );
        cx.executor().advance_clock(Duration::from_millis(1100));
        draw(cx);

        assert!(view.read_with(cx, |p, _| p.failed.is_some()));
        click(cx, "pagination-retry");

        cx.executor().advance_clock(Duration::from_millis(1100));
        draw(cx);

        assert_eq!(
            view.read_with(cx, |p, cx| p.states[8].read(cx).value().page()),
            2
        );
        page(cx, 8, "next");
        click(cx, "pagination-cancel");
        cx.executor().advance_clock(Duration::from_millis(1100));
        draw(cx);
        assert_eq!(
            view.read_with(cx, |p, cx| p.states[8].read(cx).value()),
            PaginationValue::new(20, 1, 10).unwrap()
        );
        assert!(!view.read_with(cx, |p, cx| p.states[8].read(cx).is_loading()));
    }
}
