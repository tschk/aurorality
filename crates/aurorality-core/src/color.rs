//! CSS color string → RGBA float components.
//!
//! Delegates to [`crepuscularity_native::resolve_rgba`] for full Tailwind v3 palette
//! support (`red-500`, `slate-700`, etc.) and basic CSS names, then converts the
//! `[f32; 4]` array into the UniFFI-friendly [`ResolvedColor`] record.

/// RGBA color components in 0.0–1.0 range.
/// Exposed to Swift via UniFFI as a record type.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ResolvedColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Resolve a CSS color string to RGBA components.
///
/// Supports (via `crepuscularity_native::resolve_rgba`):
/// - Basic CSS named colors: `red`, `blue`, `green`, `white`, `black`,
///   `gray`/`grey`, `clear`/`transparent`, `orange`, `yellow`, `purple`, `pink`
/// - Full Tailwind v3 palette: `red-500`, `slate-700`, `emerald-300`, etc.
/// - Hex literals: `#rrggbb` (alpha = 1.0) and `#rrggbbaa`
///
/// Returns `None` for unrecognised strings (including semantic tokens like
/// `"primary"` and `"secondary"`). Swift callers handle those as `Color.primary`
/// / `Color.secondary`.
pub fn resolve_color(css: &str) -> Option<ResolvedColor> {
    let [r, g, b, a] = crepuscularity_native::resolve_rgba(css)?;
    Some(ResolvedColor { r, g, b, a })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_red() {
        let c = resolve_color("red").unwrap();
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn clear_is_transparent() {
        let c = resolve_color("clear").unwrap();
        assert_eq!(c.a, 0.0);
        let c2 = resolve_color("transparent").unwrap();
        assert_eq!(c2.a, 0.0);
    }

    #[test]
    fn hex6() {
        let c = resolve_color("#ff0000").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g < 0.01);
    }

    #[test]
    fn hex8_with_alpha() {
        let c = resolve_color("#ff000080").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.a - 0.502).abs() < 0.01);
    }

    #[test]
    fn unknown_returns_none() {
        assert!(resolve_color("chartreuse-with-a-twist").is_none());
        assert!(resolve_color("primary").is_none());
        assert!(resolve_color("secondary").is_none());
    }

    #[test]
    fn case_insensitive() {
        assert!(resolve_color("RED").is_some());
        assert!(resolve_color("Blue").is_some());
        assert!(resolve_color("#FF0000").is_some());
    }

    #[test]
    fn tailwind_palette() {
        // Ensure wrapper stays aligned with upstream native resolver.
        let c = resolve_color("slate-500").unwrap();
        let [r, g, b, a] = crepuscularity_native::resolve_rgba("slate-500").unwrap();
        assert!((c.r - r).abs() < 0.0001);
        assert!((c.g - g).abs() < 0.0001);
        assert!((c.b - b).abs() < 0.0001);
        assert!((c.a - a).abs() < 0.0001);
        assert_eq!(c.a, 1.0);

        let c2 = resolve_color("red-500").unwrap();
        let [r2, g2, b2, a2] = crepuscularity_native::resolve_rgba("red-500").unwrap();
        assert!((c2.r - r2).abs() < 0.0001);
        assert!((c2.g - g2).abs() < 0.0001);
        assert!((c2.b - b2).abs() < 0.0001);
        assert!((c2.a - a2).abs() < 0.0001);
    }
}
