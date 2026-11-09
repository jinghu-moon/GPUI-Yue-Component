use super::*;
use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
use gpui_kit::{
    AppContext as _, Context, EntityInputHandler as _, Modifiers, ParentElement as _, Render,
    Styled as _, TestAppContext, VisualTestContext, Window, div, px,
};
struct Probe {
    state: Entity<YueTreeState>,
    events: Vec<TreeEvent>,
    _sub: gpui_kit::Subscription,
    size: TreeSize,
    action_handles: Vec<gpui_kit::FocusHandle>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let handles = self.action_handles.clone();
        div().w(px(360.)).child(
            YueTree::new("tree", &self.state)
                .height(px(192.))
                .size(self.size)
                .show_lines(true)
                .searchable(true)
                .render_actions(move |node, _, _| {
                    let mut row = div().flex();
                    if node.key() == "a" {
                        for (i, h) in handles.iter().enumerate() {
                            row = row.child(
                                crate::YueButton::new(format!("action-{i}"), format!("Action {i}"))
                                    .track_focus(h)
                                    .tab_stop(false),
                            );
                        }
                    }
                    row
                }),
        )
    }
}
fn open(cx: &mut TestAppContext, large: bool) -> (Entity<Probe>, &mut VisualTestContext) {
    open_in_mode(cx, large, TreeSelectionMode::Multiple)
}
fn open_in_mode(
    cx: &mut TestAppContext,
    large: bool,
    selection: TreeSelectionMode,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            crate::YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap();
        cx.set_reduce_motion(true);
    });
    let mut view = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let probe = cx.new(|cx| {
            let state = cx.new(|cx| {
                let mut s = YueTreeState::new(selection, TreeCheckMode::Strict, w, cx);
                s.set_nodes(
                    if large {
                        (0..10000)
                            .map(|i| TreeNode::new(format!("n{i}"), format!("Node {i}")))
                            .collect()
                    } else {
                        vec![
                            TreeNode::new("p", "Parent").children([
                                TreeNode::new("a", "Alpha"),
                                TreeNode::new("b", "Beta"),
                                TreeNode::new("d", "Disabled").disabled(true),
                            ]),
                            TreeNode::new("u", "Unloaded").unloaded(),
                        ]
                    },
                    cx,
                )
                .unwrap();
                if !large {
                    s.set_expanded_keys(vec!["p".into()], cx).unwrap();
                }
                s
            });
            let sub = cx.subscribe(&state, |p: &mut Probe, e, event, cx| {
                let _ = e;
                p.events.push(event.clone());
                cx.notify();
            });
            let action_handles = vec![cx.focus_handle(), cx.focus_handle()];
            if !large {
                state.update(cx, |s, cx| {
                    s.set_row_action_handles("a", action_handles.clone(), cx)
                        .unwrap()
                });
            }
            Probe {
                action_handles,
                state,
                events: vec![],
                _sub: sub,
                size: TreeSize::Medium,
            }
        });
        view = Some(probe.clone());
        gpui_kit::base::Root::new(probe, w, cx)
    });
    draw(cx);
    draw(cx);
    (view.unwrap(), cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
}
fn key(cx: &mut VisualTestContext, k: &str) {
    let k = gpui_kit::Keystroke::parse(k).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: k.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: k });
    draw(cx);
    draw(cx);
}
fn row(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|s| s.role() == Some(gpui_kit::Role::TreeItem) && s.label() == Some(label))
            .unwrap()
    })
}
fn click(cx: &mut VisualTestContext, label: &str, modifiers: Modifiers) {
    let bounds = row(cx, label).bounds();
    let point = gpui_kit::point(bounds.left() + px(100.), bounds.center().y);
    cx.simulate_click(point, modifiers);
    draw(cx);
    draw(cx);
}
#[gpui_kit::test]
fn tree_single_click_selects_and_enter_activates(cx: &mut TestAppContext) {
    let (view, cx) = open_in_mode(cx, false, TreeSelectionMode::Single);
    click(cx, "Alpha", Modifiers::default());
    assert_eq!(
        view.read_with(cx, |v, cx| v.state.read(cx).selected_keys().to_vec()),
        [SharedString::from("a")]
    );
    assert!(!view.read_with(cx, |v, _| {
        v.events
            .iter()
            .any(|e| matches!(e, TreeEvent::Activate { .. }))
    }));
    key(cx, "enter");
    assert_eq!(
        view.read_with(cx, |v, _| v
            .events
            .iter()
            .filter(|e| matches!(e, TreeEvent::Activate { key, .. } if key == "a"))
            .count()),
        1
    );
}
#[gpui_kit::test]
fn tree_pointer_selection_and_checkbox_are_independent(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    click(cx, "Alpha", Modifiers::default());
    click(
        cx,
        "Beta",
        Modifiers {
            control: true,
            ..Default::default()
        },
    );
    assert_eq!(
        s.read_with(cx, |s, _| s.selected_keys().to_vec()),
        vec![SharedString::from("a"), "b".into()]
    );
    assert!(s.read_with(cx, |s, _| s.checked_keys().is_empty()));
    key(cx, "space");
    assert_eq!(
        s.read_with(cx, |s, _| s.checked_keys().to_vec()),
        vec![SharedString::from("b")]
    );
    assert_eq!(
        view.read_with(cx, |p, _| p
            .events
            .iter()
            .filter(|e| matches!(e, TreeEvent::CheckChanged(_)))
            .count()),
        1
    );
}
#[gpui_kit::test]
fn tree_keyboard_shift_range_captures_previous_active(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|w, cx| s.update(cx, |s, cx| s.focus(w, cx)));
    key(cx, "down");
    assert!(s.read_with(cx, |s, _| s.selected_keys().is_empty()));
    key(cx, "shift-down");
    assert_eq!(
        s.read_with(cx, |s, _| s.selected_keys().to_vec()),
        vec![SharedString::from("a"), "b".into()]
    );
    key(cx, "down");
    assert_eq!(
        s.read_with(cx, |s, _| s.active_key().cloned()).unwrap(),
        "u"
    );
}
#[gpui_kit::test]
fn tree_search_is_temporary_and_composition_uses_real_input(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    let input = s.read_with(cx, |s, _| s.query_input().clone());
    cx.update(|w, cx| {
        s.update(cx, |s, cx| s.set_expanded_keys(vec![], cx).unwrap());
        input.update(cx, |i, cx| {
            i.replace_and_mark_text_in_range(None, "Alpha", None, w, cx)
        });
    });
    draw(cx);
    assert_eq!(s.read_with(cx, |s, _| s.model.rows().len()), 2);
    cx.update(|w, cx| input.update(cx, |i, cx| i.unmark_text(w, cx)));
    draw(cx);
    assert_eq!(
        s.read_with(cx, |s, _| s
            .model
            .rows()
            .iter()
            .map(|r| r.key().to_string())
            .collect::<Vec<_>>()),
        ["p", "a"]
    );
    cx.update(|w, cx| s.update(cx, |s, cx| s.set_query("", w, cx)));
    draw(cx);
    assert!(s.read_with(cx, |s, _| s.expanded_keys().is_empty()));
}
#[gpui_kit::test]
fn tree_keyboard_can_retry_failed_unloaded_branch(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|w, cx| {
        s.update(cx, |s, cx| s.reveal("u", cx).unwrap());
        s.update(cx, |s, cx| s.focus(w, cx));
    });
    draw(cx);
    key(cx, "right");
    let ticket = view.read_with(cx, |p, _| {
        p.events
            .iter()
            .rev()
            .find_map(|e| {
                if let TreeEvent::LoadRequested(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .unwrap()
    });
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.reject_load(&ticket, "Offline", cx);
        })
    });
    draw(cx);
    key(cx, "right");
    assert_eq!(
        view.read_with(cx, |p, _| p
            .events
            .iter()
            .filter(|e| matches!(e, TreeEvent::LoadRequested(_)))
            .count()),
        2
    );
}
#[gpui_kit::test]
fn tree_ten_thousand_nodes_render_only_viewport_and_retained_active(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, true);
    let s = view.read_with(cx, |v, _| v.state.clone());
    let count = cx.update(|w, _| {
        snapshots(w)
            .iter()
            .filter(|s| s.role() == Some(gpui_kit::Role::TreeItem))
            .count()
    });
    assert!(count < 60, "rendered {count} rows");
    let rows = s.read_with(cx, |s, _| s.model.rows());
    cx.update(|w, cx| s.update(cx, |s, cx| s.focus(w, cx)));
    key(cx, "end");
    assert_eq!(
        s.read_with(cx, |s, _| s.active_key().cloned()).unwrap(),
        "n9999"
    );
    let node = row(cx, "Node 9999");
    assert!(node.bounds().size.height > px(0.));
    let after = s.read_with(cx, |s, _| s.model.rows());
    assert!(
        Rc::ptr_eq(&rows, &after),
        "navigation must not rebuild projection"
    );
    key(cx, "enter");
    assert_eq!(
        s.read_with(cx, |s, _| s.selected_keys().to_vec()),
        vec![SharedString::from("n9999")]
    );
    assert!(Rc::ptr_eq(&rows, &s.read_with(cx, |s, _| s.model.rows())));
}
#[gpui_kit::test]
fn tree_row_geometry_follows_three_sizes_and_rem_scale(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    assert_eq!(row(cx, "Parent").bounds().size.height, px(32.));
    view.update(cx, |p, cx| {
        p.size = TreeSize::Large;
        cx.notify();
    });
    draw(cx);
    assert_eq!(row(cx, "Parent").bounds().size.height, px(40.));
    cx.update(|w, _| w.set_rem_size(px(20.)));
    draw(cx);
    draw(cx);
    assert_eq!(row(cx, "Parent").bounds().size.height, px(50.));
}

#[gpui_kit::test]
fn tree_lines_align_with_ancestor_arrow_tips_at_all_sizes(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let state = view.read_with(cx, |v, _| v.state.clone());
    state.update(cx, |s, cx| {
        s.set_nodes(
            [
                TreeNode::new("p", "Parent").children([
                    TreeNode::new("q", "Branch")
                        .children([TreeNode::new("a", "Alpha"), TreeNode::new("b", "Beta")]),
                    TreeNode::new("z", "Sibling"),
                ]),
                TreeNode::new("u", "Unloaded").unloaded(),
            ],
            cx,
        )
        .unwrap();
        s.set_expanded_keys(vec!["p".into(), "q".into()], cx)
            .unwrap();
    });
    for rem in [16., 20.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        for size in [TreeSize::Small, TreeSize::Medium, TreeSize::Large] {
            view.update(cx, |p, cx| {
                p.size = size;
                cx.notify();
            });
            draw(cx);
            draw(cx);
            cx.update(|w, _| {
                let nodes = snapshots(w);
                let bounds = |id: &str| {
                    let id = gpui_kit::ElementId::from((gpui_kit::ElementId::from("tree"), id.to_owned()));
                    nodes.iter().find(|n| n.path().last() == Some(&id)).unwrap().bounds()
                };
                for (line, arrow) in [("line:a:0", "expand:p"), ("line:a:1", "expand:q")] {
                    let line_x = bounds(line).center().x;
                    let arrow_x = bounds(arrow).center().x;
                    assert!(
                        (line_x - arrow_x).abs() < px(0.1),
                        "{size:?}, rem={rem}: {line} center {line_x:?} must align with {arrow} tip {arrow_x:?}"
                    );
                }
            });
        }
    }
}

#[gpui_kit::test]
fn tree_action_content_cannot_shift_full_height_line_guides(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    for rem in [16., 20.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        for size in [TreeSize::Small, TreeSize::Medium, TreeSize::Large] {
            view.update(cx, |p, cx| {
                p.size = size;
                cx.notify();
            });
            draw(cx);
            draw(cx);
            let action_row = row(cx, "Alpha").bounds();
            let following_row = row(cx, "Beta").bounds();
            cx.update(|w, _| {
                let nodes = snapshots(w);
                let id = gpui_kit::ElementId::from((gpui_kit::ElementId::from("tree"), "line:a:0"));
                let guide = nodes
                    .iter()
                    .find(|n| n.path().last() == Some(&id))
                    .unwrap()
                    .bounds();
                assert!(
                    (guide.top() - action_row.top()).abs() < px(0.1),
                    "{size:?}, rem={rem}: actions shifted guide top {:?} from row {:?}",
                    guide.top(),
                    action_row.top()
                );
                assert!(
                    (guide.bottom() - following_row.top()).abs() < px(0.1),
                    "{size:?}, rem={rem}: guide must reach the next row"
                );
            });
        }
    }
}

#[gpui_kit::test]
fn tree_f2_enters_registered_actions_and_escape_restores_tree(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |p, _| p.state.clone());
    click(cx, "Alpha", Modifiers::default());
    key(cx, "f2");
    let handles = view.read_with(cx, |p, _| p.action_handles.clone());
    assert!(cx.update(|w, _| handles[0].is_focused(w)));
    key(cx, "tab");
    assert!(
        cx.update(|w, _| handles[1].is_focused(w)),
        "action state {:?}, focus {:?}",
        s.read_with(cx, |s, _| s.action_row.clone()),
        cx.update(|w, _| snapshots(w)
            .into_iter()
            .filter(|s| s.focused() == Some(true))
            .collect::<Vec<_>>())
    );
    key(cx, "escape");
    assert!(cx.update(|w, cx| s.read(cx).focus_handle().is_focused(w)));
    assert_eq!(
        s.read_with(cx, |s, _| s.selected_keys().to_vec()),
        vec![SharedString::from("a")]
    );
    key(cx, "f2");
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.set_nodes([TreeNode::new("p", "Parent")], cx).unwrap()
        })
    });
    draw(cx);
    assert!(cx.update(|w, cx| s.read(cx).focus_handle().is_focused(w)));
}

fn arrow(cx: &mut VisualTestContext, label: &str) {
    let bounds = cx.update(|w, _| {
        let nodes = snapshots(w);
        let parent = nodes
            .iter()
            .find(|s| {
                s.role() == Some(gpui_kit::Role::TreeItem)
                    && matches!(s.label(), Some("Parent" | "Huge"))
            })
            .unwrap()
            .bounds();
        nodes
            .into_iter()
            .find(|s| s.label() == Some(label) && parent.contains(&s.bounds().center()))
            .unwrap()
            .bounds()
    });
    cx.simulate_click(bounds.center(), Modifiers::default());
    draw(cx);
}
fn enable_motion(cx: &mut VisualTestContext) {
    cx.update(|w, cx| {
        w.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    draw(cx);
}
fn motion_frame(cx: &mut VisualTestContext, ms: u64) -> usize {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(ms));
    let callbacks = cx.update(|w, cx| w.simulate_next_frame(cx));
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
    callbacks
}
fn total(s: &Entity<YueTreeState>, cx: &VisualTestContext) -> f64 {
    s.read_with(cx, |s, cx| s.virtualizer.read(cx).total_extent())
}
#[gpui_kit::test]
fn tree_motion_collapses_geometry_but_detaches_children_immediately(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    enable_motion(cx);
    draw(cx);
    click(cx, "Alpha", Modifiers::default());
    let expanded = total(&s, cx);
    arrow(cx, "折叠节点");
    assert!(
        s.read_with(cx, |s, _| s.motion.running()),
        "expanded {:?}, rows {}, width {:?}, active {:?}",
        s.read_with(cx, |s, _| s.expanded_keys()),
        s.read_with(cx, |s, _| s.motion.rows.len()),
        s.read_with(cx, |s, _| s.viewport_width),
        cx.update(|w, cx| (w.is_window_active(), w.is_visible(), cx.reduce_motion()))
    );
    assert!((total(&s, cx) - expanded).abs() < 0.1);
    assert!(s.read_with(cx, |s, _| s.motion.ghost(&"a".into()).is_some()));
    assert!(cx.update(|w, _| snapshots(w).iter().all(|n| n.label() != Some("Alpha"))));
    assert_eq!(
        s.read_with(cx, |s, _| s.active_key().cloned()),
        Some("p".into())
    );
    assert!(
        motion_frame(cx, 50) > 0,
        "motion must request its own frame"
    );
    assert!(total(&s, cx) < expanded && total(&s, cx) > 64.);
    motion_frame(cx, 1000);
    assert!(!s.read_with(cx, |s, _| s.motion.running()));
    assert!((total(&s, cx) - 64.).abs() < 0.1);
    assert_eq!(
        s.read_with(cx, |s, _| s.selected_keys().to_vec()),
        [SharedString::from("a")]
    );
    assert_eq!(
        view.read_with(cx, |v, _| v
            .events
            .iter()
            .filter(|e| matches!(e, TreeEvent::ExpansionChanged { .. }))
            .count()),
        1
    );
    assert_eq!(s.read_with(cx, |s, _| s.motion.rows.len()), 2);
}
#[gpui_kit::test]
fn tree_motion_reverse_continues_from_current_height_and_reduce_motion_settles(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    enable_motion(cx);
    draw(cx);
    arrow(cx, "折叠节点");
    motion_frame(cx, 50);
    let midway = total(&s, cx);
    arrow(cx, "展开节点");
    assert!(
        (total(&s, cx) - midway).abs() < 0.1,
        "reverse must not jump"
    );
    motion_frame(cx, 30);
    assert!(total(&s, cx) > midway);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    draw(cx);
    assert!(!s.read_with(cx, |s, _| s.motion.running()));
    assert!((total(&s, cx) - 160.).abs() < 0.1);
    assert_eq!(row(cx, "Alpha").bounds().size.height, px(32.));
}
#[gpui_kit::test]
fn tree_motion_keyboard_target_and_data_replace_cancel_stale_geometry(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    enable_motion(cx);
    draw(cx);
    arrow(cx, "折叠节点");
    motion_frame(cx, 50);
    arrow(cx, "展开节点");
    key(cx, "down");
    assert!(!s.read_with(cx, |s, _| s.motion.running()));
    assert_eq!(
        s.read_with(cx, |s, _| s.active_key().cloned()),
        Some("a".into())
    );
    assert_eq!(row(cx, "Alpha").bounds().size.height, px(32.));
    arrow(cx, "折叠节点");
    motion_frame(cx, 30);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.set_nodes([TreeNode::new("new", "Replacement")], cx)
                .unwrap()
        })
    });
    draw(cx);
    assert!(!s.read_with(cx, |s, _| s.motion.running()));
    assert!((total(&s, cx) - 32.).abs() < 0.1);
    assert_eq!(
        s.read_with(cx, |s, _| s.motion.rows[0].key().clone()),
        "new"
    );
}
#[gpui_kit::test]
fn tree_large_branch_motion_keeps_virtual_mounts_bounded(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.set_nodes(
                [TreeNode::new("huge", "Huge").children(
                    (0..10000).map(|i| TreeNode::new(format!("h{i}"), format!("Child {i}"))),
                )],
                cx,
            )
            .unwrap()
        })
    });
    draw(cx);
    enable_motion(cx);
    arrow(cx, "展开节点");
    assert!(
        !s.read_with(cx, |s, _| s.motion.running()),
        "huge subtree skips size tween"
    );
    assert_eq!(s.read_with(cx, |s, _| s.model.rows().len()), 10001);
    assert!(
        cx.update(|w, _| snapshots(w)
            .iter()
            .filter(|n| n.role() == Some(gpui_kit::Role::TreeItem))
            .count())
            < 60
    );
    key(cx, "end");
    assert_eq!(
        s.read_with(cx, |s, _| s.active_key().cloned()),
        Some("h9999".into())
    );
}

#[gpui_kit::test]
fn tree_motion_window_deactivation_and_rem_change_settle_safely(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = view.read_with(cx, |v, _| v.state.clone());
    enable_motion(cx);
    arrow(cx, "折叠节点");
    motion_frame(cx, 30);
    assert!(s.read_with(cx, |s, _| s.motion.running()));
    cx.deactivate_window();
    draw(cx);
    assert!(!s.read_with(cx, |s, _| s.motion.running()));
    assert!((total(&s, cx) - 64.).abs() < 0.1);
    enable_motion(cx);
    arrow(cx, "展开节点");
    motion_frame(cx, 30);
    cx.update(|w, _| w.set_rem_size(px(20.)));
    draw(cx);
    assert!(!s.read_with(cx, |s, _| s.motion.running()));
    assert!((total(&s, cx) - 200.).abs() < 0.1);
    assert_eq!(row(cx, "Alpha").bounds().size.height, px(40.));
}
