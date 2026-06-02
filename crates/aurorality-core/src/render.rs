//! Thin wrapper around `crepuscularity_native` rendering.

use crepuscularity_core::context::{TemplateContext, TemplateValue};
use crepuscularity_native::{render_template_to_ir, to_json};
use serde_json::Value;

use crate::AurorError;

/// Parse a `.crepus` template string and a JSON context object, returning the
/// `ViewIr` serialized as a compact JSON string.
///
/// `context_json` must be a JSON object whose keys become template variables.
/// Pass `"{}"` for an empty context.
pub fn render(template: &str, context_json: &str) -> Result<String, AurorError> {
    let ctx = context_from_json(context_json)?;
    let ir = render_template_to_ir(template, &ctx)
        .map_err(|e| AurorError::RenderError { message: e })?;
    to_json(&ir).map_err(|e| AurorError::RenderError {
        message: e.to_string(),
    })
}

fn context_from_json(json: &str) -> Result<TemplateContext, AurorError> {
    let v: Value = serde_json::from_str(json).map_err(|e| AurorError::InvalidContext {
        message: e.to_string(),
    })?;

    let obj = v.as_object().ok_or_else(|| AurorError::InvalidContext {
        message: "context_json must be a JSON object".to_string(),
    })?;

    let mut ctx = TemplateContext::new();
    for (k, v) in obj {
        if let Some(tv) = json_value_to_template_value(v) {
            ctx.vars.insert(k.clone(), tv);
        }
    }
    Ok(ctx)
}

/// Convert a JSON value to a TemplateValue.
/// Arrays of objects become `List(Vec<TemplateContext>)` — each object's
/// fields become that context's variables.  Arrays of primitives and nested
/// objects are not supported by the template engine; they are silently dropped.
fn json_value_to_template_value(v: &Value) -> Option<TemplateValue> {
    match v {
        Value::Bool(b) => Some(TemplateValue::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(TemplateValue::Int(i))
            } else {
                n.as_f64().map(TemplateValue::Float)
            }
        }
        Value::String(s) => Some(TemplateValue::Str(s.clone())),
        Value::Array(arr) => {
            let contexts: Vec<TemplateContext> = arr
                .iter()
                .filter_map(|item| {
                    item.as_object().map(|obj| {
                        let mut child = TemplateContext::new();
                        for (k, v) in obj {
                            if let Some(tv) = json_value_to_template_value(v) {
                                child.vars.insert(k.clone(), tv);
                            }
                        }
                        child
                    })
                })
                .collect();
            Some(TemplateValue::List(contexts))
        }
        Value::Null => Some(TemplateValue::Null),
        Value::Object(_) => None, // nested objects not supported as standalone values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_from_json_empty() {
        let ctx = context_from_json("{}").unwrap();
        assert!(ctx.vars.is_empty());
    }

    #[test]
    fn test_context_from_json_valid_types() {
        let json = r#"{
            "bool": true,
            "int": 42,
            "float": 3.14,
            "string": "hello",
            "null": null,
            "array": [{"a": 1}, {"b": 2}],
            "ignored_nested_obj": {"x": 1},
            "array_of_primitives": [1, 2]
        }"#;

        let ctx = context_from_json(json).unwrap();

        assert!(matches!(ctx.vars.get("bool"), Some(TemplateValue::Bool(true))));
        assert!(matches!(ctx.vars.get("int"), Some(TemplateValue::Int(42))));
        // matches for float can be problematic due to precision, but here we expect exact conversion
        if let Some(TemplateValue::Float(f)) = ctx.vars.get("float") {
            assert!((f - 3.14).abs() < 1e-6);
        } else {
            panic!("Expected Float");
        }

        if let Some(TemplateValue::Str(s)) = ctx.vars.get("string") {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected Str");
        }

        assert!(matches!(ctx.vars.get("null"), Some(TemplateValue::Null)));

        // array
        if let Some(TemplateValue::List(list)) = ctx.vars.get("array") {
            assert_eq!(list.len(), 2);
            assert!(matches!(list[0].vars.get("a"), Some(TemplateValue::Int(1))));
            assert!(matches!(list[1].vars.get("b"), Some(TemplateValue::Int(2))));
        } else {
            panic!("Expected List");
        }

        // ignored_nested_obj
        assert!(ctx.vars.get("ignored_nested_obj").is_none());

        // array_of_primitives: primitives in arrays are ignored (filter_map skips them)
        if let Some(TemplateValue::List(list)) = ctx.vars.get("array_of_primitives") {
            assert_eq!(list.len(), 0);
        } else {
            panic!("Expected empty List for array of primitives");
        }
    }

    #[test]
    fn test_context_from_json_invalid_json() {
        let err = context_from_json("{ invalid }").unwrap_err();
        match err {
            AurorError::InvalidContext { .. } => {}
            _ => panic!("Expected InvalidContext error"),
        }
    }

    #[test]
    fn test_context_from_json_not_an_object() {
        let err = context_from_json("[]").unwrap_err();
        match err {
            AurorError::InvalidContext { .. } => {}
            _ => panic!("Expected InvalidContext error"),
        }

        let err2 = context_from_json("\"string\"").unwrap_err();
        match err2 {
            AurorError::InvalidContext { .. } => {}
            _ => panic!("Expected InvalidContext error"),
        }
    }
}
