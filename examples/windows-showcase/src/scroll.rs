//! Vertical showcase viewport composed only from GPUI and Base primitives.
use gpui_kit::base::{InteractiveElementExt as _, StyledExt as _};
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
                    yue_component::YueScrollbar::vertical((self.id, "scrollbar"), &scroll)
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

    /// Real Showcase pages, real wheel dispatch, full draw and effect draining.
    /// Opt-in CPU probe; no native GPU/presentation or input latency claim.
    #[gpui_kit::test]
    #[ignore = "opt-in shared scroll pipeline; prebuild before sampling"]
    fn shared_pages_wheel_sample(cx: &mut TestAppContext) {
        use crate::{Showcase, ShowcasePage};
        use gpui_kit::{InputEvent as _, ScrollDelta, ScrollWheelEvent, TouchPhase};
        use std::time::Instant;
        let case = std::env::var("YUE_SCROLL_CASE").unwrap();
        let page = match case.as_str() {
            "button" => ShowcasePage::Button,
            "table" => ShowcasePage::Table,
            "list" => ShowcasePage::List,
            _ => panic!("unknown page"),
        };
        cx.update(|cx| {
            yue_component::init(
                cx,
                yue_component::YueTheme::new(
                    yue_component::ThemeMode::Light,
                    yue_component::Accent::Azure,
                ),
            )
            .unwrap();
            cx.set_reduce_motion(true);
        });
        let (showcase, cx) = cx.add_window_view(Showcase::new);
        cx.update(|w, _| {
            w.activate_window();
            w.resize(gpui_kit::size(px(1280.), px(800.)));
            w.set_rem_size(px(16.));
        });
        showcase.update(cx, |s, cx| {
            s.page = page;
            cx.notify();
        });
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        let marker = match case.as_str() {
            "button" => "button-text",
            "table" => "table-size",
            _ => "list-view-lin",
        };
        let marker_y = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|w, _| {
                gpui_kit::base::test_support::snapshots(w)
                    .into_iter()
                    .find(|n| n.path().last() == Some(&gpui_kit::ElementId::from(marker)))
                    .map(|n| f32::from(n.bounds().top()))
            })
        };
        let initial = marker_y(cx).expect("real page control is mounted");
        let step = |frame: usize, cx: &mut gpui_kit::VisualTestContext| {
            let ns = cx.update(|w, cx| {
                let start = Instant::now();
                w.dispatch_event(
                    ScrollWheelEvent {
                        // Table keeps its category tabs above the scrolling panel.
                        position: point(px(1200.), px(if case == "table" { 220. } else { 110. })),
                        delta: ScrollDelta::Pixels(point(
                            px(0.),
                            px(if frame % 80 < 40 { -12. } else { 12. }),
                        )),
                        modifiers: Default::default(),
                        touch_phase: TouchPhase::Moved,
                    }
                    .to_platform_input(),
                    cx,
                );
                w.draw(cx).clear(cx);
                start.elapsed().as_nanos()
            });
            cx.run_until_parked();
            ns
        };
        step(0, cx);
        let moved = marker_y(cx).expect("page control still measured after wheel");
        assert!(
            moved < initial - 1.,
            "wheel must move real page content: {initial} -> {moved}"
        );
        for frame in 1..80 {
            step(frame, cx);
        }
        for frame in 0..240 {
            let ns = step(frame, cx);
            let quads = cx.update(|w, _| w.painted_quads().len());
            assert!(ns > 0 && quads > 0);
            println!("SCROLL_FRAME,case={case},frame={frame},ns={ns},quads={quads}");
        }
        let final_y = marker_y(cx).expect("reversal restores real page control");
        assert!(
            (final_y - initial).abs() <= 1.,
            "forward/reverse input must restore offset"
        );
    }
}
