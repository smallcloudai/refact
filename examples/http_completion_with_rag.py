import os, json, requests
import initialization_for_scripts

sample_code = """use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use axum::http::StatusCode;
use ropey::Rope;
use crate::custom_error::ScratchError;

fn test_valid_post1() {
    let post = CodeCompletionPost {
        inputs: CodeCompletionInputs {
            sources: HashMap::from_iter([("hello.py".to_string(), "def hello_world():".to_string())]),
            cursor: CursorPosition {
                file: "hello.py".to_string(),
                line: 0,
                character: 18,
            },
            multiline: true,
        },
        parameters: SamplingParameters {
            max_new_tokens: 20,
            temperature: Some(0.1),
            top_p: None,
            stop: None,
        },
        model: "".to_string(),
        scratchpad: "".to_string(),
        stream: false,
        no_cache: false,
|
    assert!(crate::call_validation::validate_post(post).is_ok());
}
"""

def test_completion_with_rag():
    # target/debug/refact-lsp --address-url Refact --api-key SMALLCLOUD_API_KEY --http-port 8001 --workspace-folder ../refact-lsp --ast --logs-stderr
    response = requests.post(
        "http://127.0.0.1:8001/v1/code-completion",
        json={
            "inputs": {
                "sources": {
                    "hello.rs": sample_code,
                },
                "cursor": {
                    "file": "hello.rs",
                    "line": sample_code[:sample_code.find("|")].count("\n"),
                    "character": 0
                },
                "multiline": True
            },
            "stream": False,
            "no_cache": True,
            "parameters": {
                "temperature": 0.1,
                "max_new_tokens": 20,
            }
        },
        headers={
            "Content-Type": "application/json",
        },
        timeout=60,
    )
    j = response.json()
    print(json.dumps(j, indent=4))


if __name__ == "__main__":
    test_completion_with_rag()
