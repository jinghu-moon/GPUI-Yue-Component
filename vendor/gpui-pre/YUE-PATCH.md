# Yue GPUI paint snapshot extension

Source: crates.io `gpui-pre 0.3.9`, snapshot `zed@93d7456`. The original
Apache-2.0 license is retained in `LICENSE-APACHE`. No Cargo registry source was
edited. The consuming workspace must patch `gpui-base`, `gpui-pre`, and
`gpui-pre-windows` (for the local Gaussian backdrop effect).

## Paint-only exit frame (2026-11-02)

`Window::capture_paint(origin, paint)` records scene commands while painting
normally. `Window::paint_snapshot(snapshot, new_origin)` translates those commands
and intersects their masks with the current content mask. It rejects other
windows and display scales. It does not restore any descendant element, focus,
Tab registration, input/IME handler, mouse listener, accessibility node, tooltip,
deferred popup, subscription or frame callback. This is a short-lived frozen
paint frame, without GPU readback or a bitmap screenshot.

Paths translate vertices as well as bounds and masks. Glyph/SVG sprites retain
their existing atlas rendering. Images retain their decoded `RenderImage` and
paint arguments and go through `paint_image` again on replay, so eviction and
atlas tile reuse remain safe. Frame paint metadata follows `reuse_paint`, which
also covers images inside cached views and nested captures.

`MotionDisclosure` captures only its live panel's paint, then detaches that
panel immediately on close. The frozen paint is clipped as height decreases.
Completion, geometry invalidation, width changes and motion-disabled landing
discard it. Reopening returns to live content at the current visible height.
Deferred overlays are never captured. Current-window placement can change;
the snapshot follows its panel origin rather than remaining at old coordinates.

Changed upstream files: `src/window.rs` (small image metadata and reuse hooks),
`src/scene.rs` (translation/mask handling), and the new
`src/window/paint_snapshot.rs`. A test-only primitive observation complements the
existing `painted_quads` API. The later backdrop extension is tracked below.

Targeted backend regressions:

```powershell
pwsh -NoProfile -File tools/test-gpui-snapshot.ps1 -Offline
```

The isolated integration fixture uses the production backend and consuming
workspace's dependency pins. It avoids unrelated upstream unit tests whose font
assets and dev-only macros are absent from the crates.io package.

Tests exercise path/sprite/quad translation and clipping, image eviction/reload,
cached paint image ownership, and replay without new input registrations at
1 / 1.5 / 2 display scales. Yue's Collapse headless target additionally exercises
the actual controlled component, input detachment, reversal, reduced motion,
frame scheduling and multi-panel closing. These checks do not replace user UI,
GPU pixel, native-reader or IME verification.

## Same-window backdrop foundation (2026-11-02)

`Styled::backdrop_blur(AbsoluteLength)` and `Window::paint_backdrop_blur` emit a
filter before the element's own shadow, tint and children. Sigma is a logical
Gaussian standard deviation, resolved through rem/DPI, capped at 64 logical
pixels; zero, negative and non-finite values or zero opacity emit nothing.

`Style`/`StyleRefinement` carry the effect without selecting product colors.
`Scene` retains a `BackdropBlur` primitive plus a transparent `Quad` marker,
preserving the existing Quad GPU ABI and PrimitiveBatch interface. Every filter
creates a global draw-order barrier, including in explicit layers. Subsequent
ordinary bounds-tree ordering remains relative within the new epoch; metadata
survives scene replay, cached paint and translated/clipped snapshots. Frame
clear resets both metadata and barriers. The Windows renderer recognizes
markers and filters the current framebuffer; other renderers retain normal tint.

Core integration tests in `tests/backdrop_blur.rs` cover the fluent interface,
global barriers, layers/replay/clear, clipping, rem/DPI/opacity, invalid inputs
and no unsolicited frame callbacks. Existing paint snapshot tests now also
verify filter capture, cached reuse, translation and clipping. Real GPU pixel
tests live in `vendor/gpui-pre-windows`; see its `YUE-PATCH.md` and
`docs/26-backdrop-blur.md` for performance bounds and platform scope.

Remove this extension only when upstream offers equivalent ordering, replay and
same-window filtering and the core/backend contract suites still pass.
