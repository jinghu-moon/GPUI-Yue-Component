use super::*;
use gpui_kit::Context;
use std::collections::{BTreeMap, HashSet};
/// Persisted host preference. Width is an unclamped preference, never a measured
/// or temporarily responsive width. None inherits the current declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumnPreference {
    version: u64,
    visible: Option<bool>,
    pin: Option<TablePin>,
    width: Option<Rems>,
}
impl TableColumnPreference {
    pub fn new(semantic_version: u64) -> Self {
        Self {
            version: semantic_version,
            visible: None,
            pin: None,
            width: None,
        }
    }
    pub fn semantic_version(&self) -> u64 {
        self.version
    }
    pub fn visible(&self) -> Option<bool> {
        self.visible
    }
    pub fn pin(&self) -> Option<TablePin> {
        self.pin
    }
    pub fn width(&self) -> Option<Rems> {
        self.width
    }
    pub fn with_visible(mut self, value: bool) -> Self {
        self.visible = Some(value);
        self
    }
    pub fn with_pin(mut self, value: TablePin) -> Self {
        self.pin = Some(value);
        self
    }
    pub fn with_width(mut self, value: Rems) -> Self {
        self.width = Some(value);
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumnPreferences {
    schema: u64,
    order: Vec<TableColumnKey>,
    values: BTreeMap<TableColumnKey, TableColumnPreference>,
}
impl TableColumnPreferences {
    pub fn new(schema: u64) -> Self {
        Self {
            schema,
            order: Vec::new(),
            values: BTreeMap::new(),
        }
    }
    pub fn schema(&self) -> u64 {
        self.schema
    }
    pub fn order(&self) -> &[TableColumnKey] {
        &self.order
    }
    pub fn values(&self) -> &BTreeMap<TableColumnKey, TableColumnPreference> {
        &self.values
    }
    pub fn with_order(mut self, order: Vec<TableColumnKey>) -> Self {
        self.order = order;
        self
    }
    pub fn with_column(mut self, key: TableColumnKey, value: TableColumnPreference) -> Self {
        self.values.insert(key, value);
        self
    }
    /// Migrate saved settings by stable key and semantic version. Removed,
    /// unauthorized and changed-meaning columns lose their old settings.
    pub fn reconcile<S: TableSource>(&self, view: &TableView<S>) -> Self {
        let valid = |k: &TableColumnKey| {
            view.declarations.iter().find(|c| {
                c.key() == k
                    && c.allowed
                    && c.configurable
                    && self.values.get(k).is_none_or(|v| v.version == c.version)
            })
        };
        let mut values: BTreeMap<_, _> = self
            .values
            .iter()
            .filter(|(k, _)| valid(k).is_some())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut seen = HashSet::new();
        let mut order: Vec<_> = self
            .order
            .iter()
            .filter(|k| valid(k).is_some() && seen.insert((*k).clone()))
            .cloned()
            .collect();
        order.extend(
            view.declarations
                .iter()
                .filter(|c| seen.insert(c.key().clone()))
                .map(|c| c.key().clone()),
        );
        // Protected declarations keep their slots; saved order only fills movable slots.
        let movable: Vec<_> = order
            .iter()
            .filter(|k| {
                view.declarations
                    .iter()
                    .any(|c| c.key() == *k && c.allowed && c.configurable)
            })
            .cloned()
            .collect();
        let mut movable = movable.into_iter();
        order = view
            .declarations
            .iter()
            .map(|c| {
                if c.allowed && c.configurable {
                    movable.next().unwrap()
                } else {
                    c.key().clone()
                }
            })
            .collect();
        for c in view
            .declarations
            .iter()
            .filter(|c| c.allowed && c.configurable)
        {
            values
                .entry(c.key().clone())
                .or_insert_with(|| TableColumnPreference::new(c.version));
        }
        Self {
            schema: view.schema,
            order,
            values,
        }
    }
    pub(super) fn resolve<S: TableSource>(&self, view: &TableView<S>) -> Rc<[TableColumn<S::Row>]> {
        let mut cols = Vec::with_capacity(view.declarations.len());
        let mut seen = HashSet::new();
        for key in self
            .order
            .iter()
            .chain(view.declarations.iter().map(|c| c.key()))
        {
            if !seen.insert(key) {
                continue;
            }
            let Some(declared) = view.declarations.iter().find(|c| c.key() == key) else {
                continue;
            };
            let mut c = declared.clone();
            if c.allowed
                && c.configurable
                && let Some(p) = self.values.get(key).filter(|p| p.version == c.version)
            {
                if let Some(v) = p.visible {
                    c.visible = v;
                }
                if let Some(pin) = p.pin {
                    c.pin = pin;
                }
                if let Some(w) = p.width {
                    c.width = w;
                    c.flex = 0.;
                }
            }
            c.visible &= c.allowed;
            cols.push(c);
        }
        cols.into()
    }
    pub fn apply_to<S: TableSource>(
        &self,
        view: &TableView<S>,
    ) -> Result<TableView<S>, TableError> {
        if self.schema != view.schema {
            return Err(TableError::StaleView);
        }
        let mut seen = HashSet::new();
        for key in &self.order {
            if !seen.insert(key) || !view.declarations.iter().any(|c| c.key() == key) {
                return Err(TableError::MissingKey);
            }
        }
        let effective: Vec<_> = self
            .order
            .iter()
            .chain(view.declarations.iter().map(|c| c.key()))
            .scan(HashSet::new(), |seen, k| Some(seen.insert(k).then_some(k)))
            .flatten()
            .collect();
        for (index, c) in view.declarations.iter().enumerate() {
            if (!c.allowed || !c.configurable) && effective.get(index).copied() != Some(c.key()) {
                return Err(TableError::UnsupportedOperation(
                    "protected column order cannot change",
                ));
            }
        }
        for (key, p) in &self.values {
            let c = view
                .declarations
                .iter()
                .find(|c| c.key() == key)
                .ok_or(TableError::MissingKey)?;
            if c.version != p.version {
                return Err(TableError::StaleView);
            }
            if !c.allowed || !c.configurable {
                return Err(TableError::UnsupportedOperation(
                    "column configuration is not permitted",
                ));
            }
            if p.width.is_some_and(|w| !w.0.is_finite() || w.0 <= 0.) {
                return Err(TableError::InvalidGeometry);
            }
        }
        let mut next = view.clone();
        next.columns = self.resolve(view);
        let mut snapshot = self.clone();
        for c in view
            .declarations
            .iter()
            .filter(|c| c.allowed && c.configurable)
        {
            snapshot
                .values
                .entry(c.key().clone())
                .or_insert_with(|| TableColumnPreference::new(c.version));
        }
        next.preferences = Rc::new(snapshot);
        next.validate(false)?;
        Ok(next)
    }
}
/// Dialog-local draft. No committed Table changes until one confirmation request.
#[derive(Clone)]
pub struct TableColumnDraft {
    scope: SharedString,
    schema: u64,
    preferences: TableColumnPreferences,
    permissions: BTreeMap<TableColumnKey, (u64, bool)>,
    fixed_order: Vec<Option<TableColumnKey>>,
}
impl TableColumnDraft {
    pub fn new<S: TableSource>(view: &TableView<S>) -> Self {
        let mut preferences = (*view.preferences).clone();
        preferences.order = view.columns.iter().map(|c| c.key().clone()).collect();
        Self {
            scope: view.source.scope().clone(),
            schema: view.schema,
            preferences,
            fixed_order: view
                .declarations
                .iter()
                .map(|c| (!c.allowed || !c.configurable).then(|| c.key().clone()))
                .collect(),
            permissions: view
                .declarations
                .iter()
                .map(|c| (c.key().clone(), (c.version, c.allowed && c.configurable)))
                .collect(),
        }
    }
    pub fn preferences(&self) -> &TableColumnPreferences {
        &self.preferences
    }
    fn entry(&mut self, key: &TableColumnKey) -> Result<&mut TableColumnPreference, TableError> {
        let (version, allowed) = self
            .permissions
            .get(key)
            .copied()
            .ok_or(TableError::MissingKey)?;
        if !allowed {
            return Err(TableError::UnsupportedOperation(
                "column configuration is not permitted",
            ));
        }
        Ok(self
            .preferences
            .values
            .entry(key.clone())
            .or_insert_with(|| TableColumnPreference::new(version)))
    }
    pub fn set_visible(&mut self, key: &TableColumnKey, v: bool) -> Result<(), TableError> {
        self.entry(key)?.visible = Some(v);
        Ok(())
    }
    pub fn set_pin(&mut self, key: &TableColumnKey, pin: TablePin) -> Result<(), TableError> {
        self.entry(key)?.pin = Some(pin);
        Ok(())
    }
    pub fn set_width(&mut self, key: &TableColumnKey, width: Rems) -> Result<(), TableError> {
        if !width.0.is_finite() || width.0 <= 0. {
            return Err(TableError::InvalidGeometry);
        }
        self.entry(key)?.width = Some(width);
        Ok(())
    }
    pub fn set_order(&mut self, order: Vec<TableColumnKey>) -> Result<(), TableError> {
        let keys: HashSet<_> = order.iter().collect();
        if order.len() != self.permissions.len()
            || keys.len() != order.len()
            || order.iter().any(|k| !self.permissions.contains_key(k))
        {
            return Err(TableError::MissingKey);
        }
        if self
            .fixed_order
            .iter()
            .enumerate()
            .any(|(i, k)| k.as_ref().is_some_and(|k| order.get(i) != Some(k)))
        {
            return Err(TableError::UnsupportedOperation(
                "protected column order cannot change",
            ));
        }
        self.preferences.order = order;
        Ok(())
    }
    pub fn reset(&mut self) {
        self.preferences = TableColumnPreferences::new(self.schema);
    }
    pub fn validate_for<S: TableSource>(
        &self,
        view: &TableView<S>,
    ) -> Result<TableView<S>, TableError> {
        if self.scope != *view.source.scope() || self.schema != view.schema {
            return Err(TableError::StaleView);
        }
        if self.permissions.len() != view.declarations.len() {
            return Err(TableError::StaleView);
        }
        for (key, (version, allowed)) in &self.permissions {
            let c = view
                .declarations
                .iter()
                .find(|c| c.key() == key)
                .ok_or(TableError::StaleView)?;
            if c.version != *version || *allowed != (c.allowed && c.configurable) {
                return Err(TableError::StaleView);
            }
        }
        self.preferences.apply_to(view)
    }
}
#[derive(Clone, Debug)]
pub struct TableColumnConfigurationRequest {
    id: TableRequestId,
    scope: SharedString,
    revision: u64,
    preferences: TableColumnPreferences,
}
impl TableColumnConfigurationRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn preferences(&self) -> &TableColumnPreferences {
        &self.preferences
    }
    pub fn apply_to<S: TableSource>(
        &self,
        view: &TableView<S>,
    ) -> Result<TableView<S>, TableError> {
        if self.scope != *view.source.scope() || self.revision != view.revision() {
            return Err(TableError::StaleRequest);
        }
        self.preferences.apply_to(view)
    }
}
impl<D: TableDelegate> YueTableState<D> {
    pub fn request_column_configuration(
        &mut self,
        draft: &TableColumnDraft,
        cx: &mut Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        let view = self.view().ok_or(TableError::Unbound)?;
        draft.validate_for(view)?;
        if self.edit.is_some()
            || self
                .presentation
                .as_ref()
                .is_some_and(|p| p.disabled || p.readonly)
        {
            return Err(TableError::UnsupportedOperation(
                "column configuration is unavailable",
            ));
        }
        if self.requests.len() >= 64 {
            return Err(TableError::TooManyPendingRequests);
        }
        let scope = view.source.scope().clone();
        let revision = view.revision();
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        self.requests.insert(id);
        self.pending = Some(id);
        cx.emit(TableEvent::ColumnConfigurationRequested(
            TableColumnConfigurationRequest {
                id,
                scope,
                revision,
                preferences: draft.preferences.clone(),
            },
        ));
        Ok(id)
    }
}
