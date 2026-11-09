//! Host-side fixture decoding, revision handling and Dialog composition.
mod gallery;
mod timing;
use super::super::*;
use gpui_kit::{AppContext as _, Entity, Subscription, Task};
use std::time::Duration;
use yue_component::{
    DialogFooter, DialogSize, ImagePreviewDocument, ImagePreviewEvent, ImagePreviewImage,
    ImagePreviewLimits, ImagePreviewLoad, ImagePreviewToolbar, ImagePreviewView, YueDialog,
    YueDialogState, YueImagePreview, YueImagePreviewState, YueSegmented, YueSegmentedOption,
};

const DESKTOP: &[u8] = include_bytes!("../../assets/image-preview/desktop.png");
const LONG: &[u8] = include_bytes!("../../assets/image-preview/long-screenshot.png");
const LONG_PREVIEW: &[u8] = include_bytes!("../../assets/image-preview/long-preview.jpg");
const ALPHA: &[u8] = include_bytes!("../../assets/image-preview/transparent.png");
const JPEG: &[u8] = include_bytes!("../../assets/image-preview/desktop-q70.jpg");
const HALF: &[u8] = include_bytes!("../../assets/image-preview/desktop-half-q70.jpg");

pub(crate) struct ImagePreviewPage {
    preview: Entity<YueImagePreviewState>,
    large_preview: Entity<YueImagePreviewState>,
    dialog: Entity<YueDialogState>,
    choice: SharedString,
    multiple: bool,
    source: Option<ImagePreviewImage>,
    result: Option<ImagePreviewImage>,
    generation: u64,
    revision: u64,
    animated: bool,
    disabled: bool,
    provide_result: bool,
    dark: bool,
    markers: bool,
    last: SharedString,
    task: Option<Task<()>>,
    gallery_session: Option<gallery::GallerySession>,
    gallery_tasks: Vec<Task<()>>,
    gallery_source_task: Option<Task<()>>,
    _events: Subscription,
}
impl ImagePreviewPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let preview = cx.new(|cx| YueImagePreviewState::new("preview-demo", w, cx));
        let events = cx.subscribe_in(&preview, w, |p: &mut Self, _, e, w, cx| {
            if matches!(e, ImagePreviewEvent::RetryRequested { .. }) {
                p.load(p.choice.clone(), w, cx);
            } else {
                if matches!(e, ImagePreviewEvent::GalleryChanged(..)) {
                    p.prioritize_gallery(w, cx);
                }
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
            multiple: false,
            source: None,
            result: None,
            generation: 0,
            revision: 0,
            animated: true,
            disabled: false,
            provide_result: false,
            dark: true,
            markers: false,
            last: "正在加载测试图片".into(),
            task: None,
            gallery_session: None,
            gallery_tasks: Vec::new(),
            gallery_source_task: None,
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
            .with_result(if self.provide_result {
                result
            } else {
                ImagePreviewLoad::Empty
            })
            .full_frame(self.choice == "desktop" || self.choice == "half")
    }
    fn load(&mut self, choice: SharedString, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(session) = &self.gallery_session {
            session.finish_profile();
        }
        self.task = None;
        self.gallery_tasks.clear();
        self.gallery_source_task = None;
        self.gallery_session = None;
        self.generation += 1;
        self.revision += 1;
        self.choice = choice.clone();
        self.source = None;
        self.result = None;
        let doc = self.document(ImagePreviewLoad::Loading);
        let _ = self.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
        let generation = self.generation;
        if self.multiple {
            self.load_directory(w, cx);
            return;
        }
        let profile = timing::Profile::new(generation);
        let pipeline = profile.clock();
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            if choice == "long" {
                let quick_profile = profile.clone();
                let queued = profile.clock();
                let (preview, publication) = cx
                    .background_spawn(async move {
                        quick_profile.record("quick_executor_wait", "long-preview", queued);
                        quick_profile.handoff(|| {
                            timing::prepare(
                                &quick_profile,
                                LONG_PREVIEW,
                                &ImagePreviewLimits::default(),
                                "长截图 · 快速预览",
                            )
                            .and_then(|image| image.with_source_size([3840, 4998]))
                            .map_err(|e| e.to_string())
                        })
                    })
                    .await;
                if let Ok(preview) = preview {
                    let _ = weak.update_in(cx, |p, w, cx| {
                        if p.generation != generation {
                            return;
                        }
                        p.source = Some(preview);
                        profile.record("quick_publish_wait", "long-preview", publication);
                        let apply = profile.clock();
                        let doc = p.document(ImagePreviewLoad::Loading);
                        let _ = p.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
                        p.last = "长截图快速预览已就绪，正在准备更高清资源".into();
                        cx.notify();
                        profile.record("quick_ui_apply", "long-preview", apply);
                        profile.record("single_preview_ready", "long-preview", pipeline);
                    });
                }
            }
            let worker_profile = profile.clone();
            let queued = profile.clock();
            let (images, publication) = cx
                .background_spawn(async move {
                    worker_profile.record("source_executor_wait", &choice, queued);
                    worker_profile.handoff(|| {
                        let bytes = match choice.as_ref() {
                            "long" => LONG,
                            "alpha" => ALPHA,
                            _ => DESKTOP,
                        };
                        let limits = if choice == "proxy" {
                            ImagePreviewLimits::default().texture_budget(240, 240 * 240 * 4)
                        } else if choice == "long" {
                            ImagePreviewLimits::default().texture_budget(2048, 16 * 1024 * 1024)
                        } else {
                            ImagePreviewLimits::default()
                        };
                        let source_name = match choice.as_ref() {
                            "long" => "PixPin_2026-11-02_18-55-46.png",
                            "alpha" => "透明图片示例.png",
                            "proxy" => "桌面截图 · 代理预览",
                            _ => "PixPin_2026-11-02_18-55-14.png",
                        };
                        let source = timing::prepare(&worker_profile, bytes, &limits, source_name)
                            .map_err(|e| e.to_string())?;
                        let result = match choice.as_ref() {
                            "desktop" | "half" => {
                                let b = if choice == "half" { HALF } else { JPEG };
                                Some(
                                    timing::prepare(
                                        &worker_profile,
                                        b,
                                        &limits,
                                        "真实JPEG编码结果，quality=70",
                                    )
                                    .map_err(|e| e.to_string())?,
                                )
                            }
                            _ => None,
                        };
                        Ok::<_, String>((source, result))
                    })
                })
                .await;
            let _ = weak.update_in(cx, |p, w, cx| {
                if p.generation != generation {
                    return;
                }
                profile.record("source_publish_wait", &p.choice, publication);
                let apply = profile.clock();
                match images {
                    Ok((source, result)) => {
                        p.source = Some(source);
                        p.result = result;
                        let doc = p.document(
                            p.result
                                .clone()
                                .map_or(ImagePreviewLoad::Empty, ImagePreviewLoad::Ready),
                        );
                        p.preview.update(cx, |s, cx| {
                            let _ = s.set_document(doc, w, cx);
                        });
                        p.last = if p.result.is_some() {
                            "图片已就绪 · JPEG为真实编码样本"
                        } else {
                            "图片已就绪"
                        }
                        .into();
                    }
                    Err(error) => {
                        let doc = ImagePreviewDocument::new(p.choice.clone(), p.revision)
                            .with_source(ImagePreviewLoad::Failed(error.into()));
                        let _ = p.preview.update(cx, |s, cx| s.set_document(doc, w, cx));
                    }
                }
                cx.notify();
                profile.record("source_ui_apply", &p.choice, apply);
                profile.record("single_load_total", &p.choice, pipeline);
                profile.summary("single_image_settled");
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
    fn open_large(&mut self, view: &ImagePreviewView, w: &mut Window, cx: &mut Context<Self>) {
        if self
            .large_preview
            .update(cx, |s, cx| s.restore_view(view, w, cx))
            .is_ok()
        {
            let _ = self.dialog.update(cx, |s, cx| s.open(w, cx));
        }
    }
}
impl Render for ImagePreviewPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_theme = yue_component::current_theme(cx);
        let theme = YueTheme::new(
            if self.dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            app_theme.accent(),
        )
        .with_neutral(app_theme.neutral());
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
        let multiple = cx.entity();
        let appearance = cx.entity();
        let controls = row()
            .child(
                YueSegmented::new("preview-fixtures", theme, self.choice.clone())
                    .label("测试图片")
                    .options([
                        YueSegmentedOption::new("desktop", "桌面截图"),
                        YueSegmentedOption::new("half", "半尺寸结果"),
                        YueSegmentedOption::new("long", "长截图"),
                        YueSegmentedOption::new("alpha", "透明图片"),
                        YueSegmentedOption::new("proxy", "代理预览"),
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
                "提供处理结果",
                YueSwitch::new("preview-result", theme, self.provide_result, "提供处理结果")
                    .on_change(move |v, _, w, cx| {
                        toolbar.update(cx, |p, cx| {
                            p.provide_result = v;
                            p.load(p.choice.clone(), w, cx);
                        })
                    }),
            ))
            .child(labeled(
                "传入多张图片",
                YueSwitch::new("preview-multiple", theme, self.multiple, "传入多张图片").on_change(
                    move |value, _, w, cx| {
                        multiple.update(cx, |p, cx| {
                            p.multiple = value;
                            p.load(p.choice.clone(), w, cx);
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
        let commands = row()
            .child(
                YueButton::new("preview-refresh", "模拟更新")
                    .theme(theme)
                    .disabled(self.result.is_none() || !self.provide_result)
                    .on_click(cx.listener(|p, _, w, cx| p.refresh(false, w, cx))),
            )
            .child(
                YueButton::new("preview-failure", "模拟失败")
                    .theme(theme)
                    .disabled(self.result.is_none() || !self.provide_result)
                    .on_click(cx.listener(|p, _, w, cx| p.refresh(true, w, cx))),
            );
        let mut preview = YueImagePreview::new(&self.preview)
            .theme(theme)
            .animated(self.animated)
            .disabled(self.disabled)
            .toolbar(ImagePreviewToolbar::Footer)
            .result_label("编码后")
            .on_expand(cx.listener(|p, view, w, cx| p.open_large(view, w, cx)));
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
            .bg(gpui_kit::rgb(theme.page())).text_color(gpui_kit::rgb(theme.text_primary()))
            .child(row().justify_between().child(div().text_xl().child("图片预览 ImagePreview"))
                .child(labeled("深色", YueSwitch::new("preview-dark", theme, self.dark, "深色预览").on_change(move |value, _, _, cx| { appearance.update(cx, |p, cx| { p.dark = value; cx.notify(); }); }))))
            .child(div().text_sm().text_color(gpui_kit::rgb(theme.text_secondary())).child("专注查看图片，需要时再展开对比。传入多张图片后可通过两侧按钮切换，移至画布底部查看胶片条。"))
            .child(controls).when(self.provide_result, |d| d.child(commands))
            .child(div().flex_1().min_h(gpui_kit::rems(20.)).min_w_0().child(preview))
            .child(YueDialog::new("preview-dialog-shell",theme,&self.dialog).title("图片预览").size(DialogSize::ExtraLarge)
                .width(gpui_kit::rems(58.)).footer(DialogFooter::None)
                .child(div().h(gpui_kit::rems(32.)).child(YueImagePreview::new(&self.large_preview).theme(theme).animated(self.animated))))
    }
}
