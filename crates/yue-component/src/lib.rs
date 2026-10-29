//! Yue design tokens and components for applications built on GPUI Kit.
//!
//! Applications own their window and lifecycle. This crate provides presentation
//! data and components while reusing the GPUI types pinned by `gpui-kit`.

mod button;
mod checkbox;
mod form;
mod input;
mod radio;
mod switch;
pub mod theme;
mod tokens;

pub use button::{ButtonGroup, ButtonShape, ButtonSize, ButtonVariant, IconPosition, YueButton};
pub use checkbox::{CheckboxSize, CheckboxState, YueCheckbox, YueCheckboxGroup, YueCheckboxOption};
pub use form::YueForm;
pub use input::{
    InputSize, InputStatus, InputTextAlign, InputType, YueField, YueInput, YueNumberInput,
    YueTextarea,
};
pub use radio::{RadioAppearance, RadioSize, YueRadio, YueRadioGroup, YueRadioOption};
pub use switch::{SwitchIndicator, SwitchShape, SwitchSize, YueSwitch};
pub use theme::{Accent, ColorRole, ThemeMode, YueTheme, YueToken};
pub use yue_component_icons::{Icon, IconDefinition, filled, outline};

/// Which way a group lays its options out.
///
/// A vertical column is the form default; a horizontal row is for a short,
/// bounded set of options and wraps onto another row when the width runs out.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GroupDirection {
    #[default]
    Vertical,
    Horizontal,
}
