use super::*;
use crate::tokens::{pixels, rems, rgba};
use gpui_kit::base::{Avatar as BaseAvatar, AvatarFallback, AvatarImage, TestSupportExt as _};
use gpui_kit::{
    App, InteractiveElement as _, IntoElement as _, ObjectFit, ParentElement as _, RenderOnce,
    Role, StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _,
};

impl RenderOnce for YueAvatar {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl gpui_kit::IntoElement {
        if self.use_default_theme {
            self.theme = crate::current_theme(cx);
        }
        let theme = self.theme;
        let size = rems(theme, self.size.token());
        let icon_size = size * 0.56;
        let fallback = self
            .fallback_text
            .clone()
            .or_else(|| self.name.as_deref().and_then(derive_fallback));
        let mut text_size = rems(theme, crate::theme::keys::AVATAR_FONT_SIZE);
        if self.auto_fit_font_size
            && let Some(text) = &fallback
        {
            let chars = text.chars().count().max(1) as f32;
            let available = pixels(theme, self.size.token()) * 0.78;
            let estimated = pixels(theme, crate::theme::keys::AVATAR_FONT_SIZE) * chars * 0.58;
            text_size = text_size * (available / estimated).clamp(0.55, 1.0);
        }
        let radius = match self.shape {
            AvatarShape::Circle => None,
            AvatarShape::Rounded => Some(rems(theme, crate::theme::keys::AVATAR_SHAPE_RADIUS)),
        };
        let shape = move |element: gpui_kit::Div| match radius {
            Some(radius) => element.rounded(radius),
            None => element.rounded_full(),
        };
        let bg = rgba(theme, crate::theme::keys::AVATAR_BACKGROUND);
        let fg = rgba(theme, crate::theme::keys::AVATAR_COLOR);
        let icon = self.placeholder;
        let fallback_id = (self.id.clone(), "fallback-content");
        let fallback_element = move || {
            let mut content = div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .bg(bg)
                .text_color(fg);
            content = if let Some(text) = fallback.clone() {
                content
                    .font_family(crate::typography::ui_font(theme).family)
                    .text_size(text_size)
                    .child(text)
            } else {
                content.child(icon.size(icon_size))
            };
            shape(content)
                .id(fallback_id.clone())
                .test_support()
                .into_any_element()
        };

        let mut avatar = BaseAvatar::new()
            .size(size)
            .flex_shrink_0()
            .overflow_hidden()
            .bg(bg)
            .text_color(fg)
            .fallback(AvatarFallback::new().size_full().child(fallback_element()));
        if let Some(radius) = radius {
            avatar = avatar.rounded(radius);
        } else {
            avatar = avatar.rounded_full();
        }
        if let Some(source) = self.source {
            avatar = avatar.image(
                AvatarImage::new(source)
                    .size(size)
                    .object_fit(ObjectFit::Cover)
                    .when_some(radius, |image, radius| image.rounded(radius))
                    .when(radius.is_none(), |image| image.rounded_full())
                    .with_fallback(fallback_element),
            );
        }
        let avatar = avatar.into_any_element();
        let root = div()
            .id((self.id, "avatar-semantics"))
            .size(size)
            .flex_shrink_0()
            .child(avatar);
        if self.semantics {
            root.role(Role::Image)
                .aria_label(
                    self.accessibility_label
                        .or(self.name)
                        .unwrap_or_else(|| "头像".into()),
                )
                .test_support()
                .into_any_element()
        } else {
            root.into_any_element()
        }
    }
}

impl RenderOnce for YueAvatarGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl gpui_kit::IntoElement {
        let id = self.id;
        let theme = self.theme;
        let avatars = self.avatars;
        let size = self.size;
        let shape = self.shape;
        let max = self.max;
        let overlap = self.overlap;
        let label = self.accessibility_label;
        if avatars.is_empty() {
            return div().into_any_element();
        }
        let total = avatars.len();
        let visible_count = if max == 0 { total } else { total.min(max) };
        let hidden_count = total - visible_count;
        let resolved_size = size.unwrap_or(AvatarSize::Medium);
        let resolved_shape = shape.unwrap_or(AvatarShape::Circle);
        let diameter = rems(theme, resolved_size.token());
        let step = diameter * (1. - overlap);
        let item_count = visible_count + usize::from(hidden_count > 0);
        let rim_width = rems(theme, crate::theme::keys::BORDER_2);
        let rim_color = rgba(theme, crate::theme::keys::SURFACE);
        let default_label: SharedString =
            format!("头像组，共 {total} 人，显示 {visible_count} 人").into();
        let mut group = div()
            .id((id.clone(), "avatar-group"))
            .relative()
            .flex_shrink_0()
            .w(diameter + step * (item_count - 1) as f32)
            .h(diameter)
            .role(Role::Group)
            .aria_label(label.unwrap_or(default_label))
            .aria_description(format!("共 {total} 人，显示 {visible_count} 人"));
        let mut children: Vec<gpui_kit::AnyElement> = avatars
            .into_iter()
            .take(visible_count)
            .map(|mut avatar| {
                avatar.size = resolved_size;
                avatar.shape = resolved_shape;
                avatar.theme = theme;
                avatar.use_default_theme = false;
                avatar.semantics = false;
                avatar.into_any_element()
            })
            .collect();
        if hidden_count > 0 {
            children.push(
                YueAvatar::new((id, "avatar-overflow"), theme)
                    .fallback_text(format!("+{hidden_count}"))
                    .size(resolved_size)
                    .shape(resolved_shape)
                    .semantics(false)
                    .into_any_element(),
            );
        }
        // Paint right to left while keeping input order on screen. The earlier
        // avatar and its rim must cover the following avatar at their overlap.
        for (ix, child) in children.into_iter().enumerate().rev() {
            let rim = div()
                .absolute()
                .inset_0()
                .border(rim_width)
                .border_color(rim_color)
                .when(resolved_shape == AvatarShape::Circle, |rim| {
                    rim.rounded_full()
                })
                .when(resolved_shape == AvatarShape::Rounded, |rim| {
                    rim.rounded(rems(theme, crate::theme::keys::AVATAR_SHAPE_RADIUS))
                });
            let item = div()
                .absolute()
                .left(step * ix as f32)
                .top_0()
                .size(diameter)
                .child(child)
                // Overlay the separator so image/fallback content cannot cover it.
                .child(rim);
            group = group.child(item);
        }
        group.test_support().into_any_element()
    }
}
