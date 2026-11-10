use super::*;
use crate::{
    ButtonSize, ButtonVariant, InputSize, InputStatus, InputTextAlign, SelectSize, YueButton,
    YueInput, YueSelect,
    tokens::{rems, rgba},
};
use gpui_kit::base::{Pagination, PaginationItem, PaginationState, TestSupportExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, EntityInputHandler as _, InteractiveElement as _, ParentElement as _,
    RenderOnce, Role, StatefulInteractiveElement as _, Styled as _, Window, div,
};
use yue_component_icons::outline;

impl PaginationSize {
    fn button(self) -> ButtonSize {
        match self {
            Self::Small => ButtonSize::Small,
            Self::Medium => ButtonSize::Medium,
            Self::Large => ButtonSize::Large,
        }
    }
    fn input(self) -> InputSize {
        match self {
            Self::Small => InputSize::Small,
            Self::Medium => InputSize::Medium,
            Self::Large => InputSize::Large,
        }
    }
    fn select(self) -> SelectSize {
        match self {
            Self::Small => SelectSize::Small,
            Self::Medium => SelectSize::Medium,
            Self::Large => SelectSize::Large,
        }
    }
    fn suffix(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
}
impl RenderOnce for YuePagination {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let value = self.state.read(cx).value;
        let hidden = self.hide_single && value.page_count() == 1;
        self.state.update(cx, |s, cx| {
            s.configure_visibility(
                self.jumper && !hidden,
                self.selector && !hidden,
                hidden,
                w,
                cx,
            )
        });
        if hidden {
            return div().into_any_element();
        }
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let blocked = self.state.read(cx).blocked();
        let loading = self.state.read(cx).is_loading();
        let base = PaginationState::new(value.page(), value.page_count())
            .visible_pages(self.visible)
            .disabled(blocked);
        let items = if value.page_count() == 1 {
            vec![PaginationItem::Page(1)]
        } else {
            base.items()
        };
        let pages = if self.mode == PaginationMode::Pages {
            items
                .iter()
                .filter_map(|p| match p {
                    PaginationItem::Page(n) => Some(*n),
                    _ => None,
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let focus = self.state.update(cx, |s, cx| s.focus_pages(&pages, w, cx));
        let nav_focus = self.state.read(cx).navigation_focus.clone();
        let input = self.state.read(cx).input.clone();
        let select = self.state.read(cx).select.clone();
        let invalid = self.state.read(cx).invalid;
        let id = self.id.clone();
        let gap = rems(theme, "--pagination-item-gap");
        let height = rems(
            theme,
            format!("--button-height-{}", self.size.suffix()).as_str(),
        );
        let plain = match self.variant {
            PaginationVariant::Ghost => ButtonVariant::Ghost,
            PaginationVariant::Outline => ButtonVariant::Outline,
        };
        let command = |button: YueButton,
                       target: usize,
                       source: PaginationChangeSource,
                       selected: bool,
                       disabled: bool,
                       handle: &gpui_kit::FocusHandle| {
            let state = self.state.clone();
            button
                .theme(theme)
                .size(self.size.button())
                .variant(if selected {
                    ButtonVariant::Primary
                } else {
                    plain
                })
                .min_width(height)
                .track_focus(handle)
                .disabled(blocked || disabled)
                .on_click(move |_, w, cx| {
                    state.update(cx, |s, cx| {
                        if s.value == value {
                            s.request_page(target, source, w, cx);
                        }
                    })
                })
        };
        let navigation = |slot: usize,
                          target: usize,
                          source: PaginationChangeSource,
                          label: &str,
                          disabled: bool| {
            let key = match slot {
                0 => "previous",
                1 => "next",
                2 => "first",
                3 => "last",
                4 => "backward",
                _ => "forward",
            };
            let button_id: ElementId = (id.clone(), key).into();
            let button = if slot < 2
                && let Some(labels) = self.labels.as_ref()
            {
                YueButton::new(
                    button_id,
                    if slot == 0 {
                        labels.0.clone()
                    } else {
                        labels.1.clone()
                    },
                )
                .accessibility_label(label.to_owned())
            } else {
                let button = match slot {
                    0 => YueButton::icon(button_id, theme, outline::ChevronLeft, label.to_owned()),
                    1 => YueButton::icon(button_id, theme, outline::ChevronRight, label.to_owned()),
                    2 => YueButton::icon(button_id, theme, outline::ChevronsLeft, label.to_owned()),
                    3 => {
                        YueButton::icon(button_id, theme, outline::ChevronsRight, label.to_owned())
                    }
                    _ => YueButton::icon(button_id, theme, outline::Dots, label.to_owned()),
                };
                button.tooltip(label.to_owned())
            };
            command(button, target, source, false, disabled, &nav_focus[slot])
        };
        let mut controls = div().min_w_0().flex().flex_wrap().items_center().gap(gap);
        if self.first_last {
            controls = controls.child(navigation(
                2,
                1,
                PaginationChangeSource::First,
                "第一页",
                value.page() == 1,
            ));
        }
        controls = controls.child(navigation(
            0,
            value.page().saturating_sub(1),
            PaginationChangeSource::Previous,
            "上一页",
            base.previous_page().is_none(),
        ));
        if self.mode == PaginationMode::Pages {
            for item in items {
                controls = match item {
                    PaginationItem::Page(n) => {
                        let selected = n == value.page();
                        let name = if selected {
                            format!("第 {n} 页，当前页")
                        } else {
                            format!("第 {n} 页")
                        };
                        controls.child(command(
                            YueButton::new((id.clone(), format!("page-{n}")), n.to_string())
                                .accessibility_label(name),
                            n,
                            PaginationChangeSource::Page,
                            selected,
                            false,
                            &focus[&n],
                        ))
                    }
                    PaginationItem::Ellipsis(range) => {
                        if self.ellipsis_jump {
                            let backwards = range.end <= value.page();
                            let target = if backwards {
                                value.page().saturating_sub(self.visible - 2)
                            } else {
                                value.page().saturating_add(self.visible - 2)
                            }
                            .clamp(range.start, range.end - 1);
                            controls.child(navigation(
                                if backwards { 4 } else { 5 },
                                target,
                                if backwards {
                                    PaginationChangeSource::BackwardJump
                                } else {
                                    PaginationChangeSource::ForwardJump
                                },
                                &format!(
                                    "{}跳至第 {target} 页",
                                    if backwards { "向前" } else { "向后" }
                                ),
                                false,
                            ))
                        } else {
                            controls.child(
                                div()
                                    .id((
                                        id.clone(),
                                        if range.end <= value.page() {
                                            "backward-ellipsis"
                                        } else {
                                            "forward-ellipsis"
                                        },
                                    ))
                                    .test_support()
                                    .min_w(height)
                                    .h(height)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child("…"),
                            )
                        }
                    }
                };
            }
        } else if self.jumper {
            controls = controls
                .child(jumper(&self, theme, &input, blocked, invalid))
                .child(format!("/ {} 页", value.page_count()));
        } else {
            controls = controls.child(
                div()
                    .id((id.clone(), "position"))
                    .test_support()
                    .child(format!("{} / {} 页", value.page(), value.page_count())),
            );
        }
        controls = controls.child(navigation(
            1,
            value.page().saturating_add(1),
            PaginationChangeSource::Next,
            "下一页",
            base.next_page().is_none(),
        ));
        if self.first_last {
            controls = controls.child(navigation(
                3,
                value.page_count(),
                PaginationChangeSource::Last,
                "最后一页",
                value.page() == value.page_count(),
            ));
        }
        let mut content = div()
            .id((id.clone(), "content"))
            .test_support()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(rems(theme, "--pagination-group-gap"))
            .text_size(rems(
                theme,
                format!("--pagination-font-size-{}", self.size.suffix()).as_str(),
            ))
            .text_color(rgba(theme, "--pagination-color"));
        if self.total {
            let text = self.formatter.as_ref().map_or_else(
                || format!("共 {} 条", value.total()).into(),
                |format| format(value),
            );
            content = content.child(
                div()
                    .id((id.clone(), "total"))
                    .test_support()
                    .role(Role::Label)
                    .aria_label(text.clone())
                    .child(text),
            );
        }
        if self.selector {
            content = content.child(
                YueSelect::new((id.clone(), "page-size"), &select)
                    .theme(theme)
                    .size(self.size.select())
                    .width(rems(theme, "--pagination-selector-width"))
                    .accessibility_label("每页条数"),
            );
        }
        content = content.child(controls);
        if self.jumper && self.mode == PaginationMode::Pages {
            content = content.child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(gap)
                    .child("跳至")
                    .child(jumper(&self, theme, &input, blocked, invalid))
                    .child("页"),
            );
        }
        if loading {
            content = content.child(
                div()
                    .id((id.clone(), "loading"))
                    .test_support()
                    .role(Role::Status)
                    .aria_label("正在加载")
                    .child("正在加载"),
            );
        }
        self.layout
            .apply(
                Pagination::new(id, base)
                    .accessibility_label(self.name)
                    .min_w_0()
                    .child(content),
            )
            .into_any_element()
    }
}
fn jumper(
    p: &YuePagination,
    theme: YueTheme,
    input: &Entity<crate::InputState>,
    blocked: bool,
    invalid: bool,
) -> AnyElement {
    let state = p.state.clone();
    let editor = input.clone();
    let next_editor = input.clone();
    let previous_editor = input.clone();
    div()
        .id((p.id.clone(), "jump-group"))
        .key_context("YuePaginationJump")
        .on_action(move |_: &NextField, w, cx| {
            if !next_editor.update(cx, |s, cx| s.marked_text_range(w, cx).is_some()) {
                gpui_kit::base::Root::focus_next(w, cx);
            }
        })
        .on_action(move |_: &PreviousField, w, cx| {
            if !previous_editor.update(cx, |s, cx| s.marked_text_range(w, cx).is_some()) {
                gpui_kit::base::Root::focus_prev(w, cx);
            }
        })
        .test_support()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(rems(theme, "--pagination-item-gap"))
        .capture_key_down(move |event, w, cx| {
            if !editor.read(cx).presentation().focus_handle().is_focused(w)
                || event.keystroke.key != "escape"
                || editor.update(cx, |s, cx| s.marked_text_range(w, cx).is_some())
            {
                return;
            }
            state.update(cx, |s, cx| s.reset_draft(w, cx));
            cx.stop_propagation();
        })
        .child(
            YueInput::new((p.id.clone(), "jump"), theme, input)
                .size(p.size.input())
                .text_align(InputTextAlign::Center)
                .width(rems(theme, "--pagination-jumper-width"))
                .disabled(blocked)
                .status(if invalid {
                    InputStatus::Error
                } else {
                    InputStatus::Normal
                })
                .accessibility_label("跳转页码")
                .accessibility_description("输入正整数页码，Enter 确认，Escape 取消"),
        )
        .when(invalid, |d| {
            d.child(
                div()
                    .id((p.id.clone(), "jump-error"))
                    .test_support()
                    .role(Role::Alert)
                    .aria_label("请输入有效的正整数页码")
                    .text_color(rgba(theme, "--error-foreground"))
                    .child("请输入正整数"),
            )
        })
        .into_any_element()
}
