//! An explicitly bounded, nonmodal inspector with optional content reflow.
use super::*;
use gpui_kit::base::{Presence, StyledExt as _, TestSupportExt as _};
use gpui_kit::{
    Bounds, InteractiveElement as _, StyleRefinement, Styled, div, prelude::FluentBuilder as _,
};
/// A local host always uses nonmodal behavior. Give it a definite width/height.
/// Window-level decisions should use YueDrawer directly instead.
#[derive(IntoElement)]
pub struct YueDrawerHost {
    id: ElementId,
    drawer: YueDrawer,
    children: Vec<AnyElement>,
    push: bool,
    style: StyleRefinement,
}
impl YueDrawerHost {
    pub fn new(id: impl Into<ElementId>, drawer: YueDrawer) -> Self {
        Self {
            id: id.into(),
            drawer: drawer.modal(false),
            children: vec![],
            push: false,
            style: StyleRefinement::default(),
        }
    }
    pub fn push(mut self, value: bool) -> Self {
        self.push = value;
        self
    }
}
impl ParentElement for YueDrawerHost {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}
impl Styled for YueDrawerHost {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for YueDrawerHost {
    fn render(mut self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let bounds = w
            .use_keyed_state((self.id.clone(), "drawer-host-bounds"), cx, |_, _| {
                Rc::new(Cell::new(None::<Bounds<Pixels>>))
            })
            .read(cx)
            .clone();
        let memory = w
            .use_keyed_state((self.id.clone(), "drawer-host-memory"), cx, |_, _| {
                resize::Memory::default()
            })
            .read(cx)
            .clone();
        let open = self.drawer.inner_state().read(cx).is_open();
        let presence = Presence::new((self.id.clone(), "drawer-host-push"), open)
            .transition(Presentation::transition())
            .sample(w, cx);
        let size = bounds
            .get()
            .map_or(gpui_kit::size(px(0.), px(0.)), |b| b.size);
        let resolved = memory.resolve(&self.drawer.presentation, size, w);
        let inset = if self.push {
            resolved.extent * presence.progress
        } else {
            px(0.)
        };
        let placement = self.drawer.presentation.placement;
        self.drawer.presentation.host = Some(bounds.clone());
        self.drawer.presentation.memory = Some(memory);
        let measure = gpui_kit::canvas(
            move |measured, w, _| {
                if bounds.get() != Some(measured) {
                    bounds.set(Some(measured));
                    w.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();
        div()
            .id(self.id)
            .test_support()
            .relative()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(
                div()
                    .id("drawer-host-content")
                    .test_support()
                    .size_full()
                    .overflow_hidden()
                    .map(|d| match placement {
                        DrawerPlacement::Left => d.pl(inset),
                        DrawerPlacement::Right => d.pr(inset),
                        DrawerPlacement::Top => d.pt(inset),
                        DrawerPlacement::Bottom => d.pb(inset),
                    })
                    .children(self.children),
            )
            .child(measure)
            .child(self.drawer)
            .refine_style(&self.style)
    }
}
