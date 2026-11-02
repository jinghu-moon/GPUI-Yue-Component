# Yue Base patches

## Controlled accordion and safe measured disclosure (2026-11-02)

`accordion/controller.rs` adds explicit descriptors/snapshots, Multiple/Accordion
projection, atomic read-only change requests, keyed trigger/panel focus handles,
per-heading Tab stops, Up/Down/Home/End navigation and focus repair before old
panels are detached. The owner alone commits expanded keys. Change handlers run
outside controller updates so a host may synchronously accept a request.

`AccordionTrigger::in_group` opts into that controller. Standalone triggers keep
their bool/ClickEvent callback and now compose Base Button for native focus,
disabled semantics and single Enter/Space clicks. Physical keydown records the
source; activation remains Button's native key-release click, avoiding a second
toggle. A keyboard-shaped native activation without a physical keydown reports
Accessibility. Mouse and touch report Pointer. Grouped triggers use only the
controller's handler, with ambiguous standalone/group callbacks rejected.

`motion/disclosure.rs` adds opt-in `MotionDisclosure`. It measures the normal-flow,
nonshrinking child at the actual width, uses an interruptible pixel timeline and
shares Base Transition easing/reversal policy. Initially open adopts natural
height; a new expansion starts at zero with no fully shown measurement frame.
Closing requires `None` content immediately and animates only retained geometry.
Stable expanded content uses auto height; dynamic targets retarget current pixels,
while width/geometry/reduced-motion/visibility changes settle. Only visible active
motion requests frames. Stable content is not unnecessarily clipped. Existing
MotionReveal, Collapsible and standalone Accordion mounting behavior are retained.

Validation: production headless Collapse tests cover input, focus, clipping,
geometry, retained input and popup-anchor disposal. The Base accordion fixture
also verifies native structural roles and reentrant accessibility requests.

## Toast foundation extensions (2026-11-02)

Yue uses the existing Base 0.8.0 ToastManager and ToastStack rather than a second
lifecycle/layout implementation. `src/toast.rs` adds independent entry/exit
deadlines, in-place update and timeout reset, remaining/total reading snapshots,
transition sampling and the nearest effective deadline. Late wakes consume time
across phases; only Present reading time pauses. Existing same-id push and the
default natural-height collapse policy remain available.

ToastStack adds opt-in front-card shells, center-front anchoring, external natural
measurement and controlled expansion/sample time. Natural measurements stay
separate from clipped presentation heights. Centering offsets items and hover
bounds, preserving caller `bottom_0()` positioning. Yue supplies only active
items to rank/layout and paints frozen Ending snapshots independently. The Base
defaults retain their existing behavior; `optional_front_shell` and the original
tall-behind/bottom-anchor regressions cover both policies.

`src/motion.rs` exports `spring_at` and `transition_at` for an optional local
diagnostic clock and frame policy. Existing APIs delegate with executor time and
normal scheduling. `src/modal.rs` publishes window-scoped `ModalScopeChanges`
only when the active scope changes, including a monotonic timestamp for precise
pause boundaries; released windows remove their entries.

`src/a11y_live.rs` provides the forwarding `A11yLive` adapter for live, hidden,
atomic and actual semantic-write notification. It adds no layout, focus or input
node. Toast roots can choose their native role/live/hidden policy. Yue uses
non-live visual cards and separately keyed native Status/Alert events with both
label and value; semantic delivery and modal restoration are tested independently
from visual opacity. These hooks do not prove real OS screen-reader output.

Validation: Yue Toast headless regressions and Showcase tests, plus Base's
`toast::tests`, `a11y_live::tests`, `modal::tests` and `motion::` tests. Run
`pwsh -File tools/test-gpui-base.ps1 -Offline -Filter toast::tests` (change Filter
for another module). The script creates a disposable manifest/lock under target,
using the actual vendored source and the workspace's production pins, and removes
the published manifest's unconditional Fast backend development dependencies.
It never edits Cargo registry sources, the vendor manifest or the root lockfile.
Yue owns all colors, spacing, timings, eight-position policy and reading defaults.

## Dialog foundation extensions (2026-11-01)

`src/dialog.rs` adds opt-in controlled `DialogIntent` routing, independent default
submission, configurable deferred priority and popup-owned semantic role. The
controlled path uses bubbling fallback keys so a menu, text editor or button
consumes its own input first. Backdrop dismissal requires a complete primary
press/release; an exiting topmost surface can retain its input barrier while
disabling descendant activation. Existing legacy Dialog callbacks remain intact.

`src/modal.rs` adds window-local active/building `ModalScope`, a forwarding
`ModalBoundary` for native modal/busy/background-hidden semantics, and weak,
live-editability-checked editor candidates. Input state registers its own focus
while rendered in a declaring modal scope; widget traversal and pointer position
do not determine ownership. `Root` exposes trap-aware traversal and checks for a
retained handle's live Root dispatch path. Active modal focus takes precedence
over incidental background focus. Window release drops these weak registrations.
These are unstyled mechanisms, not a global dialog service.

`src/constrained_drag.rs` provides an explicit non-focusable handle, measured
viewport clamping, a pointer threshold, capture move/release and cancellation.
Consumers choose their own draggable region, spacing and animation. Yue uses it
only for an enabled title region; interactive custom headers require a separate
handle. Regression tests cover Dialog input, drag bounds, layer dismissal, focus,
lifecycle, asynchronous tickets and Popup integration. Native node-hook tests do
not stand in for real Windows UIA/screen-reader verification.

Source: crates.io `gpui-base 0.8.0`, upstream `longbridge/gpui-kit`, commit
`1676826d9d49b90f4d7b6808b69924f58796f6f1`, directory `crates/base`.
The distributed Apache-2.0 license, manifest, sources, tests and benches are
retained. Cargo's registry copy is never modified.

The Popup extension changes `src/positioner.rs`:

- Read an optional anchor source during prepaint, after the trigger is measured.
- Expose the existing resolver and decorated viewport bounds for bounded probes;
  support a fixed side without flipping, and include the offset in fit tests.
- Allow a position filter to reject an unsafe frame or provide sampled geometry
  before both hitboxes and child prepaint. Rejected surfaces are not painted.
- Preserve fractional element offsets; Yue snaps the final sample with GPUI's
  physical-pixel helpers before publishing geometry and inserting hitboxes.

## Slider and numeric input (2026-11-01)

`src/slider.rs` and `src/slider/domain.rs` now provide the shared behavior for
YueSlider, ColorPicker and the Layout example:

- `SliderDomain` validates finite ordered bounds and a positive, representable
  step. The step grid starts at min and includes max even for a short final
  interval. Decimal-grid arithmetic uses shortest round-trip f32 decimals.
- Programmatic Base assignments clamp/order and preserve sub-step precision
  (required by ColorPicker); user edits snap to the grid. Percentages come from
  the final value and same-value edits do not emit Change.
- `Slider::snap_points(points, distance)` configures local pointer magnets in
  logical pixels. Each radius is capped at a quarter of the nearest mark gap;
  outside it the normal step grid applies immediately. Exact marks retain their
  off-grid value. Keyboard, accessible actions and number steppers keep the
  domain step, which remains exposed on native Slider/SpinButton nodes. Policy
  changes cancel capture without moving the current value. Same-value Yue owner
  echoes remain no-ops even after removing a mark. No mark policy is installed
  in ColorPicker, whose exact programmatic values remain unchanged.
- Mouse capture uses window MouseMove/MouseUp handlers, without a DnD threshold.
  Thumb-edge grabs preserve the pointer offset. Range-track presses choose the
  nearest endpoint, with the last active endpoint resolving ties; initial ties
  choose upper. Endpoints do not cross and capture continues outside the track.
- Start/Change/Release/Cancel distinguish real edit lifecycles. Cancelled touch,
  disable and window deactivation do not commit. Public `cancel_interaction`
  silently clears the old gesture without altering another control's drag.
- Arrow, Home, End, PageUp/PageDown and held-key completion share the domain.
  `thumb_semantics(true)` places stable focus handles and native Slider roles on
  each endpoint, including orientation, value, dynamic bounds, step, description
  and accessible increment/decrement. Caller focus handles receive tab policy.
  The default root semantic mode remains usable by existing Base consumers.
- Start does not notify observers before applying the same operation's value;
  this prevents ColorPicker mistaking its own user edit for an external setter.

Yue owns presentation, numeric drafts and generation-checked Change/Commit
transactions. These are not extra Base numeric values. Regression coverage:
`crates/yue-component/tests/slider.rs`, `tests/color_picker.rs`, and Base's
retained slider unit tests. Read-only `is_dragging` remains available for visual
feedback; thumb-only presses emit no Change.

When presentation animates a thumb independently of the canonical value,
pointer capture now uses its actual last-laid-out center and preserves the
grab offset. A press on a displaced thumb takes over that visible coordinate
through the existing domain/step path instead of jumping to the old destination.
Touch capture and nearest-thumb selection likewise use the presented geometry.
Ordinary stationary thumb presses remain silent. Yue alone decides whether and
how to animate; the Base mechanism adds no timing or visual policy. Yue's
headless Slider regressions cover mouse and vertical touch takeover.

`src/number_input.rs` adds a description slot and writes finite numeric
value/min/max/step to its native SpinButton.
Accessible increment/decrement uses the same step callback as pointer controls.
`NumberInput::controls(false)` omits the pointer buttons while retaining the
editor, SpinButton semantics, Up/Down and accessible increment/decrement actions.
`tab_stop(false)` applies to the editor's focus handle; disabled still wins.
Slider uses these for the arrow-free field beside end buttons. Real pointer,
keyboard, traversal and invalid-draft regressions live in Yue's Slider tests.
`src/input/base/mod.rs` forwards disabled to the actual native input node through
the existing adapter. Yue's NumberInput now supplies the actual Base Input
editor through `.input(...)`, and adds right-side controls and `on_step`;
Input and Slider integration tests cover real edits, disabled input and numeric
semantics.

`src/test_support.rs` additionally observes existing native description, numeric
value/bounds/step and orientation attributes. It does not inject replacement
semantics or prove Windows UIA/Narrator output.

With `test-support`, Input's last layout also exposes `rendered_text_color()`:
the resolved default color passed to the real TextRun shaping input. This is a
read-only observation, not a second color policy, GPU pixel readback, or a
syntax-highlight override. Yue's numeric Error/Normal regression uses it in all
four theme profiles to catch inherited text color regressions.

`src/motion/presence.rs` retains the previous leg's timing policy when sampling
an interrupted transition. Enter and exit may have different easing and duration;
the old leg must be sampled with its own policy before adopting the new one,
otherwise an exit-to-enter reversal jumps in opacity and position. This is a
mechanism fix, with no default visual policy. Yue Tooltip's twelve-placement
reentry regression exercises asymmetric enter/exit policies through the real
Popup host, checking the exact sampled geometry on reversal.

Yue uses these hooks without copying the resolver. Replace this patch with an
upstream release once equivalent public APIs exist; run the popup and dropdown
regressions before removing it. No styled Component or second GPUI backend is
enabled by this patch.

Cargo applies patches from the consuming workspace root only. A downstream
workspace using Yue as a path dependency must add its own `[patch.crates-io]`
entry pointing to this directory until the required APIs are released upstream.

`src/link.rs` accepts an explicit caller-owned `track_focus` handle, with the
same tab_index/tab_stop policy as its keyed default. Its actual semantic div
also uses Base's existing optional test observation, as Button already does.
YueLink therefore paints its focus ring around the one native Link node rather
than creating a competing focus target. Link integration tests cover activation,
disabled state, caller focus, traversal and opening policy through this node.

`A11yDisabledExt` and its `A11yDisabled` adapter are now public re-exports. The
implementation is unchanged: it forwards layout, input and observation and only
sets the native accessibility disabled flag. Yue's composed menu rows use the
same adapter as Base controls rather than duplicating it; Breadcrumb tests cover
disabled menu semantics as well as skipped keyboard activation.

## Yue Select integration (2026-11-02)

`Select::on_open_change` and `Combobox::on_open_change` now return the resulting
open state (`Fn(bool, &mut Window, &mut App) -> bool`). This is a breaking API
change in this development workspace. Return false when rejecting an opening;
the primitive then keeps focus on the trigger instead of hidden content. A
confirm on an already open Select does not refocus content after its owner
commits, closes and restores trigger focus. A first closed Up/Down request is
consumed after opening, preventing an outer navigator from moving twice.

Select also exposes readonly, required, invalid, accessibility_description,
tab_stop and tab_index. Readonly retains focus and semantics while rejecting
activation. Tab policy refines the supplied FocusHandle, not a second focus
target. The existing no-layout/no-extra-node disabled adapter adds
`SelectionA11y` / `A11ySelectionExt`: disabled, read_only, required, invalid and
multiselectable reach the actual AccessKit node. Native-node tests verify those
flags; they do not establish Windows UIA/Narrator behavior.

Input's `EntityInputHandler::unmark_text` now notifies observers after ending
composition. Some platform commits only unmark the existing preedit without a
second text replacement; consumers must be able to observe that boundary.
This does not emit a second text Change event or add Select behavior to Input.
Yue's headless test drives preedit, guarded Enter, unmark and subsequent query
filtering through the actual InputState. Ordinary Input remains Tab-to-indent;
only Select's local query context supplies its composite traversal.

Regressions: `select::tests` (including owner-rejected open, readonly and caller
tab policy), `combobox::tests`, `a11y_disabled::tests`, and the Input composition
tests. Run them with `tools/test-gpui-base.ps1 -Offline -Filter <filter>`; the
fixture uses the consuming workspace's production backend and lockfile.

## Yue Tabs integration (2026-11-02)

`Tab::in_list` opts into a retained `TabsController` and stable item key while
preserving standalone Tab behavior. `TabsSnapshot` supplies the owner's items,
selection, navigation axis, disabled state and automatic/manual activation.
User activation emits `TabsChangeRequest`; close emits `TabsCloseRequest` with
a suggested enabled neighbor. Callbacks run after releasing the entity borrow
and never silently commit selection or remove items.

The controller owns roving focus, axis-aware arrows, Home/End, Enter/Space,
Delete, key-preserving reorder/removal reconciliation and ScrollHandle geometry
in content coordinates. `Tabs::axis` adds native orientation. `TabPanel` attaches
only active descendants to layout, input and accessibility; its tab stop is
explicit. Yue composes these APIs with its token-driven cells and motion.

Native Tab, TabList and TabPanel roles, selection, disabled state and orientation
are available. GPUI does not publicly expose the stable AccessKit node IDs or
controls/labelled-by builders needed for native Tab/Panel relations; no private
hash reproduction or Cargo registry edits are used. Windows UIA verification
remains separate from native-node and headless regressions.

Run `tools/test-gpui-base.ps1 -Offline -Filter 'tabs::'` and Yue's headless `tabs`
integration target. Consuming workspaces must retain the root Cargo patch.

## Yue loading feedback integration (2026-11-02)

`ProgressValue` normalizes controlled percentages and represents unknown values
without leaking NaN or infinity into native numeric properties. Existing `Progress`
builders remain compatible and now support descriptive `value_text`.

`LoadingDelay` owns one cancellable weak-entity deadline with a generation guard;
it gates presentation without running or extending host work. `loading_phase`
uses one application epoch, requesting frames only for positive-area visible,
unclipped content in an active window. Reduced motion and static variants do not
request loop frames. Returning windows rejoin the common phase.

`LoadingRegion` attaches only its active child branch, projecting Group/busy plus
one Polite Status. `A11yLive::busy` sets or clears the existing native flag without
adding focus targets. Timers, native properties and frame requests are covered by
`loading::tests`, `progress::tests` and `a11y_live::tests`; actual Windows UIA and
reader behavior remain separate from these headless contracts.
