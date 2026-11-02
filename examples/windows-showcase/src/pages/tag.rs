use super::super::*;
use gpui_kit::{Entity, FocusHandle, Subscription};
use yue_component::{
    InputEvent, InputState, TagColors, TagShape, TagSize, TagTone, TagVariant, YueSegmented,
    YueSegmentedOption, YueTag, YueTagGroup,
};

gpui_kit::actions!(tag_page, [CancelAddTag]);

const TONES: [(&str, TagTone); 5] = [
    ("中性", TagTone::Default),
    ("主色", TagTone::Primary),
    ("成功", TagTone::Success),
    ("警告", TagTone::Warning),
    ("危险", TagTone::Danger),
];
const VARIANTS: [(&str, TagVariant); 4] = [
    ("实色", TagVariant::Filled),
    ("浅色", TagVariant::Tint),
    ("描边", TagVariant::Outline),
    ("浅色描边", TagVariant::TintOutline),
];

struct PathTag {
    id: usize,
    label: SharedString,
    focus: FocusHandle,
    close: FocusHandle,
}

pub(crate) struct TagPage {
    mode: u8,
    tone: TagTone,
    variant: TagVariant,
    size: TagSize,
    shape: TagShape,
    disabled: bool,
    closable: bool,
    leading: bool,
    preview_visible: bool,
    preview_checked: bool,
    actions: usize,
    closes: usize,
    checks: [[bool; 3]; 3],
    single: Option<usize>,
    copy_path: bool,
    open_folder: bool,
    scale: usize,
    paths: Vec<PathTag>,
    next_id: usize,
    last_path: SharedString,
    adding: bool,
    input: Entity<InputState>,
    add_focus: FocusHandle,
    error: Option<SharedString>,
    _input_events: Subscription,
}
impl TagPage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.bind_keys([gpui_kit::KeyBinding::new(
            "escape",
            CancelAddTag,
            Some("TagEditor"),
        )]);
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入标签名称，Enter 添加"));
        let events = cx.subscribe_in(&input, window, |page: &mut Self, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) && page.adding {
                page.submit(window, cx);
            }
        });
        let paths = ["截图目录", "项目素材", "下载目录"]
            .into_iter()
            .enumerate()
            .map(|(id, label)| PathTag {
                id,
                label: label.into(),
                focus: cx.focus_handle(),
                close: cx.focus_handle(),
            })
            .collect();
        Self {
            mode: 0,
            tone: TagTone::Default,
            variant: TagVariant::Tint,
            size: TagSize::Medium,
            shape: TagShape::Rounded,
            disabled: false,
            closable: false,
            leading: false,
            preview_visible: true,
            preview_checked: false,
            actions: 0,
            closes: 0,
            checks: [[true, false, false]; 3],
            single: Some(0),
            copy_path: true,
            open_folder: false,
            scale: 100,
            paths,
            next_id: 3,
            last_path: "尚未选择".into(),
            adding: false,
            input,
            add_focus: cx.focus_handle(),
            error: None,
            _input_events: events,
        }
    }

    fn remove_path(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.paths.iter().position(|p| p.id == id) else {
            return;
        };
        let focused =
            self.paths[ix].focus.is_focused(window) || self.paths[ix].close.is_focused(window);
        self.paths.remove(ix);
        self.closes += 1;
        if focused {
            if let Some(next) = self.paths.get(ix).or_else(|| self.paths.last()) {
                next.focus.focus(window, cx);
            } else {
                self.add_focus.focus(window, cx);
            }
        }
        cx.notify();
    }

    fn begin_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.adding = true;
        self.error = None;
        self.input.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.focus(window, cx);
        });
        cx.notify();
    }

    fn cancel_add(&mut self, _: &CancelAddTag, window: &mut Window, cx: &mut Context<Self>) {
        self.adding = false;
        self.error = None;
        self.add_focus.focus(window, cx);
        cx.notify();
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label: SharedString = self.input.read(cx).value().trim().to_owned().into();
        if label.is_empty() {
            self.error = Some("请输入标签名称".into());
        } else if self.paths.iter().any(|p| p.label == label) {
            self.error = Some("此标签已存在".into());
        } else {
            let focus = cx.focus_handle();
            self.paths.push(PathTag {
                id: self.next_id,
                label,
                focus: focus.clone(),
                close: cx.focus_handle(),
            });
            self.next_id += 1;
            self.adding = false;
            self.error = None;
            focus.focus(window, cx);
        }
        cx.notify();
    }
}

fn column() -> gpui_kit::Div {
    div().w_full().flex().flex_col().items_start().gap_4()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_4()
}

impl Render for TagPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut preview = match self.mode {
            0 => YueTag::new("tag-preview", "标签 Tag"),
            1 => YueTag::action("tag-preview", "标签 Tag").on_click(cx.listener(|p, _, _, cx| {
                p.actions += 1;
                cx.notify();
            })),
            _ => YueTag::checkable("tag-preview", "标签 Tag", self.preview_checked).on_change({
                let page = cx.entity();
                move |checked, _, _, cx| {
                    page.update(cx, |p, cx| {
                        p.preview_checked = checked;
                        cx.notify();
                    })
                }
            }),
        }
        .tone(self.tone)
        .variant(self.variant)
        .size(self.size)
        .shape(self.shape)
        .disabled(self.disabled)
        .when(self.leading, |tag| tag.leading_icon(outline::Tag));
        if self.closable {
            preview = preview.on_close(cx.listener(|p, _, _, cx| {
                p.closes += 1;
                p.preview_visible = false;
                cx.notify();
            }));
        }
        let mut switches = row();
        for (id, label, checked) in [
            ("disabled", "禁用", self.disabled),
            ("closable", "可关闭", self.closable),
            ("leading", "前置图标", self.leading),
        ] {
            let page = cx.entity();
            switches = switches.child(
                row().child(label).child(
                    YueSwitch::new(format!("tag-{id}"), theme, checked, label)
                        .size(SwitchSize::Small)
                        .on_change(move |checked, _, _, cx| {
                            page.update(cx, |p, cx| {
                                match id {
                                    "disabled" => p.disabled = checked,
                                    "closable" => p.closable = checked,
                                    _ => p.leading = checked,
                                }
                                cx.notify();
                            })
                        }),
                ),
            );
        }
        let settings = column()
            .child(switches)
            .child(
                row().child("主体行为").child(
                    YueSegmented::new("tag-mode", theme, self.mode.to_string())
                        .label("标签主体行为")
                        .options([
                            YueSegmentedOption::new("0", "静态"),
                            YueSegmentedOption::new("1", "动作"),
                            YueSegmentedOption::new("2", "可选"),
                        ])
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.mode = value.parse().unwrap();
                            cx.notify();
                        })),
                ),
            )
            .child(
                row().child("配色").child(
                    YueSegmented::new("tag-tone", theme, format!("{:?}", self.tone))
                        .label("标签配色")
                        .options(
                            TONES
                                .map(|(label, v)| YueSegmentedOption::new(format!("{v:?}"), label)),
                        )
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.tone = TONES
                                .iter()
                                .find(|(_, v)| format!("{v:?}") == value.as_ref())
                                .unwrap()
                                .1;
                            cx.notify();
                        })),
                ),
            )
            .child(
                row().child("外观").child(
                    YueSegmented::new("tag-variant", theme, format!("{:?}", self.variant))
                        .label("标签外观")
                        .options(
                            VARIANTS
                                .map(|(label, v)| YueSegmentedOption::new(format!("{v:?}"), label)),
                        )
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.variant = VARIANTS
                                .iter()
                                .find(|(_, v)| format!("{v:?}") == value.as_ref())
                                .unwrap()
                                .1;
                            cx.notify();
                        })),
                ),
            )
            .child(
                row().child("尺寸").child(
                    YueSegmented::new("tag-size", theme, format!("{:?}", self.size))
                        .label("标签尺寸")
                        .options([
                            YueSegmentedOption::new("Small", "小"),
                            YueSegmentedOption::new("Medium", "中"),
                            YueSegmentedOption::new("Large", "大"),
                        ])
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.size = match value.as_ref() {
                                "Small" => TagSize::Small,
                                "Large" => TagSize::Large,
                                _ => TagSize::Medium,
                            };
                            cx.notify();
                        })),
                ),
            )
            .child(
                row().child("形状").child(
                    YueSegmented::new("tag-shape", theme, format!("{:?}", self.shape))
                        .label("标签形状")
                        .options([
                            YueSegmentedOption::new("Rounded", "圆角"),
                            YueSegmentedOption::new("Pill", "圆头"),
                            YueSegmentedOption::new("Mark", "标记"),
                        ])
                        .on_change(cx.listener(|p, value: &SharedString, _, cx| {
                            p.shape = match value.as_ref() {
                                "Pill" => TagShape::Pill,
                                "Mark" => TagShape::Mark,
                                _ => TagShape::Rounded,
                            };
                            cx.notify();
                        })),
                ),
            );
        let preview_area = row()
            .min_h(gpui_kit::rems(6.))
            .gap_6()
            .when(self.preview_visible, |d| d.child(preview))
            .when(!self.preview_visible, |d| {
                d.child(
                    YueButton::new("tag-reset", "恢复预览")
                        .variant(ButtonVariant::Outline)
                        .on_click(cx.listener(|p, _, _, cx| {
                            p.preview_visible = true;
                            cx.notify();
                        })),
                )
            });

        let mut matrix = column();
        for (name, variant) in VARIANTS {
            matrix = matrix.child(
                row()
                    .child(div().w(gpui_kit::rems(6.)).text_sm().child(name))
                    .child(
                        YueTagGroup::new(format!("tag-matrix-{variant:?}")).children(TONES.map(
                            |(label, tone)| {
                                YueTag::new(
                                    format!("tag-matrix-{variant:?}-{tone:?}"),
                                    format!("{label}标签"),
                                )
                                .tone(tone)
                                .variant(variant)
                            },
                        )),
                    ),
            );
        }
        let mut check_rows = column();
        for (row_ix, (label, variant)) in VARIANTS[..3].iter().copied().enumerate() {
            let mut group = YueTagGroup::new(format!("tag-checks-{row_ix}"));
            for ix in 0..3 {
                let page = cx.entity();
                group = group.child(
                    YueTag::checkable(
                        format!("tag-check-{row_ix}-{ix}"),
                        format!("选项 {}", ix + 1),
                        self.checks[row_ix][ix],
                    )
                    .variant(variant)
                    .disabled(ix == 2)
                    .on_change(move |next, _, _, cx| {
                        page.update(cx, |p, cx| {
                            p.checks[row_ix][ix] = next;
                            cx.notify();
                        })
                    }),
                );
            }
            check_rows = check_rows.child(
                row()
                    .child(div().w(gpui_kit::rems(6.)).child(label))
                    .child(group),
            );
        }
        let paths = YueTagGroup::new("tag-paths").children(self.paths.iter().map(|path| {
            let id = path.id;
            let label = path.label.clone();
            YueTag::action(format!("tag-path-{id}"), label.clone())
                .leading_icon(outline::Folder)
                .track_focus(&path.focus)
                .track_close_focus(&path.close)
                .on_click(cx.listener(move |p, _, _, cx| {
                    p.last_path = label.clone();
                    p.actions += 1;
                    cx.notify();
                }))
                .on_close(cx.listener(move |p, _, window, cx| p.remove_path(id, window, cx)))
        }));
        let add = column()
            .key_context("TagEditor")
            .on_action(cx.listener(Self::cancel_add))
            .child(paths)
            .child(if self.adding {
                column()
                    .child(
                        row()
                            .child(
                                YueField::new(theme)
                                    .label("新标签名称")
                                    .when_some(self.error.clone(), |f, e| {
                                        f.message(InputStatus::Error, e)
                                    })
                                    .input(
                                        YueInput::new("tag-add-input", theme, &self.input)
                                            .width(gpui_kit::rems(18.)),
                                    ),
                            )
                            .child(
                                YueButton::new("tag-add-submit", "添加")
                                    .on_click(cx.listener(|p, _, w, cx| p.submit(w, cx))),
                            )
                            .child(
                                YueButton::new("tag-add-cancel", "取消")
                                    .variant(ButtonVariant::Text)
                                    .on_click(cx.listener(|p, _, w, cx| {
                                        p.cancel_add(&CancelAddTag, w, cx)
                                    })),
                            ),
                    )
                    .into_any_element()
            } else {
                YueButton::icon_text("tag-add", theme, outline::Plus, "添加标签")
                    .variant(ButtonVariant::Outline)
                    .track_focus(&self.add_focus)
                    .on_click(cx.listener(|p, _, w, cx| p.begin_add(w, cx)))
                    .into_any_element()
            });

        let mut single = YueTagGroup::new("tag-single");
        for (ix, label) in ["全部", "图片", "视频", "文档"].into_iter().enumerate() {
            let page = cx.entity();
            single = single.child(
                YueTag::checkable(format!("tag-single-{ix}"), label, self.single == Some(ix))
                    .on_change(move |next, _, _, cx| {
                        page.update(cx, |p, cx| {
                            p.single = next.then_some(ix);
                            cx.notify();
                        })
                    }),
            );
        }
        let mut presets = YueTagGroup::new("tag-presets");
        for value in [100, 75, 50, 25] {
            presets = presets.child(
                YueTag::action(format!("tag-scale-{value}"), format!("{value}%")).on_click(
                    cx.listener(move |p, _, _, cx| {
                        p.scale = value;
                        p.actions += 1;
                        cx.notify();
                    }),
                ),
            );
        }
        let mut flags = YueTagGroup::new("tag-save-flags");
        for (id, label, value) in [
            ("copy", "复制路径", self.copy_path),
            ("open", "打开文件夹", self.open_folder),
        ] {
            let page = cx.entity();
            flags = flags.child(
                YueTag::checkable(format!("tag-{id}"), label, value).on_change(
                    move |next, _, _, cx| {
                        page.update(cx, |p, cx| {
                            if id == "copy" {
                                p.copy_path = next;
                            } else {
                                p.open_folder = next;
                            }
                            cx.notify();
                        })
                    },
                ),
            );
        }
        let sizes = row().children(
            [
                (TagSize::Small, "小标签"),
                (TagSize::Medium, "中标签"),
                (TagSize::Large, "大标签"),
            ]
            .map(|(size, label)| {
                YueTag::new(format!("tag-size-{size:?}"), label)
                    .size(size)
                    .leading_icon(outline::Tag)
            }),
        );
        let shapes = row().children([TagShape::Rounded, TagShape::Pill, TagShape::Mark].map(
            |shape| {
                YueTag::new(format!("tag-shape-{shape:?}"), format!("{shape:?}"))
                    .tone(TagTone::Primary)
                    .shape(shape)
            },
        ));
        let custom = TagColors::new(
            gpui_kit::rgb(theme.surface()),
            gpui_kit::rgb(theme.text_primary()),
            gpui_kit::rgb(theme.accent_solid()),
        );
        column().debug_selector(|| "tag-content".into()).p_8().gap_6()
            .child(div().text_2xl().child("Tag 标签"))
            .child(readout(theme, "短文本、轻量动作和可选条件共用 Tag 家族；关闭操作独立。".into()))
            .child(section(theme, "交互预览", "切换模式和外观；关闭后可恢复预览。", column().child(preview_area).child(settings)))
            .child(readout(theme, format!("主体动作 {} 次 · 关闭 {} 次 · 当前路径：{}", self.actions, self.closes, self.last_path)))
            .child(section(theme, "基础标签", "五种语义色 × 四种外观；静态标签不加入 Tab 顺序。", matrix))
            .child(section(theme, "图标与禁用", "装饰图标不增加操作区，禁用后不能切换或关闭。", YueTagGroup::new("tag-icon-examples").children([
                YueTag::new("tag-icon-plain", "带图标标签").leading_icon(outline::Tag),
                YueTag::action("tag-icon-action", "打开素材目录").leading_icon(outline::Folder).on_click(cx.listener(|p, _, _, cx| { p.actions += 1; cx.notify(); })),
                YueTag::checkable("tag-disabled-checked", "已选中且禁用", true).disabled(true).on_change(|_, _, _, _| {}).on_close(|_, _, _| {}),
            ])))
            .child(section(theme, "可关闭与添加", "关闭不切换路径；Enter 添加，Escape 取消；空白与重复名称会提示。", add))
            .child(section(theme, "可选择标签", "未选中时无图标占位、两侧留白对称；选中时展开勾选图标，宽度平滑过渡。", check_rows))
            .child(section(theme, "单个生效的筛选", "宿主控制一个筛选值，再次点击可取消；必选表单项使用 Radio。", single))
            .child(section(theme, "保存面板组合", "尺寸预设是动作，两个保存选项独立切换。", column().child(presets).child(flags).child(readout(theme, format!("尺寸 {}% · 复制路径 {} · 打开文件夹 {}", self.scale, self.copy_path, self.open_folder)))))
            .child(section(theme, "长文本省略", "只截断文字，关闭区固定；实际溢出才显示完整提示，本例关闭请求只计数。", row().child(
                YueTag::action("tag-overflow", "默认截图保存目录：图片与工作项目素材").max_width(gpui_kit::rems(13.)).leading_icon(outline::Folder)
                    .on_click(cx.listener(|p, _, _, cx| { p.actions += 1; cx.notify(); }))
                    .on_close(cx.listener(|p, _, _, cx| { p.closes += 1; cx.notify(); })),
            )))
            .child(section(theme, "尺寸与形状", "20 / 24 / 30px 紧凑阶梯随 rem 缩放；轮廓不改变交互。", column().child(sizes).child(shapes)))
            .child(section(theme, "显式自定义三色", "背景、文字和边框必须同时提供，使用当前主题保持可读性。", YueTag::new("tag-custom", "自定义边界色").colors(custom)))
            .child(readout(theme, "动效：勾选时先展开，再逐笔画勾；取消时先擦除，再收起。hover、按下、选中色连续过渡；支持 reduced motion。V1 增删不播放退出或重排动画。".into()))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests;
