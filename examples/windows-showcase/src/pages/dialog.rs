use super::super::*;
use gpui_kit::{Entity, Focusable as _, Subscription, Task};
use std::time::Duration;
use yue_component::{
    DialogDecision, DialogFooter, DialogOutcome, DialogSize, DialogStatus, YueDialog,
    YueDialogEvent, YueDialogState, YueSegmented, YueSegmentedOption, YueTooltip,
};

pub(crate) struct DialogPage {
    dialog: Entity<YueDialogState>,
    child: Entity<YueDialogState>,
    name: Entity<gpui_kit::base::input::InputState>,
    size: DialogSize,
    draggable: bool,
    overlay: bool,
    escape: bool,
    overlay_click: bool,
    closable: bool,
    long: bool,
    custom: bool,
    guard: bool,
    asynchronous: bool,
    fail: bool,
    error: Option<SharedString>,
    last: SharedString,
    task: Option<Task<()>>,
    _events: Subscription,
}
impl DialogPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let dialog = cx.new(YueDialogState::new);
        let events = cx.subscribe(&dialog, |s: &mut Self, _, e, cx| {
            s.last = match e {
                YueDialogEvent::OpenChanged(change) => format!(
                    "OpenChanged · open={} · {:?}",
                    change.is_open(),
                    change.reason()
                ),
                YueDialogEvent::Opened => "Opened · 进入完成".into(),
                YueDialogEvent::Closed(change) => {
                    format!("Closed · {:?} · 焦点已恢复", change.reason())
                }
            }
            .into();
            cx.notify();
        });
        Self {
            dialog,
            child: cx.new(YueDialogState::new),
            name: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(w, cx).default_value("Yue Component")
            }),
            size: DialogSize::Medium,
            draggable: false,
            overlay: true,
            escape: true,
            overlay_click: true,
            closable: true,
            long: false,
            custom: false,
            guard: false,
            asynchronous: false,
            fail: false,
            error: None,
            last: "尚未打开对话框".into(),
            task: None,
            _events: events,
        }
    }
}
fn column() -> gpui_kit::Div {
    div().w_full().flex().flex_col().items_start().gap_4()
}
fn row() -> gpui_kit::Div {
    div().flex().min_w_0().items_center().flex_wrap().gap_4()
}
impl Render for DialogPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut settings = column().child(
            row().child("首选宽度").child(
                YueSegmented::new(
                    "dialog-size",
                    theme,
                    match self.size {
                        DialogSize::Small => "sm",
                        DialogSize::Medium => "md",
                        DialogSize::Large => "lg",
                        DialogSize::ExtraLarge => "xl",
                    },
                )
                .label("对话框宽度")
                .options([
                    YueSegmentedOption::new("sm", "192"),
                    YueSegmentedOption::new("md", "320"),
                    YueSegmentedOption::new("lg", "480"),
                    YueSegmentedOption::new("xl", "640"),
                ])
                .on_change(cx.listener(|s, value: &SharedString, _, cx| {
                    s.size = match value.as_str() {
                        "sm" => DialogSize::Small,
                        "lg" => DialogSize::Large,
                        "xl" => DialogSize::ExtraLarge,
                        _ => DialogSize::Medium,
                    };
                    cx.notify();
                })),
            ),
        );
        let mut switches = row().w_full();
        for (id, label, value) in [
            ("draggable", "拖动标题", self.draggable),
            ("overlay", "显示遮罩", self.overlay),
            ("escape", "Escape 关闭", self.escape),
            ("overlay-click", "点击遮罩关闭", self.overlay_click),
            ("closable", "关闭按钮", self.closable),
            ("long", "长内容", self.long),
            ("custom", "自定义底部", self.custom),
            ("guard", "拦截关闭", self.guard),
            ("async", "异步保存", self.asynchronous),
            ("fail", "模拟保存失败", self.fail),
        ] {
            let listener = cx.listener(move |s, value: &bool, _, cx| {
                match id {
                    "draggable" => s.draggable = *value,
                    "overlay" => s.overlay = *value,
                    "escape" => s.escape = *value,
                    "overlay-click" => s.overlay_click = *value,
                    "closable" => s.closable = *value,
                    "long" => s.long = *value,
                    "custom" => s.custom = *value,
                    "guard" => s.guard = *value,
                    "async" => s.asynchronous = *value,
                    "fail" => s.fail = *value,
                    _ => {}
                }
                cx.notify();
            });
            switches = switches.child(
                row().child(label).child(
                    YueSwitch::new(format!("dialog-setting-{id}"), theme, value, label)
                        .size(SwitchSize::Small)
                        .on_change(move |value, _, w, cx| listener(&value, w, cx)),
                ),
            );
        }
        settings = settings.child(switches);
        let open = cx.listener(|s: &mut Self, _, w, cx| {
            s.error = None;
            if let Err(error) = s.dialog.update(cx, |s, cx| s.open(w, cx)) {
                s.last = error.to_string().into();
            }
            cx.notify();
        });
        let owner = cx.weak_entity();
        let veto = cx.weak_entity();
        let initial_focus = self.name.read(cx).focus_handle(cx);
        let mut dialog = YueDialog::new("project-dialog", theme, &self.dialog)
            .title("编辑项目")
            .description("名称会显示在工作区列表中。输入状态由宿主保留。")
            .size(self.size)
            .draggable(self.draggable)
            .overlay(self.overlay)
            .close_on_escape(self.escape)
            .close_on_overlay_click(self.overlay_click)
            .closable(self.closable)
            .confirm_label("保存")
            .cancel_label("取消")
            .close_label("关闭对话框")
            .initial_focus(&initial_focus)
            .child(
                YueField::new(theme).label("项目名称").required(true).input(
                    YueInput::new("project-name", theme, &self.name)
                        .disabled(self.dialog.read(cx).is_pending())
                        .width(gpui_kit::relative(1.)),
                ),
            )
            .on_confirm(move |request, w, cx| {
                owner
                    .update(cx, |s, cx| {
                        if s.name.read(cx).value().trim().is_empty() {
                            s.error = Some("请填写项目名称。".into());
                            cx.notify();
                            return DialogDecision::KeepOpen;
                        }
                        if !s.asynchronous {
                            return DialogDecision::Close;
                        }
                        s.error = None;
                        let ticket = request.ticket();
                        let handle = w.window_handle();
                        let fail = s.fail;
                        s.task = Some(cx.spawn(async move |owner, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(1200))
                                .await;
                            let completed = handle.update(cx, |_, w, cx| {
                                ticket.complete(
                                    if fail {
                                        DialogOutcome::KeepOpen
                                    } else {
                                        DialogOutcome::Close
                                    },
                                    w,
                                    cx,
                                )
                            });
                            // Business feedback belongs to the same accepted
                            // session as the completion, never a reopened form.
                            if completed.unwrap_or(false) {
                                let _ = owner.update(cx, |s, cx| {
                                    if fail {
                                        s.error =
                                            Some("保存失败，输入已保留；可修改后重试。".into());
                                    }
                                    cx.notify();
                                });
                            }
                        }));
                        DialogDecision::Pending
                    })
                    .unwrap_or(DialogDecision::KeepOpen)
            })
            .on_close_request(move |reason, _, cx| {
                veto.update(cx, |s, cx| {
                    if s.guard {
                        s.error =
                            Some(format!("已拦截 {reason:?}。请先完成子层“允许关闭”。").into());
                        cx.notify();
                        false
                    } else {
                        true
                    }
                })
                .unwrap_or(false)
            });
        if let Some(error) = &self.error {
            dialog = dialog.child(
                div()
                    .text_color(theme.token("--error").unwrap().rgba().unwrap())
                    .child(error.clone()),
            );
        }
        let child = self.child.clone();
        let parent = self.dialog.clone();
        dialog = dialog.child(
            row()
                .child(
                    YueDropdownMenu::new(
                        "dialog-actions",
                        theme,
                        YueButton::text("dialog-menu", theme, "项目操作"),
                    )
                    .label("项目操作")
                    .items([YueMenuItem::new("查看历史"), YueMenuItem::new("复制名称")]),
                )
                .child(
                    YueTooltip::new(
                        "dialog-help",
                        theme,
                        "子浮层先响应 Escape，再由对话框响应。",
                    )
                    .trigger_with(move |focus, _, _| {
                        YueButton::icon(
                            "dialog-help-trigger",
                            theme,
                            outline::InfoCircle,
                            "操作帮助",
                        )
                        .variant(ButtonVariant::Ghost)
                        .track_focus(focus)
                        .into_any_element()
                    }),
                )
                .child(
                    YueButton::text("dialog-child-open", theme, "允许关闭…").on_click(
                        move |_, w, cx| {
                            let _ = child.update(cx, |s, cx| s.open_child(&parent, w, cx));
                        },
                    ),
                ),
        );
        if self.long {
            dialog = dialog.children((1..=30).map(|n| {
                div().child(format!(
                    "第 {n} 项说明：长内容只滚动正文，标题与操作保持可达。"
                ))
            }));
        }
        if self.custom {
            let confirm = self.dialog.clone();
            let cancel = self.dialog.clone();
            dialog = dialog.footer(DialogFooter::None).footer_content(
                row()
                    .child(
                        YueButton::text("custom-cancel", theme, "返回")
                            .disabled(self.dialog.read(cx).is_pending())
                            .on_click(move |_, w, cx| {
                                cancel.update(cx, |s, cx| s.cancel(w, cx));
                            }),
                    )
                    .child(
                        YueButton::text("custom-save", theme, "保存项目")
                            .variant(ButtonVariant::Primary)
                            .disabled(self.dialog.read(cx).is_pending())
                            .loading(self.dialog.read(cx).is_pending())
                            .on_click(move |_, w, cx| {
                                confirm.update(cx, |s, cx| s.confirm(w, cx));
                            }),
                    ),
            );
        }
        let allow = cx.weak_entity();
        let child = YueDialog::alert("allow-close", theme, &self.child)
            .title("允许关闭编辑窗口？")
            .description("只关闭这一层；随后可保存或取消项目编辑。")
            .status(DialogStatus::Warning)
            .confirm_label("允许关闭")
            .cancel_label("继续编辑")
            .on_confirm(move |_, _, cx| {
                let _ = allow.update(cx, |s, cx| {
                    s.guard = false;
                    s.error = None;
                    cx.notify();
                });
                DialogDecision::Close
            });
        column()
            .p_8()
            .gap_6()
            .items_stretch()
            .debug_selector(|| "dialog-content".into())
            .child(div().text_2xl().child("Dialog 对话框"))
            .child(readout(
                theme,
                "窗口模态 · 250ms 进入 / 150ms 退出 · 8px 平移与淡入淡出 · 支持减少动效".into(),
            ))
            .child(section(
                theme,
                "配置与预览",
                "关闭保护、异步失败、拖动和长正文使用同一个状态管线。",
                settings.child(
                    YueButton::text("open-project-dialog", theme, "编辑项目…")
                        .variant(ButtonVariant::Primary)
                        .on_click(open),
                ),
            ))
            .child(section(
                theme,
                "生命周期",
                "Esc 仅关闭最上层；退出期间仍保留输入屏障。",
                readout(theme, self.last.to_string()),
            ))
            .child(dialog)
            .child(child)
    }
}
