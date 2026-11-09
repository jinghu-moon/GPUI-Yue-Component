mod pages;
use pages::button::*;
use pages::checkbox::*;
use pages::dropdown::*;
use pages::form::*;
use pages::input::*;
use pages::radio::*;
use pages::switch::*;
mod color;
mod dark_mode;
mod font;
mod icon;
mod layout;
mod motion;
mod scroll;
mod sidebar;

use crate::scroll::ScrollableElement as _;
use gpui_kit::base::slider::SliderEvent;
use gpui_kit::base::{Transition, transition};
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    WindowOptions, div, prelude::FluentBuilder as _,
};
use yue_component::{
    Accent, ButtonGroup, ButtonShape, ButtonSize, ButtonVariant, CheckboxSize, CheckboxState,
    DropdownAlign, DropdownIndicatorSide, DropdownPlacement, GroupDirection, IconPosition,
    InputSize, InputStatus, InputTextAlign, InputType, NeutralTone, RadioAppearance, RadioSize,
    SwitchIndicator, SwitchShape, SwitchSize, ThemeMode, ThemePreference, YueButton, YueCheckbox,
    YueCheckboxGroup, YueCheckboxOption, YueContextMenu, YueDropdownMenu, YueDropdownSplitButton,
    YueField, YueForm, YueInput, YueMenuItem, YueNumberInput, YueRadio, YueRadioGroup,
    YueRadioOption, YueSwitch, YueTextarea, YueTheme, outline,
};

fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure))
            .expect("initialize Yue");
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Showcase::new(window, cx))
        })
        .expect("failed to open Yue component showcase window");
    });
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ShowcasePage {
    Scrollbar,
    Resizable,
    Tree,
    TreeSelect,
    DatePicker,
    TimePicker,
    Empty,
    Avatar,
    Badge,
    Breadcrumb,
    #[default]
    Button,
    Icon,
    IconDesign,
    Input,
    ImagePreview,
    Link,
    Progress,
    Spinner,
    Skeleton,
    Form,
    Switch,
    Tag,
    Shortcut,
    Tabs,
    Radio,
    Segmented,
    Select,
    Slider,
    Checkbox,
    Collapse,
    List,
    VirtualList,
    VirtualLayouts,
    ColorPicker,
    Tooltip,
    Toast,
    Dialog,
    Divider,
    Dropdown,
    Color,
    DarkMode,
    Font,
    Motion,
    Layout,
}

struct Showcase {
    scrollbar_demo: gpui_kit::Entity<pages::scrollbar::ScrollbarPage>,
    resizable_demo: gpui_kit::Entity<pages::resizable::ResizablePage>,
    tree_demo: gpui_kit::Entity<pages::tree::TreePage>,
    tree_select_demo: gpui_kit::Entity<pages::tree_select::TreeSelectPage>,
    time_picker_demo: gpui_kit::Entity<pages::time_picker::TimePickerPage>,
    date_picker_demo: gpui_kit::Entity<pages::date_picker::DatePickerPage>,
    empty_demo: gpui_kit::Entity<pages::empty::EmptyPage>,
    badge_demo: gpui_kit::Entity<pages::badge::BadgePage>,
    shortcut_demo: Option<gpui_kit::Entity<pages::shortcut::ShortcutPage>>,
    breadcrumb_demo: gpui_kit::Entity<pages::breadcrumb::BreadcrumbPage>,
    slider_demo: gpui_kit::Entity<pages::slider::SliderPage>,
    page: ShowcasePage,
    color_picker_demo: gpui_kit::Entity<pages::color_picker::ColorPickerPage>,
    tooltip_demo: gpui_kit::Entity<pages::tooltip::TooltipPage>,
    link_demo: gpui_kit::Entity<pages::link::LinkPage>,
    divider_demo: gpui_kit::Entity<pages::divider::DividerPage>,
    dialog_demo: gpui_kit::Entity<pages::dialog::DialogPage>,
    toast_demo: gpui_kit::Entity<pages::toast::ToastPage>,
    tag_demo: gpui_kit::Entity<pages::tag::TagPage>,
    tabs_demo: gpui_kit::Entity<pages::tabs::TabsPage>,
    collapse_demo: gpui_kit::Entity<pages::collapse::CollapsePage>,
    list_demo: Option<gpui_kit::Entity<pages::list::ListPage>>,
    virtual_list_demo: Option<gpui_kit::Entity<pages::virtual_list::VirtualListPage>>,
    virtual_layouts_demo: Option<gpui_kit::Entity<pages::virtual_layouts::VirtualLayoutsPage>>,
    image_preview_demo: Option<gpui_kit::Entity<pages::image_preview::ImagePreviewPage>>,
    progress_demo: gpui_kit::Entity<pages::loading::FeedbackPage>,
    spinner_demo: gpui_kit::Entity<pages::loading::FeedbackPage>,
    skeleton_demo: gpui_kit::Entity<pages::loading::FeedbackPage>,
    select_demo: gpui_kit::Entity<pages::select::SelectPageView>,
    sidebar_open: [bool; 2],
    segmented_values: [SharedString; 16],
    segmented_changes: [usize; 16],
    /// Whether the app forces Base's reduced-motion flag on, overriding what
    /// `gpui_kit::init` read from the system.
    motion_off: bool,
    /// The Motion page: which demos are played, the hovered tile and menu
    /// rows, and whether every demo runs four times slower.
    motion_play: [bool; motion::PLAY_SLOTS],
    motion_hover: bool,
    motion_rows: [Option<usize>; 2],
    motion_chase: usize,
    motion_slow: bool,
    /// The ramp self-check runs while the pointer holds its box down.
    probe_held: bool,
    /// How many times the window repainted since the self-check was grabbed:
    /// a ramp that is being fed frames climbs across dozens of these.
    probe_frames: u32,
    /// The Layout page: the window width the slider is simulating.
    layout_sim: f32,
    layout_width: gpui_kit::Entity<gpui_kit::base::slider::SliderState>,
    input_basic: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_disabled: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_readonly: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_password: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_affix: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_unit: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    input_sizes: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_statuses: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_alignments: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_narrow: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_semantic: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_locked: Vec<gpui_kit::Entity<gpui_kit::base::input::InputState>>,
    input_counter: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    textarea: gpui_kit::Entity<gpui_kit::base::input::TextareaState>,
    number_input: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    form: gpui_kit::Entity<YueForm>,
    form_name: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    form_email: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    form_note: gpui_kit::Entity<gpui_kit::base::input::TextareaState>,
    last_submit: Option<bool>,
    switch_states: Vec<bool>,
    switch_async: bool,
    switch_async_loading: bool,
    radio_single: Vec<bool>,
    radio_vertical: Option<SharedString>,
    radio_horizontal: Option<SharedString>,
    radio_segment: Option<SharedString>,
    radio_sizes: Vec<Option<SharedString>>,
    radio_segments: Vec<Option<SharedString>>,
    radio_profiles: Vec<Option<SharedString>>,
    radio_empty: Option<SharedString>,
    radio_locked: Option<SharedString>,
    radio_keyboard: Option<SharedString>,
    radio_narrow: Option<SharedString>,
    checkbox_single: Vec<CheckboxState>,
    checkbox_sizes: Vec<CheckboxState>,
    checkbox_tags: Vec<SharedString>,
    checkbox_manual: Vec<SharedString>,
    checkbox_built_in: Vec<SharedString>,
    checkbox_mixed: Vec<SharedString>,
    checkbox_locked: Vec<SharedString>,
    checkbox_narrow: Vec<SharedString>,
    checkbox_profiles: Vec<Vec<SharedString>>,
    dropdown_log: Vec<Option<SharedString>>,
    dropdown_sets: Vec<Vec<SharedString>>,
    dropdown_open: bool,
    dropdown_loading: bool,
    dropdown_remote: Vec<SharedString>,
    dropdown_suffix: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    /// The Color page shows one profile at a time, picked on the page itself.
    color_mode: ThemeMode,
    color_accent: Accent,
    color_neutral: NeutralTone,
    /// Pointer over the two live ramps of the blending demo.
    color_hover: [bool; 2],
    color_input: gpui_kit::Entity<gpui_kit::base::input::InputState>,
    /// The profile last projected onto Base's global theme.
    base_profile: (ThemeMode, Accent, NeutralTone),
    /// The Dark Mode page's theme: the user's pick, and the window's
    /// appearance, kept current by `_appearance`.
    theme_preference: ThemePreference,
    appearance: gpui_kit::WindowAppearance,
    _appearance: gpui_kit::Subscription,
}

/// The Dropdown page reports the row each menu ran and keeps its own copy of the
/// marks it shows, so every click has a consequence the reader can see. One slot
/// per section that needs either.
const DD_COMMAND: usize = 0;
const DD_ALIGN: usize = 1;
const DD_FLIP: usize = 2;
const DD_SUBMENU: usize = 3;
const DD_LONG: usize = 4;
const DD_TRUNCATE: usize = 5;
const DD_ICON: usize = 6;
const DD_GROUP: usize = 7;
const DD_SUFFIX: usize = 8;
const DD_CONTEXT: usize = 9;
const DD_SPLIT: usize = 10;
const DD_ASYNC: usize = 11;
const DD_KEYBOARD: usize = 12;
const DD_LOG_SLOTS: usize = 13;
const DD_VIEWS: usize = 0;
const DD_THEME: usize = 1;
const DD_COLOR: usize = 2;
const DD_CHANNELS: usize = 3;
const DD_PACE: usize = 4;

/// How long the dropdown's async example holds its loading row before the remote
/// names arrive: three turns of the row's own spinner.
const DROPDOWN_ASYNC_LOAD: std::time::Duration = std::time::Duration::from_millis(2700);

impl Showcase {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form_name = cx.new(|cx| {
            gpui_kit::base::input::InputState::new(window, cx).placeholder("例如 Roadmap")
        });
        let form_email = cx.new(|cx| {
            gpui_kit::base::input::InputState::new(window, cx).placeholder("name@example.com")
        });
        let form_note = cx.new(|cx| {
            gpui_kit::base::input::TextareaState::new(window, cx)
                .auto_grow(2, 5)
                .placeholder("可选：这次改动要解决的问题")
        });
        let form = cx.new(|_| YueForm::new());
        let this = cx.entity();
        form.update(cx, |form, cx| {
            form.require("name", &form_name, "请填写项目名称");
            form.rule("email", &form_email, |value, _| {
                let complete = value
                    .rsplit_once('@')
                    .is_some_and(|(_, domain)| !domain.is_empty() && domain.contains('.'));
                (!complete).then(|| SharedString::from("填写完整的邮箱，例如 name@example.com"))
            });
            form.attach(&form_name, window, cx);
            form.attach(&form_email, window, cx);
            form.on_submit(move |valid, _window, cx| {
                this.update(cx, |showcase, cx| {
                    if showcase.last_submit != Some(valid) {
                        showcase.last_submit = Some(valid);
                        cx.notify();
                    }
                });
            });
        });
        let layout_width = cx.new(|_| layout::simulated_width_state());
        cx.subscribe(&layout_width, |showcase, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                showcase.layout_sim = value.end();
                cx.notify();
            }
        })
        .detach();
        Self {
            color_picker_demo: cx.new(|cx| pages::color_picker::ColorPickerPage::new(window, cx)),
            tooltip_demo: cx.new(pages::tooltip::TooltipPage::new),
            link_demo: cx.new(pages::link::LinkPage::new),
            divider_demo: cx.new(pages::divider::DividerPage::new),
            dialog_demo: cx.new(|cx| pages::dialog::DialogPage::new(window, cx)),
            toast_demo: cx.new(|cx| pages::toast::ToastPage::new(window, cx)),
            tag_demo: cx.new(|cx| pages::tag::TagPage::new(window, cx)),
            tabs_demo: cx.new(|cx| pages::tabs::TabsPage::new(window, cx)),
            collapse_demo: cx.new(|cx| pages::collapse::CollapsePage::new(window, cx)),
            list_demo: None,
            virtual_list_demo: None,
            virtual_layouts_demo: None,
            image_preview_demo: None,
            progress_demo: cx.new(|_| {
                pages::loading::FeedbackPage::new(pages::loading::FeedbackPageKind::Progress)
            }),
            spinner_demo: cx.new(|_| {
                pages::loading::FeedbackPage::new(pages::loading::FeedbackPageKind::Spinner)
            }),
            skeleton_demo: cx.new(|_| {
                pages::loading::FeedbackPage::new(pages::loading::FeedbackPageKind::Skeleton)
            }),
            select_demo: cx.new(|cx| pages::select::SelectPageView::new(window, cx)),
            breadcrumb_demo: cx.new(pages::breadcrumb::BreadcrumbPage::new),
            slider_demo: cx.new(|cx| pages::slider::SliderPage::new(window, cx)),
            badge_demo: cx.new(|_| pages::badge::BadgePage::new()),
            time_picker_demo: cx.new(|cx| pages::time_picker::TimePickerPage::new(window, cx)),
            tree_demo: cx.new(|cx| pages::tree::TreePage::new(window, cx)),
            tree_select_demo: cx.new(|cx| pages::tree_select::TreeSelectPage::new(window, cx)),
            date_picker_demo: cx.new(|cx| pages::date_picker::DatePickerPage::new(window, cx)),
            empty_demo: cx.new(pages::empty::EmptyPage::new),
            scrollbar_demo: cx.new(pages::scrollbar::ScrollbarPage::new),
            resizable_demo: cx.new(|cx| pages::resizable::ResizablePage::new(window, cx)),
            shortcut_demo: None,
            page: ShowcasePage::Button,
            sidebar_open: [true, true],
            segmented_values: [
                "day", "grid", "yearly", "day", "day", "day", "day", "day", "day", "list", "list",
                "day", "absent", "day", "day", "day",
            ]
            .map(SharedString::from),
            segmented_changes: [0; 16],
            input_basic: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).placeholder("请输入内容")
            }),
            input_disabled: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("不可编辑")
            }),
            input_readonly: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("可聚焦与复制")
            }),
            input_password: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("yue-component")
            }),
            input_affix: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).placeholder("搜索项目")
            }),
            input_unit: cx
                .new(|cx| gpui_kit::base::input::InputState::new(window, cx).placeholder("重量")),
            input_sizes: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx)
                            .default_value("Yue Input")
                    })
                })
                .collect(),
            input_statuses: (0..4)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).placeholder("请输入内容")
                    })
                })
                .collect(),
            input_alignments: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx)
                            .default_value("Yue Input")
                    })
                })
                .collect(),
            input_narrow: ["Yue", "yue@example.com", "3.14"]
                .into_iter()
                .map(|value| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).default_value(value)
                    })
                })
                .collect(),
            input_semantic: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).placeholder("请输入内容")
                    })
                })
                .collect(),
            input_locked: (0..3)
                .map(|_| {
                    cx.new(|cx| {
                        gpui_kit::base::input::InputState::new(window, cx).default_value("锁定内容")
                    })
                })
                .collect(),
            input_counter: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx)
                    .default_value("表单可访问性与状态一致性")
            }),
            textarea: cx.new(|cx| {
                gpui_kit::base::input::TextareaState::new(window, cx)
                    .auto_grow(3, 6)
                    .placeholder("请输入多行内容")
            }),
            number_input: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx)
                    .step(1.0)
                    .min(0.0)
                    .max(100.0)
                    .default_value("12")
            }),
            form,
            form_name,
            form_email,
            form_note,
            last_submit: None,
            switch_states: std::iter::once(false)
                .chain(std::iter::once(true))
                .chain((0..24).map(|index| index % 2 == 1))
                .collect(),
            switch_async: false,
            switch_async_loading: false,
            radio_single: vec![false, true, true, false],
            radio_vertical: Some("png".into()),
            radio_horizontal: Some("light".into()),
            radio_segment: Some("regular".into()),
            radio_sizes: vec![Some("regular".into()); 3],
            radio_segments: vec![
                None,
                Some("regular".into()),
                Some("regular".into()),
                Some("regular".into()),
            ],
            radio_profiles: vec![Some("svg".into()); 4],
            radio_empty: None,
            radio_locked: Some("locked".into()),
            radio_keyboard: Some("weekly".into()),
            radio_narrow: Some("long".into()),
            checkbox_single: vec![
                CheckboxState::Unchecked,
                CheckboxState::Checked,
                CheckboxState::Indeterminate,
                CheckboxState::Checked,
                CheckboxState::Unchecked,
            ],
            checkbox_sizes: vec![
                CheckboxState::Unchecked,
                CheckboxState::Checked,
                CheckboxState::Indeterminate,
            ],
            checkbox_tags: vec!["docs".into()],
            checkbox_manual: vec!["desktop".into()],
            checkbox_built_in: vec!["mail".into(), "tasks".into()],
            checkbox_mixed: vec!["web".into()],
            checkbox_locked: vec!["readme".into()],
            checkbox_narrow: vec!["long".into()],
            checkbox_profiles: vec![vec!["docs".into()]; 4],
            dropdown_log: vec![None; DD_LOG_SLOTS],
            dropdown_sets: vec![
                vec!["已完成".into()],
                vec!["跟随系统".into()],
                vec!["天蓝".into()],
                vec!["邮件".into()],
                vec!["即时".into()],
            ],
            dropdown_open: false,
            dropdown_loading: false,
            dropdown_remote: Vec::new(),
            dropdown_suffix: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx).default_value("周报草稿")
            }),
            color_mode: ThemeMode::Light,
            color_accent: Accent::Azure,
            color_neutral: NeutralTone::Neutral,
            color_hover: [false; 2],
            color_input: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx)
                    .default_value("拖选这段文字查看选区颜色")
            }),
            base_profile: (ThemeMode::Light, Accent::Azure, NeutralTone::Neutral),
            theme_preference: ThemePreference::System,
            appearance: window.appearance(),
            _appearance: cx.observe_window_appearance(window, |showcase, window, cx| {
                showcase.appearance = window.appearance();
                showcase.sync_theme(cx);
                cx.notify();
            }),
            motion_off: cx.reduce_motion(),
            motion_play: [false; motion::PLAY_SLOTS],
            motion_hover: false,
            motion_rows: [None; 2],
            motion_chase: 0,
            motion_slow: false,
            probe_held: false,
            probe_frames: 0,
            layout_sim: layout::SIM_DEFAULT,
            layout_width,
        }
    }
}

impl Showcase {
    fn sync_theme(&mut self, cx: &mut Context<Self>) {
        let window_profile = if self.page == ShowcasePage::Color {
            (self.color_mode, self.color_accent, self.color_neutral)
        } else if self.page == ShowcasePage::DarkMode {
            (self.dark_mode_theme(), Accent::Azure, NeutralTone::Neutral)
        } else {
            (ThemeMode::Light, Accent::Azure, NeutralTone::Neutral)
        };
        if window_profile != self.base_profile {
            yue_component::set_theme(
                cx,
                YueTheme::new(window_profile.0, window_profile.1).with_neutral(window_profile.2),
            );
            self.base_profile = window_profile;
        }
    }

    /// The mode the Dark Mode page wears: the user's pick, or the system's.
    fn dark_mode_theme(&self) -> ThemeMode {
        self.theme_preference.resolve(self.appearance)
    }
}

impl Render for Showcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        self.probe_frames = if self.probe_held {
            self.probe_frames + 1
        } else {
            0
        };

        div()
            .size_full()
            .flex()
            .font_family(yue_component::typography::ui_font(theme).family)
            .bg(gpui_kit::rgb(theme.page()))
            .text_color(gpui_kit::rgb(theme.text_primary()))
            .child(sidebar::sidebar(window, theme, self, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .min_h_0()
                    .child(match self.page {
                        ShowcasePage::Tree => self.tree_demo.clone().into_any_element(),
                        ShowcasePage::TreeSelect => self.tree_select_demo.clone().into_any_element(),
                        ShowcasePage::TimePicker => self.time_picker_demo.clone().into_any_element(),
                        ShowcasePage::DatePicker => self.date_picker_demo.clone().into_any_element(),
                        ShowcasePage::Empty => self.empty_demo.clone().into_any_element(),
                        ShowcasePage::Scrollbar => self.scrollbar_demo.clone().into_any_element(),
                        ShowcasePage::Resizable => self.resizable_demo.clone().into_any_element(),
                        ShowcasePage::Badge => self.badge_demo.clone().into_any_element(),
                        ShowcasePage::Avatar => {
                            pages::avatar::avatar_content(theme).into_any_element()
                        }
                        ShowcasePage::Button => content(theme).into_any_element(),
                        ShowcasePage::Icon => pages::icon::icon_content(theme).into_any_element(),
                        ShowcasePage::IconDesign => icon::icon_content(theme).into_any_element(),
                        ShowcasePage::Input => input_content(theme, self).into_any_element(),
                        ShowcasePage::Link => self.link_demo.clone().into_any_element(),
                        ShowcasePage::Form => form_content(theme, self, cx).into_any_element(),
                        ShowcasePage::Switch => switch_content(theme, self, cx).into_any_element(),
                        ShowcasePage::Radio => radio_content(theme, self, cx).into_any_element(),
                        ShowcasePage::ColorPicker => {
                            self.color_picker_demo.clone().into_any_element()
                        }
                        ShowcasePage::Tooltip => self.tooltip_demo.clone().into_any_element(),
                        ShowcasePage::Toast => self.toast_demo.clone().into_any_element(),
                        ShowcasePage::Tag => self.tag_demo.clone().into_any_element(),
                        ShowcasePage::Shortcut => self
                            .shortcut_demo
                            .get_or_insert_with(|| {
                                cx.new(|cx| pages::shortcut::ShortcutPage::new(window, cx))
                            })
                            .clone()
                            .into_any_element(),
                        ShowcasePage::Tabs => self.tabs_demo.clone().into_any_element(),
                        ShowcasePage::Collapse => self.collapse_demo.clone().into_any_element(),
                        ShowcasePage::List => self
                            .list_demo
                            .get_or_insert_with(|| {
                                cx.new(|cx| pages::list::ListPage::new(window, cx))
                            })
                            .clone()
                            .into_any_element(),
                        ShowcasePage::VirtualLayouts => self
                            .virtual_layouts_demo
                            .get_or_insert_with(|| {
                                cx.new(|cx| {
                                    pages::virtual_layouts::VirtualLayoutsPage::new_with_focus(
                                        window, cx,
                                    )
                                })
                            })
                            .clone()
                            .into_any_element(),
                        ShowcasePage::VirtualList => self
                            .virtual_list_demo
                            .get_or_insert_with(|| {
                                cx.new(|cx| pages::virtual_list::VirtualListPage::new(window, cx))
                            })
                            .clone()
                            .into_any_element(),
                        ShowcasePage::ImagePreview => self
                            .image_preview_demo
                            .get_or_insert_with(|| {
                                cx.new(|cx| pages::image_preview::ImagePreviewPage::new(window, cx))
                            })
                            .clone()
                            .into_any_element(),
                        ShowcasePage::Progress => self.progress_demo.clone().into_any_element(),
                        ShowcasePage::Spinner => self.spinner_demo.clone().into_any_element(),
                        ShowcasePage::Skeleton => self.skeleton_demo.clone().into_any_element(),
                        ShowcasePage::Select => self.select_demo.clone().into_any_element(),
                        ShowcasePage::Dialog => self.dialog_demo.clone().into_any_element(),
                        ShowcasePage::Divider => self.divider_demo.clone().into_any_element(),
                        ShowcasePage::Breadcrumb => self.breadcrumb_demo.clone().into_any_element(),
                        ShowcasePage::Slider => self.slider_demo.clone().into_any_element(),
                        ShowcasePage::Segmented => {
                            pages::segmented::segmented_content(theme, self, cx).into_any_element()
                        }
                        ShowcasePage::Checkbox => {
                            checkbox_content(theme, self, cx).into_any_element()
                        }
                        ShowcasePage::Dropdown => {
                            dropdown_content(theme, self, cx).into_any_element()
                        }
                        ShowcasePage::Color => {
                            color::color_content(self, window, cx).into_any_element()
                        }
                        ShowcasePage::DarkMode => {
                            dark_mode::dark_mode_content(self, cx).into_any_element()
                        }
                        ShowcasePage::Font => font::font_content(theme).into_any_element(),
                        ShowcasePage::Motion => {
                            motion::motion_content(self, window, cx).into_any_element()
                        }
                        ShowcasePage::Layout => {
                            layout::layout_content(self, window, cx).into_any_element()
                        }
                    })
                    .overflow_y_scrollbar(),
            )
    }
}

/// How long the async switch holds its loading ring before the state commits:
/// one revolution of the ring, whose period is `--spinner-duration` (900ms).
const SWITCH_ASYNC_COMMIT: std::time::Duration = std::time::Duration::from_millis(900);

fn readout(theme: YueTheme, text: String) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(gpui_kit::rgb(theme.text_secondary()))
        .child(text)
}

fn value_text(value: Option<&SharedString>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "（空）".to_string())
}

fn values_text(values: &[SharedString]) -> String {
    if values.is_empty() {
        return "（空）".to_string();
    }
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>()
        .join("、")
}

fn theme_tile(title: &'static str, theme: YueTheme, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
        .text_color(gpui_kit::rgb(theme.text_primary()))
        .child(div().text_sm().child(title))
        .child(body)
}

const PROFILES: [(&str, ThemeMode, Accent); 4] = [
    ("Light · Azure", ThemeMode::Light, Accent::Azure),
    ("Dark · Azure", ThemeMode::Dark, Accent::Azure),
    ("Light · Neutral", ThemeMode::Light, Accent::Neutral),
    ("Dark · Neutral", ThemeMode::Dark, Accent::Neutral),
];

fn section(
    theme: YueTheme,
    title: &'static str,
    description: &'static str,
    example: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child(title))
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(description),
        )
        .child(
            div()
                .flex()
                .items_center()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .bg(gpui_kit::rgb(theme.surface()))
                .child(example),
        )
}
