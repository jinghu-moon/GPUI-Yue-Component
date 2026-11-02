use super::super::*;

pub(crate) fn form_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let submit_form = showcase.form.clone();
    let submit = move |_: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut gpui_kit::App| {
        submit_form.update(cx, |form, cx| form.submit(window, cx));
    };
    let reset_form = showcase.form.clone();
    let showcase_entity = cx.entity();
    let reset = move |_: &gpui_kit::ClickEvent, _window: &mut Window, cx: &mut gpui_kit::App| {
        reset_form.update(cx, |form, cx| form.reset(cx));
        showcase_entity.update(cx, |showcase, cx| {
            showcase.last_submit = None;
            cx.notify();
        });
    };
    let outcome = match showcase.last_submit {
        Some(true) => "已提交：所有字段都通过校验",
        Some(false) => "已校验：焦点落在第一个未通过的字段",
        None => "按回车或点击保存，走同一套校验",
    };

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
                .child(div().text_2xl().child("Form"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("校验规则与错误定位由 YueForm 统一负责，字段只声明绑定"),
                ),
        )
        .child(section(
            theme,
            "Bound fields",
            "红色边框与错误说明来自同一条规则；回车提交并聚焦首个未通过项，Tab 在输入框内仍是缩进",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueField::new(theme)
                        .label("项目名称")
                        .required(true)
                        .bind(&showcase.form, "name")
                        .input(
                            YueInput::new("form-name", theme, &showcase.form_name)
                                .max_length(24)
                                .show_counter(true)
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .label("邮箱")
                        .description("用于接收构建通知")
                        .bind(&showcase.form, "email")
                        .input(
                            YueInput::new("form-email", theme, &showcase.form_email)
                                .input_type(InputType::Email)
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .label("说明")
                        .description("没有注册规则，因此永远不会显示错误")
                        .textarea(
                            YueTextarea::new("form-note", theme, &showcase.form_note)
                                .max_length(120)
                                .show_counter(true)
                                .width(gpui_kit::rems(32.0)),
                        ),
                ),
        ))
        .child(section(
            theme,
            "Submit and reset",
            "保存校验全部字段并把焦点交给第一个失败项；清除提示收起已展示的错误",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("form-submit", theme, "保存").on_click(submit))
                .child(
                    YueButton::text("form-reset", theme, "清除提示")
                        .variant(ButtonVariant::Outline)
                        .on_click(reset),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child(outcome),
                ),
        ))
}
