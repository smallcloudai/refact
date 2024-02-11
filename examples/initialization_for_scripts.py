import os, json, subprocess, requests

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

def start_rust():
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


rust = start_rust()

symbol = "pretty_print_wrapper"
response = requests.post(
    "http://127.0.0.1:8001/v1/ast-query-search",
    json={
        "query": symbol,
        "top_n": 3,
    },
    headers={
        "Content-Type": "application/json",
    },
    timeout=60,
)
j = response.json()
# print(response.json())
rust.terminate()
rust.wait()
print("DONE")
print(json.dumps(j, indent=4))
