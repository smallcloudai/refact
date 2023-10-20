# Refact LSP Server

This code converts high level code completion or chat calls into low level LLM prompts and converts results back.

It's written in Rust, compiles into the `refact-lsp` binary. This binary is bunlded with
[VS Code](https://github.com/smallcloudai/refact-vscode/)
[JetBrains IDEs](https://github.com/smallcloudai/refact-intellij)
[VS Classic](https://github.com/smallcloudai/refact-vs-classic/),
[Sublime Text](https://github.com/smallcloudai/refact-sublime/),
and
[Qt Creator](https://github.com/smallcloudai/refact-qtcreator)
plugins.

It's a great way to organize code for the plugins, because it can absorb all the common logic, such as cache, debounce,
telemetry, scratchpads for different models.


## Compiling and Running

Depending on which API key you have handy, or maybe you have Refact self-hosting server:

```
cargo build && target/debug/refact-lsp --address-url Refact --api-key YYYY --http-port 8001 --lsp-port 8002 --logs-stderr
cargo build && target/debug/refact-lsp --address-url HF --api-key hf_XXXX --http-port 8001 --lsp-port 8002 --logs-stderr
cargo build && target/debug/refact-lsp --address-url http://127.0.0.1:8008/ --http-port 8001 --lsp-port 8002 --logs-stderr
```

Try `--help` for more options.


## Usage

HTTP example:

```
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
  "parameters": {
    "temperature": 0.1,
    "max_new_tokens": 20
  }
}'
```

Output is `[{"code_completion": "\n    return \"Hello World!\"\n"}]`.

[LSP example](examples/lsp_completion.py)


## Telemetry

The flags `--basic-telemetry` and `--snippet-telemetry` control what telemetry is sent. To be clear: without
these flags, no telemetry is sent. Those flags are typically controlled from IDE plugin settings.

Basic telemetry means counters and error messages without information about you or your code. It is "compressed"
into `.cache/refact/telemetry/compressed` folder, then from time to time it's sent and moved
to `.cache/refact/telemetry/sent` folder.

"Compressed" means similar records are joined together, increasing the counter. "Sent" means the rust binary
communicates with a HTTP endpoint specified in caps (see Caps section below) and sends .json file exactly how
you see it in `.cache/refact/telemetry`. The files are human-readable.

When using Refact self-hosted server, telemetry goes to the self-hosted server, not to the cloud.


## Caps File

The `--address-url` parameter controls the behavior of this program by a lot. The address is first used
to construct `$URL/coding_assistant_caps.json` address to fetch the caps file. Furthermore, there are
compiled-in caps you can use by magic addresses "Refact" and "HF".

The caps file describes which models are running, default models for completion and chat,
where to send the telemetry, how to download a
tokenizer, where is the endpoint to access actual language models. To read more, check out
compiled-in caps in [caps.rs](src/caps.rs).


## Tests

The one to run often is [test_edge_cases.py](tests/test_edge_cases.py).

You can also run [measure_humaneval_fim.py](tests/measure_humaneval_fim.py) for your favorite model.


## Credits

The initial version of this project was written by looking at llm-ls by [@McPatate](https://github.com/McPatate). He's a Rust fan who inspired this project!
