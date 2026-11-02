//! Host-side fixture decoding, revision handling and Dialog composition.
use super::super::*;
use gpui_kit::{AppContext as _, Entity, Image, ImageFormat, Subscription, Task};
use std::time::Duration;
use yue_component::{
    DialogFooter, DialogSize, ImagePreviewDocument, ImagePreviewEvent, ImagePreviewImage,
    ImagePreviewLoad, ImagePreviewMode, ImagePreviewToolbar, YueDialog, YueDialogState,
    YueImagePreview, YueImagePreviewState, YueSegmented, YueSegmentedOption,
};

const DESKTOP: &[u8] = include_bytes!("../../assets/image-preview/desktop.png");
const LONG: &[u8] = include_bytes!("../../assets/image-preview/long-screenshot.png");
const ALPHA: &[u8] = include_bytes!("../../assets/image-preview/transparent.png");
const JPEG: &[u8] = include_bytes!("../../assets/image-preview/desktop-q70.jpg");
const HALF: &[u8] = include_bytes!("../../assets/image-preview/desktop-half-q70.jpg");

pub(crate) struct ImagePreviewPage {
    preview: Entity<YueImagePreviewState>,
    large_preview: Entity<YueImagePreviewState>,
    dialog: Entity<YueDialogState>,
    choice: SharedString,
    source: Option<ImagePreviewImage>,
    result: Option<ImagePreviewImage>,
    generation: u64,
    revision: u64,
    animated: bool,
    disabled: bool,
    floating: bool,
    markers: bool,
    last: SharedString,
    task: Option<Task<()>>,
    _events: Subscription,
}
impl ImagePreviewPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let preview = cx.new(|cx| YueImagePreviewState::new("preview-demo", w, cx));
        let events = cx.subscribe_in(&preview, w, |p: &mut Self, _, e, w, cx| {
            if matches!(e, ImagePreviewEvent::RetryRequested { .. }) {
                p.load(p.choice.clone(), w, cx);
            } else {
                let s = p.preview.read(cx).snapshot();
                p.last = format!(
                    "{:?} · {:.1}% · split {:.0}%",
                    s.mode(),
                    s.zoom() * 100.,
                    s.split() * 100.
                )
                .into();
                cx.notify();
            }
        });
        let mut p = Self {
            preview,
            large_preview: cx.new(|cx| YueImagePreviewState::new("preview-dialog", w, cx)),
            dialog: cx.new(YueDialogState::new),
            choice: "desktop".into(),
            source: None,
            result: None,
            generation: 0,
            revision: 0,
            animated: true,
            disabled: false,
            floating: true,
            markers: false,
            last: "正在加载测试图片".into(),
            task: None,
            _events: events,
        };
        p.load("desktop".into(), w, cx);
        p
    }
    fn document(&self, result: ImagePreviewLoad) -> ImagePreviewDocument {
        ImagePreviewDocument::new(self.choice.clone(), self.revision)
            .with_source(
                self.source
                    .clone()
                    .map_or(ImagePreviewLoad::Loading, ImagePreviewLoad::Ready),
            )
            .with_result(result)
            .full_frame(self.choice == "desktop" || self.choice == "half")
    }
    fn load(&mut self, choice: SharedString, w: &mut Window, cx: &mut Context<Self>) {
        self.task = None;
        self.generation += 1;
        self.revision += 1;
        self.choice = choice.clone();
        self.source = None;
        self.result = None;
        let doc = self.document(ImagePreviewLoad::Loading);
        let _ = self.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
        let svg = cx.svg_renderer();
        let generation = self.generation;
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            let images = cx
                .background_spawn(async move {
                    let bytes = match choice.as_ref() {
                        "long" => LONG,
                        "alpha" => ALPHA,
                        _ => DESKTOP,
                    };
                    let source = Image::from_bytes(ImageFormat::Png, bytes.to_vec())
                        .to_image_data(svg.clone())
                        .map_err(|e| e.to_string())?;
                    let result = match choice.as_ref() {
                        "desktop" | "half" => {
                            let b = if choice == "half" { HALF } else { JPEG };
                            Some(
                                Image::from_bytes(ImageFormat::Jpeg, b.to_vec())
                                    .to_image_data(svg)
                                    .map_err(|e| e.to_string())?,
                            )
                        }
                        _ => None,
                    };
                    Ok::<_, String>((source, result))
                })
                .await;
            let _ = weak.update_in(cx, |p, w, cx| {
                if p.generation != generation {
                    return;
                }
                match images {
                    Ok((source, result)) => {
                        p.source = Some(
                            ImagePreviewImage::new(source, "用户提供的测试截图")
                                .expect("valid fixture"),
                        );
                        p.result = result.map(|i| {
                            ImagePreviewImage::new(i, "真实JPEG编码结果，quality=70")
                                .expect("valid fixture")
                        });
                        let doc = p.document(
                            p.result
                                .clone()
                                .map_or(ImagePreviewLoad::Empty, ImagePreviewLoad::Ready),
                        );
                        let compare = p.result.is_some();
                        p.preview.update(cx, |s, cx| {
                            let _ = s.set_document(doc, w, cx);
                            if compare {
                                let _ = s.set_mode(ImagePreviewMode::Compare, w, cx);
                            }
                        });
                        p.last = "图片已就绪 · JPEG为真实编码样本".into();
                    }
                    Err(error) => {
                        let doc = ImagePreviewDocument::new(p.choice.clone(), p.revision)
                            .with_source(ImagePreviewLoad::Failed(error.into()));
                        let _ = p.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn refresh(&mut self, fail: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.result.is_none() {
            return;
        }
        self.task = None;
        self.generation += 1;
        self.revision += 1;
        let generation = self.generation;
        let doc = self.document(ImagePreviewLoad::Loading);
        let _ = self.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(900))
                .await;
            let _ = weak.update_in(cx, |p, w, cx| {
                if p.generation != generation {
                    return;
                }
                let result = if fail {
                    ImagePreviewLoad::Failed("演示失败，可重试".into())
                } else {
                    ImagePreviewLoad::Ready(p.result.clone().expect("ready result"))
                };
                let doc = p.document(result);
                let _ = p.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn open_large(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        let doc = ImagePreviewDocument::new(self.choice.clone(), self.revision)
            .with_source(ImagePreviewLoad::Ready(source))
            .with_result(
                self.result
                    .clone()
                    .map_or(ImagePreviewLoad::Empty, ImagePreviewLoad::Ready),
            )
            .full_frame(true);
        let mode = self.preview.read(cx).snapshot().mode();
        self.large_preview.update(cx, |s, cx| {
            let _ = s.set_document(doc, w, cx);
            let _ = s.set_mode(mode, w, cx);
        });
        let _ = self.dialog.update(cx, |s, cx| s.open(w, cx));
    }
}
impl Render for ImagePreviewPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let row = || div().flex().items_center().flex_wrap().gap_3();
        let labeled = |label: &'static str, switch: YueSwitch| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(label)
                .child(switch)
        };
        let owner = cx.entity();
        let animation = cx.entity();
        let disable = cx.entity();
        let toolbar = cx.entity();
        let marker = cx.entity();
        let controls = row()
            .child(
                YueSegmented::new("preview-fixtures", theme, self.choice.clone())
                    .label("测试图片")
                    .options([
                        YueSegmentedOption::new("desktop", "截图 + JPEG"),
                        YueSegmentedOption::new("half", "半尺寸结果"),
                        YueSegmentedOption::new("long", "长截图"),
                        YueSegmentedOption::new("alpha", "透明图片"),
                    ])
                    .on_change(move |v, w, cx| owner.update(cx, |p, cx| p.load(v.clone(), w, cx))),
            )
            .child(labeled(
                "动效",
                YueSwitch::new("preview-animated", theme, self.animated, "动效").on_change(
                    move |v, _, _, cx| {
                        animation.update(cx, |p, cx| {
                            p.animated = v;
                            cx.notify();
                        })
                    },
                ),
            ))
            .child(labeled(
                "禁用预览交互",
                YueSwitch::new("preview-disabled", theme, self.disabled, "禁用预览交互").on_change(
                    move |v, _, _, cx| {
                        disable.update(cx, |p, cx| {
                            p.disabled = v;
                            cx.notify();
                        })
                    },
                ),
            ))
            .child(labeled(
                "浮动工具条",
                YueSwitch::new("preview-floating", theme, self.floating, "浮动工具条").on_change(
                    move |v, _, _, cx| {
                        toolbar.update(cx, |p, cx| {
                            p.floating = v;
                            cx.notify();
                        })
                    },
                ),
            ))
            .child(labeled(
                "长图标记（25% / 50% / 75%）",
                YueSwitch::new("preview-markers", theme, self.markers, "长图标记").on_change(
                    move |v, _, _, cx| {
                        marker.update(cx, |p, cx| {
                            p.markers = v;
                            cx.notify();
                        })
                    },
                ),
            ));
        let state = self.preview.clone();
        let actual = self.preview.clone();
        let commands = row()
            .child(
                YueButton::new("preview-width", "适应宽度")
                    .on_click(move |_, w, cx| state.update(cx, |s, cx| s.fit_width(w, cx))),
            )
            .child(
                YueButton::new("preview-native", "100%").on_click(move |_, w, cx| {
                    let _ = actual.update(cx, |s, cx| s.actual_size(w, cx));
                }),
            )
            .child(
                YueButton::new("preview-refresh", "模拟更新")
                    .disabled(self.result.is_none())
                    .on_click(cx.listener(|p, _, w, cx| p.refresh(false, w, cx))),
            )
            .child(
                YueButton::new("preview-failure", "模拟失败")
                    .disabled(self.result.is_none())
                    .on_click(cx.listener(|p, _, w, cx| p.refresh(true, w, cx))),
            )
            .child(
                YueButton::new("preview-large", "大图查看…")
                    .disabled(self.source.is_none())
                    .on_click(cx.listener(|p, _, w, cx| p.open_large(w, cx))),
            );
        let mut preview = YueImagePreview::new(&self.preview)
            .animated(self.animated)
            .disabled(self.disabled)
            .toolbar(if self.floating {
                ImagePreviewToolbar::Floating
            } else {
                ImagePreviewToolbar::Footer
            })
            .result_label("编码后");
        if self.markers {
            preview = preview.image_overlay(move |overlay, w, _| {
                for v in [0.25, 0.5, 0.75] {
                    let p = overlay.point(0., v);
                    let q = overlay.point(1., v);
                    w.paint_quad(gpui_kit::fill(
                        gpui_kit::Bounds::new(
                            p,
                            gpui_kit::size(q.x - p.x, gpui_kit::px(1. / w.scale_factor())),
                        ),
                        gpui_kit::rgb(theme.accent_solid()),
                    ));
                }
            });
        }
        div().size_full().min_h_0().min_w_0().flex().flex_col().p_6().gap_4()
            .child(div().text_xl().child("图片预览 ImagePreview"))
            .child("滚轮围绕指针缩放，左键拖动平移；分割握把可拖动或用左右键调整。预览获得焦点后：0适应、1原始比例、+ / −缩放。")
            .child(controls).child(commands)
            .child(div().flex_1().min_h(gpui_kit::rems(20.)).min_w_0().child(preview))
            .child(div().text_sm().text_color(gpui_kit::rgb(theme.text_secondary())).child(self.last.clone()))
            .child(YueDialog::new("preview-dialog-shell",theme,&self.dialog).title("图片预览").size(DialogSize::ExtraLarge)
                .width(gpui_kit::rems(58.)).footer(DialogFooter::None)
                .child(div().h(gpui_kit::rems(32.)).child(YueImagePreview::new(&self.large_preview).animated(self.animated))))
    }
}
