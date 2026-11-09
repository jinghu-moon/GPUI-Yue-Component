use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use gpui_kit::base::{CalendarGranularity, CalendarSelection};
pub use gpui_kit::base::{HourCycle, TimePrecision};
use gpui_kit::{ElementId, SharedString};
use std::{fmt, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerGranularity {
    #[default]
    Date,
    Week,
    Month,
    Quarter,
    Year,
}
impl From<DatePickerGranularity> for CalendarGranularity {
    fn from(value: DatePickerGranularity) -> Self {
        match value {
            DatePickerGranularity::Date => Self::Day,
            DatePickerGranularity::Week => Self::Week,
            DatePickerGranularity::Month => Self::Month,
            DatePickerGranularity::Quarter => Self::Quarter,
            DatePickerGranularity::Year => Self::Year,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerSelection {
    #[default]
    Single,
    Range,
    Multiple,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerCommitMode {
    #[default]
    Auto,
    Immediate,
    Confirm,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerEndpoint {
    #[default]
    Start,
    End,
}
impl DatePickerEndpoint {
    pub(super) fn index(self) -> usize {
        usize::from(self == Self::End)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateYear(i32);
impl DateYear {
    pub fn new(year: i32) -> Result<Self, DatePickerError> {
        if (1..=9999).contains(&year) {
            Ok(Self(year))
        } else {
            Err(DatePickerError::OutOfBounds)
        }
    }
    pub fn year(self) -> i32 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateMonth {
    year: DateYear,
    month: u32,
}
impl DateMonth {
    pub fn new(year: i32, month: u32) -> Result<Self, DatePickerError> {
        let year = DateYear::new(year)?;
        if !(1..=12).contains(&month) {
            return Err(DatePickerError::InvalidValue);
        }
        Ok(Self { year, month })
    }
    pub fn year(self) -> i32 {
        self.year.year()
    }
    pub fn month(self) -> u32 {
        self.month
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateQuarter {
    year: DateYear,
    quarter: u32,
}
impl DateQuarter {
    pub fn new(year: i32, quarter: u32) -> Result<Self, DatePickerError> {
        let year = DateYear::new(year)?;
        if !(1..=4).contains(&quarter) {
            return Err(DatePickerError::InvalidValue);
        }
        Ok(Self { year, quarter })
    }
    pub fn year(self) -> i32 {
        self.year.year()
    }
    pub fn quarter(self) -> u32 {
        self.quarter
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateWeek {
    year: DateYear,
    week: u32,
}
impl DateWeek {
    pub fn new(year: i32, week: u32) -> Result<Self, DatePickerError> {
        let year = DateYear::new(year)?;
        let start = NaiveDate::from_isoywd_opt(year.year(), week, Weekday::Mon)
            .ok_or(DatePickerError::InvalidValue)?;
        DateYear::new(start.year())?;
        Ok(Self { year, week })
    }
    /// ISO week-year, which can differ from the calendar year of the Monday.
    pub fn year(self) -> i32 {
        self.year.year()
    }
    pub fn week(self) -> u32 {
        self.week
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatePickerUnit {
    Date(NaiveDate),
    Week(DateWeek),
    Month(DateMonth),
    Quarter(DateQuarter),
    Year(DateYear),
}
impl DatePickerUnit {
    pub fn start_date(self) -> NaiveDate {
        match self {
            Self::Date(v) => v,
            Self::Week(v) => NaiveDate::from_isoywd_opt(v.year(), v.week(), Weekday::Mon)
                .expect("validated ISO week"),
            Self::Month(v) => {
                NaiveDate::from_ymd_opt(v.year(), v.month(), 1).expect("validated month")
            }
            Self::Quarter(v) => NaiveDate::from_ymd_opt(v.year(), (v.quarter() - 1) * 3 + 1, 1)
                .expect("validated quarter"),
            Self::Year(v) => NaiveDate::from_ymd_opt(v.year(), 1, 1).expect("validated year"),
        }
    }
    pub fn granularity(self) -> DatePickerGranularity {
        match self {
            Self::Date(_) => DatePickerGranularity::Date,
            Self::Week(_) => DatePickerGranularity::Week,
            Self::Month(_) => DatePickerGranularity::Month,
            Self::Quarter(_) => DatePickerGranularity::Quarter,
            Self::Year(_) => DatePickerGranularity::Year,
        }
    }
    pub(super) fn from_date(
        date: NaiveDate,
        granularity: DatePickerGranularity,
    ) -> Result<Self, DatePickerError> {
        DateYear::new(date.year())?;
        Ok(match granularity {
            DatePickerGranularity::Date => Self::Date(date),
            DatePickerGranularity::Week => Self::Week(DateWeek::new(
                date.iso_week().year(),
                date.iso_week().week(),
            )?),
            DatePickerGranularity::Month => Self::Month(DateMonth::new(date.year(), date.month())?),
            DatePickerGranularity::Quarter => {
                Self::Quarter(DateQuarter::new(date.year(), date.month0() / 3 + 1)?)
            }
            DatePickerGranularity::Year => Self::Year(DateYear::new(date.year())?),
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DatePickerValue {
    #[default]
    Empty,
    Single(DatePickerUnit),
    Range(DatePickerUnit, DatePickerUnit),
    DateTime(NaiveDateTime),
    DateTimeRange(NaiveDateTime, NaiveDateTime),
    Multiple(Vec<NaiveDate>),
}
impl DatePickerValue {
    pub fn date(date: NaiveDate) -> Self {
        Self::Single(DatePickerUnit::Date(date))
    }
    pub fn date_range(start: NaiveDate, end: NaiveDate) -> Self {
        Self::Range(DatePickerUnit::Date(start), DatePickerUnit::Date(end))
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty) || matches!(self, Self::Multiple(v) if v.is_empty())
    }
    pub fn dates(&self) -> Vec<NaiveDate> {
        match self {
            Self::Empty => vec![],
            Self::Single(v) => vec![v.start_date()],
            Self::Range(a, b) => vec![a.start_date(), b.start_date()],
            Self::DateTime(v) => vec![v.date()],
            Self::DateTimeRange(a, b) => vec![a.date(), b.date()],
            Self::Multiple(v) => v.clone(),
        }
    }
    pub(super) fn calendar(&self, selection: DatePickerSelection) -> CalendarSelection {
        let dates = self.dates();
        match selection {
            DatePickerSelection::Single => CalendarSelection::Single(dates.first().copied()),
            DatePickerSelection::Range => {
                CalendarSelection::Range(dates.first().copied(), dates.get(1).copied())
            }
            DatePickerSelection::Multiple => {
                CalendarSelection::Multiple(dates.into_iter().collect())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatePickerError {
    InvalidOptions,
    InvalidFormat,
    InvalidValue,
    Incomplete,
    OutOfBounds,
    Unavailable,
    RangeRejected,
    TooMany,
    LockedEndpoint,
}
impl fmt::Display for DatePickerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidOptions => "日期配置组合不受支持",
            Self::InvalidFormat => "日期格式不受支持",
            Self::InvalidValue => "请输入符合格式的有效日期",
            Self::Incomplete => "请填写完整的日期范围",
            Self::OutOfBounds => "日期超出允许范围",
            Self::Unavailable => "此日期或时间不可选",
            Self::RangeRejected => "此日期范围不符合限制",
            Self::TooMany => "已达到可选日期上限",
            Self::LockedEndpoint => "不能修改锁定的范围端点",
        })
    }
}
impl std::error::Error for DatePickerError {}
type DatePredicate = Rc<dyn Fn(NaiveDate) -> bool>;
type TimePredicate = Rc<dyn Fn(NaiveDateTime) -> bool>;
type RangeValidator = Rc<dyn Fn(NaiveDate, NaiveDate) -> bool>;
#[derive(Clone, Default)]
pub struct DatePickerConstraints {
    pub(super) min: Option<NaiveDate>,
    pub(super) max: Option<NaiveDate>,
    pub(super) allowed: Option<DatePredicate>,
    pub(super) time_allowed: Option<TimePredicate>,
    pub(super) range_allowed: Option<RangeValidator>,
    pub(super) max_selected: Option<usize>,
    pub(super) max_span_days: Option<u64>,
}
impl DatePickerConstraints {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn min(mut self, value: NaiveDate) -> Self {
        self.min = Some(value);
        self
    }
    pub fn max(mut self, value: NaiveDate) -> Self {
        self.max = Some(value);
        self
    }
    /// True means selectable. Called on the normalized start of a selected unit.
    pub fn allowed(mut self, predicate: impl Fn(NaiveDate) -> bool + 'static) -> Self {
        self.allowed = Some(Rc::new(predicate));
        self
    }
    pub fn time_allowed(mut self, predicate: impl Fn(NaiveDateTime) -> bool + 'static) -> Self {
        self.time_allowed = Some(Rc::new(predicate));
        self
    }
    /// Endpoints are inclusive. The host chooses whether to inspect the interior.
    pub fn range_allowed(
        mut self,
        predicate: impl Fn(NaiveDate, NaiveDate) -> bool + 'static,
    ) -> Self {
        self.range_allowed = Some(Rc::new(predicate));
        self
    }
    pub fn max_selected(mut self, count: usize) -> Self {
        self.max_selected = Some(count);
        self
    }
    /// Difference between endpoints in natural days, not inclusive item count.
    pub fn max_span_days(mut self, days: u64) -> Self {
        self.max_span_days = Some(days);
        self
    }
    pub(super) fn check(&self) -> Result<(), DatePickerError> {
        if matches!((self.min, self.max), (Some(a),Some(b)) if a>b) || self.max_selected == Some(0)
        {
            return Err(DatePickerError::InvalidOptions);
        }
        for date in [self.min, self.max].into_iter().flatten() {
            DateYear::new(date.year())?;
        }
        Ok(())
    }
    pub(super) fn check_date(&self, date: NaiveDate) -> Result<(), DatePickerError> {
        DateYear::new(date.year())?;
        if self.min.is_some_and(|v| date < v) || self.max.is_some_and(|v| date > v) {
            return Err(DatePickerError::OutOfBounds);
        }
        if self.allowed.as_ref().is_some_and(|v| !v(date)) {
            return Err(DatePickerError::Unavailable);
        }
        Ok(())
    }
    pub(super) fn check_calendar(&self, value: &CalendarSelection) -> Result<(), DatePickerError> {
        for date in value.dates() {
            self.check_date(date)?;
        }
        if let CalendarSelection::Range(Some(a), Some(b)) = value
            && (self
                .max_span_days
                .is_some_and(|max| (*b - *a).num_days().unsigned_abs() > max)
                || self.range_allowed.as_ref().is_some_and(|v| !v(*a, *b)))
        {
            return Err(DatePickerError::RangeRejected);
        }
        if let CalendarSelection::Multiple(dates) = value
            && self.max_selected.is_some_and(|max| dates.len() > max)
        {
            return Err(DatePickerError::TooMany);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DatePickerOptions {
    pub(super) id: ElementId,
    pub(super) granularity: DatePickerGranularity,
    pub(super) selection: DatePickerSelection,
    pub(super) commit: DatePickerCommitMode,
    pub(super) precision: Option<TimePrecision>,
    pub(super) hour_cycle: HourCycle,
    pub(super) week_start: Weekday,
    pub(super) format: Option<SharedString>,
    pub(super) constraints: DatePickerConstraints,
    pub(super) locked: [bool; 2],
    pub(super) clock: Rc<dyn Fn() -> NaiveDateTime>,
}
impl DatePickerOptions {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            granularity: Default::default(),
            selection: Default::default(),
            commit: Default::default(),
            precision: None,
            hour_cycle: HourCycle::H23,
            week_start: Weekday::Mon,
            format: None,
            constraints: Default::default(),
            locked: [false; 2],
            clock: Rc::new(|| Local::now().naive_local()),
        }
    }
    pub fn granularity(mut self, value: DatePickerGranularity) -> Self {
        self.granularity = value;
        self
    }
    pub fn selection(mut self, value: DatePickerSelection) -> Self {
        self.selection = value;
        self
    }
    pub fn commit_mode(mut self, value: DatePickerCommitMode) -> Self {
        self.commit = value;
        self
    }
    pub fn time_precision(mut self, value: TimePrecision) -> Self {
        self.precision = Some(value);
        self
    }
    pub fn hour_cycle(mut self, value: HourCycle) -> Self {
        self.hour_cycle = value;
        self
    }
    pub fn week_start(mut self, value: Weekday) -> Self {
        self.week_start = value;
        self
    }
    pub fn format(mut self, value: impl Into<SharedString>) -> Self {
        self.format = Some(value.into());
        self
    }
    pub fn constraints(mut self, value: DatePickerConstraints) -> Self {
        self.constraints = value;
        self
    }
    pub fn locked_endpoint(mut self, endpoint: DatePickerEndpoint, locked: bool) -> Self {
        self.locked[endpoint.index()] = locked;
        self
    }
    pub fn clock(mut self, clock: impl Fn() -> NaiveDateTime + 'static) -> Self {
        self.clock = Rc::new(clock);
        self
    }
    pub(super) fn requires_confirmation(&self) -> bool {
        match self.commit {
            DatePickerCommitMode::Confirm => true,
            DatePickerCommitMode::Immediate => false,
            DatePickerCommitMode::Auto => {
                self.precision.is_some() || self.selection == DatePickerSelection::Multiple
            }
        }
    }
    pub(super) fn format_str(&self) -> &str {
        self.format
            .as_deref()
            .unwrap_or(match (self.granularity, self.precision) {
                (_, Some(TimePrecision::Minute)) => "%Y-%m-%d %H:%M",
                (_, Some(TimePrecision::Second)) => "%Y-%m-%d %H:%M:%S",
                (DatePickerGranularity::Date, None) => "%Y-%m-%d",
                (DatePickerGranularity::Month, None) => "%Y-%m",
                (DatePickerGranularity::Year, None) => "%Y",
                (DatePickerGranularity::Week, None) => "%G-W%V",
                (DatePickerGranularity::Quarter, None) => "%Y-Q%q",
            })
    }
    pub(super) fn check(&self) -> Result<(), DatePickerError> {
        if (self.granularity != DatePickerGranularity::Date
            && (self.precision.is_some() || self.selection == DatePickerSelection::Multiple))
            || (self.selection == DatePickerSelection::Multiple && self.precision.is_some())
            || (self.selection != DatePickerSelection::Range && self.locked.iter().any(|v| *v))
        {
            return Err(DatePickerError::InvalidOptions);
        }
        self.constraints.check()?;
        // A deliberately bounded reversible format surface. Chrono directives, not JS tokens.
        let formats: &[&str] = match (self.granularity, self.precision) {
            (_, Some(TimePrecision::Minute)) => &["%Y-%m-%d %H:%M", "%Y/%m/%d %H:%M"],
            (_, Some(TimePrecision::Second)) => &["%Y-%m-%d %H:%M:%S", "%Y/%m/%d %H:%M:%S"],
            (DatePickerGranularity::Date, None) => &["%Y-%m-%d", "%Y/%m/%d", "%d/%m/%Y"],
            (DatePickerGranularity::Month, None) => &["%Y-%m", "%Y/%m"],
            (DatePickerGranularity::Year, None) => &["%Y"],
            (DatePickerGranularity::Week, None) => &["%G-W%V"],
            (DatePickerGranularity::Quarter, None) => &["%Y-Q%q"],
        };
        if !formats.contains(&self.format_str()) {
            return Err(DatePickerError::InvalidFormat);
        }
        Ok(())
    }
    pub(super) fn normalize(
        &self,
        value: DatePickerValue,
    ) -> Result<DatePickerValue, DatePickerError> {
        let value = match (self.selection, self.precision, value) {
            (_, _, DatePickerValue::Empty) => DatePickerValue::Empty,
            (DatePickerSelection::Single, None, DatePickerValue::Single(unit))
                if unit.granularity() == self.granularity =>
            {
                DatePickerValue::Single(unit)
            }
            (DatePickerSelection::Range, None, DatePickerValue::Range(a, b))
                if a.granularity() == self.granularity && b.granularity() == self.granularity =>
            {
                if a.start_date() <= b.start_date() {
                    DatePickerValue::Range(a, b)
                } else {
                    DatePickerValue::Range(b, a)
                }
            }
            (DatePickerSelection::Single, Some(precision), DatePickerValue::DateTime(v)) => {
                DatePickerValue::DateTime(v.date().and_time(precision.truncate(v.time())))
            }
            (DatePickerSelection::Range, Some(precision), DatePickerValue::DateTimeRange(a, b)) => {
                let a = a.date().and_time(precision.truncate(a.time()));
                let b = b.date().and_time(precision.truncate(b.time()));
                DatePickerValue::DateTimeRange(a.min(b), a.max(b))
            }
            (DatePickerSelection::Multiple, None, DatePickerValue::Multiple(mut v)) => {
                v.sort_unstable();
                v.dedup();
                if v.is_empty() {
                    DatePickerValue::Empty
                } else {
                    DatePickerValue::Multiple(v)
                }
            }
            _ => return Err(DatePickerError::InvalidValue),
        };
        self.constraints
            .check_calendar(&value.calendar(self.selection))?;
        let times = match &value {
            DatePickerValue::DateTime(v) => vec![*v],
            DatePickerValue::DateTimeRange(a, b) => vec![*a, *b],
            _ => vec![],
        };
        if times.iter().any(|v| {
            self.constraints
                .time_allowed
                .as_ref()
                .is_some_and(|p| !p(*v))
        }) {
            return Err(DatePickerError::Unavailable);
        }
        Ok(value)
    }
    pub(super) fn candidate(
        &self,
        calendar: &CalendarSelection,
        times: [NaiveTime; 2],
    ) -> Result<DatePickerValue, DatePickerError> {
        let unit = |date| DatePickerUnit::from_date(date, self.granularity);
        self.normalize(match calendar {
            CalendarSelection::Single(Some(v)) => {
                if self.precision.is_some() {
                    DatePickerValue::DateTime(v.and_time(times[0]))
                } else {
                    DatePickerValue::Single(unit(*v)?)
                }
            }
            CalendarSelection::Range(Some(a), Some(b)) => {
                if self.precision.is_some() {
                    DatePickerValue::DateTimeRange(a.and_time(times[0]), b.and_time(times[1]))
                } else {
                    DatePickerValue::Range(unit(*a)?, unit(*b)?)
                }
            }
            CalendarSelection::Multiple(v) => {
                DatePickerValue::Multiple(v.iter().copied().collect())
            }
            CalendarSelection::Single(None) | CalendarSelection::Range(None, None) => {
                DatePickerValue::Empty
            }
            _ => return Err(DatePickerError::Incomplete),
        })
    }
    pub(super) fn display_unit(&self, unit: DatePickerUnit) -> String {
        match unit {
            DatePickerUnit::Quarter(v) => format!("{:04}-Q{}", v.year(), v.quarter()),
            DatePickerUnit::Week(v) => format!("{:04}-W{:02}", v.year(), v.week()),
            _ => unit.start_date().format(self.format_str()).to_string(),
        }
    }
    pub(super) fn texts(&self, value: &DatePickerValue) -> [String; 2] {
        match value {
            DatePickerValue::Empty => Default::default(),
            DatePickerValue::Single(v) => [self.display_unit(*v), String::new()],
            DatePickerValue::Range(a, b) => [self.display_unit(*a), self.display_unit(*b)],
            DatePickerValue::DateTime(v) => {
                [v.format(self.format_str()).to_string(), String::new()]
            }
            DatePickerValue::DateTimeRange(a, b) => [
                a.format(self.format_str()).to_string(),
                b.format(self.format_str()).to_string(),
            ],
            DatePickerValue::Multiple(_) => Default::default(),
        }
    }
    pub(super) fn parse_unit(&self, text: &str) -> Result<DatePickerUnit, DatePickerError> {
        let invalid = DatePickerError::InvalidValue;
        let unit = match self.granularity {
            DatePickerGranularity::Week => {
                let (year, week) = text.split_once("-W").ok_or(invalid.clone())?;
                DatePickerUnit::Week(DateWeek::new(
                    year.parse().map_err(|_| invalid.clone())?,
                    week.parse().map_err(|_| invalid.clone())?,
                )?)
            }
            DatePickerGranularity::Quarter => {
                let (year, quarter) = text.split_once("-Q").ok_or(invalid.clone())?;
                DatePickerUnit::Quarter(DateQuarter::new(
                    year.parse().map_err(|_| invalid.clone())?,
                    quarter.parse().map_err(|_| invalid.clone())?,
                )?)
            }
            DatePickerGranularity::Year => {
                DatePickerUnit::Year(DateYear::new(text.parse().map_err(|_| invalid.clone())?)?)
            }
            DatePickerGranularity::Month => {
                let date = NaiveDate::parse_from_str(
                    &format!("{text}-01"),
                    &format!("{}-%d", self.format_str()),
                )
                .map_err(|_| invalid.clone())?;
                DatePickerUnit::from_date(date, self.granularity)?
            }
            DatePickerGranularity::Date => DatePickerUnit::Date(
                NaiveDate::parse_from_str(text, self.format_str()).map_err(|_| invalid.clone())?,
            ),
        };
        // Full consumption, exact supported format, and no permissive auto-carry.
        if self.display_unit(unit) != text {
            return Err(invalid);
        }
        Ok(unit)
    }
    pub(super) fn parse(&self, texts: [&str; 2]) -> Result<DatePickerValue, DatePickerError> {
        if texts.iter().all(|v| v.is_empty()) {
            return Ok(DatePickerValue::Empty);
        }
        let range = self.selection == DatePickerSelection::Range;
        if texts[0].is_empty() || (range && texts[1].is_empty()) {
            return Err(DatePickerError::Incomplete);
        }
        let value = if self.precision.is_some() {
            let parse = |text: &str| {
                let time = NaiveDateTime::parse_from_str(text, self.format_str())
                    .map_err(|_| DatePickerError::InvalidValue)?;
                if time.format(self.format_str()).to_string() != text {
                    return Err(DatePickerError::InvalidValue);
                }
                Ok(time)
            };
            let a = parse(texts[0])?;
            if range {
                DatePickerValue::DateTimeRange(a, parse(texts[1])?)
            } else {
                DatePickerValue::DateTime(a)
            }
        } else {
            let a = self.parse_unit(texts[0])?;
            if range {
                DatePickerValue::Range(a, self.parse_unit(texts[1])?)
            } else {
                DatePickerValue::Single(a)
            }
        };
        self.normalize(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerChangeSource {
    Calendar,
    Input,
    Today,
    Preset,
    Time,
    Clear,
    Tag,
    Confirm,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerCloseReason {
    Escape,
    OutsidePress,
    Confirm,
    Selection,
    Lifecycle,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatePickerChange {
    value: DatePickerValue,
    source: DatePickerChangeSource,
}
impl DatePickerChange {
    pub(super) fn new(value: DatePickerValue, source: DatePickerChangeSource) -> Self {
        Self { value, source }
    }
    pub fn value(&self) -> &DatePickerValue {
        &self.value
    }
    pub fn source(&self) -> DatePickerChangeSource {
        self.source
    }
}
#[derive(Clone, Debug)]
pub struct DatePickerOpenChange {
    open: bool,
    reason: Option<DatePickerCloseReason>,
}
impl DatePickerOpenChange {
    pub(super) fn new(open: bool, reason: Option<DatePickerCloseReason>) -> Self {
        Self { open, reason }
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn reason(&self) -> Option<DatePickerCloseReason> {
        self.reason
    }
}
#[derive(Clone, Debug)]
pub struct DatePickerInvalidInput {
    error: DatePickerError,
    endpoint: DatePickerEndpoint,
}
impl DatePickerInvalidInput {
    pub(super) fn new(error: DatePickerError, endpoint: DatePickerEndpoint) -> Self {
        Self { error, endpoint }
    }
    pub fn error(&self) -> &DatePickerError {
        &self.error
    }
    pub fn endpoint(&self) -> DatePickerEndpoint {
        self.endpoint
    }
}
#[derive(Clone, Debug)]
pub enum DatePickerEvent {
    Change(DatePickerChange),
    DraftChange(Option<DatePickerValue>),
    OpenChange(DatePickerOpenChange),
    PanelChange(gpui_kit::base::CalendarView, i32, u32),
    InvalidInput(DatePickerInvalidInput),
}
#[derive(Clone)]
pub struct DatePickerPreset {
    pub(super) id: ElementId,
    pub(super) label: SharedString,
    pub(super) resolve: Rc<dyn Fn(NaiveDateTime) -> DatePickerValue>,
}
impl DatePickerPreset {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        resolve: impl Fn(NaiveDateTime) -> DatePickerValue + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            resolve: Rc::new(resolve),
        }
    }
}
