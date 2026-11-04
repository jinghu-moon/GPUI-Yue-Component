use super::*;

/// Pure values for one stable-lane slot, including unmeasured size hints.
#[derive(Clone, Debug)]
pub struct VirtualMasonryCacheItem {
    measurement: VirtualizerMeasurement,
    lane: usize,
    measured: bool,
}
impl VirtualMasonryCacheItem {
    pub fn new(
        key: impl Into<ElementId>,
        content_revision: u64,
        extent: Pixels,
        lane: usize,
        measured: bool,
    ) -> Self {
        Self {
            measurement: VirtualizerMeasurement::new(key, content_revision, extent),
            lane,
            measured,
        }
    }
    pub fn key(&self) -> &ElementId {
        self.measurement.key()
    }
    pub fn content_revision(&self) -> u64 {
        self.measurement.content_revision()
    }
    pub fn extent(&self) -> Pixels {
        self.measurement.extent()
    }
    pub fn lane(&self) -> usize {
        self.lane
    }
    pub fn is_measured(&self) -> bool {
        self.measured
    }
}

/// Cold-remount values, not a live snapshot. Export is O(N); restore is O(N log(lanes)) in metadata
/// operations. No source, entity, view, owner token or scroll handle is retained.
/// Applications choose persistence/serialization and discard stale caches.
#[derive(Clone, Debug)]
pub struct VirtualMasonryCache {
    version: u32,
    layout_revision: u64,
    lane_count: usize,
    cross: Pixels,
    gap: Pixels,
    column_gap: Pixels,
    items: Vec<VirtualMasonryCacheItem>,
    anchor: Option<(ElementId, VirtualizerAnchorPosition)>,
}
impl VirtualMasonryCache {
    pub fn new(
        layout_revision: u64,
        lane_count: usize,
        cross: Pixels,
        gap: Pixels,
        column_gap: Pixels,
        items: Vec<VirtualMasonryCacheItem>,
    ) -> Self {
        Self {
            version: 1,
            layout_revision,
            lane_count,
            cross,
            gap,
            column_gap,
            items,
            anchor: None,
        }
    }
    pub fn anchor(
        mut self,
        key: impl Into<ElementId>,
        position: VirtualizerAnchorPosition,
    ) -> Self {
        self.anchor = Some((key.into(), position));
        self
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn layout_revision(&self) -> u64 {
        self.layout_revision
    }
    pub fn lane_count(&self) -> usize {
        self.lane_count
    }
    pub fn cross_extent(&self) -> Pixels {
        self.cross
    }
    pub fn gap(&self) -> Pixels {
        self.gap
    }
    pub fn column_gap(&self) -> Pixels {
        self.column_gap
    }
    pub fn items(&self) -> &[VirtualMasonryCacheItem] {
        &self.items
    }
    pub fn scroll_anchor(&self) -> Option<(&ElementId, VirtualizerAnchorPosition)> {
        self.anchor.as_ref().map(|(k, p)| (k, *p))
    }
}

impl VirtualMasonryState {
    pub fn measurement_cache(&self) -> Result<VirtualMasonryCache, VirtualizerError> {
        if !self.dynamic {
            return Err(VirtualizerError::WrongBackend);
        }
        let cross =
            self.cross
                .filter(|c| valid_positive(*c))
                .ok_or(VirtualizerError::InvalidGeometry(
                    "cache requires resolved column width",
                ))?;
        let extents = self.source_extents();
        let items = extents
            .into_iter()
            .enumerate()
            .map(|(ix, (extent, measured))| {
                VirtualMasonryCacheItem::new(
                    self.source.key_at(ix),
                    self.source.revision_at(ix),
                    extent,
                    self.slots[ix].0,
                    measured,
                )
            })
            .collect();
        let mut cache = VirtualMasonryCache::new(
            self.layout.revision,
            self.lanes.len(),
            cross,
            self.layout.gap,
            self.column_gap,
            items,
        );
        cache.anchor = self
            .blank_region_anchor((-self.scroll.precise_offset().y).max(0.))
            .map(|(k, p, _)| (k, p));
        Ok(cache)
    }
    /// Validate the entire value set before mutation. Deleted keys are skipped;
    /// changed content keeps its lane but uses current unmeasured hints. New keys
    /// join the shortest lane in current source order. Existing lane membership
    /// is explicit, so a historical stable-lane layout survives reconstruction.
    /// A different actual column width invalidates the restored measurements on
    /// first layout, rather than relabelling them with the new width.
    pub fn restore_measurement_cache(
        &mut self,
        cache: &VirtualMasonryCache,
    ) -> Result<usize, VirtualizerError> {
        if !self.dynamic {
            return Err(VirtualizerError::WrongBackend);
        }
        if cache.version != 1
            || cache.layout_revision != self.layout.revision
            || cache.lane_count != self.lanes.len()
            || cache.gap != self.layout.gap
            || cache.column_gap != self.column_gap
            || !valid_positive(cache.cross)
            || self.cross.is_some_and(|c| c != cache.cross)
            || cache.anchor.as_ref().is_some_and(|(_, p)| !p.is_valid())
        {
            return Err(VirtualizerError::InvalidGeometry(
                "incompatible masonry cache",
            ));
        }
        let mut entries = std::collections::HashMap::with_capacity(cache.items.len());
        for item in &cache.items {
            if item.lane >= cache.lane_count
                || !valid_positive(item.extent())
                || !valid_positive(item.extent() + cache.gap)
                || entries.insert(item.key(), item).is_some()
            {
                return Err(VirtualizerError::InvalidGeometry("masonry cache entry"));
            }
        }
        let current = self.source_extents();
        let mut indexes = vec![IndexVector::default(); self.lanes.len()];
        let mut sizes = vec![Vec::new(); self.lanes.len()];
        let mut ends = vec![0.0f64; self.lanes.len()];
        let mut shortest: std::collections::BTreeSet<_> = (0..self.lanes.len())
            .map(|lane| LaneEnd(0., lane))
            .collect();
        let mut slots = IndexVector::default();
        let mut count = 0;
        let mut seen = HashSet::new();
        for (ix, (hint, _)) in current.into_iter().enumerate() {
            let key = self.source.key_at(ix);
            if self.source.index_of(&key) != Some(ix) || !seen.insert(key.clone()) {
                return Err(VirtualizerError::InvalidSource);
            }
            let old = entries.get(&key);
            let lane = old.map_or_else(|| shortest.last().unwrap().1, |item| item.lane);
            let (extent, measured) = old
                .filter(|item| item.content_revision() == self.source.revision_at(ix))
                .map_or((hint, false), |item| (item.extent(), item.is_measured()));
            count += usize::from(measured);
            slots.push((lane, indexes[lane].len()));
            indexes[lane].push(ix);
            sizes[lane].push((extent, measured));
            shortest.remove(&LaneEnd(ends[lane], lane));
            ends[lane] += f64::from(extent + self.layout.gap);
            shortest.insert(LaneEnd(ends[lane], lane));
        }
        let lanes = indexes
            .into_iter()
            .zip(sizes)
            .map(|(indexes, sizes)| Lane {
                indexes,
                geometry: geometry::KnownGeometry::with_measurements(sizes, self.layout.gap.into()),
            })
            .collect();
        self.cancel_scroll();
        self.lanes = lanes;
        self.slots = slots;
        self.cross = Some(cache.cross);
        self.revision += 1;
        self.anchor = cache.anchor.clone().and_then(|(key, p)| {
            self.source.index_of(&key).map(|ix| {
                let (lane, local) = self.slots[ix];
                let measured = self.lanes[lane].geometry.item(local).unwrap().2;
                (key, p, measured)
            })
        });
        self.restore_anchor();
        Ok(count)
    }
}
