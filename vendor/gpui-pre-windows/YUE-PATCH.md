# Yue Windows local Gaussian backdrop extension

Source: crates.io `gpui-pre-windows 0.3.9`, Zed revision
`93d745668805029476e6d7fbb19652116fed51de`. Original Apache-2.0 license retained
in `LICENSE-APACHE`. Cargo registry sources are untouched. Consumers must patch
both this backend and `gpui-pre`; see the workspace root Cargo.toml.

## Changes (2026-11-02)

- `src/directx_renderer.rs`: dispatch filtered quad markers at their exact paint
  order, lazy pipeline ownership, resize/device-lost cleanup, trace counters.
- `src/directx_renderer/backdrop.rs`: clipped framebuffer ROI copy, reusable
  bucketed textures, optional prefilter/downsample, horizontal/vertical paired
  Gaussian, rounded/opacity-aware replacement composite. No CPU readback, no
  animation timer, per-window 64 MiB retained scratch budget. An oversized
  region keeps its normal tint without disabling later small filters.
- `src/shaders.hlsl` and `build.rs`: shader entries and release FXC embedding;
  debug uses the existing runtime compiler. CLAMP sampling of valid texels is
  independent of existing sprite samplers; bindings and framebuffer are restored.
- `src/directx_renderer/backdrop_profile.rs`: optional CPU/stage GPU timing,
  disabled by default (`GPUI_BACKDROP_PROFILE=N` enables sampling every N regions).
  At most one disjoint interval per frame, 8 frame / 128 sample query slots,
  nonblocking DONOTFLUSH polling with explicit S_FALSE handling. Busy rings skip
  samples; diagnostics never request frames or wait on the GPU in production.

The first consumers are ImagePreview's floating toolbar and circular comparison
grip. This backend knows nothing about ImagePreview, Yue colors, icons or tokens.
It executes public GPUI `Styled::backdrop_blur`/`Window::paint_backdrop_blur`
commands. Other renderers retain tint but do not implement the filter.

## Verification and removal

`pwsh -NoProfile -File tools/test-gpui-backdrop.ps1` runs production-pipeline
D3D11 WARP tests without HWND, swap chain or user UI. `-Release` also verifies
the embedded precompiled shader path. Tests cover Gaussian pixel values,
clip/rounding/alpha, repeated framebuffer sampling, pooled allocation, DPI cost
and the allocation budget. Core Scene ordering and paint snapshot replay are
covered by `tools/test-gpui-snapshot.ps1 -Offline`.

`tools/test-gpui-backdrop.ps1 -Profile` explicitly runs the Release hardware
timestamp benchmark (no WARP fallback), contrasting per-stage profiling with
one timestamp pair around all filters while profiling is disabled. Only this
offscreen benchmark flushes/drains queries, outside CPU submission timing.
First measurements and limitations are recorded in `docs/26-backdrop-blur.md`.

Remove only after an upstream version provides equivalent same-window filtering,
order barriers, clipping/opacity and bounded resource reuse, with both test
suites passing. Do not replace it with a whole-window backdrop material or a
preblurred source image. WARP timings are not hardware GPU performance claims.
