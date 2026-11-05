//! Navigation belongs to this arithmetic-record consumer, not the layout engine.
//! The retained cursor is one stable key pair, never an axis cross product.
use super::*;
use gpui_kit::KeyDownEvent;

impl VirtualLayoutsPage {
    pub(super) fn grid_origin(&self, (row, column): (usize, usize)) -> (usize, usize) {
        if self.spanning && (1..3).contains(&row) && (1..4).contains(&column) {
            (1, 1)
        } else {
            (row, column)
        }
    }

    pub(super) fn grid_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.grid_focus.is_focused(window) {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if modifiers.alt || modifiers.platform || modifiers.shift {
            return;
        }
        let key = event.keystroke.key.as_str();
        if modifiers.control && !matches!(key, "home" | "end") {
            return;
        }
        let (row, column) = self.grid_cursor;
        let merged = self.spanning && (row, column) == (1, 1);
        let page_rows = self.rows.read(cx).viewport().map_or(1, |v| {
            let body = f32::from(v.extent()) - if self.frozen { 36. } else { 0. };
            (body / 36.).floor().max(1.) as usize
        });
        let next = match key {
            "up" => (row.saturating_sub(1), column),
            "down" => (
                row.saturating_add(if merged { 2 } else { 1 })
                    .min(GRID_ROWS - 1),
                column,
            ),
            "left" | "right" => {
                let forward = (key == "right") != self.rtl;
                (
                    row,
                    if forward {
                        column
                            .saturating_add(if merged { 3 } else { 1 })
                            .min(GRID_COLUMNS - 1)
                    } else {
                        column.saturating_sub(1)
                    },
                )
            }
            "home" if modifiers.control => (0, 0),
            "end" if modifiers.control => (GRID_ROWS - 1, GRID_COLUMNS - 1),
            "home" => (row, 0),
            "end" => (row, GRID_COLUMNS - 1),
            "pageup" => (row.saturating_sub(page_rows), column),
            "pagedown" => (row.saturating_add(page_rows).min(GRID_ROWS - 1), column),
            _ => return,
        };
        // Entering any covered position selects the one logical origin.
        self.grid_cursor = self.grid_origin(next);
        self.reveal_grid_cursor(modifiers.control && key == "home", cx);
        cx.notify();
        cx.stop_propagation();
    }

    pub(super) fn reveal_grid_cursor(&self, first_viewport: bool, cx: &mut Context<Self>) {
        for (axis, key, frozen) in [
            (&self.rows, self.grid_cursor.0, self.frozen),
            (&self.columns, self.grid_cursor.1, self.frozen),
        ] {
            // A frozen target is already visible; scrolling its axis to zero
            // would unnecessarily move the other body cells.
            if frozen && key == 0 && !first_viewport {
                continue;
            }
            axis.update(cx, |state, cx| {
                let result = if first_viewport {
                    state.scroll_to_offset(0., VirtualizerScrollOptions::default())
                } else {
                    state.scroll_to_key(key.into(), VirtualizerScrollOptions::default())
                };
                result.expect("cursor is inside the immutable arithmetic source");
                cx.notify();
            });
        }
    }
}
