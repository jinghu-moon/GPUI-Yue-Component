use super::super::*;
use chrono::{NaiveTime, Timelike as _};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{Entity, Subscription};
use yue_component::{
    HourCycle, TimePickerCommitMode as Commit, TimePickerConstraints as Constraints,
    TimePickerEvent, TimePickerOptions as Options, TimePickerSelection as Selection,
    TimePickerSteps as Steps, TimePickerValue as Value, TimePrecision, YueTimePanel, YueTimePicker,
    YueTimePickerState,
};
fn time(h: u32, m: u32, s: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, s).unwrap()
}
pub(crate) struct TimePickerPage {
    states: Vec<Entity<YueTimePickerState>>,
    form: Entity<YueForm>,
    changes: usize,
    last: String,
    draft: String,
    bookings: Vec<(NaiveTime, NaiveTime)>,
    _subscriptions: Vec<Subscription>,
}
impl TimePickerPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let configs = vec![
            Options::new("time-inline").commit_mode(Commit::Immediate),
            Options::new("time-inline-h12")
                .hour_cycle(HourCycle::H12)
                .commit_mode(Commit::Immediate),
            Options::new("time-small"),
            Options::new("time-medium"),
            Options::new("time-large"),
            Options::new("time-readonly"),
            Options::new("time-disabled"),
            Options::new("time-uneditable"),
            Options::new("time-warning"),
            Options::new("time-error"),
            Options::new("time-borderless"),
            Options::new("time-range").selection(Selection::Range),
            Options::new("time-minute").precision(TimePrecision::Minute),
            Options::new("time-h12").hour_cycle(HourCycle::H12),
            Options::new("time-step15")
                .precision(TimePrecision::Minute)
                .steps(Steps::new().minute(15)),
            Options::new("time-step10").steps(Steps::new().second(10)),
            Options::new("time-immediate").commit_mode(Commit::Immediate),
            Options::new("time-limits").constraints(
                Constraints::new()
                    .min(time(9, 0, 0))
                    .max(time(17, 30, 0))
                    .disabled_hours(|h| h == 12),
            ),
            Options::new("time-hidden")
                .constraints(Constraints::new().disabled_minutes(|_, m| m % 5 != 0)),
            Options::new("time-unavailable")
                .constraints(Constraints::new().disabled_hours(|_| true)),
            Options::new("time-controlled"),
            Options::new("time-booking")
                .selection(Selection::Range)
                .precision(TimePrecision::Minute)
                .steps(Steps::new().minute(15))
                .constraints(
                    Constraints::new()
                        .min(time(8, 0, 0))
                        .max(time(20, 0, 0))
                        .range_allowed(|a, b| a < b),
                ),
            Options::new("time-step7")
                .precision(TimePrecision::Minute)
                .steps(Steps::new().minute(7)),
        ];
        let states: Vec<_> = configs
            .into_iter()
            .map(|o| {
                cx.new(|cx| YueTimePickerState::new(o, w, cx).expect("valid showcase time options"))
            })
            .collect();
        for i in [0, 1, 5, 6, 12, 13, 20] {
            states[i].update(cx, |s, cx| {
                s.set_value(Value::Single(time(9, 30, 0)), w, cx).unwrap()
            });
        }
        states[11].update(cx, |s, cx| {
            s.set_value(Value::Range(time(9, 0, 0), time(18, 0, 0)), w, cx)
                .unwrap()
        });
        let form = cx.new(|_| {
            let mut f = YueForm::new();
            f.require_time_picker("reminder", &states[20], "请选择提醒时间");
            f
        });
        let subscriptions = states
            .iter()
            .map(|state| {
                cx.subscribe(state, |s, _, e: &TimePickerEvent, cx| {
                    match e {
                        TimePickerEvent::Change(change) => {
                            s.changes += 1;
                            s.last = format!("{:?}", change.value());
                        }
                        TimePickerEvent::DraftChange(draft) => {
                            s.draft = format!("{:?} — {:?}", draft.start(), draft.end())
                        }
                        _ => {}
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            states,
            form,
            changes: 0,
            last: "尚未提交时间".into(),
            draft: "—".into(),
            bookings: vec![
                (time(9, 0, 0), time(10, 30, 0)),
                (time(14, 0, 0), time(15, 30, 0)),
            ],
            _subscriptions: subscriptions,
        }
    }
}
fn row() -> gpui_kit::Div {
    div().flex().flex_wrap().items_start().gap_4()
}
fn field(label: &'static str, e: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .min_w_0()
        .child(label)
        .child(e)
}
impl Render for TimePickerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let panels = row()
            .child(field("时分秒，即时", YueTimePanel::new(&self.states[0])))
            .child(field("12 小时制，即时", YueTimePanel::new(&self.states[1])));
        let fields = row()
            .child(field(
                "小号",
                YueTimePicker::new(&self.states[2]).size(InputSize::Small),
            ))
            .child(field(
                "默认",
                YueTimePicker::new(&self.states[3]).leading_icon(outline::Clock),
            ))
            .child(field(
                "大号",
                YueTimePicker::new(&self.states[4]).size(InputSize::Large),
            ))
            .child(field(
                "只读",
                YueTimePicker::new(&self.states[5]).readonly(true),
            ))
            .child(field(
                "禁用",
                YueTimePicker::new(&self.states[6]).disabled(true),
            ))
            .child(field(
                "仅面板选择",
                YueTimePicker::new(&self.states[7]).editable(false),
            ))
            .child(field(
                "警告",
                YueTimePicker::new(&self.states[8]).status(InputStatus::Warning),
            ))
            .child(field(
                "错误",
                YueTimePicker::new(&self.states[9]).status(InputStatus::Error),
            ))
            .child(field(
                "无边框",
                YueTimePicker::new(&self.states[10]).bordered(false),
            ));
        let formats = row()
            .child(field("分钟", YueTimePicker::new(&self.states[12])))
            .child(field("12 小时制", YueTimePicker::new(&self.states[13])))
            .child(field("15 分钟刻度", YueTimePicker::new(&self.states[14])))
            .child(field("10 秒刻度", YueTimePicker::new(&self.states[15])))
            .child(field("7 分钟刻度", YueTimePicker::new(&self.states[22])));
        let limits = row()
            .child(field(
                "9:00 至 17:30，午休禁用",
                YueTimePicker::new(&self.states[17]),
            ))
            .child(field(
                "隐藏禁用分钟",
                YueTimePicker::new(&self.states[18]).hide_disabled(true),
            ))
            .child(field("没有可选时间", YueTimePicker::new(&self.states[19])));
        let controlled = row()
            .child(
                YueField::new(theme)
                    .label("提醒时间")
                    .required(true)
                    .bind(&self.form, "reminder")
                    .time_picker(YueTimePicker::new(&self.states[20])),
            )
            .child(
                YueButton::new("time-external", "外部赋值 08:30").on_click(cx.listener(
                    |s, _, w, cx| {
                        s.states[20].update(cx, |s, cx| {
                            s.set_value(Value::Single(time(8, 30, 0)), w, cx).unwrap()
                        });
                    },
                )),
            )
            .child(
                YueButton::new("time-reset", "重置").on_click(cx.listener(|s, _, w, cx| {
                    s.states[20].update(cx, |s, cx| s.set_value(Value::Empty, w, cx).unwrap());
                })),
            )
            .child(
                YueButton::new("time-save", "保存提醒").on_click(cx.listener(|s, _, w, cx| {
                    if s.form.update(cx, |f, cx| f.submit(w, cx)) {
                        s.last = format!("已保存提醒 {:?}", s.states[20].read(cx).value());
                        cx.notify();
                    }
                })),
            );
        let value = self.states[21].read(cx).value().clone();
        let selected = if let Value::Range(a, b) = value {
            Some((a, b))
        } else {
            None
        };
        let conflict =
            selected.is_some_and(|(a, b)| self.bookings.iter().any(|(x, y)| a < *y && b > *x));
        let booking = row()
            .child(
                YueTimePicker::new(&self.states[21])
                    .footer(|_, _, _| div().text_sm().child("营业时间 08:00–20:00，15 分钟刻度")),
            )
            .child(div().text_sm().child(match selected {
                Some((a, b)) => format!(
                    "已提交 {}–{} · {} 分钟{}",
                    a.format("%H:%M"),
                    b.format("%H:%M"),
                    (b - a).num_minutes(),
                    if conflict {
                        "，与已有预约冲突"
                    } else {
                        ""
                    }
                ),
                None => "请选择并确认预约时段".into(),
            }))
            .child(
                YueButton::new("time-book", "预约")
                    .disabled(selected.is_none() || conflict)
                    .on_click(cx.listener(|s, _, w, cx| {
                        let Value::Range(a, b) = s.states[21].read(cx).value().clone() else {
                            return;
                        };
                        if a >= b || s.bookings.iter().any(|(x, y)| a < *y && b > *x) {
                            return;
                        }
                        s.bookings.push((a, b));
                        s.states[21].update(cx, |s, cx| s.set_value(Value::Empty, w, cx).unwrap());
                        cx.notify();
                    })),
            );
        let mut timeline = div()
            .flex()
            .w_full()
            .h_12()
            .relative()
            .rounded_md()
            .bg(gpui_kit::rgb(theme.page()));
        for (a, b) in &self.bookings {
            let left = (a.num_seconds_from_midnight() as f32 / 60. - 480.) / 720.;
            let width = (*b - *a).num_minutes() as f32 / 720.;
            timeline = timeline.child(
                div()
                    .id(format!("time-booked-{a}-{b}"))
                    .absolute()
                    .top_1()
                    .bottom_1()
                    .left(gpui_kit::relative(left))
                    .w(gpui_kit::relative(width))
                    .rounded_sm()
                    .bg(gpui_kit::rgb(theme.accent_subtle()))
                    .text_color(gpui_kit::rgb(theme.accent_subtle_text()))
                    .overflow_hidden()
                    .text_xs()
                    .child(format!("{}–{}", a.format("%H:%M"), b.format("%H:%M"))),
            );
        }
        div()
            .id("time-picker-content")
            .test_support()
            .debug_selector(|| "time-picker-content".into())
            .flex()
            .flex_col()
            .gap_6()
            .min_w_0()
            .child(div().text_2xl().child("时间选择 TimePicker"))
            .child(readout(
                theme,
                format!(
                    "已提交 {} 次 · {}；当前候选 {}",
                    self.changes, self.last, self.draft
                ),
            ))
            .child(section(
                theme,
                "独立面板",
                "每列单独滚动；方向键选择，左右切列，Home / End 跳转。",
                panels,
            ))
            .child(section(
                theme,
                "字段与状态",
                "真实输入、三尺寸及只读禁用。Alt+Down 打开；Enter 接受，Escape 取消。",
                fields,
            ))
            .child(section(
                theme,
                "格式与刻度",
                "分钟、秒和 AM / PM；手动输入、此刻与程序赋值遵守相同刻度。",
                formats,
            ))
            .child(section(
                theme,
                "范围与即时提交",
                "范围默认确认；开始越过结束时联动到合法结束。窄窗口切换活动端。",
                row()
                    .child(YueTimePicker::new(&self.states[11]))
                    .child(YueTimePicker::new(&self.states[16])),
            ))
            .child(section(
                theme,
                "时间限制",
                "边界与联动禁用；没有合法时间时不能确认。",
                limits,
            ))
            .child(section(
                theme,
                "外部同步与表单",
                "外部赋值不增加用户提交次数；保存仅读已提交值。",
                controlled,
            ))
            .child(section(
                theme,
                "会议室预约",
                "草稿仅预览；确认时间后才可预约。冲突检测和已预约时段由宿主管理。",
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(booking)
                    .child(timeline),
            ))
    }
}
