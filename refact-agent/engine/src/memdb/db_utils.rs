use serde_json::Value;

/// Merges two JSON values, with values from `b` overriding those in `a`.
/// 
/// This function recursively merges objects, replacing values in `a` with those from `b`.
/// For non-object values, `b` completely replaces the value in `a`.
/// 
/// # Arguments
/// 
/// * `a` - The target JSON value to be modified
/// * `b` - The source JSON value whose properties will be merged into `a`
pub fn merge_json(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                // Recursively merge nested objects
                merge_json(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            // For non-object values, simply replace the value
            *a = b.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_merge_json() {
        // Test merging objects
        let mut a = json!({
            "name": "John",
            "age": 30,
            "address": {
                "city": "New York",
                "zip": "10001"
            }
        });
        
        let b = json!({
            "age": 31,
            "address": {
                "street": "Broadway",
                "zip": "10002"
            }
        });
        
        merge_json(&mut a, &b);
        
        assert_eq!(a, json!({
            "name": "John",
            "age": 31,
            "address": {
                "city": "New York",
                "street": "Broadway",
                "zip": "10002"
            }
        }));
        
        // Test replacing non-object values
        let mut c = json!("old value");
        let d = json!("new value");
        
        merge_json(&mut c, &d);
        
        assert_eq!(c, json!("new value"));
    }
}
