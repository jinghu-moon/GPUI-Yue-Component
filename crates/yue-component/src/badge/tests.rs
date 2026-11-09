use super::*;

#[test]
fn formatting_visibility_and_replacing_content_preserve_real_counts() {
    assert!(!YueBadge::new("zero").is_visible());
    assert!(YueBadge::new("zero").show_zero(true).is_visible());
    for (count, max, expected) in [
        (9, 9, "9"),
        (10, 9, "9+"),
        (99, 99, "99"),
        (100, 99, "99+"),
        (u64::MAX, u64::MAX, "18446744073709551615"),
    ] {
        let badge = YueBadge::new("count").count(count).max_count(max);
        assert_eq!(badge.display_text().as_deref(), Some(expected));
        assert_eq!(badge.accessible_name().as_ref(), count.to_string());
    }
    let badge = YueBadge::new("mode").count(8).dot().text("  新\n内容\t ");
    assert_eq!(badge.display_text().as_deref(), Some("新 内容"));
    assert_eq!(badge.accessible_name().as_ref(), "新 内容");
    assert!(YueBadge::new("dot").count(0).dot().is_visible());
    assert!(!YueBadge::new("dot").dot().visible(false).is_visible());
    assert_eq!(
        YueBadge::new("dot")
            .dot()
            .label("已同步")
            .accessible_name()
            .as_ref(),
        "已同步"
    );
    assert_eq!(
        YueBadge::new("count")
            .count(128)
            .label("通知")
            .accessible_name()
            .as_ref(),
        "通知，128"
    );
}

#[test]
#[should_panic(expected = "positive")]
fn zero_max_is_rejected() {
    let _ = YueBadge::new("invalid").max_count(0);
}

#[test]
#[should_panic(expected = "nonblank")]
fn blank_text_is_rejected() {
    let _ = YueBadge::new("invalid").text("\n\t");
}

#[test]
#[should_panic(expected = "finite")]
fn nonfinite_offsets_are_rejected() {
    let _ = YueBadge::new("invalid").offset(point(rems(f32::NAN), rems(0.)));
}

#[test]
#[should_panic(expected = "finite")]
fn invalid_color_channels_are_rejected() {
    let _ = YueBadge::new("invalid").colors(
        Rgba {
            r: -1.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        gpui_kit::rgb(0xffffff),
    );
}

#[test]
fn badge_text_contrast_meets_45_across_all_theme_profiles() {
    use crate::{Accent, NeutralTone, ThemeMode};
    let luminance = |c: Rgba| {
        let linear = |v: f32| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    };
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            for neutral in NeutralTone::ALL {
                let theme = YueTheme::new(mode, accent).with_neutral(neutral);
                for tone in [
                    BadgeTone::Default,
                    BadgeTone::Primary,
                    BadgeTone::Success,
                    BadgeTone::Warning,
                    BadgeTone::Danger,
                ] {
                    let (bg, fg) = tone.tokens();
                    let a = luminance(crate::tokens::rgba(theme, bg));
                    let b = luminance(crate::tokens::rgba(theme, fg));
                    let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                    assert!(
                        ratio >= 4.5,
                        "{mode:?}/{accent:?}/{neutral:?}/{tone:?}: {ratio}"
                    );
                }
            }
        }
    }
}

#[cfg(feature = "headless-tests")]
mod visual;
