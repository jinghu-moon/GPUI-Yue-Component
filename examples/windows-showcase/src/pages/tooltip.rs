use super::super::*;
use gpui_kit::{Axis, FocusHandle};
use std::time::Duration;
use yue_component::{
    TooltipAlign, TooltipOptions, TooltipPlacement, TooltipTrigger, TooltipVariant,
    YueOverflowTooltip, YueTooltip, YueTooltipGroup,
};

pub(crate) struct TooltipPage {
    manual_open: bool,
    disabled: bool,
    single_line: bool,
    bubble_fits_trigger: bool,
    wide_slot: bool,
    changes: usize,
    focus: FocusHandle,
}

impl TooltipPage {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            manual_open: false,
            disabled: false,
            single_line: false,
            bubble_fits_trigger: false,
            wide_slot: false,
            changes: 0,
            focus: cx.focus_handle(),
        }
    }
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().items_start().gap_4()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_4()
}
fn help(
    id: impl Into<gpui_kit::ElementId>,
    theme: YueTheme,
    label: &'static str,
    text: &'static str,
) -> YueTooltip {
    let id = id.into();
    let button_id = id.clone();
    YueTooltip::new(id, theme, text).trigger_with(move |focus, _, _| {
        YueButton::new(button_id, label)
            .track_focus(focus)
            .accessibility_description(text)
            .into_any_element()
    })
}
impl Render for TooltipPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut variants = row();
        for (name, variant) in [
            ("Default", TooltipVariant::Default),
            ("Primary", TooltipVariant::Primary),
            ("Success", TooltipVariant::Success),
            ("Danger", TooltipVariant::Danger),
            ("Warning", TooltipVariant::Warning),
            ("Light", TooltipVariant::Light),
        ] {
            variants = variants.child(
                help(
                    format!("variant-{name}"),
                    theme,
                    name,
                    "说明当前操作的作用范围",
                )
                .variant(variant),
            );
        }
        let placement_text = match (self.single_line, self.bubble_fits_trigger) {
            (false, false) => {
                "这里是文字提示，用来说明当前按钮\n第二行补充对齐规则\n第三行查看箭头位置"
            }
            (true, false) => "这里是文字提示，用来说明当前按钮",
            (false, true) => "说明\n补充\n详情",
            (true, true) => "说明",
        };
        let placement = |label, side, align| {
            help(format!("placement-{label}"), theme, label, placement_text)
                .placement(side)
                .align(align)
        };
        let update_single_line = cx.listener(|page, checked: &bool, _, cx| {
            page.single_line = *checked;
            cx.notify();
        });
        let update_bubble_width = cx.listener(|page, checked: &bool, _, cx| {
            page.bubble_fits_trigger = *checked;
            cx.notify();
        });
        let placements = column()
            .w_full()
            .child(row()
                .child(row()
                    .child(YueSwitch::new("placement-single-line", theme, self.single_line, "单行内容")
                        .size(SwitchSize::Small)
                        .accessibility_description("开启后 Left/Right 的 Start/End 自动居中；关闭恢复多行。")
                        .on_change(move |checked, _, window, cx| update_single_line(&checked, window, cx)))
                    .child("单行内容"))
                .child(row()
                    .child(YueSwitch::new("placement-narrow-bubble", theme, self.bubble_fits_trigger, "气泡宽度 ≤ 触发器")
                        .size(SwitchSize::Small)
                        .accessibility_description("开启后缩短提示文字，Top/Bottom 的 Start/End 自动居中；关闭恢复长内容。")
                        .on_change(move |checked, _, window, cx| update_bubble_width(&checked, window, cx)))
                    .child("气泡宽度 ≤ 触发器")))
            .child(div().w_full().flex().justify_center().py_6().child(
                column().debug_selector(|| "tooltip-placement-ring".into())
                    .w_full().max_w(gpui_kit::rems(42.)).gap_6()
                    .child(div().w_full().flex().justify_center().gap_4()
                        .child(placement("top-left", TooltipPlacement::Top, TooltipAlign::Start))
                        .child(placement("top", TooltipPlacement::Top, TooltipAlign::Center))
                        .child(placement("top-right", TooltipPlacement::Top, TooltipAlign::End)))
                    .child(div().w_full().flex().justify_between()
                        .child(column().gap_6()
                            .child(placement("left-top", TooltipPlacement::Left, TooltipAlign::Start))
                            .child(placement("left", TooltipPlacement::Left, TooltipAlign::Center))
                            .child(placement("left-bottom", TooltipPlacement::Left, TooltipAlign::End)))
                        .child(column().items_end().gap_6()
                            .child(placement("right-top", TooltipPlacement::Right, TooltipAlign::Start))
                            .child(placement("right", TooltipPlacement::Right, TooltipAlign::Center))
                            .child(placement("right-bottom", TooltipPlacement::Right, TooltipAlign::End))))
                    .child(div().w_full().flex().justify_center().gap_4()
                        .child(placement("bottom-left", TooltipPlacement::Bottom, TooltipAlign::Start))
                        .child(placement("bottom", TooltipPlacement::Bottom, TooltipAlign::Center))
                        .child(placement("bottom-right", TooltipPlacement::Bottom, TooltipAlign::End)))))
            .child(readout(theme, "单行时左右侧偏置方位居中；气泡不宽于触发器时上下侧偏置方位居中。两个开关可叠加，方位标签始终保留请求值。".into()));
        let mut horizontal = YueTooltipGroup::new("horizontal-tools", Axis::Horizontal).gap_2();
        for (id, label, text) in [
            ("left", "左对齐", "将文本左对齐"),
            ("center", "居中", "将文本居中排"),
            ("right", "右对齐", "将文本右对齐"),
        ] {
            horizontal = horizontal.child(YueButton::new(id, label).tooltip(text));
        }
        let vertical = YueTooltipGroup::new("vertical-tools", Axis::Vertical)
            .gap_2()
            .child(YueButton::new("up", "上一项").tooltip("切换到上一项"))
            .child(YueButton::new("next", "下一项").tooltip("切换到下一项"))
            .child(YueButton::new("last", "最后项").tooltip("切换到最后项"));
        let protected = YueTooltipGroup::new("protected-tools", Axis::Horizontal)
            .gap_2()
            .child(YueButton::new("protected-a", "按钮 A").tooltip("左侧提示"))
            .protected_child("plain", YueButton::new("protected-plain", "无提示按钮"))
            .child(YueButton::new("protected-b", "按钮 B").tooltip("右侧提示"));
        let far = YueTooltipGroup::new("far-tools", Axis::Horizontal)
            .gap_12()
            .child(YueButton::new("far-a", "远端 A").tooltip("左侧提示"))
            .child(YueButton::new("far-b", "远端 B").tooltip("右侧提示"));
        let manual = YueTooltip::new("manual-help", theme, "由外部 open 值控制，Escape 请求关闭")
            .track_focus(&self.focus)
            .trigger_mode(TooltipTrigger::Manual)
            .open(self.manual_open)
            .disabled(self.disabled)
            .on_open_change(cx.listener(|page, open: &bool, _, cx| {
                page.manual_open = *open;
                page.changes += 1;
                cx.notify();
            }))
            .trigger(YueButton::new("manual-target", "受控目标").track_focus(&self.focus));
        column().debug_selector(|| "tooltip-content".into()).w_full().p_8().gap_6()
            .child(div().text_2xl().child("Tooltip 文字提示"))
            .child(readout(theme, "Hover 等待 300ms，焦点无需等待；200ms 淡入并向外移动 4px，100ms 淡出。仅相邻且路径安全的同组提示平移 150ms；Escape 关闭。".into()))
            .child(section(theme, "基础与主题", "气泡只放文字；提示移入后可继续阅读，移出后保留 300ms。", column()
                .child(variants)
                .child(row()
                    .child(YueButton::new("simple-help", "Button 便捷入口").tooltip("与独立 Tooltip 共用同一窗口显示槽"))
                    .child(help("no-arrow", theme, "无箭头", "保留安全间距与窗口避让").show_arrow(false))
                    .child(help("auto-hide", theme, "两秒自动关闭", "显示后两秒自动关闭").auto_hide_after(Some(Duration::from_secs(2)))))
                .child(row()
                    .child(help("click-help", theme, "Click", "再次点击或点击外部关闭").trigger_mode(TooltipTrigger::Click))
                    .child(help("context-help", theme, "右键说明", "使用真实右键打开").trigger_mode(TooltipTrigger::ContextMenu))
                    .child(help("focus-help", theme, "Focus", "Tab 获焦立即显示").trigger_mode(TooltipTrigger::Focus)))))
            .child(section(theme, "十二方位", "按上、右、下、左环绕排列，中央留空；靠近窗口边缘会翻转或避让。", placements))
            .child(section(theme, "连续 hover 与按钮避让", "整组使用同一外侧轨道。跨过无提示按钮或间隔过大时重新等待，不飞越按钮。", column()
                .child(horizontal).child(vertical).child(protected).child(far)
                .child(YueTooltipGroup::new("different-size-tools", Axis::Horizontal).gap_2()
                    .child(YueButton::new("size-a", "短说明").tooltip("短说明"))
                    .child(YueButton::new("size-b", "长说明").tooltip("内容尺寸改变时，旧位置淡出、新位置淡入；没有额外 hover 等待。")))))
            .child(section(theme, "受控与禁用", "外部同步不回调；失效只请求关闭一次，新的 false→true 周期可重开。", column()
                .child(row().child(manual)
                    .child(YueButton::new("toggle-open", "切换 open").on_click(cx.listener(|page, _, _, cx| { page.manual_open = !page.manual_open; cx.notify(); })))
                    .child(YueButton::new("toggle-disabled", "切换提示禁用").on_click(cx.listener(|page, _, _, cx| { page.disabled = !page.disabled; cx.notify(); }))))
                .child(readout(theme, format!("open={} · disabled={} · 关闭请求 {} 次", self.manual_open, self.disabled, self.changes)))))
            .child(section(theme, "真实文字溢出", "只有真实排版宽度超过槽宽才启用；扩宽后取消等待并关闭。", column()
                .child(YueButton::new("toggle-slot", "切换文本槽宽度").on_click(cx.listener(|page, _, _, cx| { page.wide_slot = !page.wide_slot; cx.notify(); })))
                .child(YueOverflowTooltip::new("overflow-path", theme, "D:\\Projects\\Yue\\components\\Tooltip.rs")
                    .width(gpui_kit::rems(if self.wide_slot { 32. } else { 12. })))
                .child(YueOverflowTooltip::new("fits-text", theme, "完整显示，无提示").width(gpui_kit::rems(20.)))))
            .child(section(theme, "与 Dropdown 共用基础层", "Tooltip 提供说明，Dropdown 承担命令；各自保留焦点与输入策略。", YueDropdownMenu::new("tooltip-menu", theme,
                YueButton::new("tooltip-menu-trigger", "更多操作").tooltip("打开命令菜单").tooltip_options(TooltipOptions::new().show_arrow(false)))
                .items([YueMenuItem::new("复制"), YueMenuItem::new("导出…")])) )
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{
        Root,
        test_support::{ElementSnapshot, snapshots},
    };
    use gpui_kit::{
        Bounds, Entity, Modifiers, Pixels, Role, TestAppContext, VisualTestContext, px,
    };

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }

    fn node(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label) && n.visible())
                .unwrap_or_else(|| panic!("visible Tooltip showcase control {label}"))
        })
    }

    fn open(cx: &mut TestAppContext) -> (Entity<TooltipPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(TooltipPage::new);
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1200.), px(1200.)));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        (page.unwrap(), cx)
    }

    fn click(cx: &mut VisualTestContext, label: &str) {
        let point = node(cx, label).bounds().center();
        cx.simulate_click(point, Modifiers::default());
        draw(cx);
    }

    fn bubble(cx: &mut VisualTestContext, label: &str) -> Bounds<Pixels> {
        // Leave the old owner and its 300ms grace period before the next hover.
        cx.simulate_mouse_move(gpui_kit::point(px(1.), px(1.)), None, Modifiers::default());
        draw(cx);
        cx.executor().advance_clock(Duration::from_millis(300));
        draw(cx);
        let point = node(cx, label).bounds().center();
        cx.simulate_mouse_move(point, None, Modifiers::default());
        draw(cx);
        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.role() == Some(Role::Tooltip) && n.visible())
                .unwrap_or_else(|| panic!("Tooltip shown for {label}"))
                .bounds()
        })
    }

    #[gpui_kit::test]
    fn twelve_placements_surround_an_empty_center(cx: &mut TestAppContext) {
        let (_, cx) = open(cx);
        let top = node(cx, "top").bounds();
        let bottom = node(cx, "bottom").bounds();
        let left = node(cx, "left").bounds();
        let right = node(cx, "right").bounds();
        for labels in [
            ["top-left", "top", "top-right"],
            ["bottom-left", "bottom", "bottom-right"],
        ] {
            let buttons = labels.map(|label| node(cx, label).bounds());
            assert!(
                buttons
                    .windows(2)
                    .all(|pair| pair[0].right() < pair[1].left())
            );
            assert!(
                buttons
                    .windows(2)
                    .all(|pair| pair[0].top() == pair[1].top())
            );
        }
        for labels in [
            ["left-top", "left", "left-bottom"],
            ["right-top", "right", "right-bottom"],
        ] {
            let buttons = labels.map(|label| node(cx, label).bounds());
            assert!(
                buttons
                    .windows(2)
                    .all(|pair| pair[0].bottom() < pair[1].top())
            );
            assert!(top.bottom() < buttons[0].top());
            assert!(buttons[2].bottom() < bottom.top());
        }
        assert!(left.right() < top.left() && top.right() < right.left());
        assert!(left.right() < bottom.left() && bottom.right() < right.left());
        assert_eq!(top.center().x, bottom.center().x);
        assert_eq!(left.center().y, right.center().y);
    }

    #[gpui_kit::test]
    fn independent_switches_drive_real_size_rules_and_restore_alignment(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let labels = [
            "top-left",
            "top",
            "top-right",
            "left-top",
            "left",
            "left-bottom",
            "right-top",
            "right",
            "right-bottom",
            "bottom-left",
            "bottom",
            "bottom-right",
        ];
        let baseline = labels.map(|label| node(cx, label).bounds());
        for (single_line, fits_trigger, toggle) in [
            (false, false, None),
            (true, false, Some("单行内容")),
            (true, true, Some("气泡宽度 ≤ 触发器")),
            (false, true, Some("单行内容")),
            (false, false, Some("气泡宽度 ≤ 触发器")),
        ] {
            if let Some(toggle) = toggle {
                click(cx, toggle);
            }
            assert_eq!(
                page.read_with(cx, |p, _| (p.single_line, p.bubble_fits_trigger)),
                (single_line, fits_trigger)
            );
            assert_eq!(node(cx, "单行内容").checked(), Some(single_line));
            assert_eq!(node(cx, "气泡宽度 ≤ 触发器").checked(), Some(fits_trigger));
            assert_eq!(
                labels.map(|label| node(cx, label).bounds()),
                baseline,
                "switches keep the ring stable"
            );
            for label in labels {
                let target = node(cx, label).bounds();
                let tip = bubble(cx, label);
                let vertical = label.starts_with("top") || label.starts_with("bottom");
                assert_eq!(
                    tip.size.width <= target.size.width,
                    fits_trigger,
                    "{label}: {tip:?}, {target:?}"
                );
                if vertical {
                    assert!(
                        tip.bottom() < target.top() || target.bottom() < tip.top(),
                        "{label} must not cover its trigger"
                    );
                    let centered = (tip.center().x - target.center().x).abs() < px(1.);
                    assert_eq!(
                        centered,
                        fits_trigger || matches!(label, "top" | "bottom"),
                        "{label}: {tip:?}, {target:?}"
                    );
                } else {
                    assert!(
                        tip.right() < target.left() || target.right() < tip.left(),
                        "{label} must not cover its trigger"
                    );
                    let centered = (tip.center().y - target.center().y).abs() < px(1.);
                    assert_eq!(
                        centered,
                        single_line || matches!(label, "left" | "right"),
                        "{label}: {tip:?}, {target:?}"
                    );
                }
            }
        }
    }
}
