//! Text content transforms (uppercase, lowercase, capitalize).

/// Apply a CSS `text-transform`-style transform to a string.
///
/// - `"uppercase"` → all caps
/// - `"lowercase"` → all lowercase
/// - `"capitalize"` → first letter of each whitespace-separated word capitalised
/// - anything else (including `""`) → returned unchanged
pub fn transform_text(content: &str, transform: &str) -> String {
    match transform {
        "uppercase" => content.to_uppercase(),
        "lowercase" => content.to_lowercase(),
        "capitalize" => capitalize_words(content),
        _ => content.to_string(),
    }
}

fn capitalize_words(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch.is_whitespace() {
            capitalize_next = true;
            result.push(ch);
        } else if capitalize_next {
            if ch.is_ascii() {
                result.push(ch.to_ascii_uppercase());
            } else {
                for c in ch.to_uppercase() {
                    result.push(c);
                }
            }
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase() {
        assert_eq!(transform_text("hello world", "uppercase"), "HELLO WORLD");
    }

    #[test]
    fn lowercase() {
        assert_eq!(transform_text("HELLO", "lowercase"), "hello");
    }

    #[test]
    fn capitalize() {
        assert_eq!(transform_text("hello world", "capitalize"), "Hello World");
    }

    #[test]
    fn capitalize_preserves_existing_case() {
        assert_eq!(transform_text("hELLO wORLD", "capitalize"), "HELLO WORLD");
    }

    #[test]
    fn empty_transform_passthrough() {
        assert_eq!(transform_text("Hello", ""), "Hello");
        assert_eq!(transform_text("Hello", "none"), "Hello");
    }

    #[test]
    fn empty_content() {
        assert_eq!(transform_text("", "uppercase"), "");
    }
}
