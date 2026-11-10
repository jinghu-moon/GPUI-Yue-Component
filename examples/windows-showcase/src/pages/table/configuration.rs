use super::*;
use gpui_kit::base::input::InputState;
use yue_component::{
    CheckboxState, DialogDecision, TableColumnDraft, TableColumnKey, TablePin, YueCheckbox,
    YueDialog, YueDialogState, YueInput,
};

/// A host-owned settings dialog shared by the isolated and combined examples.
pub(super) struct ColumnSettings<D: TableDelegate> {
    table: Entity<YueTableState<D>>,
    dialog: Entity<YueDialogState>,
    draft: Option<TableColumnDraft>,
    widths: Vec<(TableColumnKey, Entity<InputState>, SharedString)>,
    message: SharedString,
    requests: usize,
    _subscription: Subscription,
}
impl<D: TableDelegate> ColumnSettings<D> {
    pub(super) fn new(table: &Entity<YueTableState<D>>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(table, |s, table, event: &TableEvent, cx| {
            if let TableEvent::ColumnConfigurationRequested(request) = event {
                s.requests += 1;
                let current = table.read(cx).view().unwrap().clone();
                let result = request.apply_to(&current).and_then(|next| {
                    table.update(cx, |s, cx| s.commit_view(next, Some(request.id()), cx))
                });
                if result.is_err() {
                    let _ = table.update(cx, |s, cx| s.reject_request(request.id(), cx));
                }
                s.message = format!(
                    "列配置请求 {} 次 · {}",
                    s.requests,
                    if result.is_ok() {
                        "已接受"
                    } else {
                        "未接受，保留原配置"
                    }
                )
                .into();
                cx.notify();
            }
        });
        Self {
            table: table.clone(),
            dialog: cx.new(YueDialogState::new),
            draft: None,
            widths: vec![],
            message: "取消不会发布草稿".into(),
            requests: 0,
            _subscription: subscription,
        }
    }
    fn prepare(&mut self, reset: bool, w: &mut Window, cx: &mut Context<Self>) {
        let view = self.table.read(cx).view().unwrap().clone();
        let mut draft = TableColumnDraft::new(&view);
        if reset {
            draft.reset();
        }
        self.widths = view
            .declared_columns()
            .iter()
            .filter(|c| c.is_allowed() && c.is_configurable())
            .map(|c| {
                let width = if reset {
                    c.preferred_width()
                } else {
                    view.column_preferences()
                        .values()
                        .get(c.key())
                        .and_then(|p| p.width())
                        .unwrap_or(c.preferred_width())
                };
                {
                    let initial: SharedString = width.0.to_string().into();
                    (
                        c.key().clone(),
                        cx.new(|cx| {
                            let mut s = InputState::new(w, cx);
                            s.set_value(initial.clone(), w, cx);
                            s
                        }),
                        initial,
                    )
                }
            })
            .collect();
        self.draft = Some(draft);
        cx.notify();
    }
}
impl<D: TableDelegate> Render for ColumnSettings<D> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let current = self.table.read(cx).view().unwrap().clone();
        let mut body = div().flex().flex_col().gap_2();
        if let Some(draft) = &self.draft {
            let effective = draft.validate_for(&current).ok();
            let columns = effective
                .as_ref()
                .map_or(current.columns(), |v| v.columns());
            for (index, c) in columns.iter().enumerate() {
                if !c.is_allowed() || !c.is_configurable() {
                    continue;
                }
                let visible = c.is_visible();
                let key = c.key().clone();
                let toggle = key.clone();
                let pin_key = key.clone();
                let order_key = key.clone();
                let next_pin = match c.pin_kind() {
                    TablePin::None => TablePin::Start,
                    TablePin::Start => TablePin::End,
                    TablePin::End => TablePin::None,
                };
                let owner = cx.weak_entity();
                let pin_owner = owner.clone();
                let order_owner = owner.clone();
                let mut row = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        YueCheckbox::new(
                            (
                                gpui_kit::ElementId::from("column-visible"),
                                key.as_str().to_owned(),
                            ),
                            theme,
                            if visible {
                                CheckboxState::Checked
                            } else {
                                CheckboxState::Unchecked
                            },
                        )
                        .label(c.title().clone())
                        .on_change(move |v, _, _, cx| {
                            let _ = owner.update(cx, |s, cx| {
                                let _ = s
                                    .draft
                                    .as_mut()
                                    .unwrap()
                                    .set_visible(&toggle, v == CheckboxState::Checked);
                                cx.notify();
                            });
                        }),
                    )
                    .child(
                        YueButton::new(
                            (
                                gpui_kit::ElementId::from("column-pin"),
                                key.as_str().to_owned(),
                            ),
                            format!("固定：{:?}", c.pin_kind()),
                        )
                        .on_click(move |_, _, cx| {
                            let _ = pin_owner.update(cx, |s, cx| {
                                let _ = s.draft.as_mut().unwrap().set_pin(&pin_key, next_pin);
                                cx.notify();
                            });
                        }),
                    )
                    .child(
                        YueButton::new(
                            (
                                gpui_kit::ElementId::from("column-up"),
                                key.as_str().to_owned(),
                            ),
                            "上移",
                        )
                        .disabled(index == 0)
                        .on_click(move |_, _, cx| {
                            let _ = order_owner.update(cx, |s, cx| {
                                let view = s.table.read(cx).view().unwrap();
                                let draft = s.draft.as_mut().unwrap();
                                if let Ok(v) = draft.validate_for(view) {
                                    let mut order: Vec<_> =
                                        v.columns().iter().map(|c| c.key().clone()).collect();
                                    let ix = order.iter().position(|k| k == &order_key).unwrap();
                                    if ix > 0 {
                                        order.swap(ix, ix - 1);
                                        let _ = draft.set_order(order);
                                    }
                                }
                                cx.notify();
                            });
                        }),
                    );
                if let Some((_, input, _)) = self.widths.iter().find(|(k, _, _)| k == &key) {
                    row = row.child(
                        YueInput::new(
                            (
                                gpui_kit::ElementId::from("column-width"),
                                key.as_str().to_owned(),
                            ),
                            theme,
                            input,
                        )
                        .width(rems(5.))
                        .accessibility_label(format!("{}宽度，rem", c.title())),
                    );
                }
                body = body.child(row);
            }
        }
        let owner = cx.weak_entity();
        let dialog = YueDialog::new("table-column-settings-dialog", theme, &self.dialog)
            .title("列设置")
            .description("勾选、固定、顺序和宽度在确认后一次生效。宽度单位 rem。")
            .child(body)
            .child(
                YueButton::new("column-settings-reset", "恢复声明默认值")
                    .on_click(cx.listener(|s, _, w, cx| s.prepare(true, w, cx))),
            )
            .on_confirm(move |_, _, cx| {
                owner
                    .update(cx, |s, cx| {
                        let Some(draft) = &mut s.draft else {
                            return DialogDecision::KeepOpen;
                        };
                        for (key, input, initial) in &s.widths {
                            if input.read(cx).value() == *initial {
                                continue;
                            }
                            let Ok(width) = input.read(cx).value().parse::<f32>() else {
                                s.message = "宽度必须为正数".into();
                                cx.notify();
                                return DialogDecision::KeepOpen;
                            };
                            if draft.set_width(key, rems(width)).is_err() {
                                s.message = "宽度必须为有限正数".into();
                                cx.notify();
                                return DialogDecision::KeepOpen;
                            }
                        }
                        match s.table.update(cx, |state, cx| {
                            state.request_column_configuration(draft, cx)
                        }) {
                            Ok(_) => DialogDecision::Close,
                            Err(_) => {
                                s.message = "声明或权限已变化，请重新打开设置".into();
                                cx.notify();
                                DialogDecision::KeepOpen
                            }
                        }
                    })
                    .unwrap_or(DialogDecision::KeepOpen)
            });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                YueButton::new("table-column-settings", "列设置…").on_click(cx.listener(
                    |s, _, w, cx| {
                        s.prepare(false, w, cx);
                        let _ = s.dialog.update(cx, |s, cx| s.open(w, cx));
                    },
                )),
            )
            .child(self.message.clone())
            .child(dialog)
    }
}
