use super::*;
use crate::{
    theme::keys as k,
    tokens::{number, rems, rgba},
};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{
    App, FontWeight, InteractiveElement as _, ObjectFit, ParentElement as _, RenderOnce, Role,
    StatefulInteractiveElement as _, Styled as _, StyledImage as _, Window, div, img,
    prelude::FluentBuilder as _,
};

impl RenderOnce for YueEmpty {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let small = self.size == EmptySize::Small;
        let vertical = self.axis == Axis::Vertical;
        let id = self.id;
        let icon_size = rems(
            theme,
            if small {
                k::EMPTY_ICON_SIZE_SM
            } else {
                k::EMPTY_ICON_SIZE_MD
            },
        );
        let media_gap = rems(
            theme,
            if small {
                k::EMPTY_MEDIA_GAP_SM
            } else {
                k::EMPTY_MEDIA_GAP_MD
            },
        );
        let actions_gap = rems(
            theme,
            if small {
                k::EMPTY_ACTIONS_GAP_SM
            } else {
                k::EMPTY_ACTIONS_GAP_MD
            },
        );

        let has_media = !matches!(self.media, Media::Hidden);
        let media = match self.media {
            Media::Hidden => None,
            Media::Icon(icon) => Some(
                div()
                    .id((id.clone(), "media"))
                    .test_support()
                    .min_w_0()
                    .max_w_full()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(theme, k::EMPTY_ICON_COLOR))
                    .child(icon.size(icon_size))
                    .into_any_element(),
            ),
            Media::Image(source) => {
                let image_size = rems(
                    theme,
                    if small {
                        k::EMPTY_IMAGE_SIZE_SM
                    } else {
                        k::EMPTY_IMAGE_SIZE_MD
                    },
                );
                let fallback_id = (id.clone(), "image-fallback");
                let fallback = move || {
                    div()
                        .id(fallback_id.clone())
                        .test_support()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgba(theme, k::EMPTY_ICON_COLOR))
                        .child(Icon::new(YueIcon::Empty).size(icon_size))
                        .into_any_element()
                };
                Some(
                    div()
                        .id((id.clone(), "media"))
                        .test_support()
                        .w(image_size)
                        .max_w_full()
                        .min_w_0()
                        .aspect_ratio(1.)
                        .flex_shrink_0()
                        .child(
                            img(source)
                                .size_full()
                                .object_fit(ObjectFit::Contain)
                                .with_loading(|| div().size_full().into_any_element())
                                .with_fallback(fallback),
                        )
                        .into_any_element(),
                )
            }
            Media::Custom(content) => Some(
                div()
                    .id((id.clone(), "media"))
                    .test_support()
                    .min_w_0()
                    .max_w_full()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(content)
                    .into_any_element(),
            ),
        };
        let has_text = self.title.is_some() || self.description.is_some();
        let has_body = has_text || self.actions.is_some();
        let mut text = div()
            .id((id.clone(), "text"))
            .test_support()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(rems(theme, k::EMPTY_TEXT_GAP));
        if let Some(title) = self.title {
            text = text.child(
                div()
                    .id((id.clone(), "title"))
                    .test_support()
                    .min_w_0()
                    .w_full()
                    .role(Role::Label)
                    .aria_label(title.clone())
                    .text_color(rgba(theme, k::EMPTY_TITLE_COLOR))
                    .font_weight(FontWeight(number(theme, k::EMPTY_TITLE_WEIGHT)))
                    .child(title),
            );
        }
        if let Some(description) = self.description {
            let wrapper = div()
                .id((id.clone(), "description"))
                .test_support()
                .min_w_0()
                .w_full()
                .text_color(rgba(theme, k::EMPTY_DESCRIPTION_COLOR));
            text = text.child(match description {
                Description::Text(text) => wrapper
                    .role(Role::Label)
                    .aria_label(text.clone())
                    .child(text),
                Description::Custom(content) => wrapper.child(content),
            });
        }
        let mut body = div()
            .id((id.clone(), "body"))
            .test_support()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(actions_gap)
            .when(vertical, |d| d.w_full().items_center().text_center())
            .when(!vertical, |d| d.flex_1().items_start().text_left());
        if has_text {
            body = body.child(text);
        }
        if let Some(actions) = self.actions {
            body = body.child(
                div()
                    .id((id.clone(), "actions"))
                    .test_support()
                    .min_w_0()
                    .max_w_full()
                    .child(actions),
            );
        }
        let mut content = div()
            .id((id.clone(), "content"))
            .test_support()
            .w_full()
            .min_w_0()
            .max_w(rems(theme, k::EMPTY_CONTENT_MAX_WIDTH))
            .flex()
            .gap(media_gap)
            .when(vertical, |d| d.flex_col().items_center())
            .when(!vertical, |d| d.items_start());
        if let Some(media) = media {
            content = content.child(media);
        }
        if has_body {
            content = content.child(body);
        }
        let root = div()
            .id(id)
            .test_support()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .items_center()
            .flex_shrink_0()
            .font_family(crate::typography::ui_font(theme).family)
            .text_size(rems(
                theme,
                if small {
                    k::EMPTY_FONT_SIZE_SM
                } else {
                    k::EMPTY_FONT_SIZE_MD
                },
            ))
            .line_height(rems(
                theme,
                if small {
                    k::EMPTY_LINE_HEIGHT_SM
                } else {
                    k::EMPTY_LINE_HEIGHT_MD
                },
            ))
            .whitespace_normal()
            .when(self.inset, |d| {
                d.p(rems(
                    theme,
                    if small {
                        k::EMPTY_PADDING_SM
                    } else {
                        k::EMPTY_PADDING_MD
                    },
                ))
            });
        if has_body || has_media {
            root.child(content)
        } else {
            root
        }
    }
}
