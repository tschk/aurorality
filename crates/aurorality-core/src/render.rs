//! Thin wrapper around `crepuscularity_native` rendering.

use crepuscularity_core::context::{TemplateContext, TemplateValue};
use crepuscularity_native::{render_template_to_ir, to_json};
use serde_json::Value;
use std::path::Path;

use crate::AurorError;

/// Parse a `.crepus` template string and a JSON context object, returning the
/// `ViewIr` serialized as a compact JSON string.
///
/// `context_json` must be a JSON object whose keys become template variables.
/// Pass `"{}"` for an empty context.
pub fn render(template: &str, context_json: &str) -> Result<String, AurorError> {
    render_with_context(template, context_from_json(context_json)?)
}

/// Render with filesystem includes confined to the given directory.
pub fn render_with_include_root(
    template: &str,
    context_json: &str,
    root: &Path,
) -> Result<String, AurorError> {
    let mut ctx = context_from_json(context_json)?;
    ctx.base_dir = Some(root.canonicalize().map_err(|e| AurorError::RenderError {
        message: format!("include root {}: {e}", root.display()),
    })?);
    render_with_context(template, ctx)
}

fn render_with_context(template: &str, ctx: TemplateContext) -> Result<String, AurorError> {
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

    let obj = match v {
        Value::Object(o) => o,
        _ => {
            return Err(AurorError::InvalidContext {
                message: "context_json must be a JSON object".to_string(),
            })
        }
    };

    let mut ctx = TemplateContext::new();
    for (k, v) in obj {
        let path = field_path("$", &k);
        ctx.vars.insert(k, json_value_to_template_value(v, &path)?);
    }
    Ok(ctx)
}

fn field_path(parent: &str, key: &str) -> String {
    if !key.is_empty()
        && key
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        format!("{parent}.{key}")
    } else {
        format!("{parent}[{}]", serde_json::to_string(key).unwrap())
    }
}

/// Convert a JSON value to a TemplateValue.
/// Arrays of objects become `List(Vec<TemplateContext>)` — each object's
/// fields become that context's variables. Nested objects and non-object array
/// elements are not supported by the template engine.
fn json_value_to_template_value(v: Value, path: &str) -> Result<TemplateValue, AurorError> {
    match v {
        Value::Bool(b) => Ok(TemplateValue::Bool(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(TemplateValue::Int(i))
            } else {
                n.as_f64()
                    .map(TemplateValue::Float)
                    .ok_or_else(|| AurorError::InvalidContext {
                        message: format!("{path}: number cannot be represented"),
                    })
            }
        }
        Value::String(s) => Ok(TemplateValue::Str(s)),
        Value::Array(arr) => {
            let mut contexts = Vec::with_capacity(arr.len());
            for (index, item) in arr.into_iter().enumerate() {
                let item_path = format!("{path}[{index}]");
                let Value::Object(obj) = item else {
                    return Err(AurorError::InvalidContext {
                        message: format!("{item_path}: array elements must be objects"),
                    });
                };
                let mut child = TemplateContext::new();
                for (k, v) in obj {
                    let child_path = field_path(&item_path, &k);
                    child
                        .vars
                        .insert(k, json_value_to_template_value(v, &child_path)?);
                }
                contexts.push(child);
            }
            Ok(TemplateValue::List(contexts))
        }
        Value::Null => Ok(TemplateValue::Null),
        Value::Object(_) => Err(AurorError::InvalidContext {
            message: format!("{path}: nested objects are not supported"),
        }),
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
            "float": 2.5,
            "string": "hello",
            "null": null,
            "array": [{"a": 1, "children": [{"name": "child"}]}, {"b": 2}],
            "empty": []
        }"#;

        let ctx = context_from_json(json).unwrap();

        assert!(matches!(
            ctx.vars.get("bool"),
            Some(TemplateValue::Bool(true))
        ));
        assert!(matches!(ctx.vars.get("int"), Some(TemplateValue::Int(42))));
        // matches for float can be problematic due to precision, but here we expect exact conversion
        if let Some(TemplateValue::Float(f)) = ctx.vars.get("float") {
            assert!((f - 2.5).abs() < 1e-6);
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
            assert!(
                matches!(list[0].vars.get("children"), Some(TemplateValue::List(children)) if matches!(children[0].vars.get("name"), Some(TemplateValue::Str(name)) if name == "child"))
            );
        } else {
            panic!("Expected List");
        }

        assert!(
            matches!(ctx.vars.get("empty"), Some(TemplateValue::List(list)) if list.is_empty())
        );
    }

    #[test]
    fn rejects_unsupported_values_with_paths() {
        for (json, path, reason) in [
            (r#"{"user":{"name":"Ada"}}"#, "$.user", "nested objects"),
            (r#"{"items":[{}, 1]}"#, "$.items[1]", "array elements"),
            (
                r#"{"items":[{"child":{"x":1}}]}"#,
                "$.items[0].child",
                "nested objects",
            ),
            (
                r#"{"items":[{"children":[{}, null]}]}"#,
                "$.items[0].children[1]",
                "array elements",
            ),
            (r#"{"a.b":[[]]}"#, "$[\"a.b\"][0]", "array elements"),
        ] {
            match render("", json).unwrap_err() {
                AurorError::InvalidContext { message } => {
                    assert!(message.contains(path), "{message}");
                    assert!(message.contains(reason), "{message}");
                }
                other => panic!("Expected InvalidContext, got {other:?}"),
            }
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
