//! Retained Base split geometry with Yue presentation and relative dimensions.
use crate::{
    YueTheme,
    tokens::{rems, rgba},
};
use gpui_kit::base::{
    ResizableInteraction, ResizablePanelGroup, ResizeHandleState, resizable_panel,
};
use gpui_kit::{
    AbsoluteLength, AnyElement, App, Axis, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};
use std::rc::Rc;

pub use gpui_kit::base::{
    ResizablePanelEvent as ResizableEvent, ResizableState as YueResizableState,
};

#[cfg(all(test, feature = "headless-tests"))]
mod tests;

/// A retained group of panels. Supply one state entity per group, kept by its host.
///
/// ```
/// use yue_component::{YueResizable, YueResizablePanel, YueResizableState};
/// use yue_component::gpui_kit::{Entity, IntoElement, ParentElement, div, rems};
/// fn workspace(state: &Entity<YueResizableState>) -> impl IntoElement {
///     YueResizable::horizontal("workspace", state)
///         .child(YueResizablePanel::new("navigation").label("Navigation")
///             .initial_size(rems(15.)).min_size(rems(10.)).fixed(true)
///             .child(div().child("Files")))
///         .child(YueResizablePanel::new("document").label("Document")
///             .min_size(rems(16.)).child(div().child("Document")))
/// }
/// ```
#[derive(IntoElement)]
pub struct YueResizable {
    id: ElementId,
    axis: Axis,
    state: Entity<YueResizableState>,
    panels: Vec<YueResizablePanel>,
    disabled: bool,
    theme: Option<YueTheme>,
    style: StyleRefinement,
    on_resize: Option<Rc<ResizeCallback>>,
}
type ResizeCallback = dyn Fn(&Entity<YueResizableState>, &mut Window, &mut App);

impl YueResizable {
    pub fn horizontal(id: impl Into<ElementId>, state: &Entity<YueResizableState>) -> Self {
        Self {
            id: id.into(),
            axis: Axis::Horizontal,
            state: state.clone(),
            panels: Vec::new(),
            disabled: false,
            theme: None,
            style: StyleRefinement::default(),
            on_resize: None,
        }
    }
    pub fn vertical(id: impl Into<ElementId>, state: &Entity<YueResizableState>) -> Self {
        Self {
            axis: Axis::Vertical,
            ..Self::horizontal(id, state)
        }
    }
    pub fn child(mut self, panel: YueResizablePanel) -> Self {
        self.panels.push(panel);
        self
    }
    pub fn children(mut self, panels: impl IntoIterator<Item = YueResizablePanel>) -> Self {
        self.panels.extend(panels);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    /// User commits only. Subscribe to ResizableEvent for programmatic commits too.
    pub fn on_resize(
        mut self,
        callback: impl Fn(&Entity<YueResizableState>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Rc::new(callback));
        self
    }
}
impl Styled for YueResizable {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Content for one keyed panel. The parent owns all split sizes.
pub struct YueResizablePanel {
    id: ElementId,
    label: Option<SharedString>,
    initial: Option<AbsoluteLength>,
    min: Option<AbsoluteLength>,
    max: Option<AbsoluteLength>,
    fixed: bool,
    children: Vec<AnyElement>,
}
impl YueResizablePanel {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            initial: None,
            min: None,
            max: None,
            fixed: false,
            children: Vec::new(),
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn initial_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.initial = Some(size.into());
        self
    }
    pub fn min_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.min = Some(size.into());
        self
    }
    pub fn max_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.max = Some(size.into());
        self
    }
    /// Keep the explicit initial width/height on container resize. Requires initial_size.
    pub fn fixed(mut self, fixed: bool) -> Self {
        self.fixed = fixed;
        self
    }
}
impl ParentElement for YueResizablePanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for YueResizable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use gpui_kit::base::StyledExt as _;
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let metric = |name| rems(theme, name).to_pixels(window.rem_size());
        let line = metric("--resizable-line-width");
        let hit_width = metric("--resizable-hit-width");
        let focus_width = metric("--resizable-outline-width");
        let min = metric("--resizable-panel-min-size");
        let base_rem = window.rem_size();
        let resolve = |length: AbsoluteLength| {
            let value = length.to_pixels(base_rem);
            assert!(
                value.as_f32().is_finite() && value >= gpui_kit::px(0.),
                "invalid resizable length"
            );
            value
        };
        let disabled = self.disabled;
        let appearance = Rc::new(
            move |context: &gpui_kit::base::ResizeHandleContext, _: &mut Window, _: &mut App| {
                let color = if disabled {
                    "--resizable-line-color-disabled"
                } else if context.is_focused() {
                    "--resizable-outline-color"
                } else {
                    match context.state() {
                        ResizeHandleState::Idle => "--resizable-line-color",
                        ResizeHandleState::Hovered => "--resizable-line-color-hover",
                        ResizeHandleState::Pressed | ResizeHandleState::Dragging => {
                            "--resizable-line-color-active"
                        }
                    }
                };
                let width = if !disabled && (context.is_active() || context.is_focused()) {
                    focus_width
                } else {
                    line
                };
                Some(
                    div()
                        .absolute()
                        .bg(rgba(theme, color))
                        .map(|d| {
                            if context.axis() == Axis::Horizontal {
                                d.h_full().w(width).left((hit_width - width) / 2.).top_0()
                            } else {
                                d.w_full().h(width).top((hit_width - width) / 2.).left_0()
                            }
                        })
                        .into_any_element(),
                )
            },
        );
        let mut group = ResizablePanelGroup::new((self.id.clone(), "base"))
            .axis(self.axis)
            .with_state(&self.state)
            .with_interaction(
                ResizableInteraction::new(
                    metric("--resizable-keyboard-step"),
                    metric("--resizable-keyboard-step-large"),
                )
                .disabled(disabled),
            )
            .with_handle_geometry(hit_width, line)
            .with_handle_appearance(appearance);
        if let Some(callback) = self.on_resize {
            group = group.on_resize(move |s, w, cx| callback(s, w, cx));
        }
        for panel in self.panels {
            let lower = panel.min.map(resolve).unwrap_or(min);
            let upper = panel.max.map(resolve).unwrap_or(gpui_kit::Pixels::MAX);
            assert!(upper >= lower, "resizable max_size must be >= min_size");
            let mut base = resizable_panel()
                .id(panel.id.clone())
                .size_range(lower..upper);
            if let Some(label) = panel.label {
                base = base.label(label);
            }
            if let Some(initial) = panel.initial {
                base = base.size(resolve(initial));
            }
            if panel.fixed {
                base = base.fixed();
            }
            // Clip the contents, not the panel: its resize hit band overlaps
            // the boundary. A child can install its own existing scroll owner.
            group = group.child(
                base.child(
                    div()
                        .id((panel.id, "content"))
                        .size_full()
                        .min_w_0()
                        .min_h_0()
                        .overflow_hidden()
                        .children(panel.children),
                ),
            );
        }
        div()
            .id(self.id)
            .size_full()
            .min_w_0()
            .min_h_0()
            .relative()
            .refine_style(&self.style)
            .child(group)
    }
}
