use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, WindowOptions, div,
    prelude::FluentBuilder as _,
};
use yue_component::{
    Accent, ButtonGroup, ButtonShape, ButtonSize, ButtonVariant, CheckboxSize, CheckboxState,
    GroupDirection, Icon, IconPosition, InputSize, InputStatus, InputTextAlign, InputType,
    RadioAppearance, RadioSize, SwitchIndicator, SwitchShape, SwitchSize, ThemeMode, YueButton,
    YueCheckbox, YueCheckboxGroup, YueCheckboxOption, YueField, YueForm, YueInput, YueNumberInput,
    YueRadio, YueRadioGroup, YueRadioOption, YueSwitch, YueTextarea, YueTheme, outline,
};

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Showcase::new(window, cx))
        })
        .expect("failed to open Yue component showcase window");
    });
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ShowcasePage {
    #[default]
    Button,
    Icon,
    Input,
    Form,
    Switch,
    Radio,
    Checkbox,
}

struct Showcase {
    page: ShowcasePage,
    input_basic: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_disabled: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_readonly: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_password: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_affix: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_unit: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_sizes: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_statuses: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_alignments: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_narrow: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_semantic: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_locked: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_counter: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    textarea: gpui_kit::Entity<gpui_kit::base::input::TextareaState>,
    number_input: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    form: gpui_kit::Entity<YueForm>,
    form_name: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    form_email: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    form_note: gpui_kit::Entity<gpui_kit::base::input::TextareaState>,
    last_submit: Option<bool>,
    switch_states: Vec<bool>,
    switch_async: bool,
    switch_async_loading: bool,
    radio_single: Vec<bool>,
    radio_vertical: Option<SharedString>,
    radio_horizontal: Option<SharedString>,
    radio_segment: Option<SharedString>,
    radio_sizes: Vec<Option<SharedString>>,
    radio_segments: Vec<Option<SharedString>>,
    radio_profiles: Vec<Option<SharedString>>,
    radio_empty: Option<SharedString>,
    radio_locked: Option<SharedString>,
    radio_keyboard: Option<SharedString>,
    radio_narrow: Option<SharedString>,
    checkbox_single: Vec<CheckboxState>,
    checkbox_sizes: Vec<CheckboxState>,
    checkbox_tags: Vec<SharedString>,
    checkbox_manual: Vec<SharedString>,
    checkbox_built_in: Vec<SharedString>,
    checkbox_mixed: Vec<SharedString>,
    checkbox_locked: Vec<SharedString>,
    checkbox_narrow: Vec<SharedString>,
    checkbox_profiles: Vec<Vec<SharedString>>,
}

impl Showcase {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form_name = cx.new(|cx| {
            gpui_kit::base::input::InputState::new(window, cx).placeholder("例如 Roadmap")
        });
        let form_email = cx.new(|cx| {
            gpui_kit::base::input::InputState::new(window, cx).placeholder("name@example.com")
        });
        let form_note = cx.new(|cx| {
            gpui_kit::base::input::TextareaState::new(window, cx)
                .auto_grow(2, 5)
                .placeholder("可选：这次改动要解决的问题")
        });
        let form = cx.new(|_| YueForm::new());
        let this = cx.entity();
        form.update(cx, |form, cx| {
            form.require("name", &form_name, "请填写项目名称");
            form.rule("email", &form_email, |value, _| {
                let complete = value
                    .rsplit_once('@')
                    .is_some_and(|(_, domain)| !domain.is_empty() && domain.contains('.'));
                (!complete).then(|| SharedString::from("填写完整的邮箱，例如 name@example.com"))
            });
            form.attach(&form_name, window, cx);
            form.attach(&form_email, window, cx);
            form.on_submit(move |valid, _window, cx| {
                this.update(cx, |showcase, cx| {
                    if showcase.last_submit != Some(valid) {
                        showcase.last_submit = Some(valid);
                        cx.notify();
                    }
                });
            });
        });
        Self {
            page: ShowcasePage::Button,
            input_basic: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).placeholder("请输入内容")
            }),
            input_disabled: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("不可编辑")
            }),
            input_readonly: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("可聚焦与复制")
            }),
            input_password: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("yue-component")
            }),
            input_affix: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).placeholder("搜索项目")
            }),
            input_unit: cx
                .new(|cx| gpui_kit::base::input::InputState::new(window, cx).placeholder("重量")),
            input_sizes: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx)
                            .default_value("Yue Input")
                    })
                })
                .collect(),
            input_statuses: (0..4)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).placeholder("请输入内容")
                    })
                })
                .collect(),
            input_alignments: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx)
                            .default_value("Yue Input")
                    })
                })
                .collect(),
            input_narrow: ["Yue", "yue@example.com", "3.14"]
                .into_iter()
                .map(|value| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).default_value(value)
                    })
                })
                .collect(),
            input_semantic: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).placeholder("请输入内容")
                    })
                })
                .collect(),
            input_locked: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).default_value("锁定内容")
                    })
                })
                .collect(),
            input_counter: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx)
                    .default_value("表单可访问性与状态一致性")
            }),
            textarea: cx.new(|cx| {
                gpui_kit::base::input::TextareaState::new(window, cx)
                    .auto_grow(3, 6)
                    .placeholder("请输入多行内容")
            }),
            number_input: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx)
                    .step(1.0)
                    .min(0.0)
                    .max(100.0)
                    .default_value("12")
            }),
            form,
            form_name,
            form_email,
            form_note,
            last_submit: None,
            switch_states: std::iter::once(false)
                .chain(std::iter::once(true))
                .chain((0..24).map(|index| index % 2 == 1))
                .collect(),
            switch_async: false,
            switch_async_loading: false,
            radio_single: vec![false, true, true, false],
            radio_vertical: Some("png".into()),
            radio_horizontal: Some("light".into()),
            radio_segment: Some("regular".into()),
            radio_sizes: vec![Some("regular".into()); 3],
            radio_segments: vec![
                None,
                Some("regular".into()),
                Some("regular".into()),
                Some("regular".into()),
            ],
            radio_profiles: vec![Some("svg".into()); 4],
            radio_empty: None,
            radio_locked: Some("locked".into()),
            radio_keyboard: Some("weekly".into()),
            radio_narrow: Some("long".into()),
            checkbox_single: vec![
                CheckboxState::Unchecked,
                CheckboxState::Checked,
                CheckboxState::Indeterminate,
                CheckboxState::Checked,
                CheckboxState::Unchecked,
            ],
            checkbox_sizes: vec![
                CheckboxState::Unchecked,
                CheckboxState::Checked,
                CheckboxState::Indeterminate,
            ],
            checkbox_tags: vec!["docs".into()],
            checkbox_manual: vec!["desktop".into()],
            checkbox_built_in: vec!["mail".into(), "tasks".into()],
            checkbox_mixed: vec!["web".into()],
            checkbox_locked: vec!["readme".into()],
            checkbox_narrow: vec!["long".into()],
            checkbox_profiles: vec![vec!["docs".into()]; 4],
        }
    }
}

impl Render for Showcase {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);

        div()
            .size_full()
            .flex()
            .bg(gpui_kit::rgb(theme.page()))
            .text_color(gpui_kit::rgb(theme.text_primary()))
            .child(sidebar(theme, self.page, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .min_h_0()
                    .child(match self.page {
                        ShowcasePage::Button => content(theme).into_any_element(),
                        ShowcasePage::Icon => icon_content(theme).into_any_element(),
                        ShowcasePage::Input => input_content(theme, self).into_any_element(),
                        ShowcasePage::Form => form_content(theme, self, cx).into_any_element(),
                        ShowcasePage::Switch => switch_content(theme, self, cx).into_any_element(),
                        ShowcasePage::Radio => radio_content(theme, self, cx).into_any_element(),
                        ShowcasePage::Checkbox => {
                            checkbox_content(theme, self, cx).into_any_element()
                        }
                    })
                    .overflow_y_scrollbar(),
            )
    }
}

fn sidebar(
    theme: YueTheme,
    active_page: ShowcasePage,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    div()
        .w_56()
        .h_full()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .border_r_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
        .child(
            div()
                .pb_4()
                .text_lg()
                .font_weight(gpui_kit::FontWeight::BOLD)
                .child("Yue Components"),
        )
        .child(sidebar_item(
            "button-page",
            "Button",
            active_page == ShowcasePage::Button,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Button;
                cx.notify();
            }),
        ))
        .child(sidebar_item(
            "input-page",
            "Input",
            active_page == ShowcasePage::Input,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Input;
                cx.notify();
            }),
        ))
        .child(sidebar_item(
            "form-page",
            "Form",
            active_page == ShowcasePage::Form,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Form;
                cx.notify();
            }),
        ))
        .child(sidebar_item(
            "switch-page",
            "Switch",
            active_page == ShowcasePage::Switch,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Switch;
                cx.notify();
            }),
        ))
        .child(sidebar_item(
            "radio-page",
            "Radio",
            active_page == ShowcasePage::Radio,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Radio;
                cx.notify();
            }),
        ))
        .child(sidebar_item(
            "checkbox-page",
            "Checkbox",
            active_page == ShowcasePage::Checkbox,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Checkbox;
                cx.notify();
            }),
        ))
        .child(sidebar_item(
            "icon-page",
            "Icon",
            active_page == ShowcasePage::Icon,
            theme,
            cx.listener(|showcase, _, _, cx| {
                showcase.page = ShowcasePage::Icon;
                cx.notify();
            }),
        ))
        .child(
            div()
                .px_3()
                .py_2()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("Coming soon"),
        )
}

fn input_content(theme: YueTheme, showcase: &Showcase) -> impl IntoElement {
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
                .child(div().text_2xl().child("Input")),
        )
        .child(section(
            theme,
            "Basic",
            "The field names the control for assistive technology",
            YueField::new(theme)
                .label("项目名称")
                .description("名称会展示在工作区列表中")
                .required(true)
                .input(
                    YueInput::new("input-basic", theme, &showcase.input_basic)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Sizes",
            "Small、Medium、Large",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-small", theme, &showcase.input_sizes[0])
                        .size(InputSize::Small)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-medium", theme, &showcase.input_sizes[1])
                        .size(InputSize::Medium)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-large", theme, &showcase.input_sizes[2])
                        .size(InputSize::Large)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Validation",
            "Normal、Success、Warning、Error，边框与说明同源",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Normal, "提示文字不改变边框")
                        .input(
                            YueInput::new("input-normal", theme, &showcase.input_statuses[0])
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Success, "名称可用")
                        .input(
                            YueInput::new("input-success", theme, &showcase.input_statuses[1])
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Warning, "包含多余空格")
                        .input(
                            YueInput::new("input-warning", theme, &showcase.input_statuses[2])
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Error, "名称不能为空")
                        .input(
                            YueInput::new("input-error", theme, &showcase.input_statuses[3])
                                .width(gpui_kit::rems(26.0)),
                        ),
                ),
        ))
        .child(section(
            theme,
            "Disabled and readonly",
            "Disabled cannot receive edits; readonly remains selectable",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-disabled", theme, &showcase.input_disabled)
                        .disabled(true)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-readonly", theme, &showcase.input_readonly)
                        .readonly(true)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Password and clearable",
            "Password visibility and clear actions update Base state",
            YueInput::new("input-password", theme, &showcase.input_password)
                .password(true)
                .clearable(true)
                .width(gpui_kit::rems(26.0)),
        ))
        .child(section(
            theme,
            "Narrow slots",
            "窄宽度下前后缀与清除、显隐按钮同时保留",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-narrow-search", theme, &showcase.input_narrow[0])
                        .leading_icon(outline::Search)
                        .trailing("kg")
                        .clearable(true)
                        .width(gpui_kit::rems(8.0)),
                )
                .child(
                    YueInput::new("input-narrow-password", theme, &showcase.input_narrow[1])
                        .leading_icon(outline::Lock)
                        .password(true)
                        .clearable(true)
                        .width(gpui_kit::rems(11.0)),
                )
                .child(
                    YueInput::new("input-narrow-value", theme, &showcase.input_narrow[2])
                        .trailing("kg")
                        .width(gpui_kit::rems(14.0)),
                ),
        ))
        .child(section(
            theme,
            "Length limit",
            "超出 12 字的粘贴与输入会被截断，计数显示在框内",
            YueInput::new("input-counter", theme, &showcase.input_counter)
                .max_length(12)
                .show_counter(true)
                .width(gpui_kit::rems(26.0)),
        ))
        .child(section(
            theme,
            "Semantic types",
            "Search、Email、URL 决定读屏播报的角色",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueField::new(theme).label("搜索项目").input(
                        YueInput::new("input-type-search", theme, &showcase.input_semantic[0])
                            .input_type(InputType::Search)
                            .leading_icon(outline::Search)
                            .width(gpui_kit::rems(26.0)),
                    ),
                )
                .child(
                    YueField::new(theme).label("邮箱").input(
                        YueInput::new("input-type-email", theme, &showcase.input_semantic[1])
                            .input_type(InputType::Email)
                            .leading_icon(outline::Mail)
                            .width(gpui_kit::rems(26.0)),
                    ),
                )
                .child(
                    YueField::new(theme).label("主页").input(
                        YueInput::new("input-type-url", theme, &showcase.input_semantic[2])
                            .input_type(InputType::Url)
                            .leading_icon(outline::Link)
                            .width(gpui_kit::rems(26.0)),
                    ),
                ),
        ))
        .child(section(
            theme,
            "Locked combinations",
            "禁用与只读叠加前后缀、清除、密码、计数",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-locked-affix", theme, &showcase.input_locked[0])
                        .leading_icon(outline::Tag)
                        .trailing("kg")
                        .clearable(true)
                        .disabled(true)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-locked-password", theme, &showcase.input_locked[1])
                        .password(true)
                        .clearable(true)
                        .readonly(true)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-locked-counter", theme, &showcase.input_locked[2])
                        .readonly(true)
                        .max_length(12)
                        .show_counter(true)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Affixes and icons",
            "Leading and trailing slots accept any GPUI element",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-search", theme, &showcase.input_affix)
                        .leading_icon(outline::Search)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-unit", theme, &showcase.input_unit)
                        .trailing("kg")
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Text alignment",
            "Left、Center、Right",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-align-left", theme, &showcase.input_alignments[0])
                        .text_align(InputTextAlign::Left)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-align-center", theme, &showcase.input_alignments[1])
                        .text_align(InputTextAlign::Center)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-align-right", theme, &showcase.input_alignments[2])
                        .text_align(InputTextAlign::Right)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Textarea",
            "Multi-line editing with content-driven height",
            YueField::new(theme)
                .label("Description")
                .description("The field grows between three and six lines")
                .textarea(
                    YueTextarea::new("textarea-description", theme, &showcase.textarea)
                        .max_length(120)
                        .show_counter(true)
                        .width(gpui_kit::rems(32.0)),
                ),
        ))
        .child(section(
            theme,
            "Number input",
            "Arrow keys and step buttons adjust a bounded value",
            YueField::new(theme)
                .label("Quantity")
                .message(InputStatus::Error, "Enter a value from 0 to 100")
                .number_input(
                    YueNumberInput::new("number-quantity", theme, &showcase.number_input)
                        .width(gpui_kit::rems(16.0)),
                ),
        ))
}

fn form_content(
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

fn sidebar_item(
    id: &'static str,
    label: &'static str,
    active: bool,
    theme: YueTheme,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .rounded_sm()
        .when(active, |item| {
            item.bg(gpui_kit::rgb(theme.accent_subtle()))
                .text_color(gpui_kit::rgb(theme.accent_subtle_text()))
        })
        .when(!active, |item| {
            item.text_color(gpui_kit::rgb(theme.text_secondary()))
        })
        .px_3()
        .py_2()
        .cursor_pointer()
        .on_click(on_click)
        .child(label)
}

/// How long the async switch holds its loading ring before the state commits:
/// one revolution of the ring, whose period is `--spinner-duration` (900ms).
const SWITCH_ASYNC_COMMIT: std::time::Duration = std::time::Duration::from_millis(900);

fn switch_content(
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

fn radio_formats() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("png", "PNG"),
        YueRadioOption::new("svg", "SVG（当前不可选）").disabled(true),
        YueRadioOption::new("pdf", "PDF"),
        YueRadioOption::new("jpg", "JPG"),
    ]
}

fn radio_modes() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("light", "浅色"),
        YueRadioOption::new("dark", "深色"),
        YueRadioOption::new("auto", "跟随系统"),
    ]
}

fn radio_rhythm() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("regular", "常规"),
        YueRadioOption::new("occasional", "偶尔"),
        YueRadioOption::new("never", "从不"),
    ]
}

fn radio_frequencies() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("daily", "每天"),
        YueRadioOption::new("weekly", "每周"),
        YueRadioOption::new("monthly", "每月"),
    ]
}

fn radio_locks() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("locked", "锁定编辑"),
        YueRadioOption::new("unlocked", "允许编辑"),
        YueRadioOption::new("archive", "归档只读"),
    ]
}

fn radio_long_labels() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("long", "这一条标签写得很长，用来检查窄宽度下是否换行"),
        YueRadioOption::new("short", "短标签"),
        YueRadioOption::new("middle", "中等长度标签"),
    ]
}

fn checkbox_tags_options() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("docs", "文档"),
        YueCheckboxOption::new("design", "设计稿"),
        YueCheckboxOption::new("release", "发布说明"),
    ]
}

fn checkbox_channels() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("mail", "邮件"),
        YueCheckboxOption::new("tasks", "任务"),
        YueCheckboxOption::new("calendar", "日历（当前不可选）").disabled(true),
        YueCheckboxOption::new("notes", "备忘"),
    ]
}

fn checkbox_scopes() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("web", "网页端"),
        YueCheckboxOption::new("design", "设计稿"),
        YueCheckboxOption::new("print", "印刷（当前不可选）").disabled(true),
    ]
}

fn checkbox_locks() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("readme", "更新 README"),
        YueCheckboxOption::new("draft", "同时更新草稿"),
    ]
}

fn checkbox_long_labels() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("long", "这一条标签写得很长，用来检查窄宽度下是否换行"),
        YueCheckboxOption::new("short", "短标签"),
        YueCheckboxOption::new("middle", "中等长度标签"),
    ]
}

fn readout(theme: YueTheme, text: String) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(gpui_kit::rgb(theme.text_secondary()))
        .child(text)
}

fn value_text(value: Option<&SharedString>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "（空）".to_string())
}

fn values_text(values: &[SharedString]) -> String {
    if values.is_empty() {
        return "（空）".to_string();
    }
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>()
        .join("、")
}

fn theme_tile(title: &'static str, theme: YueTheme, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
        .text_color(gpui_kit::rgb(theme.text_primary()))
        .child(div().text_sm().child(title))
        .child(body)
}

const PROFILES: [(&str, ThemeMode, Accent); 4] = [
    ("Light · Azure", ThemeMode::Light, Accent::Azure),
    ("Dark · Azure", ThemeMode::Dark, Accent::Azure),
    ("Light · Neutral", ThemeMode::Light, Accent::Neutral),
    ("Dark · Neutral", ThemeMode::Dark, Accent::Neutral),
];

fn radio_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let mut alone: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, label) in ["未选中（Tab 到此查看焦点环）", "已选中", "禁用（已选中）"]
        .into_iter()
        .enumerate()
    {
        let checked = showcase.radio_single[index];
        let update = cx.listener(move |showcase, checked: &bool, _, cx| {
            showcase.radio_single[index] = *checked;
            cx.notify();
        });
        alone.push(
            YueRadio::new(format!("radio-alone-{index}"), theme, checked)
                .label(label)
                .disabled(index == 2)
                .on_change(move |checked, _, window, cx| update(&checked, window, cx))
                .into_any_element(),
        );
    }
    let name_update = cx.listener(|showcase, checked: &bool, _, cx| {
        showcase.radio_single[3] = *checked;
        cx.notify();
    });
    let name_only = div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .child(
            YueRadio::new("radio-alone-name", theme, showcase.radio_single[3])
                .accessibility_label("只有圆点的选项")
                .on_change(move |checked, _, window, cx| name_update(&checked, window, cx)),
        )
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("无可见文字，名称来自 accessibility_label"),
        );

    let vertical_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_vertical = Some(value.clone());
        cx.notify();
    });
    let horizontal_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_horizontal = Some(value.clone());
        cx.notify();
    });
    let segment_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_segment = Some(value.clone());
        cx.notify();
    });
    let column_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_segments[0] = Some(value.clone());
        cx.notify();
    });
    let empty_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_empty = Some(value.clone());
        cx.notify();
    });
    let keyboard_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_keyboard = Some(value.clone());
        cx.notify();
    });
    let narrow_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_narrow = Some(value.clone());
        cx.notify();
    });

    let mut sizes: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, size) in [RadioSize::Small, RadioSize::Medium, RadioSize::Large]
        .into_iter()
        .enumerate()
    {
        let indicator_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_sizes[index] = Some(value.clone());
            cx.notify();
        });
        let segmented_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_segments[index + 1] = Some(value.clone());
            cx.notify();
        });
        sizes.push(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(format!("{size:?}")))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .gap_6()
                        .child(
                            YueRadioGroup::new(
                                format!("radio-size-{index}"),
                                theme,
                                showcase.radio_sizes[index].clone(),
                            )
                            .label("圆点外观")
                            .size(size)
                            .options(radio_rhythm())
                            .on_change(move |value, window, cx| {
                                indicator_update(&value, window, cx)
                            }),
                        )
                        .child(
                            YueRadioGroup::new(
                                format!("radio-size-segment-{index}"),
                                theme,
                                showcase.radio_segments[index + 1].clone(),
                            )
                            .label("分段外观")
                            .size(size)
                            .appearance(RadioAppearance::Button)
                            .options(radio_rhythm())
                            .on_change(move |value, window, cx| {
                                segmented_update(&value, window, cx)
                            }),
                        ),
                )
                .into_any_element(),
        );
    }

    let mut tiles: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, (title, mode, accent)) in PROFILES.into_iter().enumerate() {
        let profile_theme = YueTheme::new(mode, accent);
        let indicator_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_profiles[index] = Some(value.clone());
            cx.notify();
        });
        let segment_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_profiles[index] = Some(value.clone());
            cx.notify();
        });
        tiles.push(
            theme_tile(
                title,
                profile_theme,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        YueRadioGroup::new(
                            format!("radio-profile-{index}"),
                            profile_theme,
                            showcase.radio_profiles[index].clone(),
                        )
                        .label("输出格式")
                        .options(radio_formats())
                        .on_change(move |value, window, cx| indicator_update(&value, window, cx)),
                    )
                    .child(
                        YueRadioGroup::new(
                            format!("radio-profile-segment-{index}"),
                            profile_theme,
                            showcase.radio_profiles[index].clone(),
                        )
                        .label("提醒频率")
                        .appearance(RadioAppearance::Button)
                        .options(radio_rhythm())
                        .on_change(move |value, window, cx| segment_update(&value, window, cx)),
                    ),
            )
            .into_any_element(),
        );
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
                .child(div().text_2xl().child("Radio"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("互斥单选：组持有 value，选项定义 label 与禁用；圆点与分段两种外观共用同一套行为"),
                ),
        )
        .child(section(
            theme,
            "Single options",
            "未选中 / 已选中 / 禁用三态与仅有可访问名称的控件；Tab 聚焦可见焦点环，Space 或 Enter 激活",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_6()
                .children(alone)
                .child(name_only),
        ))
        .child(section(
            theme,
            "Controlled groups",
            "垂直列与水平行；组只持有一个 value，点选或方向键换档后回调写回 Showcase，再传回组件",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-vertical",
                                theme,
                                showcase.radio_vertical.clone(),
                            )
                            .label("输出格式")
                            .options(radio_formats())
                            .on_change(move |value, window, cx| {
                                vertical_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_vertical.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-horizontal",
                                theme,
                                showcase.radio_horizontal.clone(),
                            )
                            .label("界面模式")
                            .options(radio_modes())
                            .direction(GroupDirection::Horizontal)
                            .on_change(move |value, window, cx| {
                                horizontal_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_horizontal.as_ref())
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Button appearance",
            "整组一个连续轮廓：首尾圆角、相邻段共用一条分界线；选中段持续填充，禁用段留在行内",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-segment-row",
                                theme,
                                showcase.radio_segment.clone(),
                            )
                            .label("提醒频率")
                            .appearance(RadioAppearance::Button)
                            .options(radio_rhythm())
                            .on_change(move |value, window, cx| {
                                segment_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_segment.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-segment-column",
                                theme,
                                showcase.radio_segments[0].clone(),
                            )
                            .label("未选中时的分段列")
                            .appearance(RadioAppearance::Button)
                            .direction(GroupDirection::Vertical)
                            .options(radio_frequencies())
                            .on_change(move |value, window, cx| {
                                column_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_segments[0].as_ref())
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Size ladder",
            "Small / Medium / Large 两种外观对照",
            div().flex().flex_col().gap_4().children(sizes),
        ))
        .child(section(
            theme,
            "Keyboard",
            "这里每个组都能直接用键盘操作：Tab / Shift+Tab 进出组，方向键在可用项之间移动并立即选中，Space 或 Enter 激活聚焦项",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_sm()
                        .child("整组只有一个 Tab 停靠点，落在当前选中项；无选中时落在第一个可用项。")
                        .child("垂直组读 ↑ ↓，水平组读 ← →；越过首尾循环，禁用项跳过。")
                        .child("Home / End 跳到首 / 末可用项并选中。")
                        .child("已选项上再按 Space 不会取消——这是单选与多选的差别。"),
                )
                .child(
                    YueRadioGroup::new(
                        "radio-keyboard",
                        theme,
                        showcase.radio_keyboard.clone(),
                    )
                    .label("提醒周期")
                    .options(radio_frequencies())
                    .on_change(move |value, window, cx| keyboard_update(&value, window, cx)),
                ),
        ))
        .child(section(
            theme,
            "Empty, locked and narrow",
            "空 value 的组、整组禁用与窄宽度下的长标签",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new("radio-empty", theme, showcase.radio_empty.clone())
                                .label("尚未选择的组")
                                .options(radio_formats())
                                .on_change(move |value, window, cx| {
                                    empty_update(&value, window, cx)
                                }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_empty.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new("radio-locked", theme, showcase.radio_locked.clone())
                                .label("整组禁用")
                                .disabled(true)
                                .options(radio_locks()),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}（不可操作，回调不会触发）",
                                value_text(showcase.radio_locked.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .w_72()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-narrow",
                                theme,
                                showcase.radio_narrow.clone(),
                            )
                            .label("窄宽度水平组")
                            .options(radio_long_labels())
                            .direction(GroupDirection::Horizontal)
                            .on_change(move |value, window, cx| {
                                narrow_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_narrow.as_ref())
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Theme profiles",
            "四种 Yue profile 下的两种外观对照",
            div().flex().flex_wrap().gap_4().children(tiles),
        ))
}

fn checkbox_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let mut alone: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, label) in [
        "未选中（Tab 到此查看焦点环）",
        "已选中",
        "半选（Indeterminate）",
        "禁用（已选中）",
    ]
    .into_iter()
    .enumerate()
    {
        let state = showcase.checkbox_single[index];
        let update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
            showcase.checkbox_single[index] = *next;
            cx.notify();
        });
        alone.push(
            YueCheckbox::new(format!("checkbox-alone-{index}"), theme, state)
                .label(label)
                .disabled(index == 3)
                .on_change(move |next, _, window, cx| update(&next, window, cx))
                .into_any_element(),
        );
    }
    let name_update = cx.listener(|showcase, next: &CheckboxState, _, cx| {
        showcase.checkbox_single[4] = *next;
        cx.notify();
    });
    let name_only = div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .child(
            YueCheckbox::new("checkbox-alone-name", theme, showcase.checkbox_single[4])
                .accessibility_label("只有方框的选项")
                .on_change(move |next, _, window, cx| name_update(&next, window, cx)),
        )
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("无可见文字，名称来自 accessibility_label"),
        );

    let mut sizes: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, size) in [
        CheckboxSize::Small,
        CheckboxSize::Medium,
        CheckboxSize::Large,
    ]
    .into_iter()
    .enumerate()
    {
        let state = showcase.checkbox_sizes[index];
        let update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
            showcase.checkbox_sizes[index] = *next;
            cx.notify();
        });
        sizes.push(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().text_sm().child(format!("{size:?}")))
                .child(
                    YueCheckbox::new(format!("checkbox-size-{index}"), theme, state)
                        .size(size)
                        .label("包含附件")
                        .on_change(move |next, _, window, cx| update(&next, window, cx)),
                )
                .into_any_element(),
        );
    }

    let tags_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_tags = values.clone();
        cx.notify();
    });
    let tags_reset = cx.listener(|showcase, _: &gpui_kit::ClickEvent, _, cx| {
        showcase.checkbox_tags = vec!["docs".into()];
        cx.notify();
    });
    let built_in_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_built_in = values.clone();
        cx.notify();
    });
    let mixed_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_mixed = values.clone();
        cx.notify();
    });
    let narrow_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_narrow = values.clone();
        cx.notify();
    });

    let manual_options: [(&'static str, &'static str, bool); 3] = [
        ("desktop", "桌面端", false),
        ("web", "网页端", false),
        ("mobile", "移动端（当前不可选）", true),
    ];
    let manual_values: Vec<SharedString> = manual_options
        .iter()
        .filter(|(_, _, disabled)| !*disabled)
        .map(|(value, _, _)| SharedString::from(*value))
        .collect();
    let manual_hits = manual_values
        .iter()
        .filter(|value| showcase.checkbox_manual.contains(value))
        .count();
    let manual_header = if manual_hits == 0 {
        CheckboxState::Unchecked
    } else if manual_hits == manual_values.len() {
        CheckboxState::Checked
    } else {
        CheckboxState::Indeterminate
    };
    let manual_all_update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
        showcase.checkbox_manual = if *next == CheckboxState::Unchecked {
            Vec::new()
        } else {
            manual_values.clone()
        };
        cx.notify();
    });
    let mut manual_rows: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, (value, label, disabled)) in manual_options.into_iter().enumerate() {
        let value = SharedString::from(value);
        let state = if showcase.checkbox_manual.contains(&value) {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        let update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
            if *next == CheckboxState::Unchecked {
                showcase.checkbox_manual.retain(|held| held != &value);
            } else if !showcase.checkbox_manual.contains(&value) {
                showcase.checkbox_manual.push(value.clone());
            }
            cx.notify();
        });
        manual_rows.push(
            YueCheckbox::new(format!("checkbox-manual-{index}"), theme, state)
                .label(label)
                .disabled(disabled)
                .on_change(move |next, _, window, cx| update(&next, window, cx))
                .into_any_element(),
        );
    }

    let mut tiles: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, (title, mode, accent)) in PROFILES.into_iter().enumerate() {
        let profile_theme = YueTheme::new(mode, accent);
        let update = cx.listener(move |showcase, values: &Vec<SharedString>, _, cx| {
            showcase.checkbox_profiles[index] = values.clone();
            cx.notify();
        });
        tiles.push(
            theme_tile(
                title,
                profile_theme,
                YueCheckboxGroup::new(
                    format!("checkbox-profile-{index}"),
                    profile_theme,
                    showcase.checkbox_profiles[index].clone(),
                )
                .label("导出内容")
                .select_all("全选")
                .options(checkbox_tags_options())
                .on_change(move |values, _, window, cx| update(&values, window, cx)),
            )
            .into_any_element(),
        );
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
                .child(div().text_2xl().child("Checkbox"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("多选控件：勾与半选短横由 GPUI 矢量绘制，三态为未选 / 已选 / 半选，组的选中集合由 Showcase 持有"),
                ),
        )
        .child(section(
            theme,
            "Single options",
            "未选中 / 已选中 / 半选 / 禁用；半选首次激活变成已选，而不是清空",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_6()
                .children(alone)
                .child(name_only),
        ))
        .child(section(
            theme,
            "Size ladder",
            "Small / Medium / Large 的方框、勾与 dash 一同缩放",
            div().flex().flex_col().gap_3().children(sizes),
        ))
        .child(section(
            theme,
            "Controlled multi-select",
            "三个选项的组：回调报告新的完整集合，页面同时显示当前 value 并可重置",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_4()
                .child(
                    YueCheckboxGroup::new(
                        "checkbox-tags",
                        theme,
                        showcase.checkbox_tags.clone(),
                    )
                    .label("导出内容")
                    .options(checkbox_tags_options())
                    .on_change(move |values, _, window, cx| tags_update(&values, window, cx)),
                )
                .child(
                    YueButton::text("checkbox-tags-reset", theme, "重置")
                        .variant(ButtonVariant::Outline)
                        .on_click(move |event, window, cx| tags_reset(event, window, cx)),
                )
                .child(readout(
                    theme,
                    format!("当前 value：{}", values_text(&showcase.checkbox_tags)),
                )),
        ))
        .child(section(
            theme,
            "Grouping A: app-drawn select-all",
            "页面自己组合独立 YueCheckbox：全选态由可用子项派生，组结构只有可见标题，容器不带 Group 语义",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                                        .child("导出平台"),
                                )
                                .child(
                                    YueCheckbox::new(
                                        "checkbox-manual-all",
                                        theme,
                                        manual_header,
                                    )
                                    .label("全选")
                                    .on_change(move |next, _, window, cx| {
                                        manual_all_update(&next, window, cx)
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .pl_4()
                                .children(manual_rows),
                        )
                        .child(readout(
                            theme,
                            format!("当前 value：{}", values_text(&showcase.checkbox_manual)),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Grouping B: built-in select-all",
            "YueCheckboxGroup 的全选头由选项定义派生：禁用项不参与三态判断，也不会被全选或清空",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-built-in",
                                theme,
                                showcase.checkbox_built_in.clone(),
                            )
                            .label("通知渠道")
                            .select_all("全选")
                            .options(checkbox_channels())
                            .on_change(move |values, _, window, cx| {
                                built_in_update(&values, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                values_text(&showcase.checkbox_built_in)
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Mixed, locked and empty",
            "全选头的三态来自可用子项的聚合：半选、个别禁用、整组禁用与空选项组",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-mixed",
                                theme,
                                showcase.checkbox_mixed.clone(),
                            )
                            .label("范围（含禁用项）")
                            .select_all("全选")
                            .options(checkbox_scopes())
                            .on_change(move |values, _, window, cx| {
                                mixed_update(&values, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!("当前 value：{}", values_text(&showcase.checkbox_mixed)),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-locked",
                                theme,
                                showcase.checkbox_locked.clone(),
                            )
                            .label("整组禁用")
                            .select_all("全选")
                            .disabled(true)
                            .options(checkbox_locks()),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}（不可操作，回调不会触发）",
                                values_text(&showcase.checkbox_locked)
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-empty",
                                theme,
                                Vec::<SharedString>::new(),
                            )
                            .label("空选项组")
                            .select_all("全选")
                            .options(Vec::<YueCheckboxOption>::new()),
                        )
                        .child(readout(
                            theme,
                            "没有可操作的选项，全选头自动禁用".to_string(),
                        )),
                )
                .child(
                    div()
                        .w_72()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-narrow",
                                theme,
                                showcase.checkbox_narrow.clone(),
                            )
                            .label("窄宽度水平组")
                            .direction(GroupDirection::Horizontal)
                            .options(checkbox_long_labels())
                            .on_change(move |values, _, window, cx| {
                                narrow_update(&values, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!("当前 value：{}", values_text(&showcase.checkbox_narrow)),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Keyboard",
            "上面的每个控件都能直接用键盘操作；这里给出多选与单选在行为上的差别",
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_sm()
                .child("Tab 逐个进入组内每一项与全选头，Space 或 Enter 切换聚焦的控件。")
                .child("与单选组不同：多选组没有漫游焦点，每一项都是独立的 Tab 停靠点，方向键不属于它。")
                .child("已选中的项再按一次会取消；单选的 Space 只会把选中留在当前项。")
                .child("半选首次激活变成已选，而不是清空；全选头在禁用项存在时只聚合可用项。")
                .child("没有只读开关：不可改变的状态由 disabled 表达，校验文案交给 YueField。"),
        ))
        .child(section(
            theme,
            "Theme profiles",
            "四种 Yue profile 下的方框、勾与 dash 对照",
            div().flex().flex_wrap().gap_4().children(tiles),
        ))
}

fn content(theme: YueTheme) -> impl IntoElement {
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
                .child(div().text_2xl().child("Button"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("按钮的三种内容形态、变体与交互状态"),
                ),
        )
        .child(section(
            theme,
            "Content forms",
            "文本、图标、图标加文本",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    YueButton::text("button-text", theme, "Save").variant(ButtonVariant::Primary),
                )
                .child(
                    YueButton::icon("button-icon", theme, outline::Search, "Search")
                        .variant(ButtonVariant::Outline),
                )
                .child(YueButton::icon_text(
                    "button-icon-text",
                    theme,
                    outline::Plus,
                    "Add",
                )),
        ))
        .child(section(
            theme,
            "Variants",
            "Default、Primary、Outline、Ghost、Text、Danger",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("default", theme, "Default"))
                .child(YueButton::text("primary", theme, "Primary").variant(ButtonVariant::Primary))
                .child(YueButton::text("outline", theme, "Outline").variant(ButtonVariant::Outline))
                .child(YueButton::text("ghost", theme, "Ghost").variant(ButtonVariant::Ghost))
                .child(YueButton::text("text-variant", theme, "Text").variant(ButtonVariant::Text))
                .child(YueButton::text("danger", theme, "Delete").variant(ButtonVariant::Danger)),
        ))
        .child(section(
            theme,
            "Sizes and shapes",
            "Small、Medium、Large 与圆角形状",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("small", theme, "Small").size(ButtonSize::Small))
                .child(YueButton::text("medium", theme, "Medium"))
                .child(YueButton::text("large", theme, "Large").size(ButtonSize::Large))
                .child(
                    YueButton::icon("circle", theme, outline::Plus, "Add")
                        .shape(ButtonShape::Circle),
                )
                .child(
                    YueButton::icon("square", theme, outline::Settings, "Settings")
                        .shape(ButtonShape::RoundedSquare),
                )
                .child(YueButton::text("pill", theme, "Pill").shape(ButtonShape::Pill)),
        ))
        .child(section(
            theme,
            "Icon position",
            "图标可以位于文本前或文本后",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::icon_text(
                    "leading",
                    theme,
                    outline::ArrowRight,
                    "Continue",
                ))
                .child(
                    YueButton::icon_text("trailing", theme, outline::ArrowRight, "Continue")
                        .icon_position(IconPosition::Trailing),
                ),
        ))
        .child(section(
            theme,
            "Loading and tooltip",
            "Loading 会阻止重复激活并保留按钮尺寸",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    YueButton::icon_text("loading", theme, outline::DeviceFloppy, "Saving")
                        .loading(true),
                )
                .child(
                    YueButton::text("tooltip", theme, "Hover me")
                        .tooltip("Saves the current document"),
                ),
        ))
        .child(section(
            theme,
            "Block",
            "占满父容器宽度",
            YueButton::text("block", theme, "Full width").block(),
        ))
        .child(section(
            theme,
            "Button group",
            "组合布局不改变成员按钮的交互与可访问性",
            ButtonGroup::new(theme)
                .child(YueButton::text("group-left", theme, "Day"))
                .child(YueButton::text("group-mid", theme, "Week"))
                .child(YueButton::text("group-right", theme, "Month")),
        ))
        .child(section(
            theme,
            "Disabled",
            "禁用态保留可读性，并阻止交互",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("disabled-text", theme, "Disabled").disabled(true))
                .child(
                    YueButton::icon_text("disabled-icon-text", theme, outline::Plus, "Disabled")
                        .disabled(true),
                ),
        ))
        .child(section(
            theme,
            "Theme profiles",
            "Yue Design System profiles",
            div()
                .flex()
                .gap_3()
                .child(profile_card(
                    "Light / Azure",
                    YueTheme::new(ThemeMode::Light, Accent::Azure),
                ))
                .child(profile_card(
                    "Dark / Azure",
                    YueTheme::new(ThemeMode::Dark, Accent::Azure),
                ))
                .child(profile_card(
                    "Light / Neutral",
                    YueTheme::new(ThemeMode::Light, Accent::Neutral),
                ))
                .child(profile_card(
                    "Dark / Neutral",
                    YueTheme::new(ThemeMode::Dark, Accent::Neutral),
                )),
        ))
}

fn icon_content(theme: YueTheme) -> impl IntoElement {
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
                .child(div().text_2xl().child("Icon"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("独立 SVG 图标组件：资源加载、尺寸和颜色由 Icon 负责"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(div().text_lg().child("Tabler icons"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .p_4()
                        .rounded_md()
                        .border_1()
                        .border_color(gpui_kit::rgb(theme.border_default()))
                        .bg(gpui_kit::rgb(theme.surface()))
                        .child(icon_sample(theme, Icon::new(outline::Plus), "Plus"))
                        .child(icon_sample(theme, Icon::new(outline::Search), "Search")),
                ),
        )
}

fn icon_sample(theme: YueTheme, icon: Icon, name: &'static str) -> impl IntoElement {
    div()
        .w_32()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .p_3()
        .rounded_sm()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .child(icon.size(gpui_kit::rems(1.5)).color(theme.accent_solid()))
        .child(SharedString::from(name))
}

fn section(
    theme: YueTheme,
    title: &'static str,
    description: &'static str,
    example: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child(title))
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(description),
        )
        .child(
            div()
                .flex()
                .items_center()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .bg(gpui_kit::rgb(theme.surface()))
                .child(example),
        )
}

fn profile_card(title: &'static str, theme: YueTheme) -> impl IntoElement {
    div()
        .w_40()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
        .text_color(gpui_kit::rgb(theme.text_primary()))
        .child(title)
        .child(
            div()
                .p_2()
                .rounded_sm()
                .bg(gpui_kit::rgb(theme.accent_solid()))
                .text_color(gpui_kit::rgb(theme.on_accent()))
                .child("Accent"),
        )
}
