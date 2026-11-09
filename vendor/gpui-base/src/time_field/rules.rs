//! Opt-in wall-clock selection rules shared by segmented editors and scrolling columns.
use super::{HourCycle, TimePrecision, TimeSegment};
use chrono::{NaiveTime, Timelike as _};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeSteps {
    hour: u32,
    minute: u32,
    second: u32,
}
impl Default for TimeSteps {
    fn default() -> Self {
        Self::new()
    }
}
impl TimeSteps {
    pub fn new() -> Self {
        Self {
            hour: 1,
            minute: 1,
            second: 1,
        }
    }
    pub fn hour(mut self, value: u32) -> Self {
        self.hour = value;
        self
    }
    pub fn minute(mut self, value: u32) -> Self {
        self.minute = value;
        self
    }
    pub fn second(mut self, value: u32) -> Self {
        self.second = value;
        self
    }
    pub fn is_valid(self, precision: TimePrecision) -> bool {
        (1..=24).contains(&self.hour)
            && (1..=60).contains(&self.minute)
            && (1..=60).contains(&self.second)
            && (precision == TimePrecision::Second || self.second == 1)
    }
}
type HourPredicate = Rc<dyn Fn(u32) -> bool>;
type MinutePredicate = Rc<dyn Fn(u32, u32) -> bool>;
type SecondPredicate = Rc<dyn Fn(u32, u32, u32) -> bool>;
/// Immutable rule configuration. Predicates return true for a disabled unit.
#[derive(Clone, Default)]
pub struct TimeRules {
    min: Option<NaiveTime>,
    max: Option<NaiveTime>,
    hours: Option<HourPredicate>,
    minutes: Option<MinutePredicate>,
    seconds: Option<SecondPredicate>,
    steps: TimeSteps,
    precision: TimePrecision,
}
impl TimeRules {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn min(mut self, v: NaiveTime) -> Self {
        self.min = Some(v);
        self
    }
    /// Intersect an endpoint's lower bound with the configured availability window.
    pub fn not_before(mut self, v: NaiveTime) -> Self {
        self.min = Some(self.min.map_or(v, |old| old.max(v)));
        self
    }
    pub fn max(mut self, v: NaiveTime) -> Self {
        self.max = Some(v);
        self
    }
    pub fn disabled_hours(mut self, f: impl Fn(u32) -> bool + 'static) -> Self {
        self.hours = Some(Rc::new(f));
        self
    }
    pub fn disabled_minutes(mut self, f: impl Fn(u32, u32) -> bool + 'static) -> Self {
        self.minutes = Some(Rc::new(f));
        self
    }
    pub fn disabled_seconds(mut self, f: impl Fn(u32, u32, u32) -> bool + 'static) -> Self {
        self.seconds = Some(Rc::new(f));
        self
    }
    pub fn configured(mut self, precision: TimePrecision, steps: TimeSteps) -> Option<Self> {
        self.precision = precision;
        self.steps = steps;
        if !steps.is_valid(precision)
            || self.min.is_some_and(|v| v.nanosecond() >= 1_000_000_000)
            || self.max.is_some_and(|v| v.nanosecond() >= 1_000_000_000)
        {
            return None;
        }
        self.min = self.min.map(|v| precision.truncate(v));
        self.max = self.max.map(|v| precision.truncate(v));
        if self.min.zip(self.max).is_some_and(|(a, b)| a > b) {
            None
        } else {
            Some(self)
        }
    }
    pub fn allows(&self, t: NaiveTime) -> bool {
        t.nanosecond() == 0
            && (self.precision == TimePrecision::Second || t.second() == 0)
            && t.hour().is_multiple_of(self.steps.hour)
            && t.minute().is_multiple_of(self.steps.minute)
            && t.second().is_multiple_of(self.steps.second)
            && self.min.is_none_or(|v| t >= v)
            && self.max.is_none_or(|v| t <= v)
            && self.hours.as_ref().is_none_or(|f| !f(t.hour()))
            && self
                .minutes
                .as_ref()
                .is_none_or(|f| !f(t.hour(), t.minute()))
            && self
                .seconds
                .as_ref()
                .is_none_or(|f| !f(t.hour(), t.minute(), t.second()))
    }
    fn visit(&self, mut f: impl FnMut(NaiveTime)) {
        for h in (0..24).step_by(self.steps.hour as usize) {
            if self.hours.as_ref().is_some_and(|p| p(h)) {
                continue;
            }
            for m in (0..60).step_by(self.steps.minute as usize) {
                if self.minutes.as_ref().is_some_and(|p| p(h, m)) {
                    continue;
                }
                for s in (0..if self.precision == TimePrecision::Minute {
                    1
                } else {
                    60
                })
                    .step_by(self.steps.second as usize)
                {
                    let t = NaiveTime::from_hms_opt(h, m, s).unwrap();
                    if self.allows(t) {
                        f(t)
                    }
                }
            }
        }
    }
    pub fn nearest(&self, target: NaiveTime) -> Option<NaiveTime> {
        let mut best = None;
        self.visit(|t| keep_nearest(&mut best, t, target));
        best
    }
    /// Prepare all unit proposals in one bounded traversal, never once per painted item.
    pub fn choices(&self, time: NaiveTime) -> TimeChoices {
        let mut out = TimeChoices::default();
        self.visit(|t| {
            let hour_target = time.with_hour(t.hour()).unwrap();
            keep_nearest(&mut out.hours[t.hour() as usize], t, hour_target);
            if t.hour() % 12 == time.hour() % 12 {
                keep_nearest(&mut out.periods[(t.hour() / 12) as usize], t, hour_target);
            }
            if t.hour() == time.hour() {
                let target = time.with_minute(t.minute()).unwrap();
                keep_nearest(&mut out.minutes[t.minute() as usize], t, target);
                if t.minute() == time.minute() {
                    out.seconds[t.second() as usize] = Some(t);
                }
            }
        });
        out
    }
    pub fn values(&self, segment: TimeSegment, cycle: HourCycle, time: NaiveTime) -> Vec<u32> {
        match segment {
            TimeSegment::Hour => (0..24)
                .filter(|h| {
                    h % self.steps.hour == 0
                        && (cycle == HourCycle::H23 || h / 12 == time.hour() / 12)
                })
                .collect(),
            TimeSegment::Minute => (0..60).step_by(self.steps.minute as usize).collect(),
            TimeSegment::Second => (0..60).step_by(self.steps.second as usize).collect(),
            TimeSegment::Period => vec![0, 1],
        }
    }
}
fn keep_nearest(best: &mut Option<NaiveTime>, t: NaiveTime, target: NaiveTime) {
    let distance = |v: NaiveTime| {
        (i64::from(v.num_seconds_from_midnight()) - i64::from(target.num_seconds_from_midnight()))
            .abs()
    };
    if best.is_none_or(|old| distance(t) < distance(old)) {
        *best = Some(t);
    }
}
#[derive(Clone)]
pub struct TimeChoices {
    hours: [Option<NaiveTime>; 24],
    minutes: [Option<NaiveTime>; 60],
    seconds: [Option<NaiveTime>; 60],
    periods: [Option<NaiveTime>; 2],
}
impl Default for TimeChoices {
    fn default() -> Self {
        Self {
            hours: [None; 24],
            minutes: [None; 60],
            seconds: [None; 60],
            periods: [None; 2],
        }
    }
}
impl TimeChoices {
    pub fn candidate(&self, segment: TimeSegment, value: u32) -> Option<NaiveTime> {
        match segment {
            TimeSegment::Hour => self.hours.get(value as usize),
            TimeSegment::Minute => self.minutes.get(value as usize),
            TimeSegment::Second => self.seconds.get(value as usize),
            TimeSegment::Period => self.periods.get(value as usize),
        }
        .copied()
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(h: u32, m: u32, s: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, s).unwrap()
    }
    fn rules() -> TimeRules {
        TimeRules::new()
            .configured(TimePrecision::Second, TimeSteps::new())
            .unwrap()
    }
    #[test]
    fn non_divisor_steps_and_12_hour_values_use_real_hours() {
        let r = TimeRules::new()
            .configured(TimePrecision::Minute, TimeSteps::new().hour(2).minute(7))
            .unwrap();
        assert_eq!(
            r.values(TimeSegment::Minute, HourCycle::H23, t(0, 0, 0)),
            vec![0, 7, 14, 21, 28, 35, 42, 49, 56]
        );
        assert_eq!(
            r.values(TimeSegment::Hour, HourCycle::H12, t(12, 0, 0)),
            vec![12, 14, 16, 18, 20, 22]
        );
        assert!(r.allows(t(22, 56, 0)));
        assert!(!r.allows(t(23, 56, 0)));
        assert!(
            TimeRules::new()
                .configured(TimePrecision::Minute, TimeSteps::new().second(10))
                .is_none()
        );
    }
    #[test]
    fn candidates_repair_only_the_selected_prefix_and_period() {
        let r = rules()
            .min(t(9, 30, 0))
            .disabled_minutes(|h, m| h == 12 && m < 45)
            .disabled_seconds(|h, m, s| h == 10 && m == 15 && s < 20);
        let c = r.choices(t(10, 15, 0));
        assert_eq!(c.candidate(TimeSegment::Hour, 9), Some(t(9, 30, 0)));
        assert_eq!(c.candidate(TimeSegment::Hour, 12), Some(t(12, 45, 0)));
        assert_eq!(c.candidate(TimeSegment::Minute, 15), Some(t(10, 15, 20)));
        assert_eq!(c.candidate(TimeSegment::Period, 0), Some(t(10, 14, 59)));
        assert_eq!(c.candidate(TimeSegment::Period, 1), Some(t(22, 15, 0)));
        assert!(c.candidate(TimeSegment::Hour, 8).is_none());
    }
    #[test]
    fn nearest_ties_choose_earlier_and_never_wrap_midnight() {
        let r = TimeRules::new()
            .configured(TimePrecision::Minute, TimeSteps::new().minute(10))
            .unwrap();
        assert_eq!(r.nearest(t(10, 5, 0)), Some(t(10, 0, 0)));
        assert_eq!(r.nearest(t(23, 59, 59)), Some(t(23, 50, 0)));
        assert_eq!(rules().disabled_hours(|_| true).nearest(t(12, 0, 0)), None);
    }
    #[test]
    fn intersected_range_bound_cannot_relax_availability() {
        let r = rules().min(t(9, 0, 0)).not_before(t(8, 0, 0));
        assert!(!r.allows(t(8, 0, 0)));
        assert_eq!(r.nearest(t(8, 0, 0)), Some(t(9, 0, 0)));
        assert!(
            r.not_before(t(23, 0, 0))
                .max(t(17, 0, 0))
                .nearest(t(12, 0, 0))
                .is_none()
        );
    }
}
