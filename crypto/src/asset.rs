//! Asset types and identifiers.

mod denom;
mod id;
mod registry;

pub use denom::{BaseDenom, DisplayDenom};
pub use id::Id;
pub use registry::{Registry, REGISTRY};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_native_token() {
        // We should be able to use `parse_base` with the valid base denomination.
        let base_denom = REGISTRY.parse_base("upenumbra").unwrap();
        assert_eq!(format!("{}", base_denom), "upenumbra".to_string());

        // If we try to use `parse_base` with a display denomination, we should get `None`.
        let display_denoms = vec!["mpenumbra", "penumbra"];
        for display_denom in &display_denoms {
            assert!(REGISTRY.parse_base(display_denom).is_none());
        }

        // We should be able to use the display denominations with `parse_display` however.
        for display_denom in display_denoms {
            let parsed_display_denom = REGISTRY.parse_display(display_denom).unwrap();

            assert_eq!(
                format!("{}", parsed_display_denom.base()),
                "upenumbra".to_string()
            );

            assert_eq!(format!("{}", parsed_display_denom), display_denom)
        }

        // The base denomination (upenumbra) can also be used for display purposes.
        let parsed_display_denom = REGISTRY.parse_display("upenumbra").unwrap();
        assert_eq!(
            format!("{}", parsed_display_denom.base()),
            "upenumbra".to_string()
        );
    }

    #[test]
    fn test_displaydenom_format_value() {
        // with exponent 6, 1782000 formats to 1.782
        let penumbra_display_denom = REGISTRY.parse_display("penumbra").unwrap();
        assert_eq!(penumbra_display_denom.format_value(1782000), "1.782");

        // with exponent 3, 1782000 formats to 1782
        let mpenumbra_display_denom = REGISTRY.parse_display("mpenumbra").unwrap();
        assert_eq!(mpenumbra_display_denom.format_value(1782000), "1782");

        // with exponent 0, 1782000 formats to 1782000
        let upenumbra_display_denom = REGISTRY.parse_display("upenumbra").unwrap();
        assert_eq!(upenumbra_display_denom.format_value(1782000), "1782000");
    }

    #[test]
    fn test_registry_fallthrough() {
        // We should be able to use `parse_base` with a base denomination for assets
        // not included in the hardcoded registry.
        let base_denom = REGISTRY.parse_base("cube").unwrap();
        assert_eq!(format!("{}", base_denom), "cube".to_string());
    }
}
