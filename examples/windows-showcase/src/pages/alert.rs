//! Independently discoverable inline feedback recipes.
use super::super::*;
use gpui_kit::{Entity, base::input::InputState, rems};
use std::time::Duration;
use yue_component::{AlertKind, AlertSize, AlertVariant, YueAlert, YueCard, YueCardHeader};

pub(crate) struct AlertPage {
    visible: [bool; 3],
    expanded: bool,
    timed: bool,
    rejected: usize,
    closed: usize,
    name: Entity<InputState>,
    failed: bool,
    fail_next: bool,
    saved: bool,
    update_details: bool,
    narrow_visible: bool,
    narrow_retried: bool,
}

impl AlertPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            visible: [true; 3],
            expanded: false,
            timed: false,
            rejected: 0,
            closed: 0,
            name: cx.new(|cx| InputState::new(w, cx).default_value("Yue 项目")),
            failed: false,
            fail_next: true,
            saved: false,
            update_details: false,
            narrow_visible: true,
            narrow_retried: false,
        }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        self.failed = self.name.read(cx).value().trim().is_empty() || self.fail_next;
        self.fail_next = false;
        self.saved = !self.failed;
        cx.notify();
    }
}
fn stack() -> gpui_kit::Div {
    div().flex().flex_col().w_full().min_w_0().gap_3()
}
fn section(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    content: impl IntoElement,
) -> YueCard {
    YueCard::new(id)
        .header(YueCardHeader::new(title).description(description))
        .child(content)
}
impl Render for AlertPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let kinds = [
            (AlertKind::Info, "信息提示：今晚 22:00 进行维护"),
            (AlertKind::Success, "保存成功：所有更改已保存"),
            (AlertKind::Warning, "存储空间不足：请清理后再导出"),
            (AlertKind::Error, "同步失败：连接中断，本地更改已保留"),
        ];
        let semantic =
            stack().children(kinds.into_iter().enumerate().map(|(i, (kind, text))| {
                YueAlert::new(format!("alert-kind-{i}"), text).kind(kind)
            }));
        let variants = stack().children(
            [
                AlertVariant::Tonal,
                AlertVariant::Outlined,
                AlertVariant::Solid,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, variant)| {
                YueAlert::new(
                    format!("alert-variant-{i}"),
                    format!("{variant:?} · 导出已完成"),
                )
                .kind(AlertKind::Success)
                .variant(variant)
                .description("结果已写入所选文件夹，可以继续编辑。")
            }),
        );
        let icons = stack()
            .child(YueAlert::new("alert-no-icon", "纯文字提示").show_icon(false))
            .child(
                YueAlert::new("alert-custom-icon", "发现可安装更新")
                    .icon(outline::Download)
                    .when(self.update_details, |a| {
                        a.description("更新内容：改进导出流程，修复离线编辑同步。")
                    })
                    .action(
                        YueButton::new("alert-install", "查看更新").on_click(cx.listener(
                            |s, _, _, cx| {
                                s.update_details = !s.update_details;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(
                YueAlert::new("alert-rich", "正在使用本地副本")
                    .child(div().child("离线编辑仍可继续；恢复连接后再同步。")),
            );
        let sizes = stack()
            .child(YueAlert::new("alert-small", "紧凑提示：此区域只读").size(AlertSize::Small))
            .child(YueAlert::new("alert-medium", "标准提示：此区域只读"));
        let mut closing = stack();
        for (i, label) in ["图标关闭", "文字关闭", "自定义图标关闭"]
            .into_iter()
            .enumerate()
        {
            let h = cx.listener(move |s, _: &yue_component::AlertCloseReason, _, cx| {
                s.visible[i] = false;
                cx.notify();
            });
            let done = cx.listener(|s, _: &(), _, cx| {
                s.closed += 1;
                cx.notify();
            });
            let alert = YueAlert::new(format!("alert-close-{i}"), label)
                .closable(true)
                .visible(self.visible[i])
                .on_close(move |r, w, cx| h(&r, w, cx))
                .on_closed(move |w, cx| done(&(), w, cx));
            closing = closing.child(match i {
                1 => alert.close_text("知道了"),
                2 => alert.close_icon(outline::CircleX),
                _ => alert,
            });
        }
        closing = closing.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    YueButton::new("alert-restore", "恢复提示").on_click(cx.listener(
                        |s, _, _, cx| {
                            s.visible = [true; 3];
                            cx.notify();
                        },
                    )),
                )
                .child(readout(theme, format!("关闭完成 {} 次", self.closed))),
        );
        let rejected = cx.listener(|s, _: &yue_component::AlertCloseReason, _, cx| {
            s.rejected += 1;
            cx.notify();
        });
        let controlled = stack()
            .child(
                YueAlert::new("alert-rejected", "此重要提示由宿主保持显示")
                    .kind(AlertKind::Warning)
                    .closable(true)
                    .on_close(move |r, w, cx| rejected(&r, w, cx)),
            )
            .child(readout(
                theme,
                format!("已收到 {} 次操作请求", self.rejected),
            ));
        let expand = cx.listener(|s, value: &bool, _, cx| {
            s.expanded = *value;
            cx.notify();
        });
        let folding=YueAlert::new("alert-folding","更新说明")
            .description("新版本调整了导出流程。请先选择保存目录，再确认文件名；已有文件不会自动覆盖。所有未保存的编辑内容都会保留。长段落提示应允许用户按自己的速度阅读，并可以通过键盘展开与收起。".repeat(4))
            .max_lines(2).expanded(self.expanded).on_expanded_change(move |v,w,cx|expand(&v,w,cx));
        let close_narrow = cx.listener(|s, _: &yue_component::AlertCloseReason, _, cx| {
            s.narrow_visible = false;
            cx.notify();
        });
        let narrow=stack().w(rems(18.)).max_w_full().child(YueAlert::new("alert-narrow",if self.narrow_retried {"连接已恢复"}else{"窄面板中的完整提示"})
            .description("长中文说明完整换行。LongIdentifierWithoutSpacesABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .visible(self.narrow_visible).closable(true).on_close(move |r,w,cx|close_narrow(&r,w,cx))
            .action(YueButton::new("alert-narrow-retry","重试").disabled(self.narrow_retried).on_click(cx.listener(|s,_,_,cx|{s.narrow_retried=true;cx.notify();}))))
            .child(YueButton::new("alert-narrow-restore","恢复窄面板示例").on_click(cx.listener(|s,_,_,cx|{s.narrow_retried=false;s.narrow_visible=true;cx.notify();})));
        let timeout = cx.listener(|s, _: &yue_component::AlertCloseReason, _, cx| {
            s.timed = false;
            cx.notify();
        });
        let timed = stack()
            .child(
                YueButton::new("alert-start-timer", "显示 8 秒提示").on_click(cx.listener(
                    |s, _, _, cx| {
                        s.timed = true;
                        cx.notify();
                    },
                )),
            )
            .child(
                YueAlert::new("alert-timed", "导出完成，可继续编辑")
                    .kind(AlertKind::Success)
                    .description("悬停或将键盘焦点移入后暂停，切换窗口或滚出可视区域也会暂停。")
                    .visible(self.timed)
                    .duration(Some(Duration::from_secs(8)))
                    .closable(true)
                    .on_close(move |r, w, cx| timeout(&r, w, cx)),
            );
        let form = stack()
            .child(YueField::new(theme).label("项目名称").input(YueInput::new(
                "alert-project-name",
                theme,
                &self.name,
            )))
            .child(
                YueButton::new("alert-save", "保存项目")
                    .variant(ButtonVariant::Primary)
                    .on_click(cx.listener(|s, _, _, cx| s.save(cx))),
            )
            .child(
                YueAlert::new("alert-save-error", "项目未保存")
                    .kind(AlertKind::Error)
                    .visible(self.failed)
                    .description(if self.name.read(cx).value().trim().is_empty() {
                        "请输入项目名称后重试。"
                    } else {
                        "连接中断，输入内容已保留。请重试保存。"
                    })
                    .action(
                        YueButton::new("alert-save-retry", "重试保存")
                            .on_click(cx.listener(|s, _, _, cx| s.save(cx))),
                    ),
            )
            .child(
                YueAlert::new("alert-save-success", "项目已保存")
                    .kind(AlertKind::Success)
                    .visible(self.saved),
            );
        stack()
            .id("alert-content")
            .debug_selector(|| "alert-content".into())
            .child(section(
                "alert-states",
                "四种状态",
                "信息、成功、警告和错误；默认常驻。",
                semantic,
            ))
            .child(section(
                "alert-variants",
                "三种外观",
                "Tonal / Outlined / Solid 使用成对语义颜色。",
                variants,
            ))
            .child(section(
                "alert-icons",
                "图标与富内容",
                "隐藏图标、自定义 Tabler 图标和独立操作位。",
                icons,
            ))
            .child(section(
                "alert-sizes",
                "两档密度",
                "Small / Medium，与已有文字和按钮尺寸组合。",
                sizes,
            ))
            .child(section(
                "alert-closing",
                "关闭与恢复",
                "退出时淡出并收起高度，完成后通知宿主。",
                closing,
            ))
            .child(section(
                "alert-controlled",
                "受控关闭",
                "宿主可以保留重要提示。",
                controlled,
            ))
            .child(section(
                "alert-fold",
                "描述折叠",
                "只在描述确实超出两行时提供展开按钮。",
                folding,
            ))
            .child(section(
                "alert-banner-demo",
                "公告提示",
                "内容完整换行，无跑马灯、无自动固定。",
                YueAlert::new(
                    "alert-banner",
                    "维护公告：今晚 22:00—23:00 暂停云端同步，本地编辑可照常使用。",
                )
                .banner(true)
                .kind(AlertKind::Warning),
            ))
            .child(section(
                "alert-narrow-demo",
                "窄容器与长文案",
                "操作移至正文下方，关闭位独立保留。",
                narrow,
            ))
            .child(section(
                "alert-timer-demo",
                "可暂停倒计时",
                "显式启用；底部线条显示剩余阅读时间。",
                timed,
            ))
            .child(section(
                "alert-form-demo",
                "保存失败与重试",
                "首次模拟连接中断；输入保留，重试后保存成功。",
                form,
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::{
        ElementId, Modifiers, TestAppContext, VisualTestContext,
        base::test_support::{ElementSnapshot, snapshots},
        point, px,
    };
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.simulate_next_frame(cx);
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn node(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
                .unwrap_or_else(|| panic!("missing {id}"))
        })
    }
    fn click(cx: &mut VisualTestContext, id: &str) {
        let p = node(cx, id).bounds().center();
        let height = cx.update(|w, _| w.viewport_size().height);
        if p.y < px(0.) || p.y > height {
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                position: point(p.x, height / 2.),
                delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), height / 2. - p.y)),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });
            draw(cx);
        }
        let p = node(cx, id).bounds().center();
        cx.simulate_click(p, Modifiers::default());
        draw(cx);
    }
    fn open(cx: &mut TestAppContext) -> (Entity<Showcase>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let (view, cx) = cx.add_window_view(Showcase::new);
        cx.update(|w, _| w.activate_window());
        view.update(cx, |s, cx| {
            s.page = ShowcasePage::Alert;
            cx.notify();
        });
        draw(cx);
        draw(cx);
        (view, cx)
    }
    #[gpui_kit::test]
    fn gallery_exposes_independent_recipes_and_restores_closed_notices(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        for id in [
            "alert-states",
            "alert-variants",
            "alert-icons",
            "alert-sizes",
            "alert-closing",
            "alert-controlled",
            "alert-fold",
            "alert-banner-demo",
            "alert-narrow-demo",
            "alert-timer-demo",
            "alert-form-demo",
        ] {
            node(cx, id);
        }
        // Locate the close control by its stable, structured child ID.
        let close = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| {
                    n.path().last()
                        == Some(&ElementId::from((
                            ElementId::from("alert-close-1"),
                            "close",
                        )))
                })
                .unwrap()
                .bounds()
                .center()
        });
        let height = cx.update(|w, _| w.viewport_size().height);
        cx.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(close.x, height / 2.),
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), height / 2. - close.y)),
            touch_phase: gpui_kit::TouchPhase::Moved,
            ..Default::default()
        });
        draw(cx);
        let close = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| {
                    n.path().last()
                        == Some(&ElementId::from((
                            ElementId::from("alert-close-1"),
                            "close",
                        )))
                })
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(close, Modifiers::default());
        draw(cx);
        view.read_with(cx, |s, cx| assert!(!s.alert_demo.read(cx).visible[1]));
        click(cx, "alert-restore");
        view.read_with(cx, |s, cx| {
            assert_eq!(s.alert_demo.read(cx).visible, [true; 3])
        });
    }
    #[gpui_kit::test]
    fn save_failure_keeps_input_and_retry_recovers(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let page = view.read_with(cx, |s, _| s.alert_demo.clone());
        let name = page.read_with(cx, |s, _| s.name.clone());
        cx.update(|w, cx| name.update(cx, |s, cx| s.set_value("Mercury project", w, cx)));
        draw(cx);
        click(cx, "alert-save");
        page.read_with(cx, |s, cx| {
            assert!(s.failed);
            assert!(!s.saved);
            assert_eq!(s.name.read(cx).value(), "Mercury project");
        });
        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        click(cx, "alert-save-retry");
        page.read_with(cx, |s, cx| {
            assert!(!s.failed);
            assert!(s.saved);
            assert_eq!(s.name.read(cx).value(), "Mercury project");
        });
        node(cx, "alert-save-success");
    }
}
