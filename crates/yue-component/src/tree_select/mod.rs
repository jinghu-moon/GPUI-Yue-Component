//! Tree choice fields: one keyed tree owner, shared field parts and Popup.
mod behavior;
mod element;
pub(crate) use behavior::init;
mod state;
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
use crate::{
    InputStatus, SelectPanelWidth, SelectTagLayout, SelectVariant, TreeCheckMode,
    TreeCheckProjection, TreeSize, YueTheme,
};
use gpui_kit::{
    AnyElement, App, ElementId, Entity, EntityId, FocusHandle, IntoElement, RenderOnce,
    SharedString, Window,
};
pub use state::YueTreeSelectState;
use std::rc::Rc;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSelectMode {
    #[default]
    Single,
    Multiple,
    Checked(TreeCheckMode),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeSelectValue {
    Single(Option<SharedString>),
    Multiple(Vec<SharedString>),
    Checked(Vec<SharedString>),
}
impl TreeSelectValue {
    pub fn keys(&self) -> &[SharedString] {
        match self {
            Self::Single(k) => k.as_slice(),
            Self::Multiple(k) | Self::Checked(k) => k,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.keys().is_empty()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSelectSearch {
    #[default]
    None,
    Local,
    Remote,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSelectValueDisplay {
    #[default]
    Label,
    Path,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeSelectChangeSource {
    Pointer,
    Keyboard,
    Accessibility,
    TagRemove,
    Clear,
}
#[derive(Clone, Debug)]
pub struct TreeSelectItemSnapshot {
    key: SharedString,
    label: SharedString,
    path: Vec<SharedString>,
    locked: bool,
    resolved: bool,
}
impl TreeSelectItemSnapshot {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            path: vec![],
            locked: false,
            resolved: false,
        }
    }
    pub fn path(mut self, path: impl IntoIterator<Item = SharedString>) -> Self {
        self.path = path.into_iter().collect();
        self
    }
    pub fn locked(mut self, value: bool) -> Self {
        self.locked = value;
        self
    }
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn ancestors(&self) -> &[SharedString] {
        &self.path
    }
    pub fn is_locked(&self) -> bool {
        self.locked
    }
    pub fn is_resolved(&self) -> bool {
        self.resolved
    }
}
#[derive(Clone, Debug)]
pub struct TreeSelectChange {
    before: TreeSelectValue,
    after: TreeSelectValue,
    trigger_key: Option<SharedString>,
    source: TreeSelectChangeSource,
    items: Vec<TreeSelectItemSnapshot>,
}
impl TreeSelectChange {
    pub fn trigger_key(&self) -> Option<&SharedString> {
        self.trigger_key.as_ref()
    }
    pub fn before(&self) -> &TreeSelectValue {
        &self.before
    }
    pub fn value(&self) -> &TreeSelectValue {
        &self.after
    }
    pub fn source(&self) -> TreeSelectChangeSource {
        self.source
    }
    pub fn items(&self) -> &[TreeSelectItemSnapshot] {
        &self.items
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSelectSearchTicket {
    owner: EntityId,
    session: u64,
    generation: u64,
}
#[derive(Clone, Debug)]
pub struct TreeSelectQuery {
    query: SharedString,
    ticket: TreeSelectSearchTicket,
}
impl TreeSelectQuery {
    pub fn query(&self) -> &SharedString {
        &self.query
    }
    pub fn ticket(&self) -> &TreeSelectSearchTicket {
        &self.ticket
    }
}
#[derive(Clone, Debug)]
pub enum TreeSelectEvent {
    Change(TreeSelectChange),
    OpenChanged(bool),
    QueryRequested(TreeSelectQuery),
    LoadRequested(crate::TreeLoadTicket),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeSelectResultState {
    Ready,
    Loading,
    Failed(SharedString),
}
type PanelRenderer = Rc<dyn Fn(bool, &mut Window, &mut App) -> AnyElement>;
type NodeRenderer = Rc<dyn Fn(&crate::TreeNode, &mut Window, &mut App) -> AnyElement>;
type ValueRenderer = Rc<dyn Fn(&[TreeSelectItemSnapshot], &mut Window, &mut App) -> AnyElement>;
type TagRenderer =
    Rc<dyn Fn(crate::YueTag, &TreeSelectItemSnapshot, &mut Window, &mut App) -> crate::YueTag>;
type EmptyRenderer = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
#[derive(Clone)]
pub(super) struct Presentation {
    id: ElementId,
    name: SharedString,
    description: Option<SharedString>,
    theme: Option<YueTheme>,
    size: TreeSize,
    placeholder: SharedString,
    prefix: Option<SharedString>,
    prefix_icon: Option<crate::Icon>,
    width: Option<gpui_kit::Pixels>,
    block: bool,
    panel_width: SelectPanelWidth,
    height: Option<gpui_kit::Pixels>,
    variant: SelectVariant,
    clearable: bool,
    required: bool,
    status: InputStatus,
    projection: TreeCheckProjection,
    display: TreeSelectValueDisplay,
    tags: SelectTagLayout,
    collapse: usize,
    lines: bool,
    icons: bool,
    label: Option<NodeRenderer>,
    extra: Option<NodeRenderer>,
    value: Option<ValueRenderer>,
    tag: Option<TagRenderer>,
    empty: Option<EmptyRenderer>,
    header: Option<PanelRenderer>,
    footer: Option<PanelRenderer>,
    header_focus: Vec<FocusHandle>,
    footer_focus: Vec<FocusHandle>,
}
#[derive(IntoElement)]
pub struct YueTreeSelect {
    state: Entity<YueTreeSelectState>,
    p: Presentation,
}
impl YueTreeSelect {
    pub fn new(id: impl Into<ElementId>, state: &Entity<YueTreeSelectState>) -> Self {
        Self {
            state: state.clone(),
            p: Presentation {
                id: id.into(),
                name: "".into(),
                description: None,
                theme: None,
                size: TreeSize::Medium,
                placeholder: "请选择".into(),
                prefix: None,
                prefix_icon: None,
                width: None,
                block: false,
                panel_width: SelectPanelWidth::MatchTrigger,
                height: None,
                variant: SelectVariant::Outline,
                clearable: false,
                required: false,
                status: InputStatus::Normal,
                projection: TreeCheckProjection::Parents,
                display: TreeSelectValueDisplay::Label,
                tags: SelectTagLayout::Collapse,
                collapse: 2,
                lines: false,
                icons: false,
                label: None,
                extra: None,
                value: None,
                tag: None,
                empty: None,
                header: None,
                footer: None,
                header_focus: vec![],
                footer_focus: vec![],
            },
        }
    }
    pub fn accessibility_label(mut self, v: impl Into<SharedString>) -> Self {
        self.p.name = v.into();
        self
    }
    pub fn accessibility_description(mut self, v: impl Into<SharedString>) -> Self {
        self.p.description = Some(v.into());
        self
    }
    pub fn theme(mut self, v: YueTheme) -> Self {
        self.p.theme = Some(v);
        self
    }
    pub fn size(mut self, v: TreeSize) -> Self {
        self.p.size = v;
        self
    }
    pub fn placeholder(mut self, v: impl Into<SharedString>) -> Self {
        self.p.placeholder = v.into();
        self
    }
    pub fn prefix_text(mut self, v: impl Into<SharedString>) -> Self {
        self.p.prefix = Some(v.into());
        self
    }
    pub fn width(mut self, v: gpui_kit::Pixels) -> Self {
        self.p.width = Some(v);
        self.p.block = false;
        self
    }
    pub fn block(mut self) -> Self {
        self.p.block = true;
        self
    }
    pub fn panel_width(mut self, v: SelectPanelWidth) -> Self {
        self.p.panel_width = v;
        self
    }
    pub fn max_height(mut self, v: gpui_kit::Pixels) -> Self {
        self.p.height = Some(v);
        self
    }
    pub fn variant(mut self, v: SelectVariant) -> Self {
        self.p.variant = v;
        self
    }
    pub fn clearable(mut self, v: bool) -> Self {
        self.p.clearable = v;
        self
    }
    pub fn required(mut self, v: bool) -> Self {
        self.p.required = v;
        self
    }
    pub fn status(mut self, v: InputStatus) -> Self {
        self.p.status = v;
        self
    }
    pub fn check_projection(mut self, v: TreeCheckProjection) -> Self {
        self.p.projection = v;
        self
    }
    pub fn value_display(mut self, v: TreeSelectValueDisplay) -> Self {
        self.p.display = v;
        self
    }
    pub fn tag_layout(mut self, v: SelectTagLayout) -> Self {
        self.p.tags = v;
        self
    }
    pub fn collapse_count(mut self, v: usize) -> Self {
        self.p.collapse = v;
        self
    }
    pub fn show_lines(mut self, v: bool) -> Self {
        self.p.lines = v;
        self
    }
    pub fn show_icons(mut self, v: bool) -> Self {
        self.p.icons = v;
        self
    }
    pub fn prefix_icon(mut self, icon: impl crate::IconDefinition) -> Self {
        self.p.prefix_icon = Some(crate::Icon::new(icon));
        self
    }
    /// Noninteractive fixed-height row content; the standard TreeItem root remains.
    pub fn render_label<R: IntoElement>(
        mut self,
        f: impl Fn(&crate::TreeNode, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.label = Some(Rc::new(move |n, w, cx| f(n, w, cx).into_any_element()));
        self
    }
    pub fn render_extra<R: IntoElement>(
        mut self,
        f: impl Fn(&crate::TreeNode, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.extra = Some(Rc::new(move |n, w, cx| f(n, w, cx).into_any_element()));
        self
    }
    pub fn render_value<R: IntoElement>(
        mut self,
        f: impl Fn(&[TreeSelectItemSnapshot], &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.value = Some(Rc::new(move |items, w, cx| {
            f(items, w, cx).into_any_element()
        }));
        self
    }
    /// Decorate the supplied Tag; retain its close callback and focus handle.
    pub fn render_tag(
        mut self,
        f: impl Fn(crate::YueTag, &TreeSelectItemSnapshot, &mut Window, &mut App) -> crate::YueTag
        + 'static,
    ) -> Self {
        self.p.tag = Some(Rc::new(f));
        self
    }
    pub fn empty<R: IntoElement>(
        mut self,
        f: impl Fn(&mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.empty = Some(Rc::new(move |w, cx| f(w, cx).into_any_element()));
        self
    }
    /// Registered slot controls must use tab_stop(false) and honor `interactive`.
    pub fn header<R: IntoElement>(
        mut self,
        f: impl Fn(bool, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.header = Some(Rc::new(move |interactive, w, cx| {
            f(interactive, w, cx).into_any_element()
        }));
        self
    }
    pub fn footer<R: IntoElement>(
        mut self,
        f: impl Fn(bool, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.footer = Some(Rc::new(move |interactive, w, cx| {
            f(interactive, w, cx).into_any_element()
        }));
        self
    }
    pub fn header_focus_handles(mut self, v: Vec<FocusHandle>) -> Self {
        self.p.header_focus = v;
        self
    }
    pub fn footer_focus_handles(mut self, v: Vec<FocusHandle>) -> Self {
        self.p.footer_focus = v;
        self
    }
}
impl RenderOnce for YueTreeSelect {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |s, _| s.presentation = Some(self.p));
        self.state
    }
}
