use super::*;
use crate::{InputState, SelectEvent, SelectMode, SelectOption, SelectValue, YueSelectState};
use gpui_kit::base::input::InputEvent;
use gpui_kit::{AppContext as _, Context, Entity, EventEmitter, FocusHandle, Subscription, Window};
use std::collections::BTreeMap;

/// One retained controller per mounted Pagination. Hosts accept requests explicitly.
pub struct YuePaginationState {
    pub(super) value: PaginationValue,
    disabled: bool,
    loading: bool,
    pub(super) input: Entity<InputState>,
    pub(super) select: Entity<YueSelectState>,
    sizes: Vec<usize>,
    policy: PaginationSizeChangePolicy,
    input_visible: bool,
    select_visible: bool,
    pub(super) invalid: bool,
    pub(super) page_focus: BTreeMap<usize, FocusHandle>,
    pub(super) navigation_focus: [FocusHandle; 6],
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<PaginationChange> for YuePaginationState {}
impl YuePaginationState {
    pub fn new(value: PaginationValue, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input =
            cx.new(|cx| InputState::new(window, cx).default_value(value.page().to_string()));
        let select = cx.new(|cx| YueSelectState::new(SelectMode::Single, window, cx));
        let subscriptions = vec![
            cx.subscribe_in(&input, window, |s, _, event, w, cx| match event {
                InputEvent::PressEnter {
                    secondary: false, ..
                } if !s.blocked() && s.input_visible => s.submit(w, cx),
                InputEvent::Blur => s.reset_draft(w, cx),
                InputEvent::Change => {
                    s.invalid = false;
                    cx.notify();
                }
                _ => {}
            }),
            cx.subscribe_in(&select, window, |s, _, event, w, cx| {
                if let SelectEvent::Change(change) = event {
                    let size = change
                        .after()
                        .keys()
                        .first()
                        .and_then(|key| model::parse_page(key));
                    // Select normally commits its own value. Restore the host-owned value
                    // before notifying, including when the host declines the request.
                    s.sync_select_value(w, cx);
                    if let Some(size) = size.filter(|size| s.sizes.contains(size))
                        && !s.blocked()
                        && s.select_visible
                    {
                        s.request(
                            s.value.resized(size, s.policy),
                            PaginationChangeSource::PageSize,
                            cx,
                        );
                    }
                }
            }),
        ];
        let mut s = Self {
            value,
            disabled: false,
            loading: false,
            input,
            select,
            sizes: vec![10, 20, 50, 100],
            policy: PaginationSizeChangePolicy::Reset,
            input_visible: false,
            select_visible: false,
            invalid: false,
            page_focus: BTreeMap::new(),
            navigation_focus: std::array::from_fn(|_| cx.focus_handle()),
            _subscriptions: subscriptions,
        };
        s.sync_options(window, cx);
        s
    }
    pub fn value(&self) -> PaginationValue {
        self.value
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_loading(&self) -> bool {
        self.loading
    }
    pub(super) fn blocked(&self) -> bool {
        self.disabled || self.loading
    }
    /// External updates are silent and also cancel any uncommitted draft.
    pub fn set_value(&mut self, value: PaginationValue, w: &mut Window, cx: &mut Context<Self>) {
        let size_changed = self.value.page_size() != value.page_size();
        self.value = value;
        self.reset_draft(w, cx);
        if size_changed {
            self.sync_options(w, cx);
        } else {
            self.sync_select_value(w, cx);
        }
        cx.notify();
    }
    pub fn set_disabled(&mut self, value: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == value {
            return;
        }
        self.disabled = value;
        self.sync_disabled(w, cx);
    }
    pub fn set_loading(&mut self, value: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.loading == value {
            return;
        }
        self.loading = value;
        self.sync_disabled(w, cx);
    }
    pub fn set_size_change_policy(&mut self, policy: PaginationSizeChangePolicy) {
        self.policy = policy;
    }
    pub fn set_page_sizes(
        &mut self,
        sizes: impl IntoIterator<Item = usize>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), PaginationError> {
        let sizes = sizes.into_iter().take(33).collect::<Vec<_>>();
        if sizes.is_empty() {
            return Err(PaginationError::EmptyPageSizes);
        }
        if sizes.len() > 32 {
            return Err(PaginationError::TooManyPageSizes);
        }
        for (ix, size) in sizes.iter().enumerate() {
            if *size == 0 {
                return Err(PaginationError::ZeroPageSize);
            }
            if sizes[..ix].contains(size) {
                return Err(PaginationError::DuplicatePageSize);
            }
        }
        self.sizes = sizes;
        self.sync_options(w, cx);
        cx.notify();
        Ok(())
    }
    fn sync_options(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let mut sizes = self.sizes.clone();
        if !sizes.contains(&self.value.page_size()) {
            sizes.push(self.value.page_size());
        }
        sizes.sort_unstable();
        self.select.update(cx, |s, cx| {
            s.set_options(
                sizes
                    .into_iter()
                    .map(|n| SelectOption::new(n.to_string(), format!("{n} 条/页"))),
                cx,
            )
            .unwrap()
        });
        self.sync_select_value(w, cx);
    }
    fn sync_select_value(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let value = SelectValue::Single(Some(self.value.page_size().to_string().into()));
        self.select.update(cx, |s, cx| {
            s.set_open(false, w, cx);
            s.set_value(value, w, cx).unwrap();
        });
    }
    fn sync_disabled(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let disabled = self.blocked();
        self.select
            .update(cx, |s, cx| s.set_disabled(disabled, w, cx));
        self.input.update(cx, |s, cx| s.set_disabled(disabled, cx));
        self.reset_draft(w, cx);
        cx.notify();
    }
    pub(super) fn reset_draft(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.invalid = false;
        let text = self.value.page().to_string();
        if self.input.read(cx).value().as_ref() != text {
            self.input.update(cx, |s, cx| s.set_value(text, w, cx));
        }
        cx.notify();
    }
    fn submit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let page = model::parse_page(&self.input.read(cx).value());
        if let Some(page) = page {
            self.request_page(page, PaginationChangeSource::JumpInput, w, cx);
            self.reset_draft(w, cx);
        } else {
            self.invalid = true;
            cx.notify();
        }
    }
    pub(super) fn request_page(
        &mut self,
        page: usize,
        source: PaginationChangeSource,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Base is the shared authority for same-page, bounds and disabled guards.
        let target = self.value.at(page);
        let request = PaginationChange {
            before: self.value,
            value: target,
            source,
        };
        let allowed = std::rc::Rc::new(std::cell::Cell::new(false));
        let flag = allowed.clone();
        gpui_kit::base::PaginationState::new(self.value.page(), self.value.page_count())
            .disabled(self.blocked())
            .on_change(move |_, _, _| flag.set(true))
            .request_page(target.page(), w, cx);
        if allowed.get() {
            cx.emit(request);
        }
    }
    fn request(
        &mut self,
        value: PaginationValue,
        source: PaginationChangeSource,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() || value == self.value {
            return;
        }
        cx.emit(PaginationChange {
            before: self.value,
            value,
            source,
        });
    }
    pub(super) fn focus_pages(
        &mut self,
        pages: &[usize],
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> BTreeMap<usize, FocusHandle> {
        let lost = self
            .page_focus
            .iter()
            .any(|(p, f)| !pages.contains(p) && f.is_focused(w));
        self.page_focus.retain(|p, _| pages.contains(p));
        for page in pages {
            self.page_focus
                .entry(*page)
                .or_insert_with(|| cx.focus_handle());
        }
        if lost && !self.blocked() {
            self.restore_focus(w, cx);
        }
        self.page_focus.clone()
    }
    fn restore_focus(&self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.page_focus.get(&self.value.page()) {
            f.focus(w, cx);
        } else if self.value.page() > 1 {
            self.navigation_focus[0].focus(w, cx);
        } else if self.value.page_count() > 1 {
            self.navigation_focus[1].focus(w, cx);
        } else {
            w.blur(cx);
        }
    }
    pub(super) fn configure_visibility(
        &mut self,
        input: bool,
        selector: bool,
        hidden: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input_lost = !input
            && self
                .input
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(w);
        let select_lost = !selector
            && (self.select.read(cx).focus_handle().is_focused(w)
                || self.select.read(cx).is_open());
        let buttons_lost = hidden
            && (self.navigation_focus.iter().any(|f| f.is_focused(w))
                || self.page_focus.values().any(|f| f.is_focused(w)));
        if self.input_visible && !input {
            self.reset_draft(w, cx);
        }
        if self.select_visible && !selector {
            self.select.update(cx, |s, cx| s.set_open(false, w, cx));
        }
        self.input_visible = input;
        self.select_visible = selector;
        if input_lost || select_lost || buttons_lost {
            if hidden || self.blocked() {
                w.blur(cx);
            } else {
                self.restore_focus(w, cx);
            }
        }
    }
}
