use super::*;
use gpui_kit::{
    InteractiveElement as _, Length, Role, StatefulInteractiveElement as _, TextRun, canvas,
};

/// A text slot which explains its full text only when truncated.
#[derive(IntoElement)]
pub struct YueOverflowTooltip {
    id: ElementId,
    text: SharedString,
    theme: YueTheme,
    width: Option<Length>,
    max_width: Option<Length>,
    focus: Option<FocusHandle>,
    lines: usize,
    keyboard: bool,
}
impl YueOverflowTooltip {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            theme,
            width: None,
            max_width: None,
            focus: None,
            lines: 1,
            keyboard: false,
        }
    }
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }
    pub fn max_width(mut self, width: impl Into<Length>) -> Self {
        self.max_width = Some(width.into());
        self
    }
    /// Maximum visible lines; only actually truncated text offers a tooltip.
    pub fn lines(mut self, lines: usize) -> Self {
        assert!(lines > 0, "OverflowTooltip lines must be positive");
        self.lines = lines;
        self
    }
    /// Opt into a conditional keyboard stop. Existing consumers stay unchanged.
    pub fn focusable_when_overflow(mut self, enabled: bool) -> Self {
        self.keyboard = enabled;
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
}
impl RenderOnce for YueOverflowTooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use gpui_kit::prelude::FluentBuilder as _;
        let overflow = window
            .use_keyed_state((self.id.clone(), "overflow"), cx, |_, _| {
                Rc::new(Cell::new(false))
            })
            .read(cx)
            .clone();
        let style = window.text_style();
        let size = style.font_size.to_pixels(window.rem_size());
        let runs = vec![TextRun {
            len: self.text.len(),
            font: style.font(),
            color: style.color,
            ..Default::default()
        }];
        let text = self.text.clone();
        let lines = self.lines;
        // A one-line slot cannot retain physical line breaks. The payload and
        // accessible name remain the original text, including those breaks.
        let display_text: SharedString = if lines == 1 {
            self.text.replace(['\r', '\n'], " ").into()
        } else {
            self.text.clone()
        };
        let focus = self.focus.or_else(|| {
            self.keyboard.then(|| {
                window
                    .use_keyed_state(
                        (self.id.clone(), "overflow-focus"),
                        cx,
                        |w, cx: &mut gpui_kit::Context<FocusHandle>| {
                            let handle = w.window_handle();
                            cx.on_release(move |focus, cx| {
                                let focus = focus.clone();
                                cx.defer(move |cx| {
                                    let _ = handle.update(cx, |_, w, cx| {
                                        if focus.is_focused(w) {
                                            w.focus_next(cx);
                                        }
                                    });
                                });
                            })
                            .detach();
                            cx.focus_handle()
                        },
                    )
                    .read(cx)
                    .clone()
            })
        });
        let owned_focus = if self.keyboard {
            focus.clone().map(|f| f.tab_stop(overflow.get()))
        } else {
            None
        };
        let repair = owned_focus.clone();
        let observer = overflow.clone();
        let capture = canvas(
            move |bounds, window, cx| {
                let width = bounds.size.width.max(gpui_kit::px(0.));
                let next = if width <= gpui_kit::px(0.) {
                    false
                } else if lines == 1 {
                    text.contains('\n')
                        || window
                            .text_system()
                            .shape_line(text.clone(), size, &runs, None)
                            .width()
                            > width
                } else {
                    window
                        .text_system()
                        .shape_text(text.clone(), size, &runs, Some(width), None)
                        .is_ok_and(|shaped| {
                            shaped
                                .iter()
                                .map(|l| l.wrap_boundaries().len() + 1)
                                .sum::<usize>()
                                > lines
                        })
                };
                if observer.replace(next) != next {
                    let observer = observer.clone();
                    let repair = repair.clone();
                    let handle = window.window_handle();
                    cx.defer(move |cx| {
                        let _ = handle.update(cx, |_, window, cx| {
                            if let Some(focus) = repair {
                                let _ = focus.clone().tab_stop(next);
                                if !next && focus.is_focused(window) {
                                    window.focus_next(cx);
                                }
                            }
                            super::controller::availability_changed(&observer, window, cx);
                            window.refresh();
                        });
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();
        let mut config = Config::new(self.text.clone(), self.theme, TooltipOptions::default());
        config.availability = Some(overflow);
        let attachment =
            super::controller::attach(self.id.clone(), config, focus.clone(), window, cx);
        use gpui_kit::base::TestSupportExt as _;
        div()
            .id(self.id)
            .test_support()
            .relative()
            .min_w_0()
            .when_some(self.width, |d, w| d.w(w))
            .when_some(self.max_width, |d, w| d.max_w(w))
            .when(self.lines == 1, |d| d.truncate())
            .when(self.lines > 1, |d| d.text_ellipsis().line_clamp(self.lines))
            .when(self.keyboard, |d| {
                d.role(Role::Label).aria_label(self.text.clone())
            })
            .when_some(owned_focus, |d, f| {
                d.track_focus(&f).focus_visible(|d| {
                    d.shadow(vec![gpui_kit::BoxShadow {
                        inset: false,
                        color: crate::tokens::rgba(self.theme, "--button-focus-ring-color").into(),
                        offset: gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(0.)),
                        blur_radius: gpui_kit::px(0.),
                        spread_radius: crate::tokens::rems(self.theme, "--button-focus-ring-width")
                            .to_pixels(window.rem_size()),
                    }])
                })
            })
            .child(display_text)
            .child(capture)
            .child(attachment)
    }
}
