use super::*;
use chrono::{NaiveDate, NaiveTime};
use gpui_kit::Focusable as _;
use gpui_kit::base::input::InputEvent;
use yue_component::{
    DatePickerEndpoint, DatePickerOptions, DatePickerValue, TableEditMode, TableEditToken,
    TimePickerEndpoint, TimePickerOptions, TimePickerValue, YueDatePicker, YueDatePickerState,
    YueTimePicker, YueTimePickerState,
};
struct Appointment {
    key: SharedString,
    date: NaiveDate,
    time: NaiveTime,
}
struct Appointments;
impl TableDelegate for Appointments {
    type Source = TableRows<Appointment>;
}
pub(super) struct TemporalExample {
    _delegate: Entity<Appointments>,
    table: Entity<YueTableState<Appointments>>,
    date: Entity<YueDatePickerState>,
    time: Entity<YueTimePickerState>,
    _subscriptions: Vec<Subscription>,
    message: SharedString,
}
impl TemporalExample {
    pub(super) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let date = cx.new(|cx| {
            YueDatePickerState::new(
                DatePickerOptions::new("table-edit-date")
                    .commit_mode(yue_component::DatePickerCommitMode::Immediate),
                w,
                cx,
            )
            .unwrap()
        });
        let time = cx.new(|cx| {
            YueTimePickerState::new(
                TimePickerOptions::new("table-edit-time")
                    .commit_mode(yue_component::TimePickerCommitMode::Immediate),
                w,
                cx,
            )
            .unwrap()
        });
        let date_editor = date.clone();
        let time_editor = time.clone();
        let delegate = cx.new(|_| Appointments);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        let columns = vec![
            TableColumn::text("key", "预约", |r: &Appointment| r.key.clone()).width(rems(8.)),
            TableColumn::custom("date", "日期", move |r: &Appointment, cell, _, _| {
                if cell.is_editing() {
                    YueDatePicker::new(&date_editor)
                        .accessibility_label("预约日期")
                        .into_any_element()
                } else {
                    div().child(r.date.to_string()).into_any_element()
                }
            })
            .editable(true)
            .width(rems(19.)),
            TableColumn::custom("time", "时间", move |r: &Appointment, cell, _, _| {
                if cell.is_editing() {
                    YueTimePicker::new(&time_editor)
                        .accessibility_label("预约时间")
                        .into_any_element()
                } else {
                    div().child(r.time.to_string()).into_any_element()
                }
            })
            .editable(true)
            .width(rems(16.)),
        ];
        let rows = (0..3)
            .map(|i| Appointment {
                key: format!("appointment-{i}").into(),
                date: NaiveDate::from_ymd_opt(2026, 11, 11 + i).unwrap(),
                time: NaiveTime::from_hms_opt(9 + i, 0, 0).unwrap(),
            })
            .collect::<Vec<_>>();
        table.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    Rc::new(
                        TableRows::try_new("appointments", 1, rows.into(), |r: &Appointment| {
                            r.key.clone()
                        })
                        .unwrap(),
                    ),
                    columns,
                ),
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
                TableEvent::EditSaveRequested(token) => s.save(token, w, cx),
                _ => {}
            },
        )];
        let date_input = date.read(cx).input(DatePickerEndpoint::Start).clone();
        let time_input = time.read(cx).input(TimePickerEndpoint::Start).clone();
        for input in [&date_input, &time_input] {
            subscriptions.push(
                cx.subscribe(input, |s: &mut Self, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change)
                        && s.table.read(cx).edit_session().is_some()
                    {
                        let _ = s.table.update(cx, |s, cx| s.edit_draft_changed(cx));
                    }
                }),
            );
        }
        subscriptions.push(cx.subscribe(
            &date,
            |s: &mut Self, _, event: &yue_component::DatePickerEvent, cx| {
                if matches!(event, yue_component::DatePickerEvent::Change(_))
                    && s.table.read(cx).edit_session().is_some()
                {
                    let _ = s.table.update(cx, |s, cx| s.edit_draft_changed(cx));
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &time,
            |s: &mut Self, _, event: &yue_component::TimePickerEvent, cx| {
                if matches!(event, yue_component::TimePickerEvent::Change(_))
                    && s.table.read(cx).edit_session().is_some()
                {
                    let _ = s.table.update(cx, |s, cx| s.edit_draft_changed(cx));
                }
            },
        ));
        Self {
            _delegate: delegate,
            table,
            date,
            time,
            _subscriptions: subscriptions,
            message: "日期与时间复用已有控件；未点控件确认时，保存也会读取实际输入草稿。".into(),
        }
    }
    fn start(
        &mut self,
        key: yue_component::TableCellKey,
        mode: TableEditMode,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.table.read(cx).edit_session().is_some() {
            return;
        }
        let view = self.table.read(cx).view().unwrap();
        let row = view.source().row(key.row()).unwrap();
        let (date, time) = (row.date, row.time);
        self.date.update(cx, |s, cx| {
            s.set_value(DatePickerValue::date(date), w, cx).unwrap()
        });
        self.time.update(cx, |s, cx| {
            s.set_value(TimePickerValue::Single(time), w, cx).unwrap()
        });
        let time_first = key.column().as_str() == "time";
        if self
            .table
            .update(cx, |s, cx| s.start_edit(key, mode, w, cx))
            .is_err()
        {
            return;
        }
        let handles = vec![
            self.date
                .read(cx)
                .input(DatePickerEndpoint::Start)
                .read(cx)
                .focus_handle(cx),
            self.time
                .read(cx)
                .input(TimePickerEndpoint::Start)
                .read(cx)
                .focus_handle(cx),
        ];
        self.table.update(cx, |s, cx| {
            s.set_edit_focus_handles(handles.clone(), cx).unwrap()
        });
        handles[usize::from(time_first)].focus(w, cx);
        cx.notify();
    }
    fn request_save(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.date.update(cx, |s, cx| {
            s.accept_input(w, cx);
        });
        self.time.update(cx, |s, cx| {
            s.accept_input(false, w, cx);
        });
        // Picker change events invalidate the draft before a save token is created.
        let owner = cx.weak_entity();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |s, cx| {
                let _ = s.table.update(cx, |s, cx| s.request_edit_save(cx));
            });
        });
    }
    fn save(&mut self, token: &TableEditToken, w: &mut Window, cx: &mut Context<Self>) {
        if !self.table.read(cx).is_current_edit(token) {
            return;
        }
        let candidate = (
            Ok::<_, yue_component::DatePickerError>(self.date.read(cx).value().clone()),
            self.time.read(cx).candidate(cx),
        );
        let invalid = self.date.read(cx).error().is_some() || self.time.read(cx).error().is_some();
        let (Ok(DatePickerValue::Single(date)), Ok(TimePickerValue::Single(time))) = candidate
        else {
            self.table.update(cx, |s, cx| {
                s.reject_edit(token, "请输入完整的日期和时间", cx).unwrap()
            });
            self.message = "日期或时间无效，草稿和旧记录保留".into();
            cx.notify();
            return;
        };
        if invalid {
            self.table.update(cx, |s, cx| {
                s.reject_edit(token, "请输入有效日期与时间", cx).unwrap()
            });
            cx.notify();
            return;
        }
        let current = self.table.read(cx).view().unwrap().clone();
        let session = self.table.read(cx).edit_session().unwrap();
        let key = session.key().clone();
        let mode = session.mode();
        let rows = (0..current.source().len())
            .map(|i| {
                let k = current.source().row_key(i).unwrap();
                let r = current.source().row(k).unwrap();
                let target = k == key.row();
                Appointment {
                    key: r.key.clone(),
                    date: if target
                        && (mode == TableEditMode::Row || key.column().as_str() == "date")
                    {
                        date.start_date()
                    } else {
                        r.date
                    },
                    time: if target
                        && (mode == TableEditMode::Row || key.column().as_str() == "time")
                    {
                        time
                    } else {
                        r.time
                    },
                }
            })
            .collect::<Vec<_>>();
        let source = Rc::new(
            TableRows::try_new(
                "appointments",
                current.source().revision() + 1,
                rows.into(),
                |r: &Appointment| r.key.clone(),
            )
            .unwrap(),
        );
        let next = TableView::new(source, current.declared_columns().to_vec());
        self.table.update(cx, |s, cx| {
            s.accept_edit_validation(token, cx).unwrap();
            s.commit_edit(token, next, w, cx).unwrap();
        });
        self.message = "保存成功；只更新当前编辑行／单元格".into();
        cx.notify();
    }
}
impl Render for TemporalExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        YueButton::new("temporal-edit-row", "编辑首条预约").on_click(cx.listener(
                            |s, _, w, cx| {
                                let key = yue_component::TableCellKey::new(
                                    TableRowKey::try_new("appointment-0").unwrap(),
                                    yue_component::TableColumnKey::try_new("date").unwrap(),
                                );
                                s.start(key, TableEditMode::Row, w, cx);
                            },
                        )),
                    )
                    .child(
                        YueButton::new("temporal-save", "保存预约")
                            .disabled(self.table.read(cx).edit_session().is_none())
                            .on_click(cx.listener(|s, _, w, cx| s.request_save(w, cx))),
                    )
                    .child(YueButton::new("temporal-cancel", "取消预约编辑").on_click(
                        cx.listener(|s, _, w, cx| s.table.update(cx, |s, cx| s.cancel_edit(w, cx))),
                    )),
            )
            .child(
                YueTable::new("temporal-table", &self.table)
                    .height(rems(12.))
                    .row_height(rems(3.5)),
            )
            .child(self.message.clone())
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::base::Root;
    use gpui_kit::{TestAppContext, VisualTestContext, px, size};
    fn open(cx: &mut TestAppContext) -> (Entity<TemporalExample>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(
                cx,
                yue_component::YueTheme::new(
                    yue_component::ThemeMode::Light,
                    yue_component::Accent::Azure,
                ),
            )
            .unwrap()
        });
        let mut entity = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| TemporalExample::new(w, cx));
            entity = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.update(|w, _| w.resize(size(px(1300.), px(800.))));
        draw(cx);
        (entity.unwrap(), cx)
    }
    fn draft(p: &Entity<TemporalExample>, cx: &mut VisualTestContext, date: &str, time: &str) {
        cx.update(|w, cx| {
            p.update(cx, |p, cx| {
                let a = p.date.read(cx).input(DatePickerEndpoint::Start).clone();
                let b = p.time.read(cx).input(TimePickerEndpoint::Start).clone();
                a.update(cx, |s, cx| s.set_value(date.to_owned(), w, cx));
                b.update(cx, |s, cx| s.set_value(time.to_owned(), w, cx));
            })
        });
        draw(cx);
    }
    #[gpui_kit::test]
    fn date_and_time_real_drafts_flush_validate_retry_and_cancel(cx: &mut TestAppContext) {
        let (p, cx) = open(cx);
        click(cx, "temporal-edit-row");
        draft(&p, cx, "2027-01-02", "13:14:15");
        click(cx, "temporal-save");
        assert!(p.read_with(cx, |p, cx| p.table.read(cx).edit_session().is_none()));
        let values = p.read_with(cx, |p, cx| {
            let v = p.table.read(cx).view().unwrap();
            let r = v.source().row(v.source().row_key(0).unwrap()).unwrap();
            (r.date, r.time)
        });
        assert_eq!(
            values,
            (
                NaiveDate::from_ymd_opt(2027, 1, 2).unwrap(),
                NaiveTime::from_hms_opt(13, 14, 15).unwrap()
            )
        );
        click(cx, "temporal-edit-row");
        draft(&p, cx, "bad-date", "25:99:00");
        click(cx, "temporal-save");
        assert!(p.read_with(cx, |p, cx| p.table.read(cx).edit_session().is_some()));
        draft(&p, cx, "2028-02-29", "08:09:10");
        click(cx, "temporal-save");
        assert!(p.read_with(cx, |p, cx| p.table.read(cx).edit_session().is_none()));
        let revision = p.read_with(cx, |p, cx| p.table.read(cx).view().unwrap().revision());
        click(cx, "temporal-edit-row");
        draft(&p, cx, "2029-01-01", "01:02:03");
        click(cx, "temporal-cancel");
        assert_eq!(
            p.read_with(cx, |p, cx| p.table.read(cx).view().unwrap().revision()),
            revision
        );
    }
}
