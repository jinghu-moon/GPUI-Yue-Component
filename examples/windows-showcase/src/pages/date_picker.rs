use super::super::*;
use chrono::{Datelike, Days, Local, NaiveDate};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{Entity, Subscription};
use yue_component::{
    DatePickerCommitMode as CommitMode, DatePickerConstraints as Constraints, DatePickerEndpoint,
    DatePickerEvent, DatePickerGranularity as Granularity, DatePickerOptions as Options,
    DatePickerPreset, DatePickerSelection as Selection, DatePickerValue as Value, TagTone,
    TimePrecision, YueCalendar, YueDatePicker, YueDatePickerState, YueTag,
};

pub(crate) struct DatePickerPage {
    calendar: Entity<YueDatePickerState>,
    sunday: Entity<YueDatePickerState>,
    dates: Vec<Entity<YueDatePickerState>>,
    range: Entity<YueDatePickerState>,
    locked: Entity<YueDatePickerState>,
    minute: Entity<YueDatePickerState>,
    seconds: Entity<YueDatePickerState>,
    datetime_range: Entity<YueDatePickerState>,
    units: Vec<Entity<YueDatePickerState>>,
    multiple: Entity<YueDatePickerState>,
    multiple_immediate: Entity<YueDatePickerState>,
    limited: Entity<YueDatePickerState>,
    controlled: Entity<YueDatePickerState>,
    form: Entity<YueForm>,
    hotel: Entity<YueDatePickerState>,
    last: String,
    changes: usize,
    _subscriptions: Vec<Subscription>,
}
impl DatePickerPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut all = Vec::new();
        let mut create = |options: Options, window: &mut Window, cx: &mut Context<Self>| {
            let state = cx.new(|cx| {
                YueDatePickerState::new(options, window, cx).expect("valid showcase date options")
            });
            all.push(state.clone());
            state
        };
        let calendar = create(Options::new("date-inline"), window, cx);
        let sunday = create(
            Options::new("date-sunday").week_start(chrono::Weekday::Sun),
            window,
            cx,
        );
        let dates = (0..10)
            .map(|i| {
                create(
                    Options::new(format!("date-{i}")).format(if i == 3 {
                        "%Y/%m/%d"
                    } else {
                        "%Y-%m-%d"
                    }),
                    window,
                    cx,
                )
            })
            .collect();
        let range = create(
            Options::new("date-range").selection(Selection::Range),
            window,
            cx,
        );
        let locked = create(
            Options::new("date-locked")
                .selection(Selection::Range)
                .locked_endpoint(DatePickerEndpoint::Start, true),
            window,
            cx,
        );
        let minute = create(
            Options::new("date-minute").time_precision(TimePrecision::Minute),
            window,
            cx,
        );
        let seconds = create(
            Options::new("date-seconds").time_precision(TimePrecision::Second),
            window,
            cx,
        );
        let datetime_range = create(
            Options::new("date-time-range")
                .selection(Selection::Range)
                .time_precision(TimePrecision::Minute),
            window,
            cx,
        );
        let mut units = Vec::new();
        for granularity in [
            Granularity::Week,
            Granularity::Month,
            Granularity::Quarter,
            Granularity::Year,
        ] {
            for selection in [Selection::Single, Selection::Range] {
                units.push(create(
                    Options::new(format!("date-{granularity:?}-{selection:?}"))
                        .granularity(granularity)
                        .selection(selection),
                    window,
                    cx,
                ));
            }
        }
        let multiple = create(
            Options::new("date-multiple")
                .selection(Selection::Multiple)
                .constraints(Constraints::new().max_selected(8)),
            window,
            cx,
        );
        let multiple_immediate = create(
            Options::new("date-multiple-immediate")
                .selection(Selection::Multiple)
                .commit_mode(CommitMode::Immediate),
            window,
            cx,
        );
        let today = Local::now().date_naive();
        let limited = create(
            Options::new("date-limited").constraints(
                Constraints::new()
                    .min(today)
                    .allowed(|date| date.weekday().number_from_monday() <= 5),
            ),
            window,
            cx,
        );
        let controlled = create(Options::new("date-controlled"), window, cx);
        let hotel = create(
            Options::new("date-hotel")
                .selection(Selection::Range)
                .constraints(
                    Constraints::new()
                        .min(today)
                        .max_span_days(14)
                        .range_allowed(|a, b| a < b),
                ),
            window,
            cx,
        );
        locked.update(cx, |s, cx| {
            s.set_value(
                Value::date_range(today, today.checked_add_days(Days::new(3)).unwrap_or(today)),
                window,
                cx,
            )
            .unwrap()
        });
        let form = cx.new(|_| {
            let mut form = YueForm::new();
            form.require_date_picker("start", &controlled, "请选择开始日期");
            form
        });
        let mut subscriptions: Vec<_> = all
            .iter()
            .map(|entity| {
                cx.subscribe(entity, |s, _, event: &DatePickerEvent, cx| {
                    if let DatePickerEvent::Change(change) = event {
                        s.changes += 1;
                        s.last = format!("{:?} · {:?}", change.source(), change.value());
                        cx.notify();
                    }
                })
            })
            .collect();
        subscriptions.push(cx.observe(&form, |_, _, cx| cx.notify()));
        Self {
            calendar,
            sunday,
            dates,
            range,
            locked,
            minute,
            seconds,
            datetime_range,
            units,
            multiple,
            multiple_immediate,
            limited,
            controlled,
            form,
            hotel,
            last: "尚未提交日期".into(),
            changes: 0,
            _subscriptions: subscriptions,
        }
    }
}
fn row() -> gpui_kit::Div {
    div().w_full().flex().flex_wrap().items_start().gap_4()
}
fn field(label: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .min_w_0()
        .child(label)
        .child(content)
}
impl Render for DatePickerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let calendars = row()
            .child(field("周一开始", YueCalendar::new(&self.calendar)))
            .child(field(
                "周日开始",
                YueCalendar::new(&self.sunday).size(InputSize::Small),
            ));
        let basic = row()
            .child(field(
                "小号",
                YueDatePicker::new(&self.dates[0]).size(InputSize::Small),
            ))
            .child(field(
                "默认",
                YueDatePicker::new(&self.dates[1]).leading_icon(outline::Calendar),
            ))
            .child(field(
                "大号",
                YueDatePicker::new(&self.dates[2]).size(InputSize::Large),
            ))
            .child(field("斜杠格式", YueDatePicker::new(&self.dates[3])))
            .child(field(
                "不可输入，仍可选择",
                YueDatePicker::new(&self.dates[4]).editable(false),
            ))
            .child(field(
                "只读",
                YueDatePicker::new(&self.dates[5]).readonly(true),
            ))
            .child(field(
                "禁用",
                YueDatePicker::new(&self.dates[6]).disabled(true),
            ))
            .child(field(
                "无边框",
                YueDatePicker::new(&self.dates[7]).bordered(false),
            ))
            .child(field(
                "警告",
                YueDatePicker::new(&self.dates[8]).status(InputStatus::Warning),
            ))
            .child(field(
                "错误",
                YueDatePicker::new(&self.dates[9]).status(InputStatus::Error),
            ));
        let ranges = row()
            .child(field("日期范围", YueDatePicker::new(&self.range)))
            .child(field("开始日期锁定", YueDatePicker::new(&self.locked)));
        let times = row()
            .child(field("时分，确认提交", YueDatePicker::new(&self.minute)))
            .child(field("时分秒", YueDatePicker::new(&self.seconds)))
            .child(field(
                "日期时间范围",
                YueDatePicker::new(&self.datetime_range),
            ));
        let labels = [
            "ISO 周",
            "ISO 周范围",
            "月份",
            "月份范围",
            "季度",
            "季度范围",
            "年份",
            "年份范围",
        ];
        let mut units = row();
        for (index, state) in self.units.iter().enumerate() {
            units = units.child(field(labels[index], YueDatePicker::new(state)));
        }
        let multi = row()
            .child(field(
                "最多八天，完成后提交",
                YueDatePicker::new(&self.multiple),
            ))
            .child(field(
                "即时提交，继续选择",
                YueDatePicker::new(&self.multiple_immediate),
            ));
        let limit = row().child(YueDatePicker::new(&self.limited).presets([
            DatePickerPreset::new("today", "今天", |now| Value::date(now.date())),
            DatePickerPreset::new("tomorrow", "明天", |now| {
                Value::date(
                    now.date()
                        .checked_add_days(Days::new(1))
                        .unwrap_or(now.date()),
                )
            }),
            DatePickerPreset::new("past", "昨天（应拒绝）", |now| {
                Value::date(
                    now.date()
                        .checked_sub_days(Days::new(1))
                        .unwrap_or(now.date()),
                )
            }),
        ]));
        let controlled = row()
            .child(
                YueField::new(theme)
                    .label("计划开始日期")
                    .required(true)
                    .bind(&self.form, "start")
                    .date_picker(YueDatePicker::new(&self.controlled)),
            )
            .child(
                YueButton::new("date-save", "保存计划").on_click(cx.listener(|s, _, w, cx| {
                    if s.form.update(cx, |form, cx| form.submit(w, cx)) {
                        s.last = format!("已保存计划开始日期：{:?}", s.controlled.read(cx).value());
                        cx.notify();
                    }
                })),
            )
            .child(
                YueButton::new("date-external-today", "外部设为今天").on_click(cx.listener(
                    |s, _, w, cx| {
                        s.controlled.update(cx, |s, cx| {
                            s.set_value(Value::date(Local::now().date_naive()), w, cx)
                                .unwrap()
                        });
                    },
                )),
            )
            .child(
                YueButton::new("date-external-reset", "外部重置").on_click(cx.listener(
                    |s, _, w, cx| {
                        s.controlled
                            .update(cx, |s, cx| s.set_value(Value::Empty, w, cx).unwrap());
                    },
                )),
            );
        let nights = match self.hotel.read(cx).value() {
            Value::Range(a, b) => (b.start_date() - a.start_date()).num_days(),
            _ => 0,
        };
        let hotel = row()
            .child(
                YueDatePicker::new(&self.hotel)
                    .cell_height(gpui_kit::rems(3.))
                    .render_day(move |item, st, _, _| {
                        if let Some(date) = st.date() {
                            let price = price(date);
                            item.child(
                                div()
                                    .text_xs()
                                    .text_color(gpui_kit::rgb(theme.text_secondary()))
                                    .child(format!("¥{price}")),
                            )
                            .flex_col()
                            .into_any_element()
                        } else {
                            item.into_any_element()
                        }
                    })
                    .footer(move |_, _, _| {
                        div().text_sm().child("最多 14 晚；价格和晚数由宿主计算")
                    }),
            )
            .child(YueTag::new("hotel-nights", format!("{nights} 晚")).tone(TagTone::Default));
        div().id("date-picker-content").test_support()
            .debug_selector(|| "date-picker-content".into())
            .flex().flex_col().gap_6().min_w_0()
            .child(div().text_2xl().child("日期选择 DatePicker"))
            .child(readout(theme,format!("已提交 {} 次 · {}",self.changes,self.last)))
            .child(section(theme,"独立日历","固定六周；点击月、年标题切换视图。方向键移动，PageUp / PageDown 翻月，Enter 选择。",calendars))
            .child(section(theme,"字段与状态","真实文本输入；非法文本保留，Enter 或离开控件时校验。",basic))
            .child(section(theme,"范围","连续双月共享选择；反向点击自动排序；窄窗口自动退为单月。",ranges))
            .child(section(theme,"日期时间","复用 Base TimeField 分段编辑。点击确定提交，Escape 丢弃未确认草稿。",times))
            .child(section(theme,"周、月、季、年","独立类型值，ISO 周包含周年。每个粒度同时展示单值和范围。",units))
            .child(section(theme,"多选","两个标签后折叠；+N 可以展开并移除；面板内也能查看全部日期。",multi))
            .child(section(theme,"限制与快捷项","只允许今天起的工作日；预设与键盘输入使用相同校验。",limit))
            .child(section(theme,"外部同步","程序赋值和重置不增加上方用户提交次数；已打开的旧草稿同时失效。",controlled))
            .child(section(theme,"酒店预订组合","价格为本地示例数据，选中范围表示入住／离店端点；总晚数由宿主计算。",hotel))
    }
}
fn price(date: NaiveDate) -> u32 {
    320 + (date.day() % 7) * 20
}
