//! Yue design tokens and components for applications built on GPUI Kit.
//!
//! Applications own their window and lifecycle. This crate provides presentation
//! data and components while reusing the GPUI types pinned by `gpui-kit`.

mod application;
mod avatar;
mod badge;
mod breadcrumb;
mod button;
mod card;
mod checkbox;
mod choice_field;
mod collapse;
mod color_picker;
mod control_layout;
mod date_picker;
mod descriptions;
mod time_picker;
mod dialog;
mod divider;
mod drawer;
mod dropdown;
mod empty;
mod field;
mod form;
pub mod icon;
mod image_preview;
mod input;
mod kbd;
pub mod layout;
mod link;
mod list;
mod loading;
pub mod motion;
mod popup;
mod pagination;
mod radio;
mod resizable;
mod scrollbar;
mod segmented;
mod select;
mod shortcut_recorder;
mod slider;
mod steps;
mod table;
pub use table::TableRemotePage;
pub use table::TableRowSizing;
pub use table::{
    TableCapabilities, TableColumnConfigurationRequest, TableColumnDraft, TableColumnMoveRequest,
    TableColumnPreference, TableColumnPreferences, TableMovePlacement, TableRowDrag,
    TableRowMoveRequest, TableRowMoveScope,
};
pub use table::{TableSpan, TableSpanPlan, TableRangeRows, TableRangeRequest};
pub use table::{TableOutline, TableOutlineRow, TableOutlineSource};
pub use table::{TableEditMode, TableEditMove, TableEditStage, TableEditToken, TableEditSession};
pub use table::{TableDataMode, TableTotal, TableSortDirection, TableSort, TablePage, TableFilterValue, TableFilterKey, TableQuery, TableValueColumn, TableProjection, TableLocalQuery, TableQuerySource, TableQueryRequest, sync_table_pagination};
mod swatch;
mod switch;
mod tabs;
mod tag;
pub mod theme;
mod toast;
mod tree;
mod tree_select;
mod tokens;
mod tooltip;
pub mod typography;
mod virtual_list;
pub use table::{TableColumnResizeRequest, TablePin, TableHeaderGroup, TableAlign, TableBorders, TableCellContext, TableCellKey, TableColumn, TableColumnKey, TableCommitOutcome,
    TableDelegate, TableError, TableEvent, TableLoadState, TableRenderMode, TableRequestId, TableRowKey,
    TableRows, TableSelectionMode, TableSelectionRequest, TableSize, TableSource, TableTextOverflow,
    TableVerticalAlign, TableView, YueTable, YueTableState};
pub use virtual_list::{
    VirtualGrid, VirtualGridCell, VirtualGridCellIdentity, VirtualGridCellKey, VirtualGridSnapshot,
    VirtualGridSpan, VirtualGridSpanSet, VirtualGridTrack, VirtualMasonry, VirtualMasonryCache,
    VirtualMasonryCacheItem, VirtualMasonryCapabilities, VirtualMasonryItem,
    VirtualMasonrySnapshot, VirtualMasonryState, VirtualizerAlign, VirtualizerAnchorPosition,
    VirtualizerAttachment, VirtualizerBinding, VirtualizerCache, VirtualizerCapabilities,
    VirtualizerChange, VirtualizerCommandOutcome, VirtualizerError, VirtualizerEvent,
    VirtualizerItem, VirtualizerItemIdentity, VirtualizerLayout, VirtualizerMeasurement,
    VirtualizerMotion, VirtualizerRange, VirtualizerRangeExtractor, VirtualizerScrollDirection,
    VirtualizerScrollOptions, VirtualizerScrollRequest, VirtualizerScrollbarHandle,
    VirtualizerSizeUpdate, VirtualizerSizing, VirtualizerSizingKind, VirtualizerSnapshot,
    VirtualizerSource, VirtualizerState, VirtualizerViewport, YueVirtualList,
};

pub use application::{current_theme, init, set_theme};
pub use drawer::{DrawerPlacement, DrawerSize, DrawerChange, DrawerCloseReason, DrawerDecision,
    DrawerFooter, DrawerOpenError, DrawerOutcome, DrawerSubmitRequest, DrawerTicket,
    YueDrawer, YueDrawerHost, YueDrawerEvent, YueDrawerState};
pub use scrollbar::{ScrollbarVisibility, YueScrollbar};
pub use tree::{
    TreeApply, TreeChange, TreeChangeSource, TreeCheckMode, TreeCheckProjection, TreeChildren,
    TreeDataError, TreeEvent, TreeFilter, TreeLoadState, TreeLoadTicket, TreeMark, TreeMove,
    TreeNode, TreeReconciliation, TreeRow, TreeRowAction, TreeSelectionMode, TreeSize, YueTree,
    YueTreeState,
};
pub use tree_select::{
    TreeSelectChange, TreeSelectChangeSource, TreeSelectEvent, TreeSelectItemSnapshot,
    TreeSelectMode, TreeSelectQuery, TreeSelectResultState, TreeSelectSearch,
    TreeSelectSearchTicket, TreeSelectValue, TreeSelectValueDisplay, YueTreeSelect,
    YueTreeSelectState,
};
pub use avatar::{AvatarShape, AvatarSize, YueAvatar, YueAvatarGroup};
pub use badge::{BadgeShape, BadgeSize, BadgeTone, YueBadge};
pub use breadcrumb::{
    BreadcrumbCollapse, BreadcrumbEllipsis, BreadcrumbNavigate, BreadcrumbNavigateSource,
    BreadcrumbSeparator, YueBreadcrumb, YueBreadcrumbItem,
};
pub use button::{ButtonGroup, ButtonShape, ButtonSize, ButtonVariant, IconPosition, YueButton};
pub use card::{
    CardCoverPlacement, CardHoverEffect, CardSize, CardVariant, YueCard, YueCardCover, YueCardFooter,
    YueCardHeader, YueCardMeta,
};
pub use steps::{
    StepsChange, StepsLabelPlacement, StepsLayout, StepsSize, StepsStatus, StepsValue, StepsVariant,
    YueStepItem, YueSteps,
};
pub use pagination::{
    PaginationChange, PaginationChangeSource, PaginationError, PaginationMode, PaginationSize,
    PaginationSizeChangePolicy, PaginationValue, PaginationVariant, YuePagination, YuePaginationState,
};
pub use checkbox::{CheckboxSize, CheckboxState, YueCheckbox, YueCheckboxGroup, YueCheckboxOption};
pub use collapse::{
    CollapseChangeRequest, CollapseChangeSource, CollapseExpandTrigger, CollapseIndicatorPosition,
    CollapseMode, CollapseMotion, CollapseSize, CollapseVariant, YueCollapse, YueCollapseItem,
};
pub use color_picker::{
    ColorFormat, ColorPickerEvent, ColorPickerSize, ColorPickerTrigger, YueColorPicker,
    YueColorPickerPanel, YueColorPickerState,
};
pub use dialog::{
    DialogChange, DialogCloseReason, DialogDecision, DialogFooter, DialogOpenError, DialogOutcome,
    DialogSize, DialogStatus, DialogSubmitRequest, DialogTicket, YueDialog, YueDialogEvent,
    YueDialogState,
};
pub use divider::{DividerAlign, DividerLayout, DividerLineStyle, YueDivider};
pub use descriptions::{
    DescriptionsAlign, DescriptionsLayout, DescriptionsSize, DescriptionsSpan,
    DescriptionsTextOverflow, DescriptionsVerticalAlign, YueDescriptionItem, YueDescriptions,
};
pub use date_picker::{
    DateMonth, DateQuarter, DateWeek, DateYear, DatePickerChange, DatePickerChangeSource,
    DatePickerCloseReason, DatePickerCommitMode, DatePickerConstraints, DatePickerEndpoint,
    DatePickerError, DatePickerEvent, DatePickerGranularity, DatePickerInvalidInput,
    DatePickerOpenChange, DatePickerOptions, DatePickerPreset, DatePickerSelection,
    DatePickerUnit, DatePickerValue, HourCycle, TimePrecision, YueCalendar, YueCalendarState,
    YueDatePicker, YueDatePickerState,
};
pub use dropdown::{
    DropdownAlign, DropdownIndicatorSide, DropdownPlacement, YueContextMenu, YueDropdownMenu,
    YueDropdownSplitButton, YueMenuItem,
};
pub use empty::{EmptySize, YueEmpty};
pub use field::YueField;
pub use form::YueForm;
pub use gpui_kit;
pub use gpui_kit::base::input::{InputEvent, InputState, TextareaState};
pub use icon::{IconSize, YueIcon};
pub use image_preview::{
    ImagePreviewBackground, ImagePreviewChangeSource, ImagePreviewColorSpace, ImagePreviewDocument,
    ImagePreviewError, ImagePreviewEvent, ImagePreviewFit, ImagePreviewImage, ImagePreviewLimits,
    ImagePreviewLoad, ImagePreviewMode, ImagePreviewOverlay, ImagePreviewPixels,
    ImagePreviewSampling, ImagePreviewSnapshot, ImagePreviewToolbar, ImagePreviewView,
    YueImagePreview, YueImagePreviewState,
};
pub use input::{
    InputSize, InputStatus, InputTextAlign, InputType, YueInput, YueNumberInput, YueTextarea,
};
pub use kbd::{
    KbdSize, KbdState, KbdVariant, ShortcutParseError, ShortcutSeparator, ShortcutSpec, YueKbd,
    YueShortcut,
};
pub use layout::Breakpoint;
pub use link::{LinkHover, LinkSize, LinkVariant, YueLink};
pub use list::{
    ListGrid, ListGridGeometry, ListGridSizing, ListGridState, ListItemLayout, ListLayout,
    ListSize, YueList, YueListGroup, YueListItem, YueListItemMeta, YueVirtualListGrid,
};
pub use loading::{
    FeedbackSize, ProgressShape, ProgressStatus, ProgressValue, SkeletonAnimation, SkeletonShape,
    YueLoadingRegion, YueProgress, YueSkeleton, YueSpinner,
};
pub use motion::{MotionDuration, MotionEase};
pub use radio::{RadioAppearance, RadioSize, YueRadio, YueRadioGroup, YueRadioOption};
pub use resizable::{YueResizable, YueResizablePanel, YueResizableState, ResizableEvent};
pub use segmented::{
    SegmentedDistribution, SegmentedShape, SegmentedSize, SegmentedVariant, YueSegmented,
    YueSegmentedOption,
};
pub use select::{
    SelectApply, SelectChange, SelectChangeSource, SelectCloseReason, SelectEntry, SelectError,
    SelectEvent, SelectGroup, SelectMode, SelectOpenChange, SelectOption, SelectOptionContext,
    SelectPage, SelectPanelWidth, SelectRequest, SelectRequestTicket, SelectResultState,
    SelectRowLayout, SelectSearch, SelectSize, SelectTagLayout, SelectValue, SelectVariant,
    SelectVirtualization, YueSelect, YueSelectState,
};
pub use shortcut_recorder::{
    ShortcutRecorderEvent, ShortcutRecorderValidation, YueShortcutRecorder,
    YueShortcutRecorderState,
};
pub use slider::{
    SliderControls, SliderDomain, SliderLayout, SliderMark, SliderSize, SliderTooltip, SliderValue,
    YueSlider, YueSliderEvent, YueSliderState,
};
pub use swatch::{YueSwatchGroup, YueSwatchOption};
pub use switch::{SwitchIndicator, SwitchShape, SwitchSize, YueSwitch};
pub use tabs::{
    TabsActivation, TabsChangeRequest, TabsChangeSource, TabsCloseRequest, TabsContentMotion,
    TabsHandle, TabsPlacement, TabsScrollAlign, TabsSize, TabsVariant, YueTabItem, YueTabs,
};
pub use tag::{TagColors, TagShape, TagSize, TagTone, TagVariant, YueTag, YueTagGroup};
pub use theme::{Accent, ColorRole, NeutralTone, ThemeMode, ThemePreference, YueTheme, YueToken};
pub use toast::{
    FocusToasts, ToastAction, ToastAnnouncement, ToastAppearance, ToastDecision,
    ToastDismissReason, ToastError, ToastEvent, ToastId, ToastInsets, ToastKind, ToastPhase,
    ToastPlayback, ToastPosition, ToastSnapshot, ToastSpec, ToastTicket, ToasterOptions,
    YueToaster, YueToasterState,
};
pub use tooltip::{
    TooltipAlign, TooltipOptions, TooltipPlacement, TooltipTrigger, TooltipVariant,
    YueOverflowTooltip, YueTooltip, YueTooltipGroup,
};
pub use typography::{FontStack, TextRole, TypeStyle, load_fonts, tabular_nums};
/// Common component types and GPUI composition traits. Import less frequently
/// used component options explicitly from the crate root.
pub mod prelude {
    pub use crate::{YueDrawer, YueDrawerState, DrawerPlacement, DrawerSize};
    pub use crate::{YueDescriptionItem, YueDescriptions};
    pub use crate::{StepsValue, YueStepItem, YueSteps};
    pub use crate::{YueCard, YueCardCover, YueCardFooter, YueCardHeader, YueCardMeta};
    pub use crate::{YuePagination, YuePaginationState, PaginationValue};
    pub use crate::{YueResizable, YueResizablePanel, YueResizableState, ResizableEvent};
    pub use crate::{TimePickerOptions,TimePickerValue,TimePickerSelection,TimePickerCommitMode,TimePickerConstraints,TimePickerEvent,TimePickerSteps,YueTimePicker,YueTimePickerState,YueTimePanel};
    pub use crate::{DatePickerOptions, DatePickerValue, DatePickerGranularity,
        DatePickerSelection, DatePickerCommitMode, DatePickerConstraints, DatePickerEvent,
        YueCalendar, YueCalendarState, YueDatePicker, YueDatePickerState};
    pub use crate::{
        Accent, AvatarShape, AvatarSize, Icon, KbdSize, KbdState, KbdVariant, SelectMode,
        SelectOption, SelectValue, ShortcutSeparator, ShortcutSpec, SliderControls, SliderDomain,
        SliderLayout, SliderMark, SliderSize, SliderTooltip, SliderValue, ThemeMode,
        ThemePreference, YueAvatar, YueAvatarGroup, YueBreadcrumb, YueButton, YueCheckbox,
        YueColorPicker, YueColorPickerPanel, YueColorPickerState, YueDivider, YueDropdownMenu,
        YueField, YueForm, YueInput, YueKbd, YueLink, YueMenuItem, YueNumberInput,
        YueOverflowTooltip, YueRadio, YueRadioGroup, YueSegmented, YueSegmentedOption, YueSelect,
        YueSelectState, YueShortcut, YueSlider, YueSliderEvent, YueSliderState, YueSwatchGroup,
        YueSwatchOption, YueSwitch, YueTabItem, YueTabs, YueTextarea, YueTheme, YueTooltip,
        YueTooltipGroup, current_theme, outline,
    };
    pub use crate::{BadgeShape, BadgeSize, BadgeTone, YueBadge};
    pub use crate::{EmptySize, YueEmpty};
    pub use crate::{
        ShortcutRecorderEvent, ShortcutRecorderValidation, YueShortcutRecorder,
        YueShortcutRecorderState,
    };
    pub use crate::{
        ToastSpec, VirtualizerSizing, VirtualizerSource, VirtualizerState, YueCollapse,
        YueCollapseItem, YueDialog, YueDialogState, YueImagePreview, YueImagePreviewState, YueList,
        YueListGroup, YueListItem, YueListItemMeta, YueLoadingRegion, YueProgress, YueSkeleton,
        YueSpinner, YueTag, YueTagGroup, YueToaster, YueToasterState, YueVirtualList,
    };
    pub use gpui_kit::{
        InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
        prelude::FluentBuilder,
    };
}
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

pub use time_picker::{
    TimePickerChange, TimePickerChangeSource, TimePickerCloseReason, TimePickerCommitMode,
    TimePickerConstraints, TimePickerDraft, TimePickerEndpoint, TimePickerError, TimePickerEvent,
    TimePickerInvalidInput, TimePickerOpenChange, TimePickerOptions, TimePickerSelection,
    TimePickerSteps, TimePickerValue, YueTimePanel, YueTimePicker, YueTimePickerState,
};
