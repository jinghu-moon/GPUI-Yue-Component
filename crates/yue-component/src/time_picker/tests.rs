use super::*;
use crate::{Accent, ThemeMode};
use chrono::NaiveTime;
use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, EntityInputHandler as _, Focusable as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window, div,
};

fn time(h: u32, m: u32, s: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, s).unwrap()
}
fn options() -> TimePickerOptions {
    TimePickerOptions::new("time").clock(|| time(13, 42, 17))
}
struct Probe {
    state: Entity<YueTimePickerState>,
    inline: bool,
    disabled: bool,
    readonly: bool,
    size: InputSize,
    changes: Vec<TimePickerChange>,
    events: Vec<&'static str>,
    _subscription: gpui_kit::Subscription,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let content = if self.inline {
            YueTimePanel::new(&self.state)
                .size(self.size)
                .disabled(self.disabled)
                .readonly(self.readonly)
                .into_any_element()
        } else {
            YueTimePicker::new(&self.state)
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
    options: TimePickerOptions,
    inline: bool,
    cx: &mut TestAppContext,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, crate::YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let probe = cx.new(|cx| {
            let state = cx.new(|cx| YueTimePickerState::new(options, window, cx).unwrap());
            let subscription = cx.subscribe(
                &state,
                |s: &mut Probe, _, event: &TimePickerEvent, _| match event {
                    TimePickerEvent::Change(change) => {
                        s.changes.push(change.clone());
                        s.events.push("change");
                    }
                    TimePickerEvent::OpenChange(v) => {
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

fn state(probe: &Entity<Probe>, cx: &App) -> Entity<YueTimePickerState> {
    probe.read(cx).state.clone()
}

fn click_id(cx: &mut VisualTestContext, id: &str) {
    draw(cx);
    let position = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| {
                n.path().last()
                    == Some(&gpui_kit::ElementId::from((
                        gpui_kit::ElementId::from("time"),
                        id.to_string(),
                    )))
            })
            .unwrap_or_else(|| panic!("missing {id}"))
            .bounds()
            .center()
    });
    cx.simulate_click(position, Default::default());
    draw(cx);
}
fn type_text(probe: &Entity<Probe>, cx: &mut VisualTestContext, text: &str) {
    cx.update(|w, cx| {
        let input = state(probe, cx)
            .read(cx)
            .input(TimePickerEndpoint::Start)
            .clone();
        input.focus_handle(cx).focus(w, cx);
        input.update(cx, |s, cx| {
            s.select_all(w, cx);
            s.replace_text_in_range(None, text, w, cx);
        });
    });
    draw(cx);
}
#[test]
fn strict_formats_precision_and_12_hour_mapping() {
    let o = options();
    assert_eq!(o.parse_time("09:30:15"), Ok(time(9, 30, 15)));
    for text in [
        "9:30:15",
        "24:00:00",
        "09:60:00",
        "09:30:60",
        "09:30:15x",
        "09:30",
        "09:30:15.100",
    ] {
        assert!(o.parse_time(text).is_err(), "{text}")
    }
    let o = o.hour_cycle(HourCycle::H12);
    assert_eq!(o.parse_time(" 12:00:00 am "), Ok(time(0, 0, 0)));
    assert_eq!(o.parse_time("12:00:00 PM"), Ok(time(12, 0, 0)));
    assert!(o.parse_time("00:00:00 AM").is_err());
}
#[test]
fn every_entrance_uses_steps_bounds_and_range_validation() {
    let o = options()
        .precision(TimePrecision::Minute)
        .steps(TimePickerSteps::new().minute(15))
        .constraints(
            TimePickerConstraints::new()
                .min(time(9, 30, 0))
                .max(time(17, 30, 0)),
        );
    assert!(o.parse_time("09:15").is_err());
    assert!(
        o.normalize(TimePickerValue::Single(time(10, 7, 0)))
            .is_err()
    );
    assert_eq!(
        o.normalize(TimePickerValue::Single(time(10, 15, 59))),
        Ok(TimePickerValue::Single(time(10, 15, 0)))
    );
    assert!(
        options()
            .steps(TimePickerSteps::new().minute(0))
            .check()
            .is_err()
    );
    assert!(options().format("HH:mm:ss").check().is_err());
    let o = o.selection(TimePickerSelection::Range);
    assert_eq!(
        o.normalize(TimePickerValue::Range(time(15, 0, 0), time(10, 0, 0))),
        Err(TimePickerError::ReversedRange)
    );
}
#[test]
fn prefix_choices_keep_reachable_hours_and_repair_lower_units() {
    let o = options().constraints(
        TimePickerConstraints::new()
            .min(time(9, 30, 0))
            .max(time(17, 30, 0))
            .disabled_minutes(|h, m| h == 12 && m < 45),
    );
    let rules = o.rules().unwrap();
    let c = rules.choices(time(10, 10, 0));
    assert_eq!(
        c.candidate(gpui_kit::base::TimeSegment::Hour, 9),
        Some(time(9, 30, 0))
    );
    assert_eq!(
        c.candidate(gpui_kit::base::TimeSegment::Hour, 12),
        Some(time(12, 45, 0))
    );
    assert!(c.candidate(gpui_kit::base::TimeSegment::Hour, 18).is_none());
    let o = options().steps(TimePickerSteps::new().minute(10));
    assert_eq!(
        o.rules().unwrap().nearest(time(10, 5, 0)),
        Some(time(10, 0, 59))
    );
}
#[gpui_kit::test]
fn opening_empty_does_not_commit_and_pointer_confirmation_is_once(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    click(cx, "打开时间选择");
    assert!(probe.read_with(cx, |p, _| p.changes.is_empty()));
    click_id(cx, "item-0-0-2");
    assert!(probe.read_with(cx, |p, _| p.changes.is_empty()));
    click(cx, "确定");
    probe.read_with(cx, |p, cx| {
        assert_eq!(p.changes.len(), 1);
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Single(time(2, 0, 0))
        );
        assert!(!p.state.read(cx).is_open());
    });
}
#[gpui_kit::test]
fn keyboard_columns_and_escape_preserve_committed_snapshot(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    click(cx, "打开时间选择");
    keys(cx, "down right down enter");
    probe.read_with(cx, |p, cx| {
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Single(time(1, 1, 0))
        )
    });
    click(cx, "打开时间选择");
    keys(cx, "down escape");
    probe.read_with(cx, |p, cx| {
        assert_eq!(p.changes.len(), 1);
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Single(time(1, 1, 0))
        );
    });
}
#[gpui_kit::test]
fn input_invalid_is_retained_enter_blur_is_deduplicated(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    type_text(&probe, cx, "25:12:00");
    keys(cx, "enter");
    probe.read_with(cx, |p, cx| {
        assert!(p.state.read(cx).error().is_some());
        assert_eq!(
            p.state
                .read(cx)
                .input(TimePickerEndpoint::Start)
                .read(cx)
                .value()
                .as_str(),
            "25:12:00"
        );
        assert!(p.changes.is_empty());
    });
    type_text(&probe, cx, "09:30:15");
    keys(cx, "enter");
    click(cx, "外部");
    assert_eq!(probe.read_with(cx, |p, _| p.changes.len()), 1);
}
#[gpui_kit::test]
fn immediate_changes_once_and_outside_discards_unconfirmed_text(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().commit_mode(TimePickerCommitMode::Immediate),
        false,
        cx,
    );
    click(cx, "打开时间选择");
    click_id(cx, "item-0-0-2");
    assert_eq!(probe.read_with(cx, |p, _| p.changes.len()), 1);
    click(cx, "外部");
    assert!(!probe.read_with(cx, |p, cx| p.state.read(cx).is_open()));
}
#[gpui_kit::test]
fn range_follow_is_atomic_and_reversed_input_is_rejected(cx: &mut TestAppContext) {
    let (probe, cx) = open(options().selection(TimePickerSelection::Range), false, cx);
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.set_value(TimePickerValue::Range(time(1, 0, 0), time(2, 0, 0)), w, cx)
                .unwrap()
        })
    });
    draw(cx);
    click(cx, "打开时间选择");
    click_id(cx, "item-0-0-3");
    click(cx, "确定");
    probe.read_with(cx, |p, cx| {
        assert_eq!(p.changes.len(), 1);
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Range(time(3, 0, 0), time(3, 0, 0))
        );
    });
}
#[gpui_kit::test]
fn external_set_is_silent_and_invalid_set_preserves_session(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().steps(TimePickerSteps::new().minute(15)),
        false,
        cx,
    );
    click(cx, "打开时间选择");
    click_id(cx, "item-0-0-3");
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            assert!(
                s.set_value(TimePickerValue::Single(time(9, 7, 0)), w, cx)
                    .is_err()
            );
            assert_eq!(
                s.candidate(cx).unwrap(),
                TimePickerValue::Single(time(3, 0, 0))
            );
            s.set_value(TimePickerValue::Single(time(8, 15, 0)), w, cx)
                .unwrap();
        })
    });
    draw(cx);
    click(cx, "取消");
    assert!(probe.read_with(cx, |p, _| p.changes.is_empty()));
}
#[gpui_kit::test]
fn readonly_blocks_all_commands_and_rule_changes_keep_history(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.set_value(TimePickerValue::Single(time(9, 0, 0)), w, cx)
                .unwrap()
        })
    });
    probe.update(cx, |p, cx| {
        p.readonly = true;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.clear(w, cx);
            s.now(w, cx);
            s.open(w, cx);
            assert!(!s.is_open());
            assert_eq!(s.value(), &TimePickerValue::Single(time(9, 0, 0)));
        })
    });
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.set_constraints(TimePickerConstraints::new().disabled_hours(|_| true), w, cx)
                .unwrap();
            assert!(s.error().is_some());
            assert!(!s.value().is_empty());
        })
    });
}
#[gpui_kit::test]
fn now_uses_injected_clock_and_all_disabled_stays_empty(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options()
            .precision(TimePrecision::Minute)
            .steps(TimePickerSteps::new().minute(15)),
        true,
        cx,
    );
    click(cx, "此刻");
    click(cx, "确定");
    probe.read_with(cx, |p, cx| {
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Single(time(13, 45, 0))
        )
    });
}
#[gpui_kit::test]
fn tab_leaves_nonmodal_panel_and_cancels(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    click(cx, "打开时间选择");
    keys(cx, "down");
    for _ in 0..8 {
        if !probe.read_with(cx, |p, cx| p.state.read(cx).is_open()) {
            break;
        }
        keys(cx, "tab");
    }
    probe.read_with(cx, |p, cx| {
        assert!(!p.state.read(cx).is_open());
        assert!(p.state.read(cx).value().is_empty());
        assert!(p.changes.is_empty());
    });
}

#[gpui_kit::test]
fn ime_enter_and_step_leave_preedit_intact(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    let input = cx.update(|w, cx| {
        let input = state(&probe, cx).read(cx).inputs[0].clone();
        input.focus_handle(cx).focus(w, cx);
        input
    });
    draw(cx);
    cx.update(|w, cx| {
        input.update(cx, |s, cx| {
            s.replace_and_mark_text_in_range(None, "09:30:15", None, w, cx)
        })
    });
    draw(cx);
    keys(cx, "enter up");
    cx.update(|w, cx| {
        assert!(state(&probe, cx).read(cx).value().is_empty());
        assert!(input.update(cx, |s, cx| s.marked_text_range(w, cx).is_some()));
        input.update(cx, |s, cx| s.unmark_text(w, cx));
    });
    keys(cx, "enter");
    assert_eq!(probe.read_with(cx, |p, _| p.changes.len()), 1);
}
#[gpui_kit::test]
fn panel_selection_accepts_valid_text_as_draft_without_early_commit(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    type_text(&probe, cx, "09:30:15");
    click(cx, "打开时间选择");
    // Focus is on the hour column; applying a minute change must retain typed hours/seconds.
    keys(cx, "right down");
    probe.read_with(cx, |p, cx| {
        assert_eq!(
            p.state.read(cx).candidate(cx).unwrap(),
            TimePickerValue::Single(time(9, 31, 15))
        );
        assert!(p.state.read(cx).value().is_empty());
        assert!(p.changes.is_empty());
    });
    keys(cx, "enter");
    assert_eq!(probe.read_with(cx, |p, _| p.changes.len()), 1);
}
#[gpui_kit::test]
fn rejected_range_follow_and_reversed_text_leave_both_endpoints_unchanged(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options()
            .selection(TimePickerSelection::Range)
            .constraints(TimePickerConstraints::new().range_allowed(|a, b| a < b)),
        false,
        cx,
    );
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.set_value(TimePickerValue::Range(time(1, 0, 0), time(2, 0, 0)), w, cx)
                .unwrap()
        })
    });
    click(cx, "打开时间选择");
    click_id(cx, "item-0-0-3");
    probe.read_with(cx, |p, cx| {
        assert!(p.state.read(cx).error().is_some());
        assert_eq!(
            p.state.read(cx).candidate(cx).unwrap(),
            TimePickerValue::Range(time(1, 0, 0), time(2, 0, 0))
        );
        assert!(p.changes.is_empty());
    });
    keys(cx, "escape");
    type_text(&probe, cx, "18:00:00");
    keys(cx, "enter");
    probe.read_with(cx, |p, cx| {
        assert_eq!(
            p.state.read(cx).error(),
            Some(&TimePickerError::ReversedRange)
        );
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Range(time(1, 0, 0), time(2, 0, 0))
        );
    });
}
#[gpui_kit::test]
fn all_disabled_has_no_default_candidate_or_commit(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().constraints(TimePickerConstraints::new().disabled_hours(|_| true)),
        true,
        cx,
    );
    click(cx, "此刻");
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            assert!(!s.confirm(w, cx));
            assert!(s.value().is_empty());
            assert!(s.candidate(cx).unwrap().is_empty());
        })
    });
    assert!(probe.read_with(cx, |p, _| p.changes.is_empty()));
}
#[gpui_kit::test]
fn observer_replacement_invalidates_pending_confirm_close(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    let subscription = cx.update(|w, cx| {
        probe.update(cx, |_, cx| {
            cx.subscribe_in(&editor, w, |_, entity, event: &TimePickerEvent, w, cx| {
                if matches!(event, TimePickerEvent::Change(_)) {
                    entity.update(cx, |s, cx| {
                        s.set_value(TimePickerValue::Single(time(18, 0, 0)), w, cx)
                            .unwrap()
                    });
                }
            })
        })
    });
    click(cx, "打开时间选择");
    keys(cx, "down enter");
    probe.read_with(cx, |p, cx| {
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Single(time(18, 0, 0))
        );
        assert!(p.state.read(cx).is_open());
        assert_eq!(p.changes.len(), 1);
    });
    drop(subscription);
}
#[gpui_kit::test]
fn typed_form_reads_committed_values_and_rejects_invalid_history(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    let editor = cx.update(|_, cx| state(&probe, cx));
    let form = cx.update(|_, cx| {
        cx.new(|_| {
            let mut f = crate::YueForm::new();
            f.require_time_picker("start", &editor, "请选择时间");
            f
        })
    });
    cx.update(|w, cx| {
        assert!(!form.update(cx, |s, cx| s.submit(w, cx)));
        assert!(editor.read(cx).inputs[0].focus_handle(cx).is_focused(w));
    });
    click(cx, "打开时间选择");
    keys(cx, "down");
    cx.update(|_, cx| {
        assert_eq!(
            form.read(cx).error("start", cx).as_deref(),
            Some("请选择时间")
        )
    });
    keys(cx, "enter");
    cx.update(|w, cx| {
        assert!(form.update(cx, |s, cx| s.submit(w, cx)));
        editor.update(cx, |s, cx| {
            s.set_constraints(TimePickerConstraints::new().disabled_hours(|_| true), w, cx)
                .unwrap()
        });
        assert!(!form.update(cx, |s, cx| s.submit(w, cx)));
    });
}
#[gpui_kit::test]
fn wheel_only_browses_and_interrupts_programmatic_positioning(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options().commit_mode(TimePickerCommitMode::Immediate),
        true,
        cx,
    );
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.set_value(TimePickerValue::Single(time(12, 30, 0)), w, cx)
                .unwrap();
            s.columns[0][1].focus.focus(w, cx);
        })
    });
    draw(cx);
    keys(cx, "end");
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(30));
    draw(cx);
    let before = cx.update(|_, cx| state(&probe, cx).read(cx).columns[0][1].scroll.offset().y);
    let position = cx.update(|w, _| named(w, "分").bounds().center());
    cx.simulate_event(gpui_kit::ScrollWheelEvent {
        position,
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(64.))),
        touch_phase: gpui_kit::TouchPhase::Moved,
        ..Default::default()
    });
    draw(cx);
    let after = cx.update(|_, cx| state(&probe, cx).read(cx).columns[0][1].scroll.offset().y);
    assert!(after > before, "wheel must move the native column");
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(500));
    draw(cx);
    probe.read_with(cx, |p, cx| {
        assert_eq!(p.state.read(cx).columns[0][1].scroll.offset().y, after);
        assert_eq!(
            p.state.read(cx).value(),
            &TimePickerValue::Single(time(12, 59, 0))
        );
        assert_eq!(p.changes.len(), 1);
    });
}
#[gpui_kit::test]
fn sizes_scale_and_narrow_h12_range_keeps_footer_and_last_item_visible(cx: &mut TestAppContext) {
    let (probe, cx) = open(
        options()
            .selection(TimePickerSelection::Range)
            .hour_cycle(HourCycle::H12),
        false,
        cx,
    );
    cx.simulate_resize(gpui_kit::size(gpui_kit::px(900.), gpui_kit::px(800.)));
    draw(cx);
    click(cx, "打开时间选择");
    cx.update(|w, _| {
        assert_eq!(
            snapshots(w)
                .iter()
                .filter(|n| n.role() == Some(gpui_kit::Role::ListBox))
                .count(),
            8
        )
    });
    cx.simulate_resize(gpui_kit::size(gpui_kit::px(360.), gpui_kit::px(600.)));
    draw(cx);
    for (size, height) in [
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
                let columns: Vec<_> = snapshots(w)
                    .into_iter()
                    .filter(|n| n.role() == Some(gpui_kit::Role::ListBox))
                    .collect();
                assert_eq!(columns.len(), 4);
                for c in columns {
                    assert!(c.bounds().left() >= gpui_kit::px(0.));
                    assert!(c.bounds().right() <= w.viewport_size().width);
                }
                let footer = named(w, "确定").bounds();
                assert!(footer.bottom() <= w.viewport_size().height);
                let item = snapshots(w)
                    .into_iter()
                    .find(|n| n.role() == Some(gpui_kit::Role::ListBoxOption))
                    .unwrap();
                assert_eq!(item.bounds().size.height, gpui_kit::px(height * rem / 16.));
            });
        }
    }
    cx.update(|w, cx| {
        let focus = state(&probe, cx).read(cx).columns[0][1].focus.clone();
        focus.focus(w, cx);
    });
    keys(cx, "end");
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(500));
    draw(cx);
    cx.update(|w, _| {
        let list = named(w, "开始分").bounds();
        let item = snapshots(w)
            .into_iter()
            .find(|n| {
                n.path().last()
                    == Some(&gpui_kit::ElementId::from((
                        gpui_kit::ElementId::from("time"),
                        "item-0-1-59",
                    )))
            })
            .unwrap()
            .bounds();
        assert!(item.top() >= list.top());
        assert!(item.bottom() <= list.bottom());
    });
}

#[gpui_kit::test]
fn external_reset_clears_half_typed_segment_and_reduced_motion_snaps(cx: &mut TestAppContext) {
    let (probe, cx) = open(options(), false, cx);
    click(cx, "打开时间选择");
    keys(cx, "1");
    cx.update(|w, cx| {
        state(&probe, cx).update(cx, |s, cx| {
            s.set_value(TimePickerValue::Single(time(9, 0, 0)), w, cx)
                .unwrap()
        })
    });
    draw(cx);
    keys(cx, "2");
    probe.read_with(cx, |p, cx| {
        assert_eq!(
            p.state.read(cx).candidate(cx).unwrap(),
            TimePickerValue::Single(time(2, 0, 0))
        )
    });
    cx.update(|_, cx| cx.set_reduce_motion(true));
    keys(cx, "right end");
    cx.update(|w, cx| {
        let s = state(&probe, cx);
        let s = s.read(cx);
        assert_eq!(
            s.columns[0][1].scroll.offset().y,
            -s.columns[0][1].scroll.max_offset().y
        );
        assert!(s.columns[0][1].focus.is_focused(w));
    });
}
