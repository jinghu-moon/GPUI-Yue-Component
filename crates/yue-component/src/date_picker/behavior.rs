use gpui_kit::{App, KeyBinding};
gpui_kit::actions!(yue_date_picker, [NextField, PreviousField]);
pub(crate) fn init(cx: &mut App) {
    // Specific descendant contexts take precedence over Input's editing bindings.
    // TimeField keeps its own Tab segment actions until their boundary propagates.
    for context in [
        "YueDatePicker > Input",
        "YueDatePicker > Button",
        "YueDatePicker && !Input",
        "YueDatePickerPanel > Button",
        "YueDatePickerPanel && !TimeField",
    ] {
        cx.bind_keys([
            KeyBinding::new("tab", NextField, Some(context)),
            KeyBinding::new("shift-tab", PreviousField, Some(context)),
        ]);
    }
}
