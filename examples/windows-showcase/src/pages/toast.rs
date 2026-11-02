use super::super::*;
use gpui_kit::{Entity, Subscription, Task};
use std::time::Duration;
use yue_component::{
    ToastAction, ToastAppearance, ToastDecision, ToastEvent, ToastKind, ToastPlayback,
    ToastPosition, ToastSpec, ToastTicket, ToasterOptions, YueDialog, YueDialogState, YueSegmented,
    YueSegmentedOption, YueToaster, YueToasterState,
};

struct ToastDemo {
    toaster: Entity<YueToasterState>,
    dialog: Entity<YueDialogState>,
    pending: Option<ToastPosition>,
    description: bool,
    icon: bool,
    action: bool,
    close: bool,
    progress: bool,
    appearance: ToastAppearance,
    latest: Option<ToastTicket>,
    history: Vec<ToastTicket>,
    log: SharedString,
    serial: usize,
    task: Option<Task<()>>,
    _events: Subscription,
    _changes: Subscription,
}
impl ToastDemo {
    pub fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let toaster = YueToaster::for_window(w, cx).expect("Root installs the window's Toaster");
        let events = cx.subscribe_in(&toaster, w, |s, _, event: &ToastEvent, _, cx| {
            s.log = format!("{event:?}").into();
            cx.notify();
        });
        let changes = cx.observe_in(&toaster, w, |s, _, w, cx| {
            if s.toaster.read(cx).is_empty()
                && let Some(position) = s.pending.take()
            {
                s.toaster
                    .update(cx, |state, cx| {
                        state.configure(ToasterOptions::new().position(position), w, cx)
                    })
                    .expect("empty after exit");
            }
            cx.notify();
        });
        Self {
            toaster,
            dialog: cx.new(YueDialogState::new),
            pending: None,
            description: true,
            icon: true,
            action: false,
            close: true,
            progress: true,
            appearance: ToastAppearance::Standard,
            latest: None,
            history: vec![],
            log: "尚无通知".into(),
            serial: 0,
            task: None,
            _events: events,
            _changes: changes,
        }
    }
    fn add(&mut self, kind: ToastKind, w: &mut Window, cx: &mut Context<Self>) {
        self.serial += 1;
        let mut spec = ToastSpec::new(format!(
            "{} · 第 {} 条通知",
            match kind {
                ToastKind::Normal => "已保存项目设置",
                ToastKind::Success => "导出完成",
                ToastKind::Info => "同步使用最新设置",
                ToastKind::Warning => "部分文件尚未同步",
                ToastKind::Error => "导出失败",
                ToastKind::Loading => "正在导出文件",
            },
            self.serial
        ))
        .kind(kind)
        .appearance(self.appearance)
        .dismissible(self.close && kind != ToastKind::Loading)
        .progress(self.progress);
        if self.description {
            spec = spec.description("这是当前操作的结果。指针进入后展开，离开后继续剩余倒计时。");
        }
        if !self.icon {
            spec = spec.icon(None);
        }
        if self.action {
            spec = spec.action(ToastAction::new("撤销", |ticket, w, cx| {
                let _ = ticket.update(
                    ToastSpec::new("已撤销这次操作")
                        .kind(ToastKind::Info)
                        .timeout(Some(Duration::from_secs(4))),
                    w,
                    cx,
                );
                ToastDecision::KeepOpen
            }));
        }
        self.accept(spec, w, cx);
    }
    fn accept(&mut self, spec: ToastSpec, w: &mut Window, cx: &mut Context<Self>) {
        match self.toaster.update(cx, |s, cx| s.push(spec, w, cx)) {
            Ok(ticket) => {
                self.latest = Some(ticket.clone());
                self.history.push(ticket);
                if self.history.len() > 32 {
                    self.history.remove(0);
                }
            }
            Err(error) => self.log = error.to_string().into(),
        }
        cx.notify();
    }
}
fn row() -> gpui_kit::Div {
    div().flex().min_w_0().items_center().flex_wrap().gap_3()
}
fn column() -> gpui_kit::Div {
    div().w_full().flex().flex_col().gap_4()
}
impl Render for ToastDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let position = self.toaster.read(cx).options().toast_position();
        let playback = self.toaster.read(cx).playback();
        let mut positions = column().max_w(gpui_kit::rems(36.));
        for (r, items) in [
            [
                Some(ToastPosition::TopLeft),
                Some(ToastPosition::Top),
                Some(ToastPosition::TopRight),
            ],
            [Some(ToastPosition::Left), None, Some(ToastPosition::Right)],
            [
                Some(ToastPosition::BottomLeft),
                Some(ToastPosition::Bottom),
                Some(ToastPosition::BottomRight),
            ],
        ]
        .into_iter()
        .enumerate()
        {
            let mut line = div().w_full().flex().gap_3();
            for (col, p) in items.into_iter().enumerate() {
                line = line.child(div().flex_1().min_w_0().child(if let Some(p) = p {
                    YueButton::text(("toast-position", r * 3 + col), theme, p.label())
                        .width(gpui_kit::relative(1.))
                        .variant(if position == p {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Default
                        })
                        .on_click(cx.listener(move |s, _, w, cx| {
                            s.pending = Some(p);
                            s.toaster.update(cx, |state, cx| {
                                state.set_playback(ToastPlayback::Normal, w, cx);
                                state.dismiss_all(w, cx);
                                if state.is_empty() {
                                    state
                                        .configure(ToasterOptions::new().position(p), w, cx)
                                        .unwrap();
                                    s.pending = None;
                                }
                            });
                            cx.notify();
                        }))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .h(gpui_kit::rems(2.25))
                        .text_sm()
                        .child("窗口内容")
                        .into_any_element()
                }));
            }
            positions = positions.child(line);
        }
        let mut switches = row();
        let owner = cx.weak_entity();
        for (ix, label, value) in [
            (0, "描述", self.description),
            (1, "前置图标", self.icon),
            (2, "单个撤销操作", self.action),
            (3, "关闭按钮", self.close),
            (4, "倒计时条", self.progress),
        ] {
            let owner = owner.clone();
            switches = switches.child(
                YueSwitch::new(("toast-part", ix as usize), theme, value, label)
                    .size(SwitchSize::Small)
                    .on_change(move |value, _, _, cx| {
                        let _ = owner.update(cx, |s, cx| {
                            match ix {
                                0 => s.description = value,
                                1 => s.icon = value,
                                2 => s.action = value,
                                3 => s.close = value,
                                _ => s.progress = value,
                            }
                            cx.notify();
                        });
                    }),
            );
        }
        let mut kinds = row();
        for (ix, kind, label) in [
            (0, ToastKind::Normal, "普通"),
            (1, ToastKind::Success, "成功"),
            (2, ToastKind::Info, "信息"),
            (3, ToastKind::Warning, "警告"),
            (4, ToastKind::Error, "错误"),
            (5, ToastKind::Loading, "加载"),
        ] {
            kinds = kinds.child(
                YueButton::text(("toast-kind", ix as usize), theme, label)
                    .on_click(cx.listener(move |s, _, w, cx| s.add(kind, w, cx))),
            );
        }
        let appearance = YueSegmented::new(
            "toast-appearance",
            theme,
            match self.appearance {
                ToastAppearance::Standard => "standard",
                ToastAppearance::Rich => "rich",
                ToastAppearance::Inverse => "inverse",
            },
        )
        .label("通知外观")
        .options([
            YueSegmentedOption::new("standard", "标准"),
            YueSegmentedOption::new("rich", "柔和彩色"),
            YueSegmentedOption::new("inverse", "反色"),
        ])
        .on_change(cx.listener(|s, value: &SharedString, _, cx| {
            s.appearance = match value.as_str() {
                "rich" => ToastAppearance::Rich,
                "inverse" => ToastAppearance::Inverse,
                _ => ToastAppearance::Standard,
            };
            cx.notify();
        }));
        let playback_control = YueSegmented::new(
            "toast-playback",
            theme,
            match playback {
                ToastPlayback::Normal => "normal",
                ToastPlayback::Slow => "slow",
                ToastPlayback::Paused => "paused",
            },
        )
        .label("通知动效播放速度")
        .options([
            YueSegmentedOption::new("normal", "正常"),
            YueSegmentedOption::new("slow", "四分之一慢放"),
            YueSegmentedOption::new("paused", "暂停"),
        ])
        .on_change(cx.listener(|s, value: &SharedString, w, cx| {
            let playback = match value.as_str() {
                "slow" => ToastPlayback::Slow,
                "paused" => ToastPlayback::Paused,
                _ => ToastPlayback::Normal,
            };
            s.toaster
                .update(cx, |t, cx| t.set_playback(playback, w, cx));
            cx.notify();
        }));
        let labs=column().child(row()
            .child(YueButton::text("toast-burst",theme,"连续加入三张").on_click(cx.listener(|s,_,w,cx|{for kind in [ToastKind::Success,ToastKind::Info,ToastKind::Warning]{s.add(kind,w,cx);}})))
            .child(YueButton::text("toast-tall",theme,"加入长内容").on_click(cx.listener(|s,_,w,cx|s.accept(ToastSpec::new("文件列表已更新").description("项目/资源/很长的文件路径/项目说明与最终交付内容.md\n第二行：不同自然高度不会撑出折叠后景。\n第三行：展开后恢复完整内容。\n第四行：操作与关闭各有独立空间。").timeout(None),w,cx))))
            .child(YueButton::text("toast-update",theme,"原位更新最后一条").on_click(cx.listener(|s,_,w,cx|{if let Some(ticket)=&s.latest && let Err(e)=ticket.update(ToastSpec::new("原位置更新完成").kind(ToastKind::Success).description("这是新的完整描述符，原来的动作已被清除。\n高度变化由布局弹簧连续衔接。"),w,cx){s.log=e.to_string().into();}cx.notify();})))
            .child(YueButton::text("toast-middle",theme,"关闭中间项").on_click(cx.listener(|s,_,w,cx|{let current=s.toaster.read(cx).snapshots(cx);if let Some(snapshot)=current.get(current.len()/2)&&let Some(ticket)=s.history.iter().find(|t|t.generation()==snapshot.generation){let _=ticket.dismiss(w,cx);}cx.notify();})))
            .child(YueButton::text("toast-clear",theme,"关闭全部").on_click(cx.listener(|s,_,w,cx|s.toaster.update(cx,|t,cx|t.dismiss_all(w,cx))))))
            .child(row().child(playback_control)
                .child(YueButton::text("toast-step",theme,"单步 16ms").disabled(playback!=ToastPlayback::Paused).on_click(cx.listener(|s,_,w,cx|s.toaster.update(cx,|t,cx|t.step(Duration::from_millis(16),w,cx)))))
                .child(YueButton::text("toast-focus",theme,"聚焦通知").on_click(cx.listener(|s,_,w,cx|{s.toaster.update(cx,|t,cx|t.focus_notifications(w,cx));}))))
            .child(row()
                .child(YueButton::text("toast-async",theme,"异步导出 · 2 秒").on_click(cx.listener(|s,_,w,cx|{
                    s.accept(ToastSpec::new("正在导出项目…").id("showcase-export").kind(ToastKind::Loading),w,cx);
                    let Some(ticket)=s.latest.clone() else{return;};
                    s.task=Some(cx.spawn_in(w,async move|_,cx|{cx.background_executor().timer(Duration::from_secs(2)).await;let _=cx.update(|w,cx|ticket.update(ToastSpec::new("导出成功").kind(ToastKind::Success).description("结果保留原 ID，切换图标并启动新的四秒倒计时。").progress(true),w,cx));}));
                })))
                .child(YueButton::text("toast-modal",theme,"模态期间加入通知").on_click(cx.listener(|s,_,w,cx|{s.dialog.update(cx,|d,cx|d.open(w,cx)).unwrap();s.accept(ToastSpec::new("模态关闭后再出现").kind(ToastKind::Info).progress(true),w,cx);}))))
            .child(readout(theme,self.log.to_string()));
        let snapshots = self.toaster.read(cx).snapshots(cx);
        let diagnostic = column()
            .child(format!(
                "{} 条 · {:?} · {}",
                snapshots.len(),
                position,
                if self.toaster.read(cx).is_paused() {
                    "阅读计时暂停"
                } else {
                    "阅读计时运行"
                }
            ))
            .children(snapshots.iter().map(|s| {
                div().text_sm().child(format!(
                    "{} · gen {} · {:?} · 剩余 {:?}",
                    s.id.as_str(),
                    s.generation,
                    s.phase,
                    s.remaining
                ))
            }));
        column()
            .p_8()
            .gap_6()
            .debug_selector(|| "toast-content".into())
            .child(div().text_2xl().child("Toast 消息通知"))
            .child(readout(
                theme,
                "300ms 入场 · 400ms response 重排 · 200ms 退出 · 每张独立倒计时".into(),
            ))
            .child(section(
                theme,
                "八方位",
                "非空时先退出，再切换位置；通知属于整个窗口。",
                positions,
            ))
            .child(section(
                theme,
                "卡片组成与状态",
                "只有一个可选业务动作，关闭入口独立。",
                column().child(switches).child(appearance).child(kinds),
            ))
            .child(section(
                theme,
                "动效实验",
                "快速加入、hover 反向、退出途中再加入、原位更新均可独立检查。",
                labs,
            ))
            .child(section(
                theme,
                "状态诊断",
                "单步同时推进存在性、阅读时间和重排；不改变其他组件时钟。",
                diagnostic,
            ))
            .child(
                YueDialog::new("toast-modal-demo", theme, &self.dialog)
                    .title("通知暂停演示")
                    .child("此时已有通知隐藏并暂停，新通知等待；退出屏障结束后恢复。"),
            )
    }
}

/// Resolve the Root plugin only after the application view has been mounted.
pub(crate) struct ToastPage {
    demo: Option<Entity<ToastDemo>>,
}
impl ToastPage {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self { demo: None }
    }
}
impl Render for ToastPage {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.demo.is_none() && YueToaster::for_window(w, cx).is_ok() {
            self.demo = Some(cx.new(|cx| ToastDemo::new(w, cx)));
        }
        div()
            .w_full()
            .when_some(self.demo.clone(), |d, demo| d.child(demo))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{Modifiers, Role, TestAppContext, VisualTestContext, px, size};
    use yue_component::ToastPhase;

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn tick(cx: &mut VisualTestContext, ms: u64) {
        cx.executor().advance_clock(Duration::from_millis(ms));
        draw(cx);
        draw(cx);
    }
    fn click(cx: &mut VisualTestContext, label: &str, role: Role) {
        let center = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label) && n.role() == Some(role) && n.visible())
                .unwrap_or_else(|| panic!("missing {role:?}: {label}"))
                .bounds()
                .center()
        });
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
        draw(cx);
    }
    fn setup(cx: &mut TestAppContext) -> (Entity<ToastDemo>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| ToastPage::new(w, cx));
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(size(px(1300.), px(1500.)));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        draw(cx);
        let demo = page
            .unwrap()
            .read_with(cx, |p, _| p.demo.clone())
            .expect("lazy Root lookup");
        (demo, cx)
    }

    #[gpui_kit::test]
    fn parts_positions_and_stack_controls_use_the_window_toaster(cx: &mut TestAppContext) {
        let (demo, cx) = setup(cx);
        let state = demo.read_with(cx, |s, _| s.toaster.clone());
        click(cx, "单个撤销操作", Role::Switch);
        click(cx, "前置图标", Role::Switch);
        assert!(demo.read_with(cx, |s, _| s.action && !s.icon));
        click(cx, "连续加入三张", Role::Button);
        assert_eq!(state.read_with(cx, |s, _| s.len()), 3);
        let generation = demo.read_with(cx, |s, _| s.latest.as_ref().unwrap().generation());
        click(cx, "原位更新最后一条", Role::Button);
        tick(cx, 160);
        assert_eq!(
            demo.read_with(cx, |s, _| s.latest.as_ref().unwrap().generation()),
            generation
        );
        click(cx, "关闭中间项", Role::Button);
        assert_eq!(state.read_with(cx, |s, _| s.len()), 2);
        for position in ToastPosition::ALL {
            click(cx, position.label(), Role::Button);
            assert!(state.read_with(cx, |s, _| s.is_empty()));
            assert_eq!(
                state.read_with(cx, |s, _| s.options().toast_position()),
                position
            );
        }
        click(cx, "普通", Role::Button);
        assert_eq!(state.read_with(cx, |s, _| s.len()), 1);
        click(cx, "关闭全部", Role::Button);
        assert!(state.read_with(cx, |s, _| s.is_empty()));
    }

    #[gpui_kit::test]
    fn asynchronous_result_modal_waiting_and_diagnostic_step_work(cx: &mut TestAppContext) {
        let (demo, cx) = setup(cx);
        let state = demo.read_with(cx, |s, _| s.toaster.clone());
        click(cx, "异步导出 · 2 秒", Role::Button);
        let before = state.read_with(cx, |s, cx| s.snapshots(cx));
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].remaining, None);
        tick(cx, 2000);
        let after = state.read_with(cx, |s, cx| s.snapshots(cx));
        assert_eq!(before[0].generation, after[0].generation);
        assert_eq!(after[0].remaining, Some(Duration::from_secs(4)));
        assert!(cx.update(|w, _| {
            snapshots(w)
                .iter()
                .any(|n| n.label().is_some_and(|s| s.contains("导出成功")))
        }));
        click(cx, "暂停", Role::RadioButton);
        let frozen = state.read_with(cx, |s, cx| s.snapshots(cx)[0].remaining.unwrap());
        tick(cx, 1000);
        assert_eq!(
            state.read_with(cx, |s, cx| s.snapshots(cx)[0].remaining.unwrap()),
            frozen
        );
        click(cx, "单步 16ms", Role::Button);
        assert_eq!(
            state.read_with(cx, |s, cx| s.snapshots(cx)[0].remaining.unwrap()),
            frozen - Duration::from_millis(16)
        );
        click(cx, "正常", Role::RadioButton);
        click(cx, "关闭全部", Role::Button);
        click(cx, "模态期间加入通知", Role::Button);
        assert_eq!(
            state.read_with(cx, |s, cx| s.snapshots(cx)[0].phase),
            ToastPhase::Waiting
        );
        let dialog = demo.read_with(cx, |s, _| s.dialog.clone());
        cx.update(|w, cx| {
            dialog.update(cx, |s, cx| s.close(w, cx));
        });
        draw(cx);
        draw(cx);
        assert_eq!(
            state.read_with(cx, |s, cx| s.snapshots(cx)[0].phase),
            ToastPhase::Present
        );
    }
}
