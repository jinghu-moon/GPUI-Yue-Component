#[path = "table/basic.rs"]
mod basic;
#[path = "table/configuration.rs"]
mod configuration;
#[path = "table/customization.rs"]
mod customization;
#[path = "table/playground.rs"]
mod playground;
#[path = "table/temporal.rs"]
mod temporal;
use crate::scroll::ScrollableElement as _;
#[path = "table/advanced.rs"]
mod advanced;
#[path = "table/editing.rs"]
mod editing;
#[path = "table/query.rs"]
mod query;
#[path = "table/structure.rs"]
mod structure;
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, div, rems,
};
use std::rc::Rc;
use yue_component::{
    AvatarSize, TableAlign, TableBorders, TableColumn, TableDelegate, TableEvent, TableLoadState,
    TableRenderMode, TableRowKey, TableRows, TableSelectionMode, TableSize, TableSource,
    TableTextOverflow, TableView, TabsChangeRequest, TagTone, YueAvatar, YueButton, YueCard,
    YueCardHeader, YueTabItem, YueTable, YueTableState, YueTabs, YueTag,
};

struct Order {
    key: SharedString,
    name: SharedString,
    note: SharedString,
    amount: i32,
    done: bool,
}
struct Orders;
impl TableDelegate for Orders {
    type Source = TableRows<Order>;
}

fn records(count: usize, revision: u64, reverse: bool) -> Rc<TableRows<Order>> {
    let mut records: Vec<_> = (0..count)
        .map(|i| Order {
            key: format!("order-{i}").into(),
            name: format!("项目 {}", i + 1).into(),
            note: match i % 3 {
                0 => "待处理的工单；此处演示中文长文本、换行与完整内容提示。",
                1 => "https://example.com/orders/very-long-record-identifier-2026",
                _ => "已核验",
            }
            .into(),
            amount: if i % 7 == 0 { 0 } else { (i as i32 + 1) * 100 },
            done: i % 3 == 2,
        })
        .collect();
    if reverse {
        records.reverse();
    }
    Rc::new(
        TableRows::try_new("showcase-orders", revision, records.into(), |r: &Order| {
            r.key.clone()
        })
        .unwrap(),
    )
}
fn columns(wrap: bool) -> Rc<[TableColumn<Order>]> {
    vec![
        TableColumn::text("number", "工单", |r: &Order| r.key.clone())
            .width(rems(9.))
            .footer("当前结果"),
        TableColumn::custom("owner", "负责人", |r: &Order, _, _, cx| {
            let theme = yue_component::current_theme(cx);
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    YueAvatar::new("avatar", theme)
                        .name("岳")
                        .size(AvatarSize::Small),
                )
                .child(r.name.clone())
        })
        .width(rems(11.)),
        TableColumn::text("note", "说明", |r: &Order| r.note.clone())
            .width(rems(18.))
            .overflow(if wrap {
                TableTextOverflow::Wrap
            } else {
                TableTextOverflow::Ellipsis
            }),
        TableColumn::text("amount", "金额（元）", |r: &Order| {
            r.amount.to_string()
        })
        .width(rems(8.))
        .align(TableAlign::End),
        TableColumn::custom("status", "状态", |r: &Order, _, _, _| {
            YueTag::new("status", if r.done { "已完成" } else { "处理中" }).tone(if r.done {
                TagTone::Success
            } else {
                TagTone::Primary
            })
        })
        .width(rems(8.)),
        TableColumn::text("done", "已核验", |r: &Order| r.done.to_string()).width(rems(7.)),
        TableColumn::custom("action", "", |r: &Order, cell, _, _| {
            let key = r.key.clone();
            let button = YueButton::new("view-order", "复制编号")
                .size(match cell.size() {
                    TableSize::Small => yue_component::ButtonSize::Small,
                    TableSize::Medium => yue_component::ButtonSize::Medium,
                    TableSize::Large => yue_component::ButtonSize::Large,
                })
                .disabled(cell.is_disabled() || cell.is_busy())
                .tab_stop(cell.is_active())
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(key.to_string()))
                });
            if let Some(focus) = cell.control_focus() {
                button.track_focus(focus)
            } else {
                button
            }
        })
        .width(rems(8.))
        .interactive(true),
    ]
    .into()
}
/// Retains ordinary example state; Tabs mounts only the selected panel.
pub(crate) struct TablePage {
    selected: SharedString,
    basic: Entity<basic::BasicExamples>,
    playground: Option<Entity<playground::Playground>>,
    local: Option<Entity<query::QueryExample>>,
    remote: Option<Entity<query::QueryExample>>,
    structure: Option<Entity<structure::StructureExample>>,
    editing: Option<Entity<editing::EditingExample>>,
    temporal: Option<Entity<temporal::TemporalExample>>,
    advanced: Option<Entity<advanced::AdvancedExample>>,
    customization: Option<Entity<customization::CustomizationExample>>,
}

impl TablePage {
    pub(crate) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            selected: "basic".into(),
            basic: cx.new(basic::BasicExamples::new),
            playground: None,
            local: None,
            remote: None,
            structure: None,
            editing: None,
            temporal: None,
            advanced: None,
            customization: None,
        }
    }

    fn select(&mut self, key: &SharedString, w: &mut Window, cx: &mut Context<Self>) {
        if key != &self.selected {
            // Stress fixtures have no unsaved edits. Release them when leaving;
            // ordinary queries and editor drafts retain their entity identity.
            self.advanced = None;
            if let Some(playground) = &self.playground {
                playground.update(cx, |s, cx| s.release_large(cx));
            }
        }
        match key.as_str() {
            "basic" => {}
            "playground" => {
                self.playground
                    .get_or_insert_with(|| cx.new(|cx| playground::Playground::new(w, cx)));
            }
            "local" => {
                self.local
                    .get_or_insert_with(|| cx.new(|cx| query::QueryExample::new(false, w, cx)));
            }
            "remote" => {
                self.remote
                    .get_or_insert_with(|| cx.new(|cx| query::QueryExample::new(true, w, cx)));
            }
            "structure" => {
                self.structure
                    .get_or_insert_with(|| cx.new(|cx| structure::StructureExample::new(w, cx)));
            }
            "editing" => {
                self.editing
                    .get_or_insert_with(|| cx.new(|cx| editing::EditingExample::new(w, cx)));
            }
            "temporal" => {
                self.temporal
                    .get_or_insert_with(|| cx.new(|cx| temporal::TemporalExample::new(w, cx)));
            }
            "advanced" => {
                self.advanced
                    .get_or_insert_with(|| cx.new(advanced::AdvancedExample::new));
            }
            "customization" => {
                self.customization
                    .get_or_insert_with(|| cx.new(customization::CustomizationExample::new));
            }
            _ => return,
        }
        self.selected = key.clone();
        cx.notify();
    }
}

fn example_tab<V: Render>(
    key: &'static str,
    label: &'static str,
    description: &'static str,
    view: Option<Entity<V>>,
) -> YueTabItem {
    YueTabItem::new(key, label)
        .panel_tab_stop(false)
        .content(move |_, _| {
            let mut body = div()
                .p_6()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_5()
                .child(div().text_xl().child(label))
                .child(description);
            if let Some(view) = &view {
                body = body.child(view.clone());
            }
            div()
                .id((gpui_kit::ElementId::from("table-section"), key))
                .size_full()
                .min_w_0()
                .min_h_0()
                .child(body)
                .overflow_y_scrollbar()
                .into_any_element()
        })
}

impl Render for TablePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("table-content").debug_selector(|| "table-content".into())
            .size_full().min_w_0().min_h_0().flex().flex_col()
            .child(div().p_6().pb_3().flex().flex_col().gap_2()
                .child(div().text_2xl().child("Table 表格"))
                .child("按类别浏览独立示例，比较展示、布局与数据交互。"))
            .child(div().flex_1().min_h_0().min_w_0().child(
                YueTabs::new("table-examples", Some(self.selected.clone()))
                    .accessibility_label("表格示例分类")
                    .height(gpui_kit::relative(1.))
                    .width(gpui_kit::relative(1.))
                    .items([
                        example_tab("basic", "基础样式", "10 个独立示例：边框、斑马纹、密度、自定义内容、换行、空表格与加载状态。", Some(self.basic.clone())),
                        example_tab("playground", "布局与交互", "固定表头与列、伸缩列、分组表头、响应式、选择、键盘导航，以及 10 万行虚拟列表。", self.playground.clone()),
                        example_tab("local", "本地查询", "排序、筛选、分页、跨页选择与批量操作；负责人编辑后重新执行当前查询。", self.local.clone()),
                        example_tab("remote", "远程查询", "模拟异步请求、过期响应、失败重试与未知结果；保留已接受的数据。", self.remote.clone()),
                        example_tab("structure", "树与详情", "树形工单、父子级联、根分页、节点懒加载、展开详情与嵌套子表。", self.structure.clone()),
                        example_tab("editing", "编辑与校验", "使用 Input、Select、Form 和 Field 编辑单元格与整行，演示保存、校验与取消。", self.editing.clone()),
                        example_tab("temporal", "日期与时间", "复用 DatePicker 和 TimePicker 编辑预约，保存前校验实际输入。", self.temporal.clone()),
                        example_tab("advanced", "合并与加载", "双轴虚拟化、自然高度合并、表头吸顶与范围加载，分别展示不同布局场景。", self.advanced.clone()),
                        example_tab("customization", "重排与列设置", "行列拖动、列显示与宽度设置，以及尾游标追加、失败与重试。", self.customization.clone()),
                    ])
                    .on_change(cx.listener(|s, request: &TabsChangeRequest, w, cx| s.select(request.key(), w, cx))),
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::test_support::snapshots;
    use gpui_kit::{ElementId, Role, TestAppContext, VisualTestContext, px, size};
    pub(super) fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    pub(super) fn click(cx: &mut VisualTestContext, id: &str) {
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        draw(cx);
    }

    fn open(cx: &mut TestAppContext) -> (Entity<TablePage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(
                cx,
                yue_component::YueTheme::new(
                    yue_component::ThemeMode::Light,
                    yue_component::Accent::Azure,
                ),
            )
            .unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| TablePage::new(w, cx));
            page = Some(view.clone());
            gpui_kit::base::Root::new(view, w, cx)
        });
        cx.update(|w, _| {
            w.activate_window();
            w.resize(size(px(1800.), px(1200.)));
        });
        draw(cx);
        (page.unwrap(), cx)
    }

    fn tab(cx: &mut VisualTestContext, label: &str) {
        let center = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.role() == Some(Role::Tab) && n.label() == Some(label))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(center, Default::default());
        draw(cx);
    }

    #[gpui_kit::test]
    fn gallery_exposes_ten_examples_and_only_mounts_the_selected_category(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let tables = |cx: &mut VisualTestContext| {
            cx.update(|w, _| {
                snapshots(w)
                    .into_iter()
                    .filter(|n| n.role() == Some(Role::Table))
                    .count()
            })
        };
        assert_eq!(tables(cx), 10);
        assert!(page.read_with(cx, |p, _| p.playground.is_none()
            && p.local.is_none()
            && p.remote.is_none()
            && p.advanced.is_none()));
        for (label, key) in [
            ("布局与交互", "playground"),
            ("本地查询", "local"),
            ("远程查询", "remote"),
            ("树与详情", "structure"),
            ("编辑与校验", "editing"),
            ("日期与时间", "temporal"),
            ("合并与加载", "advanced"),
            ("重排与列设置", "customization"),
            ("基础样式", "basic"),
        ] {
            tab(cx, label);
            assert_eq!(page.read_with(cx, |p, _| p.selected.to_string()), key);
            let panels = cx.update(|w, _| {
                snapshots(w)
                    .into_iter()
                    .filter(|n| n.role() == Some(Role::TabPanel))
                    .map(|n| n.label().unwrap().to_owned())
                    .collect::<Vec<_>>()
            });
            assert_eq!(panels, vec![label.to_owned()]);
        }
        assert_eq!(
            tables(cx),
            10,
            "inactive advanced tables must leave the render tree"
        );
        assert!(page.read_with(cx, |p, _| p.advanced.is_none()));
        let local = page.read_with(cx, |p, _| p.local.as_ref().unwrap().entity_id());
        tab(cx, "本地查询");
        assert_eq!(
            page.read_with(cx, |p, _| p.local.as_ref().unwrap().entity_id()),
            local
        );
    }

    #[gpui_kit::test]
    fn gallery_density_is_independent_and_narrow_tabs_support_keyboard_navigation(
        cx: &mut TestAppContext,
    ) {
        let (page, cx) = open(cx);
        let height = |cx: &mut VisualTestContext, label: &str| {
            cx.update(|w, _| {
                snapshots(w)
                    .into_iter()
                    .find(|n| n.role() == Some(Role::Table) && n.label() == Some(label))
                    .unwrap()
                    .bounds()
                    .size
                    .height
            })
        };
        let plain = height(cx, "基础表格");
        let dense = height(cx, "表格密度");
        click(cx, "table-size");
        assert_eq!(height(cx, "基础表格"), plain);
        assert!(height(cx, "表格密度") > dense);
        cx.update(|w, _| w.resize(size(px(760.), px(800.))));
        draw(cx);
        tab(cx, "基础样式");
        cx.simulate_keystrokes("end");
        draw(cx);
        assert_eq!(
            page.read_with(cx, |p, _| p.selected.to_string()),
            "customization"
        );
        let last = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.role() == Some(Role::Tab) && n.label() == Some("重排与列设置"))
                .unwrap()
        });
        assert_eq!(last.focused(), Some(true));
        assert!(last.bounds().right() <= px(760.) && last.bounds().left() >= px(0.));
        cx.simulate_keystrokes("home");
        draw(cx);
        assert_eq!(page.read_with(cx, |p, _| p.selected.to_string()), "basic");
    }
}
