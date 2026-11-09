use super::{Presentation, model::*};
use crate::popup::PopupAnchor;
use chrono::{NaiveTime, Timelike as _};
use gpui_kit::base::{TimeChoices, TimeRules};
use gpui_kit::base::{
    TimeFieldState, TimeSegment,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, EntityInputHandler as _, EventEmitter, FocusHandle,
    Focusable as _, IntoElement, Render, ScrollHandle, SharedString, Subscription, Window,
};

pub(super) struct Column {
    pub(super) focus: FocusHandle,
    pub(super) scroll: ScrollHandle,
    pub(super) values: Vec<u32>,
    pub(super) active: Option<u32>,
    pub(super) positioning: super::scrolling::ColumnPosition,
}
pub struct YueTimePickerState {
    pub(super) options: TimePickerOptions,
    value: TimePickerValue,
    pub(super) times: [Entity<TimeFieldState>; 2],
    pub(super) inputs: [Entity<InputState>; 2],
    present: [bool; 2],
    pub(super) open: bool,
    pub(super) active_endpoint: usize,
    pub(super) error: Option<TimePickerError>,
    pub(super) presentation: Option<Presentation>,
    pub(super) focus: FocusHandle,
    pub(super) panel_focus: FocusHandle,
    pub(super) anchor: PopupAnchor,
    pub(super) columns: [Vec<Column>; 2],
    pub(super) choices: [TimeChoices; 2],
    rules: TimeRules,
    dirty: bool,
    last_validated: Option<[SharedString; 2]>,
    generation: u64,
    pub(super) source: TimePickerChangeSource,
    pub(super) now_note: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<TimePickerEvent> for YueTimePickerState {}
impl YueTimePickerState {
    pub fn new(
        options: TimePickerOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Self, TimePickerError> {
        options.check()?;
        let rules = options.rules()?;
        let times = std::array::from_fn(|_| {
            cx.new(|cx| {
                TimeFieldState::new(window, cx)
                    .precision(options.precision)
                    .hour_cycle(options.cycle)
            })
        });
        let inputs = std::array::from_fn(|_| cx.new(|cx| InputState::new(window, cx)));
        let columns: [Vec<Column>; 2] = std::array::from_fn(|_| {
            (0..4)
                .map(|_| Column {
                    focus: cx.focus_handle().tab_stop(true),
                    scroll: ScrollHandle::new(),
                    values: vec![],
                    active: None,
                    positioning: Default::default(),
                })
                .collect()
        });
        let focus = cx.focus_handle();
        let panel_focus = cx.focus_handle();
        let mut subscriptions = vec![];
        for (i, input) in inputs.iter().enumerate() {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |s, _, e: &InputEvent, w, cx| match e {
                    InputEvent::Focus => {
                        s.active_endpoint = i;
                        cx.notify();
                    }
                    InputEvent::Change if s.can_modify() => {
                        let text = s.texts(cx);
                        if s.last_validated.as_ref() != Some(&text) {
                            s.dirty = true;
                            s.error = None;
                            cx.notify();
                        }
                    }
                    InputEvent::PressEnter { .. } => {
                        s.accept_input(true, w, cx);
                    }
                    InputEvent::Blur => {
                        let generation = s.generation;
                        cx.defer_in(w, move |s, w, cx| {
                            if s.generation == generation && !s.contains_focus(w, cx) {
                                if s.open {
                                    s.close(TimePickerCloseReason::OutsidePress, false, w, cx)
                                } else {
                                    s.accept_input(false, w, cx);
                                }
                            }
                        });
                    }
                    _ => {}
                },
            ));
        }
        for (i, endpoint) in columns.iter().enumerate() {
            for column in endpoint {
                subscriptions.push(cx.on_focus_in(&column.focus, window, move |s, _, cx| {
                    if s.active_endpoint != i {
                        s.active_endpoint = i;
                        cx.notify();
                    }
                }));
            }
        }
        for handle in [&focus, &panel_focus] {
            subscriptions.push(cx.on_focus_out(handle, window, |s, _, w, cx| {
                let generation = s.generation;
                cx.defer_in(w, move |s, w, cx| {
                    if s.open && s.generation == generation && !s.contains_focus(w, cx) {
                        s.close(TimePickerCloseReason::OutsidePress, false, w, cx);
                    }
                });
            }));
        }
        let mut state = Self {
            options,
            value: TimePickerValue::Empty,
            times,
            inputs,
            present: [false; 2],
            open: false,
            active_endpoint: 0,
            error: None,
            presentation: None,
            focus,
            panel_focus,
            anchor: Default::default(),
            columns,
            choices: [TimeChoices::default(), TimeChoices::default()],
            rules,
            dirty: false,
            last_validated: None,
            generation: 0,
            source: TimePickerChangeSource::Panel,
            now_note: None,
            _subscriptions: subscriptions,
        };
        state.rebuild(window, cx);
        Ok(state)
    }
    pub fn value(&self) -> &TimePickerValue {
        &self.value
    }
    pub fn error(&self) -> Option<&TimePickerError> {
        self.error.as_ref()
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn input(&self, endpoint: TimePickerEndpoint) -> &Entity<InputState> {
        &self.inputs[endpoint.index()]
    }
    pub fn candidate(&self, cx: &App) -> Result<TimePickerValue, TimePickerError> {
        self.options.value_from_parts(self.candidate_parts(cx))
    }
    pub fn focus(&self, w: &mut Window, cx: &mut App) {
        self.inputs[self.active_endpoint]
            .focus_handle(cx)
            .focus(w, cx);
    }
    pub fn set_value(
        &mut self,
        value: TimePickerValue,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TimePickerError> {
        let value = self.options.normalize(value)?;
        self.generation = self.generation.wrapping_add(1);
        self.value = value;
        self.error = None;
        self.reset(w, cx);
        cx.notify();
        Ok(())
    }
    pub fn set_constraints(
        &mut self,
        constraints: TimePickerConstraints,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TimePickerError> {
        let mut next = self.options.clone();
        next.constraints = constraints;
        let rules = next.rules()?;
        self.options = next;
        self.rules = rules;
        self.generation = self.generation.wrapping_add(1);
        self.reset(w, cx);
        self.error = self.options.normalize(self.value.clone()).err();
        cx.notify();
        Ok(())
    }
    pub(super) fn prepare(&mut self, p: Presentation, w: &mut Window, cx: &mut Context<Self>) {
        let old_blocked = self
            .presentation
            .as_ref()
            .is_some_and(|p| p.disabled || p.readonly);
        let old_hide = self.presentation.as_ref().is_some_and(|p| p.hide_disabled);
        let blocked = p.disabled || p.readonly;
        let hide = p.hide_disabled;
        self.presentation = Some(p);
        if blocked && !old_blocked {
            self.close(TimePickerCloseReason::Lifecycle, false, w, cx);
            self.reset(w, cx);
        }
        if hide != old_hide {
            self.rebuild(w, cx);
        }
    }
    pub(super) fn can_modify(&self) -> bool {
        self.presentation
            .as_ref()
            .is_none_or(|p| !p.disabled && !p.readonly)
    }
    pub(super) fn range(&self) -> bool {
        self.options.selection == TimePickerSelection::Range
    }
    pub(super) fn segments(&self) -> Vec<TimeSegment> {
        let mut v = vec![TimeSegment::Hour, TimeSegment::Minute];
        if self.options.precision == TimePrecision::Second {
            v.push(TimeSegment::Second)
        }
        if self.options.cycle == HourCycle::H12 {
            v.push(TimeSegment::Period)
        }
        v
    }
    pub(super) fn candidate_parts(&self, cx: &App) -> [Option<NaiveTime>; 2] {
        std::array::from_fn(|i| self.present[i].then(|| self.times[i].read(cx).time()))
    }
    fn texts(&self, cx: &App) -> [SharedString; 2] {
        std::array::from_fn(|i| self.inputs[i].read(cx).value())
    }
    fn sync_inputs(&mut self, w: &mut Window, cx: &mut App) {
        let parts = self.candidate_parts(cx);
        for (i, t) in parts
            .into_iter()
            .enumerate()
            .take(if self.range() { 2 } else { 1 })
        {
            let text = t.map(|t| self.options.text(t)).unwrap_or_default();
            if self.inputs[i].read(cx).value().as_str() != text {
                self.inputs[i].update(cx, |s, cx| s.set_value(text, w, cx));
            }
        }
        self.dirty = false;
        self.last_validated = Some(self.texts(cx));
    }
    fn load_parts(
        &mut self,
        parts: [Option<NaiveTime>; 2],
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let seed = if parts
            .iter()
            .take(if self.range() { 2 } else { 1 })
            .any(Option::is_none)
        {
            self.rules.nearest(NaiveTime::MIN).unwrap_or(NaiveTime::MIN)
        } else {
            NaiveTime::MIN
        };
        for (i, t) in parts
            .into_iter()
            .enumerate()
            .take(if self.range() { 2 } else { 1 })
        {
            self.present[i] = t.is_some();
            let seed = t.unwrap_or(seed);
            self.times[i].update(cx, |s, cx| {
                s.reset_pending_input();
                s.set_time(seed, w, cx);
            });
        }
        self.sync_inputs(w, cx);
        self.rebuild(w, cx);
    }
    fn reset(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.now_note = None;
        self.load_parts(self.value.parts(), w, cx);
    }
    fn rebuild(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let segments = self.segments();
        let hide = self.presentation.as_ref().is_some_and(|p| p.hide_disabled);
        for i in 0..if self.range() { 2 } else { 1 } {
            let t = self.times[i].read(cx).time();
            let mut rules = self.rules.clone();
            if i == 1 && self.range() && self.present[0] {
                rules = rules.not_before(self.times[0].read(cx).time());
            }
            self.choices[i] = rules.choices(t);
            for (j, segment) in segments.iter().copied().enumerate() {
                let values = rules
                    .values(segment, self.options.cycle, t)
                    .into_iter()
                    .filter(|v| {
                        !hide
                            || self.choices[i].candidate(segment, *v).is_some()
                            || self.present[i] && value(t, segment) == *v
                    })
                    .collect::<Vec<_>>();
                let active = if self.present[i] && values.contains(&value(t, segment)) {
                    Some(value(t, segment))
                } else {
                    values
                        .iter()
                        .copied()
                        .find(|v| self.choices[i].candidate(segment, *v).is_some())
                };
                let column = &mut self.columns[i][j];
                column.values = values;
                column.active = active;
                if let Some(ix) = active.and_then(|v| column.values.iter().position(|x| *x == v)) {
                    column.positioning.request(ix, false);
                }
            }
        }
        let _ = w;
    }
    pub(super) fn contains_focus(&self, w: &Window, cx: &App) -> bool {
        self.focus.contains_focused(w, cx)
            || self.panel_focus.contains_focused(w, cx)
            || self.inputs.iter().any(|i| i.focus_handle(cx).is_focused(w))
            || self.columns.iter().flatten().any(|c| c.focus.is_focused(w))
    }
    pub(super) fn composing(&self, w: &mut Window, cx: &mut App) -> bool {
        self.inputs
            .iter()
            .any(|i| i.update(cx, |s, cx| s.marked_text_range(w, cx).is_some()))
    }
    pub fn open(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.open || !self.can_modify() {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        if !self.dirty {
            self.reset(w, cx);
        }
        self.open = true;
        cx.notify();
        cx.emit(TimePickerEvent::OpenChange(TimePickerOpenChange::new(
            true, None,
        )));
    }
    pub fn close(
        &mut self,
        reason: TimePickerCloseReason,
        restore: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.open {
            return;
        }
        self.open = false;
        self.generation = self.generation.wrapping_add(1);
        self.reset(w, cx);
        self.error = self.options.normalize(self.value.clone()).err();
        if restore {
            self.focus(w, cx)
        }
        cx.notify();
        cx.emit(TimePickerEvent::OpenChange(TimePickerOpenChange::new(
            false,
            Some(reason),
        )));
    }
    pub(super) fn cancel(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(TimePickerCloseReason::Escape, true, w, cx)
        } else {
            self.reset(w, cx);
            self.error = self.options.normalize(self.value.clone()).err();
            cx.notify();
        }
    }
    pub(super) fn focus_panel(&mut self, w: &mut Window, cx: &mut App) {
        if self.can_modify() {
            self.columns[self.active_endpoint][0].focus.focus(w, cx);
        }
    }
    pub(super) fn traverse_tab(&mut self, back: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.composing(w, cx) {
            return;
        }
        if back {
            gpui_kit::base::Root::focus_prev(w, cx)
        } else {
            gpui_kit::base::Root::focus_next(w, cx)
        }
        let generation = self.generation;
        cx.defer_in(w, move |s, w, cx| {
            if s.generation == generation && s.open && !s.contains_focus(w, cx) {
                s.close(TimePickerCloseReason::Tab, false, w, cx);
            }
        });
    }
    fn changed_draft(
        &mut self,
        source: TimePickerChangeSource,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.source = source;
        self.error = None;
        self.sync_inputs(w, cx);
        cx.emit(TimePickerEvent::DraftChange(TimePickerDraft::new(
            self.candidate_parts(cx),
        )));
        if self.options.commit == TimePickerCommitMode::Immediate
            && let Ok(value) = self.candidate(cx)
        {
            self.commit(value, source, false, w, cx);
        }
        cx.notify();
    }
    pub(super) fn select(
        &mut self,
        endpoint: usize,
        segment: TimeSegment,
        v: u32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_modify() {
            return;
        }
        if self.dirty && !self.accept_input(false, w, cx) {
            return;
        }
        let Some(t) = self.choices[endpoint].candidate(segment, v) else {
            return;
        };
        self.select_time(endpoint, t, TimePickerChangeSource::Panel, w, cx);
    }
    fn select_time(
        &mut self,
        endpoint: usize,
        t: NaiveTime,
        source: TimePickerChangeSource,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_modify() {
            return;
        }
        let before = self.candidate_parts(cx);
        let mut next = before;
        next[endpoint] = Some(t);
        if self.range()
            && endpoint == 0
            && let Some(end) = next[1]
            && end < t
        {
            next[1] = self.rules.clone().not_before(t).nearest(t);
            if next[1].is_none() {
                self.error = Some(TimePickerError::NoAvailableTime);
                cx.notify();
                return;
            }
        }
        if let [Some(a), Some(b)] = next
            && self.range()
            && let Err(e) = self.options.normalize(TimePickerValue::Range(a, b))
        {
            self.error = Some(e);
            cx.notify();
            return;
        }
        self.active_endpoint = endpoint;
        if next == before {
            if self.error.take().is_some() {
                cx.notify();
            }
            return;
        }
        for (i, t) in next.into_iter().enumerate() {
            self.present[i] = t.is_some();
            if let Some(t) = t {
                self.times[i].update(cx, |s, cx| s.accept_time_proposal(t, cx));
            }
        }
        self.sync_inputs(w, cx);
        self.rebuild(w, cx);
        for column in self.columns.iter().flatten() {
            if let Some(ix) = column
                .active
                .and_then(|v| column.values.iter().position(|x| *x == v))
            {
                column.positioning.request(ix, true);
            }
        }
        self.changed_draft(source, w, cx);
    }
    pub(super) fn column_key(
        &mut self,
        endpoint: usize,
        column: usize,
        key: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_modify() {
            return;
        }
        self.active_endpoint = endpoint;
        if self.dirty && !matches!(key, "escape" | "enter") && !self.accept_input(false, w, cx) {
            return;
        }
        let segments = self.segments();
        match key {
            "left" | "right" => {
                let next = if key == "left" {
                    column.checked_sub(1)
                } else {
                    (column + 1 < segments.len()).then_some(column + 1)
                };
                if let Some(j) = next {
                    self.columns[endpoint][j].focus.focus(w, cx);
                }
                return;
            }
            "enter" => {
                self.confirm(w, cx);
                return;
            }
            "escape" => {
                self.cancel(w, cx);
                return;
            }
            _ => {}
        }
        if key.chars().count() == 1
            && key
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || matches!(c, 'a' | 'A' | 'p' | 'P'))
        {
            let segment = segments[column];
            let proposal = self.times[endpoint].update(cx, |s, _| {
                s.input_proposal(segment, key.chars().next().unwrap())
            });
            if let Some((t, next)) = proposal
                && let Some(t) = self.choices[endpoint].candidate(segment, value(t, segment))
            {
                self.select_time(endpoint, t, TimePickerChangeSource::Panel, w, cx);
                if let Some(j) = segments.iter().position(|s| *s == next) {
                    self.columns[endpoint][j].focus.focus(w, cx);
                }
            }
            return;
        }
        let segment = segments[column];
        let col = &self.columns[endpoint][column];
        let choices = &self.choices[endpoint];
        let enabled: Vec<_> = col
            .values
            .iter()
            .copied()
            .filter(|v| choices.candidate(segment, *v).is_some())
            .collect();
        let current = col
            .active
            .and_then(|v| enabled.iter().position(|x| *x == v));
        let target = match key {
            "home" => enabled.first().copied(),
            "end" => enabled.last().copied(),
            "down" => enabled.get(current.map_or(0, |i| i + 1)).copied(),
            "up" => current
                .and_then(|i| i.checked_sub(1))
                .and_then(|i| enabled.get(i))
                .copied(),
            "space" => col.active,
            _ => None,
        };
        if let Some(target) = target {
            self.select(endpoint, segment, target, w, cx);
        }
    }
    pub(super) fn no_available(&self) -> bool {
        self.columns[0][0]
            .values
            .iter()
            .all(|v| self.choices[0].candidate(TimeSegment::Hour, *v).is_none())
    }
    pub(super) fn has_candidate(&self, cx: &App) -> bool {
        self.candidate(cx).is_ok_and(|v| !v.is_empty())
    }
    pub fn accept_input(&mut self, close: bool, w: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.can_modify() || self.composing(w, cx) {
            return false;
        }
        let texts = self.texts(cx);
        if self.last_validated.as_ref() == Some(&texts) && !self.dirty {
            if close && self.open && self.error.is_none() {
                return self.confirm(w, cx);
            }
            return false;
        }
        self.last_validated = Some(texts.clone());
        let mut parts = [None; 2];
        for i in 0..if self.range() { 2 } else { 1 } {
            if !texts[i].trim().is_empty() {
                match self.options.parse_time(&texts[i]) {
                    Ok(t) => parts[i] = Some(t),
                    Err(e) => {
                        self.invalid(e, i, cx);
                        return false;
                    }
                }
            }
        }
        let candidate = self.options.value_from_parts(parts);
        // A range's first endpoint can be accepted while the popup remains open.
        if matches!(candidate, Err(TimePickerError::Incomplete)) && self.open && !close {
            self.load_parts(parts, w, cx);
            self.changed_draft(TimePickerChangeSource::Input, w, cx);
            return true;
        }
        match candidate {
            Ok(v) => {
                self.load_parts(parts, w, cx);
                if self.open && !close {
                    self.changed_draft(TimePickerChangeSource::Input, w, cx);
                } else {
                    cx.emit(TimePickerEvent::DraftChange(TimePickerDraft::new(parts)));
                    self.commit(v, TimePickerChangeSource::Input, close, w, cx);
                }
                true
            }
            Err(e) => {
                self.invalid(e, self.active_endpoint, cx);
                false
            }
        }
    }
    fn invalid(&mut self, error: TimePickerError, endpoint: usize, cx: &mut Context<Self>) {
        self.error = Some(error.clone());
        self.dirty = false;
        cx.notify();
        cx.emit(TimePickerEvent::InvalidInput(TimePickerInvalidInput::new(
            error,
            if endpoint == 0 {
                TimePickerEndpoint::Start
            } else {
                TimePickerEndpoint::End
            },
        )));
    }
    fn commit(
        &mut self,
        value: TimePickerValue,
        source: TimePickerChangeSource,
        close: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_modify() {
            return;
        }
        let value = match self.options.normalize(value) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e);
                cx.notify();
                return;
            }
        };
        let changed = self.value != value;
        self.value = value.clone();
        self.error = None;
        self.dirty = false;
        cx.notify();
        if changed {
            cx.emit(TimePickerEvent::Change(TimePickerChange::new(
                value, source,
            )));
        }
        if close && self.open {
            let generation = self.generation;
            cx.defer_in(w, move |s, w, cx| {
                if s.generation == generation {
                    s.close(TimePickerCloseReason::Confirm, true, w, cx);
                }
            });
        }
    }
    pub fn confirm(&mut self, w: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.can_modify() || self.composing(w, cx) {
            return false;
        }
        if self.dirty {
            return self.accept_input(true, w, cx);
        }
        if self.error.is_some() {
            return false;
        }
        match self.candidate(cx) {
            Ok(v) if !v.is_empty() => {
                self.commit(v, self.source, true, w, cx);
                true
            }
            _ => {
                self.error = Some(TimePickerError::Incomplete);
                cx.notify();
                false
            }
        }
    }
    pub fn clear(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.can_modify() {
            return;
        }
        self.load_parts([None, None], w, cx);
        self.commit(
            TimePickerValue::Empty,
            TimePickerChangeSource::Clear,
            true,
            w,
            cx,
        );
        self.focus(w, cx);
    }
    pub fn now(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.can_modify() {
            return;
        }
        if self.dirty && !self.accept_input(false, w, cx) {
            return;
        }
        let raw = (self.options.clock)();
        let raw = self.options.precision.truncate(raw);
        let mut rules = self.rules.clone();
        if self.active_endpoint == 1 && self.present[0] {
            rules = rules.not_before(self.times[0].read(cx).time());
        }
        if let Some(t) = rules.nearest(raw) {
            self.select_time(self.active_endpoint, t, TimePickerChangeSource::Now, w, cx);
            self.now_note =
                (t != raw).then(|| format!("已采用最近可选时间 {}", self.options.text(t)));
            cx.notify();
        } else {
            self.error = Some(TimePickerError::NoAvailableTime);
            cx.notify();
        }
    }
    pub(super) fn step_input(&mut self, delta: i32, w: &mut Window, cx: &mut Context<Self>) {
        if !self.can_modify()
            || self.composing(w, cx)
            || self.presentation.as_ref().is_some_and(|p| !p.editable)
        {
            return;
        }
        let i = self.active_endpoint;
        let text = self.inputs[i].read(cx).value();
        let t = if text.is_empty() {
            self.rules.nearest(NaiveTime::MIN)
        } else {
            self.options.parse_time(&text).ok()
        };
        let Some(t) = t else {
            self.invalid(TimePickerError::InvalidInput, i, cx);
            return;
        };
        let selected = self.inputs[i].update(cx, |s, cx| s.selected_text_range(false, w, cx));
        let pos = selected.map_or(0, |v| v.range.start);
        let segment = if pos < 2 {
            TimeSegment::Hour
        } else if pos < 5 {
            TimeSegment::Minute
        } else if self.options.precision == TimePrecision::Second && pos < 8 {
            TimeSegment::Second
        } else if self.options.cycle == HourCycle::H12 {
            TimeSegment::Period
        } else if self.options.precision == TimePrecision::Second {
            TimeSegment::Second
        } else {
            TimeSegment::Minute
        };
        self.open(w, cx);
        self.times[i].update(cx, |s, cx| s.set_time(t, w, cx));
        self.rebuild(w, cx);
        let choices = &self.choices[i];
        let values: Vec<_> = self
            .rules
            .values(segment, self.options.cycle, t)
            .into_iter()
            .filter(|v| choices.candidate(segment, *v).is_some())
            .collect();
        if values.is_empty() {
            return;
        }
        let current = value(t, segment);
        let ix = if delta > 0 {
            values.iter().position(|v| *v > current).unwrap_or(0)
        } else {
            values
                .iter()
                .rposition(|v| *v < current)
                .unwrap_or(values.len() - 1)
        };
        self.select(i, segment, values[ix], w, cx);
        let start = match segment {
            TimeSegment::Hour => 0,
            TimeSegment::Minute => 3,
            TimeSegment::Second => 6,
            TimeSegment::Period => {
                if self.options.precision == TimePrecision::Second {
                    9
                } else {
                    6
                }
            }
        };
        self.inputs[i].update(cx, |s, cx| s.set_selected_range(start..start + 2, cx));
    }
}
pub(super) fn value(t: NaiveTime, segment: TimeSegment) -> u32 {
    match segment {
        TimeSegment::Hour => t.hour(),
        TimeSegment::Minute => t.minute(),
        TimeSegment::Second => t.second(),
        TimeSegment::Period => t.hour() / 12,
    }
}
impl Render for YueTimePickerState {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::element::render(self, w, cx)
    }
}
