use super::super::*;
use gpui_kit::base::InteractiveElementExt as _;
use gpui_kit::{
    FocusHandle, KeyDownEvent, KeyUpEvent, Modifiers, ModifiersChangedEvent, Subscription,
};
use gpui_kit::{InteractiveElement as _, StatefulInteractiveElement as _, Styled as _};
use std::collections::BTreeSet;
use yue_component::{
    KbdSize, KbdState, KbdVariant, ShortcutRecorderEvent, ShortcutRecorderValidation,
    ShortcutSeparator, ShortcutSpec, YueKbd, YueList, YueListItem, YueListItemMeta, YueSegmented,
    YueSegmentedOption, YueShortcut, YueShortcutRecorder, YueShortcutRecorderState, YueTooltip,
};

gpui_kit::actions!(shortcut_demo, [SaveDemo, OpenPanelDemo]);

struct RecorderDemo {
    id: &'static str,
    title: &'static str,
    state: gpui_kit::Entity<YueShortcutRecorderState>,
    default: ShortcutSpec,
}

pub(crate) struct ShortcutPage {
    size: KbdSize,
    variant: KbdVariant,
    state: KbdState,
    separator: ShortcutSeparator,
    focus: FocusHandle,
    held: BTreeSet<String>,
    live_spec: ShortcutSpec,
    live_keys: Vec<String>,
    last: SharedString,
    recorders: Vec<RecorderDemo>,
    _subscriptions: Vec<Subscription>,
}
impl ShortcutPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.bind_keys([
            gpui_kit::KeyBinding::new("ctrl-s", SaveDemo, Some("ShortcutDemo")),
            gpui_kit::KeyBinding::new("ctrl-shift-p", OpenPanelDemo, Some("ShortcutDemo")),
        ]);
        let focus = cx.focus_handle();
        let blur = cx.on_blur(&focus, window, |page, _, cx| {
            page.held.clear();
            cx.notify();
        });
        let activation = cx.observe_window_activation(window, |page, window, cx| {
            if !window.is_window_active() {
                page.held.clear();
                cx.notify();
            }
        });
        let owner = cx.weak_entity();
        let live_focus = focus.clone();
        let intercept = cx.intercept_keystrokes(move |event, window, cx| {
            if live_focus.is_focused(window)
                && event.keystroke.key != "tab"
                && event
                    .context_stack
                    .iter()
                    .any(|context| context.contains("ShortcutLive"))
            {
                let _ = owner.update(cx, |page, cx| {
                    page.key_down(
                        &KeyDownEvent {
                            keystroke: event.keystroke.clone(),
                            is_held: false,
                            prefer_character_input: false,
                        },
                        window,
                        cx,
                    )
                });
            }
        });
        let recorders = [
            ("recorder-demo", "演示字段", "Ctrl+Shift+K"),
            ("recorder-save", "保存文件", "Ctrl+S"),
            ("recorder-panel", "命令面板", "Ctrl+Shift+P"),
        ]
        .map(|(id, title, value)| {
            let default = spec(value);
            let state = cx.new(|cx| {
                let mut s = YueShortcutRecorderState::new(window, cx);
                s.set_value(Some(default.clone()), cx).unwrap();
                s
            });
            RecorderDemo {
                id,
                title,
                state,
                default,
            }
        })
        .into_iter()
        .collect::<Vec<_>>();
        let mut subscriptions = vec![blur, activation, intercept];
        for recorder in &recorders {
            let others = recorders
                .iter()
                .filter(|r| r.id != recorder.id)
                .map(|r| (r.title, r.state.downgrade()))
                .collect::<Vec<_>>();
            recorder.state.update(cx, |s, cx| {
                s.set_validator(
                    move |candidate, cx| {
                        others
                            .iter()
                            .find_map(|(title, state)| {
                                let state = state.upgrade()?;
                                (state.read(cx).value() == Some(candidate)).then(|| {
                                    ShortcutRecorderValidation::Conflict(
                                        format!("与「{title}」冲突").into(),
                                    )
                                })
                            })
                            .unwrap_or(ShortcutRecorderValidation::Accept)
                    },
                    cx,
                )
            });
            let title = recorder.title;
            subscriptions.push(cx.subscribe(
                &recorder.state,
                move |page, sender, event: &ShortcutRecorderEvent, cx| {
                    let ShortcutRecorderEvent::Change { value, overwrite } = event;
                    if *overwrite {
                        for other in &page.recorders {
                            if other.state != sender
                                && other.state.read(cx).value() == value.as_ref()
                            {
                                other
                                    .state
                                    .update(cx, |s, cx| s.set_value(None, cx).unwrap());
                            }
                        }
                    }
                    page.last = format!(
                        "{title}已更新：{}",
                        value
                            .as_ref()
                            .map(ShortcutSpec::display_label)
                            .unwrap_or_else(|| "未设置".into())
                    )
                    .into();
                    cx.notify();
                },
            ));
        }
        Self {
            size: KbdSize::Medium,
            variant: KbdVariant::Raised,
            state: KbdState::Default,
            separator: ShortcutSeparator::Plus,
            focus,
            held: BTreeSet::new(),
            live_spec: spec("Ctrl+Shift+K"),
            live_keys: vec!["Ctrl".into(), "Shift".into(), "K".into()],
            last: "尚未执行操作".into(),
            recorders,
            _subscriptions: subscriptions,
        }
    }
    fn sync_modifiers(&mut self, modifiers: Modifiers) {
        for (key, down) in [
            ("Ctrl", modifiers.control),
            ("Alt", modifiers.alt),
            ("Shift", modifiers.shift),
            ("Win", modifiers.platform),
        ] {
            if down {
                self.held.insert(key.into());
            } else {
                self.held.remove(key);
            }
        }
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "tab" {
            return;
        }
        self.sync_modifiers(event.keystroke.modifiers);
        if let Ok(key) = ShortcutSpec::parse(&event.keystroke.key) {
            let key = key.display_label();
            self.held.insert(key.clone());
            let mut keys = modifier_names(event.keystroke.modifiers);
            keys.push(key);
            if let Ok(value) = ShortcutSpec::parse(&keys.join("+")) {
                self.live_spec = value;
                self.live_keys = keys;
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn key_up(&mut self, event: &KeyUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "tab" {
            return;
        }
        self.sync_modifiers(event.keystroke.modifiers);
        if let Ok(key) = ShortcutSpec::parse(&event.keystroke.key) {
            self.held.remove(&key.display_label());
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn live(&self, theme: YueTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let mut shortcut = YueShortcut::new("live-shortcut", self.live_spec.clone())
            .theme(theme)
            .size(KbdSize::Large);
        for key in &self.live_keys {
            shortcut = shortcut
                .key_state(
                    key,
                    if self.held.contains(key) {
                        KbdState::Pressed
                    } else {
                        KbdState::Default
                    },
                )
                .expect("normalized live key");
        }
        let held = if self.held.is_empty() {
            "全部松开".to_owned()
        } else {
            self.held.iter().cloned().collect::<Vec<_>>().join(" + ")
        };
        let focus = self.focus.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .w_full()
            .child(
                div()
                    .id("shortcut-live-input")
                    .key_context("ShortcutLive")
                    .track_focus(&self.focus)
                    .tab_index(0)
                    .role(gpui_kit::Role::Group)
                    .aria_label("快捷键按键演示区，Tab 离开")
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .p_6()
                    .w_full()
                    .border_1()
                    .border_color(gpui_kit::rgb(theme.border_default()))
                    .rounded_md()
                    .focus(|s| s.border_color(gpui_kit::rgb(theme.accent_solid())))
                    .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                        focus.focus(w, cx);
                        w.refresh();
                    })
                    .on_key_up(cx.listener(Self::key_up))
                    .on_modifiers_changed(cx.listener(|page, e: &ModifiersChangedEvent, _, cx| {
                        page.sync_modifiers(e.modifiers);
                        cx.notify();
                    }))
                    .child(shortcut)
                    .child(div().text_sm().child(format!("当前按住：{held}"))),
            )
            .child(readout(
                theme,
                "点击或 Tab 聚焦后按组合键。松开恢复，失焦清空；Tab / Shift+Tab 正常离开。".into(),
            ))
    }
    fn controls(&self, theme: YueTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let mut row = div().flex().flex_wrap().gap_3();
        row = row.child(
            YueSegmented::new(
                "kbd-size",
                theme,
                match self.size {
                    KbdSize::Small => "sm",
                    KbdSize::Medium => "md",
                    KbdSize::Large => "lg",
                },
            )
            .label("尺寸")
            .options(
                [("sm", "小"), ("md", "中"), ("lg", "大")]
                    .map(|(v, l)| YueSegmentedOption::new(v, l)),
            )
            .on_change(cx.listener(|s, v: &SharedString, _, cx| {
                s.size = match v.as_ref() {
                    "sm" => KbdSize::Small,
                    "lg" => KbdSize::Large,
                    _ => KbdSize::Medium,
                };
                cx.notify();
            })),
        );
        row = row.child(
            YueSegmented::new(
                "kbd-variant",
                theme,
                match self.variant {
                    KbdVariant::Raised => "raised",
                    KbdVariant::Flat => "flat",
                    KbdVariant::Ghost => "ghost",
                    KbdVariant::Inverse => "inverse",
                },
            )
            .label("样式")
            .options(
                [
                    ("raised", "立体"),
                    ("flat", "平面"),
                    ("ghost", "行内"),
                    ("inverse", "反色"),
                ]
                .map(|(v, l)| YueSegmentedOption::new(v, l)),
            )
            .on_change(cx.listener(|s, v: &SharedString, _, cx| {
                s.variant = match v.as_ref() {
                    "flat" => KbdVariant::Flat,
                    "ghost" => KbdVariant::Ghost,
                    "inverse" => KbdVariant::Inverse,
                    _ => KbdVariant::Raised,
                };
                cx.notify();
            })),
        );
        row = row.child(
            YueSegmented::new(
                "kbd-state",
                theme,
                match self.state {
                    KbdState::Default => "default",
                    KbdState::Pressed => "pressed",
                    KbdState::Active => "active",
                    KbdState::Disabled => "disabled",
                },
            )
            .label("状态")
            .options(
                [
                    ("default", "默认"),
                    ("pressed", "按下"),
                    ("active", "激活"),
                    ("disabled", "禁用"),
                ]
                .map(|(v, l)| YueSegmentedOption::new(v, l)),
            )
            .on_change(cx.listener(|s, v: &SharedString, _, cx| {
                s.state = match v.as_ref() {
                    "pressed" => KbdState::Pressed,
                    "active" => KbdState::Active,
                    "disabled" => KbdState::Disabled,
                    _ => KbdState::Default,
                };
                cx.notify();
            })),
        );
        row.child(
            YueSegmented::new(
                "kbd-separator",
                theme,
                if self.separator == ShortcutSeparator::Plus {
                    "plus"
                } else {
                    "none"
                },
            )
            .label("组合分隔")
            .options([
                YueSegmentedOption::new("plus", "+"),
                YueSegmentedOption::new("none", "无"),
            ])
            .on_change(cx.listener(|s, v: &SharedString, _, cx| {
                s.separator = if v == "plus" {
                    ShortcutSeparator::Plus
                } else {
                    ShortcutSeparator::None
                };
                cx.notify();
            })),
        )
    }
    fn playground(&self, theme: YueTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let mut samples = div().flex().flex_col().items_start().gap_3().p_4().w_full();
        if self.variant == KbdVariant::Inverse {
            samples = samples
                .bg(gpui_kit::rgb(theme.text_primary()))
                .text_color(gpui_kit::rgb(theme.surface()));
        }
        for (id, value) in [
            ("play-chord", "Ctrl+Shift+P"),
            ("play-sequence", "G I"),
            ("play-alternative", "Ctrl+Y | Ctrl+Shift+Z"),
        ] {
            samples = samples.child(
                YueShortcut::new(id, spec(value))
                    .theme(theme)
                    .size(self.size)
                    .variant(self.variant)
                    .state(self.state)
                    .separator(self.separator),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .w_full()
            .child(self.controls(theme, cx))
            .child(samples)
    }
    fn composition(&self, theme: YueTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.weak_entity();
        let menu_owner = owner.clone();
        let panel_owner = owner.clone();
        let menu = YueDropdownMenu::new(
            "shortcut-real-menu",
            theme,
            YueButton::new("shortcut-menu-trigger", "文件菜单"),
        )
        .items([
            YueMenuItem::new("保存")
                .id("save")
                .shortcut(spec("Ctrl+S"))
                .on_click(move |_, _, cx| {
                    let _ = menu_owner.update(cx, |s, cx| {
                        s.last = "已保存（菜单）".into();
                        cx.notify();
                    });
                }),
            YueMenuItem::new("命令面板")
                .id("panel")
                .shortcut(spec("Ctrl+Shift+P"))
                .on_click(move |_, _, cx| {
                    let _ = panel_owner.update(cx, |s, cx| {
                        s.last = "已打开命令面板（菜单）".into();
                        cx.notify();
                    });
                }),
            YueMenuItem::new("导出 PDF")
                .id("export")
                .shortcut(spec("Ctrl+E"))
                .disabled(true),
        ]);
        let button = YueButton::new("shortcut-primary", "保存")
            .variant(ButtonVariant::Primary)
            .shortcut(spec("Ctrl+S"))
            .on_click(move |_, _, cx| {
                let _ = owner.update(cx, |s, cx| {
                    s.last = "已保存（按钮）".into();
                    cx.notify();
                });
            });
        let tooltip = YueTooltip::new("shortcut-rich-tooltip", theme, "保存当前文件")
            .shortcut(spec("Ctrl+S"))
            .trigger_with(|focus, _, _| {
                YueButton::new("shortcut-tooltip-trigger", "悬停或聚焦查看提示")
                    .track_focus(focus)
                    .into_any_element()
            });
        div()
            .flex()
            .flex_col()
            .gap_4()
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_4()
                    .child(menu)
                    .child(button)
                    .child(tooltip),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child("按")
                    .child(
                        YueShortcut::new("shortcut-inline", spec("Ctrl+Shift+P"))
                            .theme(theme)
                            .variant(KbdVariant::Ghost),
                    )
                    .child("打开命令面板。"),
            )
            .child(readout(
                theme,
                format!(
                    "{}。页面内 Ctrl+S / Ctrl+Shift+P 可触发演示命令。",
                    self.last
                ),
            ))
    }

    fn recorder_settings(&self, theme: YueTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = YueList::new("shortcut-recorder-settings").theme(theme);
        for recorder in &self.recorders {
            let state = recorder.state.clone();
            let default = recorder.default.clone();
            let changed = state.read(cx).value() != Some(&default);
            let title = recorder.title;
            list = list.child(
                YueListItem::new(recorder.id)
                    .content(
                        YueListItemMeta::new(title)
                            .description(format!("默认 {}", default.display_label())),
                    )
                    .extra(
                        div()
                            .flex()
                            .items_start()
                            .gap_2()
                            .child(
                                YueShortcutRecorder::new(recorder.id, &state)
                                    .theme(theme)
                                    .accessibility_label(format!("{title}快捷键")),
                            )
                            .child(
                                YueButton::new(
                                    (gpui_kit::ElementId::from(recorder.id), "reset"),
                                    "恢复默认",
                                )
                                .size(ButtonSize::Small)
                                .disabled(!changed)
                                .on_click(cx.listener(
                                    move |page, _, _, cx| {
                                        // Restoration is a settings command: resolve conflicts in
                                        // the same owner as an explicit recorder overwrite.
                                        for other in &page.recorders {
                                            if other.state != state
                                                && other.state.read(cx).value() == Some(&default)
                                            {
                                                other.state.update(cx, |s, cx| {
                                                    s.set_value(None, cx).unwrap()
                                                });
                                            }
                                        }
                                        state.update(cx, |s, cx| {
                                            s.set_value(Some(default.clone()), cx).unwrap()
                                        });
                                        page.last = format!("{title}已恢复默认").into();
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
            );
        }
        div().flex().flex_col().gap_3().w_full().child(list)
            .child(readout(theme, "试试 Ctrl+S 冲突、Alt+F4 保留键、单独 A；Esc 取消，Backspace 清除。这里修改演示设置，不改页面原有命令绑定。".into()))
            .child(readout(theme, self.last.to_string()))
    }
}
fn spec(value: &str) -> ShortcutSpec {
    ShortcutSpec::parse(value).expect("valid showcase shortcut")
}
fn modifier_names(m: Modifiers) -> Vec<String> {
    [
        ("Ctrl", m.control),
        ("Alt", m.alt),
        ("Shift", m.shift),
        ("Win", m.platform),
    ]
    .into_iter()
    .filter(|(_, v)| *v)
    .map(|(k, _)| k.into())
    .collect()
}

fn key_row(keys: &[(&str, f32)]) -> impl IntoElement {
    let mut row = div().flex().items_center().gap_1().flex_shrink_0();
    for (label, width) in keys {
        let slot = if label.is_empty() {
            div()
                .w(gpui_kit::rems(*width))
                .h(gpui_kit::rems(1.25))
                .flex_shrink_0()
                .into_any_element()
        } else {
            YueKbd::new(*label)
                .size(KbdSize::Small)
                .width(gpui_kit::rems(*width))
                .into_any_element()
        };
        row = row.child(slot);
    }
    row
}

fn keyboard(theme: YueTheme) -> impl IntoElement {
    let main = div()
        .flex()
        .flex_col()
        .gap_1()
        .flex_shrink_0()
        .child(key_row(&[
            ("Esc", 2.),
            ("", 2.),
            ("F1", 2.),
            ("F2", 2.),
            ("F3", 2.),
            ("F4", 2.),
            ("", 2.),
            ("F5", 2.),
            ("F6", 2.),
            ("F7", 2.),
            ("F8", 2.),
            ("", 2.),
            ("F9", 2.),
            ("F10", 2.),
            ("F11", 2.),
            ("F12", 2.),
            ("", 2.),
            ("PrtSc", 2.),
            ("ScrLk", 2.),
            ("Pause", 2.75),
        ]))
        .child(key_row(&[
            ("`", 2.),
            ("1", 2.),
            ("2", 2.),
            ("3", 2.),
            ("4", 2.),
            ("5", 2.),
            ("6", 2.),
            ("7", 2.),
            ("8", 2.),
            ("9", 2.),
            ("0", 2.),
            ("-", 2.),
            ("=", 2.),
            ("Backspace", 4.),
        ]))
        .child(key_row(&[
            ("Tab", 3.),
            ("Q", 2.),
            ("W", 2.),
            ("E", 2.),
            ("R", 2.),
            ("T", 2.),
            ("Y", 2.),
            ("U", 2.),
            ("I", 2.),
            ("O", 2.),
            ("P", 2.),
            ("[", 2.),
            ("]", 2.),
            ("\\", 3.),
        ]))
        .child(key_row(&[
            ("Caps", 3.5),
            ("A", 2.),
            ("S", 2.),
            ("D", 2.),
            ("F", 2.),
            ("G", 2.),
            ("H", 2.),
            ("J", 2.),
            ("K", 2.),
            ("L", 2.),
            (";", 2.),
            ("'", 2.),
            ("Enter", 4.),
        ]))
        .child(key_row(&[
            ("Shift", 4.5),
            ("Z", 2.),
            ("X", 2.),
            ("C", 2.),
            ("V", 2.),
            ("B", 2.),
            ("N", 2.),
            ("M", 2.),
            (",", 2.),
            (".", 2.),
            ("/", 2.),
            ("Shift", 4.5),
        ]))
        .child(key_row(&[
            ("Ctrl", 2.5),
            ("Win", 2.5),
            ("Alt", 2.5),
            ("Space", 12.),
            ("Alt", 2.5),
            ("Win", 2.5),
            ("Menu", 2.5),
            ("Ctrl", 2.5),
        ]));

    let navigation = div()
        .flex()
        .flex_col()
        .gap_1()
        .flex_shrink_0()
        .child(key_row(&[("Ins", 2.), ("Home", 2.), ("PgUp", 2.)]))
        .child(key_row(&[("Del", 2.), ("End", 2.), ("PgDn", 2.)]))
        .child(div().h(gpui_kit::rems(2.)).flex_shrink_0())
        .child(key_row(&[("", 2.), ("↑", 2.), ("", 2.)]))
        .child(key_row(&[("←", 2.), ("↓", 2.), ("→", 2.)]));

    let keypad = div()
        .flex()
        .flex_col()
        .gap_1()
        .flex_shrink_0()
        .child(key_row(&[("Num", 2.), ("/", 2.), ("*", 2.), ("-", 2.)]))
        .child(key_row(&[("7", 2.), ("8", 2.), ("9", 2.), ("+", 2.)]))
        .child(key_row(&[("4", 2.), ("5", 2.), ("6", 2.), ("", 2.)]))
        .child(key_row(&[("1", 2.), ("2", 2.), ("3", 2.), ("Enter", 2.)]))
        .child(key_row(&[("0", 4.), (".", 2.), ("", 2.)]));

    div()
        .flex()
        .items_start()
        .gap_4()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.page()))
        .child(main)
        .child(navigation)
        .child(keypad)
}

fn example_row(
    theme: YueTheme,
    title: &'static str,
    shortcut: &'static str,
    variant: KbdVariant,
) -> YueListItem {
    YueListItem::new(title)
        .content(YueListItemMeta::new(title))
        .extra(
            YueShortcut::parse((gpui_kit::ElementId::from(title), "shortcut"), shortcut)
                .expect("valid showcase shortcut")
                .theme(theme)
                .size(KbdSize::Small)
                .variant(variant),
        )
}

impl Render for ShortcutPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let spec = ShortcutSpec::parse("ctrl+shift+p").expect("valid shortcut sample");
        div()
        .debug_selector(|| "shortcut-content".into())
        .id("shortcut-content-scroll")
        .key_context("ShortcutDemo")
        .on_action(cx.listener(|s, _: &SaveDemo, _, cx| {s.last="已保存（Ctrl+S）".into();cx.notify();}))
        .on_action(cx.listener(|s, _: &OpenPanelDemo, _, cx| {s.last="已打开命令面板（Ctrl+Shift+P）".into();cx.notify();}))
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .overflow_y_scroll()
        .child(div().text_2xl().child("快捷键 Shortcut"))
        .child(readout(theme, "YueKbd 呈现单键，YueShortcut 组合按键、先后序列与备选方案；实际绑定仍由 GPUI Action owner 管理。".into()))
        .child(section(
            theme,
            "组件结构",
            "键帽 → 同时按下的组合 → 先后按下的序列 → 任选其一的备选。",
            div().flex().items_center().justify_center().w_full().py_8()
                .bg(gpui_kit::rgb(theme.page()))
                .child(YueShortcut::parse("shortcut-anatomy", "ctrl+k c | ctrl+/")
                    .expect("valid reference shortcut").theme(theme).size(KbdSize::Large)),
        ))
        .child(section(theme,"交互式规格对照","同一份组合、序列与备选，切换尺寸、样式、状态和分隔符。",self.playground(theme,cx)))
        .child(section(theme,"局部实时按键反馈","按键状态由此演示页面管理，展示组件不监听输入。",self.live(theme,cx)))
        .child(section(theme,"真实组件组合","复用 Dropdown、Button、Tooltip；禁用菜单项及其快捷键同步变灰。",self.composition(theme,cx)))
        .child(section(theme,"快捷键录制与设置","点击字段或按 Enter 录制；按下主键即确认，冲突时显式覆盖或取消。",self.recorder_settings(theme,cx)))
        .child(section(
            theme,
            "完整键盘布局",
            "ANSI Windows 键盘示意，展示功能键、主键区、导航键、方向键与数字小键盘。",
            div()
                .id("shortcut-keyboard-viewport")
                .w_full()
                .min_w_0()
                .overflow_x_scroll()
                .lock_scroll_axis()
                .child(keyboard(theme)),
        ))
        .child(section(
            theme,
            "尺寸、变体与状态",
            "三档键帽尺寸；Raised / Flat / Ghost / Inverse 表面；状态由 owner 显式传入。",
            div().flex().flex_col().items_start().gap_3()
                .child(div().flex().flex_wrap().items_center().gap_3()
                    .child(YueKbd::new("Small").theme(theme).size(KbdSize::Small))
                    .child(YueKbd::new("Medium").theme(theme))
                    .child(YueKbd::new("Large").theme(theme).size(KbdSize::Large)))
                .child(div().flex().flex_wrap().items_center().gap_3()
                    .child(YueKbd::new("Raised").theme(theme).variant(KbdVariant::Raised))
                    .child(YueKbd::new("Flat").theme(theme).variant(KbdVariant::Flat))
                    .child(YueKbd::new("Ghost").theme(theme).variant(KbdVariant::Ghost))
                    .child(div().px_3().py_2().rounded_md()
                        .bg(gpui_kit::rgb(theme.text_primary()))
                        .text_color(gpui_kit::rgb(theme.surface()))
                        .child(YueKbd::new("Inverse").theme(theme).variant(KbdVariant::Inverse))))
                .child(div().flex().flex_wrap().items_center().gap_3()
                    .child(YueKbd::new("Pressed").theme(theme).state(KbdState::Pressed))
                    .child(YueKbd::new("Active").theme(theme).state(KbdState::Active))
                    .child(YueKbd::new("Disabled").theme(theme).state(KbdState::Disabled))),
        ))
        .child(section(
            theme,
            "组合、序列与备选",
            "修饰键自动排序；空格表达先后，竖线表达同一动作的备选绑定。",
            div().flex().flex_col().items_start().gap_3()
                .child(YueShortcut::new("shortcut-chord", spec).theme(theme))
                .child(YueShortcut::parse("shortcut-windows", "win+e").expect("valid Windows shortcut").theme(theme))
                .child(YueShortcut::parse("shortcut-sequence", "g i").expect("valid sequence").theme(theme))
                .child(YueShortcut::parse("shortcut-alternatives", "ctrl+y | ctrl+shift+z").expect("valid alternatives").theme(theme))
                .child(readout(theme, "ARIA 名称由结构化快捷键生成；此组件不监听键盘、不注册 Action。".into())),
        ))
        .child(section(
            theme,
            "设置与菜单行",
            "菜单展示使用紧凑键帽；设置项的当前值与修改行为由宿主控制。",
            YueList::new("shortcut-settings").theme(theme)
                .child(example_row(theme, "保存文件", "ctrl+s", KbdVariant::Ghost))
                .child(example_row(theme, "命令面板", "ctrl+shift+p", KbdVariant::Ghost))
                .child(example_row(theme, "撤销", "ctrl+z", KbdVariant::Flat))
                .child(example_row(theme, "重做", "ctrl+y | ctrl+shift+z", KbdVariant::Ghost)),
        ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::{Keystroke, TestAppContext, VisualTestContext};
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    #[gpui_kit::test]
    fn recorder_settings_replace_conflicts_and_restore_defaults(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let page = cx.new(|cx| ShortcutPage::new(w, cx));
            saved = Some(page.clone());
            gpui_kit::base::Root::new(page, w, cx)
        });
        let page = saved.unwrap();
        cx.update(|w, _| w.activate_window());
        cx.simulate_resize(gpui_kit::size(gpui_kit::px(1500.), gpui_kit::px(3600.)));
        draw(cx);
        let click = |cx: &mut VisualTestContext, label: &str| {
            let center = cx.update(|w, _| {
                gpui_kit::base::test_support::snapshots(w)
                    .into_iter()
                    .find(|n| n.label().is_some_and(|s| s.starts_with(label)))
                    .unwrap_or_else(|| panic!("missing {label}"))
                    .bounds()
                    .center()
            });
            cx.simulate_click(center, Modifiers::default());
            draw(cx);
        };
        click(cx, "演示字段快捷键，");
        cx.simulate_keystrokes("ctrl-s");
        draw(cx);
        let recorders = page.read_with(cx, |s, _| {
            s.recorders
                .iter()
                .map(|r| r.state.clone())
                .collect::<Vec<_>>()
        });
        assert!(recorders[0].read_with(cx, |s, _| s.pending_value().is_some()));
        assert_eq!(
            page.read_with(cx, |s, _| s.last.as_ref().to_owned()),
            "尚未执行操作"
        );
        click(cx, "覆盖");
        assert_eq!(
            recorders[0].read_with(cx, |s, _| s.value().cloned()),
            Some(spec("Ctrl+S"))
        );
        assert!(recorders[1].read_with(cx, |s, _| s.value().is_none()));
        // Select the save row's reset using its stable identity, since all rows
        // intentionally use the same visible 恢复默认 command label.
        let reset = cx.update(|w, _| {
            gpui_kit::base::test_support::snapshots(w)
                .into_iter()
                .find(|n| {
                    n.label() == Some("恢复默认")
                        && n.path().contains(&gpui_kit::ElementId::from((
                            gpui_kit::ElementId::from("recorder-save"),
                            "reset",
                        )))
                })
                .expect("save reset")
                .bounds()
                .center()
        });
        cx.simulate_click(reset, Modifiers::default());
        draw(cx);
        assert_eq!(
            recorders[1].read_with(cx, |s, _| s.value().cloned()),
            Some(spec("Ctrl+S"))
        );
        assert!(recorders[0].read_with(cx, |s, _| s.value().is_none()));
        assert!(page.read_with(cx, |s, _| s.last.as_ref().contains("已恢复默认")));
    }
    #[gpui_kit::test]
    fn playground_controls_update_the_same_shortcut_samples(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let page = cx.new(|cx| ShortcutPage::new(w, cx));
            saved = Some(page.clone());
            gpui_kit::base::Root::new(page, w, cx)
        });
        let page = saved.unwrap();
        cx.simulate_resize(gpui_kit::size(gpui_kit::px(1500.), gpui_kit::px(1800.)));
        draw(cx);
        for label in ["大", "反色", "按下", "无"] {
            let center = cx.update(|w, _| {
                gpui_kit::base::test_support::snapshots(w)
                    .into_iter()
                    .find(|n| {
                        n.label() == Some(label) && n.role() == Some(gpui_kit::Role::RadioButton)
                    })
                    .unwrap_or_else(|| panic!("missing control: {label}"))
                    .bounds()
                    .center()
            });
            cx.simulate_click(center, Modifiers::default());
            draw(cx);
        }
        page.read_with(cx, |s, _| {
            assert_eq!(s.size, KbdSize::Large);
            assert_eq!(s.variant, KbdVariant::Inverse);
            assert_eq!(s.state, KbdState::Pressed);
            assert_eq!(s.separator, ShortcutSeparator::None);
        });
        cx.update(|w, _| {
            let labels: Vec<_> = gpui_kit::base::test_support::snapshots(w)
                .into_iter()
                .filter_map(|n| n.label().map(str::to_owned))
                .collect();
            for sample in [
                "Ctrl + Shift + P",
                "G 然后 I",
                "Ctrl + Y 或 Ctrl + Shift + Z",
            ] {
                assert!(
                    labels.iter().any(|label| label == sample),
                    "sample semantics preserved: {sample}"
                );
            }
        });
    }
    #[gpui_kit::test]
    fn live_input_captures_commands_releases_keys_and_allows_tab(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let page = cx.new(|cx| ShortcutPage::new(w, cx));
            saved = Some(page.clone());
            gpui_kit::base::Root::new(page, w, cx)
        });
        let page = saved.unwrap();
        cx.update(|w, _| w.activate_window());
        cx.simulate_resize(gpui_kit::size(gpui_kit::px(1500.), gpui_kit::px(1800.)));
        draw(cx);
        let focus = page.read_with(cx, |s, _| s.focus.clone());
        cx.update(|w, cx| focus.focus(w, cx));
        draw(cx);
        let stroke = Keystroke::parse("ctrl-s").unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: stroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        draw(cx);
        page.read_with(cx, |s, _| {
            assert!(
                s.held.contains("Ctrl") && s.held.contains("S"),
                "held={:?}, last={}",
                s.held,
                s.last
            );
            assert_eq!(
                s.last.as_ref(),
                "尚未执行操作",
                "local input must suppress the page's Ctrl+S command"
            );
        });
        cx.simulate_event(KeyUpEvent { keystroke: stroke });
        draw(cx);
        assert_eq!(
            page.read_with(cx, |s, _| s.held.iter().cloned().collect::<Vec<_>>()),
            vec!["Ctrl"]
        );
        cx.simulate_event(ModifiersChangedEvent::default());
        draw(cx);
        assert!(page.read_with(cx, |s, _| s.held.is_empty()));
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert!(
            !cx.update(|w, _| focus.is_focused(w)),
            "Tab must leave the live region"
        );
        cx.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse("ctrl-s").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        draw(cx);
        assert_eq!(
            page.read_with(cx, |s, _| s.last.clone()).as_ref(),
            "已保存（Ctrl+S）"
        );
        cx.update(|w, cx| focus.focus(w, cx));
        draw(cx);
        cx.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse("ctrl-k").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        draw(cx);
        cx.update(|w, cx| w.blur(cx));
        draw(cx);
        assert!(
            page.read_with(cx, |s, _| s.held.is_empty()),
            "blur must not leave stuck pressed keys"
        );
        cx.update(|w, cx| focus.focus(w, cx));
        draw(cx);
        cx.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse("alt-k").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        draw(cx);
        cx.deactivate_window();
        draw(cx);
        assert!(
            page.read_with(cx, |s, _| s.held.is_empty()),
            "deactivating the window must clear pressed keys"
        );
    }
}
