import os, json, requests

sample_code = """import initialization_for_scripts;

def start():
    initialization_for_scripts.start_rust(|


if __name__ == "__main__":
    start()
"""



def test_completion_with_rag():
    # target/debug/refact-lsp --address-url Refact --api-key SMALLCLOUD_API_KEY --http-port 8001 --workspace-folder ../refact-lsp --ast --logs-stderr
    lines = sample_code.split("\n")
    cursor_line_n = sample_code[:sample_code.find("|")].count("\n");
    cursor_line = lines[cursor_line_n]
    print(f"cursor_line_n: {cursor_line_n}")
    print(f"cursor_line: {cursor_line}")
    response = requests.post(
        "http://127.0.0.1:8001/v1/code-completion",
        json={
            "inputs": {
                "sources": {
                    "hello.py": sample_code.replace("|", "")
                 },
                "cursor": {
                    "file": "hello.py",
                    "line": cursor_line_n,
                    "character": cursor_line.find("|"),
                },
                "multiline": True
            },
            "stream": False,
            "no_cache": True,
            "parameters": {
                "temperature": 0.1,
                "max_new_tokens": 20,
            },
            "use_ast": True,
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
