use super::*;
use gpui_kit::{Context, KeyBinding};
gpui_kit::actions!(
    yue_tree_select,
    [
        QueryNext,
        QueryPrevious,
        QueryAccept,
        Leave,
        LeaveBack,
        Dismiss
    ]
);
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", QueryNext, Some("YueTreeSelectQuery > Input")),
        KeyBinding::new("up", QueryPrevious, Some("YueTreeSelectQuery > Input")),
        KeyBinding::new("enter", QueryAccept, Some("YueTreeSelectQuery > Input")),
        KeyBinding::new("escape", Dismiss, Some("YueTreeSelectQuery > Input")),
    ]);
    for context in [
        "YueTreeSelectPanel > Input",
        "YueTreeSelectPanel > Button",
        "YueTreeSelectPanel && !Input",
    ] {
        cx.bind_keys([
            KeyBinding::new("tab", Leave, Some(context)),
            KeyBinding::new("shift-tab", LeaveBack, Some(context)),
            KeyBinding::new("escape", Dismiss, Some(context)),
        ]);
    }
}
impl YueTreeSelectState {
    pub(super) fn enter_results(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.open && self.result == TreeSelectResultState::Ready && !self.composing(w, cx) {
            self.tree.read(cx).focus.clone().focus(w, cx);
        }
    }
    pub(super) fn dismiss(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composing(w, cx) {
            return;
        }
        if self.collapsed {
            self.collapsed = false;
            self.collapsed_focus.focus(w, cx);
            cx.notify();
        } else if self.open {
            self.close(true, w, cx);
        }
    }
    pub(super) fn leave(&mut self, back: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.composing(w, cx) {
            return;
        }
        if self.collapsed {
            let order: Vec<_> = self
                .hidden_tags
                .iter()
                .filter_map(|k| self.tag_focus.get(k))
                .filter(|h| gpui_kit::base::Root::is_focus_available(h, w))
                .collect();
            let current = order.iter().position(|h| h.is_focused(w));
            let target = match current {
                Some(i) => {
                    if back {
                        i.checked_sub(1)
                    } else {
                        Some(i + 1)
                    }
                }
                None => {
                    if back {
                        None
                    } else {
                        Some(0)
                    }
                }
            };
            if let Some(target) = target.and_then(|i| order.get(i)) {
                target.focus(w, cx);
                return;
            }
        }
        if self.open {
            let mut order = vec![];
            if self.search != TreeSelectSearch::None {
                order.push(self.content_focus(cx));
            }
            if let Some(p) = &self.presentation {
                order.extend(
                    p.header_focus
                        .iter()
                        .filter(|&h| gpui_kit::base::Root::is_focus_available(h, w))
                        .cloned(),
                );
                if self.result == TreeSelectResultState::Ready {
                    order.push(self.tree.read(cx).focus.clone());
                }
                order.extend(
                    p.footer_focus
                        .iter()
                        .filter(|&h| gpui_kit::base::Root::is_focus_available(h, w))
                        .cloned(),
                );
            }
            if let Some(i) = order.iter().position(|h| h.contains_focused(w, cx)) {
                let next = if back { i.checked_sub(1) } else { Some(i + 1) };
                if let Some(target) = next.and_then(|i| order.get(i)) {
                    target.focus(w, cx);
                    return;
                }
            }
        }
        self.close(true, w, cx);
        for _ in 0..self.value(cx).keys().len() + 6 {
            if back {
                gpui_kit::base::Root::focus_prev(w, cx)
            } else {
                gpui_kit::base::Root::focus_next(w, cx)
            }
            if !self.focus.contains_focused(w, cx) {
                return;
            }
        }
        w.blur(cx);
    }
}
