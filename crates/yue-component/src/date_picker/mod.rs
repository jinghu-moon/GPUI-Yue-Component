//! Retained calendar behavior composed with Yue Input, Popup, Button, Tag and Base TimeField.
mod behavior;
mod element;
pub(crate) use behavior::init;
mod model;
mod panel;
mod state;
pub use model::*;
pub use state::YueDatePickerState;
/// Calendar and picker share the same retained controller and event contract.
/// Use separate entities when mounting two independent controls.
pub type YueCalendarState = YueDatePickerState;

use crate::{InputSize, InputStatus, YueTheme};
use gpui_kit::base::{CalendarItem, CalendarItemState};
use gpui_kit::{AnyElement, App, Entity, IntoElement, RenderOnce, SharedString, Window};
use std::rc::Rc;
type DayRenderer = Rc<dyn Fn(CalendarItem, CalendarItemState, &mut Window, &mut App) -> AnyElement>;
type FooterRenderer = Rc<dyn Fn(bool, &mut Window, &mut App) -> AnyElement>;
#[derive(Clone)]
pub(super) struct Presentation {
    calendar_only: bool,
    theme: Option<YueTheme>,
    size: InputSize,
    status: InputStatus,
    editable: bool,
    clearable: bool,
    readonly: bool,
    disabled: bool,
    bordered: bool,
    placeholder: SharedString,
    name: SharedString,
    description: Option<SharedString>,
    required: bool,
    presets: Vec<DatePickerPreset>,
    day: Option<DayRenderer>,
    footer: Option<FooterRenderer>,
    show_today: bool,
    block: bool,
    leading: Option<crate::Icon>,
    cell_height: Option<gpui_kit::Rems>,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            calendar_only: false,
            theme: None,
            size: InputSize::Medium,
            status: InputStatus::Normal,
            editable: true,
            clearable: true,
            readonly: false,
            disabled: false,
            bordered: true,
            placeholder: "请选择日期".into(),
            name: "日期".into(),
            description: None,
            required: false,
            presets: vec![],
            day: None,
            footer: None,
            show_today: true,
            block: false,
            leading: None,
            cell_height: None,
        }
    }
}
#[derive(IntoElement)]
pub struct YueDatePicker {
    state: Entity<YueDatePickerState>,
    presentation: Presentation,
}
impl YueDatePicker {
    pub fn accessibility_description(mut self, value: impl Into<SharedString>) -> Self {
        self.presentation.description = Some(value.into());
        self
    }
    pub fn required(mut self, value: bool) -> Self {
        self.presentation.required = value;
        self
    }
    pub fn leading_icon(mut self, icon: impl crate::IconDefinition) -> Self {
        self.presentation.leading = Some(crate::Icon::new(icon));
        self
    }
    /// Uniform height for every date cell, for example when rendering prices.
    pub fn cell_height(mut self, height: gpui_kit::Rems) -> Self {
        assert!(height.0.is_finite() && height.0 > 0.);
        self.presentation.cell_height = Some(height);
        self
    }
    pub fn new(state: &Entity<YueDatePickerState>) -> Self {
        Self {
            state: state.clone(),
            presentation: Default::default(),
        }
    }
    pub fn theme(mut self, value: YueTheme) -> Self {
        self.presentation.theme = Some(value);
        self
    }
    pub fn size(mut self, value: InputSize) -> Self {
        self.presentation.size = value;
        self
    }
    pub fn status(mut self, value: InputStatus) -> Self {
        self.presentation.status = value;
        self
    }
    pub fn editable(mut self, value: bool) -> Self {
        self.presentation.editable = value;
        self
    }
    pub fn clearable(mut self, value: bool) -> Self {
        self.presentation.clearable = value;
        self
    }
    pub fn readonly(mut self, value: bool) -> Self {
        self.presentation.readonly = value;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.presentation.disabled = value;
        self
    }
    pub fn bordered(mut self, value: bool) -> Self {
        self.presentation.bordered = value;
        self
    }
    pub fn block(mut self) -> Self {
        self.presentation.block = true;
        self
    }
    pub fn placeholder(mut self, value: impl Into<SharedString>) -> Self {
        self.presentation.placeholder = value.into();
        self
    }
    pub fn accessibility_label(mut self, value: impl Into<SharedString>) -> Self {
        self.presentation.name = value.into();
        self
    }
    pub fn presets(mut self, value: impl IntoIterator<Item = DatePickerPreset>) -> Self {
        self.presentation.presets = value.into_iter().collect();
        self
    }
    pub fn show_today(mut self, value: bool) -> Self {
        self.presentation.show_today = value;
        self
    }
    /// Decorate the pre-wired Base day item; preserve its root, semantics and events.
    pub fn render_day(
        mut self,
        render: impl Fn(CalendarItem, CalendarItemState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.presentation.day = Some(Rc::new(render));
        self
    }
    /// Extra footer content. Disable any interactive children when the argument is false.
    pub fn footer<E: IntoElement>(
        mut self,
        render: impl Fn(bool, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.presentation.footer = Some(Rc::new(move |active, w, cx| {
            render(active, w, cx).into_any_element()
        }));
        self
    }
}
impl RenderOnce for YueDatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, cx| state.prepare(self.presentation, window, cx));
        self.state
    }
}
#[derive(IntoElement)]
pub struct YueCalendar {
    picker: YueDatePicker,
}
impl YueCalendar {
    pub fn cell_height(mut self, height: gpui_kit::Rems) -> Self {
        self.picker = self.picker.cell_height(height);
        self
    }
    pub fn new(state: &Entity<YueCalendarState>) -> Self {
        let mut picker = YueDatePicker::new(state);
        picker.presentation.calendar_only = true;
        Self { picker }
    }
    pub fn theme(mut self, value: YueTheme) -> Self {
        self.picker = self.picker.theme(value);
        self
    }
    pub fn size(mut self, value: InputSize) -> Self {
        self.picker = self.picker.size(value);
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.picker = self.picker.disabled(value);
        self
    }
    pub fn readonly(mut self, value: bool) -> Self {
        self.picker = self.picker.readonly(value);
        self
    }
    pub fn show_today(mut self, value: bool) -> Self {
        self.picker = self.picker.show_today(value);
        self
    }
    pub fn render_day(
        mut self,
        render: impl Fn(CalendarItem, CalendarItemState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.picker = self.picker.render_day(render);
        self
    }
    pub fn footer<E: IntoElement>(
        mut self,
        render: impl Fn(bool, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.picker = self.picker.footer(render);
        self
    }
}
impl RenderOnce for YueCalendar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.picker
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests;
