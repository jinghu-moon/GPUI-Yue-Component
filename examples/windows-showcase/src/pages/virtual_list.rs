use super::super::*;
use gpui_kit::base::{Scrollbar, TestSupportExt as _, virtualization::VirtualList};
use gpui_kit::{AnyElement, App, Axis, ElementId, Entity, Focusable as _, Role, WeakEntity, px};
use std::{cell::Cell, rc::Rc, time::Duration};
use yue_component::{
    InputState, TabsChangeRequest, VirtualizerAlign, VirtualizerChange, VirtualizerLayout,
    VirtualizerMotion, VirtualizerScrollOptions, VirtualizerSizing, VirtualizerSource,
    VirtualizerState, YueTabItem, YueTabs, YueVirtualList,
};

struct Records {
    first: usize,
    count: usize,
}

/// Preparation survives offset-only frames; GPUI still owns this view's live
/// layout and interaction. Theme is sampled while rendering, not cached here.
struct PreparedCard {
    index: usize,
    body: gpui_kit::SharedString,
}
impl Render for PreparedCard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        div()
            .w_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .border_b_1()
            .border_color(gpui_kit::rgb(theme.border_default()))
            .child(format!("动态记录 {}", self.index + 1))
            .child(self.body.clone())
    }
}
impl VirtualizerSource for Records {
    fn len(&self) -> usize {
        self.count
    }
    fn key_at(&self, index: usize) -> ElementId {
        (self.first + index).into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if let ElementId::Integer(key) = key {
            let key = *key as usize;
            if key >= self.first && key - self.first < self.count {
                Some(key - self.first)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Demo {
    Fixed,
    Known,
    Measured,
    Horizontal,
    Cards,
    Editor,
    Logs,
    Empty,
}
impl Demo {
    const ALL: [Self; 8] = [
        Self::Fixed,
        Self::Known,
        Self::Measured,
        Self::Horizontal,
        Self::Cards,
        Self::Editor,
        Self::Logs,
        Self::Empty,
    ];
    fn ix(self) -> usize {
        self as usize
    }
    fn key(self) -> &'static str {
        [
            "fixed",
            "known",
            "measured",
            "horizontal",
            "cards",
            "editor",
            "logs",
            "empty",
        ][self.ix()]
    }
    fn label(self) -> &'static str {
        [
            "百万记录",
            "分组记录",
            "动态卡片",
            "横向时间轴",
            "多列卡片",
            "编辑保留",
            "追加日志",
            "空状态",
        ][self.ix()]
    }
    fn description(self) -> &'static str {
        match self {
            Self::Fixed => {
                "Fixed · 一百万项。固定行高 36px，不创建逐项尺寸表；头插后保留原记录锚点。"
            }
            Self::Known => "Known · 一万项。组标题 52px、记录 36px；使用共享尺寸和前缀直接定位。",
            Self::Measured => "Measured · 一万张卡片。真实文字自然换行；展开首张卡片后重新测量。",
            Self::Horizontal => {
                "Known / Horizontal · 五百个时间节点。宽度为 144px 或 208px，复用 Base 与横向滚动条。"
            }
            Self::Cards => {
                "Fixed · 九千张卡片，每行三张。只虚拟化三千行，并非单元格级双轴 Grid；选择按卡片身份保存。"
            }
            Self::Editor => {
                "Measured · 一千条记录。首行使用一个真实 Input；滚出视口、返回或切换示例后保留编辑值。"
            }
            Self::Logs => {
                "Measured · 不同高度的构建日志。手动追加，无后台定时器；在尾部时跟随，查看历史时保持阅读位置。"
            }
            Self::Empty => {
                "Fixed · 空数据与一千项之间切换。空视口不构建条目，恢复数据后可正常定位。"
            }
        }
    }
}
#[derive(Clone, Copy)]
enum Command {
    First,
    Last,
    Near,
    Prepend,
    Expand,
    SelectCard(usize),
    Append,
    Follow,
    Pause,
    Fill,
    Clear,
    Stats,
}
impl Command {
    fn key(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Last => "last",
            Self::Near => "near",
            Self::Prepend => "prepend",
            Self::Expand => "expand",
            Self::SelectCard(_) => "select-card",
            Self::Append => "append",
            Self::Follow => "follow",
            Self::Pause => "pause",
            Self::Fill => "fill",
            Self::Clear => "clear",
            Self::Stats => "stats",
        }
    }
}

pub(crate) struct VirtualListPage {
    states: [Entity<VirtualizerState>; 8],
    builds: [Rc<Cell<usize>>; 8],
    active: Demo,
    expanded: Rc<Cell<bool>>,
    selected_card: Rc<Cell<Option<usize>>>,
    editor: Entity<InputState>,
    first: usize,
    count: usize,
    log_count: usize,
    empty_count: usize,
    report: String,
}

fn sizing(demo: Demo, count: usize) -> VirtualizerSizing {
    // Explicit pixel extents are geometry/performance fixtures, not UI spacing.
    // Measured content and controls use the application's relative scale.
    match demo {
        Demo::Fixed | Demo::Empty => VirtualizerSizing::fixed(px(36.)),
        Demo::Known => VirtualizerSizing::known(
            (0..count)
                .map(|i| px(if i % 5 == 0 { 52. } else { 36. }))
                .collect::<Vec<_>>(),
        ),
        Demo::Horizontal => VirtualizerSizing::known(
            (0..count)
                .map(|i| px(if i % 4 == 0 { 208. } else { 144. }))
                .collect::<Vec<_>>(),
        ),
        Demo::Cards => VirtualizerSizing::fixed(px(176.)),
        Demo::Editor | Demo::Logs => VirtualizerSizing::measured(px(64.)),
        Demo::Measured => VirtualizerSizing::measured(px(100.)),
    }
}

impl VirtualListPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor =
            cx.new(|cx| InputState::new(window, cx).placeholder("编辑后滚出，再返回这一行"));
        let states = std::array::from_fn(|ix| {
            let demo = Demo::ALL[ix];
            let count = [1_000_000, 10_000, 10_000, 500, 3_000, 1_000, 200, 0][ix];
            cx.new(|_| {
                let mut state = VirtualizerState::new(
                    if demo == Demo::Horizontal {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    },
                    Rc::new(Records {
                        first: 1_000,
                        count,
                    }),
                    sizing(demo, count),
                )
                .unwrap();
                state
                    .set_layout(VirtualizerLayout::default().overscan(px(72.)).gap(px(
                        if matches!(demo, Demo::Horizontal | Demo::Cards) {
                            8.
                        } else {
                            0.
                        },
                    )))
                    .unwrap();
                state
            })
        });
        let focus = editor.read(cx).focus_handle(cx);
        states[Demo::Editor.ix()].update(cx, |s, _| {
            s.set_focus_handle(1_000usize.into(), Some(focus)).unwrap();
        });
        states[Demo::Logs.ix()].update(cx, |s, _| s.set_follow_tail(true).unwrap());
        Self {
            states,
            builds: std::array::from_fn(|_| Rc::new(Cell::new(0))),
            active: Demo::Fixed,
            expanded: Rc::new(Cell::new(false)),
            selected_card: Rc::new(Cell::new(None)),
            editor,
            first: 1_000,
            count: 1_000_000,
            log_count: 200,
            empty_count: 0,
            report: "点击“读取统计”查看当前场景的最近布局；renderer 为累计构建次数，不是帧率。"
                .into(),
        }
    }
    fn len(&self, demo: Demo) -> usize {
        match demo {
            Demo::Fixed => self.count,
            Demo::Known | Demo::Measured => 10_000,
            Demo::Horizontal => 500,
            Demo::Cards => 3_000,
            Demo::Editor => 1_000,
            Demo::Logs => self.log_count,
            Demo::Empty => self.empty_count,
        }
    }
    fn seek(&self, demo: Demo, last: bool, cx: &mut Context<Self>) {
        let count = self.len(demo);
        if count == 0 {
            return;
        }
        let first = if demo == Demo::Fixed {
            self.first
        } else {
            1_000
        };
        let key = first + if last { count - 1 } else { 0 };
        self.states[demo.ix()].update(cx, |state, cx| {
            state
                .scroll_to_key(
                    key.into(),
                    VirtualizerScrollOptions::default()
                        .align(if last {
                            VirtualizerAlign::End
                        } else {
                            VirtualizerAlign::Start
                        })
                        .reduced_motion(cx.reduce_motion()),
                )
                .unwrap();
            cx.notify();
        });
    }
    fn command(&mut self, demo: Demo, command: Command, cx: &mut Context<Self>) {
        match command {
            Command::First | Command::Last => self.seek(demo, matches!(command, Command::Last), cx),
            Command::Near => self.states[demo.ix()].update(cx, |state, cx| {
                if let Some(snapshot) = state.viewport().and_then(|v| state.snapshot(v).ok()) {
                    let index = (snapshot.visible_range().start + 2).min(snapshot.len() - 1);
                    state
                        .scroll_to_key(
                            snapshot.item(index).unwrap().key().clone(),
                            VirtualizerScrollOptions::default()
                                .align(VirtualizerAlign::Start)
                                .motion(VirtualizerMotion::Smooth {
                                    duration: Duration::from_millis(180),
                                    max_distance: px(400.),
                                })
                                .reduced_motion(cx.reduce_motion()),
                        )
                        .unwrap();
                    cx.notify();
                }
            }),
            Command::Prepend => {
                if self.first == 0 {
                    return;
                }
                self.first -= 1;
                self.count += 1;
                self.states[demo.ix()].update(cx, |state, cx| {
                    state
                        .replace_source(
                            Rc::new(Records {
                                first: self.first,
                                count: self.count,
                            }),
                            sizing(demo, self.count),
                            VirtualizerChange::Splice {
                                old: 0..0,
                                inserted: 1,
                            },
                        )
                        .unwrap();
                    cx.notify();
                });
            }
            Command::Expand => {
                self.expanded.set(!self.expanded.get());
                self.states[demo.ix()].update(cx, |state, cx| {
                    state.invalidate_items(0..1).unwrap();
                    cx.notify();
                });
            }
            Command::SelectCard(key) => self.selected_card.set(Some(key)),
            Command::Append => {
                let old = self.log_count;
                self.log_count += 20;
                self.states[demo.ix()].update(cx, |state, cx| {
                    state
                        .replace_source(
                            Rc::new(Records {
                                first: 1_000,
                                count: self.log_count,
                            }),
                            sizing(demo, self.log_count),
                            VirtualizerChange::Splice {
                                old: old..old,
                                inserted: 20,
                            },
                        )
                        .unwrap();
                    cx.notify();
                });
            }
            Command::Follow | Command::Pause => {
                let follow = matches!(command, Command::Follow);
                self.states[demo.ix()].update(cx, |state, cx| {
                    state.set_follow_tail(follow).unwrap();
                    cx.notify();
                });
                if follow {
                    self.seek(demo, true, cx);
                }
            }
            Command::Fill | Command::Clear => {
                self.empty_count = if matches!(command, Command::Fill) {
                    1_000
                } else {
                    0
                };
                self.states[demo.ix()].update(cx, |state, cx| {
                    state
                        .replace_source(
                            Rc::new(Records {
                                first: 1_000,
                                count: self.empty_count,
                            }),
                            sizing(demo, self.empty_count),
                            VirtualizerChange::Replace {
                                preserve_anchor: false,
                            },
                        )
                        .unwrap();
                    cx.notify();
                });
            }
            Command::Stats => {
                let state = self.states[demo.ix()].read(cx);
                self.report = state.viewport().and_then(|v| state.snapshot(v).ok()).map(|s| {
                    format!("{}：可见 {:?}（{} 项），工作 {:?}（{} 项），主轴总长 {:.0}px{}；累计 renderer {} 次{}",
                        demo.label(), s.visible_range(), s.visible_range().len(), s.rendered_range(), s.rendered_range().len(),
                        s.total_extent(), if s.is_estimated() { "（估算）" } else { "" }, self.builds[demo.ix()].get(),
                        if demo == Demo::Logs { if state.is_following_tail() { "；正在跟随尾部" } else { "；当前查看历史" } } else { "" })
                }).unwrap_or_else(|| "当前场景尚未布局".into());
            }
        }
        cx.notify();
    }
    fn button(
        &self,
        demo: Demo,
        label: &'static str,
        command: Command,
        theme: YueTheme,
        page: &WeakEntity<Self>,
    ) -> YueButton {
        let page = page.clone();
        YueButton::new((ElementId::from(demo.key()), command.key()), label)
            .theme(theme)
            .on_click(move |_, _, cx| {
                let _ = page.update(cx, |page, cx| page.command(demo, command, cx));
            })
    }
    fn scenario(
        &self,
        demo: Demo,
        theme: YueTheme,
        page: &WeakEntity<Self>,
        cx: &App,
    ) -> AnyElement {
        let mut controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                self.button(demo, "回到开头", Command::First, theme, page)
                    .disabled(self.len(demo) == 0),
            )
            .child(
                self.button(demo, "定位末尾", Command::Last, theme, page)
                    .disabled(self.len(demo) == 0),
            );
        match demo {
            Demo::Fixed => {
                controls = controls
                    .child(
                        self.button(demo, "头部插入一项", Command::Prepend, theme, page)
                            .disabled(self.first == 0),
                    )
                    .child(self.button(demo, "向后平移两行", Command::Near, theme, page))
            }
            Demo::Measured => {
                controls = controls.child(self.button(
                    demo,
                    "展开／收起首张卡片",
                    Command::Expand,
                    theme,
                    page,
                ))
            }
            Demo::Logs => {
                controls = controls
                    .child(self.button(demo, "追加二十条", Command::Append, theme, page))
                    .child(self.button(demo, "跟随最新", Command::Follow, theme, page))
                    .child(self.button(demo, "暂停跟随", Command::Pause, theme, page))
                    .child(format!("共 {} 条日志", self.log_count))
            }
            Demo::Empty => {
                controls = controls
                    .child(
                        self.button(demo, "加载示例数据", Command::Fill, theme, page)
                            .disabled(self.empty_count > 0),
                    )
                    .child(
                        self.button(demo, "清空示例", Command::Clear, theme, page)
                            .disabled(self.empty_count == 0),
                    )
            }
            _ => {}
        }
        controls = controls.child(self.button(demo, "读取统计", Command::Stats, theme, page));
        let builds = self.builds[demo.ix()].clone();
        let expanded = self.expanded.clone();
        let selected = self.selected_card.clone();
        let input = self.editor.clone();
        let target = page.clone();
        let render = move |item: yue_component::VirtualizerItem, _: &mut Window, _: &mut App| {
            builds.set(builds.get() + 1);
            let ix = item.index();
            match demo {
                Demo::Measured => {
                    let body = if ix == 0 && expanded.get() {
                        "这是一张可展开的长卡片。".repeat(100)
                    } else {
                        "卡片中的自然换行文字按视口宽度测量；离屏项不构建。".repeat(ix % 3 + 1)
                    };
                    div()
                        .w_full()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .border_b_1()
                        .border_color(gpui_kit::rgb(theme.border_default()))
                        .child(format!("动态记录 {}", ix + 1))
                        .child(body)
                        .into_any_element()
                }
                Demo::Horizontal => div()
                    .size_full()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .border_1()
                    .border_color(gpui_kit::rgb(theme.border_default()))
                    .child(format!("里程碑 {}", ix + 1))
                    .child(["需求确认", "组件实现", "测试回归", "交付评审"][ix % 4])
                    .child(format!("节点宽度 {:.0}px", f32::from(item.extent())))
                    .into_any_element(),
                Demo::Cards => {
                    let mut row = div().size_full().flex().gap_3().px_2().py_2();
                    for col in 0..3 {
                        let card = ix * 3 + col + 1;
                        let page = target.clone();
                        row = row.child(
                            div()
                                .h_full()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .border_1()
                                .border_color(gpui_kit::rgb(if selected.get() == Some(card) {
                                    theme.accent_solid()
                                } else {
                                    theme.border_default()
                                }))
                                .child(div().truncate().child(format!("项目 {card}")))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                                        .child("三列卡片按整行虚拟化"),
                                )
                                .child(
                                    YueButton::new(
                                        ("virtual-card-select", card),
                                        if selected.get() == Some(card) {
                                            "已选中"
                                        } else {
                                            "选择卡片"
                                        },
                                    )
                                    .theme(theme)
                                    .accessibility_label(format!("选择项目 {card}"))
                                    .on_click(
                                        move |_, _, cx| {
                                            let _ = page.update(cx, |p, cx| {
                                                p.command(demo, Command::SelectCard(card), cx)
                                            });
                                        },
                                    ),
                                ),
                        );
                    }
                    row.into_any_element()
                }
                Demo::Editor => {
                    let row = div()
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_3()
                        .p_3()
                        .border_b_1()
                        .border_color(gpui_kit::rgb(theme.border_default()))
                        .child(format!("记录 {}", ix + 1));
                    if ix == 0 {
                        row.child(
                            YueInput::new("virtual-record-editor", theme, &input)
                                .accessibility_label("首条记录备注")
                                .width(gpui_kit::rems(18.)),
                        )
                        .into_any_element()
                    } else {
                        row.child("此行无需编辑器，不创建 Input Entity")
                            .into_any_element()
                    }
                }
                Demo::Logs => div()
                    .w_full()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .border_b_1()
                    .border_color(gpui_kit::rgb(theme.border_default()))
                    .child(format!(
                        "{} · 构建事件 {:05}",
                        if ix.is_multiple_of(7) { "WARN" } else { "INFO" },
                        ix + 1
                    ))
                    .child("正在处理组件资源，测量结果已写入日志。".repeat(ix % 3 + 1))
                    .into_any_element(),
                _ => div()
                    .size_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .when(demo == Demo::Known && ix.is_multiple_of(5), |d| {
                        d.bg(gpui_kit::rgb(theme.surface_component()))
                    })
                    .child(if demo == Demo::Known && ix.is_multiple_of(5) {
                        format!("分组 {}", ix / 5 + 1)
                    } else {
                        format!("记录 {:?}", item.key())
                    })
                    .into_any_element(),
            }
        };
        let list = if demo == Demo::Horizontal {
            let handle = self.states[demo.ix()].read(cx).scrollbar_handle().unwrap();
            div()
                .relative()
                .size_full()
                .child(
                    VirtualList::new(("virtual-demo", demo.ix()), &self.states[demo.ix()], render)
                        .role(Role::List)
                        .item_role(Some(Role::ListItem))
                        .accessibility_label(demo.label())
                        .test_support(),
                )
                .child(Scrollbar::horizontal(&handle))
                .into_any_element()
        } else if demo == Demo::Measured {
            let builds = self.builds[demo.ix()].clone();
            let expanded = self.expanded.get();
            YueVirtualList::new_with_item_views(
                ("virtual-demo", demo.ix()),
                &self.states[demo.ix()],
                move |item, _, cx| {
                    builds.set(builds.get() + 1);
                    let ix = item.index();
                    let body = if ix == 0 && expanded {
                        "这是一张可展开的长卡片。".repeat(100)
                    } else {
                        "卡片中的自然换行文字按视口宽度测量；离屏项不构建。".repeat(ix % 3 + 1)
                    };
                    cx.new(|_| PreparedCard {
                        index: ix,
                        body: body.into(),
                    })
                },
            )
            .content_revision(u64::from(expanded))
            .theme(theme)
            .accessibility_label(demo.label())
            .into_any_element()
        } else {
            YueVirtualList::new(("virtual-demo", demo.ix()), &self.states[demo.ix()], render)
                .theme(theme)
                .accessibility_label(demo.label())
                .into_any_element()
        };
        let mut viewport = div()
            .id(("virtual-viewport", demo.ix()))
            .test_support()
            .relative()
            .w_full()
            .h(gpui_kit::rems(if demo == Demo::Horizontal {
                12.
            } else {
                22.
            }))
            .flex_shrink_0()
            .min_h_0()
            .border_1()
            .border_color(gpui_kit::rgb(theme.border_default()))
            .child(list);
        if demo == Demo::Empty && self.empty_count == 0 {
            viewport = viewport.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(gpui_kit::rgb(theme.text_secondary()))
                    .child("暂无记录，使用“加载示例数据”恢复列表"),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .min_w_0()
            .pt_4()
            .child(demo.description())
            .child(controls)
            .when(demo == Demo::Cards, |d| {
                d.child(match self.selected_card.get() {
                    Some(card) => format!("已选择项目 {card}；滚出再返回仍保留选择"),
                    None => "尚未选择卡片".into(),
                })
            })
            .child(viewport)
            .into_any_element()
    }
}

impl Render for VirtualListPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let page = cx.entity().downgrade();
        let items = Demo::ALL.map(|demo| {
            let page = page.clone();
            YueTabItem::new(demo.key(), demo.label())
                .panel_tab_stop(false)
                .content(move |_, cx| {
                    page.upgrade()
                        .map(|p| p.read(cx).scenario(demo, theme, &page, cx))
                        .unwrap_or_else(|| div().into_any_element())
                })
        });
        div().id("virtual-list-content").debug_selector(|| "virtual-list-content".into())
            .size_full().min_w_0().overflow_y_scroll().p_6().flex().flex_col().gap_4()
            .child(div().text_2xl().child("VirtualList 虚拟滚动"))
            .child("只挂载当前示例；切换时保留数据和滚动状态。滚动中的条目不播放入场动画。")
            .child(YueTabs::new("virtual-scenarios", Some(self.active.key().into())).theme(theme)
                .accessibility_label("虚拟滚动示例类型").items(items)
                .on_change(cx.listener(|page, request: &TabsChangeRequest, _, cx| {
                    page.active = Demo::ALL.into_iter().find(|demo| demo.key() == request.key().as_ref()).unwrap();
                    page.report = "示例已切换，点击“读取统计”获取本场景的最新布局".into();
                    cx.notify();
                })))
            .child(div().text_sm().text_color(gpui_kit::rgb(theme.text_secondary())).child(self.report.clone()))
            .child(div().text_sm().text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("性能提示：条目总数决定数据与尺寸索引成本；工作项数量和条目内部复杂度决定渲染成本。统计不代表真实 GPU 帧率。"))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
#[path = "virtual_list/tests.rs"]
mod tests;
