# VirtualGrid UIA Provider Investigation

Date: 2026-11-05. This continues the Windows accessibility gap recorded in
[the keyboard and hardware acceptance](2026-11-05-virtual-grid-keyboard-and-hardware-boundary.md).

## Result

No production change was adopted. The pinned `accesskit_windows 0.34.0` does
not expose UIA Grid, GridItem, ItemContainer, or VirtualizedItem patterns. A
source check of the latest crates.io release available during this review,
`0.35.2`, found none of those interfaces either. Upgrading alone therefore
does not close the gap.

The existing bridge does provide a real action route: UIA actions on published
nodes become AccessKit `ActionRequest`s, and GPUI dispatches registered actions
by target node. `Action::CustomAction` carries only `ActionData::CustomAction(i32)`.
That is insufficient as a general Grid owner contract: it does not identify a
Grid instance and cannot represent arbitrary `usize` row and column coordinates
without narrowing or encoding policy. It also cannot make an off-tree node
addressable to UIA by itself.

An implementation must extend the Windows COM provider itself. `IGridProvider::GetItem`
must return stable providers for both mounted and unmounted coordinates;
unmounted providers need bounded-lifetime virtual proxies implementing
`IGridItemProvider` and `IVirtualizedItemProvider::Realize`. The owner route
must identify a live Grid, validate coordinates and spans, issue a realization
request without keeping a window or view alive, and allow UIA to resolve the
mounted semantic node afterward. `IItemContainerProvider` search must remain
bounded and must not enumerate the million-by-thousand logical space.

The AccessKit `0.34.0` Windows adapter is a 12-file, approximately 158 KB
implementation. Its `PlatformNode` and `Context` are private. Supporting these
patterns requires maintaining a fork or changing the adapter upstream; adding
an independent COM helper beside GPUI cannot replace the provider returned by
the existing `WM_GETOBJECT` path. The probe fixture from the previous round
continues to establish the pinned bridge's current capabilities.

## Adoption decision

Do not vendor or upgrade AccessKit as an unreviewed shortcut. A fork spike is
only ready when it first demonstrates all of the following in an isolated COM
fixture:

- mounted Grid and GridItem metadata comes from the live AccessKit tree;
- unmounted `GetItem` returns a virtual proxy without creating a semantic node
  or retaining a GPUI owner;
- `Realize` routes a full-width coordinate and Grid identity through a typed,
  multi-instance-safe request and rejects stale owners and out-of-range cells;
- spans return one origin provider and covered coordinates resolve consistently;
- after realization, UIA can resolve the mounted node with stable identity;
- repeated distant `GetItem` calls keep proxy and owner memory bounded;
- existing Invoke, SetFocus, selection, text, and tree navigation contracts
  still pass unchanged.

Only after this spike passes should the repository carry a patched bridge. The
next review must compare the upstream and fork source diff, keep production
dependency versions locked, and add Windows CI checks for every newly supported
pattern. A real Windows UIA client must then verify the actual provider
interfaces and Realize behavior. Narrator position announcements, spanning,
navigation, recycling, and Read All remain a separate manual acceptance gate.

## Rating impact

This investigation changes no user-visible behavior and earns no score increase.
The current engineering self-rating remains **8.9/10** under the existing
six-dimension rubric. The Windows provider and Narrator chain remains the main
software gap; true cross-DPI and multi-monitor validation remains device-limited
and unpassed. The 9.6 target is not met.

