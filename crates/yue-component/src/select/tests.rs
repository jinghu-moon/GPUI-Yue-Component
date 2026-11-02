use super::*;
use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
use gpui_kit::{
    AppContext as _, Context, Modifiers, ParentElement as _, Render, Role, Styled as _,
    TestAppContext, VisualTestContext, Window, div, px,
};
struct Probe {
    state: Entity<YueSelectState>,
    events: Vec<SelectEvent>,
    _subscription: gpui_kit::Subscription,
    form: Entity<crate::YueForm>,
    size: SelectSize,
    width: gpui_kit::Pixels,
    visible: bool,
    decorate: Option<Rc<dyn Fn(YueSelect) -> YueSelect>>,
}
impl Probe {
    fn new(mode: SelectMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            let mut state = YueSelectState::new(mode, window, cx);
            state
                .set_options(
                    [
                        SelectOption::new("a", "Apple"),
                        SelectOption::new("b", "Banana").disabled(true),
                        SelectOption::new("c", "Cherry"),
                    ],
                    cx,
                )
                .unwrap();
            state
        });
        let subscription = cx.subscribe(&state, |probe, _, event, cx| {
            probe.events.push(event.clone());
            cx.notify();
        });
        let form = cx.new(|_| crate::YueForm::new());
        form.update(cx, |form, _| {
            form.require_select("fruit", &state, "请选择水果");
        });
        Self {
            state,
            events: vec![],
            _subscription: subscription,
            form,
            size: SelectSize::Medium,
            width: px(240.),
            visible: true,
            decorate: None,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut select = YueSelect::new("fruit", &self.state)
            .accessibility_label("Fruit")
            .size(self.size)
            .width(self.width)
            .clearable(true);
        if let Some(decorate) = &self.decorate {
            select = decorate(select);
        }
        div()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(600.))
            .child(crate::YueButton::new("before", "Before"))
            .children(self.visible.then_some(select))
            .child(crate::YueButton::new("after", "After"))
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}
fn open(cx: &mut TestAppContext, mode: SelectMode) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            crate::YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap()
    });
    let mut probe = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| Probe::new(mode, window, cx));
        probe = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    draw(cx);
    draw(cx);
    (probe.unwrap(), cx)
}
fn find(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
    cx.update(|window, _| {
        let found = snapshots(window)
            .into_iter()
            .filter(|snapshot| snapshot.label() == Some(label))
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "one target for {label}: {found:?}");
        found[0].clone()
    })
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let point = find(cx, label).bounds().center();
    cx.simulate_click(point, Modifiers::default());
    draw(cx);
}
fn key(cx: &mut VisualTestContext, key: &str) {
    let key = gpui_kit::Keystroke::parse(key).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
    draw(cx);
}
fn change_count(view: &Entity<Probe>, cx: &VisualTestContext) -> usize {
    view.read_with(cx, |probe, _| {
        probe
            .events
            .iter()
            .filter(|event| matches!(event, SelectEvent::Change(_)))
            .count()
    })
}

fn select_panel(cx: &mut VisualTestContext) -> Option<ElementSnapshot> {
    let id: gpui_kit::ElementId = (gpui_kit::ElementId::from("fruit"), "select-panel").into();
    cx.update(|window, _| {
        snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&id))
    })
}

fn tick(cx: &mut VisualTestContext, millis: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(millis));
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
}

fn panel_alpha(cx: &mut VisualTestContext) -> f32 {
    let panel = select_panel(cx).expect("Select panel remains mounted during its animation");
    cx.update(|window, _| {
        window
            .painted_quads()
            .into_iter()
            .find(|quad| quad.bounds == panel.bounds().scale(window.scale_factor()))
            .and_then(|quad| quad.background.as_solid())
            .expect("panel background submitted to scene")
            .a
    })
}

fn assert_complete_focus_ring(cx: &mut VisualTestContext, frame: gpui_kit::Bounds<Pixels>) {
    cx.update(|window, _| {
        let theme = crate::YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure);
        let ring: gpui_kit::Hsla = crate::tokens::rgba(theme, "--input-focus-ring-color").into();
        let outside = px(crate::tokens::pixels(theme, "--input-focus-ring-width")
            + crate::tokens::pixels(theme, "--input-focus-ring-offset"));
        let expected = gpui_kit::Bounds::new(
            frame.origin - gpui_kit::point(outside, outside),
            frame.size + gpui_kit::size(outside * 2., outside * 2.),
        )
        .scale(window.scale_factor());
        let rings = window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.border_color == ring && quad.bounds == expected)
            .collect::<Vec<_>>();
        assert!(!rings.is_empty(), "whole focus ring at {expected:?}");
        let half_width = rings[0].border_widths.top / 2.;
        let center = expected.center();
        for point in [
            gpui_kit::point(center.x, expected.top() + half_width),
            gpui_kit::point(center.x, expected.bottom() - half_width),
            gpui_kit::point(expected.left() + half_width, center.y),
            gpui_kit::point(expected.right() - half_width, center.y),
        ] {
            assert!(
                rings
                    .iter()
                    .any(|quad| quad.content_mask.bounds.contains(&point)),
                "focus border at {point:?} must survive clipping: {rings:?}"
            );
        }
    });
}

#[gpui_kit::test]
fn popup_fades_and_moves_on_both_dropdown_motion_legs(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, SelectMode::Single);
    click(cx, "展开选项");
    let start = select_panel(cx).unwrap().bounds();
    tick(cx, 50);
    let middle = select_panel(cx).unwrap().bounds();
    let alpha = panel_alpha(cx);
    tick(cx, 150);
    let rest = select_panel(cx).unwrap().bounds();
    assert!((f32::from(rest.top() - start.top()) - 4.).abs() < 0.01);
    assert!(start.top() < middle.top() && middle.top() < rest.top());
    let theme = crate::YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure);
    let ease_enter = crate::tokens::easing(theme, "--motion-ease-enter");
    assert!(
        (alpha - ease_enter(0.25)).abs() < 0.01,
        "enter alpha: {alpha}"
    );
    assert!((panel_alpha(cx) - 1.).abs() < 0.001);

    key(cx, "escape");
    assert_eq!(select_panel(cx).unwrap().bounds(), rest);
    tick(cx, 50);
    let exiting = select_panel(cx).unwrap().bounds();
    let ease_exit = crate::tokens::easing(theme, "--motion-ease-exit");
    assert!((panel_alpha(cx) - (1. - ease_exit(0.5))).abs() < 0.01);
    assert!(start.top() < exiting.top() && exiting.top() < rest.top());
    cx.update(|window, _| {
        assert!(
            !snapshots(window)
                .iter()
                .any(|node| node.role() == Some(Role::ListBoxOption))
        );
    });
    tick(cx, 50);
    assert!(select_panel(cx).is_none());

    // Reduced motion settles both geometry and opacity immediately.
    cx.update(|_, cx| cx.set_reduce_motion(true));
    click(cx, "展开选项");
    assert_eq!(select_panel(cx).unwrap().bounds(), rest);
    assert!((panel_alpha(cx) - 1.).abs() < 0.001);
    key(cx, "escape");
    assert!(select_panel(cx).is_none());
}

#[gpui_kit::test]
fn single_keyboard_skips_disabled_and_restores_trigger(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| state.read(cx).focus_handle().clone().focus(window, cx));
    draw(cx);
    key(cx, "down");
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("a".into())
    );
    key(cx, "down");
    key(cx, "enter");
    assert_eq!(
        state.read_with(cx, |s, _| s.value().clone()),
        SelectValue::Single(Some("c".into()))
    );
    assert_eq!(change_count(&view, cx), 1);
    cx.update(|window, cx| {
        assert!(!state.read(cx).is_open());
        assert!(state.read(cx).focus_handle().is_focused(window));
    });
}
#[gpui_kit::test]
fn pointer_selection_clear_and_no_change_on_external_echo(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    click(cx, "展开选项");
    click(cx, "Apple");
    assert_eq!(change_count(&view, cx), 1);
    cx.update(|window, cx| {
        state.update(cx, |state, cx| {
            state.set_value(state.value().clone(), window, cx).unwrap()
        })
    });
    draw(cx);
    assert_eq!(change_count(&view, cx), 1);
    click(cx, "清空选择");
    assert!(state.read_with(cx, |s, _| s.value().is_empty()));
    assert_eq!(change_count(&view, cx), 2);
}
#[gpui_kit::test]
fn multi_toggle_stays_open_and_locked_clear_is_preserved(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Multiple);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(SelectValue::Multiple(vec!["b".into()]), window, cx)
                .unwrap()
        })
    });
    draw(cx);
    click(cx, "展开选项");
    click(cx, "Apple");
    assert!(state.read_with(cx, |s, _| s.is_open()));
    assert_eq!(change_count(&view, cx), 1);
    key(cx, "escape");
    click(cx, "清除可移除的选择");
    assert_eq!(
        state.read_with(cx, |s, _| s.value().clone()),
        SelectValue::Multiple(vec!["b".into()])
    );
    assert_eq!(change_count(&view, cx), 2);
}
#[gpui_kit::test]
fn disabled_after_focus_and_readonly_do_not_open(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.read(cx).focus_handle().clone().focus(window, cx);
        state.update(cx, |s, cx| s.set_disabled(true, window, cx));
    });
    draw(cx);
    key(cx, "enter");
    key(cx, "down");
    assert!(!state.read_with(cx, |s, _| s.is_open()));
    assert_eq!(change_count(&view, cx), 0);
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_disabled(false, window, cx);
            s.set_readonly(true, window, cx);
            s.focus_handle().focus(window, cx);
        })
    });
    draw(cx);
    key(cx, "enter");
    assert!(!state.read_with(cx, |s, _| s.is_open()));
}
#[gpui_kit::test]
fn query_is_existing_yue_input_and_clear_filters_once(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Local, window, cx)
        })
    });
    draw(cx);
    click(cx, "展开选项");
    assert_eq!(find(cx, "搜索选项").role(), Some(Role::SearchInput));
    cx.simulate_input("cher");
    draw(cx);
    assert_eq!(state.read_with(cx, |s, cx| s.query(cx)), "cher");
    assert_eq!(find(cx, "Fruit选项").description(), Some("共 1 个选项"));
    assert_eq!(
        state.read_with(cx, |s, _| s
            .rows
            .iter()
            .filter(|row| matches!(row, model::Row::Option(_)))
            .count()),
        1
    );
    click(cx, "Clear input");
    assert_eq!(state.read_with(cx, |s, cx| s.query(cx)), "");
    assert_eq!(state.read_with(cx, |s, _| s.rows.len()), 3);
    assert_eq!(find(cx, "Fruit选项").description(), Some("共 3 个选项"));
    assert_eq!(change_count(&view, cx), 0);
}
#[gpui_kit::test]
fn remote_tickets_reject_other_owner_close_reopen_and_old_error(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Remote, window, cx);
            s.set_open(true, window, cx);
        })
    });
    draw(cx);
    let ticket = view.read_with(cx, |v, _| {
        v.events
            .iter()
            .find_map(|e| {
                if let SelectEvent::SearchRequested(r) = e {
                    Some(r.ticket().clone())
                } else {
                    None
                }
            })
            .unwrap()
    });
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_open(false, window, cx);
            assert_eq!(
                s.result_state(),
                &SelectResultState::Ready,
                "closing cancels loading visuals as well as tickets"
            );
            s.set_open(true, window, cx);
            assert_eq!(
                s.apply_page(
                    &ticket,
                    SelectPage::new([SelectOption::new("old", "Old")]),
                    cx
                )
                .unwrap(),
                SelectApply::Stale
            );
            assert_eq!(
                s.reject_request(&ticket, "late error", cx),
                SelectApply::Stale
            );
        })
    });
}
#[gpui_kit::test]
fn form_reads_typed_selection_and_focuses_first_failure(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let (form, state) = view.read_with(cx, |v, _| (v.form.clone(), v.state.clone()));
    cx.update(|window, cx| {
        assert!(!form.update(cx, |form, cx| form.submit(window, cx)));
        assert!(state.read(cx).focus_handle().is_focused(window));
        state.update(cx, |s, cx| {
            s.set_value(SelectValue::Single(Some("a".into())), window, cx)
                .unwrap()
        });
        assert!(form.read(cx).is_valid(cx));
        assert!(form.read(cx).error("fruit", cx).is_none());
    });
}
#[gpui_kit::test]
fn focus_ring_surrounds_the_whole_trigger_without_changing_layout(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |state, cx| {
            state
                .set_value(SelectValue::Single(Some("a".into())), window, cx)
                .unwrap();
        })
    });
    for (size, width) in [
        (SelectSize::Small, 180.),
        (SelectSize::Medium, 240.),
        (SelectSize::Large, 320.),
    ] {
        view.update(cx, |v, cx| {
            v.size = size;
            v.width = px(width);
            cx.notify();
        });
        cx.update(|window, cx| window.blur(cx));
        draw(cx);
        let before = find(cx, "Fruit").bounds();
        cx.update(|window, cx| state.read(cx).focus_handle().clone().focus(window, cx));
        draw(cx);
        assert_eq!(find(cx, "Fruit").bounds(), before);
        assert_complete_focus_ring(cx, before);
    }
}

#[gpui_kit::test]
fn width_and_sizes_stay_independent_of_parent(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    for (size, height) in [
        (SelectSize::Small, 28.),
        (SelectSize::Medium, 32.),
        (SelectSize::Large, 40.),
    ] {
        view.update(cx, |v, cx| {
            v.size = size;
            cx.notify();
        });
        draw(cx);
        let frame = find(cx, "Fruit");
        assert!((f32::from(frame.bounds().size.width) - 240.).abs() < 1.);
        assert!((f32::from(frame.bounds().size.height) - height).abs() < 1.);
    }
}

fn last_request(view: &Entity<Probe>, cx: &VisualTestContext) -> SelectRequestTicket {
    view.read_with(cx, |v, _| {
        v.events
            .iter()
            .rev()
            .find_map(|event| match event {
                SelectEvent::SearchRequested(r)
                | SelectEvent::LoadMoreRequested(r)
                | SelectEvent::CreateRequested(r) => Some(r.ticket().clone()),
                _ => None,
            })
            .expect("request emitted")
    })
}

#[gpui_kit::test]
fn external_close_and_disable_do_not_leave_hidden_input_focused(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Local, window, cx)
        })
    });
    draw(cx);
    click(cx, "展开选项");
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            assert!(s.content_focus(cx).is_focused(window));
            s.set_open(false, window, cx);
            assert!(s.focus.is_focused(window));
        })
    });
    draw(cx);
    click(cx, "展开选项");
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            let query = s.content_focus(cx);
            s.set_disabled(true, window, cx);
            assert!(!query.is_focused(window));
            assert!(!s.focus.is_focused(window));
        })
    });
}

#[gpui_kit::test]
fn invalid_remote_page_is_atomic_and_retryable(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Remote, window, cx);
            s.set_open(true, window, cx);
        })
    });
    draw(cx);
    let ticket = last_request(&view, cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            assert!(matches!(
                s.apply_page(
                    &ticket,
                    SelectPage::new([
                        SelectOption::new("d", "D"),
                        SelectOption::new("d", "Duplicate")
                    ]),
                    cx
                ),
                Err(SelectError::DuplicateKey(_))
            ));
            assert_eq!(s.options().len(), 3);
            assert!(matches!(s.result_state(), SelectResultState::Failed(_)));
            s.retry_request(cx);
            assert!(matches!(s.result_state(), SelectResultState::Loading));
            assert_eq!(s.reject_request(&ticket, "late", cx), SelectApply::Stale);
        })
    });
}

#[gpui_kit::test]
fn remote_pagination_rejects_no_progress_and_duplicate_keys(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Remote, window, cx);
            s.set_open(true, window, cx);
        })
    });
    draw(cx);
    let ticket = last_request(&view, cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            s.apply_page(
                &ticket,
                SelectPage::new([SelectOption::new("x", "X")]).next_cursor("next"),
                cx,
            )
            .unwrap();
            s.request_more(cx);
        })
    });
    draw(cx);
    let more = last_request(&view, cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            assert_eq!(
                s.apply_page(
                    &more,
                    SelectPage::new([SelectOption::new("y", "Y")]).next_cursor("next"),
                    cx
                ),
                Err(SelectError::NoProgress)
            );
            assert_eq!(s.options().len(), 1);
            s.retry_request(cx);
        })
    });
    draw(cx);
    let retry = last_request(&view, cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            assert!(matches!(
                s.apply_page(
                    &retry,
                    SelectPage::new([SelectOption::new("x", "Duplicate")]),
                    cx
                ),
                Err(SelectError::DuplicateKey(_))
            ));
            assert!(matches!(s.more_result, SelectResultState::Failed(_)));
            assert_eq!(s.options().len(), 1);
        })
    });
}

#[gpui_kit::test]
fn unknown_selection_hydrates_without_changing_identity_or_emitting(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Multiple);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(SelectValue::Multiple(vec!["missing".into()]), window, cx)
                .unwrap();
            assert_eq!(s.selected_items()[0].label(), "missing");
            s.set_selected_items(
                [SelectOption::new("missing", "Hydrated").disabled(true)],
                cx,
            )
            .unwrap();
            s.set_options([SelectOption::new("other", "Other")], cx)
                .unwrap();
            assert_eq!(s.selected_items()[0].label(), "Hydrated");
            s.clear(window, cx);
            assert_eq!(s.value().keys(), &[SharedString::from("missing")]);
        })
    });
    assert_eq!(change_count(&view, cx), 0);
}

#[gpui_kit::test]
fn filtering_and_replacing_scrolled_options_keeps_active_row_visible(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_options(
                (0..10_000).map(|i| SelectOption::new(format!("k{i}"), format!("Item {i}"))),
                cx,
            )
            .unwrap();
            s.set_search_mode(SelectSearch::Local, window, cx);
        })
    });
    draw(cx);
    click(cx, "展开选项");
    key(cx, "down");
    key(cx, "end");
    draw(cx);
    find(cx, "Item 9999");
    click(cx, "搜索选项");
    cx.simulate_input("Item 1");
    draw(cx);
    draw(cx);
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("k1".into())
    );
    let first = find(cx, "Item 1");
    let viewport = find(cx, "Fruit选项");
    assert!(viewport.bounds().contains(&first.bounds().center()));

    // Keep the active identity but move it to the end of a replacement model.
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            s.set_options(
                (100..1200)
                    .map(|i| SelectOption::new(format!("k{i}"), format!("Item {i}")))
                    .chain([SelectOption::new("k1", "Item 1")]),
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    draw(cx);
    let moved = find(cx, "Item 1");
    let viewport = find(cx, "Fruit选项");
    assert!(viewport.bounds().contains(&moved.bounds().center()));
}

#[gpui_kit::test]
fn ten_thousand_options_virtualize_and_end_selects_last_model_key(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            s.set_options(
                (0..10_000).map(|i| SelectOption::new(format!("k{i}"), format!("Item {i}"))),
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    click(cx, "展开选项");
    cx.update(|window, _| {
        let count = snapshots(window)
            .iter()
            .filter(|s| s.role() == Some(Role::ListBoxOption))
            .count();
        assert!(count > 0 && count < 40, "bounded mounted rows: {count}");
    });
    key(cx, "end");
    draw(cx);
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("k9999".into())
    );
    let last = find(cx, "Item 9999");
    assert!(last.bounds().size.height > px(0.));
    key(cx, "enter");
    assert_eq!(
        state.read_with(cx, |s, _| s.value().clone()),
        SelectValue::Single(Some("k9999".into()))
    );
}

#[gpui_kit::test]
fn ime_preedit_does_not_filter_or_select_and_unmark_commits_query(cx: &mut TestAppContext) {
    use gpui_kit::EntityInputHandler as _;
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Local, window, cx)
        })
    });
    draw(cx);
    click(cx, "展开选项");
    let input = state.read_with(cx, |s, _| s.query_input().unwrap().clone());
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "cher", None, window, cx)
        })
    });
    draw(cx);
    assert_eq!(state.read_with(cx, |s, _| s.rows.len()), 3);
    key(cx, "enter");
    assert_eq!(change_count(&view, cx), 0);
    cx.update(|window, cx| input.update(cx, |input, cx| input.unmark_text(window, cx)));
    draw(cx);
    assert_eq!(state.read_with(cx, |s, _| s.rows.len()), 1);
    key(cx, "enter");
    assert_eq!(change_count(&view, cx), 0, "first Enter enters results");
    key(cx, "enter");
    assert_eq!(change_count(&view, cx), 1);
}

#[gpui_kit::test]
fn query_tab_leaves_entire_field_and_shift_tab_returns_to_previous(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Multiple);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(
                SelectValue::Multiple(vec!["a".into(), "c".into()]),
                window,
                cx,
            )
            .unwrap();
            s.set_search_mode(SelectSearch::Local, window, cx);
        })
    });
    draw(cx);
    click(cx, "展开选项");
    key(cx, "tab");
    assert_eq!(find(cx, "After").focused(), Some(true));
    assert!(!state.read_with(cx, |s, _| s.is_open()));
    click(cx, "展开选项");
    key(cx, "shift-tab");
    assert_eq!(find(cx, "Before").focused(), Some(true));
    assert_eq!(state.read_with(cx, |s, cx| s.query(cx)), "");
    assert_eq!(change_count(&view, cx), 0);
}

#[gpui_kit::test]
fn collapsed_tags_remove_without_opening_main_popup_and_escape_restores_count(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx, SelectMode::Multiple);
    let state = view.read_with(cx, |v, _| v.state.clone());
    view.update(cx, |v, cx| {
        v.decorate = Some(Rc::new(|s| s.collapse_count(0)));
        cx.notify();
    });
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(
                SelectValue::Multiple(vec!["a".into(), "b".into(), "c".into()]),
                window,
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    click(cx, "查看其余 3 个选择");
    click(cx, "移除 Apple");
    assert!(!state.read_with(cx, |s, _| s.is_open()));
    assert_eq!(change_count(&view, cx), 1);
    draw(cx);
    key(cx, "escape");
    assert_eq!(find(cx, "查看其余 2 个选择").focused(), Some(true));
    assert!(!state.read_with(cx, |s, _| s.collapsed_open));
}

#[gpui_kit::test]
fn page_keys_use_visible_rows_and_never_mode_mounts_all_options(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    view.update(cx, |v, cx| {
        v.decorate = Some(Rc::new(|s| {
            s.max_panel_height(px(96.))
                .virtualization(SelectVirtualization::Never)
        }));
        cx.notify();
    });
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            s.set_options(
                (0..25).map(|i| SelectOption::new(format!("k{i}"), format!("Item {i}"))),
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    click(cx, "展开选项");
    draw(cx);
    assert_eq!(state.read_with(cx, |s, _| s.page_size), 3);
    key(cx, "pagedown");
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("k3".into())
    );
    cx.update(|window, _| {
        assert_eq!(
            snapshots(window)
                .iter()
                .filter(|s| s.role() == Some(Role::ListBoxOption))
                .count(),
            25
        )
    });
    key(cx, "pageup");
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("k0".into())
    );
}

#[gpui_kit::test]
fn custom_option_content_is_vertically_centered_in_each_row(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    for virtualization in [SelectVirtualization::Never, SelectVirtualization::Always] {
        for row_layout in [SelectRowLayout::SingleLine, SelectRowLayout::Description] {
            for size in [SelectSize::Small, SelectSize::Medium, SelectSize::Large] {
                view.update(cx, |v, cx| {
                    v.size = size;
                    v.decorate = Some(Rc::new(move |select| {
                        select
                            .row_layout(row_layout)
                            .virtualization(virtualization)
                            .render_option(|row, _, _| {
                                crate::YueTag::new(
                                    format!("custom:{}", row.option().key()),
                                    format!("Tag {}", row.option().label()),
                                )
                                .size(match row.size() {
                                    SelectSize::Small => crate::TagSize::Small,
                                    SelectSize::Medium => crate::TagSize::Medium,
                                    SelectSize::Large => crate::TagSize::Large,
                                })
                            })
                    }));
                    cx.notify();
                });
                draw(cx);
                click(cx, "展开选项");
                tick(cx, 250);
                for label in ["Apple", "Banana", "Cherry"] {
                    let row = find(cx, label).bounds();
                    let tag = find(cx, &format!("Tag {label}")).bounds();
                    assert!(
                        f32::from(row.center().y - tag.center().y).abs() < 0.01,
                        "{size:?} / {virtualization:?} / {row_layout:?}: custom tag {tag:?} centered in {row:?}"
                    );
                    assert!(tag.top() >= row.top() && tag.bottom() <= row.bottom());
                }
                let before = state.read_with(cx, |state, _| state.value().clone());
                click(cx, "Tag Banana");
                assert_eq!(
                    state.read_with(cx, |state, _| state.value().clone()),
                    before
                );
                assert!(state.read_with(cx, |state, _| state.is_open()));
                click(cx, "Tag Cherry");
                assert_eq!(
                    state.read_with(cx, |state, _| state.value().clone()),
                    SelectValue::Single(Some("c".into()))
                );
                assert!(!state.read_with(cx, |state, _| state.is_open()));
                tick(cx, 150);
            }
        }
    }
}

#[gpui_kit::test]
fn footer_tab_order_and_exit_policy_use_registered_handles(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let footer_focus = cx.update(|_, cx| cx.focus_handle().tab_stop(false));
    let factory_focus = footer_focus.clone();
    view.update(cx, |v, cx| {
        v.decorate = Some(Rc::new(move |s| {
            let focus = factory_focus.clone();
            s.panel_footer_focus([focus.clone()])
                .panel_footer(move |interactive, _, _| {
                    crate::YueButton::new("footer-command", "Footer command")
                        .track_focus(&focus)
                        .tab_stop(false)
                        .disabled(!interactive)
                })
        }));
        cx.notify();
    });
    draw(cx);
    click(cx, "展开选项");
    key(cx, "tab");
    cx.update(|window, _| assert!(footer_focus.is_focused(window)));
    key(cx, "shift-tab");
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| assert!(state.read(cx).list_focus.is_focused(window)));
    key(cx, "tab");
    key(cx, "tab");
    assert_eq!(find(cx, "After").focused(), Some(true));
}

#[gpui_kit::test]
fn unmount_cancels_session_even_when_state_is_retained(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Remote, window, cx);
            s.set_open(true, window, cx);
        })
    });
    draw(cx);
    let ticket = last_request(&view, cx);
    view.update(cx, |v, cx| {
        v.visible = false;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    draw(cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            assert!(!s.is_open());
            assert_eq!(
                s.apply_page(
                    &ticket,
                    SelectPage::new([SelectOption::new("late", "Late")]),
                    cx
                )
                .unwrap(),
                SelectApply::Stale
            );
        })
    });
}

#[gpui_kit::test]
fn create_requests_validate_and_stale_responses_cannot_replace_selection(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Multiple);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Local, window, cx);
            s.set_creatable(true, cx);
        })
    });
    draw(cx);
    click(cx, "展开选项");
    cx.simulate_input("New fruit");
    draw(cx);
    key(cx, "enter");
    key(cx, "enter");
    let ticket = last_request(&view, cx);
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            assert_eq!(
                s.resolve_create(
                    &ticket,
                    SelectOption::new("new", "New fruit").disabled(true),
                    window,
                    cx
                ),
                Err(SelectError::DisabledOption)
            );
            assert!(matches!(s.create_result, SelectResultState::Failed(_)));
            s.retry_request(cx);
        })
    });
    draw(cx);
    let retry = last_request(&view, cx);
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.resolve_create(&retry, SelectOption::new("new", "New fruit"), window, cx)
                .unwrap();
            assert!(s.value().contains("new"));
            assert!(s.is_open());
        })
    });
    draw(cx);
    assert_eq!(change_count(&view, cx), 1);
    key(cx, "ctrl-f");
    click(cx, "Clear input");
    cx.simulate_input("Another");
    draw(cx);
    key(cx, "enter");
    key(cx, "enter");
    let pending = last_request(&view, cx);
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(SelectValue::Multiple(vec!["a".into()]), window, cx)
                .unwrap();
            assert_eq!(
                s.resolve_create(&pending, SelectOption::new("late", "Late"), window, cx)
                    .unwrap(),
                SelectApply::Stale
            );
        })
    });
    assert_eq!(change_count(&view, cx), 1);
}

#[gpui_kit::test]
fn outside_click_keeps_external_focus_and_escape_reports_cancel(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    click(cx, "展开选项");
    key(cx, "down");
    key(cx, "escape");
    assert_eq!(change_count(&view, cx), 0);
    view.read_with(cx, |v, _| assert!(v.events.iter().any(|e| matches!(e, SelectEvent::OpenChange(change) if !change.is_open() && change.reason() == SelectCloseReason::Escape))));
    draw(cx);
    click(cx, "展开选项");
    click(cx, "Before");
    assert_eq!(find(cx, "Before").focused(), Some(true));
    assert!(!view.read_with(cx, |v, cx| v.state.read(cx).is_open()));
}

#[gpui_kit::test]
fn remote_query_generation_debounces_and_rejects_other_owner(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Remote, window, cx)
        })
    });
    draw(cx);
    click(cx, "展开选项");
    let initial = last_request(&view, cx);
    cx.simulate_input("a");
    draw(cx);
    cx.simulate_input("b");
    draw(cx);
    cx.update(|window, cx| {
        let other = cx.new(|cx| YueSelectState::new(SelectMode::Single, window, cx));
        other.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Remote, window, cx);
            s.set_open(true, window, cx);
            assert_eq!(
                s.apply_page(
                    &initial,
                    SelectPage::new([SelectOption::new("wrong", "Wrong owner")]),
                    cx
                )
                .unwrap(),
                SelectApply::Stale
            );
        });
        state.update(cx, |s, cx| {
            assert_eq!(
                s.apply_page(
                    &initial,
                    SelectPage::new([SelectOption::new("old", "Old query")]),
                    cx
                )
                .unwrap(),
                SelectApply::Stale
            )
        });
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(220));
    draw(cx);
    view.read_with(cx, |v, _| {
        let searches = v
            .events
            .iter()
            .filter_map(|event| {
                if let SelectEvent::SearchRequested(request) = event {
                    Some(request.query().as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(searches, ["", "ab"]);
    });
}

#[gpui_kit::test]
fn caller_focus_tab_policy_and_same_value_echo_preserve_live_query(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let caller = cx.update(|_, cx| cx.focus_handle());
    let handle = caller.clone();
    view.update(cx, |v, cx| {
        v.decorate = Some(Rc::new(move |s| {
            s.track_focus(&handle).tab_index(4).tab_stop(false)
        }));
        cx.notify();
    });
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_search_mode(SelectSearch::Local, window, cx);
            s.set_value(SelectValue::Single(Some("a".into())), window, cx)
                .unwrap();
        })
    });
    draw(cx);
    cx.update(|window, cx| {
        caller.focus(window, cx);
        let policy = window.focused(cx).unwrap();
        assert!(!policy.tab_stop);
        assert_eq!(policy.tab_index, 4);
    });
    click(cx, "展开选项");
    cx.simulate_input("cher");
    draw(cx);
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(s.value().clone(), window, cx).unwrap();
            assert!(s.is_open());
            assert_eq!(s.query(cx), "cher");
            assert!(s.content_focus(cx).is_focused(window));
        })
    });
    key(cx, "escape");
    cx.update(|window, _| assert!(caller.is_focused(window)));
    assert_eq!(change_count(&view, cx), 0);
}

#[gpui_kit::test]
fn selection_limit_is_atomic_and_custom_filter_keeps_group_identity(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Multiple);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_options(
                [SelectGroup::new(
                    "one",
                    "Group",
                    [
                        SelectOption::new("a", "Same"),
                        SelectOption::new("c", "Same"),
                    ],
                )],
                cx,
            )
            .unwrap();
            s.set_value(
                SelectValue::Multiple(vec!["a".into(), "c".into()]),
                window,
                cx,
            )
            .unwrap();
            assert_eq!(
                s.set_max_selected(Some(1), cx),
                Err(SelectError::SelectionLimit)
            );
            assert_eq!(s.value().keys().len(), 2);
            s.set_search_mode(SelectSearch::Local, window, cx);
            s.set_filter(|option, query| option.key().as_str() == query, cx);
        })
    });
    draw(cx);
    click(cx, "展开选项");
    cx.simulate_input("c");
    draw(cx);
    state.read_with(cx, |s, _| {
        assert_eq!(s.rows.len(), 2);
        assert!(matches!(&s.rows[0], model::Row::Group(key, _) if key == "one"));
        assert_eq!(s.active_key(), Some(&SharedString::from("c")));
    });
}

#[gpui_kit::test]
fn all_themes_preserve_roles_and_layout_cache_on_hover(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    for mode in [crate::ThemeMode::Light, crate::ThemeMode::Dark] {
        for accent in [crate::Accent::Azure, crate::Accent::Neutral] {
            cx.update(|_, cx| crate::set_theme(cx, crate::YueTheme::new(mode, accent)));
            draw(cx);
            click(cx, "展开选项");
            assert_eq!(find(cx, "Fruit").role(), Some(Role::ComboBox));
            assert_eq!(find(cx, "Banana").disabled(), Some(true));
            let geometry = state.read_with(cx, |s, _| s.list_cache.clone().unwrap());
            draw(cx);
            assert!(state.read_with(cx, |s, _| Rc::ptr_eq(
                &geometry,
                s.list_cache.as_ref().unwrap()
            )));
            key(cx, "escape");
            draw(cx);
        }
    }
}

struct ModalProbe {
    dialog: Entity<crate::YueDialogState>,
    select: Entity<YueSelectState>,
}
impl Render for ModalProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::YueDialog::new("select-modal", crate::current_theme(cx), &self.dialog)
            .title("Modal select")
            .child(YueSelect::new("modal-select", &self.select).accessibility_label("Modal fruit"))
    }
}
#[gpui_kit::test]
fn modal_select_uses_same_scope_and_escape_closes_only_top_surface(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            crate::YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap()
    });
    let mut entities = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let dialog = cx.new(crate::YueDialogState::new);
        let select = cx.new(|cx| {
            let mut state = YueSelectState::new(SelectMode::Single, window, cx);
            state
                .set_options([SelectOption::new("a", "Apple")], cx)
                .unwrap();
            state
        });
        entities = Some((dialog.clone(), select.clone()));
        let view = cx.new(|_| ModalProbe { dialog, select });
        gpui_kit::base::Root::new(view, window, cx)
    });
    let (dialog, select) = entities.unwrap();
    draw(cx);
    cx.update(|window, cx| dialog.update(cx, |s, cx| s.open(window, cx).unwrap()));
    draw(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(300));
    draw(cx);
    click(cx, "展开选项");
    assert_eq!(find(cx, "Apple").role(), Some(Role::ListBoxOption));
    key(cx, "escape");
    assert!(!select.read_with(cx, |s, _| s.is_open()));
    assert!(dialog.read_with(cx, |s, _| s.is_open()));
    cx.update(|window, cx| {
        select.update(cx, |state, cx| {
            state
                .set_value(SelectValue::Single(Some("a".into())), window, cx)
                .unwrap();
        })
    });
    draw(cx);
    let frame = find(cx, "Modal fruit").bounds();
    assert_complete_focus_ring(cx, frame);
    key(cx, "escape");
    assert!(!dialog.read_with(cx, |s, _| s.is_open()));
}

#[gpui_kit::test]
fn typeahead_cycles_repeated_prefix_and_pointer_release_outside_does_not_commit(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx, SelectMode::Single);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |s, cx| {
            s.set_options(
                [
                    SelectOption::new("a", "Apple"),
                    SelectOption::new("b", "Apricot"),
                    SelectOption::new("c", "Avocado").disabled(true),
                ],
                cx,
            )
            .unwrap();
            s.focus.focus(window, cx);
        })
    });
    draw(cx);
    key(cx, "a");
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("a".into())
    );
    key(cx, "a");
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("b".into())
    );
    key(cx, "a");
    assert_eq!(
        state.read_with(cx, |s, _| s.active_key().cloned()),
        Some("a".into())
    );
    let position = find(cx, "Apple").bounds().center();
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    let outside = find(cx, "Before").bounds().center();
    cx.simulate_event(gpui_kit::MouseUpEvent {
        position: outside,
        button: gpui_kit::MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    draw(cx);
    assert_eq!(change_count(&view, cx), 0);
}
