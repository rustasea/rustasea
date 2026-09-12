//! Supported starter-kit presentation variants.
//!
//! The shared auth/domain core is byte-identical across variants (ADR-0002
//! decision 1); only `resources/` and the generated `Cargo.toml` feature set
//! change. This module is the single source of truth for those deltas.

use std::str::FromStr;

use crate::error::ScaffoldError;

/// Presentation strategies a generated starter kit can adopt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterKitVariant {
    /// Server-rendered askama views (`resources/views`), umbrella feature `view`.
    Blade,
    /// Dioxus WASM + Inertia (`resources/js`), umbrella features `inertia`, `wasm-dioxus`.
    React,
    /// Leptos WASM + Inertia (`resources/js`), umbrella features `inertia`, `wasm-leptos`.
    Vue,
    /// askama + HTMX + broadcast (`resources/views`), umbrella features `view`, `broadcast`.
    Livewire,
}

impl StarterKitVariant {
    /// Every variant, in CLI help order.
    pub const ALL: [StarterKitVariant; 4] = [
        StarterKitVariant::Blade,
        StarterKitVariant::React,
        StarterKitVariant::Vue,
        StarterKitVariant::Livewire,
    ];

    /// Comma-separated list of accepted variant tokens.
    pub const SUPPORTED: &'static str = "blade, react, vue, livewire";

    /// Lowercase token used on the command line and in generated files.
    pub const fn as_str(self) -> &'static str {
        match self {
            StarterKitVariant::Blade => "blade",
            StarterKitVariant::React => "react",
            StarterKitVariant::Vue => "vue",
            StarterKitVariant::Livewire => "livewire",
        }
    }

    /// `rustasea` umbrella feature set that wires this variant's presentation.
    pub const fn cargo_features(self) -> &'static [&'static str] {
        match self {
            StarterKitVariant::Blade => &["view"],
            StarterKitVariant::React => &["inertia", "wasm-dioxus"],
            StarterKitVariant::Vue => &["inertia", "wasm-leptos"],
            StarterKitVariant::Livewire => &["view", "broadcast"],
        }
    }

    /// Whether the variant renders server-side askama templates.
    pub const fn uses_askama(self) -> bool {
        matches!(self, StarterKitVariant::Blade | StarterKitVariant::Livewire)
    }

    /// Whether the variant uses the Inertia server/WASM contract.
    pub const fn uses_inertia(self) -> bool {
        matches!(self, StarterKitVariant::React | StarterKitVariant::Vue)
    }

    /// Whether the variant enhances views with HTMX partial swaps.
    pub const fn uses_htmx(self) -> bool {
        matches!(self, StarterKitVariant::Livewire)
    }

    /// WASM framework feature required by `rustasea-inertia-adapters`, if any.
    pub const fn wasm_feature(self) -> Option<&'static str> {
        match self {
            StarterKitVariant::React => Some("react"),
            StarterKitVariant::Vue => Some("vue"),
            StarterKitVariant::Blade | StarterKitVariant::Livewire => None,
        }
    }
}

impl FromStr for StarterKitVariant {
    type Err = ScaffoldError;

    /// Parse a variant token, rejecting anything outside the four kits.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "blade" => Ok(StarterKitVariant::Blade),
            "react" => Ok(StarterKitVariant::React),
            "vue" => Ok(StarterKitVariant::Vue),
            "livewire" => Ok(StarterKitVariant::Livewire),
            _ => Err(ScaffoldError::UnknownVariant {
                variant: value.to_string(),
                supported: Self::SUPPORTED.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for StarterKitVariant {
    /// Render the lowercase CLI token.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::StarterKitVariant;
    use std::str::FromStr;

    #[test]
    fn parses_all_tokens() {
        for variant in StarterKitVariant::ALL {
            assert_eq!(
                StarterKitVariant::from_str(variant.as_str()).expect("known variant"),
                variant
            );
        }
    }

    #[test]
    fn rejects_unknown_variant() {
        let err = StarterKitVariant::from_str("svelte").expect_err("must reject");
        let message = err.to_string();
        assert!(message.contains("svelte"));
        assert!(message.contains("blade"));
    }

    #[test]
    fn feature_sets_match_blueprint() {
        assert_eq!(StarterKitVariant::Blade.cargo_features(), &["view"]);
        assert_eq!(
            StarterKitVariant::React.cargo_features(),
            &["inertia", "wasm-dioxus"]
        );
        assert_eq!(
            StarterKitVariant::Vue.cargo_features(),
            &["inertia", "wasm-leptos"]
        );
        assert_eq!(
            StarterKitVariant::Livewire.cargo_features(),
            &["view", "broadcast"]
        );
    }
}
