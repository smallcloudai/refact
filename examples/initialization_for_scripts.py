import os, json, subprocess, requests


def start_rust(cmdline):
    rust = subprocess.Popen(cmdline, stderr=subprocess.PIPE)
    ready = [0, 0]
    while True:
        if rust.poll() is not None:
            print("Oops, something went wrong, exit code: %i" % rust.returncode)
            exit(1)
        line = rust.stderr.readline().decode("utf-8").strip()
        print("RUST", line)
        if line == "AST COMPLETED":
            ready[0] = 1
        if line == "CAPS":
            ready[1] = 1
        if all(ready):
            break
    return rust


def test_ast_search():
    cmdline = [
        "target/debug/refact-lsp",
        "--address-url", "Refact",
        "--api-key", os.environ["SMALLCLOUD_API_KEY"],
        "--http-port", "8001",
        "--workspace-folder", os.path.dirname(os.path.dirname(__file__)),
        "--ast",
        # "--logs-stderr"
        ]
    print(" ".join(cmdline))
    rust = start_rust(cmdline)
    j = None
    try:
        response = requests.post(
            "http://127.0.0.1:8001/v1/ast-references-query-search",
            json={
                "query": "pretty_print_wrapper",
                "top_n": 3,
            },
            headers={
                "Content-Type": "application/json",
            },
            timeout=60,
        )
        print(response.text)
        j = response.json()
    finally:
        rust.terminate()
        rust.wait()
        print("DONE")
    print(json.dumps(j, indent=4))


if __name__ == "__main__":
    test_ast_search()
