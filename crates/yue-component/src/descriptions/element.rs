use super::*;
use crate::{
    ButtonSize, ButtonVariant, TextRole, YueButton, YueOverflowTooltip,
    tokens::{duration, rems, rgba},
};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{
    App, ClipboardItem, FocusHandle, InteractiveElement as _, ParentElement as _, RenderOnce, Role,
    StatefulInteractiveElement as _, Styled, Task, Window, canvas, div,
    prelude::FluentBuilder as _,
};
use std::collections::HashMap;

struct CopyState {
    payload: SharedString,
    revision: Option<SharedString>,
    focus: FocusHandle,
    copied: bool,
    task: Option<Task<()>>,
    generation: u64,
}
#[derive(Default)]
struct Runtime {
    copies: HashMap<SharedString, CopyState>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Geometry {
    columns: u16,
    stacked: bool,
}

fn align(mut d: gpui_kit::Div, horizontal: DescriptionsAlign) -> gpui_kit::Div {
    d = match horizontal {
        DescriptionsAlign::Left => d.text_left(),
        DescriptionsAlign::Center => d.text_center(),
        DescriptionsAlign::Right => d.text_right(),
    };
    d
}
fn justify<T: Styled>(mut d: T, vertical: DescriptionsVerticalAlign) -> T {
    d = match vertical {
        DescriptionsVerticalAlign::Top => d.justify_start(),
        DescriptionsVerticalAlign::Center => d.justify_center(),
        DescriptionsVerticalAlign::Bottom => d.justify_end(),
    };
    d
}
fn horizontal<T: Styled>(d: T, align: DescriptionsAlign) -> T {
    match align {
        DescriptionsAlign::Left => d.justify_start(),
        DescriptionsAlign::Center => d.justify_center(),
        DescriptionsAlign::Right => d.justify_end(),
    }
}
impl RenderOnce for YueDescriptions {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        for item in &self.items {
            item.validate();
        }
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let suffix = match self.size {
            DescriptionsSize::Small => "sm",
            DescriptionsSize::Medium => "md",
            DescriptionsSize::Large => "lg",
        };
        let token = |name: &str| rems(theme, format!("--descriptions-{name}-{suffix}").as_str());
        let (padding_x, padding_y) = (token("padding-x"), token("padding-y"));
        let border = rems(theme, "--descriptions-border-width");
        let gap_x = if self.bordered {
            border
        } else {
            token("column-gap")
        };
        let gap_y = if self.bordered {
            border
        } else {
            token("row-gap")
        };
        let min = self
            .min_item_width
            .unwrap_or_else(|| rems(theme, "--descriptions-item-min-width").into())
            .to_pixels(window.rem_size());
        let label_width = self
            .label_width
            .unwrap_or_else(|| rems(theme, "--descriptions-label-width").into());
        let narrow = label_width.to_pixels(window.rem_size())
            + rems(theme, "--descriptions-value-min-width").to_pixels(window.rem_size())
            + padding_x.to_pixels(window.rem_size()) * 2.;
        let max = self.columns;
        // Native grid sizes tracks with this frame's width. Only its discrete
        // column/stack rule is retained; no pixel width is copied into children.
        let geometry =
            window.use_keyed_state((self.id.clone(), "descriptions-geometry"), cx, |_, _| {
                Geometry {
                    columns: 1,
                    stacked: false,
                }
            });
        let g = *geometry.read(cx);
        let columns = g.columns.min(max);
        let stacked = self.orientation == DescriptionsLayout::Vertical || g.stacked;
        let plan = model::pack(self.items.iter().map(|i| i.span), columns, self.bordered);
        let last_row = plan.last().map_or(0, |p| p.row);
        let runtime = window.use_keyed_state(
            (self.id.clone(), "descriptions-runtime"),
            cx,
            |w, cx: &mut gpui_kit::Context<Runtime>| {
                let handle = w.window_handle();
                cx.on_release(move |rt, cx| {
                    let focus = rt
                        .copies
                        .values()
                        .map(|s| s.focus.clone())
                        .collect::<Vec<_>>();
                    cx.defer(move |cx| {
                        let _ = handle.update(cx, |_, w, cx| {
                            if focus.iter().any(|f| f.is_focused(w)) {
                                w.focus_next(cx);
                            }
                        });
                    });
                })
                .detach();
                Runtime::default()
            },
        );
        let payloads = self
            .items
            .iter()
            .filter(|i| i.copyable)
            .map(|i| {
                let text = i.copy_text.clone().unwrap_or_else(|| match &i.value {
                    Value::Text(t) => t.clone(),
                    Value::Content(_) => unreachable!(),
                });
                let revision = match &i.value {
                    Value::Text(t) => Some(t.clone()),
                    Value::Content(_) => None,
                };
                (i.key.clone(), (text, revision))
            })
            .collect::<HashMap<_, _>>();
        runtime.update(cx, |rt, cx| {
            let removed_focus = rt.copies.iter().find_map(|(key, s)| {
                (!payloads.contains_key(key) && s.focus.is_focused(window)).then(|| s.focus.clone())
            });
            if let Some(focus) = removed_focus {
                window.defer(cx, move |w, cx| {
                    if focus.is_focused(w) {
                        w.focus_next(cx);
                    }
                });
            }
            rt.copies.retain(|key, _| payloads.contains_key(key));
            for (key, (payload, revision)) in payloads {
                let state = rt.copies.entry(key).or_insert_with(|| CopyState {
                    payload: payload.clone(),
                    revision: revision.clone(),
                    focus: cx.focus_handle(),
                    copied: false,
                    task: None,
                    generation: 0,
                });
                if state.payload != payload || state.revision != revision {
                    state.payload = payload;
                    state.revision = revision;
                    state.copied = false;
                    state.task = None;
                    state.generation += 1;
                }
            }
        });
        let mut root = div()
            .id(self.id)
            .test_support()
            .min_w_0()
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .role(Role::DescriptionList)
            .aria_label(self.name)
            .text_size(token("font-size"))
            .line_height(token("line-height"))
            .text_color(rgba(theme, "--descriptions-color"));
        if self.title.is_some() || self.extra.is_some() {
            let mut header = div()
                .id("descriptions-header")
                .test_support()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(rems(theme, "--descriptions-header-gap"))
                .mb(rems(theme, "--descriptions-header-margin-bottom"));
            if let Some(title) = self.title {
                header = header.child(
                    TextRole::Title
                        .apply(theme, div().min_w_0().flex_1())
                        .child(title),
                );
            }
            if let Some(extra) = self.extra {
                header = header.child(div().min_w_0().flex_shrink_0().child(extra));
            }
            root = root.child(header);
        }
        let capture = canvas(
            move |bounds, window, cx| {
                let width = f32::from(bounds.size.width).max(0.);
                let c = model::columns(
                    width,
                    min.into(),
                    gap_x.to_pixels(window.rem_size()).into(),
                    max,
                );
                let next = Geometry {
                    columns: c,
                    stacked: c == 1 && width < f32::from(narrow),
                };
                if *geometry.read(cx) != next {
                    window.defer(cx, move |_, cx| {
                        geometry.update(cx, |g, cx| {
                            if *g != next {
                                *g = next;
                                cx.notify();
                            }
                        });
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();
        let mut grid = div()
            .id("descriptions-grid")
            .test_support()
            .relative()
            .min_w_0()
            .w_full()
            .flex_shrink_0()
            .grid()
            .grid_cols(columns)
            .gap_x(gap_x)
            .gap_y(gap_y)
            .when(self.bordered && !self.items.is_empty(), |d| {
                d.border(border)
                    .border_color(rgba(theme, "--descriptions-border-color"))
                    .rounded(rems(theme, "--descriptions-radius"))
            });
        for (item, cell) in self.items.into_iter().zip(plan) {
            let vertical = item.vertical.unwrap_or(self.vertical);
            let mut label = justify(
                div()
                    .id("description-label")
                    .test_support()
                    .relative()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .role(Role::Term)
                    .aria_label(item.label.clone()),
                if stacked {
                    DescriptionsVerticalAlign::Top
                } else {
                    vertical
                },
            );
            label = if stacked {
                label.w_full()
            } else {
                label.w(label_width)
            };
            label = if self.bordered {
                label.px(padding_x).py(padding_y)
            } else {
                label.pr(rems(theme, "--descriptions-label-value-gap"))
            };
            let label_text = if self.colon {
                format!("{}：", item.label).into()
            } else {
                item.label.clone()
            };
            let label_content = item.label_content.unwrap_or_else(|| {
                align(div().min_w_0().w_full(), self.label_align)
                    .child(label_text)
                    .into_any_element()
            });
            label = label
                .text_color(rgba(theme, "--descriptions-label-color"))
                .child(
                    horizontal(div().flex(), self.label_align)
                        .id("description-label-content")
                        .test_support()
                        .min_w_0()
                        .w_full()
                        .child(label_content),
                );
            let mut value = justify(
                div()
                    .id("description-value")
                    .test_support()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .role(Role::Definition)
                    .aria_label(item.label),
                vertical,
            );
            if self.bordered {
                value = value.px(padding_x).py(padding_y);
            } else if stacked {
                value = value.mt(rems(theme, "--descriptions-label-value-gap"));
            }
            let mut content = horizontal(
                div()
                    .id("description-value-content")
                    .test_support()
                    .min_w_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(rems(theme, "--descriptions-content-gap")),
                self.value_align,
            );
            let is_text = matches!(item.value, Value::Text(_));
            let rendered = match item.value {
                Value::Text(text) => match item.overflow {
                    DescriptionsTextOverflow::Wrap => div()
                        .id("description-text")
                        .test_support()
                        .min_w_0()
                        .w_full()
                        .child(text)
                        .into_any_element(),
                    overflow => YueOverflowTooltip::new("description-text", theme, text)
                        .width(gpui_kit::relative(1.))
                        .lines(match overflow {
                            DescriptionsTextOverflow::Lines(n) => n,
                            _ => 1,
                        })
                        .focusable_when_overflow(true)
                        .into_any_element(),
                },
                Value::Content(content) => content,
            };
            let mut text_box = align(div(), self.value_align)
                .id("description-content")
                .test_support()
                .min_w_0();
            if is_text {
                text_box = text_box.flex_1();
            }
            content = content.child(text_box.child(rendered));
            if item.copyable {
                let s = &runtime.read(cx).copies[&item.key];
                let (copied, focus) = (s.copied, s.focus.clone());
                let label: SharedString = if copied {
                    "已复制".into()
                } else {
                    "复制完整内容".into()
                };
                let button = if copied {
                    YueButton::icon(
                        "description-copy",
                        theme,
                        crate::outline::Check,
                        label.clone(),
                    )
                } else {
                    YueButton::icon(
                        "description-copy",
                        theme,
                        crate::outline::Copy,
                        label.clone(),
                    )
                };
                let weak = runtime.downgrade();
                let key = item.key.clone();
                content = content.child(
                    button
                        .size(ButtonSize::Small)
                        .variant(ButtonVariant::Ghost)
                        .tooltip(label)
                        .track_focus(&focus)
                        .on_click(move |_, w, cx| {
                            cx.stop_propagation();
                            let _ = weak.update(cx, |rt, cx| {
                                let Some(s) = rt.copies.get_mut(&key) else {
                                    return;
                                };
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    s.payload.to_string(),
                                ));
                                s.copied = true;
                                s.generation += 1;
                                let generation = s.generation;
                                let key = key.clone();
                                s.task = Some(cx.spawn_in(w, async move |weak, cx| {
                                    cx.background_executor()
                                        .timer(duration(theme, "--motion-duration-settle") * 4)
                                        .await;
                                    let _ = weak.update(cx, |rt, cx| {
                                        if let Some(s) = rt.copies.get_mut(&key)
                                            && s.generation == generation
                                        {
                                            s.copied = false;
                                            s.task = None;
                                            cx.notify();
                                        }
                                    });
                                }));
                                cx.notify();
                            });
                        }),
                );
            }
            value = value.child(content);
            if self.bordered {
                label = label.child(
                    div()
                        .absolute()
                        .bg(rgba(theme, "--descriptions-border-color"))
                        .when(stacked, |d| d.bottom_0().left_0().w_full().h(border))
                        .when(!stacked, |d| d.right_0().top_0().h_full().w(border)),
                );
            }
            // A flat keyed tree: row/column changes never change ancestors.
            grid = grid.child(
                div()
                    .id(item.key)
                    .test_support()
                    .relative()
                    .min_w_0()
                    .flex()
                    .items_stretch()
                    .col_span(cell.span)
                    .when(stacked, |d| d.flex_col())
                    .child(label)
                    .child(value)
                    // Paint only separators in the reserved grid gaps. Cells
                    // remain transparent, including behind custom content.
                    .when(self.bordered && cell.column + cell.span < columns, |d| {
                        d.child(
                            div()
                                .id("description-separator-right")
                                .test_support()
                                .absolute()
                                .right(-border)
                                .top_0()
                                .bottom_0()
                                .w(border)
                                .bg(rgba(theme, "--descriptions-border-color")),
                        )
                    })
                    .when(self.bordered && cell.row < last_row, |d| {
                        d.child(
                            div()
                                .id("description-separator-bottom")
                                .test_support()
                                .absolute()
                                .bottom(-border)
                                .left_0()
                                .right_0()
                                .h(border)
                                .when(cell.column + cell.span < columns, |d| d.right(-border))
                                .bg(rgba(theme, "--descriptions-border-color")),
                        )
                    }),
            );
        }
        root = root.child(grid.child(capture));
        self.layout.apply(root)
    }
}
