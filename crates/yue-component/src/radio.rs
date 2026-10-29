//! Yue radio: one controlled option and the group that owns mutual exclusion.
//!
//! The option keeps GPUI Base's activation, focus and accessibility behaviour and
//! adds Yue's appearance: either a circle that fills with the accent and grows a
//! centre dot, or a segment of the outlined row the docs call the Button
//! appearance. The group owns what Base leaves open — the selected value, the one
//! tab stop the whole set lives behind, and the arrow keys that move between
//! options.

use std::rc::Rc;

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ClickEvent, ElementId, FocusHandle, FontWeight, InteractiveElement as _, IntoElement,
    KeyDownEvent, MouseButton, ParentElement as _, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use crate::tokens::{duration, easing, mix_rgba, number, pixels, rems, rgba};
use crate::{GroupDirection, YueTheme};

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;
type ValueHandler = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;

/// The keyed slot a Yue radio keeps its focus handle in, so the ring it draws and
/// the handle a group focuses are the same handle.
const FOCUS_SLOT: &str = "yue-radio-focus";

/// Three Yue radio size profiles.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RadioSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// How an option presents itself. Both appearances carry the same roles, the same
/// keyboard and the same mutual exclusion; only the drawing changes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RadioAppearance {
    /// A circle and a label — the default, and what a form reads as a radio.
    #[default]
    Indicator,
    /// One segment of a continuous, outlined row of mutually exclusive short
    /// labels. A [`YueRadioGroup`] composes these; an option used on its own so
    /// styled draws the whole outline around itself.
    ///
    /// Long labels, many options, or a row that would have to wrap belong to the
    /// Indicator appearance or to a Select: a wrapped segment row no longer reads
    /// as one choice set.
    Button,
}

/// Where a segment sits inside its row. The row outlines itself once and clips
/// its own ends, so a segment only ever carries the divider it shares with the
/// neighbour before it — on the axis the row runs along.
#[derive(Clone, Copy, Debug)]
struct Segment {
    vertical: bool,
    first: bool,
}

/// A controlled Yue radio option. The caller owns the selected value.
#[derive(IntoElement)]
pub struct YueRadio {
    id: ElementId,
    theme: YueTheme,
    checked: bool,
    disabled: bool,
    size: RadioSize,
    appearance: RadioAppearance,
    segment: Option<Segment>,
    label: Option<SharedString>,
    accessibility_label: Option<SharedString>,
    focus_handle: Option<FocusHandle>,
    tab_stop: bool,
    position: Option<(usize, usize)>,
    on_change: Option<ChangeHandler>,
}

impl YueRadio {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme, checked: bool) -> Self {
        Self {
            id: id.into(),
            theme,
            checked,
            disabled: false,
            size: RadioSize::Medium,
            appearance: RadioAppearance::default(),
            segment: None,
            label: None,
            accessibility_label: None,
            focus_handle: None,
            tab_stop: true,
            position: None,
            on_change: None,
        }
    }

    pub fn size(mut self, size: RadioSize) -> Self {
        self.size = size;
        self
    }

    /// Draws the option as a segment of an outlined row instead of as a circle. A
    /// group sets this for its whole row, so a lone radio keeps the group's own
    /// outline shape around itself.
    pub fn appearance(mut self, appearance: RadioAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Visible text beside the circle. The whole row — circle and label — is the
    /// click target, and the text names the control in the accessibility tree.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Names a radio that carries no visible text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reports the requested selection. Base only asks when the option is enabled
    /// and not already chosen, so a radio cannot switch itself off and this never
    /// carries `false`.
    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Uses the handle a group drives roving focus through, instead of creating a
    /// keyed one. The group also reads it to tell which option is focused.
    pub(crate) fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    /// Whether Tab reaches this option. A group keeps exactly one option in the
    /// tab order and moves that stop with the selection.
    pub(crate) fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Places this segment in a group's row. Internal: the row is the one that
    /// knows its own axis and which option leads it.
    pub(crate) fn segment(mut self, vertical: bool, first: bool) -> Self {
        self.segment = Some(Segment { vertical, first });
        self
    }

    /// This option's one-based position and the size of its set, so assistive
    /// technology can announce "option 2 of 5".
    pub(crate) fn set_position(mut self, position: usize, size: usize) -> Self {
        self.position = Some((position, size));
        self
    }

    fn accessible_name(&self) -> SharedString {
        match (&self.label, &self.accessibility_label) {
            (Some(label), _) | (None, Some(label)) => label.clone(),
            (None, None) => {
                panic!("YueRadio needs a label, or an accessibility_label without visible text")
            }
        }
    }
}

/// One option of a [`YueRadioGroup`]: a stable value, its visible text, and
/// whether it can currently be chosen.
#[derive(Clone)]
pub struct YueRadioOption {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl YueRadioOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A single-choice group: the caller holds the selected value, the group offers
/// the options and moves the selection one option at a time.
///
/// The callback takes the new value and the contexts, without the `ClickEvent` a
/// pointer or a Space key carries: a group's selection also changes when an arrow
/// key moves onto another option, and a click event invented for that keystroke
/// would report a button the user never pressed.
#[derive(IntoElement)]
pub struct YueRadioGroup {
    id: ElementId,
    theme: YueTheme,
    selected: Option<SharedString>,
    options: Vec<YueRadioOption>,
    label: Option<SharedString>,
    disabled: bool,
    size: RadioSize,
    appearance: RadioAppearance,
    direction: GroupDirection,
    on_change: Option<ValueHandler>,
}

impl YueRadioGroup {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme, selected: Option<SharedString>) -> Self {
        Self {
            id: id.into(),
            theme,
            selected,
            options: Vec::new(),
            label: None,
            disabled: false,
            size: RadioSize::Medium,
            appearance: RadioAppearance::default(),
            direction: GroupDirection::Vertical,
            on_change: None,
        }
    }

    /// The options offered, in visual order. Values must be unique: they are the
    /// identity the selection is stored against, and a repeated value would make
    /// two options indistinguishable.
    pub fn options(mut self, options: impl IntoIterator<Item = YueRadioOption>) -> Self {
        self.options = options.into_iter().collect();
        let mut seen: Vec<SharedString> = Vec::with_capacity(self.options.len());
        for option in &self.options {
            assert!(
                !seen.contains(&option.value),
                "YueRadioGroup option values must be unique, found {:?} twice",
                option.value.as_ref()
            );
            seen.push(option.value.clone());
        }
        self
    }

    /// A visible caption that also names the group in the accessibility tree.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: RadioSize) -> Self {
        self.size = size;
        self
    }

    /// Whether the options read as circles beside their labels or as one outlined
    /// row of segments. The keyboard contract is the same either way: one tab stop
    /// for the group, arrows moving and choosing along its axis.
    pub fn appearance(mut self, appearance: RadioAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn direction(mut self, direction: GroupDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Reports the newly selected value. Choosing the option that is already
    /// selected asks for nothing, so this is not called.
    pub fn on_change(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// The option Tab reaches: the selected one while it can still be operated,
/// otherwise the first operable option, and nothing when the whole group is inert.
///
/// Unlike a checkbox group's select-all header, which reports a state and so keeps
/// reading its options even while the group is disabled, this is about what Tab
/// can land on — a disabled group offers nothing.
fn tab_stop_index(
    options: &[YueRadioOption],
    selected: Option<&SharedString>,
    group_disabled: bool,
) -> Option<usize> {
    if group_disabled {
        return None;
    }
    let operable = |option: &YueRadioOption| !option.disabled;
    options
        .iter()
        .position(|option| operable(option) && Some(&option.value) == selected)
        .or_else(|| options.iter().position(operable))
}

/// Where an arrow key inside a group goes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RadioMove {
    Next,
    Previous,
    First,
    Last,
}

/// The keystroke a group of this running direction reads as a move, or `None` for
/// a key the group leaves to the rest of the window.
///
/// A vertical column is read top to bottom and a horizontal row left to right, so
/// the perpendicular pair stays free — a nested group of the other orientation
/// keeps its own keys.
fn move_for(key: &str, vertical: bool) -> Option<RadioMove> {
    match key {
        "home" => Some(RadioMove::First),
        "end" => Some(RadioMove::Last),
        "down" if vertical => Some(RadioMove::Next),
        "up" if vertical => Some(RadioMove::Previous),
        "right" if !vertical => Some(RadioMove::Next),
        "left" if !vertical => Some(RadioMove::Previous),
        _ => None,
    }
}

/// The option a move lands on, counted among the `operable` options in visual
/// order and skipping the ones that cannot be chosen. `current` is the focused
/// option's own place in that list.
///
/// Next and Previous wrap, so an arrow key never runs out of group; First and Last
/// ignore `current`. With nothing to move to there is nothing to focus.
fn move_index(operable: usize, current: usize, direction: RadioMove) -> Option<usize> {
    if operable == 0 {
        return None;
    }
    Some(match direction {
        RadioMove::First => 0,
        RadioMove::Last => operable - 1,
        RadioMove::Next => (current + 1) % operable,
        RadioMove::Previous => (current + operable - 1) % operable,
    })
}

fn size_tokens(size: RadioSize) -> (&'static str, &'static str, &'static str, &'static str) {
    match size {
        RadioSize::Small => (
            "--radio-size-sm",
            "--radio-dot-size-sm",
            "--radio-height-sm",
            "--radio-label-font-size-sm",
        ),
        RadioSize::Medium => (
            "--radio-size-md",
            "--radio-dot-size-md",
            "--radio-height-md",
            "--radio-label-font-size-md",
        ),
        RadioSize::Large => (
            "--radio-size-lg",
            "--radio-dot-size-lg",
            "--radio-height-lg",
            "--radio-label-font-size-lg",
        ),
    }
}

/// The row a segment lives in, the label it carries, and the inline padding that
/// gives it its width — the same three rungs the Indicator appearance reads,
/// resolved against the segmented control's own ladder.
fn segment_tokens(size: RadioSize) -> (&'static str, &'static str, &'static str) {
    match size {
        RadioSize::Small => (
            "--radio-button-height-sm",
            "--radio-button-font-size-sm",
            "--radio-button-padding-inline-sm",
        ),
        RadioSize::Medium => (
            "--radio-button-height-md",
            "--radio-button-font-size-md",
            "--radio-button-padding-inline-md",
        ),
        RadioSize::Large => (
            "--radio-button-height-lg",
            "--radio-button-font-size-lg",
            "--radio-button-padding-inline-lg",
        ),
    }
}

/// Where a segment's focus ring sits. `outline-offset` measures from the border
/// box and GPUI offsets an absolute child from the content box, so the segment's
/// own divider is one leg of the distance the ring travels inward.
fn ring_inset(ring_offset: f32, border: f32) -> f32 {
    ring_offset - border
}

/// Paints a translucent overlay the way a browser paints a `color-mix()`
/// background over a fill: the result keeps the fill's own opacity and only its
/// channels move toward the overlay, so a hovered segment tints itself instead of
/// punching a hole through to whatever sits behind the row.
fn tint(fill: gpui_kit::Rgba, overlay: gpui_kit::Rgba, progress: f32) -> gpui_kit::Rgba {
    fill.blend(gpui_kit::Rgba {
        a: overlay.a * progress,
        ..overlay
    })
}

/// The pointer states that drive the hover halo and the press shrink. Kept as one
/// piece of keyed state so a move or a release costs a single repaint.
#[derive(Clone, Copy, Default)]
struct Pointer {
    hovered: bool,
    pressed: bool,
}

/// What both appearances need before they style anything: the identity their
/// transitions key on, the name the node carries, and the pointer and focus facts
/// that decide which state gets drawn.
struct RadioFrame<'a> {
    id: ElementId,
    name: SharedString,
    focus: &'a FocusHandle,
    ring_visible: bool,
    hovered: bool,
    pressed: bool,
}

impl YueRadio {
    /// The option as Base sees it: its state, its name, and its place in the tab
    /// order and the set. Both appearances start here and add only drawing.
    fn base(&self, frame: &RadioFrame) -> gpui_kit::base::Radio {
        gpui_kit::base::Radio::new(frame.id.clone())
            .checked(self.checked)
            .disabled(self.disabled)
            .accessibility_label(frame.name.clone())
            .tab_stop(self.tab_stop)
            .tab_index(if self.tab_stop { 0 } else { -1 })
            .track_focus(frame.focus)
            .when(!self.disabled, |radio| radio.cursor_pointer())
            .when_some(self.position, |radio, (position, size)| {
                radio.set_position(position, size)
            })
    }

    /// The circle, with the label beside it.
    fn indicator(
        &self,
        frame: &RadioFrame,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui_kit::base::Radio {
        let id = frame.id.clone();
        let checked = self.checked;
        let (hovered, pressed, ring_visible) = (frame.hovered, frame.pressed, frame.ring_visible);
        let (box_token, dot_token, height_token, font_token) = size_tokens(self.size);
        let box_size = pixels(self.theme, box_token);
        let dot_size = pixels(self.theme, dot_token);
        let border = pixels(self.theme, "--radio-border-width");
        let radius = pixels(self.theme, "--radio-border-radius");
        let halo_width = pixels(self.theme, "--radio-halo-width");
        let halo_color = rgba(self.theme, "--radio-halo-color");
        let ring_width = pixels(self.theme, "--radio-focus-ring-width");
        let ring_offset = pixels(self.theme, "--radio-focus-ring-offset");
        let ring_color = rgba(self.theme, "--radio-focus-ring-color");

        let standard = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--radio-duration"))
                .ease(easing(theme, "--radio-ease"))
        };
        // The dot grows and collapses on the same rung the circle changes on; the
        // reference switches only its easing, entering with the ease-in curve and
        // leaving with the ease-out one.
        let dot_motion = |theme: YueTheme, growing| {
            gpui_kit::base::Transition::new(duration(theme, "--radio-duration")).ease(easing(
                theme,
                if growing {
                    "--radio-ease-enter"
                } else {
                    "--radio-ease-exit"
                },
            ))
        };
        // The circle shrinks on the short rung, eased the same way as the fill.
        let press = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--radio-duration-fast"))
                .ease(easing(theme, "--radio-ease"))
        };

        let mut progress = |channel: &'static str, target: f32, policy| {
            gpui_kit::base::transition(
                (id.clone(), format!("yue-radio-{channel}")),
                target,
                policy,
                window,
                cx,
            )
            .clamp(0.0, 1.0)
        };
        let fill = progress("fill", f32::from(checked), standard(self.theme));
        let emphasis = progress(
            "border",
            f32::from(hovered || checked),
            standard(self.theme),
        );
        let halo = progress("halo", f32::from(hovered), standard(self.theme));
        let dot = progress("dot", f32::from(checked), dot_motion(self.theme, checked));
        let scale = gpui_kit::base::transition(
            (id.clone(), "yue-radio-press"),
            if pressed {
                number(self.theme, "--radio-press-scale")
            } else {
                1.0
            },
            press(self.theme),
            window,
            cx,
        );

        // A disabled circle is repainted rather than transitioned, the way the
        // reference's disabled rule replaces the fill outright.
        let background = if self.disabled {
            rgba(self.theme, "--radio-disabled-background")
        } else {
            mix_rgba(
                rgba(self.theme, "--radio-background"),
                rgba(self.theme, "--radio-background-checked"),
                fill,
            )
        };
        let border_color = if self.disabled {
            rgba(self.theme, "--radio-disabled-border-color")
        } else {
            mix_rgba(
                rgba(self.theme, "--radio-border-color"),
                rgba(self.theme, "--radio-border-color-checked"),
                emphasis,
            )
        };
        let dot_color = rgba(
            self.theme,
            if self.disabled {
                "--radio-disabled-dot-color"
            } else {
                "--radio-dot-color"
            },
        );
        // Everything about the circle shrinks together with the press, the dot
        // included: the reference scales the element, and its `::after` rides the
        // same box.
        let inner = box_size * scale;
        let content = inner - 2.0 * border * scale;
        let margin = (box_size - inner) / 2.0;
        let dot_inset = (content - dot_size * scale) / 2.0;
        // `outline-offset` measures from the border box, and GPUI offsets an
        // absolute child from the content box, so the border is one leg of the
        // distance.
        let ring_inset = -scale * (ring_offset + ring_width + border);

        let box_layer = div()
            .absolute()
            .left(px(margin))
            .top(px(margin))
            .w(px(inner))
            .h(px(inner))
            .rounded(gpui_kit::rems(radius * scale / 16.0))
            .bg(background)
            .border(px(border * scale))
            .border_color(border_color)
            .when(halo > 0.0, |layer| {
                // The hover halo is a spread shadow rather than a laid-out border,
                // so hovering never moves the circle or the label beside it.
                layer.shadow(vec![
                    gpui_kit::BoxShadow::new(
                        px(0.0),
                        px(0.0),
                        gpui_kit::Hsla::from(halo_color).opacity(halo * halo_color.a),
                    )
                    .spread_radius(px(halo_width * halo)),
                ])
            })
            .when(dot > 0.0, |layer| {
                layer.child(
                    div()
                        .absolute()
                        .top(px(dot_inset))
                        .left(px(dot_inset))
                        .right(px(dot_inset))
                        .bottom(px(dot_inset))
                        .rounded(gpui_kit::rems(radius * scale / 16.0))
                        .bg(dot_color),
                )
            })
            .when(ring_visible, |layer| {
                layer.child(
                    div()
                        .absolute()
                        .top(px(ring_inset))
                        .left(px(ring_inset))
                        .right(px(ring_inset))
                        .bottom(px(ring_inset))
                        .border(px(ring_width * scale))
                        .border_color(ring_color)
                        .rounded(gpui_kit::rems(
                            (radius + ring_offset + ring_width) * scale / 16.0,
                        )),
                )
            });

        self.base(frame)
            .flex()
            .items_center()
            .gap(rems(self.theme, "--radio-label-gap"))
            .min_h(rems(self.theme, height_token))
            .text_size(rems(self.theme, font_token))
            .text_color(rgba(
                self.theme,
                if self.disabled {
                    "--radio-label-color-disabled"
                } else {
                    "--radio-label-color"
                },
            ))
            // A fixed slot holds the circle, so shrinking it on press moves nothing
            // else in the row — the reference's `transform: scale(.9)` never reflows
            // either.
            .child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .w(px(box_size))
                    .h(px(box_size))
                    .child(box_layer),
            )
            .when_some(self.label.clone(), |radio, label| radio.child(label))
    }

    /// One cell of the outlined row: the option's state as a fill and a label
    /// colour, no circle, and the same focus ring the circle draws — inset,
    /// because the row clips its ends and a ring outside a segment would be cut
    /// in half.
    fn segment_row(
        &self,
        frame: &RadioFrame,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui_kit::base::Radio {
        let id = frame.id.clone();
        let checked = self.checked;
        let (hovered, pressed, ring_visible) = (frame.hovered, frame.pressed, frame.ring_visible);
        let (height_token, font_token, padding_token) = segment_tokens(self.size);
        let border = pixels(self.theme, "--radio-button-border-width");
        let radius = pixels(self.theme, "--radio-button-border-radius");
        let ring_width = pixels(self.theme, "--radio-focus-ring-width");
        let ring_offset = pixels(self.theme, "--radio-focus-ring-offset");
        let ring_color = rgba(self.theme, "--radio-focus-ring-color");

        // The choice arrives on the row's enter rung and the press leaves on the
        // exit one, the way the reference splits a segmented row's transitions. A
        // segment never scales, so the press reads as its deeper overlay.
        let recolour = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--radio-button-duration"))
                .ease(easing(theme, "--radio-button-ease"))
        };
        let release = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--radio-button-duration-fast"))
                .ease(easing(theme, "--radio-button-ease"))
        };

        let mut progress = |channel: &'static str, target: f32, policy| {
            gpui_kit::base::transition(
                (id.clone(), format!("yue-radio-{channel}")),
                target,
                policy,
                window,
                cx,
            )
            .clamp(0.0, 1.0)
        };
        let selected = progress("segment-fill", f32::from(checked), recolour(self.theme));
        // Only an unchosen segment takes a hover or a press overlay: the chosen one
        // already reads as selected, and the reference lifts neither its fill nor
        // its label further.
        let hover = progress(
            "segment-hover",
            f32::from(hovered && !checked),
            recolour(self.theme),
        );
        let press = progress(
            "segment-press",
            f32::from(pressed && !checked),
            release(self.theme),
        );
        let lift = progress(
            "segment-text",
            f32::from(hovered && !checked),
            recolour(self.theme),
        );

        // A disabled segment keeps the fill it died with — losing the chosen tint
        // would hide the very selection the caller still holds.
        let background = if self.disabled && !checked {
            rgba(self.theme, "--radio-button-disabled-background")
        } else {
            let fill = mix_rgba(
                rgba(self.theme, "--radio-button-background"),
                rgba(self.theme, "--radio-button-background-selected"),
                selected,
            );
            let fill = tint(
                fill,
                rgba(self.theme, "--radio-button-hover-background"),
                hover,
            );
            tint(
                fill,
                rgba(self.theme, "--radio-button-pressed-background"),
                press,
            )
        };
        let color = if self.disabled {
            rgba(self.theme, "--radio-button-disabled-color")
        } else {
            let ink = mix_rgba(
                rgba(self.theme, "--radio-button-color"),
                rgba(self.theme, "--radio-button-color-hover"),
                lift,
            );
            mix_rgba(
                ink,
                rgba(self.theme, "--radio-button-color-selected"),
                selected,
            )
        };

        self.base(frame)
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .min_h(px(pixels(self.theme, height_token)))
            .px(px(pixels(self.theme, padding_token)))
            .text_size(px(pixels(self.theme, font_token)))
            .text_color(color)
            .bg(background)
            .whitespace_nowrap()
            // The chosen label carries weight as well as colour, so the selection
            // still reads in a monochrome or high-contrast profile.
            .when(checked && !self.disabled, |radio| {
                radio.font_weight(FontWeight(number(
                    self.theme,
                    "--radio-button-weight-selected",
                )))
            })
            .when_some(self.label.clone(), |radio, label| radio.child(label))
            // The row outlines itself once, so a segment adds only the divider it
            // shares with the neighbour before it — on the axis the row runs along.
            .when_some(self.segment, |radio, segment| {
                if segment.first {
                    radio
                } else if segment.vertical {
                    radio.border_t(px(border))
                } else {
                    radio.border_l(px(border))
                }
            })
            .when(self.segment.is_none(), |radio| {
                radio
                    .border(px(border))
                    .rounded(gpui_kit::rems(radius / 16.0))
            })
            .border_color(rgba(self.theme, "--radio-button-border-color"))
            .when(ring_visible, move |radio| {
                let inset = ring_inset(ring_offset, border);
                radio.child(
                    div()
                        .absolute()
                        .top(px(inset))
                        .left(px(inset))
                        .right(px(inset))
                        .bottom(px(inset))
                        .border(px(ring_width))
                        .border_color(ring_color)
                        .rounded(gpui_kit::rems(
                            (radius - ring_offset - ring_width).max(0.0) / 16.0,
                        )),
                )
            })
    }
}

impl RenderOnce for YueRadio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();

        // The ring belongs to the control rather than to the row around it, so the
        // focus handle is Yue's own — or the group's, when one drives roving focus
        // through the option. `last_input_was_keyboard` is what GPUI's own
        // `focus_visible` condition uses, so a mouse click leaves it unringed.
        let focus_handle = match self.focus_handle.as_ref() {
            Some(handle) => handle.clone(),
            None => window
                .use_keyed_state((id.clone(), FOCUS_SLOT), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone(),
        };
        let ring_visible =
            !self.disabled && focus_handle.is_focused(window) && window.last_input_was_keyboard();

        let pointer = (!self.disabled).then(|| {
            window.use_keyed_state((id.clone(), "yue-radio-pointer"), cx, |_, _| {
                Pointer::default()
            })
        });
        let (hovered, pressed, pointer_state) = match pointer.as_ref() {
            None => (false, false, None),
            Some(state) => {
                let current = state.read(cx);
                (current.hovered, current.pressed, Some(state.clone()))
            }
        };

        let frame = RadioFrame {
            id,
            name: self.accessible_name(),
            focus: &focus_handle,
            ring_visible,
            hovered,
            pressed,
        };
        let mut radio = match self.appearance {
            RadioAppearance::Indicator => self.indicator(&frame, window, cx),
            RadioAppearance::Button => self.segment_row(&frame, window, cx),
        };

        if let Some(state) = pointer_state {
            let hover = window.listener_for(&state, |state, hovered, window, cx| {
                if state.hovered != *hovered {
                    state.hovered = *hovered;
                    cx.notify();
                    window.refresh();
                }
            });
            let down = window.listener_for(&state, |state, _, window, cx| {
                if !state.pressed {
                    state.pressed = true;
                    cx.notify();
                    window.refresh();
                }
            });
            let up = window.listener_for(&state, |state, _, window, cx| {
                if state.pressed {
                    state.pressed = false;
                    cx.notify();
                    window.refresh();
                }
            });
            let up_out = window.listener_for(&state, |state, _, window, cx| {
                if state.pressed {
                    state.pressed = false;
                    state.hovered = false;
                    cx.notify();
                    window.refresh();
                }
            });
            radio = radio
                .on_hover(hover)
                .on_mouse_down(MouseButton::Left, down)
                .on_mouse_up(MouseButton::Left, up)
                .on_mouse_up_out(MouseButton::Left, up_out);
        }
        if let Some(handler) = self.on_change {
            radio = radio.on_change(move |checked, event, window, cx| {
                handler(checked, event, window, cx);
            });
        }
        radio
    }
}

impl RenderOnce for YueRadioGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = rems(self.theme, "--radio-group-gap");
        let segmented = self.appearance == RadioAppearance::Button;
        let vertical = self.direction == GroupDirection::Vertical;
        let group_disabled = self.disabled;
        let selected = self.selected.clone();
        let count = self.options.len();
        let stop = tab_stop_index(&self.options, selected.as_ref(), group_disabled);

        // The options a Tab or an arrow key can reach, in visual order. The group
        // holds their handles because it, not the rows, decides where a move goes,
        // and reads them back to tell which option is focused.
        let mut operable: Vec<(SharedString, FocusHandle)> = Vec::new();
        let mut rows = Vec::with_capacity(count);
        for (index, option) in self.options.iter().enumerate() {
            let option_id = ElementId::from((self.id.clone(), option.value.clone()));
            let focus = if group_disabled || option.disabled {
                None
            } else {
                Some(
                    window
                        .use_keyed_state((option_id.clone(), FOCUS_SLOT), cx, |_, cx| {
                            cx.focus_handle()
                        })
                        .read(cx)
                        .clone(),
                )
            };
            if let Some(handle) = focus.as_ref() {
                operable.push((option.value.clone(), handle.clone()));
            }
            let mut radio = YueRadio::new(
                option_id,
                self.theme,
                Some(&option.value) == selected.as_ref(),
            )
            .size(self.size)
            .appearance(self.appearance)
            .disabled(group_disabled || option.disabled)
            .label(option.label.clone())
            .tab_stop(stop == Some(index))
            .set_position(index + 1, count);
            if segmented {
                radio = radio.segment(vertical, index == 0);
            }
            if let Some(handle) = focus {
                radio = radio.track_focus(&handle);
            }
            if let Some(handler) = self.on_change.as_ref() {
                let handler = Rc::clone(handler);
                let value = option.value.clone();
                radio = radio.on_change(move |_, _, window, cx| {
                    handler(value.clone(), window, cx);
                });
            }
            rows.push(radio);
        }

        // The container is Yue's own div rather than Base's RadioGroup, because the
        // group has to be an observable node: it carries the name and the axis a
        // screen reader announces before the options, and it is the element that
        // receives the arrow keys while an option inside it holds the focus.
        let mut group = div()
            .id((self.id.clone(), "yue-radio-group"))
            .test_support()
            .role(Role::RadioGroup)
            .aria_orientation(if vertical {
                gpui_kit::accesskit::Orientation::Vertical
            } else {
                gpui_kit::accesskit::Orientation::Horizontal
            })
            .flex()
            .flex_col()
            .items_start()
            .gap(gap);
        // The name is carried by the container as well as by the visible caption,
        // so a screen reader announces the group before its options.
        group = group.when_some(self.label.clone(), |group, label| {
            group.aria_label(label.clone()).child(
                div()
                    .text_size(rems(self.theme, "--radio-label-font-size-md"))
                    .text_color(rgba(self.theme, "--radio-label-color"))
                    .child(label),
            )
        });
        if let Some(handler) = self.on_change.as_ref() {
            let handler = Rc::clone(handler);
            let operable = operable.clone();
            group = group.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let Some(direction) = move_for(&event.keystroke.key, vertical) else {
                    return;
                };
                let Some(current) = operable
                    .iter()
                    .position(|(_, focus)| focus.is_focused(window))
                else {
                    return;
                };
                let Some(target) = move_index(operable.len(), current, direction) else {
                    return;
                };
                let (value, focus) = &operable[target];
                focus.focus(window, cx);
                // A radio follows its focus: the option an arrow key lands on is
                // chosen, the way a native group behaves.
                handler(value.clone(), window, cx);
                // The keys belong to the group; an enclosing scroll view must not
                // also treat them as a scroll.
                cx.stop_propagation();
            });
        }

        group.child(
            div()
                .flex()
                .when(vertical, |rows| rows.flex_col())
                .when(!vertical, |rows| rows.flex_row())
                .when(segmented, |track| {
                    // One continuous outline, drawn once around the row and clipped
                    // so only its ends show a radius; the segments between them
                    // carry the shared dividers instead of gaps. A wrapped segment
                    // row would no longer read as one choice set, so it never
                    // wraps — long labels belong to the Indicator appearance.
                    track
                        .gap(gpui_kit::rems(0.0))
                        .items_stretch()
                        .border(px(pixels(self.theme, "--radio-button-border-width")))
                        .border_color(rgba(self.theme, "--radio-button-border-color"))
                        .rounded(gpui_kit::rems(
                            pixels(self.theme, "--radio-button-border-radius") / 16.0,
                        ))
                        .overflow_hidden()
                })
                .when(!segmented, |rows| rows.gap(gap).items_start())
                .when(!segmented && !vertical, |rows| rows.flex_wrap())
                .children(rows),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        RadioAppearance, RadioMove, RadioSize, YueRadio, YueRadioGroup, YueRadioOption, move_for,
        move_index, ring_inset, segment_tokens, size_tokens, tab_stop_index, tint,
    };
    use crate::tokens::{duration, easing, number, pixels, rgba};
    use crate::{Accent, ThemeMode, YueTheme};
    use gpui_kit::SharedString;

    /// Every Yue profile a colour test has to walk.
    const PROFILES: [(ThemeMode, Accent); 4] = [
        (ThemeMode::Light, Accent::Azure),
        (ThemeMode::Dark, Accent::Azure),
        (ThemeMode::Light, Accent::Neutral),
        (ThemeMode::Dark, Accent::Neutral),
    ];

    /// How far two colours sit apart, counted the way the RGB mix travels.
    fn gap(a: gpui_kit::Rgba, b: gpui_kit::Rgba) -> f32 {
        (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
    }

    /// A Yue easing token has to start on the current value, land on the target,
    /// and travel between them without rewinding or overshooting.
    fn ramp_is_sane(theme: YueTheme, name: &'static str) {
        let curve = easing(theme, name);
        assert_eq!(curve(0.0), 0.0, "{name} starts at the current value");
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

    fn option(value: &str, disabled: bool) -> YueRadioOption {
        let option = YueRadioOption::new(value, value);
        if disabled {
            option.disabled(true)
        } else {
            option
        }
    }

    fn value(value: &str) -> SharedString {
        SharedString::from(value)
    }

    #[test]
    fn radio_needs_a_name_from_text_or_label() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(
            YueRadio::new("a", theme, false)
                .label("PNG")
                .accessible_name(),
            "PNG"
        );
        assert_eq!(
            YueRadio::new("b", theme, true)
                .accessibility_label("Compact")
                .accessible_name(),
            "Compact",
            "a text-less circle is still named"
        );
    }

    #[test]
    #[should_panic(expected = "YueRadio needs a label")]
    fn a_nameless_radio_is_rejected() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let radio = YueRadio::new("c", theme, false);
        let _ = radio.accessible_name();
    }

    #[test]
    fn tab_lands_on_the_selected_option_and_falls_back_to_the_first() {
        let options = [
            option("png", false),
            option("svg", false),
            option("pdf", false),
        ];
        assert_eq!(
            tab_stop_index(&options, Some(&value("svg")), false),
            Some(1),
            "the chosen option is the one Tab reaches"
        );
        assert_eq!(
            tab_stop_index(&options, None, false),
            Some(0),
            "an empty selection still leaves the group enterable"
        );
        assert_eq!(
            tab_stop_index(&options, Some(&value("removed")), false),
            Some(0),
            "a value the group no longer offers falls back to the first option"
        );

        let with_disabled = [
            option("png", true),
            option("svg", false),
            option("pdf", false),
        ];
        assert_eq!(
            tab_stop_index(&with_disabled, Some(&value("png")), false),
            Some(1),
            "a disabled option cannot be the tab stop even while it holds the value"
        );
        assert_eq!(
            tab_stop_index(&[option("png", true)], Some(&value("png")), false),
            None,
            "an all-disabled group puts nothing in the tab order"
        );
        assert_eq!(
            tab_stop_index(&[], None, false),
            None,
            "an empty group has no stop"
        );
        assert_eq!(
            tab_stop_index(&options, Some(&value("png")), true),
            None,
            "a disabled group puts nothing in the tab order"
        );
    }

    #[test]
    fn arrow_keys_move_only_along_the_groups_own_axis() {
        assert_eq!(move_for("down", true), Some(RadioMove::Next));
        assert_eq!(move_for("up", true), Some(RadioMove::Previous));
        assert_eq!(move_for("right", false), Some(RadioMove::Next));
        assert_eq!(move_for("left", false), Some(RadioMove::Previous));
        for key in ["up", "down"] {
            assert_eq!(move_for(key, false), None, "{key} is not a row's move");
        }
        for key in ["left", "right"] {
            assert_eq!(move_for(key, true), None, "{key} is not a column's move");
        }
        for vertical in [true, false] {
            assert_eq!(move_for("home", vertical), Some(RadioMove::First));
            assert_eq!(move_for("end", vertical), Some(RadioMove::Last));
        }
        for key in ["space", "enter", "tab", "a"] {
            assert_eq!(move_for(key, true), None, "{key} is left to the row");
            assert_eq!(move_for(key, false), None, "{key} is left to the row");
        }
    }

    #[test]
    fn moves_walk_the_operable_options_and_wrap() {
        // Three operable options, with the disabled ones already dropped from the
        // count, so every move skips them by construction.
        assert_eq!(
            move_index(3, 0, RadioMove::Next),
            Some(1),
            "forward goes to the next option"
        );
        assert_eq!(
            move_index(3, 2, RadioMove::Next),
            Some(0),
            "past the last option wraps to the first"
        );
        assert_eq!(
            move_index(3, 0, RadioMove::Previous),
            Some(2),
            "before the first option wraps to the last"
        );
        assert_eq!(move_index(3, 1, RadioMove::First), Some(0));
        assert_eq!(move_index(3, 0, RadioMove::Last), Some(2));
        assert_eq!(
            move_index(1, 0, RadioMove::Next),
            Some(0),
            "a single option stays where it is"
        );
        for direction in [
            RadioMove::Next,
            RadioMove::Previous,
            RadioMove::First,
            RadioMove::Last,
        ] {
            assert_eq!(
                move_index(0, 0, direction),
                None,
                "{direction:?} has nowhere to go in an empty group"
            );
        }
    }

    #[test]
    #[should_panic(expected = "option values must be unique")]
    fn duplicate_option_values_are_rejected() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let _ = YueRadioGroup::new("format", theme, None).options([
            YueRadioOption::new("png", "PNG"),
            YueRadioOption::new("png", "PNG（重复）"),
        ]);
    }

    #[test]
    fn size_profiles_pair_each_circle_with_its_row_and_label() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        for (size, box_size, height, font) in [
            (RadioSize::Small, 12.0, 28.0, 12.0),
            (RadioSize::Medium, 16.0, 32.0, 14.0),
            (RadioSize::Large, 20.0, 40.0, 16.0),
        ] {
            let (box_token, dot_token, height_token, font_token) = size_tokens(size);
            assert_eq!(pixels(theme, box_token), box_size, "{box_token}");
            assert_eq!(pixels(theme, height_token), height, "{height_token}");
            assert_eq!(pixels(theme, font_token), font, "{font_token}");
            assert!(
                box_size < height,
                "{size:?} keeps its circle inside its row"
            );
            assert!(
                pixels(theme, dot_token) < box_size - 2.0 * pixels(theme, "--radio-border-width"),
                "{size:?} keeps the dot inside the circle it sits in"
            );
        }
    }

    #[test]
    fn the_dot_keeps_the_reference_geometry_at_every_size() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let border = pixels(theme, "--radio-border-width");
        for (size, box_size) in [
            (RadioSize::Small, 12.0),
            (RadioSize::Medium, 16.0),
            (RadioSize::Large, 20.0),
        ] {
            let (_, dot_token, _, _) = size_tokens(size);
            let dot = pixels(theme, dot_token);
            assert_eq!(
                dot,
                box_size * 0.375,
                "{size:?} keeps the reference's 6-in-16 dot ratio"
            );
            assert!(
                dot > 0.0 && dot < box_size - 2.0 * border,
                "{size:?} keeps the dot inside the content box it centres in"
            );
            // The reference places that dot with `inset: var(--space-4)` on a 16px
            // box, which is what the 0.375 ratio works out to once the border is
            // paid for; the same ratio at other rungs scales the inset with it.
            if size == RadioSize::Medium {
                let inset = (box_size - 2.0 * border - dot) / 2.0;
                assert_eq!(inset, 4.0, "Medium is the reference's own 4px inset");
            }
        }
    }

    #[test]
    fn radio_motion_resolves_the_reference_rungs() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(
            duration(theme, "--radio-duration"),
            Duration::from_millis(200),
            "both the circle and the dot change state on the enter rung"
        );
        assert_eq!(
            duration(theme, "--radio-duration-fast"),
            Duration::from_millis(100),
            "the press shrinks on the exit rung"
        );
        assert_eq!(
            number(theme, "--radio-press-scale"),
            0.9,
            "the press scales the circle the way the reference does"
        );
        assert_eq!(
            pixels(theme, "--radio-halo-width"),
            4.0,
            "the hover halo reaches 4px past the circle"
        );
        for name in ["--radio-ease", "--radio-ease-enter", "--radio-ease-exit"] {
            ramp_is_sane(theme, name);
        }
    }

    #[test]
    fn radio_colours_come_from_yue_roles_in_every_profile() {
        for (mode, accent) in PROFILES {
            let theme = YueTheme::new(mode, accent);
            let checked = rgba(theme, "--radio-background-checked");
            let dot = rgba(theme, "--radio-dot-color");
            assert!(
                checked.a > 0.99,
                "{mode:?}/{accent:?} fills the circle opaquely"
            );
            assert!(dot.a > 0.99, "{mode:?}/{accent:?} keeps a solid dot");
            // The dot is painted on the accent fill, never on the surface: at
            // Medium the unchecked circle and the dot are the same colour in the
            // light profiles, so a dot on the surface would read as nothing.
            assert!(
                gap(checked, dot) > 0.25,
                "{mode:?}/{accent:?} keeps the dot legible on the fill"
            );
            assert!(
                rgba(theme, "--radio-disabled-border-color").a == 0.0,
                "{mode:?}/{accent:?} drops the disabled border out, as the reference does"
            );
            assert!(
                rgba(theme, "--radio-disabled-dot-color").a < dot.a,
                "{mode:?}/{accent:?} dims a disabled dot below the live one"
            );
            let halo = rgba(theme, "--radio-halo-color");
            assert!(
                halo.a > 0.0 && halo.a < 0.5,
                "{mode:?}/{accent:?} keeps the hover halo a tint, not a fill"
            );
        }
    }

    #[test]
    fn an_option_defaults_to_the_circle() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(RadioAppearance::default(), RadioAppearance::Indicator);
        let lone = YueRadio::new("png", theme, false);
        assert_eq!(lone.appearance, RadioAppearance::Indicator);
        assert!(
            lone.segment.is_none(),
            "an option nobody has placed in a row outlines itself"
        );
        assert_eq!(
            YueRadioGroup::new("format", theme, None).appearance,
            RadioAppearance::Indicator
        );
    }

    #[test]
    fn segment_sizes_map_the_control_ladder() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let mut previous = (0.0, 0.0, 0.0);
        for (size, rung) in [
            (RadioSize::Small, (28.0, 12.0, 8.0)),
            (RadioSize::Medium, (32.0, 14.0, 12.0)),
            (RadioSize::Large, (40.0, 16.0, 16.0)),
        ] {
            let (height_token, font_token, padding_token) = segment_tokens(size);
            let actual = (
                pixels(theme, height_token),
                pixels(theme, font_token),
                pixels(theme, padding_token),
            );
            assert_eq!(actual, rung, "{size:?} reads the control ladder");
            assert!(
                actual.1 < actual.0,
                "{size:?} keeps its label inside its row"
            );
            assert!(
                actual.0 > previous.0 && actual.2 >= previous.2,
                "{size:?} grows the row and never shrinks its padding"
            );
            previous = actual;
        }
    }

    #[test]
    fn the_segment_ring_stays_inside_its_own_row() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let border = pixels(theme, "--radio-button-border-width");
        let offset = pixels(theme, "--radio-focus-ring-offset");
        let width = pixels(theme, "--radio-focus-ring-width");
        // The row clips its ends, so the ring is drawn inside the segment. It has
        // to clear the divider the segment shares with its neighbour, and still
        // leave ink on the row rather than past it.
        let inset = ring_inset(offset, border);
        assert!(inset >= 0.0, "the ring never reaches over the divider");
        for size in [RadioSize::Small, RadioSize::Medium, RadioSize::Large] {
            let (height_token, _, _) = segment_tokens(size);
            assert!(
                inset + 2.0 * width <= pixels(theme, height_token),
                "{size:?} keeps the whole ring inside the row it rings"
            );
        }
    }

    #[test]
    fn a_hover_overlay_tints_its_fill_instead_of_punching_through_it() {
        for (mode, accent) in PROFILES {
            let theme = YueTheme::new(mode, accent);
            let surface = rgba(theme, "--radio-button-background");
            let overlay = rgba(theme, "--radio-button-hover-background");
            assert!(
                surface.a > 0.99,
                "{mode:?}/{accent:?} paints the row on an opaque surface"
            );
            assert!(
                overlay.a > 0.0 && overlay.a < 0.5,
                "{mode:?}/{accent:?} keeps hover a tint rather than a fill"
            );
            assert_eq!(
                tint(surface, overlay, 0.0),
                surface,
                "{mode:?}/{accent:?} leaves an unhovered row alone"
            );
            let hovered = tint(surface, overlay, 1.0);
            assert_eq!(
                hovered.a, surface.a,
                "{mode:?}/{accent:?} never turns a hovered segment see-through"
            );
            assert_ne!(
                hovered, surface,
                "{mode:?}/{accent:?} shows the hover at all"
            );
            assert!(
                gap(surface, hovered) > gap(hovered, tint(surface, overlay, 0.5)),
                "{mode:?}/{accent:?} travels the same axis at every rung"
            );
        }
    }

    #[test]
    fn a_selected_segment_borrows_the_selection_roles() {
        for (mode, accent) in PROFILES {
            let theme = YueTheme::new(mode, accent);
            // The chosen row is filled with the system's own selection tint, never
            // with a pressed button's: a radio's checked state is a fact about the
            // value, not about the pointer.
            assert_eq!(
                rgba(theme, "--radio-button-background-selected"),
                rgba(theme, "--selected-background"),
                "{mode:?}/{accent:?} fills with the selected role"
            );
            assert_eq!(
                rgba(theme, "--radio-button-color-selected"),
                rgba(theme, "--selected-color"),
                "{mode:?}/{accent:?} writes the chosen label in the selected role"
            );
            let surface = rgba(theme, "--radio-button-background");
            let chosen = rgba(theme, "--radio-button-background-selected");
            assert!(
                gap(chosen, surface) > 0.02,
                "{mode:?}/{accent:?} shows the choice without a border change"
            );
            assert!(
                gap(rgba(theme, "--radio-button-color-selected"), chosen) > 0.5,
                "{mode:?}/{accent:?} keeps the chosen label legible on the chosen fill"
            );
            assert_eq!(
                rgba(theme, "--radio-button-color-hover"),
                rgba(theme, "--radio-label-color"),
                "{mode:?}/{accent:?} lifts a hovered segment to the text a hovered circle uses"
            );
            let quiet = rgba(theme, "--radio-button-color");
            let lift = rgba(theme, "--radio-button-color-hover");
            assert!(
                gap(quiet, surface) < gap(lift, surface),
                "{mode:?}/{accent:?} keeps an unchosen label quieter than a hovered one"
            );
            assert!(
                number(theme, "--radio-button-weight-selected") > 400.0,
                "{mode:?}/{accent:?} weights the chosen label above normal text"
            );
        }
    }

    #[test]
    fn segment_motion_splits_the_choice_from_the_press() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(
            duration(theme, "--radio-button-duration"),
            Duration::from_millis(200),
            "the fill and the label colour arrive on the enter rung"
        );
        assert_eq!(
            duration(theme, "--radio-button-duration-fast"),
            Duration::from_millis(100),
            "the press overlay leaves on the exit rung"
        );
        ramp_is_sane(theme, "--radio-button-ease");
        // Both appearances ride the same two semantic rungs, so a segmented row
        // and a circle never feel like they are on different clocks.
        assert_eq!(
            duration(theme, "--radio-button-duration"),
            duration(theme, "--radio-duration")
        );
        assert_eq!(
            duration(theme, "--radio-button-duration-fast"),
            duration(theme, "--radio-duration-fast")
        );
    }
}
