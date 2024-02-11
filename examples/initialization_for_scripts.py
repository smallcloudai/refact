import os, subprocess

cmdline = [
    "target/debug/refact-lsp",
    "--address-url", "Refact",
    "--api-key", os.environ["SMALLCLOUD_API_KEY"],
    "--http-port", "8001",
    "--files-jsonl-path", "hurray.jsonl",
    "--ast",
    ]
print(" ".join(cmdline))

def start_rust():
    rust = subprocess.Popen(cmdline, stderr=subprocess.PIPE)
    while True:
        if rust.poll() is not None:
            print("Oops, something went wrong, exit code: %i" % rust.returncode)
            exit(1)
        line = rust.stderr.readline().decode("utf-8").strip()
        print("RUST", line)
        if line == "AST COMPLETED":
            break
    return rust


rust = start_rust()
print("DONE")
rust.terminate()
rust.wait()
