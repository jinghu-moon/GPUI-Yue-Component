//! Ordered packing; configured spans are never overwritten by a resolved plan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DescriptionsSize {
    Small,
    #[default]
    Medium,
    Large,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DescriptionsLayout {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DescriptionsAlign {
    #[default]
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DescriptionsVerticalAlign {
    Top,
    #[default]
    Center,
    Bottom,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DescriptionsSpan {
    Columns(u16),
    Filled,
}
impl Default for DescriptionsSpan {
    fn default() -> Self {
        Self::Columns(1)
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DescriptionsTextOverflow {
    #[default]
    Wrap,
    Ellipsis,
    Lines(usize),
}
pub(super) fn columns(width: f32, min: f32, gap: f32, max: u16) -> u16 {
    (((f64::from(width.max(0.)) + f64::from(gap)) / (f64::from(min) + f64::from(gap)))
        .floor()
        .clamp(1., f64::from(max))) as u16
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cell {
    pub row: usize,
    pub column: u16,
    pub span: u16,
}
pub(super) fn pack(
    spans: impl IntoIterator<Item = DescriptionsSpan>,
    columns: u16,
    bordered: bool,
) -> Vec<Cell> {
    let mut cells: Vec<Cell> = vec![];
    let (mut row, mut column) = (0, 0u16);
    for span in spans {
        let span = match span {
            DescriptionsSpan::Columns(s) => s.clamp(1, columns),
            DescriptionsSpan::Filled => columns - column,
        };
        if u32::from(column) + u32::from(span) > u32::from(columns) {
            if bordered && let Some(last) = cells.last_mut() {
                last.span += columns - column;
            }
            row += 1;
            column = 0;
        }
        cells.push(Cell { row, column, span });
        column += span;
        if column == columns {
            row += 1;
            column = 0;
        }
    }
    if bordered && column > 0 {
        cells.last_mut().unwrap().span += columns - column;
    }
    cells
}
