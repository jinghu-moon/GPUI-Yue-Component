//! Production layout adapters, mounted only on the active tab.
#[cfg(all(test, feature = "virtual-profile"))]
mod native_profile;
use super::super::*;
use gpui_kit::base::{Scrollbar, TestSupportExt as _, virtualization::VirtualList};
use gpui_kit::{AnyElement, App, Axis, ElementId, Entity, Role, ScrollHandle, px};
use std::{cell::Cell, rc::Rc};
use yue_component::{
    VirtualGrid, VirtualMasonry, VirtualMasonryState, VirtualizerAlign, VirtualizerBinding,
    VirtualizerLayout, VirtualizerScrollOptions, VirtualizerSizing, VirtualizerSource,
    VirtualizerState, YueTabItem, YueTabs,
};

struct Records(usize);
impl VirtualizerSource for Records {
    fn len(&self) -> usize {
        self.0
    }
    fn key_at(&self, ix: usize) -> ElementId {
        ix.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if let ElementId::Integer(key) = key {
            let ix = *key as usize;
            (ix < self.0).then_some(ix)
        } else {
            None
        }
    }
}
pub(crate) struct VirtualLayoutsPage {
    rows: Entity<VirtualizerState>,
    columns: Entity<VirtualizerState>,
    scroll: ScrollHandle,
    masonry: Entity<VirtualMasonryState>,
    horizontal: Entity<VirtualizerState>,
    active: usize,
    builds: [Rc<Cell<usize>>; 3],
}
impl VirtualLayoutsPage {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let binding = VirtualizerBinding::new();
        let rows = cx.new(|_| {
            VirtualizerState::new_attached(
                Axis::Vertical,
                Rc::new(Records(1_000_000)),
                VirtualizerSizing::fixed(px(36.)),
                &binding,
            )
            .unwrap()
        });
        let columns = cx.new(|_| {
            VirtualizerState::new_attached(
                Axis::Horizontal,
                Rc::new(Records(1_000)),
                VirtualizerSizing::fixed(px(144.)),
                &binding,
            )
            .unwrap()
        });
        let masonry = cx.new(|_| {
            let mut state = VirtualMasonryState::new(
                Rc::new(Records(10_000)),
                VirtualizerSizing::measured_with_estimates(px(140.), |ix| {
                    px(100. + (ix % 4) as f32 * 30.)
                }),
                3,
            )
            .unwrap();
            state
                .set_layout(
                    VirtualizerLayout::default().gap(px(12.)).overscan(px(72.)),
                    px(12.),
                )
                .unwrap();
            state
        });
        let horizontal = cx.new(|_| {
            let mut state = VirtualizerState::new(
                Axis::Horizontal,
                Rc::new(Records(2_000)),
                VirtualizerSizing::measured(px(160.)),
            )
            .unwrap();
            state
                .set_layout(VirtualizerLayout::default().gap(px(8.)).overscan(px(72.)))
                .unwrap();
            state
        });
        Self {
            rows,
            columns,
            scroll: ScrollHandle::new(),
            masonry,
            horizontal,
            active: 0,
            builds: std::array::from_fn(|_| Rc::new(Cell::new(0))),
        }
    }
    fn panel(&self, ix: usize, theme: YueTheme, cx: &App) -> AnyElement {
        let builds = self.builds[ix].clone();
        let (description, controls, content) = match ix {
            0 => {
                let rows = self.rows.clone();
                let columns = self.columns.clone();
                let controls = YueButton::new("grid-last", "定位右下角")
                    .theme(theme)
                    .on_click(move |_, _, cx| {
                        for (state, key) in [(&rows, 999_999usize), (&columns, 999usize)] {
                            state.update(cx, |s, cx| {
                                s.scroll_to_key(
                                    key.into(),
                                    VirtualizerScrollOptions::default()
                                        .align(VirtualizerAlign::End),
                                )
                                .unwrap();
                                cx.notify();
                            });
                        }
                    })
                    .into_any_element();
                let content = div()
                    .relative()
                    .size_full()
                    .child(
                        VirtualGrid::new(
                            "showcase-grid",
                            &self.rows,
                            &self.columns,
                            &self.scroll,
                            move |cell, _, _| {
                                builds.set(builds.get() + 1);
                                div()
                                    .id("grid-cell")
                                    .role(Role::Cell)
                                    .size_full()
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_r_1()
                                    .border_color(gpui_kit::rgb(theme.border_default()))
                                    .child(format!(
                                        "{} × {}",
                                        cell.row().index() + 1,
                                        cell.column().index() + 1
                                    ))
                            },
                        )
                        .role(Role::Grid)
                        .test_support(),
                    )
                    .child(Scrollbar::vertical(&self.scroll))
                    .child(Scrollbar::horizontal(&self.scroll))
                    .into_any_element();
                (
                    "Grid · 一百万行 × 一千列，逐单元格双轴虚拟化。两轴共用一个滚动容器，不构建完整行。",
                    controls,
                    content,
                )
            }
            1 => {
                let state = self.masonry.clone();
                let controls = YueButton::new("masonry-reflow", "切换三列／五列")
                    .theme(theme)
                    .on_click(move |_, _, cx| {
                        state.update(cx, |s, cx| {
                            s.reflow(if s.lane_count() == 3 { 5 } else { 3 }).unwrap();
                            cx.notify();
                        });
                    })
                    .into_any_element();
                let handle = self.masonry.read(cx).scroll_handle().clone();
                let content = div()
                    .relative()
                    .size_full()
                    .child(
                        VirtualMasonry::new(
                            "showcase-masonry",
                            &self.masonry,
                            move |item, _, _| {
                                builds.set(builds.get() + 1);
                                let ix = item.item().index();
                                div()
                                    .w_full()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .border_1()
                                    .border_color(gpui_kit::rgb(theme.border_default()))
                                    .child(format!("卡片 {} · 第 {} 列", ix + 1, item.lane() + 1))
                                    .child(
                                        "卡片内容自然换行，尺寸由真实布局反馈；滚动时保持列归属。"
                                            .repeat(ix % 4 + 1),
                                    )
                            },
                        )
                        .role(Role::List)
                        .test_support(),
                    )
                    .child(Scrollbar::vertical(&handle))
                    .into_any_element();
                (
                    "Masonry · 一万张自然高度卡片。初始分配到最短列；实测更新只影响该列，显式切换列数才重新分配。",
                    controls,
                    content,
                )
            }
            _ => {
                let state = self.horizontal.clone();
                let controls = YueButton::new("horizontal-measured-last", "定位最后节点")
                    .theme(theme)
                    .on_click(move |_, _, cx| {
                        state.update(cx, |s, cx| {
                            s.scroll_to_key(
                                1_999usize.into(),
                                VirtualizerScrollOptions::default().align(VirtualizerAlign::End),
                            )
                            .unwrap();
                            cx.notify();
                        });
                    })
                    .into_any_element();
                let handle = self.horizontal.read(cx).scrollbar_handle().unwrap();
                let content = div()
                    .relative()
                    .size_full()
                    .child(
                        VirtualList::new(
                            "showcase-horizontal-measured",
                            &self.horizontal,
                            move |item, _, _| {
                                builds.set(builds.get() + 1);
                                div()
                                    .w(gpui_kit::rems(8. + (item.index() % 4) as f32 * 2.))
                                    .h_full()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .border_1()
                                    .border_color(gpui_kit::rgb(theme.border_default()))
                                    .child(format!("节点 {}", item.index() + 1))
                                    .child("实际宽度由条目布局确定")
                            },
                        )
                        .role(Role::List)
                        .item_role(Some(Role::ListItem))
                        .test_support(),
                    )
                    .child(Scrollbar::horizontal(&handle))
                    .into_any_element();
                (
                    "Measured / Horizontal · 两千个不同宽度节点。复用原生测量、定位、尾部跟随和滚动条机制。",
                    controls,
                    content,
                )
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .pt_4()
            .min_w_0()
            .child(description)
            .child(controls)
            .child(
                div()
                    .w_full()
                    .h(gpui_kit::rems(24.))
                    .relative()
                    .border_1()
                    .border_color(gpui_kit::rgb(theme.border_default()))
                    .child(content),
            )
            .into_any_element()
    }
}
impl Render for VirtualLayoutsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let owner = cx.entity().downgrade();
        let items = ["双轴 Grid", "动态 Masonry", "横向 Measured"]
            .into_iter()
            .enumerate()
            .map(|(ix, label)| {
                let owner = owner.clone();
                YueTabItem::new(format!("layout-{ix}"), label)
                    .panel_tab_stop(false)
                    .content(move |_, cx| {
                        owner
                            .upgrade()
                            .map(|owner| owner.read(cx).panel(ix, theme, cx))
                            .unwrap_or_else(|| div().into_any_element())
                    })
            })
            .collect::<Vec<_>>();
        div().id("virtual-layouts-content").size_full().min_w_0().overflow_y_scroll().p_6().flex().flex_col().gap_4()
            .child(div().text_2xl().child("虚拟布局适配器"))
            .child("只挂载当前示例；键盘可切换标签，按钮可定位或调整列数。尺寸与滚动机制由 Base 拥有。")
            .child(YueTabs::new("virtual-layout-tabs", Some(format!("layout-{}", self.active).into())).theme(theme).items(items)
                .on_change(cx.listener(|page, request: &yue_component::TabsChangeRequest, _, cx| {
                    page.active = request.key().strip_prefix("layout-").unwrap().parse().unwrap(); cx.notify();
                })))
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "virtual_layouts/native_tests.rs"]
mod native_tests;

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    fn click(cx: &mut gpui_kit::VisualTestContext, id: &'static str) {
        let point = cx.update(|w, _| {
            gpui_kit::base::test_support::snapshots(w)
                .into_iter()
                .find(|node| node.path().last() == Some(&ElementId::from(id)))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        cx.update(|w, cx| w.draw(cx).clear(cx));
    }
    #[gpui_kit::test]
    fn layout_tabs_mount_one_adapter_and_operate_real_controls(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let (view, cx) = cx.add_window_view(|_, cx| VirtualLayoutsPage::new(cx));
        cx.simulate_resize(gpui_kit::size(px(1_400.), px(1_000.)));
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(view.read_with(cx, |p, _| p.builds[1].get()), 0);
        click(cx, "grid-last");
        assert!(
            view.read_with(cx, |p, cx| p.scroll.offset().x < px(-100_000.)
                && p.rows.read(cx).viewport().unwrap().offset() > 1_000_000.)
        );
        view.update(cx, |p, cx| {
            p.active = 1;
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        click(cx, "masonry-reflow");
        assert_eq!(
            view.read_with(cx, |p, cx| p.masonry.read(cx).lane_count()),
            5
        );
        view.update(cx, |p, cx| {
            p.active = 2;
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        click(cx, "horizontal-measured-last");
        assert!(!view.read_with(cx, |p, cx| p.horizontal.read(cx).has_scroll_command()));
    }
}
