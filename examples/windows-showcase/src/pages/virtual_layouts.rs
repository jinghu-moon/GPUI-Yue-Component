//! Production layout adapters, mounted only on the active tab.
#[cfg(all(test, feature = "virtual-profile"))]
mod native_profile;
use super::super::*;
use gpui_kit::base::{Scrollbar, TestSupportExt as _, virtualization::VirtualList};
use gpui_kit::{AnyElement, App, Axis, ElementId, Entity, Role, ScrollHandle, px};
use std::{cell::Cell, rc::Rc};
use yue_component::{
    VirtualGrid, VirtualGridCellKey, VirtualGridSpan, VirtualMasonry, VirtualMasonryState,
    VirtualizerAlign, VirtualizerBinding, VirtualizerLayout, VirtualizerScrollOptions,
    VirtualizerSizing, VirtualizerSource, VirtualizerState, YueTabItem, YueTabs,
};

const GRID_ROWS: usize = 1_000_000;
const GRID_COLUMNS: usize = 1_000;
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
    frozen: bool,
    rtl: bool,
    spanning: bool,
    builds: [Rc<Cell<usize>>; 3],
}
impl VirtualLayoutsPage {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let binding = VirtualizerBinding::new();
        let rows = cx.new(|_| {
            VirtualizerState::new_attached(
                Axis::Vertical,
                Rc::new(Records(GRID_ROWS)),
                VirtualizerSizing::fixed(px(36.)),
                &binding,
            )
            .unwrap()
        });
        let columns = cx.new(|_| {
            VirtualizerState::new_attached(
                Axis::Horizontal,
                Rc::new(Records(GRID_COLUMNS)),
                VirtualizerSizing::fixed(px(144.)),
                &binding,
            )
            .unwrap()
        });
        let masonry = Self::new_masonry(3, cx);
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
            frozen: true,
            rtl: false,
            spanning: false,
            builds: std::array::from_fn(|_| Rc::new(Cell::new(0))),
        }
    }
    fn new_masonry(lane_count: usize, cx: &mut App) -> Entity<VirtualMasonryState> {
        cx.new(|_| {
            let mut state = VirtualMasonryState::new(
                Rc::new(Records(10_000)),
                VirtualizerSizing::measured_with_estimates(px(140.), |ix| {
                    px(100. + (ix % 4) as f32 * 30.)
                }),
                lane_count,
            )
            .unwrap();
            state
                .set_layout(
                    VirtualizerLayout::default()
                        .gap(px(12.))
                        .overscan(px(72.))
                        .directional_overscan(px(24.)),
                    px(12.),
                )
                .unwrap();
            state
        })
    }
    fn panel(
        &self,
        ix: usize,
        theme: YueTheme,
        owner: gpui_kit::WeakEntity<Self>,
        cx: &App,
    ) -> AnyElement {
        let builds = self.builds[ix].clone();
        let (description, controls, content) = match ix {
            0 => {
                let rows = self.rows.clone();
                let columns = self.columns.clone();
                let frozen = self.frozen;
                let freeze_owner = owner.clone();
                let rtl_owner = owner.clone();
                let span_owner = owner.clone();
                let controls = div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        YueButton::new(
                            "grid-freeze",
                            if frozen {
                                "取消冻结首行首列"
                            } else {
                                "冻结首行首列"
                            },
                        )
                        .theme(theme)
                        .on_click(move |_, _, cx| {
                            let _ = freeze_owner.update(cx, |p, cx| {
                                p.frozen = !p.frozen;
                                cx.notify();
                            });
                        }),
                    )
                    .child(
                        YueButton::new(
                            "grid-direction",
                            if self.rtl {
                                "切换为 LTR"
                            } else {
                                "切换为 RTL"
                            },
                        )
                        .theme(theme)
                        .on_click(move |_, _, cx| {
                            let _ = rtl_owner.update(cx, |p, cx| {
                                p.rtl = !p.rtl;
                                cx.notify();
                            });
                        }),
                    )
                    .child(
                        YueButton::new(
                            "grid-span",
                            if self.spanning {
                                "取消单元格合并"
                            } else {
                                "合并正文 2×3 单元格"
                            },
                        )
                        .theme(theme)
                        .on_click(move |_, _, cx| {
                            let _ = span_owner.update(cx, |p, cx| {
                                p.spanning = !p.spanning;
                                cx.notify();
                            });
                        }),
                    )
                    .child(
                        YueButton::new("grid-last", "定位右下角")
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
                            }),
                    )
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
                                let is_frozen = cell.is_frozen_row() || cell.is_frozen_column();
                                let label = format!(
                                    "{} × {}{}",
                                    cell.row().index() + 1,
                                    cell.column().index() + 1,
                                    if cell.row_span() > 1 || cell.column_span() > 1 {
                                        " · 合并 2×3"
                                    } else {
                                        ""
                                    }
                                );
                                let cell_element = div().bg(gpui_kit::rgb(if is_frozen {
                                    theme.surface_component()
                                } else {
                                    theme.surface()
                                }));
                                cell_element
                                    .id("grid-cell")
                                    .role(Role::Cell)
                                    .aria_label(label.clone())
                                    .aria_row_index(cell.row().index())
                                    .aria_column_index(cell.column().index())
                                    .aria_description(format!(
                                        "第 {} 行，第 {} 列；跨 {} 行、{} 列",
                                        cell.row().index() + 1,
                                        cell.column().index() + 1,
                                        cell.row_span(),
                                        cell.column_span()
                                    ))
                                    .when(cell.row_span() > 1, |this| {
                                        this.aria_row_span(cell.row_span())
                                    })
                                    .when(cell.column_span() > 1, |this| {
                                        this.aria_column_span(cell.column_span())
                                    })
                                    .size_full()
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_r_1()
                                    .border_color(gpui_kit::rgb(theme.border_default()))
                                    .child(label)
                                    .test_support()
                            },
                        )
                        .frozen_rows(usize::from(frozen))
                        .frozen_columns(usize::from(frozen))
                        .right_to_left(self.rtl)
                        .spans(
                            &if self.spanning {
                                vec![VirtualGridSpan::new(
                                    VirtualGridCellKey::new(1usize, 1usize),
                                    2,
                                    3,
                                )]
                            } else {
                                Vec::new()
                            },
                            cx,
                        )
                        .expect(
                            "showcase body span is inside Fixed source and does not cross freezing",
                        )
                        .role(Role::Grid)
                        .aria_label("双轴虚拟网格")
                        .aria_row_count(GRID_ROWS)
                        .aria_column_count(GRID_COLUMNS)
                        .aria_description(
                            "一百万行、一千列；冻结首行首列可切换，单元格说明包含逻辑位置和跨度",
                        )
                        .test_support(),
                    )
                    .child(Scrollbar::vertical(&self.scroll))
                    .child(Scrollbar::horizontal(&self.scroll))
                    .into_any_element();
                (
                    "Grid · 一百万行 × 一千列，支持首行首列冻结、正文合并与 RTL。共用一个原生滚动位置，定位按逻辑起点避开冻结条带。",
                    controls,
                    content,
                )
            }
            1 => {
                let state = self.masonry.clone();
                let rebuild_owner = owner.clone();
                let controls = div()
                    .flex()
                    .gap_2()
                    .child(
                        YueButton::new("masonry-rebuild", "缓存并重建 Masonry")
                            .theme(theme)
                            .on_click(move |_, _, cx| {
                                let _ = rebuild_owner.update(cx, |p, cx| {
                                    if let Ok(cache) = p.masonry.read(cx).measurement_cache() {
                                        let state = Self::new_masonry(cache.lane_count(), cx);
                                        state.update(cx, |s, _| {
                                            s.restore_measurement_cache(&cache).unwrap();
                                        });
                                        p.masonry = state;
                                        cx.notify();
                                    }
                                });
                            }),
                    )
                    .child(
                        YueButton::new("masonry-reflow", "切换三列／五列")
                            .theme(theme)
                            .on_click(move |_, _, cx| {
                                state.update(cx, |s, cx| {
                                    s.reflow(if s.lane_count() == 3 { 5 } else { 3 }).unwrap();
                                    cx.notify();
                                });
                            }),
                    )
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
                                let title = format!("卡片 {} · 第 {} 列", ix + 1, item.lane() + 1);
                                let content =
                                    "卡片内容自然换行，尺寸由真实布局反馈；滚动时保持列归属。"
                                        .repeat(ix % 4 + 1);
                                div()
                                    .id("masonry-card")
                                    .role(Role::ListItem)
                                    .aria_label(title.clone())
                                    .aria_description(content.clone())
                                    .aria_position_in_set(ix + 1)
                                    .aria_size_of_set(10_000)
                                    .test_support()
                                    .w_full()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .border_1()
                                    .border_color(gpui_kit::rgb(theme.border_default()))
                                    .child(title)
                                    .child(content)
                            },
                        )
                        .role(Role::List)
                        .test_support(),
                    )
                    .child(Scrollbar::vertical(&handle))
                    .into_any_element();
                (
                    "Masonry · 一万张自然高度卡片，启用方向性缓冲。可缓存后销毁并重建控制器，恢复兼容测量、分栏和阅读位置。",
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
                                let title = format!("节点 {}", item.index() + 1);
                                div()
                                    .id("horizontal-node")
                                    .role(Role::ListItem)
                                    .aria_label(title.clone())
                                    .aria_description("实际宽度由条目布局确定")
                                    .aria_position_in_set(item.index() + 1)
                                    .aria_size_of_set(2_000)
                                    .test_support()
                                    .w(gpui_kit::rems(8. + (item.index() % 4) as f32 * 2.))
                                    .h_full()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .border_1()
                                    .border_color(gpui_kit::rgb(theme.border_default()))
                                    .child(title)
                                    .child("实际宽度由条目布局确定")
                            },
                        )
                        .role(Role::List)
                        .item_role(None)
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
                            .map(|owner| owner.read(cx).panel(ix, theme, owner.downgrade(), cx))
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
#[path = "virtual_layouts/native_acceptance.rs"]
mod native_acceptance;
#[cfg(all(test, target_os = "windows"))]
#[path = "virtual_layouts/native_tests.rs"]
mod native_tests;

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use std::collections::HashSet;

    fn grid_semantics(cx: &mut gpui_kit::VisualTestContext) -> Vec<(usize, usize, usize, usize)> {
        cx.update(|w, cx| {
            w.draw(cx).clear(cx);
            let nodes = gpui_kit::base::test_support::snapshots(w);
            let grid = nodes.iter().find(|n| n.role() == Some(Role::Grid)).unwrap();
            assert_eq!(grid.row_count(), Some(1_000_000));
            assert_eq!(grid.column_count(), Some(1_000));
            let cells = nodes
                .iter()
                .filter(|n| n.role() == Some(Role::Cell))
                .map(|n| {
                    let row = n.row_index().expect("logical row metadata");
                    let column = n.column_index().expect("logical column metadata");
                    assert!(row < 1_000_000 && column < 1_000);
                    assert!(n.label().unwrap().starts_with(&format!(
                        "{} × {}",
                        row + 1,
                        column + 1
                    )));
                    assert!(n.description().unwrap().contains(&format!(
                        "第 {} 行，第 {} 列",
                        row + 1,
                        column + 1
                    )));
                    (
                        row,
                        column,
                        n.row_span().unwrap_or(1),
                        n.column_span().unwrap_or(1),
                    )
                })
                .collect::<Vec<_>>();
            assert!(!cells.is_empty() && cells.len() < 200);
            let unique = cells.iter().map(|c| (c.0, c.1)).collect::<HashSet<_>>();
            assert_eq!(cells.len(), unique.len());
            cells
        })
    }

    #[gpui_kit::test]
    fn grid_accessibility_preserves_logical_coordinates_counts_and_spans(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let (_, cx) = cx.add_window_view(|_, cx| VirtualLayoutsPage::new(cx));
        cx.simulate_resize(gpui_kit::size(px(1_400.), px(1_000.)));
        assert!(grid_semantics(cx).contains(&(0, 0, 1, 1)));
        click(cx, "grid-span");
        let merged = grid_semantics(cx);
        assert_eq!(merged.iter().filter(|c| **c == (1, 1, 2, 3)).count(), 1);
        assert!(
            !merged
                .iter()
                .any(|c| (1..3).contains(&c.0) && (1..4).contains(&c.1) && (c.0, c.1) != (1, 1))
        );
        click(cx, "grid-direction");
        assert_eq!(
            grid_semantics(cx).into_iter().collect::<HashSet<_>>(),
            merged.into_iter().collect()
        );
        click(cx, "grid-last");
        let last = grid_semantics(cx);
        assert!(last.contains(&(999_999, 999, 1, 1)));
        assert!(
            last.contains(&(0, 0, 1, 1)),
            "frozen corner retains logical coordinates"
        );
    }

    fn named_items(cx: &mut gpui_kit::VisualTestContext, prefix: &str) -> HashSet<String> {
        cx.update(|w, cx| {
            w.draw(cx).clear(cx);
            let items = gpui_kit::base::test_support::snapshots(w)
                .into_iter()
                .filter(|node| node.role() == Some(Role::ListItem))
                .collect::<Vec<_>>();
            assert!(
                !items.is_empty(),
                "visible virtual items need semantic nodes"
            );
            assert!(items.len() < 100, "virtual semantics must remain bounded");
            let names = items
                .iter()
                .map(|item| {
                    let name = item.label().expect("every item needs an accessible name");
                    assert!(name.starts_with(prefix), "unexpected item name: {name}");
                    name.to_owned()
                })
                .collect::<HashSet<_>>();
            assert_eq!(names.len(), items.len(), "no duplicate item semantics");
            names
        })
    }

    #[gpui_kit::test]
    fn masonry_accessibility_names_follow_scroll_rebuild_and_reflow(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let (view, cx) = cx.add_window_view(|_, cx| {
            let mut page = VirtualLayoutsPage::new(cx);
            page.active = 1;
            page
        });
        cx.simulate_resize(gpui_kit::size(px(1_400.), px(1_000.)));
        let first = named_items(cx, "卡片 ");
        view.update(cx, |p, cx| {
            p.masonry.update(cx, |s, cx| {
                s.scroll_to_key(5_000usize.into(), VirtualizerScrollOptions::default())
                    .unwrap();
                cx.notify();
            });
        });
        let scrolled = named_items(cx, "卡片 ");
        assert!(
            first.is_disjoint(&scrolled),
            "names must follow recycled content"
        );
        click(cx, "masonry-rebuild");
        assert_eq!(named_items(cx, "卡片 "), scrolled);
        click(cx, "masonry-reflow");
        let reflowed = named_items(cx, "卡片 ");
        assert!(reflowed.iter().all(|name| {
            let lane = name.split(" · 第 ").nth(1).unwrap().trim_end_matches(" 列");
            (1..=5).contains(&lane.parse::<usize>().unwrap())
        }));
    }

    #[gpui_kit::test]
    fn horizontal_accessibility_names_follow_measured_scroll(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let (_, cx) = cx.add_window_view(|_, cx| {
            let mut page = VirtualLayoutsPage::new(cx);
            page.active = 2;
            page
        });
        cx.simulate_resize(gpui_kit::size(px(1_400.), px(1_000.)));
        let first = named_items(cx, "节点 ");
        click(cx, "horizontal-measured-last");
        let last = named_items(cx, "节点 ");
        assert!(first.is_disjoint(&last));
        assert!(last.contains("节点 2000"));
    }

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
        assert!(view.read_with(cx, |p, _| p.frozen));
        click(cx, "grid-freeze");
        assert!(!view.read_with(cx, |p, _| p.frozen));
        click(cx, "grid-freeze");
        click(cx, "grid-direction");
        assert!(view.read_with(cx, |p, _| p.rtl));
        click(cx, "grid-span");
        assert!(view.read_with(cx, |p, _| p.spanning));
        let merged = cx.update(|w, _| {
            gpui_kit::base::test_support::snapshots(w)
                .into_iter()
                .filter(|node| node.label() == Some("2 × 2 · 合并 2×3"))
                .collect::<Vec<_>>()
        });
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].bounds().size, gpui_kit::size(px(432.), px(72.)));
        click(cx, "grid-direction");
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
        let (old, before) = view.read_with(cx, |p, cx| {
            (
                p.masonry.downgrade(),
                p.masonry.read(cx).measurement_cache().unwrap(),
            )
        });
        click(cx, "masonry-rebuild");
        assert!(old.upgrade().is_none());
        let after = view.read_with(cx, |p, cx| p.masonry.read(cx).measurement_cache().unwrap());
        assert_eq!(after.lane_count(), before.lane_count());
        assert_eq!(after.cross_extent(), before.cross_extent());
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
