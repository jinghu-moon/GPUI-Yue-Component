use super::*;

/// Bounded, independent examples. Sources and columns are prepared once, never per frame.
pub(super) struct BasicExamples {
    cases: Vec<BasicCase>,
    density: TableSize,
    _delegate: Entity<Orders>,
}

struct BasicCase {
    key: &'static str,
    title: &'static str,
    description: &'static str,
    table: Entity<YueTableState<Orders>>,
    borders: TableBorders,
    striped: bool,
}

impl BasicExamples {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let delegate = cx.new(|_| Orders);
        let source = records(3, 1, false);
        let empty = records(0, 1, false);
        let text_columns = |overflow| -> Rc<[TableColumn<Order>]> {
            vec![
                TableColumn::text("number", "工单", |r: &Order| r.key.clone()).width(rems(8.)),
                TableColumn::text("note", "说明", |r: &Order| r.note.clone())
                    .flex(1.)
                    .min_width(rems(10.))
                    .overflow(overflow),
                TableColumn::text("amount", "金额（元）", |r: &Order| {
                    r.amount.to_string()
                })
                .width(rems(8.))
                .align(TableAlign::End),
            ]
            .into()
        };
        let ellipsis = text_columns(TableTextOverflow::Ellipsis);
        let wrap = text_columns(TableTextOverflow::Wrap);
        let mut custom =
            vec![TableColumn::text("number", "工单", |r: &Order| r.key.clone()).width(rems(8.))];
        custom.extend(
            columns(false)
                .iter()
                .filter(|c| matches!(c.key().as_str(), "owner" | "status" | "action"))
                .cloned(),
        );
        let custom: Rc<[_]> = custom.into();
        let mut cases = Vec::new();
        for (key, title, description, borders, striped, load) in [
            (
                "plain",
                "基础表格",
                "无边框展示；金额右对齐，保留 0 值。",
                TableBorders::None,
                false,
                TableLoadState::Ready,
            ),
            (
                "density",
                "表格密度",
                "单独调整此表的行高，比较紧凑、默认和宽松三档。",
                TableBorders::Rows,
                false,
                TableLoadState::Ready,
            ),
            (
                "rows",
                "行分隔线",
                "只显示行分隔线，适合普通业务列表。",
                TableBorders::Rows,
                false,
                TableLoadState::Ready,
            ),
            (
                "grid",
                "完整边框",
                "行列边界清晰，便于逐格比较。",
                TableBorders::Grid,
                false,
                TableLoadState::Ready,
            ),
            (
                "striped",
                "斑马纹",
                "交替行底色帮助横向阅读；悬停与键盘焦点仍可辨认。",
                TableBorders::Rows,
                true,
                TableLoadState::Ready,
            ),
            (
                "custom",
                "自定义单元格",
                "复用 Avatar、Tag 和 Button；复制按钮写入该行的工单编号。",
                TableBorders::Rows,
                false,
                TableLoadState::Ready,
            ),
            (
                "wrap",
                "长文本换行",
                "同一行按最长内容确定高度，其余单元格保持垂直居中。",
                TableBorders::Grid,
                false,
                TableLoadState::Ready,
            ),
            (
                "empty",
                "空表格",
                "保留列定义，使用组件自带的空状态。",
                TableBorders::Rows,
                false,
                TableLoadState::Ready,
            ),
            (
                "loading",
                "初次加载",
                "点击开始加载，比较等待与空结果；结束加载后恢复空状态。",
                TableBorders::Rows,
                false,
                TableLoadState::Ready,
            ),
            (
                "failure",
                "刷新失败",
                "刷新失败时保留原有记录，并在表格内说明失败。",
                TableBorders::Rows,
                false,
                TableLoadState::Failed("暂时无法刷新，原有工单仍可查看。".into()),
            ),
        ] {
            let table = cx.new(|cx| YueTableState::new(&delegate, cx));
            let rows = if matches!(key, "empty" | "loading") {
                empty.clone()
            } else {
                source.clone()
            };
            let cols = match key {
                "wrap" => wrap.clone(),
                "custom" => custom.clone(),
                _ => ellipsis.clone(),
            };
            table.update(cx, |s, cx| {
                s.commit_view(TableView::new(rows, cols).load_state(load), None, cx)
                    .unwrap();
            });
            cases.push(BasicCase {
                key,
                title,
                description,
                table,
                borders,
                striped,
            });
        }
        Self {
            cases,
            density: TableSize::Small,
            _delegate: delegate,
        }
    }
}

impl Render for BasicExamples {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = div().flex().flex_col().gap_5().min_w_0();
        for case in &self.cases {
            let mut header = YueCardHeader::new(case.title).description(case.description);
            if case.key == "density" {
                header = header.extra(
                    YueButton::new(
                        "table-size",
                        match self.density {
                            TableSize::Small => "密度示例：紧凑",
                            TableSize::Medium => "密度示例：默认",
                            TableSize::Large => "密度示例：宽松",
                        },
                    )
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.density = match s.density {
                            TableSize::Small => TableSize::Medium,
                            TableSize::Medium => TableSize::Large,
                            TableSize::Large => TableSize::Small,
                        };
                        cx.notify();
                    })),
                );
            }
            if case.key == "loading" {
                let loading =
                    case.table.read(cx).view().unwrap().load() == &TableLoadState::Loading;
                header = header.extra(
                    YueButton::new(
                        "table-basic-loading",
                        if loading {
                            "结束加载"
                        } else {
                            "开始加载"
                        },
                    )
                    .on_click(cx.listener(|s, _, _, cx| {
                        let table = &s.cases.iter().find(|c| c.key == "loading").unwrap().table;
                        table.update(cx, |state, cx| {
                            let view = state.view().unwrap().as_ref().clone();
                            let loading = view.load() != &TableLoadState::Loading;
                            state
                                .commit_view(
                                    view.load_state(if loading {
                                        TableLoadState::Loading
                                    } else {
                                        TableLoadState::Ready
                                    }),
                                    None,
                                    cx,
                                )
                                .unwrap();
                        });
                        cx.notify();
                    })),
                );
            }
            content = content.child(
                YueCard::new((gpui_kit::ElementId::from("table-basic-case"), case.key))
                    .header(header)
                    .body_inset(false)
                    .child(
                        YueTable::new(
                            (gpui_kit::ElementId::from("table-basic"), case.key),
                            &case.table,
                        )
                        .accessibility_label(case.title)
                        .size(if case.key == "density" {
                            self.density
                        } else {
                            TableSize::Medium
                        })
                        .borders(case.borders)
                        .striped(case.striped)
                        .render_mode(TableRenderMode::Static),
                    ),
            );
        }
        content
    }
}
