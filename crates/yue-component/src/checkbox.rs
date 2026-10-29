//! Yue checkbox: one controlled three-state control and a multi-select group.
//!
//! The control keeps GPUI Base's activation, focus and accessibility behaviour
//! and adds Yue's appearance: an accent-filled box whose check and dash marks are
//! drawn as vector strokes, so the mark scales with the box instead of being an
//! icon asset sized against it.

use std::rc::Rc;

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Bounds, ClickEvent, ElementId, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, PathBuilder, PathStyle, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, canvas, div, px,
};
use lyon::tessellation::{LineCap, LineJoin, StrokeOptions};

use crate::tokens::{duration, easing, mix_rgba, number, pixels, rems, rgba};
use crate::{GroupDirection, YueTheme};

type ChangeHandler = Rc<dyn Fn(CheckboxState, &ClickEvent, &mut Window, &mut App)>;
type SelectionHandler = Rc<dyn Fn(Vec<SharedString>, &ClickEvent, &mut Window, &mut App)>;

/// The reference's `viewBox="0 0 16 16"`: the mark coordinates below are in
/// those units, and a drawn box of `16 - 2 * border` pixels maps onto it.
const MARK_BOX: f32 = 16.0;

/// The check: the reference's `d="M4 8.5l3 3 5-6"`.
const CHECK_LINE: [(f32, f32); 3] = [(4.0, 8.5), (7.0, 11.5), (12.0, 5.5)];

/// The indeterminate dash: the reference's `d="M4.5 8h7"`.
const DASH_LINE: [(f32, f32); 2] = [(4.5, 8.0), (11.5, 8.0)];

/// Which selection a checkbox reports.
///
/// This is Yue's own type rather than the base crate's so a consumer reads its
/// value without naming a `gpui-base` module.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckboxState {
    #[default]
    Unchecked,
    Checked,
    /// Part of what the control stands for is selected. Activating it asks for
    /// [`CheckboxState::Checked`] rather than clearing the selection.
    Indeterminate,
}

impl CheckboxState {
    fn to_base(self) -> gpui_kit::base::CheckboxState {
        match self {
            Self::Unchecked => gpui_kit::base::CheckboxState::Unchecked,
            Self::Checked => gpui_kit::base::CheckboxState::Checked,
            Self::Indeterminate => gpui_kit::base::CheckboxState::Indeterminate,
        }
    }

    fn from_base(state: gpui_kit::base::CheckboxState) -> Self {
        match state {
            gpui_kit::base::CheckboxState::Unchecked => Self::Unchecked,
            gpui_kit::base::CheckboxState::Checked => Self::Checked,
            gpui_kit::base::CheckboxState::Indeterminate => Self::Indeterminate,
        }
    }

    /// Whether the box takes the accent fill: the reference fills an
    /// indeterminate box exactly like a checked one and only the mark differs.
    fn chosen(self) -> bool {
        self != Self::Unchecked
    }
}

/// Three Yue checkbox size profiles.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckboxSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// A controlled Yue checkbox. The caller owns and updates the state.
#[derive(IntoElement)]
pub struct YueCheckbox {
    id: ElementId,
    theme: YueTheme,
    state: CheckboxState,
    disabled: bool,
    size: CheckboxSize,
    label: Option<SharedString>,
    accessibility_label: Option<SharedString>,
    on_change: Option<ChangeHandler>,
}

impl YueCheckbox {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme, state: CheckboxState) -> Self {
        Self {
            id: id.into(),
            theme,
            state,
            disabled: false,
            size: CheckboxSize::Medium,
            label: None,
            accessibility_label: None,
            on_change: None,
        }
    }

    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    /// Visible text beside the box. The whole row — box and label — is the click
    /// target, and the text names the control in the accessibility tree.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Names a checkbox that carries no visible text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reports the state the control should move to: `Unchecked` or
    /// `Indeterminate` become `Checked` and `Checked` becomes `Unchecked`. The
    /// caller writes the new value back; the control keeps no second copy.
    pub fn on_change(
        mut self,
        handler: impl Fn(CheckboxState, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn accessible_name(&self) -> SharedString {
        match (&self.label, &self.accessibility_label) {
            (Some(label), _) | (None, Some(label)) => label.clone(),
            (None, None) => {
                panic!("YueCheckbox needs a label, or an accessibility_label without visible text")
            }
        }
    }
}

/// One option of a [`YueCheckboxGroup`]: a stable value, its visible text, and
/// whether it can currently be changed.
#[derive(Clone)]
pub struct YueCheckboxOption {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl YueCheckboxOption {
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

/// A multi-select group: the caller holds the selected values, the group edits
/// them one option at a time or through a derived select-all header.
#[derive(IntoElement)]
pub struct YueCheckboxGroup {
    id: ElementId,
    theme: YueTheme,
    selected: Vec<SharedString>,
    options: Vec<YueCheckboxOption>,
    label: Option<SharedString>,
    select_all: Option<SharedString>,
    disabled: bool,
    size: CheckboxSize,
    direction: GroupDirection,
    on_change: Option<SelectionHandler>,
}

impl YueCheckboxGroup {
    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        selected: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            theme,
            selected: selected.into_iter().map(Into::into).collect(),
            options: Vec::new(),
            label: None,
            select_all: None,
            disabled: false,
            size: CheckboxSize::Medium,
            direction: GroupDirection::Vertical,
            on_change: None,
        }
    }

    /// The options offered. Values must be unique: they are the identity the
    /// selection is stored against, and a repeated value would make two rows
    /// write the same entry.
    pub fn options(mut self, options: impl IntoIterator<Item = YueCheckboxOption>) -> Self {
        self.options = options.into_iter().collect();
        let mut seen: Vec<SharedString> = Vec::with_capacity(self.options.len());
        for option in &self.options {
            assert!(
                !seen.contains(&option.value),
                "YueCheckboxGroup option values must be unique, found {:?} twice",
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

    /// Add a header row above the options. Its state is never stored: it is
    /// derived from the options, and clicking it selects or clears every option
    /// that is not individually disabled. The text is the header's visible label
    /// and its accessible name.
    pub fn select_all(mut self, label: impl Into<SharedString>) -> Self {
        self.select_all = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    pub fn direction(mut self, direction: GroupDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Reports the whole new selection, not a delta, so the caller writes one
    /// value back per change.
    pub fn on_change(
        mut self,
        handler: impl Fn(Vec<SharedString>, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// The select-all header's state, given the options and the current selection.
///
/// `E` is the set of options a click on the header can still change — every
/// option that is not individually disabled — and `S` the selection. No eligible
/// option is selected, or there are none at all, leaves the header unchecked; a
/// selection that covers all of `E` checks it; anything in between is the mixed
/// state. Disabled options never decide the header, whether or not they are
/// selected.
fn select_all_state(options: &[YueCheckboxOption], selected: &[SharedString]) -> CheckboxState {
    let eligible = eligible_values(options);
    if eligible.is_empty() || eligible.iter().all(|value| !selected.contains(value)) {
        CheckboxState::Unchecked
    } else if eligible.iter().all(|value| selected.contains(value)) {
        CheckboxState::Checked
    } else {
        CheckboxState::Indeterminate
    }
}

/// The values a header click is allowed to change.
///
/// A group disabled as a whole still reports its options through the header: the
/// group flag makes the header inert rather than rewriting what it says, so this
/// looks only at the options' own `disabled`.
fn eligible_values(options: &[YueCheckboxOption]) -> Vec<SharedString> {
    options
        .iter()
        .filter(|option| !option.disabled)
        .map(|option| option.value.clone())
        .collect()
}

/// Applying the header: checking moves to `S ∪ E` and clearing to `S − E`, so a
/// disabled option keeps whatever value it had and the caller's own order of the
/// selection is preserved. Values the caller selected that are not offered by
/// this group pass through untouched.
fn apply_select_all(
    options: &[YueCheckboxOption],
    selected: &[SharedString],
    checked: bool,
) -> Vec<SharedString> {
    let eligible = eligible_values(options);
    if checked {
        let mut next = selected.to_vec();
        next.extend(
            eligible
                .into_iter()
                .filter(|value| !next.contains(value))
                .collect::<Vec<_>>(),
        );
        next
    } else {
        selected
            .iter()
            .filter(|value| !eligible.contains(value))
            .cloned()
            .collect()
    }
}

/// Applying one option: the value enters or leaves the selection in place.
fn apply_option(
    selected: &[SharedString],
    value: &SharedString,
    state: CheckboxState,
) -> Vec<SharedString> {
    let mut next = selected.to_vec();
    if state.chosen() {
        if !next.contains(value) {
            next.push(value.clone());
        }
    } else {
        next.retain(|entry| entry != value);
    }
    next
}

fn size_tokens(size: CheckboxSize) -> (&'static str, &'static str, &'static str, &'static str) {
    match size {
        CheckboxSize::Small => (
            "--checkbox-size-sm",
            "--checkbox-height-sm",
            "--checkbox-label-font-size-sm",
            "--checkbox-mark-stroke-width-sm",
        ),
        CheckboxSize::Medium => (
            "--checkbox-size-md",
            "--checkbox-height-md",
            "--checkbox-label-font-size-md",
            "--checkbox-mark-stroke-width-md",
        ),
        CheckboxSize::Large => (
            "--checkbox-size-lg",
            "--checkbox-height-lg",
            "--checkbox-label-font-size-lg",
            "--checkbox-mark-stroke-width-lg",
        ),
    }
}

/// The pointer states that drive the hover halo and the press shrink. Kept as one
/// piece of keyed state so a move or a release costs a single repaint.
#[derive(Clone, Copy, Default)]
struct Pointer {
    hovered: bool,
    pressed: bool,
}

/// One mark of the box, inked to `ink` of its own length.
///
/// The canvas fills the box's content area, which is where the reference places
/// its `position: absolute; inset: 0` SVG, so the 16-unit path scales with the
/// box and the press shrink moves the mark with it.
fn mark_layer(
    points: &'static [(f32, f32)],
    ink: f32,
    unit: f32,
    stroke: f32,
    color: gpui_kit::Hsla,
) -> impl IntoElement {
    canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| paint_mark(window, bounds, points, ink, unit, stroke, color),
    )
    .absolute()
    .inset_0()
}

fn paint_mark(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    points: &[(f32, f32)],
    ink: f32,
    unit: f32,
    stroke: f32,
    color: gpui_kit::Hsla,
) {
    if ink <= 0.0 {
        return;
    }
    let centre = bounds.center();
    let place = |point: (f32, f32)| {
        gpui_kit::point(
            centre.x + px((point.0 - MARK_BOX / 2.0) * unit),
            centre.y + px((point.1 - MARK_BOX / 2.0) * unit),
        )
    };
    let mut length = 0.0;
    for pair in points.windows(2) {
        length += ((pair[1].0 - pair[0].0).powi(2) + (pair[1].1 - pair[0].1).powi(2)).sqrt();
    }
    let length = length * unit;
    let mut builder = PathBuilder::default().with_style(PathStyle::Stroke(
        StrokeOptions::default()
            .with_line_width(stroke)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round),
    ));
    builder.move_to(place(points[0]));
    for point in &points[1..] {
        builder.line_to(place(*point));
    }
    // `pathLength="1"` with `stroke-dasharray: 1` and a dash offset running 1 → 0
    // inks the leading `ink` of the stroke, so the mark is written from the end
    // its path starts at, the way the reference draws it.
    let dashes = [px(length * ink.clamp(0.0, 1.0)), px(length)];
    if let Ok(path) = builder.dash_array(&dashes).build() {
        window.paint_path(path, color);
    }
}

impl RenderOnce for YueCheckbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (box_token, height_token, font_token, stroke_token) = size_tokens(self.size);
        let box_size = pixels(self.theme, box_token);
        let border = pixels(self.theme, "--checkbox-border-width");
        let radius = pixels(self.theme, "--checkbox-border-radius");
        let halo_width = pixels(self.theme, "--checkbox-halo-width");
        let halo_color = rgba(self.theme, "--checkbox-halo-color");
        let ring_width = pixels(self.theme, "--checkbox-focus-ring-width");
        let ring_offset = pixels(self.theme, "--checkbox-focus-ring-offset");
        let ring_color = rgba(self.theme, "--checkbox-focus-ring-color");
        let mark_stroke = pixels(self.theme, stroke_token);
        let chosen = self.state.chosen();
        let id = self.id.clone();

        // The ring belongs to the box, not to the row, so the focus handle is
        // Yue's own: the base control is told to track it and the ring reads it
        // back. `last_input_was_keyboard` is what GPUI's own `focus_visible`
        // condition uses, so a mouse click leaves the box unringed.
        let focus_handle = window
            .use_keyed_state((id.clone(), "yue-checkbox-focus"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let ring_visible =
            !self.disabled && focus_handle.is_focused(window) && window.last_input_was_keyboard();

        let pointer = (!self.disabled).then(|| {
            window.use_keyed_state((id.clone(), "yue-checkbox-pointer"), cx, |_, _| {
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

        let standard = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--checkbox-duration"))
                .ease(easing(theme, "--checkbox-ease"))
        };
        // The mark is written in only once the box has finished filling, and is
        // wiped off on the short rung before the fill has gone.
        let ink = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--checkbox-duration"))
                .delay(duration(theme, "--checkbox-duration-fast"))
                .ease(easing(theme, "--checkbox-ease-enter"))
        };
        let erase = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--checkbox-duration-fast"))
                .ease(easing(theme, "--checkbox-ease-exit"))
        };
        // The box shrinks on the short rung, and the shrink is eased the same way
        // as the fill rather than on its own curve.
        let press = |theme: YueTheme| {
            gpui_kit::base::Transition::new(duration(theme, "--checkbox-duration-fast"))
                .ease(easing(theme, "--checkbox-ease"))
        };

        let mut progress = |channel: &'static str, target: f32, policy| {
            gpui_kit::base::transition(
                (id.clone(), format!("yue-checkbox-{channel}")),
                target,
                policy,
                window,
                cx,
            )
            .clamp(0.0, 1.0)
        };
        let fill = progress("fill", f32::from(chosen), standard(self.theme));
        let emphasis = progress("border", f32::from(hovered || chosen), standard(self.theme));
        let halo = progress("halo", f32::from(hovered), standard(self.theme));
        let scale = gpui_kit::base::transition(
            (id.clone(), "yue-checkbox-press"),
            if pressed {
                number(self.theme, "--checkbox-press-scale")
            } else {
                1.0
            },
            press(self.theme),
            window,
            cx,
        );
        let check_inked = gpui_kit::base::transition(
            (id.clone(), "yue-checkbox-check"),
            f32::from(self.state == CheckboxState::Checked),
            if self.state == CheckboxState::Checked {
                ink(self.theme)
            } else {
                erase(self.theme)
            },
            window,
            cx,
        )
        .clamp(0.0, 1.0);
        let dash_inked = gpui_kit::base::transition(
            (id.clone(), "yue-checkbox-dash"),
            f32::from(self.state == CheckboxState::Indeterminate),
            if self.state == CheckboxState::Indeterminate {
                ink(self.theme)
            } else {
                erase(self.theme)
            },
            window,
            cx,
        )
        .clamp(0.0, 1.0);

        // A disabled box is repainted rather than transitioned, the way the
        // reference's own disabled rule replaces the fill outright.
        let background = if self.disabled {
            rgba(self.theme, "--checkbox-disabled-background")
        } else {
            mix_rgba(
                rgba(self.theme, "--checkbox-background"),
                rgba(self.theme, "--checkbox-background-checked"),
                fill,
            )
        };
        let border_color = if self.disabled {
            rgba(self.theme, "--checkbox-disabled-border-color")
        } else {
            mix_rgba(
                rgba(self.theme, "--checkbox-border-color"),
                rgba(self.theme, "--checkbox-border-color-checked"),
                emphasis,
            )
        };
        let mark_color = gpui_kit::Hsla::from(rgba(
            self.theme,
            if self.disabled {
                "--checkbox-disabled-mark-color"
            } else {
                "--checkbox-mark-color"
            },
        ));
        // The mark is painted on the box's content area, so its unit is that
        // area's share of the reference's 16-unit box. Its stroke is not: the
        // weight is the size's own pixel rung, shared with the switch's glyphs, so
        // a mark keeps that weight wherever the box scales it and only the press
        // shrinks it.
        let inner = box_size * scale;
        let content = inner - 2.0 * border * scale;
        let unit = content / MARK_BOX;
        let stroke = mark_stroke * scale;
        let margin = (box_size - inner) / 2.0;
        // `outline-offset` measures from the border box, and GPUI offsets an
        // absolute child from the content box, so the border is one leg of the
        // distance. Every part of the ring shrinks with the box it rings.
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
                // so hovering never moves the box or the label beside it.
                layer.shadow(vec![
                    gpui_kit::BoxShadow::new(
                        px(0.0),
                        px(0.0),
                        gpui_kit::Hsla::from(halo_color).opacity(halo * halo_color.a),
                    )
                    .spread_radius(px(halo_width * halo)),
                ])
            })
            .child(mark_layer(
                &CHECK_LINE,
                check_inked,
                unit,
                stroke,
                mark_color,
            ))
            .child(mark_layer(&DASH_LINE, dash_inked, unit, stroke, mark_color))
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

        let mut checkbox = gpui_kit::base::Checkbox::new(id)
            .state(self.state.to_base())
            .disabled(self.disabled)
            .accessibility_label(self.accessible_name())
            .track_focus(&focus_handle)
            .flex()
            .items_center()
            .gap(rems(self.theme, "--checkbox-label-gap"))
            .min_h(rems(self.theme, height_token))
            .text_size(rems(self.theme, font_token))
            .text_color(rgba(
                self.theme,
                if self.disabled {
                    "--checkbox-label-color-disabled"
                } else {
                    "--checkbox-label-color"
                },
            ))
            .when(!self.disabled, |checkbox| checkbox.cursor_pointer())
            // A fixed slot holds the box, so shrinking it on press moves nothing
            // else in the row — the reference's `transform: scale(.9)` never
            // reflows either.
            .child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .w(px(box_size))
                    .h(px(box_size))
                    .child(box_layer),
            );
        if let Some(label) = self.label {
            checkbox = checkbox.child(label);
        }

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
            checkbox = checkbox
                .on_hover(hover)
                .on_mouse_down(MouseButton::Left, down)
                .on_mouse_up(MouseButton::Left, up)
                .on_mouse_up_out(MouseButton::Left, up_out);
        }
        if let Some(handler) = self.on_change {
            checkbox = checkbox.on_change(move |state, event, window, cx| {
                handler(CheckboxState::from_base(state), event, window, cx);
            });
        }
        checkbox
    }
}

impl RenderOnce for YueCheckboxGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let gap = rems(self.theme, "--checkbox-group-gap");
        let vertical = self.direction == GroupDirection::Vertical;
        let group_disabled = self.disabled;
        let has_header = self.select_all.is_some();
        let nothing_left_to_change = eligible_values(&self.options).is_empty();

        // Registered for UI queries like the radio group's container: the name and
        // the Group role live here, so a test can see the options sit inside a
        // container the tree can name.
        let mut group = div()
            .id((self.id.clone(), "yue-checkbox-group"))
            .test_support()
            .flex()
            // The container always stacks its own parts — caption, header, options
            // — so a horizontal group keeps them above the row rather than in it.
            // The option direction is the rows container's business below.
            .flex_col()
            .items_start()
            .gap(gap);
        // The name is carried by the container as well as by the visible caption,
        // so a screen reader announces the group before its options.
        group = group
            .role(gpui_kit::Role::Group)
            .when_some(self.label.clone(), |group, label| group.aria_label(label));

        group = group.when_some(self.label.clone(), |group, label| {
            group.child(
                div()
                    .text_size(rems(self.theme, "--checkbox-label-font-size-md"))
                    .text_color(rgba(self.theme, "--checkbox-label-color"))
                    .child(label),
            )
        });

        if let Some(header_label) = self.select_all {
            let state = select_all_state(&self.options, &self.selected);
            let mut header = YueCheckbox::new(
                (self.id.clone(), "yue-checkbox-select-all"),
                self.theme,
                state,
            )
            .size(self.size)
            // With nothing left to change, the header itself cannot be operated.
            .disabled(group_disabled || nothing_left_to_change)
            // The header's text is both its visible label and its accessible name,
            // so the row reads like any other option and still answers to that name.
            .label(header_label);
            if let Some(handler) = self.on_change.as_ref() {
                let handler = Rc::clone(handler);
                let selected = self.selected.clone();
                let options = self.options.clone();
                header = header.on_change(move |next, event, window, cx| {
                    handler(
                        apply_select_all(&options, &selected, next.chosen()),
                        event,
                        window,
                        cx,
                    );
                });
            }
            group = group.child(header);
        }

        let rows = self.options.iter().map(|option| {
            let state = if self.selected.contains(&option.value) {
                CheckboxState::Checked
            } else {
                CheckboxState::Unchecked
            };
            let mut checkbox =
                YueCheckbox::new((self.id.clone(), option.value.clone()), self.theme, state)
                    .size(self.size)
                    .disabled(group_disabled || option.disabled)
                    .label(option.label.clone());
            if let Some(handler) = self.on_change.as_ref() {
                let handler = Rc::clone(handler);
                let selected = self.selected.clone();
                let value = option.value.clone();
                checkbox = checkbox.on_change(move |next, event, window, cx| {
                    handler(apply_option(&selected, &value, next), event, window, cx);
                });
            }
            checkbox
        });
        // Options are the header's children, so a vertical group indents them
        // under it; a horizontal group has no such column to hang off.
        let indent = rems(self.theme, "--checkbox-group-indent");
        group.child(
            div()
                .flex()
                .gap(gap)
                .when(vertical, |rows| rows.flex_col().items_start())
                .when(!vertical, |rows| rows.flex_row().flex_wrap())
                .when(vertical && has_header, |rows| rows.pl(indent))
                .children(rows),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        CHECK_LINE, CheckboxSize, CheckboxState, DASH_LINE, MARK_BOX, YueCheckbox,
        YueCheckboxGroup, YueCheckboxOption, apply_option, apply_select_all, eligible_values,
        select_all_state, size_tokens,
    };
    use crate::tokens::{duration, easing, number, pixels, rgba};
    use crate::{Accent, ThemeMode, YueTheme};
    use gpui_kit::SharedString;

    fn option(value: &str, disabled: bool) -> YueCheckboxOption {
        let option = YueCheckboxOption::new(value, value);
        if disabled {
            option.disabled(true)
        } else {
            option
        }
    }

    fn selected(values: &[&str]) -> Vec<SharedString> {
        values
            .iter()
            .map(|value| SharedString::from(*value))
            .collect()
    }

    #[test]
    fn checkbox_state_names_the_accent_fill_for_both_chosen_states() {
        assert!(!CheckboxState::Unchecked.chosen());
        assert!(CheckboxState::Checked.chosen());
        assert!(
            CheckboxState::Indeterminate.chosen(),
            "the reference fills a mixed box the same way it fills a checked one"
        );
    }

    #[test]
    fn checkbox_needs_a_name_from_text_or_label() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(
            YueCheckbox::new("a", theme, CheckboxState::Unchecked)
                .label("读取")
                .accessible_name(),
            "读取"
        );
        assert_eq!(
            YueCheckbox::new("b", theme, CheckboxState::Unchecked)
                .accessibility_label("Accept terms")
                .accessible_name(),
            "Accept terms",
            "a text-less box is still named"
        );
    }

    #[test]
    #[should_panic(expected = "YueCheckbox needs a label")]
    fn a_nameless_checkbox_is_rejected() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let checkbox = YueCheckbox::new("c", theme, CheckboxState::Unchecked);
        let _ = checkbox.accessible_name();
    }

    #[test]
    fn select_all_state_follows_the_derivation_table() {
        let options = [option("read", false), option("write", false)];
        assert_eq!(
            select_all_state(&options, &[]),
            CheckboxState::Unchecked,
            "nothing selected"
        );
        assert_eq!(
            select_all_state(&options, &["write".into()]),
            CheckboxState::Indeterminate,
            "part of the eligible set selected"
        );
        assert_eq!(
            select_all_state(&options, &["read".into(), "write".into()]),
            CheckboxState::Checked,
            "the whole eligible set selected"
        );
        assert_eq!(
            select_all_state(&[], &[]),
            CheckboxState::Unchecked,
            "an empty group has nothing to complete"
        );

        let with_disabled = [
            option("read", false),
            option("write", false),
            option("admin", true),
        ];
        assert_eq!(
            select_all_state(&with_disabled, &["read".into(), "write".into()]),
            CheckboxState::Checked,
            "an unselected disabled option does not hold the header at mixed"
        );
        assert_eq!(
            select_all_state(&with_disabled, &["admin".into()]),
            CheckboxState::Unchecked,
            "a disabled option's own value does not count as progress"
        );
        assert_eq!(
            select_all_state(&[option("admin", true)], &["admin".into()]),
            CheckboxState::Unchecked,
            "an all-disabled group reports an empty eligible set"
        );
    }

    #[test]
    fn header_click_moves_only_eligible_values() {
        let options = [
            option("read", false),
            option("write", false),
            option("admin", true),
        ];
        let clearing = apply_select_all(&options, &["admin".into()], true);
        assert_eq!(
            clearing,
            selected(&["admin", "read", "write"]),
            "checking adds S ∪ E and leaves the disabled option's value in place"
        );
        let unchecking = apply_select_all(&options, &["admin".into(), "read".into()], false);
        assert_eq!(
            unchecking,
            selected(&["admin"]),
            "clearing subtracts only S ∩ E, so a disabled option keeps its value"
        );

        let already = apply_select_all(&options, &["read".into(), "write".into()], true);
        assert_eq!(
            already,
            selected(&["read", "write"]),
            "a header click on a complete group cannot duplicate a value"
        );
        let foreign = apply_select_all(&options, &["other".into()], true);
        assert_eq!(
            foreign,
            selected(&["other", "read", "write"]),
            "a value this group does not offer passes through untouched"
        );
    }

    #[test]
    fn an_inert_header_offers_nothing() {
        let empty: Vec<YueCheckboxOption> = Vec::new();
        assert!(
            eligible_values(&empty).is_empty(),
            "a group with no options has no header to operate"
        );
        let all_disabled = [option("admin", true)];
        assert!(
            eligible_values(&all_disabled).is_empty(),
            "a group whose every option is disabled has nothing to change"
        );
    }

    #[test]
    fn option_toggle_is_idempotent_and_keeps_order() {
        let current = selected(&["read", "write"]);
        let added = apply_option(&current, &"admin".into(), CheckboxState::Checked);
        assert_eq!(added, selected(&["read", "write", "admin"]));
        let repeated = apply_option(&added, &"admin".into(), CheckboxState::Checked);
        assert_eq!(repeated, added, "selecting a value twice stays one value");
        let removed = apply_option(&repeated, &"read".into(), CheckboxState::Unchecked);
        assert_eq!(removed, selected(&["write", "admin"]));
        assert_eq!(
            apply_option(&removed, &"missing".into(), CheckboxState::Unchecked),
            removed,
            "clearing a value that was never selected changes nothing"
        );
    }

    #[test]
    #[should_panic(expected = "option values must be unique")]
    fn duplicate_option_values_are_rejected() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let _ = YueCheckboxGroup::new("permissions", theme, Vec::<String>::new()).options([
            YueCheckboxOption::new("read", "读取"),
            YueCheckboxOption::new("read", "读取（重复）"),
        ]);
    }

    #[test]
    fn checkbox_marks_are_the_reference_paths_on_a_16_unit_box() {
        assert_eq!(MARK_BOX, 16.0);
        // `d="M4 8.5l3 3 5-6"`: the legs the check is written with, in order.
        assert_eq!(CHECK_LINE[0], (4.0, 8.5));
        let legs = CHECK_LINE
            .windows(2)
            .map(|pair| (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1));
        let want = [(3.0, 3.0), (5.0, -6.0)];
        for (leg, want) in legs.zip(want) {
            assert_eq!(leg, want, "the check keeps the reference's leg lengths");
        }
        // `d="M4.5 8h7"`: a 7-unit dash, centred on the box.
        assert_eq!(DASH_LINE[0], (4.5, 8.0));
        assert_eq!(DASH_LINE[1], (11.5, 8.0));
        assert_eq!(
            DASH_LINE[0].0 + DASH_LINE[1].0,
            MARK_BOX,
            "the dash is centred horizontally"
        );
        assert_eq!(DASH_LINE[0].1, MARK_BOX / 2.0, "and on the middle line");
        for point in CHECK_LINE.iter().chain(DASH_LINE.iter()) {
            assert!(
                point.0 > 0.0 && point.0 < MARK_BOX && point.1 > 0.0 && point.1 < MARK_BOX,
                "{point:?} stays inside the box it is drawn on"
            );
        }
    }

    #[test]
    fn size_profiles_pair_each_box_with_its_row_and_label() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let border = pixels(theme, "--checkbox-border-width");
        for (size, box_size, height, font, stroke_size) in [
            (CheckboxSize::Small, 12.0, 28.0, 12.0, 1.0),
            (CheckboxSize::Medium, 16.0, 32.0, 14.0, 1.5),
            (CheckboxSize::Large, 20.0, 40.0, 16.0, 2.0),
        ] {
            let (box_token, height_token, font_token, stroke_token) = size_tokens(size);
            assert_eq!(pixels(theme, box_token), box_size, "{box_token}");
            assert_eq!(pixels(theme, height_token), height, "{height_token}");
            assert_eq!(pixels(theme, font_token), font, "{font_token}");
            assert!(box_size < height, "{size:?} keeps its box inside its row");
            // The mark is painted on the box's content area, so the border has to
            // leave room for it at every rung.
            let unit = (box_size - 2.0 * border) / MARK_BOX;
            assert!(unit > 0.0, "{size:?} keeps a positive mark unit");
            assert_eq!(
                pixels(theme, stroke_token),
                stroke_size,
                "{size:?} keeps the mark on the glyph stroke ladder, not on its box"
            );
        }
    }

    #[test]
    fn checkbox_motion_resolves_the_reference_rungs() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert_eq!(
            duration(theme, "--checkbox-duration"),
            Duration::from_millis(200),
            "the box changes state on the enter rung"
        );
        assert_eq!(
            duration(theme, "--checkbox-duration-fast"),
            Duration::from_millis(100),
            "the mark erases, the delay runs and the box shrinks on the exit rung"
        );
        assert_eq!(
            number(theme, "--checkbox-press-scale"),
            0.9,
            "the press scales the box the way the reference does"
        );
        assert_eq!(
            pixels(theme, "--checkbox-halo-width"),
            4.0,
            "the hover halo reaches 4px past the box"
        );
        for name in [
            "--checkbox-ease",
            "--checkbox-ease-enter",
            "--checkbox-ease-exit",
        ] {
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
    }

    #[test]
    fn checkbox_colours_come_from_yue_roles_in_every_profile() {
        for (mode, accent) in [
            (ThemeMode::Light, Accent::Azure),
            (ThemeMode::Dark, Accent::Azure),
            (ThemeMode::Light, Accent::Neutral),
            (ThemeMode::Dark, Accent::Neutral),
        ] {
            let theme = YueTheme::new(mode, accent);
            let fill = rgba(theme, "--checkbox-background-checked");
            let mark = rgba(theme, "--checkbox-mark-color");
            assert!(
                fill.a > 0.99 && mark.a > 0.99,
                "{mode:?}/{accent:?} keeps an opaque fill and mark, so the mark is not a ghost"
            );
            let disabled = rgba(theme, "--checkbox-disabled-background");
            assert!(
                disabled.a < fill.a,
                "{mode:?}/{accent:?} dims the disabled fill below the checked one"
            );
            assert!(
                rgba(theme, "--checkbox-disabled-border-color").a == 0.0,
                "{mode:?}/{accent:?} drops the disabled border out, as the reference does"
            );
            let halo = rgba(theme, "--checkbox-halo-color");
            assert!(
                halo.a > 0.0 && halo.a < 0.5,
                "{mode:?}/{accent:?} keeps the hover halo a tint, not a fill"
            );
        }
    }
}
