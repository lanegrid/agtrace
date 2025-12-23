pub fn format_compact(value: &serde_json::Value) -> String {
    if let Some(obj) = value.as_object() {
        let pairs: Vec<String> = obj
            .iter()
            .map(|(k, v)| {
                let value_str = match v {
                    serde_json::Value::String(s) => {
                        if s.len() > 50 {
                            let truncated: String = s.chars().take(47).collect();
                            let truncated_value =
                                serde_json::Value::String(format!("{}...", truncated));
                            serde_json::to_string(&truncated_value)
                                .unwrap_or_else(|_| "\"...\"".to_string())
                        } else {
                            serde_json::to_string(v).unwrap_or_else(|_| format!("\"{}\"", s))
                        }
                    }
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    serde_json::Value::Array(_) => "[...]".to_string(),
                    serde_json::Value::Object(_) => "{...}".to_string(),
                };
                format!("{}: {}", k, value_str)
            })
            .collect();

        if pairs.is_empty() {
            "{}".to_string()
        } else {
            pairs.join(", ")
        }
    } else {
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
    }
}
