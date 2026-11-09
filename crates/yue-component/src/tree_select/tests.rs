use super::*;
use crate::{TreeCheckMode, TreeNode};
use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
use gpui_kit::{
    AppContext as _, Context, Modifiers, ParentElement as _, Render, Role, Styled as _,
    TestAppContext, VisualTestContext, Window, div, px,
};

struct Probe {
    state: Entity<YueTreeSelectState>,
    events: Vec<TreeSelectEvent>,
    _sub: gpui_kit::Subscription,
    visible: bool,
    slots: bool,
    panel_content: bool,
}
fn nodes() -> Vec<TreeNode> {
    vec![
        TreeNode::new("p", "Parent").children([
            TreeNode::new("a", "Alpha"),
            TreeNode::new("b", "Beta"),
            TreeNode::new("locked", "Locked").disabled(true),
        ]),
        TreeNode::new("u", "Unloaded").unloaded(),
    ]
}
impl Probe {
    fn new(mode: TreeSelectMode, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            let mut s = YueTreeSelectState::new(mode, w, cx).unwrap();
            s.set_nodes(nodes(), cx).unwrap();
            s.tree.update(cx, |t, cx| {
                t.set_expanded_keys(vec!["p".into()], cx).unwrap()
            });
            s
        });
        let sub = cx.subscribe(&state, |p, _, e, cx| {
            p.events.push(e.clone());
            cx.notify();
        });
        Self {
            state,
            events: vec![],
            _sub: sub,
            visible: true,
            slots: false,
            panel_content: false,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .flex()
            .flex_col()
            .w(px(600.))
            .child(crate::YueButton::new("before", "Before"))
            .children(self.visible.then(|| {
                let picker = YueTreeSelect::new("pick", &self.state)
                    .accessibility_label("Category")
                    .clearable(true)
                    .show_lines(true);
                let picker = if self.slots {
                    picker
                        .header(|_, _, _| div().h(px(64.)).child("Header"))
                        .footer(|_, _, _| div().h(px(48.)).child("Footer"))
                } else {
                    picker
                };
                if self.panel_content {
                    picker.panel_width(SelectPanelWidth::Content)
                } else {
                    picker
                }
            }))
            .child(crate::YueButton::new("after", "After"))
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
}
fn open(cx: &mut TestAppContext, mode: TreeSelectMode) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            crate::YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap();
        cx.set_reduce_motion(true);
    });
    let mut probe = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|cx| Probe::new(mode, w, cx));
        probe = Some(view.clone());
        gpui_kit::base::Root::new(view, w, cx)
    });
    draw(cx);
    draw(cx);
    (probe.unwrap(), cx)
}
fn find(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
    cx.update(|w, _| {
        let found: Vec<_> = snapshots(w)
            .into_iter()
            .filter(|s| s.label() == Some(label) && s.role() != Some(Role::Caption))
            .collect();
        assert_eq!(found.len(), 1, "{label}: {found:?}");
        found[0].clone()
    })
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let point = find(cx, label).bounds().center();
    cx.simulate_click(point, Modifiers::default());
    draw(cx);
    draw(cx);
}
fn key(cx: &mut VisualTestContext, k: &str) {
    let keystroke = gpui_kit::Keystroke::parse(k).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    draw(cx);
    draw(cx);
}
fn state(view: &Entity<Probe>, cx: &VisualTestContext) -> Entity<YueTreeSelectState> {
    view.read_with(cx, |v, _| v.state.clone())
}
fn changes(view: &Entity<Probe>, cx: &VisualTestContext) -> usize {
    view.read_with(cx, |v, _| {
        v.events
            .iter()
            .filter(|e| matches!(e, TreeSelectEvent::Change(_)))
            .count()
    })
}

#[gpui_kit::test]
fn tree_select_single_keyboard_navigation_does_not_commit(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    click(cx, "展开树选项");
    assert!(s.read_with(cx, |s, _| s.is_open()));
    key(cx, "down");
    assert!(s.read_with(cx, |s, cx| s.value(cx).is_empty()));
    key(cx, "enter");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Single(Some("a".into()))
    );
    assert!(!s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 1);
}
#[gpui_kit::test]
fn tree_select_multi_click_clear_keeps_locked_values(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_value(TreeSelectValue::Multiple(vec!["locked".into()]), w, cx)
                .unwrap()
        })
    });
    draw(cx);
    click(cx, "展开树选项");
    click(cx, "Alpha");
    assert!(s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 1);
    key(cx, "escape");
    click(cx, "清除可移除的选择");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Multiple(vec!["locked".into()])
    );
    assert_eq!(changes(&view, cx), 2);
}
#[gpui_kit::test]
fn tree_select_cascade_parent_tag_removes_leaf_range(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Checked(TreeCheckMode::Cascade));
    let s = state(&view, cx);
    click(cx, "展开树选项");
    click(cx, "Parent");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Checked(vec!["a".into(), "b".into()])
    );
    assert_eq!(changes(&view, cx), 1);
    key(cx, "escape");
    click(cx, "移除 Parent");
    assert!(s.read_with(cx, |s, cx| s.value(cx).is_empty()));
    assert_eq!(changes(&view, cx), 2);
}
#[gpui_kit::test]
fn tree_select_search_results_preserve_value_and_reject_late_response(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_search(TreeSelectSearch::Remote, w, cx).unwrap();
            s.set_value(TreeSelectValue::Single(Some("outside".into())), w, cx)
                .unwrap();
            s.set_open(true, w, cx);
        })
    });
    draw(cx);
    let ticket = view.read_with(cx, |p, _| {
        p.events
            .iter()
            .rev()
            .find_map(|e| match e {
                TreeSelectEvent::QueryRequested(q) => Some(q.ticket().clone()),
                _ => None,
            })
            .unwrap()
    });
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            assert_eq!(
                s.apply_search_results(&ticket, vec![TreeNode::new("new", "New")], cx)
                    .unwrap(),
                crate::TreeApply::Applied
            )
        })
    });
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Single(Some("outside".into()))
    );
    assert_eq!(changes(&view, cx), 0);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_open(false, w, cx);
            s.set_open(true, w, cx);
            assert_eq!(
                s.apply_search_results(&ticket, vec![], cx).unwrap(),
                crate::TreeApply::Stale
            );
        })
    });
}
#[gpui_kit::test]
fn tree_select_setter_echo_does_not_close_or_emit(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    click(cx, "展开树选项");
    click(cx, "Alpha");
    cx.update(|w, cx| s.update(cx, |s, cx| s.set_value(s.value(cx), w, cx).unwrap()));
    assert!(s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 1);
}
#[gpui_kit::test]
fn tree_select_form_requires_committed_value_and_focuses_trigger(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        let form = cx.new(|_| {
            let mut f = crate::YueForm::new();
            f.require_tree_select("category", &s, "Required");
            f
        });
        assert!(!form.update(cx, |f, cx| f.submit(w, cx)));
        assert!(s.read(cx).focus_handle().is_focused(w));
    });
}
#[gpui_kit::test]
fn tree_select_tree_roles_never_become_flat_options(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, TreeSelectMode::Checked(TreeCheckMode::Strict));
    click(cx, "展开树选项");
    let roles = cx.update(|w, _| snapshots(w));
    assert!(roles.iter().any(|s| s.role() == Some(Role::Tree)));
    assert!(
        roles
            .iter()
            .any(|s| s.role() == Some(Role::TreeItem) && s.label() == Some("Parent"))
    );
    assert!(!roles.iter().any(|s| matches!(
        s.role(),
        Some(Role::ListBox | Role::ListBoxOption | Role::CheckBox)
    )));
}
#[gpui_kit::test]
fn tree_select_remote_cascade_is_explicitly_rejected(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Checked(TreeCheckMode::Cascade));
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            assert_eq!(
                s.set_search(TreeSelectSearch::Remote, w, cx),
                Err(crate::TreeDataError::Mode)
            )
        })
    });
}

#[gpui_kit::test]
fn tree_arrow_checkbox_and_row_have_one_action_each(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Checked(TreeCheckMode::Cascade));
    let s = state(&view, cx);
    click(cx, "展开树选项");
    let tree = s.read_with(cx, |s, _| s.tree.clone());
    let before = tree.read_with(cx, |s, _| s.expanded_keys());
    click(cx, "折叠节点");
    assert!(tree.read_with(cx, |t, _| t.checked_keys().is_empty()));
    assert_eq!(changes(&view, cx), 0);
    assert_ne!(tree.read_with(cx, |s, _| s.expanded_keys()), before);
    let check_id = ElementId::from((
        ElementId::from((ElementId::from("pick"), "tree")),
        "check:p".to_string(),
    ));
    let check = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&check_id))
            .unwrap()
    });
    cx.simulate_click(check.bounds().center(), Modifiers::default());
    draw(cx);
    draw(cx);
    assert_eq!(
        tree.read_with(cx, |t, _| t.checked_keys().to_vec()),
        vec![SharedString::from("a"), "b".into()]
    );
    assert_eq!(changes(&view, cx), 1);
    click(cx, "Parent");
    assert!(tree.read_with(cx, |t, _| t.checked_keys().is_empty()));
    assert_eq!(changes(&view, cx), 2);
}

#[gpui_kit::test]
fn tree_select_branch_animation_keeps_field_value_and_session(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        w.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    click(cx, "展开树选项");
    let tree = s.read_with(cx, |s, _| s.tree.clone());
    click(cx, "折叠节点");
    let initial_extent = tree.read_with(cx, |s, cx| s.virtualizer.read(cx).total_extent());
    assert!(s.read_with(cx, |s, cx| s.value(cx).is_empty()));
    assert_eq!(changes(&view, cx), 0);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(50));
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    draw(cx);
    assert!(tree.read_with(cx, |s, cx| s.virtualizer.read(cx).total_extent()) < initial_extent);
    assert!(s.read_with(cx, |s, _| s.is_open()));
    assert!(cx.update(|w, _| snapshots(w).iter().all(|n| n.label() != Some("Alpha"))));
    key(cx, "escape");
    assert!(!s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 0);
}

fn last_request(view: &Entity<Probe>, cx: &VisualTestContext) -> TreeSelectSearchTicket {
    view.read_with(cx, |p, _| {
        p.events
            .iter()
            .rev()
            .find_map(|e| match e {
                TreeSelectEvent::QueryRequested(q) => Some(q.ticket().clone()),
                _ => None,
            })
            .unwrap()
    })
}
#[gpui_kit::test]
fn tree_select_unmount_invalidates_remote_session(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_search(TreeSelectSearch::Remote, w, cx).unwrap();
            s.set_open(true, w, cx);
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
        s.update(cx, |s, cx| {
            assert!(!s.is_open());
            assert_eq!(
                s.apply_search_results(&ticket, vec![], cx).unwrap(),
                crate::TreeApply::Stale
            );
        })
    });
}
#[gpui_kit::test]
fn tree_select_unknown_snapshot_lock_survives_clear(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_value(
                TreeSelectValue::Multiple(vec!["missing".into(), "a".into()]),
                w,
                cx,
            )
            .unwrap();
            s.set_selected_items(
                [TreeSelectItemSnapshot::new("missing", "External locked").locked(true)],
                cx,
            );
        })
    });
    draw(cx);
    click(cx, "清除可移除的选择");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Multiple(vec!["missing".into()])
    );
    assert_eq!(changes(&view, cx), 1);
}
#[gpui_kit::test]
fn tree_select_backspace_focuses_tag_before_removal(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_value(
                TreeSelectValue::Multiple(vec!["a".into(), "b".into()]),
                w,
                cx,
            )
            .unwrap();
            s.focus(w, cx);
        })
    });
    draw(cx);
    key(cx, "backspace");
    assert_eq!(changes(&view, cx), 0);
    assert_eq!(s.read_with(cx, |s, cx| s.value(cx).keys().len()), 2);
    key(cx, "backspace");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Multiple(vec!["a".into()])
    );
    assert_eq!(changes(&view, cx), 1);
}
#[gpui_kit::test]
fn tree_select_nonselectable_parent_does_not_commit_or_close(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.set_nodes(
                [TreeNode::new("p", "Parent")
                    .selectable(false)
                    .children([TreeNode::new("a", "Alpha")])],
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    click(cx, "展开树选项");
    click(cx, "Parent");
    assert!(s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 0);
    click(cx, "展开节点");
    click(cx, "Alpha");
    assert!(!s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 1);
}
#[gpui_kit::test]
fn tree_select_limit_allows_removal_but_no_new_value(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|_, cx| s.update(cx, |s, cx| s.set_max_selected(Some(1), cx).unwrap()));
    click(cx, "展开树选项");
    click(cx, "Alpha");
    click(cx, "Beta");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Multiple(vec!["a".into()])
    );
    assert_eq!(changes(&view, cx), 1);
    click(cx, "Alpha");
    click(cx, "Beta");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Multiple(vec!["b".into()])
    );
    assert_eq!(changes(&view, cx), 3);
}
#[gpui_kit::test]
fn tree_select_ime_never_commits_closes_or_filters_candidate(cx: &mut TestAppContext) {
    use gpui_kit::EntityInputHandler as _;
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_search(TreeSelectSearch::Local, w, cx).unwrap()
        })
    });
    click(cx, "展开树选项");
    let input = s.read_with(cx, |s, cx| s.query_input(cx));
    cx.update(|w, cx| {
        input.update(cx, |i, cx| {
            i.replace_and_mark_text_in_range(None, "Alpha", None, w, cx)
        })
    });
    key(cx, "enter");
    assert!(
        s.read_with(cx, |s, _| s.is_open()),
        "preedit Enter must keep popup open"
    );
    assert_eq!(changes(&view, cx), 0);
    cx.update(|w, cx| {
        input.update(cx, |i, cx| {
            i.replace_and_mark_text_in_range(None, "Alpha", None, w, cx)
        })
    });
    cx.update(|w, cx| {
        assert!(
            input.update(cx, |i, cx| i.marked_text_range(w, cx).is_some()),
            "Escape setup must have preedit"
        )
    });
    key(cx, "escape");
    assert!(
        s.read_with(cx, |s, _| s.is_open()),
        "preedit Escape must keep popup open"
    );
    assert_eq!(changes(&view, cx), 0);
    cx.update(|w, cx| input.update(cx, |i, cx| i.unmark_text(w, cx)));
    draw(cx);
    key(cx, "down");
    key(cx, "down");
    key(cx, "enter");
    assert_eq!(
        s.read_with(cx, |s, cx| s.value(cx)),
        TreeSelectValue::Single(Some("a".into()))
    );
}
#[gpui_kit::test]
fn tree_select_remote_query_blocks_old_choices_and_rejects_previous_ticket(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_search(TreeSelectSearch::Remote, w, cx).unwrap();
            s.set_open(true, w, cx);
        })
    });
    draw(cx);
    let old = last_request(&view, cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.apply_search_results(&old, nodes(), cx).unwrap();
        })
    });
    draw(cx);
    cx.update(|w, cx| s.update(cx, |s, cx| s.set_query("Beta", w, cx)));
    draw(cx);
    assert_eq!(
        s.read_with(cx, |s, _| s.result_state().clone()),
        TreeSelectResultState::Loading
    );
    key(cx, "enter");
    assert_eq!(changes(&view, cx), 0);
    assert!(s.read_with(cx, |s, _| s.is_open()));
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            assert_eq!(
                s.apply_search_results(&old, nodes(), cx).unwrap(),
                crate::TreeApply::Stale
            )
        })
    });
}
#[gpui_kit::test]
fn tree_select_authoritative_update_removes_missing_values_but_reports_them(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_value(
                TreeSelectValue::Multiple(vec!["a".into(), "missing".into()]),
                w,
                cx,
            )
            .unwrap();
            let report = s.set_nodes([TreeNode::new("a", "Renamed")], cx).unwrap();
            assert_eq!(report.removed_selected(), ["missing"]);
            assert_eq!(s.selected_items(cx)[0].label(), "Renamed");
        })
    });
    assert_eq!(changes(&view, cx), 0);
}
#[gpui_kit::test]
fn tree_select_invalid_setter_preserves_open_session_and_value(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    click(cx, "展开树选项");
    click(cx, "Alpha");
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            assert!(
                s.set_value(
                    TreeSelectValue::Multiple(vec!["a".into(), "a".into()]),
                    w,
                    cx
                )
                .is_err()
            );
            assert!(s.is_open());
            assert_eq!(s.value(cx), TreeSelectValue::Multiple(vec!["a".into()]));
        })
    });
    assert_eq!(changes(&view, cx), 1);
}

#[gpui_kit::test]
fn tree_select_tab_leaves_whole_field_and_reverse_returns_before(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_value(
                TreeSelectValue::Multiple(vec!["a".into(), "b".into()]),
                w,
                cx,
            )
            .unwrap();
            s.focus(w, cx);
        })
    });
    draw(cx);
    key(cx, "down");
    assert!(s.read_with(cx, |s, _| s.is_open()));
    key(cx, "tab");
    assert!(!s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(find(cx, "After").focused(), Some(true));
}
#[gpui_kit::test]
fn tree_select_content_width_and_measured_slots_reserve_space(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.set_nodes(
                [TreeNode::new(
                    "long",
                    "An exceptionally long category name that needs a wider popup than its trigger",
                )],
                cx,
            )
            .unwrap();
        })
    });
    view.update(cx, |v, cx| {
        v.slots = true;
        v.panel_content = true;
        cx.notify();
    });
    draw(cx);
    click(cx, "展开树选项");
    draw(cx);
    draw(cx);
    let (panel, frame) = cx.update(|w, _| {
        let all = snapshots(w);
        (
            all.iter()
                .find(|n| {
                    n.path().last() == Some(&ElementId::from((ElementId::from("pick"), "panel")))
                })
                .unwrap()
                .clone(),
            all.iter()
                .find(|n| {
                    n.path().last() == Some(&ElementId::from((ElementId::from("pick"), "frame")))
                })
                .unwrap()
                .clone(),
        )
    });
    assert!(panel.bounds().size.width > frame.bounds().size.width);
    assert_eq!(s.read_with(cx, |s, _| s.slot_heights), [px(64.), px(48.)]);
    let row = find(
        cx,
        "An exceptionally long category name that needs a wider popup than its trigger",
    );
    assert!(row.bounds().bottom() <= panel.bounds().bottom());
}
#[gpui_kit::test]
fn tree_select_dynamic_readonly_closes_and_blocks_old_row(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Single);
    let s = state(&view, cx);
    click(cx, "展开树选项");
    cx.update(|w, cx| s.update(cx, |s, cx| s.set_readonly(true, w, cx)));
    draw(cx);
    assert!(!s.read_with(cx, |s, _| s.is_open()));
    key(cx, "enter");
    assert!(!s.read_with(cx, |s, _| s.is_open()));
    assert_eq!(changes(&view, cx), 0);
}

#[gpui_kit::test]
fn tree_select_collapsed_tags_are_keyboard_reachable_and_removable(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, TreeSelectMode::Multiple);
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            s.set_value(
                TreeSelectValue::Multiple(vec!["p".into(), "a".into(), "b".into()]),
                w,
                cx,
            )
            .unwrap()
        })
    });
    draw(cx);
    let more = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label().is_some_and(|s| s.starts_with("查看其余")))
            .unwrap()
    });
    cx.simulate_click(more.bounds().center(), Modifiers::default());
    draw(cx);
    draw(cx);
    assert!(s.read_with(cx, |s, _| s.collapsed));
    key(cx, "backspace");
    assert_eq!(changes(&view, cx), 1);
    assert_eq!(s.read_with(cx, |s, cx| s.value(cx).keys().len()), 2);
    key(cx, "escape");
    assert!(!s.read_with(cx, |s, _| s.collapsed));
}
