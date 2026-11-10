use super::*;
use crate::{
    MotionDuration, MotionEase, YueScrollbar,
    tokens::{mix_rgba, rems, rgba},
};
use gpui_kit::base::{Button, ElementExt as _, Interpolate, TestSupportExt as _, transition};
use gpui_kit::{
    Entity, FocusHandle, InteractiveElement as _, ParentElement as _, PathBuilder, RenderOnce,
    Rgba, Role, ScrollHandle, StatefulInteractiveElement as _, Styled as _, canvas, div, point,
};
use std::collections::HashMap;
use yue_component_icons::outline;

#[derive(Clone, Copy, PartialEq)]
struct RevealGeometry {
    size: gpui_kit::Size<gpui_kit::Pixels>,
    target: Option<usize>,
    rem: gpui_kit::Pixels,
}
#[derive(Default)]
struct Runtime {
    items: Vec<(SharedString, bool)>,
    value: StepsValue,
    handler: Option<ChangeHandler>,
    focus: HashMap<SharedString, FocusHandle>,
    scroll: ScrollHandle,
    last_focused: Option<SharedString>,
    last_rem: Option<gpui_kit::Pixels>,
    generation: u64,
    geometry: Rc<std::cell::Cell<Option<RevealGeometry>>>,
}
fn request(
    runtime: &Entity<Runtime>,
    generation: u64,
    key: &SharedString,
    w: &mut Window,
    cx: &mut App,
) {
    let state = runtime.read(cx);
    if state.generation != generation || state.value.key() == Some(key) {
        return;
    }
    let Some(index) = state
        .items
        .iter()
        .position(|(k, enabled)| k == key && *enabled)
    else {
        return;
    };
    let event = StepsChange {
        key: key.clone(),
        index,
        before: state.value.clone(),
    };
    let handler = state.handler.clone();
    // The callback can synchronously replace or remove its owner.
    if let Some(handler) = handler {
        handler(&event, w, cx);
    }
}

impl RenderOnce for YueSteps {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let current = self.validate();
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let runtime = w.use_keyed_state((self.id.clone(), "steps-runtime"), cx, |_, _| {
            Runtime::default()
        });
        let old_focus = runtime
            .read(cx)
            .focus
            .iter()
            .find(|(_, f)| f.is_focused(w))
            .map(|(k, _)| k.clone());
        let old_index = old_focus
            .as_ref()
            .and_then(|key| runtime.read(cx).items.iter().position(|(k, _)| k == key))
            .unwrap_or(0);
        let actionable = self.handler.is_some() && !self.readonly;
        let metadata = self
            .items
            .iter()
            .map(|i| (i.key.clone(), actionable && !self.disabled && !i.disabled))
            .collect::<Vec<_>>();
        let changed = runtime.read(cx).items != metadata || runtime.read(cx).value != self.value;
        let (generation, scroll) = runtime.update(cx, |rt, cx| {
            if changed {
                rt.generation += 1;
            }
            rt.items = metadata;
            rt.value = self.value.clone();
            rt.handler = self.handler.clone();
            rt.focus
                .retain(|key, _| self.items.iter().any(|i| &i.key == key));
            for item in &self.items {
                rt.focus
                    .entry(item.key.clone())
                    .or_insert_with(|| cx.focus_handle());
            }
            (rt.generation, rt.scroll.clone())
        });
        if let Some(old) = old_focus.as_ref()
            && !runtime
                .read(cx)
                .items
                .iter()
                .any(|(key, enabled)| key == old && *enabled)
        {
            let target = current
                .filter(|&ix| runtime.read(cx).items[ix].1)
                .or_else(|| {
                    runtime
                        .read(cx)
                        .items
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, enabled))| *enabled)
                        .min_by_key(|(ix, _)| ix.abs_diff(old_index))
                        .map(|(ix, _)| ix)
                });
            if let Some(ix) = target {
                w.focus(&runtime.read(cx).focus[&self.items[ix].key].clone(), cx);
            } else {
                w.blur(cx);
            }
        }
        let focused = runtime
            .read(cx)
            .focus
            .iter()
            .find(|(_, f)| f.is_focused(w))
            .map(|(k, _)| k.clone());
        let reveal = changed
            || runtime.read(cx).last_focused != focused
            || runtime.read(cx).last_rem != Some(w.rem_size());
        runtime.update(cx, |rt, _| {
            rt.last_focused = focused.clone();
            rt.last_rem = Some(w.rem_size());
        });
        if reveal
            && self.orientation == StepsLayout::Horizontal
            && let Some(ix) = focused
                .as_ref()
                .or(self.value.key())
                .and_then(|key| self.items.iter().position(|i| &i.key == key))
        {
            scroll.scroll_to_item(if self.reverse {
                self.items.len() - 1 - ix
            } else {
                ix
            });
        }
        let states = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| model::status(&self.value, current, ix, self.status, item.status))
            .collect::<Vec<_>>();
        let count = self.items.len();
        let mut order = self.items.into_iter().enumerate().collect::<Vec<_>>();
        if self.reverse {
            order.reverse();
        }
        let mut list = div()
            .id("steps-list")
            .test_support()
            .role(Role::List)
            .aria_label(self.name)
            .relative()
            .flex()
            .w_full()
            .min_w_0();
        if self.orientation == StepsLayout::Vertical {
            list = list.flex_col();
        } else {
            list = list
                .overflow_x_scroll()
                .track_scroll(&scroll)
                .pb(rems(theme, "--steps-gap"));
        }
        for (slot, (ix, item)) in order.into_iter().enumerate() {
            let next = if self.reverse {
                ix.checked_sub(1)
            } else {
                (ix + 1 < count).then_some(ix + 1)
            };
            let color = next.map(|other| {
                let (a, b) = if ix < other { (ix, other) } else { (other, ix) };
                rgba(theme, model::line(states[a], states[b]))
            });
            let previous = if self.reverse {
                (ix + 1 < count).then_some(ix + 1)
            } else {
                ix.checked_sub(1)
            };
            let incoming = previous.map(|other| {
                let (a, b) = if ix < other { (ix, other) } else { (other, ix) };
                rgba(theme, model::line(states[a], states[b]))
            });
            let key = item.key.clone();
            let focus = runtime.read(cx).focus[&key].clone();
            list = list.child(StepCell {
                id: (ElementId::from("step"), key).into(),
                item,
                index: ix,
                count,
                slot,
                state: states[ix],
                current: current == Some(ix),
                theme,
                size: self.size,
                orientation: self.orientation,
                variant: self.variant,
                placement: self.placement,
                disabled: self.disabled,
                actionable,
                line: color,
                incoming,
                focus,
                runtime: runtime.clone(),
                generation,
            });
        }
        let geometry = runtime.read(cx).geometry.clone();
        let target = focused
            .as_ref()
            .or(self.value.key())
            .and_then(|key| runtime.read(cx).items.iter().position(|(k, _)| k == key))
            .map(|ix| if self.reverse { count - 1 - ix } else { ix });
        let observed_scroll = scroll.clone();
        let horizontal = self.orientation == StepsLayout::Horizontal;
        let mut root = div()
            .id(self.id)
            .relative()
            .w_full()
            .min_w_0()
            .child(list)
            .on_prepaint(move |bounds, w, _| {
                let now = RevealGeometry {
                    size: bounds.size,
                    target,
                    rem: w.rem_size(),
                };
                if horizontal && geometry.get() != Some(now) {
                    geometry.set(Some(now));
                    if let Some(ix) = target {
                        observed_scroll.scroll_to_item(ix);
                        w.request_animation_frame();
                    }
                }
            });
        if self.orientation == StepsLayout::Horizontal {
            root = root.child(YueScrollbar::horizontal("steps-scrollbar", &scroll).theme(theme));
        }
        self.layout.apply(root)
    }
}

#[derive(Clone, PartialEq)]
struct Palette {
    foreground: Rgba,
    background: Rgba,
    label: Rgba,
    line: Rgba,
    incoming: Rgba,
}
impl Interpolate for Palette {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            foreground: mix_rgba(self.foreground, other.foreground, t),
            background: mix_rgba(self.background, other.background, t),
            label: mix_rgba(self.label, other.label, t),
            line: mix_rgba(self.line, other.line, t),
            incoming: mix_rgba(self.incoming, other.incoming, t),
        }
    }
}
#[derive(IntoElement)]
struct StepCell {
    id: ElementId,
    item: YueStepItem,
    index: usize,
    count: usize,
    slot: usize,
    state: StepsStatus,
    current: bool,
    theme: YueTheme,
    size: StepsSize,
    orientation: StepsLayout,
    variant: StepsVariant,
    placement: StepsLabelPlacement,
    disabled: bool,
    actionable: bool,
    line: Option<Rgba>,
    incoming: Option<Rgba>,
    focus: FocusHandle,
    runtime: Entity<Runtime>,
    generation: u64,
}
impl RenderOnce for StepCell {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme;
        let diameter = rems(
            theme,
            &format!("--steps-indicator-size-{}", self.size.suffix()),
        );
        let icon_size = rems(theme, &format!("--steps-icon-size-{}", self.size.suffix()));
        let font_size = rems(theme, &format!("--steps-font-size-{}", self.size.suffix()));
        let gap = rems(theme, "--steps-gap");
        let width = rems(theme, "--steps-line-width");
        let disabled = self.disabled || self.item.disabled;
        let (fg, bg, label) = if disabled {
            (
                "--steps-color-disabled",
                "--steps-background",
                "--steps-color-disabled",
            )
        } else {
            match self.state {
                StepsStatus::Process => (
                    "--steps-on-current",
                    "--steps-background-current",
                    "--steps-color-current",
                ),
                StepsStatus::Finish => (
                    "--steps-on-finish",
                    "--steps-background-finish",
                    "--steps-color-finish",
                ),
                StepsStatus::Error => (
                    "--steps-color-error",
                    "--steps-background-error",
                    "--steps-color-error",
                ),
                StepsStatus::Waiting => ("--steps-color", "--steps-background", "--steps-color"),
            }
        };
        let palette = transition(
            (self.id.clone(), "steps-colors"),
            Palette {
                foreground: rgba(theme, fg),
                background: rgba(theme, bg),
                label: rgba(theme, label),
                incoming: self
                    .incoming
                    .unwrap_or_else(|| rgba(theme, "--steps-line-color")),
                line: self
                    .line
                    .unwrap_or_else(|| rgba(theme, "--steps-line-color")),
            },
            crate::motion::transition(theme, MotionDuration::Interaction, MotionEase::Standard),
            w,
            cx,
        );
        let vertical = self.orientation == StepsLayout::Vertical;
        let below = !vertical && self.placement == StepsLabelPlacement::Below;
        let arrow = self.variant == StepsVariant::Arrow;
        let navigation = self.variant == StepsVariant::Navigation;
        let has_body = self.item.description.is_some() || self.item.extra.is_some();
        let mut marker = div()
            .id("steps-marker")
            .test_support()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .size(diameter)
            .text_size(font_size)
            .line_height(diameter)
            .text_color(palette.foreground);
        if self.variant == StepsVariant::Dot {
            marker = marker.child(
                div()
                    .size(rems(theme, "--steps-dot-size"))
                    .rounded_full()
                    .bg(if self.state == StepsStatus::Waiting {
                        rgba(theme, "--steps-line-color")
                    } else {
                        palette.label
                    }),
            );
        } else {
            marker = marker
                .rounded_full()
                .bg(palette.background)
                .border(width)
                .border_color(if self.state == StepsStatus::Waiting {
                    rgba(theme, "--steps-line-color")
                } else {
                    palette.background
                });
            let icon = match self.state {
                StepsStatus::Finish => Some(Icon::new(outline::Check)),
                StepsStatus::Error => Some(Icon::new(outline::X)),
                _ => self.item.icon,
            };
            if let Some(icon) = icon {
                marker = marker.child(icon.size(icon_size).color_value(palette.foreground));
            } else {
                marker = marker.child((self.index + 1).to_string());
            }
        }
        let label = format!(
            "第 {}/{} 步，{}，{}{}",
            self.index + 1,
            self.count,
            self.item.title,
            self.state.name(),
            if self.current { "，当前步骤" } else { "" }
        );
        let label = if disabled {
            format!("{label}，不可用")
        } else {
            label
        };
        let description = [
            self.item.subtitle.as_deref(),
            self.item.description.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("；");
        let mut title = div()
            .id("steps-title")
            .test_support()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(gap)
            .child(div().min_w_0().max_w_full().child(self.item.title));
        if let Some(subtitle) = self.item.subtitle {
            title = title.child(
                div()
                    .text_size(rems(theme, "--steps-description-font-size"))
                    .text_color(rgba(
                        theme,
                        if disabled {
                            "--steps-color-disabled"
                        } else {
                            "--steps-description-color"
                        },
                    ))
                    .child(subtitle),
            );
        }
        let mut header = div().relative().flex().items_start().min_w_0();
        title = title.line_height(diameter);
        if below {
            header = header.flex_col().items_center().w_full().gap(gap);
            title = title.w_full().px(gap).justify_center().text_center();
        } else {
            title = title.pl(gap);
        }
        if !below {
            header = header.w_full();
            if !vertical && !arrow && !navigation {
                title = title.max_w(gpui_kit::relative(0.65));
            }
        }
        if navigation {
            header = header.pr(icon_size + gap);
        }
        header = header.child(marker).child(title);
        if !vertical && !below && !arrow && !navigation && self.line.is_some() {
            header = header.child(
                div()
                    .id("steps-connector")
                    .test_support()
                    .flex_1()
                    .min_w(gap)
                    .ml(gap)
                    .mt((diameter - width) / 2.)
                    .h(width)
                    .bg(palette.line),
            );
        }
        let mut cell = div()
            .id(self.id)
            .test_support()
            .role(Role::ListItem)
            .aria_position_in_set(self.index + 1)
            .aria_size_of_set(self.count)
            .relative()
            .min_w_0();
        if vertical {
            cell = cell.w_full().pb(if self.slot + 1 < self.count {
                rems(theme, "--steps-item-gap")
            } else {
                gpui_kit::rems(0.)
            });
        } else {
            cell = cell
                .flex_1()
                .flex_shrink_0()
                .min_w(rems(theme, "--steps-item-min-width"));
        }
        if !arrow && !navigation {
            if vertical {
                if self.line.is_some() {
                    cell = cell.child(
                        div()
                            .id("steps-connector")
                            .test_support()
                            .absolute()
                            .left((diameter - width) / 2.)
                            .top(diameter / 2.)
                            .bottom(-diameter / 2.)
                            .w(width)
                            .bg(palette.line),
                    );
                }
            } else {
                // Below uses two halves; Right continues from the previous
                // cell boundary to this fixed marker column.
                let midpoint: gpui_kit::DefiniteLength = if below {
                    gpui_kit::relative(0.5)
                } else {
                    (diameter / 2.).into()
                };
                if below && self.line.is_some() {
                    cell = cell.child(
                        div()
                            .id("steps-connector")
                            .test_support()
                            .absolute()
                            .left(midpoint)
                            .right_0()
                            .top((diameter - width) / 2.)
                            .h(width)
                            .bg(palette.line),
                    );
                }
                if self.incoming.is_some() {
                    cell = cell.child(
                        div()
                            .absolute()
                            .left_0()
                            .w(midpoint)
                            .top((diameter - width) / 2.)
                            .h(width)
                            .bg(palette.incoming),
                    );
                }
            }
        }
        if arrow {
            // The arrow surface owns both outer insets; header padding must
            // not leave description/extra content against the bottom edge.
            cell = cell.py(gap);
            let background = if disabled {
                rgba(theme, "--steps-background")
            } else if self.current {
                rgba(theme, "--steps-background-finish")
            } else {
                rgba(theme, "--steps-background")
            };
            let notch = rems(theme, "--steps-arrow-width").to_pixels(w.rem_size());
            let first = self.slot == 0;
            let last = self.slot + 1 == self.count;
            cell = cell.child(
                canvas(
                    |bounds, _, _| bounds,
                    move |bounds, _, w, _| {
                        let l = bounds.left();
                        let r = bounds.right();
                        let t = bounds.top();
                        let b = bounds.bottom();
                        let mid = bounds.center().y;
                        let mut path = PathBuilder::fill();
                        path.move_to(point(l, t));
                        path.line_to(point(if last { r } else { r - notch }, t));
                        path.line_to(point(r, mid));
                        path.line_to(point(if last { r } else { r - notch }, b));
                        path.line_to(point(l, b));
                        if !first {
                            path.line_to(point(l + notch, mid));
                        }
                        path.close();
                        if let Ok(path) = path.build() {
                            w.paint_path(path, background);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
            header = header.px(rems(theme, "--steps-arrow-width") * 2.);
        }
        let enabled = self.actionable && !disabled;
        let header = if self.actionable {
            let runtime = self.runtime;
            let key = self.item.key;
            let generation = self.generation;
            let mut button = Button::new("steps-action")
                .track_focus(&self.focus)
                .disabled(disabled)
                .tab_stop(enabled)
                .accessibility_label(label.clone())
                .aria_description(description.clone())
                .relative()
                .min_w_0()
                .flex()
                .justify_start()
                .rounded(rems(theme, "--steps-border-radius"))
                .text_size(font_size)
                .text_color(palette.label)
                .child(header);
            if below || arrow || navigation {
                button = button.w_full();
            }
            if enabled {
                button = button
                    .cursor_pointer()
                    .hover(move |s| s.text_color(rgba(theme, "--steps-color-finish")));
            }
            if enabled && self.focus.is_focused(w) && w.last_input_was_keyboard() {
                button = button.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(rems(theme, "--steps-border-radius"))
                        .border(rems(theme, "--focus-ring-width"))
                        .border_color(rgba(theme, "--focus-ring-color")),
                );
            }
            button
                .on_click(move |_, w, cx| request(&runtime, generation, &key, w, cx))
                .into_any_element()
        } else {
            div()
                .text_size(font_size)
                .text_color(palette.label)
                .child(header)
                .into_any_element()
        };
        if !self.actionable {
            cell = cell.aria_label(label).aria_description(description);
        }
        cell = cell.child(header);
        if navigation {
            cell = cell
                .pb(gap)
                .border_b(rems(theme, "--steps-navigation-line-width"))
                .border_color(if self.current {
                    palette.label
                } else {
                    rgba(theme, "--steps-line-color")
                });
            if self.slot + 1 < self.count {
                cell = cell.child(
                    div()
                        .absolute()
                        .right_0()
                        .top((diameter - icon_size) / 2.)
                        .child(
                            Icon::new(outline::ChevronRight)
                                .size(icon_size)
                                .color_value(rgba(theme, "--steps-color"))
                                .into_any_element(),
                        ),
                );
            }
        }
        let mut body = div()
            .id("steps-body")
            .test_support()
            .relative()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(rems(theme, "--steps-description-gap"));
        if below {
            body = body.text_center().mt(gap);
        } else if !arrow {
            body = body.pl(diameter + gap);
        } else {
            body = body.px(rems(theme, "--steps-arrow-width") * 2.);
            if has_body {
                body = body.mt(gap);
            }
        }
        if let Some(text) = self.item.description {
            body = body.child(
                div()
                    .text_size(rems(theme, "--steps-description-font-size"))
                    .text_color(rgba(
                        theme,
                        if disabled {
                            "--steps-color-disabled"
                        } else {
                            "--steps-description-color"
                        },
                    ))
                    .child(text),
            );
        }
        if let Some(extra) = self.item.extra {
            body = body.child(extra);
        }
        cell.child(body)
    }
}
