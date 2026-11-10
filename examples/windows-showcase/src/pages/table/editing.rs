use super::*;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    EntityInputHandler as _, Focusable as _,
    base::input::{InputEvent, InputState},
};
use yue_component::{
    InputSize, InputStatus, SelectEvent, SelectMode, SelectOption, SelectValue, TableCellKey,
    TableEditMode, TableEditStage, TableEditToken, YueInput, YueSelect, YueSelectState,
};

pub(super) struct EditingExample {
    _delegate: Entity<Orders>,
    table: Entity<YueTableState<Orders>>,
    name: Entity<InputState>,
    amount: Entity<InputState>,
    status: Entity<YueSelectState>,
    form: Entity<yue_component::YueForm>,
    save: Option<Save>,
    old_save: Option<Save>,
    delayed: bool,
    requests: usize,
    draft: Option<(SharedString, SharedString, SelectValue)>,
    message: SharedString,
    _subscriptions: Vec<Subscription>,
}
struct Save {
    token: TableEditToken,
    key: TableRowKey,
    name: SharedString,
    amount: i32,
    done: bool,
}
impl EditingExample {
    pub(super) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(w, cx));
        let amount = cx.new(|cx| InputState::new(w, cx));
        let status = cx.new(|cx| {
            let mut s = YueSelectState::new(SelectMode::Single, w, cx);
            s.set_options(
                [
                    SelectOption::new("yes", "已核验"),
                    SelectOption::new("no", "待核验"),
                ],
                cx,
            )
            .unwrap();
            s
        });
        let form = cx.new(|_| {
            let mut form = yue_component::YueForm::new();
            form.require("name", &name, "负责人不能为空");
            form.rule("amount", &amount, |value, _| {
                (!value.parse::<i32>().is_ok_and(|v| v >= 0)).then(|| "金额必须为非负整数".into())
            });
            form
        });
        let name_form = form.clone();
        let amount_form = form.clone();
        let input_name = name.clone();
        let input_amount = amount.clone();
        let input_status = status.clone();
        let columns: Rc<[_]> = vec![
            TableColumn::text("key", "工单", |r: &Order| r.key.clone()).width(rems(8.)),
            TableColumn::custom("name", "负责人", move |row: &Order, cell, _, cx| {
                if !cell.is_editing() {
                    return div().child(row.name.clone()).into_any_element();
                }
                editor(
                    "edit-name",
                    "负责人",
                    &input_name,
                    &name_form,
                    "name",
                    cell,
                    cx,
                )
            })
            .editable(true)
            .width(rems(17.)),
            TableColumn::custom("amount", "金额", move |row: &Order, cell, _, cx| {
                if !cell.is_editing() {
                    return div().child(row.amount.to_string()).into_any_element();
                }
                editor(
                    "edit-amount",
                    "金额",
                    &input_amount,
                    &amount_form,
                    "amount",
                    cell,
                    cx,
                )
            })
            .editable(true)
            .width(rems(13.)),
            TableColumn::custom("done", "核验状态", move |row: &Order, cell, _, _| {
                if !cell.is_editing() {
                    return div()
                        .child(if row.done { "已核验" } else { "待核验" })
                        .into_any_element();
                }
                YueSelect::new("edit-status", &input_status)
                    .accessibility_label("核验状态")
                    .into_any_element()
            })
            .editable(true)
            .width(rems(13.)),
        ]
        .into();
        let delegate = cx.new(|_| Orders);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(records(1000, 1, false), columns)
                    .selection_mode(TableSelectionMode::Multiple)
                    .with_row_sizing(yue_component::TableRowSizing::measured(rems(2.5)).unwrap()),
                None,
                cx,
            )
            .unwrap()
        });
        let mut subscriptions = vec![cx.subscribe_in(
            &table,
            w,
            |s: &mut Self, _, event: &TableEvent, w, cx| match event {
                TableEvent::EditRequested { key, mode } => s.start(key.clone(), *mode, w, cx),
                TableEvent::EditSaveRequested(token) => s.validate(token.clone(), w, cx),
                TableEvent::SelectionRequested(request) => {
                    let next = request.apply_to(s.table.read(cx).view().unwrap()).unwrap();
                    s.table.update(cx, |state, cx| {
                        state.commit_view(next, Some(request.id()), cx).unwrap()
                    });
                }
                TableEvent::EditEnded => {
                    s.message = "编辑会话已结束".into();
                    cx.notify();
                }
                _ => {}
            },
        )];
        for input in [&name, &amount] {
            subscriptions.push(cx.subscribe_in(
                input,
                w,
                |s: &mut Self, _, event: &InputEvent, w, cx| match event {
                    InputEvent::Change => {
                        s.observe_draft(cx);
                    }
                    InputEvent::PressEnter {
                        secondary: false, ..
                    } => {
                        s.request_save(yue_component::TableEditMove::Stay, cx);
                    }
                    _ => {
                        let _ = w;
                    }
                },
            ));
        }
        subscriptions.push(
            cx.subscribe(&status, |s: &mut Self, _, event: &SelectEvent, cx| {
                if matches!(event, SelectEvent::Change(_)) {
                    s.observe_draft(cx);
                }
            }),
        );
        Self {
            _delegate: delegate,
            table,
            name,
            amount,
            status,
            form,
            save: None,
            old_save: None,
            delayed: false,
            requests: 0,
            draft: None,
            message: "双击或 F2 编辑单元格；也可以编辑整行。Enter 保存，Escape 取消".into(),
            _subscriptions: subscriptions,
        }
    }
    fn start(
        &mut self,
        key: TableCellKey,
        mode: TableEditMode,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.table.read(cx).edit_session().is_some() {
            return;
        }
        self.form.update(cx, |s, cx| s.reset(cx));
        let current = self.table.read(cx).view().unwrap();
        let row = current.source().row(key.row()).unwrap();
        let name = row.name.clone();
        let amount = row.amount.to_string();
        let done = row.done;
        self.name.update(cx, |s, cx| s.set_value(name, w, cx));
        self.amount.update(cx, |s, cx| s.set_value(amount, w, cx));
        self.status.update(cx, |s, cx| {
            s.set_value(
                SelectValue::Single(Some(if done { "yes" } else { "no" }.into())),
                w,
                cx,
            )
            .unwrap()
        });
        let column = key.column().as_str().to_owned();
        if self
            .table
            .update(cx, |s, cx| s.start_edit(key, mode, w, cx))
            .is_err()
        {
            return;
        }
        let handles = vec![
            self.name.read(cx).focus_handle(cx),
            self.amount.read(cx).focus_handle(cx),
            self.status.read(cx).focus_handle().clone(),
        ];
        self.table.update(cx, |s, cx| {
            s.set_edit_focus_handles(handles.clone(), cx).unwrap()
        });
        handles[match column.as_str() {
            "amount" => 1,
            "done" => 2,
            _ => 0,
        }]
        .focus(w, cx);
        self.draft = Some((
            self.name.read(cx).value(),
            self.amount.read(cx).value(),
            self.status.read(cx).value().clone(),
        ));
        cx.notify();
    }
    fn observe_draft(&mut self, cx: &mut Context<Self>) {
        if self.table.read(cx).edit_session().is_none() {
            self.draft = None;
            return;
        }
        let draft = (
            self.name.read(cx).value(),
            self.amount.read(cx).value(),
            self.status.read(cx).value().clone(),
        );
        if self.draft.as_ref() != Some(&draft) {
            self.draft = Some(draft);
            let _ = self
                .table
                .update(cx, |state, cx| state.edit_draft_changed(cx));
        }
    }
    fn request_save(&mut self, movement: yue_component::TableEditMove, cx: &mut Context<Self>) {
        self.observe_draft(cx);
        let _ = self.table.update(cx, |state, cx| {
            state.request_edit_save_with_move(movement, cx)
        });
    }
    fn tab(&mut self, shift: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(mode) = self
            .table
            .read(cx)
            .edit_session()
            .map(|session| session.mode())
        else {
            cx.propagate();
            return;
        };
        if self
            .name
            .update(cx, |input, cx| input.marked_text_range(w, cx).is_some())
            || self
                .amount
                .update(cx, |input, cx| input.marked_text_range(w, cx).is_some())
            || self.status.read(cx).is_open()
        {
            return;
        }
        if mode == TableEditMode::Row {
            let name = self.name.read(cx).focus_handle(cx);
            let amount = self.amount.read(cx).focus_handle(cx);
            let status = self.status.read(cx).focus_handle().clone();
            if !shift && name.is_focused(w) {
                amount.focus(w, cx);
                cx.stop_propagation();
                return;
            }
            if !shift && amount.is_focused(w) {
                status.focus(w, cx);
                cx.stop_propagation();
                return;
            }
            if shift && status.is_focused(w) {
                amount.focus(w, cx);
                cx.stop_propagation();
                return;
            }
            if shift && amount.is_focused(w) {
                name.focus(w, cx);
                cx.stop_propagation();
                return;
            }
        }
        let movement = if shift {
            yue_component::TableEditMove::Previous
        } else {
            yue_component::TableEditMove::Next
        };
        self.request_save(movement, cx);
        cx.stop_propagation();
    }
    fn validate(&mut self, token: TableEditToken, w: &mut Window, cx: &mut Context<Self>) {
        if !self.table.read(cx).is_current_edit(&token) {
            return;
        }
        self.requests += 1;
        let name = self.name.read(cx).value();
        let amount = self.amount.read(cx).value().parse::<i32>();
        let done = matches!(self.status.read(cx).value(),SelectValue::Single(Some(key)) if key.as_ref()=="yes");
        if !self.form.update(cx, |form, cx| form.submit(w, cx)) {
            self.table.update(cx, |s, cx| {
                s.reject_edit(&token, "负责人不能为空，金额必须为非负整数", cx)
                    .unwrap()
            });
            return;
        }
        self.table
            .update(cx, |s, cx| s.accept_edit_validation(&token, cx).unwrap());
        let key = self
            .table
            .read(cx)
            .edit_session()
            .unwrap()
            .key()
            .row()
            .clone();
        let save = Save {
            token,
            key,
            name,
            amount: amount.unwrap(),
            done,
        };
        if self.delayed {
            self.old_save = self.save.replace(save);
            self.message = "等待保存结果；继续输入会使旧结果失效".into();
        } else {
            self.complete(save, w, cx);
        }
        cx.notify();
    }
    fn external_change(&mut self, remove: bool, cx: &mut Context<Self>) {
        let Some(key) = self
            .table
            .read(cx)
            .edit_session()
            .map(|s| s.key().row().clone())
        else {
            return;
        };
        let current = self.table.read(cx).view().unwrap();
        let rows = (0..current.source().len())
            .filter_map(|ix| {
                let row_key = current.source().row_key(ix).unwrap();
                if remove && row_key == &key {
                    return None;
                }
                let row = current.source().row(row_key).unwrap();
                Some(Order {
                    key: row.key.clone(),
                    name: if row_key == &key {
                        "外部更新的负责人".into()
                    } else {
                        row.name.clone()
                    },
                    amount: row.amount,
                    done: row.done,
                    note: row.note.clone(),
                })
            })
            .collect::<Vec<_>>();
        let source = Rc::new(
            TableRows::try_new(
                current.source().scope().clone(),
                current.source().revision() + 1,
                rows.into(),
                |r: &Order| r.key.clone(),
            )
            .unwrap(),
        );
        let selection = current
            .selected_keys()
            .iter()
            .filter(|key| source.contains_key(key))
            .cloned()
            .collect::<Vec<_>>();
        let next = TableView::new(source, current.shared_columns().clone())
            .selection_mode(current.selection_kind())
            .selection(selection)
            .with_row_sizing(current.row_sizing().clone());
        self.table
            .update(cx, |s, cx| s.commit_view(next, None, cx).unwrap());
        self.message = if remove {
            "记录已删除，编辑会话已清除"
        } else {
            "记录被外部更新，保留草稿并阻止覆盖"
        }
        .into();
        cx.notify();
    }
    fn complete(&mut self, save: Save, w: &mut Window, cx: &mut Context<Self>) {
        if !self.table.read(cx).is_current_edit(&save.token) {
            self.message = "已忽略过期保存结果".into();
            cx.notify();
            return;
        }
        let current = self.table.read(cx).view().unwrap().clone();
        let session = self.table.read(cx).edit_session().unwrap();
        let mode = session.mode();
        let column = session.key().column().as_str().to_owned();
        let rows = (0..current.source().len())
            .map(|i| {
                let key = current.source().row_key(i).unwrap();
                let row = current.source().row(key).unwrap();
                let target = key == &save.key;
                Order {
                    key: row.key.clone(),
                    note: row.note.clone(),
                    name: if target && (mode == TableEditMode::Row || column == "name") {
                        save.name.clone()
                    } else {
                        row.name.clone()
                    },
                    amount: if target && (mode == TableEditMode::Row || column == "amount") {
                        save.amount
                    } else {
                        row.amount
                    },
                    done: if target && (mode == TableEditMode::Row || column == "done") {
                        save.done
                    } else {
                        row.done
                    },
                }
            })
            .collect::<Vec<_>>();
        let source = Rc::new(
            TableRows::try_new(
                current.source().scope().clone(),
                current.source().revision() + 1,
                rows.into(),
                |r: &Order| r.key.clone(),
            )
            .unwrap(),
        );
        let next = TableView::new(source, current.shared_columns().clone())
            .selection_mode(current.selection_kind())
            .selection(current.selected_keys().iter().cloned())
            .with_row_sizing(current.row_sizing().clone());
        self.table
            .update(cx, |s, cx| s.commit_edit(&save.token, next, w, cx).unwrap());
        self.message = "更改已保存".into();
        cx.notify();
    }
}
fn editor(
    id: &str,
    label: &str,
    input: &Entity<InputState>,
    form: &Entity<yue_component::YueForm>,
    field: &str,
    cell: &yue_component::TableCellContext,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyElement {
    let error = match cell.edit_stage() {
        Some(TableEditStage::Error(error)) => Some(error.clone()),
        Some(TableEditStage::Conflict) => Some("记录已更新，请取消后重新编辑".into()),
        _ => None,
    };
    yue_component::YueField::new(yue_component::current_theme(cx))
        .use_default_theme()
        .bind(form, field.to_owned())
        .input(
            YueInput::new(id.to_owned(), yue_component::current_theme(cx), input)
                .use_default_theme()
                .width(gpui_kit::relative(1.))
                .size(InputSize::Small)
                .accessibility_label(label.to_owned()),
        )
        .when_some(error, |field, error| {
            field.message(InputStatus::Error, error)
        })
        .into_any_element()
}
impl Render for EditingExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.table.read(cx).edit_session().is_some();
        div()
            .on_action(
                cx.listener(|s, _: &gpui_kit::base::input::IndentInline, w, cx| {
                    s.tab(false, w, cx)
                }),
            )
            .on_action(
                cx.listener(|s, _: &gpui_kit::base::input::OutdentInline, w, cx| {
                    s.tab(true, w, cx)
                }),
            )
            .on_key_down(cx.listener(|s, event: &gpui_kit::KeyDownEvent, w, cx| {
                if event.keystroke.key != "tab" {
                    return;
                }
                s.tab(event.keystroke.modifiers.shift, w, cx);
            }))
            .flex()
            .flex_col()
            .gap_3()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        YueButton::new("edit-first-cell", "编辑首个负责人")
                            .disabled(active)
                            .on_click(cx.listener(|s, _, _, cx| {
                                let _ = s.table.update(cx, |state, cx| {
                                    state.request_edit(
                                        TableCellKey::new(
                                            TableRowKey::try_new("order-0").unwrap(),
                                            yue_component::TableColumnKey::try_new("name").unwrap(),
                                        ),
                                        TableEditMode::Cell,
                                        cx,
                                    )
                                });
                            })),
                    )
                    .child(
                        YueButton::new("edit-first-row", "编辑首行")
                            .disabled(active)
                            .on_click(cx.listener(|s, _, _, cx| {
                                let _ = s.table.update(cx, |state, cx| {
                                    state.request_edit(
                                        TableCellKey::new(
                                            TableRowKey::try_new("order-0").unwrap(),
                                            yue_component::TableColumnKey::try_new("name").unwrap(),
                                        ),
                                        TableEditMode::Row,
                                        cx,
                                    )
                                });
                            })),
                    )
                    .child(
                        YueButton::new("edit-save", "保存")
                            .disabled(!active)
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.request_save(yue_component::TableEditMove::Stay, cx);
                            })),
                    )
                    .child(
                        YueButton::new("edit-cancel", "取消")
                            .disabled(!active)
                            .on_click(cx.listener(|s, _, w, cx| {
                                s.table.update(cx, |state, cx| state.cancel_edit(w, cx));
                            })),
                    )
                    .child(
                        YueButton::new(
                            "edit-delay",
                            if self.delayed {
                                "恢复即时保存"
                            } else {
                                "模拟延迟保存"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.delayed = !s.delayed;
                            cx.notify();
                        })),
                    )
                    .child(
                        YueButton::new("edit-complete", "完成保存")
                            .disabled(self.save.is_none())
                            .on_click(cx.listener(|s, _, w, cx| {
                                if let Some(save) = s.save.take() {
                                    s.complete(save, w, cx);
                                }
                            })),
                    )
                    .child(
                        YueButton::new("edit-old", "完成旧保存")
                            .disabled(self.old_save.is_none())
                            .on_click(cx.listener(|s, _, w, cx| {
                                if let Some(save) = s.old_save.take() {
                                    s.complete(save, w, cx);
                                }
                            })),
                    )
                    .child(
                        YueButton::new("edit-fail", "模拟保存失败")
                            .disabled(self.save.is_none())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if let Some(save) = s.save.take() {
                                    let _ = s.table.update(cx, |state, cx| {
                                        state.reject_edit(
                                            &save.token,
                                            "保存失败，输入已保留，可重试",
                                            cx,
                                        )
                                    });
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        YueButton::new("edit-conflict", "模拟外部更新")
                            .disabled(!active)
                            .on_click(cx.listener(|s, _, _, cx| s.external_change(false, cx))),
                    )
                    .child(
                        YueButton::new("edit-delete", "模拟记录删除")
                            .disabled(!active)
                            .on_click(cx.listener(|s, _, _, cx| s.external_change(true, cx))),
                    ),
            )
            .child(self.message.clone())
            .child(
                YueTable::new("editing-table", &self.table)
                    .height(rems(24.))
                    .accessibility_label("可编辑工单"),
            )
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{ElementId, TestAppContext, VisualTestContext, px, size};
    use yue_component::{Accent, ThemeMode, YueTheme};
    pub(super) fn open(
        cx: &mut TestAppContext,
    ) -> (Entity<EditingExample>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut entity = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| EditingExample::new(w, cx));
            entity = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.update(|w, _| w.resize(size(px(1500.), px(1000.))));
        draw(cx);
        (entity.unwrap(), cx)
    }
    fn name(example: &Entity<EditingExample>, cx: &VisualTestContext) -> SharedString {
        example.read_with(cx, |s, cx| {
            s.table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .row(&TableRowKey::try_new("order-0").unwrap())
                .unwrap()
                .name
                .clone()
        })
    }
    #[gpui_kit::test]
    fn actual_input_save_cancel_validation_and_tab_flush(cx: &mut TestAppContext) {
        let (example, cx) = open(cx);
        click(cx, "edit-first-cell");
        assert!(cx.update(|w, cx| {
            example
                .read(cx)
                .name
                .read(cx)
                .focus_handle(cx)
                .is_focused(w)
        }));
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("更新负责人");
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(name(&example, cx).as_ref(), "更新负责人");
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).edit_session().is_none()));
        click(cx, "edit-first-cell");
        cx.simulate_keystrokes("ctrl-a backspace enter");
        draw(cx);
        assert!(example.read_with(cx, |s, cx| matches!(
            s.table.read(cx).edit_session().unwrap().stage(),
            TableEditStage::Error(_)
        )));
        cx.simulate_input("保留输入");
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert_eq!(name(&example, cx).as_ref(), "更新负责人");
        click(cx, "edit-first-cell");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("Tab立即保存");
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_eq!(name(&example, cx).as_ref(), "Tab立即保存");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .edit_session()
                .unwrap()
                .key()
                .column()
                .as_str()
                .to_owned()),
            "amount"
        );
        cx.simulate_keystrokes("escape");
        draw(cx);
    }
    #[gpui_kit::test]
    fn delayed_save_keeps_draft_rejects_old_result_and_retries_after_failure(
        cx: &mut TestAppContext,
    ) {
        let (example, cx) = open(cx);
        click(cx, "edit-delay");
        click(cx, "edit-first-cell");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("旧草稿");
        cx.simulate_keystrokes("enter");
        draw(cx);
        click(cx, "edit-name");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("新草稿");
        cx.simulate_keystrokes("enter");
        draw(cx);
        click(cx, "edit-old");
        assert_ne!(name(&example, cx).as_ref(), "旧草稿");
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).edit_session().is_some()));
        click(cx, "edit-fail");
        assert_eq!(
            example
                .read_with(cx, |s, cx| s.name.read(cx).value())
                .as_ref(),
            "新草稿"
        );
        click(cx, "edit-save");
        click(cx, "edit-complete");
        assert_eq!(name(&example, cx).as_ref(), "新草稿");
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).edit_session().is_none()));
    }
    #[gpui_kit::test]
    fn row_controls_keep_drafts_offscreen_and_commit_current_fields(cx: &mut TestAppContext) {
        let (example, cx) = open(cx);
        click(cx, "edit-first-row");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("整行负责人");
        cx.simulate_keystrokes("tab ctrl-a");
        cx.simulate_input("4321");
        draw(cx);
        assert_eq!(
            example
                .read_with(cx, |s, cx| s.amount.read(cx).value())
                .as_ref(),
            "4321"
        );
        let table = example.read_with(cx, |s, _| s.table.clone());
        cx.update(|_, cx| {
            table
                .read(cx)
                .scroll_handle()
                .set_offset(gpui_kit::point(px(0.), px(-12000.)))
        });
        draw(cx);
        let retained = cx.update(|w, _| {
            snapshots(w)
                .iter()
                .filter(|n| n.path().last() == Some(&ElementId::from("edit-name")))
                .count()
        });
        assert_eq!(
            retained, 1,
            "retain one actual editor, not all intervening rows"
        );
        assert_eq!(
            example
                .read_with(cx, |s, cx| s.name.read(cx).value())
                .as_ref(),
            "整行负责人"
        );
        click(cx, "edit-save");
        assert_eq!(name(&example, cx).as_ref(), "整行负责人");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .row(&TableRowKey::try_new("order-0").unwrap())
                .unwrap()
                .amount),
            4321
        );
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod conflict_tests {
    use super::tests::open;
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn external_revision_preserves_actual_draft_and_deletion_repairs_only_owned_focus(
        cx: &mut TestAppContext,
    ) {
        let (example, cx) = open(cx);
        click(cx, "edit-first-cell");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("未保存的真实草稿");
        draw(cx);
        click(cx, "edit-conflict");
        assert_eq!(
            example
                .read_with(cx, |s, cx| s.name.read(cx).value())
                .as_ref(),
            "未保存的真实草稿"
        );
        assert!(example.read_with(cx, |s, cx| matches!(
            s.table.read(cx).edit_session().unwrap().stage(),
            TableEditStage::Conflict
        )));
        click(cx, "edit-name");
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).edit_session().is_some()));
        // Delete while the actual editor still owns focus, rather than testing only toolbar focus.
        example.update(cx, |s, cx| s.external_change(true, cx));
        draw(cx);
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).edit_session().is_none()));
        assert!(cx.update(|w, cx| {
            example
                .read(cx)
                .table
                .read(cx)
                .focus_handle(cx)
                .is_focused(w)
        }));
        assert!(example.read_with(cx, |s, cx| {
            s.table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .position_of(&TableRowKey::try_new("order-0").unwrap())
                .is_none()
        }));
    }
}
