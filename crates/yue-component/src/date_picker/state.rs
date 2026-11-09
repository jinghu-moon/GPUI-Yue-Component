use super::{Presentation, model::*};
use crate::popup::PopupAnchor;
use chrono::{NaiveDate, NaiveTime};
use gpui_kit::base::{
    CalendarInteractionEvent, CalendarSelection, CalendarState, TimeFieldEvent, TimeFieldState,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, EntityInputHandler as _, EventEmitter, FocusHandle,
    Focusable as _, IntoElement, Render, Subscription, Window,
};

/// Retain once in the host. Committed value is separate from Base's candidate;
/// input text, calendar candidate and time segments each have exactly one owner.
pub struct YueDatePickerState {
    pub(super) options: DatePickerOptions,
    value: DatePickerValue,
    pub(super) calendar: Entity<CalendarState>,
    pub(super) inputs: [Entity<InputState>; 2],
    pub(super) times: [Entity<TimeFieldState>; 2],
    pub(super) anchor: PopupAnchor,
    pub(super) focus: FocusHandle,
    pub(super) panel_focus: FocusHandle,
    pub(super) open: bool,
    pub(super) active_endpoint: DatePickerEndpoint,
    pub(super) error: Option<DatePickerError>,
    pub(super) presentation: Option<Presentation>,
    pub(super) expanded_tags: bool,
    dirty: [bool; 2],
    // Immutable observation used only to deduplicate Base's trailing Change
    // notification after Enter. Editable text remains exclusively in InputState.
    last_validated_text: Option<[gpui_kit::SharedString; 2]>,
    generation: u64,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<DatePickerEvent> for YueDatePickerState {}
impl YueDatePickerState {
    pub fn new(
        options: DatePickerOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Self, DatePickerError> {
        options.check()?;
        let today = (options.clock)().date();
        let calendar = cx.new(|cx| {
            let mut state = CalendarState::new(window, cx).enhanced(options.granularity.into());
            state.set_today(today);
            state.browse_to(today);
            state.apply_selection(DatePickerValue::Empty.calendar(options.selection));
            state
        });
        let inputs = std::array::from_fn(|_| cx.new(|cx| InputState::new(window, cx)));
        let times = std::array::from_fn(|_| {
            cx.new(|cx| {
                TimeFieldState::new(window, cx)
                    .precision(options.precision.unwrap_or_default())
                    .hour_cycle(options.hour_cycle)
            })
        });
        let mut subscriptions = vec![
            cx.observe(&calendar, |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &calendar,
                window,
                |s, entity, event: &CalendarInteractionEvent, window, cx| {
                    if !s.can_modify() {
                        return;
                    }
                    match event {
                        CalendarInteractionEvent::Changed(selection) => {
                            if entity.read(cx).selection() != selection {
                                return;
                            }
                            s.dirty = [false; 2];
                            s.error = None;
                            let candidate = s.candidate(cx);
                            s.sync_candidate_inputs(window, cx);
                            cx.emit(DatePickerEvent::DraftChange(candidate.clone().ok()));
                            if !s.options.requires_confirmation()
                                && selection.is_complete()
                                && let Ok(value) = candidate
                            {
                                let close = s.options.selection != DatePickerSelection::Multiple
                                    && s.options.precision.is_none();
                                s.commit(
                                    value,
                                    DatePickerChangeSource::Calendar,
                                    close,
                                    window,
                                    cx,
                                );
                            }
                            cx.notify();
                        }
                        CalendarInteractionEvent::PanelChanged(view, year, month) => {
                            cx.emit(DatePickerEvent::PanelChange(*view, *year, *month))
                        }
                    }
                },
            ),
        ];
        for index in 0..2 {
            subscriptions.push(cx.subscribe_in(
                &inputs[index],
                window,
                move |s, _, event: &InputEvent, window, cx| {
                    match event {
                        InputEvent::Change if s.can_modify() => {
                            let text = std::array::from_fn(|i| s.inputs[i].read(cx).value());
                            if s.last_validated_text.as_ref() == Some(&text) {
                                return;
                            }
                            s.dirty[index] = true;
                            s.error = None;
                            cx.notify();
                        }
                        InputEvent::Focus => {
                            s.active_endpoint = if index == 0 {
                                DatePickerEndpoint::Start
                            } else {
                                DatePickerEndpoint::End
                            };
                        }
                        InputEvent::PressEnter { .. } => {
                            s.accept_input(window, cx);
                        }
                        InputEvent::Blur => {
                            // Check after focus transfer: the calendar/other endpoint is still part of this control.
                            let generation = s.generation;
                            cx.defer_in(window, move |s, window, cx| {
                                if s.generation == generation && !s.contains_focus(window, cx) {
                                    if s.options.requires_confirmation() {
                                        if s.open {
                                            s.close(
                                                DatePickerCloseReason::OutsidePress,
                                                false,
                                                window,
                                                cx,
                                            );
                                        }
                                    } else {
                                        s.accept_input(window, cx);
                                    }
                                }
                            });
                        }
                        _ => {}
                    }
                },
            ));
            subscriptions.push(cx.subscribe_in(
                &times[index],
                window,
                |s, _, _: &TimeFieldEvent, window, cx| {
                    if !s.can_modify() {
                        return;
                    }
                    let candidate = s.candidate(cx);
                    s.error = candidate.as_ref().err().cloned();
                    s.sync_candidate_inputs(window, cx);
                    cx.emit(DatePickerEvent::DraftChange(candidate.clone().ok()));
                    if !s.options.requires_confirmation()
                        && let Ok(value) = candidate
                    {
                        s.commit(value, DatePickerChangeSource::Time, false, window, cx);
                    }
                    cx.notify();
                },
            ));
        }
        let focus = cx.focus_handle();
        let panel_focus = cx.focus_handle();
        for handle in [&focus, &panel_focus] {
            subscriptions.push(cx.on_focus_out(handle, window, |s, _, window, cx| {
                let generation = s.generation;
                cx.defer_in(window, move |s, window, cx| {
                    if s.open && s.generation == generation && !s.contains_focus(window, cx) {
                        s.dismiss_outside(window, cx);
                    }
                });
            }));
        }
        let state = Self {
            options,
            value: DatePickerValue::Empty,
            calendar,
            inputs,
            times,
            anchor: Default::default(),
            focus,
            panel_focus,
            open: false,
            active_endpoint: Default::default(),
            error: None,
            presentation: None,
            expanded_tags: false,
            dirty: [false; 2],
            last_validated_text: None,
            generation: 0,
            _subscriptions: subscriptions,
        };
        state.install_rules(cx);
        Ok(state)
    }
    pub fn value(&self) -> &DatePickerValue {
        &self.value
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.restore_focus(window, cx);
    }
    pub fn error(&self) -> Option<&DatePickerError> {
        self.error.as_ref()
    }
    pub fn input(&self, endpoint: DatePickerEndpoint) -> &Entity<InputState> {
        &self.inputs[endpoint.index()]
    }
    pub fn calendar(&self) -> &Entity<CalendarState> {
        &self.calendar
    }
    pub fn candidate(&self, cx: &App) -> Result<DatePickerValue, DatePickerError> {
        let times = std::array::from_fn(|i| self.times[i].read(cx).time());
        self.options
            .candidate(self.calendar.read(cx).selection(), times)
    }
    /// Silent external update. Invalid values leave the current session intact.
    pub fn set_value(
        &mut self,
        value: DatePickerValue,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), DatePickerError> {
        let value = self.options.normalize(value)?;
        self.generation = self.generation.wrapping_add(1);
        self.value = value;
        self.error = None;
        self.dirty = [false; 2];
        self.reset_candidate(window, cx);
        cx.notify();
        Ok(())
    }
    pub fn set_constraints(
        &mut self,
        constraints: DatePickerConstraints,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), DatePickerError> {
        constraints.check()?;
        self.options.constraints = constraints;
        self.generation = self.generation.wrapping_add(1);
        self.install_rules(cx);
        self.error = self.options.normalize(self.value.clone()).err();
        self.reset_candidate(window, cx);
        self.dirty = [false; 2];
        cx.notify();
        Ok(())
    }
    pub(super) fn prepare(
        &mut self,
        presentation: Presentation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old = self
            .presentation
            .as_ref()
            .is_some_and(|p| p.disabled || p.readonly);
        let blocked = presentation.disabled || presentation.readonly;
        self.presentation = Some(presentation);
        if !old && blocked {
            self.generation = self.generation.wrapping_add(1);
            self.reset_candidate(window, cx);
            if self.open {
                self.close(DatePickerCloseReason::Lifecycle, false, window, cx);
            }
        }
    }
    pub(super) fn can_modify(&self) -> bool {
        !self
            .presentation
            .as_ref()
            .is_some_and(|p| p.disabled || p.readonly)
    }
    fn locked(&self) -> [Option<NaiveDate>; 2] {
        let dates = self.value.dates();
        std::array::from_fn(|i| {
            if self.options.locked[i] {
                dates.get(i).copied()
            } else {
                None
            }
        })
    }
    fn install_rules(&self, cx: &mut App) {
        let constraints = self.options.constraints.clone();
        let dates = constraints.clone();
        let locked = self.locked();
        self.calendar.update(cx, |base, _| {
            base.set_range_locks(locked);
            base.set_date_validator(move |date| dates.check_date(date).is_ok());
            base.set_selection_validator(move |value| constraints.check_calendar(value).is_ok());
        });
    }
    fn validate_locked(&self, value: &DatePickerValue) -> Result<(), DatePickerError> {
        if let (DatePickerValue::DateTimeRange(a, b), DatePickerValue::DateTimeRange(c, d)) =
            (&self.value, value)
            && ((self.options.locked[0] && a != c) || (self.options.locked[1] && b != d))
        {
            return Err(DatePickerError::LockedEndpoint);
        }
        let old = self.value.dates();
        let new = value.dates();
        if (0..2).any(|i| self.options.locked[i] && old.get(i) != new.get(i)) {
            return Err(DatePickerError::LockedEndpoint);
        }
        Ok(())
    }
    fn reset_candidate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selection = self.value.calendar(self.options.selection);
        self.calendar.update(cx, |s, cx| {
            s.restore_selection(selection);
            s.set_view(s.initial_view());
            cx.notify();
        });
        match &self.value {
            DatePickerValue::DateTime(v) => {
                self.times[0].update(cx, |s, cx| s.set_time(v.time(), window, cx))
            }
            DatePickerValue::DateTimeRange(a, b) => {
                for (i, time) in [a.time(), b.time()].into_iter().enumerate() {
                    self.times[i].update(cx, |s, cx| s.set_time(time, window, cx));
                }
            }
            _ => {
                for time in &self.times {
                    time.update(cx, |s, cx| s.set_time(NaiveTime::MIN, window, cx));
                }
            }
        }
        self.sync_inputs(&self.value.clone(), window, cx);
        self.dirty = [false; 2];
        self.install_rules(cx);
    }
    fn sync_inputs(&self, value: &DatePickerValue, window: &mut Window, cx: &mut App) {
        for (index, text) in self.options.texts(value).into_iter().enumerate() {
            if self.inputs[index].read(cx).value().as_str() != text {
                self.inputs[index].update(cx, |s, cx| s.set_value(text, window, cx));
            }
        }
    }
    fn sync_candidate_inputs(&self, window: &mut Window, cx: &mut App) {
        if let Ok(value) = self.candidate(cx) {
            self.sync_inputs(&value, window, cx);
        } else if let CalendarSelection::Range(Some(a), None) = self.calendar.read(cx).selection()
            && let Ok(unit) = DatePickerUnit::from_date(*a, self.options.granularity)
        {
            let text = if let Some(precision) = self.options.precision {
                a.and_time(precision.truncate(self.times[0].read(cx).time()))
                    .format(self.options.format_str())
                    .to_string()
            } else {
                self.options.display_unit(unit)
            };
            self.inputs[0].update(cx, |s, cx| s.set_value(text, window, cx));
            self.inputs[1].update(cx, |s, cx| s.set_value("", window, cx));
        }
    }
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open || !self.can_modify() {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.reset_candidate(window, cx);
        let today = (self.options.clock)().date();
        self.calendar.update(cx, |s, cx| {
            s.set_today(today);
            if s.selection().start().is_none() {
                let date = self
                    .options
                    .constraints
                    .min
                    .map_or(today, |min| today.max(min));
                let date = self
                    .options
                    .constraints
                    .max
                    .map_or(date, |max| date.min(max));
                s.browse_to(date);
            }
            cx.notify();
        });
        self.open = true;
        self.error = None;
        cx.notify();
        cx.emit(DatePickerEvent::OpenChange(DatePickerOpenChange::new(
            true, None,
        )));
    }
    pub fn close(
        &mut self,
        reason: DatePickerCloseReason,
        restore: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.open {
            return;
        }
        self.open = false;
        self.generation = self.generation.wrapping_add(1);
        self.reset_candidate(window, cx);
        self.error = None;
        if restore {
            self.restore_focus(window, cx);
        }
        cx.notify();
        cx.emit(DatePickerEvent::OpenChange(DatePickerOpenChange::new(
            false,
            Some(reason),
        )));
    }
    pub(super) fn cancel_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_candidate(window, cx);
        self.error = None;
        self.generation = self.generation.wrapping_add(1);
        cx.notify();
    }
    pub(super) fn dismiss_outside(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let invalid = !self.options.requires_confirmation()
            && self.dirty.iter().any(|v| *v)
            && !self.accept_input(window, cx);
        if invalid {
            if self.open {
                self.open = false;
                self.generation = self.generation.wrapping_add(1);
                cx.emit(DatePickerEvent::OpenChange(DatePickerOpenChange::new(
                    false,
                    Some(DatePickerCloseReason::OutsidePress),
                )));
                cx.notify();
            }
        } else {
            self.close(DatePickerCloseReason::OutsidePress, false, window, cx);
        }
    }
    pub(super) fn restore_focus(&self, window: &mut Window, cx: &mut App) {
        if self.presentation.as_ref().is_some_and(|p| p.disabled) {
            return;
        }
        if self.options.selection == DatePickerSelection::Multiple {
            self.focus.focus(window, cx);
        } else {
            self.inputs[self.active_endpoint.index()]
                .focus_handle(cx)
                .focus(window, cx);
        }
    }
    pub(super) fn focus_calendar(&self, window: &mut Window, cx: &mut App) {
        if !self.open && !self.presentation.as_ref().is_some_and(|p| p.calendar_only) {
            return;
        }
        self.calendar
            .read(cx)
            .focus_handle
            .clone()
            .focus(window, cx);
    }
    pub(super) fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
            || self.panel_focus.contains_focused(window, cx)
            || self
                .inputs
                .iter()
                .any(|input| input.focus_handle(cx).is_focused(window))
    }
    pub(super) fn composing(&self, window: &mut Window, cx: &mut App) -> bool {
        self.inputs
            .iter()
            .any(|input| input.update(cx, |s, cx| s.marked_text_range(window, cx).is_some()))
    }
    pub(super) fn traverse_tab(
        &mut self,
        backwards: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.composing(window, cx) {
            return;
        }
        if backwards {
            gpui_kit::base::Root::focus_prev(window, cx);
        } else {
            gpui_kit::base::Root::focus_next(window, cx);
        }
        if self.open && !self.contains_focus(window, cx) {
            self.dismiss_outside(window, cx);
        }
    }
    pub fn accept_input(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.can_modify() || self.composing(window, cx) {
            return false;
        }
        let texts: Vec<_> = self.inputs.iter().map(|v| v.read(cx).value()).collect();
        if !self.dirty.iter().any(|v| *v) {
            let current = [texts[0].clone(), texts[1].clone()];
            if self.last_validated_text.as_ref() == Some(&current)
                || self.options.texts(&self.value) == [texts[0].to_string(), texts[1].to_string()]
            {
                return false;
            }
        }
        self.last_validated_text = Some([texts[0].clone(), texts[1].clone()]);
        match self
            .options
            .parse([texts[0].as_str(), texts[1].as_str()])
            .and_then(|value| {
                self.validate_locked(&value)?;
                Ok(value)
            }) {
            Ok(value) => {
                self.error = None;
                self.dirty = [false; 2];
                if self.options.requires_confirmation() {
                    if !self.open && !self.presentation.as_ref().is_some_and(|p| p.calendar_only) {
                        self.open(window, cx);
                    }
                    // A typed candidate participates in the same transaction as calendar/time edits.
                    self.load_candidate(&value, window, cx);
                    cx.emit(DatePickerEvent::DraftChange(Some(value)));
                    cx.notify();
                } else {
                    self.commit(value, DatePickerChangeSource::Input, true, window, cx);
                }
                true
            }
            Err(error) => {
                self.error = Some(error.clone());
                cx.notify();
                cx.emit(DatePickerEvent::InvalidInput(DatePickerInvalidInput::new(
                    error,
                    self.active_endpoint,
                )));
                false
            }
        }
    }
    fn load_candidate(
        &mut self,
        value: &DatePickerValue,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.calendar.update(cx, |s, cx| {
            s.apply_selection(value.calendar(self.options.selection));
            cx.notify();
        });
        let times = match value {
            DatePickerValue::DateTime(v) => Some([v.time(), NaiveTime::MIN]),
            DatePickerValue::DateTimeRange(a, b) => Some([a.time(), b.time()]),
            _ => None,
        };
        if let Some(times) = times {
            for (i, time) in times.into_iter().enumerate() {
                self.times[i].update(cx, |s, cx| s.set_time(time, window, cx));
            }
        }
        self.sync_inputs(value, window, cx);
    }
    fn commit(
        &mut self,
        value: DatePickerValue,
        source: DatePickerChangeSource,
        close: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_modify() {
            return;
        }
        let value = match self.options.normalize(value).and_then(|value| {
            self.validate_locked(&value)?;
            Ok(value)
        }) {
            Ok(v) => v,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.error = None;
        self.dirty = [false; 2];
        let changed = self.value != value;
        self.value = value.clone();
        self.install_rules(cx);
        self.sync_inputs(&value, window, cx);
        cx.notify();
        if changed {
            cx.emit(DatePickerEvent::Change(DatePickerChange::new(
                value, source,
            )));
        }
        if close && self.open {
            // Let Change observers run first. External updates invalidate this continuation.
            let generation = self.generation;
            cx.defer_in(window, move |s, window, cx| {
                if s.generation == generation {
                    s.close(
                        if source == DatePickerChangeSource::Confirm {
                            DatePickerCloseReason::Confirm
                        } else {
                            DatePickerCloseReason::Selection
                        },
                        true,
                        window,
                        cx,
                    );
                }
            });
        }
    }
    pub fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.can_modify() || self.composing(window, cx) {
            return false;
        }
        if self.dirty.iter().any(|v| *v) && !self.accept_input(window, cx) {
            return false;
        }
        match self.candidate(cx) {
            Ok(value) => {
                self.commit(value, DatePickerChangeSource::Confirm, true, window, cx);
                true
            }
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                false
            }
        }
    }
    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_modify() || self.options.locked.iter().any(|v| *v) {
            return;
        }
        self.commit(
            DatePickerValue::Empty,
            DatePickerChangeSource::Clear,
            false,
            window,
            cx,
        );
        self.reset_candidate(window, cx);
        self.restore_focus(window, cx);
    }
    pub fn choose(
        &mut self,
        value: DatePickerValue,
        source: DatePickerChangeSource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), DatePickerError> {
        if !self.can_modify() {
            return Err(DatePickerError::Unavailable);
        }
        let value = self.options.normalize(value)?;
        self.validate_locked(&value)?;
        self.load_candidate(&value, window, cx);
        self.error = None;
        self.dirty = [false; 2];
        if self.options.requires_confirmation() {
            cx.emit(DatePickerEvent::DraftChange(Some(value)));
            cx.notify();
        } else {
            self.commit(
                value,
                source,
                self.options.selection != DatePickerSelection::Multiple,
                window,
                cx,
            );
        }
        Ok(())
    }
    pub(super) fn today(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = (self.options.clock)();
        let value = if self.options.precision.is_some() {
            if self.options.selection == DatePickerSelection::Range {
                DatePickerValue::DateTimeRange(now, now)
            } else {
                DatePickerValue::DateTime(now)
            }
        } else {
            let Ok(unit) = DatePickerUnit::from_date(now.date(), self.options.granularity) else {
                return;
            };
            match self.options.selection {
                DatePickerSelection::Single => DatePickerValue::Single(unit),
                DatePickerSelection::Range => DatePickerValue::Range(unit, unit),
                DatePickerSelection::Multiple => {
                    let mut dates = self.calendar.read(cx).selection().dates();
                    dates.push(now.date());
                    DatePickerValue::Multiple(dates)
                }
            }
        };
        if let Err(error) = self.choose(value, DatePickerChangeSource::Today, window, cx) {
            self.error = Some(error);
            cx.notify();
        }
    }
    pub(super) fn remove_date(
        &mut self,
        date: NaiveDate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editing = self.open || self.presentation.as_ref().is_some_and(|p| p.calendar_only);
        let mut dates = if editing {
            self.calendar.read(cx).selection().dates()
        } else {
            self.value.dates()
        };
        dates.retain(|v| *v != date);
        if editing {
            let _ = self.choose(
                DatePickerValue::Multiple(dates),
                DatePickerChangeSource::Tag,
                window,
                cx,
            );
        } else {
            self.commit(
                DatePickerValue::Multiple(dates),
                DatePickerChangeSource::Tag,
                false,
                window,
                cx,
            );
            self.reset_candidate(window, cx);
        }
        self.restore_focus(window, cx);
    }
}
impl Render for YueDatePickerState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::element::render(self, window, cx)
    }
}
