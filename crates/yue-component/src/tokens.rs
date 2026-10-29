//! Reading Yue tokens as the GPUI values a component styles with.
//!
//! A control asks for a token by its CSS name and gets back a length, colour,
//! duration or easing curve. The parsing lives here so that every component
//! resolves `calc()` sizes, `color-mix()` halos and `cubic-bezier()` curves from
//! the same code path and a retuned token behaves the same wherever it is read.

use std::time::Duration;

use crate::{YueTheme, YueToken};

pub(crate) fn token(theme: YueTheme, name: &'static str) -> YueToken {
    theme
        .token(name)
        .unwrap_or_else(|| panic!("missing Yue token {name}"))
}

/// A token already scaled to pixels: a plain `12px` or a `calc()` of one.
pub(crate) fn pixels(theme: YueTheme, name: &'static str) -> f32 {
    token(theme, name)
        .pixels()
        .unwrap_or_else(|| panic!("Yue token {name} is not a pixel length"))
}

pub(crate) fn rems(theme: YueTheme, name: &'static str) -> gpui_kit::Rems {
    gpui_kit::rems(pixels(theme, name) / 16.0)
}

pub(crate) fn rgba(theme: YueTheme, name: &'static str) -> gpui_kit::Rgba {
    token(theme, name)
        .rgba()
        .unwrap_or_else(|| panic!("Yue token {name} is not a supported color"))
}

pub(crate) fn number(theme: YueTheme, name: &'static str) -> f32 {
    token(theme, name)
        .number()
        .unwrap_or_else(|| panic!("Yue token {name} is not a number"))
}

/// A millisecond duration, including the `calc(200ms*0.5)` form a retuned
/// motion ladder can produce.
pub(crate) fn duration(theme: YueTheme, name: &'static str) -> Duration {
    let value = token(theme, name).value().trim();
    let value = value
        .strip_prefix("calc(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(value);
    if let Some((left, right)) = value.split_once('*') {
        let millis = left
            .trim()
            .strip_suffix("ms")
            .and_then(|value| value.trim().parse::<f64>().ok());
        let factor = right.trim().parse::<f64>().ok();
        if let (Some(millis), Some(factor)) = (millis, factor) {
            return Duration::from_secs_f64(millis * factor / 1000.0);
        }
    }
    let millis = value
        .strip_suffix("ms")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or_else(|| panic!("Yue token {name} is not a millisecond duration"));
    Duration::from_secs_f64(millis / 1000.0)
}

/// A `cubic-bezier()` curve as the progress remapper a transition wants.
///
/// GPUI drives a transition by time, so the curve is read the way a browser
/// reads it: solve for the parameter whose x equals the elapsed fraction, then
/// report that parameter's y.
pub(crate) fn easing(theme: YueTheme, name: &'static str) -> Box<dyn Fn(f32) -> f32> {
    let (x1, y1, x2, y2) = token(theme, name)
        .value()
        .strip_prefix("cubic-bezier(")
        .and_then(|value| value.strip_suffix(')'))
        .and_then(|value| {
            let values: Vec<_> = value
                .split(',')
                .filter_map(|part| part.trim().parse::<f32>().ok())
                .collect();
            (values.len() == 4).then_some((values[0], values[1], values[2], values[3]))
        })
        .unwrap_or_else(|| panic!("Yue token {name} is not a cubic-bezier easing"));
    Box::new(move |time| {
        let sample_x = |t: f32| {
            let u = 1.0 - t;
            3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t
        };
        let sample_y = |t: f32| {
            let u = 1.0 - t;
            3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t
        };
        let derivative = |t: f32| {
            3.0 * (1.0 - t) * (1.0 - t) * x1
                + 6.0 * (1.0 - t) * t * (x2 - x1)
                + 3.0 * t * t * (1.0 - x2)
        };
        let mut t = time;
        for _ in 0..8 {
            let slope = derivative(t);
            if slope.abs() < 1e-6 {
                break;
            }
            t = (t - (sample_x(t) - time) / slope).clamp(0.0, 1.0);
        }
        sample_y(t)
    })
}

/// Mix two token colours along the RGB axes a CSS transition travels.
///
/// Blending the resolved colours keeps a state change on the same path as the
/// browser's, instead of swinging through the hues between two colours.
pub(crate) fn mix_rgba(from: gpui_kit::Rgba, to: gpui_kit::Rgba, progress: f32) -> gpui_kit::Rgba {
    let channel = |from: f32, to: f32| from + (to - from) * progress;
    gpui_kit::Rgba {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
        a: channel(from.a, to.a),
    }
}
