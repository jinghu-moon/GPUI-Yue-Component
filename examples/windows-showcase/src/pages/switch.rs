use super::super::*;

pub(crate) fn switch_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let update_enabled = cx.listener(|showcase, checked: &bool, _, cx| {
        showcase.switch_states[0] = *checked;
        cx.notify();
    });
    let start_async = cx.listener(|showcase, checked: &bool, _, cx| {
        let target = *checked;
        showcase.switch_async_loading = true;
        cx.spawn(
            async move |this: gpui_kit::WeakEntity<Showcase>, cx: &mut gpui_kit::AsyncApp| {
                cx.background_executor().timer(SWITCH_ASYNC_COMMIT).await;
                let Some(this) = this.upgrade() else {
                    return;
                };
                this.update(cx, |showcase, cx| {
                    showcase.switch_async = target;
                    showcase.switch_async_loading = false;
                    cx.notify();
                });
                cx.refresh();
            },
        )
        .detach();
    });
    let mut combinations: Vec<gpui_kit::AnyElement> = Vec::new();
    let sizes = [SwitchSize::Small, SwitchSize::Medium, SwitchSize::Large];
    let shapes = [SwitchShape::Capsule, SwitchShape::RoundedRectangle];
    let indicators = [
        SwitchIndicator::None,
        SwitchIndicator::Check,
        SwitchIndicator::X,
        SwitchIndicator::CheckX,
    ];
    for (size_index, size) in sizes.into_iter().enumerate() {
        for (shape_index, shape) in shapes.into_iter().enumerate() {
            for (indicator_index, indicator) in indicators.into_iter().enumerate() {
                let index = size_index * 8 + shape_index * 4 + indicator_index;
                let state_index = index + 2;
                let checked = showcase.switch_states[state_index];
                let update = cx.listener(move |showcase, checked: &bool, _, cx| {
                    showcase.switch_states[state_index] = *checked;
                    cx.notify();
                });
                combinations.push(
                    div()
                        .w_40()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(format!("{size:?} · {shape:?} · {indicator:?}"))
                        .child(
                            YueSwitch::new(
                                format!("switch-combination-{index}"),
                                theme,
                                checked,
                                format!("Example switch {index}"),
                            )
                            .size(size)
                            .shape(shape)
                            .indicator(indicator)
                            .on_change(
                                move |checked, _, window, cx| {
                                    update(&checked, window, cx);
                                },
                            ),
                        )
                        .into_any_element(),
                );
            }
        }
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_2xl().child("Switch"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("受控二值开关，支持三种尺寸、两种形状与独立图标选择"),
                ),
        )
        .child(section(
            theme,
            "Interactive states",
            "点击或使用空格切换；禁用状态保持静止",
            div()
                .flex()
                .items_center()
                .gap_6()
                .child(
                    YueSwitch::new(
                        "switch-enabled",
                        theme,
                        showcase.switch_states[0],
                        "Notifications",
                    )
                    .indicator(SwitchIndicator::CheckX)
                    .on_change(move |checked, _, window, cx| {
                        update_enabled(&checked, window, cx);
                    }),
                )
                .child(
                    YueSwitch::new("switch-disabled", theme, showcase.switch_states[1], "Locked")
                        .disabled(true)
                        .indicator(SwitchIndicator::Check),
                ),
        ))
        .child(section(
            theme,
            "Async commit",
            "点击后滑块原地转加载环，约 0.9 秒后才滑到目标位置；期间重复点击无效",
            div()
                .flex()
                .items_center()
                .gap_6()
                .child(
                    YueSwitch::new("switch-async", theme, showcase.switch_async, "Publish draft")
                        .indicator(SwitchIndicator::CheckX)
                        .loading(showcase.switch_async_loading)
                        .on_change(move |checked, _, window, cx| {
                            start_async(&checked, window, cx);
                        }),
                ),
        ))
        .child(section(
            theme,
            "Size, shape and indicator matrix",
            "组合展示 Small / Medium / Large、Capsule / RoundedRectangle、None / Check / X / CheckX",
            div().flex().flex_col().gap_3().children(combinations),
        ))
}
