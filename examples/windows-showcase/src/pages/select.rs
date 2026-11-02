use super::super::*;
use gpui_kit::{Entity, FocusHandle, Subscription, px};
use std::time::Duration;
use yue_component::{
    SelectEntry, SelectEvent, SelectGroup, SelectMode, SelectOption, SelectPage, SelectPanelWidth,
    SelectRowLayout, SelectSearch, SelectSize, SelectTagLayout, SelectValue, SelectVariant,
    YueDialog, YueDialogState, YueSelect, YueSelectState, YueTag,
};

pub(crate) struct SelectPageView {
    states: Vec<Entity<YueSelectState>>,
    changes: usize,
    last_event: SharedString,
    form: Entity<YueForm>,
    footer_focus: FocusHandle,
    dialog: Entity<YueDialogState>,
    _subscriptions: Vec<Subscription>,
}
fn options() -> Vec<SelectEntry> {
    [
        SelectOption::new("rust", "Rust").leading_icon(outline::Code),
        SelectOption::new("typescript", "TypeScript")
            .leading_icon(outline::Code)
            .keywords(["ts", "前端"]),
        SelectOption::new("python", "Python").leading_icon(outline::Code),
        SelectOption::new("locked", "已停用的语言").disabled(true),
    ]
    .into_iter()
    .map(Into::into)
    .collect()
}
impl SelectPageView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let states = (0..22)
            .map(|ix| {
                cx.new(|cx| {
                    let multiple = (5..=11).contains(&ix) || ix == 13 || ix == 15 || ix == 20;
                    let mut s = YueSelectState::new(
                        if multiple {
                            SelectMode::Multiple
                        } else {
                            SelectMode::Single
                        },
                        window,
                        cx,
                    );
                    s.set_options(options(), cx).unwrap();
                    if [1, 11, 12, 15, 16, 18].contains(&ix) {
                        s.set_search_mode(SelectSearch::Local, window, cx);
                    }
                    if ix == 2 {
                        s.set_disabled(true, window, cx);
                    }
                    if ix == 3 {
                        s.set_value(SelectValue::Single(Some("rust".into())), window, cx)
                            .unwrap();
                        s.set_readonly(true, window, cx);
                    }
                    if ix == 4 {
                        s.set_loading(true, cx);
                    }
                    if (5..=11).contains(&ix) || ix == 13 {
                        s.set_value(
                            SelectValue::Multiple(vec![
                                "rust".into(),
                                "typescript".into(),
                                "locked".into(),
                            ]),
                            window,
                            cx,
                        )
                        .unwrap();
                    }
                    if ix == 13 {
                        s.set_max_selected(Some(3), cx).unwrap();
                    }
                    if ix == 12 {
                        s.set_options(
                            [
                                SelectGroup::new(
                                    "frontend",
                                    "前端语言",
                                    [
                                        SelectOption::new("ts", "TypeScript")
                                            .leading_icon(outline::Code)
                                            .with_description("类型安全的前端开发")
                                            .with_meta("TS"),
                                        SelectOption::new("js", "JavaScript")
                                            .with_description("浏览器原生脚本"),
                                    ],
                                ),
                                SelectGroup::new(
                                    "backend",
                                    "后端语言",
                                    [
                                        SelectOption::new("rs", "Rust")
                                            .leading_icon(outline::Code)
                                            .with_description("内存安全与高性能")
                                            .with_meta("推荐"),
                                        SelectOption::new("py", "Python")
                                            .with_description("自动化与数据分析"),
                                    ],
                                ),
                            ],
                            cx,
                        )
                        .unwrap();
                    }
                    if ix == 14 || ix == 19 {
                        s.set_search_mode(SelectSearch::Remote, window, cx);
                    }
                    if ix == 15 {
                        s.set_creatable(true, cx);
                    }
                    if ix == 16 {
                        s.set_options(
                            (0..10_000).map(|i| {
                                SelectOption::new(format!("item-{i}"), format!("选项 {i:05}"))
                            }),
                            cx,
                        )
                        .unwrap();
                    }
                    if ix == 20 {
                        s.set_value(SelectValue::Multiple(vec!["server".into()]), window, cx)
                            .unwrap();
                        s.set_selected_items(
                            [SelectOption::new("server", "已从服务端补全的选择")],
                            cx,
                        )
                        .unwrap();
                    }
                    s
                })
            })
            .collect::<Vec<_>>();
        let subscriptions = states
            .iter()
            .enumerate()
            .map(|(ix, state)| {
                cx.subscribe_in(
                    state,
                    window,
                    move |this: &mut Self, state, event, window, cx| {
                        match event {
                            SelectEvent::Change(change) => {
                                this.changes += 1;
                                this.last_event = format!(
                                    "第 {} 组 · {:?} · {:?}",
                                    ix + 1,
                                    change.source(),
                                    change.after()
                                )
                                .into();
                            }
                            SelectEvent::SearchRequested(request)
                            | SelectEvent::LoadMoreRequested(request) => {
                                let query = request.query().to_string();
                                let ticket = request.ticket().clone();
                                let cursor = request
                                    .cursor()
                                    .and_then(|c| c.parse::<usize>().ok())
                                    .unwrap_or(0);
                                let state = state.downgrade();
                                cx.spawn_in(window, async move |_, cx| {
                                    cx.background_executor()
                                        .timer(Duration::from_millis(if query.len() == 1 {
                                            700
                                        } else {
                                            180
                                        }))
                                        .await;
                                    let _ = state.update_in(cx, |s, _, cx| {
                                        if query == "error" {
                                            s.reject_request(
                                                &ticket,
                                                "模拟请求失败，清空搜索或重试",
                                                cx,
                                            );
                                            return;
                                        }
                                        let count = if ix == 19 { 12 } else { 5 };
                                        let mut page =
                                            SelectPage::new((cursor..cursor + count).map(|n| {
                                                SelectOption::new(
                                                    format!("{query}:{n}"),
                                                    format!(
                                                        "{} · 结果 {}",
                                                        if query.is_empty() {
                                                            "全部"
                                                        } else {
                                                            &query
                                                        },
                                                        n + 1
                                                    ),
                                                )
                                            }));
                                        if ix == 19 && cursor + count < 60 {
                                            page = page.next_cursor((cursor + count).to_string());
                                        }
                                        let _ = s.apply_page(&ticket, page, cx);
                                    });
                                })
                                .detach();
                            }
                            SelectEvent::CreateRequested(request) => {
                                let ticket = request.ticket().clone();
                                let query = request.query().clone();
                                let state = state.downgrade();
                                cx.spawn_in(window, async move |_, cx| {
                                    cx.background_executor()
                                        .timer(Duration::from_millis(250))
                                        .await;
                                    let _ = state.update_in(cx, |s, window, cx| {
                                        let _ = s.resolve_create(
                                            &ticket,
                                            SelectOption::new(format!("created:{query}"), query),
                                            window,
                                            cx,
                                        );
                                    });
                                })
                                .detach();
                            }
                            _ => {}
                        }
                        cx.notify();
                    },
                )
            })
            .collect();
        let form = cx.new(|_| YueForm::new());
        form.update(cx, |form, _| {
            form.require_select("language", &states[17], "请选择开发语言");
        });
        Self {
            states,
            changes: 0,
            last_event: "尚未选择".into(),
            form,
            footer_focus: cx.focus_handle().tab_stop(false),
            dialog: cx.new(YueDialogState::new),
            _subscriptions: subscriptions,
        }
    }
    fn field(&self, ix: usize, label: &str) -> YueSelect {
        YueSelect::new(format!("select-demo-{ix}"), &self.states[ix])
            .accessibility_label(label)
            .clearable(true)
    }
}
impl Render for SelectPageView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let row = || div().flex().flex_wrap().items_center().gap_4();
        let col = || div().flex().flex_col().items_start().gap_4();
        let mut sizes = col();
        for (ix, size) in [SelectSize::Small, SelectSize::Medium, SelectSize::Large]
            .into_iter()
            .enumerate()
        {
            sizes = sizes.child(
                row()
                    .child(
                        self.field(5 + ix, &format!("描边多选 {size:?}"))
                            .size(size)
                            .width(px(320.)),
                    )
                    .child(
                        self.field(8 + ix, &format!("文字多选 {size:?}"))
                            .size(size)
                            .variant(SelectVariant::Text)
                            .width(px(320.)),
                    ),
            );
        }
        let form = self.form.clone();
        let reset = self.states[17].clone();
        let custom = self.states[18].clone();
        let footer_focus = self.footer_focus.clone();
        let dialog = self.dialog.clone();
        col().debug_selector(|| "select-content".into()).p_8().gap_6()
            .child(div().text_2xl().child("Select 下拉选择器"))
            .child(readout(theme, "稳定 key 选择 · YueInput 搜索 · YueTag 多选 · 统一 Popup 定位与退出".into()))
            .child(readout(theme, format!("Change {} 次 · {}", self.changes, self.last_event)))
            .child(section(theme, "基础与状态", "默认宽 240px；readonly 保留焦点；禁用不响应操作。", row()
                .child(self.field(0, "基础单选"))
                .child(self.field(1, "本地搜索").placeholder("搜索语言"))
                .child(self.field(2, "禁用"))
                .child(self.field(3, "只读"))
                .child(self.field(4, "外部加载中"))))
            .child(section(theme, "三尺寸与两种外观", "多选即时提交并保持面板打开；锁定项不能移除。", sizes))
            .child(section(theme, "标签布局与数量上限", "自动折叠按实际字宽预算；+N 打开可操作的已选面板。", col()
                .child(self.field(11, "自动折叠").tag_layout(SelectTagLayout::Auto).width(px(260.)))
                .child(self.field(13, "最多选择三项").tag_layout(SelectTagLayout::Wrap).width(px(320.)))
                .child(self.field(20, "服务端选择回填").width(px(320.)))))
            .child(section(theme, "分组、图标与说明", "组标题不参与选择；关键词支持 ts / 前端。", self.field(12, "分组语言").row_layout(SelectRowLayout::Description).panel_width(SelectPanelWidth::Content).prefix_text("语言")))
            .child(section(theme, "远程与分页", "单字查询延迟 700ms、其他 180ms，用于验证乱序回写；输入 error 模拟失败。分页共 60 项。", row()
                .child(self.field(14, "远程搜索"))
                .child(self.field(19, "远程分页").max_panel_height(px(192.)))))
            .child(section(theme, "创建选项", "输入新名称，进入创建行后确认；宿主异步确认稳定 key，取消会话后不回写。", self.field(15, "创建语言").width(px(320.))))
            .child(section(theme, "一万项虚拟列表", "仅创建视口中的行；End 到末尾、Home 回开头；搜索 09999。", self.field(16, "一万项").max_panel_height(px(224.))))
            .child(section(theme, "内容与固定页脚", "内容渲染只替换行内部；页脚登记焦点，Tab 按可见顺序退出。", self.field(18, "自定义行")
                .render_option(|row, _, _| YueTag::new(format!("custom:{}", row.option().key()), row.option().label().clone()))
                .panel_footer_focus([self.footer_focus.clone()])
                .panel_footer(move |interactive, _, _| {
                    let owner = custom.clone();
                    YueButton::new("select-reset-footer", "重置选择").theme(theme).track_focus(&footer_focus).tab_stop(false).disabled(!interactive)
                        .on_click(move |_, window, cx| owner.update(cx, |s, cx| { s.set_value(SelectValue::Single(None), window, cx).unwrap(); }))
                })))
            .child(section(theme, "表单校验", "校验读取 SelectValue；提交失败聚焦字段，搜索文本不能替代选择。", col()
                .child(YueField::new(theme).label("开发语言").required(true).bind(&self.form, "language").select(self.field(17, "开发语言")))
                .child(row()
                    .child(YueButton::new("select-submit", "提交表单").theme(theme).on_click(move |_, window, cx| { form.update(cx, |form, cx| { form.submit(window, cx); }); }))
                    .child(YueButton::new("select-form-reset", "清空字段").theme(theme).on_click(move |_, window, cx| reset.update(cx, |s, cx| { s.set_value(SelectValue::Single(None), window, cx).unwrap(); }))))))
            .child(section(theme, "模态窗口内选择", "Select 浮层保持在 Dialog 的作用域；先按 Escape 收起选项，再按一次关闭对话框。", YueButton::new("select-open-dialog", "在对话框中选择").theme(theme).on_click(move |_, window, cx| { dialog.update(cx, |dialog, cx| { dialog.open(window, cx).expect("open select dialog"); }); })))
            .child(YueDialog::new("select-dialog", theme, &self.dialog).title("选择项目语言").child(self.field(21, "对话框语言")))
            .child(readout(theme, "键盘：↑↓ 导航，Home/End、PageUp/PageDown 跳转；查询框 Enter 进入列表，列表 Enter/Space 确认，Ctrl+F 返回查询，Escape 取消，Tab 离开。".into()))
    }
}
