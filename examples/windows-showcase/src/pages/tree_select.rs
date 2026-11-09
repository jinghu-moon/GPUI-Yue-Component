use super::super::*;
use gpui_kit::{Entity, px};
use std::time::Duration;
use yue_component::{
    SelectTagLayout, TreeCheckMode as Check, TreeCheckProjection as Projection, TreeNode,
    TreeSelectEvent as Event, TreeSelectMode as Mode, TreeSelectSearch as Search,
    TreeSelectValue as Value, TreeSelectValueDisplay, TreeSize, YueField, YueForm, YueTreeSelect,
    YueTreeSelectState,
};
pub(crate) struct TreeSelectPage {
    states: Vec<Entity<YueTreeSelectState>>,
    form: Entity<YueForm>,
    last: String,
    _subs: Vec<gpui_kit::Subscription>,
}
impl TreeSelectPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let modes = [
            Mode::Single,
            Mode::Multiple,
            Mode::Checked(Check::Strict),
            Mode::Checked(Check::Cascade),
            Mode::Single,
            Mode::Multiple,
            Mode::Single,
            Mode::Single,
            Mode::Single,
            Mode::Single,
            Mode::Multiple,
            Mode::Single,
        ];
        let states: Vec<_> = modes
            .into_iter()
            .enumerate()
            .map(|(i, mode)| {
                cx.new(|cx| {
                    let mut s = YueTreeSelectState::new(mode, w, cx).unwrap();
                    s.set_nodes(super::tree::nodes(), cx).unwrap();
                    s.set_search(
                        if i == 4 {
                            Search::Remote
                        } else {
                            Search::Local
                        },
                        w,
                        cx,
                    )
                    .unwrap();
                    if i == 5 {
                        s.set_value(
                            Value::Multiple(vec![
                                "main".into(),
                                "theme".into(),
                                "fonts".into(),
                                "images".into(),
                            ]),
                            w,
                            cx,
                        )
                        .unwrap()
                    }
                    if i == 6 {
                        s.set_disabled(true, w, cx)
                    }
                    if i == 7 {
                        s.set_readonly(true, w, cx)
                    }
                    if i == 9 {
                        s.set_leaves_only(true, cx).unwrap();
                    }
                    if i == 10 {
                        s.set_max_selected(Some(2), cx).unwrap();
                    }
                    s
                })
            })
            .collect();
        let subs = states
            .iter()
            .map(|s| {
                cx.subscribe_in(s, w, |page, state, event, w, cx| {
                    match event {
                        Event::Change(change) => {
                            page.last = format!("规范值：{:?}", change.value())
                        }
                        Event::QueryRequested(request) => {
                            let ticket = request.ticket().clone();
                            let query = request.query().clone();
                            let state = state.downgrade();
                            cx.spawn_in(w, async move |_, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(if query.len() % 2 == 0 {
                                        650
                                    } else {
                                        200
                                    }))
                                    .await;
                                let _ = state.update(cx, |s, cx| {
                                    if query.as_str() == "error" {
                                        s.reject_search(&ticket, "模拟查询失败", cx);
                                    } else {
                                        let _ = s.apply_search_results(
                                            &ticket,
                                            vec![TreeNode::new("results", "搜索结果").children(
                                                (0..5).map(|i| {
                                                    TreeNode::new(
                                                        format!("remote-{query}-{i}"),
                                                        format!("{query} 结果 {i}"),
                                                    )
                                                }),
                                            )],
                                            cx,
                                        );
                                    }
                                });
                            })
                            .detach();
                        }
                        Event::LoadRequested(ticket) => {
                            let ticket = ticket.clone();
                            let state = state.downgrade();
                            cx.spawn_in(w, async move |_, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(400))
                                    .await;
                                let _ = state.update(cx, |s, cx| {
                                    let _ = s.apply_children(
                                        &ticket,
                                        vec![
                                            TreeNode::new("lazy-a", "远程子节点 A"),
                                            TreeNode::new("lazy-b", "远程子节点 B"),
                                        ],
                                        cx,
                                    );
                                });
                            })
                            .detach();
                        }
                        _ => {}
                    }
                    cx.notify();
                })
            })
            .collect();
        let form = cx.new(|_| {
            let mut form = YueForm::new();
            form.require_tree_select("department", &states[8], "请选择一个部门");
            form
        });
        Self {
            states,
            form,
            last: "尚未更改选择".into(),
            _subs: subs,
        }
    }
    fn picker(&self, i: usize, name: &str) -> YueTreeSelect {
        YueTreeSelect::new(format!("tree-select-{i}"), &self.states[i])
            .accessibility_label(name)
            .clearable(true)
            .show_lines(true)
            .show_icons(true)
            .width(px(340.))
    }
}
impl Render for TreeSelectPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let form = self.form.clone();
        let fields = div().flex().flex_wrap().gap_6().children(
            [
                (
                    "分类单选",
                    self.picker(0, "分类单选")
                        .size(TreeSize::Small)
                        .value_display(TreeSelectValueDisplay::Path),
                ),
                ("独立多选", self.picker(1, "独立多选")),
                (
                    "Strict 勾选",
                    self.picker(2, "Strict 勾选").size(TreeSize::Large),
                ),
                (
                    "级联勾选",
                    self.picker(3, "级联勾选")
                        .check_projection(Projection::Parents),
                ),
                ("远程搜索", self.picker(4, "远程搜索")),
                (
                    "标签换行",
                    self.picker(5, "标签换行").tag_layout(SelectTagLayout::Wrap),
                ),
                ("禁用", self.picker(6, "禁用")),
                ("只读", self.picker(7, "只读")),
                (
                    "仅选叶子",
                    self.picker(9, "仅选叶子")
                        .prefix_icon(yue_component::outline::Folder),
                ),
                ("最多两项", self.picker(10, "最多两项")),
                (
                    "自定义面板",
                    self.picker(11, "自定义面板")
                        .variant(yue_component::SelectVariant::Text)
                        .panel_width(yue_component::SelectPanelWidth::Content)
                        .header(|_, _, _| div().text_sm().child("选择一个分类，或搜索节点"))
                        .footer(|_, _, _| div().text_sm().child("选择后立即生效，Escape 关闭")),
                ),
            ]
            .into_iter()
            .map(|(label, picker)| div().flex().flex_col().gap_2().child(label).child(picker)),
        );
        div().id("tree-select-page").overflow_y_scroll().h_full().flex().flex_col().gap_6().p_8()
            .child(div().text_2xl().child("TreeSelect 树选择器"))
            .child(section(theme,"层级字段","搜索在面板顶部；单选立即关闭，多选保持。Parents 标签仅压缩展示，提交仍为规范叶子值。",fields))
            .child(div().text_sm().child("远程示例输入 error 模拟失败；交替查询延迟用于观察旧结果丢弃。级联未知分支先加载，避免扩大选择范围。"))
            .child(section(theme,"表单验证","提交失败聚焦真实字段",div().flex().items_end().gap_4().child(YueField::new(theme).label("部门").required(true).bind(&self.form,"department").tree_select(self.picker(8,"部门"))).child(YueButton::new("tree-form-submit","提交").on_click(move|_,w,cx|{form.update(cx,|s,cx|{s.submit(w,cx);});}))))
            .child(div().child(self.last.clone()))
    }
}
