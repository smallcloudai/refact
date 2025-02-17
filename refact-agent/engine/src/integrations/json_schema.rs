use lazy_static::lazy_static;
use serde_json::{json, Value};

lazy_static! {
pub static ref INTEGRATION_JSON_SCHEMA: Value = json!({
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "properties": {
        "parameters": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "pattern": "^[a-zA-Z0-9_-]{1,64}$",
                        "description": "Parameter name - alphanumeric characters, underscores and hyphens, 1-64 chars"
                    },
                    "type": {
                        "type": "string",
                        "description": "Parameter type"
                    },
                    "description": {
                        "type": "string",
                        "description": "Parameter description"
                    }
                },
                "required": ["name", "type", "description"],
                "additionalProperties": false
            },
            "description": "List of command parameters"
        },
        "name": {
            "type": "string",
            "pattern": "^[a-zA-Z0-9_-]{1,64}$",
            "description": "Command name - alphanumeric characters, underscores and hyphens, 1-64 chars"
        }
    },
    "additionalProperties": true
});
}
