//! Opt-in calendar interaction contracts. No theme, input editor or popup ownership.
use super::*;
use chrono::{Days, Months};
use gpui::{KeyDownEvent, Role, prelude::FluentBuilder as _};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarGranularity {
    #[default]
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// The calendar's sole candidate selection. Legacy Date remains source compatible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarSelection {
    Single(Option<NaiveDate>),
    Range(Option<NaiveDate>, Option<NaiveDate>),
    Multiple(BTreeSet<NaiveDate>),
}
impl From<Date> for CalendarSelection {
    fn from(value: Date) -> Self {
        match value {
            Date::Single(v) => Self::Single(v),
            Date::Range(a, b) => Self::Range(a, b),
        }
    }
}
impl CalendarSelection {
    pub fn start(&self) -> Option<NaiveDate> {
        match self {
            Self::Single(v) | Self::Range(v, _) => *v,
            Self::Multiple(v) => v.first().copied(),
        }
    }
    pub fn is_complete(&self) -> bool {
        matches!(
            self,
            Self::Single(Some(_)) | Self::Range(Some(_), Some(_)) | Self::Multiple(_)
        )
    }
    pub fn contains(&self, date: NaiveDate) -> bool {
        match self {
            Self::Single(v) => *v == Some(date),
            Self::Range(a, b) => *a == Some(date) || *b == Some(date),
            Self::Multiple(v) => v.contains(&date),
        }
    }
    pub fn in_range(&self, date: NaiveDate) -> bool {
        matches!(self, Self::Range(Some(a), Some(b)) if date >= *a && date <= *b)
    }
    pub fn dates(&self) -> Vec<NaiveDate> {
        match self {
            Self::Single(v) => v.iter().copied().collect(),
            Self::Range(a, b) => a.iter().chain(b.iter()).copied().collect(),
            Self::Multiple(v) => v.iter().copied().collect(),
        }
    }
}

/// Includes incomplete ranges; legacy Selected still reports complete legacy values.
#[derive(Clone, Debug)]
pub enum CalendarInteractionEvent {
    Changed(CalendarSelection),
    PanelChanged(CalendarView, i32, u32),
}
type SelectionValidator = Rc<dyn Fn(&CalendarSelection) -> bool>;
#[derive(Default)]
pub(super) struct CalendarInteraction {
    pub(super) enabled: bool,
    granularity: CalendarGranularity,
    active: Option<NaiveDate>,
    preview: Option<NaiveDate>,
    validator: Option<SelectionValidator>,
    date_allowed: Option<Rc<dyn Fn(NaiveDate) -> bool>>,
    locks: [Option<NaiveDate>; 2],
}

fn supported(date: NaiveDate) -> bool {
    (1..=9999).contains(&date.year())
}

pub fn normalize_unit(date: NaiveDate, unit: CalendarGranularity) -> Option<NaiveDate> {
    let date = match unit {
        CalendarGranularity::Day => date,
        CalendarGranularity::Week => {
            date.checked_sub_days(Days::new(date.weekday().num_days_from_monday() as u64))?
        }
        CalendarGranularity::Month => date.with_day(1)?,
        CalendarGranularity::Quarter => {
            NaiveDate::from_ymd_opt(date.year(), (date.month0() / 3) * 3 + 1, 1)?
        }
        CalendarGranularity::Year => NaiveDate::from_ymd_opt(date.year(), 1, 1)?,
    };
    supported(date).then_some(date)
}

impl CalendarState {
    /// Enables fixed six-week grids, sorted reverse ranges, roving grid focus,
    /// ISO weeks, complete navigation and twelve-cell decade pages.
    /// Existing Calendar clients keep the legacy defaults unless they opt in.
    pub fn enhanced(mut self, granularity: CalendarGranularity) -> Self {
        self.interaction.enabled = true;
        self.interaction.granularity = granularity;
        self.interaction.active = Some(self.today);
        self.view = self.initial_view();
        self
    }
    pub fn granularity(&self) -> CalendarGranularity {
        self.interaction.granularity
    }
    pub fn selection(&self) -> &CalendarSelection {
        &self.selection
    }
    pub fn active_date(&self) -> NaiveDate {
        self.interaction.active.unwrap_or(self.today)
    }
    pub fn initial_view(&self) -> CalendarView {
        match self.granularity() {
            CalendarGranularity::Day | CalendarGranularity::Week => CalendarView::Day,
            CalendarGranularity::Month | CalendarGranularity::Quarter => CalendarView::Month,
            CalendarGranularity::Year => CalendarView::Year,
        }
    }
    pub fn set_selection_validator(
        &mut self,
        validator: impl Fn(&CalendarSelection) -> bool + 'static,
    ) {
        self.interaction.validator = Some(Rc::new(validator));
    }
    pub fn selection_allowed(&self, selection: &CalendarSelection) -> bool {
        selection
            .dates()
            .iter()
            .all(|date| self.date_allowed(*date))
            && !self
                .interaction
                .validator
                .as_ref()
                .is_some_and(|v| !v(selection))
    }
    pub fn set_date_validator(&mut self, predicate: impl Fn(NaiveDate) -> bool + 'static) {
        self.interaction.date_allowed = Some(Rc::new(predicate));
    }
    pub fn date_allowed(&self, date: NaiveDate) -> bool {
        supported(date)
            && !self
                .disabled_matcher_ref()
                .is_some_and(|m| m.matched(&date))
            && self
                .interaction
                .date_allowed
                .as_ref()
                .is_none_or(|predicate| predicate(date))
    }
    pub fn set_range_locks(&mut self, locks: [Option<NaiveDate>; 2]) {
        self.interaction.locks = locks;
    }
    /// Restore an already accepted snapshot when cancelling. Rules may have
    /// changed since its acceptance; restoration must not discard historical data.
    /// New user candidates must use apply_selection/activate_date instead.
    pub fn restore_selection(&mut self, selection: CalendarSelection) {
        if let Some(date) = selection.start() {
            self.browse_to(date);
        }
        self.selection = selection;
        self.interaction.preview = None;
    }
    /// Silent external synchronization; validates before replacing any candidate.
    pub fn apply_selection(&mut self, selection: CalendarSelection) -> bool {
        if !self.selection_allowed(&selection) {
            return false;
        }
        if let Some(date) = selection.start() {
            self.browse_to(date);
        }
        self.selection = selection;
        self.interaction.preview = None;
        true
    }
    pub fn browse_to(&mut self, date: NaiveDate) {
        if !supported(date) {
            return;
        }
        self.current_year = date.year();
        self.current_month = date.month() as u8;
        self.interaction.active = Some(date);
    }
    /// A clock can refresh today when reopening without creating a midnight timer.
    pub fn set_today(&mut self, today: NaiveDate) {
        if supported(today) {
            self.today = today;
        }
    }
    pub fn fixed_month_days(year: i32, month: u32, first_day: Weekday) -> Vec<Vec<NaiveDate>> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Vec::new();
        }
        let mut weeks = days_in_month(year, month, first_day);
        while weeks.len() < 6 {
            let start = *weeks.last().unwrap().last().unwrap();
            weeks.push(
                (1..=7)
                    .filter_map(|n| start.checked_add_days(Days::new(n)))
                    .collect(),
            );
        }
        weeks
    }
    fn unit(&self, date: NaiveDate) -> Option<NaiveDate> {
        normalize_unit(date, self.granularity())
    }
    pub(super) fn select_enhanced(&mut self, date: NaiveDate) -> bool {
        let Some(value) = self.unit(date) else {
            return false;
        };
        let selection = match &self.selection {
            CalendarSelection::Single(_) => CalendarSelection::Single(Some(value)),
            CalendarSelection::Range(_, _) if self.interaction.locks[0].is_some() => {
                let start = self.interaction.locks[0].unwrap();
                if value < start || self.interaction.locks[1].is_some_and(|end| value != end) {
                    return false;
                }
                CalendarSelection::Range(Some(start), Some(value))
            }
            CalendarSelection::Range(_, _) if self.interaction.locks[1].is_some() => {
                let end = self.interaction.locks[1].unwrap();
                if value > end {
                    return false;
                }
                CalendarSelection::Range(Some(value), Some(end))
            }
            CalendarSelection::Range(Some(a), None) => {
                CalendarSelection::Range(Some((*a).min(value)), Some((*a).max(value)))
            }
            CalendarSelection::Range(_, _) => CalendarSelection::Range(Some(value), None),
            CalendarSelection::Multiple(values) => {
                let mut values = values.clone();
                if !values.remove(&value) {
                    values.insert(value);
                }
                CalendarSelection::Multiple(values)
            }
        };
        if !self.selection_allowed(&selection) {
            return false;
        }
        self.selection = selection;
        self.interaction.preview = None;
        self.selection.is_complete()
    }
    pub(super) fn activate_enhanced(&mut self, date: NaiveDate, cx: &mut Context<Self>) -> bool {
        let old = self.selection.clone();
        let complete = self.select_enhanced(date);
        if self.selection != old {
            self.interaction.active = Some(date);
            if date.month() != self.current_month as u32 || date.year() != self.current_year {
                self.browse_to(date);
            }
            cx.emit(CalendarInteractionEvent::Changed(self.selection.clone()));
            if complete && !matches!(self.selection, CalendarSelection::Multiple(_)) {
                cx.emit(CalendarEvent::Selected(self.date()));
            }
            cx.notify();
        }
        complete
    }
    fn preview_contains(&self, date: NaiveDate) -> bool {
        match (&self.selection, self.interaction.preview) {
            (CalendarSelection::Range(Some(start), None), Some(end)) => {
                date >= (*start).min(end) && date <= (*start).max(end)
            }
            _ => false,
        }
    }
    fn navigation_step(&self) -> i32 {
        match self.view {
            CalendarView::Day => 1,
            CalendarView::Month => 12,
            CalendarView::Year => 120,
        }
    }
    fn shifted_month(&self, delta: i32) -> Option<NaiveDate> {
        let date = NaiveDate::from_ymd_opt(self.current_year, self.current_month as u32, 1)?;
        let date = if delta < 0 {
            date.checked_sub_months(Months::new(delta.unsigned_abs()))
        } else {
            date.checked_add_months(Months::new(delta as u32))
        }?;
        supported(date).then_some(date)
    }
    pub fn navigate(&mut self, delta: i32, cx: &mut Context<Self>) {
        if let Some(date) = self.shifted_month(delta) {
            self.browse_to(date);
            cx.emit(CalendarInteractionEvent::PanelChanged(
                self.view,
                self.current_year,
                self.current_month as u32,
            ));
            cx.notify();
        }
    }
    fn activate_cell(&mut self, date: NaiveDate, cx: &mut Context<Self>) {
        match self.view {
            CalendarView::Year if self.granularity() != CalendarGranularity::Year => {
                self.browse_to(date);
                self.view = CalendarView::Month;
                cx.emit(CalendarInteractionEvent::PanelChanged(
                    self.view,
                    self.current_year,
                    self.current_month as u32,
                ));
                cx.notify();
            }
            CalendarView::Month
                if matches!(
                    self.granularity(),
                    CalendarGranularity::Day | CalendarGranularity::Week
                ) =>
            {
                self.browse_to(date);
                self.view = CalendarView::Day;
                cx.emit(CalendarInteractionEvent::PanelChanged(
                    self.view,
                    self.current_year,
                    self.current_month as u32,
                ));
                cx.notify();
            }
            _ => {
                self.activate_enhanced(date, cx);
            }
        }
    }
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        first_day: Weekday,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt {
            return;
        }
        let active = self.active_date();
        let columns = if self.view.is_day() {
            7
        } else if self.granularity() == CalendarGranularity::Quarter && self.view.is_month() {
            2
        } else {
            3
        };
        let row_index = if self.view.is_day() {
            (active.weekday().num_days_from_sunday() + 7 - first_day.num_days_from_sunday()) % 7
        } else if self.view.is_year() {
            ((active.year() - self.decade_start()).max(0) as u32) % columns
        } else {
            ((active.month0()
                / if self.granularity() == CalendarGranularity::Quarter {
                    3
                } else {
                    1
                })
                % columns) as u32
        };
        let delta = match key {
            "left" => -1,
            "right" => 1,
            "up" => -(columns as i32),
            "down" => columns as i32,
            "home" => -(row_index as i32),
            "end" => columns as i32 - 1 - row_index as i32,
            "enter" | "space" => {
                self.activate_cell(active, cx);
                cx.stop_propagation();
                return;
            }
            "pageup" | "pagedown" => {
                let delta = if modifiers.shift {
                    12
                } else {
                    self.navigation_step()
                } * if key == "pageup" { -1 } else { 1 };
                let next = if delta < 0 {
                    active.checked_sub_months(Months::new(delta.unsigned_abs()))
                } else {
                    active.checked_add_months(Months::new(delta as u32))
                };
                if let Some(next) = next.filter(|date| supported(*date)) {
                    self.move_active(next, cx);
                }
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        let next = if self.view.is_day() {
            active.checked_add_signed(chrono::Duration::days(delta as i64))
        } else {
            let months = delta
                * if self.view.is_year() {
                    12
                } else if self.granularity() == CalendarGranularity::Quarter {
                    3
                } else {
                    1
                };
            if months < 0 {
                active.checked_sub_months(Months::new(months.unsigned_abs()))
            } else {
                active.checked_add_months(Months::new(months as u32))
            }
        };
        if let Some(next) = next.filter(|date| supported(*date)) {
            self.move_active(next, cx);
        }
        // Keep focus on the single roving handle after the cell is re-rendered.
        self.focus_handle.focus(window, cx);
        cx.stop_propagation();
    }
    fn move_active(&mut self, next: NaiveDate, cx: &mut Context<Self>) {
        let old = (self.current_year, self.current_month);
        self.browse_to(next);
        self.interaction.preview = self.unit(next);
        if old != (self.current_year, self.current_month) {
            cx.emit(CalendarInteractionEvent::PanelChanged(
                self.view,
                self.current_year,
                self.current_month as u32,
            ));
        }
        cx.notify();
    }
    fn decade_start(&self) -> i32 {
        (self.current_year / 10 * 10 - 1).max(1)
    }
}
impl EventEmitter<CalendarInteractionEvent> for CalendarState {}

impl Calendar {
    fn enhanced_item(
        &self,
        id: String,
        date: NaiveDate,
        kind: CalendarItemKind,
        muted: bool,
        tab_target: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let s = self.state.read(cx);
        let unit = s.unit(date);
        let terminal = s.view == s.initial_view();
        let disabled =
            !supported(date) || (terminal && unit.is_none_or(|date| !s.date_allowed(date)));
        let active = tab_target;
        let st = CalendarItemState {
            kind,
            date: Some(date),
            active: unit.is_some_and(|date| s.selection.contains(date)),
            in_range: unit.is_some_and(|date| s.selection.in_range(date)),
            muted,
            disabled: disabled || !self.interactive,
            today: date == s.today,
            focused: active && s.focus_handle.is_focused(window),
            preview: unit.is_some_and(|date| s.preview_contains(date)),
            navigation_months: 0,
        };
        let label = if !supported(date) {
            SharedString::default()
        } else {
            match kind {
                CalendarItemKind::Month if s.granularity() == CalendarGranularity::Quarter => {
                    format!("Q{}", date.month0() / 3 + 1).into()
                }
                CalendarItemKind::Month => (self.label)(kind, date.month() as i32),
                CalendarItemKind::Year => (self.label)(kind, date.year()),
                _ => (self.label)(kind, date.day() as i32),
            }
        };
        let mut item = CalendarItem::new(id, st)
            .with_label(label)
            .role(Role::GridCell)
            .aria_label(date.to_string())
            .aria_description(if st.disabled {
                "此日期不可选"
            } else if st.today {
                "今天"
            } else {
                ""
            })
            .aria_selected(st.active);
        if active && self.interactive {
            item = item.track_focus(&s.focus_handle.clone().tab_stop(true));
        }
        if self.interactive {
            let entity = self.state.clone();
            item = item.on_click(move |_, window, cx| {
                entity.update(cx, |s, cx| {
                    s.interaction.active = Some(date);
                    s.focus_handle.focus(window, cx);
                    if !disabled {
                        s.activate_cell(date, cx);
                    }
                    cx.notify();
                });
            });
            let entity = self.state.clone();
            item = item.on_hover(move |hover, _, cx| {
                entity.update(cx, |s, cx| {
                    let preview = if *hover { s.unit(date) } else { None };
                    if s.interaction.preview != preview
                        && matches!(s.selection, CalendarSelection::Range(Some(_), None))
                    {
                        s.interaction.preview = preview;
                        cx.notify();
                    }
                });
            });
        }
        (self.item)(item, st, window, cx)
    }
    pub(super) fn render_enhanced(self, window: &mut Window, cx: &mut App) -> AnyElement {
        let state = self.state.read(cx);
        let first_day = if state.granularity() == CalendarGranularity::Week {
            Weekday::Mon
        } else {
            self.first_day_of_week
        };
        let view = state.view;
        let year = state.current_year;
        let month = state.current_month as u32;
        let active_date = state.active_date();
        let count = if view.is_day() {
            self.number_of_months
                .min(2)
                .min(if year == 9999 && month == 12 { 1 } else { 2 })
        } else {
            1
        };
        let mut header = h_flex().items_center().justify_between();
        for (kind, value) in [
            (CalendarItemKind::Previous, -12),
            (CalendarItemKind::Previous, 0),
            (CalendarItemKind::MonthToggle, month as i32),
            (CalendarItemKind::YearToggle, year),
            (CalendarItemKind::Next, 0),
            (CalendarItemKind::Next, 12),
        ] {
            if value.unsigned_abs() == 12 && !view.is_day() {
                continue;
            }
            if kind == CalendarItemKind::MonthToggle
                && (view.is_year()
                    || self.state.read(cx).granularity() == CalendarGranularity::Year)
            {
                continue;
            }
            let s = self.state.read(cx);
            let delta = if value.unsigned_abs() == 12 {
                value
            } else {
                s.navigation_step()
                    * if kind == CalendarItemKind::Previous {
                        -1
                    } else {
                        1
                    }
            };
            let navigation = matches!(kind, CalendarItemKind::Previous | CalendarItemKind::Next);
            let disabled = !self.interactive || (navigation && s.shifted_month(delta).is_none());
            let mut st = CalendarItemState::new(kind).disabled(disabled);
            if navigation {
                st.navigation_months = delta;
            }
            let mut item = CalendarItem::new((self.id.clone(), format!("{kind:?}-{value}")), st)
                .with_label((self.label)(kind, value))
                .role(Role::Button);
            if !disabled {
                let focus = window.use_keyed_state(
                    (self.id.clone(), format!("header-focus-{kind:?}-{value}")),
                    cx,
                    |_, cx| cx.focus_handle().tab_stop(true),
                );
                let entity = self.state.clone();
                let key_entity = entity.clone();
                item = item
                    .track_focus(focus.read(cx))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |s, cx| {
                            if navigation {
                                s.navigate(delta, cx);
                            } else {
                                s.view = if kind == CalendarItemKind::YearToggle {
                                    CalendarView::Year
                                } else {
                                    CalendarView::Month
                                };
                                cx.emit(CalendarInteractionEvent::PanelChanged(
                                    s.view,
                                    s.current_year,
                                    s.current_month as u32,
                                ));
                                cx.notify();
                            }
                        });
                    })
                    .on_key_down(move |event, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            key_entity.update(cx, |s, cx| {
                                if navigation {
                                    s.navigate(delta, cx);
                                } else {
                                    s.view = if kind == CalendarItemKind::YearToggle {
                                        CalendarView::Year
                                    } else {
                                        CalendarView::Month
                                    };
                                    cx.emit(CalendarInteractionEvent::PanelChanged(
                                        s.view,
                                        s.current_year,
                                        s.current_month as u32,
                                    ));
                                    cx.notify();
                                }
                            });
                            cx.stop_propagation();
                        }
                    });
            }
            header = header.child((self.item)(item, st, window, cx));
        }
        let mut body = h_flex().items_start();
        if view.is_day() {
            for offset in 0..count {
                let (year, month) = self.state.read(cx).offset_year_month(offset);
                let mut panel = v_flex()
                    .id((self.id.clone(), format!("grid-{offset}")))
                    .role(Role::Grid)
                    .aria_label(format!("{year}年{month}月"));
                if count > 1 {
                    panel = panel.child(div().child(format!("{year}年{month}月")));
                }
                let mut weekdays = h_flex()
                    .id((self.id.clone(), format!("weekdays-{offset}")))
                    .role(Role::Row);
                for index in 0..7 {
                    let st = CalendarItemState::new(CalendarItemKind::Weekday)
                        .muted(true)
                        .disabled(true);
                    let item = CalendarItem::new(
                        (self.id.clone(), format!("weekday-{offset}-{index}")),
                        st,
                    )
                    .with_label((self.label)(
                        st.kind(),
                        (index + first_day.num_days_from_sunday()) as i32 % 7,
                    ))
                    .role(Role::ColumnHeader);
                    weekdays = weekdays.child((self.item)(item, st, window, cx));
                }
                panel = panel.child(weekdays);
                for (row, week) in CalendarState::fixed_month_days(year, month, first_day)
                    .into_iter()
                    .enumerate()
                {
                    let mut line = h_flex()
                        .id((self.id.clone(), format!("row-{offset}-{row}")))
                        .role(Role::Row);
                    for date in week {
                        let tab_target =
                            date == active_date && date.year() == year && date.month() == month;
                        line = line.child(self.enhanced_item(
                            format!("{}-day-{offset}-{row}-{date}", self.id),
                            date,
                            CalendarItemKind::Day,
                            date.month() != month,
                            tab_target,
                            window,
                            cx,
                        ));
                    }
                    panel = panel.child(line);
                }
                body = body.child(panel);
            }
        } else {
            let s = self.state.read(cx);
            let quarter = s.granularity() == CalendarGranularity::Quarter && view.is_month();
            let columns = if quarter { 2 } else { 3 };
            let dates: Vec<_> = if view.is_year() {
                (s.decade_start()..s.decade_start() + 12)
                    .filter_map(|year| NaiveDate::from_ymd_opt(year, 1, 1))
                    .collect()
            } else {
                (1..=12)
                    .step_by(if quarter { 3 } else { 1 })
                    .filter_map(|month| NaiveDate::from_ymd_opt(year, month, 1))
                    .collect()
            };
            let mut grid = v_flex()
                .id((self.id.clone(), "units"))
                .role(Role::Grid)
                .aria_label(format!("{year}年"));
            for (index, row) in dates.chunks(columns).enumerate() {
                let mut line = h_flex()
                    .id((self.id.clone(), format!("unit-row-{index}")))
                    .role(Role::Row);
                for date in row {
                    let target = if view.is_year() {
                        date.year() == active_date.year()
                    } else if quarter {
                        date.month0() / 3 == active_date.month0() / 3
                    } else {
                        date.month() == active_date.month()
                    };
                    line = line.child(self.enhanced_item(
                        format!("{}-unit-{date}", self.id),
                        *date,
                        if view.is_year() {
                            CalendarItemKind::Year
                        } else {
                            CalendarItemKind::Month
                        },
                        !supported(*date),
                        target,
                        window,
                        cx,
                    ));
                }
                grid = grid.child(line);
            }
            body = body.child(grid);
        }
        let entity = self.state.clone();
        v_flex()
            .id(self.id)
            .child(header)
            .child(body)
            .when(self.interactive, |root| {
                root.on_key_down(move |event, window, cx| {
                    if entity.read(cx).focus_handle.is_focused(window) {
                        entity.update(cx, |s, cx| s.key_down(event, first_day, window, cx));
                    }
                })
            })
            .refine_style(&self.style)
            .into_any_element()
    }
}
