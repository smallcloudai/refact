# Code Scratchpads

This code converts high level code completion calls to low level prompts and converts result back. This is useful for many IDE plugins (VS Code, JB) as a common code that handles the low level.


## Usage

Simple example:

```
curl http://127.0.0.1:8008/code-completion -k \
  -H 'Content-Type: application/json' \
  -d '{
  "inputs": {
    "sources": {"hello.py": "def hello_world():"},
    "cursor": {
      "file": "hello.py",
      "line": 0,
      "character": 18
    },
    "multiline": true
  },
  "model": "bigcode/starcoder",
  "stream": false,
  "parameters": {
    "temperature": 0.1,
    "max_new_tokens": 20
  }
}'
```

Output is `[{"code_completion": "\n    return \"Hello World!\"\n"}]`.

To check out more examples, look at [code_scratchpads/test_api.py](code_scratchpads/test_api.py).

