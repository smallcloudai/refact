import os, json, requests, termcolor

# To test, run in a second console:
# target/debug/refact-lsp --address-url Refact --api-key SMALLCLOUD_API_KEY --http-port 8001 --workspace-folder tests/emergency_frog_situation --ast --logs-stderr
# and wait for AST COMPLETE

TEST_THESE_FILES = [
os.path.join(os.path.dirname(__file__), "emergency_frog_situation", "jump_to_conclusions.py"),
os.path.join(os.path.dirname(__file__), "emergency_frog_situation", "set_as_avatar.py"),
os.path.join(os.path.dirname(__file__), "emergency_frog_situation", "work_day.py"),
]

CURSOR_AT = "W, H"


def test_completion_with_rag(fpath):
    sample_code = open(fpath, "r").read()
    assert CURSOR_AT in sample_code
    lines = sample_code.split("\n")
    cursor_line_n = sample_code[:sample_code.find(CURSOR_AT)].count("\n");
    cursor_line_str = lines[cursor_line_n]
    cursor_column = cursor_line_str.find(CURSOR_AT)
    response = requests.post(
        "http://127.0.0.1:8001/v1/code-completion",
        json={
            "inputs": {
                "sources": {
                    fpath: sample_code.replace(CURSOR_AT, "")
                 },
                "cursor": {
                    "file": fpath,
                    "line": cursor_line_n,
                    "character": cursor_column,
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
    # print(json.dumps(j, indent=4))
    completion = j["choices"][0]["code_completion"]
    cleared_line_str = cursor_line_str.replace(CURSOR_AT, "")
    print(cleared_line_str[:cursor_column] + termcolor.colored(completion, "green") + cleared_line_str[cursor_column:])


if __name__ == "__main__":
    for x in TEST_THESE_FILES:
        print(x)
        test_completion_with_rag(x)
