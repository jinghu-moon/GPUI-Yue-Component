//! Grouped showcase navigation, using Base disclosure behavior and motion.
use super::*;
use gpui_kit::base::{Button, Collapsible};
use yue_component::{Icon, IconSize, MotionDuration, MotionEase};

const COMPONENTS: &[(&str, &str, ShowcasePage)] = &[
    ("avatar-page", "头像 Avatar", ShowcasePage::Avatar),
    ("badge-page", "徽标 Badge", ShowcasePage::Badge),
    (
        "breadcrumb-page",
        "面包屑 Breadcrumb",
        ShowcasePage::Breadcrumb,
    ),
    ("button-page", "按钮 Button", ShowcasePage::Button),
    ("card-page", "卡片 Card", ShowcasePage::Card),
    ("checkbox-page", "复选框 Checkbox", ShowcasePage::Checkbox),
    ("collapse-page", "折叠面板 Collapse", ShowcasePage::Collapse),
    (
        "color-picker-page",
        "颜色选择器 ColorPicker",
        ShowcasePage::ColorPicker,
    ),
    ("date-picker-page", "日期选择 DatePicker", ShowcasePage::DatePicker),
    ("descriptions-page", "描述列表 Descriptions", ShowcasePage::Descriptions),
    ("dialog-page", "对话框 Dialog", ShowcasePage::Dialog),
    ("divider-page", "分隔线 Divider", ShowcasePage::Divider),
    ("drawer-page", "抽屉 Drawer", ShowcasePage::Drawer),
    ("dropdown-page", "下拉菜单 Dropdown", ShowcasePage::Dropdown),
    ("empty-page", "空状态 Empty", ShowcasePage::Empty),
    ("form-page", "表单 Form", ShowcasePage::Form),
    ("icon-page", "图标 Icon", ShowcasePage::Icon),
    (
        "image-preview-page",
        "图片预览 ImagePreview",
        ShowcasePage::ImagePreview,
    ),
    ("input-page", "输入框 Input", ShowcasePage::Input),
    ("link-page", "链接 Link", ShowcasePage::Link),
    ("list-page", "列表 List", ShowcasePage::List),
    ("pagination-page", "分页 Pagination", ShowcasePage::Pagination),
    ("progress-page", "进度 Progress", ShowcasePage::Progress),
    ("radio-page", "单选框 Radio", ShowcasePage::Radio),
    ("resizable-page", "可调整分栏 Resizable", ShowcasePage::Resizable),
    ("scrollbar-page", "滚动条 Scrollbar", ShowcasePage::Scrollbar),
    (
        "segmented-page",
        "分段控制器 Segmented",
        ShowcasePage::Segmented,
    ),
    ("select-page", "下拉选择器 Select", ShowcasePage::Select),
    ("shortcut-page", "快捷键 Shortcut", ShowcasePage::Shortcut),
    ("skeleton-page", "骨架屏 Skeleton", ShowcasePage::Skeleton),
    ("slider-page", "滑动条 Slider", ShowcasePage::Slider),
    ("spinner-page", "旋转指示器 Spinner", ShowcasePage::Spinner),
    ("steps-page", "步骤条 Steps", ShowcasePage::Steps),
    ("switch-page", "开关 Switch", ShowcasePage::Switch),
    ("table-page", "表格 Table", ShowcasePage::Table),
    ("tabs-page", "标签页 Tabs", ShowcasePage::Tabs),
    ("tag-page", "标签 Tag", ShowcasePage::Tag),
    ("time-picker-page", "时间选择 TimePicker", ShowcasePage::TimePicker),
    ("toast-page", "消息通知 Toast", ShowcasePage::Toast),
    ("tooltip-page", "文字提示 Tooltip", ShowcasePage::Tooltip),
    ("tree-page", "树 Tree", ShowcasePage::Tree),
    ("tree-select-page", "树选择器 TreeSelect", ShowcasePage::TreeSelect),
    (
        "virtual-layouts-page",
        "虚拟布局 Grid / Masonry",
        ShowcasePage::VirtualLayouts,
    ),
    (
        "virtual-list-page",
        "虚拟列表 VirtualList",
        ShowcasePage::VirtualList,
    ),
];
const DESIGN: &[(&str, &str, ShowcasePage)] = &[
    ("color-page", "色彩 Color", ShowcasePage::Color),
    (
        "dark-mode-page",
        "深色模式 Dark Mode",
        ShowcasePage::DarkMode,
    ),
    ("font-page", "字体 Font", ShowcasePage::Font),
    ("icon-design-page", "图标 Icon", ShowcasePage::IconDesign),
    ("layout-page", "布局 Layout", ShowcasePage::Layout),
    ("motion-page", "动效 Motion", ShowcasePage::Motion),
];

pub(super) fn sidebar(
    window: &mut Window,
    theme: YueTheme,
    view: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let motion_off = view.motion_off;
    let probe_frames = view.probe_frames;
    let probe = transition(
        "showcase-motion-probe",
        if view.probe_held { 1.0_f32 } else { 0.0 },
        Transition::new(std::time::Duration::from_millis(1000)),
        window,
        cx,
    );
    let groups = [SidebarGroup::Components, SidebarGroup::Design]
        .into_iter()
        .map(|group| section(group, view, theme, window, cx).into_any_element())
        .collect::<Vec<_>>();
    let showcase = cx.entity();
    div()
        .debug_selector(|| "showcase-sidebar".into())
        .w(yue_component::layout::sidebar_width(theme))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_2()
        .py_4()
        .border_r_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
        .child(div().px_4().pb_4().text_lg().font_weight(gpui_kit::FontWeight::BOLD).child("Yue Components"))
        .children(groups)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .px_4()
                .py_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("动效"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child("界面动效")
                        .child(
                            YueSwitch::new("showcase-motion", theme, !motion_off, "界面动效")
                                .size(SwitchSize::Small)
                                .on_change(move |checked, _, _, cx| {
                                    showcase.update(cx, |showcase, cx| {
                                        showcase.motion_off = !checked;
                                        cx.set_reduce_motion(!checked);
                                        cx.notify();
                                    });
                                }),
                        ),
                )
                .child(
                    div()
                        .id("showcase-motion-probe-box")
                        .flex()
                        .items_center()
                        .h(gpui_kit::px(30.))
                        .px_2()
                        .rounded(gpui_kit::px(4.))
                        .border_1()
                        .border_color(gpui_kit::rgb(theme.border_default()))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(
                            |showcase, _, _, cx| {
                                showcase.probe_held = true;
                                cx.notify();
                            },
                        ))
                        .on_mouse_up(MouseButton::Left, cx.listener(
                            |showcase, _, _, cx| {
                                showcase.probe_held = false;
                                cx.notify();
                            },
                        ))
                        .on_mouse_up_out(MouseButton::Left, cx.listener(
                            |showcase, _, _, cx| {
                                showcase.probe_held = false;
                                cx.notify();
                            },
                        ))
                        .child(
                            div()
                                .w(gpui_kit::px(140.))
                                .h(gpui_kit::px(4.))
                                .rounded(gpui_kit::px(2.))
                                .bg(gpui_kit::rgb(theme.border_default()))
                                .child(
                                    div()
                                        .h_full()
                                        .w(gpui_kit::px(140. * probe))
                                        .rounded(gpui_kit::px(2.))
                                        .bg(gpui_kit::rgb(theme.accent_solid())),
                                ),
                        ),
                )
                .child(readout(
                    theme,
                    format!(
                        "按住进度条 1 秒：采样 {:.2} · 已重绘 {} 帧。条子爬到 1.00 且帧数上几十，说明逐帧重绘在跑；瞬间满格而帧数只有个位，说明帧没跟上。",
                        probe, probe_frames
                    ),
                )),
        )
        .overflow_y_scrollbar()
}

#[derive(Clone, Copy)]
enum SidebarGroup {
    Components,
    Design,
}

fn section(
    group: SidebarGroup,
    view: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let (id, title, index, pages) = match group {
        SidebarGroup::Components => ("sidebar-components", "组件", 0, COMPONENTS),
        SidebarGroup::Design => ("sidebar-design", "设计", 1, DESIGN),
    };
    let open = view.sidebar_open[index];
    let progress = transition(
        (id, "reveal"),
        f32::from(open),
        yue_component::motion::transition(
            theme,
            if open {
                MotionDuration::Enter
            } else {
                MotionDuration::Exit
            },
            if open {
                MotionEase::Enter
            } else {
                MotionEase::Stow
            },
        ),
        window,
        cx,
    )
    .clamp(0.0, 1.0);
    let focus = window
        .use_keyed_state((gpui_kit::ElementId::from(id), "focus"), cx, |_, cx| {
            cx.focus_handle()
        })
        .read(cx)
        .clone();
    let header = Button::new(id)
        .track_focus(&focus)
        .accessibility_label(title)
        .aria_expanded(open)
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .py_2()
        .text_sm()
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .text_color(gpui_kit::rgb(theme.text_secondary()))
        .cursor_pointer()
        .focus_visible(|style| {
            style
                .bg(gpui_kit::rgb(theme.accent_subtle()))
                .text_color(gpui_kit::rgb(theme.accent_subtle_text()))
        })
        .on_click(cx.listener(move |view, _, window, cx| {
            view.sidebar_open[index] = !view.sidebar_open[index];
            window.focus(&focus, cx);
            cx.notify();
        }))
        .child(title)
        .child(
            Icon::new(outline::ChevronRight)
                .size(IconSize::Small.rems(theme))
                .color(theme.text_secondary())
                .rotation(progress * 0.25),
        );
    let content = div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .children(pages.iter().map(|&(id, label, page)| {
            let active = view.page == page;
            Button::new(id)
                .accessibility_label(label)
                .selected(active)
                .tab_stop(open)
                .disabled(!open)
                .w_full()
                .justify_start()
                .px_4()
                .py_2()
                .text_color(gpui_kit::rgb(if active {
                    theme.accent_subtle_text()
                } else {
                    theme.text_secondary()
                }))
                .when(active, |item| item.bg(gpui_kit::rgb(theme.accent_subtle())))
                .cursor_pointer()
                .focus_visible(|style| style.bg(gpui_kit::rgb(theme.accent_subtle())))
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.page = page;
                    view.sync_theme(cx);
                    cx.notify();
                }))
                .child(label)
        }));
    Collapsible::new()
        .open(open)
        // The gutter between a row's state fill and the panel edge belongs to the group, not to
        // each row: the header and its pages share one inset, and their text keeps its spine.
        .px_2()
        .reveal((gpui_kit::ElementId::from(id), "content"), progress)
        .child(header)
        .content(content)
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
    use gpui_kit::{Entity, Modifiers, TestAppContext, VisualTestContext};

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }

    fn settle(cx: &mut VisualTestContext, millis: u64) {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(millis));
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });
    }

    fn node(cx: &mut VisualTestContext, id: &'static str) -> ElementSnapshot {
        cx.update(|window, _| {
            snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&gpui_kit::ElementId::from(id)))
                .unwrap_or_else(|| panic!("sidebar node {id}"))
        })
    }

    fn open(cx: &mut TestAppContext) -> (Entity<Showcase>, &mut VisualTestContext) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Showcase::new);
        draw(cx);
        settle(cx, 500);
        (view, cx)
    }

    #[gpui_kit::test]
    fn categories_are_sorted_and_pointer_collapse_keeps_the_current_page(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let sidebar = cx.debug_bounds("showcase-sidebar").unwrap();
        let inset = gpui_kit::px(8.);
        for id in [
            "sidebar-components",
            "sidebar-design",
            "button-page",
            "color-page",
        ] {
            let row = node(cx, id).bounds();
            assert_eq!(row.left(), sidebar.left() + inset, "{id}: {row:?}");
            assert_eq!(
                row.right(),
                sidebar.right() - gpui_kit::px(1.) - inset,
                "{id}: {row:?}"
            );
        }
        for pages in [COMPONENTS, DESIGN] {
            // Labels carry Chinese, so alphabetical order is asserted on the ids.
            assert!(pages.windows(2).all(|pair| pair[0].0 < pair[1].0));
            let ys: Vec<_> = pages
                .iter()
                .map(|&(id, _, _)| node(cx, id).bounds().origin.y)
                .collect();
            assert!(ys.windows(2).all(|pair| pair[0] < pair[1]));
        }
        let start = node(cx, "sidebar-design").bounds().origin.y;
        let position = node(cx, "sidebar-components").bounds().center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        assert_eq!(node(cx, "sidebar-components").expanded(), Some(false));
        settle(cx, 40);
        let middle = node(cx, "sidebar-design").bounds().origin.y;
        settle(cx, 500);
        let end = node(cx, "sidebar-design").bounds().origin.y;
        assert!(
            end < middle && middle < start,
            "the group height should animate"
        );
        assert!(!node(cx, "button-page").visible());
        assert_eq!(
            view.read_with(cx, |view, _| view.page),
            ShowcasePage::Button
        );
        assert_eq!(node(cx, "sidebar-design").expanded(), Some(true));
        let position = node(cx, "sidebar-components").bounds().center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        settle(cx, 500);
        assert!(node(cx, "button-page").visible());
        assert_eq!(node(cx, "sidebar-design").bounds().origin.y, start);
        for (id, page, content) in [
            (
                "segmented-page",
                ShowcasePage::Segmented,
                "segmented-content",
            ),
            (
                "color-picker-page",
                ShowcasePage::ColorPicker,
                "color-picker-content",
            ),
            ("icon-page", ShowcasePage::Icon, "icon-component-content"),
            ("badge-page", ShowcasePage::Badge, "badge-content"),
            ("date-picker-page", ShowcasePage::DatePicker, "date-picker-content"),
            ("time-picker-page", ShowcasePage::TimePicker, "time-picker-content"),
            ("empty-page", ShowcasePage::Empty, "empty-content"),
            ("card-page", ShowcasePage::Card, "card-content"),
            ("pagination-page", ShowcasePage::Pagination, "pagination-content"),
            ("tooltip-page", ShowcasePage::Tooltip, "tooltip-content"),
            ("link-page", ShowcasePage::Link, "link-content"),
            ("list-page", ShowcasePage::List, "list-content"),
            ("tag-page", ShowcasePage::Tag, "tag-content"),
            ("select-page", ShowcasePage::Select, "select-content"),
            ("progress-page", ShowcasePage::Progress, "progress-content"),
            ("spinner-page", ShowcasePage::Spinner, "spinner-content"),
            ("steps-page", ShowcasePage::Steps, "steps-content"),
            ("descriptions-page", ShowcasePage::Descriptions, "descriptions-content"),
            ("table-page", ShowcasePage::Table, "table-content"),
            ("skeleton-page", ShowcasePage::Skeleton, "skeleton-content"),
            ("collapse-page", ShowcasePage::Collapse, "collapse-content"),
            ("drawer-page", ShowcasePage::Drawer, "drawer-content"),
            ("divider-page", ShowcasePage::Divider, "divider-content"),
            (
                "breadcrumb-page",
                ShowcasePage::Breadcrumb,
                "breadcrumb-content",
            ),
            (
                "icon-design-page",
                ShowcasePage::IconDesign,
                "icon-design-content",
            ),
        ] {
            // The catalogue can be taller than the native test window. Reach
            // offscreen entries through the production sidebar scroll route.
            let height = cx.update(|w, _| w.viewport_size().height);
            let target = node(cx, id).bounds().center();
            if target.y < gpui_kit::px(60.) || target.y > height - gpui_kit::px(60.) {
                cx.simulate_event(gpui_kit::ScrollWheelEvent {
                    position: gpui_kit::point(sidebar.center().x, height / 2.),
                    delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(
                        gpui_kit::px(0.),
                        height / 2. - target.y,
                    )),
                    touch_phase: gpui_kit::TouchPhase::Moved,
                    ..Default::default()
                });
                draw(cx);
            }
            let position = node(cx, id).bounds().center();
            cx.simulate_click(position, Modifiers::default());
            draw(cx);
            assert_eq!(view.read_with(cx, |view, _| view.page), page);
            assert!(cx.debug_bounds(content).is_some(), "renders {content}");
        }
    }

    #[gpui_kit::test]
    fn segmented_page_selection_writes_back_to_the_showcase(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let position = node(cx, "segmented-page").bounds().center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        let position = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| {
                    n.label() == Some("周")
                        && n.path()
                            .contains(&gpui_kit::ElementId::from("segmented-demo-0"))
                })
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        assert_eq!(
            view.read_with(cx, |v, _| (
                v.segmented_values[0].clone(),
                v.segmented_changes[0]
            )),
            ("week".into(), 1)
        );
    }

    #[gpui_kit::test]
    fn keyboard_disclosure_and_page_selection_work_with_reduced_motion(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        cx.update(|_, cx| cx.set_reduce_motion(true));
        let start = node(cx, "sidebar-design").bounds().origin.y;
        cx.update(|window, cx| window.focus_next(cx));
        draw(cx);
        assert_eq!(node(cx, "sidebar-components").focused(), Some(true));
        let key = gpui_kit::Keystroke::parse("space").unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: key.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
        draw(cx);
        assert_eq!(node(cx, "sidebar-components").expanded(), Some(false));
        assert!(node(cx, "sidebar-design").bounds().origin.y < start);
        cx.update(|window, cx| window.focus_next(cx));
        draw(cx);
        assert_eq!(
            node(cx, "sidebar-design").focused(),
            Some(true),
            "closed descendants leave the Tab order"
        );
        let point = node(cx, "color-page").bounds().center();
        cx.simulate_click(point, Modifiers::default());
        draw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.page), ShowcasePage::Color);
        assert_eq!(
            view.read_with(cx, |view, _| view.sidebar_open),
            [false, true]
        );
    }
}
