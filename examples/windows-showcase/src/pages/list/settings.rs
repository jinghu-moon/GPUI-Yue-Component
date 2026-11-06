//! Host-owned settings composed from List slots and existing Yue controls.
use super::meta;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, rems,
};
use yue_component::{
    ButtonVariant, CheckboxState, ListItemLayout, ListSize, SelectEvent, SelectMode, SelectOption,
    SelectValue, SliderDomain, YueButton, YueCheckbox, YueList, YueListItem, YueSegmented,
    YueSegmentedOption, YueSelect, YueSelectState, YueSlider, YueSliderEvent, YueSliderState,
    YueSwitch, YueTag, YueTheme,
};

pub(super) struct SettingsExamples {
    stacked: bool,
    compact: bool,
    autosave: bool,
    notifications: bool,
    sound: bool,
    language: gpui_kit::Entity<YueSelectState>,
    volume: gpui_kit::Entity<YueSliderState>,
    cache_mb: usize,
    feedback: SharedString,
    _subscriptions: Vec<Subscription>,
}
impl SettingsExamples {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = cx.new(|cx| {
            let mut state = YueSelectState::new(SelectMode::Single, window, cx);
            state
                .set_options(
                    [
                        SelectOption::new("system", "跟随系统"),
                        SelectOption::new("zh-CN", "简体中文"),
                        SelectOption::new("en", "English"),
                    ],
                    cx,
                )
                .unwrap();
            state
                .set_value(SelectValue::Single(Some("system".into())), window, cx)
                .unwrap();
            state
        });
        let volume =
            cx.new(|cx| YueSliderState::new(SliderDomain::new(0., 100.).step(5.), 60., window, cx));
        let subscriptions = vec![
            cx.subscribe(&language, |this, state, event, cx| {
                if matches!(event, SelectEvent::Change(_)) {
                    let items = state.read(cx).selected_items();
                    let label = items
                        .first()
                        .map(|item| item.label().as_str())
                        .unwrap_or("跟随系统");
                    this.feedback = format!("示例语言已设为{label}").into();
                    cx.notify();
                }
            }),
            cx.subscribe(&volume, |this, _, event, cx| {
                if let YueSliderEvent::Change(value) = event {
                    this.feedback = format!("示例提示音量：{}%", value.end()).into();
                    cx.notify();
                }
            }),
        ];
        Self {
            stacked: false,
            compact: false,
            autosave: true,
            notifications: true,
            sound: true,
            language,
            volume,
            cache_mb: 128,
            feedback: "修改即时反映在本示例中".into(),
            _subscriptions: subscriptions,
        }
    }
    fn row(
        &self,
        id: &'static str,
        theme: YueTheme,
        title: &'static str,
        description: &'static str,
        control: impl IntoElement,
    ) -> YueListItem {
        let item = YueListItem::new(id).content(meta(theme, title, description));
        // The host chooses the arrangement explicitly. No bounds observer or
        // second scroll owner is needed for a small settings collection.
        if self.stacked {
            item.layout(ListItemLayout::Vertical).actions(control)
        } else {
            item.extra(
                div()
                    .w(rems(14.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(control),
            )
        }
    }
    fn list(&self, id: &'static str, theme: YueTheme, label: &'static str) -> YueList {
        YueList::new(id)
            .theme(theme)
            .bordered(true)
            .accessibility_label(label)
            .size(if self.compact {
                ListSize::Small
            } else {
                ListSize::Medium
            })
    }
    fn set_notifications(&mut self, enabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.notifications = enabled;
        self.sync_sound(window, cx);
        self.feedback = if enabled {
            "已启用示例通知"
        } else {
            "已暂停示例通知，保留声音偏好"
        }
        .into();
        cx.notify();
    }
    fn sync_sound(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.volume.update(cx, |state, cx| {
            state.set_disabled(!self.notifications || !self.sound, window, cx)
        });
    }
    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.compact = false;
        self.autosave = true;
        self.notifications = true;
        self.sound = true;
        self.language.update(cx, |state, cx| {
            state
                .set_value(SelectValue::Single(Some("system".into())), window, cx)
                .unwrap()
        });
        self.volume
            .update(cx, |state, cx| state.set_value(60., window, cx));
        self.sync_sound(window, cx);
        self.feedback = "已恢复本示例的默认偏好".into();
        cx.notify();
    }
}
impl Render for SettingsExamples {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let arrangement = YueSegmented::new(
            "settings-layout",
            theme,
            if self.stacked { "stacked" } else { "inline" },
        )
        .accessibility_label("设置控件位置")
        .options([
            YueSegmentedOption::new("inline", "行尾控件"),
            YueSegmentedOption::new("stacked", "下方控件"),
        ])
        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
            this.stacked = value == "stacked";
            cx.notify();
        }));
        let owner = cx.entity();
        let general = self
            .list("settings-general", theme, "常规偏好")
            .header("常规偏好")
            .child(self.row(
                "settings-autosave-row",
                theme,
                "自动保存",
                "编辑后自动保存示例文档。",
                YueSwitch::new("settings-autosave", theme, self.autosave, "自动保存").on_change(
                    move |next, _, _, cx| {
                        owner.update(cx, |this, cx| {
                            this.autosave = next;
                            this.feedback = if next {
                                "已启用示例自动保存"
                            } else {
                                "已关闭示例自动保存"
                            }
                            .into();
                            cx.notify();
                        })
                    },
                ),
            ))
            .child(
                self.row(
                    "settings-language-row",
                    theme,
                    "显示语言",
                    "选择当前示例的语言偏好。",
                    YueSelect::new("settings-language", &self.language)
                        .theme(theme)
                        .width(rems(12.))
                        .accessibility_label("显示语言"),
                ),
            )
            .child(
                self.row(
                    "settings-density-row",
                    theme,
                    "列表密度",
                    "立即调整这三组设置列表的间距。",
                    YueSegmented::new(
                        "settings-density",
                        theme,
                        if self.compact {
                            "compact"
                        } else {
                            "comfortable"
                        },
                    )
                    .accessibility_label("设置列表密度")
                    .options([
                        YueSegmentedOption::new("comfortable", "舒适"),
                        YueSegmentedOption::new("compact", "紧凑"),
                    ])
                    .on_change(cx.listener(
                        |this, value: &SharedString, _, cx| {
                            this.compact = value == "compact";
                            cx.notify();
                        },
                    )),
                ),
            );
        let owner = cx.entity();
        let sound_owner = cx.entity();
        let volume_value = self.volume.read(cx).value(cx).end();
        let notifications = self
            .list("settings-notifications", theme, "通知与权限")
            .header("通知与权限")
            .child(
                self.row(
                    "settings-notifications-row",
                    theme,
                    "允许通知",
                    "关闭后暂停提示音，原有偏好仍然保留。",
                    YueSwitch::new(
                        "settings-notifications-toggle",
                        theme,
                        self.notifications,
                        "允许通知",
                    )
                    .on_change(move |next, _, w, cx| {
                        owner.update(cx, |this, cx| this.set_notifications(next, w, cx))
                    }),
                ),
            )
            .child(
                self.row(
                    "settings-sound-row",
                    theme,
                    "声音提醒",
                    "独立选择是否播放提示音。",
                    YueCheckbox::new(
                        "settings-sound",
                        theme,
                        if self.sound {
                            CheckboxState::Checked
                        } else {
                            CheckboxState::Unchecked
                        },
                    )
                    .label("播放提示音")
                    .disabled(!self.notifications)
                    .on_change(move |next, _, w, cx| {
                        sound_owner.update(cx, |this, cx| {
                            this.sound = next == CheckboxState::Checked;
                            this.sync_sound(w, cx);
                            cx.notify();
                        })
                    }),
                )
                .disabled(!self.notifications),
            )
            .child(
                self.row(
                    "settings-volume-row",
                    theme,
                    "提示音量",
                    "可拖动滑块，也可用方向键以 5% 调整。",
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            YueSlider::new("settings-volume", theme, &self.volume)
                                .width(rems(10.))
                                .accessibility_label("提示音量")
                                .value_format(|value| format!("{value}%").into()),
                        )
                        .child(
                            div()
                                .w(rems(3.))
                                .text_right()
                                .child(format!("{volume_value}%")),
                        ),
                )
                .disabled(!self.notifications || !self.sound),
            )
            .child(
                self.row(
                    "settings-policy-row",
                    theme,
                    "安全审计",
                    "此项由组织管理，当前账户无权修改。",
                    YueSwitch::new("settings-policy", theme, true, "安全审计（由组织管理）")
                        .disabled(true),
                )
                .disabled(true),
            );
        let maintenance = self
            .list("settings-maintenance", theme, "存储与维护")
            .header("存储与维护")
            .child(
                self.row(
                    "settings-cache-row",
                    theme,
                    "预览缓存",
                    "清理本示例的缓存计数，文档内容不受影响。",
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(YueTag::new(
                            "settings-cache-size",
                            format!("{} MB", self.cache_mb),
                        ))
                        .child(
                            YueButton::new("settings-clear-cache", "清理缓存")
                                .variant(ButtonVariant::Outline)
                                .disabled(self.cache_mb == 0)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cache_mb = 0;
                                    this.feedback = "示例缓存已清理".into();
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(self.row(
                "settings-version-row",
                theme,
                "应用版本",
                "只读信息与设置项共用相同的文字对齐。",
                div().child("0.1.0"),
            ))
            .child(
                self.row(
                    "settings-reset-row",
                    theme,
                    "恢复默认偏好",
                    "恢复语言、密度与通知设置；缓存计数保持当前值。",
                    YueButton::new("settings-reset", "恢复默认")
                        .variant(ButtonVariant::Outline)
                        .on_click(cx.listener(|this, _, w, cx| this.reset(w, cx))),
                ),
            );
        div()
            .id("list-settings-examples")
            .test_support()
            .w_full()
            .min_w_0()
            .child(div().mb_4().child(arrangement))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .when(self.stacked, |el| el.max_w(rems(28.)))
                    .child(div().mb_4().child(general))
                    .child(div().mb_4().child(notifications))
                    .child(maintenance),
            )
            .child(
                div()
                    .id("settings-feedback")
                    .test_support()
                    .mt_3()
                    .text_sm()
                    .text_color(gpui_kit::rgb(theme.text_secondary()))
                    .child(self.feedback.clone()),
            )
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests;
