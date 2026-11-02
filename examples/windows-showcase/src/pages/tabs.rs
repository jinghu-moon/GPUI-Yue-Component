use super::super::*;
use gpui_kit::Entity;
use yue_component::{
    InputState, TabsActivation, TabsChangeRequest, TabsCloseRequest, TabsContentMotion, TabsHandle,
    TabsPlacement, TabsScrollAlign, TabsSize, TabsVariant, YueSegmented, YueSegmentedOption,
    YueTabItem, YueTabs,
};

pub(crate) struct TabsPage {
    values: Vec<Option<SharedString>>,
    placement: TabsPlacement,
    variant: TabsVariant,
    size: TabsSize,
    motion: TabsContentMotion,
    icons: bool,
    disabled: bool,
    manual: bool,
    protected: bool,
    lazy: bool,
    dynamic: Vec<SharedString>,
    active_document: Option<SharedString>,
    next_document: usize,
    changes: usize,
    last: String,
    inputs: [Entity<InputState>; 2],
    scroll: TabsHandle,
}
impl TabsPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut values = vec![Some(SharedString::from("one")); 16];
        values[8] = Some("long-0".into());
        Self {
            values,
            placement: TabsPlacement::Top,
            variant: TabsVariant::Normal,
            size: TabsSize::Medium,
            motion: TabsContentMotion::None,
            icons: true,
            disabled: false,
            manual: false,
            protected: false,
            lazy: true,
            dynamic: vec!["文档 1".into(), "文档 2".into()],
            active_document: Some("文档 1".into()),
            next_document: 3,
            changes: 0,
            last: "尚未切换".into(),
            inputs: [
                cx.new(|cx| InputState::new(window, cx).placeholder("切换后仍保留输入")),
                cx.new(|cx| InputState::new(window, cx).placeholder("另一个面板的独立输入")),
            ],
            scroll: TabsHandle::new(),
        }
    }
    fn basic(
        &self,
        id: &'static str,
        slot: usize,
        theme: YueTheme,
        cx: &mut Context<Self>,
    ) -> YueTabs {
        YueTabs::new(id, self.values[slot].clone())
            .theme(theme)
            .accessibility_label(format!("{id} 标签页"))
            .items(items(self.icons, false))
            .height(gpui_kit::rems(8.))
            .on_change(
                cx.listener(move |page, request: &TabsChangeRequest, _, cx| {
                    page.values[slot] = Some(request.key().clone());
                    page.changes += 1;
                    page.last = format!("{} · {:?}", request.key(), request.source());
                    cx.notify();
                }),
            )
    }
    fn toggle(
        &self,
        id: &'static str,
        label: &'static str,
        value: bool,
        field: u8,
        theme: YueTheme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let page = cx.entity();
        YueSwitch::new(id, theme, value, label)
            .size(SwitchSize::Small)
            .on_change(move |checked, _, _, cx| {
                page.update(cx, |page, cx| {
                    match field {
                        0 => page.icons = checked,
                        1 => page.disabled = checked,
                        2 => page.manual = checked,
                        3 => page.protected = checked,
                        _ => page.lazy = checked,
                    }
                    cx.notify();
                })
            })
    }
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().gap_4().min_w_0()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_3()
}
fn items(icons: bool, disabled: bool) -> Vec<YueTabItem> {
    [("one", "常规"), ("two", "外观"), ("three", "高级")]
        .into_iter()
        .enumerate()
        .map(|(ix, (key, label))| {
            let item = YueTabItem::new(key, label)
                .disabled(disabled && ix == 1)
                .content(move |_, _| {
                    div()
                        .p_4()
                        .child(format!("{label}的内容面板"))
                        .into_any_element()
                });
            if icons {
                match ix {
                    0 => item.icon(outline::Settings),
                    1 => item.icon(outline::Palette),
                    _ => item.icon(outline::Adjustments),
                }
            } else {
                item
            }
        })
        .collect()
}
impl Render for TabsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let size_value = match self.size {
            TabsSize::Small => "small",
            TabsSize::Medium => "medium",
            TabsSize::Large => "large",
        };
        let placement_value = match self.placement {
            TabsPlacement::Top => "top",
            TabsPlacement::Bottom => "bottom",
            TabsPlacement::Left => "left",
            TabsPlacement::Right => "right",
        };
        let motion_value = match self.motion {
            TabsContentMotion::None => "none",
            TabsContentMotion::Fade => "fade",
            TabsContentMotion::Slide => "slide",
        };
        let settings = column()
            .child(
                row().child("外观").child(
                    YueSegmented::new(
                        "tabs-variant",
                        theme,
                        if self.variant == TabsVariant::Normal {
                            "normal"
                        } else {
                            "card"
                        },
                    )
                    .label("外观")
                    .options([
                        YueSegmentedOption::new("normal", "线形"),
                        YueSegmentedOption::new("card", "卡片"),
                    ])
                    .on_change(cx.listener(
                        |p, value: &SharedString, _, cx| {
                            p.variant = if value.as_ref() == "card" {
                                TabsVariant::Card
                            } else {
                                TabsVariant::Normal
                            };
                            cx.notify();
                        },
                    )),
                ),
            )
            .child(
                row().child("位置").child(
                    YueSegmented::new("tabs-placement", theme, placement_value)
                        .label("位置")
                        .options(
                            [
                                ("top", "上"),
                                ("bottom", "下"),
                                ("left", "左"),
                                ("right", "右"),
                            ]
                            .map(|(v, l)| YueSegmentedOption::new(v, l)),
                        )
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.placement = match value.as_ref() {
                                "bottom" => TabsPlacement::Bottom,
                                "left" => TabsPlacement::Left,
                                "right" => TabsPlacement::Right,
                                _ => TabsPlacement::Top,
                            };
                            cx.notify();
                        })),
                ),
            )
            .child(
                row().child("尺寸").child(
                    YueSegmented::new("tabs-size", theme, size_value)
                        .label("尺寸")
                        .options(
                            [("small", "小"), ("medium", "中"), ("large", "大")]
                                .map(|(v, l)| YueSegmentedOption::new(v, l)),
                        )
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.size = match value.as_ref() {
                                "small" => TabsSize::Small,
                                "large" => TabsSize::Large,
                                _ => TabsSize::Medium,
                            };
                            cx.notify();
                        })),
                ),
            )
            .child(
                row().child("内容动效").child(
                    YueSegmented::new("tabs-motion", theme, motion_value)
                        .label("内容动效")
                        .options(
                            [("none", "即时"), ("fade", "淡入"), ("slide", "平移")]
                                .map(|(v, l)| YueSegmentedOption::new(v, l)),
                        )
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.motion = match value.as_ref() {
                                "fade" => TabsContentMotion::Fade,
                                "slide" => TabsContentMotion::Slide,
                                _ => TabsContentMotion::None,
                            };
                            cx.notify();
                        })),
                ),
            )
            .child(
                row()
                    .child(self.toggle("tabs-icons", "前置图标", self.icons, 0, theme, cx))
                    .child(self.toggle("tabs-disabled", "整组禁用", self.disabled, 1, theme, cx))
                    .child(self.toggle(
                        "tabs-manual",
                        "按 Enter 确认切换",
                        self.manual,
                        2,
                        theme,
                        cx,
                    )),
            );
        let preview = self
            .basic("tabs-preview", 0, theme, cx)
            .height(gpui_kit::rems(12.))
            .placement(self.placement)
            .size(self.size)
            .variant(self.variant)
            .content_motion(self.motion)
            .disabled(self.disabled)
            .activation(if self.manual {
                TabsActivation::Manual
            } else {
                TabsActivation::Automatic
            });
        let documents = self
            .dynamic
            .iter()
            .map(|key| {
                let label = key.clone();
                YueTabItem::new(key.clone(), key.clone())
                    .closable(true)
                    .icon(outline::File)
                    .content(move |_, _| {
                        div()
                            .p_4()
                            .child(format!("{label} 的内容；关闭与切换是独立命令"))
                            .into_any_element()
                    })
            })
            .collect::<Vec<_>>();
        let page = cx.entity();
        let dynamic = YueTabs::new("tabs-documents", self.active_document.clone())
            .theme(theme)
            .variant(TabsVariant::Card)
            .height(gpui_kit::rems(8.))
            .addable(true)
            .items(documents)
            .empty(|_, _| {
                div()
                    .p_4()
                    .child("没有打开的文档，使用 + 新建")
                    .into_any_element()
            })
            .on_change(cx.listener(|p, r: &TabsChangeRequest, _, cx| {
                p.active_document = Some(r.key().clone());
                cx.notify();
            }))
            .on_close(cx.listener(|p, r: &TabsCloseRequest, _, cx| {
                p.dynamic.retain(|key| key != r.key());
                if p.active_document.as_ref() == Some(r.key()) {
                    p.active_document = r.next_key().cloned();
                }
                p.last = format!("已关闭 {}", r.key());
                cx.notify();
            }))
            .on_add(move |_, cx| {
                page.update(cx, |p, cx| {
                    let key: SharedString = format!("文档 {}", p.next_document).into();
                    p.next_document += 1;
                    p.dynamic.push(key.clone());
                    p.active_document = Some(key);
                    cx.notify();
                })
            });
        let sizes = column().children(
            [TabsSize::Small, TabsSize::Medium, TabsSize::Large]
                .into_iter()
                .enumerate()
                .map(|(ix, size)| {
                    self.basic(
                        ["tabs-small", "tabs-medium", "tabs-large"][ix],
                        ix + 2,
                        theme,
                        cx,
                    )
                    .size(size)
                }),
        );
        let positions = column().children(
            [
                TabsPlacement::Top,
                TabsPlacement::Bottom,
                TabsPlacement::Left,
                TabsPlacement::Right,
            ]
            .into_iter()
            .enumerate()
            .map(|(ix, placement)| {
                self.basic(
                    ["tabs-top", "tabs-bottom", "tabs-left", "tabs-right"][ix],
                    ix + 4,
                    theme,
                    cx,
                )
                .placement(placement)
            }),
        );
        let long = self
            .basic("tabs-scroll", 8, theme, cx)
            .handle(&self.scroll)
            .items((0..25).map(|ix| {
                let label = format!("选项卡 {}", ix + 1);
                let text = label.clone();
                YueTabItem::new(format!("long-{ix}"), label).content(move |_, _| {
                    div()
                        .p_4()
                        .child(format!("{text} 的面板"))
                        .into_any_element()
                })
            }));
        let alignment = row().children(
            [
                ("start", "起始", TabsScrollAlign::Start),
                ("center", "居中", TabsScrollAlign::Center),
                ("end", "末尾", TabsScrollAlign::End),
            ]
            .into_iter()
            .map(|(id, label, align)| {
                let handle = self.scroll.clone();
                let key = self.values[8].clone().unwrap_or_else(|| "long-0".into());
                YueButton::new(id, label)
                    .size(ButtonSize::Small)
                    .on_click(move |_, w, cx| handle.scroll_to(key.clone(), align, w, cx))
            }),
        );
        let panes = self
            .inputs
            .iter()
            .enumerate()
            .map(|(ix, input)| {
                let input = input.clone();
                let id = if ix == 0 { "one" } else { "two" };
                YueTabItem::new(
                    id,
                    if ix == 0 {
                        "基本资料"
                    } else {
                        "其他资料"
                    },
                )
                .panel_tab_stop(false)
                .content(move |_, cx| {
                    let theme = yue_component::current_theme(cx);
                    div()
                        .p_4()
                        .child(
                            YueInput::new(("tabs-persistent-input", ix), theme, &input)
                                .width(gpui_kit::rems(18.)),
                        )
                        .into_any_element()
                })
            })
            .collect::<Vec<_>>();
        let lazy = self
            .basic("tabs-lazy", 9, theme, cx)
            .lazy(self.lazy)
            .items(panes);
        let guarded = YueTabs::new("tabs-guard", self.values[10].clone())
            .theme(theme)
            .height(gpui_kit::rems(8.))
            .items(items(false, false))
            .on_change(cx.listener(|p, r: &TabsChangeRequest, _, cx| {
                if p.protected {
                    p.last = format!("保留当前页：拒绝切换到 {}", r.key());
                } else {
                    p.values[10] = Some(r.key().clone());
                    p.last = format!("已切换到 {}", r.key());
                }
                cx.notify();
            }));
        column().debug_selector(||"tabs-content".into()).p_8().gap_6()
            .child(div().text_2xl().child("Tabs 标签页"))
            .child(readout(theme,"同级内容面板的受控切换；焦点、选中和业务数据分别保留。".into()))
            .child(section(theme,"交互预览","方向键导航，Home / End 到两端；手动模式使用 Enter / Space。",column().child(settings).child(preview)))
            .child(readout(theme,format!("切换 {} 次 · {}",self.changes,self.last)))
            .child(section(theme,"基础与卡片","线形使用共享指示线；卡片使用选中表面。",column().child(self.basic("tabs-normal",1,theme,cx)).child(self.basic("tabs-card",11,theme,cx).variant(TabsVariant::Card))))
            .child(section(theme,"新增与关闭","+ 新建并选中；关闭按钮不激活标签；聚焦后 Delete 关闭。",dynamic))
            .child(section(theme,"三个尺寸","字号、图标、间距和命令热区成套缩放。",sizes))
            .child(section(theme,"四个位置","左、右布局仍使用横排文字，键盘改用上下方向键。",positions))
            .child(section(theme,"滚动与定位","多标签保持单行。滚动命令在轨道之外，定位命令面向当前选中项。",column().child(alignment).child(long)))
            .child(section(theme,"禁用选项","禁用项不响应切换；禁用选中标签不抹掉已显示的内容。",self.basic("tabs-disabled-option",12,theme,cx).items(items(true,true))))
            .child(section(theme,"受控拒绝","焦点可以移动；宿主未接受时，选中指示和内容保持原样。",column().child(self.toggle("tabs-protect","保护当前页",self.protected,3,theme,cx)).child(guarded)))
            .child(section(theme,"懒渲染与输入保留","输入 Entity 由宿主持有。切换标签后输入仍在，隐藏面板不接收键盘或鼠标。",column().child(self.toggle("tabs-lazy-toggle","懒渲染",self.lazy,4,theme,cx)).child(lazy)))
            .child(readout(theme,"动效：指示线平移 150ms；远跳直接定位。可选内容淡入 / 8px 平移只绘制新面板，旧交互树立即移除。全局关闭动效会立即到位。".into()))
    }
}
