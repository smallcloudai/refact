use serde_json::json;
use uuid::Uuid;

pub fn merge_tool_call(accumulated: &mut Vec<serde_json::Value>, new_tc: serde_json::Value) {
    let index = new_tc.get("index")
        .and_then(|i| {
            i.as_u64().or_else(|| i.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0) as usize;

    while accumulated.len() <= index {
        accumulated.push(json!({
            "type": "function",
            "function": {
                "name": "",
                "arguments": ""
            }
        }));
    }

    let existing = &mut accumulated[index];

    if let Some(id) = new_tc.get("id") {
        if !id.is_null() {
            if let Some(id_str) = id.as_str() {
                if !id_str.is_empty() {
                    existing["id"] = id.clone();
                }
            }
        }
    }

    if existing.get("id").map_or(true, |v| v.is_null() || v.as_str().map_or(true, |s| s.is_empty())) {
        existing["id"] = json!(format!("call_{}", Uuid::new_v4().to_string().replace("-", "")));
    }

    if let Some(t) = new_tc.get("type") {
        if !t.is_null() {
            existing["type"] = t.clone();
        }
    }
    if existing.get("type").map_or(true, |v| v.is_null()) {
        existing["type"] = json!("function");
    }

    if let Some(func) = new_tc.get("function") {
        if !func.is_null() {
            if existing.get("function").map_or(true, |v| v.is_null()) {
                existing["function"] = json!({"name": "", "arguments": ""});
            }

            if let Some(name) = func.get("name") {
                if !name.is_null() {
                    if let Some(name_str) = name.as_str() {
                        if !name_str.is_empty() {
                            existing["function"]["name"] = name.clone();
                        }
                    }
                }
            }

            if let Some(args) = func.get("arguments") {
                if !args.is_null() {
                    let new_args = args.as_str().unwrap_or("");
                    let prev_args = existing["function"]["arguments"].as_str().unwrap_or("");
                    existing["function"]["arguments"] = json!(format!("{}{}", prev_args, new_args));
                }
            }
        }
    }

    existing["index"] = json!(index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_tool_calls_basic() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_123",
            "type": "function",
            "function": {"name": "test", "arguments": "{\"a\":"}
        }));
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "function": {"arguments": " 1}"}
        }));

        assert_eq!(accumulated.len(), 1);
        assert_eq!(accumulated[0]["id"], "call_123");
        assert_eq!(accumulated[0]["function"]["name"], "test");
        assert_eq!(accumulated[0]["function"]["arguments"], "{\"a\": 1}");
    }

    #[test]
    fn test_merge_tool_calls_missing_id() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "type": "function",
            "function": {"name": "test", "arguments": "{}"}
        }));

        assert!(accumulated[0]["id"].as_str().unwrap().starts_with("call_"));
    }

    #[test]
    fn test_merge_tool_calls_parallel() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_1",
            "function": {"name": "func1", "arguments": "{}"}
        }));
        merge_tool_call(&mut accumulated, json!({
            "index": 1,
            "id": "call_2",
            "function": {"name": "func2", "arguments": "{}"}
        }));

        assert_eq!(accumulated.len(), 2);
        assert_eq!(accumulated[0]["function"]["name"], "func1");
        assert_eq!(accumulated[1]["function"]["name"], "func2");
    }

    #[test]
    fn test_merge_tool_calls_missing_index_defaults_to_zero() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "id": "call_no_index",
            "function": {"name": "test", "arguments": "{}"}
        }));

        assert_eq!(accumulated.len(), 1);
        assert_eq!(accumulated[0]["index"], 0);
    }

    #[test]
    fn test_merge_tool_calls_invalid_index_string_defaults_to_zero() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": "abc",
            "id": "call_bad_index",
            "function": {"name": "test", "arguments": "{}"}
        }));

        assert_eq!(accumulated.len(), 1);
        assert_eq!(accumulated[0]["index"], 0);
    }

    #[test]
    fn test_merge_tool_calls_numeric_string_index_parsed() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": "2",
            "id": "call_str_index",
            "function": {"name": "test", "arguments": "{}"}
        }));

        assert_eq!(accumulated.len(), 3);
        assert_eq!(accumulated[2]["index"], 2);
        assert_eq!(accumulated[2]["id"], "call_str_index");
    }

    #[test]
    fn test_merge_tool_calls_null_id_generates_uuid() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": null,
            "function": {"name": "test", "arguments": "{}"}
        }));

        let id = accumulated[0]["id"].as_str().unwrap();
        assert!(id.starts_with("call_"));
        assert!(id.len() > 10);
    }

    #[test]
    fn test_merge_tool_calls_empty_id_generates_uuid() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "",
            "function": {"name": "test", "arguments": "{}"}
        }));

        let id = accumulated[0]["id"].as_str().unwrap();
        assert!(id.starts_with("call_"));
    }

    #[test]
    fn test_merge_tool_calls_null_type_defaults_to_function() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_1",
            "type": null,
            "function": {"name": "test", "arguments": "{}"}
        }));

        assert_eq!(accumulated[0]["type"], "function");
    }

    #[test]
    fn test_merge_tool_calls_missing_function_creates_placeholder() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_1"
        }));

        assert_eq!(accumulated.len(), 1);
        assert!(accumulated[0].get("function").is_some());
        assert_eq!(accumulated[0]["function"]["name"], "");
        assert_eq!(accumulated[0]["function"]["arguments"], "");
    }

    #[test]
    fn test_merge_tool_calls_arguments_object_treated_as_empty() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_1",
            "function": {"name": "test", "arguments": {"key": "value"}}
        }));

        assert_eq!(accumulated[0]["function"]["arguments"], "");
    }

    #[test]
    fn test_merge_tool_calls_arguments_number_treated_as_empty() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_1",
            "function": {"name": "test", "arguments": 123}
        }));

        assert_eq!(accumulated[0]["function"]["arguments"], "");
    }

    #[test]
    fn test_merge_tool_calls_sparse_indices_creates_placeholders() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 2,
            "id": "call_2",
            "function": {"name": "test2", "arguments": "{}"}
        }));

        assert_eq!(accumulated.len(), 3);
        assert_eq!(accumulated[2]["id"], "call_2");
        assert_eq!(accumulated[2]["function"]["name"], "test2");
        assert_eq!(accumulated[0]["function"]["name"], "");
        assert_eq!(accumulated[1]["function"]["name"], "");
    }

    #[test]
    fn test_merge_tool_calls_preserves_existing_name_on_continuation() {
        let mut accumulated = Vec::new();
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "id": "call_1",
            "function": {"name": "original_name", "arguments": "{\"a\":"}
        }));
        merge_tool_call(&mut accumulated, json!({
            "index": 0,
            "function": {"name": "", "arguments": " 1}"}
        }));

        assert_eq!(accumulated[0]["function"]["name"], "original_name");
        assert_eq!(accumulated[0]["function"]["arguments"], "{\"a\": 1}");
    }
}
