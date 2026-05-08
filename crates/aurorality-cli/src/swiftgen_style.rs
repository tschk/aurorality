//! Maps a small Tailwind-inspired class subset to SwiftUI modifier chains for `swiftgen`.

/// Modifiers applied after `Text(...)`.
pub fn text_modifiers(classes: &[String]) -> String {
    let mut s = String::new();
    if classes.iter().any(|c| c == "text-xl") {
        s.push_str("\n            .font(.title3)");
    } else if classes.iter().any(|c| c == "text-lg") {
        s.push_str("\n            .font(.headline)");
    } else if classes.iter().any(|c| c == "text-base") {
        s.push_str("\n            .font(.body)");
    } else if classes.iter().any(|c| c == "text-sm") {
        s.push_str("\n            .font(.callout)");
    } else if classes.iter().any(|c| c == "text-xs") {
        s.push_str("\n            .font(.caption)");
    }
    if classes.iter().any(|c| c == "font-bold") {
        s.push_str("\n            .fontWeight(.bold)");
    } else if classes.iter().any(|c| c == "font-semibold") {
        s.push_str("\n            .fontWeight(.semibold)");
    } else if classes.iter().any(|c| c == "font-medium") {
        s.push_str("\n            .fontWeight(.medium)");
    }
    if let Some(c) = classes.iter().find_map(|cl| text_color_class(cl)) {
        s.push_str(&format!(
            "\n            .foregroundStyle({})",
            color_expr(c)
        ));
    }
    s
}

/// Modifiers applied after a container (`VStack` / `HStack` / `ScrollView` / etc.).
pub fn container_modifiers(classes: &[String], for_scroll: bool) -> String {
    let mut s = String::new();
    if let Some(p) = padding_points(classes) {
        s.push_str(&format!("\n            .padding({p})"));
    }
    let r = corner_radius(classes);
    if let Some(bg) = background_color(classes) {
        let radius = r.unwrap_or(0.0);
        if radius > 0.0 {
            s.push_str(&format!(
                "\n            .background(RoundedRectangle(cornerRadius: {radius}, style: .continuous).fill({}))",
                color_expr(bg)
            ));
        } else {
            s.push_str(&format!("\n            .background({})", color_expr(bg)));
        }
    } else if let Some(radius) = r {
        s.push_str(&format!(
            "\n            .background(RoundedRectangle(cornerRadius: {radius}, style: .continuous).fill(Color.clear))"
        ));
    }
    if border_bottom_only(classes) {
        if let Some(c) = border_color_from_classes(classes) {
            s.push_str(&format!(
                "\n            .overlay(alignment: .bottom) {{\n                Rectangle()\n                    .fill({})\n                    .frame(height: 1)\n            }}",
                color_expr(c)
            ));
        }
    } else if has_full_border(classes) {
        let stroke = border_color_from_classes(classes).unwrap_or(Zinc::Z300);
        let rad = r.unwrap_or(6.0);
        s.push_str(&format!(
            "\n            .overlay(\n                RoundedRectangle(cornerRadius: {rad}, style: .continuous)\n                    .stroke({}, lineWidth: 1)\n            )",
            color_expr(stroke)
        ));
    }
    if classes.iter().any(|c| c == "flex-grow" || c == "flex-1") {
        s.push_str("\n            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)");
    } else if for_scroll {
        s.push_str("\n            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)");
    }
    if let Some(fr) = width_frame(classes) {
        s.push_str(fr);
    }
    s
}

/// Top-level fill so the template occupies the window.
pub fn root_frame() -> &'static str {
    "\n        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)"
}

fn padding_points(classes: &[String]) -> Option<f64> {
    for c in classes {
        if let Some(rest) = c.strip_prefix("p-") {
            return match rest {
                "0" => Some(0.0),
                "0.5" => Some(2.0),
                "1" => Some(4.0),
                "1.5" => Some(6.0),
                "2" => Some(8.0),
                "2.5" => Some(10.0),
                "3" => Some(12.0),
                "4" => Some(16.0),
                "6" => Some(24.0),
                "8" => Some(32.0),
                _ => None,
            };
        }
    }
    None
}

fn corner_radius(classes: &[String]) -> Option<f64> {
    if classes.iter().any(|c| c == "rounded-2xl") {
        return Some(16.0);
    }
    if classes.iter().any(|c| c == "rounded-xl") {
        return Some(12.0);
    }
    if classes.iter().any(|c| c == "rounded-lg") {
        return Some(8.0);
    }
    if classes.iter().any(|c| c == "rounded-md") {
        return Some(6.0);
    }
    if classes.iter().any(|c| c == "rounded-sm") {
        return Some(4.0);
    }
    if classes.iter().any(|c| c == "rounded") {
        return Some(8.0);
    }
    if classes.iter().any(|c| c == "rounded-full") {
        return Some(999.0);
    }
    None
}

#[derive(Clone, Copy)]
pub enum Zinc {
    Z50,
    Z100,
    Z200,
    Z300,
    Z500,
    Z600,
    Z700,
    Z900,
    Z950,
    Blue700,
    White,
}

pub fn color_expr(c: Zinc) -> &'static str {
    match c {
        Zinc::Z50 => "Color(nsColor: .windowBackgroundColor)",
        Zinc::Z100 => "Color(nsColor: .underPageBackgroundColor)",
        Zinc::Z200 => "Color(nsColor: .controlBackgroundColor)",
        Zinc::Z300 => "Color(nsColor: .separatorColor)",
        Zinc::Z500 => "Color.secondary",
        Zinc::Z600 => "Color.secondary",
        Zinc::Z700 => "Color.primary",
        Zinc::Z900 => "Color.primary",
        Zinc::Z950 => "Color.primary",
        Zinc::Blue700 => "Color.accentColor",
        Zinc::White => "Color.white",
    }
}

fn background_color(classes: &[String]) -> Option<Zinc> {
    for c in classes {
        match c.as_str() {
            "bg-white" => return Some(Zinc::White),
            "bg-zinc-50" => return Some(Zinc::Z50),
            "bg-zinc-100" => return Some(Zinc::Z100),
            "bg-zinc-200" => return Some(Zinc::Z200),
            "bg-blue-700" => return Some(Zinc::Blue700),
            _ => {}
        }
    }
    None
}

fn text_color_class(c: &str) -> Option<Zinc> {
    match c {
        "text-white" => Some(Zinc::White),
        "text-zinc-500" => Some(Zinc::Z500),
        "text-zinc-600" => Some(Zinc::Z600),
        "text-zinc-700" => Some(Zinc::Z700),
        "text-zinc-900" => Some(Zinc::Z900),
        "text-zinc-950" => Some(Zinc::Z950),
        "text-blue-700" => Some(Zinc::Blue700),
        _ => None,
    }
}

fn border_bottom_only(classes: &[String]) -> bool {
    classes.iter().any(|c| c == "border-b")
}

fn has_full_border(classes: &[String]) -> bool {
    classes.iter().any(|c| c == "border") && !border_bottom_only(classes)
}

fn border_color_from_classes(classes: &[String]) -> Option<Zinc> {
    for c in classes {
        match c.as_str() {
            "border-zinc-200" => return Some(Zinc::Z200),
            "border-zinc-300" => return Some(Zinc::Z300),
            _ => {}
        }
    }
    None
}

/// TextField — bordered “rounded” style like the crepus example; `pill` adds iMessage-like capsule chrome.
pub fn text_field_extras(classes: &[String]) -> String {
    let mut s = String::new();
    if classes.iter().any(|c| c == "pill") {
        s.push_str(
            "\n            .textFieldStyle(.plain)\n            .padding(.horizontal, 12)\n            .padding(.vertical, 8)\n            .background(RoundedRectangle(cornerRadius: 18, style: .continuous).fill(Color(nsColor: .textBackgroundColor)))",
        );
    } else if classes.iter().any(|c| c == "input-plain") {
        s.push_str("\n            .textFieldStyle(.plain)");
    } else {
        s.push_str("\n            .textFieldStyle(.roundedBorder)");
    }
    s
}

pub fn button_extras(classes: &[String]) -> &'static str {
    if classes
        .iter()
        .any(|c| c == "button-prominent" || c == "prominent")
    {
        "\n            .buttonStyle(.borderedProminent)"
    } else if classes.iter().any(|c| c == "button-plain" || c == "plain") {
        "\n            .buttonStyle(.plain)"
    } else if classes
        .iter()
        .any(|c| c == "button-bordered" || c == "bordered")
    {
        "\n            .buttonStyle(.bordered)"
    } else {
        ""
    }
}

pub fn list_extras(classes: &[String]) -> &'static str {
    if classes.iter().any(|c| c == "sidebar") {
        "\n            .listStyle(.sidebar)"
    } else if classes.iter().any(|c| c == "inset") {
        "\n            .listStyle(.inset)"
    } else {
        ""
    }
}

fn width_frame(classes: &[String]) -> Option<&'static str> {
    if classes.iter().any(|c| c == "w-80") {
        return Some("\n            .frame(width: 320, alignment: .leading)");
    }
    if classes.iter().any(|c| c == "w-72") {
        return Some("\n            .frame(width: 288, alignment: .leading)");
    }
    if classes.iter().any(|c| c == "w-64") {
        return Some("\n            .frame(width: 256, alignment: .leading)");
    }
    None
}
