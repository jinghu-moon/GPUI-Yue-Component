# GPUI Yue Component

GPUI component library for Windows desktop applications built on GPUI Kit. The library reuses `gpui-base` behavior through `gpui-kit` and maps Yue Design System tokens into Rust theme profiles.

## Workspace

- `crates/yue-component`: reusable library crate with Yue theme profiles and text/icon/icon-text Buttons; it does not own the application lifecycle.
- `crates/yue-component-icons`: generated Rust names and GPUI SVG renderer for the official Tabler outline and filled catalog.
- `examples/windows-showcase`: Windows consumer application for integration checks.
- `tools/sync-yue-theme.mjs`: generates the checked-in catalog of every Yue token and all supported theme-profile values from the design system inventory.
- `tools/sync-tabler-icons.ps1`: syncs an exact Tabler release or discovers the latest GitHub release.

## Use From a GPUI Kit Application

Use the same `gpui-kit` release as this workspace to keep GPUI types unified. Do not add a separate `gpui` dependency.

```toml
[dependencies]
gpui-kit = { version = "0.8.0", default-features = false }
yue-component = { path = "../GPUI-Yue-Component/crates/yue-component" }
```

The consumer owns `gpui_kit::application()`, initializes GPUI Kit, opens its window, and composes Yue components inside its view. `YueTheme` exposes semantic profile colors and the complete resolved token catalog. Use `theme.token("--space-8")` for lookup or iterate `theme.all_tokens()`; each `YueToken` retains its CSS value, namespace, and layer. Color, number, and pixel helpers are available when the value has that form.

Use Tabler icons by importing a generated name. `yue-component` re-exports the icon API, so applications using the component library need no second icon dependency. SVG loading and rendering stay inside the icon crate:

```rust
use yue_component::{Icon, outline};

Icon::new(outline::Search);
```

Projects using only the icon renderer can depend on `yue-component-icons` directly.

Sync the latest official release with `tools/sync-tabler-icons.ps1 -Latest`. Use `-Version vX.Y.Z` to pin an exact release and `-Check` to check whether a newer release exists.

## Verify

On Windows with the Rust MSVC toolchain installed:

```powershell
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
node tools/sync-yue-theme.mjs --design-system D:\100_Projects\110_Daily\Yue-Design-System --check
```

To refresh the generated catalog after changing Yue tokens:

```powershell
node tools/sync-yue-theme.mjs --design-system D:\100_Projects\110_Daily\Yue-Design-System --write
```

Button API and icon asset workflow: [docs/02-button.md](docs/02-button.md). Input, field and form API: [docs/03-input.md](docs/03-input.md). Switch API: [docs/04-switch.md](docs/04-switch.md). Radio design: [docs/05-radio.md](docs/05-radio.md). Checkbox design: [docs/06-checkbox.md](docs/06-checkbox.md). Dropdown menu design: [docs/07-dropdown.md](docs/07-dropdown.md).

The headless element-tree suite in `crates/yue-component/tests/` is behind an optional feature and is not part of `cargo test --workspace`; run it with `cargo test -p yue-component --features headless-tests`. See [docs/03-input.md](docs/03-input.md) for why it stays opt-in.
