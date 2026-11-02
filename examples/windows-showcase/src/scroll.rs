//! Vertical showcase viewport composed only from GPUI and Base primitives.
use gpui_kit::base::{InteractiveElementExt as _, Scrollbar, StyledExt as _};
use gpui_kit::{
    App, Element, ElementId, InteractiveElement, IntoElement, Overflow, ParentElement,
    PointRefinement, RenderOnce, ScrollHandle, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div,
};

pub trait ScrollableElement: InteractiveElement + Styled + ParentElement + Element + Sized {
    #[track_caller]
    fn overflow_y_scrollbar(self) -> VerticalScroll<Self> {
        VerticalScroll {
            id: ElementId::CodeLocation(*std::panic::Location::caller()),
            content: self,
        }
    }
}
impl<E: InteractiveElement + Styled + ParentElement + Element> ScrollableElement for E {}

#[derive(IntoElement)]
pub struct VerticalScroll<E: InteractiveElement + Styled + ParentElement + Element> {
    id: ElementId,
    content: E,
}
impl<E: InteractiveElement + Styled + ParentElement + Element + 'static> RenderOnce
    for VerticalScroll<E>
{
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll = window
            .use_keyed_state(self.id.clone(), cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();
        let s = self.content.style();
        let outer = StyleRefinement {
            size: s.size.clone(),
            min_size: s.min_size.clone(),
            max_size: s.max_size.clone(),
            flex_grow: s.flex_grow,
            flex_shrink: s.flex_shrink,
            flex_basis: s.flex_basis,
            align_self: s.align_self,
            overflow: PointRefinement {
                x: None,
                y: Some(Overflow::Hidden),
            },
            ..Default::default()
        };
        let content = self
            .content
            .id((self.id.clone(), "content"))
            .flex_none()
            .h_auto()
            .min_h_full();
        div()
            .id(self.id.clone())
            .refine_style(&outer)
            .relative()
            .child(
                div()
                    .id((self.id.clone(), "viewport"))
                    .size_full()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    // A sideways swipe would otherwise be remapped onto the
                    // only scrolling axis.
                    .lock_scroll_axis()
                    .track_scroll(&scroll)
                    .child(content),
            )
            .child(
                div().absolute().inset_0().child(
                    Scrollbar::vertical(&scroll)
                        .id((self.id, "scrollbar"))
                        .viewport_from_layout(),
                ),
            )
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::{Context, Render, TestAppContext, point, px, rgb};

    struct Viewport;
    impl Render for Viewport {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(200.)).h(px(100.)).child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .children([
                        div().h(px(80.)).flex_none().bg(rgb(0xff0000)),
                        div().h(px(320.)).flex_none().bg(rgb(0x00ff00)),
                    ])
                    .overflow_y_scrollbar(),
            )
        }
    }

    #[gpui_kit::test]
    fn wheel_moves_content_inside_the_original_viewport(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_, cx) = cx.add_window_view(|_, _| Viewport);
        let draw = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
        };
        let red = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, _| {
                window
                    .painted_quads()
                    .into_iter()
                    .find(|quad| quad.background == rgb(0xff0000).into())
                    .unwrap()
            })
        };
        draw(cx);
        let before = red(cx);
        let scale = cx.update(|window, _| window.scale_factor());
        assert_eq!(
            before.bounds.size.width,
            gpui_kit::ScaledPixels(200. * scale)
        );
        cx.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(50.), px(50.)),
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-40.))),
            modifiers: Default::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        draw(cx);
        let after = red(cx);
        assert!(after.bounds.origin.y < before.bounds.origin.y);
        assert_eq!(after.bounds.origin.x, before.bounds.origin.x);
        assert_eq!(after.content_mask, before.content_mask);
        assert_eq!(
            after.content_mask.bounds.size.height,
            gpui_kit::ScaledPixels(100. * scale)
        );
    }
}
