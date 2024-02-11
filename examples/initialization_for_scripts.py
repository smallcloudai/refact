import subprocess

cmdline = [
    "target/debug/refact-lsp",
    "--address-url", "Refact",
    "--api-key", "aaabbbxxxyyy",
    "--http-port", "8001",
    # "--lsp-port", "8002",
    # "--logs-stderr",
    "--files-jsonl-path", "hurray.jsonl",
    # "--vecdb",
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
        if line == "CAPS":
            break
    return rust


rust = start_rust()
print("DONE")
rust.terminate()
rust.wait()
