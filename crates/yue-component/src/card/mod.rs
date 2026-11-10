//! Related content on a themed surface. Hosts own data, commands and selection.
mod element;

use crate::{YueButton, YueTheme};
use gpui_kit::{AbsoluteLength, AnyElement, DefiniteLength, ElementId, IntoElement, SharedString};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CardVariant {
    #[default]
    Outlined,
    Elevated,
    Filled,
    Borderless,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CardSize {
    Small,
    #[default]
    Medium,
    Large,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CardHoverEffect {
    #[default]
    None,
    Outline,
    Lift,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CardCoverPlacement {
    #[default]
    BeforeHeader,
    AfterHeader,
    Left,
    Right,
}

/// A non-focusable content container. Arbitrary children keep their own behavior.
///
/// ```
/// use yue_component::{YueCard, YueCardHeader, YueCardFooter, YueButton};
/// let card = YueCard::new("work-order")
///     .header(YueCardHeader::new("6205-ZZ").description("今日排产"))
///     .child("轴承工单")
///     .footer(YueCardFooter::new(YueButton::new("details", "详情…")));
/// ```
#[derive(IntoElement)]
pub struct YueCard {
    id: ElementId,
    theme: Option<YueTheme>,
    size: CardSize,
    variant: CardVariant,
    hover: Option<CardHoverEffect>,
    selected: bool,
    label: Option<SharedString>,
    header: Option<YueCardHeader>,
    cover: Option<YueCardCover>,
    body: Vec<AnyElement>,
    body_inset: bool,
    footer: Option<YueCardFooter>,
    primary: Option<YueButton>,
    layout: crate::control_layout::ControlLayout,
}
crate::control_layout::control_layout!(YueCard);
impl YueCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            theme: None,
            size: CardSize::Medium,
            variant: CardVariant::Outlined,
            hover: None,
            selected: false,
            label: None,
            header: None,
            cover: None,
            body: Vec::new(),
            body_inset: true,
            footer: None,
            primary: None,
            layout: Default::default(),
        }
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn size(mut self, size: CardSize) -> Self {
        self.size = size;
        self
    }
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn hover_effect(mut self, effect: CardHoverEffect) -> Self {
        self.hover = Some(effect);
        self
    }
    /// Visual state only. Supply the same host-owned value to a real selection control.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        assert!(!label.trim().is_empty());
        self.label = Some(label);
        self
    }
    pub fn header(mut self, header: YueCardHeader) -> Self {
        self.header = Some(header);
        self
    }
    pub fn cover(mut self, cover: YueCardCover) -> Self {
        self.cover = Some(cover);
        self
    }
    pub fn body_inset(mut self, inset: bool) -> Self {
        self.body_inset = inset;
        self
    }
    pub fn child(mut self, content: impl IntoElement) -> Self {
        self.body.push(content.into_any_element());
        self
    }
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.body
            .extend(children.into_iter().map(IntoElement::into_any_element));
        self
    }
    pub fn footer(mut self, footer: YueCardFooter) -> Self {
        self.footer = Some(footer);
        self
    }
    /// Extend a visible footer command to static title, body and cover regions.
    /// Do not use with interactive body/cover children; use a normal footer Button instead.
    /// Custom headers, leading/extra slots and other footer actions never activate it.
    /// Keyboard activation, focus, disabled and loading remain owned by this Button.
    pub fn primary_action(mut self, action: YueButton) -> Self {
        self.primary = Some(action);
        self
    }
}

pub struct YueCardHeader {
    content: HeaderContent,
    divided: bool,
}
enum HeaderContent {
    Standard {
        title: SharedString,
        description: Option<SharedString>,
        leading: Option<AnyElement>,
        extra: Option<AnyElement>,
        truncate: bool,
    },
    Custom(AnyElement),
}
impl YueCardHeader {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            content: HeaderContent::Standard {
                title: title.into(),
                description: None,
                leading: None,
                extra: None,
                truncate: false,
            },
            divided: false,
        }
    }
    pub fn custom(content: impl IntoElement) -> Self {
        Self {
            content: HeaderContent::Custom(content.into_any_element()),
            divided: false,
        }
    }
    fn standard(
        &mut self,
    ) -> (
        &mut Option<SharedString>,
        &mut Option<AnyElement>,
        &mut Option<AnyElement>,
        &mut bool,
    ) {
        match &mut self.content {
            HeaderContent::Standard {
                description,
                leading,
                extra,
                truncate,
                ..
            } => (description, leading, extra, truncate),
            HeaderContent::Custom(_) => {
                panic!("custom Card header does not accept standard fields")
            }
        }
    }
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        *self.standard().0 = Some(text.into());
        self
    }
    pub fn leading(mut self, content: impl IntoElement) -> Self {
        *self.standard().1 = Some(content.into_any_element());
        self
    }
    pub fn extra(mut self, content: impl IntoElement) -> Self {
        *self.standard().2 = Some(content.into_any_element());
        self
    }
    pub fn truncate_title(mut self, truncate: bool) -> Self {
        *self.standard().3 = truncate;
        self
    }
    pub fn divided(mut self, divided: bool) -> Self {
        self.divided = divided;
        self
    }
}

pub struct YueCardCover {
    content: Box<dyn FnOnce(gpui_kit::Corners<gpui_kit::Rems>) -> AnyElement>,
    placement: CardCoverPlacement,
    geometry: CoverGeometry,
    side_width: Option<DefiniteLength>,
}
enum CoverGeometry {
    Ratio(f32),
    Height(AbsoluteLength),
}
impl YueCardCover {
    /// Media must expose its root style so its visible corners can follow the Card.
    /// Nested backgrounds and image fallbacks must also respect that shape.
    pub fn new(content: impl IntoElement + gpui_kit::Styled + 'static) -> Self {
        Self {
            content: Box::new(move |corners| {
                content
                    .rounded_tl(corners.top_left)
                    .rounded_tr(corners.top_right)
                    .rounded_bl(corners.bottom_left)
                    .rounded_br(corners.bottom_right)
                    .into_any_element()
            }),
            placement: CardCoverPlacement::BeforeHeader,
            geometry: CoverGeometry::Ratio(16. / 9.),
            side_width: None,
        }
    }
    pub fn placement(mut self, placement: CardCoverPlacement) -> Self {
        self.placement = placement;
        self
    }
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        assert!(ratio.is_finite() && ratio > 0.);
        self.geometry = CoverGeometry::Ratio(ratio);
        self
    }
    pub fn height(mut self, height: impl Into<AbsoluteLength>) -> Self {
        let height = height.into();
        let px = height.to_pixels(gpui_kit::px(16.));
        assert!(f32::from(px).is_finite() && px >= gpui_kit::px(0.));
        self.geometry = CoverGeometry::Height(height);
        self
    }
    pub fn side_width(mut self, width: impl Into<DefiniteLength>) -> Self {
        let width = width.into();
        let valid = match width {
            DefiniteLength::Absolute(x) => {
                let v = f32::from(x.to_pixels(gpui_kit::px(16.)));
                v.is_finite() && v > 0.
            }
            DefiniteLength::Fraction(x) => x.is_finite() && x > 0. && x < 1.,
        };
        assert!(
            valid,
            "Card side width must be positive and finite; relative widths must be below one"
        );
        self.side_width = Some(width);
        self
    }
}

/// A non-interactive title/description row with an optional leading element.
#[derive(IntoElement)]
pub struct YueCardMeta {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    leading: Option<AnyElement>,
    truncate: bool,
    theme: Option<YueTheme>,
    size: CardSize,
}
impl YueCardMeta {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            leading: None,
            truncate: false,
            theme: None,
            size: CardSize::Medium,
        }
    }
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }
    pub fn leading(mut self, content: impl IntoElement) -> Self {
        self.leading = Some(content.into_any_element());
        self
    }
    pub fn truncate_title(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn size(mut self, size: CardSize) -> Self {
        self.size = size;
        self
    }
}

pub struct YueCardFooter {
    content: FooterContent,
    divided: bool,
    inset: bool,
}
enum FooterContent {
    Custom(AnyElement),
    Actions(Vec<YueButton>),
}
impl YueCardFooter {
    pub fn new(content: impl IntoElement) -> Self {
        Self {
            content: FooterContent::Custom(content.into_any_element()),
            divided: true,
            inset: true,
        }
    }
    pub fn actions(actions: impl IntoIterator<Item = YueButton>) -> Self {
        Self {
            content: FooterContent::Actions(actions.into_iter().collect()),
            divided: true,
            inset: true,
        }
    }
    pub fn divided(mut self, divided: bool) -> Self {
        self.divided = divided;
        self
    }
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests;
