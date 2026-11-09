//! Row displacement follows the resolved side, including a first-frame flip.
use crate::popup::PopupSnapshotHandle;
use gpui_kit::base::Placement;
use gpui_kit::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, point, px,
};

pub(super) struct RowEntrance {
    child: AnyElement,
    snapshot: PopupSnapshotHandle,
    rise: Pixels,
}

impl RowEntrance {
    pub(super) fn wrap(
        child: AnyElement,
        snapshot: PopupSnapshotHandle,
        rise: Pixels,
    ) -> AnyElement {
        if rise == px(0.) {
            return child;
        }
        Self {
            child,
            snapshot,
            rise,
        }
        .into_any_element()
    }
}

impl IntoElement for RowEntrance {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for RowEntrance {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        // PopupHost publishes the current side before prepainting its rows.
        // Offset prepaint so paint, hitboxes and submenu anchors move together.
        let offset = match self.snapshot.read().position().and_then(|p| p.placement) {
            Some(Placement::Top) => point(px(0.), -self.rise),
            Some(Placement::Left) => point(-self.rise, px(0.)),
            Some(Placement::Right) => point(self.rise, px(0.)),
            _ => point(px(0.), self.rise),
        };
        window.with_element_offset(offset, |window| {
            self.child.prepaint(window, cx);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
