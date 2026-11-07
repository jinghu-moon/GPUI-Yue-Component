//! Intrinsic dimensions for hosts which measure their popup before mounting it.
use super::*;
use gpui_kit::{FontWeight, Pixels, Size, TextRun, px, size};

impl ShortcutSpec {
    pub(crate) fn measured_size(
        &self,
        theme: YueTheme,
        size_kind: KbdSize,
        variant: KbdVariant,
        separator: ShortcutSeparator,
        window: &Window,
    ) -> Size<Pixels> {
        let m = size_kind.metrics();
        let scale = f32::from(window.rem_size()) / 16.;
        let text = |label: &str, font_size: f32, weight: FontWeight| {
            let stack = crate::typography::ui_font(theme);
            let mut font = gpui_kit::font(stack.family.clone());
            font.fallbacks = stack.font_fallbacks();
            font.weight = weight;
            f32::from(
                window
                    .text_system()
                    .shape_line(
                        SharedString::from(label.to_owned()),
                        px(font_size * scale),
                        &[TextRun {
                            len: label.len(),
                            font,
                            color: window.text_style().color,
                            ..Default::default()
                        }],
                        None,
                    )
                    .width(),
            )
            .ceil()
        };
        let ghost = variant == KbdVariant::Ghost;
        let key_gap = if ghost { 1. } else { m.key_gap } * scale;
        let plus = text("+", m.font, FontWeight::NORMAL);
        let join_width = |label: &str| text(label, 12., FontWeight::NORMAL) + 16. * scale;
        let mut width = 0.;
        for (ai, alternative) in self.0.iter().enumerate() {
            if ai > 0 {
                width += join_width("或");
            }
            for (si, chord) in alternative.0.iter().enumerate() {
                if si > 0 {
                    width += join_width("然后");
                }
                for (ki, key) in chord.0.iter().enumerate() {
                    if ki > 0 {
                        width += if separator == ShortcutSeparator::Plus {
                            2. * key_gap + plus
                        } else {
                            key_gap
                        };
                    }
                    let label = key_label(key);
                    let content = text(&label, m.font, FontWeight::MEDIUM);
                    let natural = content + (2. + 2. * if ghost { 1. } else { m.padding }) * scale;
                    width += natural.max(if ghost { 0. } else { m.height * scale });
                }
            }
        }
        size(px(width), px(m.height * scale))
    }
}
