//! Styled, virtualized trees sharing Base's keyed model with TreeSelect.
mod element;
mod motion;
mod state;
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
use crate::{Icon, YueTheme};
pub use gpui_kit::base::keyed_tree::{
    TreeApply, TreeCheckMode, TreeCheckProjection, TreeChildren, TreeDataError, TreeFilter,
    TreeLoadState, TreeLoadTicket, TreeMark, TreeMove, TreeNode, TreeReconciliation, TreeRow,
    TreeSelectionMode,
};
use gpui_kit::{AnyElement, App, ElementId, Entity, IntoElement, RenderOnce, SharedString, Window};
pub use state::YueTreeState;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSize {
    Small,
    #[default]
    Medium,
    Large,
}
impl TreeSize {
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
    pub(crate) fn input(self) -> crate::InputSize {
        match self {
            Self::Small => crate::InputSize::Small,
            Self::Medium => crate::InputSize::Medium,
            Self::Large => crate::InputSize::Large,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeRowAction {
    #[default]
    Select,
    Check,
    Expand,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeChangeSource {
    Pointer,
    Keyboard,
    Accessibility,
}
#[derive(Clone, Debug)]
pub struct TreeChange {
    before: Vec<SharedString>,
    after: Vec<SharedString>,
    key: SharedString,
    source: TreeChangeSource,
}
impl TreeChange {
    pub fn before(&self) -> &[SharedString] {
        &self.before
    }
    pub fn value(&self) -> &[SharedString] {
        &self.after
    }
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn source(&self) -> TreeChangeSource {
        self.source
    }
}
#[derive(Clone, Debug)]
pub enum TreeEvent {
    SelectionChanged(TreeChange),
    CheckChanged(TreeChange),
    ExpansionChanged {
        key: SharedString,
        expanded: bool,
    },
    Activate {
        key: SharedString,
        source: TreeChangeSource,
    },
    LoadRequested(TreeLoadTicket),
    ContextRequested(SharedString),
}
pub(crate) type NodeRenderer = Rc<dyn Fn(&TreeNode, &mut Window, &mut App) -> AnyElement>;
type EmptyRenderer = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
#[derive(Clone)]
pub(crate) struct Presentation {
    id: ElementId,
    name: SharedString,
    theme: Option<YueTheme>,
    size: TreeSize,
    height: Option<gpui_kit::Pixels>,
    lines: bool,
    icons: bool,
    search: bool,
    label: Option<NodeRenderer>,
    extra: Option<NodeRenderer>,
    actions: Option<NodeRenderer>,
    tab_stop: bool,
    empty: Option<EmptyRenderer>,
}
#[derive(IntoElement)]
pub struct YueTree {
    state: Entity<YueTreeState>,
    p: Presentation,
}
impl YueTree {
    pub fn new(id: impl Into<ElementId>, state: &Entity<YueTreeState>) -> Self {
        Self {
            state: state.clone(),
            p: Presentation {
                id: id.into(),
                name: "树".into(),
                theme: None,
                size: TreeSize::Medium,
                height: None,
                lines: false,
                icons: false,
                search: false,
                label: None,
                extra: None,
                actions: None,
                tab_stop: true,
                empty: None,
            },
        }
    }
    pub fn accessibility_label(mut self, name: impl Into<SharedString>) -> Self {
        self.p.name = name.into();
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.p.theme = Some(theme);
        self
    }
    pub fn size(mut self, size: TreeSize) -> Self {
        self.p.size = size;
        self
    }
    pub fn height(mut self, height: gpui_kit::Pixels) -> Self {
        self.p.height = Some(height.max(gpui_kit::px(1.)));
        self
    }
    pub fn show_lines(mut self, value: bool) -> Self {
        self.p.lines = value;
        self
    }
    pub fn show_icons(mut self, value: bool) -> Self {
        self.p.icons = value;
        self
    }
    pub fn searchable(mut self, value: bool) -> Self {
        self.p.search = value;
        self
    }
    pub fn tab_stop(mut self, value: bool) -> Self {
        self.p.tab_stop = value;
        self
    }
    pub fn empty<R: IntoElement>(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.empty = Some(Rc::new(move |w, cx| render(w, cx).into_any_element()));
        self
    }
    pub fn render_label<R: IntoElement>(
        mut self,
        render: impl Fn(&TreeNode, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.label = Some(Rc::new(move |n, w, cx| render(n, w, cx).into_any_element()));
        self
    }
    /// Explicit interactive row slot. Register matching focus handles on the state.
    pub fn render_actions<R: IntoElement>(
        mut self,
        render: impl Fn(&TreeNode, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.actions = Some(Rc::new(move |n, w, cx| render(n, w, cx).into_any_element()));
        self
    }
    pub fn render_extra<R: IntoElement>(
        mut self,
        render: impl Fn(&TreeNode, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.p.extra = Some(Rc::new(move |n, w, cx| render(n, w, cx).into_any_element()));
        self
    }
}
impl RenderOnce for YueTree {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, _| state.presentation = Some(self.p));
        self.state
    }
}

gpui_kit::actions!(
    yue_tree,
    [EnterActions, ActionsNext, ActionsPrevious, ExitActions]
);
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([gpui_kit::KeyBinding::new(
        "f2",
        EnterActions,
        Some("YueTree"),
    )]);
    for context in [
        "YueTreeActions > Button",
        "YueTreeActions > Input",
        "YueTreeActions",
    ] {
        cx.bind_keys([
            gpui_kit::KeyBinding::new("tab", ActionsNext, Some(context)),
            gpui_kit::KeyBinding::new("shift-tab", ActionsPrevious, Some(context)),
            gpui_kit::KeyBinding::new("escape", ExitActions, Some(context)),
        ]);
    }
}
