use gpui_kit::base::TestSupportExt as _;
use gpui_kit::base::input::{InputBaseState, InputModeKind};
use gpui_kit::{
    AccessibleAction, AnyElement, App, DefiniteLength, ElementId, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _, TextAlign, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{ButtonShape, ButtonSize, ButtonVariant, YueButton, YueForm, YueTheme};
use yue_component_icons::{Icon, IconDefinition};

use crate::icon::YueIcon;
use crate::tokens::{pixels as token_pixels, rems, rgba as color};

/// An input frame with its id, styles and accessibility setters applied.
///
/// `test_support()` registers the frame for UI queries; without the `test-support`
/// feature of `gpui-kit` the alias collapses to the plain stateful div.
type ObservedFrame = gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>>;

pub(crate) fn border_token(
    status: InputStatus,
    disabled: bool,
    focused: bool,
) -> crate::theme::TokenKey {
    if disabled {
        crate::theme::keys::INPUT_BORDER_COLOR_DISABLED
    } else if status == InputStatus::Normal && focused {
        crate::theme::keys::INPUT_BORDER_COLOR_FOCUS
    } else {
        match status {
            InputStatus::Normal => crate::theme::keys::INPUT_BORDER_COLOR,
            InputStatus::Success => crate::theme::keys::SUCCESS,
            InputStatus::Warning => crate::theme::keys::WARNING,
            InputStatus::Error => crate::theme::keys::INPUT_BORDER_COLOR_INVALID,
        }
    }
}

fn field_message_token(status: InputStatus) -> crate::theme::TokenKey {
    match status {
        InputStatus::Error => crate::theme::keys::ERROR_FOREGROUND,
        InputStatus::Warning => crate::theme::keys::WARNING_FOREGROUND,
        InputStatus::Success => crate::theme::keys::SUCCESS_FOREGROUND,
        InputStatus::Normal => crate::theme::keys::INPUT_AFFIX_COLOR,
    }
}

/// Clamp the value to a character limit and park the caret where the user left it.
///
/// GPUI Base has no `max_length`: its `validate` and `pattern` hooks only see the
/// text one edit inserts, so a paste or an IME commit that overshoots is caught
/// here instead. `replace_all` keeps the trim undoable, unlike `set_value`.
fn truncate_to_limit<M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    limit: usize,
    window: &mut Window,
    cx: &mut App,
) {
    let (value, selection) = state.update(cx, |input, _| (input.value(), input.selected_range()));
    let characters: Vec<char> = value.chars().collect();
    if characters.len() <= limit {
        return;
    }

    let mut caret_characters = 0;
    let mut bytes = 0;
    while bytes < selection.start && caret_characters < characters.len() {
        bytes += characters[caret_characters].len_utf8();
        caret_characters += 1;
    }
    let kept: String = characters[..limit].iter().collect();
    let caret_bytes = kept
        .chars()
        .take(caret_characters.min(limit))
        .map(char::len_utf8)
        .sum();

    state.update(cx, |input, cx| {
        input.replace_all(kept, window, cx);
        input.set_selected_range(caret_bytes..caret_bytes, cx);
    });
}

fn counter_slot(theme: YueTheme, count: usize, limit: Option<usize>) -> AnyElement {
    let text = match limit {
        Some(limit) => format!("{count} / {limit}"),
        None => count.to_string(),
    };
    div()
        .flex_shrink_0()
        .text_size(rems(theme, crate::theme::keys::INPUT_FONT_SIZE_SM))
        .text_color(color(theme, crate::theme::keys::INPUT_AFFIX_COLOR))
        .child(text)
        .into_any_element()
}

/// The copy assistive technology announces with a field's control.
///
/// GPUI has no `aria-required` or `aria-invalid` slot, so the visible label carries
/// the asterisk and the announced text carries the rest.
fn announced(
    required: bool,
    description: &Option<SharedString>,
    message: &Option<SharedString>,
) -> Option<SharedString> {
    let mut parts: Vec<&str> = Vec::new();
    if required {
        parts.push("Required");
    }
    if let Some(description) = description {
        parts.push(description.as_str());
    }
    if let Some(message) = message {
        parts.push(message.as_str());
    }
    (!parts.is_empty()).then(|| SharedString::from(parts.join(", ")))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputStatus {
    #[default]
    Normal,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputTextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// What kind of value a [`YueInput`] collects, announced to assistive technology.
///
/// Windows has no native channel for the text content type, so the choice is
/// carried by the accessibility role rather than by an on-screen keyboard hint.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputType {
    #[default]
    Text,
    Search,
    Email,
    Url,
}

impl InputType {
    fn role(self, password: bool) -> Role {
        match self {
            // A masked field is protected even when its value is plain text.
            InputType::Text if password => Role::PasswordInput,
            InputType::Text => Role::TextInput,
            InputType::Search => Role::SearchInput,
            InputType::Email => Role::EmailInput,
            InputType::Url => Role::UrlInput,
        }
    }
}

/// The accessibility node a Yue input frame publishes.
struct FrameA11y {
    role: Role,
    label: Option<SharedString>,
    description: Option<SharedString>,
    placeholder: Option<SharedString>,
    value: Option<SharedString>,
    editable: bool,
}

/// Turn the surrounding frame into the control's accessibility node.
///
/// GPUI Base renders the editor as an unlabelled generic container, so the frame
/// is the only node assistive technology can name, read, or focus.
fn accessible_frame<M: InputModeKind>(
    frame: ObservedFrame,
    semantics: FrameA11y,
    state: &Entity<InputBaseState<M>>,
    focus_handle: &gpui_kit::FocusHandle,
) -> ObservedFrame {
    let FrameA11y {
        role,
        label,
        description,
        placeholder,
        value,
        editable,
    } = semantics;
    let focus_state = state.clone();
    let write_state = state.clone();
    frame
        .track_focus(focus_handle)
        .role(role)
        .when_some(label, |this, label| this.aria_label(label))
        .when_some(description, |this, description| {
            this.aria_description(description)
        })
        .when_some(placeholder, |this, placeholder| {
            this.aria_placeholder(placeholder)
        })
        .when_some(value, |this, value| this.aria_value(value))
        .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
            focus_state.update(cx, |input, cx| input.focus(window, cx));
        })
        .when(editable, |this| {
            this.on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                if let Some(gpui_kit::accesskit::ActionData::Value(text)) = data {
                    write_state.update(cx, |input, cx| input.set_value(text.as_ref(), window, cx));
                }
            })
        })
}

enum InputSlot {
    Element(AnyElement),
    Icon(Icon),
}

///
/// ```
/// use yue_component::{Accent, ThemeMode, YueTheme, YueInput};
/// use yue_component::gpui_kit::{Entity, base::input::InputState};
/// # fn example(state: &Entity<InputState>) {
/// let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
/// let control = YueInput::new("field", theme, state).accessibility_label("名称");
/// # }
/// ```
#[derive(IntoElement)]
pub struct YueInput {
    id: ElementId,
    theme: YueTheme,
    use_default_theme: bool,
    state: Entity<gpui_kit::base::input::InputState>,
    size: InputSize,
    status: InputStatus,
    text_align: InputTextAlign,
    input_type: InputType,
    accessibility_label: Option<SharedString>,
    accessibility_description: Option<SharedString>,
    max_length: Option<usize>,
    show_counter: bool,
    placeholder: Option<SharedString>,
    leading: Option<InputSlot>,
    trailing: Option<InputSlot>,
    disabled: bool,
    readonly: bool,
    bordered: bool,
    block: bool,
    clearable: bool,
    password: bool,
    width: Option<DefiniteLength>,
}

/// Yue-styled multi-line text editor backed by GPUI Base `TextareaState`.
///
/// ```
/// use yue_component::{Accent, ThemeMode, YueTheme, YueTextarea};
/// use yue_component::gpui_kit::{Entity, base::input::TextareaState};
/// # fn example(state: &Entity<TextareaState>) {
/// let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
/// let control = YueTextarea::new("field", theme, state).accessibility_label("名称");
/// # }
/// ```
#[derive(IntoElement)]
pub struct YueTextarea {
    id: ElementId,
    theme: YueTheme,
    use_default_theme: bool,
    state: Entity<gpui_kit::base::input::TextareaState>,
    status: InputStatus,
    accessibility_label: Option<SharedString>,
    accessibility_description: Option<SharedString>,
    max_length: Option<usize>,
    show_counter: bool,
    disabled: bool,
    readonly: bool,
    bordered: bool,
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    block: bool,
}

impl YueTextarea {
    /// Resolve the application default at render time, including later theme changes.
    pub fn use_default_theme(mut self) -> Self {
        self.use_default_theme = true;
        self
    }
    /// Override the application default for this component instance.
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.use_default_theme = false;
        self
    }

    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        state: &Entity<gpui_kit::base::input::TextareaState>,
    ) -> Self {
        Self {
            id: id.into(),
            theme,
            use_default_theme: false,
            state: state.clone(),
            status: InputStatus::Normal,
            accessibility_label: None,
            accessibility_description: None,
            max_length: None,
            show_counter: false,
            disabled: false,
            readonly: false,
            bordered: true,
            width: None,
            height: None,
            block: false,
        }
    }

    pub fn status(mut self, status: InputStatus) -> Self {
        self.status = status;
        self
    }
    /// Name this control to assistive technology when no [`YueField`] label owns it.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
    /// Announce a hint, requirement, or error together with this control.
    pub fn accessibility_description(mut self, description: impl Into<SharedString>) -> Self {
        self.accessibility_description = Some(description.into());
        self
    }
    /// Trim every edit that overshoots this character count.
    pub fn max_length(mut self, limit: usize) -> Self {
        self.max_length = Some(limit);
        self
    }
    /// Show the character count inside the frame.
    pub fn show_counter(mut self, show: bool) -> Self {
        self.show_counter = show;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }
    pub fn height(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }
    pub fn block(mut self) -> Self {
        self.block = true;
        self
    }
}

/// The control a [`YueField`] wraps.
///
/// Keeping the builder, rather than a finished element, lets the field inject its
/// label, description, and validation status into the control's accessibility node.
enum FieldChild {
    Select(Box<crate::YueSelect>),
    Element(AnyElement),
    Input(YueInput),
    Textarea(YueTextarea),
    Number(YueNumberInput),
}

/// A field layout that keeps labels and validation copy outside the input editor.
///
/// Prefer [`input`](YueField::input), [`textarea`](YueField::textarea), and
/// [`number_input`](YueField::number_input): the field then names the control for
/// assistive technology and makes the message the single source of its status, so
/// a red border always arrives with the copy that explains it.
///
/// ```
/// use yue_component::{Accent, ThemeMode, YueTheme, YueField};
/// let field = YueField::new(YueTheme::new(ThemeMode::Light, Accent::Azure)).label("名称");
/// ```
#[derive(IntoElement)]
pub struct YueField {
    theme: YueTheme,
    use_default_theme: bool,
    label: Option<SharedString>,
    description: Option<SharedString>,
    message: Option<SharedString>,
    message_status: InputStatus,
    required: bool,
    form: Option<(Entity<YueForm>, SharedString)>,
    child: Option<FieldChild>,
}

impl YueField {
    /// Resolve the application default at render time, including later theme changes.
    pub fn use_default_theme(mut self) -> Self {
        self.use_default_theme = true;
        self
    }
    /// Override the application default for this component instance.
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.use_default_theme = false;
        self
    }

    pub fn new(theme: YueTheme) -> Self {
        Self {
            theme,
            use_default_theme: false,
            label: None,
            description: None,
            message: None,
            message_status: InputStatus::Normal,
            required: false,
            form: None,
            child: None,
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
    pub fn message(mut self, status: InputStatus, message: impl Into<SharedString>) -> Self {
        self.message_status = status;
        self.message = Some(message.into());
        self
    }
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
    /// Take this field's error from a [`YueForm`], revealed by its `submit`.
    ///
    /// The `id` is the one registered with [`YueForm::require`] or [`YueForm::rule`].
    /// While a rule fails, the wrapped control becomes invalid and the rule's message
    /// replaces any set by [`message`](YueField::message).
    pub fn bind(mut self, form: &Entity<YueForm>, id: impl Into<SharedString>) -> Self {
        self.form = Some((form.clone(), id.into()));
        self
    }
    pub fn input(mut self, input: YueInput) -> Self {
        self.child = Some(FieldChild::Input(input));
        self
    }
    /// Preserve the Select's typed value and forward field semantics/status.
    pub fn select(mut self, select: crate::YueSelect) -> Self {
        self.child = Some(FieldChild::Select(Box::new(select)));
        self
    }
    pub fn textarea(mut self, textarea: YueTextarea) -> Self {
        self.child = Some(FieldChild::Textarea(textarea));
        self
    }
    pub fn number_input(mut self, number: YueNumberInput) -> Self {
        self.child = Some(FieldChild::Number(number));
        self
    }
    /// Place an element the field cannot reach: it keeps no label or status.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(FieldChild::Element(child.into_any_element()));
        self
    }
}

/// Yue-styled number input using GPUI Base's spinbutton behavior.
///
/// ```
/// use yue_component::{Accent, ThemeMode, YueTheme, YueNumberInput};
/// use yue_component::gpui_kit::{Entity, base::input::InputState};
/// # fn example(state: &Entity<InputState>) {
/// let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
/// let control = YueNumberInput::new("field", theme, state).accessibility_label("名称");
/// # }
/// ```
#[derive(IntoElement)]
pub struct YueNumberInput {
    id: ElementId,
    theme: YueTheme,
    use_default_theme: bool,
    state: Entity<gpui_kit::base::input::InputState>,
    size: InputSize,
    status: InputStatus,
    accessibility_label: Option<SharedString>,
    accessibility_description: Option<SharedString>,
    on_step: Option<std::rc::Rc<NumberStepHandler>>,
    controls_right: bool,
    controls: bool,
    tab_stop: bool,
    disabled: bool,
    width: Option<DefiniteLength>,
    block: bool,
}

type NumberStepHandler = dyn Fn(gpui_kit::base::StepAction, &mut Window, &mut App);

impl YueNumberInput {
    /// Resolve the application default at render time, including later theme changes.
    pub fn use_default_theme(mut self) -> Self {
        self.use_default_theme = true;
        self
    }
    /// Override the application default for this component instance.
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.use_default_theme = false;
        self
    }

    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        state: &Entity<gpui_kit::base::input::InputState>,
    ) -> Self {
        Self {
            id: id.into(),
            theme,
            use_default_theme: false,
            state: state.clone(),
            size: InputSize::Medium,
            status: InputStatus::Normal,
            accessibility_label: None,
            accessibility_description: None,
            on_step: None,
            controls_right: false,
            controls: true,
            tab_stop: true,
            disabled: false,
            width: None,
            block: false,
        }
    }
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }
    pub fn status(mut self, status: InputStatus) -> Self {
        self.status = status;
        self
    }
    /// Name this spin button to assistive technology.
    ///
    /// GPUI Base publishes the spin button node inside [`gpui_kit::base::NumberInput`].
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
    pub fn accessibility_description(mut self, text: impl Into<SharedString>) -> Self {
        self.accessibility_description = Some(text.into());
        self
    }
    /// Override stepping for a composed editor with a shared numeric model.
    pub fn on_step(
        mut self,
        step: impl Fn(gpui_kit::base::StepAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_step = Some(std::rc::Rc::new(step));
        self
    }
    /// Stack the two step controls in a compact trailing lane.
    pub fn controls_right(mut self) -> Self {
        self.controls_right = true;
        self
    }
    /// Hide the step icons while keeping numeric editing, Up/Down and accessibility actions.
    pub fn controls(mut self, visible: bool) -> Self {
        self.controls = visible;
        self
    }
    pub fn tab_stop(mut self, enabled: bool) -> Self {
        self.tab_stop = enabled;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }
    pub fn block(mut self) -> Self {
        self.block = true;
        self
    }
}

impl YueInput {
    /// Resolve the application default at render time, including later theme changes.
    pub fn use_default_theme(mut self) -> Self {
        self.use_default_theme = true;
        self
    }
    /// Override the application default for this component instance.
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.use_default_theme = false;
        self
    }

    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        state: &Entity<gpui_kit::base::input::InputState>,
    ) -> Self {
        Self {
            id: id.into(),
            theme,
            use_default_theme: false,
            state: state.clone(),
            size: InputSize::Medium,
            status: InputStatus::Normal,
            text_align: InputTextAlign::Left,
            input_type: InputType::Text,
            accessibility_label: None,
            accessibility_description: None,
            max_length: None,
            show_counter: false,
            placeholder: None,
            leading: None,
            trailing: None,
            disabled: false,
            readonly: false,
            bordered: true,
            block: false,
            clearable: false,
            password: false,
            width: None,
        }
    }

    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }
    pub fn status(mut self, status: InputStatus) -> Self {
        self.status = status;
        self
    }
    pub fn text_align(mut self, align: InputTextAlign) -> Self {
        self.text_align = align;
        self
    }
    /// Declare what the value means, so screen readers announce the right kind of box.
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        self
    }
    /// Name this control to assistive technology when no [`YueField`] label owns it.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
    /// Announce a hint, requirement, or error together with this control.
    pub fn accessibility_description(mut self, description: impl Into<SharedString>) -> Self {
        self.accessibility_description = Some(description.into());
        self
    }
    /// Trim every edit that overshoots this character count.
    ///
    /// Pasted and IME text is trimmed while rendering, one frame after it lands.
    /// Filters that must reject an edit before it appears belong to GPUI Base: build
    /// the state with `mask_pattern`, `pattern`, or `validate`.
    pub fn max_length(mut self, limit: usize) -> Self {
        self.max_length = Some(limit);
        self
    }
    /// Show the character count inside the frame, as `used / limit`.
    pub fn show_counter(mut self, show: bool) -> Self {
        self.show_counter = show;
        self
    }
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = Some(text.into());
        self
    }
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(InputSlot::Element(element.into_any_element()));
        self
    }
    pub fn leading_icon(mut self, icon: impl IconDefinition) -> Self {
        self.leading = Some(InputSlot::Icon(Icon::new(icon)));
        self
    }
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(InputSlot::Element(element.into_any_element()));
        self
    }
    pub fn trailing_icon(mut self, icon: impl IconDefinition) -> Self {
        self.trailing = Some(InputSlot::Icon(Icon::new(icon)));
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
    pub fn block(mut self) -> Self {
        self.block = true;
        self
    }
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }
}

impl RenderOnce for YueInput {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.use_default_theme {
            self.theme = crate::current_theme(cx);
        }
        use gpui_kit::base::input::InputEditorStyle;

        let (height, padding, font_size, icon_size) = match self.size {
            InputSize::Small => (
                crate::theme::keys::INPUT_HEIGHT_SM,
                crate::theme::keys::INPUT_PADDING_INLINE_SM,
                crate::theme::keys::INPUT_FONT_SIZE_SM,
                crate::theme::keys::INPUT_ICON_SIZE_SM,
            ),
            InputSize::Medium => (
                crate::theme::keys::INPUT_HEIGHT_MD,
                crate::theme::keys::INPUT_PADDING_INLINE_MD,
                crate::theme::keys::INPUT_FONT_SIZE_MD,
                crate::theme::keys::INPUT_ICON_SIZE_MD,
            ),
            InputSize::Large => (
                crate::theme::keys::INPUT_HEIGHT_LG,
                crate::theme::keys::INPUT_PADDING_INLINE_LG,
                crate::theme::keys::INPUT_FONT_SIZE_LG,
                crate::theme::keys::INPUT_ICON_SIZE_LG,
            ),
        };
        let visibility = self.password.then(|| {
            window.use_keyed_state(
                (self.id.clone(), "yue-input-password-visible"),
                cx,
                |_, _| false,
            )
        });
        let password_visible = visibility.as_ref().is_some_and(|state| *state.read(cx));
        let state = self.state.clone();
        state.update(cx, |input, cx| {
            input.set_disabled(self.disabled, cx);
            input.set_readonly(self.readonly, cx);
            input.set_text_align(
                match self.text_align {
                    InputTextAlign::Left => TextAlign::Left,
                    InputTextAlign::Center => TextAlign::Center,
                    InputTextAlign::Right => TextAlign::Right,
                },
                cx,
            );
            input.set_editor_style(InputEditorStyle {
                foreground: color(
                    self.theme,
                    if self.disabled {
                        crate::theme::keys::INPUT_COLOR_DISABLED
                    } else {
                        crate::theme::keys::INPUT_COLOR
                    },
                )
                .into(),
                muted_foreground: color(self.theme, crate::theme::keys::INPUT_PLACEHOLDER_COLOR)
                    .into(),
                caret: color(self.theme, crate::theme::keys::INPUT_COLOR).into(),
                selection: self.theme.selection(),
                ..Default::default()
            });
            if let Some(placeholder) = &self.placeholder
                && input.presentation().placeholder() != placeholder
            {
                input.set_placeholder(placeholder.clone(), window, cx);
            }
            let should_mask = self.password && !password_visible;
            if input.presentation().is_masked() != should_mask {
                input.set_masked(should_mask, window, cx);
            }
        });
        if let Some(limit) = self.max_length {
            truncate_to_limit(&state, limit, window, cx);
        }

        let presentation = state.read(cx).presentation();
        let focused = presentation.focus_handle().is_focused(window);
        let masked = presentation.is_masked();
        let border = border_token(self.status, self.disabled, focused);
        let height = rems(self.theme, height);
        let pad = rems(self.theme, padding);
        let gap = rems(self.theme, crate::theme::keys::INPUT_GAP);
        let text_size = rems(self.theme, font_size);
        let icon_size = rems(self.theme, icon_size);
        let disabled = self.disabled;
        let value = state.read(cx).value();
        let current_value_nonempty = !value.is_empty();
        let input = div()
            .flex_1()
            .min_w_0()
            .child(gpui_kit::base::input::Input::new(&state));
        let mut frame = div()
            .id(self.id.clone())
            .test_support()
            .flex()
            .items_center()
            .gap(gap)
            .h(height)
            .px(pad)
            .text_size(text_size)
            .bg(color(
                self.theme,
                if disabled {
                    crate::theme::keys::INPUT_BACKGROUND_DISABLED
                } else if self.readonly {
                    crate::theme::keys::INPUT_BACKGROUND_READONLY
                } else {
                    crate::theme::keys::INPUT_BACKGROUND
                },
            ))
            .rounded(rems(self.theme, crate::theme::keys::INPUT_BORDER_RADIUS))
            .border(if self.bordered {
                rems(self.theme, crate::theme::keys::INPUT_BORDER_WIDTH)
            } else {
                gpui_kit::rems(0.0)
            })
            .border_color(color(self.theme, border))
            .when(self.block, |this| this.w_full())
            .when_some(self.width, |this, width| this.w(width))
            .when(disabled, |this| this.opacity(0.72))
            .when(!self.bordered, |this| this.border_color(gpui_kit::rgba(0)))
            .when(!disabled && self.status == InputStatus::Normal, |this| {
                this.hover(|style| {
                    style.border_color(color(
                        self.theme,
                        crate::theme::keys::INPUT_BORDER_COLOR_HOVER,
                    ))
                })
            })
            .when(focused, |this| this.border_color(color(self.theme, border)))
            .when_some(self.leading, |this, leading| {
                let leading = match leading {
                    InputSlot::Element(element) => element,
                    InputSlot::Icon(icon) => icon
                        .size(icon_size)
                        .color_value(color(self.theme, crate::theme::keys::INPUT_AFFIX_COLOR))
                        .into_any_element(),
                };
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_color(color(self.theme, crate::theme::keys::INPUT_AFFIX_COLOR))
                        .child(leading),
                )
            })
            .child(input);
        if self.show_counter {
            frame = frame.child(counter_slot(
                self.theme,
                value.chars().count(),
                self.max_length,
            ));
        }
        if self.password {
            let reveal = state.clone();
            let visibility = visibility.expect("password visibility state exists");
            let mut toggle = if masked {
                YueButton::icon(
                    (self.id.clone(), "input-password-toggle"),
                    self.theme,
                    YueIcon::Visible,
                    "Show password",
                )
            } else {
                YueButton::icon(
                    (self.id.clone(), "input-password-toggle"),
                    self.theme,
                    YueIcon::Hidden,
                    "Hide password",
                )
            };
            toggle = toggle
                .size(match self.size {
                    InputSize::Small => ButtonSize::Small,
                    InputSize::Medium => ButtonSize::Medium,
                    InputSize::Large => ButtonSize::Large,
                })
                .shape(ButtonShape::Circle)
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| {
                    let should_mask = visibility.update(cx, |visible, _| {
                        *visible = !*visible;
                        !*visible
                    });
                    reveal.update(cx, |input, cx| input.set_masked(should_mask, window, cx));
                });
            frame = frame.child(toggle);
        }
        if self.clearable && current_value_nonempty && !disabled && !self.readonly {
            let clear = state.clone();
            frame = frame.child(
                gpui_kit::base::Button::new((self.id.clone(), "input-clear"))
                    .accessibility_label("Clear input")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(rems(self.theme, crate::theme::keys::INPUT_CLEAR_HIT_SIZE))
                    .rounded(rems(
                        self.theme,
                        crate::theme::keys::INPUT_CLEAR_BORDER_RADIUS,
                    ))
                    .bg(gpui_kit::rgba(0))
                    .text_color(color(self.theme, crate::theme::keys::INPUT_CLEAR_COLOR))
                    .cursor_pointer()
                    .hover(|style| {
                        style.text_color(color(
                            self.theme,
                            crate::theme::keys::INPUT_CLEAR_COLOR_HOVER,
                        ))
                    })
                    .on_click(move |_, window, cx| {
                        clear.update(cx, |input, cx| input.clean(window, cx))
                    })
                    .child(
                        Icon::new(YueIcon::Close)
                            .size(icon_size)
                            .color_value(color(self.theme, crate::theme::keys::INPUT_CLEAR_COLOR)),
                    ),
            );
        }
        let frame = accessible_frame(
            frame,
            FrameA11y {
                role: self.input_type.role(self.password),
                label: self.accessibility_label,
                description: self.accessibility_description,
                placeholder: self
                    .placeholder
                    .clone()
                    .or_else(|| Some(presentation.placeholder().clone()))
                    .filter(|text| !text.is_empty()),
                value: (!masked).then(|| value.clone()),
                editable: !disabled && !self.readonly,
            },
            &state,
            presentation.focus_handle(),
        );
        frame.when_some(self.trailing, |this, trailing| {
            let trailing = match trailing {
                InputSlot::Element(element) => element,
                InputSlot::Icon(icon) => icon
                    .size(icon_size)
                    .color_value(color(self.theme, crate::theme::keys::INPUT_AFFIX_COLOR))
                    .into_any_element(),
            };
            this.child(
                div()
                    .flex_shrink_0()
                    .text_color(color(self.theme, crate::theme::keys::INPUT_AFFIX_COLOR))
                    .child(trailing),
            )
        })
    }
}

impl RenderOnce for YueTextarea {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.use_default_theme {
            self.theme = crate::current_theme(cx);
        }
        use gpui_kit::base::input::InputEditorStyle;
        let state = self.state.clone();
        state.update(cx, |input, cx| {
            input.set_disabled(self.disabled, cx);
            input.set_readonly(self.readonly, cx);
            input.set_editor_style(InputEditorStyle {
                foreground: color(
                    self.theme,
                    if self.disabled {
                        crate::theme::keys::INPUT_COLOR_DISABLED
                    } else {
                        crate::theme::keys::INPUT_COLOR
                    },
                )
                .into(),
                muted_foreground: color(self.theme, crate::theme::keys::INPUT_PLACEHOLDER_COLOR)
                    .into(),
                caret: color(self.theme, crate::theme::keys::INPUT_COLOR).into(),
                selection: self.theme.selection(),
                ..Default::default()
            });
            input.set_editor_paddings(gpui_kit::Edges {
                top: gpui_kit::px(token_pixels(
                    self.theme,
                    crate::theme::keys::TEXTAREA_PADDING_BLOCK,
                )),
                right: gpui_kit::px(token_pixels(
                    self.theme,
                    crate::theme::keys::INPUT_PADDING_INLINE_MD,
                )),
                bottom: gpui_kit::px(token_pixels(
                    self.theme,
                    crate::theme::keys::TEXTAREA_PADDING_BLOCK,
                )),
                left: gpui_kit::px(token_pixels(
                    self.theme,
                    crate::theme::keys::INPUT_PADDING_INLINE_MD,
                )),
            });
        });
        if let Some(limit) = self.max_length {
            truncate_to_limit(&state, limit, window, cx);
        }
        let presentation = state.read(cx).presentation();
        let focused = presentation.focus_handle().is_focused(window);
        let value = state.read(cx).value();
        let border = border_token(self.status, self.disabled, focused);
        let mut frame = div()
            .id(self.id)
            .test_support()
            .flex()
            .items_start()
            .text_size(rems(self.theme, crate::theme::keys::INPUT_FONT_SIZE_MD))
            .bg(color(
                self.theme,
                if self.disabled {
                    crate::theme::keys::INPUT_BACKGROUND_DISABLED
                } else {
                    crate::theme::keys::INPUT_BACKGROUND
                },
            ))
            .rounded(rems(self.theme, crate::theme::keys::INPUT_BORDER_RADIUS))
            .border(if self.bordered {
                rems(self.theme, crate::theme::keys::INPUT_BORDER_WIDTH)
            } else {
                gpui_kit::rems(0.0)
            })
            .border_color(color(self.theme, border))
            .when(self.block, |this| this.w_full())
            .when_some(self.width, |this, width| this.w(width))
            .when_some(self.height, |this, height| this.h(height))
            .when(self.disabled, |this| this.opacity(0.72))
            .child(gpui_kit::base::input::Textarea::new(&state));
        if self.show_counter {
            frame = frame.child(
                div()
                    .self_end()
                    .pr(rems(
                        self.theme,
                        crate::theme::keys::INPUT_PADDING_INLINE_SM,
                    ))
                    .child(counter_slot(
                        self.theme,
                        value.chars().count(),
                        self.max_length,
                    )),
            );
        }
        let mut frame = accessible_frame(
            frame,
            FrameA11y {
                role: Role::MultilineTextInput,
                label: self.accessibility_label,
                description: self.accessibility_description,
                placeholder: Some(presentation.placeholder().clone())
                    .filter(|text| !text.is_empty()),
                value: Some(value),
                editable: !self.disabled && !self.readonly,
            },
            &state,
            presentation.focus_handle(),
        );
        if focused && self.status == InputStatus::Normal {
            frame = frame.border_color(color(
                self.theme,
                crate::theme::keys::INPUT_BORDER_COLOR_FOCUS,
            ));
        }
        frame
    }
}

impl RenderOnce for YueNumberInput {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.use_default_theme {
            self.theme = crate::current_theme(cx);
        }
        use gpui_kit::base::input::InputEditorStyle;
        let (height, font_size) = match self.size {
            InputSize::Small => (
                crate::theme::keys::INPUT_HEIGHT_SM,
                crate::theme::keys::INPUT_FONT_SIZE_SM,
            ),
            InputSize::Medium => (
                crate::theme::keys::INPUT_HEIGHT_MD,
                crate::theme::keys::INPUT_FONT_SIZE_MD,
            ),
            InputSize::Large => (
                crate::theme::keys::INPUT_HEIGHT_LG,
                crate::theme::keys::INPUT_FONT_SIZE_LG,
            ),
        };
        let foreground = color(
            self.theme,
            if self.disabled {
                crate::theme::keys::INPUT_COLOR_DISABLED
            } else if self.status == InputStatus::Error {
                crate::theme::keys::ERROR_FOREGROUND
            } else {
                crate::theme::keys::INPUT_COLOR
            },
        );
        let state = self.state.clone();
        state.update(cx, |input, cx| {
            input.set_disabled(self.disabled, cx);
            input.set_text_align(TextAlign::Center, cx);
            input.set_editor_style(InputEditorStyle {
                foreground: foreground.into(),
                muted_foreground: color(self.theme, crate::theme::keys::INPUT_PLACEHOLDER_COLOR)
                    .into(),
                caret: color(self.theme, crate::theme::keys::INPUT_COLOR).into(),
                selection: self.theme.selection(),
                ..Default::default()
            });
        });
        let focused = state
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window);
        let border = border_token(self.status, self.disabled, focused);
        div()
            .id(self.id)
            .when(self.block, |this| this.w_full())
            .when_some(self.width, |this, width| this.w(width))
            .child(
                gpui_kit::base::NumberInput::new(&state)
                    // Base shapes ordinary text from the inherited TextStyle.
                    .text_color(foreground)
                    .controls(self.controls)
                    .tab_stop(self.tab_stop)
                    .when(self.controls_right, |b| b.controls_right())
                    .disabled(self.disabled)
                    .when_some(self.accessibility_label, |this, label| {
                        this.accessibility_label(label)
                    })
                    .when_some(self.accessibility_description, |this, text| {
                        this.accessibility_description(text)
                    })
                    .when_some(self.on_step, |this, handler| {
                        this.on_step(move |action, window, cx| handler(action, window, cx))
                    })
                    .input(gpui_kit::base::input::Input::new(&state))
                    .decrement_button(move |button| {
                        button
                            .size(rems(self.theme, font_size))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(rems(
                                self.theme,
                                crate::theme::keys::INPUT_STEPPER_BORDER_RADIUS,
                            ))
                            .when(!self.disabled, |button| {
                                button
                                    .cursor_pointer()
                                    .hover(|style| {
                                        style.bg(color(
                                            self.theme,
                                            crate::theme::keys::INPUT_STEPPER_BACKGROUND_HOVER,
                                        ))
                                    })
                                    .active(|style| {
                                        style.bg(color(
                                            self.theme,
                                            crate::theme::keys::INPUT_STEPPER_BACKGROUND_PRESSED,
                                        ))
                                    })
                            })
                            .child(
                                Icon::new(YueIcon::ChevronDown)
                                    .size(rems(self.theme, font_size))
                                    .color_value(color(
                                        self.theme,
                                        crate::theme::keys::INPUT_COLOR,
                                    )),
                            )
                    })
                    .increment_button(move |button| {
                        button
                            .size(rems(self.theme, font_size))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(rems(
                                self.theme,
                                crate::theme::keys::INPUT_STEPPER_BORDER_RADIUS,
                            ))
                            .when(!self.disabled, |button| {
                                button
                                    .cursor_pointer()
                                    .hover(|style| {
                                        style.bg(color(
                                            self.theme,
                                            crate::theme::keys::INPUT_STEPPER_BACKGROUND_HOVER,
                                        ))
                                    })
                                    .active(|style| {
                                        style.bg(color(
                                            self.theme,
                                            crate::theme::keys::INPUT_STEPPER_BACKGROUND_PRESSED,
                                        ))
                                    })
                            })
                            .child(
                                Icon::new(YueIcon::ChevronUp)
                                    .size(rems(self.theme, font_size))
                                    .color_value(color(
                                        self.theme,
                                        crate::theme::keys::INPUT_COLOR,
                                    )),
                            )
                    })
                    .text_size(rems(self.theme, font_size))
                    .h(rems(self.theme, height))
                    .px(rems(
                        self.theme,
                        crate::theme::keys::INPUT_NUMBER_PADDING_INLINE,
                    ))
                    .bg(color(
                        self.theme,
                        if self.disabled {
                            crate::theme::keys::INPUT_BACKGROUND_DISABLED
                        } else {
                            crate::theme::keys::INPUT_BACKGROUND
                        },
                    ))
                    .rounded(rems(self.theme, crate::theme::keys::INPUT_BORDER_RADIUS))
                    .border(rems(self.theme, crate::theme::keys::INPUT_BORDER_WIDTH))
                    .border_color(color(self.theme, border)),
            )
    }
}

impl RenderOnce for YueField {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.use_default_theme {
            self.theme = crate::current_theme(cx);
        }
        let bound_error = match &self.form {
            Some((form, id)) => form.read(cx).error(id, cx),
            None => None,
        };
        let status = if bound_error.is_some() {
            InputStatus::Error
        } else {
            self.message_status
        };
        let message = bound_error.or(self.message.clone());
        // While the field has a message, its status is the control's status: a red
        // border never appears without the copy underneath that explains it.
        let child_status = message.is_some().then_some(status);
        let label = self.label.clone();
        let announced = announced(self.required, &self.description, &message);
        let child = self.child.map(|child| match child {
            FieldChild::Select(select) => (*select)
                .required(self.required)
                .when_some(label, |this, label| this.accessibility_label(label))
                .when_some(announced, |this, text| this.accessibility_description(text))
                .when_some(child_status, |this, status| this.status(status))
                .render(window, cx)
                .into_any_element(),
            FieldChild::Element(element) => element,
            FieldChild::Input(input) => input
                .when_some(label, |this, label| this.accessibility_label(label))
                .when_some(announced, |this, description| {
                    this.accessibility_description(description)
                })
                .when_some(child_status, |this, status| this.status(status))
                .render(window, cx)
                .into_any_element(),
            FieldChild::Textarea(textarea) => textarea
                .when_some(label, |this, label| this.accessibility_label(label))
                .when_some(announced, |this, description| {
                    this.accessibility_description(description)
                })
                .when_some(child_status, |this, status| this.status(status))
                .render(window, cx)
                .into_any_element(),
            FieldChild::Number(number) => number
                .when_some(label, |this, label| this.accessibility_label(label))
                .when_some(announced, |this, text| this.accessibility_description(text))
                .when_some(child_status, |this, status| this.status(status))
                .render(window, cx)
                .into_any_element(),
        });
        let message_color = field_message_token(status);
        div()
            .flex()
            .flex_col()
            .gap(rems(self.theme, crate::theme::keys::INPUT_GAP))
            .when_some(self.label, |this, label| {
                this.child(
                    div()
                        .text_color(color(self.theme, crate::theme::keys::INPUT_COLOR))
                        .child(label)
                        .when(self.required, |label| label.child(" *")),
                )
            })
            .when_some(self.description, |this, description| {
                this.child(
                    div()
                        .text_color(color(self.theme, crate::theme::keys::INPUT_AFFIX_COLOR))
                        .text_size(rems(self.theme, crate::theme::keys::INPUT_FONT_SIZE_SM))
                        .child(description),
                )
            })
            .when_some(child, |this, child| this.child(child))
            .when_some(message, |this, message| {
                this.child(
                    div()
                        .text_color(color(self.theme, message_color))
                        .text_size(rems(self.theme, crate::theme::keys::INPUT_FONT_SIZE_SM))
                        .child(message),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{InputStatus, border_token, color, field_message_token, rems, token_pixels};
    use crate::{Accent, ThemeMode, YueTheme};

    #[test]
    fn input_options_and_tokens_cover_every_theme_profile() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            for accent in [Accent::Azure, Accent::Neutral] {
                let theme = YueTheme::new(mode, accent);
                assert_eq!(rems(theme, crate::theme::keys::INPUT_HEIGHT_SM).0, 1.75);
                assert_eq!(rems(theme, crate::theme::keys::INPUT_HEIGHT_MD).0, 2.0);
                assert_eq!(rems(theme, crate::theme::keys::INPUT_HEIGHT_LG).0, 2.5);
                assert_eq!(
                    token_pixels(theme, crate::theme::keys::TEXTAREA_PADDING_BLOCK),
                    8.0
                );
                assert_eq!(color(theme, crate::theme::keys::INPUT_BACKGROUND).a, 1.0);
                assert!(color(theme, crate::theme::keys::INPUT_BACKGROUND_DISABLED).a < 1.0);
                assert!(color(theme, crate::theme::keys::INPUT_BORDER_COLOR_INVALID).a > 0.99);
            }
        }
    }

    #[test]
    fn validation_border_semantics_survive_hover_focus_and_disabled_states() {
        assert_eq!(
            border_token(InputStatus::Normal, false, false),
            crate::theme::keys::INPUT_BORDER_COLOR
        );
        assert_eq!(
            border_token(InputStatus::Normal, false, true),
            crate::theme::keys::INPUT_BORDER_COLOR_FOCUS
        );
        assert_eq!(
            border_token(InputStatus::Success, false, true),
            crate::theme::keys::SUCCESS
        );
        assert_eq!(
            border_token(InputStatus::Warning, false, true),
            crate::theme::keys::WARNING
        );
        assert_eq!(
            border_token(InputStatus::Error, false, true),
            crate::theme::keys::INPUT_BORDER_COLOR_INVALID
        );
        assert_eq!(
            border_token(InputStatus::Error, true, true),
            crate::theme::keys::INPUT_BORDER_COLOR_DISABLED
        );
    }

    #[test]
    fn field_message_statuses_resolve_to_yue_semantic_tokens() {
        assert_eq!(
            field_message_token(InputStatus::Normal),
            crate::theme::keys::INPUT_AFFIX_COLOR
        );
        assert_eq!(
            field_message_token(InputStatus::Success),
            crate::theme::keys::SUCCESS_FOREGROUND
        );
        assert_eq!(
            field_message_token(InputStatus::Warning),
            crate::theme::keys::WARNING_FOREGROUND
        );
        assert_eq!(
            field_message_token(InputStatus::Error),
            crate::theme::keys::ERROR_FOREGROUND
        );
    }
}
