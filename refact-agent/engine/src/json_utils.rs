use crate::custom_error::MapErrToString;

pub fn extract_json_object<T: for<'de> serde::Deserialize<'de>>(text: &str) -> Result<T, String> {
    let start = text.find('{').ok_or_else(|| "No opening brace '{' found".to_string())?;
    let end = text.rfind('}').ok_or_else(|| "No closing brace '}' found".to_string())?;
    
    if end <= start {
        return Err("Closing brace appears before opening brace".to_string());
    }

    serde_json::from_str(&text[start..=end]).map_err_to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde::Deserialize;
    use indexmap::IndexMap;

    #[test]
    fn test_extract_json_clean_input() {
        let input = r#"{"key": "value", "number": 42}"#;
        let result: Result<serde_json::Value, _> = extract_json_object(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"key": "value", "number": 42}));
    }

    #[test]
    fn test_extract_json_with_text_before_after() {
        let input = "Some text before\n {\"key\": \"value\",\n \"number\": 42}\n\n and text after";
        let result: Result<serde_json::Value, _> = extract_json_object(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"key": "value", "number": 42}));
    }

    #[test]
    fn test_extract_json_nested() {
        let input = r#"LLM response: {"FOUND": {"file1.rs": "symbol1,symbol2"}, "SIMILAR": {"file2.rs": "symbol3"}}"#;
        let result: Result<IndexMap<String, IndexMap<String, String>>, _> = extract_json_object(input);
        assert!(result.is_ok());
        
        let map = result.unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("FOUND").unwrap().get("file1.rs").unwrap(), "symbol1,symbol2");
        assert_eq!(map.get("SIMILAR").unwrap().get("file2.rs").unwrap(), "symbol3");
    }
    
    #[derive(Deserialize, Debug, PartialEq)]
    struct FollowUpResponse {
        pub follow_ups: Vec<String>,
        pub topic_changed: bool,
    }
    
    #[test]
    fn test_follow_up_response_format() {
        let input = r#"
        Here are the follow up questions:
        ```json
        {
          "follow_ups": ["How?", "More examples", "Thank you"],
          "topic_changed": false
        }
        ```
        "#;
        
        let result: Result<FollowUpResponse, _> = extract_json_object(input);
        
        assert!(result.is_ok());
        let response = result.unwrap();
        
        assert_eq!(response.follow_ups, vec!["How?", "More examples", "Thank you"]);
        assert_eq!(response.topic_changed, false);
    }
}
