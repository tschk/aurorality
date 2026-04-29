//! IR tree mutation engine — applies `IrMutation` patches to a `ViewIr` JSON string.
//!
//! Operates on `serde_json::Value` directly to stay in sync with the
//! JSON schema produced by `crepuscularity_native` without duplicating types.

use serde_json::{json, Value};

/// Apply a JSON-encoded `IrMutation[]` to a JSON-encoded `ViewIr`.
/// Returns the updated `ViewIr` JSON string.
pub fn apply(ir_json: &str, mutations_json: &str) -> Result<String, String> {
    let mut ir: Value =
        serde_json::from_str(ir_json).map_err(|e| format!("bad IR JSON: {e}"))?;
    let mutations: Value =
        serde_json::from_str(mutations_json).map_err(|e| format!("bad mutations JSON: {e}"))?;

    let muts = mutations
        .as_array()
        .ok_or("mutations must be a JSON array")?;

    for m in muts {
        apply_one(&mut ir, m)?;
    }

    serde_json::to_string(&ir).map_err(|e| e.to_string())
}

fn apply_one(ir: &mut Value, m: &Value) -> Result<(), String> {
    let op = m["op"].as_str().unwrap_or("");
    let root = ir["root"]
        .as_array_mut()
        .ok_or("IR missing root array")?;

    match op {
        "replaceRoot" => {
            if let Some(new_root) = m.get("root") {
                ir["root"] = new_root.clone();
            }
        }
        "replaceNode" => {
            let path = int_path(m)?;
            let node = m.get("node").ok_or("replaceNode missing node")?;
            replace_at(root, &path, node.clone());
        }
        "insertNode" => {
            let parent_path = int_path_field(m, "parentPath")?;
            let idx = m["index"].as_u64().unwrap_or(0) as usize;
            let node = m.get("node").ok_or("insertNode missing node")?;
            insert_at(root, &parent_path, idx, node.clone());
        }
        "removeNode" => {
            let path = int_path(m)?;
            remove_at(root, &path);
        }
        "updateText" => {
            let path = int_path(m)?;
            let content = m["content"].as_str().unwrap_or("").to_string();
            update_field_at(root, &path, "content", json!(content));
        }
        "updateStyle" => {
            let path = int_path(m)?;
            // m["style"] may be null (remove style) or an object (set style)
            let style = m.get("style").cloned().unwrap_or(Value::Null);
            if style.is_null() {
                update_field_at(root, &path, "style", Value::Null);
            } else {
                update_field_at(root, &path, "style", style);
            }
        }
        _ => {} // unknown op: ignore (forward-compat)
    }
    Ok(())
}

// ── Path helpers ─────────────────────────────────────────────────────────────

fn int_path(m: &Value) -> Result<Vec<usize>, String> {
    int_path_field(m, "path")
}

fn int_path_field(m: &Value, field: &str) -> Result<Vec<usize>, String> {
    m.get(field)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|n| n.as_u64().unwrap_or(0) as usize).collect())
        .ok_or_else(|| format!("mutation missing or invalid {field:?}"))
}

// ── Tree operations ───────────────────────────────────────────────────────────

// NOTE: The raw pointer casts below (`as *mut Vec<Value>`) are needed to work
// around the borrow checker when recursing through `serde_json::Value`. They
// are safe because we only ever follow a single path through the tree at a time
// and never create aliasing mutable references.

#[allow(clippy::ptr_arg)]
fn replace_at(nodes: &mut Vec<Value>, path: &[usize], replacement: Value) {
    let Some(&first) = path.first() else { return };
    if first >= nodes.len() { return }
    if path.len() == 1 {
        nodes[first] = replacement;
    } else {
        let children = nodes[first]["children"]
            .as_array_mut()
            .map(|c| c as *mut Vec<Value>);
        if let Some(children) = children {
            replace_at(unsafe { &mut *children }, &path[1..], replacement);
        }
    }
}

fn insert_at(nodes: &mut Vec<Value>, parent_path: &[usize], idx: usize, node: Value) {
    if parent_path.is_empty() {
        let safe_idx = idx.min(nodes.len());
        nodes.insert(safe_idx, node);
        return;
    }
    let Some(&first) = parent_path.first() else { return };
    if first >= nodes.len() { return }
    let children = nodes[first]["children"]
        .as_array_mut()
        .map(|c| c as *mut Vec<Value>);
    if let Some(children) = children {
        insert_at(unsafe { &mut *children }, &parent_path[1..], idx, node);
    }
}

fn remove_at(nodes: &mut Vec<Value>, path: &[usize]) {
    let Some(&first) = path.first() else { return };
    if first >= nodes.len() { return }
    if path.len() == 1 {
        nodes.remove(first);
        return;
    }
    let children = nodes[first]["children"]
        .as_array_mut()
        .map(|c| c as *mut Vec<Value>);
    if let Some(children) = children {
        remove_at(unsafe { &mut *children }, &path[1..]);
    }
}

#[allow(clippy::ptr_arg)]
fn update_field_at(nodes: &mut Vec<Value>, path: &[usize], field: &str, value: Value) {
    let Some(&first) = path.first() else { return };
    if first >= nodes.len() { return }
    if path.len() == 1 {
        nodes[first][field] = value;
        return;
    }
    let children = nodes[first]["children"]
        .as_array_mut()
        .map(|c| c as *mut Vec<Value>);
    if let Some(children) = children {
        update_field_at(unsafe { &mut *children }, &path[1..], field, value);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_ir() -> &'static str {
        r#"{"version":2,"root":[{"kind":"text","content":"hello"},{"kind":"text","content":"world"}]}"#
    }

    #[test]
    fn replace_root() {
        let muts = r#"[{"op":"replaceRoot","root":[{"kind":"text","content":"new"}]}]"#;
        let result = apply(simple_ir(), muts).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["root"][0]["content"], "new");
        assert_eq!(v["root"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn update_text() {
        let muts = r#"[{"op":"updateText","path":[0],"content":"updated"}]"#;
        let result = apply(simple_ir(), muts).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["root"][0]["content"], "updated");
        assert_eq!(v["root"][1]["content"], "world"); // unchanged
    }

    #[test]
    fn insert_then_remove() {
        let muts = r#"[{"op":"insertNode","parentPath":[],"index":1,"node":{"kind":"text","content":"mid"}}]"#;
        let result = apply(simple_ir(), muts).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["root"].as_array().unwrap().len(), 3);
        assert_eq!(v["root"][1]["content"], "mid");

        let muts2 = r#"[{"op":"removeNode","path":[1]}]"#;
        let result2 = apply(&result, muts2).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&result2).unwrap();
        assert_eq!(v2["root"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn unknown_op_is_noop() {
        let muts = r#"[{"op":"futureOp","path":[0]}]"#;
        let result = apply(simple_ir(), muts).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["root"][0]["content"], "hello");
    }
}
