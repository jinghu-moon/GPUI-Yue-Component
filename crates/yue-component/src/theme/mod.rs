mod generated;

/// Theme mode supported by the Yue design token profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

/// Accent profile supported by the Yue design token profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Accent {
    Azure,
    Neutral,
}

/// Stable semantic color roles shared by Yue components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorRole {
    Page,
    Surface,
    SurfaceComponent,
    ButtonDefaultBackground,
    ButtonDefaultColor,
    ButtonPrimaryBackground,
    ButtonPrimaryColor,
    ButtonSecondaryBackground,
    ButtonSecondaryColor,
    ButtonSecondaryBorder,
    ButtonDangerBackground,
    ButtonDangerColor,
    TextPrimary,
    TextSecondary,
    BorderDefault,
    FocusRing,
    AccentSolid,
    AccentText,
    AccentSubtle,
    AccentSubtleText,
    OnAccent,
}

/// One resolved Yue Design System token for the selected theme profile.
///
/// The catalog keeps the original CSS value because the design system contains
/// colors, lengths, numbers, shadows, curves, and keywords.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YueToken {
    name: &'static str,
    namespace: &'static str,
    layer: &'static str,
    value: &'static str,
}

impl YueToken {
    pub const fn name(self) -> &'static str {
        self.name
    }
    pub const fn namespace(self) -> &'static str {
        self.namespace
    }
    pub const fn layer(self) -> &'static str {
        self.layer
    }
    pub const fn value(self) -> &'static str {
        self.value
    }

    pub fn color(self) -> Option<u32> {
        parse_hex(self.value)
    }

    pub fn rgba(self) -> Option<gpui_kit::Rgba> {
        if self.value == "transparent" {
            return Some(gpui_kit::rgba(0));
        }
        if let Some(color) = parse_hex(self.value) {
            return Some(gpui_kit::rgb(color));
        }
        let value = self
            .value
            .strip_prefix("color-mix(in srgb, ")?
            .strip_suffix(')')?;
        let (left, right) = value.split_once(", ")?;
        let (blend, amount) = left.split_once(" calc(")?;
        let amount = amount.strip_suffix("*100%)")?.parse::<f32>().ok()?;
        let blend = parse_hex(blend)?;
        if right == "transparent" {
            return Some(gpui_kit::rgba(
                (blend << 8) | ((amount * 255.0).round() as u32),
            ));
        }
        let background = parse_hex(right)?;
        Some(gpui_kit::rgb(mix_srgb(blend, background, amount)))
    }

    pub fn pixels(self) -> Option<f32> {
        let value = self.value.trim();
        if let Some(value) = value.strip_suffix("px") {
            return value.trim().parse().ok();
        }
        let value = value
            .strip_prefix("calc(")
            .and_then(|value| value.strip_suffix(')'))?;
        if let Some((left, right)) = value.split_once('/') {
            let left = left.trim().strip_suffix("px")?.trim().parse::<f32>().ok()?;
            let right = right.trim().parse::<f32>().ok()?;
            return Some(left / right);
        }
        if let Some((left, right)) = value.split_once('*') {
            let left = left.trim().strip_suffix("px")?.trim().parse::<f32>().ok()?;
            let right = right.trim().parse::<f32>().ok()?;
            return Some(left * right);
        }
        None
    }

    pub fn number(self) -> Option<f32> {
        self.value.parse().ok()
    }
}

fn parse_hex(value: &str) -> Option<u32> {
    let value = value.strip_prefix('#')?;
    let value = match value.len() {
        3 => value
            .chars()
            .map(|channel| format!("{channel}{channel}"))
            .collect::<String>(),
        6 => value.to_owned(),
        _ => return None,
    };
    u32::from_str_radix(&value, 16).ok()
}

/// Resolved Yue semantic colors for a theme/accent profile.
///
/// Values are packed RGB (`0xRRGGBB`) generated from the Yue Design System's
/// Token inventory. Convert to GPUI colors with `gpui_kit::rgb(theme.page())`.
#[derive(Clone, Copy, Debug)]
pub struct YueTheme {
    mode: ThemeMode,
    accent: Accent,
    colors: [u32; 21],
    hover_opacity: f32,
    pressed_opacity: f32,
}

impl YueTheme {
    pub const fn new(mode: ThemeMode, accent: Accent) -> Self {
        let colors = match (mode, accent) {
            (ThemeMode::Light, Accent::Azure) => generated::LIGHT_AZURE,
            (ThemeMode::Dark, Accent::Azure) => generated::DARK_AZURE,
            (ThemeMode::Light, Accent::Neutral) => generated::LIGHT_NEUTRAL,
            (ThemeMode::Dark, Accent::Neutral) => generated::DARK_NEUTRAL,
        };
        let (hover_opacity, pressed_opacity) = match (mode, accent) {
            (ThemeMode::Light, Accent::Azure) => (
                generated::LIGHT_AZURE_HOVER_OPACITY,
                generated::LIGHT_AZURE_PRESSED_OPACITY,
            ),
            (ThemeMode::Dark, Accent::Azure) => (
                generated::DARK_AZURE_HOVER_OPACITY,
                generated::DARK_AZURE_PRESSED_OPACITY,
            ),
            (ThemeMode::Light, Accent::Neutral) => (
                generated::LIGHT_NEUTRAL_HOVER_OPACITY,
                generated::LIGHT_NEUTRAL_PRESSED_OPACITY,
            ),
            (ThemeMode::Dark, Accent::Neutral) => (
                generated::DARK_NEUTRAL_HOVER_OPACITY,
                generated::DARK_NEUTRAL_PRESSED_OPACITY,
            ),
        };
        Self {
            mode,
            accent,
            colors,
            hover_opacity,
            pressed_opacity,
        }
    }

    pub const fn mode(self) -> ThemeMode {
        self.mode
    }

    pub const fn accent(self) -> Accent {
        self.accent
    }

    fn profile_index(self) -> usize {
        match (self.mode, self.accent) {
            (ThemeMode::Light, Accent::Azure) => 0,
            (ThemeMode::Dark, Accent::Azure) => 1,
            (ThemeMode::Light, Accent::Neutral) => 2,
            (ThemeMode::Dark, Accent::Neutral) => 3,
        }
    }

    /// Looks up any token from the complete generated Yue catalog.
    pub fn token(self, name: &str) -> Option<YueToken> {
        let profile = self.profile_index();
        generated::TOKENS
            .iter()
            .find(|(token_name, _, _, _)| *token_name == name)
            .map(|(token_name, namespace, layer, values)| YueToken {
                name: token_name,
                namespace,
                layer,
                value: values[profile],
            })
    }

    /// Iterates over every resolved token in the selected theme profile.
    pub fn all_tokens(self) -> impl Iterator<Item = YueToken> {
        let profile = self.profile_index();
        generated::TOKENS
            .iter()
            .map(move |(name, namespace, layer, values)| YueToken {
                name,
                namespace,
                layer,
                value: values[profile],
            })
    }

    pub const fn token_count(self) -> usize {
        generated::TOKEN_COUNT
    }

    pub const fn color(self, role: ColorRole) -> u32 {
        let index = match role {
            ColorRole::Page => 0,
            ColorRole::Surface => 1,
            ColorRole::SurfaceComponent => 2,
            ColorRole::ButtonDefaultBackground => 3,
            ColorRole::ButtonDefaultColor => 4,
            ColorRole::ButtonPrimaryBackground => 5,
            ColorRole::ButtonPrimaryColor => 6,
            ColorRole::ButtonSecondaryBackground => 7,
            ColorRole::ButtonSecondaryColor => 8,
            ColorRole::ButtonSecondaryBorder => 9,
            ColorRole::ButtonDangerBackground => 10,
            ColorRole::ButtonDangerColor => 11,
            ColorRole::TextPrimary => 12,
            ColorRole::TextSecondary => 13,
            ColorRole::BorderDefault => 14,
            ColorRole::FocusRing => 15,
            ColorRole::AccentSolid => 16,
            ColorRole::AccentText => 17,
            ColorRole::AccentSubtle => 18,
            ColorRole::AccentSubtleText => 19,
            ColorRole::OnAccent => 20,
        };
        self.colors[index]
    }

    pub const fn page(self) -> u32 {
        self.color(ColorRole::Page)
    }

    pub const fn surface(self) -> u32 {
        self.color(ColorRole::Surface)
    }

    pub const fn surface_component(self) -> u32 {
        self.color(ColorRole::SurfaceComponent)
    }

    pub const fn button_default_background(self) -> u32 {
        self.color(ColorRole::ButtonDefaultBackground)
    }
    pub const fn button_default_color(self) -> u32 {
        self.color(ColorRole::ButtonDefaultColor)
    }
    pub const fn button_primary_background(self) -> u32 {
        self.color(ColorRole::ButtonPrimaryBackground)
    }
    pub const fn button_primary_color(self) -> u32 {
        self.color(ColorRole::ButtonPrimaryColor)
    }
    pub const fn button_secondary_background(self) -> u32 {
        self.color(ColorRole::ButtonSecondaryBackground)
    }
    pub const fn button_secondary_color(self) -> u32 {
        self.color(ColorRole::ButtonSecondaryColor)
    }
    pub const fn button_secondary_border(self) -> u32 {
        self.color(ColorRole::ButtonSecondaryBorder)
    }
    pub const fn button_danger_background(self) -> u32 {
        self.color(ColorRole::ButtonDangerBackground)
    }
    pub const fn button_danger_color(self) -> u32 {
        self.color(ColorRole::ButtonDangerColor)
    }
    pub const fn hover_opacity(self) -> f32 {
        self.hover_opacity
    }
    pub const fn pressed_opacity(self) -> f32 {
        self.pressed_opacity
    }

    pub const fn text_primary(self) -> u32 {
        self.color(ColorRole::TextPrimary)
    }

    pub const fn text_secondary(self) -> u32 {
        self.color(ColorRole::TextSecondary)
    }

    pub const fn border_default(self) -> u32 {
        self.color(ColorRole::BorderDefault)
    }

    pub const fn focus_ring(self) -> u32 {
        self.color(ColorRole::FocusRing)
    }

    pub const fn accent_solid(self) -> u32 {
        self.color(ColorRole::AccentSolid)
    }

    pub const fn accent_text(self) -> u32 {
        self.color(ColorRole::AccentText)
    }

    pub const fn accent_subtle(self) -> u32 {
        self.color(ColorRole::AccentSubtle)
    }

    pub const fn accent_subtle_text(self) -> u32 {
        self.color(ColorRole::AccentSubtleText)
    }

    pub const fn on_accent(self) -> u32 {
        self.color(ColorRole::OnAccent)
    }
}

pub(crate) fn mix_srgb(foreground: u32, background: u32, amount: f32) -> u32 {
    let channel = |shift: u32| {
        let foreground = ((foreground >> shift) & 0xff_u32) as f32;
        let background = ((background >> shift) & 0xff_u32) as f32;
        (foreground * amount + background * (1.0 - amount)).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

#[cfg(test)]
mod tests {
    use super::{Accent, ColorRole, ThemeMode, YueTheme};

    #[test]
    fn all_yue_profiles_resolve_accent_and_surface_roles() {
        let light_azure = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let dark_azure = YueTheme::new(ThemeMode::Dark, Accent::Azure);
        let light_neutral = YueTheme::new(ThemeMode::Light, Accent::Neutral);
        let dark_neutral = YueTheme::new(ThemeMode::Dark, Accent::Neutral);

        assert_eq!(light_azure.color(ColorRole::Page), 0xf5f5f5);
        assert_eq!(dark_azure.color(ColorRole::Page), 0x101010);
        assert_eq!(light_azure.accent_solid(), 0x1f75db);
        assert_eq!(dark_azure.focus_ring(), 0x93c1ff);
        assert_eq!(light_neutral.accent_solid(), 0x1f1f1f);
        assert_eq!(dark_neutral.accent_solid(), 0xf5f5f5);
        assert_eq!(dark_neutral.on_accent(), 0x121212);
        assert_eq!(light_azure.button_danger_background(), 0xa14138);
        assert_eq!(light_azure.button_danger_color(), 0xffffff);
        assert_eq!(dark_azure.button_danger_background(), 0xf5aaa3);
        assert_eq!(dark_azure.button_danger_color(), 0x121212);
        assert_eq!(light_azure.all_tokens().count(), light_azure.token_count());
        assert!(light_azure.token_count() >= 646);
        let spacing = light_azure.token("--space-8").expect("space token");
        assert_eq!(spacing.namespace(), "space");
        assert_eq!(spacing.layer(), "primitives");
        assert_eq!(spacing.value(), "8px");
        assert_eq!(spacing.pixels(), Some(8.0));
        assert_eq!(
            light_azure.token("--accent-600").unwrap().color(),
            Some(0x1f75db)
        );
    }
}
