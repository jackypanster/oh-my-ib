//! Output layer. Success → stdout (JSON default | table). Error → a JSON envelope
//! on stderr, always, so failures are machine-detectable regardless of --format.

use serde_json::Value;

use crate::cli::Format;
use crate::error::AppError;

pub fn emit_success(value: &Value, format: Format) {
    match format {
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            );
        }
        Format::Table => print!("{}", render_table(value)),
    }
}

/// The structured error envelope: `{"error":{"code","message","context"?}}`.
pub fn error_envelope(err: &AppError) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("code".to_string(), Value::from(err.code()));
    obj.insert("message".to_string(), Value::from(err.message.clone()));
    if let Some(ctx) = &err.context {
        obj.insert("context".to_string(), Value::from(ctx.clone()));
    }
    let mut root = serde_json::Map::new();
    root.insert("error".to_string(), Value::Object(obj));
    Value::Object(root)
}

/// Errors always serialize as compact JSON to stderr (the agent branches on this).
pub fn emit_error(err: &AppError) {
    let env = error_envelope(err);
    match serde_json::to_string(&env) {
        Ok(s) => eprintln!("{s}"),
        Err(_) => eprintln!("{{\"error\":{{\"code\":\"{}\"}}}}", err.code()),
    }
}

/// Minimal human-readable rendering. Not part of the frozen contract.
fn render_table(value: &Value) -> String {
    let mut out = String::new();
    render_value("", value, &mut out);
    out
}

fn render_value(prefix: &str, value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                render_value(&key, v, out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                render_value(&format!("{prefix}[{i}]"), v, out);
            }
        }
        scalar => {
            let s = match scalar {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            out.push_str(&format!("{prefix:<32} {s}\n"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_envelope_shape() {
        let err = AppError::connection("cannot connect", "127.0.0.1:65000");
        let v = error_envelope(&err);
        assert_eq!(v["error"]["code"], "connection");
        assert_eq!(v["error"]["message"], "cannot connect");
        assert_eq!(v["error"]["context"], "127.0.0.1:65000");
    }

    #[test]
    fn other_envelope_has_no_context() {
        let err = AppError::other("boom");
        let v = error_envelope(&err);
        assert_eq!(v["error"]["code"], "error");
        assert!(v["error"].get("context").is_none());
    }
}
