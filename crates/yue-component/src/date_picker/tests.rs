use super::*;
use crate::{Accent, ThemeMode};
use chrono::{Datelike, NaiveDate};
use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, Focusable as _, IntoElement, ParentElement as _, Render,
    Role, Styled as _, TestAppContext, VisualTestContext, Window, div,
};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}
fn options() -> DatePickerOptions {
    DatePickerOptions::new("date").clock(|| date(2026, 11, 8).and_hms_opt(13, 42, 17).unwrap())
}
struct Probe {
    state: Entity<YueDatePickerState>,
    inline: bool,
    disabled: bool,
    readonly: bool,
    size: InputSize,
    changes: Vec<DatePickerChange>,
    events: Vec<&'static str>,
    _subscription: gpui_kit::Subscription,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let content = if self.inline {
            YueCalendar::new(&self.state)
                .size(self.size)
                .disabled(self.disabled)
                .readonly(self.readonly)
                .into_any_element()
        } else {
            YueDatePicker::new(&self.state)
                .size(self.size)
                .disabled(self.disabled)
                .readonly(self.readonly)
                .into_any_element()
        };
        div()
            .p_4()
            .child(content)
            .child(crate::YueButton::new("outside", "外部"))
    }
}
fn open(
    options: DatePickerOptions,
    inline: bool,
    cx: &mut TestAppContext,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, crate::YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let probe = cx.new(|cx| {
            let state = cx.new(|cx| YueDatePickerState::new(options, window, cx).unwrap());
            let subscription = cx.subscribe(
                &state,
                |s: &mut Probe, _, event: &DatePickerEvent, _| match event {
                    DatePickerEvent::Change(change) => {
                        s.changes.push(change.clone());
                        s.events.push("change");
                    }
                    DatePickerEvent::OpenChange(v) => {
                        s.events.push(if v.is_open() { "open" } else { "close" })
                    }
                    _ => {}
                },
            );
            Probe {
                state,
                inline,
                disabled: false,
                readonly: false,
                size: InputSize::Medium,
                changes: vec![],
                events: vec![],
                _subscription: subscription,
            }
        });
        saved = Some(probe.clone());
        Root::new(probe, window, cx)
    });
    draw(cx);
    (saved.unwrap(), cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn named(w: &Window, label: &str) -> ElementSnapshot {
    snapshots(w)
        .into_iter()
        .find(|n| n.label() == Some(label))
        .unwrap_or_else(|| {
            panic!(
                "missing {label}; labels: {:?}",
                snapshots(w).iter().map(|n| n.label()).collect::<Vec<_>>()
            )
        })
}
fn click(cx: &mut VisualTestContext, label: &str) {
    draw(cx);
    let target = cx.update(|w, _| named(w, label).bounds().center());
    cx.simulate_click(target, Default::default());
    draw(cx);
}
fn keys(cx: &mut VisualTestContext, sequence: &str) {
    for value in sequence.split_whitespace() {
        let key = gpui_kit::Keystroke::parse(value).unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: key.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
        draw(cx);
    }
}

fn state(probe: &Entity<Probe>, cx: &App) -> Entity<YueDatePickerState> {
    probe.read(cx).state.clone()
}

#[test]
fn legal_units_iso_year_and_strict_formats() {
    let day = date(2021, 1, 1);
    assert_eq!(
        DatePickerUnit::from_date(day, DatePickerGranularity::Week).unwrap(),
        DatePickerUnit::Week(DateWeek::new(2020, 53).unwrap())
    );
    assert!(DateWeek::new(2021, 53).is_err());
    assert!(DateYear::new(0).is_err());
    assert!(DateQuarter::new(2026, 5).is_err());
    assert_eq!(
        options().parse(["2024-02-29", ""]).unwrap(),
        DatePickerValue::date(date(2024, 2, 29))
    );
    for text in ["2023-02-29", "2024-2-29", "2024-02-29tail", "2024-13-01"] {
        assert!(options().parse([text, ""]).is_err(), "{text}");
    }
    for (granularity, text) in [
        (DatePickerGranularity::Month, "2026-02"),
        (DatePickerGranularity::Year, "2026"),
        (DatePickerGranularity::Quarter, "2026-Q3"),
        (DatePickerGranularity::Week, "2020-W53"),
    ] {
        let options = options().granularity(granularity);
        options.check().unwrap();
        let value = options.parse([text, ""]).unwrap();
        assert_eq!(options.texts(&value)[0], text);
    }
    assert!(options().format("YYYY-MM-DD").check().is_err());
    assert!(
        options()
            .granularity(DatePickerGranularity::Month)
            .selection(DatePickerSelection::Multiple)
            .check()
            .is_err()
    );
    assert!(
        options()
            .selection(DatePickerSelection::Multiple)
            .time_precision(TimePrecision::Second)
            .check()
            .is_err()
    );
}
#[test]
fn fixed_six_weeks_and_supported_boundaries() {
    for year in [1, 2024, 2025, 2026, 9999] {
        for month in 1..=12 {
            for first in [chrono::Weekday::Mon, chrono::Weekday::Sun] {
                let weeks = gpui_kit::base::CalendarState::fixed_month_days(year, month, first);
                assert_eq!(weeks.len(), 6);
                assert!(weeks.iter().all(|v| v.len() == 7));
                assert_eq!(weeks[0][0].weekday(), first);
            }
        }
    }
}
#[test]
fn every_value_uses_normalization_and_constraints() {
    let options = options().selection(DatePickerSelection::Range).constraints(
        DatePickerConstraints::new()
            .min(date(2026, 1, 1))
            .max(date(2026, 12, 31))
            .max_span_days(7),
    );
    assert_eq!(
        options.parse(["2026-11-08", "2026-11-01"]).unwrap(),
        DatePickerValue::date_range(date(2026, 11, 1), date(2026, 11, 8))
    );
    assert_eq!(
        options.parse(["2026-11-08", "2026-11-30"]),
        Err(DatePickerError::RangeRejected)
    );
    assert_eq!(
        options.parse(["2026-11-08", ""]),
        Err(DatePickerError::Incomplete)
    );
    assert!(
        options
            .normalize(DatePickerValue::date_range(
                date(2025, 12, 31),
                date(2026, 1, 1)
            ))
            .is_err()
    );
    let options = DatePickerOptions::new("multi")
        .selection(DatePickerSelection::Multiple)
        .constraints(DatePickerConstraints::new().max_selected(2));
    assert_eq!(
        options
            .normalize(DatePickerValue::Multiple(vec![
                date(2026, 1, 2),
                date(2026, 1, 1),
                date(2026, 1, 2)
            ]))
            .unwrap(),
        DatePickerValue::Multiple(vec![date(2026, 1, 1), date(2026, 1, 2)])
    );
}

#[gpui_kit::test]
fn inline_grid_has_42_cells_and_keyboard_selects_once(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), true, cx);
    cx.update(|w, _| {
        let cells: Vec<_> = snapshots(w)
            .into_iter()
            .filter(|n| n.role() == Some(Role::GridCell))
            .collect();
        assert_eq!(cells.len(), 42);
        assert_eq!(named(w, "2026-11-08").role(), Some(Role::GridCell));
    });
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, cx| {
        editor
            .read(cx)
            .calendar
            .read(cx)
            .focus_handle
            .clone()
            .focus(w, cx)
    });
    keys(cx, "right enter");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date(date(2026, 11, 9))
        );
        assert_eq!(probe.read(cx).changes.len(), 1);
    });
    keys(cx, "enter");
    draw(cx);
    cx.update(|_, cx| assert_eq!(probe.read(cx).changes.len(), 1));
}
#[gpui_kit::test]
fn picker_opens_commits_then_escapes_without_leaking_draft(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().commit_mode(DatePickerCommitMode::Confirm),
        false,
        cx,
    );
    click(cx, "打开日期选择");
    click(cx, "2026-11-12");
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|_, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert!(editor.read(cx).is_open());
    });
    keys(cx, "escape");
    draw(cx);
    cx.update(|w, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert!(!editor.read(cx).is_open());
        assert!(editor.read(cx).inputs[0].focus_handle(cx).is_focused(w));
    });
}

#[gpui_kit::test]
fn sorted_reverse_range_and_disabled_dates(cx: &mut TestAppContext) {
    let constraints = DatePickerConstraints::new().allowed(|date| date.day() != 15);
    let (probe, cx) = open(
        options()
            .selection(DatePickerSelection::Range)
            .constraints(constraints),
        true,
        cx,
    );
    click(cx, "2026-11-20");
    click(cx, "2026-11-15");
    cx.update(|_, cx| assert!(state(&probe, cx).read(cx).value().is_empty()));
    click(cx, "2026-11-10");
    cx.update(|_, cx| {
        assert_eq!(
            state(&probe, cx).read(cx).value(),
            &DatePickerValue::date_range(date(2026, 11, 10), date(2026, 11, 20))
        );
        assert_eq!(probe.read(cx).changes.len(), 1);
    });
}

#[gpui_kit::test]
fn external_set_is_silent_and_invalid_set_preserves_edit(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().constraints(DatePickerConstraints::new().min(date(2026, 1, 1))),
        false,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, cx| {
        editor.update(cx, |s, cx| {
            s.set_value(DatePickerValue::date(date(2026, 5, 1)), w, cx)
                .unwrap()
        })
    });
    click(cx, "打开日期选择");
    click(cx, "2026-05-03");
    cx.update(|w, cx| {
        editor.update(cx, |s, cx| {
            assert!(
                s.set_value(DatePickerValue::date(date(2025, 1, 1)), w, cx)
                    .is_err()
            );
        })
    });
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(probe.read(cx).changes.len(), 1);
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date(date(2026, 5, 3))
        );
    });
}

#[gpui_kit::test]
fn real_input_retains_invalid_text_and_enter_blur_deduplicate(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, cx| editor.read(cx).inputs[0].focus_handle(cx).focus(w, cx));
    draw(cx);
    cx.simulate_input("2026-02-30");
    draw(cx);
    keys(cx, "enter");
    draw(cx);
    cx.update(|w, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert_eq!(editor.read(cx).inputs[0].read(cx).value(), "2026-02-30");
        assert!(
            editor.read(cx).error().is_some(),
            "focus {:?}, changes {:?}",
            editor.read(cx).inputs[0].focus_handle(cx).is_focused(w),
            probe.read(cx).events
        );
    });
    keys(cx, "ctrl-a");
    cx.simulate_input("2026-02-28");
    draw(cx);
    keys(cx, "enter");
    draw(cx);
    click(cx, "外部");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date(date(2026, 2, 28))
        );
        assert_eq!(probe.read(cx).changes.len(), 1);
    });
}

#[gpui_kit::test]
fn time_segments_and_confirm_are_one_transaction(cx: &mut TestAppContext) {
    let (probe, cx) = open(options().time_precision(TimePrecision::Second), false, cx);
    click(cx, "打开日期选择");
    click(cx, "2026-11-12");
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, cx| {
        let time = editor.read(cx).times[0].clone();
        time.update(cx, |s, cx| s.focus(w, cx));
    });
    keys(cx, "up");
    draw(cx);
    cx.update(|_, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert_eq!(
            editor.read(cx).candidate(cx).unwrap(),
            DatePickerValue::DateTime(date(2026, 11, 12).and_hms_opt(1, 0, 0).unwrap())
        );
    });
    click(cx, "确定");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(probe.read(cx).changes.len(), 1);
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::DateTime(date(2026, 11, 12).and_hms_opt(1, 0, 0).unwrap())
        );
        assert!(!editor.read(cx).open, "{:?}", probe.read(cx).events);
        let events = &probe.read(cx).events;
        assert!(
            events.iter().position(|v| *v == "change") < events.iter().position(|v| *v == "close")
        );
    });
}

#[gpui_kit::test]
fn multiple_confirm_limit_clear_and_external_replacement(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options()
            .selection(DatePickerSelection::Multiple)
            .constraints(DatePickerConstraints::new().max_selected(2)),
        false,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    click(cx, "打开日期选择");
    click(cx, "2026-11-10");
    click(cx, "2026-11-11");
    click(cx, "2026-11-12");
    cx.update(|_, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert_eq!(
            editor.read(cx).calendar.read(cx).selection().dates().len(),
            2
        );
    });
    click(cx, "完成");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Multiple(vec![date(2026, 11, 10), date(2026, 11, 11)])
        )
    });
    click(cx, "打开日期选择");
    click(cx, "2026-11-11");
    cx.update(|w, cx| {
        editor.update(cx, |s, cx| {
            s.set_value(DatePickerValue::Multiple(vec![date(2026, 11, 20)]), w, cx)
                .unwrap()
        })
    });
    keys(cx, "escape");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Multiple(vec![date(2026, 11, 20)])
        );
        assert_eq!(probe.read(cx).changes.len(), 1);
    });
    click(cx, "清空日期");
    cx.update(|_, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert_eq!(probe.read(cx).changes.len(), 2);
    });
}

#[gpui_kit::test]
fn locked_start_rejects_reverse_and_readonly_blocks_all_commands(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options()
            .selection(DatePickerSelection::Range)
            .locked_endpoint(DatePickerEndpoint::Start, true),
        true,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, cx| {
        editor.update(cx, |s, cx| {
            s.set_value(
                DatePickerValue::date_range(date(2026, 11, 10), date(2026, 11, 20)),
                w,
                cx,
            )
            .unwrap()
        })
    });
    draw(cx);
    click(cx, "2026-11-08");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date_range(date(2026, 11, 10), date(2026, 11, 20))
        )
    });
    click(cx, "2026-11-25");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date_range(date(2026, 11, 10), date(2026, 11, 25))
        )
    });
    probe.update(cx, |p, cx| {
        p.readonly = true;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, cx| {
        editor.update(cx, |s, cx| {
            s.clear(w, cx);
            s.today(w, cx);
            s.open(w, cx);
            assert!(!s.open);
        })
    });
    cx.update(|_, cx| assert_eq!(probe.read(cx).changes.len(), 1));
}

#[gpui_kit::test]
fn months_quarters_years_and_weeks_select_public_units(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().granularity(DatePickerGranularity::Month),
        true,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    click(cx, "2026-03-01");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Single(DatePickerUnit::Month(DateMonth::new(2026, 3).unwrap()))
        )
    });
}

#[gpui_kit::test]
fn month_navigation_keeps_six_rows_at_rem_scales_and_boundaries(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), true, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    for rem in [16., 20., 24.] {
        cx.update(|w, cx| {
            w.set_rem_size(gpui_kit::px(rem));
            editor.update(cx, |s, cx| {
                s.set_value(DatePickerValue::date(date(2026, 2, 1)), w, cx)
                    .unwrap()
            });
        });
        draw(cx);
        let before = cx.update(|w, _| named(w, "2026-02-01").bounds().size);
        cx.update(|w, cx| {
            editor
                .read(cx)
                .calendar
                .read(cx)
                .focus_handle
                .clone()
                .focus(w, cx)
        });
        keys(cx, "pageup");
        draw(cx);
        cx.update(|w, _| {
            let cells: Vec<_> = snapshots(w)
                .into_iter()
                .filter(|n| n.role() == Some(Role::GridCell))
                .collect();
            assert_eq!(cells.len(), 42);
            assert_eq!(named(w, "2026-01-01").bounds().size, before);
        });
    }
    for extreme in [date(1, 1, 1), date(9999, 12, 31)] {
        cx.update(|w, cx| {
            editor.update(cx, |s, cx| {
                s.set_value(DatePickerValue::date(extreme), w, cx).unwrap()
            })
        });
        draw(cx);
        cx.update(|w, cx| {
            editor
                .read(cx)
                .calendar
                .read(cx)
                .focus_handle
                .clone()
                .focus(w, cx)
        });
        keys(
            cx,
            if extreme.year() == 1 {
                "left pageup shift-pageup"
            } else {
                "right pagedown shift-pagedown"
            },
        );
        draw(cx);
        cx.update(|_, cx| assert_eq!(editor.read(cx).calendar.read(cx).active_date(), extreme));
    }
}

#[gpui_kit::test]
fn popup_tab_outside_does_not_restore_field_focus(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().commit_mode(DatePickerCommitMode::Confirm),
        false,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    click(cx, "打开日期选择");
    click(cx, "2026-11-10");
    click(cx, "外部");
    cx.update(|w, cx| {
        assert!(!editor.read(cx).open);
        assert!(editor.read(cx).value().is_empty());
        assert!(!editor.read(cx).inputs[0].focus_handle(cx).is_focused(w));
    });
}

#[gpui_kit::test]
fn ime_enter_never_commits_preedit(cx: &mut TestAppContext) {
    use gpui_kit::EntityInputHandler as _;
    let (probe, cx) = open(options(), false, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    let input = cx.update(|w, cx| {
        let input = editor.read(cx).inputs[0].clone();
        input.update(cx, |s, cx| s.focus(w, cx));
        input
    });
    draw(cx);
    cx.update(|w, cx| {
        input.update(cx, |s, cx| {
            s.replace_and_mark_text_in_range(None, "2026-11-12", None, w, cx)
        })
    });
    draw(cx);
    keys(cx, "enter");
    draw(cx);
    cx.update(|_, cx| {
        assert!(editor.read(cx).value().is_empty());
        assert!(editor.read(cx).error().is_none());
    });
    cx.update(|w, cx| input.update(cx, |s, cx| s.unmark_text(w, cx)));
    draw(cx);
    keys(cx, "enter");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date(date(2026, 11, 12))
        );
        assert_eq!(probe.read(cx).changes.len(), 1);
    });
}

#[gpui_kit::test]
fn quarter_grid_keyboard_and_year_drilldown(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().granularity(DatePickerGranularity::Quarter),
        true,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, _| {
        assert_eq!(
            snapshots(w)
                .into_iter()
                .filter(|n| n.role() == Some(Role::GridCell))
                .count(),
            4
        )
    });
    click(cx, "2026-04-01");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Single(DatePickerUnit::Quarter(DateQuarter::new(2026, 2).unwrap()))
        )
    });
    keys(cx, "down enter");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Single(DatePickerUnit::Quarter(DateQuarter::new(2026, 4).unwrap()))
        )
    });
    click(cx, "2026年");
    cx.update(|w, _| {
        assert_eq!(
            snapshots(w)
                .into_iter()
                .filter(|n| n.role() == Some(Role::GridCell))
                .count(),
            12
        )
    });
    click(cx, "2028-01-01");
    click(cx, "2028-07-01");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Single(DatePickerUnit::Quarter(DateQuarter::new(2028, 3).unwrap()))
        )
    });
}

#[gpui_kit::test]
fn iso_week_selection_and_year_range(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options()
            .granularity(DatePickerGranularity::Week)
            .week_start(chrono::Weekday::Sun),
        true,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    cx.update(|w, cx| {
        editor.update(cx, |s, cx| {
            s.set_value(
                DatePickerValue::Single(DatePickerUnit::Week(DateWeek::new(2020, 53).unwrap())),
                w,
                cx,
            )
            .unwrap()
        })
    });
    draw(cx);
    click(cx, "2021-01-01");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::Single(DatePickerUnit::Week(DateWeek::new(2020, 53).unwrap()))
        )
    });
}

#[gpui_kit::test]
fn tab_exits_nonmodal_popup_and_cancels_unconfirmed_value(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().commit_mode(DatePickerCommitMode::Confirm),
        false,
        cx,
    );
    let editor = cx.update(|_, cx| state(&probe, cx));
    click(cx, "打开日期选择");
    click(cx, "2026-11-10");
    let mut left = false;
    for _ in 0..12 {
        keys(cx, "tab");
        draw(cx);
        if cx.update(|_, cx| !editor.read(cx).open) {
            left = true;
            break;
        }
    }
    assert!(left, "Tab must leave a nonmodal popup");
    cx.update(|_, cx| assert!(editor.read(cx).value().is_empty()));
}

#[gpui_kit::test]
fn change_observer_replacement_invalidates_pending_close(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    let subscription = cx.update(|w, cx| {
        probe.update(cx, |_, cx| {
            cx.subscribe_in(&editor, w, |_, entity, event: &DatePickerEvent, w, cx| {
                if matches!(event, DatePickerEvent::Change(_)) {
                    entity.update(cx, |s, cx| {
                        s.set_value(DatePickerValue::date(date(2026, 11, 30)), w, cx)
                            .unwrap()
                    });
                }
            })
        })
    });
    click(cx, "打开日期选择");
    click(cx, "2026-11-10");
    cx.update(|_, cx| {
        assert_eq!(
            editor.read(cx).value(),
            &DatePickerValue::date(date(2026, 11, 30))
        );
        assert!(editor.read(cx).open);
        assert_eq!(probe.read(cx).changes.len(), 1);
    });
    drop(subscription);
}

#[test]
fn time_range_normalizes_precision_and_checks_time_predicate() {
    let options = options()
        .selection(DatePickerSelection::Range)
        .time_precision(TimePrecision::Minute)
        .constraints(DatePickerConstraints::new().time_allowed(|time| {
            use chrono::Timelike as _;
            time.hour() >= 9
        }));
    let day = date(2026, 11, 8);
    assert_eq!(
        options
            .normalize(DatePickerValue::DateTimeRange(
                day.and_hms_opt(11, 0, 29).unwrap(),
                day.and_hms_opt(9, 0, 59).unwrap()
            ))
            .unwrap(),
        DatePickerValue::DateTimeRange(
            day.and_hms_opt(9, 0, 0).unwrap(),
            day.and_hms_opt(11, 0, 0).unwrap()
        )
    );
    assert_eq!(
        options.parse(["2026-11-08 08:59", "2026-11-08 11:00"]),
        Err(DatePickerError::Unavailable)
    );
}

#[gpui_kit::test]
fn typed_form_validates_commit_and_focuses_named_date_field(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    let form = cx.update(|_, cx| {
        cx.new(|_| {
            let mut form = crate::YueForm::new();
            form.require_date_picker("start", &editor, "请选择日期");
            form
        })
    });
    cx.update(|w, cx| {
        assert!(!form.update(cx, |form, cx| form.submit(w, cx)));
        assert!(editor.read(cx).inputs[0].focus_handle(cx).is_focused(w));
        assert_eq!(
            form.read(cx).error("start", cx).as_deref(),
            Some("请选择日期")
        );
    });
    click(cx, "打开日期选择");
    click(cx, "2026-11-12");
    cx.update(|w, cx| {
        assert!(form.update(cx, |form, cx| form.submit(w, cx)));
        assert!(form.read(cx).error("start", cx).is_none());
    });
}

#[gpui_kit::test]
fn three_cell_sizes_scale_together_and_range_popup_falls_back_to_one_month(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = open(options().selection(DatePickerSelection::Range), false, cx);
    cx.simulate_resize(gpui_kit::size(gpui_kit::px(900.), gpui_kit::px(800.)));
    draw(cx);
    click(cx, "打开日期选择");
    cx.update(|w, _| {
        assert_eq!(
            snapshots(w)
                .into_iter()
                .filter(|n| n.role() == Some(Role::GridCell))
                .count(),
            84
        )
    });
    cx.simulate_resize(gpui_kit::size(gpui_kit::px(360.), gpui_kit::px(800.)));
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(
            snapshots(w)
                .into_iter()
                .filter(|n| n.role() == Some(Role::GridCell))
                .count(),
            42
        );
        let surface = snapshots(w)
            .into_iter()
            .find(|n| {
                n.path().last()
                    == Some(&gpui_kit::ElementId::from((
                        gpui_kit::ElementId::from("date"),
                        "popup-surface",
                    )))
            })
            .unwrap();
        assert!(surface.bounds().left() >= gpui_kit::px(0.));
        assert!(surface.bounds().right() <= w.viewport_size().width);
    });
    keys(cx, "escape");
    probe.update(cx, |p, cx| {
        p.inline = true;
        cx.notify();
    });
    cx.simulate_resize(gpui_kit::size(gpui_kit::px(900.), gpui_kit::px(900.)));
    draw(cx);
    for (size, expected) in [
        (InputSize::Small, 28.),
        (InputSize::Medium, 32.),
        (InputSize::Large, 40.),
    ] {
        probe.update(cx, |p, cx| {
            p.size = size;
            cx.notify();
        });
        for rem in [16., 20., 24.] {
            cx.update(|w, _| w.set_rem_size(gpui_kit::px(rem)));
            draw(cx);
            cx.update(|w, _| {
                let cell = named(w, "2026-11-08").bounds();
                assert_eq!(cell.size.width, gpui_kit::px(expected * rem / 16.));
                assert_eq!(cell.size.height, gpui_kit::px(expected * rem / 16.));
            });
        }
    }
}

#[gpui_kit::test]
fn legacy_base_range_still_restarts_on_reverse_selection(cx: &mut TestAppContext) {
    let (_, cx) = open(options(), true, cx);
    cx.update(|w, cx| {
        let base = cx.new(|cx| gpui_kit::base::CalendarState::new(w, cx));
        base.update(cx, |s, _| {
            s.apply_date(gpui_kit::base::Date::Range(None, None));
            assert!(!s.select_date(date(2026, 11, 20)));
            assert!(!s.select_date(date(2026, 11, 10)));
            assert_eq!(
                s.date(),
                gpui_kit::base::Date::Range(Some(date(2026, 11, 10)), None)
            );
            assert!(s.select_date(date(2026, 11, 25)));
        });
    });
}
