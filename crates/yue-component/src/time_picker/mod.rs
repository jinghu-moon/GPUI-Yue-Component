//! Time inputs and scrolling panels; hosts retain one controller per mounted control.
mod behavior;
mod element;
mod model;
mod panel;
mod scrolling;
mod state;
use crate::{InputSize, InputStatus, YueTheme};
pub(crate) use behavior::init;
use gpui_kit::{AnyElement, App, Entity, IntoElement, RenderOnce, SharedString, Window};
pub use model::*;
pub use state::YueTimePickerState;
use std::rc::Rc;
type Footer = Rc<dyn Fn(bool, &mut Window, &mut App) -> AnyElement>;
#[derive(Clone)]
pub(super) struct Presentation {
    panel_only: bool,
    theme: Option<YueTheme>,
    size: InputSize,
    status: InputStatus,
    editable: bool,
    readonly: bool,
    disabled: bool,
    clearable: bool,
    bordered: bool,
    block: bool,
    hide_disabled: bool,
    show_now: bool,
    leading: Option<crate::Icon>,
    placeholder: SharedString,
    range_placeholders: [SharedString; 2],
    name: SharedString,
    description: Option<SharedString>,
    required: bool,
    footer: Option<Footer>,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            panel_only: false,
            theme: None,
            size: InputSize::Medium,
            status: InputStatus::Normal,
            editable: true,
            readonly: false,
            disabled: false,
            clearable: true,
            bordered: true,
            block: false,
            hide_disabled: false,
            show_now: true,
            leading: None,
            placeholder: "请选择时间".into(),
            range_placeholders: ["开始时间".into(), "结束时间".into()],
            name: "时间".into(),
            description: None,
            required: false,
            footer: None,
        }
    }
}
#[derive(IntoElement)]
pub struct YueTimePicker {
    state: Entity<YueTimePickerState>,
    presentation: Presentation,
}
impl YueTimePicker {
    pub fn new(state: &Entity<YueTimePickerState>) -> Self {
        Self {
            state: state.clone(),
            presentation: Default::default(),
        }
    }
    pub fn theme(mut self, v: YueTheme) -> Self {
        self.presentation.theme = Some(v);
        self
    }
    pub fn size(mut self, v: InputSize) -> Self {
        self.presentation.size = v;
        self
    }
    pub fn status(mut self, v: InputStatus) -> Self {
        self.presentation.status = v;
        self
    }
    pub fn editable(mut self, v: bool) -> Self {
        self.presentation.editable = v;
        self
    }
    pub fn readonly(mut self, v: bool) -> Self {
        self.presentation.readonly = v;
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.presentation.disabled = v;
        self
    }
    pub fn clearable(mut self, v: bool) -> Self {
        self.presentation.clearable = v;
        self
    }
    pub fn bordered(mut self, v: bool) -> Self {
        self.presentation.bordered = v;
        self
    }
    pub fn block(mut self) -> Self {
        self.presentation.block = true;
        self
    }
    pub fn hide_disabled(mut self, v: bool) -> Self {
        self.presentation.hide_disabled = v;
        self
    }
    pub fn show_now(mut self, v: bool) -> Self {
        self.presentation.show_now = v;
        self
    }
    pub fn placeholder(mut self, v: impl Into<SharedString>) -> Self {
        self.presentation.placeholder = v.into();
        self
    }
    pub fn range_placeholders(
        mut self,
        a: impl Into<SharedString>,
        b: impl Into<SharedString>,
    ) -> Self {
        self.presentation.range_placeholders = [a.into(), b.into()];
        self
    }
    pub fn accessibility_label(mut self, v: impl Into<SharedString>) -> Self {
        self.presentation.name = v.into();
        self
    }
    pub fn accessibility_description(mut self, v: impl Into<SharedString>) -> Self {
        self.presentation.description = Some(v.into());
        self
    }
    pub fn required(mut self, v: bool) -> Self {
        self.presentation.required = v;
        self
    }
    pub fn leading_icon(mut self, v: impl crate::IconDefinition) -> Self {
        self.presentation.leading = Some(crate::Icon::new(v));
        self
    }
    pub fn footer<E: IntoElement>(
        mut self,
        f: impl Fn(bool, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.presentation.footer = Some(Rc::new(move |a, w, cx| f(a, w, cx).into_any_element()));
        self
    }
}
impl RenderOnce for YueTimePicker {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |s, cx| s.prepare(self.presentation, w, cx));
        self.state
    }
}
#[derive(IntoElement)]
pub struct YueTimePanel {
    picker: YueTimePicker,
}
impl YueTimePanel {
    pub fn new(state: &Entity<YueTimePickerState>) -> Self {
        let mut picker = YueTimePicker::new(state);
        picker.presentation.panel_only = true;
        Self { picker }
    }
    pub fn theme(mut self, v: YueTheme) -> Self {
        self.picker = self.picker.theme(v);
        self
    }
    pub fn size(mut self, v: InputSize) -> Self {
        self.picker = self.picker.size(v);
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.picker = self.picker.disabled(v);
        self
    }
    pub fn readonly(mut self, v: bool) -> Self {
        self.picker = self.picker.readonly(v);
        self
    }
    pub fn hide_disabled(mut self, v: bool) -> Self {
        self.picker = self.picker.hide_disabled(v);
        self
    }
    pub fn show_now(mut self, v: bool) -> Self {
        self.picker = self.picker.show_now(v);
        self
    }
    pub fn footer<E: IntoElement>(
        mut self,
        f: impl Fn(bool, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.picker = self.picker.footer(f);
        self
    }
}
impl RenderOnce for YueTimePanel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.picker
    }
}
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
