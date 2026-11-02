use super::super::*;
use yue_component::{
    DividerAlign, DividerLayout, DividerLineStyle, YueDivider, YueSegmented, YueSegmentedOption,
};

pub(crate) struct DividerPage {
    line_style: DividerLineStyle,
    labelled: bool,
    align: DividerAlign,
    layout: DividerLayout,
}
impl DividerPage {
    pub(crate) fn new(_: &mut Context<Self>) -> Self {
        Self {
            line_style: DividerLineStyle::Solid,
            labelled: true,
            align: DividerAlign::Center,
            layout: DividerLayout::Horizontal,
        }
    }
}
fn column() -> gpui_kit::Div {
    div().w_full().flex().flex_col().gap_4()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_4()
}

impl Render for DividerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let vertical = self.layout == DividerLayout::Vertical;
        let update_labelled = cx.listener(|page, checked: &bool, _, cx| {
            page.labelled = *checked;
            cx.notify();
        });
        let settings = column()
            .child(
                row()
                    .child(
                        row().child("线型").child(
                            YueSegmented::new(
                                "divider-line-style",
                                theme,
                                match self.line_style {
                                    DividerLineStyle::Solid => "solid",
                                    DividerLineStyle::Dashed => "dashed",
                                    DividerLineStyle::Dotted => "dotted",
                                },
                            )
                            .label("分隔线线型")
                            .options([
                                YueSegmentedOption::new("solid", "实线"),
                                YueSegmentedOption::new("dashed", "虚线"),
                                YueSegmentedOption::new("dotted", "点线"),
                            ])
                            .on_change(cx.listener(
                                |page, value: &SharedString, _, cx| {
                                    page.line_style = match value.as_str() {
                                        "dashed" => DividerLineStyle::Dashed,
                                        "dotted" => DividerLineStyle::Dotted,
                                        _ => DividerLineStyle::Solid,
                                    };
                                    cx.notify();
                                },
                            )),
                        ),
                    )
                    .child(
                        row().child("显示文字").child(
                            YueSwitch::new("divider-labelled", theme, self.labelled, "显示文字")
                                .size(SwitchSize::Small)
                                .disabled(vertical)
                                .on_change(move |checked, _, w, cx| {
                                    update_labelled(&checked, w, cx)
                                }),
                        ),
                    ),
            )
            .child(
                row().child("方向").child(
                    YueSegmented::new(
                        "divider-layout",
                        theme,
                        if vertical { "vertical" } else { "horizontal" },
                    )
                    .label("分隔线方向")
                    .options([
                        YueSegmentedOption::new("horizontal", "水平"),
                        YueSegmentedOption::new("vertical", "垂直"),
                    ])
                    .on_change(cx.listener(
                        |page, value: &SharedString, _, cx| {
                            page.layout = if value.as_str() == "vertical" {
                                DividerLayout::Vertical
                            } else {
                                DividerLayout::Horizontal
                            };
                            cx.notify();
                        },
                    )),
                ),
            )
            .child(
                row().child("文字位置").child(
                    YueSegmented::new(
                        "divider-align",
                        theme,
                        match self.align {
                            DividerAlign::Start => "start",
                            DividerAlign::Center => "center",
                            DividerAlign::End => "end",
                        },
                    )
                    .label("分隔线文字位置")
                    .disabled(vertical || !self.labelled)
                    .options([
                        YueSegmentedOption::new("start", "左对齐"),
                        YueSegmentedOption::new("center", "居中"),
                        YueSegmentedOption::new("end", "右对齐"),
                    ])
                    .on_change(cx.listener(
                        |page, value: &SharedString, _, cx| {
                            page.align = match value.as_str() {
                                "start" => DividerAlign::Start,
                                "end" => DividerAlign::End,
                                _ => DividerAlign::Center,
                            };
                            cx.notify();
                        },
                    )),
                ),
            );
        let mut preview = YueDivider::new("divider-preview", theme)
            .layout(self.layout)
            .line_style(self.line_style)
            .align(self.align);
        if vertical {
            preview = preview.height(gpui_kit::rems(4.));
        } else if self.labelled {
            preview = preview.label("Yue Design");
        }
        let preview = div()
            .id("divider-preview-area")
            .debug_selector(|| "divider-preview-area".into())
            .w_full()
            .h(gpui_kit::rems(12.))
            .flex()
            .items_center()
            .justify_center()
            .px_6()
            .child(if vertical {
                row()
                    .child("正直")
                    .child(preview)
                    .child("进取")
                    .into_any_element()
            } else {
                column()
                    .child("正直")
                    .child(preview)
                    .child("进取")
                    .into_any_element()
            });
        let mut labels = column();
        for (id, label, align, style) in [
            (
                "start",
                "左对齐 · 实线",
                DividerAlign::Start,
                DividerLineStyle::Solid,
            ),
            (
                "center",
                "居中 · 虚线",
                DividerAlign::Center,
                DividerLineStyle::Dashed,
            ),
            (
                "end",
                "右对齐 · 点线",
                DividerAlign::End,
                DividerLineStyle::Dotted,
            ),
        ] {
            labels = labels
                .child(div().text_sm().child(label))
                .child("将相关内容组织在一起，用分隔线区分不同的信息组。")
                .child(
                    YueDivider::new(format!("divider-label-{id}"), theme)
                        .label("Yue Design")
                        .align(align)
                        .line_style(style),
                );
        }
        let mut themes = column();
        for (id, mode, accent) in [
            ("light-azure", ThemeMode::Light, Accent::Azure),
            ("dark-azure", ThemeMode::Dark, Accent::Azure),
            ("light-neutral", ThemeMode::Light, Accent::Neutral),
            ("dark-neutral", ThemeMode::Dark, Accent::Neutral),
        ] {
            let profile = YueTheme::new(mode, accent);
            themes = themes.child(
                div()
                    .w_full()
                    .p_4()
                    .bg(gpui_kit::rgb(profile.surface()))
                    .child(
                        YueDivider::new(format!("divider-theme-{id}"), profile)
                            .label(format!("{mode:?} / {accent:?}")),
                    ),
            );
        }
        column()
            .debug_selector(|| "divider-content".into())
            .p_8()
            .gap_6()
            .child(div().text_2xl().child("Divider 分隔线"))
            .child(readout(
                theme,
                "用轻量线条组织内容；支持水平、垂直、实线、虚线、点线和带文字的分隔线。".into(),
            ))
            .child(section(
                theme,
                "配置预览",
                "切换方向、线型及文字位置。垂直线不带文字；返回水平时保留之前的文字设置。",
                column().child(settings).child(preview),
            ))
            .child(section(
                theme,
                "基础分隔线",
                "水平线占满容器；垂直线用于同行元素之间，不参与键盘焦点。",
                column()
                    .child("第一组内容")
                    .child(YueDivider::new("divider-basic", theme))
                    .child("第二组内容")
                    .child(YueDivider::new("divider-dashed-basic", theme).dashed(true))
                    .child("第三组内容")
                    .child(
                        YueDivider::new("divider-dotted-basic", theme)
                            .line_style(DividerLineStyle::Dotted),
                    )
                    .child(
                        row()
                            .child("文件")
                            .child(
                                YueDivider::new("divider-inline-a", theme)
                                    .layout(DividerLayout::Vertical),
                            )
                            .child("编辑")
                            .child(
                                YueDivider::new("divider-inline-b", theme)
                                    .layout(DividerLayout::Vertical)
                                    .dashed(true),
                            )
                            .child("视图"),
                    )
                    .child(
                        row()
                            .child("点线示例")
                            .child(
                                YueDivider::new("divider-inline-dotted", theme)
                                    .layout(DividerLayout::Vertical)
                                    .line_style(DividerLineStyle::Dotted),
                            )
                            .child("设置"),
                    ),
            ))
            .child(section(
                theme,
                "带文字的分隔线",
                "文字默认居中，也可放在左右两侧；文字背景保持透明，长文字可换行。",
                labels,
            ))
            .child(section(
                theme,
                "主题",
                "线条、文字随 Yue 主题解析。",
                themes,
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{
        Root,
        test_support::{ElementSnapshot, snapshots},
    };
    use gpui_kit::{Modifiers, Role, TestAppContext, VisualTestContext, px};

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
                .unwrap_or_else(|| panic!("missing Divider configuration {label}"))
        })
    }
    fn click(cx: &mut VisualTestContext, label: &str) {
        let center = node(cx, label).bounds().center();
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
    }
    fn preview(cx: &mut VisualTestContext) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&gpui_kit::ElementId::from("divider-preview")))
                .expect("mounted preview")
        })
    }

    #[gpui_kit::test]
    fn configuration_updates_preview_and_preserves_horizontal_settings(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(DividerPage::new);
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1200.), px(1400.)));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        let page = page.unwrap();
        assert_eq!(preview(cx).role(), Some(Role::Caption));
        click(cx, "虚线");
        assert_eq!(
            page.read_with(cx, |p, _| p.line_style),
            DividerLineStyle::Dashed
        );
        assert_eq!(node(cx, "虚线").checked(), Some(true));
        click(cx, "点线");
        assert_eq!(
            page.read_with(cx, |p, _| p.line_style),
            DividerLineStyle::Dotted
        );
        assert_eq!(node(cx, "点线").checked(), Some(true));
        assert_ne!(node(cx, "虚线").checked(), Some(true));
        for (label, align) in [
            ("左对齐", DividerAlign::Start),
            ("右对齐", DividerAlign::End),
        ] {
            click(cx, label);
            assert_eq!(page.read_with(cx, |p, _| p.align), align);
        }
        click(cx, "显示文字");
        assert!(!page.read_with(cx, |p, _| p.labelled));
        assert_eq!(preview(cx).role(), None);
        assert_eq!(node(cx, "居中").disabled(), Some(true));
        click(cx, "显示文字");
        click(cx, "垂直");
        assert_eq!(page.read_with(cx, |p, _| p.layout), DividerLayout::Vertical);
        assert_eq!(node(cx, "显示文字").disabled(), Some(true));
        assert_eq!(node(cx, "右对齐").disabled(), Some(true));
        assert_eq!(preview(cx).role(), None);
        assert!(preview(cx).bounds().size.height > preview(cx).bounds().size.width);
        assert_eq!(node(cx, "点线").checked(), Some(true));
        click(cx, "实线");
        assert_eq!(
            page.read_with(cx, |p, _| p.line_style),
            DividerLineStyle::Solid
        );
        click(cx, "点线");
        click(cx, "水平");
        assert_eq!(
            page.read_with(cx, |p, _| (p.layout, p.align, p.labelled, p.line_style)),
            (
                DividerLayout::Horizontal,
                DividerAlign::End,
                true,
                DividerLineStyle::Dotted
            )
        );
        assert_eq!(preview(cx).role(), Some(Role::Caption));
        assert_eq!(preview(cx).label(), Some("Yue Design"));
        assert_ne!(node(cx, "显示文字").disabled(), Some(true));
        assert_ne!(node(cx, "右对齐").disabled(), Some(true));
        click(cx, "显示文字");
        assert_eq!(preview(cx).role(), None);
        click(cx, "显示文字");
        click(cx, "居中");
        assert_eq!(page.read_with(cx, |p, _| p.align), DividerAlign::Center);
    }
}
