//! Private presentation shared by Select and TreeSelect; owns no selection or popup state.
use crate::{InputStatus, YueTheme};
use gpui_kit::{Pixels, SharedString, Styled as _, TextRun, Window, div, px};
pub(crate) fn metric(theme: YueTheme, name: &str, window: &Window) -> Pixels {
    px(crate::tokens::pixels(theme, name) * f32::from(window.rem_size()) / 16.)
}
pub(crate) fn measure(text: &SharedString, font: Pixels, window: &Window) -> Pixels {
    let style = window.text_style();
    window
        .text_system()
        .shape_line(
            text.clone(),
            font,
            &[TextRun {
                len: text.len(),
                font: style.font(),
                color: style.color,
                ..Default::default()
            }],
            None,
        )
        .width()
}
pub(crate) fn visible_tags(
    widths: &[Pixels],
    limit: usize,
    budget: Pixels,
    gap: Pixels,
    mut counter: impl FnMut(usize) -> Pixels,
) -> usize {
    let mut show = limit.min(widths.len());
    let mut used = widths[..show].iter().copied().fold(px(0.), |a, b| a + b)
        + gap * show.saturating_sub(1) as f32;
    while show > 0 {
        let hidden = widths.len() - show;
        let count = if hidden > 0 {
            counter(hidden) + gap
        } else {
            px(0.)
        };
        if used + count <= budget {
            break;
        }
        show -= 1;
        used = used - widths[show] - if show > 0 { gap } else { px(0.) };
    }
    show
}
pub(crate) fn frame_geometry(
    height: Pixels,
    padding: Pixels,
    gap: Pixels,
    font: Pixels,
) -> gpui_kit::StyleRefinement {
    let mut frame = div()
        .relative()
        .w_full()
        .min_h(height)
        .px(padding)
        .flex()
        .items_center()
        .gap(gap)
        .text_size(font);
    frame.style().clone()
}
pub(crate) fn surface(
    theme: YueTheme,
    status: InputStatus,
    disabled: bool,
    readonly: bool,
    bordered: bool,
    focused: bool,
) -> gpui_kit::StyleRefinement {
    crate::input::surface_style(theme, status, disabled, readonly, bordered, focused)
}
