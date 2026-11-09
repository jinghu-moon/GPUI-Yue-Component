use super::super::*;
use gpui_kit::{Entity, px};
use std::time::Duration;
use yue_component::{
    TreeCheckMode as Check, TreeCheckProjection, TreeEvent, TreeNode, TreeRowAction,
    TreeSelectionMode as Selection, TreeSize, YueTree, YueTreeState,
};
pub(crate) fn nodes() -> Vec<TreeNode> {
    vec![
        TreeNode::new("work", "工作空间").children([
            TreeNode::new("src", "源代码").children([
                TreeNode::new("main", "main.rs"),
                TreeNode::new("theme", "theme.rs"),
                TreeNode::new("readonly", "只读配置").check_disabled(true),
            ]),
            TreeNode::new("docs", "设计文档").children([
                TreeNode::new("tree-doc", "Tree 树组件"),
                TreeNode::new("select-doc", "TreeSelect 树选择器"),
            ]),
            TreeNode::new("remote", "远程目录").unloaded(),
        ]),
        TreeNode::new("archive", "归档").disabled(true),
        TreeNode::new("assets", "资源文件").children([
            TreeNode::new("images", "图片"),
            TreeNode::new("fonts", "字体"),
        ]),
    ]
}
pub(crate) struct TreePage {
    states: Vec<Entity<YueTreeState>>,
    last: String,
    _subs: Vec<gpui_kit::Subscription>,
    action_focus: gpui_kit::FocusHandle,
    failed_once: std::collections::HashSet<(gpui_kit::EntityId, String)>,
}
impl TreePage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let modes = [
            (Selection::Single, Check::None),
            (Selection::Multiple, Check::None),
            (Selection::None, Check::Cascade),
            (Selection::None, Check::Strict),
            (Selection::Single, Check::None),
        ];
        let states: Vec<_> = modes
            .into_iter()
            .enumerate()
            .map(|(index, (selection, check))| {
                cx.new(|cx| {
                    let mut s = YueTreeState::new(selection, check, w, cx);
                    s.set_nodes(
                        if index == 4 {
                            (0..100)
                                .map(|group| {
                                    TreeNode::new(format!("group-{group}"), format!("分组 {group}"))
                                        .children((0..100).map(|row| {
                                            TreeNode::new(
                                                format!("item-{group}-{row}"),
                                                format!("节点 {group} / {row}"),
                                            )
                                        }))
                                })
                                .collect()
                        } else {
                            nodes()
                        },
                        cx,
                    )
                    .unwrap();
                    s.set_expanded_keys(
                        if index == 4 {
                            (0..100).map(|i| format!("group-{i}").into()).collect()
                        } else {
                            vec!["work".into(), "src".into(), "docs".into()]
                        },
                        cx,
                    )
                    .unwrap();
                    if check != Check::None {
                        s.set_row_action(TreeRowAction::Check, cx);
                    }
                    s
                })
            })
            .collect();
        let subs = states
            .iter()
            .map(|s| {
                cx.subscribe_in(s, w, |this, state, event, w, cx| {
                    this.last = format!("{event:?}");
                    if let TreeEvent::LoadRequested(ticket) = event {
                        let ticket = ticket.clone();
                        let should_fail = this.failed_once.insert((
                            state.entity_id(),
                            ticket.target().map_or("root", |k| k.as_str()).to_string(),
                        ));
                        let state = state.downgrade();
                        cx.spawn_in(w, async move |_, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(500))
                                .await;
                            let _ = state.update(cx, |s, cx| {
                                if should_fail {
                                    s.reject_load(&ticket, "模拟首次加载失败，请重试", cx);
                                    return;
                                }
                                let target = ticket.target().map_or("root", |s| s.as_str());
                                let _ = s.apply_children(
                                    &ticket,
                                    vec![
                                        TreeNode::new(
                                            format!("{target}-child-1"),
                                            "加载的子节点 1",
                                        ),
                                        TreeNode::new(
                                            format!("{target}-child-2"),
                                            "加载的子节点 2",
                                        ),
                                    ],
                                    cx,
                                );
                            });
                        })
                        .detach();
                    }
                    cx.notify();
                })
            })
            .collect();
        let action_focus = cx.focus_handle();
        states[0].update(cx, |s, cx| {
            s.set_row_action_handles("main", vec![action_focus.clone()], cx)
                .unwrap()
        });
        Self {
            action_focus,
            failed_once: Default::default(),
            states,
            last: "选择、展开、勾选操作会显示在这里".into(),
            _subs: subs,
        }
    }
}
impl Render for TreePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let state = self.states[0].clone();
        let permission = self.states[2]
            .read(cx)
            .projected_checked(TreeCheckProjection::Parents);
        let action_focus = self.action_focus.clone();
        let sizes = div().flex().flex_wrap().gap_4().children(
            [TreeSize::Small, TreeSize::Medium, TreeSize::Large]
                .into_iter()
                .enumerate()
                .map(|(i, size)| {
                    let focus = action_focus.clone();
                    let widget = YueTree::new(format!("tree-size-{i}"), &self.states[i])
                        .size(size)
                        .height(px(260.))
                        .show_lines(true)
                        .show_icons(true);
                    let widget = if i == 0 {
                        widget.render_actions(move |node, _, _| {
                            div().children((node.key() == "main").then(|| {
                                YueButton::new("tree-row-inspect", "复制路径")
                                    .size(yue_component::ButtonSize::Small)
                                    .track_focus(&focus)
                                    .tab_stop(false)
                                    .on_click(|_, _, cx| {
                                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                            "src/main.rs".to_string(),
                                        ))
                                    })
                            }))
                        })
                    } else {
                        widget
                    };
                    div().w(px(320.)).child(widget)
                }),
        );
        let refresh = self.states[0].clone();
        let clear = self.states[0].clone();
        let updates = div()
            .flex()
            .gap_3()
            .child(
                YueButton::new("tree-update", "更新标题并保留 key").on_click(move |_, _, cx| {
                    refresh.update(cx, |s, cx| {
                        let mut n = nodes();
                        let children = match n[0].child_state().clone() {
                            yue_component::TreeChildren::Loaded(c) => c,
                            _ => vec![],
                        };
                        n[0] = TreeNode::new("work", "工作空间（已更新）").children(children);
                        let _ = s.set_nodes(n, cx);
                    })
                }),
            )
            .child(
                YueButton::new("tree-reset", "恢复示例数据").on_click(move |_, _, cx| {
                    clear.update(cx, |s, cx| {
                        let _ = s.set_nodes(nodes(), cx);
                    })
                }),
            );
        div()
            .id("tree-page")
            .overflow_y_scroll()
            .h_full()
            .flex()
            .flex_col()
            .gap_6()
            .p_8()
            .child(div().text_2xl().child("Tree 树"))
            .child(section(
                theme,
                "目录与三尺寸",
                "小号单选、中号 Ctrl / Shift 多选、大号级联勾选；箭头只开合",
                sizes,
            ))
            .child(
                YueButton::new("tree-reveal", "定位到字体目录").on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        let _ = s.reveal("fonts", cx);
                    })
                }),
            )
            .child(section(
                theme,
                "搜索与独立勾选",
                "只搜索已加载节点；清空恢复展开。Strict 的父子互不关联。",
                YueTree::new("tree-strict", &self.states[3])
                    .searchable(true)
                    .show_lines(true)
                    .height(px(280.)),
            ))
            .child(updates)
            .child(div().text_sm().child("焦点在 main.rs 时按 F2 进入复制路径按钮，Escape 返回树；远程目录首次失败，Right 或重试按钮可重试。"))
            .child(div().child(format!("级联 Parents 投影：{permission:?}")))
            .child(section(
                theme,
                "一万节点虚拟树",
                "100 个分组，每组 100 个子节点；所有分组已展开。",
                YueTree::new("tree-large", &self.states[4])
                    .searchable(true)
                    .height(px(320.))
                    .show_lines(true),
            ))
            .child(div().text_sm().child(self.last.clone()))
    }
}
