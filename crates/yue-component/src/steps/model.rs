use gpui_kit::SharedString;

/// Committed workflow position. Completion is explicit, never an out-of-range index.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub enum StepsValue {
    #[default]
    Pending,
    Current(SharedString),
    Complete,
}
impl From<&str> for StepsValue {
    fn from(key: &str) -> Self {
        Self::Current(key.to_owned().into())
    }
}
impl From<SharedString> for StepsValue {
    fn from(key: SharedString) -> Self {
        Self::Current(key)
    }
}
impl StepsValue {
    pub fn key(&self) -> Option<&SharedString> {
        if let Self::Current(key) = self {
            Some(key)
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StepsStatus {
    Waiting,
    #[default]
    Process,
    Finish,
    Error,
}
impl StepsStatus {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Waiting => "未开始",
            Self::Process => "进行中",
            Self::Finish => "已完成",
            Self::Error => "错误",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StepsSize {
    Small,
    #[default]
    Medium,
    Large,
}
impl StepsSize {
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StepsLayout {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StepsVariant {
    #[default]
    Default,
    Dot,
    Navigation,
    Arrow,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StepsLabelPlacement {
    #[default]
    Right,
    Below,
}
/// A requested position. Hosts explicitly accept by supplying a new value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepsChange {
    pub(super) key: SharedString,
    pub(super) index: usize,
    pub(super) before: StepsValue,
}
impl StepsChange {
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn before(&self) -> &StepsValue {
        &self.before
    }
}
pub(super) fn status(
    value: &StepsValue,
    current: Option<usize>,
    index: usize,
    current_status: StepsStatus,
    explicit: Option<StepsStatus>,
) -> StepsStatus {
    explicit.unwrap_or_else(|| match value {
        StepsValue::Pending => StepsStatus::Waiting,
        StepsValue::Complete => StepsStatus::Finish,
        StepsValue::Current(_) => match index.cmp(&current.expect("validated current key")) {
            std::cmp::Ordering::Less => StepsStatus::Finish,
            std::cmp::Ordering::Equal => current_status,
            std::cmp::Ordering::Greater => StepsStatus::Waiting,
        },
    })
}
pub(super) fn line(from: StepsStatus, to: StepsStatus) -> &'static str {
    if to == StepsStatus::Error {
        "--steps-line-color-error"
    } else if from == StepsStatus::Finish {
        "--steps-line-color-finish"
    } else {
        "--steps-line-color"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_states_override_position_without_changing_workflow_identity() {
        let value = StepsValue::from("two");
        assert_eq!(
            status(&value, Some(1), 0, StepsStatus::Error, None),
            StepsStatus::Finish
        );
        assert_eq!(
            status(&value, Some(1), 1, StepsStatus::Error, None),
            StepsStatus::Error
        );
        assert_eq!(
            status(&value, Some(1), 2, StepsStatus::Process, None),
            StepsStatus::Waiting
        );
        assert_eq!(
            status(
                &StepsValue::Complete,
                None,
                0,
                StepsStatus::Process,
                Some(StepsStatus::Error)
            ),
            StepsStatus::Error
        );
        assert_eq!(
            status(&StepsValue::Pending, None, 0, StepsStatus::Process, None),
            StepsStatus::Waiting
        );
        assert_eq!(
            status(&StepsValue::Complete, None, 0, StepsStatus::Process, None),
            StepsStatus::Finish
        );
        assert_eq!(
            line(StepsStatus::Finish, StepsStatus::Error),
            "--steps-line-color-error"
        );
        assert_eq!(
            line(StepsStatus::Process, StepsStatus::Waiting),
            "--steps-line-color"
        );
    }
}
