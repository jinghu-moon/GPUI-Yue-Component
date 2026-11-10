use super::*;
use gpui_kit::{Pixels, px};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TablePin {
    Start,
    #[default]
    None,
    End,
}

/// Stable ancestor identity, outermost first. Leaves own widths and renderers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableHeaderGroup {
    key: SharedString,
    title: SharedString,
}
impl TableHeaderGroup {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
        }
    }
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn title(&self) -> &SharedString {
        &self.title
    }
}

#[derive(Clone, Default)]
pub(super) struct ResolvedColumns {
    pub indices: Vec<usize>,
    pub widths: Vec<Pixels>,
    pub offsets: Vec<Pixels>,
    pub pins: Vec<TablePin>,
    pub total: Pixels,
    pub leading: Pixels,
    pub trailing: Pixels,
    pub available: Pixels,
    pub selection: Pixels,
    pub depth: usize,
}
impl ResolvedColumns {
    pub fn resolve<R: 'static>(
        columns: &[TableColumn<R>],
        available: Pixels,
        rem: Pixels,
        selection: Pixels,
    ) -> Self {
        let mut indices: Vec<_> = columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.visible)
            .map(|(i, _)| i)
            .collect();
        // Hide only responsive columns. Preserve explicit user-hidden columns.
        let mut candidates: Vec<_> = indices
            .iter()
            .copied()
            .filter(|i| columns[*i].responsive.is_some())
            .collect();
        candidates.sort_by_key(|i| (std::cmp::Reverse(columns[*i].responsive.unwrap()), *i));
        let mut needed = indices
            .iter()
            .map(|i| {
                columns[*i].width.to_pixels(rem).clamp(
                    columns[*i].minimum.to_pixels(rem),
                    columns[*i].maximum.to_pixels(rem),
                )
            })
            .fold(selection, |a, b| a + b);
        if available > px(0.) {
            for i in candidates {
                if needed <= available {
                    break;
                }
                indices.retain(|j| *j != i);
                needed -= columns[i].width.to_pixels(rem).clamp(
                    columns[i].minimum.to_pixels(rem),
                    columns[i].maximum.to_pixels(rem),
                );
            }
        }
        indices.sort_by_key(|i| (columns[*i].pin, *i));
        let tracks: Vec<_> = indices
            .iter()
            .map(|i| {
                let c = &columns[*i];
                let preferred = c
                    .width
                    .to_pixels(rem)
                    .clamp(c.minimum.to_pixels(rem), c.maximum.to_pixels(rem));
                crate::VirtualGridTrack::new(
                    if c.flex > 0. {
                        c.minimum.to_pixels(rem)
                    } else {
                        preferred
                    },
                    if c.flex > 0. {
                        c.maximum.to_pixels(rem)
                    } else {
                        preferred
                    },
                    c.flex,
                )
            })
            .collect();
        let widths = crate::VirtualGridTrack::resolve(
            &tracks,
            available.max(px(0.)),
            px(0.),
            selection,
            px(0.),
        )
        .expect("validated column constraints");
        let mut total = selection;
        let offsets = widths
            .iter()
            .map(|w| {
                let offset = total;
                total += *w;
                offset
            })
            .collect();
        let mut pins: Vec<_> = indices.iter().map(|i| columns[*i].pin).collect();
        let sum = |pin| {
            indices
                .iter()
                .zip(&widths)
                .filter(|(i, _)| columns[**i].pin == pin)
                .map(|(_, w)| *w)
                .fold(px(0.), |a, b| a + b)
        };
        let mut leading = sum(TablePin::Start);
        let mut trailing = sum(TablePin::End);
        if leading > px(0.) || trailing > px(0.) {
            leading += selection;
        }
        // Keep at least one minimum-width body lane. End loses priority first;
        // preferences are retained, so widening restores both sides.
        let clearance = rem * 2.;
        if leading + trailing + clearance > available {
            for pin in &mut pins {
                if *pin == TablePin::End {
                    *pin = TablePin::None;
                }
            }
            trailing = px(0.);
        }
        if leading + clearance > available {
            pins.fill(TablePin::None);
            leading = px(0.);
        }
        if leading == selection && trailing == px(0.) && !pins.contains(&TablePin::Start) {
            leading = px(0.);
        }
        let depth = indices
            .iter()
            .map(|i| columns[*i].groups.len())
            .max()
            .unwrap_or(0);
        Self {
            indices,
            widths,
            offsets,
            pins,
            total,
            leading,
            trailing,
            available,
            selection,
            depth,
        }
    }
    pub fn has_pins(&self) -> bool {
        self.leading > px(0.) || self.trailing > px(0.)
    }
}

pub(super) fn validate_groups<R: 'static>(columns: &[TableColumn<R>]) -> Result<(), TableError> {
    use std::collections::{HashMap, HashSet};
    let mut groups: HashMap<SharedString, (SharedString, Vec<SharedString>, TablePin, usize)> =
        HashMap::new();
    let mut ordered: Vec<_> = columns.iter().filter(|c| c.visible).collect();
    ordered.sort_by_key(|c| c.pin);
    for (ix, c) in ordered.iter().enumerate() {
        let mut path = Vec::new();
        let mut seen = HashSet::new();
        for g in c.groups.iter() {
            if g.key.trim().is_empty() || !seen.insert(&g.key) {
                return Err(TableError::EmptyKey);
            }
            if let Some((title, parent, pin, last)) = groups.get_mut(&g.key) {
                if *title != g.title || *parent != path || *last + 1 != ix {
                    return Err(TableError::UnsupportedCombination(
                        "header groups must be contiguous with consistent titles and ancestry",
                    ));
                }
                *last = ix;
                *pin = c.pin;
            } else {
                groups.insert(g.key.clone(), (g.title.clone(), path.clone(), c.pin, ix));
            }
            path.push(g.key.clone());
        }
    }
    Ok(())
}
