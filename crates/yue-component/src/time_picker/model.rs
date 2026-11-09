use chrono::{Local, NaiveTime, Timelike as _};
pub use gpui_kit::base::TimeSteps as TimePickerSteps;
pub use gpui_kit::base::{HourCycle, TimePrecision};
use gpui_kit::base::{TimeRules, TimeSteps};
use gpui_kit::{ElementId, SharedString};
use std::{fmt, rc::Rc};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimePickerSelection {
    #[default]
    Single,
    Range,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimePickerCommitMode {
    #[default]
    Confirm,
    Immediate,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimePickerEndpoint {
    #[default]
    Start,
    End,
}
impl TimePickerEndpoint {
    pub(crate) fn index(self) -> usize {
        if self == Self::Start { 0 } else { 1 }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TimePickerValue {
    #[default]
    Empty,
    Single(NaiveTime),
    Range(NaiveTime, NaiveTime),
}
impl TimePickerValue {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
    pub(crate) fn parts(&self) -> [Option<NaiveTime>; 2] {
        match self {
            Self::Empty => [None, None],
            Self::Single(t) => [Some(*t), None],
            Self::Range(a, b) => [Some(*a), Some(*b)],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimePickerError {
    InvalidOptions,
    InvalidFormat,
    InvalidInput,
    Unavailable,
    Incomplete,
    ReversedRange,
    NotAllowed,
    NoAvailableTime,
}
impl fmt::Display for TimePickerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidOptions => "时间配置无效",
            Self::InvalidFormat => "时间格式与精度或时制不匹配",
            Self::InvalidInput => "请输入完整且格式正确的时间",
            Self::Unavailable => "当前时间控件不可编辑",
            Self::Incomplete => "请完整选择时间",
            Self::ReversedRange => "结束时间不能早于开始时间",
            Self::NotAllowed => "时间不符合可选刻度或限制",
            Self::NoAvailableTime => "没有可选时间",
        })
    }
}
impl std::error::Error for TimePickerError {}
type RangePredicate = Rc<dyn Fn(NaiveTime, NaiveTime) -> bool>;
#[derive(Clone, Default)]
pub struct TimePickerConstraints {
    pub(crate) rules: TimeRules,
    pub(crate) range: Option<RangePredicate>,
}
impl TimePickerConstraints {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn min(mut self, t: NaiveTime) -> Self {
        self.rules = self.rules.min(t);
        self
    }
    pub fn max(mut self, t: NaiveTime) -> Self {
        self.rules = self.rules.max(t);
        self
    }
    pub fn disabled_hours(mut self, f: impl Fn(u32) -> bool + 'static) -> Self {
        self.rules = self.rules.disabled_hours(f);
        self
    }
    pub fn disabled_minutes(mut self, f: impl Fn(u32, u32) -> bool + 'static) -> Self {
        self.rules = self.rules.disabled_minutes(f);
        self
    }
    pub fn disabled_seconds(mut self, f: impl Fn(u32, u32, u32) -> bool + 'static) -> Self {
        self.rules = self.rules.disabled_seconds(f);
        self
    }
    pub fn range_allowed(mut self, f: impl Fn(NaiveTime, NaiveTime) -> bool + 'static) -> Self {
        self.range = Some(Rc::new(f));
        self
    }
}
#[derive(Clone)]
pub struct TimePickerOptions {
    pub(crate) id: ElementId,
    pub(crate) selection: TimePickerSelection,
    pub(crate) precision: TimePrecision,
    pub(crate) cycle: HourCycle,
    pub(crate) commit: TimePickerCommitMode,
    pub(crate) steps: TimeSteps,
    pub(crate) constraints: TimePickerConstraints,
    format: Option<SharedString>,
    pub(crate) clock: Rc<dyn Fn() -> NaiveTime>,
}
impl TimePickerOptions {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selection: Default::default(),
            precision: TimePrecision::Second,
            cycle: HourCycle::H23,
            commit: Default::default(),
            steps: Default::default(),
            constraints: Default::default(),
            format: None,
            clock: Rc::new(|| Local::now().time()),
        }
    }
    pub fn selection(mut self, v: TimePickerSelection) -> Self {
        self.selection = v;
        self
    }
    pub fn precision(mut self, v: TimePrecision) -> Self {
        self.precision = v;
        self
    }
    pub fn hour_cycle(mut self, v: HourCycle) -> Self {
        self.cycle = v;
        self
    }
    pub fn commit_mode(mut self, v: TimePickerCommitMode) -> Self {
        self.commit = v;
        self
    }
    pub fn steps(mut self, v: TimePickerSteps) -> Self {
        self.steps = v;
        self
    }
    pub fn constraints(mut self, v: TimePickerConstraints) -> Self {
        self.constraints = v;
        self
    }
    pub fn format(mut self, v: impl Into<SharedString>) -> Self {
        self.format = Some(v.into());
        self
    }
    pub fn clock(mut self, f: impl Fn() -> NaiveTime + 'static) -> Self {
        self.clock = Rc::new(f);
        self
    }
    pub(crate) fn format_str(&self) -> &str {
        self.format
            .as_deref()
            .unwrap_or(self.precision.format(self.cycle))
    }
    pub(crate) fn rules(&self) -> Result<TimeRules, TimePickerError> {
        self.constraints
            .rules
            .clone()
            .configured(self.precision, self.steps)
            .ok_or(TimePickerError::InvalidOptions)
    }
    pub(crate) fn check(&self) -> Result<(), TimePickerError> {
        self.rules()?;
        if self.format_str() != self.precision.format(self.cycle) {
            return Err(TimePickerError::InvalidFormat);
        }
        Ok(())
    }
    pub(crate) fn normalize_time(&self, t: NaiveTime) -> Result<NaiveTime, TimePickerError> {
        if t.nanosecond() >= 1_000_000_000 {
            return Err(TimePickerError::NotAllowed);
        }
        let t = self.precision.truncate(t);
        if self.rules()?.allows(t) {
            Ok(t)
        } else {
            Err(TimePickerError::NotAllowed)
        }
    }
    pub(crate) fn normalize(&self, v: TimePickerValue) -> Result<TimePickerValue, TimePickerError> {
        Ok(match v {
            TimePickerValue::Empty => TimePickerValue::Empty,
            TimePickerValue::Single(t) if self.selection == TimePickerSelection::Single => {
                TimePickerValue::Single(self.normalize_time(t)?)
            }
            TimePickerValue::Range(a, b) if self.selection == TimePickerSelection::Range => {
                let (a, b) = (self.normalize_time(a)?, self.normalize_time(b)?);
                if a > b {
                    return Err(TimePickerError::ReversedRange);
                }
                if self.constraints.range.as_ref().is_some_and(|f| !f(a, b)) {
                    return Err(TimePickerError::NotAllowed);
                }
                TimePickerValue::Range(a, b)
            }
            _ => return Err(TimePickerError::InvalidOptions),
        })
    }
    pub(crate) fn text(&self, t: NaiveTime) -> String {
        t.format(self.format_str()).to_string()
    }
    pub(crate) fn parse_time(&self, text: &str) -> Result<NaiveTime, TimePickerError> {
        let text = text.trim().to_ascii_uppercase();
        let t = NaiveTime::parse_from_str(&text, self.format_str())
            .map_err(|_| TimePickerError::InvalidInput)?;
        if self.text(t) != text {
            return Err(TimePickerError::InvalidInput);
        }
        self.normalize_time(t)
    }
    pub(crate) fn value_from_parts(
        &self,
        p: [Option<NaiveTime>; 2],
    ) -> Result<TimePickerValue, TimePickerError> {
        self.normalize(match (self.selection, p) {
            (_, [None, None]) => TimePickerValue::Empty,
            (TimePickerSelection::Single, [Some(t), _]) => TimePickerValue::Single(t),
            (TimePickerSelection::Range, [Some(a), Some(b)]) => TimePickerValue::Range(a, b),
            _ => return Err(TimePickerError::Incomplete),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimePickerChangeSource {
    Input,
    Panel,
    Now,
    Clear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimePickerCloseReason {
    Confirm,
    Escape,
    OutsidePress,
    Tab,
    Lifecycle,
}
#[derive(Clone, Debug)]
pub struct TimePickerChange {
    value: TimePickerValue,
    source: TimePickerChangeSource,
}
impl TimePickerChange {
    pub(crate) fn new(value: TimePickerValue, source: TimePickerChangeSource) -> Self {
        Self { value, source }
    }
    pub fn value(&self) -> &TimePickerValue {
        &self.value
    }
    pub fn source(&self) -> TimePickerChangeSource {
        self.source
    }
}
#[derive(Clone, Debug)]
pub struct TimePickerDraft {
    parts: [Option<NaiveTime>; 2],
}
impl TimePickerDraft {
    pub(crate) fn new(parts: [Option<NaiveTime>; 2]) -> Self {
        Self { parts }
    }
    pub fn start(&self) -> Option<NaiveTime> {
        self.parts[0]
    }
    pub fn end(&self) -> Option<NaiveTime> {
        self.parts[1]
    }
}
#[derive(Clone, Debug)]
pub struct TimePickerOpenChange {
    open: bool,
    reason: Option<TimePickerCloseReason>,
}
impl TimePickerOpenChange {
    pub(crate) fn new(open: bool, reason: Option<TimePickerCloseReason>) -> Self {
        Self { open, reason }
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn reason(&self) -> Option<TimePickerCloseReason> {
        self.reason
    }
}
#[derive(Clone, Debug)]
pub struct TimePickerInvalidInput {
    error: TimePickerError,
    endpoint: TimePickerEndpoint,
}
impl TimePickerInvalidInput {
    pub(crate) fn new(error: TimePickerError, endpoint: TimePickerEndpoint) -> Self {
        Self { error, endpoint }
    }
    pub fn error(&self) -> &TimePickerError {
        &self.error
    }
    pub fn endpoint(&self) -> TimePickerEndpoint {
        self.endpoint
    }
}
#[derive(Clone, Debug)]
pub enum TimePickerEvent {
    Change(TimePickerChange),
    DraftChange(TimePickerDraft),
    OpenChange(TimePickerOpenChange),
    InvalidInput(TimePickerInvalidInput),
}
