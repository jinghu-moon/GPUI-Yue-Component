use gpui_kit::{App, KeyBinding};
gpui_kit::actions!(
    yue_time_picker,
    [NextField, PreviousField, OpenPanel, StepUp, StepDown]
);
pub(crate) fn init(cx: &mut App) {
    for context in [
        "YueTimePicker > Input",
        "YueTimePicker > Button",
        "YueTimePicker && !Input",
        "YueTimePickerPanel > Button",
        "YueTimePickerPanel",
    ] {
        cx.bind_keys([
            KeyBinding::new("tab", NextField, Some(context)),
            KeyBinding::new("shift-tab", PreviousField, Some(context)),
        ]);
    }
    cx.bind_keys([
        KeyBinding::new("alt-down", OpenPanel, Some("YueTimePicker")),
        KeyBinding::new("up", StepUp, Some("YueTimePicker > Input")),
        KeyBinding::new("down", StepDown, Some("YueTimePicker > Input")),
    ]);
}
