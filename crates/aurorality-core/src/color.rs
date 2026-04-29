//! CSS color string → RGBA float components.

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
/// Supports:
/// - Named colors: `red`, `blue`, `green`, `white`, `black`, `gray`/`grey`,
///   `clear`/`transparent`, `orange`, `yellow`, `purple`, `pink`
/// - Hex: `#rrggbb` (alpha = 1.0) and `#rrggbbaa`
///
/// Returns `None` for unrecognised strings. Callers should fall back to
/// `Color.primary` / `Color.secondary` as needed in Swift.
pub fn resolve_color(css: &str) -> Option<ResolvedColor> {
    match css.trim().to_lowercase().as_str() {
        "red"                       => Some(rgba(1.0, 0.0, 0.0, 1.0)),
        "blue"                      => Some(rgba(0.0, 0.0, 1.0, 1.0)),
        "green"                     => Some(rgba(0.0, 0.502, 0.0, 1.0)),
        "white"                     => Some(rgba(1.0, 1.0, 1.0, 1.0)),
        "black"                     => Some(rgba(0.0, 0.0, 0.0, 1.0)),
        "gray" | "grey"             => Some(rgba(0.502, 0.502, 0.502, 1.0)),
        "clear" | "transparent"     => Some(rgba(0.0, 0.0, 0.0, 0.0)),
        "orange"                    => Some(rgba(1.0, 0.647, 0.0, 1.0)),
        "yellow"                    => Some(rgba(1.0, 1.0, 0.0, 1.0)),
        "purple"                    => Some(rgba(0.502, 0.0, 0.502, 1.0)),
        "pink"                      => Some(rgba(1.0, 0.753, 0.796, 1.0)),
        // "primary" and "secondary" are semantic; can't map to RGBA — return None
        // Swift side handles those as special cases
        _ if css.starts_with('#')   => parse_hex(css),
        _                           => None,
    }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> ResolvedColor {
    ResolvedColor { r, g, b, a }
}

fn parse_hex(s: &str) -> Option<ResolvedColor> {
    let hex = s.trim_start_matches('#');
    match hex.len() {
        6 => {
            let n = u32::from_str_radix(hex, 16).ok()?;
            Some(rgba(
                ((n >> 16) & 0xFF) as f32 / 255.0,
                ((n >> 8)  & 0xFF) as f32 / 255.0,
                (n         & 0xFF) as f32 / 255.0,
                1.0,
            ))
        }
        8 => {
            let n = u32::from_str_radix(hex, 16).ok()?;
            Some(rgba(
                ((n >> 24) & 0xFF) as f32 / 255.0,
                ((n >> 16) & 0xFF) as f32 / 255.0,
                ((n >> 8)  & 0xFF) as f32 / 255.0,
                (n         & 0xFF) as f32 / 255.0,
            ))
        }
        _ => None,
    }
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
}
