curl http://127.0.0.1:8001/v1/code-completion -k \
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
  "stream": false,
  "no_cache": true,
  "parameters": {
    "temperature": 0.1,
    "max_new_tokens": 20
  }
}'

# Other possible parameters:
# "scratchpad": "FIM-PSM",
# "model": "smallcloudai/Refact-1_6b-fim",
