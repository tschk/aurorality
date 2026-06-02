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

    let obj = match v {
        Value::Object(o) => o,
        _ => return Err(AurorError::InvalidContext {
            message: "context_json must be a JSON object".to_string(),
        }),
    };

    let mut ctx = TemplateContext::new();
    for (k, v) in obj {
        if let Some(tv) = json_value_to_template_value(v) {
            ctx.vars.insert(k, tv);
        }
    }
    Ok(ctx)
}

/// Convert a JSON value to a TemplateValue.
/// Arrays of objects become `List(Vec<TemplateContext>)` — each object's
/// fields become that context's variables.  Arrays of primitives and nested
/// objects are not supported by the template engine; they are silently dropped.
fn json_value_to_template_value(v: Value) -> Option<TemplateValue> {
    match v {
        Value::Bool(b) => Some(TemplateValue::Bool(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(TemplateValue::Int(i))
            } else {
                n.as_f64().map(TemplateValue::Float)
            }
        }
        Value::String(s) => Some(TemplateValue::Str(s)),
        Value::Array(arr) => {
            let contexts: Vec<TemplateContext> = arr
                .into_iter()
                .filter_map(|item| {
                    if let Value::Object(obj) = item {
                        let mut child = TemplateContext::new();
                        for (k, v) in obj {
                            if let Some(tv) = json_value_to_template_value(v) {
                                child.vars.insert(k, tv);
                            }
                        }
                        Some(child)
                    } else {
                        None
                    }
                })
                .collect();
            Some(TemplateValue::List(contexts))
        }
        Value::Null => Some(TemplateValue::Null),
        Value::Object(_) => None, // nested objects not supported as standalone values
    }
}
