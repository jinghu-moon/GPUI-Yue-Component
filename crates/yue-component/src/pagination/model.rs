use std::ops::Range;

/// An exact, normalized query snapshot. Pages are one-based; ranges are zero-based.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationValue {
    total: usize,
    page: usize,
    page_size: usize,
}
impl PaginationValue {
    pub fn new(total: usize, page: usize, page_size: usize) -> Result<Self, PaginationError> {
        if page_size == 0 {
            return Err(PaginationError::ZeroPageSize);
        }
        let mut value = Self {
            total,
            page: 1,
            page_size,
        };
        value.page = page.clamp(1, value.page_count());
        Ok(value)
    }
    pub fn total(self) -> usize {
        self.total
    }
    pub fn page(self) -> usize {
        self.page
    }
    pub fn page_size(self) -> usize {
        self.page_size
    }
    pub fn page_count(self) -> usize {
        (self.total / self.page_size + usize::from(!self.total.is_multiple_of(self.page_size)))
            .max(1)
    }
    pub fn item_range(self) -> Range<usize> {
        let start = (self.page - 1) * self.page_size;
        start..start.saturating_add(self.page_size).min(self.total)
    }
    pub(super) fn at(self, page: usize) -> Self {
        Self::new(self.total, page, self.page_size).unwrap()
    }
    pub(super) fn resized(self, size: usize, policy: PaginationSizeChangePolicy) -> Self {
        let page = match policy {
            PaginationSizeChangePolicy::Reset => 1,
            PaginationSizeChangePolicy::KeepFirstItem => self.item_range().start / size + 1,
        };
        Self::new(self.total, page, size).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaginationSizeChangePolicy {
    #[default]
    Reset,
    KeepFirstItem,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaginationSize {
    Small,
    #[default]
    Medium,
    Large,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaginationVariant {
    #[default]
    Ghost,
    Outline,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaginationMode {
    #[default]
    Pages,
    Simple,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaginationChangeSource {
    Page,
    Previous,
    Next,
    First,
    Last,
    BackwardJump,
    ForwardJump,
    JumpInput,
    PageSize,
}

/// A request, not a committed notification. Accept with State::set_value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationChange {
    pub(super) before: PaginationValue,
    pub(super) value: PaginationValue,
    pub(super) source: PaginationChangeSource,
}
impl PaginationChange {
    pub fn before(self) -> PaginationValue {
        self.before
    }
    pub fn value(self) -> PaginationValue {
        self.value
    }
    pub fn source(self) -> PaginationChangeSource {
        self.source
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaginationError {
    ZeroPageSize,
    EmptyPageSizes,
    DuplicatePageSize,
    TooManyPageSizes,
}
impl std::fmt::Display for PaginationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PaginationError {}

pub(super) fn parse_page(text: &str) -> Option<usize> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<usize>().ok().filter(|n| *n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_boundaries_never_overflow_and_empty_ranges_are_empty() {
        assert_eq!(
            PaginationValue::new(10, 1, 0),
            Err(PaginationError::ZeroPageSize)
        );
        for total in [0, 1, 9, 10, 11, usize::MAX - 1, usize::MAX] {
            for size in [1, 2, 10, usize::MAX] {
                for page in [0, 1, 2, usize::MAX] {
                    let v = PaginationValue::new(total, page, size).unwrap();
                    assert!((1..=v.page_count()).contains(&v.page()));
                    let r = v.item_range();
                    assert!(r.start <= r.end && r.end <= total);
                    assert!(r.len() <= size);
                    if total == 0 {
                        assert_eq!(r, 0..0);
                    }
                }
            }
        }
    }
    #[test]
    fn resizing_preserves_the_first_item_only_when_requested() {
        let v = PaginationValue::new(445, 12, 10).unwrap();
        assert_eq!(v.resized(30, PaginationSizeChangePolicy::Reset).page(), 1);
        let next = v.resized(30, PaginationSizeChangePolicy::KeepFirstItem);
        assert_eq!(next.page(), 4);
        assert!(next.item_range().contains(&110));
    }
    #[test]
    fn drafts_use_exact_unsigned_decimal_integers() {
        for text in [
            "",
            "  ",
            "0",
            "-1",
            "+2",
            "1.0",
            "1e2",
            "第2页",
            "１２",
            "184467440737095516160",
        ] {
            assert_eq!(parse_page(text), None, "{text}");
        }
        assert_eq!(parse_page(" 002 "), Some(2));
        assert_eq!(parse_page(&usize::MAX.to_string()), Some(usize::MAX));
    }
}
