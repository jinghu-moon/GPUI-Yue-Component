use std::rc::Rc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Animation, AnimationExt as _, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, PathBuilder, PathStyle, Pixels, Point, RenderOnce,
    SharedString, Styled as _, Window, canvas, div, px,
};
use lyon::tessellation::{LineCap, LineJoin, StrokeOptions};

use crate::tokens::{duration, easing, mix_rgba, number, pixels, rems, rgba};
use crate::{ThemeMode, YueTheme};

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// Side of the square grid the reference icons are drawn on: the demo's SVGs use
/// a 24×24 viewBox, so every glyph coordinate below is in those units.
const GLYPH_BOX: f32 = 24.0;

/// `stroke-width: 3` of the demo, in viewBox units. It shrinks with the glyph,
/// the way an element transform scales its whole rendering.
const GLYPH_STROKE: f32 = 3.0;

/// Half of a 90° sweep, expressed as a fraction of a full turn: the thumb icons
/// swing 45° as they cross-fade. The design system has no motion token for an
/// angle, so the spec's value lives here.
const INDICATOR_SWING_TURNS: f32 = 0.125;

/// The size an indicator icon shrinks to at the far end of its fade.
const INDICATOR_MIN_SCALE: f32 = 0.5;

/// The loading ring: `circle r=9` carrying `stroke-dasharray:28 100`, so the ink
/// spans 28 of the 9-unit radius's arc units.
const RING_RADIUS: f32 = 9.0;
const RING_DASH: f32 = 28.0;

/// The two diagonals of ×, in the order the demo inks them. Endpoints follow the
/// demo's own path data.
const CROSS_FIRST: [(f32, f32); 2] = [(18.0, 6.0), (6.0, 18.0)];
const CROSS_SECOND: [(f32, f32); 2] = [(6.0, 6.0), (18.0, 18.0)];

/// The single polyline of ✓: the demo's `M5 12l5 5l10 -10` grown 1.1× about its
/// own box, whose centre then moves back to the centre of the glyph box. The
/// demo sizes both glyphs by the same box, where × spans 12×12 but ✓ only 15×10,
/// so the cross-fade swapped a tall icon for a short one and the check read as
/// the smaller of the pair. 1.1 takes ✓ to 11 units tall — one short of ×, which
/// is the point at which it still reads as the same icon and not as a bulky one;
/// full parity would need 1.2. Re-derive the endpoints from the demo's path as
/// `x' = 12 + 1.1 * (x - 12.5)`, `y' = 12 + 1.1 * (y - 12)`. The stroke keeps
/// [`GLYPH_STROKE`]: only the geometry grew, not the weight.
const CHECK_LINE: [(f32, f32); 3] = [(3.75, 12.0), (9.25, 17.5), (20.25, 6.5)];

/// Three coordinated Yue Switch size profiles.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitchSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Track and thumb corner geometry.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitchShape {
    #[default]
    Capsule,
    RoundedRectangle,
}

/// Which icons the thumb shows.
///
/// Each icon fades, shrinks and swings on the switch's own duration and curve,
/// so an icon that should not be seen is the same icon at opacity 0 rather than a
/// mounted or dropped element.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitchIndicator {
    #[default]
    None,
    /// The check only: it swings in from the un-checked side as the switch turns
    /// on, and back out as it turns off.
    Check,
    /// The cross only: it swings out towards the checked side as the switch turns
    /// on.
    X,
    /// Both icons stacked in the thumb, cross-fading in opposite directions.
    CheckX,
}

/// A controlled Yue switch. The caller owns and updates the checked value.
#[derive(IntoElement)]
pub struct YueSwitch {
    id: ElementId,
    theme: YueTheme,
    checked: bool,
    disabled: bool,
    loading: bool,
    size: SwitchSize,
    shape: SwitchShape,
    indicator: SwitchIndicator,
    accessibility_label: SharedString,
    on_change: Option<ChangeHandler>,
}

impl YueSwitch {
    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        checked: bool,
        accessibility_label: impl Into<SharedString>,
    ) -> Self {
        let accessibility_label = accessibility_label.into();
        assert!(
            !accessibility_label.is_empty(),
            "YueSwitch requires an accessible name"
        );
        Self {
            id: id.into(),
            theme,
            checked,
            disabled: false,
            loading: false,
            size: SwitchSize::Medium,
            shape: SwitchShape::Capsule,
            indicator: SwitchIndicator::None,
            accessibility_label,
            on_change: None,
        }
    }

    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn shape(mut self, shape: SwitchShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn indicator(mut self, indicator: SwitchIndicator) -> Self {
        self.indicator = indicator;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Spin the thumb in place while an async commit is in flight.
    ///
    /// The thumb keeps its position and the indicators fade out behind the
    /// spinner. `on_change` is not reported until loading clears, so a slow
    /// commit cannot be toggled twice; pressing still stretches the thumb.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// How much wider the thumb goes while the pointer is held down: two track
/// insets on the trailing side, and nothing on a switch that cannot be pressed.
/// The demo swells its 20px thumb to 24px against a 2px inset, so the amount
/// scales with the inset rather than with a fixed pixel count.
fn press_stretch(pressed: bool, disabled: bool, inset: f32) -> f32 {
    if pressed && !disabled {
        2.0 * inset
    } else {
        0.0
    }
}

/// The thumb's travel from its resting corner. A checked thumb hands its stretch
/// back to the offset, so its right edge stays flush with the track instead of
/// riding over the edge of it.
fn thumb_offset(checked: bool, shift: f32, stretch: f32) -> f32 {
    if checked { shift - stretch } else { 0.0 }
}

/// The cross icon's (rotation, scale) at `progress`: it swings out towards the
/// checked side while it collapses.
fn cross_motion(progress: f32) -> (f32, f32) {
    (
        INDICATOR_SWING_TURNS * progress,
        1.0 - (1.0 - INDICATOR_MIN_SCALE) * progress,
    )
}

/// The check icon's (rotation, scale) at `progress`: the mirror image, entering
/// from the other side of where it settles.
fn check_motion(progress: f32) -> (f32, f32) {
    (
        -INDICATOR_SWING_TURNS * (1.0 - progress),
        INDICATOR_MIN_SCALE + (1.0 - INDICATOR_MIN_SCALE) * progress,
    )
}

/// The thumb's drop shadow: the demo's `0 1px 2px` over black at 20%, lifted to
/// 50% on the dark profile so the thumb still separates from a dark track. Yue
/// keeps its shadows as multi-layer CSS strings with no single-layer equivalent
/// to read, so this one stays here.
fn thumb_shadow(mode: ThemeMode) -> Vec<gpui_kit::BoxShadow> {
    let alpha = match mode {
        ThemeMode::Light => 0.2,
        ThemeMode::Dark => 0.5,
    };
    vec![
        gpui_kit::BoxShadow::new(px(0.0), px(1.0), gpui_kit::black().opacity(alpha))
            .blur_radius(px(2.0)),
    ]
}

/// The ink leg of a stroke. The reference only fades and swings these glyphs, so
/// the drawing is Yue's own addition, and it borrows the checkbox mark's clock:
/// the enter rung, started one fast rung after the click.
fn ink_policy(theme: YueTheme) -> gpui_kit::base::Transition {
    gpui_kit::base::Transition::new(duration(theme, "--switch-duration"))
        .delay(duration(theme, "--switch-duration-fast"))
        .ease(easing(theme, "--ease-enter"))
}

/// The erase leg: the ink goes before the container has finished fading out.
fn erase_policy(theme: YueTheme) -> gpui_kit::base::Transition {
    gpui_kit::base::Transition::new(duration(theme, "--switch-duration-fast"))
        .ease(easing(theme, "--ease-exit"))
}

/// One stroked sub-path of a thumb glyph: `drawn` is the fraction of the stroke the
/// ink has covered. The reference writes `pathLength="1"` + a dash offset on the
/// checkbox and menu marks only, and just fades these glyphs; Yue draws the switch
/// icon the same way it draws the check, so both components ink with one pen.
#[derive(Clone, Copy)]
struct GlyphStroke {
    points: &'static [(f32, f32)],
    drawn: f32,
}

const NO_STROKE: GlyphStroke = GlyphStroke {
    points: &[],
    drawn: 0.0,
};

/// One `<svg class="ico">` container of the demo: a glyph box centred on the
/// thumb, turned by `turns` of a turn, shrunk to `scale` and faded to `alpha`.
/// The alpha goes into the paint, not the element style, because path drawing
/// does not read an element's own opacity.
fn glyph_layer(
    size: f32,
    alpha: f32,
    turns: f32,
    scale: f32,
    color: gpui_kit::Hsla,
    strokes: [GlyphStroke; 2],
) -> impl IntoElement {
    canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| {
            let centre = bounds.center();
            for stroke in strokes {
                paint_glyph(window, centre, size, scale, turns, stroke, color, alpha);
            }
        },
    )
    .absolute()
    .inset_0()
}

#[expect(clippy::too_many_arguments, reason = "one glyph's whole paint recipe")]
fn paint_glyph(
    window: &mut Window,
    centre: Point<Pixels>,
    size: f32,
    scale: f32,
    turns: f32,
    stroke: GlyphStroke,
    color: gpui_kit::Hsla,
    alpha: f32,
) {
    if stroke.points.len() < 2 || stroke.drawn <= 0.0 || alpha <= 0.0 {
        return;
    }
    let unit = size / GLYPH_BOX;
    let (sin, cos) = (std::f32::consts::TAU * turns).sin_cos();
    // Offsets from the glyph centre stay in plain f32 until the path is built:
    // `Pixels` keeps its own arithmetic private, and the ink length is a ratio.
    let offset = |point: (f32, f32)| {
        let x = (point.0 - GLYPH_BOX / 2.0) * unit * scale;
        let y = (point.1 - GLYPH_BOX / 2.0) * unit * scale;
        (x * cos - y * sin, x * sin + y * cos)
    };
    let place = |point: (f32, f32)| {
        let (x, y) = offset(point);
        gpui_kit::point(centre.x + px(x), centre.y + px(y))
    };
    let mut length = 0.0;
    for pair in stroke.points.windows(2) {
        let (from, to) = (offset(pair[0]), offset(pair[1]));
        length += (to.0 - from.0).hypot(to.1 - from.1);
    }
    if length <= 0.0 {
        return;
    }
    let mut builder = PathBuilder::default().with_style(PathStyle::Stroke(
        StrokeOptions::default()
            .with_line_width(GLYPH_STROKE * unit * scale)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round),
    ));
    builder.move_to(place(stroke.points[0]));
    for point in &stroke.points[1..] {
        builder.line_to(place(*point));
    }
    // `stroke-dasharray: 1` with `stroke-dashoffset: 1 - drawn` on a normalised
    // path leaves the leading `drawn` of the stroke inked, so the ink grows from
    // the end the path starts at, exactly as the demo draws it.
    let dashes = [px(length * stroke.drawn), px(length)];
    if let Ok(path) = builder.dash_array(&dashes).build() {
        window.paint_path(path, color.opacity(alpha));
    }
}

/// The loading ring: the demo's `circle r=9` with `stroke-dasharray: 28 100`,
/// which inks 28 of its 56-unit circumference. Rotating it is the same as
/// starting that arc later along the circle, so the arc is drawn where it should
/// be instead of transforming an element.
fn ring_layer(size: f32, alpha: f32, turns: f32, color: gpui_kit::Hsla) -> impl IntoElement {
    canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| {
            if alpha <= 0.0 {
                return;
            }
            let centre = bounds.center();
            let unit = size / GLYPH_BOX;
            let radius = RING_RADIUS * unit;
            let at = |angle: f32| {
                gpui_kit::point(
                    centre.x + px(radius * angle.cos()),
                    centre.y + px(radius * angle.sin()),
                )
            };
            // The dashed arc is shorter than half the circle, so the sweep is the
            // small one and `large_arc` stays off.
            let span = RING_DASH / RING_RADIUS;
            let start = std::f32::consts::TAU * turns;
            let mut builder = PathBuilder::default().with_style(PathStyle::Stroke(
                StrokeOptions::default()
                    .with_line_width(GLYPH_STROKE * unit)
                    .with_line_cap(LineCap::Round),
            ));
            builder.move_to(at(start));
            builder.arc_to(
                gpui_kit::point(px(radius), px(radius)),
                px(0.0),
                span > std::f32::consts::PI,
                true,
                at(start + span),
            );
            if let Ok(path) = builder.build() {
                window.paint_path(path, color.opacity(alpha));
            }
        },
    )
    .absolute()
    .inset_0()
}

fn size_tokens(size: SwitchSize) -> (&'static str, &'static str, &'static str) {
    match size {
        SwitchSize::Small => (
            "--switch-width-sm",
            "--switch-height-sm",
            "--switch-thumb-sm",
        ),
        SwitchSize::Medium => (
            "--switch-width-md",
            "--switch-height-md",
            "--switch-thumb-md",
        ),
        SwitchSize::Large => (
            "--switch-width-lg",
            "--switch-height-lg",
            "--switch-thumb-lg",
        ),
    }
}

impl RenderOnce for YueSwitch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (width_token, height_token, thumb_token) = size_tokens(self.size);
        let inset_token = match self.size {
            SwitchSize::Small => "--switch-inset-sm",
            SwitchSize::Medium => "--switch-inset-md",
            SwitchSize::Large => "--switch-inset-lg",
        };
        let (track_radius, thumb_radius) = match self.shape {
            SwitchShape::Capsule => (
                "--switch-border-radius-capsule",
                "--switch-thumb-border-radius-capsule",
            ),
            SwitchShape::RoundedRectangle => (
                "--switch-border-radius-rounded",
                "--switch-thumb-border-radius-rounded",
            ),
        };
        let indicator_token = match self.size {
            SwitchSize::Small => "--switch-indicator-size-sm",
            SwitchSize::Medium => "--switch-indicator-size-md",
            SwitchSize::Large => "--switch-indicator-size-lg",
        };
        let width = pixels(self.theme, width_token);
        let thumb_size = pixels(self.theme, thumb_token);
        let inset = pixels(self.theme, inset_token);
        let shift = width - thumb_size - 2.0 * inset;
        let switch_duration = duration(self.theme, "--switch-duration");
        let spinner_duration = duration(self.theme, "--spinner-duration");
        // Every animated property of a switch rides the same duration and curve,
        // so each channel below asks for the same policy.
        let policy = |theme: YueTheme| {
            gpui_kit::base::Transition::new(switch_duration).ease(easing(theme, "--switch-ease"))
        };
        let press_state = (!self.disabled).then(|| {
            window.use_keyed_state((self.id.clone(), "yue-switch-press"), cx, |_, _| false)
        });
        let pressed = press_state.as_ref().is_some_and(|state| *state.read(cx));
        let stretch = press_stretch(pressed, self.disabled, inset);
        let offset = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-offset"),
            thumb_offset(self.checked, shift, stretch),
            policy(self.theme),
            window,
            cx,
        );
        let thumb_width = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-width"),
            thumb_size + stretch,
            policy(self.theme),
            window,
            cx,
        );
        let cross_progress = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-indicator"),
            f32::from(self.checked),
            policy(self.theme),
            window,
            cx,
        )
        .clamp(0.0, 1.0);
        let loading_progress = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-loading"),
            f32::from(self.loading),
            policy(self.theme),
            window,
            cx,
        )
        .clamp(0.0, 1.0);
        let track_color = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-track"),
            f32::from(!self.disabled && self.checked),
            policy(self.theme),
            window,
            cx,
        );
        let track_color = mix_rgba(
            rgba(self.theme, "--switch-background"),
            rgba(self.theme, "--switch-background-checked"),
            track_color.clamp(0.0, 1.0),
        );
        // The thumb colour only changes with the disabled state, which the demo
        // also leaves unanimated.
        let thumb_color = rgba(
            self.theme,
            if self.disabled {
                "--switch-thumb-color"
            } else if self.checked {
                "--switch-thumb-color-checked"
            } else {
                "--switch-thumb-color"
            },
        );
        // The ink runs on its own clock: it starts a fast rung after the click and
        // is wiped off before the fade is done, so it cannot ride the cross-fade.
        // The cross's two diagonals are drawn from the same moment, so they share
        // one channel rather than carrying a stagger neither glyph needs.
        let cross_inked = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-cross"),
            f32::from(!self.checked),
            if self.checked {
                erase_policy(self.theme)
            } else {
                ink_policy(self.theme)
            },
            window,
            cx,
        );
        let check_inked = gpui_kit::base::transition(
            (self.id.clone(), "yue-switch-check"),
            f32::from(self.checked),
            if self.checked {
                ink_policy(self.theme)
            } else {
                erase_policy(self.theme)
            },
            window,
            cx,
        );
        let drawn = |ink: f32| ink.clamp(0.0, 1.0);
        let indicator_size = pixels(self.theme, indicator_token);
        let icons_alpha = 1.0 - loading_progress;
        let (cross_turns, cross_scale) = cross_motion(cross_progress);
        let (check_turns, check_scale) = check_motion(cross_progress);
        let cross_color =
            gpui_kit::Hsla::from(rgba(self.theme, "--switch-indicator-color-unchecked"));
        let check_color =
            gpui_kit::Hsla::from(rgba(self.theme, "--switch-indicator-color-checked"));
        let spinner_color = gpui_kit::Hsla::from(rgba(self.theme, "--spinner-color"));
        let thumb = gpui_kit::base::SwitchThumb::new(self.checked)
            .disabled(self.disabled)
            .absolute()
            .top(rems(self.theme, inset_token))
            .left(gpui_kit::rems((inset + offset) / 16.0))
            .w(gpui_kit::rems(thumb_width / 16.0))
            .h(rems(self.theme, thumb_token))
            .rounded(rems(self.theme, thumb_radius))
            .bg(thumb_color)
            .when(!self.disabled, |thumb| {
                thumb.shadow(thumb_shadow(self.theme.mode()))
            })
            .when(
                matches!(self.indicator, SwitchIndicator::X | SwitchIndicator::CheckX),
                |thumb| {
                    thumb.child(glyph_layer(
                        indicator_size,
                        (1.0 - cross_progress) * icons_alpha,
                        cross_turns,
                        cross_scale,
                        cross_color,
                        [
                            GlyphStroke {
                                points: &CROSS_FIRST,
                                drawn: drawn(cross_inked),
                            },
                            GlyphStroke {
                                points: &CROSS_SECOND,
                                drawn: drawn(cross_inked),
                            },
                        ],
                    ))
                },
            )
            .when(
                matches!(
                    self.indicator,
                    SwitchIndicator::Check | SwitchIndicator::CheckX
                ),
                |thumb| {
                    thumb.child(glyph_layer(
                        indicator_size,
                        cross_progress * icons_alpha,
                        check_turns,
                        check_scale,
                        check_color,
                        [
                            GlyphStroke {
                                points: &CHECK_LINE,
                                drawn: drawn(check_inked),
                            },
                            NO_STROKE,
                        ],
                    ))
                },
            )
            .when(loading_progress > 0.0, |thumb| {
                thumb.child(div().absolute().inset_0().with_animation(
                    "yue-switch-spinner",
                    Animation::new(spinner_duration).repeat(),
                    move |layer, delta| {
                        layer.child(ring_layer(
                            indicator_size,
                            loading_progress,
                            delta,
                            spinner_color,
                        ))
                    },
                ))
            });
        let track = gpui_kit::base::SwitchTrack::new((self.id.clone(), "track"))
            .checked(self.checked)
            .disabled(self.disabled)
            .relative()
            .w(rems(self.theme, width_token))
            .h(rems(self.theme, height_token))
            .rounded(rems(self.theme, track_radius))
            .bg(track_color)
            // The track carries no border, like the demo's `border: 0`: a border
            // would shrink the box the thumb is placed inside, so `top: inset`
            // would measure from below it and the thumb would sit low.
            .when(self.disabled, |track| {
                track.opacity(number(self.theme, "--switch-opacity-disabled"))
            })
            .child(thumb);

        let mut switch = gpui_kit::base::Switch::new(self.id)
            .checked(self.checked)
            .disabled(self.disabled)
            .accessibility_label(self.accessibility_label)
            .child(track)
            .focus_visible(|style| {
                style
                    .border(rems(self.theme, "--switch-border-width"))
                    .border_color(rgba(self.theme, "--focus-ring-color"))
            });
        if let Some(state) = press_state {
            let down = window.listener_for(&state, |state, _, window, cx| {
                if !*state {
                    *state = true;
                    cx.notify();
                    window.refresh();
                }
            });
            let up = window.listener_for(&state, |state, _, window, cx| {
                if *state {
                    *state = false;
                    cx.notify();
                    window.refresh();
                }
            });
            let up_out = window.listener_for(&state, |state, _, window, cx| {
                if *state {
                    *state = false;
                    cx.notify();
                    window.refresh();
                }
            });
            switch = switch
                .on_mouse_down(MouseButton::Left, down)
                .on_mouse_up(MouseButton::Left, up)
                .on_mouse_up_out(MouseButton::Left, up_out);
        }
        let loading = self.loading;
        if let Some(handler) = self.on_change.filter(|_| !loading) {
            switch = switch.on_change(move |checked, event, window, cx| {
                handler(checked, event, window, cx);
            });
        }
        switch
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{SwitchIndicator, SwitchShape, SwitchSize, YueSwitch};
    use crate::tokens::token;
    use crate::{Accent, ThemeMode, YueTheme};

    #[test]
    fn switch_builder_keeps_size_shape_and_indicator_selection() {
        let switch = YueSwitch::new(
            "power",
            YueTheme::new(ThemeMode::Light, Accent::Azure),
            false,
            "Power",
        )
        .size(SwitchSize::Large)
        .shape(SwitchShape::RoundedRectangle)
        .indicator(SwitchIndicator::CheckX)
        .loading(true)
        .disabled(true);
        assert_eq!(switch.size, SwitchSize::Large);
        assert_eq!(switch.shape, SwitchShape::RoundedRectangle);
        assert_eq!(switch.indicator, SwitchIndicator::CheckX);
        assert!(switch.loading);
        assert!(switch.disabled);
    }

    #[test]
    fn switch_profiles_resolve_geometry_and_icon_tokens() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        for (size, width, height, thumb, inset_token) in [
            (SwitchSize::Small, 32.0, 20.0, 16.0, "--switch-inset-sm"),
            (SwitchSize::Medium, 40.0, 24.0, 20.0, "--switch-inset-md"),
            (SwitchSize::Large, 48.0, 28.0, 24.0, "--switch-inset-lg"),
        ] {
            let (width_token, height_token, thumb_token) = super::size_tokens(size);
            let inset = super::pixels(theme, inset_token);
            assert_eq!(super::pixels(theme, width_token), width);
            assert_eq!(super::pixels(theme, height_token), height);
            assert_eq!(super::pixels(theme, thumb_token), thumb);
            assert_eq!(inset, 2.0, "{inset_token}");
            assert_eq!(
                height,
                thumb + 2.0 * inset,
                "{size:?} centers the thumb: two insets stack on top of it"
            );
        }
        assert_eq!(super::pixels(theme, "--switch-indicator-size-md"), 12.0);
        for (indicator_token, stroke_token) in [
            (
                "--switch-indicator-size-sm",
                "--checkbox-mark-stroke-width-sm",
            ),
            (
                "--switch-indicator-size-md",
                "--checkbox-mark-stroke-width-md",
            ),
            (
                "--switch-indicator-size-lg",
                "--checkbox-mark-stroke-width-lg",
            ),
        ] {
            assert_eq!(
                super::pixels(theme, stroke_token),
                super::GLYPH_STROKE * super::pixels(theme, indicator_token) / super::GLYPH_BOX,
                "{stroke_token} keeps the checkbox marks at the weight these glyphs paint with"
            );
        }
        assert_eq!(
            token(theme, "--switch-background").color(),
            Some(0xa3a3a3),
            "the unchecked track stays visibly distinct from its surface"
        );
        assert_eq!(
            token(theme, "--switch-thumb-color").color(),
            Some(0xffffff),
            "the unchecked thumb is a light surface against the gray track"
        );
        assert_eq!(
            token(theme, "--switch-thumb-color-checked").color(),
            Some(0xffffff),
            "the checked thumb remains white against the accent track"
        );
        assert_eq!(
            token(theme, "--switch-indicator-color-checked").color(),
            Some(0x1f75db),
            "the checked indicator uses the accent color on the white thumb"
        );
        assert_eq!(
            super::duration(theme, "--switch-duration"),
            std::time::Duration::from_millis(200)
        );
    }

    #[test]
    fn switch_thumb_shadow_lifts_the_thumb_without_borrowing_its_size() {
        for (mode, alpha) in [(ThemeMode::Light, 0.2), (ThemeMode::Dark, 0.5)] {
            let shadows = super::thumb_shadow(mode);
            let [shadow] = shadows.as_slice() else {
                panic!("the thumb casts exactly one shadow");
            };
            assert_eq!(shadow.offset.x, gpui_kit::px(0.0));
            assert_eq!(shadow.offset.y, gpui_kit::px(1.0), "{mode:?} drops 1px");
            assert_eq!(shadow.blur_radius, gpui_kit::px(2.0));
            assert_eq!(
                shadow.spread_radius,
                gpui_kit::px(0.0),
                "no spread, so the shadow never implies a bigger thumb"
            );
            assert!(!shadow.inset);
            assert!(
                (shadow.color.a - alpha).abs() < 1e-6,
                "{mode:?} keeps the thumb readable without darkening the track"
            );
        }
    }

    #[test]
    fn switch_check_glyph_is_the_demo_path_grown_and_centred() {
        let extent = |axis: usize, points: &[(f32, f32)]| {
            let coord = |point: &(f32, f32)| if axis == 0 { point.0 } else { point.1 };
            let min = points.iter().map(coord).fold(f32::MAX, f32::min);
            let max = points.iter().map(coord).fold(f32::MIN, f32::max);
            (min, max)
        };
        let (left, right) = extent(0, &super::CHECK_LINE);
        let (top, bottom) = extent(1, &super::CHECK_LINE);
        assert_eq!(
            left + right,
            top + bottom,
            "the enlarged check stays centred on the glyph box"
        );
        let (cross_top, cross_bottom) = extent(1, &super::CROSS_SECOND);
        assert_eq!(
            bottom - top,
            cross_bottom - cross_top - 1.0,
            "the check is 11 units tall: 1.1x the demo's 10, one short of the cross"
        );
        for pair in super::CHECK_LINE.windows(2) {
            let (dx, dy) = (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
            assert_eq!(dx.abs(), dy.abs(), "both legs of the check stay at 45°");
        }
        assert!(
            left - super::GLYPH_STROKE / 2.0 >= 0.0
                && right + super::GLYPH_STROKE / 2.0 <= super::GLYPH_BOX,
            "the check's ink, round caps included, stays inside the shared glyph box"
        );
    }

    #[test]
    fn switch_press_geometry_keeps_a_stretched_thumb_inside_the_track() {
        let (inset, shift, thumb) = (2.0, 16.0, 20.0);
        let stretch = super::press_stretch(true, false, inset);
        assert_eq!(super::press_stretch(false, false, inset), 0.0);
        assert_eq!(
            super::press_stretch(true, true, inset),
            0.0,
            "a switch that cannot be toggled cannot be stretched either"
        );
        assert_eq!(stretch, 4.0, "the demo swells its 20px thumb to 24px");
        assert_eq!(super::thumb_offset(false, shift, 0.0), 0.0);
        assert_eq!(
            super::thumb_offset(false, shift, stretch),
            0.0,
            "an un-checked thumb grows towards the checked side, not off its corner"
        );
        assert_eq!(super::thumb_offset(true, shift, 0.0), shift);
        assert_eq!(
            super::thumb_offset(true, shift, stretch) + thumb + stretch,
            shift + thumb,
            "the stretched thumb hands its extra width back to the offset, so the leading edge stays put"
        );
    }

    #[test]
    fn switch_indicator_glyphs_mirror_across_the_cross_fade() {
        for (progress, cross, check) in [
            (0.0, (0.0, 1.0), (-0.125, 0.5)),
            (0.5, (0.0625, 0.75), (-0.0625, 0.75)),
            (1.0, (0.125, 0.5), (0.0, 1.0)),
        ] {
            assert_eq!(super::cross_motion(progress), cross);
            assert_eq!(super::check_motion(progress), check);
        }
    }

    #[test]
    fn switch_motion_legs_resolve_token_durations_and_curves() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(
            super::duration(theme, "--switch-duration-fast"),
            Duration::from_millis(100),
            "the erase leg and the ink delay run on the switch's own fast rung"
        );
        // The ink leg is no longer on a literal clock of its own: it reads the same
        // two rungs the checkbox mark is drawn with.
        assert_eq!(
            super::duration(theme, "--switch-duration"),
            super::duration(theme, "--checkbox-duration"),
            "the glyph is written for as long as the checkbox mark"
        );
        assert_eq!(
            super::duration(theme, "--switch-duration-fast"),
            super::duration(theme, "--checkbox-duration-fast"),
            "and starts as late as the checkbox mark"
        );
        assert_eq!(
            super::duration(theme, "--spinner-duration"),
            Duration::from_millis(900),
            "the ring resolves a calc() duration"
        );
        for name in ["--switch-ease", "--ease-enter", "--ease-exit"] {
            let curve = super::easing(theme, name);
            assert_eq!(curve(0.0), 0.0, "{name} starts where the value starts");
            assert_eq!(curve(1.0), 1.0, "{name} ends on target");
            let mut previous = 0.0;
            for step in 1..=20u32 {
                let value = curve(step as f32 / 20.0);
                assert!(
                    (previous - 1e-6..=1.0 + 1e-6).contains(&value),
                    "{name} never rewinds or overshoots at {step}/20"
                );
                previous = value;
            }
        }
    }

    #[test]
    fn switch_track_colour_mixes_along_the_rgb_axes() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let off = super::rgba(theme, "--switch-background");
        let on = super::rgba(theme, "--switch-background-checked");
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        for (progress, want, note) in [
            (0.0, off, "an unmoved track keeps its unchecked token"),
            (
                1.0,
                on,
                "a committed track lands on its checked token, not near it",
            ),
        ] {
            let mixed = super::mix_rgba(off, on, progress);
            assert!(
                close(mixed.r, want.r) && close(mixed.g, want.g) && close(mixed.b, want.b),
                "{note}: got {mixed:?} want {want:?}"
            );
        }
        let half = super::mix_rgba(off, on, 0.5);
        for (from, to, mid, channel) in [
            (off.r, on.r, half.r, "red"),
            (off.g, on.g, half.g, "green"),
            (off.b, on.b, half.b, "blue"),
        ] {
            assert!(
                (from.min(to)..from.max(to)).contains(&mid),
                "{channel} travels the straight line, got {mid} for {from} and {to}"
            );
            assert!(
                close(mid, (from + to) / 2.0),
                "{channel} reaches its average at half progress"
            );
        }
        assert!(close(half.a, off.a), "both track colours are opaque");
    }
}
