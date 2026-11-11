//! Independently discoverable Drawer recipes with retained host-owned form data.
use super::super::*;
use gpui_kit::{
    AnyElement, Entity, Focusable as _, Subscription, Task,
    base::input::{InputEvent, InputState},
    rems,
};
use std::time::Duration;
use yue_component::{
    DrawerCloseReason, DrawerDecision, DrawerFooter, DrawerOutcome, DrawerPlacement, DrawerSize,
    DrawerSubmitRequest, InputStatus, YueCard, YueCardHeader, YueDescriptionItem, YueDescriptions,
    YueDialog, YueDialogState, YueDrawer, YueDrawerEvent, YueDrawerHost, YueDrawerState,
    YueLoadingRegion,
};
const COUNT: usize = 14;
const BASIC: usize = 0;
const DIRECTIONS: usize = 1;
const SIZES: usize = 2;
const CUSTOM: usize = 3;
const TRANSPARENT: usize = 4;
const NONMODAL: usize = 5;
const LONG: usize = 6;
const LOADING: usize = 7;
const FORM: usize = 8;
const NESTED: usize = 9;
const RESIZE: usize = 10;
const LOCAL: usize = 11;
const DETAILS: usize = 12;
const NO_HEADER: usize = 13;
const ORDERS: [&str; 3] = ["检查主轴温升", "校准传感器", "更换冷却管"];
const RECIPES: [(&str, &str); COUNT] = [
    ("基础详情", "查看记录，保留背景上下文；关闭后返回触发位置。"),
    (
        "四个方向",
        "Left / Right / Top / Bottom；顶部和底部的 extent 表示高度。",
    ),
    (
        "预设与相对尺寸",
        "320 / 480 / 640 / Full 和 60%；窗口空间不足时自动钳制。",
    ),
    (
        "自定义头尾",
        "自定义标题内容、额外操作、关闭符号与底部操作。",
    ),
    ("隐藏遮罩", "仅隐藏遮罩；仍是模态，背景保持输入隔离。"),
    (
        "非模态面板",
        "背景按钮仍可操作，焦点可离开面板，不拦截背景滚轮。",
    ),
    (
        "长正文",
        "80 段正文仅在显示期间构建；标题、底部固定，正文复用 YueScrollbar。",
    ),
    (
        "异步加载",
        "使用 YueLoadingRegion；关闭后重开会启动新的加载请求。",
    ),
    (
        "编辑与保存",
        "修改名称后尝试 Escape 或点击遮罩：显示未保存保护；支持异步保存失败后重试。",
    ),
    (
        "嵌套决策",
        "Drawer → Drawer → Dialog 共用一条层级链；Escape 每次只关闭最上层。",
    ),
    (
        "调整尺寸",
        "拖动内侧边缘；聚焦边缘后方向键调整，Shift 加大步长，Home/End 到边界，双击复原。",
    ),
    (
        "局部容器与 Push",
        "抽屉限制在下面的工作区；推开正文重新布局，窗口其他部分仍可操作。",
    ),
    (
        "列表详情联动",
        "直接打开指定记录；面板中的上一条/下一条保持当前抽屉会话。",
    ),
    (
        "隐藏默认头尾",
        "提供语义名称与正文关闭按钮；自定义布局不会失去关闭保护。",
    ),
];
pub(crate) struct DrawerPage {
    drawers: Vec<Entity<YueDrawerState>>,
    child: Entity<YueDrawerState>,
    decision: Entity<YueDialogState>,
    name: Entity<InputState>,
    saved_name: SharedString,
    error: Option<SharedString>,
    guarded: bool,
    discard: bool,
    fail: bool,
    loading: bool,
    load_generation: u64,
    selected: usize,
    placement: DrawerPlacement,
    size: DrawerSize,
    fraction: bool,
    push: bool,
    background: usize,
    last: SharedString,
    resize: SharedString,
    save_task: Option<Task<()>>,
    load_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}
impl DrawerPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let drawers: Vec<_> = (0..COUNT).map(|_| cx.new(YueDrawerState::new)).collect();
        let name = cx.new(|cx| InputState::new(w, cx).default_value("Yue Component"));
        let mut subscriptions = Vec::new();
        for (index, state) in drawers.iter().enumerate() {
            subscriptions.push(cx.subscribe(state, move |s: &mut Self, _, event, cx| {
                s.last = match event {
                    YueDrawerEvent::OpenChanged(change) => format!(
                        "{} · open={} · {:?}",
                        RECIPES[index].0,
                        change.is_open(),
                        change.reason()
                    ),
                    YueDrawerEvent::Opened => format!("{} · 进入完成", RECIPES[index].0),
                    YueDrawerEvent::Closed(change) => {
                        if index == FORM {
                            s.guarded = false;
                            s.discard = false;
                        }
                        format!("{} · 退出完成 · {:?}", RECIPES[index].0, change.reason())
                    }
                }
                .into();
                cx.notify();
            }));
        }
        subscriptions.push(cx.subscribe(&name, |_: &mut Self, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        }));
        Self {
            drawers,
            child: cx.new(YueDrawerState::new),
            decision: cx.new(YueDialogState::new),
            name,
            saved_name: "Yue Component".into(),
            error: None,
            guarded: false,
            discard: false,
            fail: false,
            loading: false,
            load_generation: 0,
            selected: 0,
            placement: DrawerPlacement::Right,
            size: DrawerSize::Medium,
            fraction: false,
            push: true,
            background: 0,
            last: "选择下面任一示例".into(),
            resize: "尚未调整".into(),
            save_task: None,
            load_task: None,
            _subscriptions: subscriptions,
        }
    }
    fn open(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        match self.drawers[index].update(cx, |s, cx| s.open(w, cx)) {
            Ok(()) => {
                if index == FORM {
                    self.error = None;
                    self.guarded = false;
                    self.discard = false;
                }
                if index == LOADING {
                    self.loading = true;
                    self.load_generation += 1;
                    let generation = self.load_generation;
                    self.load_task = Some(cx.spawn(async move |owner, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(900))
                            .await;
                        let _ = owner.update(cx, |s, cx| {
                            if s.load_generation == generation {
                                s.loading = false;
                                cx.notify();
                            }
                        });
                    }));
                }
            }
            Err(error) => self.last = error.to_string().into(),
        }
        cx.notify();
    }
    fn allow_close(&mut self, reason: DrawerCloseReason, cx: &mut Context<Self>) -> bool {
        let dirty = self.name.read(cx).value() != self.saved_name;
        if reason != DrawerCloseReason::Confirm && dirty && !self.discard {
            self.guarded = true;
            cx.notify();
            false
        } else {
            true
        }
    }
    fn save(
        &mut self,
        request: &DrawerSubmitRequest,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> DrawerDecision {
        if self.name.read(cx).value().trim().is_empty() {
            self.error = Some("请填写项目名称。".into());
            cx.notify();
            return DrawerDecision::KeepOpen;
        }
        self.error = None;
        self.guarded = false;
        let fail = self.fail;
        let name = self.name.read(cx).value();
        let ticket = request.ticket();
        let window = w.window_handle();
        self.save_task = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(700))
                .await;
            let _ = window.update(cx, |_, w, cx| {
                if ticket.complete(
                    if fail {
                        DrawerOutcome::KeepOpen
                    } else {
                        DrawerOutcome::Close
                    },
                    w,
                    cx,
                ) {
                    let _ = owner.update(cx, |s, cx| {
                        if fail {
                            s.error = Some("保存失败，输入已保留；请关闭模拟失败后重试。".into());
                        } else {
                            s.saved_name = name;
                            s.error = None;
                        }
                        cx.notify();
                    });
                }
            });
        }));
        DrawerDecision::Pending
    }
    fn opener(
        &self,
        index: usize,
        label: impl Into<SharedString>,
        theme: YueTheme,
        cx: &Context<Self>,
    ) -> YueButton {
        YueButton::text(("drawer-open", index), theme, label)
            .on_click(cx.listener(move |s, _, w, cx| s.open(index, w, cx)))
    }
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_3()
}
fn column() -> gpui_kit::Div {
    div().min_w_0().w_full().flex().flex_col().gap_4()
}
fn details(theme: YueTheme, index: usize) -> impl IntoElement {
    YueDescriptions::new("work-order-info")
        .columns(1)
        .items([
            YueDescriptionItem::new("id", "编号")
                .text(format!("WO-{:03}", index + 1))
                .copyable(true),
            YueDescriptionItem::new("name", "事项").text(ORDERS[index]),
            YueDescriptionItem::new("owner", "负责人").text(["张工", "李工", "王工"][index]),
            YueDescriptionItem::new("state", "状态").text("进行中"),
            YueDescriptionItem::new("note", "备注")
                .text("完成后保留测量记录，并与负责人确认后续安排。长文本在可用宽度内换行。"),
        ])
        .theme(theme)
}
impl Render for DrawerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut page = column()
            .p_8()
            .gap_6()
            .debug_selector(|| "drawer-content".into())
            .child(div().text_2xl().child("Drawer 抽屉"))
            .child(readout(
                theme,
                format!("{} · 背景操作 {} 次", self.last, self.background),
            ));
        let mut declarations = Vec::<AnyElement>::new();
        for (index, (title, description)) in RECIPES.iter().enumerate() {
            let state = self.drawers[index].clone();
            let mut drawer = YueDrawer::new(("drawer-demo", index), theme, &state)
                .title(*title)
                .close_label("关闭抽屉")
                .confirm_label("完成")
                .cancel_label("取消");
            let mut controls = row().child(self.opener(index, "打开示例…", theme, cx));
            match index {
                BASIC => {
                    drawer = drawer
                        .footer(DrawerFooter::None)
                        .content(move |_, _| details(theme, 0).into_any_element());
                }
                DIRECTIONS => {
                    controls = row();
                    for (label, placement) in [
                        ("左侧", DrawerPlacement::Left),
                        ("右侧", DrawerPlacement::Right),
                        ("顶部", DrawerPlacement::Top),
                        ("底部", DrawerPlacement::Bottom),
                    ] {
                        controls = controls.child(
                            YueButton::text(format!("drawer-direction-{label}"), theme, label)
                                .on_click(cx.listener(move |s, _, w, cx| {
                                    s.placement = placement;
                                    s.open(DIRECTIONS, w, cx);
                                })),
                        );
                    }
                    drawer = drawer
                        .placement(self.placement)
                        .size(DrawerSize::Small)
                        .child("沿所属边缘滑入，交叉轴填满可用空间。");
                }
                SIZES => {
                    controls = row();
                    for (label, size) in [
                        ("320", DrawerSize::Small),
                        ("480", DrawerSize::Medium),
                        ("640", DrawerSize::Large),
                        ("Full", DrawerSize::Full),
                    ] {
                        controls = controls.child(
                            YueButton::text(format!("drawer-size-{label}"), theme, label).on_click(
                                cx.listener(move |s, _, w, cx| {
                                    s.size = size;
                                    s.fraction = false;
                                    s.open(SIZES, w, cx);
                                }),
                            ),
                        );
                    }
                    controls =
                        controls.child(YueButton::text("drawer-fraction", theme, "60%").on_click(
                            cx.listener(|s, _, w, cx| {
                                s.fraction = true;
                                s.open(SIZES, w, cx);
                            }),
                        ));
                    drawer = drawer.size(self.size);
                    if self.fraction {
                        drawer = drawer.extent(gpui_kit::relative(0.6));
                    }
                    drawer = drawer.child("调整窗口大小，面板始终受可用区域约束。");
                }
                CUSTOM => {
                    let close = state.clone();
                    drawer = drawer
                        .header(
                            row()
                                .child("工作区")
                                .child(div().text_sm().child("自定义标题")),
                        )
                        .closable(false)
                        .extra(
                            YueButton::icon(
                                "drawer-custom-close",
                                theme,
                                outline::ArrowRight,
                                "返回工作区",
                            )
                            .variant(ButtonVariant::Ghost)
                            .on_click(move |_, w, cx| {
                                close.update(cx, |s, cx| s.close(w, cx));
                            }),
                        )
                        .footer(DrawerFooter::None)
                        .footer_content(row().child("更改已自动保存").child({
                            let close = state.clone();
                            YueButton::text("drawer-custom-done", theme, "返回工作区")
                                .variant(ButtonVariant::Primary)
                                .on_click(move |_, w, cx| {
                                    close.update(cx, |s, cx| s.cancel(w, cx));
                                })
                        }))
                        .child("自定义头尾继续复用 YueButton 和原有关闭入口。");
                }
                TRANSPARENT => {
                    drawer = drawer
                        .overlay(false)
                        .child("没有视觉遮罩，背景依然不能操作。Escape 或关闭按钮返回。");
                }
                NONMODAL => {
                    controls = controls.child(
                        YueButton::text("drawer-background", theme, "背景操作").on_click(
                            cx.listener(|s, _, _, cx| {
                                s.background += 1;
                                cx.notify();
                            }),
                        ),
                    );
                    drawer=drawer.modal(false).size(DrawerSize::Small).footer(DrawerFooter::None).child("试试背景操作按钮，或把焦点移动到背景控件。关闭面板不会夺走已在背景的焦点。");
                }
                LONG => {
                    drawer=drawer.content(move |_,_|column().children((1usize..=80).map(|n|div().flex_shrink_0().child(format!("第 {n} 项：正文独立滚动，标题与底部保持固定。只在面板显示时构建这些段落。")))).into_any_element());
                }
                LOADING => {
                    let loading = self.loading;
                    drawer = drawer.footer(DrawerFooter::None).content(move |_, _| {
                        YueLoadingRegion::new("drawer-load", loading)
                            .label("正在加载记录")
                            .content(move |_, _| details(theme, 1).into_any_element())
                            .into_any_element()
                    });
                }
                FORM => {
                    controls = controls.child(
                        row().child("模拟失败").child(
                            YueSwitch::new("drawer-save-fail", theme, self.fail, "模拟保存失败")
                                .on_change({
                                    let notify = cx.listener(|s, value: &bool, _, cx| {
                                        s.fail = *value;
                                        cx.notify();
                                    });
                                    move |value, _, w, cx| notify(&value, w, cx)
                                }),
                        ),
                    );
                    let pending = state.read(cx).is_pending();
                    let input = self.name.clone();
                    let error = self.error.clone();
                    let focus = self.name.read(cx).focus_handle(cx);
                    let owner = cx.weak_entity();
                    let guard = cx.weak_entity();
                    let fail_owner = cx.weak_entity();
                    let fail = self.fail;
                    drawer =
                        drawer
                            .title("编辑项目")
                            .confirm_label("保存")
                            .initial_focus(&focus)
                            .content(move |_, _| {
                                column()
                                    .child(
                                        YueField::new(theme)
                                            .label("项目名称")
                                            .required(true)
                                            .when_some(error, |field, error| {
                                                field.message(InputStatus::Error, error)
                                            })
                                            .input(
                                                YueInput::new("drawer-name", theme, &input)
                                                    .width(gpui_kit::relative(1.))
                                                    .disabled(pending),
                                            ),
                                    )
                                    .child(
                                        row().child("模拟保存失败").child(
                                            YueSwitch::new(
                                                "drawer-inner-fail",
                                                theme,
                                                fail,
                                                "模拟保存失败",
                                            )
                                            .disabled(pending)
                                            .on_change(move |value, _, _, cx| {
                                                let _ = fail_owner.update(cx, |s, cx| {
                                                    s.fail = value;
                                                    cx.notify();
                                                });
                                            }),
                                        ),
                                    )
                                    .into_any_element()
                            })
                            .on_close_request(move |reason, _, cx| {
                                guard
                                    .update(cx, |s, cx| s.allow_close(reason, cx))
                                    .unwrap_or(false)
                            })
                            .on_confirm(move |request, w, cx| {
                                owner
                                    .update(cx, |s, cx| s.save(request, w, cx))
                                    .unwrap_or(DrawerDecision::KeepOpen)
                            });
                    if self.guarded {
                        let keep = cx.weak_entity();
                        let discard = cx.weak_entity();
                        let close = state.clone();
                        drawer = drawer.footer_content(
                            column()
                                .child("有未保存的更改。继续编辑或放弃更改？")
                                .child(
                                    row()
                                        .child(
                                            YueButton::text("drawer-continue", theme, "继续编辑")
                                                .on_click(move |_, _, cx| {
                                                    let _ = keep.update(cx, |s, cx| {
                                                        s.guarded = false;
                                                        cx.notify();
                                                    });
                                                }),
                                        )
                                        .child(
                                            YueButton::text("drawer-discard", theme, "放弃更改")
                                                .variant(ButtonVariant::Danger)
                                                .on_click(move |_, w, cx| {
                                                    let _ = discard.update(cx, |s, cx| {
                                                        s.discard = true;
                                                        let value = s.saved_name.clone();
                                                        s.name.update(cx, |s, cx| {
                                                            s.set_value(value, w, cx)
                                                        });
                                                        cx.notify();
                                                    });
                                                    close.update(cx, |s, cx| s.cancel(w, cx));
                                                }),
                                        ),
                                ),
                        );
                    }
                }
                NESTED => {
                    let child = self.child.clone();
                    let parent = state.clone();
                    drawer = drawer
                        .footer(DrawerFooter::None)
                        .child("打开子抽屉后，父层保留但无法操作；关闭子层恢复到这里。")
                        .child(
                            YueButton::text("drawer-child-open", theme, "打开子抽屉…").on_click(
                                move |_, w, cx| {
                                    let _ = child.update(cx, |s, cx| s.open_child(&parent, w, cx));
                                },
                            ),
                        );
                }
                RESIZE => {
                    drawer = drawer
                        .resizable(true)
                        .resize_limits(rems(16.), gpui_kit::relative(0.85))
                        .on_resize({
                            let notify = cx.listener(|s, value: &gpui_kit::Pixels, _, cx| {
                                s.resize = format!("当前主轴尺寸 {:.0}px", value.as_f32()).into();
                                cx.notify();
                            });
                            move |value, w, cx| notify(&value, w, cx)
                        })
                        .child("调整窗口后尺寸自动钳制，恢复窗口会恢复首选尺寸。");
                    controls = controls.child(self.resize.clone());
                }
                LOCAL => {
                    controls = controls.child(row().child("推开内容").child(
                        YueSwitch::new("drawer-push", theme, self.push, "推开内容").on_change({
                            let notify = cx.listener(|s, value: &bool, _, cx| {
                                s.push = *value;
                                cx.notify();
                            });
                            move |value, _, w, cx| notify(&value, w, cx)
                        }),
                    ));
                    let local = YueDrawerHost::new(
                        "drawer-local-host",
                        drawer
                            .title("局部详情")
                            .extent(rems(17.))
                            .resizable(true)
                            .resize_limits(rems(12.), rems(26.))
                            .footer(DrawerFooter::None)
                            .child("抽屉始终限制在这个工作区。"),
                    )
                    .push(self.push)
                    .w_full()
                    .h(rems(22.))
                    .child(
                        column()
                            .p_4()
                            .child("工作区文档")
                            .child(
                                "窗口其余区域仍能使用。打开面板后改变 Push 开关，比较覆盖与重排。",
                            )
                            .child(
                                YueButton::text("drawer-local-command", theme, "工作区操作")
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.background += 1;
                                        cx.notify();
                                    })),
                            ),
                    );
                    page = page.child(
                        YueCard::new(("drawer-card", index))
                            .header(YueCardHeader::new(*title).description(*description))
                            .child(column().child(controls).child(local)),
                    );
                    continue;
                }
                DETAILS => {
                    controls = row();
                    for (i, name) in ORDERS.iter().enumerate() {
                        controls = controls.child(
                            YueButton::text(("drawer-order", i), theme, *name).on_click(
                                cx.listener(move |s, _, w, cx| {
                                    s.selected = i;
                                    s.open(DETAILS, w, cx);
                                }),
                            ),
                        );
                    }
                    let selected = self.selected;
                    drawer = drawer
                        .title(format!("WO-{:03} · {}", selected + 1, ORDERS[selected]))
                        .footer(DrawerFooter::None)
                        .extra(
                            row()
                                .child(
                                    YueButton::icon(
                                        "drawer-previous",
                                        theme,
                                        outline::ChevronLeft,
                                        "上一条",
                                    )
                                    .disabled(selected == 0)
                                    .on_click(cx.listener(
                                        |s, _, _, cx| {
                                            s.selected = s.selected.saturating_sub(1);
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    YueButton::icon(
                                        "drawer-next",
                                        theme,
                                        outline::ChevronRight,
                                        "下一条",
                                    )
                                    .disabled(selected == ORDERS.len() - 1)
                                    .on_click(cx.listener(
                                        |s, _, _, cx| {
                                            s.selected = (s.selected + 1).min(ORDERS.len() - 1);
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                        .content(move |_, _| details(theme, selected).into_any_element());
                }
                NO_HEADER => {
                    let close = state.clone();
                    drawer = drawer
                        .label("自定义预览")
                        .header_visible(false)
                        .closable(false)
                        .footer(DrawerFooter::None)
                        .child(div().text_lg().child("自定义预览区域"))
                        .child("语义名称独立于可见标题；仍保留 Escape 与关闭保护。")
                        .child(
                            YueButton::text("drawer-preview-close", theme, "关闭预览").on_click(
                                move |_, w, cx| {
                                    close.update(cx, |s, cx| s.close(w, cx));
                                },
                            ),
                        );
                }
                _ => unreachable!(),
            }
            declarations.push(drawer.into_any_element());
            page = page.child(
                YueCard::new(("drawer-card", index))
                    .header(YueCardHeader::new(*title).description(*description))
                    .child(controls),
            );
        }
        let decision = self.decision.clone();
        let parent = self.child.clone();
        page.children(declarations)
            .child(
                YueDrawer::new("drawer-child", theme, &self.child)
                    .title("子抽屉")
                    .size(DrawerSize::Small)
                    .footer(DrawerFooter::None)
                    .child("父抽屉仍在下方，输入只进入当前层。")
                    .child(
                        YueButton::text("drawer-dialog-open", theme, "打开确认对话框…").on_click(
                            move |_, w, cx| {
                                let _ = decision.update(cx, |s, cx| s.open_child(&parent, w, cx));
                            },
                        ),
                    ),
            )
            .child(
                YueDialog::alert("drawer-decision", theme, &self.decision)
                    .title("继续处理这条记录？")
                    .description("确认后只关闭这一层对话框。"),
            )
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{
        Root,
        test_support::{ElementSnapshot, snapshots},
    };
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, px, size};
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn setup(cx: &mut TestAppContext) -> (Entity<DrawerPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let page = cx.new(|cx| DrawerPage::new(w, cx));
            saved = Some(page.clone());
            Root::new(page, w, cx)
        });
        cx.simulate_resize(size(px(1440.), px(3400.)));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        draw(cx);
        (saved.unwrap(), cx)
    }
    fn node(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label))
                .unwrap_or_else(|| panic!("missing {label}"))
        })
    }
    fn click(cx: &mut VisualTestContext, label: &str) {
        let p = node(cx, label).bounds().center();
        cx.simulate_click(p, Modifiers::default());
        draw(cx);
    }
    fn open(page: &Entity<DrawerPage>, index: usize, cx: &mut VisualTestContext) {
        cx.update(|w, cx| page.update(cx, |s, cx| s.open(index, w, cx)));
        draw(cx);
        draw(cx);
    }
    fn press(cx: &mut VisualTestContext, key: &str) {
        let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
        draw(cx);
        draw(cx);
    }
    #[gpui_kit::test]
    fn all_fourteen_recipes_open_and_release_their_session(cx: &mut TestAppContext) {
        let (page, cx) = setup(cx);
        for index in 0..COUNT {
            open(&page, index, cx);
            assert!(
                page.read_with(cx, |s, cx| s.drawers[index].read(cx).is_open()),
                "recipe {index}"
            );
            let dialogs = cx.update(|w, _| {
                snapshots(w)
                    .into_iter()
                    .filter(|n| n.role() == Some(gpui_kit::Role::Dialog))
                    .collect::<Vec<_>>()
            });
            assert_eq!(dialogs.len(), 1, "recipe {index}: {dialogs:?}");
            let state = page.read_with(cx, |s, _| s.drawers[index].clone());
            cx.update(|w, cx| {
                assert!(state.update(cx, |s, cx| s.close(w, cx)));
            });
            draw(cx);
            draw(cx);
            assert!(!state.read_with(cx, |s, _| s.is_open()));
        }
    }
    #[gpui_kit::test]
    fn real_form_vetoes_unsaved_close_and_retries_failed_save(cx: &mut TestAppContext) {
        let (page, cx) = setup(cx);
        open(&page, FORM, cx);
        let input = page.read_with(cx, |s, _| s.name.clone());
        cx.update(|w, cx| input.update(cx, |s, cx| s.set_value("Changed project", w, cx)));
        draw(cx);
        press(cx, "escape");
        assert!(page.read_with(cx, |s, _| s.guarded));
        click(cx, "继续编辑");
        page.update(cx, |s, cx| {
            s.fail = true;
            cx.notify();
        });
        draw(cx);
        click(cx, "保存");
        assert!(page.read_with(cx, |s, cx| s.drawers[FORM].read(cx).is_pending()));
        cx.executor().advance_clock(Duration::from_millis(710));
        draw(cx);
        draw(cx);
        assert!(page.read_with(cx, |s, _| s.error.is_some()));
        assert_eq!(input.read_with(cx, |s, _| s.value()), "Changed project");
        page.update(cx, |s, cx| {
            s.fail = false;
            cx.notify();
        });
        draw(cx);
        click(cx, "保存");
        cx.executor().advance_clock(Duration::from_millis(710));
        draw(cx);
        draw(cx);
        assert!(!page.read_with(cx, |s, cx| s.drawers[FORM].read(cx).is_open()));
        assert_eq!(
            page.read_with(cx, |s, _| s.saved_name.clone()),
            "Changed project"
        );
    }
    #[gpui_kit::test]
    fn nested_chain_escape_unwinds_one_layer_at_a_time(cx: &mut TestAppContext) {
        let (page, cx) = setup(cx);
        open(&page, NESTED, cx);
        click(cx, "打开子抽屉…");
        draw(cx);
        click(cx, "打开确认对话框…");
        draw(cx);
        assert!(page.read_with(cx, |s, cx| s.decision.read(cx).is_open()));
        press(cx, "escape");
        assert!(!page.read_with(cx, |s, cx| s.decision.read(cx).is_open()));
        assert!(page.read_with(cx, |s, cx| s.child.read(cx).is_open()));
        press(cx, "escape");
        assert!(!page.read_with(cx, |s, cx| s.child.read(cx).is_open()));
        assert!(page.read_with(cx, |s, cx| s.drawers[NESTED].read(cx).is_open()));
        press(cx, "escape");
        assert!(!page.read_with(cx, |s, cx| s.drawers[NESTED].read(cx).is_open()));
    }
    #[gpui_kit::test]
    fn loading_finishes_and_details_change_without_reopen(cx: &mut TestAppContext) {
        let (page, cx) = setup(cx);
        open(&page, LOADING, cx);
        assert!(page.read_with(cx, |s, _| s.loading));
        cx.executor().advance_clock(Duration::from_millis(910));
        draw(cx);
        assert!(!page.read_with(cx, |s, _| s.loading));
        press(cx, "escape");
        open(&page, DETAILS, cx);
        let last = page.read_with(cx, |s, _| s.last.clone());
        click(cx, "下一条");
        assert_eq!(page.read_with(cx, |s, _| s.selected), 1);
        assert_eq!(page.read_with(cx, |s, _| s.last.clone()), last);
        click(cx, "上一条");
        assert_eq!(page.read_with(cx, |s, _| s.selected), 0);
    }
    #[gpui_kit::test]
    fn form_discard_restores_saved_data(cx: &mut TestAppContext) {
        let (page, cx) = setup(cx);
        open(&page, FORM, cx);
        let input = page.read_with(cx, |s, _| s.name.clone());
        cx.update(|w, cx| input.update(cx, |s, cx| s.set_value("Discard me", w, cx)));
        draw(cx);
        press(cx, "escape");
        click(cx, "放弃更改");
        draw(cx);
        assert!(!page.read_with(cx, |s, cx| s.drawers[FORM].read(cx).is_open()));
        assert_eq!(input.read_with(cx, |s, _| s.value()), "Yue Component");
    }
}
