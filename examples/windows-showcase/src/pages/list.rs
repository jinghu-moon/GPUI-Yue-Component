use super::super::*;
use gpui_kit::{Axis, ElementId, Entity, FocusHandle, px, rems};
use std::rc::Rc;
use yue_component::{
    ListGrid, ListGridSizing, ListGridState, ListItemLayout, ListLayout, ListSize,
    VirtualizerAlign, VirtualizerSizing, VirtualizerSource, VirtualizerState, YueList,
    YueListGroup, YueListItem, YueListItemMeta, YueSegmented, YueSegmentedOption, YueSkeleton,
    YueTag, YueVirtualList, YueVirtualListGrid,
};
mod settings;

pub(crate) struct ListPage {
    settings: Entity<settings::SettingsExamples>,
    size: ListSize,
    split: bool,
    striped: bool,
    bordered: bool,
    vertical: bool,
    disabled: bool,
    checked: bool,
    status: SharedString,
    count: usize,
    last: String,
    virtual_state: Entity<VirtualizerState>,
    grid_state: Entity<ListGridState>,
    grid_focus: FocusHandle,
    grid_narrow: bool,
    grid_fixed: bool,
    grid_tall: bool,
}
struct Records;
impl VirtualizerSource for Records {
    fn len(&self) -> usize {
        10_000
    }
    fn key_at(&self, index: usize) -> ElementId {
        format!("record-{index}").into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if let ElementId::Name(name) = key {
            name.strip_prefix("record-")
                .and_then(|value| value.parse().ok())
                .filter(|ix| *ix < self.len())
        } else {
            None
        }
    }
}
impl ListPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let grid_focus = cx.focus_handle().tab_stop(true);
        let grid_state = cx.new(|cx| {
            ListGridState::new(
                Rc::new(Records),
                card_grid(false),
                ListGridSizing::measured(px(180.)),
                cx,
            )
            .expect("valid card grid")
        });
        grid_state.update(cx, |state, _| {
            state
                .set_focus_handle("record-7".into(), Some(grid_focus.clone()))
                .unwrap()
        });
        Self {
            settings: cx.new(|cx| settings::SettingsExamples::new(window, cx)),
            size: ListSize::Medium,
            split: true,
            striped: false,
            bordered: true,
            vertical: false,
            disabled: false,
            checked: false,
            status: "ready".into(),
            count: 3,
            last: "尚未操作".into(),
            grid_state,
            grid_focus,
            grid_narrow: false,
            grid_fixed: false,
            grid_tall: false,
            virtual_state: cx.new(|_| {
                VirtualizerState::new(
                    Axis::Vertical,
                    Rc::new(Records),
                    VirtualizerSizing::measured(px(64.)),
                )
                .expect("valid measured list")
            }),
        }
    }
    fn toggle(
        &self,
        id: &'static str,
        label: &'static str,
        value: bool,
        update: fn(&mut Self, bool),
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let owner = cx.entity();
        div().flex().items_center().gap_2().child(label).child(
            YueSwitch::new(id, yue_component::current_theme(cx), value, label).on_change(
                move |next, _, _, cx| {
                    owner.update(cx, |page, cx| {
                        update(page, next);
                        cx.notify();
                    })
                },
            ),
        )
    }
}
fn card_grid(fixed: bool) -> ListGrid {
    if fixed {
        ListGrid::columns(3, rems(1.))
    } else {
        ListGrid::min_width(rems(14.), rems(1.))
    }
    .expect("valid showcase grid")
}
fn card(theme: YueTheme, index: usize, key: ElementId, tall: bool) -> YueListItem {
    YueListItem::new(key).theme(theme).layout(ListItemLayout::Vertical).hoverable(true)
        .content(meta(theme, format!("项目 {}", index + 1), "同一套条目内容，在普通与虚拟路径中复用"))
        .body(if tall || index % 3 == 1 { "这张卡片包含更多内容。调整容器宽度后，文字会自然换行；同行卡片保持相同高度，下一行统一对齐。" } else { "规则卡片，按数据顺序排列。" })
}
fn column() -> gpui_kit::Div {
    div().w_full().min_w_0().flex().flex_col().gap_4()
}
fn meta(
    theme: YueTheme,
    title: impl Into<SharedString>,
    description: &'static str,
) -> YueListItemMeta {
    YueListItemMeta::new(title.into())
        .description(description)
        .theme(theme)
}
// These examples fill the available width. Normal flow avoids another pair of
// intrinsic flex measurements around every nested list; spacing stays explicit.
fn list_section(
    theme: YueTheme,
    title: &'static str,
    description: &'static str,
    example: impl IntoElement,
) -> impl IntoElement {
    div()
        .min_w_0()
        .flex_shrink_0()
        .child(div().text_lg().mb_3().child(title))
        .child(
            div()
                .text_sm()
                .mb_3()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(description),
        )
        .child(
            div()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .bg(gpui_kit::rgb(theme.surface()))
                .child(example),
        )
}
impl Render for ListPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let controls = div()
            .flex()
            .items_center()
            .flex_wrap()
            .gap_4()
            .child(
                YueSegmented::new(
                    "list-size",
                    theme,
                    match self.size {
                        ListSize::Small => "small",
                        ListSize::Medium => "medium",
                        ListSize::Large => "large",
                    },
                )
                .label("列表尺寸")
                .options([
                    YueSegmentedOption::new("small", "小"),
                    YueSegmentedOption::new("medium", "中"),
                    YueSegmentedOption::new("large", "大"),
                ])
                .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                    page.size = match value.as_str() {
                        "small" => ListSize::Small,
                        "large" => ListSize::Large,
                        _ => ListSize::Medium,
                    };
                    cx.notify();
                })),
            )
            .child(self.toggle("list-split", "分隔线", self.split, |p, v| p.split = v, cx))
            .child(self.toggle(
                "list-striped",
                "斑马纹",
                self.striped,
                |p, v| p.striped = v,
                cx,
            ))
            .child(self.toggle(
                "list-bordered",
                "外边框",
                self.bordered,
                |p, v| p.bordered = v,
                cx,
            ))
            .child(self.toggle(
                "list-vertical",
                "条目竖排",
                self.vertical,
                |p, v| p.vertical = v,
                cx,
            ))
            .child(self.toggle(
                "list-disabled",
                "禁用操作",
                self.disabled,
                |p, v| p.disabled = v,
                cx,
            ));
        let owner = cx.entity();
        let preview = YueList::new("list-preview")
            .accessibility_label("成员预览")
            .size(self.size)
            .split(self.split)
            .striped(self.striped)
            .bordered(self.bordered)
            .item_layout(if self.vertical {
                ListItemLayout::Vertical
            } else {
                ListItemLayout::Horizontal
            })
            .header("项目成员")
            .footer(readout(theme, self.last.clone()))
            .child(
                YueListItem::new("member-lin")
                    .selected(self.checked)
                    .disabled(self.disabled)
                    .leading(
                        YueCheckbox::new(
                            "list-choose-lin",
                            theme,
                            if self.checked {
                                CheckboxState::Checked
                            } else {
                                CheckboxState::Unchecked
                            },
                        )
                        .accessibility_label("选择林悦")
                        .disabled(self.disabled)
                        .on_change(move |state, _, _, cx| {
                            owner.update(cx, |page, cx| {
                                page.checked = state == CheckboxState::Checked;
                                page.last = if page.checked {
                                    "已选择林悦"
                                } else {
                                    "已取消选择林悦"
                                }
                                .into();
                                cx.notify();
                            })
                        }),
                    )
                    .content(meta(theme, "林悦", "负责组件设计与文档"))
                    .body("将相关内容组织成连续条目；选择值和业务操作始终由页面持有。")
                    .extra(YueTag::new("lin-role", "维护者"))
                    .actions(
                        YueButton::new("list-view-lin", "查看")
                            .disabled(self.disabled)
                            .on_click(cx.listener(|page, _, _, cx| {
                                page.last = "正在查看林悦".into();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                YueListItem::new("member-chen")
                    .content(meta(theme, "陈星", "负责测试与示例"))
                    .extra("昨天加入"),
            )
            .child(YueListItem::new("member-zhou").content("周宁 · 只有一行内容也可独立使用"));
        let mut density = column();
        for (name, size) in [
            ("Small", ListSize::Small),
            ("Medium", ListSize::Medium),
            ("Large", ListSize::Large),
        ] {
            density = density.child(
                YueList::new(format!("list-density-{name}"))
                    .size(size)
                    .header(name)
                    .children(["设计文档", "测试记录"].into_iter().map(|title| {
                        YueListItem::new(title).content(meta(
                            theme,
                            title,
                            "内容自然撑高，不指定固定行高",
                        ))
                    })),
            );
        }
        let groups =
            YueList::new("list-groups")
                .striped(true)
                .bordered(true)
                .group(YueListGroup::new("today", "今天").children([
                    YueListItem::new("review").content(meta(theme, "评审组件", "上午 10:00")),
                    YueListItem::new("test").content(meta(theme, "复测布局", "下午 14:00")),
                ]))
                .group(YueListGroup::new("tomorrow", "明天").child(
                    YueListItem::new("publish").content(meta(theme, "整理发布记录", "待安排")),
                ));
        let article = YueList::new("list-articles").item_layout(ListItemLayout::Vertical)
            .child(YueListItem::new("layout-article")
                .content(meta(theme, "为桌面列表建立稳定的阅读顺序", "组件指南 · 6 分钟"))
                .body("主信息、正文和操作纵向排布，封面仍在右侧。缩窄窗口时正文自然换行；需要移动封面时由宿主明确改变组合。")
                .extra(div().w_24().h_20().rounded_md()
                    .bg(theme.token("--surface-subtle").unwrap().rgba().unwrap()).flex().items_center().justify_center().child("封面"))
                .actions(YueButton::new("list-read-article", "阅读").on_click(cx.listener(|page, _, _, cx| {
                    page.last = "已打开阅读示例".into(); cx.notify();
                }))));
        let status_controls = YueSegmented::new("list-status", theme, self.status.clone())
            .label("列表内容状态")
            .options([
                YueSegmentedOption::new("ready", "内容"),
                YueSegmentedOption::new("empty", "空"),
                YueSegmentedOption::new("loading", "加载"),
                YueSegmentedOption::new("error", "错误"),
            ])
            .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                page.status = value.clone();
                cx.notify();
            }));
        let mut status = YueList::new("list-status-content")
            .header("文档")
            .bordered(true);
        if self.status == "ready" {
            status = status
                .children((0..self.count).map(|ix| {
                    YueListItem::new(format!("doc-{ix}")).content(format!("文档 {}", ix + 1))
                }))
                .footer(
                    YueButton::new("list-more", "加载更多").on_click(cx.listener(
                        |page, _, _, cx| {
                            page.count += 2;
                            cx.notify();
                        },
                    )),
                );
        } else if self.status == "loading" {
            status = status.empty(
                column()
                    .child(YueSkeleton::new("list-loading-title"))
                    .child(YueSkeleton::new("list-loading-description")),
            );
        } else if self.status == "error" {
            status = status.empty(column().child("暂时无法加载文档").child(
                YueButton::new("list-retry", "重试").on_click(cx.listener(|page, _, _, cx| {
                    page.status = "ready".into();
                    cx.notify();
                })),
            ));
        } else {
            status = status.empty("暂无文档");
        }
        let horizontal = YueList::new("list-horizontal")
            .layout(ListLayout::Horizontal)
            .children(
                ["全部", "最近", "归档"]
                    .into_iter()
                    .map(|name| YueListItem::new(name).content(name)),
            );
        let grid_controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_4()
            .child(self.toggle(
                "list-grid-narrow",
                "窄容器",
                self.grid_narrow,
                |p, v| p.grid_narrow = v,
                cx,
            ))
            .child({
                let owner = cx.entity();
                div().flex().items_center().gap_2().child("固定三列").child(
                    YueSwitch::new("list-grid-fixed", theme, self.grid_fixed, "固定三列")
                        .on_change(move |next, _, _, cx| {
                            owner.update(cx, |page, cx| {
                                page.grid_fixed = next;
                                page.grid_state
                                    .update(cx, |s, cx| s.set_grid(card_grid(next), cx).unwrap());
                                cx.notify();
                            })
                        }),
                )
            })
            .child({
                let owner = cx.entity();
                div().flex().items_center().gap_2().child("增加正文").child(
                    YueSwitch::new("list-grid-tall", theme, self.grid_tall, "增加正文").on_change(
                        move |next, _, _, cx| {
                            owner.update(cx, |page, cx| {
                                page.grid_tall = next;
                                page.grid_state
                                    .update(cx, |s, cx| s.invalidate(cx).unwrap());
                                cx.notify();
                            })
                        },
                    ),
                )
            })
            .child(
                YueButton::new("list-grid-jump", "跳至项目 9901").on_click(cx.listener(
                    |page, _, _, cx| {
                        page.grid_state.update(cx, |s, cx| {
                            s.scroll_to_key(
                                &ElementId::from("record-9900"),
                                VirtualizerAlign::Start.into(),
                                cx,
                            )
                            .unwrap()
                        });
                    },
                )),
            );
        let normal_cards = YueList::new("list-cards")
            .grid(card_grid(self.grid_fixed))
            .bordered(true)
            .children(
                (0..7).map(|index| card(theme, index, Records.key_at(index), self.grid_tall)),
            );
        let owner = cx.entity();
        let focus = self.grid_focus.clone();
        let tall = self.grid_tall;
        let virtual_cards = YueVirtualListGrid::new(
            "list-virtual-cards",
            &self.grid_state,
            move |index, key, _, cx| {
                let owner = owner.clone();
                let mut action =
                    YueButton::new("open-card", "打开项目").on_click(move |_, _, cx| {
                        owner.update(cx, |page, cx| {
                            page.last = format!("打开项目 {}", index + 1);
                            cx.notify();
                        })
                    });
                if index == 7 {
                    action = action.track_focus(&focus);
                }
                card(yue_component::current_theme(cx), index, key.clone(), tall).actions(action)
            },
        )
        .accessibility_label("一万张项目卡片");
        let cards = column().child(grid_controls).child(
            column()
                .when(self.grid_narrow, |el| el.max_w(rems(28.)))
                .child(normal_cards)
                .child(
                    div()
                        .w_full()
                        .h(rems(22.))
                        // The child handles its wheel first. Keep the same
                        // gesture from also moving the surrounding demo page.
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .child(virtual_cards),
                ),
        );
        column()
            .debug_selector(|| "list-content".into())
            .p_8()
            .gap_6()
            .child(div().text_2xl().child("List 列表"))
            .child(readout(
                theme,
                "组织文本、元信息和独立操作；支持连续列表、分组及文章条目。".into(),
            ))
            .child(list_section(
                theme,
                "配置预览",
                "调整局部密度和表面。复选框与查看按钮分别获得焦点。",
                column().child(controls).child(preview),
            ))
            .child(list_section(
                theme,
                "尺寸",
                "三种尺寸共同调整间距和排版，内容高度自然变化。",
                density,
            ))
            .child(list_section(
                theme,
                "设置列表",
                "常规偏好、通知联动、只读信息与维护操作。修改仅保留在本示例中，可切换行尾和下方控件布局。",
                self.settings.clone(),
            ))
            .child(list_section(
                theme,
                "分组",
                "普通分组标题随内容滚动，斑马纹按完整数据顺序连续计算。",
                groups,
            ))
            .child(list_section(
                theme,
                "文章条目",
                "条目方向与集合方向分别配置；右侧附加内容不强制竖排。",
                article,
            ))
            .child(list_section(
                theme,
                "内容状态与头尾",
                "切换示例状态，或追加文档；加载、重试和页码由宿主组合。",
                column().child(status_controls).child(status),
            ))
            .child(list_section(
                theme,
                "横向短集合",
                "单行排列，不自动换行或创建滚动容器。",
                horizontal,
            ))
            .child(list_section(
                theme,
                "规则卡片网格",
                "按容器宽度自动分列，或固定三列。上方为普通集合，下方为一万张按行虚拟化卡片；可测试窄容器、变高正文和远端跳转。",
                cards,
            ))
            .child(list_section(
                theme,
                "一万条虚拟内容",
                "复用现有虚拟列表与动态测量，只挂载视口内的条目。",
                div().w_full().h(rems(18.))
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(
                    YueVirtualList::new("list-virtual", &self.virtual_state, |item, _, cx| {
                        let theme = yue_component::current_theme(cx);
                        YueListItem::new(item.key().clone())
                            .semantics(false)
                            .striped(true)
                            .logical_index(item.index())
                            .content(meta(
                                theme,
                                format!("记录 {}", item.index() + 1),
                                "复用同一套 ListItem 与 Meta，测量与滚动由基础层持有",
                            ))
                    })
                    .accessibility_label("一万条记录"),
                ),
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, size};
    #[gpui_kit::test]
    #[ignore = "manual production-page scroll CPU probe; run --release --ignored --nocapture"]
    fn list_page_scroll_cpu_evidence(cx: &mut TestAppContext) {
        use std::time::Instant;
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| ListPage::new(w, cx));
            Root::new(cx.new(|_| ScrollProbe { page: view }), w, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        draw(cx);
        draw(cx);
        for step in 0..24 {
            let (elapsed, nodes, bounds) = cx.update(|w, cx| {
                w.refresh();
                let start = Instant::now();
                w.draw(cx).clear(cx);
                let elapsed = start.elapsed().as_micros();
                let nodes = snapshots(w);
                let bounds: Vec<_> = nodes
                    .iter()
                    .filter(|n| {
                        n.label() == Some("一万张项目卡片") || n.label() == Some("一万条记录")
                    })
                    .map(|n| (n.label().unwrap().to_string(), n.bounds()))
                    .collect();
                (elapsed, nodes.len(), bounds)
            });
            println!("LIST_PAGE_SCROLL,{step},{elapsed},{nodes},{bounds:?}");
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                // Page padding routes the wheel to the outer viewport even
                // when a virtual example crosses the middle of the window.
                position: gpui_kit::point(px(1192.), px(400.)),
                delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-120.))),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });
            cx.run_until_parked();
        }
    }
    struct ScrollProbe {
        page: Entity<ListPage>,
    }
    impl Render for ScrollProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(self.page.clone())
                .overflow_y_scrollbar()
        }
    }
    #[gpui_kit::test]
    fn page_scroll_preserves_natural_layout_and_inner_scroll_ownership(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| ListPage::new(w, cx));
            page = Some(view.clone());
            Root::new(cx.new(|_| ScrollProbe { page: view }), w, cx)
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        draw(cx);
        let page = page.unwrap();
        let grid = page.read_with(cx, |p, _| p.grid_state.clone());
        let initial = cx.debug_bounds("list-content").unwrap();
        let distance = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some("一万张项目卡片"))
                .unwrap()
                .bounds()
                .top()
                - px(300.)
        });
        cx.simulate_event(gpui_kit::ScrollWheelEvent {
            position: gpui_kit::point(px(1192.), px(400.)),
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), -distance)),
            touch_phase: gpui_kit::TouchPhase::Moved,
            ..Default::default()
        });
        draw(cx);
        let scrolled = cx.debug_bounds("list-content").unwrap();
        assert_eq!(
            initial.size, scrolled.size,
            "scrolling must not resize the content"
        );
        assert_eq!(scrolled.top(), initial.top() - distance);
        let viewport = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some("一万张项目卡片"))
                .unwrap()
                .bounds()
        });
        assert_eq!(viewport.size.height, px(352.));
        assert!(viewport.center().y < px(800.));
        let before = grid.read_with(cx, |s, cx| {
            s.virtualizer().read(cx).viewport().unwrap().offset()
        });
        cx.simulate_event(gpui_kit::ScrollWheelEvent {
            position: viewport.center(),
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-120.))),
            touch_phase: gpui_kit::TouchPhase::Moved,
            ..Default::default()
        });
        draw(cx);
        assert_eq!(cx.debug_bounds("list-content").unwrap(), scrolled);
        let after = grid.read_with(cx, |s, cx| {
            s.virtualizer().read(cx).viewport().unwrap().offset()
        });
        assert!(
            after > before,
            "inner wheel must reach the existing virtual scroll owner"
        );
    }
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn click(cx: &mut VisualTestContext, label: &str) {
        let center = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label) && n.visible())
                .unwrap_or_else(|| panic!("missing List example control {label}"))
                .bounds()
                .center()
        });
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn example_configuration_selection_and_action_write_back_to_the_page(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| ListPage::new(w, cx));
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(size(px(1200.), px(1600.)));
        draw(cx);
        let page = page.unwrap();
        click(cx, "大");
        assert_eq!(page.read_with(cx, |page, _| page.size), ListSize::Large);
        click(cx, "条目竖排");
        assert!(page.read_with(cx, |page, _| page.vertical));
        click(cx, "选择林悦");
        assert!(page.read_with(cx, |page, _| page.checked));
        click(cx, "查看");
        assert_eq!(
            page.read_with(cx, |page, _| page.last.clone()),
            "正在查看林悦"
        );
        click(cx, "禁用操作");
        click(cx, "查看");
        assert!(page.read_with(cx, |page, _| page.disabled));
        assert_eq!(
            page.read_with(cx, |page, _| page.last.clone()),
            "正在查看林悦"
        );
    }
    #[gpui_kit::test]
    fn card_controls_reflow_real_bounds_and_jump_to_business_key(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| ListPage::new(w, cx));
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(size(px(1200.), px(5000.)));
        draw(cx);
        let page = page.unwrap();
        let state = page.read_with(cx, |p, _| p.grid_state.clone());
        cx.update(|w, _| {
            let contents: Vec<_> = snapshots(w)
                .into_iter()
                .filter(|n| {
                    n.path().contains(&ElementId::from("list-horizontal"))
                        && n.path().last() == Some(&ElementId::from("list-content"))
                })
                .collect();
            assert_eq!(contents.len(), 3);
            assert!(
                contents.iter().all(|n| n.bounds().size.height <= px(24.)),
                "short horizontal labels should stay on one text line"
            );
        });
        click(cx, "固定三列");
        assert_eq!(state.read_with(cx, |s, _| s.geometry().columns()), 3);
        click(cx, "窄容器");
        click(cx, "固定三列");
        assert_eq!(state.read_with(cx, |s, _| s.geometry().columns()), 1);
        click(cx, "增加正文");
        assert!(page.read_with(cx, |p, _| p.grid_tall));
        click(cx, "跳至项目 9901");
        assert!(state.read_with(cx, |s, cx| {
            let v = s.virtualizer().read(cx);
            let snapshot = v.snapshot(v.viewport().unwrap()).unwrap();
            let item = snapshot.item_for_key(&"record-9900".into()).unwrap();
            snapshot.visible_range().contains(&item.index())
        }));
    }
}
