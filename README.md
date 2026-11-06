# GPUI Yue Component

Windows component library built on GPUI Kit's Base behavior and Yue Design System tokens. Includes Breadcrumb, Button, Link, Input/Textarea/NumberInput, Field/Form, Switch, Radio, Checkbox, Segmented, Tabs, Tag/TagGroup, Divider, Dropdown menus, Tooltip/OverflowTooltip, ColorPicker/Panel and SwatchGroup, plus Tabler icons and typography/layout/motion helpers.

## Integrate

The host owns its application, windows and retained state. Use the same resolved `gpui-kit` and GPUI backend throughout the dependency graph; do not add a separate `gpui` dependency. The workspace currently resolves Kit 0.8.0; Cargo's `"0.8.0"` requirement allows compatible patch releases.

```toml
[dependencies]
yue-component = { path = "../Yue-GPUI-Component/crates/yue-component", default-features = false }
# Optional if using yue_component::gpui_kit, which re-exports the same Kit types:
gpui-kit = { version = "0.8.0", default-features = false }

# Required in the consuming workspace's root Cargo.toml during development:
[patch.crates-io]
gpui-base = { path = "../Yue-GPUI-Component/vendor/gpui-base" }
gpui-pre = { path = "../Yue-GPUI-Component/vendor/gpui-pre" }
gpui-pre-windows = { path = "../Yue-GPUI-Component/vendor/gpui-pre-windows" }
```

Cargo patches are not inherited from a path dependency. This workspace maintains Base 0.8.0 extensions for shared Popup positioning, Dialog, Toast lifecycle/stack/live semantics, Slider domain/drag/focus semantics, numeric input and Link caller-owned focus; a consuming framework must resolve the same patched Base. See [the patch record](vendor/gpui-base/YUE-PATCH.md). Remove the patch only after an upstream version provides equivalent APIs and the integration tests pass.

The maintained GPUI core and Windows backend also provide reusable same-window
Gaussian backdrop filtering through `Styled::backdrop_blur(rems(1.0))`. It samples
only the local background on the GPU; the caller owns tint and other styling.
See [the foundation contract and performance limits](docs/26-backdrop-blur.md).

Initialize once before opening windows:

```rust
use yue_component::{Accent, ThemeMode, YueTheme, gpui_kit};

gpui_kit::application().run(|cx| {
    yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure))
        .expect("initialize Yue");
    // Open your application's view with gpui_kit::open_window(...).
});
```

`init` initializes Kit when Base has not been initialized, registers enabled bundled fonts once, and installs the default Yue/Base theme. In an existing framework, call the framework's initialization first, then Yue's initialization before its first window. Yue does not introduce another application or Root.

The complete [minimal consumer](examples/minimal/src/main.rs) uses only `yue-component`, with a controlled switch and a save command. Run `cargo run -p yue-minimal` or browse all components with `cargo run -p windows-showcase`. Both applications use Base only; neither enables Kit's styled `component` or Lucide `assets` layer.

Showcase navigation has two independently collapsible groups, 组件 and 设计, with entries alphabetized inside each group. Disclosure supports keyboard activation, animated height and reduced motion.

## Theme and composition

`YueButton::new("save", "保存")` follows the application default. Existing constructors accepting a `YueTheme` keep that explicit palette; `.use_default_theme()` opts into the application default and `.theme(theme)` selects a local override. Update the application with `yue_component::set_theme(cx, theme)` from an event/update, so Base colors and all inherited Yue components refresh together. Do not install themes inside `render`.

Base has one application-wide theme. Explicit Yue overrides support previews, but cannot give Base global consumers, such as scrollbars, independent palettes in different windows. A Dropdown panel and its supplied trigger each own their theme; use an inherited Button trigger when the whole menu should follow the default.

Buttons and selection controls expose `width`, `min_width`, `max_width`, `margin` and `flex_grow` for outer layout. Internal dimensions and visual states remain token-owned. Button/Switch expose caller-owned `track_focus`, `key_context`, `tab_index`, `tab_stop`, accessible names and descriptions; parent action handlers continue to receive application actions. Input retains Base's state and keyboard behavior, including Tab indentation. Public constructor examples are compiled as doctests.

## Selects

Retain `Entity<YueSelectState>` and render `YueSelect::new(id, &state)`. Stable keys support single/multiple values, local or remote search, groups, disabled selections, creation, paging and virtualized lists. Search composes the existing **YueInput**; multiple values use YueTag and the shared Popup foundation. User changes arrive through retained `SelectEvent` subscriptions; external setters are silent.

`YueField.select(...)` forwards field semantics and `YueForm.require_select` / `rule_select` validate typed `SelectValue`. Browse **组件 → Select** in Showcase, including remote race/error simulations, ten thousand options and Dialog integration. See [the Select guide](docs/20-select.md) for public APIs, slot focus contracts, token sources and verification limits. The maintained Base Select/Combobox patch adds `on_open_request` returning the actual open state and retains upstream `on_open_change` notifications for Kit styled consumers; downstream workspaces must retain the root Cargo patch described in [YUE-PATCH.md](vendor/gpui-base/YUE-PATCH.md).

## Tabs

`YueTabs::new(id, selected).items(...)` composes a controlled tab list and its active panel, where `selected` is `Option<SharedString>`. Supply stable `YueTabItem::new(key, label).content(...)` items; `.on_change(...)` requests a key and the host commits or rejects it. Normal/Card, Top/Bottom/Left/Right, three sizes, icons, independent add/close commands, automatic/manual keyboard activation and keyed scrolling share Base's Tabs controller. Keep panel input/editor Entities in the host; `.lazy(true)` builds only the active panel.

The Normal indicator moves and resizes over 150ms for nearby visible targets; distant jumps snap. Optional Fade/Slide animate only the incoming panel, with the outgoing input tree removed immediately. Reduced motion snaps all transitions. Browse **组件 → Tabs** and see [the Tabs guide](docs/21-tabs.md) for APIs and native accessibility limits. This component requires the maintained Base patch.

## Collapse

`YueCollapse::new(id, expanded_keys).items(...)` is a controlled disclosure group.
Build stable `YueCollapseItem::new(key, title).content(...)` items and accept or
reject each `.on_change(...)` request in the host. Multiple is the default;
`CollapseMode::Accordion` permits at most one expanded item, including none.
Actions remain independent siblings of each heading's trigger. All enabled
headings are normal Tab stops, with Up/Down/Home/End focus navigation.

The default measured motion reveals and clips content over 320ms in both
directions, using cubic-bezier(.4, 0, .2, 1). Closing immediately detaches the old
input and accessibility subtree and clips a frozen paint frame; retain
input/editor Entities in the host to preserve values. No title hover background
is applied. Interrupted
motion continues from current pixels. None/reduced motion snaps to the final
state. Three sizes, two variants, leading/trailing/custom indicators and icon-only
activation share Yue tokens. Browse **组件 → Collapse** and see the
[Collapse guide](docs/23-collapse.md). This component uses the maintained Base and
[GPUI paint snapshot patches](vendor/gpui-pre/YUE-PATCH.md). Downstream workspaces
must include both Cargo patches shown above.

## Breadcrumbs

`YueBreadcrumb::new("path", theme).items(...)` composes Base text buttons, decorative icons/separators and the existing Dropdown. Supply stable `YueBreadcrumbItem::new(value, label)` destinations and handle `.on_navigate(...)` in the host; the final item is the current page and is static by default. Navigation requests expose `value()` and `source()` without changing your route.

Use `.collapse(BreadcrumbCollapse::new(5).before(2).after(2))` to retain both ends and expose the hidden middle through a keyboard-accessible menu. Optional item icons, disabled destinations, custom separators/ellipsis, `.current_clickable(true)` and actual-overflow-only tooltips share Yue tokens. Browse **组件 → Breadcrumb** for configuration and working navigation; see [the component guide](docs/15-breadcrumb.md) for API and validation details.

## Dividers

`YueDivider::new("section-rule", theme)` renders a full-width horizontal rule. Add `.label("Details").align(DividerAlign::Start)` for a text divider, `.dashed(true)` for dashed lines, or `.layout(DividerLayout::Vertical).height(gpui_kit::rems(1.5))` for an unlabelled inline rule. Dividers use Yue tokens and add no keyboard stop. Browse **组件 → Divider** in Showcase; see [the component guide](docs/14-divider.md) for API and sizing details.

## Lists

`YueList::new("members").child(YueListItem::new("alex").content(YueListItemMeta::new("Alex").description("Design team")))` provides stateless content layout. Three sizes, horizontal/vertical item content, leading/body/extra/actions slots, separators, stripes, groups, header/footer and empty content use Yue tokens. Compose existing buttons and checkboxes; hosts retain data, selection, loading and scrolling.

Browse **组件 → List** for working configuration, states and ten thousand measured virtual items using `YueVirtualList`. See [the List guide](docs/29-list.md) for ownership and testing; use `ListGrid::columns` or `ListGrid::min_width` with `YueVirtualListGrid` for responsive and large card collections.

## Links

`YueLink::new("guide", "使用指南", "https://example.com/guide")` opens a resource through the system's default application. `LinkVariant` provides Default/Primary/Danger/Warning/Success, `LinkSize` provides three sizes, and `LinkHover` selects animated underline, color, or no hover effect. Primary is the default; `.underline(true)` remains visible independently of the hover policy. Optional `.leading_icon(outline::Link)` and `.trailing_icon(outline::ExternalLink)` reuse Tabler SVGs.

Use `.open_with(...)` to replace system opening, and `.on_activate(...)` to observe after it. Link exposes the same outer layout, theme, focus, Tab and accessibility builders as the other controls. Disabled links neither open nor notify. Browse **组件 → Link** for the live configuration panel, five-color matrices, icons and host override example. See [the Link guide](docs/13-link.md) for the API, source tokens and validation limits.

## Tooltips

`YueButton::new("save", "Save").tooltip("Save the current document")` uses the shared Popup foundation. Initialize Yue before creating the window's Base Root; an already-created Root cannot acquire plugins later. Button and Segmented no longer use GPUI's independent native tooltip overlay.

For explicit placement, variants or trigger modes, use `YueTooltip::new(id, theme, text).trigger_with(...)`, and attach its supplied FocusHandle to the actual control. An existing control can share its handle through `.track_focus(&focus).trigger(control)`. The tooltip adds no Tab stop, preserves command activation, and supports Escape even when the window has no focused control. Button uses tooltip text as an accessibility description only when no explicit description is supplied; arbitrary trigger wrappers cannot rewrite their children's descriptions.

`YueTooltipGroup::new(id, gpui_kit::Axis::Horizontal)` / `Vertical` defines one toolbar's continuity and protection boundary. Register buttons without help using `.protected_child(id, button)`. Only nearby, adjacent targets in the same group, parent and scope can translate: 150ms on a safe outer rail, with the whole bubble and arrow clear of protected buttons. Different content sizes or variants use a safe fade; there is no layout-size morph in V1. Distant or nonadjacent targets restart the hover delay.

Hover reads Base `TooltipDefaults` (500ms show, 300ms grace by default); focus shows immediately. Six variants, twelve side/alignment combinations, controlled open, optional auto-hide and reduced motion are supported. `YueOverflowTooltip` measures actual shaped text against its single-line slot and disables help when the text fits. Browse **组件 → Tooltip** in Showcase and see [the component guide](docs/10-tooltip.md) for sizing rules and validation limits.

## Color editing

Create and retain `Entity<YueColorPickerState>` in the host's constructor, then build `YueColorPicker::new("fill", theme, &state).label("Fill")` in `render`. The default field is 160px wide; its popup editor independently defaults to 300px. `YueColorPickerPanel` embeds the same editor in a persistent inspector. One state should have only one mounted editing panel.

The HSV plane, hue slider and independent alpha slider synchronize HEX/RGB/HSL channel inputs. Subscribe to `ColorPickerEvent::Change` for live previews and `Commit` for completed edits; `set_value` is silent and cancels an old gesture. Values and history use RGBA8 sRGB identity. `YueSwatchGroup` is separately controlled by a stable string value and supports solids, checkerboards, two-stop linear gradients and an optional custom-color picker.

The hue/alpha sliders follow the reference demo: colored circular thumbs, white borders, a fine dark outline and shadow, with a centered 1.15× press animation. Their hit regions and pointer mapping remain fixed; reduced motion settles the visual scale immediately. The appearance is backed by the source ColorPicker tokens.

Browse **组件 → ColorPicker** with `cargo run -p windows-showcase`. It includes sizes, compact triggers, embedded panels, disabled controls, four theme profiles, alpha=0, event counters and background choices. The [component guide](docs/11-color-picker.md) documents the API, token sources and validation limits.

## Slider

Browse **组件 → Slider** with `cargo run -p windows-showcase`. Single/range values,
horizontal/vertical layouts, three sizes, disabled state, marks, live value tooltips
and numeric editors are included. `SliderControls::{None, Input, Buttons}` selects
one editing form: centered numeric fields with arrows, or −/+ buttons at the track
ends followed by arrow-free numeric fields. Width follows the current font and at least
three digits. Valid typing updates the track immediately; invalid drafts show an error
without moving the last valid value. Marks attract the pointer only within a small
token-defined radius; free dragging, keyboard and numeric steps remain available.
`.snap_to_marks(false)` disables these local magnets. Pointer movement follows
the snapped value immediately, including outside-track dragging and release. See the
[Slider guide](docs/16-slider.md) for API, tokens, events and validation boundaries.

## Tags

`YueTag::new("kind", "PNG")` creates a static compact label. Use `YueTag::action(id, label).on_click(...)` for a command or `YueTag::checkable(id, label, checked).on_change(...)` for a host-controlled toggle. Add `.on_close(...)` for an independent remove button; closing neither activates nor unchecks the body, and the host decides whether to remove the item and where focus goes afterwards.

Five `TagTone` values, four `TagVariant` values, three sizes (20/24/30px at the 16px rem baseline), `TagShape::{Rounded, Pill, Mark}` and optional leading icons share the existing Tag token matrix. `YueTagGroup::new(id).children(tags)` wraps a small collection without inventing a selection model. Overflow help uses the existing Tooltip; hover/press/checked feedback honors reduced motion. V1 removal and reflow are immediate. Browse **组件 → Tag** for configuration, add/remove, selection and save-panel examples; see [the Tag guide](docs/19-tag.md) for API and verification boundaries.

## Tokens

The checked-in catalog contains the complete token source snapshot, including 13 Tooltip geometry/motion tokens, with four theme profiles and neutral-tone overrides. Static `theme::keys::*` access avoids catalog name lookup; dynamic CSS names remain supported:

```rust
use yue_component::{Accent, ThemeMode, YueTheme, theme::{keys, TokenValue}};
let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
let space = theme.token(keys::SPACE_8).expect("generated token");
assert!(matches!(space.typed(), TokenValue::Pixels(_)));
assert_eq!(space, theme.token("--space-8").unwrap());
```

Colors, pixels, numbers, durations and bezier curves have generated typed values. Font stacks, shadows and other contextual expressions retain their original strings and use explicit adapters; this is not a complete CSS engine. Components share the same token helpers and reject unsupported formats instead of discarding parts silently.

The [source snapshot](crates/yue-component/tokens/inventory.json) records package version, Git revision, dirty status and SHA-256 hashes. External tokens need not be present for builds or offline checks:

```powershell
node tools/sync-yue-theme.mjs --check
# Refresh from the external design system, then review snapshot and generated Rust:
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --write
# Verify the current external source still matches the snapshot:
node tools/sync-yue-theme.mjs --design-system D:/100_Projects/110_Daily/Yue-Design-System --check
```

## Toasts

Initialize Yue before creating Base `Root`; it installs one `YueToasterState` per window. Push from an event/update callback:

```rust
use gpui_kit::{App, Window};
use yue_component::{ToastError, ToastKind, ToastSpec, ToastTicket, YueToaster};

fn notify_saved(window: &mut Window, cx: &mut App) -> Result<ToastTicket, ToastError> {
    YueToaster::push(
        ToastSpec::new("设置已保存").kind(ToastKind::Success).progress(true),
        window,
        cx,
    )
}
```

`YueToaster::for_window` returns the existing owner. Configure `ToasterOptions` while empty to select one of eight positions and viewport insets. A `ToastTicket` supports `update`, `dismiss` and `begin_operation`; async work remains host-owned. Updating Loading to Success/Error retains its position and starts the result's timer. Ordinary updates preserve remaining time; `.timeout(Some(duration))` explicitly restarts it, and `.timeout(None)` makes it persistent. A card has at most one optional `ToastAction` plus its close control.

The stack measures natural heights, collapses to front-card shells, expands on hover/focus, and keeps independent reading clocks. It uses 300ms entry, 400ms response layout springs, 200ms exit and 150ms content changes; reduced motion removes travel and the rotating spinner. Modal presence and window inactivity hide/pause notifications. `FocusToasts` is an application-bindable action, with no default global shortcut. Browse **组件 → Toast** for eight positions, card switches, burst/update/close, async/modal cases, slow playback, pause and 16ms stepping. See [the Toast guide](docs/18-toast.md).

## Loading feedback

Progress, Spinner and Skeleton share Base progress semantics, a cancellable display
deadline and one loop epoch. Choose real percentages for measured work, Spinner for
unquantified local waiting, and Skeleton when the eventual layout is known.

```rust
use yue_component::{ProgressValue, SkeletonShape, YueProgress, YueSkeleton, YueSpinner};

let upload = YueProgress::new("upload", ProgressValue::new(42.)).label("上传附件");
let waiting = YueSpinner::new("search").active(true).label("正在搜索文件");
let avatar = YueSkeleton::new("avatar").shape(SkeletonShape::Circle);
```

Progress supports linear/circle, determinate/indeterminate and normal/success/error/paused.
Spinner defaults to a 150ms display delay. Skeleton supports four shapes and wave/pulse/none;
loops synchronize and respect reduced motion, visibility and clipping. `YueLoadingRegion`
builds only its active placeholder/content factory and adds native busy/status semantics.
Button's loading glyph also uses the shared painter and clock while retaining its own
size, color and duration tokens. See [the loading feedback guide](docs/22-loading-feedback.md)
and **组件 → Progress / Spinner / Skeleton** in Showcase.

## Image preview

Retain an `Entity<YueImagePreviewState>` in the host and render
`YueImagePreview::new(&state)` inside a container with a definite height. Submit an
`ImagePreviewDocument` through `state.set_document(...)`, with a stable document ID,
monotonic revision and decoded single-frame `ImagePreviewImage` resources. The host
owns loading, encoding and cancellation; user operations emit `ImagePreviewEvent`.

Prepare images on the host's background executor with
`ImagePreviewImage::from_rgba8(...)`, `from_bgra8(...)` or
`from_encoded(&bytes, &limits, description)`. `ImagePreviewPixels` also exposes
decoded pixels for hosts that retain the original for copying/exporting. Owned
native-size buffers move; borrowed native-size buffers copy once. Default limits
bound decoding and texture preparation. Oversized textures become explicit
proxies: `pixel_size()` is the original size, `texture_size()` is available detail,
and 100% always uses the original dimensions. EXIF orientation is applied; embedded
ICC/non-sRGB metadata is reported as unmanaged, without an ICC conversion claim.

Source, Compare and Result share one camera. The viewport supports pointer-anchored
zoom, captured panning, fit/100%, four transparency backgrounds, scoped keyboard
commands, smooth/nearest display, loading/error feedback and previous-result retention during updates.
Comparison requires declared full-frame alignment and compatible aspect ratios.
View commands use 260ms travel and mode changes use 180ms; gestures respond directly
and reduced motion snaps to the target. Windows pixel grids require full-resolution
images and at least 400% zoom; proxies retain the preference but suppress grids
and nearest sampling. Static integer zooms align to physical pixels.

Browse **组件 → 图片预览 ImagePreview** for the two user-provided screenshots,
actual JPEG/half-size results, transparency and Dialog composition. Fixtures live in
[the Showcase asset directory](examples/windows-showcase/assets/image-preview/README.md),
with original hashes recorded. See [the ImagePreview guide](docs/25-image-preview.md)
and [resource measurements and limits](docs/performance/2026-11-03-image-preview-resources.md).
Full-frame linear/nearest sampling and upload retirement use the maintained GPUI
core and Windows patches in `vendor`; consumers must use both patches. Editing,
animated-image playback, tiled original-detail loading and full ICC/HDR management
are outside this implementation.

## Icons and fonts

```rust
use yue_component::{Icon, outline};
let icon = Icon::new(outline::Search);
```

SVG bytes and rendering belong to `yue-component-icons`, which Yue re-exports. Icon-only applications can depend on that crate directly. Sync the latest official Tabler release with `tools/sync-tabler-icons.ps1 -Latest`; use `-Version vX.Y.Z` to pin a release or `-Check` to check updates.

The library's default `bundled-fonts` feature embeds HarmonyOS Sans SC, Fira Code and TCloudNumber. Disable default features as above for Segoe UI/Microsoft YaHei UI and Consolas/Courier New. Cargo features are additive: another dependency enabling `bundled-fonts` enables it for the same package in that build. Showcase opts in; minimal opts out. Build minimal separately to measure its system-font variant.

Measured Windows MSVC release binaries retain only Search when only Search is referenced; all 6,238 SVG files are not automatically bundled. The six font files total 34,018,828 bytes; disabling them reduced the minimal executable from 46,642,176 to 12,622,336 bytes. These are linker-retention results for the measured configuration, not a cross-toolchain guarantee. See [the report and reproduction commands](docs/08-integration-hardening.md).

## Verify

Run on Windows with the Rust MSVC toolchain and Node.js:

```powershell
node tools/sync-yue-theme.mjs --check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --features headless-tests
cargo test -p yue-component --locked --no-default-features --features headless-tests
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo clippy --workspace --locked --all-targets --features headless-tests -- -D warnings
pwsh -File tools/test-gpui-base.ps1 -Offline -Filter toast::tests
pwsh -NoProfile -File tools/test-gpui-snapshot.ps1 -Offline
pwsh -NoProfile -File tools/test-gpui-backdrop.ps1
pwsh -NoProfile -File tools/test-gpui-backdrop.ps1 -Release
pwsh -NoProfile -File tools/test-gpui-backdrop.ps1 -Profile
pwsh -NoProfile -File tools/test-gpui-hdr.ps1
pwsh -NoProfile -File tools/test-gpui-hdr.ps1 -Release
```

`headless-tests` is opt-in and enables Base observation directly, without Kit's optional Fast backend. Workspace tests now support this feature together with the Base-only showcase. Tests dispatch real input in headless windows and check callbacks, focus, layout and submitted drawing data; Windows reader output, GPU pixels, DPI and IME still require native-window validation. The Windows CI runs these checks.

`-Profile` is an opt-in Release hardware GPU timestamp benchmark without a window;
it reports cold and steady stage timings plus a minimally instrumented envelope.
See [measurement conditions and hotspot findings](docs/26-backdrop-blur.md#5-先测量再优化).

## Documentation

- [Component architecture and refactoring guide](docs/24-component-architecture.md)
- [Button and icon workflow](docs/02-button.md)
- [Input, Field and Form](docs/03-input.md)
- [Switch](docs/04-switch.md)
- [Radio](docs/05-radio.md)
- [Checkbox](docs/06-checkbox.md)
- [Dropdown](docs/07-dropdown.md)
- [P0–P2 implementation and resource measurements](docs/08-integration-hardening.md)
- [Segmented design and implementation](docs/09-segmented.md)
- [Tooltip design and implementation](docs/10-tooltip.md)
- [ColorPicker and SwatchGroup design and implementation](docs/11-color-picker.md)
- [Popup foundation design and implementation](docs/12-popup.md)
- [Slider design and implementation](docs/16-slider.md)
- [Dialog design and implementation](docs/17-dialog.md)
- [Toast design and implementation](docs/18-toast.md)
- [Tag design and implementation](docs/19-tag.md)
- [Select guide and implementation](docs/20-select.md)
- [Tabs design and implementation](docs/21-tabs.md)
- [Progress / Spinner / Skeleton design and implementation](docs/22-loading-feedback.md)
- [Collapse design and implementation](docs/23-collapse.md)
- [ImagePreview design and implementation](docs/25-image-preview.md)
- [Local Gaussian backdrop foundation](docs/26-backdrop-blur.md)
- [Windows HDR pipeline and implementation status](docs/27-hdr-pipeline.md) — high-precision resources and opt-in Windows scRGB output are implemented for verification; native-display acceptance and SnapClip capture/consumer migration remain incomplete. See [evidence and outstanding gates](docs/performance/2026-11-03-hdr-resources.md).
- [List design and implementation](docs/29-list.md) — linear layouts, item slots, grouping, states, responsive card grids and row-based virtualization.
