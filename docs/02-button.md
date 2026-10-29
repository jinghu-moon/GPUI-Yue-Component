# 02 Button

Button is the first Yue component and the integration test for GPUI Base behavior, Yue theme mapping, and packaged SVG icons.

## Content forms

```rust
use yue_component::{ButtonSize, ButtonVariant, IconPosition, YueButton, outline};

YueButton::text("save", theme, "Save")
    .variant(ButtonVariant::Primary)
    .on_click(|_, _, _| save());

YueButton::icon("search", theme, outline::Search, "Search");

YueButton::icon_text("create", theme, outline::Plus, "Create");
YueButton::icon_text("continue", theme, outline::ArrowRight, "Continue")
    .icon_position(IconPosition::Trailing)
    .size(ButtonSize::Large);
```

The constructors make the three content forms explicit. An icon-only button requires a non-empty accessible name. Use stable IDs when a button is repeated.

## Behavior and presentation

- `gpui_kit::base::Button` owns button role, focus, Tab traversal, Enter/Space activation, disabled behavior, and click dispatch.
- `YueButton` supplies Yue visual presentation and the text, icon, and icon-plus-text constructors.
- `ButtonVariant` exposes `Default`, `Primary`, `Outline`, `Ghost`, `Text`, and `Danger`; Danger colors come from the Yue Design System token inventory.
- `ButtonSize` provides Small, Medium, and Large; `ButtonShape` provides rounded rectangle, rounded square, pill, and circle. Pill keeps the button width and applies the full radius token; Circle is intended for equal-size icon buttons.
- Button presentation reads Yue Button Component Tokens for variant colors and hover colors, dimensions, typography, icon size, padding, spacing, radius, borders, focus, disabled state, and motion. The current `--button-duration` is 100ms, so it is the source for both hover and press transitions.
- Icon plus text buttons default to a leading icon and can set `IconPosition::Trailing`.
- `.loading(true)` displays an animated Tabler loader and disables activation. `.block()` fills the available width. `.tooltip("...")` uses GPUI's native tooltip behavior.
- `ButtonGroup` arranges Yue buttons horizontally by default, or vertically with `.vertical()`, while each button retains its Base behavior.
- Hover color eases between states; pointer press scales the button down slightly and springs it back on release. Keyboard-visible focus and disabled states have distinct treatment. Transitions follow GPUI reduced-motion preferences.
- Callers own the application/window lifecycle and pass their chosen `YueTheme` profile.

## SVG icons

`yue-component-icons` owns the synchronized Tabler outline and filled SVG catalog. Generated icon names are available as Rust types in `outline` and `filled`; use `Icon::new(outline::Search)` for a standalone icon. The component renders the selected embedded SVG with GPUI's native `svg().data(...)`, so applications do not configure `AssetSource` or handle SVG paths.

Use the standalone `Icon` component when an icon is not inside a Button:

```rust
use yue_component::{Icon, outline};

Icon::new(outline::Search).size(gpui_kit::rems(1.0));
```

The crate owns the official SVG files under `crates/yue-component-icons/assets/tabler`. Refresh them from the official repository with `tools/sync-tabler-icons.ps1 -Latest`, or pin an exact release with `-Version vX.Y.Z`. The sync writes the official tag and commit to `tabler-icons-version.json`; normal builds use only the checked-in assets and do not access the network.

## Yue tokens

The checked-in Rust catalog contains every token from the Yue Design System inventory, with resolved values for Light/Azure, Dark/Azure, Light/Neutral, and Dark/Neutral. Values remain CSS strings so lengths, opacity, shadows, curves, and keywords are preserved alongside colors.

```rust
let spacing = theme.token("--space-8").expect("known Yue token");
assert_eq!(spacing.pixels(), Some(8.0));

for token in theme.all_tokens() {
    println!("{} = {}", token.name(), token.value());
}
```

Regenerate the catalog with `node tools/sync-yue-theme.mjs --design-system <Yue-Design-System-root> --write`.

## Verify

On Windows:

```powershell
cargo test --workspace
cargo check --workspace
cargo run -p windows-showcase
```

The showcase renders all three content forms and Yue's four theme profiles. GPUI Base supplies the interaction contract; workspace tests cover constructors and generated SVG assets.
