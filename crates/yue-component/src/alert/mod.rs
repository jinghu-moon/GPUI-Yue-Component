//! Controlled inline feedback, composed from Base lifecycle and Yue controls.
mod element;
use crate::{YueTheme, control_layout::ControlLayout};
pub use gpui_kit::base::NoticeCloseReason as AlertCloseReason;
use gpui_kit::{
    AnyElement, App, ElementId, FocusHandle, IntoElement, ParentElement, SharedString, Window,
};
use std::{rc::Rc, time::Duration};
use yue_component_icons::{Icon, IconDefinition};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlertKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlertVariant {
    #[default]
    Tonal,
    Outlined,
    Solid,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlertSize {
    Small,
    #[default]
    Medium,
}
/// Native semantic urgency. Quiet is useful for historical, persistent content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlertAnnouncement {
    Quiet,
    Polite,
    Assertive,
}
type CloseHandler = Rc<dyn Fn(AlertCloseReason, &mut Window, &mut App)>;
type Handler = Rc<dyn Fn(&mut Window, &mut App)>;
type ExpandHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// The host owns visibility and expansion. Keep this declaration mounted until
/// `on_closed` if an animated exit and its completion notification are required.
#[derive(IntoElement)]
pub struct YueAlert {
    id: ElementId,
    message: SharedString,
    theme: Option<YueTheme>,
    kind: AlertKind,
    variant: AlertVariant,
    size: AlertSize,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    action: Option<AnyElement>,
    show_icon: bool,
    icon: Option<Icon>,
    banner: bool,
    closable: bool,
    close_text: Option<SharedString>,
    close_icon: Option<Icon>,
    close_label: SharedString,
    visible: bool,
    on_close: Option<CloseHandler>,
    on_closed: Option<Handler>,
    max_lines: Option<usize>,
    expanded: bool,
    on_expanded: Option<ExpandHandler>,
    expand_label: SharedString,
    collapse_label: SharedString,
    duration: Option<Duration>,
    focus_after: Option<FocusHandle>,
    announcement: Option<AlertAnnouncement>,
    layout: ControlLayout,
}
crate::control_layout::control_layout!(YueAlert);
impl YueAlert {
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        let message = message.into();
        assert!(
            !message.trim().is_empty(),
            "YueAlert message must be nonblank"
        );
        Self {
            id: id.into(),
            message,
            theme: None,
            kind: AlertKind::Info,
            variant: AlertVariant::Tonal,
            size: AlertSize::Medium,
            description: None,
            children: vec![],
            action: None,
            show_icon: true,
            icon: None,
            banner: false,
            closable: false,
            close_text: None,
            close_icon: None,
            close_label: "关闭提示".into(),
            visible: true,
            on_close: None,
            on_closed: None,
            max_lines: None,
            expanded: false,
            on_expanded: None,
            expand_label: "展开".into(),
            collapse_label: "收起".into(),
            duration: None,
            focus_after: None,
            announcement: None,
            layout: ControlLayout::default(),
        }
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn use_default_theme(mut self) -> Self {
        self.theme = None;
        self
    }
    pub fn kind(mut self, value: AlertKind) -> Self {
        self.kind = value;
        self
    }
    pub fn variant(mut self, value: AlertVariant) -> Self {
        self.variant = value;
        self
    }
    pub fn size(mut self, value: AlertSize) -> Self {
        self.size = value;
        self
    }
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        self.description = (!text.trim().is_empty()).then_some(text);
        self
    }
    pub fn action(mut self, content: impl IntoElement) -> Self {
        self.action = Some(content.into_any_element());
        self
    }
    pub fn show_icon(mut self, value: bool) -> Self {
        self.show_icon = value;
        self
    }
    pub fn icon(mut self, icon: impl IconDefinition) -> Self {
        self.icon = Some(Icon::new(icon));
        self
    }
    pub fn banner(mut self, value: bool) -> Self {
        self.banner = value;
        self
    }
    pub fn closable(mut self, value: bool) -> Self {
        self.closable = value;
        self
    }
    pub fn close_text(mut self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        assert!(!text.trim().is_empty(), "close text must be nonblank");
        self.close_text = Some(text);
        self
    }
    pub fn close_icon(mut self, icon: impl IconDefinition) -> Self {
        self.close_icon = Some(Icon::new(icon));
        self
    }
    pub fn close_label(mut self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        assert!(!text.trim().is_empty(), "close label must be nonblank");
        self.close_label = text;
        self
    }
    pub fn visible(mut self, value: bool) -> Self {
        self.visible = value;
        self
    }
    pub fn on_close(
        mut self,
        handler: impl Fn(AlertCloseReason, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
    pub fn on_closed(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_closed = Some(Rc::new(handler));
        self
    }
    /// Clamp only the plain description; the expand control is conditional on overflow.
    pub fn max_lines(mut self, lines: usize) -> Self {
        assert!(lines > 0, "max_lines must be positive");
        self.max_lines = Some(lines);
        self
    }
    pub fn expanded(mut self, value: bool) -> Self {
        self.expanded = value;
        self
    }
    pub fn on_expanded_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_expanded = Some(Rc::new(handler));
        self
    }
    pub fn expand_label(mut self, value: impl Into<SharedString>) -> Self {
        let value = value.into();
        assert!(!value.trim().is_empty(), "expand label must be nonblank");
        self.expand_label = value;
        self
    }
    pub fn collapse_label(mut self, value: impl Into<SharedString>) -> Self {
        let value = value.into();
        assert!(!value.trim().is_empty(), "collapse label must be nonblank");
        self.collapse_label = value;
        self
    }
    /// Explicit opt-in. Hover, focus, inactive window and viewport clipping pause it.
    pub fn duration(mut self, value: Option<Duration>) -> Self {
        assert!(
            value.is_none_or(|v| !v.is_zero()),
            "duration must be nonzero"
        );
        self.duration = value;
        self
    }
    pub fn focus_after_close(mut self, focus: &FocusHandle) -> Self {
        self.focus_after = Some(focus.clone());
        self
    }
    pub fn announcement(mut self, value: AlertAnnouncement) -> Self {
        self.announcement = Some(value);
        self
    }
}
impl ParentElement for YueAlert {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
