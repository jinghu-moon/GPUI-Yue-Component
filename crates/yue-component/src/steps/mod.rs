//! Controlled workflow steps with token-owned presentation and Base button behavior.
mod element;
mod model;
use crate::{Icon, IconDefinition, YueTheme};
use gpui_kit::{AnyElement, App, ElementId, IntoElement, SharedString, Window};
pub use model::{
    StepsChange, StepsLabelPlacement, StepsLayout, StepsSize, StepsStatus, StepsValue, StepsVariant,
};
use std::rc::Rc;

type ChangeHandler = Rc<dyn Fn(&StepsChange, &mut Window, &mut App)>;

/// A stable workflow step. Extra content is outside the step activation button.
pub struct YueStepItem {
    key: SharedString,
    title: SharedString,
    description: Option<SharedString>,
    subtitle: Option<SharedString>,
    icon: Option<Icon>,
    status: Option<StepsStatus>,
    disabled: bool,
    extra: Option<AnyElement>,
}
impl YueStepItem {
    /// # Panics
    /// Panics when key or title is blank. Keys must also be unique in the group.
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        let key = key.into();
        let title = title.into();
        assert!(
            !key.trim().is_empty() && !title.trim().is_empty(),
            "Steps key and title must not be blank"
        );
        Self {
            key,
            title,
            description: None,
            subtitle: None,
            icon: None,
            status: None,
            disabled: false,
            extra: None,
        }
    }
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn title(&self) -> &SharedString {
        &self.title
    }
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }
    pub fn subtitle(mut self, text: impl Into<SharedString>) -> Self {
        self.subtitle = Some(text.into());
        self
    }
    pub fn icon(mut self, icon: impl IconDefinition) -> Self {
        self.icon = Some(Icon::new(icon));
        self
    }
    pub fn status(mut self, status: StepsStatus) -> Self {
        self.status = Some(status);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn extra(mut self, content: impl IntoElement) -> Self {
        self.extra = Some(content.into_any_element());
        self
    }
}

/// The host owns workflow position, validation and content. A callback requests
/// a key; supply that key on the next render to accept it. No callback is static.
///
/// ```
/// use yue_component::{YueSteps, YueStepItem, StepsValue};
/// let steps = YueSteps::new("import", "configure")
///     .items([
///         YueStepItem::new("source", "Select source"),
///         YueStepItem::new("configure", "Configure import").description("Check the columns"),
///         YueStepItem::new("result", "Import result"),
///     ]).on_change(|request, _, _| { let _ = request.key(); });
/// let finished = YueSteps::new("finished", StepsValue::Complete)
///     .items([YueStepItem::new("done", "Imported")]);
/// ```
#[derive(IntoElement)]
pub struct YueSteps {
    id: ElementId,
    value: StepsValue,
    items: Vec<YueStepItem>,
    theme: Option<YueTheme>,
    size: StepsSize,
    orientation: StepsLayout,
    variant: StepsVariant,
    placement: StepsLabelPlacement,
    status: StepsStatus,
    reverse: bool,
    readonly: bool,
    disabled: bool,
    name: SharedString,
    handler: Option<ChangeHandler>,
    layout: crate::control_layout::ControlLayout,
}
crate::control_layout::control_layout!(YueSteps);
impl YueSteps {
    pub fn new(id: impl Into<ElementId>, value: impl Into<StepsValue>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            items: vec![],
            theme: None,
            size: StepsSize::Medium,
            orientation: StepsLayout::Horizontal,
            variant: StepsVariant::Default,
            placement: StepsLabelPlacement::Right,
            status: StepsStatus::Process,
            reverse: false,
            readonly: false,
            disabled: false,
            name: "步骤".into(),
            handler: None,
            layout: Default::default(),
        }
    }
    /// # Panics
    /// Panics when keys repeat. A Current key must exist when rendered.
    pub fn items(mut self, items: impl IntoIterator<Item = YueStepItem>) -> Self {
        self.items = items.into_iter().collect();
        for (i, item) in self.items.iter().enumerate() {
            assert!(
                !self.items[..i].iter().any(|old| old.key == item.key),
                "Steps keys must be unique"
            );
        }
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn size(mut self, size: StepsSize) -> Self {
        self.size = size;
        self
    }
    /// Navigation and Arrow require Horizontal + Right; invalid combinations panic at render.
    pub fn layout(mut self, layout: StepsLayout) -> Self {
        self.orientation = layout;
        self
    }
    pub fn variant(mut self, variant: StepsVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn label_placement(mut self, placement: StepsLabelPlacement) -> Self {
        self.placement = placement;
        self
    }
    pub fn status(mut self, status: StepsStatus) -> Self {
        self.status = status;
        self
    }
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// # Panics
    /// Panics when the name is blank.
    pub fn accessibility_label(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        assert!(!name.trim().is_empty());
        self.name = name;
        self
    }
    pub fn on_change(
        mut self,
        handler: impl Fn(&StepsChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Some(Rc::new(handler));
        self
    }
    fn validate(&self) -> Option<usize> {
        assert!(
            matches!(self.variant, StepsVariant::Default | StepsVariant::Dot)
                || (self.orientation == StepsLayout::Horizontal
                    && self.placement == StepsLabelPlacement::Right),
            "Navigation and Arrow Steps require Horizontal + Right"
        );
        self.value.key().map(|key| {
            self.items
                .iter()
                .position(|i| &i.key == key)
                .expect("Steps current key must exist")
        })
    }
}
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
