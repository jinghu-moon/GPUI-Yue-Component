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

## Explicit nearest image sampling (2026-11-03)

`Window::paint_image_nearest` uses the existing decoded image and sprite atlas,
without an enlarged bitmap, CPU readback or another texture upload. It currently
requires the maintained Windows backend; other platforms return a clear error.
The existing `paint_image` signature and linear sampling path are unchanged.

Nearest sprites retain full-image bounds and the entire atlas tile. Content
masks clip the requested viewport without first rounding the visible region to
source texels, preserving fractional pan alignment. Bit 0 of the existing
`PolychromeSprite.pad` field selects sampling; the GPU instance ABI is unchanged.
Visible outer corners retain radii while newly clipped inner corners are square.

Paint snapshots retain this sampling choice, full geometry and image ownership,
including cached paint and atlas eviction/reload. The snapshot contract script
adds a nearest case at 1 / 1.5 / 2 display scales. Windows production-shader WARP
tests cover exact texels, atlas-edge isolation, fractional pan, clipping, rounded
corners, alpha, opacity, grayscale and the unchanged linear default. See the
Windows patch record for commands. ImagePreview uses the path for its explicit
pixel-display preference or an active pixel grid at 400% or above, only for
full-resolution resources; the core has no such UI policy.

## Precise linear images and upload lifetime (2026-11-03)

`Window::paint_image_precise` selects bit 1 of the same instance field. Like
nearest, it preserves full-image bounds and UVs under clipping, avoiding the
ordinary path's rounded source sub-tile geometry. The maintained Windows shader
provides tile-local alpha-weighted bilinear sampling. This is a separate opt-in
API; ordinary `paint_image` geometry and sampler behavior remain unchanged.
Paint snapshots retain and replay the sampling flags and image ownership.

Direct precise/nearest uploads register a `Weak<RenderImage>` per image/frame.
After old frame data is cleared in draw, uploads whose last owner has gone are
removed from the atlas. Retained resources and frozen snapshots keep uploads
alive; the registry adds no strong owner. Explicit `drop_image` also removes the
weak entry. This bounds obsolete direct uploads across result revisions without
one component evicting another consumer's live resource. Retirement needs a
subsequent draw; atlas allocation overhead and host-held resources still count
toward memory. This is not a global memory or VRAM budget.

## Explicit FP16 images and window color output (2026-11-03)

`RenderImageFormat` distinguishes BGRA8/sRGB, relative-linear SDR RGBA16F and
absolute scRGB RGBA16F (1=80 nit). Buffer constructors validate actual storage,
finite half floats and straight alpha; atlas keys, sprite flags, snapshot replay
and weak upload ownership preserve the format. `RenderTargetReadback` carries
native format/stride and associated alpha; it is not an `RgbaImage` byte cast.

`WindowColorPolicy` defaults to SDR. Opt-in `AutomaticHdr` asks the platform for
verified output; `Window::color_output` reports actual mode separately from
display capability/current state, with luminance, generation and fallback cause.
Windows implements the event/renderer lifecycle. Other backends retain SDR.
The legacy test `render_to_image` is explicitly SDR straight-alpha on Windows.

Run `tools/test-gpui-snapshot.ps1 -Offline` for 5 backdrop and 6 snapshot
contracts, including FP16 upload/eviction/replay/owner retirement. Windows
shader/native checks are in its patch record. See
`docs/performance/2026-11-03-hdr-resources.md` for evidence and remaining gates;
core tests do not certify physical HDR output or SnapClip capture.

`tools/test-gpui-snapshot.ps1 -Offline` covers precise-linear cached capture,
translation/clipping, eviction/reload, frozen ownership and 20 consecutive image
replacements, as well as the existing backdrop and snapshot contracts. Component
tests check full UV geometry and static integer-zoom device-pixel alignment at
1 / 1.25 / 1.5 / 2 display scales. Production-shader pixel checks are recorded in
the Windows patch log; no real-window or performance claim follows from these tests.
