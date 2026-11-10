//! Key/value record presentation, composed from ordinary layout and existing controls.
mod element;
mod model;
use crate::YueTheme;
use gpui_kit::{AbsoluteLength, AnyElement, ElementId, IntoElement, SharedString};
pub use model::{
    DescriptionsAlign, DescriptionsLayout, DescriptionsSize, DescriptionsSpan,
    DescriptionsTextOverflow, DescriptionsVerticalAlign,
};

enum Value {
    Text(SharedString),
    Content(AnyElement),
}
/// A domain-keyed attribute. The textual label also names a custom visual label.
pub struct YueDescriptionItem {
    key: SharedString,
    label: SharedString,
    label_content: Option<AnyElement>,
    value: Value,
    span: DescriptionsSpan,
    overflow: DescriptionsTextOverflow,
    copyable: bool,
    copy_text: Option<SharedString>,
    vertical: Option<DescriptionsVerticalAlign>,
}
impl YueDescriptionItem {
    /// Panics if key or label is blank. Keys must be unique within the group.
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let (key, label) = (key.into(), label.into());
        assert!(
            !key.trim().is_empty() && !label.trim().is_empty(),
            "Descriptions key and label must not be blank"
        );
        Self {
            key,
            label,
            label_content: None,
            value: Value::Text("".into()),
            span: Default::default(),
            overflow: Default::default(),
            copyable: false,
            copy_text: None,
            vertical: None,
        }
    }
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn text(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Value::Text(value.into());
        self
    }
    pub fn content(mut self, value: impl IntoElement) -> Self {
        self.value = Value::Content(value.into_any_element());
        self
    }
    pub fn label_content(mut self, value: impl IntoElement) -> Self {
        self.label_content = Some(value.into_any_element());
        self
    }
    pub fn span(mut self, span: DescriptionsSpan) -> Self {
        assert!(
            span != DescriptionsSpan::Columns(0),
            "Descriptions span must be positive"
        );
        self.span = span;
        self
    }
    pub fn text_overflow(mut self, overflow: DescriptionsTextOverflow) -> Self {
        assert!(
            overflow != DescriptionsTextOverflow::Lines(0),
            "Descriptions line count must be positive"
        );
        self.overflow = overflow;
        self
    }
    pub fn copyable(mut self, enabled: bool) -> Self {
        self.copyable = enabled;
        self
    }
    pub fn copy_text(mut self, text: impl Into<SharedString>) -> Self {
        self.copy_text = Some(text.into());
        self.copyable = true;
        self
    }
    pub fn vertical_align(mut self, align: DescriptionsVerticalAlign) -> Self {
        self.vertical = Some(align);
        self
    }
    fn validate(&self) {
        if matches!(self.value, Value::Content(_)) {
            assert!(
                self.overflow == DescriptionsTextOverflow::Wrap,
                "Descriptions custom content owns its overflow"
            );
            assert!(
                !self.copyable || self.copy_text.is_some(),
                "Descriptions copyable content requires copy_text"
            );
        }
    }
}
/// Read-only description structure; the host retains editor entities and record state.
///
/// ```
/// use yue_component::{YueDescriptions, YueDescriptionItem, DescriptionsSpan};
/// let details = YueDescriptions::new("record").bordered(true).items([
///     YueDescriptionItem::new("id", "编号").text("WO-42").copyable(true),
///     YueDescriptionItem::new("note", "备注").text("可自然换行").span(DescriptionsSpan::Filled),
/// ]);
/// ```
#[derive(IntoElement)]
pub struct YueDescriptions {
    id: ElementId,
    items: Vec<YueDescriptionItem>,
    title: Option<SharedString>,
    extra: Option<AnyElement>,
    columns: u16,
    min_item_width: Option<AbsoluteLength>,
    label_width: Option<AbsoluteLength>,
    orientation: DescriptionsLayout,
    bordered: bool,
    size: DescriptionsSize,
    label_align: DescriptionsAlign,
    value_align: DescriptionsAlign,
    vertical: DescriptionsVerticalAlign,
    colon: bool,
    theme: Option<YueTheme>,
    name: SharedString,
    layout: crate::control_layout::ControlLayout,
}
crate::control_layout::control_layout!(YueDescriptions);
impl YueDescriptions {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: vec![],
            title: None,
            extra: None,
            columns: 3,
            min_item_width: None,
            label_width: None,
            orientation: Default::default(),
            bordered: false,
            size: Default::default(),
            label_align: Default::default(),
            value_align: Default::default(),
            vertical: Default::default(),
            colon: false,
            theme: None,
            name: "描述列表".into(),
            layout: Default::default(),
        }
    }
    /// Panics when keys repeat. Invalid final content/overflow combinations panic at render.
    pub fn items(mut self, items: impl IntoIterator<Item = YueDescriptionItem>) -> Self {
        self.items = items.into_iter().collect();
        let mut keys = std::collections::HashSet::new();
        assert!(
            self.items.iter().all(|i| keys.insert(i.key.clone())),
            "Descriptions keys must be unique"
        );
        self
    }
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub fn extra(mut self, extra: impl IntoElement) -> Self {
        self.extra = Some(extra.into_any_element());
        self
    }
    pub fn columns(mut self, columns: u16) -> Self {
        assert!(columns > 0, "Descriptions columns must be positive");
        self.columns = columns;
        self
    }
    pub fn min_item_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        let width = width.into();
        valid_length(width, true);
        self.min_item_width = Some(width);
        self
    }
    pub fn label_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        let width = width.into();
        valid_length(width, false);
        self.label_width = Some(width);
        self
    }
    pub fn layout(mut self, layout: DescriptionsLayout) -> Self {
        self.orientation = layout;
        self
    }
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
    pub fn size(mut self, size: DescriptionsSize) -> Self {
        self.size = size;
        self
    }
    pub fn label_align(mut self, align: DescriptionsAlign) -> Self {
        self.label_align = align;
        self
    }
    pub fn value_align(mut self, align: DescriptionsAlign) -> Self {
        self.value_align = align;
        self
    }
    pub fn vertical_align(mut self, align: DescriptionsVerticalAlign) -> Self {
        self.vertical = align;
        self
    }
    pub fn colon(mut self, colon: bool) -> Self {
        self.colon = colon;
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn accessibility_label(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        assert!(!name.trim().is_empty());
        self.name = name;
        self
    }
}
fn valid_length(length: AbsoluteLength, positive: bool) {
    let value = f32::from(length.to_pixels(gpui_kit::px(1.)));
    assert!(
        value.is_finite() && if positive { value > 0. } else { value >= 0. },
        "Descriptions length is invalid"
    );
}
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
